use fmt;
use net;
use net::tcp;
use net::ip;
use io;
use os;
use bufio;
use unix::poll;
use strings;
use encoding::utf8;

// My very first Hare project: A simple TCP echo server.
// The websocket stuff will follow later (if it ever does).

def MAX_CLIENTS: size = 31;

export fn main() void = {
	fmt::println("Hello, World!")!;
	const listener = match (tcp::listen(ip::LOCAL_V4, 8000)) {
	case let socket: net::socket => yield socket;
	case let err: net::error =>
		fmt::fatalf("Cannot listen on 8000: {}", net::strerror(err));
	};
	let pollfds: [MAX_CLIENTS + 1]poll::pollfd = [
		// pollfds[0] is the listener socket.
		poll::pollfd {
			fd = listener,
			events = (poll::event::POLLIN | poll::event::POLLPRI),
			revents = 0
		}
		...
	];
	let free_slots: []size = [];
	defer free(free_slots);
	for (let i: size = 1; i < MAX_CLIENTS + 1; i += 1) {
		// All other FDs are for client connections.
		pollfds[i] = poll::pollfd { fd = 0, events = 0, revents = 0 };
		append(free_slots, i);
	};

	for (true) {
		poll::poll(pollfds, poll::INDEF)!;
		if (pollfds[0].revents == poll::event::POLLIN) {
			// Handle a new connection.
			let conn = net::accept(listener)!;
			if (len(free_slots) == 0) {
				fmt::fatal("Max. clients reached!");
			};

			// Search a new slot/free fd.
			let slot = free_slots[len(free_slots) - 1];
			delete(free_slots[len(free_slots) - 1]);
			assert(1 <= slot && slot <= MAX_CLIENTS);
			fmt::printfln("New connection: slot={}!", slot)!;
			pollfds[slot] = poll::pollfd {
				fd = conn,
				events = (poll::event::POLLIN | poll::event::POLLPRI |
					poll::event::POLLOUT | poll::event::POLLERR |
					poll::event::POLLHUP),
				revents = 0
			};
		};

		for (let i: size = 1; i < MAX_CLIENTS + 1; i += 1) {
			// Skip over slots without connections.
			if (pollfds[i].events == 0) continue;

			// Skip if nothing happend for this FD.
			if (pollfds[i].revents == 0) continue;

			let peer: (ip::addr, u16) = tcp::peeraddr(pollfds[i].fd) as (ip::addr, u16);

			let buf: [os::BUFSZ]u8 = [0...];
			let response = match (io::read(pollfds[i].fd, buf)) {
			case io::EOF =>
				fmt::printfln("Disconnect: {}:{} (slot={})",
					ip::string(peer.0), peer.1, i)!;
				io::close(pollfds[i].fd)!;
				pollfds[i] = poll::pollfd { fd = 0, events = 0, revents = 0 };
				append(free_slots, i);
				continue;
			case let err: io::error =>
				fmt::printfln("Disconnect because of error: {}:{} (slot={}): {}",
					ip::string(peer.0), peer.1, i, io::strerror(err))!;
				io::close(pollfds[i].fd)!;
				pollfds[i] = poll::pollfd { fd = 0, events = 0, revents = 0 };
				append(free_slots, i);
				yield io::strerror(err);
			case let n: size =>
				let msg = match (strings::fromutf8(buf[0..n])) {
				case let msg: str => yield msg;
				case let err: encoding::utf8::invalid => yield "<invalid UTF8>";
				};
				fmt::printfln("Message by: {}:{} (slot={}): {}",
					ip::string(peer.0), peer.1, i, msg)!;
				yield msg;
			};

			// TODO: This could also be done using I/O multiplexing.
			match (io::writeall(pollfds[i].fd, strings::toutf8(response))) {
			case let err: io::error => {
				fmt::printfln("Disconnect because of error: {}:{} (slot={}): {}",
					ip::string(peer.0), peer.1, i, io::strerror(err))!;
				io::close(pollfds[i].fd)!;
				pollfds[i] = poll::pollfd { fd = 0, events = 0, revents = 0 };
				append(free_slots, i);
				yield;
			};
			case let n: size => yield;
			};
		};
	};
};
