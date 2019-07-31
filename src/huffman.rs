use std::ops::AddAssign;
use super::auxility::UncheckedSliceExt;
use super::bits::Bits;

pub struct HuffmanEncoder {
    canonical_lens: Vec<u8>,
    encodings: Vec<u16>,
}

pub struct HuffmanDecoder {
    canonical_lens: Vec<u8>,
    canonical_lens_max: u8,
    decodings: Vec<u16>,
}

impl HuffmanEncoder {
    pub unsafe fn new(symbol_weights: &[u32], max_bits_len: u8, buf: &mut [u8], pos: &mut usize) -> HuffmanEncoder {
        let canonical_lens = compute_canonical_lens(symbol_weights, max_bits_len);
        let encodings = compute_encodings(&canonical_lens);

        (0 .. symbol_weights.len()).step_by(2).for_each(|i| buf[*pos + i / 2]  = u8::to_be(canonical_lens[i]) << 4);
        (1 .. symbol_weights.len()).step_by(2).for_each(|i| buf[*pos + i / 2] |= u8::to_be(canonical_lens[i]) << 0);
        pos.add_assign((symbol_weights.len() + 1) / 2);
        return HuffmanEncoder {canonical_lens, encodings};
    }

    pub unsafe fn encode_to_bits(&self, symbol: u16, bits: &mut Bits) {
        let bits_len = self.canonical_lens.nc()[symbol as usize];
        let bs = self.encodings.nc()[symbol as usize];
        bits.put(bits_len, bs as u64);
    }
}

impl HuffmanDecoder {
    pub unsafe fn new(num_symbols: usize, buf: &[u8], pos: &mut usize) -> HuffmanDecoder {
        let mut canonical_lens = (0..num_symbols).into_iter().map(|_| 0).collect::<Vec<_>>();
        (0 .. num_symbols).step_by(2).for_each(|i| canonical_lens[i] = u8::from_be(buf[*pos + i / 2] & 0xf0) >> 4);
        (1 .. num_symbols).step_by(2).for_each(|i| canonical_lens[i] = u8::from_be(buf[*pos + i / 2] & 0x0f) >> 0);
        pos.add_assign((num_symbols + 1) / 2);

        let canonical_lens_max = *canonical_lens.iter().max().unwrap();
        let encodings = compute_encodings(&canonical_lens);
        let decodings = compute_decodings(&canonical_lens, &encodings, canonical_lens_max);
        return HuffmanDecoder {canonical_lens, canonical_lens_max, decodings};
    }

    pub unsafe fn decode_from_bits(&self, bits: &mut Bits) -> u16 {
        let symbol = self.decodings.nc()[bits.peek(self.canonical_lens_max) as usize];
        bits.get(self.canonical_lens.nc()[symbol as usize]);
        return symbol;
    }
}

fn compute_canonical_lens(symbol_weights: &[u32], canonical_lens_max: u8) -> Vec<u8> {
    #[derive(Ord, Eq, PartialOrd, PartialEq)]
    struct Node {
        weight: i64,
        symbol: u16,
        children: Option<[Box<Node>; 2]>,
    };

    'shrink: for shrink_factor in 0 .. {
        let mut canonical_lens = vec![0; symbol_weights.len() + symbol_weights.len() % 2];
        let mut node_heap = symbol_weights.iter().enumerate().filter_map(|(symbol, &weight)| {
            match weight {
                0 => None,
                _ => Some(Box::new(Node {
                    weight: -std::cmp::max(weight as i64 / (1 << shrink_factor), 1),
                    symbol: symbol as u16,
                    children: None,
                }))
            }
        }).collect::<std::collections::BinaryHeap<_>>();

        if node_heap.len() < 2 {
            if node_heap.len() == 1 {
                canonical_lens[node_heap.pop().unwrap().symbol as usize] = 1;
            }
            return canonical_lens;
        }

        // construct huffman tree
        while node_heap.len() > 1 {
            let min_node1 = node_heap.pop().unwrap();
            let min_node2 = node_heap.pop().unwrap();
            node_heap.push(Box::new(Node {
                weight: min_node1.weight + min_node2.weight,
                symbol: 65535,
                children: Some([min_node1, min_node2]),
            }));
        }
        let root_node = node_heap.pop().unwrap();

        // iterate huffman tree and extract symbol bits length
        let mut nodes_iterator_queue = vec![(0, &root_node)];
        while !nodes_iterator_queue.is_empty() {
            let (depth, node) = nodes_iterator_queue.pop().unwrap();
            if node.symbol == 65535 {
                if depth >= canonical_lens_max {
                    continue 'shrink;
                }
                nodes_iterator_queue.push((depth + 1, &node.children.as_ref().unwrap()[0]));
                nodes_iterator_queue.push((depth + 1, &node.children.as_ref().unwrap()[1]));
            } else {
                canonical_lens[node.symbol as usize] = depth;
            }
        }
        return canonical_lens;
    }
    unreachable!()
}

unsafe fn compute_encodings(canonical_lens: &[u8]) -> Vec<u16> {
    let mut encodings = vec![0u16; canonical_lens.len()];
    let mut bits = 0;
    let mut current_bits_len = 1;

    let mut ordered_symbols = (0 .. canonical_lens.len()).filter(|&i| canonical_lens.nc()[i as usize] > 0)
        .map(|i| i as u16)
        .collect::<Vec<_>>();

    ordered_symbols.sort_by_key(|&symbol| canonical_lens.nc()[symbol as usize]);
    ordered_symbols.iter().for_each(|&symbol| {
        while current_bits_len < canonical_lens.nc()[symbol as usize] {
            bits <<= 1;
            current_bits_len += 1;
        }
        encodings.nc_mut()[symbol as usize] = bits;
        bits += 1;
    });
    return encodings;
}

unsafe fn compute_decodings(canonical_lens: &[u8], encodings: &[u16], canonical_lens_max: u8) -> Vec<u16> {
    let mut decodings = vec![0u16; 1 << canonical_lens_max];
    for symbol in 0..canonical_lens.len() as u16 {
        if canonical_lens.nc()[symbol as usize] > 0 {
            let rest_bits_len = canonical_lens_max - canonical_lens.nc()[symbol as usize];
            for i in 0..2usize.pow(rest_bits_len as u32) {
                decodings.nc_mut()[(encodings.nc()[symbol as usize] << rest_bits_len) as usize + i] = symbol;
            }
        }
    }
    return decodings;
}
