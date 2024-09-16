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
	client_addr: ip::addr,
	client_port: u16,
	bufstream: bufio::stream,
	path: str,
	headers: []([]u8, str, str),
	ws_key: []u8,
	rbuf: []u8,
	wbuf: []u8,
	timestamp_last_frame: time::instant,
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

// Yes, this leaks memory on certain error paths.
// And also, it will to blocking reads.
export fn handshake(raw: net::socket,
		on_message: *fn(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str))
		(*wsconn | io::error | net::error | utf8::invalid | io::EOF | errors::invalid) = {
	let rbuf: []u8 = alloc([0...], os::BUFSZ);
	let wbuf: []u8 = alloc([0...], os::BUFSZ);
	let peer: (ip::addr, u16) = tcp::peeraddr(raw) as (ip::addr, u16);
	let wsc: *wsconn = alloc(wsconn {
		fd = raw,
		client_addr = peer.0,
		client_port = peer.1,
		bufstream = bufio::init(raw, rbuf, wbuf),
		path = "",
		headers = alloc([([], "", "")...], 5),
		ws_key = [],
		rbuf = rbuf,
		wbuf = wbuf,
		timestamp_last_frame = time::now(time::clock::MONOTONIC),
		on_message = on_message,
	});

	let reqline: str = match (bufio::read_line(&wsc.bufstream)) {
	case let bytes: []u8 => yield match (strings::fromutf8(bytes)) {
		case let line: str => yield line;
		case utf8::invalid => { finish(wsc); return utf8::invalid; };
	};
	case let err: (io::error | io::EOF) => { finish(wsc); return err; };
	};
	defer free(reqline);

	let cut1: (str, str) = strings::cut(reqline, " ");
	let cut2: (str, str) = strings::cut(cut1.1, " ");
	if (cut1.0 != "GET" && cut2.1 != "HTTP/1.1\r") {
		finish(wsc);
		return errors::invalid;
	};
	wsc.path = strings::dup(cut2.0);
	fmt::printfln("HTTP Request: proto='{}', path='{}', method={}",
		strings::trim(cut2.1), wsc.path, cut1.0)!;
	let connection_upgrade = false;
	let upgrade_websocket = false;
	let ws_key_str: str = "";
	for (let i = 0; true; i += 1) {
		let line: []u8 = match (bufio::read_line(&wsc.bufstream)?) {
		case let line: []u8 => yield line;
		case io::EOF => break;
		};
		if (len(line) == 1 && line[0] == '\r') {
			free(line);
			break;
		};

		let cut = bytes::cut(line, [':']);
		let key = strings::trim(lowercase(strings::fromutf8(cut.0)!), ' ');
		let val = strings::trim(strings::fromutf8(cut.1)!, ' ', '\r');
		append(wsc.headers, (line, key, val));
		fmt::printfln("- Header #{}: key='{}', val='{}'", i, key, val)!;

		switch (key) {
		case "upgrade" => { upgrade_websocket = lowercase(val) == "websocket"; };
		case "connection" => { connection_upgrade = lowercase(val) == "upgrade"; };
		case "sec-websocket-version" => {
			if (val != "13") return errors::invalid;
		};
		case "sec-websocket-key" => {
			ws_key_str = val;
			wsc.ws_key = base64::decodestr(&base64::std_encoding, val)?;
		};
		case => yield;
		};
	};
	if (!connection_upgrade || !upgrade_websocket || len(wsc.ws_key) == 0)
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
	return wsc;
};

// This function will block sometimes. It might be better to split it, so that
// the poll loop is used for everything that could block.
// Also, it could panic/abort where it should not.
fn recieve_frame(wsc: *wsconn, pollfd: *poll::pollfd)
		(void | io::error | io::EOF) = {
	assert(wsc.fd: i32 == pollfd.fd: i32);
	let b0 = bufio::read_byte(&wsc.bufstream)? as u8;
	let b1 = bufio::read_byte(&wsc.bufstream)? as u8;
	let frame: wsframe = wsframe {
		fin = b0 & 0x80 != 0,
		masked = b1 & 0x80 != 0,
		opcode = b0 & 0x7f,
		masking_key = [0...],
		payload = []
	};
	if (3 < frame.opcode && frame.opcode < 8 || frame.opcode > 10)
		return errors::invalid;
	let payload_len: size = b1 & 0x7f;
	if (payload_len == 126) {
		let b2 = bufio::read_byte(&wsc.bufstream)?;
		let b3 = bufio::read_byte(&wsc.bufstream)?;
		payload_len = ((b2 as u8): size) << 8 | ((b3 as u8): size);
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

	frame.payload = alloc([0...], payload_len);
	defer free(frame.payload);
	match (io::readall(&wsc.bufstream, frame.payload)) {
	case io::EOF => return errors::invalid;
	case let err: io::error => return err;
	case let n: size => yield;
	};
	if (frame.masked)
		for (let i: size = 0; i < payload_len; i += 1)
			frame.payload[i] = frame.payload[i] ^ frame.masking_key[i % 4];

	if (!frame.fin)
		fmt::fatal("TODO: Non-fin WSC frame!");

	let res: (void | []u8 | str) = switch(frame.opcode) {
	case FRAME_CLOSE => {
		fmt::printfln("Close-Frame recieved by client: {}:{} (fd={})",
			ip::string(wsc.client_addr), wsc.client_port,
			wsc.fd: u32)!;
		send_frame(wsc, FRAME_CLOSE, [])?;
		return io::EOF;
	};
	case FRAME_PING => {
		fmt::printfln("Ping-Frame recieved by client: {}:{} (fd={})",
			ip::string(wsc.client_addr), wsc.client_port,
			wsc.fd: u32)!;
		return send_frame(wsc, FRAME_PONG, []);
	};
	case FRAME_TEXT => {
		fmt::printfln("Text-Frame recieved by client: {}:{} (fd={}, len={})",
			ip::string(wsc.client_addr), wsc.client_port,
			wsc.fd: u32, len(frame.payload))!;
		yield wsc.on_message(wsc, strings::fromutf8(frame.payload)!);
	};
	case FRAME_BINARY => {
		fmt::printfln("Binary-Frame recieved by client: {}:{} (fd={}, len={})",
			ip::string(wsc.client_addr), wsc.client_port,
			wsc.fd: u32, len(frame.payload))!;
		yield wsc.on_message(wsc, frame.payload);
	};
	case => return errors::invalid;
	};

	wsc.timestamp_last_frame = time::now(time::clock::MONOTONIC);
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

fn send_frame(wsc: *wsconn, opcode: u8, payload: const []u8) (void | io::error | io::EOF) = {
	fmt::printfln("Sending frame to {}:{} (fd={}, len={})",
		ip::string(wsc.client_addr), wsc.client_port,
		wsc.fd: u32, len(payload))!;
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

fn finish(wsc: *wsconn) void = {
	let ignore = bufio::flush(&wsc.bufstream);
	for (let i: size = 0; i < len(wsc.headers); i += 1) free(wsc.headers[i].0);
	free(wsc.path);
	free(wsc.headers);
	free(wsc.ws_key);
	free(wsc.rbuf);
	free(wsc.wbuf);
	let ignore = net::close(wsc.fd);
	free(wsc);
};

fn echo(wsc: *wsconn, msg: ([]u8 | str)) (void | []u8 | str) = {
	// For now, just echo:
	return match (msg) {
	case let text: str => {
		fmt::printfln("Echoing to {}:{} (fd={}, len={})",
			ip::string(wsc.client_addr), wsc.client_port,
			wsc.fd: u32, len(text))!;
		yield strings::dup(text);
	};
	case let bytes: []u8 => yield void;
	};
};

export fn main() void = {
	const host: (ip::addr4, u16) = (ip::LOCAL_V4, 12345);
	const listener = match (tcp::listen(host.0, host.1, tcp::reuseaddr)) {
	case let err: net::error => fmt::fatalf("Cannot listen on {}:{}: {}",
			ip::string(host.0), host.1, net::strerror(err));
	case let s: net::socket => yield s;
	};
	let pollfds: []poll::pollfd = alloc([
		// pollfds[0] is the listener socket.
		poll::pollfd {
			fd = listener,
			events = (poll::event::POLLIN | poll::event::POLLPRI),
			revents = 0,
		}
	], 16);
	let wscs: []nullable *wsconn = alloc([], 15);
	fmt::printfln("Server listening on {}:{}...", ip::string(host.0), host.1)!;
	for (true) {
		poll::poll(pollfds, poll::INDEF)!;
		if (pollfds[0].revents == poll::event::POLLIN) {
			// Handle a new connection.
			let fd: net::socket = net::accept(listener)!;
			let peer: (ip::addr, u16) = tcp::peeraddr(fd) as (ip::addr, u16);
			fmt::printfln("New connection from {}:{} (fd={})",
				ip::string(peer.0), peer.1, fd: u32)!;
			append(pollfds, poll::pollfd {
				fd = fd,
				events = (poll::event::POLLIN | poll::event::POLLPRI |
					poll::event::POLLOUT | poll::event::POLLERR |
					poll::event::POLLHUP),
				revents = 0,
			});
			append(wscs, null);
		};

		// Iter over open connections.
		for (let i: size = 1; i < len(pollfds); i += 1) {
			// Skip over slots without connections.
			if (pollfds[i].events == 0) continue;

			// Skip if nothing happend for this FD.
			if (pollfds[i].revents == 0) continue;

			let wsc: *wsconn = match (wscs[i - 1]) {
			case null => {
				let wsc = match (handshake(pollfds[i].fd, &echo)) {
				case let wsc: *wsconn => yield wsc;
				};
				fmt::printfln("Successfull handshake with {}:{} (fd={})",
					ip::string(wsc.client_addr), wsc.client_port,
					wsc.fd: u32)!;
				wscs[i - 1] = wsc;
				continue;
			};
			case let wsc: *wsconn => yield wsc;
			};

			match (recieve_frame(wsc, &pollfds[i])) {
			case void => yield;
			case let err: io::error => {
				fmt::printfln("Disconnecting client {}:{} (fd={}): error: {}",
					ip::string(wsc.client_addr), wsc.client_port,
					wsc.fd: u32, io::strerror(err))!;
				finish(wsc);
				delete(pollfds[i]);
				delete(wscs[i - 1]);
			};
			case io::EOF => {
				fmt::printfln("Disconnecting client {}:{} (fd={}): EOF",
					ip::string(wsc.client_addr), wsc.client_port,
					wsc.fd: u32)!;
				finish(wsc);
				delete(pollfds[i]);
				delete(wscs[i - 1]);
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
