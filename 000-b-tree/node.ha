use bytes;
use fmt;
use io;
use strings;

// TODO: Add more 'const'!

export type node_type = enum { INNER, LEAF };

export type node = struct {
	typ: node_type,
	page_offset: size,
	num_keys: size,
	// TODO: Add offsets here?
	raw: *[PAGE_SIZE]u8
};

export fn dump_node(f: io::handle, n: node, ident: str = "") void = {
	fmt::fprintfln(f, "{}{}-Node#{}: #keys={}, size={}", ident,
		if (n.typ == node_type::INNER) "Inner" else "Leaf",
		n.page_offset / PAGE_SIZE, n.num_keys,
		size_in_bytes(n))!;
	if (n.typ == node_type::INNER) {
		for (let i: size = 0; i < n.num_keys; i += 1) {
			fmt::fprintfln(f, "{} - off={}", ident, get_off(n, i))!;
			fmt::fprintfln(f, "{} - key='{}'", ident,
				strings::fromutf8(get_key(n, i))!)!;
		};
		fmt::fprintfln(f, "{} - off={}", ident, get_off(n, n.num_keys))!;
	} else {
		for (let i: size = 0; i < n.num_keys; i += 1) {
			fmt::fprintfln(f, "{} - key='{}' -> val='{}'", ident,
				strings::fromutf8(get_key(n, i))!,
				strings::fromutf8(get_val(n, i))!)!;
		};
	};
	fmt::fprintln(f, "")!;
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

fn get_key(n: node, i: size) []u8 = {
	assert(i < n.num_keys);
	let s = ((n.raw[4 + i*2]: size) << 8) | (n.raw[4 + i*2 + 1]: size);
	if (n.typ == node_type::LEAF) {
		let keylen = ((n.raw[s]: size) << 8) | (n.raw[s + 1]: size);
		return n.raw[(s + 4)..(s + 4 + keylen)];
	} else {
		i += 1;
		let e = ((n.raw[4 + i*2]: size) << 8) | (n.raw[4 + i*2 + 1]: size);
		return n.raw[s..e];
	};
};

fn get_offs(n: node, dst: []u32) []u32 = {
	assert(n.typ == node_type::INNER);
	let dst = dst[0..0];
	let s = 4 + 2*(n.num_keys + 1);
	for (let i: size = 0; i < n.num_keys + 1; i += 1) {
		let off = ((n.raw[s+0]: u32) << 24)
			| ((n.raw[s+1]: u32) << 16)
			| ((n.raw[s+2]: u32) << 8)
			| ((n.raw[s+3]: u32) << 0);
		append(dst, off);
		s += 4;
	};
	return dst;
};

fn get_off(n: node, i: size) u32 = {
	assert(n.typ == node_type::INNER && i <= n.num_keys);
	let s = 4 + 2*(n.num_keys + 1) + (i * 4);
	let off = ((n.raw[s+0]: u32) << 24)
		| ((n.raw[s+1]: u32) << 16)
		| ((n.raw[s+2]: u32) << 8)
		| ((n.raw[s+3]: u32) << 0);
	return off;
};

// TODO: This needs to be atomically and fsynced?
fn set_off(n: node, i: size, off: u32) void = {
	assert(n.typ == node_type::INNER && i <= n.num_keys);
	let s = 4 + 2*(n.num_keys + 1) + (i * 4);
	n.raw[s+0] = (off >> 24): u8;
	n.raw[s+1] = (off >> 16): u8;
	n.raw[s+2] = (off >> 8): u8;
	n.raw[s+3] = (off >> 0): u8;
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
fn lookup_in_node(n: node, search_key: []u8) (bool, size) = {
	for (let i: size = 0; i < n.num_keys; i += 1) {
		let cmp = compare_bytes(search_key, get_key(n, i));
		if (cmp == 0)
			return (true, i);
		if (cmp <= 0)
			return (cmp == 0, i);
	};
	return (false, n.num_keys);
};

type full = !void;

fn build_leaf_node(keys: [][]u8, vals: [][]u8, page_offset: size, raw: *[PAGE_SIZE]u8) (node, size) = {
	let buf: []u8 = raw[0..0];
	assert(page_offset % PAGE_SIZE == 0, "page_offset needs to be page-aligned");
	let page_offset = page_offset / PAGE_SIZE;
	static append(buf, ((page_offset >> 8) & 0x7f): u8);
	static append(buf, (page_offset & 0xff): u8);
	static append(buf, (len(keys) >> 8): u8);
	static append(buf, (len(keys) & 0xff): u8);
	let keyvals_off = 4 + 2*(len(keys) + 1);
	for (let i: size = 0; i < len(keys); i += 1) {
		if (i > 0) assert(compare_bytes(keys[i-1], keys[i]) < 0);
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

	return (node {
		typ = node_type::LEAF,
		page_offset = page_offset * PAGE_SIZE,
		num_keys = len(keys),
		raw = raw
	}, len(buf));
};

fn build_inner_node(keys: [][]u8, positions: []u32, page_offset: size, raw: *[PAGE_SIZE]u8) (node, size) = {
	assert(len(keys) + 1 == len(positions));
	let buf: []u8 = raw[0..0];
	assert(page_offset % PAGE_SIZE == 0, "page_offset needs to be page-aligned");
	let page_offset = page_offset / PAGE_SIZE;
	static append(buf, 0x80 | ((page_offset >> 8) & 0x7f): u8);
	static append(buf, (page_offset & 0xff): u8);
	static append(buf, (len(keys) >> 8): u8);
	static append(buf, (len(keys) & 0xff): u8);
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

	return (node {
		typ = node_type::INNER,
		page_offset = page_offset * PAGE_SIZE,
		num_keys = len(keys),
		raw = raw
	}, len(buf));
};

fn split_leaf_node(orig: node, insert_key: []u8, insert_val: []u8,
		a: (size, *[PAGE_SIZE]u8), b: (size, *[PAGE_SIZE]u8)) (node, node, size, []u8) = {
	assert(orig.num_keys >= 2);
	let keys: [][]u8 = get_keys(orig, []);
	defer free(keys);
	let vals: [][]u8 = get_vals(orig, []);
	defer free(vals);

	let (key_exists, insert_idx) = lookup_in_node(orig, insert_key);
	if (!key_exists) {
		insert(keys[insert_idx], insert_key);
		insert(vals[insert_idx], insert_val);
	} else {
		vals[insert_idx] = insert_val;
	};
	let splitidx = len(keys) / 2;
	let splitkey = keys[splitidx];
	let a = build_leaf_node(keys[..(splitidx+1)], vals[..(splitidx+1)], a.0, a.1).0;
	let b = build_leaf_node(keys[(splitidx+1)..], vals[(splitidx+1)..], b.0, b.1).0;
	return (a, b, splitidx, splitkey);
};

// When splitting a inner node, the key at splitidx moves to the parent?
// - All nodes on the LHS of splitidx and the inserted value go to the a node.
// - The key at splitidx moves up, with a to it's left (replacing the pointer to
//   orig) and b to it's right.
// - The key on the RHS of splitidx go to the b node.
fn split_inner_node(orig: node, insert_key: []u8, insert_pos: u32,
		a: (size, *[PAGE_SIZE]u8), b: (size, *[PAGE_SIZE]u8)) (node, node, size, []u8) = {
	assert(orig.num_keys > 1);
	let keys: [][]u8 = get_keys(orig, []);
	defer free(keys);
	let poses: []u32 = get_offs(orig, []);
	defer free(poses);

	let (key_exists, insert_idx) = lookup_in_node(orig, insert_key);
	if (!key_exists) {
		insert(keys[insert_idx], insert_key);
		insert(poses[insert_idx+1], insert_pos);
	} else {
		poses[insert_idx+1] = insert_pos;
	};
	let splitidx = len(keys) / 2;
	let splitkey = keys[splitidx];
	let a = build_inner_node(keys[..(splitidx+1)], poses[..(splitidx+2)], a.0, a.1).0;
	// The splitkey is not in a or b but instead returned to the parent to
	// be inserted there.
	let b = build_inner_node(keys[(splitidx+2)..], poses[(splitidx+2)..], b.0, b.1).0;
	return (a, b, splitidx, splitkey);
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
	let (test, used) = build_inner_node(
		keys, positions,
		42 * PAGE_SIZE, &raw);

	// The actual test:
	let n = get_node(test.raw);
	assert(size_in_bytes(n) == used);
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
		assert(strings::fromutf8(keys[i])! == strings::fromutf8(get_key(n, i))!);
	};

	let test_positions = get_offs(n, []);
	defer free(test_positions);
	assert(len(test_positions) == len(positions));
	for (let i: size = 0; i < len(positions); i += 1) {
		assert(positions[i] == test_positions[i]);
		assert(positions[i] == get_off(n, i));
	};

	assert(lookup_in_node(n, strings::toutf8("aaa")).1 == 0);
	for (let i: size = 0; i < len(keys); i += 1) {
		assert(lookup_in_node(n, keys[i]).1 == i);
	};
	assert(lookup_in_node(n, strings::toutf8("zzz")).1 == n.num_keys);
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
	let (test, used) = build_leaf_node(keys, vals, 42 * PAGE_SIZE, &raw);

	// The actual test:
	let n = get_node(test.raw);
	assert(size_in_bytes(n) == used);
	assert(n.typ == node_type::LEAF);
	assert(n.page_offset == 42 * PAGE_SIZE);
	assert(n.num_keys == len(keys));
	let test_keys = get_keys(n, []);
	defer free(test_keys);
	assert(len(test_keys) == len(keys));
	for (let i: size = 0; i < len(keys); i += 1) {
		assert(strings::fromutf8(keys[i])! == strings::fromutf8(test_keys[i])!);
		assert(strings::fromutf8(keys[i])! == strings::fromutf8(get_key(n, i))!);
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
		assert(res.0 == true && res.1 == i);
	};
};

@test fn test_leaf_split() void = {
	// The test data:
	let keys: [][]u8 = [];
	append(keys, strings::toutf8("aaa"));
	append(keys, strings::toutf8("bbb"));
	append(keys, strings::toutf8("ddd"));
	append(keys, strings::toutf8("eee"));
	let vals: [][]u8 = [];
	append(vals, strings::toutf8("AAA"));
	append(vals, strings::toutf8("BBB"));
	append(vals, strings::toutf8("DDD"));
	append(vals, strings::toutf8("EEE"));
	assert(len(keys) == len(vals));

	// Setup of the page:
	let raw: [PAGE_SIZE]u8 = [0...];
	let orig = build_leaf_node(keys, vals, 42 * PAGE_SIZE, &raw).0;

	let a: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(a);
	let b: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(b);

	let (a, b, split_idx, split_key) = split_leaf_node(orig,
		strings::toutf8("ccc"), strings::toutf8("CCC"),
		(123 * PAGE_SIZE, a), (456 * PAGE_SIZE, b));
	assert(a.page_offset == 123 * PAGE_SIZE);
	assert(b.page_offset == 456 * PAGE_SIZE);
	assert(a.num_keys == 3);
	assert(b.num_keys == 2);
	assert(strings::fromutf8(split_key)! == "ccc" && split_idx == 2);
	assert(compare_bytes(get_key(a, 0), strings::toutf8("aaa")) == 0);
	assert(compare_bytes(get_val(a, 0), strings::toutf8("AAA")) == 0);
	assert(compare_bytes(get_key(a, 1), strings::toutf8("bbb")) == 0);
	assert(compare_bytes(get_val(a, 1), strings::toutf8("BBB")) == 0);
	assert(compare_bytes(get_key(a, 2), strings::toutf8("ccc")) == 0);
	assert(compare_bytes(get_val(a, 2), strings::toutf8("CCC")) == 0);
	assert(compare_bytes(get_key(b, 0), strings::toutf8("ddd")) == 0);
	assert(compare_bytes(get_val(b, 0), strings::toutf8("DDD")) == 0);
	assert(compare_bytes(get_key(b, 1), strings::toutf8("eee")) == 0);
	assert(compare_bytes(get_val(b, 1), strings::toutf8("EEE")) == 0);
};

@test fn test_inner_split() void = {
	// The test data:
	let keys: [][]u8 = [];
	append(keys, strings::toutf8("aaa"));
	append(keys, strings::toutf8("bbb"));
	append(keys, strings::toutf8("ddd"));
	append(keys, strings::toutf8("eee"));
	let positions: []u32 = [];
	append(positions, 1);
	append(positions, 2);
	append(positions, 3);
	append(positions, 4);
	append(positions, 5);
	assert(len(keys) + 1 == len(positions));

	// Setup of the page:
	let raw: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(raw);
	assert(len(keys) + 1 == len(positions));
	let orig = build_inner_node(
		keys, positions,
		42 * PAGE_SIZE, raw).0;

	let a: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(a);
	let b: *[PAGE_SIZE]u8 = alloc([0...]);
	defer free(b);

	let (a, b, splitidx, splitkey) = split_inner_node(orig,
		strings::toutf8("ccc"), 10,
		(123 * PAGE_SIZE, a), (456 * PAGE_SIZE, b));
	assert(a.page_offset == 123 * PAGE_SIZE);
	assert(b.page_offset == 456 * PAGE_SIZE);
	assert(a.num_keys == 3);
	assert(b.num_keys == 1);
	assert(strings::fromutf8(splitkey)! == "ccc");
	assert(compare_bytes(get_key(a, 0), strings::toutf8("aaa")) == 0);
	assert(compare_bytes(get_key(a, 1), strings::toutf8("bbb")) == 0);
	assert(compare_bytes(get_key(a, 2), strings::toutf8("ccc")) == 0);
	assert(compare_bytes(get_key(b, 0), strings::toutf8("eee")) == 0);
	assert(get_off(a, 0) == 1);
	assert(get_off(a, 1) == 2);
	assert(get_off(a, 2) == 3);
	assert(get_off(a, 3) == 10);
	assert(get_off(b, 0) == 4);
	assert(get_off(b, 1) == 5);
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

