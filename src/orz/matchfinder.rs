pub const LZ_MATCH_MAX_LEN: usize = 255;
pub const LZ_MATCH_MIN_LEN: usize = 4;

const LZ_BUCKET_ITEM_SIZE: usize = 4096;
const LZ_BUCKET_ITEM_HASH_SIZE: usize = 8192;

pub enum MatchResult {
    Match {reduced_offset: u16, match_len: u8},
    Literal,
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

macro_rules! item_size_cycle {
    ($a:expr, "+", $b:expr) => (($a as usize + $b as usize) % LZ_BUCKET_ITEM_SIZE);
    ($a:expr, "-", $b:expr) => (($a as usize + LZ_BUCKET_ITEM_SIZE - $b as usize) % LZ_BUCKET_ITEM_SIZE)
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

    pub unsafe fn find_match_and_update(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> MatchResult {
        let entry = hash_4bytes!(buf, pos) as usize % LZ_BUCKET_ITEM_HASH_SIZE;

        macro_rules! update {
            () => {
                let new_head = item_size_cycle!(self.ring_head, "+", 1) as usize;
                *self.nexts.get_unchecked_mut(new_head) = *self.heads.get_unchecked(entry);
                *self.items.get_unchecked_mut(new_head) = pos as u32;
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
        let mut max_len = LZ_MATCH_MIN_LEN - 1;
        let mut max_node = 0;

        let buf_dword = *((buf.as_ptr() as usize + pos) as *const u32);
        let mut buf_max_len_dword = buf_dword;

        for _ in 0..match_depth {
            let node_pos = *self.items.get_unchecked(node as usize) as usize;
            let sample_matched = // sample by first and last dwords
                *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == buf_max_len_dword &&
                *((buf.as_ptr() as usize + node_pos) as *const u32) == buf_dword;

            if sample_matched {
                let lcp = {
                    let a = buf.as_ptr() as usize + node_pos;
                    let b = buf.as_ptr() as usize + pos;
                    let mut l = 4usize;
                    while l + 4 <= LZ_MATCH_MAX_LEN && *((a + l) as *const u32) == *((b + l) as *const u32) {
                        l += 4;
                    }

                    // keep max_len=255, so (l + 3 < max_len) is always true
                    l += 2 * (*((a + l) as *const u16) == *((b + l) as *const u16)) as usize;
                    l += 1 * (*((a + l) as *const  u8) == *((b + l) as *const  u8)) as usize;
                    l
                };

                if max_len < lcp {
                    max_len = lcp;
                    max_node = node;
                    if max_len == LZ_MATCH_MAX_LEN {
                        break;
                    }
                    buf_max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
                }
            }

            node = *self.nexts.get_unchecked(node as usize);
            if node == -1 || node_pos <= *self.items.get_unchecked(node as usize) as usize {
                break;
            }
        }

        let result = match max_len >= LZ_MATCH_MIN_LEN {
            false => MatchResult::Literal,
            true  => MatchResult::Match {
                reduced_offset: item_size_cycle!(self.ring_head, "-", max_node) as u16,
                match_len: max_len as u8,
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
        let buf_dword = *((buf.as_ptr() as usize + pos) as *const u32);
        let buf_max_len_dword = *((buf.as_ptr() as usize + pos + max_len - 3) as *const u32);
        for _ in 0..depth {
            let node_pos = *self.items.get_unchecked(node as usize) as usize;
            *((buf.as_ptr() as usize + node_pos + max_len - 3) as *const u32) == buf_max_len_dword &&
            *((buf.as_ptr() as usize + node_pos) as *const u32) == buf_dword && {
                return true;
            };

            node = *self.nexts.get_unchecked(node as usize);
            if node == -1 || node_pos <= *self.items.get_unchecked(node as usize) as usize {
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

    pub unsafe fn update(&mut self, pos: usize) {
        self.ring_head = item_size_cycle!(self.ring_head, "+", 1) as i16;
        *self.items.get_unchecked_mut(self.ring_head as usize) = pos as u32;
    }

    pub unsafe fn get_match_pos(&mut self, reduced_offset: u16) -> usize {
        return *self.items.get_unchecked(item_size_cycle!(self.ring_head, "-", reduced_offset)) as usize;
    }
}
