use orz::bits::*;
use std;

pub const HUFF_INVALID_SYMBOL: u16 = 65535;

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
    pub fn from_symbol_weight_vec(symbol_weight_vec: &[i32], symbol_bits_len_max: u8) -> HuffmanEncoder {
        let symbol_bits_len_vec = compute_symbol_bits_len_vec(symbol_weight_vec, symbol_bits_len_max);
        let encoding_vec = compute_encoding_vec(&symbol_bits_len_vec);
        return HuffmanEncoder {
            symbol_bits_len_vec: symbol_bits_len_vec,
            encoding_vec: encoding_vec,
        };
    }

    pub fn get_symbol_bits_lens(&self) -> &[u8] {
        return &self.symbol_bits_len_vec;
    }

    pub unsafe fn encode_to_bits(&self, symbol: u16, bits: &mut Bits) {
        let bits_len = *self.symbol_bits_len_vec.get_unchecked(symbol as usize);
        let bs = *self.encoding_vec.get_unchecked(symbol as usize) as u64;
        bits.put(bits_len, bs);
    }
}

impl HuffmanDecoder {
    pub fn from_symbol_bits_lens(symbol_bits_len_vec: &[u8], symbol_bits_len_max: u8) -> HuffmanDecoder {
        let encoding_vec = compute_encoding_vec(symbol_bits_len_vec);
        let decoding_vec = compute_decoding_vec(symbol_bits_len_vec, &encoding_vec, symbol_bits_len_max);
        return HuffmanDecoder {
            symbol_bits_len_vec: Vec::from(symbol_bits_len_vec),
            symbol_bits_len_max: symbol_bits_len_max,
            decoding_vec: decoding_vec,
        };
    }

    pub unsafe fn decode_from_bits(&self, bits: &mut Bits) -> u16 {
        let symbol = *self.decoding_vec
            .get_unchecked(bits.peek(self.symbol_bits_len_max) as usize);
        if symbol != HUFF_INVALID_SYMBOL {
            bits.skip(*self.symbol_bits_len_vec.get_unchecked(symbol as usize));
        }
        return symbol;
    }
}

fn compute_symbol_bits_len_vec(symbol_weight_vec: &[i32], symbol_bits_len_max: u8) -> Vec<u8> {
    #[derive(Ord, Eq, PartialOrd, PartialEq)]
    struct Node {
        weight: i32,
        symbol: u16,
        child1: Option<Box<Node>>,
        child2: Option<Box<Node>>,
    };

    let mut symbol_bits_len_vec = vec![0u8; 0];
    for shrink_factor in 0.. {
        symbol_bits_len_vec.resize(0, 0);
        symbol_bits_len_vec.resize(
            match symbol_weight_vec.len() % 2 {
                0 => symbol_weight_vec.len(),
                _ => symbol_weight_vec.len() + 1,
            },
            0,
        );

        let mut node_heap = symbol_weight_vec
            .iter()
            .enumerate()
            .filter_map(|(i, &weight)| match weight {
                0 => None,
                _ => Some(Box::new(Node {
                    weight: -std::cmp::max(weight / (1 << shrink_factor), 1),
                    symbol: i as u16,
                    child1: None,
                    child2: None,
                })),
            })
            .collect::<std::collections::BinaryHeap<_>>();

        if node_heap.len() == 0 {
            break;
        }
        if node_heap.len() == 1 {
            symbol_bits_len_vec[node_heap.pop().unwrap().symbol as usize] = 1;
            break;
        }

        // construct huffman tree
        while node_heap.len() > 1 {
            let min_node1 = node_heap.pop().unwrap();
            let min_node2 = node_heap.pop().unwrap();
            node_heap.push(Box::new(Node {
                weight: min_node1.weight + min_node2.weight,
                symbol: HUFF_INVALID_SYMBOL,
                child1: Some(min_node1),
                child2: Some(min_node2),
            }));
        }

        // iterate huffman tree and extract symbol bits length
        let root_node = node_heap.pop().unwrap();
        let mut nodes_iterator_queue = vec![(0, &root_node)];
        let mut need_shrink = false;
        while !nodes_iterator_queue.is_empty() {
            let (depth, node) = nodes_iterator_queue.pop().unwrap();
            if node.symbol == HUFF_INVALID_SYMBOL {
                if depth == symbol_bits_len_max {
                    need_shrink = true;
                    break;
                }
                nodes_iterator_queue.push((depth + 1, &node.child1.as_ref().unwrap()));
                nodes_iterator_queue.push((depth + 1, &node.child2.as_ref().unwrap()));
            } else {
                symbol_bits_len_vec[node.symbol as usize] = depth;
            }
        }
        if !need_shrink {
            break; // now we are done
        }
    }
    return symbol_bits_len_vec;
}

fn compute_encoding_vec(symbol_bits_len_vec: &[u8]) -> Vec<u16> {
    let mut encoding_vec = vec![0u16; symbol_bits_len_vec.len()];
    let mut bits: u16 = 0;
    let mut current_bits_len: u8 = 1;

    #[derive(Ord, Eq, PartialOrd, PartialEq)]
    struct SymbolWithBitsLens {
        bits_len: u8,
        symbol: u16,
    }
    let ordered_symbol_with_bits_lens = symbol_bits_len_vec
        .iter()
        .enumerate()
        .filter_map(|(i, &bits_len)| match bits_len {
            0 => None,
            _ => Some(Box::new(SymbolWithBitsLens {
                bits_len: bits_len,
                symbol: i as u16,
            })),
        })
        .collect::<std::collections::BTreeSet<_>>();

    ordered_symbol_with_bits_lens
        .iter()
        .for_each(|symbol_with_bits_len| {
            while current_bits_len < symbol_with_bits_len.bits_len {
                bits <<= 1;
                current_bits_len += 1;
            }
            encoding_vec[symbol_with_bits_len.symbol as usize] = bits;
            bits += 1;
        });
    encoding_vec
}

fn compute_decoding_vec(symbol_bits_len_vec: &[u8], encoding_vec: &[u16], symbol_bits_len_max: u8) -> Vec<u16> {
    let mut decoding_vec = vec![HUFF_INVALID_SYMBOL; 1 << symbol_bits_len_max];
    for symbol in 0..symbol_bits_len_vec.len() {
        if symbol_bits_len_vec[symbol] > 0 {
            let rest_bits_len = symbol_bits_len_max - symbol_bits_len_vec[symbol];
            let blo = (encoding_vec[symbol] as u32 + 0) << rest_bits_len;
            let bhi = (encoding_vec[symbol] as u32 + 1) << rest_bits_len;
            for b in blo..bhi {
                decoding_vec[b as usize] = symbol as u16
            }
        }
    }
    decoding_vec
}
