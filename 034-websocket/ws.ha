use fmt;
use net;
use net::tcp;
use net::ip;
use bufio;
use os;
use io;
use strings;
use bytes;
use encoding::utf8;
use encoding::base64;
use errors;
use unix::poll;
use time;

export type wsconn = struct {
	fd: net::socket,
	client: (ip::addr, u16),
	bufstream: bufio::stream,
	rbuf: []u8,
	wbuf: []u8,
	path: str,
	headers: []([]u8, str, str),

	timestamp: time::instant,
	handshake_done: bool,
	waiting_for_pong: bool,
	timestamp_last_pong: time::instant,

	on_message: *fn(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str),
};

export type wsframe = struct {
	fin: bool,
	masked: bool,
	opcode: u8,
	masking_key: [4]u8,
	payload: []u8
};

def FRAME_CONTINUATION: u8 = 0;
def FRAME_TEXT: u8 = 1;
def FRAME_BINARY: u8 = 2;
def FRAME_CLOSE: u8 = 8;
def FRAME_PING: u8 = 9;
def FRAME_PONG: u8 = 10;

const TIMEOUT_PING: time::duration = 10 * time::MINUTE;
const TIMEOUT_PONG: time::duration = 10 * time::SECOND;

// There are potentially-blocking reads/writes in here!
// I am not sure what the ideal way of doing this would be, probably setting the
// non-blocking flag for the fd and then split this function into chunks?
export fn handshake(wsc: *wsconn) (void | io::error | utf8::invalid | errors::invalid) = {
	// Parse the HTTP request line.
	let reqline: str = match (bufio::read_line(&wsc.bufstream)) {
	case let bytes: []u8 => yield match (strings::fromutf8(bytes)) {
		case let line: str => yield line;
		case utf8::invalid => return errors::invalid;
	};
	case let err: io::error => return err;
	case io::EOF => return errors::invalid;
	};
	defer free(reqline);

	let cut1: (str, str) = strings::cut(reqline, " ");
	let cut2: (str, str) = strings::cut(cut1.1, " ");
	if (cut1.0 != "GET" && cut2.1 != "HTTP/1.1\r")
		return errors::invalid;
	wsc.path = strings::dup(cut2.0);
	fmt::printfln("HTTP Request: proto='{}', path='{}', method={}",
		strings::trim(cut2.1), wsc.path, cut1.0)!;
	let connection_upgrade = false;
	let upgrade_websocket = false;
	let ws_key_str: str = "";
	for (let i = 0; true; i += 1) {
		// HTTP headers.
		let line: []u8 = match (bufio::read_line(&wsc.bufstream)?) {
		case let line: []u8 => yield line;
		case io::EOF => break;
		};
		if (len(line) == 1 && line[0] == '\r') {
			free(line);
			break;
		};

		let cut = bytes::cut(line, [':']);
		let key = strings::trim(lowercase(strings::fromutf8(cut.0)?), ' ');
		let val = strings::trim(strings::fromutf8(cut.1)?, ' ', '\r');
		append(wsc.headers, (line, key, val));
		fmt::printfln("- Hdr #{}: '{}'='{}'", i, key, val)!;

		switch (key) {
		case "upgrade" =>
			upgrade_websocket = lowercase(val) == "websocket";
		case "connection" => {
			let parts = strings::split(lowercase(val), ", ");
			defer free(parts);
			for (let i: size = 0; i < len(parts); i += 1)
				if (parts[i] == "upgrade")
					connection_upgrade = true;
		};
		case "sec-websocket-version" =>
			if (val != "13")
				return errors::invalid;
		case "sec-websocket-key" =>
			ws_key_str = val;
		case => yield;
		};
	};
	if (!connection_upgrade || !upgrade_websocket || len(ws_key_str) == 0)
		return errors::invalid;

	let concatenated = strings::concat(ws_key_str, "258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
	defer free(concatenated);
	let sha1hash: [20]u8 = [0...];
	SHA1(&strings::toutf8(concatenated)[0], len(concatenated), &sha1hash[0]);
	let inbase64 = base64::encodestr(&base64::std_encoding, sha1hash);
	fmt::printfln("WebSocket Connection Upgrade: Sec-WebSocket-Accept='{}'", inbase64)!;
	defer free(inbase64);
	fmt::fprintf(&wsc.bufstream,
		"HTTP/1.1 101 Switching Protocols\r\n"
		"Upgrade: websocket\r\n"
		"Connection: Upgrade\r\n"
		"Sec-WebSocket-Version: 13\t\n"
		"Sec-WebSocket-Accept: {}\r\n\r\n",
		inbase64)?;
	bufio::flush(&wsc.bufstream)?;
	wsc.handshake_done = true;
	fmt::printfln("Successfull handshake with {}:{} (fd={})",
		ip::string(wsc.client.0), wsc.client.1,
		wsc.fd: u32)!;
	return void;
};

export fn init_connection(
		fd: net::socket,
		on_message: *fn(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str)) *wsconn = {
	let rbuf: []u8 = alloc([0...], os::BUFSZ);
	let wbuf: []u8 = alloc([0...], os::BUFSZ);
	let now = time::now(time::clock::MONOTONIC);
	let peer: (ip::addr, u16) = tcp::peeraddr(fd) as (ip::addr, u16);
	let wsc: *wsconn = alloc(wsconn {
		fd = fd,
		client = peer,
		bufstream = bufio::init(fd, rbuf, wbuf),
		rbuf = rbuf,
		wbuf = wbuf,
		path = "",
		headers = alloc([], 5),
		timestamp = now,
		timestamp_last_pong = now,
		handshake_done = false,
		waiting_for_pong = false,
		on_message = on_message,
	});
	return wsc;
};

// This function might block sometimes. It might be better to split it, so that
// the poll loop is used for everything that could block.
// Also, it could panic/abort where it should not.
fn recieve_frame(wsc: *wsconn, pollfd: *poll::pollfd)
		(void | io::error | io::EOF) = {
	assert(wsc.fd: i32 == pollfd.fd: i32);
	let buf: [2]u8 = [0, 0];
	match (io::readall(&wsc.bufstream, buf)) {
	case io::EOF => return io::EOF;
	case let err: io::error => return err;
	case let n: size => yield;
	};
	let frame: wsframe = wsframe {
		fin = buf[0] & 0x80 != 0,
		masked = buf[1] & 0x80 != 0,
		opcode = buf[0] & 0x7f,
		masking_key = [0...],
		payload = []
	};
	if (3 < frame.opcode && frame.opcode < 8 || frame.opcode > 10)
		return errors::invalid;
	let payload_len: size = buf[1] & 0x7f;
	if (payload_len == 126) {
		match (io::readall(&wsc.bufstream, buf)) {
		case io::EOF => return io::EOF;
		case let err: io::error => return err;
		case let n: size => yield;
		};
		payload_len = (buf[0]: size) << 8 | (buf[1]: size);
	} else if (payload_len == 127) {
		fmt::fatal("TODO...");
	};

	if (frame.masked) {
		match (io::readall(&wsc.bufstream, frame.masking_key)) {
		case io::EOF => return errors::invalid;
		case let err: io::error => return err;
		case let n: size => yield;
		};
	};

	defer free(frame.payload);
	if (payload_len != 0) {
		frame.payload = alloc([0...], payload_len);
		match (io::readall(&wsc.bufstream, frame.payload)) {
		case io::EOF => return errors::invalid;
		case let err: io::error => return err;
		case let n: size => yield;
		};
	};
	if (frame.masked)
		for (let i: size = 0; i < payload_len; i += 1)
			frame.payload[i] = frame.payload[i] ^ frame.masking_key[i % 4];

	if (!frame.fin)
		fmt::fatal("TODO: Non-fin frame!");

	wsc.timestamp = time::now(time::clock::MONOTONIC);
	let res = switch(frame.opcode) {
	case FRAME_CLOSE => {
		fmt::printfln("Close-Frame recieved by client: {}:{} (fd={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32)!;
		send_frame(wsc, FRAME_CLOSE, [])?;
		return io::EOF;
	};
	case FRAME_PING => {
		fmt::printfln("Ping-Frame recieved by client: {}:{} (fd={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32)!;
		return send_frame(wsc, FRAME_PONG, []);
	};
	case FRAME_PONG => {
		if (!wsc.waiting_for_pong)
			return errors::invalid;

		fmt::printfln("Pong-Frame recieved by client: {}:{} (fd={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32)!;
		wsc.timestamp_last_pong = wsc.timestamp;
		wsc.waiting_for_pong = false;
		return void;
	};
	case FRAME_TEXT => yield {
		let res = strings::fromutf8(frame.payload);
		let valid = res is str;
		fmt::printfln("Text-Frame recieved by client: {}:{} (fd={}, len={}, valid={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32, len(frame.payload), valid)!;
		yield if (valid) wsc.on_message(wsc, res as str) else void;
	};
	case FRAME_BINARY => yield {
		fmt::printfln("Binary-Frame recieved by client: {}:{} (fd={}, len={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32, len(frame.payload))!;
		yield wsc.on_message(wsc, frame.payload);
	};
	case => return errors::invalid;
	};

	match (res) {
	case void => yield;
	case let bytes: []u8 => {
		send_frame(wsc, FRAME_BINARY, bytes)?;
		free(bytes);
	};
	case let text: str => {
		let bytes = strings::toutf8(text);
		send_frame(wsc, FRAME_TEXT, bytes)?;
		free(text);
	};
	};

	return void;
};

// There are potentially-blocking writes in here!
fn send_frame(wsc: *wsconn, opcode: u8, payload: const []u8) (void | io::error | io::EOF) = {
	fmt::printfln("Sending frame to {}:{} (fd={}, opcode={}, len={})",
		ip::string(wsc.client.0), wsc.client.1,
		wsc.fd: u32, opcode, len(payload))!;
	let buf: [4]u8 = [0...];
	let buf: []u8 = buf[0..0];
	static append(buf, 0x80 | opcode);
	if (len(payload) < 126) {
		static append(buf, len(payload): u8);
	} else if (len(payload) < 65536) {
		static append(buf, 126: u8);
		static append(buf, ((len(payload) & 0xff00) >> 8): u8);
		static append(buf, (len(payload) & 0xff): u8);
	} else {
		return errors::invalid;
	};
	io::writeall(&wsc.bufstream, buf)?;
	io::writeall(&wsc.bufstream, payload)?;
	bufio::flush(&wsc.bufstream)?;
	return void;
};

// If there was no communication for TIMEOUT_PING, send a ping frame,
// and disconnect the client if it does not respond withing TIMEOUT_PONG.
fn check_alive(wsc: *wsconn, now: time::instant) bool = {
	let diff = time::diff(wsc.timestamp, now);
	if (diff < TIMEOUT_PING)
		return true;

	if (wsc.waiting_for_pong) {
		let diff = time::diff(wsc.timestamp_last_pong, now);
		return diff < TIMEOUT_PONG;
	};

	match (send_frame(wsc, FRAME_PING, [])) {
	case void => yield;
	case => return false;
	};
	wsc.waiting_for_pong = true;
	return true;
};

fn finish(wsc: *wsconn) void = {
	let ignore = bufio::flush(&wsc.bufstream);
	for (let i: size = 0; i < len(wsc.headers); i += 1) free(wsc.headers[i].0);
	free(wsc.path);
	free(wsc.headers);
	free(wsc.rbuf);
	free(wsc.wbuf);
	let ignore = net::close(wsc.fd);
	free(wsc);
};

// pollfds[0] is the listener socket.
let pollfds: []poll::pollfd = [];
let connections: []*wsconn = [];

fn finish_and_remove(i: size, wsc: *wsconn) void = {
	finish(wsc);
	delete(pollfds[i]);
	assert(connections[i - 1] == wsc);
	delete(connections[i - 1]);
};

// Currently unused, can be used to create a echo server.
fn echo(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str) = {
	// For now, just echo:
	return match (msg) {
	case let text: str => yield {
		fmt::printfln("Echoing to {}:{} (fd={}, len={})",
			ip::string(wsc.client.0), wsc.client.1,
			wsc.fd: u32, len(text))!;
		yield strings::dup(text);
	};
	case let bytes: []u8 => yield void;
	};
};

// broadcast a message on every connection.
fn broadcast(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str) = {
	if (!(msg is str)) return strings::dup("please send text, not binary data.");
	for (let i: size = 0; i < len(connections); i += 1) {
		let wsc = connections[i];
		if (!wsc.handshake_done)
			continue;

		match (send_frame(wsc, FRAME_TEXT, strings::toutf8(msg as str))) {
		case void => yield;
		case let err : io::error => {
			fmt::printfln("Failed to send frame to {}:{} (fd={}): {}",
				ip::string(wsc.client.0), wsc.client.1,
				wsc.fd: u32, io::strerror(err))!;
		};
		case io::EOF => {
			fmt::printfln("Failed to send frame to {}:{} (fd={}): EOF",
				ip::string(wsc.client.0), wsc.client.1, wsc.fd: u32)!;
		};
		};
	};
	return void;
};

export fn main() void = {
	const on_message = &broadcast;
	const host: (ip::addr4, u16) = (ip::LOCAL_V4, 12345);

	const listener = match (tcp::listen(host.0, host.1, tcp::reuseaddr)) {
	case let err: net::error => fmt::fatalf("Cannot listen on {}:{}: {}",
			ip::string(host.0), host.1, net::strerror(err));
	case let s: net::socket => yield s;
	};

	// pollfds[0] is the listener socket.
	append(pollfds, poll::pollfd {
		fd = listener,
		events = (poll::event::POLLIN | poll::event::POLLPRI),
		revents = 0,
	});

	fmt::printfln("Server listening on {}:{}...", ip::string(host.0), host.1)!;
	let any_waiting_on_pong: bool = false;
	for (true) {
		// Wait for events.
		poll::poll(pollfds, if (any_waiting_on_pong) TIMEOUT_PONG else TIMEOUT_PING)!;
		let now = time::now(time::clock::MONOTONIC);

		if (pollfds[0].revents == poll::event::POLLIN) {
			// Handle a new connection.
			let fd: net::socket = net::accept(listener)!;
			let peer: (ip::addr, u16) = tcp::peeraddr(fd) as (ip::addr, u16);
			fmt::printfln("New connection from {}:{} (fd={}, slot={}, total={})",
				ip::string(peer.0), peer.1, fd: u32, len(pollfds),
				len(connections) + 1)!;
			// Register the fd for the next poll call, and create a initially
			// empty slot in the wscs slice. After the first data was recieved,
			// a wsconn will be created later.
			append(pollfds, poll::pollfd {
				fd = fd,
				events = (poll::event::POLLIN | poll::event::POLLPRI),
				revents = 0,
			});
			append(connections, init_connection(fd, on_message));
		};

		// Iter over open connections.
		for (let i: size = 1; i < len(pollfds); i += 1) {
			// Skip over slots without connections.
			if (pollfds[i].events == 0) continue;

			// Skip if nothing happend for this FD and check for
			// timeouts.
			let wsc: *wsconn = connections[i - 1];
			if (pollfds[i].revents == 0) {
				if (!check_alive(wsc, now)) {
					fmt::printfln("Timeout of {}:{} (fd={}): closing connection",
						ip::string(wsc.client.0), wsc.client.1, wsc.fd: u32)!;
					finish_and_remove(i, wsc);
				};
				any_waiting_on_pong ||= wsc.waiting_for_pong;
				continue;
			};

			if (!wsc.handshake_done) {
				let strerr: str = match (handshake(wsc)) {
				case void => continue;
				case let e: io::error => yield io::strerror(e);
				case => yield "invalid request";
				};
				finish_and_remove(i, wsc);
				fmt::printfln("Handshake with {}:{} failed (fd={}): {}",
					ip::string(wsc.client.0), wsc.client.1,
					wsc.fd: u32, strerr)!;
				continue;
			};

			// Handle a frame.
			match (recieve_frame(wsc, &pollfds[i])) {
			case void => yield;
			case let err: io::error => {
				fmt::printfln("Disconnecting client {}:{} (fd={}): error: {}",
					ip::string(wsc.client.0), wsc.client.1,
					wsc.fd: u32, io::strerror(err))!;
				finish_and_remove(i, wsc);
			};
			case io::EOF => {
				fmt::printfln("Disconnecting client {}:{} (fd={}): EOF",
					ip::string(wsc.client.0), wsc.client.1,
					wsc.fd: u32)!;
				finish_and_remove(i, wsc);
			};
			};
		};
	};
};

// Hare stdlib has no strings::to_lowercase yet?
fn lowercase(s: str) str = {
	let bytes = strings::toutf8(s);
	for (let i: size = 0; i < len(bytes); i += 1) {
		if ('A' <= bytes[i] && bytes[i] <= 'Z')
			bytes[i] = bytes[i] + ('a': u8 - 'A': u8);
	};
	return s;
};

// Hare stdlib has no sha1 hash yet?
// This is from OpenSSLs libcrypto.so.
// Prototype from: #include <openssl/sha.h>
// 	unsigned char *SHA1(const unsigned char *data, size_t count, unsigned char *md_buf);
export @symbol("SHA1") fn SHA1(data: *u8, count: size, hash: *u8) *u8;
