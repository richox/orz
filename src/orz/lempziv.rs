use orz::bits::*;
use orz::huff::*;
use orz::matchfinder::*;
use orz::mtf::*;

pub const LZ_BLOCK_SIZE: usize = 16777216;
pub const LZ_CHUNK_SIZE: usize = 262144;
pub const LZ_CHUNK_TARGET_SIZE: usize = 393216;

const LZ_MATCH_INDEX_ENCODING_ARRAY: [(u8, u8, u8); 4096] = include!("constants/LZ_MATCH_INDEX_ENCODING_ARRAY.txt");
const LZ_MATCH_INDEX_ID_BASE_ARRAY: [u16; 32]             = include!("constants/LZ_MATCH_INDEX_ID_BASE_ARRAY.txt");
const LZ_MATCH_INDEX_BITS_LEN_ARRAY: [u8; 32]             = include!("constants/LZ_MATCH_INDEX_BITS_LEN_ARRAY.txt");

pub struct LZCfg {
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}

pub struct LZEncoder {
    buckets: Vec<EncoderMFBucket>,
    mtfs: Vec<MTFEncoder>,
}

pub struct LZDecoder {
    buckets: Vec<DecoderMFBucket>,
    mtfs: Vec<MTFDecoder>,
}

impl LZEncoder {
    pub fn new() -> LZEncoder {
        LZEncoder {
            buckets: (0..256)
                .map(|_| EncoderMFBucket::new())
                .collect::<Vec<_>>(),
            mtfs: (0..256)
                .map(|_| MTFEncoder::new())
                .collect::<Vec<_>>(),
        }
    }

    pub fn reset(&mut self) {
        self.buckets.iter_mut().for_each(|bucket| bucket.reset());
    }

    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::<[u8; 3]>::with_capacity(LZ_CHUNK_SIZE);

        // start Lempel-Ziv encoding
        macro_rules! bucket {
            ($pos:expr) => {
                self.buckets.get_unchecked_mut(*sbuf.get_unchecked($pos as usize) as usize)
            }
        }
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let mtf = self.mtfs.get_unchecked_mut(*sbuf.get_unchecked(spos - 1) as usize);

            // find match
            match bucket!(spos - 1).find_match_and_update(sbuf, spos, cfg.match_depth) {
                MatchResult::Match {reduced_offset, match_len} => {
                    let match_len = match_len as usize;
                    let has_lazy_match = // perform lazy matching, (spos+2) first because it is faster
                        bucket!(spos + 1).has_lazy_match(sbuf, spos + 2, match_len, cfg.lazy_match_depth2) ||
                        bucket!(spos + 0).has_lazy_match(sbuf, spos + 1, match_len, cfg.lazy_match_depth1);

                    if has_lazy_match {
                        match_items.push([0xff, 0xff, mtf.encode(*sbuf.get_unchecked(spos))]);
                        spos += 1;
                    } else {
                        match_items.push([
                            (reduced_offset >> 8 & 0xff) as u8,
                            (reduced_offset >> 0 & 0xff) as u8,
                            match_len as u8,
                        ]);
                        spos += match_len;
                    }
                },
                MatchResult::Literal => {
                    match_items.push([0xff, 0xff, mtf.encode(*sbuf.get_unchecked(spos))]);
                    spos += 1;
                },
            }
        }

        // encode match_items_len
        tbuf[tpos + 0] = (match_items.len() >>  0) as u8;
        tbuf[tpos + 1] = (match_items.len() >>  8) as u8;
        tbuf[tpos + 2] = (match_items.len() >> 16) as u8;
        tpos += 3;

        // start Huffman encoding
        let mut bits = Bits::new();
        let mut huff_weight1 = [0i32; 512];
        let mut huff_weight2 = [0i32; 32];
        for match_item in match_items.iter() {
            match match_item[0] {
                0xff => {
                    *huff_weight1.get_unchecked_mut(match_item[2] as usize) += 1;
                }
                _ => {
                    let (match_id, _, _) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(
                        (match_item[0] as usize) << 8 |
                        (match_item[1] as usize) << 0);
                    *huff_weight1.get_unchecked_mut(match_item[2] as usize + 256) += 1;
                    *huff_weight2.get_unchecked_mut(match_id as usize) += 1;
                }
            }
        }
        let huff_encoder1 = HuffmanEncoder::from_symbol_weight_vec(&huff_weight1, 15);
        let huff_encoder2 = HuffmanEncoder::from_symbol_weight_vec(&huff_weight2, 8);

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
            match match_item[0] {
                0xff => {
                    huff_encoder1.encode_to_bits(match_item[2] as u16, &mut bits);
                }
                _ => {
                    let (match_id,
                         match_id_rest_bits_len,
                         match_id_rest_bits) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(
                             (match_item[0] as usize) << 8 |
                             (match_item[1] as usize) << 0);
                    huff_encoder1.encode_to_bits(match_item[2] as u16 + 256, &mut bits);
                    huff_encoder2.encode_to_bits(match_id as u16, &mut bits);
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

impl LZDecoder {
    pub fn new() -> LZDecoder {
        return LZDecoder {
            buckets: (0..256)
                .map(|_| DecoderMFBucket::new())
                .collect::<Vec<_>>(),
            mtfs: (0..256)
                .map(|_| MTFDecoder::new())
                .collect::<Vec<_>>(),
        };
    }

    pub fn reset(&mut self) {
        self.buckets.iter_mut().for_each(|bucket| bucket.reset());
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::<[u8; 3]>::with_capacity(LZ_CHUNK_SIZE);

        // decode match_items_len
        let match_items_len =
            (tbuf[tpos + 0] as usize) << 0 |
            (tbuf[tpos + 1] as usize) << 8 |
            (tbuf[tpos + 2] as usize) << 16;
        tpos += 3;
        match_items.reserve(match_items_len);

        // start Huffman decoding
        let mut bits = Bits::new();
        let mut huff_symbol_bits_lens1 = [0u8; 256 + LZ_MATCH_MAX_LEN + 1];
        let mut huff_symbol_bits_lens2 = [0u8; 32];

        for i in 0 .. huff_symbol_bits_lens1.len() / 2 {
            bits.put(8, *tbuf.get_unchecked(tpos + i) as u64);
            *huff_symbol_bits_lens1.get_unchecked_mut(i * 2 + 0) = bits.get(4) as u8;
            *huff_symbol_bits_lens1.get_unchecked_mut(i * 2 + 1) = bits.get(4) as u8;
        }
        tpos += huff_symbol_bits_lens1.len() / 2;

        for i in 0 .. huff_symbol_bits_lens2.len() / 2 {
            bits.put(8, *tbuf.get_unchecked(tpos + i) as u64);
            *huff_symbol_bits_lens2.get_unchecked_mut(i * 2 + 0) = bits.get(4) as u8;
            *huff_symbol_bits_lens2.get_unchecked_mut(i * 2 + 1) = bits.get(4) as u8;
        }
        tpos += huff_symbol_bits_lens2.len() / 2;

        let huff_decoder1 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens1, 15);
        let huff_decoder2 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens2, 8);
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
            let b = huff_decoder1.decode_from_bits(&mut bits);
            if b >= huff_symbol_bits_lens1.len() as u16 {
                Err(())?; // invalid data
            }

            if b < 256 {
                let symbol = b as u8;
                match_items.push([0xff, 0xff, symbol]);

            } else {
                let match_len = (b - 256) as usize;
                let b = huff_decoder2.decode_from_bits(&mut bits);
                if b >= huff_symbol_bits_lens2.len() as u16 {
                    Err(())?; // invalid data
                }
                let reduced_offset_id = b;

                let (reduced_offset_base, reduced_offset_bits_len) = (
                    *LZ_MATCH_INDEX_ID_BASE_ARRAY.get_unchecked_mut(reduced_offset_id as usize),
                    *LZ_MATCH_INDEX_BITS_LEN_ARRAY.get_unchecked_mut(reduced_offset_id as usize),
                );
                let reduced_offset = reduced_offset_base + bits.get(reduced_offset_bits_len) as u16;
                match_items.push([
                    (reduced_offset >> 8 & 0xff) as u8,
                    (reduced_offset >> 0 & 0xff) as u8,
                    match_len as u8,
                ]);
            }
        }

        for match_item in match_items {
            let bucket = self.buckets.get_unchecked_mut(*sbuf.get_unchecked(spos - 1) as usize);
            match match_item[0] {
                0xff => {
                    let mtf = self.mtfs.get_unchecked_mut(*sbuf.get_unchecked(spos - 1) as usize);
                    *sbuf.get_unchecked_mut(spos) = mtf.decode(match_item[2]);
                    bucket.update(spos);
                    spos += 1;
                }
                _ => {
                    let match_len = match_item[2] as usize;
                    let match_pos = bucket.get_match_pos((match_item[0] as u16) << 8 | match_item[1] as u16);
                    { // fast increment memcopy
                        let mut a = sbuf.as_ptr() as usize + match_pos;
                        let mut b = sbuf.as_ptr() as usize + spos;
                        let r = b + match_len;

                        while b < a + 4 {
                            *(b as *mut u32) = *(a as *const u32);
                            b += b - a;
                        }
                        while b < r {
                            *(b as *mut u32) = *(a as *const u32);
                            a += 4;
                            b += 4;
                        }
                    }
                    bucket.update(spos);
                    spos += match_len;
                }
            }
        }
        Ok((spos, tpos))
    }
}
