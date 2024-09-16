use bytes;
use errors;
use fmt;
use io;
use strings;
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

export type page_cache = struct {
	// The last time this page was looked up and the page itself.
	// TODO: Cache eviction.
	htable: [32][](time::instant, node),
	entries: uint
};

// Lookup a key in a btree and get the result.
export fn lookup(bt: *btree, key: []u8) (void | io::error | []u8) = {
	let n = bt.root;
	for (true) {
		let res = lookup_in_node(n, key);
		if (n.typ == node_type::LEAF && res.0 == false)
			return void;
		if (n.typ == node_type::LEAF)
			return get_val(n, res.1);

		let next_node_pos = get_pos(n, res.1): size * PAGE_SIZE;
		n = get_page_from_cache_or_disk(bt, next_node_pos)?;
	};
};

fn insert_into_leaf(bt: *btree, n: node, key: []u8, val: []u8) (node | (node, node)) = {
	let (found, idx) = lookup_in_node(n, key);

	fmt::fatal("TODO");
};

fn get_page_from_cache_or_disk(bt: *btree, page_offset: size) (node | io::error) = {
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
fn allocate_page_from_disk(bt: *btree) ((size, *[PAGE_SIZE]u8) | io::error) = {
	let page_offset: size = if (len(bt.free_page_list) > 0) {
		let res = bt.free_page_list[0];
		// TODO: Is the & needed here? I like hare in general, but I
		// prefere the go-style append builtin for slices.
		delete(bt.free_page_list[0]);
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
	return (x / PAGE_SIZE);
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
			return true;
		};
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

