const LZ_BUCKET_ITEM_SIZE: usize = 4096;
const LZ_BUCKET_ITEM_HASH_SIZE: usize = 8191;
const LZ_MATCH_MAX_LEN: usize = 255;
const LZ_MATCH_MIN_LEN: usize = 4;

pub enum MatchResult {
    Literal,
    Match {
        reduced_offset: u16,
        match_len: u8,
    },
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

macro_rules! hash_4bytes {
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

    pub unsafe fn find_match_and_update(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> MatchResult {
        if pos + LZ_MATCH_MAX_LEN + 4 >= buf.len() {
            return MatchResult::Literal;
        }
        let entry = hash_4bytes!(buf, pos) as usize % LZ_BUCKET_ITEM_HASH_SIZE;

        macro_rules! update {
            () => {
                let new_head = ring_add!(self.ring_head, 1, LZ_BUCKET_ITEM_SIZE) as usize;
                *self.nexts.get_unchecked_mut(new_head) = *self.heads.get_unchecked(entry);
                *self.items.get_unchecked_mut(new_head) = pos as u32 | (*buf.get_unchecked(pos) as u32) << 24;
                *self.heads.get_unchecked_mut(entry) = new_head as i16;
                self.ring_head = new_head as i16;
            }
        }

        let mut node = *self.heads.get_unchecked(entry);
        if node == -1 { // empty node
            update!();
            return MatchResult::Literal;
        }

        // start matching
        let mut max_len = (LZ_MATCH_MIN_LEN - 1) as u8;
        let mut max_node = 0;
        for _ in 0..match_depth {
            let (node_first_byte, node_pos) = (
                (*self.items.get_unchecked(node as usize) >> 24 & 0x000000ff) as u8,
                (*self.items.get_unchecked(node as usize) >>  0 & 0x00ffffff) as usize);

            if node_first_byte == *buf.get_unchecked(pos) {
                if *buf.get_unchecked(node_pos + max_len as usize) == *buf.get_unchecked(pos + max_len as usize) {
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

                    if max_len < lcp as u8 {
                        max_len = lcp as u8;
                        max_node = node;
                        if max_len as usize == LZ_MATCH_MAX_LEN {
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

        let result = {
            if max_len as usize >= LZ_MATCH_MIN_LEN {
                MatchResult::Match {
                    reduced_offset: ring_sub!(self.ring_head, max_node, LZ_BUCKET_ITEM_SIZE) as u16,
                    match_len: max_len,
                }
            } else {
                MatchResult::Literal
            }
        };
        update!();
        return result;
    }

    pub unsafe fn has_lazy_match(&self, buf: &[u8], pos: usize, max_len: usize, depth: usize) -> bool {
        let entry = hash_4bytes!(buf, pos) as usize % LZ_BUCKET_ITEM_HASH_SIZE;
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
