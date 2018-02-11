use orz;
use orz::constants::lempziv_constants::*;

pub const LZ_BLOCK_SIZE: usize = 16777216;
pub const LZ_CHUNK_SIZE: usize = 262144;
pub const LZ_CHUNK_TARGET_SIZE: usize = 393216;

pub struct Params {
    pub match_depth: usize,
    pub match_depth_lazy_evaluation1: usize,
    pub match_depth_lazy_evaluation2: usize,
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub struct MatchItem {
    raw1: u16,
    raw2: u8,
}

pub struct Encoder {
    buckets: Vec<EncoderBucket>,
    mtfs: Vec<orz::mtf::Encoder>,
}

pub struct Decoder {
    buckets: Vec<DecoderBucket>,
    mtfs: Vec<orz::mtf::Decoder>,
}

struct EncoderBucket {
    heads: [i16; LZ_BUCKET_ITEM_HASH_SIZE],
    nexts: [i16; LZ_BUCKET_ITEM_SIZE],
    items: [u32; LZ_BUCKET_ITEM_SIZE],
    ring_head: i16,
}

struct DecoderBucket {
    items: [u32; LZ_BUCKET_ITEM_SIZE],
    ring_head: i16,
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

    pub fn get_match_len(&self) -> u8 {
        self.raw2
    }

    pub fn get_literal(&self) -> u8 {
        self.raw2
    }
}

impl Encoder {
    pub fn new() -> Encoder {
        Encoder {
            buckets: (0..256)
                .map(|_| EncoderBucket::new())
                .collect::<Vec<_>>(),
            mtfs: (0..256)
                .map(|_| orz::mtf::Encoder::new())
                .collect::<Vec<_>>(),
        }
    }

    pub fn reset(&mut self) {
        for bucket in &mut self.buckets {
            bucket.nexts.iter_mut().for_each(|v| *v = -1);
            bucket.heads.iter_mut().for_each(|v| *v = -1);
            bucket.items.iter_mut().for_each(|v| *v = 0);
            bucket.ring_head = 0;
        }
    }

    pub unsafe fn encode(&mut self, params: &Params, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::<MatchItem>::with_capacity(LZ_CHUNK_SIZE);

        // skip first bytes
        if spos == 0 {
            match_items.push(MatchItem::new_literal(*sbuf.get_unchecked(spos)));
            spos += 1;
        }

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let mut match_item = {
                let bucket = self.buckets.get_unchecked_mut(*sbuf.get_unchecked(spos - 1) as usize);
                bucket.update_and_match(sbuf, spos, params.match_depth)
            };

            if params.match_depth_lazy_evaluation1 > 0 && match_item.get_match_or_literal() == 0 {
                // lazy match 1
                if self.buckets.get_unchecked_mut(*sbuf.get_unchecked(spos + 0) as usize).lazy_evaluate(
                    sbuf,
                    spos + 1,
                    match_item.get_match_len() as usize,
                    params.match_depth_lazy_evaluation1,
                ) {
                    match_item = MatchItem::new_literal(*sbuf.get_unchecked(spos));
                }
            }
            if params.match_depth_lazy_evaluation2 > 0 && match_item.get_match_or_literal() == 0 {
                // lazy match 2
                if self.buckets.get_unchecked_mut(*sbuf.get_unchecked(spos + 1) as usize).lazy_evaluate(
                    sbuf,
                    spos + 2,
                    match_item.get_match_len() as usize,
                    params.match_depth_lazy_evaluation2,
                ) {
                    match_item = MatchItem::new_literal(*sbuf.get_unchecked(spos));
                }
            }

            match match_item.get_match_or_literal() {
                0 => {
                    match_items.push(match_item);
                    spos += match_item.get_match_len() as usize;
                }
                _ => {
                    let mtf = &mut self.mtfs.get_unchecked_mut(*sbuf.get_unchecked(spos - 1) as usize);
                    let mtf_encoded_literal = mtf.encode(match_item.get_literal());
                    match_items.push(MatchItem::new_literal(mtf_encoded_literal));
                    spos += 1;
                }
            }
        }

        // encode match_items_len
        tbuf[tpos + 0] = (match_items.len() >>  0) as u8;
        tbuf[tpos + 1] = (match_items.len() >>  8) as u8;
        tbuf[tpos + 2] = (match_items.len() >> 16) as u8;
        tpos += 3;

        // start Huffman encoding
        let mut bits = orz::bits::Bits::new();
        let mut huff_weight1 = [0i32; 512];
        let mut huff_weight2 = [0i32; 32];
        for match_item in match_items.iter() {
            match match_item.get_match_or_literal() {
                1 => {
                    *huff_weight1.get_unchecked_mut(match_item.get_literal() as usize) += 1;
                }
                _ => {
                    let (match_id, _, _) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(
                        match_item.get_match_index() as usize);
                    *huff_weight1.get_unchecked_mut(match_item.get_match_len() as usize + 256) += 1;
                    *huff_weight2.get_unchecked_mut(match_id as usize) += 1;
                }
            }
        }
        let huff_encoder1 = orz::huff::HuffmanEncoder::from_symbol_weight_vec(&huff_weight1, 15);
        let huff_encoder2 = orz::huff::HuffmanEncoder::from_symbol_weight_vec(&huff_weight2, 8);

        for symbol_bits_len in huff_encoder1.get_symbol_bits_lens() {
            bits.put(4, *symbol_bits_len as u64);
            if bits.len() >= 8 {
                tbuf[tpos] = bits.get(8) as u8;
                tpos += 1;
            }
        }
        for symbol_bits_len in huff_encoder2.get_symbol_bits_lens() {
            bits.put(4, *symbol_bits_len as u64);
            if bits.len() >= 8 {
                tbuf[tpos] = bits.get(8) as u8;
                tpos += 1;
            }
        }

        for match_item in match_items.iter() {
            match match_item.get_match_or_literal() {
                1 => {
                    huff_encoder1.encode_to_bits(match_item.get_literal() as i16, &mut bits);
                }
                _ => {
                    let (match_id,
                         match_id_rest_bits_len,
                         match_id_rest_bits) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(
                             match_item.get_match_index() as usize);
                    huff_encoder1.encode_to_bits(match_item.get_match_len() as i16 + 256, &mut bits);
                    huff_encoder2.encode_to_bits(match_id as i16, &mut bits);
                    bits.put(match_id_rest_bits_len, match_id_rest_bits as u64);
                }
            }
            if bits.len() >= 32 {
                for _ in 0 .. 4 {
                    tbuf[tpos] = bits.get(8) as u8;
                    tpos += 1;
                };
            }
        }
        match bits.len() % 8 {
            1 => bits.put(7, 0u64), 2 => bits.put(6, 0u64),
            3 => bits.put(5, 0u64), 4 => bits.put(4, 0u64),
            5 => bits.put(3, 0u64), 6 => bits.put(2, 0u64),
            7 => bits.put(1, 0u64), _ => (),
        }
        while bits.len() > 0 {
            tbuf[tpos] = bits.get(8) as u8;
            tpos += 1;
        }
        return (spos, tpos);
    }
}

impl Decoder {
    pub fn new() -> Decoder {
        return Decoder {
            buckets: (0..256)
                .map(|_| DecoderBucket::new())
                .collect::<Vec<_>>(),
            mtfs: (0..256)
                .map(|_| orz::mtf::Decoder::new())
                .collect::<Vec<_>>(),
        };
    }

    pub fn reset(&mut self) {
        for bucket in &mut self.buckets {
            bucket.items.iter_mut().for_each(|v| *v = 0);
            bucket.ring_head = 0;
        }
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::<MatchItem>::with_capacity(LZ_CHUNK_SIZE);

        // decode match_items_len
        let match_items_len =
            (tbuf[tpos + 0] as usize) << 0 |
            (tbuf[tpos + 1] as usize) << 8 |
            (tbuf[tpos + 2] as usize) << 16;
        tpos += 3;
        match_items.reserve(match_items_len);

        // start Huffman decoding
        let mut bits = orz::bits::Bits::new();
        let mut huff_symbol_bits_lens1 = [0u8; 512];
        let mut huff_symbol_bits_lens2 = [0u8; 32];

        for i in 0 .. 256 {
            bits.put(8, tbuf[tpos + i] as u64);
            huff_symbol_bits_lens1[i * 2 + 0] = bits.get(4) as u8;
            huff_symbol_bits_lens1[i * 2 + 1] = bits.get(4) as u8;
        }
        tpos += 256;

        for i in 0 .. 16 {
            bits.put(8, tbuf[tpos + i] as u64);
            huff_symbol_bits_lens2[i * 2 + 0] = bits.get(4) as u8;
            huff_symbol_bits_lens2[i * 2 + 1] = bits.get(4) as u8;
        }
        tpos += 16;

        let huff_decoder1 = orz::huff::HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens1, 15);
        let huff_decoder2 = orz::huff::HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens2, 8);
        while match_items.len() < match_items_len {
            if bits.len() < 32 {
                for _ in 0 .. 4 {
                    if tpos < tbuf.len() {
                        bits.put(8, *tbuf.get_unchecked(tpos) as u64);
                        tpos += 1;
                    } else {
                        bits.put(8, 0u64);
                    }
                }
            }
            match_items.push({
                let b = huff_decoder1.decode_from_bits(&mut bits);
                if b < 0 || b >= 512 {
                    Err(())?; // invalid data
                }

                if 0 <= b && b < 256 {
                    MatchItem::new_literal(b as u8)
                } else {
                    let match_index_id = huff_decoder2.decode_from_bits(&mut bits);
                    if match_index_id < 0 || match_index_id >= 32 {
                        Err(())?; // invalid data
                    }

                    let match_len = b as u8;
                    let match_index_base = *LZ_MATCH_INDEX_ID_BASE_ARRAY.get_unchecked_mut(
                        match_index_id as usize);
                    let match_index_bits_len = *LZ_MATCH_INDEX_BITS_LEN_ARRAY.get_unchecked_mut(
                        match_index_id as usize);
                    MatchItem::new_match(match_index_base + bits.get(match_index_bits_len) as u16, match_len)
                }
            });
        }

        // start Lempel-Ziv decoding
        if spos == 0 {
            sbuf[spos] = match_items[0].get_literal();
            spos += 1;
        }
        for match_item in match_items[(spos == 1) as usize .. ].iter() {
            let bucket = &mut self.buckets[sbuf[spos - 1] as usize];
            bucket.update(spos);

            match match_item.get_match_or_literal() {
                1 => {
                    let mtf = &mut self.mtfs[sbuf[spos - 1] as usize];
                    sbuf[spos] = mtf.decode(match_item.get_literal());
                    spos += 1;
                }
                _ => {
                    let match_len = match_item.get_match_len() as usize;
                    let match_pos = bucket.get_match_pos(match_item.get_match_index() as i16);
                    lz_fast_memcopy(sbuf.get_unchecked(match_pos), sbuf.get_unchecked_mut(spos), match_len);
                    spos += match_len;
                }
            }
        }
        Ok((spos, tpos))
    }
}

impl EncoderBucket {
    fn new() -> EncoderBucket {
        EncoderBucket {
            heads: [-1; LZ_BUCKET_ITEM_HASH_SIZE],
            nexts: [-1; LZ_BUCKET_ITEM_SIZE],
            items: [0; LZ_BUCKET_ITEM_SIZE],
            ring_head: 0,
        }
    }

    unsafe fn update_and_match(&mut self, buf: &[u8], pos: usize, match_depth: usize) -> MatchItem {
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
                    let lcp = lz_fast_memlcp(
                        buf.get_unchecked(node_pos) as *const u8,
                        buf.get_unchecked(pos) as *const u8,
                        LZ_MATCH_MAX_LEN);

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

    unsafe fn lazy_evaluate(&mut self, buf: &[u8], pos: usize, max_len: usize, depth: usize) -> bool {
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

impl DecoderBucket {
    fn new() -> DecoderBucket {
        DecoderBucket {
            items: [0; LZ_BUCKET_ITEM_SIZE],
            ring_head: 0,
        }
    }

    unsafe fn update(&mut self, pos: usize) {
        self.ring_head = ring_add!(self.ring_head, 1, LZ_BUCKET_ITEM_SIZE) as i16;
        *self.items.get_unchecked_mut(self.ring_head as usize) = pos as u32;
    }

    unsafe fn get_match_pos(&self, match_index: i16) -> usize {
        let node = ring_sub!(self.ring_head, match_index, LZ_BUCKET_ITEM_SIZE);
        return *self.items.get_unchecked(node) as usize;
    }
}

unsafe fn lz_fast_memlcp(a: *const u8, b: *const u8, max_len: usize) -> usize {
    let a = a as usize;
    let b = b as usize;
    if *(a as *const u32) == *(b as *const u32) {
        let mut len = 4usize;
        while len + 4 <= max_len && *((a + len) as *const u32) == *((b + len) as *const u32) {
            len += 4;
        }

        // keep max_len=255, so (len + 3 < max_len) is always true
        len += 2 * (*((a + len) as *const u16) == *((b + len) as *const u16)) as usize;
        len += 1 * (*((a + len) as *const  u8) == *((b + len) as *const  u8)) as usize;
        return len;
    }
    return 0;
}

unsafe fn lz_fast_memcopy(a: *const u8, b: *mut u8, len: usize) {
    let mut a = a as usize;
    let mut b = b as usize;
    let mut l = len as isize;

    while a + 8 > b {
        *(b as *mut u64) = *(a as *const u64);
        l -= (b - a) as isize;
        b += (b - a) as usize;
    }
    while l > 0 {
        *(b as *mut u64) = *(a as *const u64);
        l -= 8;
        a += 8;
        b += 8;
    }
}
