use crate::mem::{BytesConstPtrExt, mem_fast_common_prefix};
use crate::mem::mem_fast_equal;
use crate::{LZ_MATCH_MAX_LEN, unchecked};
use crate::LZ_MATCH_MIN_LEN;
use crate::LZ_MF_BUCKET_ITEM_HASH_SIZE;
use crate::LZ_MF_BUCKET_ITEM_SIZE;

use modular_bitfield::prelude::*;

#[derive(Clone, Copy, Default)] // Match::default = unmatched
pub struct Match {
    pub reduced_offset: u16,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

#[derive(Clone, Copy)]
pub struct MatchInfo {
    pub match_pos: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

#[derive(Clone, Copy)]
pub struct Bucket {
    nodes: [Node; LZ_MF_BUCKET_ITEM_SIZE], // pos:25 | match_len_expected:7
    head: i16,
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
}

impl Bucket {
    pub fn new() -> Self {
        Self {
            nodes: [Node::default(); LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub unsafe fn update(&mut self, pos: usize, reduced_offset: u16, match_len: usize) {
        let mut nodes = unchecked!(&mut self.nodes);
        let new_head = node_size_bounded_add(self.head as u16, 1);

        // update match_len_min of matched position
        if match_len >= LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head as u16, reduced_offset) as usize;
            if nodes[node_index].match_len_min() <= match_len as u8 {
                nodes[node_index].set_match_len_min(match_len as u8 + 1);
            }
        }

        // update match_len_expected of incomping position
        // match_len_expected < 128 because only 7 bits reserved
        let match_len_expected = if match_len <= 127 { match_len } else { 0 };
        nodes[new_head as usize] = Node::new()
            .with_pos(pos as u32)
            .with_match_len_expected(match_len_expected as u8);

        // move head to next node
        self.head = new_head as i16;
    }

    pub fn forward(&mut self, forward_len: usize) {
        // reduce all positions
        for node in &mut self.nodes {
            node.set_pos(node.pos().saturating_sub(forward_len as u32));
        }
    }

    pub unsafe fn get_match_info(&self, reduced_offset: u16) -> MatchInfo {
        let node_index = node_size_bounded_sub(self.head as u16, reduced_offset) as usize;
        let nodes = unchecked!(&self.nodes);
        MatchInfo {
            match_pos: nodes[node_index].pos() as usize,
            match_len_expected: LZ_MATCH_MIN_LEN.max(nodes[node_index].match_len_expected() as usize),
            match_len_min: LZ_MATCH_MIN_LEN.max(nodes[node_index].match_len_min() as usize),
        }
    }
}

#[derive(Clone, Copy)]
pub struct BucketMatcher {
    heads: [i16; LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [i16; LZ_MF_BUCKET_ITEM_SIZE],
}

impl BucketMatcher {
    pub fn new() -> Self {
        Self {
            heads: [-1; LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [-1; LZ_MF_BUCKET_ITEM_SIZE],
        }
    }

    pub unsafe fn update(&mut self, bucket: &Bucket, buf: &[u8], pos: usize) {
        let mut heads = unchecked!(&mut self.heads);
        let mut nexts = unchecked!(&mut self.nexts);

        let head = bucket.head as usize;
        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;

        nexts[head] = heads[entry];
        heads[entry] = bucket.head as i16;
    }

    pub fn forward(&mut self, bucket: &Bucket) {
        // clear all entries/positions that points to out-of-date node
        self.heads
            .iter_mut()
            .filter(|head| **head != -1 && bucket.nodes[**head as usize].pos() == 0)
            .for_each(|head| *head = -1);
        self.nexts
            .iter_mut()
            .filter(|next| **next != -1 && bucket.nodes[**next as usize].pos() == 0)
            .for_each(|next| *next = -1);
    }

    pub unsafe fn find_match(
        &self,
        bucket: &Bucket,
        buf: &[u8],
        pos: usize,
        match_depth: usize,
    ) -> Match {
        let heads = &unchecked!(&self.heads);
        let nexts = &unchecked!(&self.nexts);
        let bucket_nodes = &unchecked!(&bucket.nodes);

        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = heads[entry];

        if node_index == -1 {
            return Match::default();
        }
        let mut max_len = LZ_MATCH_MIN_LEN - 1;
        let mut max_match_len_min = LZ_MATCH_MIN_LEN;
        let mut max_match_len_expected = LZ_MATCH_MIN_LEN;
        let mut max_node_index = 0;
        let mut node_pos = bucket_nodes[node_index as usize].pos();
        let mut max_len_dword: u32 = buf.as_ptr().get(pos + max_len - 3, 4);

        for _ in 0..match_depth {
            let node_max_len_dword: u32 = buf.as_ptr().get(node_pos as usize + max_len - 3, 4);
            // first check the last 4 bytes of longest match (likely to be unequal for a failed match)
            // then perform full LCP search
            if node_max_len_dword == max_len_dword {
                let lcp =
                    mem_fast_common_prefix(buf.as_ptr(), node_pos as usize, pos, LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    let bucket_node = &bucket_nodes[node_index as usize];
                    max_match_len_min = bucket_node.match_len_min() as usize;
                    max_match_len_expected = bucket_node.match_len_expected() as usize;
                    max_len = lcp;
                    max_node_index = node_index;
                    max_len_dword = buf.as_ptr().get(pos + max_len - 3, 4);
                }
                if lcp == LZ_MATCH_MAX_LEN {
                    break;
                }
                if max_match_len_expected > 0 && lcp > max_match_len_expected {
                    break;
                }
            }

            node_index = nexts[node_index as usize];
            if node_index == -1 {
                break;
            }

            let node_pos_next = bucket_nodes[node_index as usize].pos();
            if node_pos <= node_pos_next {
                break;
            }
            node_pos = node_pos_next;
        }

        if max_len >= LZ_MATCH_MIN_LEN && pos + max_len < buf.len() {
            return Match {
                reduced_offset: node_size_bounded_sub(bucket.head as u16, max_node_index as u16),
                match_len: max_len,
                match_len_expected: std::cmp::max(max_match_len_expected, LZ_MATCH_MIN_LEN),
                match_len_min: std::cmp::max(max_match_len_min, LZ_MATCH_MIN_LEN),
            };
        }
        Match::default()
    }

    pub unsafe fn has_lazy_match(
        &self,
        bucket: &Bucket,
        buf: &[u8],
        pos: usize,
        min_match_len: usize,
        depth: usize,
    ) -> bool {
        let max_len_dword = *(buf.as_ptr().add(pos + min_match_len - 4) as *const u32);
        let heads = &unchecked!(&self.heads);
        let nexts = &unchecked!(&self.nexts);
        let bucket_nodes = &unchecked!(&bucket.nodes);
        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = heads[entry];

        if node_index == -1 {
            return false;
        }
        let mut node_pos = bucket_nodes[node_index as usize].pos();

        for _ in 0..depth {
            if mem_fast_equal(
                buf.as_ptr(),
                node_pos as usize,
                pos,
                min_match_len,
                max_len_dword,
            ) {
                return true;
            };

            node_index = nexts[node_index as usize];
            if node_index == -1 {
                break;
            }

            let node_pos_next = bucket_nodes[node_index as usize].pos();
            if node_pos <= node_pos_next {
                break;
            }
            node_pos = node_pos_next;
        }
        false
    }
}

#[bitfield]
#[derive(Clone, Copy, Default)]
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

#[inline]
fn node_size_bounded_add(v1: u16, v2: u16) -> u16 {
    (v1 + v2) % LZ_MF_BUCKET_ITEM_SIZE as u16
}

#[inline]
fn node_size_bounded_sub(v1: u16, v2: u16) -> u16 {
    (v1 + LZ_MF_BUCKET_ITEM_SIZE as u16 - v2) % LZ_MF_BUCKET_ITEM_SIZE as u16
}

#[inline]
unsafe fn hash_dword(buf: &[u8], pos: usize) -> usize {
    gxhash::gxhash32(&buf.as_ptr().get::<[u8; 4]>(pos, 4), 0x9efa2b21) as usize
}
