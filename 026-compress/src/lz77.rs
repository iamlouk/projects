const ESCAPE_CHAR: u8 = 0xff;
const WINDOW_SIZE: usize = 2048;
const MIN_MATCH_LEN: usize = 8;

pub fn compress<'a>(input: &'a [u8]) -> Vec<u8> {
    let mut pos: usize = MIN_MATCH_LEN;
    let mut output: Vec<u8> = Vec::with_capacity(input.len());
    let mut raw: (usize, usize) = (0, std::cmp::min(MIN_MATCH_LEN, input.len()));

    let encode = |buf: &mut Vec<u8>, slice: &[u8]| {
        for c in slice {
            if *c == ESCAPE_CHAR {
                buf.push(ESCAPE_CHAR);
                buf.push(ESCAPE_CHAR);
                continue;
            }
            buf.push(*c);
        }
    };

    let encode_lookback = |buf: &mut Vec<u8>, by: usize, len: usize| {
        buf.push(ESCAPE_CHAR);
        let by = u16::try_from(by).unwrap();
        let len = u8::try_from(len).unwrap();
        assert!(len != 0xff);
        buf.push(len);
        buf.push((by & 0xff) as u8);
        buf.push(((by >> 8) & 0xff) as u8);
    };

    /* Iterate over the input. */
    while pos + MIN_MATCH_LEN < input.len() {
        let window_start = pos - std::cmp::min(pos, WINDOW_SIZE);

        let mut longest_match_len: usize = MIN_MATCH_LEN;
        let mut longest_match: Option<(usize, usize)> = None;

        /*
         * TODO: Instead of just finding matches by traversing all of the window,
         * how about tracking stuff that has repeated before, and trying these
         * existing matches first, only creating a new pattern if no existing one
         * matched? I am not sure but I think this is how deflate/gzip works.
         * The patterns (for which it was tracked how often they appeared) could
         * then be huffman-encoded?
         */
        /* Iterate over all characters in the WINDOW_SIZE last characters of the
         * input, and find the longest sub-sequence (and it's length) in the window
         * that contains identical characters to the sequence starting at pos. */
        for i in window_start..(pos - MIN_MATCH_LEN) {
            if input[i] != input[pos] || input[i + 1] != input[pos + 1] {
                continue;
            }

            let max_match_len = (input.len() - pos).min(pos - i).min(0xff - 1);
            let mut match_len: usize = 2;
            while match_len < max_match_len && input[i + match_len] == input[pos + match_len] {
                match_len += 1;
            }

            if match_len >= longest_match_len {
                longest_match_len = match_len;
                longest_match = Some((i, match_len));
            }
        }

        /* If a match was found, encode it. */
        if let Some(m) = longest_match {
            if raw.1 - raw.0 > 0 {
                encode(&mut output, &input[raw.0..raw.1]);
            }

            encode_lookback(&mut output, pos - m.0, m.1);
            pos += m.1;
            raw = (pos, pos);
        } else {
            raw.1 += 1;
            pos += 1;
        }
    }

    encode(&mut output, &input[raw.0..input.len()]);
    output
}

pub fn decompress<'a>(input: &'a [u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut pos: usize = 0;
    while pos < input.len() {
        /* Copy over uncompressed parts: */
        if input[pos] != ESCAPE_CHAR {
            output.push(input[pos]);
            pos += 1;
            continue;
        }

        if input[pos + 1] == ESCAPE_CHAR {
            output.push(ESCAPE_CHAR);
            pos += 1;
            continue;
        }

        /* Decode the information on how far we need to go back. */
        let lookback_len: usize = input[pos + 1] as usize;
        let lookback_by: usize =
            ((input[pos + 2] as usize) & 0xff) | (((input[pos + 3] as usize) & 0xff) << 8);
        let start = output.len() - lookback_by;
        // If the loop below would need to re-allocate, then the transmuted
        // memory reference would become invalid!
        output.reserve(lookback_len + 1);
        let slice: &'static [u8] = unsafe {
            // This is UB in rust, but it would be stupid to copy the slice to
            // a temporary buffer before appending it's contents.
            std::mem::transmute(&output[start..(start + lookback_len as usize)])
        };

        for c in slice {
            output.push(*c);
        }
        pos += 4;
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
        let raw_output = decompress(&compressed);
        eprintln!(
            "input:  {:x?},\noutput: {:x?}",
            input.as_bytes(),
            raw_output
        );
        let output = String::from_utf8(raw_output).unwrap();

        assert!(compressed.len() < input.len());
        assert_eq!(input, output.as_str());
    }

    #[test]
    fn lorem_impsum() {
        let input = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur sit amet justo nibh. Donec in diam et mi porta rhoncus eu non sem. Mauris vehicula laoreet libero at bibendum. Nam in efficitur sem, id iaculis felis. Vivamus dapibus vestibulum ultricies. Nulla justo massa, volutpat sed bibendum vel, posuere in lacus. Duis commodo lectus a eros feugiat, nec faucibus quam efficitur. Etiam blandit sapien a posuere porttitor. Curabitur fermentum, nisl nec ultrices volutpat, lectus enim cursus orci, sed finibus nisi erat hendrerit lorem. Ut eget diam eget sem egestas faucibus quis et lectus. Donec pharetra arcu cursus imperdiet aliquet. Sed nisi orci, scelerisque efficitur elit quis, sagittis rutrum velit. Duis et felis nunc. Nam urna ipsum, fermentum ut urna eu, hendrerit suscipit nisl.\nCurabitur volutpat metus ut vestibulum varius. Proin luctus metus arcu, non lobortis diam interdum at. Cras feugiat maximus lacus, a egestas urna blandit cursus. In molestie convallis massa, quis tincidunt dolor faucibus in. Praesent porttitor pulvinar turpis, luctus sagittis metus fringilla quis. Praesent venenatis tellus sit amet risus viverra, non maximus urna mattis. Aliquam ligula nunc, mollis at quam vel, vestibulum tincidunt nisl. Curabitur massa tortor, pharetra eu felis id, tincidunt sollicitudin nulla. Nullam sollicitudin iaculis tellus, eu venenatis elit scelerisque eu. Curabitur finibus diam et lectus ultricies, et pretium lacus pharetra. Vestibulum tincidunt arcu efficitur ipsum egestas, eget sollicitudin lectus efficitur. Praesent ultrices vel urna a semper.\nNam ultricies nisi neque, quis fringilla lectus tempus vel. Nam placerat ex lorem, ac molestie lorem aliquet id. Maecenas sed dui bibendum, elementum nunc non, congue lectus. Etiam semper gravida magna, a congue sapien sagittis vel. In dignissim neque quam, quis faucibus justo mattis a. Sed facilisis eleifend libero vitae fermentum. Ut porttitor neque non ipsum gravida ullamcorper. Aliquam odio nisl, posuere eu elit sit amet, auctor semper odio. Donec finibus ipsum ipsum, non sollicitudin lectus consequat bibendum.\nPellentesque consectetur semper orci mollis maximus. Integer egestas ut risus quis vulputate. Aliquam eu egestas nibh. Etiam id dapibus purus. Aenean non finibus tortor, in pharetra ex. Suspendisse ut justo tortor. Aliquam elementum nec massa vitae sollicitudin.\nEtiam libero velit, congue nec imperdiet sed, iaculis id sem. Vivamus euismod leo sem, sed faucibus nisl congue in. Quisque pulvinar ligula vitae urna vulputate, ac dictum nisl pretium. Mauris viverra justo quis rhoncus volutpat. Nullam et odio sed dui vehicula gravida. Etiam eu nibh eget felis egestas euismod. Pellentesque finibus, ligula id luctus sollicitudin, lorem justo condimentum nisi, vel euismod leo lectus gravida nunc. Suspendisse non lacus mauris. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae;";

        let compressed = compress(input.as_bytes());
        let raw_output = decompress(&compressed);
        eprintln!("size: {}, compressed: {}", input.len(), compressed.len());
        let output = String::from_utf8(raw_output).unwrap();

        assert!(compressed.len() < input.len());
        assert_eq!(input, output.as_str());
    }

    fn test_file(filepath: &str) -> (usize, usize) {
        let filepath = std::path::Path::new(filepath);
        let input = std::fs::read_to_string(filepath).unwrap();

        let compressed = compress(input.as_bytes());
        let raw_output = decompress(&compressed);
        let output = String::from_utf8(raw_output).unwrap();

        assert!(compressed.len() < input.len());
        assert_eq!(input, output.as_str());

        (input.as_bytes().len(), compressed.len())
    }

    #[test]
    fn test_sqlite3h() {
        // Just some random large-ish file:
        let (size, compressed_size) = test_file("/usr/include/sqlite3.h");
        println!("size:       {} kb", size / 1024);
        println!("compressed: {} kb", compressed_size / 1024);
    }
}
