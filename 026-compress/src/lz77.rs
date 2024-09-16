use std::simd::prelude::*;
use std::collections::VecDeque;

const WINDOW_SIZE: usize = 4096;
const MIN_MATCH_LEN: usize = 8;

#[derive(Debug)]
enum LZ77Entry<'a> {
    Raw { slice: &'a [u8] },
    Lookback { by: u16, len: u16 }
}

fn compress<'a>(input: &'a [u8]) -> Vec<LZ77Entry<'a>> {
    let mut pos: usize = MIN_MATCH_LEN;
    let mut output: Vec<LZ77Entry> = Vec::new();
    let mut raw: (usize, usize) = (0, std::cmp::min(MIN_MATCH_LEN, input.len()));

    while pos + MIN_MATCH_LEN < input.len() {
        let window_start = pos - std::cmp::min(pos, WINDOW_SIZE);

        let mut longest_match_len: usize = MIN_MATCH_LEN;
        let mut longest_match: Option<(usize, usize)> = None;

        for i in window_start..(pos - MIN_MATCH_LEN) {
            if input[i] != input[pos] || input[i + 1] != input[pos + 1] {
                continue
            }

            let max_match_len = std::cmp::min(input.len() - pos, pos - i);
            let mut match_len: usize = 2;
            while match_len < max_match_len && input[i + match_len] == input[pos + match_len] {
                match_len += 1;
            }

            if match_len >= longest_match_len {
                longest_match_len = match_len;
                longest_match = Some((i, match_len));
            }
        }

        if let Some(m) = longest_match {
            if raw.1 - raw.0 > 0 {
                output.push(LZ77Entry::Raw { slice: &input[raw.0..raw.1] });
            }

            output.push(LZ77Entry::Lookback { by: (pos - m.0) as u16, len: m.1 as u16 });
            pos += m.1;
            raw = (pos, pos);
        } else {
            raw.1 += 1;
            pos += 1;
        }
    }

    output.push(LZ77Entry::Raw { slice: &input[raw.0..input.len()] });
    output
}

fn decompress<'a>(input: Vec<LZ77Entry<'a>>) -> Vec<u8> {
    let mut output = Vec::new();
    for e in input {
        if let LZ77Entry::Raw { slice } = e {
            output.reserve(slice.len());
            for c in slice {
                output.push(*c);
            }
            continue;
        }

        if let LZ77Entry::Lookback { by, len } = e {
            let start = output.len() - by as usize;
            let slice: &'static [u8] = unsafe {
                std::mem::transmute(&output[start..(start + len as usize)])
            };
            output.reserve(slice.len());
            for c in slice {
                output.push(*c);
            }
            continue;
        }

        unimplemented!()
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_test() {
        let input = "123456789 123456789 123456789 123456789";
        let compressed = compress(input.as_bytes());
        let output = String::from_utf8(decompress(compressed)).unwrap();

        println!("input:  {:?},\noutput: {:?}", input, output);
    }
}

