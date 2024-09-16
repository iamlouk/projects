use bufio;
use os;
use io;
use fmt;
use strings;

def BUFSIZE: size = 4096;

const base64chars: str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(input: []u8, last: bool, dst: io::handle) ((size, size) | io::error) = {
    const chunks: size = len(input) / 3;
    const rem = len(input) - chunks * 3;
    const base64alphabet = strings::toutf8(base64chars);
    let outputbuf: [BUFSIZE + BUFSIZE / 2]u8 = [0...];
    let output: []u8 = outputbuf[0..0];
    for (let c = 0u; c < chunks; c += 1) {
        const b1 = input[c * 3 + 0],
              b2 = input[c * 3 + 1],
              b3 = input[c * 3 + 2];
        static append(output, base64alphabet[(b1 >> 2) & 0x3f]);
        static append(output, base64alphabet[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)]);
        static append(output, base64alphabet[((b2 << 2) & 0x3c) | ((b3 >> 6) & 0x03)]);
        static append(output, base64alphabet[b3 & 0x3f]);
    };

    if (!last || rem == 0) {
        io::writeall(dst, output)?;
        return (chunks * 3, rem);
    };

    const b1 = input[chunks * 3 + 0];
    static append(output, base64alphabet[(b1 >> 2) & 0x3f]);
    if (rem == 1) {
        static append(output, base64alphabet[(b1 << 4) & 0x30]);
        static append(output, '=');
        static append(output, '=');
    } else if (rem == 2) {
        const b2 = input[chunks * 3 + 1];
        static append(output, base64alphabet[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)]);
        static append(output, base64alphabet[(b2 << 2) & 0x3c]);
        static append(output, '=');
    };

    io::writeall(dst, output)?;
    return (len(input), 0);
};

export fn main() void = {
    let buf: [BUFSIZE]u8 = [0...];
    let n: size = 1;
    let rem: size = 0;
    for (n != 0) {
        n = match (io::read(os::stdin, buf[rem..len(buf)])) {
        case let n: size => yield n;
        case io::EOF => yield 0;
        case let err: io::error =>
            fmt::fatalf("Error reading from stdin: {}", io::strerror(err));
        };

        rem = match (encode(buf[0..(rem+n)], n == 0, os::stdout)) {
        case let tup: (size, size) =>
            for (let i = 0u; i < tup.1; i += 1) {
                buf[i] = buf[tup.0 + i];
            };
            yield tup.1;
        case let err: io::error =>
            fmt::fatalf("Error writing to stdout: {}", io::strerror(err));
        };
    };

    return;
};
