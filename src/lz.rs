use byteorder::BE;
use byteorder::ByteOrder;
use super::aux::UncheckedSliceExt;
use super::bits::Bits;
use super::huffman::HuffmanDecoder;
use super::huffman::HuffmanEncoder;
use super::matchfinder::DecoderMFBucket;
use super::matchfinder::EncoderMFBucket;
use super::matchfinder::MatchResult;
use super::mtf::MTFCoder;

const LZ_ROID_ENCODING_ARRAY: [(u8, u8, u16); super::LZ_MF_BUCKET_ITEM_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_ENCODING_ARRAY.txt"));
const LZ_ROID_DECODING_ARRAY: [(u16, u8); super::LZ_ROID_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_DECODING_ARRAY.txt"));

pub struct LZCfg {
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}
pub struct LZEncoder {
    buckets:       Vec<EncoderMFBucket>,
    mtfs:          Vec<MTFCoder>,
    words:         [u16; 32768],
    first_literal: bool,
}

pub struct LZDecoder {
    buckets:       Vec<DecoderMFBucket>,
    mtfs:          Vec<MTFCoder>,
    words:         [u16; 32768],
    first_literal: bool,
}

pub enum MatchItem {
    Match  {mtf_roid: u16, robitlen: u8, robits: u16, encoded_match_len: u8},
    Symbol {mtf_symbol: u16},
}

impl LZEncoder {
    pub fn new() -> LZEncoder {
        return LZEncoder {
            buckets:       (0 .. 256).map(|_| EncoderMFBucket::new()).collect(),
            mtfs:          (0 .. 512).map(|_| MTFCoder::new()).collect(),
            words:         [0; 32768],
            first_literal: true,
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.buckets.iter_mut().for_each(|bucket| bucket.forward(forward_len));
    }

    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::with_capacity(super::LZ_CHUNK_SIZE);

        macro_rules! sc {
            ($off:expr) => (sbuf.nocheck()[(spos as isize + $off as isize) as usize])
        }
        macro_rules! sw {
            ($off:expr) => ((sc!($off - 1) as u16) << 8 | (sc!($off) as u16))
        }
        macro_rules! shc {
            ($off:expr) => ((sc!($off) & 0x7f) | (sc!($off - 1) & 0x40) << 1)
        }
        macro_rules! shw {
            ($off:expr) => ((sw!($off) & 0x7f7f) | (sc!($off - 2) as u16 & 0x40) << 1)
        }

        let mut huff_weights1 = [0u32; 360]; // assert!(MTF.value_array.max() < 360)
        let mut huff_weights2 = [0u32; 256]; // assert!(LZ_MATCH_MAX_LEN < 256)

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let last_word_expected = self.words.nocheck()[shw!(-1) as usize];
            let unlikely_symbol = (last_word_expected >> 8) as u16;
            let mtf = &mut self.mtfs.nocheck_mut()[(self.first_literal as usize) << 8 | shc!(-1) as usize];
            let match_result = self.buckets.nocheck()[shc!(-1) as usize].find_match(sbuf, spos, cfg.match_depth);

            // encode as match
            if let Some(MatchResult {reduced_offset, match_len, match_len_expected, match_len_min}) = match_result {
                let (roid, robitlen, robits) = LZ_ROID_ENCODING_ARRAY.nocheck()[reduced_offset as usize];
                let encoded_match_len =
                    if match_len_expected < match_len_min {
                        match_len - match_len_min
                    } else {
                        match match_len_expected.cmp(&match_len) {
                            std::cmp::Ordering::Equal   => 0,
                            std::cmp::Ordering::Greater => match_len - match_len_min + 1,
                            std::cmp::Ordering::Less    => match_len - match_len_min,
                        }
                    } as u8;
                let lazy_match_len1 = match_len + 1 + (robitlen < 8) as usize;
                let lazy_match_len2 = lazy_match_len1 - (self.words.nocheck()[shw!(-1) as usize] == sw!(1)) as usize;

                let use_match = spos + match_len < sbuf.len()
                    && !self.buckets.nocheck()[shc!(0) as usize].has_lazy_match(sbuf, spos + 1,
                            lazy_match_len1, cfg.lazy_match_depth1)
                    && !self.buckets.nocheck()[shc!(1) as usize].has_lazy_match(sbuf, spos + 2,
                            lazy_match_len2, cfg.lazy_match_depth2);
                if use_match {
                    let encoded_roid_match_len = roid as u16 * 5 + std::cmp::min(4, encoded_match_len as u16);
                    let mtf_roid = mtf.encode(257 + encoded_roid_match_len, unlikely_symbol);
                    match_items.push(MatchItem::Match {mtf_roid, robitlen, robits, encoded_match_len});
                    huff_weights1.nocheck_mut()[mtf_roid as usize] += 1;
                    if encoded_match_len >= 4 {
                        huff_weights2.nocheck_mut()[encoded_match_len as usize] += 1;
                    }

                    self.buckets.nocheck_mut()[shc!(-1) as usize].update(sbuf, spos, reduced_offset, match_len);
                    spos += match_len;
                    self.first_literal = false;
                    self.words.nocheck_mut()[shw!(-3) as usize] = sw!(-1);
                    continue;
                }
            }
            self.buckets.nocheck_mut()[shc!(-1) as usize].update(sbuf, spos, 0, 0);

            // encode as symbol
            let mtf_symbol;
            if spos + 1 < sbuf.len() && last_word_expected == sw!(1) {
                mtf_symbol = mtf.encode(256, unlikely_symbol);
                spos += 2;
                self.first_literal = false;
            } else {
                mtf_symbol = mtf.encode(sc!(0) as u16, unlikely_symbol);
                spos += 1;
                self.first_literal = true;
                self.words.nocheck_mut()[shw!(-3) as usize] = sw!(-1);
            }
            match_items.push(MatchItem::Symbol {mtf_symbol});
            huff_weights1[mtf_symbol as usize] += 1; // count huffman
        }

        // encode match_items_len
        BE::write_u32(std::slice::from_raw_parts_mut(tbuf.get_unchecked_mut(tpos), 4), match_items.len() as u32);
        tpos += 4;

        // start Huffman encoding
        let huff_encoder1 = HuffmanEncoder::from_symbol_weight_vec(&huff_weights1, 15);
        let huff_encoder2 = HuffmanEncoder::from_symbol_weight_vec(&huff_weights2, 15);
        let mut bits = Bits::new();
        for huff_symbol_bits_lens in &[huff_encoder1.get_symbol_bits_lens(), huff_encoder2.get_symbol_bits_lens()] {
            for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                tbuf.nocheck_mut()[tpos + i] =
                    huff_symbol_bits_lens.nocheck()[i * 2 + 0] * 16 +
                    huff_symbol_bits_lens.nocheck()[i * 2 + 1];
            }
            tpos += huff_symbol_bits_lens.len() / 2;
        }

        for match_item in &match_items {
            match *match_item {
                MatchItem::Symbol {mtf_symbol} => {
                    huff_encoder1.encode_to_bits(mtf_symbol, &mut bits);
                },
                MatchItem::Match {mtf_roid, robitlen, robits, encoded_match_len} => {
                    huff_encoder1.encode_to_bits(mtf_roid, &mut bits);
                    bits.put(robitlen, robits as u64);
                    if encoded_match_len >= 4 {
                        huff_encoder2.encode_to_bits(encoded_match_len as u16, &mut bits);
                    }
                }
            }
            if bits.len() >= 32 {
                BE::write_u32(std::slice::from_raw_parts_mut(tbuf.get_unchecked_mut(tpos), 4), bits.get(32) as u32);
                tpos += 4;
            }
            if bits.len() >= 16 {
                BE::write_u16(std::slice::from_raw_parts_mut(tbuf.get_unchecked_mut(tpos), 2), bits.get(16) as u16);
                tpos += 2;
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
            buckets:       (0 .. 256).map(|_| DecoderMFBucket::new()).collect(),
            mtfs:          (0 .. 512).map(|_| MTFCoder::new()).collect(),
            words:         [0; 32768],
            first_literal: true,
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.buckets.iter_mut().for_each(|bucket| bucket.forward(forward_len));
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut spos = spos;
        let mut tpos = 0;

        macro_rules! sc {
            ($off:expr) => (sbuf.nocheck()[(spos as isize + $off as isize) as usize])
        }
        macro_rules! sw {
            ($off:expr) => ((sc!($off - 1) as u16) << 8 | (sc!($off) as u16))
        }
        macro_rules! shc {
            ($off:expr) => ((sc!($off) & 0x7f) | (sc!($off - 1) & 0x40) << 1)
        }
        macro_rules! shw {
            ($off:expr) => ((sw!($off) & 0x7f7f) | (sc!($off - 2) as u16 & 0x40) << 1)
        }
        macro_rules! sc_set {
            ($off:expr, $c:expr) => (sbuf.nocheck_mut()[(spos as isize + $off as isize) as usize] = $c)
        }
        macro_rules! sw_set {
            ($off:expr, $w:expr) => {{
                sc_set!($off - 1, ($w >> 8) as u8);
                sc_set!($off - 0, ($w >> 0) as u8);
            }}
        }

        // decode match_items_len
        let match_items_len = BE::read_u32(std::slice::from_raw_parts(tbuf.get_unchecked(tpos), 4)) as usize;
        tpos += 4;

        // start decoding
        let mut huff_symbol_bits_lens1 = [0u8; 360];
        let mut huff_symbol_bits_lens2 = [0u8; 256];
        for huff_symbol_bits_lens in [&mut huff_symbol_bits_lens1[..], &mut huff_symbol_bits_lens2[..]].iter_mut() {
            for i in 0 .. huff_symbol_bits_lens.len() / 2 {
                huff_symbol_bits_lens.nocheck_mut()[i * 2 + 0] = tbuf.nocheck()[tpos + i] / 16;
                huff_symbol_bits_lens.nocheck_mut()[i * 2 + 1] = tbuf.nocheck()[tpos + i] % 16;
            }
            tpos += huff_symbol_bits_lens.len() / 2;
        }

        let huff_decoder1 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens1);
        let huff_decoder2 = HuffmanDecoder::from_symbol_bits_lens(&huff_symbol_bits_lens2);
        let mut bits = Bits::new();
        for _ in 0 .. match_items_len {
            let last_word_expected = self.words.nocheck()[shw!(-1) as usize];
            let unlikely_symbol = (last_word_expected >> 8) as u16;
            let mtf = &mut self.mtfs.nocheck_mut()[(self.first_literal as usize) << 8 | shc!(-1) as usize];

            if bits.len() < 32 {
                bits.put(32, BE::read_u32(std::slice::from_raw_parts(tbuf.as_ptr().add(tpos), 4)) as u64);
                tpos += 4;
            }
            if bits.len() < 48 {
                bits.put(16, BE::read_u16(std::slice::from_raw_parts(tbuf.as_ptr().add(tpos), 2)) as u64);
                tpos += 2;
            }

            match mtf.decode(huff_decoder1.decode_from_bits(&mut bits), unlikely_symbol) {
                256 => {
                    sw_set!(1, last_word_expected);
                    self.buckets.nocheck_mut()[shc!(-1) as usize].update(spos, 0, 0);
                    spos += 2;
                    self.first_literal = false;
                }
                symbol @ 0 ..= 255 => {
                    sc_set!(0, symbol as u8);
                    self.buckets.nocheck_mut()[shc!(-1) as usize].update(spos, 0, 0);
                    spos += 1;
                    self.words.nocheck_mut()[shw!(-3) as usize] = sw!(-1);
                    self.first_literal = true;
                }
                roid_plus_257 if roid_plus_257 as usize - 257 < super::LZ_ROID_SIZE * 5 => {
                    let encoded_roid_match_len = roid_plus_257 - 257;

                    // get reduced offset
                    let roid     = encoded_roid_match_len as usize / 5;
                    let robase   = LZ_ROID_DECODING_ARRAY.nocheck()[roid].0;
                    let robitlen = LZ_ROID_DECODING_ARRAY.nocheck()[roid].1;
                    let robits   = bits.get(robitlen);
                    let reduced_offset = robase as usize + robits as usize;

                    // get match_pos/match_len
                    let encoded_match_len = match encoded_roid_match_len as usize % 5 {
                        x if x < 4 => x,
                        _ => huff_decoder2.decode_from_bits(&mut bits) as usize,
                    };
                    let (
                        match_pos,
                        match_len_expected,
                        match_len_min,
                    ) = self.buckets.nocheck()[shc!(-1) as usize].get_match_pos_and_match_len(reduced_offset as u16);

                    let match_len =
                        if match_len_expected < match_len_min {
                            encoded_match_len + match_len_min
                        } else {
                            match encoded_match_len {
                                0 => match_len_expected,
                                l if l + match_len_min <= match_len_expected => encoded_match_len + match_len_min - 1,
                                l if l + match_len_min >  match_len_expected => encoded_match_len + match_len_min,
                                _ => unreachable!(),
                            }
                        };
                    super::mem::copy_fast(sbuf, match_pos, spos, match_len);
                    self.buckets.nocheck_mut()[shc!(-1) as usize].update(spos, reduced_offset, match_len);
                    spos += match_len;
                    self.words.nocheck_mut()[shw!(-3) as usize] = sw!(-1);
                    self.first_literal = false;
                }
                _ => Err(())?
            }

            if spos >= sbuf.len() {
                break;
            }
        }
        // (spos+match_len) may overflow, but it is safe because of sentinels
        return Ok((std::cmp::min(spos, sbuf.len()), std::cmp::min(tpos, tbuf.len())));
    }
}
