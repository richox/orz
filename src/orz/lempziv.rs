use std;
use orz::bits::*;
use orz::huff::*;
use orz::matchfinder::*;
use orz::mtf::*;

pub const LZ_CHUNK_SIZE: usize = 262144;
pub const LZ_CHUNK_TARGET_SIZE: usize = 393216;

const LZ_MATCH_INDEX_SIZE: usize = 32;
const LZ_MATCH_INDEX_ENCODING_ARRAY: [(u8, u8, u8); 4096] = include!("constants/LZ_MATCH_INDEX_ENCODING_ARRAY.txt");
const LZ_MATCH_INDEX_ID_BASE_ARRAY: [u16; 32]             = include!("constants/LZ_MATCH_INDEX_ID_BASE_ARRAY.txt");
const LZ_MATCH_INDEX_BITS_LEN_ARRAY: [u8; 32]             = include!("constants/LZ_MATCH_INDEX_BITS_LEN_ARRAY.txt");

pub struct LZCfg {
    pub block_size: usize,
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}
pub struct LZEncoder {buckets: Vec<EncoderMFBucket>, mtfs: Vec<MTFEncoder>}
pub struct LZDecoder {buckets: Vec<DecoderMFBucket>, mtfs: Vec<MTFDecoder>}

enum MatchItem {
    Match   {reduced_offset: u16, match_len: u8},
    Literal {symbol: u8},
}

macro_rules! bucket_context {
    ($buf:expr, $pos:expr) => (*$buf.get_unchecked($pos as usize - 1) as usize)
}

impl LZEncoder {
    pub fn new() -> LZEncoder {
        LZEncoder {
            buckets: (0..256).map(|_| EncoderMFBucket::new()).collect::<Vec<_>>(),
            mtfs:    (0..256).map(|_| MTFEncoder::new()).collect::<Vec<_>>(),
        }
    }

    pub fn reset(&mut self) {
        self.buckets = (0..256).map(|_| EncoderMFBucket::new()).collect::<Vec<_>>();
    }

    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::with_capacity(LZ_CHUNK_SIZE);

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let mtf = self.mtfs.get_unchecked_mut(bucket_context!(sbuf, spos));
            let match_result = {
                let bucket = self.buckets.get_unchecked_mut(bucket_context!(sbuf, spos));
                bucket.find_match_and_update(sbuf, spos, cfg.match_depth)
            };

            // find match
            match match_result {
                MatchResult::Match {reduced_offset, match_len} => {
                    let bucket1 = self.buckets.get_unchecked(bucket_context!(sbuf, spos + 1));
                    let bucket2 = self.buckets.get_unchecked(bucket_context!(sbuf, spos + 2));
                    let has_lazy_match = // perform lazy matching, (spos+2) first because it is faster
                        bucket2.has_lazy_match(sbuf, spos + 2, match_len as usize, cfg.lazy_match_depth2) ||
                        bucket1.has_lazy_match(sbuf, spos + 1, match_len as usize, cfg.lazy_match_depth1);

                    if has_lazy_match {
                        match_items.push(MatchItem::Literal {symbol: mtf.encode(*sbuf.get_unchecked(spos))});
                        spos += 1;
                    } else {
                        match_items.push(MatchItem::Match {reduced_offset: reduced_offset, match_len: match_len});
                        spos += match_len as usize;
                    }
                },
                MatchResult::Literal => {
                    match_items.push(MatchItem::Literal {symbol: mtf.encode(*sbuf.get_unchecked(spos))});
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
        let huff_weights = [
            &mut [0i32; 256 + LZ_MATCH_MAX_LEN + 1][..],
            &mut [0i32; LZ_MATCH_INDEX_SIZE][..]];
        for match_item in match_items.iter() {
            match match_item {
                &MatchItem::Literal {symbol} => {
                    *huff_weights[0].get_unchecked_mut(symbol as usize) += 1;
                },
                &MatchItem::Match {reduced_offset, match_len} => {
                    let (roid, _, _) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(reduced_offset as usize);
                    *huff_weights[0].get_unchecked_mut(match_len as usize + 256) += 1;
                    *huff_weights[1].get_unchecked_mut(roid as usize) += 1;
                }
            }
        }
        let huff_encoder1 = HuffmanEncoder::from_symbol_weight_vec(huff_weights[0], 15);
        let huff_encoder2 = HuffmanEncoder::from_symbol_weight_vec(huff_weights[1], 8);
        [huff_encoder1.get_symbol_bits_lens(), huff_encoder2.get_symbol_bits_lens()].iter()
            .for_each(|huff_symbol_bits_lens| {
                for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                    *tbuf.get_unchecked_mut(tpos + i) =
                        huff_symbol_bits_lens.get_unchecked(i * 2 + 0) * 16 +
                        huff_symbol_bits_lens.get_unchecked(i * 2 + 1);
                }
                tpos += huff_symbol_bits_lens.len() / 2;
            });

        let bits = &mut Bits::new();
        for match_item in match_items.iter() {
            match match_item {
                &MatchItem::Literal {symbol} => {
                    huff_encoder1.encode_to_bits(symbol as u16, bits);
                },
                &MatchItem::Match {reduced_offset, match_len} => {
                    let (roid,
                         roid_rest_bits_len,
                         roid_rest_bits) = *LZ_MATCH_INDEX_ENCODING_ARRAY.get_unchecked(reduced_offset as usize);
                    huff_encoder1.encode_to_bits(match_len as u16 + 256, bits);
                    huff_encoder2.encode_to_bits(roid as u16, bits);
                    bits.put(roid_rest_bits_len, roid_rest_bits as u64);
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
            buckets: (0..256).map(|_| DecoderMFBucket::new()).collect::<Vec<_>>(),
            mtfs:    (0..256).map(|_| MTFDecoder::new()).collect::<Vec<_>>(),
        };
    }

    pub fn reset(&mut self) {
        self.buckets = (0..256).map(|_| DecoderMFBucket::new()).collect::<Vec<_>>();
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut spos = spos;
        let mut tpos = 0;

        // decode match_items_len
        let match_items_len =
            (tbuf[tpos + 0] as usize) << 0 |
            (tbuf[tpos + 1] as usize) << 8 |
            (tbuf[tpos + 2] as usize) << 16;
        tpos += 3;

        // start decoding
        let huff_symbol_bits_lenses = &mut [
            &mut [0u8; 256 + LZ_MATCH_MAX_LEN + 1][..],
            &mut [0u8; LZ_MATCH_INDEX_SIZE][..]];
        huff_symbol_bits_lenses.iter_mut().for_each(|huff_symbol_bits_lens| {
            for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                *huff_symbol_bits_lens.get_unchecked_mut(i * 2 + 0) = tbuf.get_unchecked(tpos + i) / 16;
                *huff_symbol_bits_lens.get_unchecked_mut(i * 2 + 1) = tbuf.get_unchecked(tpos + i) % 16;
            }
            tpos += huff_symbol_bits_lens.len() / 2;
        });

        let huff_decoder1 = HuffmanDecoder::from_symbol_bits_lens(huff_symbol_bits_lenses[0]);
        let huff_decoder2 = HuffmanDecoder::from_symbol_bits_lens(huff_symbol_bits_lenses[1]);
        let bits = &mut Bits::new();
        for _ in 0 .. match_items_len {
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

            let bucket = self.buckets.get_unchecked_mut(bucket_context!(sbuf, spos));
            let mtf = self.mtfs.get_unchecked_mut(bucket_context!(sbuf, spos));

            match huff_decoder1.decode_from_bits(bits) {
                symbol_u16 if (0..256).contains(symbol_u16) => {
                    *sbuf.get_unchecked_mut(spos) = mtf.decode(symbol_u16 as u8);
                    bucket.update(spos);
                    spos += 1;
                },
                match_len_x if (LZ_MATCH_MIN_LEN as u16 .. LZ_MATCH_MAX_LEN as u16 + 1).contains(match_len_x - 256) => {
                    let match_len = match_len_x as usize - 256;
                    let roid = match huff_decoder2.decode_from_bits(bits) {
                        roid if (0 .. LZ_MATCH_INDEX_SIZE as u16 + 1).contains(roid) => roid,
                        _ => Err(())? // invalid data
                    };

                    let (reduced_offset_base, reduced_offset_bits_len) = (
                        *LZ_MATCH_INDEX_ID_BASE_ARRAY.get_unchecked_mut(roid as usize),
                        *LZ_MATCH_INDEX_BITS_LEN_ARRAY.get_unchecked_mut(roid as usize),
                    );
                    let reduced_offset = reduced_offset_base + bits.get(reduced_offset_bits_len) as u16;
                    let match_pos = bucket.get_match_pos(reduced_offset);
                    bucket.update(spos);

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
                    spos += match_len;
                },
                _ => Err(())? // invalid data
            }
        }
        // (spos+match_len) may overflow, but it is safe because of sentinels
        Ok((std::cmp::min(spos, sbuf.len()), tpos))
    }
}
