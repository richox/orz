use super::aux::UncheckedSliceExt;

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
            heads: [-1; super::LZ_MF_BUCKET_ITEM_HASH_SIZE],
            nexts: [-1; super::LZ_MF_BUCKET_ITEM_SIZE],
            items: [0;  super::LZ_MF_BUCKET_ITEM_SIZE],
            head: 0,
        }
    }

    pub fn forward(&mut self, forward_len: u32) {
        self.items.iter_mut().for_each(|item| *item = item.saturating_sub(forward_len));
    }

    pub unsafe fn find_match_and_update(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> Option<(u16, u8)> {
        let entry = hash_dword(buf, pos) as usize % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut match_result = None;
        let mut node = *self.heads.xget(entry);

        if node != -1 {
            // start matching
            let mut max_len = super::LZ_MATCH_MIN_LEN - 1;
            let mut max_node = 0;
            let mut max_len_dword = *((buf.as_ptr() as usize + pos) as *const u32);

            for _ in 0..match_depth {
                let node_pos = *self.items.xget(node) as usize;
                if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                    let lcp = {
                        let a = buf.as_ptr() as usize + node_pos;
                        let b = buf.as_ptr() as usize + pos;
                        let mut l = 0;

                        // keep max_len=255, so (l + 3 < max_len) is always true
                        while l + 4 <= super::LZ_MATCH_MAX_LEN && *((a + l) as *const u32) == *((b + l) as *const u32) {
                            l += 4;
                        }
                        l += (*((a + l) as *const u16) == *((b + l) as *const u16)) as usize * 2;
                        l += (*((a + l) as *const  u8) == *((b + l) as *const  u8)) as usize;
                        l
                    };

                    if max_len < lcp {
                        max_len = lcp;
                        max_node = node;
                        if max_len == super::LZ_MATCH_MAX_LEN {
                            break;
                        }
                        max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
                    }
                }

                node = *self.nexts.xget(node);
                if node == -1 || node_pos <= *self.items.xget(node) as usize {
                    break;
                }
            }

            if max_len >= super::LZ_MATCH_MIN_LEN {
                match_result = Some((item_size_bounded_sub(self.head, max_node) as u16, max_len as u8));
            }
        }
        let new_head = item_size_bounded_add(self.head, 1);
        self.nexts.xset(new_head, *self.heads.xget(entry));
        self.items.xset(new_head, pos as u32);
        self.heads.xset(entry, new_head as i16);
        self.head = new_head as i16;
        return match_result;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, max_len: usize, depth: usize) -> bool {
        let entry = hash_dword(buf, pos) as usize % super::LZ_MF_BUCKET_ITEM_HASH_SIZE;
        let mut node = *self.heads.xget(entry);

        if node == -1 {
            return false;
        }
        let max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
        for _ in 0..depth {
            let node_pos = *self.items.xget(node) as usize;
            if *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == max_len_dword {
                return true;
            };

            node = *self.nexts.xget(node);
            if node == -1 || node_pos <= *self.items.xget(node) as usize {
                break;
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
        self.items.xset(self.head, pos as u32);
    }

    pub unsafe fn get_match_pos(&self, reduced_offset: u16) -> usize {
        return *self.items.xget(item_size_bounded_sub(self.head, reduced_offset as i16)) as usize;
    }
}

unsafe fn hash_dword(buf: &[u8], pos: usize) -> u32 {
    (*buf.xget(pos + 0) as u32 * 131313131) +
    (*buf.xget(pos + 1) as u32 * 1313131) +
    (*buf.xget(pos + 2) as u32 * 13131 + *buf.xget(pos + 3) as u32)
}

fn item_size_bounded_add(v1: i16, v2: i16) -> i16 {
    (v1 + v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}

fn item_size_bounded_sub(v1: i16, v2: i16) -> i16 {
    (v1 + super::LZ_MF_BUCKET_ITEM_SIZE as i16 - v2) % super::LZ_MF_BUCKET_ITEM_SIZE as i16
}
