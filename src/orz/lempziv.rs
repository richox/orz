use orz::bits::*;
use orz::constants::lempziv_constants::*;
use orz::huff::*;
use orz::matchfinder::*;
use orz::mtf::*;

pub const LZ_BLOCK_SIZE: usize = 16777216;
pub const LZ_CHUNK_SIZE: usize = 262144;
pub const LZ_CHUNK_TARGET_SIZE: usize = 393216;

pub struct LZCfg {
    pub match_depth: usize,
    pub match_depth_lazy_evaluation1: usize,
    pub match_depth_lazy_evaluation2: usize,
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
                bucket.update_and_match(sbuf, spos, cfg.match_depth)
            };

            if cfg.match_depth_lazy_evaluation1 > 0 && match_item.get_match_or_literal() == 0 { // lazy match 1
                let bucket = self.buckets.get_unchecked(*sbuf.get_unchecked(spos + 0) as usize);
                if bucket.lazy_evaluate(sbuf, spos + 1, match_item.get_match_len(), cfg.match_depth_lazy_evaluation1) {
                    match_item = MatchItem::new_literal(*sbuf.get_unchecked(spos));
                }
            }
            if cfg.match_depth_lazy_evaluation2 > 0 && match_item.get_match_or_literal() == 0 { // lazy match 2
                let bucket = self.buckets.get_unchecked(*sbuf.get_unchecked(spos + 1) as usize);
                if bucket.lazy_evaluate(sbuf, spos + 2, match_item.get_match_len(), cfg.match_depth_lazy_evaluation2) {
                    match_item = MatchItem::new_literal(*sbuf.get_unchecked(spos));
                }
            }

            match match_item.get_match_or_literal() {
                0 => {
                    match_items.push(match_item);
                    spos += match_item.get_match_len();
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
        let mut bits = Bits::new();
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
                    *huff_weight1.get_unchecked_mut(match_item.get_match_len() + 256) += 1;
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
        let mut match_items = Vec::<MatchItem>::with_capacity(LZ_CHUNK_SIZE);

        // decode match_items_len
        let match_items_len =
            (tbuf[tpos + 0] as usize) << 0 |
            (tbuf[tpos + 1] as usize) << 8 |
            (tbuf[tpos + 2] as usize) << 16;
        tpos += 3;
        match_items.reserve(match_items_len);

        // start Huffman decoding
        let mut bits = Bits::new();
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

                    MatchItem::new_match(
                        match_index_base + bits.get(match_index_bits_len) as u16,
                        match_len)
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
                    let match_len = match_item.get_match_len();
                    let match_pos = bucket.get_match_pos(match_item.get_match_index() as i16);
                    {  // fast increment memcopy
                        let mut a = sbuf.as_ptr() as usize + match_pos;
                        let mut b = sbuf.as_ptr() as usize + spos;
                        let r = b + match_len;

                        while b < a + 8 {
                            *(b as *mut u64) = *(a as *const u64);
                            b += b - a;
                        }
                        while b < r {
                            *(b as *mut u64) = *(a as *const u64);
                            a += 8;
                            b += 8;
                        }
                    }
                    spos += match_len;
                }
            }
        }
        Ok((spos, tpos))
    }
}
