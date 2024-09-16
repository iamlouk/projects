use bytes;
use errors;
use fmt;
use io;
use strings;
use time;

export type btree = struct {
	cache: page_cache,
	root: node,
	// TODO: memory-map the file?
	file: io::file,
	free_list: []size
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
			return res.1 as []u8;

		let next_node_pos = res.1 as size;
		n = get_page_from_cache_or_disk(bt, next_node_pos)?;
	};
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

fn write_page_to_disk(bt: *btree, n: node) (void | io::error) = {
	io::seek(bt.file, n.page_offset: io::off, io::whence::SET)?;
	match (io::writeall(bt.file, n.raw)) {
	case let n: size => assert(n == PAGE_SIZE);
	case let err: io::error => return err;
	};
};

fn hash_page_offset(x: size) size = {
	assert(x % PAGE_SIZE == 0);
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

