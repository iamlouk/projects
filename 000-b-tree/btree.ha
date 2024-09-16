use bytes;
use errors;
use fmt;
use io;
use log;
use math::random;
use os;
use strings;
use temp;
use time;

export def PAGE_SIZE: size = 4096;
def MAX_KEY_LEN: size = PAGE_SIZE / 4;
def MAX_VAL_LEN: size = PAGE_SIZE / 4;

export type btree = struct {
	root: node,
	nodes: [](size, *[PAGE_SIZE]u8)
};

export fn dump_btree(f: io::handle, bt: *btree) int = {
	fmt::fprintln(f, "Root:")!;
	let worklist: [](int, node) = alloc([(0, bt.root)], 16);
	defer free(worklist);
	let ident: []u8 = [];
	defer free(ident);
	let leafs_ordered = true;
	let prev_key: []u8 = [];
	let maxdepth = 0;
	for (let i: size = 0; i < len(worklist); i += 1) {
		let (depth, node) = worklist[i];
		if (depth > maxdepth) maxdepth = depth;
		let ident: []u8 = ident[0..0];
		for (let i = 0; i < depth; i += 1)
			append(ident, '\t': u8);
		dump_node(f, node, strings::fromutf8(ident)!);
		if (node.typ == node_type::INNER) {
			for (let i: size = 0; i <= node.num_keys; i += 1) {
				let n = read_node(bt, get_off(node, i): size * PAGE_SIZE)!;
				append(worklist, (depth + 1, n));
			};
		} else {
			let first_key = get_key(node, 0);
			leafs_ordered &&= len(prev_key) == 0 || compare_bytes(prev_key, first_key) <= 0;
			prev_key = get_key(node, node.num_keys - 1);
		};
	};
	assert(leafs_ordered, "leafs not visited in increasing order");
	return maxdepth;
};

// Lookup a key in a btree and get the result.
export fn get(bt: *btree, key: []u8) (void | io::error | []u8) = {
	let n = bt.root;
	for (true) {
		let res = lookup_in_node(n, key);
		if (n.typ == node_type::LEAF && res.0 == false)
			return void;
		if (n.typ == node_type::LEAF)
			return get_val(n, res.1);

		let next_node_pos = get_off(n, res.1): size * PAGE_SIZE;
		n = read_node(bt, next_node_pos)?;
	};
};

export fn set(bt: *btree, key: []u8, val: []u8) (void | io::error) = {
	assert(len(key) <= MAX_KEY_LEN, "key too long");
	assert(len(val) <= MAX_KEY_LEN, "val too long");
	match (insert_into_node(bt, bt.root, key, val)?) {
	case let n: node => {
		bt.root = n;
		return void;
	};
	case let split: (node, node, []u8) => {
		let (a, b, split_key) = split;
		let (page_offset, page) = allocate_page(bt)?;
		bt.root = build_inner_node([split_key], [
			(a.page_offset / PAGE_SIZE): u32,
			(b.page_offset / PAGE_SIZE): u32], page_offset, page).0;
		return void;
	};
	};
};

fn insert_into_node(bt: *btree, n: node, key: []u8, val: []u8) (node | (node, node, []u8) | io::error) = {
	if (n.typ == node_type::LEAF)
		return insert_into_leaf(bt, n, key, val);

	let idx = lookup_in_node(n, key).1;
	let child = read_node(bt, get_off(n, idx): size * PAGE_SIZE)?;
	match (insert_into_node(bt, child, key, val)?) {
	case let newnode: node => {
		// The child was replaced by a new node, e.g. because of a CoW
		// insert. No CoW here, just in-place update! Risky!
		set_off(n, idx, (newnode.page_offset / PAGE_SIZE): u32);
		return n;
	};
	case let split: (node, node, []u8) => {
		let (a, b, split_key) = split;
		set_off(n, idx, (b.page_offset / PAGE_SIZE): u32);
		return insert_into_inner(bt, n, split_key, a.page_offset);
	};
	};
};

fn insert_into_inner(bt: *btree, n: node, key: []u8, page_offset: size) (node | (node, node, []u8) | io::error) = {
	assert(n.typ == node_type::INNER);
	assert(page_offset % PAGE_SIZE == 0, "page_offset must be multiple of page size");
	let off = (page_offset / PAGE_SIZE): u32;
	let keys: [][]u8 = get_keys(n, []);
	defer free(keys);
	let offs: []u32 = get_offs(n, []);
	defer free(offs);

	let size_after_insert = size_in_bytes(n);
	let (key_exists, insert_idx) = lookup_in_node(n, key);
	if (!key_exists) {
		insert(keys[insert_idx], key);
		insert(offs[insert_idx], off);
		// 2 bytes for the key offsets table, and 4 for the page offset.
		size_after_insert += 6 + len(key);
	} else {
		assert(false, "WTF?");
		return errors::invalid;
	};

	let a = allocate_page(bt)?;
	if (size_after_insert < PAGE_SIZE) {
		return build_inner_node(keys, offs, a.0, a.1).0;
	} else {
		let b = allocate_page(bt)?;
		// split_key will not be in either half and needs to be inserted
		// into the parent!
		let split_idx = len(keys) / 2;
		let split_key = keys[split_idx];
		let a = build_inner_node(keys[..(split_idx)], offs[..(split_idx+1)], a.0, a.1).0;
		let b = build_inner_node(keys[(split_idx+1)..], offs[(split_idx+1)..], b.0, b.1).0;
		return (a, b, split_key);
	};
};

fn insert_into_leaf(bt: *btree, n: node, key: []u8, val: []u8) (node | (node, node, []u8) | io::error) = {
	assert(n.typ == node_type::LEAF);
	let keys: [][]u8 = get_keys(n, []);
	defer free(keys);
	let vals: [][]u8 = get_vals(n, []);
	defer free(vals);

	let size_after_insert = size_in_bytes(n);
	let (key_exists, insert_idx) = lookup_in_node(n, key);
	if (!key_exists) {
		insert(keys[insert_idx], key);
		insert(vals[insert_idx], val);
		// 2 bytes if the offsets table, 2 bytes for the key len, and
		// 2 bytes for the val len.
		size_after_insert += 6 + len(key) + len(val);
	} else {
		size_after_insert -= len(vals[insert_idx]);
		size_after_insert += len(val);
		vals[insert_idx] = val;
	};

	let a = allocate_page(bt)?;
	if (size_after_insert < PAGE_SIZE) {
		return build_leaf_node(keys, vals, a.0, a.1).0;
	} else {
		let b = allocate_page(bt)?;
		let split_idx = len(keys) / 2 - 1;
		let split_key = keys[split_idx];
		let a = build_leaf_node(keys[..(split_idx+1)], vals[..(split_idx+1)], a.0, a.1).0;
		let b = build_leaf_node(keys[(split_idx+1)..], vals[(split_idx+1)..], b.0, b.1).0;
		return (a, b, split_key);
	};
};

fn read_node(bt: *btree, page_offset: size) (node | io::error) = {
	for (let i: size = 0; i < len(bt.nodes); i += 1)
		if (bt.nodes[i].0 == page_offset) {
			let n = get_node(bt.nodes[i].1);
			assert(n.page_offset == page_offset);
			return n;
		};
	return errors::invalid;
};

fn allocate_page(bt: *btree) ((size, *[PAGE_SIZE]u8) | io::error) = {
	let page_offset = len(bt.nodes) * PAGE_SIZE;
	let res = (page_offset, alloc([0...]): *[PAGE_SIZE]u8);
	append(bt.nodes, res);
	return res;
};

@test fn test_btree_basics() void = {
	let f: io::file = temp::file(io::mode::RDWR, 0o755)!;
	defer io::close(f)!;
	let fake_root_buf: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(fake_root_buf);
	let bt = btree {
		root = build_leaf_node([], [], 0, fake_root_buf).0,
		nodes = []
	};
	append(bt.nodes, (0, bt.root.raw));
	assert(get(&bt, strings::toutf8("foo"))! is void);

	const NUM_ELEMENTS: size = 2000;
	const KEY_LEN: size = 50;
	const VAL_LEN: size = 50;

	let test_data: [](str, str) = [];
	defer {
		for (let i: size = 0; i < len(test_data); i += 1) {
			free(test_data[i].0);
			free(test_data[i].1);
		};
		free(test_data);
	};
	let r = random::init(time::unix(time::now(time::clock::REALTIME)): u64);
	for (let i: size = 0; i < NUM_ELEMENTS; i += 1) {
		let key = random_string(&r, KEY_LEN);
		let val = random_string(&r, VAL_LEN);
		append(test_data, (key, val));
	};

	for (let i: size = 0; i < len(test_data); i += 1)
		set(&bt,
			strings::toutf8(test_data[i].0),
			strings::toutf8(test_data[i].1))!;
	
	let depth = dump_btree(os::stderr, &bt);
	assert(depth >= 2, "splitting of inner nodes not tested");

	shuffle_test_data(&r, test_data);
	for (let i: size = 0; i < len(test_data); i += 1) {
		let val = get(&bt, strings::toutf8(test_data[i].0))!;
		assert(strings::fromutf8(val as []u8)! == test_data[i].1);
	};
};

fn shuffle_test_data(r: *random::random, data: [](str, str)) void = {
	let N = len(data);
	let num_shuffles = N / 2;
	for (let i: size = 0; i < num_shuffles; i += 1) {
		let idx1 = random::next(r): size % N;
		let idx2 = random::next(r): size % N;
		let tmp = data[idx1];
		data[idx1] = data[idx2];
		data[idx2] = tmp;
	};
};

fn random_string(r: *random::random, l: size) str = {
	static const alphabet: str = "123456789";
	let alphabet = strings::toutf8(alphabet);
	let bytes: []u8 = [];
	for (let i: size = 0; i < l; i += 1)
		append(bytes, alphabet[random::next(r): size % len(alphabet)]);
	return strings::fromutf8(bytes)!;
};

