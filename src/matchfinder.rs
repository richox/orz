use byteorder::BE;
use byteorder::ByteOrder;
use super::aux::UncheckedSliceExt;

// nodes:
//  [0..25)  pos
//  [25..32) match len at pos
//  requires: pos < 2^25 and match_len < 2^7

pub struct EncoderMFBucket {
    heads: [i16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [i16; super::LZ_MF_BUCKET_ITEM_SIZE],
    nodes: [Node; super::LZ_MF_BUCKET_ITEM_SIZE],
    head: i16,
}

pub struct DecoderMFBucket {
    nodes: [Node; super::LZ_MF_BUCKET_ITEM_SIZE],
    head: i16,
}

pub struct MatchResult {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
}

#[derive(Clone, Copy)]
struct Node(u32);
impl Node {
    fn new(pos: usize, match_len_expected: usize) -> Node {
        Node(pos as u32 | (match_len_expected as u32) << 25)
    }
    fn get_pos(&self) -> usize {
        self.0 as usize & 0x01ffffff
    }
    fn get_match_len_expected(&self) -> usize {
        self.0 as usize >> 25
    }
}

impl EncoderMFBucket {
    pub fn new() -> EncoderMFBucket {
        EncoderMFBucket {
            heads: [0; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            nodes: [Node::new(0, 0); super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.nodes.iter_mut().for_each(|node|
            *node = Node::new(node.get_pos().saturating_sub(forward_len), node.get_match_len_expected()));
    }

    pub unsafe fn find_match(&self, buf: &[u8], pos: usize, match_depth: usize) -> Option<MatchResult> {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut node_index = self.heads.nocheck()[entry] as usize;

        if node_index != 0 {
            // start matching
            let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
            let mut max_node_index = 0;
            let mut max_len_dword = *((buf.as_ptr() as usize + pos) as *const u32);
            let mut max_match_len_expected = super::LZ_MATCH_MIN_LEN;

            for _ in 0..match_depth {
                let node = self.nodes.nocheck()[node_index];
                let node_pos = node.get_pos();
                let node_match_len_expected = node.get_match_len_expected();

                if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                    let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                    let (lcp_fix, max_len_fix) = (
                        lcp + (lcp == node_match_len_expected) as usize,
                        max_len + (max_len == max_match_len_expected) as usize,
                    );
                    if lcp_fix > max_len_fix {
                        max_len = lcp;
                        max_node_index = node_index;
                        max_match_len_expected = node_match_len_expected;

                        if max_len == super::LZ_MATCH_MAX_LEN {
                            break;
                        }
                        max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
                    }
                }

                node_index = self.nexts.nocheck()[node_index] as usize;
                if node_index == 0 || node_pos <= self.nodes.nocheck()[node_index].get_pos() {
                    break;
                }
            }

            if max_len >= super::LZ_MATCH_MIN_LEN {
                return Some(MatchResult {
                    reduced_offset: node_size_bounded_sub(self.head, max_node_index as i16) as usize,
                    match_len: max_len,
                    match_len_expected: max_match_len_expected,
                });
            }
        }
        None
    }

    pub unsafe fn update(&mut self, buf: &[u8], pos: usize, match_len: usize) {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        self.head = node_size_bounded_add(self.head, 1) as i16;
        self.nexts.nocheck_mut()[self.head as usize] = self.heads.nocheck()[entry];
        self.nodes.nocheck_mut()[self.head as usize] = Node::new(pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
        self.heads.nocheck_mut()[entry] = self.head;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut node_index = self.heads.nocheck()[entry] as usize;

        if node_index != 0 {
            let max_len_dword = *((buf.as_ptr() as usize + pos + min_match_len - 4) as *const u32);
            for _ in 0..depth {
                let node = self.nodes.nocheck()[node_index];
                let node_pos = node.get_pos();
                if *((buf.as_ptr() as usize + node_pos + min_match_len - 4) as *const u32) == max_len_dword {
                    let lcp = super::mem::llcp_fast(buf, node_pos, pos, min_match_len - 4);
                    if lcp >= min_match_len - 4 {
                        return true;
                    }
                };

                node_index = self.nexts.nocheck()[node_index] as usize;
                if node_index == 0 || node_pos <= self.nodes.nocheck()[node_index].get_pos() {
                    break;
                }
            }
        }
        return false;
    }
}

impl DecoderMFBucket {
    pub fn new() -> DecoderMFBucket {
        DecoderMFBucket {
            nodes: [Node::new(0, 0); super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.nodes.iter_mut().for_each(|node|
            *node = Node::new(node.get_pos().saturating_sub(forward_len), node.get_match_len_expected()));
    }

    pub unsafe fn update(&mut self, pos: usize, match_len: usize) {
        self.head = node_size_bounded_add(self.head, 1);
        self.nodes.nocheck_mut()[self.head as usize] = Node::new(pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize) {
        let node = self.nodes.nocheck()[node_size_bounded_sub(self.head, reduced_offset as i16) as usize];
        return (node.get_pos(), node.get_match_len_expected());
    }
}

fn node_size_bounded_add(v1: i16, v2: i16) -> i16 {
    (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

fn node_size_bounded_sub(v1: i16, v2: i16) -> i16 {
    (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as i16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> u32 {
    let u32context= BE::read_u32(std::slice::from_raw_parts(buf.get_unchecked(pos), 4));
    return u32context * 131 + u32context / 131;
}
