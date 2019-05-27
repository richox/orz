use byteorder::BE;
use byteorder::ByteOrder;
use super::auxility::UncheckedSliceExt;

pub struct EncoderMFBucket {
    entries: [u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    buf: EncBuf,
}

pub struct DecoderMFBucket {
    buf: DecBuf,
}

pub struct MatchResult {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

macro_rules! define_bucket_type {
    ($BucketType:ident, $node_len:expr, $next_len:expr) => {
        struct $BucketType {
            head: u16,
            node_part1: [u32; $node_len], // pos:24 | match_len_expected:8
            node_part2: [u8;  $node_len], // match_len_min:8
            node_part3: [u16; $next_len], // next: 14
        }

        #[allow(dead_code)]
        impl $BucketType {
            fn new() -> $BucketType {
                return $BucketType {
                    head: 0,
                    node_part1: [0; $node_len],
                    node_part2: [0; $node_len],
                    node_part3: [0; $next_len],
                };
            }
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

            pub fn forward(&mut self, forward_len: usize) {
                unsafe {
                    for i in 0 .. $node_len {
                        let node_pos = self.get_node_pos(i);
                        self.set_node_pos(i, node_pos.saturating_sub(forward_len));
                    }
                    for i in 0 .. $next_len {
                        let next = self.get_node_next(i);
                        if next != $next_len && self.get_node_pos(next as usize) == 0 {
                            self.set_node_next(i, $next_len);
                        }
                    }
                }
            }
        }
    }
}
define_bucket_type!(EncBuf, super::LZ_MF_BUCKET_ITEM_SIZE, super::LZ_MF_BUCKET_ITEM_SIZE);
define_bucket_type!(DecBuf, super::LZ_MF_BUCKET_ITEM_SIZE, 0);

impl EncoderMFBucket {
    pub fn new() -> EncoderMFBucket {
        return EncoderMFBucket {
            entries: [super::LZ_MF_BUCKET_ITEM_SIZE as u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            buf: EncBuf::new(),
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.buf.forward(forward_len);
        unsafe {
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_HASH_SIZE {
                let head = self.entries.nocheck()[i];
                if head != super::LZ_MF_BUCKET_ITEM_SIZE as u16 && self.buf.get_node_pos(head as usize) == 0 {
                    self.entries.nocheck_mut()[i] = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
                }
            }
        }
    }

    pub unsafe fn find_match(&self, buf: &[u8], pos: usize, match_depth: usize) -> Option<MatchResult> {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.entries.nocheck()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return None;
        }
        let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
        let mut max_node_index = 0;

        for _ in 0..match_depth {
            let max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
            let node_pos = self.buf.get_node_pos(node_index);

            if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_len = lcp;
                    max_node_index = node_index;
                    if lcp == super::LZ_MATCH_MAX_LEN || lcp < self.buf.get_node_match_len_min(node_index) {
                        break;
                    }
                }
            }

            let node_next = self.buf.get_node_next(node_index);
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.buf.get_node_pos(node_next as usize) {
                break;
            }
            node_index = node_next as usize;
        }

        if max_len >= super::LZ_MATCH_MIN_LEN {
            return Some(MatchResult {
                reduced_offset: node_size_bounded_sub(self.buf.head, max_node_index as u16) as usize,
                match_len: max_len,
                match_len_expected: self.buf.get_node_match_len_expected(max_node_index),
                match_len_min: self.buf.get_node_match_len_min(max_node_index),
            });
        }
        return None;
    }

    pub unsafe fn update(&mut self, buf: &[u8], pos: usize, reduced_offset: usize, match_len: usize) {
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.buf.head, reduced_offset as u16) as usize;
            if self.buf.get_node_match_len_min(node_index) <= match_len {
                self.buf.set_node_match_len_min(node_index, match_len + 1);
            }
        }
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let entry_node_index = self.entries.nocheck()[entry] as usize;

        let new_head = node_size_bounded_add(self.buf.head, 1) as usize;
        self.buf.set_node_next(new_head, entry_node_index);
        self.buf.set_node_pos_and_match_len_expected(new_head, pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
        self.buf.set_node_match_len_min(new_head, super::LZ_MATCH_MIN_LEN);
        self.buf.head = new_head as u16;
        self.entries.nocheck_mut()[entry] = self.buf.head as u16;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        if min_match_len > super::LZ_MATCH_MAX_LEN {
            return false;
        }
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.entries.nocheck()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return false;
        }
        let max_len_dword = *((buf.as_ptr() as usize + pos + min_match_len - 4) as *const u32);
        for _ in 0..depth {
            let node_pos = self.buf.get_node_pos(node_index);
            if *((buf.as_ptr() as usize + node_pos + min_match_len - 4) as *const u32) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, min_match_len - 4);
                if lcp >= min_match_len - 4 {
                    return true;
                }
            };

            let node_next = self.buf.get_node_next(node_index);
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.buf.get_node_pos(node_next as usize) {
                break;
            }
            node_index = node_next as usize;
        }
        return false;
    }
}

impl DecoderMFBucket {
    pub fn new() -> DecoderMFBucket {
        return DecoderMFBucket {buf: DecBuf::new()};
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.buf.forward(forward_len);
    }

    pub unsafe fn update(&mut self, pos: usize, reduced_offset: usize, match_len: usize) {
        if match_len >= super::LZ_MATCH_MIN_LEN {
            let node_index = node_size_bounded_sub(self.buf.head, reduced_offset as u16) as usize;
            if self.buf.get_node_match_len_min(node_index) <= match_len {
                self.buf.set_node_match_len_min(node_index, match_len + 1);
            }
        }
        let new_head = node_size_bounded_add(self.buf.head, 1) as usize;
        self.buf.set_node_pos_and_match_len_expected(new_head, pos, std::cmp::max(match_len, super::LZ_MATCH_MIN_LEN));
        self.buf.set_node_match_len_min(new_head, super::LZ_MATCH_MIN_LEN);
        self.buf.head = new_head as u16;
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.buf.head, reduced_offset as u16) as usize;
        return (
            self.buf.get_node_pos(node_index),
            self.buf.get_node_match_len_expected(node_index),
            self.buf.get_node_match_len_min(node_index),
        );
    }
}

fn node_size_bounded_add(v1: u16, v2: u16) -> u16 {
    if v1 + v2 < super::LZ_MF_BUCKET_ITEM_SIZE as u16 {
        return v1 + v2;
    } else {
        return v1 + v2 - super::LZ_MF_BUCKET_ITEM_SIZE as u16;
    }
}

fn node_size_bounded_sub(v1: u16, v2: u16) -> u16 {
    if v1 - v2 < super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u16 {
        return v1 - v2;
    } else {
        return v1 - v2 + super::LZ_MF_BUCKET_ITEM_SIZE as u16;
    }
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> usize {
    let u32context = BE::read_u32(std::slice::from_raw_parts(buf.get_unchecked(pos), 4)) as usize;
    return u32context * 131 + u32context / 131;
}
