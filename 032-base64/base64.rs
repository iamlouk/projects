#![feature(slice_as_chunks)]

use std::io::Read;
use std::io::Write;

const BUF_SIZE: usize = 4096;

const BASE64_CHARS: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/'
];

fn base64_encode(buf: &[u8], out: &mut impl std::io::Write) -> std::io::Result<()> {
    assert!(buf.len() % 3 == 0);
    let mut outbuf = Vec::with_capacity(BUF_SIZE * 2);
    for [b1, b2, b3] in buf.as_chunks::<3>().0 {
        outbuf.push(BASE64_CHARS[((b1 >> 2) & 0x3f) as usize]);
        outbuf.push(BASE64_CHARS[(((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)) as usize]);
        outbuf.push(BASE64_CHARS[(((b2 << 2) & 0x3c) | ((b3 >> 6) & 0x03)) as usize]);
        outbuf.push(BASE64_CHARS[(b3 & 0x3f) as usize]);
    }

    out.write_all(&outbuf)
}

fn base64_encode_finish(buf: &[u8], out: &mut impl std::io::Write) -> std::io::Result<()> {
    assert!(buf.len() < 3 && buf.len() != 0);
    let mut outbuf = Vec::with_capacity(BUF_SIZE * 2);

    if buf.len() == 1 {
        let b1: u8 = buf[0];
        outbuf.push(BASE64_CHARS[((b1 >> 2) & 0x3f) as usize]);
        outbuf.push(BASE64_CHARS[((b1 << 4) & 0x30) as usize]);
        outbuf.push(b'=');
        outbuf.push(b'=');
    } else if buf.len() == 2 {
        let b1: u8 = buf[0];
        let b2: u8 = buf[1];
        outbuf.push(BASE64_CHARS[((b1 >> 2) & 0x3f) as usize]);
        outbuf.push(BASE64_CHARS[(((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)) as usize]);
        outbuf.push(BASE64_CHARS[((b2 << 2) & 0x3c) as usize]);
        outbuf.push(b'=');
    }

    out.write_all(&outbuf)
}

fn main() {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut rem: usize = 0;
    let mut buf: [u8; BUF_SIZE] = [0u8; BUF_SIZE];
    loop {
        let n = match stdin.read(&mut buf[rem..]) {
            Ok(0) => break,
            Ok(n) => n + rem,
            Err(e) => {
                eprintln!("I/O error: {}", e);
                std::process::exit(1);
            }
        };

        let chunks = n / 3;
        rem = n % 3;
        base64_encode(&buf[0..(chunks * 3)], &mut stdout).expect("I/O error");
        for i in 0..rem {
            buf[i] = buf[(chunks * 3) + i];
        }
    }
    if rem != 0 {
        base64_encode_finish(&buf[0..rem], &mut stdout).expect("I/O error");
    }
    stdout.flush().expect("I/O error");
}
