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

def MAX_KEY_LEN: size = PAGE_SIZE / 8;
def MAX_VAL_LEN: size = PAGE_SIZE / 4;

export type btree = struct {
	cache: page_cache,
	root: node,
	// TODO: memory-map the file?
	file: io::file,
	free_page_list: []size
};

export fn dump_btree(f: io::handle, bt: *btree) void = {
	fmt::fprintln(f, "Root:")!;
	let worklist: [](int, node) = alloc([(0, bt.root)], 16);
	defer free(worklist);
	let ident: []u8 = [];
	for (let i: size = 0; i < len(worklist); i += 1) {
		let (depth, node) = worklist[i];
		let ident: []u8 = ident[0..0];
		for (let i = 0; i < depth; i += 1)
			append(ident, '\t': u8);
		dump_node(f, node, strings::fromutf8(ident)!);
		if (node.typ == node_type::INNER)
			for (let i: size = 0; i <= node.num_keys; i += 1) {
				let n = read_node(bt, get_off(node, i): size * PAGE_SIZE)!;
				append(worklist, (depth + 1, n));
			};
	};
	defer free(ident);
};

export type page_cache = struct {
	// The last time this page was looked up and the page itself.
	// TODO: Cache eviction.
	htable: [32][](time::instant, node),
	entries: uint
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
	case void => return void;
	case let n: node => {
		// TODO: Free the old root! Write the change to disk!
		bt.root = n;
		insert_into_cache(&bt.cache, n);
		return void;
	};
	case let split: (node, node, size, []u8) => {
		// TODO: Free the old root! Write the change to disk!
		let (a, b, split_idx, split_key) = split;
		// log::printfln("insert_into_root(n.page_offset={}) -> split!(a.page_offset={}, b.page_offset={})",
		// bt.root.page_offset, a.page_offset, b.page_offset);
		insert_into_cache(&bt.cache, a);
		insert_into_cache(&bt.cache, b);
		let (page_offset, page) = allocate_page(bt)?;
		bt.root = build_inner_node([split_key], [
			(a.page_offset / PAGE_SIZE): u32,
			(b.page_offset / PAGE_SIZE): u32], page_offset, page).0;
		return void;
	};
	};
};

fn insert_into_node(bt: *btree, n: node, key: []u8, val: []u8) (void | node | (node, node, size, []u8) | io::error) = {
	if (n.typ == node_type::LEAF)
		return insert_into_leaf(bt, n, key, val);

	let idx = lookup_in_node(n, key).1;
	let child = read_node(bt, get_off(n, idx): size * PAGE_SIZE)?;
	match (insert_into_node(bt, child, key, val)?) {
	case void => return void;
	case let newnode: node => {
		// TODO: No copy-on-write here?! Write needs to be flushed to disk!
		set_off(n, idx, (newnode.page_offset / PAGE_SIZE): u32);
		return void;
	};
	case let split: (node, node, size, []u8) => {
		// The child was splitted.
		// TODO: Free the original child! Write the change to disk!
		// TODO: Completely untested!
		let (a, b, split_idx, split_key) = split;
		fmt::fatal("TODO");
	};
	};
};

fn insert_into_inner(bt: *btree, n: node, key: []u8, page_offset: size) (void | node | (node, node, size, []u8) | io::error) = {
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
		// TODO: No copy-on-write here?! Write needs to be flushed to disk!
		set_off(n, insert_idx, off);
		write_page_to_disk(bt, n)?;
		return void;
	};

	let a = allocate_page(bt)?;
	append(bt.free_page_list, n.page_offset);
	remove_from_cache(&bt.cache, n.page_offset);
	if (size_after_insert < PAGE_SIZE) {
		let n = build_inner_node(keys, offs, a.0, a.1).0;
		write_page_to_disk(bt, n)?;
		return n;
	} else {
		let b = allocate_page(bt)?;
		// split_key will not be in either half and needs to be inserted
		// into the parent!
		let split_idx = len(keys) / 2;
		let split_key = keys[split_idx];
		let a = build_inner_node(keys[..(split_idx+1)], offs[..(split_idx+2)], a.0, a.1).0;
		let b = build_inner_node(keys[(split_idx+2)..], offs[(split_idx+2)..], b.0, b.1).0;
		write_page_to_disk(bt, a)?;
		write_page_to_disk(bt, b)?;
		return (a, b, split_idx, split_key);
	};
};

fn insert_into_leaf(bt: *btree, n: node, key: []u8, val: []u8) (node | (node, node, size, []u8) | io::error) = {
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
	append(bt.free_page_list, n.page_offset);
	remove_from_cache(&bt.cache, n.page_offset);
	if (size_after_insert < PAGE_SIZE) {
		let n = build_leaf_node(keys, vals, a.0, a.1).0;
		write_page_to_disk(bt, n)?;
		return n;
	} else {
		let b = allocate_page(bt)?;
		let split_idx = len(keys) / 2;
		let split_key = keys[split_idx];
		let a = build_leaf_node(keys[..(split_idx+1)], vals[..(split_idx+1)], a.0, a.1).0;
		let b = build_leaf_node(keys[(split_idx+1)..], vals[(split_idx+1)..], b.0, b.1).0;
		// log::printfln("insert_into_leaf(n.page_offset={}) -> split!(a.page_offset={}, b.page_offset={})",
		// n.page_offset, a.page_offset, b.page_offset);
		write_page_to_disk(bt, a)?;
		write_page_to_disk(bt, b)?;
		return (a, b, split_idx, split_key);
	};
};

fn read_node(bt: *btree, page_offset: size) (node | io::error) = {
	// log::printfln("read_page(page_offset={})...", page_offset);
	match (find_in_cache(&bt.cache, page_offset)) {
	case void => yield;
	case let n: node => return n;
	};

	io::seek(bt.file, page_offset: io::off, io::whence::SET)?;
	let buf: *[PAGE_SIZE]u8 = alloc([0...]);
	match (io::readall(bt.file, buf)) {
	case let n: size => assert(n == PAGE_SIZE);
	case let err: io::error => { free(buf); return err; };
	case io::EOF => { free(buf); return errors::invalid; };
	};

	let n: node = get_node(buf);
	assert(n.page_offset == page_offset);
	insert_into_cache(&bt.cache, n);
	return n;
};

// Because there is no memory-mapping (yet), returned buffer is dynamically allocated.
fn allocate_page(bt: *btree) ((size, *[PAGE_SIZE]u8) | io::error) = {
	let N = len(bt.free_page_list);
	let page_offset: size = if (N > 0) {
		let res = bt.free_page_list[N - 1];
		// TODO: Is the & needed here? I like hare in general, but I
		// prefere the go-style append builtin for slices, it seams
		// more intuitive to me.
		bt.free_page_list = bt.free_page_list[0..(N - 1)];
		yield res;
	} else {
		let end = io::seek(bt.file, 0, io::whence::END)?;
		assert(end: size % PAGE_SIZE == 0);
		// This is a hacky workaround, the end could also be tracked in
		// the btree structure. Write a byte at the end of the newly
		// assigned page so that the next seek to end does not return
		// the same position.
		io::seek(bt.file, end + PAGE_SIZE: io::off - 1, io::whence::SET)?;
		io::write(bt.file, [0])?;
		yield end: size;
	};

	log::printfln("allocate_page() -> page_offset={}...", page_offset);
	assert(find_in_cache(&bt.cache, page_offset) is void,
		"page should not have been cached!");
	return (page_offset, alloc([0...]): *[PAGE_SIZE]u8);
};

fn write_page_to_disk(bt: *btree, n: node) (void | io::error) = {
	io::seek(bt.file, n.page_offset: io::off, io::whence::SET)?;
	match (io::writeall(bt.file, n.raw)) {
	case let n: size => assert(n == PAGE_SIZE);
	case let err: io::error => return err;
	};
};

fn hash_page_offset(x: size) size = {
	assert(x % PAGE_SIZE == 0, "page_offset must be multiple of page size");
	return (x / PAGE_SIZE) * 7;
};

fn find_in_cache(cache: *page_cache, page_offset: size) (void | node) = {
	let bucket = &cache.htable[hash_page_offset(page_offset) % len(cache.htable)];
	for (let i: size = 0; i < len(bucket); i += 1) {
		let slot = &bucket[i];
		if (slot.1.page_offset != page_offset)
			continue;

		slot.0 = time::now(time::clock::MONOTONIC);
		return slot.1;
	};
	return void;
};

fn insert_into_cache(cache: *page_cache, n: node) void = {
	let bucket = &cache.htable[hash_page_offset(n.page_offset) % len(cache.htable)];
	for (let i: size = 0; i < len(bucket); i += 1)
		assert(bucket[i].1.page_offset != n.page_offset,
			"Page already in cache?");
	append(bucket, (time::now(time::clock::MONOTONIC), n));
	cache.entries += 1;
};

fn remove_from_cache(cache: *page_cache, page_offset: size) bool = {
	let bucket = &cache.htable[hash_page_offset(page_offset) % len(cache.htable)];
	for (let i: size = 0; i < len(bucket); i += 1)
		if (bucket[i].1.page_offset == page_offset) {
			delete(bucket[i]);
			cache.entries -= 1;
			log::printfln("remove_from_cache(page_offset={}) -> Deleted!", page_offset);
			return true;
		};
	log::printfln("remove_from_cache(page_offset={}) -> Was not cached!", page_offset);
	return false;
};

fn remove_older_than(cache: *page_cache, t: time::instant) uint = {
	let n: uint = 0;
	for (let i: size = 0; i < len(cache.htable); i += 1) {
		let bucket = &cache.htable[i];
		for (let i: size = 0; i < len(bucket); i += 1) {
			let entry = &bucket[i];
			let d = time::diff(entry.0, t);
			if (d > 0) {
				delete(bucket[i]);
				i -= 1;
				n += 1;
			};
		};
	};
	return n;
};

@test fn test_btree_basics() void = {
	let f: io::file = temp::file(io::mode::RDWR, 0o755)!;
	defer io::close(f)!;
	let fake_root_buf: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(fake_root_buf);
	let bt = btree {
		cache = page_cache {
			htable = [[]...],
			entries = 0
		},
		root = build_leaf_node([], [], 0, fake_root_buf).0,
		file = f,
		free_page_list = []
	};
	assert(allocate_page(&bt)!.0 == 0);
	assert(get(&bt, strings::toutf8("foo"))! is void);

	const NUM_ELEMENTS: size = 200;
	const KEY_LEN: size = 15;
	const VAL_LEN: size = 25;

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

	shuffle_test_data(&r, test_data);
	for (let i: size = 0; i < len(test_data); i += 1)
		set(&bt,
			strings::toutf8(test_data[i].0),
			strings::toutf8(test_data[i].1))!;
	dump_btree(os::stderr, &bt);
	shuffle_test_data(&r, test_data);
	for (let i: size = 0; i < len(test_data); i += 1) {
		let val = get(&bt, strings::toutf8(test_data[i].0))!;
		assert(strings::fromutf8(val as []u8)! == test_data[i].1);
	};
	assert(false, "just to dump stderr");
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
	static const alphabet: str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-/";
	let alphabet = strings::toutf8(alphabet);
	let bytes: []u8 = [];
	for (let i: size = 0; i < l; i += 1)
		append(bytes, alphabet[random::next(r): size % len(alphabet)]);
	return strings::fromutf8(bytes)!;
};

