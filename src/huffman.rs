use std::cmp::Reverse;

use binary_heap_plus::{BinaryHeap, KeyComparator};
use unchecked_index::UncheckedIndex;

use crate::unchecked;

pub struct HuffmanTable {
    pub code_lens: UncheckedIndex<Vec<u8>>,
    pub max_code_len: u8,
}

impl Default for HuffmanTable {
    fn default() -> Self {
        Self {
            code_lens: unchecked!(vec![]),
            max_code_len: 0,
        }
    }
}

impl HuffmanTable {
    pub fn new(huffman_table: Vec<u8>, max_code_len: u8) -> Self {
        assert!(max_code_len <= 16);
        Self {
            code_lens: unchecked!(huffman_table),
            max_code_len,
        }
    }

    pub fn new_from_sym_weights(sym_weights: &[u32], max_code_len: u8) -> Self {
        struct Node {
            weight: u32,
            child1: u16,
            child2: u16,
        }
        let mut nodes = unchecked!(
            (0..sym_weights.len())
                .map(|sym| Node {
                    weight: sym_weights[sym],
                    child1: 0,
                    child2: 0,
                })
                .collect::<Vec<_>>()
        );
        let nodes_ptr = &mut nodes as *mut UncheckedIndex<Vec<Node>>;

        loop {
            let mut node_heap = BinaryHeap::from_vec_cmp(
                (0..nodes.len() as u16)
                    .filter(|&i| sym_weights[i as usize] > 0)
                    .collect(),
                KeyComparator(|i: &u16| Reverse(unchecked!(&*nodes_ptr)[*i as usize].weight)),
            );
            if node_heap.len() <= 1 {
                let mut code_lens = vec![0u8; sym_weights.len()];
                if let Some(node) = node_heap.pop() {
                    code_lens[node as usize] = 1;
                    return Self::new(code_lens, 1);
                }
                return Self::new(code_lens, 0);
            }
            // construct huffman tree
            while node_heap.len() > 1 {
                let min_node1 = node_heap.pop().unwrap();
                let min_node2 = node_heap.pop().unwrap();
                let weight1 = nodes[min_node1 as usize].weight;
                let weight2 = nodes[min_node2 as usize].weight;
                nodes.push(Node {
                    weight: weight1 + weight2,
                    child1: min_node1,
                    child2: min_node2,
                });
                node_heap.push(nodes.len() as u16 - 1);
            }

            // extract code lengths
            let mut code_lens = vec![0u8; nodes.len()];
            for i in (sym_weights.len()..nodes.len()).rev() {
                code_lens[nodes[i].child1 as usize] = code_lens[i] + 1;
                code_lens[nodes[i].child2 as usize] = code_lens[i] + 1;
            }
            code_lens.truncate(sym_weights.len());

            // if code lens are too long, shrink them
            let cur_max_code_len = *code_lens.iter().max().unwrap();
            if cur_max_code_len > max_code_len {
                let shrink_factor = 1 << (cur_max_code_len - max_code_len);
                nodes.truncate(sym_weights.len());
                nodes
                    .iter_mut()
                    .filter(|node| node.weight > 0)
                    .for_each(|node| node.weight = (node.weight / shrink_factor).max(1));
                continue;
            }
            return Self::new(code_lens, cur_max_code_len);
        }
    }
}

pub struct HuffmanEncoding {
    pub encodings: UncheckedIndex<Vec<(u16, u16)>>, // code, code_len
}

impl HuffmanEncoding {
    pub fn from_huffman_table(huffman_table: &HuffmanTable) -> Self {
        let code_lens = &huffman_table.code_lens;
        let mut encodings = unchecked!(vec![(0, 0); code_lens.len()]);
        let mut bits = 0;
        let mut current_bits_len = 1;

        let mut ordered_syms = (0..code_lens.len())
            .filter(|&i| code_lens[i] > 0)
            .map(|i| i as u16)
            .collect::<Vec<_>>();

        ordered_syms.sort_unstable_by_key(|&sym| (code_lens[sym as usize], sym));
        ordered_syms.iter().for_each(|&sym| {
            let shift = code_lens[sym as usize] - current_bits_len;
            if shift as i8 > 0 {
                bits <<= shift;
                current_bits_len += shift;
            }
            encodings[sym as usize] = (bits, code_lens[sym as usize] as u16);
            bits += 1;
        });
        Self { encodings }
    }
}

pub struct HuffmanDecoding {
    pub decodings: UncheckedIndex<Vec<(u16, u16)>>, // sym, code_len
    pub max_code_len: u8,
}

impl HuffmanDecoding {
    pub fn from_huffman_table(huffman_table: &HuffmanTable) -> Self {
        let encoding = HuffmanEncoding::from_huffman_table(huffman_table);
        let encodings = &encoding.encodings;
        let max_code_len = huffman_table.max_code_len;
        let mut decodings = unchecked!(vec![(0, 0); 1 << max_code_len]);

        for (sym, &(code, code_len)) in encodings.iter().enumerate() {
            if code_len > 0 {
                let rest_bits_len = max_code_len as u16 - code_len;
                let base = (code << rest_bits_len) as usize;
                decodings[base..][..1 << rest_bits_len].fill((sym as u16, code_len));
            }
        }
        Self {
            decodings,
            max_code_len,
        }
    }
}
