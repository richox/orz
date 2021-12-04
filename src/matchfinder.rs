use modular_bitfield::prelude::*;

use super::byteslice::ByteSliceExt;

#[derive(Clone, Copy)]
pub enum MatchResult {
    Unmatched,
    Matched {
        reduced_offset: u16,
        match_len: usize,
        match_len_expected: usize,
        match_len_min: usize,
    }
}

#[derive(Clone, Copy)]
pub struct Bucket {
    nodes: [Node; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:25 | match_len_expected:7
    head: u16,

    /* match_len_expected:
     *  the match length we got when searching match for this position
     *  if no match is found, this value is set to 0.
     *
     *  when a newer position matches this position, it is likely that the match length
     *  is the same with this value.
     *
     * match_len_min:
     *  the longest match of all newer position that matches this position
     *  if no match is found, this value is set to LZ_MATCH_MIN_LEN-1.
     *
     *  when a newer position matches this position, the match length is always
     *  longer than this value, because shortter matches will stop at a newer position
     *  that matches this position.
     *
     *  A A A A A B B B B B A A A A A C C C C C A A A A A
     *  |                   |
     *  |<------------------|
     *  |                   |
     *  |                   match_len_expected=5
     *  match_len_min=6
     */
} impl Bucket {
    pub fn new() -> Bucket {
        return Bucket {
            head: 0,
            nodes: [Node::new(); super::LZ_MF_BUCKET_ITEM_SIZE],
        };
    }

    pub fn update(&mut self, pos: usize, reduced_offset: u16, match_len: usize) {
        let new_head = node_size_bounded_add(self.head, 1) as usize;
        crate::assert_unchecked!(new_head < super::LZ_MF_BUCKET_ITEM_SIZE);

        // update match_len_min of matched position
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head, reduced_offset) as usize;
            if self.nodes[node_index].match_len_min() <= match_len as u8 {
                self.nodes[node_index].set_match_len_min(match_len as u8 + 1);
            }
        }

        // update match_len_expected of incomping position
        let match_len_expected = match match_len { // match_len_expected < 128 because only 7 bits reserved
            0 ..= 127 => match_len,
            _ => 0,
        };
        self.nodes[new_head] = Node::new()
            .with_pos(pos as u32)
            .with_match_len_expected(match_len_expected as u8);

        // move head to next node
        self.head = new_head as u16;
    }

    pub fn forward(&mut self, forward_len: usize) {
        // reduce all positions
        for node in &mut self.nodes {
            node.set_pos(node.pos().saturating_sub(forward_len as u32));
        }
    }

    pub fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.head, reduced_offset) as usize;
        crate::assert_unchecked!(node_index < super::LZ_MF_BUCKET_ITEM_SIZE);
        return (
            self.nodes[node_index].pos() as usize,
            std::cmp::max(self.nodes[node_index].match_len_expected() as usize, super::LZ_MATCH_MIN_LEN),
            std::cmp::max(self.nodes[node_index].match_len_min() as usize, super::LZ_MATCH_MIN_LEN),
        );
    }
}

pub struct BucketMatcher {
    heads: [u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [u16; super::LZ_MF_BUCKET_ITEM_SIZE],

} impl BucketMatcher {
    pub fn new() -> BucketMatcher {
        return BucketMatcher {
            heads: [u16::max_value(); super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [u16::max_value(); super::LZ_MF_BUCKET_ITEM_SIZE],
        };
    }

    pub unsafe fn update(&mut self, bucket: &Bucket, buf: &[u8], pos: usize) {
        let self_heads = &mut unchecked_index::unchecked_index(&mut self.heads);
        let self_nexts = &mut unchecked_index::unchecked_index(&mut self.nexts);

        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        self_nexts[bucket.head as usize] = self_heads[entry];
        self_heads[entry] = bucket.head;
    }

    pub fn forward(&mut self, bucket: &Bucket) {
        // clear all entries/positions that points to out-of-date node
        self.heads.iter_mut()
            .filter(|head| **head != u16::max_value() && bucket.nodes[**head as usize].pos() == 0)
            .for_each(|head| *head = u16::max_value());
        self.nexts.iter_mut()
            .filter(|next| **next != u16::max_value() && bucket.nodes[**next as usize].pos() == 0)
            .for_each(|next| *next = u16::max_value());
    }

    pub unsafe fn find_match(&self, bucket: &Bucket, buf: &[u8], pos: usize, match_depth: usize) -> MatchResult {
        let self_heads = &unchecked_index::unchecked_index(&self.heads);
        let self_nexts = &unchecked_index::unchecked_index(&self.nexts);
        let bucket_nodes = &unchecked_index::unchecked_index(&bucket.nodes);

        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self_heads[entry] as usize;

        if node_index == u16::max_value() as usize {
            return MatchResult::Unmatched;
        }
        let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
        let mut max_node_index = 0;
        let mut max_len_dword = buf.read(pos + max_len - 3);
        let mut max_match_len_min = 0;
        let mut max_match_len_expected = 0;

        for _ in 0..match_depth {
            let node_pos = bucket_nodes[node_index].pos() as usize;

            // check the last 4 bytes of longest match (fast)
            // then perform full LCP search
            if buf.read::<u32>(node_pos + max_len - 3) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_match_len_min = bucket_nodes[node_index].match_len_min() as usize;
                    max_match_len_expected = bucket_nodes[node_index].match_len_expected() as usize;
                    max_len = lcp;
                    max_node_index = node_index;
                    max_len_dword = buf.read(pos + max_len - 3);
                }
                if lcp == super::LZ_MATCH_MAX_LEN || (max_match_len_expected > 0 && lcp > max_match_len_expected) {
                    /*
                     * (1)                 (2)                 (3)
                     *  A A A A A B B B B B A A A A A C C C C C A A A A A C B
                     *  |                   |                   |
                     *  |<-5----------------|                   |
                     *  |                   |                   |
                     *  |                   match_len_expected=5|
                     *  match_len_min=6                         |
                     *                END<--|<-6----------------|
                     *                      |
                     *                      lcp=6 > max_match_len_expected
                     *                      ## skip further matches
                     *                      if there are better matches, (2) would have had match it
                     *                      and got a longer match_len_expected.
                     */
                    break;
                }
            }

            let node_next = self_nexts[node_index] as usize;
            if node_next == u16::max_value() as usize || node_pos <= bucket_nodes[node_next].pos() as usize {
                break;
            }
            node_index = node_next;
        }

        if max_len >= super::LZ_MATCH_MIN_LEN && pos + max_len < buf.len() {
            return MatchResult::Matched {
                reduced_offset: node_size_bounded_sub(bucket.head, max_node_index as u16),
                match_len: max_len,
                match_len_expected: std::cmp::max(max_match_len_expected, super::LZ_MATCH_MIN_LEN),
                match_len_min: std::cmp::max(max_match_len_min, super::LZ_MATCH_MIN_LEN),
            };
        }
        return MatchResult::Unmatched;
    }

    pub unsafe fn has_lazy_match(&self, bucket: &Bucket, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        let self_heads = &unchecked_index::unchecked_index(&self.heads);
        let self_nexts = &unchecked_index::unchecked_index(&self.nexts);
        let bucket_nodes = &unchecked_index::unchecked_index(&bucket.nodes);

        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self_heads[entry] as usize;

        if node_index == u16::max_value() as usize {
            return false;
        }
        let max_len_dword = buf.read::<u32>(pos + min_match_len - 4);
        for _ in 0..depth {
            let node_pos = bucket_nodes[node_index].pos() as usize;

            // first check the last 4 bytes of longest match (fast)
            // then perform full comparison
            if buf.read::<u32>(node_pos + min_match_len - 4) == max_len_dword {
                if super::mem::memequ_hack_fast(buf, node_pos, pos, min_match_len - 4) {
                    return true;
                }
            };

            let node_next = self_nexts[node_index] as usize;
            if node_next == u16::max_value() as usize || node_pos <= bucket_nodes[node_next].pos() as usize {
                break;
            }
            node_index = node_next;
        }
        return false;
    }
}

#[bitfield]
#[derive(Clone, Copy)]
struct Node {
    pos: B25,
    match_len_expected: B7,
    match_len_min: B8,
}

#[allow(dead_code)]
fn _suppress_warnings() {
    let _ = Node::new().into_bytes();
    let _ = Node::from_bytes([0u8; 5]);
}

fn node_size_bounded_add(v1: u16, v2: u16) -> u16 {
    return (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

fn node_size_bounded_sub(v1: u16, v2: u16) -> u16 {
    return (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as u16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> usize {
    return crc32c_hw::update(0, &buf.read::<[u8; 4]>(pos)) as usize;
}
