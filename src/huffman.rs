use super::aux::UncheckedSliceExt;
use super::bits::Bits;

pub struct HuffmanEncoder {
    symbol_bits_len_vec: Vec<u8>,
    encoding_vec: Vec<u16>,
}

pub struct HuffmanDecoder {
    symbol_bits_len_vec: Vec<u8>,
    symbol_bits_len_max: u8,
    decoding_vec: Vec<u16>,
}

impl HuffmanEncoder {
    pub fn from_symbol_weight_vec(symbol_weight_vec: &[u32], symbol_bits_len_max: u8) -> HuffmanEncoder {
        let symbol_bits_len_vec = compute_symbol_bits_len_vec(symbol_weight_vec, symbol_bits_len_max);
        let encoding_vec = compute_encoding_vec(&symbol_bits_len_vec);
        return HuffmanEncoder {
            symbol_bits_len_vec,
            encoding_vec,
        };
    }

    pub fn get_symbol_bits_lens(&self) -> &[u8] {
        return &self.symbol_bits_len_vec;
    }

    pub unsafe fn encode_to_bits(&self, symbol: u16, bits: &mut Bits) {
        let bits_len = self.symbol_bits_len_vec.nocheck()[symbol as usize];
        let bs = self.encoding_vec.nocheck()[symbol as usize];
        bits.put(bits_len, bs as u64);
    }
}

impl HuffmanDecoder {
    pub fn from_symbol_bits_lens(symbol_bits_len_vec: &[u8]) -> HuffmanDecoder {
        let symbol_bits_len_max = *symbol_bits_len_vec.iter().max().unwrap();
        let encoding_vec = compute_encoding_vec(symbol_bits_len_vec);
        let decoding_vec = compute_decoding_vec(symbol_bits_len_vec, &encoding_vec, symbol_bits_len_max);
        return HuffmanDecoder {
            symbol_bits_len_vec: Vec::from(symbol_bits_len_vec).to_vec(),
            symbol_bits_len_max,
            decoding_vec,
        };
    }

    pub unsafe fn decode_from_bits(&self, bits: &mut Bits) -> u16 {
        let symbol = self.decoding_vec.nocheck()[bits.peek(self.symbol_bits_len_max) as usize];
        bits.skip(self.symbol_bits_len_vec.nocheck()[symbol as usize]);
        return symbol;
    }
}

fn compute_symbol_bits_len_vec(symbol_weight_vec: &[u32], symbol_bits_len_max: u8) -> Vec<u8> {
    #[derive(Ord, Eq, PartialOrd, PartialEq)]
    struct Node {
        weight: i64,
        symbol: u16,
        child1: Option<Box<Node>>,
        child2: Option<Box<Node>>,
    };

    'shrink: for shrink_factor in 0 .. {
        let mut symbol_bits_len_vec = vec![0u8; match symbol_weight_vec.len() % 2 {
            0 => symbol_weight_vec.len(),
            _ => symbol_weight_vec.len() + 1,
        }];

        let mut node_heap = symbol_weight_vec.iter().enumerate().filter_map(|(symbol, &weight)| {
            match weight {
                0 => None,
                _ => Some(Box::new(Node {
                    weight: -std::cmp::max(weight as i64 / (1 << shrink_factor), 1),
                    symbol: symbol as u16,
                    child1: None,
                    child2: None,
                })),
            }
        }).collect::<std::collections::BinaryHeap<_>>();

        if node_heap.len() < 2 {
            if node_heap.len() == 1 {
                symbol_bits_len_vec[node_heap.pop().unwrap().symbol as usize] = 1;
            }
            return symbol_bits_len_vec;
        }

        // construct huffman tree
        while node_heap.len() > 1 {
            let min_node1 = node_heap.pop().unwrap();
            let min_node2 = node_heap.pop().unwrap();
            node_heap.push(Box::new(Node {
                weight: min_node1.weight + min_node2.weight,
                symbol: u16::max_value(),
                child1: Some(min_node1),
                child2: Some(min_node2),
            }));
        }

        // iterate huffman tree and extract symbol bits length
        let root_node = node_heap.pop().unwrap();
        let mut nodes_iterator_queue = vec![(0, &root_node)];
        while !nodes_iterator_queue.is_empty() {
            let (depth, node) = nodes_iterator_queue.pop().unwrap();
            if node.symbol == u16::max_value() {
                if depth == symbol_bits_len_max {
                    continue 'shrink;
                }
                nodes_iterator_queue.push((depth + 1, &node.child1.as_ref().unwrap()));
                nodes_iterator_queue.push((depth + 1, &node.child2.as_ref().unwrap()));
            } else {
                symbol_bits_len_vec[node.symbol as usize] = depth;
            }
        }
        return symbol_bits_len_vec;
    }
    unreachable!()
}

fn compute_encoding_vec(symbol_bits_len_vec: &[u8]) -> Vec<u16> {
    let mut encoding_vec = vec![0u16; symbol_bits_len_vec.len()];
    let mut bits: u16 = 0;
    let mut current_bits_len: u8 = 1;

    let ordered_symbol_with_bits_lens = symbol_bits_len_vec.iter().enumerate().filter_map(|(symbol, &bits_len)| {
        match bits_len {
            0 => None,
            _ => Some((bits_len, symbol as u16)),
        }
    }).collect::<std::collections::BTreeSet<_>>();

    ordered_symbol_with_bits_lens.iter().for_each(|symbol_with_bits_len| {
        while current_bits_len < symbol_with_bits_len.0 {
            bits <<= 1;
            current_bits_len += 1;
        }
        encoding_vec[symbol_with_bits_len.1 as usize] = bits;
        bits += 1;
    });
    return encoding_vec;
}

fn compute_decoding_vec(symbol_bits_len_vec: &[u8], encoding_vec: &[u16], symbol_bits_len_max: u8) -> Vec<u16> {
    let mut decoding_vec = vec![0u16; 1 << symbol_bits_len_max];
    for symbol in 0..symbol_bits_len_vec.len() {
        unsafe {
            if symbol_bits_len_vec.nocheck()[symbol as usize] > 0 {
                let rest_bits_len = symbol_bits_len_max - symbol_bits_len_vec.nocheck()[symbol as usize];
                let blo = (encoding_vec.nocheck()[symbol as usize] + 0) << rest_bits_len;
                let bhi = (encoding_vec.nocheck()[symbol as usize] + 1) << rest_bits_len;
                for b in blo..bhi {
                    decoding_vec.nocheck_mut()[b as usize] = symbol as u16;
                }
            }
        }
    }
    return decoding_vec;
}
