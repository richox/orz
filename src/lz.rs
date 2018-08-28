use std;
use std::ops::AddAssign;
use byteorder::BE;
use byteorder::ByteOrder;
use super::aux::ResultExt;
use super::aux::UncheckedSliceExt;
use super::bits::Bits;
use super::huffman::HuffmanDecoder;
use super::huffman::HuffmanEncoder;
use super::matchfinder::DecoderMFBucket;
use super::matchfinder::EncoderMFBucket;
use super::mtf::MTFDecoder;
use super::mtf::MTFEncoder;

const LZ_ROID_ENCODING_ARRAY: [(u8, u8, u8); super::LZ_MF_BUCKET_ITEM_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_ENCODING_ARRAY.txt"));
const LZ_ROID_DECODING_ARRAY: [(u16, u8); super::LZ_ROID_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_DECODING_ARRAY.txt"));

pub struct LZCfg {
    pub block_size: usize,
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}
pub struct LZEncoder {
    buckets:    Vec<EncoderMFBucket>,
    mtfs:       Vec<MTFEncoder>,
    last_words: [[[u8; 2]; super::LZ_CONTEXT_BUCKET_SIZE]; super::LZ_CONTEXT_BUCKET_SIZE],
}
pub struct LZDecoder {
    buckets:    Vec<DecoderMFBucket>,
    mtfs:       Vec<MTFDecoder>,
    last_words: [[[u8; 2]; super::LZ_CONTEXT_BUCKET_SIZE]; super::LZ_CONTEXT_BUCKET_SIZE],
}

pub enum MatchItem {
    Match    {reduced_offset: u16, match_len: u8},
    Literal  {mtf_symbol: u8},
    LastWord {},
}

impl LZEncoder {
    pub fn new() -> LZEncoder {
        LZEncoder {
            buckets:    (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| EncoderMFBucket::new()).collect(),
            mtfs:       (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| MTFEncoder::new()).collect(),
            last_words: [[[0u8; 2]; super::LZ_CONTEXT_BUCKET_SIZE]; super::LZ_CONTEXT_BUCKET_SIZE],
        }
    }

    pub fn reset(&mut self) {
        self.buckets = (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| EncoderMFBucket::new()).collect();
    }

    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::with_capacity(super::LZ_CHUNK_SIZE);

        // huff1:
        //  0 .. 256 => mtf_symbol
        //  256      => last_word
        //  257      => unused
        //  258 .. ? => roid
        let mut huff_weights1 = [0u32; 258 + super::LZ_MATCH_MAX_LEN + 1];
        let mut huff_weights2 = [0u32; super::LZ_ROID_SIZE];

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let context1 = get_bucket_context(sbuf, spos - 1);
            let context0 = get_bucket_context(sbuf, spos);

            let match_result = {
                let bucket = self.buckets.xget_mut(context0);
                bucket.find_match_and_update(sbuf, spos, cfg.match_depth)
            };

            // find match
            let mut matched = false;
            if let MatchItem::Match {reduced_offset, match_len} = match_result {
                let bucket1 = self.buckets.xget(get_bucket_context(sbuf, spos + 1));
                let bucket2 = self.buckets.xget(get_bucket_context(sbuf, spos + 2));
                let has_lazy_match =
                    bucket1.has_lazy_match(sbuf, spos + 1, match_len as usize, cfg.lazy_match_depth1) ||
                    bucket2.has_lazy_match(sbuf, spos + 2, match_len as usize, cfg.lazy_match_depth2);

                if !has_lazy_match {
                    match_items.push(MatchItem::Match {reduced_offset, match_len});
                    spos += match_len as usize;
                    let context_m3 = get_bucket_context(sbuf, spos - 3);
                    let context_m2 = get_bucket_context(sbuf, spos - 2);
                    self.last_words.xget_mut(context_m3).xset(context_m2, [*sbuf.xget(spos - 2), *sbuf.xget(spos - 1)]);

                    // count huffman
                    let roid = LZ_ROID_ENCODING_ARRAY.xget(reduced_offset).0;
                    huff_weights1.xget_mut(match_len as u16 + 258).add_assign(1);
                    huff_weights2.xget_mut(roid).add_assign(1);
                    matched = true;
                }
            }

            let mut last_word_matched = false;
            if !matched {
                if *self.last_words.xget(context1).xget(context0) == [*sbuf.xget(spos), *sbuf.xget(spos + 1)] {
                    match_items.push(MatchItem::LastWord {});
                    spos += 2;
                    huff_weights1.xget_mut(256).add_assign(1); // count huffman
                    last_word_matched = true;
                }
            }

            if !matched && !last_word_matched {
                let mtf_symbol = self.mtfs.xget_mut(context0).encode(*sbuf.xget(spos));
                match_items.push(MatchItem::Literal {mtf_symbol});
                spos += 1;
                let context_m3 = get_bucket_context(sbuf, spos - 3);
                let context_m2 = get_bucket_context(sbuf, spos - 2);
                self.last_words.xget_mut(context_m3).xset(context_m2, [*sbuf.xget(spos - 2), *sbuf.xget(spos - 1)]);
                huff_weights1.xget_mut(mtf_symbol).add_assign(1); // count huffman
            }
        }

        // encode match_items_len
        BE::write_u32(std::slice::from_raw_parts_mut(tbuf.xget_mut(tpos), 4), match_items.len() as u32);
        tpos += 4;

        // start Huffman encoding
        let huff_encoder1 = HuffmanEncoder::from_symbol_weight_vec(&huff_weights1, 15);
        let huff_encoder2 = HuffmanEncoder::from_symbol_weight_vec(&huff_weights2, 8);
        let mut bits = Bits::new();
        for huff_symbol_bits_lens in &[huff_encoder1.get_symbol_bits_lens(), huff_encoder2.get_symbol_bits_lens()] {
            for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                tbuf.xset(tpos + i, huff_symbol_bits_lens.xget(i * 2) * 16 + huff_symbol_bits_lens.xget(i * 2 + 1));
            }
            tpos += huff_symbol_bits_lens.len() / 2;
        }

        for match_item in &match_items {
            match match_item {
                &MatchItem::Literal {mtf_symbol} => {
                    huff_encoder1.encode_to_bits(mtf_symbol as u16, &mut bits);
                },
                &MatchItem::LastWord {} => {
                    huff_encoder1.encode_to_bits(256, &mut bits);
                },
                &MatchItem::Match {reduced_offset, match_len} => {
                    let (roid, robitlen, robits) = *LZ_ROID_ENCODING_ARRAY.xget(reduced_offset);
                    huff_encoder1.encode_to_bits(match_len as u16 + 258, &mut bits);
                    huff_encoder2.encode_to_bits(roid as u16, &mut bits);
                    bits.put(robitlen, robits as u64);
                }
            }
            if bits.len() >= 32 {
                BE::write_u32(std::slice::from_raw_parts_mut(tbuf.xget_mut(tpos), 4), bits.get(32) as u32);
                tpos += 4;
            }
        }
        let num_unaligned_bits = 8 - bits.len() % 8;
        bits.put(num_unaligned_bits, 0);

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
            buckets:    (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| DecoderMFBucket::new()).collect(),
            mtfs:       (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| MTFDecoder::new()).collect(),
            last_words: [[[0u8; 2]; super::LZ_CONTEXT_BUCKET_SIZE]; super::LZ_CONTEXT_BUCKET_SIZE],
        };
    }

    pub fn reset(&mut self) {
        self.buckets = (0 .. super::LZ_CONTEXT_BUCKET_SIZE).map(|_| DecoderMFBucket::new()).collect();
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut spos = spos;
        let mut tpos = 0;

        // decode match_items_len
        let match_items_len = BE::read_u32(std::slice::from_raw_parts(tbuf.xget(tpos), 4)) as usize;
        tpos += 4;

        // start decoding
        let mut huff_symbol_bits_lens1 = [0u8; 258 + super::LZ_MATCH_MAX_LEN + 1];
        let mut huff_symbol_bits_lens2 = [0u8; super::LZ_ROID_SIZE];
        for huff_symbol_bits_lens in [&mut huff_symbol_bits_lens1[..], &mut huff_symbol_bits_lens2[..]].iter_mut() {
            for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                huff_symbol_bits_lens.xset(i * 2 + 0, tbuf.xget(tpos + i) / 16);
                huff_symbol_bits_lens.xset(i * 2 + 1, tbuf.xget(tpos + i) % 16);
            }
            tpos += huff_symbol_bits_lens.len() / 2;
        }

        let huff_decoder1 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens1);
        let huff_decoder2 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens2);
        let mut bits = Bits::new();
        for _ in 0 .. match_items_len {
            if bits.len() < 32 {
                bits.put(32, BE::read_u32(std::slice::from_raw_parts(tbuf.xget(tpos), 4)) as u64);
                tpos += 4;
            }
            let context1 = get_bucket_context(sbuf, spos - 1);
            let context0 = get_bucket_context(sbuf, spos);
            let bucket = self.buckets.xget_mut(context0);

            let leader = huff_decoder1.decode_from_bits(&mut bits);
            if leader < 256 {
                let mtf_symbol = leader as u8;
                sbuf.xset(spos, self.mtfs.xget_mut(context0).decode(mtf_symbol as u8));
                bucket.update(spos);
                spos += 1;
                let context_m3 = get_bucket_context(sbuf, spos - 3);
                let context_m2 = get_bucket_context(sbuf, spos - 2);
                self.last_words.xget_mut(context_m3).xset(context_m2, [*sbuf.xget(spos - 2), *sbuf.xget(spos - 1)]);

            } else if leader == 256 {
                let last_word = *self.last_words.xget(context1).xget(context0);
                sbuf.xset(spos + 0, *last_word.xget(0));
                sbuf.xset(spos + 1, *last_word.xget(1));
                bucket.update(spos);
                spos += 2;

            } else {
                let match_len = (leader - 258) as usize;
                let roid = huff_decoder2.decode_from_bits(&mut bits) as usize;
                Result::from_bool(
                    match_len >= super::LZ_MATCH_MIN_LEN &&
                    match_len <= super::LZ_MATCH_MAX_LEN &&
                    roid < super::LZ_ROID_SIZE, Err(()))?;

                let (robase, robitlen) = *LZ_ROID_DECODING_ARRAY.xget_mut(roid);
                let reduced_offset = robase + bits.get(robitlen) as u16;
                let match_pos = bucket.get_match_pos(reduced_offset);

                { // fast increment memcopy
                    let mut a = sbuf.as_ptr() as usize + match_pos;
                    let mut b = sbuf.as_ptr() as usize + spos;
                    let r = b + match_len;

                    while b - a < 4 {
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
                let context_m3 = get_bucket_context(sbuf, spos - 3);
                let context_m2 = get_bucket_context(sbuf, spos - 2);
                self.last_words.xget_mut(context_m3).xset(context_m2, [*sbuf.xget(spos - 2), *sbuf.xget(spos - 1)]);
            }

            if spos >= sbuf.len() {
                break;
            }
        }
        // (spos+match_len) may overflow, but it is safe because of sentinels
        Ok((std::cmp::min(spos, sbuf.len()), std::cmp::min(tpos, tbuf.len())))
    }
}

unsafe fn get_bucket_context(buf: &[u8], pos: usize) -> usize {
    return *buf.xget(pos - 1) as usize % super::LZ_CONTEXT_BUCKET_SIZE;
}
