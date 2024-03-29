/* A Huffman encoding node: It has either a value or two children, never both. */
#[derive(Debug, PartialEq, Eq)]
pub struct Node {
    freq: usize,
    value: Option<u8>,
    children: [Option<Box<Node>>; 2],
}

impl Node {
    #[allow(unused)]
    fn dump(&self, prefix: &mut Vec<u8>) {
        if let Some(v) = self.value {
            assert!(self.children[0].is_none() && self.children[1].is_none());
            eprintln!("{:x?} ({:?}) -> {:?}", v, v as char, prefix);
            return;
        }

        prefix.push(0);
        self.children[0].as_ref().unwrap().dump(prefix);
        prefix.pop();
        prefix.push(1);
        self.children[1].as_ref().unwrap().dump(prefix);
        prefix.pop();
    }

    /* Returns a tuple of (encoding, number-of-bits). */
    fn get_encoding(&self, prefix: (usize, usize), encodings: &mut [(usize, usize); 256]) {
        if let Some(v) = self.value {
            assert!(self.children[0].is_none() && self.children[1].is_none());
            assert!(prefix.1 < std::mem::size_of_val(&prefix.0) * 8);
            assert!(encodings[v as usize] == (0, 0));
            encodings[v as usize] = prefix;
            return;
        }

        self.children[0]
            .as_ref()
            .unwrap()
            .get_encoding((prefix.0, prefix.1 + 1), encodings);
        self.children[1]
            .as_ref()
            .unwrap()
            .get_encoding((prefix.0 | (1 << prefix.1), prefix.1 + 1), encodings);
    }

    pub fn encode(&self, buf: &mut Vec<u8>) {
        if let Some(v) = self.value {
            buf.push(0x00);
            buf.push(v);
            return;
        }
        buf.push(0x01);
        self.children[0].as_ref().unwrap().encode(buf);
        self.children[1].as_ref().unwrap().encode(buf);
    }

    pub fn decode(buf: &[u8], pos: &mut usize) -> Box<Node> {
        let c = buf[*pos];
        assert!(c == 0x00 || c == 0x01);
        if c == 0x00 {
            let node = Box::new(Node {
                freq: 0,
                value: Some(buf[*pos + 1]),
                children: [None, None],
            });

            *pos += 2;
            return node;
        }

        *pos += 1;
        let lhs = Self::decode(buf, pos);
        let rhs = Self::decode(buf, pos);
        Box::new(Node {
            freq: 0,
            value: None,
            children: [Some(lhs), Some(rhs)],
        })
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.freq.partial_cmp(&self.freq)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.freq.cmp(&self.freq)
    }
}

/*
 * Huffman encode/compress input where the alphabet is the individual
 * bytes. The encoding tree is calculated based on the input and also returned.
 */
pub fn compress<'a>(input: &'a [u8]) -> (Box<Node>, usize, Vec<u64>) {
    let mut byte_freqs = [0 as usize; 256];
    for c in input {
        byte_freqs[*c as usize] += 1;
    }

    /* Build up a initial set of nodes, all-leaf, ordered by frequency (decreasing). */
    let mut nodes: Vec<Box<Node>> = byte_freqs
        .iter()
        .enumerate()
        .map(|(i, freq)| {
            Box::new(Node {
                value: Some(i as u8),
                freq: *freq,
                children: [None, None],
            })
        })
        .collect();
    nodes.retain(|a| a.freq > 0);
    nodes.sort();

    /* Successively merge nodes until there is only one left. */
    while nodes.len() > 1 {
        let n1 = nodes.pop().unwrap();
        let n2 = nodes.pop().unwrap();
        let n = Box::new(Node {
            value: None,
            freq: n1.freq + n2.freq,
            children: [Some(n1), Some(n2)],
        });

        /* Sorted insertion (again, by frequency): */
        let pos = nodes.binary_search(&n).unwrap_or_else(|e| e);
        nodes.insert(pos, n);
    }

    assert!(nodes.len() == 1);

    let mut encodings: [(usize, usize); 256] = [(0, 0); 256];
    nodes[0].get_encoding((0, 0), &mut encodings);

    /* Encode the input using the encoding created above. */
    let mut output: Vec<u64> = Vec::new();
    let mut current_word: u64 = 0x0;
    let mut current_bit_pos = 0;
    for c in input {
        let enc = encodings[*c as usize];
        for bitpos in 0..enc.1 {
            let bit = ((enc.0 >> bitpos) & 0x1) as u64;
            current_word |= bit << current_bit_pos;
            current_bit_pos += 1;
            if current_bit_pos >= 64 {
                output.push(current_word);
                current_bit_pos = 0;
                current_word = 0;
            }
        }
    }

    let num_bits = output.len() * 64 + current_bit_pos;
    output.push(current_word);
    (nodes.pop().unwrap(), num_bits, output)
}

pub fn decompress<'a>(tree: &Box<Node>, num_bits: usize, input: &'a [u64]) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();

    let mut current_word_pos: usize = 0;
    let mut current_bit_pos: usize = 0;

    let mut next_bit = || {
        if (current_word_pos * 64) + current_bit_pos < num_bits {
            let bit = (input[current_word_pos] >> current_bit_pos) & 0x1 != 0;
            current_bit_pos += 1;
            if current_bit_pos == 64 {
                current_word_pos += 1;
                current_bit_pos = 0;
            }
            Some(bit)
        } else {
            None
        }
    };

    while let Some(bit) = next_bit() {
        let mut t: &Node = tree.children[bit as usize].as_ref().unwrap();
        while t.value.is_none() {
            let bit = next_bit().unwrap();
            t = t.children[bit as usize].as_ref().unwrap();
        }

        output.push(t.value.unwrap());
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorem_impsum() {
        // let input = "Hello, World!!!";
        let input = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur sit amet justo nibh. Donec in diam et mi porta rhoncus eu non sem. Mauris vehicula laoreet libero at bibendum. Nam in efficitur sem, id iaculis felis. Vivamus dapibus vestibulum ultricies. Nulla justo massa, volutpat sed bibendum vel, posuere in lacus. Duis commodo lectus a eros feugiat, nec faucibus quam efficitur. Etiam blandit sapien a posuere porttitor. Curabitur fermentum, nisl nec ultrices volutpat, lectus enim cursus orci, sed finibus nisi erat hendrerit lorem. Ut eget diam eget sem egestas faucibus quis et lectus. Donec pharetra arcu cursus imperdiet aliquet. Sed nisi orci, scelerisque efficitur elit quis, sagittis rutrum velit. Duis et felis nunc. Nam urna ipsum, fermentum ut urna eu, hendrerit suscipit nisl.\nCurabitur volutpat metus ut vestibulum varius. Proin luctus metus arcu, non lobortis diam interdum at. Cras feugiat maximus lacus, a egestas urna blandit cursus. In molestie convallis massa, quis tincidunt dolor faucibus in. Praesent porttitor pulvinar turpis, luctus sagittis metus fringilla quis. Praesent venenatis tellus sit amet risus viverra, non maximus urna mattis. Aliquam ligula nunc, mollis at quam vel, vestibulum tincidunt nisl. Curabitur massa tortor, pharetra eu felis id, tincidunt sollicitudin nulla. Nullam sollicitudin iaculis tellus, eu venenatis elit scelerisque eu. Curabitur finibus diam et lectus ultricies, et pretium lacus pharetra. Vestibulum tincidunt arcu efficitur ipsum egestas, eget sollicitudin lectus efficitur. Praesent ultrices vel urna a semper.\nNam ultricies nisi neque, quis fringilla lectus tempus vel. Nam placerat ex lorem, ac molestie lorem aliquet id. Maecenas sed dui bibendum, elementum nunc non, congue lectus. Etiam semper gravida magna, a congue sapien sagittis vel. In dignissim neque quam, quis faucibus justo mattis a. Sed facilisis eleifend libero vitae fermentum. Ut porttitor neque non ipsum gravida ullamcorper. Aliquam odio nisl, posuere eu elit sit amet, auctor semper odio. Donec finibus ipsum ipsum, non sollicitudin lectus consequat bibendum.\nPellentesque consectetur semper orci mollis maximus. Integer egestas ut risus quis vulputate. Aliquam eu egestas nibh. Etiam id dapibus purus. Aenean non finibus tortor, in pharetra ex. Suspendisse ut justo tortor. Aliquam elementum nec massa vitae sollicitudin.\nEtiam libero velit, congue nec imperdiet sed, iaculis id sem. Vivamus euismod leo sem, sed faucibus nisl congue in. Quisque pulvinar ligula vitae urna vulputate, ac dictum nisl pretium. Mauris viverra justo quis rhoncus volutpat. Nullam et odio sed dui vehicula gravida. Etiam eu nibh eget felis egestas euismod. Pellentesque finibus, ligula id luctus sollicitudin, lorem justo condimentum nisi, vel euismod leo lectus gravida nunc. Suspendisse non lacus mauris. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae;";

        let (tree, n, compressed) = compress(input.as_bytes());
        let mut encoded_tree: Vec<u8> = Vec::new();
        tree.encode(&mut encoded_tree);
        let decoded_tree = Node::decode(&encoded_tree, &mut 0);

        // let mut prefix = Vec::new();
        // tree.dump(&mut prefix);

        eprintln!(
            "size: {}, compressed: {}",
            input.len(),
            compressed.len() * 8 + encoded_tree.len()
        );
        let decompressed = decompress(&decoded_tree, n, &compressed);
        assert_eq!(input.as_bytes(), decompressed.as_slice());
    }
}
