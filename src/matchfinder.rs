use super::auxility::ByteSliceExt;
use super::auxility::UncheckedSliceExt;

pub struct MatchResult {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

macro_rules! define_bucket_type {
    ($BucketType:ident, $head_len:expr, $next_len:expr) => {
        pub struct $BucketType {
            head: u16,
            node_part1: [u32; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:24 | match_len_expected:8
            node_part2: [u8;  super::LZ_MF_BUCKET_ITEM_SIZE], // match_len_min:8
            heads:      [u16; $head_len],
            nexts:      [u16; $next_len],
        }

        impl $BucketType {
            pub fn new() -> $BucketType {
                return $BucketType {
                    head: 0,
                    node_part1: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
                    node_part2: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
                    heads:      [super::LZ_MF_BUCKET_ITEM_SIZE as u16; $head_len],
                    nexts:      [super::LZ_MF_BUCKET_ITEM_SIZE as u16; $next_len],
                };
            }

            unsafe fn get_node_pos(&self, i: usize) -> usize {
                return self.node_part1.nc()[i] as usize & 0x00ff_ffff;
            }
            unsafe fn get_node_match_len_expected(&self, i: usize) -> usize {
                return self.node_part1.nc()[i] as usize >> 24;
            }
            unsafe fn get_node_match_len_min(&self, i: usize) -> usize {
                return self.node_part2.nc()[i] as usize;
            }

            unsafe fn set_node_pos(&mut self, i: usize, pos: usize) {
                self.node_part1.nc_mut()[i] = (pos | self.get_node_match_len_expected(i) << 24) as u32;
            }
            unsafe fn set_node(&mut self, i: usize, pos: usize, match_len_expected: usize, match_len_min: usize) {
                self.node_part1.nc_mut()[i] = (pos | match_len_expected << 24) as u32;
                self.node_part2.nc_mut()[i] = match_len_min as u8;
            }
            unsafe fn set_node_match_len_min(&mut self, i: usize, match_len_min: usize) {
                self.node_part2.nc_mut()[i] = match_len_min as u8;
            }

            pub unsafe fn update(&mut self, buf: &[u8], pos: usize, reduced_offset: usize, match_len: usize) {
                if match_len >= super::LZ_MATCH_MIN_LEN {
                    let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
                    if self.get_node_match_len_min(node_index) <= match_len {
                        self.set_node_match_len_min(node_index, match_len + 1);
                    }
                }
                self.head = node_size_bounded_add(self.head, 1) as u16;
                self.set_node(self.head as usize, pos, match_len, 0);
                if !self.nexts.is_empty() { // only for EncoderMFBucket
                    let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
                    self.nexts.nc_mut()[self.head as usize] = self.heads.nc()[entry];
                    self.heads.nc_mut()[entry] = self.head;
                }
            }

            pub fn forward(&mut self, forward_len: usize) {
                unsafe {
                    for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                        self.set_node_pos(i, self.get_node_pos(i).saturating_sub(forward_len));
                    }
                    for i in 0 .. $head_len { // only for EncoderMFBucket
                        let head = self.heads[i] as usize;
                        if head != super::LZ_MF_BUCKET_ITEM_SIZE && self.get_node_pos(head) == 0 {
                            self.heads[i] = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
                        }
                    }
                    for i in 0 .. $next_len { // only for EncoderMFBucket
                        let next = self.nexts[i] as usize;
                        if next != super::LZ_MF_BUCKET_ITEM_SIZE && self.get_node_pos(next) == 0 {
                            self.nexts[i] = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
                        }
                    }
                }
            }
        }
    }
}
define_bucket_type!(EncoderMFBucket, super::LZ_MF_BUCKET_ITEM_HASH_SIZE, super::LZ_MF_BUCKET_ITEM_SIZE);
define_bucket_type!(DecoderMFBucket, 0, 0);

impl EncoderMFBucket {
    pub unsafe fn find_match(&self, buf: &[u8], pos: usize, match_depth: usize) -> Option<MatchResult> {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nc()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return None;
        }
        let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
        let mut max_node_index = 0;

        for _ in 0..match_depth {
            let max_len_dword = buf.read(pos + max_len - 3);
            let node_pos = self.get_node_pos(node_index);

            if buf.read::<u32>(node_pos + max_len - 3) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_len = lcp;
                    max_node_index = node_index;
                    if lcp == super::LZ_MATCH_MAX_LEN || lcp < self.get_node_match_len_min(node_index) {
                        break;
                    }
                }
            }

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.get_node_pos(node_next) {
                break;
            }
            node_index = node_next;
        }

        if max_len >= super::LZ_MATCH_MIN_LEN && pos + max_len < buf.len() {
            return Some(MatchResult {
                reduced_offset: node_size_bounded_sub(self.head, max_node_index as u16) as usize,
                match_len: max_len,
                match_len_expected: std::cmp::max(self.get_node_match_len_expected(max_node_index), super::LZ_MATCH_MIN_LEN),
                match_len_min: std::cmp::max(self.get_node_match_len_min(max_node_index), super::LZ_MATCH_MIN_LEN),
            });
        }
        return None;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nc()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return false;
        }
        let max_len_dword = buf.read::<u32>(pos + min_match_len - 4);
        for _ in 0..depth {
            let node_pos = self.get_node_pos(node_index);
            if buf.read::<u32>(node_pos + min_match_len - 4) == max_len_dword {
                if super::mem::memeq_hack_fast(buf, node_pos, pos, min_match_len - 4) {
                    return true;
                }
            };

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.get_node_pos(node_next) {
                break;
            }
            node_index = node_next;
        }
        return false;
    }
}

impl DecoderMFBucket {
    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
        return (
            self.get_node_pos(node_index),
            std::cmp::max(self.get_node_match_len_expected(node_index), super::LZ_MATCH_MIN_LEN),
            std::cmp::max(self.get_node_match_len_min(node_index), super::LZ_MATCH_MIN_LEN),
        );
    }
}

fn node_size_bounded_add(v1: u16, v2: u16) -> u16 {
    return (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

fn node_size_bounded_sub(v1: u16, v2: u16) -> u16 {
    return (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as u16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> usize {
    let u32context = buf.read::<u32>(pos).to_be() as usize;
    return u32context * 131 + u32context / 131;
}
