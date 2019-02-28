use super::aux::UncheckedSliceExt;
use byteorder::BE;
use byteorder::ReadBytesExt;

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
        self.items.iter_mut().for_each(|item| *item = item.saturating_sub(forward_len));
    }

    pub unsafe fn find_match_and_update(&mut self, buf: *const u8, pos: usize, match_depth: usize) -> Option<(u16, u8)> {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut match_result = None;
        let mut node = self.heads.nocheck()[entry] as usize;

        if node != 0 {
            // start matching
            let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
            let mut max_node = 0;
            let mut max_len_dword = *((buf as usize + pos) as *const u32);

            for _ in 0..match_depth {
                let node_pos = self.items.nocheck()[node] as usize;
                if *((buf as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                    let lcp = get_lcp(
                        buf.offset(node_pos as isize),
                        buf.offset(pos as isize),
                        super::LZ_MATCH_MAX_LEN);
                    if lcp > max_len {
                        max_len = lcp;
                        max_node = node;
                        if max_len == super::LZ_MATCH_MAX_LEN {
                            break;
                        }
                        max_len_dword = *((buf as usize + pos + max_len - 3) as *const u32);
                    }
                }

                node = self.nexts.nocheck()[node] as usize;
                if node == 0 || node_pos <= self.items.nocheck()[node] as usize {
                    break;
                }
            }

            if max_len >= super::LZ_MATCH_MIN_LEN {
                match_result = Some((item_size_bounded_sub(self.head, max_node as i16) as u16, max_len as u8));
            }
        }
        let new_head = item_size_bounded_add(self.head, 1);
        self.nexts.nocheck_mut()[new_head as usize] = self.heads.nocheck()[entry];
        self.items.nocheck_mut()[new_head as usize] = pos as u32;
        self.heads.nocheck_mut()[entry] = new_head as i16;
        self.head = new_head as i16;
        return match_result;
    }

    pub unsafe fn has_lazy_match(&self, buf: *const u8, pos: usize, min_match_len: usize, depth: usize) -> bool {
        let entry = (hash_dword(buf, pos) % super::LZ_MF_BUCKET_ITEM_HASH_SIZE as u32) as usize;
        let mut node = self.heads.nocheck()[entry] as usize;

        if node != 0 {
            let max_len_dword = *((buf as usize + pos + min_match_len - 4) as *const u32);
            for _ in 0..depth {
                let node_pos = self.items.nocheck()[node] as usize;
                if *((buf as usize + node_pos + min_match_len - 4) as *const u32) == max_len_dword {
                    let lcp = get_lcp(buf.offset(node_pos as isize), buf.offset(pos as isize), min_match_len - 4);
                    if lcp >= min_match_len - 4 {
                        return true;
                    }
                };

                node = self.nexts.nocheck()[node] as usize;
                if node == 0 || node_pos <= self.items.nocheck()[node] as usize {
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
        self.items.iter_mut().for_each(|item| *item = item.saturating_sub(forward_len));
    }

    pub unsafe fn update(&mut self, pos: usize) {
        self.head = item_size_bounded_add(self.head, 1);
        self.items.nocheck_mut()[self.head as usize] = pos as u32;
    }

    pub unsafe fn get_match_pos(&self, reduced_offset: u16) -> usize {
        return self.items.nocheck()[item_size_bounded_sub(self.head, reduced_offset as i16) as usize] as usize;
    }
}

unsafe fn hash_dword(buf: *const u8, pos: usize) -> u32 {
    let u32context = std::slice::from_raw_parts(buf.offset(pos as isize), 4).read_u32::<BE>().unwrap();
    return u32context * 131 + u32context / 131;
}

fn item_size_bounded_add(v1: i16, v2: i16) -> i16 {
    (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

fn item_size_bounded_sub(v1: i16, v2: i16) -> i16 {
    (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as i16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

unsafe fn get_lcp(p1: *const u8, p2: *const u8, max_len: usize) -> usize {
    let p1 = p1 as usize;
    let p2 = p2 as usize;
    let mut l = 0;

    // keep max_len=255, so (l + 3 < max_len) is always true
    while l + 4 <= max_len && *((p1 + l) as *const u32) == *((p2 + l) as *const u32) {
        l += 4;
    }
    l += (*((p1 + l) as *const u16) == *((p2 + l) as *const u16)) as usize * 2;
    l += (*((p1 + l) as *const  u8) == *((p2 + l) as *const  u8)) as usize;
    l
}
