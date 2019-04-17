use byteorder::BE;
use byteorder::ByteOrder;
use super::auxility::UncheckedSliceExt;

pub struct EncoderMFBucket {
    heads: [u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    node_part1: [u32; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:24 | match_len_expected:8
    node_part2: [u8;  super::LZ_MF_BUCKET_ITEM_SIZE], // match_len_min:8
    node_part3: [u16; super::LZ_MF_BUCKET_ITEM_SIZE], // next: 14
    head: u16,
}

pub struct DecoderMFBucket {
    node_part1: [u32; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:24 | match_len_expected:8
    node_part2: [u8;  super::LZ_MF_BUCKET_ITEM_SIZE], // match_len_min:8
    head: u16,
}

pub struct MatchResult {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

impl EncoderMFBucket {
    unsafe fn get_node_pos(&self, i: usize) -> usize {
        return self.node_part1.nocheck()[i] as usize & 0x00ff_ffff;
    }
    unsafe fn get_node_match_len_expected(&self, i: usize) -> usize {
        return self.node_part1.nocheck()[i] as usize >> 24;
    }
    unsafe fn get_node_match_len_min(&self, i: usize) -> usize {
        return self.node_part2.nocheck()[i] as usize;
    }
    unsafe fn get_node_next(&self, i: usize) -> usize {
        return self.node_part3.nocheck()[i] as usize;
    }

    unsafe fn set_node_pos(&mut self, i: usize, pos: usize) {
        self.node_part1.nocheck_mut()[i] = (pos | self.get_node_match_len_expected(i) << 24) as u32;
    }
    unsafe fn set_node_pos_and_match_len_expected(&mut self, i: usize, pos: usize, match_len_expected: usize) {
        self.node_part1.nocheck_mut()[i] = (pos | match_len_expected << 24) as u32;
    }
    unsafe fn set_node_match_len_min(&mut self, i: usize, match_len_min: usize) {
        self.node_part2.nocheck_mut()[i] = match_len_min as u8;
    }
    unsafe fn set_node_next(&mut self, i: usize, next: usize) {
        self.node_part3.nocheck_mut()[i] = next as u16;
    }

    pub fn new() -> EncoderMFBucket {
        return EncoderMFBucket {
            heads:      [super::LZ_MF_BUCKET_ITEM_SIZE as u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            node_part1: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            node_part2: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            node_part3: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        unsafe {
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                let node_pos = self.get_node_pos(i);
                self.set_node_pos(i, node_pos.saturating_sub(forward_len));
            }
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                let next = self.get_node_next(i);
                if next != super::LZ_MF_BUCKET_ITEM_SIZE && self.get_node_pos(next as usize) == 0 {
                    self.set_node_next(i, super::LZ_MF_BUCKET_ITEM_SIZE);
                }
            }
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_HASH_SIZE {
                let head = self.heads.nocheck()[i];
                if head != super::LZ_MF_BUCKET_ITEM_SIZE as u16 && self.get_node_pos(head as usize) == 0 {
                    self.heads.nocheck_mut()[i] = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
                }
            }
        }
    }

    pub unsafe fn find_match(&self, buf: &[u8], pos: usize, match_depth: usize) -> Option<MatchResult> {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nocheck()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return None;
        }
        let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
        let mut max_node_index = 0;
        let mut max_len_dword = *((buf.as_ptr() as usize + pos) as *const u32);

        for _ in 0..match_depth {
            let node_pos = self.get_node_pos(node_index);

            if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_len = lcp;
                    max_node_index = node_index;
                    if lcp == super::LZ_MATCH_MAX_LEN || lcp < self.get_node_match_len_min(node_index) {
                        break;
                    }
                    max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
                }
            }

            let node_next = self.get_node_next(node_index);
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.get_node_pos(node_next as usize) {
                break;
            }
            node_index = node_next as usize;
        }

        if max_len >= super::LZ_MATCH_MIN_LEN {
            return Some(MatchResult {
                reduced_offset: node_size_bounded_sub(self.head, max_node_index as u16) as usize,
                match_len: max_len,
                match_len_expected: self.get_node_match_len_expected(max_node_index),
                match_len_min: self.get_node_match_len_min(max_node_index),
            });
        }
        return None;
    }

    pub unsafe fn update(&mut self, buf: &[u8], pos: usize, reduced_offset: usize, match_len: usize) {
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
            if self.get_node_match_len_min(node_index) <= match_len {
                self.set_node_match_len_min(node_index, match_len + 1);
            }
        }
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let entry_node_index = self.heads.nocheck()[entry] as usize;

        let new_head = node_size_bounded_add(self.head, 1) as usize;
        self.set_node_next(new_head, entry_node_index);
        self.set_node_pos_and_match_len_expected(new_head, pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
        self.set_node_match_len_min(new_head, super::LZ_MATCH_MIN_LEN);
        self.head = new_head as u16;
        self.heads.nocheck_mut()[entry] = self.head as u16;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        if min_match_len > super::LZ_MATCH_MAX_LEN {
            return false;
        }
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nocheck()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return false;
        }
        let max_len_dword = *((buf.as_ptr() as usize + pos + min_match_len - 4) as *const u32);
        for _ in 0..depth {
            let node_pos = self.get_node_pos(node_index);
            if *((buf.as_ptr() as usize + node_pos + min_match_len - 4) as *const u32) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, min_match_len - 4);
                if lcp >= min_match_len - 4 {
                    return true;
                }
            };

            let node_next = self.get_node_next(node_index);
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.get_node_pos(node_next as usize) {
                break;
            }
            node_index = node_next as usize;
        }
        return false;
    }
}

impl DecoderMFBucket {
    unsafe fn get_node_pos(&self, i: usize) -> usize {
        return self.node_part1.nocheck()[i] as usize & 0x00ff_ffff;
    }
    unsafe fn get_node_match_len_expected(&self, i: usize) -> usize {
        return self.node_part1.nocheck()[i] as usize >> 24;
    }
    unsafe fn get_node_match_len_min(&self, i: usize) -> usize {
        return self.node_part2.nocheck()[i] as usize;
    }

    unsafe fn set_node_pos(&mut self, i: usize, pos: usize) {
        self.node_part1.nocheck_mut()[i] = (pos | self.get_node_match_len_expected(i) << 24) as u32;
    }
    unsafe fn set_node_pos_and_match_len_expected(&mut self, i: usize, pos: usize, match_len_expected: usize) {
        self.node_part1.nocheck_mut()[i] = (pos | match_len_expected << 24) as u32;
    }
    unsafe fn set_node_match_len_min(&mut self, i: usize, match_len_min: usize) {
        self.node_part2.nocheck_mut()[i] = match_len_min as u8;
    }

    pub fn new() -> DecoderMFBucket {
        return DecoderMFBucket {
            node_part1: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            node_part2: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        unsafe {
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                let node_pos = self.get_node_pos(i);
                self.set_node_pos(i, node_pos.saturating_sub(forward_len));
            }
        }
    }

    pub unsafe fn update(&mut self, pos: usize, reduced_offset: usize, match_len: usize) {
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
            if self.get_node_match_len_min(node_index) <= match_len {
                self.set_node_match_len_min(node_index, match_len + 1);
            }
        }
        let new_head = node_size_bounded_add(self.head, 1) as usize;
        self.set_node_pos_and_match_len_expected(new_head, pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
        self.set_node_match_len_min(new_head, super::LZ_MATCH_MIN_LEN);
        self.head = new_head as u16;
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
        return (
            self.get_node_pos(node_index),
            self.get_node_match_len_expected(node_index),
            self.get_node_match_len_min(node_index),
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
    let u32context= BE::read_u32(std::slice::from_raw_parts(buf.get_unchecked(pos), 4)) as usize;
    return u32context * 131 + u32context / 131;
}
