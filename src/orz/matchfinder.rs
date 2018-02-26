use orz::constants::lempziv_constants::*;

#[derive(Copy, Clone)]
#[repr(packed)]
pub struct MatchItem {
    raw1: u16,
    raw2: u8,
}

pub struct EncoderMFBucket {
    heads: [i16; LZ_BUCKET_ITEM_HASH_SIZE],
    nexts: [i16; LZ_BUCKET_ITEM_SIZE],
    items: [u32; LZ_BUCKET_ITEM_SIZE],
    ring_head: i16,
}

pub struct DecoderMFBucket {
    items: [u32; LZ_BUCKET_ITEM_SIZE],
    ring_head: i16,
}

impl MatchItem {
    pub fn new_match(index: u16, len: u8) -> MatchItem {
        MatchItem {
            raw1: index,
            raw2: len,
        }
    }

    pub fn new_literal(symbol: u8) -> MatchItem {
        MatchItem {
            raw1: 0x8000,
            raw2: symbol,
        }
    }

    pub fn get_match_or_literal(&self) -> u8 {
        (self.raw1 == 0x8000) as u8
    }

    pub fn get_match_index(&self) -> u16 {
        self.raw1
    }

    pub fn get_match_len(&self) -> usize {
        self.raw2 as usize
    }

    pub fn get_literal(&self) -> u8 {
        self.raw2
    }
}

macro_rules! compute_hash_on_first_4bytes {
    ($buf:expr, $pos:expr) => {{
        *$buf.get_unchecked(($pos + 0) as usize) as u32 * 1333337 +
        *$buf.get_unchecked(($pos + 1) as usize) as u32 * 13337 +
        *$buf.get_unchecked(($pos + 2) as usize) as u32 * 137 +
        *$buf.get_unchecked(($pos + 3) as usize) as u32 * 1
    }}
}

macro_rules! ring_add {
    ($a:expr, $b:expr, $ring_size:expr) => {{
        ($a as usize + $b as usize) % $ring_size as usize
    }}
}

macro_rules! ring_sub {
    ($a:expr, $b:expr, $ring_size:expr) => {{
        ($a as usize + $ring_size as usize - $b as usize) % $ring_size as usize
    }}
}

impl EncoderMFBucket {
    pub fn new() -> EncoderMFBucket {
        EncoderMFBucket {
            heads: [-1; LZ_BUCKET_ITEM_HASH_SIZE],
            nexts: [-1; LZ_BUCKET_ITEM_SIZE],
            items: [0; LZ_BUCKET_ITEM_SIZE],
            ring_head: 0,
        }
    }

    pub fn reset(&mut self) {
        self.nexts.iter_mut().for_each(|v| *v = -1);
        self.heads.iter_mut().for_each(|v| *v = -1);
        self.items.iter_mut().for_each(|v| *v = 0);
        self.ring_head = 0;
    }

    pub unsafe fn update_and_match(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> MatchItem {
        if pos + LZ_MATCH_MAX_LEN + 8 >= buf.len() {
            return MatchItem::new_literal(*buf.get_unchecked(pos));
        }
        let mut max_len = LZ_MATCH_MIN_LEN - 1;
        let mut max_node = 0;

        let entry = compute_hash_on_first_4bytes!(&buf, pos) as usize % LZ_BUCKET_ITEM_HASH_SIZE;
        let mut node = *self.heads.get_unchecked(entry);

        // update befault matching (to make it faster)
        self.ring_head = ring_add!(self.ring_head, 1, LZ_BUCKET_ITEM_SIZE) as i16;
        *self.nexts.get_unchecked_mut(self.ring_head as usize) = *self.heads.get_unchecked(entry);
        *self.items.get_unchecked_mut(self.ring_head as usize) = pos as u32 | (*buf.get_unchecked(pos) as u32) << 24;
        *self.heads.get_unchecked_mut(entry) = self.ring_head as i16;

        // empty node
        if node == -1 || node == self.ring_head {
            return MatchItem::new_literal(*buf.get_unchecked(pos));
        }

        // start matching
        for _ in 0..match_depth {
            let (node_first_byte, node_pos) = (
                (*self.items.get_unchecked(node as usize) >> 24 & 0x000000ff) as u8,
                (*self.items.get_unchecked(node as usize) >>  0 & 0x00ffffff) as usize);

            if node_first_byte == *buf.get_unchecked(pos) {
                if *buf.get_unchecked(node_pos + max_len) == *buf.get_unchecked(pos + max_len) {
                    let lcp = {
                        let a = buf.as_ptr() as usize + node_pos;
                        let b = buf.as_ptr() as usize + pos;
                        if *(a as *const u32) == *(b as *const u32) { // require min_len >= 4
                            let mut l = 4usize;
                            while l + 4 <= LZ_MATCH_MAX_LEN && *((a + l) as *const u32) == *((b + l) as *const u32) {
                                l += 4;
                            }

                            // keep max_len=255, so (l + 3 < max_len) is always true
                            l += 2 * (*((a + l) as *const u16) == *((b + l) as *const u16)) as usize;
                            l += 1 * (*((a + l) as *const  u8) == *((b + l) as *const  u8)) as usize;
                            l
                        } else {
                            0
                        }
                    };

                    if lcp > max_len {
                        max_node = node;
                        max_len = lcp;
                        if max_len == LZ_MATCH_MAX_LEN {
                            break;
                        }
                    }
                }
            }
            node = *self.nexts.get_unchecked(node as usize);
            if node == -1 || node_pos <= (*self.items.get_unchecked(node as usize) & 0x00ffffff) as usize {
                break;
            }
        }

        if max_len >= LZ_MATCH_MIN_LEN {
            MatchItem::new_match(
                ring_sub!(self.ring_head, max_node, LZ_BUCKET_ITEM_SIZE) as u16,
                max_len as u8,
            )
        } else {
            MatchItem::new_literal(*buf.get_unchecked(pos))
        }
    }

    pub unsafe fn lazy_evaluate(&mut self, buf: &[u8], pos: usize, max_len: usize, depth: usize) -> bool {
        let entry = compute_hash_on_first_4bytes!(&buf, pos) as usize % LZ_BUCKET_ITEM_HASH_SIZE;
        let mut node = *self.heads.get_unchecked(entry);

        if node == -1 {
            return false;
        }
        let buf_base = buf.as_ptr() as usize;
        let buf_cmp_dword = *((buf_base + pos + max_len - 3) as *const u32);

        for _ in 0..depth {
            let node_pos = (*self.items.get_unchecked(node as usize) >> 0 & 0x00ffffff) as usize;
            if *((buf_base + node_pos + max_len - 3) as *const u32) == buf_cmp_dword {
                return true;
            }

            node = *self.nexts.get_unchecked(node as usize);
            if node == -1 || node_pos <= (*self.items.get_unchecked(node as usize) & 0x00ffffff) as usize {
                break;
            }
        }
        return false;
    }
}

impl DecoderMFBucket {
    pub fn new() -> DecoderMFBucket {
        DecoderMFBucket {
            items: [0; LZ_BUCKET_ITEM_SIZE],
            ring_head: 0,
        }
    }

    pub fn reset(&mut self) {
        self.items.iter_mut().for_each(|v| *v = 0);
        self.ring_head = 0;
    }

    pub unsafe fn update(&mut self, pos: usize) {
        self.ring_head = ring_add!(self.ring_head, 1, LZ_BUCKET_ITEM_SIZE) as i16;
        *self.items.get_unchecked_mut(self.ring_head as usize) = pos as u32;
    }

    pub unsafe fn get_match_pos(&self, match_index: i16) -> usize {
        let node = ring_sub!(self.ring_head, match_index, LZ_BUCKET_ITEM_SIZE);
        return *self.items.get_unchecked(node) as usize;
    }
}
