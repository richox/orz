use super::auxility::ByteSliceExt;
use super::auxility::UncheckedSliceExt;

pub enum MatchResult {
    Unmatched,
    Matched {
        reduced_offset: u16,
        match_len: usize,
        match_len_expected: usize,
        match_len_min: usize,
    }
}

pub struct Bucket {
    head: u16,
    node_part1: [u32; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:25 | match_len_expected:7
    node_part2: [u8;  super::LZ_MF_BUCKET_ITEM_SIZE], // match_len_min:8

} impl Bucket {
    pub fn new() -> Bucket {
        return Bucket {
            head: 0,
            node_part1: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            node_part2: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
        };
    }

    unsafe fn get_node_pos(&self, i: usize) -> usize {
        return self.node_part1.nc()[i] as usize & 0x01ff_ffff;
    }
    unsafe fn get_node_match_len_expected(&self, i: usize) -> usize {
        return self.node_part1.nc()[i] as usize >> 25;
    }
    unsafe fn get_node_match_len_min(&self, i: usize) -> usize {
        return self.node_part2.nc()[i] as usize;
    }

    unsafe fn set_node(&mut self, i: usize, pos: usize, match_len_expected: usize, match_len_min: usize) {
        self.node_part1.nc_mut()[i] = (pos | match_len_expected << 25) as u32;
        self.node_part2.nc_mut()[i] = match_len_min as u8;
    }
    unsafe fn set_node_match_len_min(&mut self, i: usize, match_len_min: usize) {
        self.node_part2.nc_mut()[i] = match_len_min as u8;
    }

    pub unsafe fn update(&mut self, pos: usize, reduced_offset: u16, match_len: usize) {
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head, reduced_offset) as usize;
            if self.get_node_match_len_min(node_index) <= match_len {
                self.set_node_match_len_min(node_index, match_len + 1);
            }
        }
        let new_head = node_size_bounded_add(self.head, 1) as usize;
        let match_len_expected = match match_len { // match_len_expected < 128 because only 7 bits reserved
            0 ..= 127 => match_len,
            _ => 0,
        };
        self.set_node(new_head, pos, match_len_expected, 0);
        self.head = new_head as u16;
    }

    pub fn forward(&mut self, forward_len: usize) {
        unsafe {
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                self.set_node(i, self.get_node_pos(i).saturating_sub(forward_len),
                self.get_node_match_len_expected(i),
                self.get_node_match_len_min(i));
            }
        }
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.head, reduced_offset) as usize;
        return (
            self.get_node_pos(node_index),
            std::cmp::max(self.get_node_match_len_expected(node_index), super::LZ_MATCH_MIN_LEN),
            std::cmp::max(self.get_node_match_len_min(node_index), super::LZ_MATCH_MIN_LEN),
        );
    }
}

pub struct BucketMatcher {
    heads: [u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [u16; super::LZ_MF_BUCKET_ITEM_SIZE],

} impl BucketMatcher {
    pub fn new() -> BucketMatcher {
        return BucketMatcher {
            heads: [65535; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [65535; super::LZ_MF_BUCKET_ITEM_SIZE],
        };
    }

    pub unsafe fn update(&mut self, bucket: &Bucket, buf: &[u8], pos: usize) {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        self.nexts.nc_mut()[bucket.head as usize] = self.heads.nc()[entry];
        self.heads.nc_mut()[entry] = bucket.head;
    }

    pub fn forward(&mut self, bucket: &Bucket) {
        unsafe {
            self.heads.iter_mut()
                .filter(|head| **head != 65535 && bucket.get_node_pos(**head as usize) == 0)
                .for_each(|head| *head = 65535);
            self.nexts.iter_mut()
                .filter(|next| **next != 65535 && bucket.get_node_pos(**next as usize) == 0)
                .for_each(|next| *next = 65535);
        }
    }

    pub unsafe fn find_match(&self, bucket: &Bucket, buf: &[u8], pos: usize, match_depth: usize) -> MatchResult {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nc()[entry] as usize;

        if node_index == 65535 {
            return MatchResult::Unmatched;
        }
        let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
        let mut max_node_index = 0;
        let mut max_len_dword = buf.read(pos + max_len - 3);
        let mut max_match_len_min = 0;
        let mut max_match_len_expected = 0;

        for _ in 0..match_depth {
            let node_pos = bucket.get_node_pos(node_index);
            if buf.read::<u32>(node_pos + max_len - 3) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_match_len_min = bucket.get_node_match_len_min(node_index);
                    max_match_len_expected = bucket.get_node_match_len_expected(node_index);
                    max_len = lcp;
                    max_node_index = node_index;
                    max_len_dword = buf.read(pos + max_len - 3);
                }
                if lcp == super::LZ_MATCH_MAX_LEN
                        || (lcp > max_match_len_expected && max_match_len_expected > 0)
                        || (lcp < max_match_len_min) {
                    break;
                }
            }

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == 65535 || node_pos <= bucket.get_node_pos(node_next) {
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
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nc()[entry] as usize;

        if node_index == 65535 {
            return false;
        }
        let max_len_dword = buf.read::<u32>(pos + min_match_len - 4);
        for _ in 0..depth {
            let node_pos = bucket.get_node_pos(node_index);
            if buf.read::<u32>(node_pos + min_match_len - 4) == max_len_dword {
                if super::mem::memequ_hack_fast(buf, node_pos, pos, min_match_len - 4) {
                    return true;
                }
            };

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == 65535 || node_pos <= bucket.get_node_pos(node_next) {
                break;
            }
            node_index = node_next;
        }
        return false;
    }
}

fn node_size_bounded_add(v1: u16, v2: u16) -> u16 {
    return (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

fn node_size_bounded_sub(v1: u16, v2: u16) -> u16 {
    return (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as u16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as u16;
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> usize {
    let u32context = buf.read::<u32>(pos).to_le() as usize;
    return u32context * 13131 + u32context / 13131;
}
