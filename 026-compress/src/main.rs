#![feature(portable_simd)]
#![feature(iter_array_chunks)]

use std::io::{Read, Write};

mod huffman;
mod lz77;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let compress = match args.get(1).map(|x| x.as_str()) {
        Some("c") => true,
        Some("d") => false,
        None | Some(_) => {
            eprintln!("usage: {} [c|d]", args[0]);
            std::process::exit(1);
        }
    };

    let mut stdin: Vec<u8> = Vec::new();
    std::io::stdin().lock().read_to_end(&mut stdin).expect("I/O error");

    let stdout = match compress {
        true => {
            let mut buf1 = lz77::compress(&stdin);
            let (tree, n, buf2) = huffman::compress(&buf1);
            buf1.clear();
            tree.encode(&mut buf1);
            for c in n.to_ne_bytes() {
                buf1.push(c);
            }
            for w in buf2 {
                for c in w.to_ne_bytes() {
                    buf1.push(c);
                }
            }
            buf1
        },
        false => {
            let mut pos: usize = 0;
            let tree = huffman::Node::decode(&stdin, &mut pos);
            let mut raw: [u8; 8] = std::default::Default::default();
            raw.copy_from_slice(&stdin[pos..pos+8]);
            let n = u64::from_ne_bytes(raw);
            let stdin = &stdin[pos+8..];
            assert!(stdin.len() % 8 == 0);
            let mut buf: Vec<u64> = Vec::new();
            for bytes in stdin.iter().cloned().array_chunks::<8>() {
                buf.push(u64::from_ne_bytes(bytes));
            }

            let buf1 = huffman::decompress(&tree, n as usize, &buf);
            lz77::decompress(&buf1)
        }
    };
    std::io::stdout().lock().write_all(&stdout).expect("I/O error");
}
