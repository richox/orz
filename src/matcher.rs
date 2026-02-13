// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{hint::unlikely, simd::prelude::*};

use bitfield_struct::*;
use unchecked_index::UncheckedIndex;

use crate::{
    LZ_MATCH_MAX_LEN, LZ_MATCH_MIN_LEN, LZ_MF_BUCKET_ITEM_SIZE,
    mem::{BytesConstPtrExt, mem_fast_common_prefix, mem_fast_equal},
    unchecked,
};

const LZ_MF_BUCKET_ITEM_HASH_SIZE: usize = (LZ_MF_BUCKET_ITEM_SIZE as f64 * 1.13) as usize | 1;

#[derive(Default)] // Match::default = unmatched
pub struct Match {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

pub struct Bucket {
    nodes1: UncheckedIndex<[MatchPosAndLenMin; LZ_MF_BUCKET_ITEM_SIZE]>, /* pos:25 | match_len_min:7 */
    nodes2: UncheckedIndex<[MatchLenExpected; LZ_MF_BUCKET_ITEM_SIZE]>,  // match_len_expected:8
    head: usize,
    // match_len_expected:
    //  the match length we got when searching match for this position
    //  if no match is found, this value is set to 0.
    //
    //  when a newer position matches this position, it is likely that the match length
    //  is the same with this value.
    //
    // match_len_min:
    //  the longest match of all newer position that matches this position
    //  if no match is found, this value is set to LZ_MATCH_MIN_LEN-1.
    //
    //  when a newer position matches this position, the match length is always
    //  longer than this value, because shorter matches will stop at a newer position
    //  that matches this position.
    //
    //  A A A A A B B B B B A A A A A C C C C C A A A A A
    //  |<------------------|
    //  |                   match_len_expected=5
    //  match_len_min=6
}

impl Bucket {
    pub fn new() -> Self {
        Self {
            nodes1: unchecked!([MatchPosAndLenMin::default(); LZ_MF_BUCKET_ITEM_SIZE]),
            nodes2: unchecked!([MatchLenExpected::default(); LZ_MF_BUCKET_ITEM_SIZE]),
            head: 0,
        }
    }

    pub fn update(&mut self, pos: usize, reduced_offset: usize, match_len: usize) {
        let new_head = node_size_bounded_add(self.head, 1);

        // update match_len_min of matched position
        if match_len >= LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head, reduced_offset);
            if self.nodes1[node_index].match_len_min() <= match_len {
                self.nodes1[node_index].set_match_len_min((match_len + 1).min(127));
            }
        }

        // update match_len_expected of incomping position
        // match_len_expected < 128 because only 7 bits reserved
        self.nodes1[new_head] = MatchPosAndLenMin::new().with_pos(pos);
        self.nodes2[new_head] = MatchLenExpected::new().with_match_len_expected(match_len);

        // move head to next node
        self.head = new_head;
    }

    pub fn forward(&mut self, forward_len: usize) {
        // reduce all positions
        for node in &mut self.nodes1[..] {
            node.set_pos(node.pos().saturating_sub(forward_len));
        }
    }

    pub fn get_match_node_index(&self, reduced_offset: usize) -> usize {
        node_size_bounded_sub(self.head, reduced_offset)
    }

    pub fn get_match_pos_and_len_min(&self, node_index: usize) -> MatchPosAndLenMin {
        self.nodes1[node_index]
    }

    pub fn get_match_len_expected(&self, node_index: usize) -> MatchLenExpected {
        self.nodes2[node_index]
    }
}

pub struct BucketMatcher {
    heads: UncheckedIndex<[i16; LZ_MF_BUCKET_ITEM_HASH_SIZE]>,
    nexts: UncheckedIndex<[i16; LZ_MF_BUCKET_ITEM_SIZE]>,
}

impl BucketMatcher {
    pub fn new() -> Self {
        Self {
            heads: unchecked!([-1; LZ_MF_BUCKET_ITEM_HASH_SIZE]),
            nexts: unchecked!([-1; LZ_MF_BUCKET_ITEM_SIZE]),
        }
    }

    pub fn update(&mut self, bucket: &Bucket, buf: &[u8], pos: usize) {
        let head = bucket.head;
        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;

        self.nexts[head] = self.heads[entry];
        self.heads[entry] = bucket.head as i16;
    }

    pub fn forward(&mut self, bucket: &Bucket) {
        // clear all entries/positions that points to out-of-date node
        self.heads
            .iter_mut()
            .filter(|head| **head != -1 && bucket.nodes1[**head as usize].pos() == 0)
            .for_each(|head| *head = -1);
        self.nexts
            .iter_mut()
            .filter(|next| **next != -1 && bucket.nodes1[**next as usize].pos() == 0)
            .for_each(|next| *next = -1);
    }

    pub fn find_match(&self, bucket: &Bucket, buf: &[u8], pos: usize, match_depth: usize) -> Match {
        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads[entry] as usize;

        if node_index == usize::MAX {
            return Match::default();
        }
        let mut max_len = LZ_MATCH_MIN_LEN - 1;
        let mut max_match_len_min = LZ_MATCH_MIN_LEN;
        let mut max_match_len_expected = LZ_MATCH_MIN_LEN;
        let mut max_node_index = 0;
        let mut node_pos = bucket.nodes1[node_index].pos() as usize;
        let mut max_len_dword = buf.as_ptr().get::<u32>(pos + max_len - 3);

        for _ in 0..match_depth {
            let node_max_len_dword = buf.as_ptr().get::<u32>(node_pos + max_len - 3);
            // first check the last 4 bytes of longest match (likely to be unequal for a
            // failed match) then perform full LCP search
            if unlikely(node_max_len_dword == max_len_dword) {
                let lcp = mem_fast_common_prefix(buf.as_ptr(), node_pos, pos, LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_match_len_min = bucket.nodes1[node_index].match_len_min() as usize;
                    max_match_len_expected =
                        bucket.nodes2[node_index].match_len_expected() as usize;
                    max_len = lcp;
                    max_node_index = node_index;
                    max_len_dword = buf.as_ptr().get(pos + max_len - 3);
                }
                if lcp == LZ_MATCH_MAX_LEN {
                    break;
                }
                if max_match_len_expected > 0 && lcp > max_match_len_expected {
                    break;
                }
            }

            node_index = self.nexts[node_index] as usize;
            if node_index == usize::MAX {
                break;
            }

            let node_pos_next = bucket.nodes1[node_index].pos() as usize;
            if node_pos <= node_pos_next {
                break;
            }
            node_pos = node_pos_next;
        }

        if max_len >= LZ_MATCH_MIN_LEN && pos + max_len < buf.len() {
            return Match {
                reduced_offset: node_size_bounded_sub(bucket.head, max_node_index),
                match_len: max_len,
                match_len_expected: std::cmp::max(max_match_len_expected, LZ_MATCH_MIN_LEN),
                match_len_min: std::cmp::max(max_match_len_min, LZ_MATCH_MIN_LEN),
            };
        }
        Match::default()
    }

    pub fn has_lazy_match(
        &self,
        bucket: &Bucket,
        buf: &[u8],
        pos: usize,
        min_match_len: usize,
        depth: usize,
    ) -> bool {
        let max_len_dword = buf.as_ptr().get::<u32>(pos + min_match_len - 4);
        let entry = hash_dword(buf, pos) % LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads[entry] as usize;

        if node_index == usize::MAX {
            return false;
        }
        let mut node_pos = bucket.nodes1[node_index].pos();

        for _ in 0..depth {
            if mem_fast_equal(buf.as_ptr(), node_pos, pos, min_match_len, max_len_dword) {
                return true;
            };

            node_index = self.nexts[node_index] as usize;
            if node_index == usize::MAX {
                break;
            }

            let node_pos_next = bucket.nodes1[node_index].pos();
            if node_pos <= node_pos_next {
                break;
            }
            node_pos = node_pos_next;
        }
        false
    }
}

#[bitfield(u32)]
pub struct MatchPosAndLenMin {
    #[bits(25)]
    pub pos: usize,
    #[bits(7)]
    pub match_len_min: usize,
}

#[bitfield(u8)]
pub struct MatchLenExpected {
    #[bits(8)]
    pub match_len_expected: usize,
}

#[inline]
fn node_size_bounded_add(v1: usize, v2: usize) -> usize {
    (v1 + v2) % LZ_MF_BUCKET_ITEM_SIZE
}

#[inline]
fn node_size_bounded_sub(v1: usize, v2: usize) -> usize {
    (v1 + LZ_MF_BUCKET_ITEM_SIZE - v2) % LZ_MF_BUCKET_ITEM_SIZE
}

#[inline]
fn hash_dword(buf: &[u8], pos: usize) -> usize {
    // safety: buf[pos..][..4] must be valid
    const MULS: u32x4 = u32x4::from_array([131313131, 1313131, 13131, 131]);
    const ADDS: u32x4 = u32x4::from_array([797, 79797, 7979797, 797979797]);
    let bytes = buf.as_ptr().get::<u8x4>(pos);
    let h = bytes.cast() * MULS ^ ADDS;
    h.reduce_sum() as usize
}
