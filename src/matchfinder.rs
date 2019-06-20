use byteorder::BE;
use byteorder::ByteOrder;
use super::auxility::ByteSliceExt;
use super::auxility::UncheckedSliceExt;

pub struct EncoderMFBucket {
    dec: DecoderMFBucket,
    heads: [u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [u16; super::LZ_MF_BUCKET_ITEM_SIZE],
}

pub struct DecoderMFBucket {
    head: u16,
    node_part1: [u32; super::LZ_MF_BUCKET_ITEM_SIZE], // pos:24 | match_len_expected:8
    node_part2: [u8;  super::LZ_MF_BUCKET_ITEM_SIZE], // match_len_min:8
}

pub struct MatchResult {
    pub reduced_offset: usize,
    pub match_len: usize,
    pub match_len_expected: usize,
    pub match_len_min: usize,
}

impl EncoderMFBucket {
    pub fn new() -> EncoderMFBucket {
        return EncoderMFBucket {
            dec: DecoderMFBucket::new(),
            heads: [super::LZ_MF_BUCKET_ITEM_SIZE as u16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [super::LZ_MF_BUCKET_ITEM_SIZE as u16; super::LZ_MF_BUCKET_ITEM_SIZE],
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.dec.forward(forward_len);
        for entry in self.heads.iter_mut() {
            if *entry != super::LZ_MF_BUCKET_ITEM_SIZE as u16 && unsafe {self.dec.get_node_pos(*entry as usize)} == 0 {
                *entry = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
            }
        }
        for next in self.nexts.iter_mut() {
            if *next != super::LZ_MF_BUCKET_ITEM_SIZE as u16 && unsafe {self.dec.get_node_pos(*next as usize)} == 0 {
                *next = super::LZ_MF_BUCKET_ITEM_SIZE as u16;
            }
        }
    }

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
            let node_pos = self.dec.get_node_pos(node_index);

            if buf.read::<u32>(node_pos + max_len - 3) == max_len_dword {
                let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                if lcp > max_len {
                    max_len = lcp;
                    max_node_index = node_index;
                    if lcp == super::LZ_MATCH_MAX_LEN || lcp < self.dec.get_node_match_len_min(node_index) {
                        break;
                    }
                }
            }

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.dec.get_node_pos(node_next) {
                break;
            }
            node_index = node_next;
        }

        if max_len >= super::LZ_MATCH_MIN_LEN {
            return Some(MatchResult {
                reduced_offset: node_size_bounded_sub(self.dec.head, max_node_index as u16) as usize,
                match_len: max_len,
                match_len_expected: self.dec.get_node_match_len_expected(max_node_index),
                match_len_min: self.dec.get_node_match_len_min(max_node_index),
            });
        }
        return None;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        if min_match_len > super::LZ_MATCH_MAX_LEN {
            return false;
        }
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node_index = self.heads.nc()[entry] as usize;

        if node_index == super::LZ_MF_BUCKET_ITEM_SIZE {
            return false;
        }
        let max_len_dword = buf.read::<u32>(pos + min_match_len - 4);
        for _ in 0..depth {
            let node_pos = self.dec.get_node_pos(node_index);
            if buf.read::<u32>(node_pos + min_match_len - 4) == max_len_dword {
                if super::mem::memeq_hack_fast(buf, node_pos, pos, min_match_len - 4) {
                    return true;
                }
            };

            let node_next = self.nexts.nc()[node_index] as usize;
            if node_next == super::LZ_MF_BUCKET_ITEM_SIZE || node_pos <= self.dec.get_node_pos(node_next) {
                break;
            }
            node_index = node_next;
        }
        return false;
    }

    pub unsafe fn update(&mut self, buf: &[u8], pos: usize, reduced_offset: usize, match_len: usize) {
        let entry = hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        self.dec.update(pos, reduced_offset, match_len);
        self.nexts.nc_mut()[self.dec.head as usize] = self.heads.nc()[entry];
        self.heads.nc_mut()[entry] = self.dec.head;
    }
}

impl DecoderMFBucket {
    pub fn new() -> DecoderMFBucket {
        return DecoderMFBucket {
            head: 0,
            node_part1: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            node_part2: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
        };
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

    pub fn forward(&mut self, forward_len: usize) {
        unsafe {
            for i in 0 .. super::LZ_MF_BUCKET_ITEM_SIZE {
                self.set_node_pos(i, self.get_node_pos(i).saturating_sub(forward_len));
            }
        }
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize, usize) {
        let node_index = node_size_bounded_sub(self.head, reduced_offset as u16) as usize;
        return (
            self.get_node_pos(node_index),
            self.get_node_match_len_expected(node_index),
            self.get_node_match_len_min(node_index),
        );
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
    unsafe fn set_node_pos_and_match_len_expected(&mut self, i: usize, pos: usize, match_len_expected: usize) {
        self.node_part1.nc_mut()[i] = (pos | match_len_expected << 24) as u32;
    }
    unsafe fn set_node_match_len_min(&mut self, i: usize, match_len_min: usize) {
        self.node_part2.nc_mut()[i] = match_len_min as u8;
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
