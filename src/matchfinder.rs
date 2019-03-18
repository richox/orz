use super::aux::UncheckedSliceExt;

// items:
//  [0..25)  pos
//  [25..32) match len at pos
//  requires: pos < 2^25 and match_len < 2^7

pub struct EncoderMFBucket {
    heads: [i16; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
    nexts: [i16; super::LZ_MF_BUCKET_ITEM_SIZE],
    items: [u32; super::LZ_MF_BUCKET_ITEM_SIZE],
    head: i16,
}

pub struct DecoderMFBucket {
    items: [u32; super::LZ_MF_BUCKET_ITEM_SIZE],
    head: i16,
}

pub struct MatchResult {
    pub reduced_offset: u16,
    pub match_len: u8,
    pub match_len_at_pos: u8,
}

impl EncoderMFBucket {
    pub fn new() -> EncoderMFBucket {
        EncoderMFBucket {
            heads: [0; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            items: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub fn forward(&mut self, forward_len: u32) {
        self.items.iter_mut().for_each(
            |item| *item = *item & 0xfe000000 | (*item & 0x01ffffff).saturating_sub(forward_len));
    }

    pub unsafe fn find_match_and_update(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> Option<MatchResult> {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut match_result = None;
        let mut node = self.heads.nocheck()[entry] as usize;

        if node != 0 {
            // start matching
            let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
            let mut max_node = 0;
            let mut max_len_dword = *((buf.as_ptr() as usize + pos) as *const u32);
            let mut max_match_len_at_node_pos = 4;

            for _ in 0..match_depth {
                let node_pos = self.items.nocheck()[node] as usize & 0x01ffffff;
                let match_len_at_node_pos = self.items.nocheck()[node] as usize >> 25;

                if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                    let lcp = super::mem::llcp_fast(buf, node_pos, pos, super::LZ_MATCH_MAX_LEN);
                    let (lcp_fix, max_len_fix) = (
                        lcp + (lcp == match_len_at_node_pos) as usize,
                        max_len + (max_len == max_match_len_at_node_pos) as usize,
                    );
                    if lcp_fix > max_len_fix {
                        max_len = lcp;
                        max_node = node;
                        max_match_len_at_node_pos = match_len_at_node_pos;

                        if max_len == super::LZ_MATCH_MAX_LEN {
                            break;
                        }
                        max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
                    }
                }

                node = self.nexts.nocheck()[node] as usize;
                if node == 0 || node_pos <= self.items.nocheck()[node] as usize & 0x01ffffff {
                    break;
                }
            }

            if max_len >= super::LZ_MATCH_MIN_LEN {
                match_result = Some(MatchResult {
                    reduced_offset: item_size_bounded_sub(self.head, max_node as i16) as u16,
                    match_len: max_len as u8,
                    match_len_at_pos: max_match_len_at_node_pos as u8,
                });
            }
        }
        return match_result;
    }

    pub unsafe fn update(&mut self, buf: &[u8], pos: usize, match_len: usize) {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let new_head = item_size_bounded_add(self.head, 1);
        self.nexts.nocheck_mut()[new_head as usize] = self.heads.nocheck()[entry];
        self.items.nocheck_mut()[new_head as usize] = pos as u32 | (match_len << 25) as u32;
        self.heads.nocheck_mut()[entry] = new_head as i16;
        self.head = new_head as i16;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, min_match_len: usize, depth: usize) -> bool {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut node = self.heads.nocheck()[entry] as usize;

        if node != 0 {
            let max_len_dword = *((buf.as_ptr() as usize + pos + min_match_len - 4) as *const u32);
            for _ in 0..depth {
                let node_pos = self.items.nocheck()[node] as usize & 0x01ffffff;
                if *((buf.as_ptr() as usize + node_pos + min_match_len - 4) as *const u32) == max_len_dword {
                    let lcp = super::mem::llcp_fast(buf, node_pos, pos, min_match_len - 4);
                    if lcp >= min_match_len - 4 {
                        return true;
                    }
                };

                node = self.nexts.nocheck()[node] as usize;
                if node == 0 || node_pos <= self.items.nocheck()[node] as usize & 0x01ffffff {
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
            items: [0; super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub fn forward(&mut self, forward_len: u32) {
        self.items.iter_mut().for_each(|item| *item = *item & 0xfe000000 | (*item & 0x01ffffff).saturating_sub(forward_len));
    }

    pub unsafe fn update(&mut self, pos: usize, match_len: usize) {
        self.head = item_size_bounded_add(self.head, 1);
        self.items.nocheck_mut()[self.head as usize] = pos as u32 | (match_len as u32) << 25;
    }

    pub unsafe fn get_match_pos_and_match_len(&self, reduced_offset: u16) -> (usize, usize) {
        let node = self.items.nocheck()[item_size_bounded_sub(self.head, reduced_offset as i16) as usize];
        return (
            node as usize & 0x01ffffff,
            node as usize >> 25,
        );
    }
}

fn item_size_bounded_add(v1: i16, v2: i16) -> i16 {
    (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

fn item_size_bounded_sub(v1: i16, v2: i16) -> i16 {
    (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as i16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> u32 {
    let u32context =
        (buf.nocheck()[pos + 0] as u32) << 24 |
        (buf.nocheck()[pos + 1] as u32) << 16 |
        (buf.nocheck()[pos + 2] as u32) <<  8 |
        (buf.nocheck()[pos + 3] as u32) <<  0;
    return u32context * 131 + u32context / 131;
}
