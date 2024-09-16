use bytes;
use fmt;
use strings;

export def PAGE_SIZE: size = 4096;

export type node_type = enum { INNER, LEAF };

export type node = struct {
	typ: node_type,
	page_offset: size,
	num_keys: size,
	// TODO: Add offsets here?
	raw: *[PAGE_SIZE]u8
};

fn empty_node(typ: node_type, page_offset: size, raw: *[PAGE_SIZE]u8) node = {
	assert(page_offset % PAGE_SIZE == 0);
	let page_offset = page_offset / PAGE_SIZE;
	raw[0] = (if (typ == node_type::INNER) 0x80u8 else 0x00u8) | ((page_offset >> 8): u8);
	raw[1] = ((page_offset >> 0): u8);
	return node {
		typ = typ,
		page_offset = page_offset,
		num_keys = 0,
		raw = raw
	};
};

fn get_node(raw: *[PAGE_SIZE]u8) node = {
	let pageid = ((raw[0]: u16) << 8) | (raw[1]: u16);
	let num_keys = ((raw[2]: u16) << 8) | (raw[3]: u16);
	return node {
		typ = if ((pageid & 0x8000) != 0) node_type::INNER else node_type::LEAF,
		page_offset = (pageid & 0x7fff): size * PAGE_SIZE,
		num_keys = num_keys: size,
		raw = raw
	};
};

fn size_in_bytes(n: node) size = {
	let last_offset = 4 + 2*(n.num_keys);
	let s = ((n.raw[last_offset+0]: size) << 8)
		| ((n.raw[last_offset+1]: size) << 0);
	assert(s <= PAGE_SIZE);
	return s;
};

fn get_keys(n: node, dst: [][]u8) [][]u8 = {
	let dst = dst[0..0];
	for (let i: size = 0; i < n.num_keys) {
		let s = ((n.raw[4 + i*2]: size) << 8) | (n.raw[4 + i*2 + 1]: size);
		i += 1;
		if (n.typ == node_type::LEAF) {
			let keylen = ((n.raw[s]: size) << 8) | (n.raw[s + 1]: size);
			append(dst, n.raw[(s + 4)..(s + 4 + keylen)]);
			continue;
		};
		let e = ((n.raw[4 + i*2]: size) << 8) | (n.raw[4 + i*2 + 1]: size);
		append(dst, n.raw[s..e]);
	};
	return dst;
};

fn get_positions(n: node, dst: []size) []size = {
	assert(n.typ == node_type::INNER);
	let dst = dst[0..0];
	let s = 4 + 2*(n.num_keys + 1);
	for (let i: size = 0; i < n.num_keys + 1; i += 1) {
		let off = ((n.raw[s+0]: size) << 24)
			| ((n.raw[s+1]: size) << 16)
			| ((n.raw[s+2]: size) << 8)
			| ((n.raw[s+3]: size) << 0);
		append(dst, (off: size) * PAGE_SIZE);
		s += 4;
	};
	return dst;
};

fn get_position(n: node, i: size) size = {
	assert(n.typ == node_type::INNER && i <= n.num_keys);
	let s = 4 + 2*(n.num_keys + 1) + (i * 4);
	let off = ((n.raw[s+0]: size) << 24)
		| ((n.raw[s+1]: size) << 16)
		| ((n.raw[s+2]: size) << 8)
		| ((n.raw[s+3]: size) << 0);
	return (off: size) * PAGE_SIZE;
};

fn get_vals(n: node, dst: [][]u8) [][]u8 = {
	assert(n.typ == node_type::LEAF);
	let dst = dst[0..0];
	for (let i: size = 0; i < n.num_keys; i += 1) {
		let s = ((n.raw[4 + i*2]: size) << 8) | (n.raw[4 + i*2 + 1]: size);
		let keylen = ((n.raw[s + 0]: size) << 8) | (n.raw[s + 1]: size);
		let vallen = ((n.raw[s + 2]: size) << 8) | (n.raw[s + 3]: size);
		append(dst, n.raw[(s + 4 + keylen)..(s + 4 + keylen + vallen)]);
	};
	return dst;
};

fn get_val(n: node, i: size) []u8 = {
	assert(n.typ == node_type::LEAF && i < n.num_keys);
	let s = 4 + 2*i;
	let s = ((n.raw[s + 0]: size) << 8) | (n.raw[s + 1]: size);
	let keylen = ((n.raw[s + 0]: size) << 8) | (n.raw[s + 1]: size);
	let vallen = ((n.raw[s + 2]: size) << 8) | (n.raw[s + 3]: size);
	return n.raw[(s + 4 + keylen)..(s + 4 + keylen + vallen)];
};

// Returns void if this is a leaf node and the key if not present,
// []u8 if this is a leaf node and the key is present, and the
// page offset of the child node that could contain the key if this
// is a inner node.
// TODO: Use binary search?
fn lookup_in_node(n: node, search_key: []u8) (bool, (size | []u8)) = {
	let keys = get_keys(n, []);
	defer free(keys);
	if (n.typ == node_type::LEAF) {
		for (let i: size = 0; i < n.num_keys; i += 1) {
			let cmp = compare_bytes(search_key, keys[i]);
			if (cmp == 0)
				return (true, get_val(n, i));

			if (cmp < 0)
				return (false, i);
		};
		return (false, n.num_keys);
	};

	for (let i: size = 0; i < n.num_keys; i += 1) {
		let cmp = compare_bytes(search_key, keys[i]);
		if (cmp <= 0)
			return (cmp == 0, get_position(n, i));
	};
	return (false, get_position(n, n.num_keys));
};

type full = !void;

fn can_insert_into_leaf(n: node, key: []u8, val: []u8) (void | full) = {
	assert(n.typ == node_type::LEAF);
	assert(len(key) <= PAGE_SIZE / 4, "key is too large");
	assert(len(val) <= PAGE_SIZE / 4, "val is too large");
	let s = size_in_bytes(n);
	s += 6 + len(key) + len(val);
	if (s > PAGE_SIZE)
		return full;
	return void;
};

// A copy-on-write insert into a leaf.
fn insert_into_leaf(orig: node, key: []u8, val: []u8, new_raw: *[PAGE_SIZE]u8) node = {
	assert(can_insert_into_leaf(orig, key, val) is void);
	let keys = get_keys(orig, []);
	defer free(keys);
	let vals = get_vals(orig, []);
	defer free(vals);
	let (key_exists, res) = lookup_in_node(orig, key);
	

	fmt::fatal("TODO...");
};

@test fn test_inner_node() void = {
	// The test data:
	let keys: [][]u8 = [];
	append(keys, strings::toutf8("bar"));
	append(keys, strings::toutf8("foo"));
	append(keys, strings::toutf8("hello"));
	append(keys, strings::toutf8("world"));
	let positions: []u32 = [];
	append(positions, 123);
	append(positions, 42);
	append(positions, 321);
	append(positions, 420);
	append(positions, 1234);
	assert(len(keys) + 1 == len(positions));

	// Setup of the page:
	let raw: [PAGE_SIZE]u8 = [0...];
	let buf: []u8 = raw[0..0];
	static append(buf, 0x80);
	static append(buf, 42);
	static append(buf, (len(keys) >> 8): u8);
	static append(buf, (len(keys) & 0xff): u8);
	// The keys start after the offsets and child node positions.
	let keys_off = 4 + 2*(len(keys) + 1) + 4*(len(keys) + 1);
	for (let i: size = 0; i < len(keys); i += 1) {
		static append(buf, (keys_off >> 8): u8);
		static append(buf, keys_off: u8);
		keys_off += len(keys[i]);
	};
	static append(buf, (keys_off >> 8): u8);
	static append(buf, keys_off: u8);
	for (let i: size = 0; i < len(positions); i += 1) {
		let p = positions[i];
		static append(buf, (p >> 24): u8);
		static append(buf, (p >> 16): u8);
		static append(buf, (p >> 8): u8);
		static append(buf, (p >> 0): u8);
	};
	for (let i: size = 0; i < len(keys); i += 1) {
		static append(buf, keys[i]...);
	};

	// The actual test:
	let n = get_node(&raw);
	assert(size_in_bytes(n) == len(buf));
	assert(n.typ == node_type::INNER);
	assert(n.page_offset == 42 * PAGE_SIZE);
	assert(n.num_keys == len(keys));
	let test_keys = get_keys(n, []);
	defer free(test_keys);
	assert(len(test_keys) == len(keys));
	for (let i: size = 0; i < len(keys); i += 1) {
		if (i > 1) {
			assert(compare_bytes(keys[i-1], keys[i]) == -1);
		};
		assert(strings::fromutf8(keys[i])! == strings::fromutf8(test_keys[i])!);
	};

	let test_positions = get_positions(n, []);
	defer free(test_positions);
	assert(len(test_positions) == len(positions));
	for (let i: size = 0; i < len(positions); i += 1) {
		assert(positions[i] * PAGE_SIZE == test_positions[i]);
		assert(positions[i] * PAGE_SIZE == get_position(n, i));
	};

	assert(lookup_in_node(n, strings::toutf8("aaa")).1 as size == positions[0] * PAGE_SIZE);
	for (let i: size = 0; i < len(keys); i += 1) {
		assert(lookup_in_node(n, keys[i]).1 as size == positions[i] * PAGE_SIZE);
	};
	assert(lookup_in_node(n, strings::toutf8("zzz")).1 as size == positions[len(keys)] * PAGE_SIZE);
};

@test fn test_leaf_node() void = {
	// The test data:
	let keys: [][]u8 = [];
	append(keys, strings::toutf8("bar"));
	append(keys, strings::toutf8("foo"));
	append(keys, strings::toutf8("hello"));
	append(keys, strings::toutf8("world"));
	let vals: [][]u8 = [];
	append(vals, strings::toutf8("first"));
	append(vals, strings::toutf8("second"));
	append(vals, strings::toutf8("third"));
	append(vals, strings::toutf8("fourth"));
	assert(len(keys) == len(vals));

	// Setup of the page:
	let raw: [PAGE_SIZE]u8 = [0...];
	let buf: []u8 = raw[0..0];
	static append(buf, 0x00);
	static append(buf, 42);
	static append(buf, (len(keys) >> 8): u8);
	static append(buf, (len(keys) & 0xff): u8);
	// The keys start after the offsets.
	let keyvals_off = 4 + 2*(len(keys) + 1);
	for (let i: size = 0; i < len(keys); i += 1) {
		static append(buf, (keyvals_off >> 8): u8);
		static append(buf, keyvals_off: u8);
		keyvals_off += 4 + len(keys[i]) + len(vals[i]);
	};
	static append(buf, (keyvals_off >> 8): u8);
	static append(buf, keyvals_off: u8);
	for (let i: size = 0; i < len(vals); i += 1) {
		let key = keys[i];
		let val = vals[i];
		static append(buf, (len(key) >> 8): u8);
		static append(buf, (len(key) >> 0): u8);
		static append(buf, (len(val) >> 8): u8);
		static append(buf, (len(val) >> 0): u8);
		static append(buf, key...);
		static append(buf, val...);
	};

	// The actual test:
	let n = get_node(&raw);
	assert(size_in_bytes(n) == len(buf));
	assert(n.typ == node_type::LEAF);
	assert(n.page_offset == 42 * PAGE_SIZE);
	assert(n.num_keys == len(keys));
	let test_keys = get_keys(n, []);
	defer free(test_keys);
	assert(len(test_keys) == len(keys));
	for (let i: size = 0; i < len(keys); i += 1) {
		assert(strings::fromutf8(keys[i])! == strings::fromutf8(test_keys[i])!);
	};

	let test_vals = get_vals(n, []);
	defer free(test_vals);
	assert(len(test_vals) == len(vals));
	for (let i: size = 0; i < len(keys); i += 1) {
		assert(bytes::equal(test_vals[i], get_val(n, i)));
		assert(strings::fromutf8(vals[i])! == strings::fromutf8(test_vals[i])!);
	};

	assert(lookup_in_node(n, strings::toutf8("notfound")).0 == false);
	for (let i: size = 0; i < len(keys); i += 1) {
		let res = lookup_in_node(n, keys[i]);
		assert(res.0 == true && bytes::equal(vals[i], res.1 as []u8));
	};
};

fn compare_bytes(a: []u8, b: []u8) int = {
	let len_a = len(a), len_b = len(b);
	let i: size = 0;
	for (i < len_a && i < len_b) {
		if (a[i] == b[i]) {
			i += 1;
			continue;
		};
		return if (a[i] < b[i]) -1 else 1;
	};
	return if (len_a < len_b) -1 else if (len_a == len_b) 0 else 1;
};

