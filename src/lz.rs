use super::auxility::UncheckedSliceExt;
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

macro_rules! define_coder_type {
    ($CoderType:ident, $BucketType:ty) => {
        pub struct $CoderType {
            buckets:       Vec<$BucketType>,
            mtfs:          Vec<MTFCoder>,
            words:         Vec<u16>,
            after_literal: bool,
        }

        impl $CoderType {
            pub fn new() -> $CoderType {
                return $CoderType {
                    buckets:       (0 .. 256).map(|_| <$BucketType>::new()).collect(),
                    mtfs:          (0 .. 512).map(|_| MTFCoder::new()).collect(),
                    words:         vec![0; 32768],
                    after_literal: true,
                };
            }

            pub fn forward(&mut self, forward_len: usize) {
                self.buckets.iter_mut().for_each(|bucket| bucket.forward(forward_len));
            }
        }
    }
}
define_coder_type!(LZEncoder, EncoderMFBucket);
define_coder_type!(LZDecoder, DecoderMFBucket);

impl LZEncoder {
    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        enum MatchItem {
            Match  {symbol: u16, mtf_context: u16, mtf_unlikely: u8, robitlen: u8, robits: u16, encoded_match_len: u8},
            Symbol {symbol: u16, mtf_context: u16, mtf_unlikely: u8},
        }
        let mut bits: Bits = Default::default();
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
            ($off:expr) => (((sc!($off) & 0x7f) | (sc!($off - 1) & 0x40) << 1) as usize)
        }
        macro_rules! shw {
            ($off:expr) => (((sw!($off) & 0x7f7f) | (sc!($off - 2) as u16 & 0x40) << 1) as usize)
        }

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let last_word_expected = self.words.nocheck()[shw!(-1)];
            let mtf_context = (self.after_literal as u16) << 8 | shc!(-1) as u16;
            let mtf_unlikely = (last_word_expected >> 8) as u8;

            // encode as match
            let match_result = self.buckets.nocheck()[shc!(-1)].find_match(sbuf, spos, cfg.match_depth);
            if let Some(MatchResult {reduced_offset, match_len, match_len_expected, match_len_min}) = match_result {
                let (roid, robitlen, robits) = LZ_ROID_ENCODING_ARRAY.nocheck()[reduced_offset as usize];
                let lazy_len1 = match_len + 1 + (robitlen < 8) as usize;
                let lazy_len2 = lazy_len1 - (self.words.nocheck()[shw!(-1)] == sw!(1)) as usize;

                if spos + match_len < sbuf.len()
                    && !self.buckets.nocheck()[shc!(0)].has_lazy_match(sbuf, spos + 1, lazy_len1, cfg.lazy_match_depth1)
                    && !self.buckets.nocheck()[shc!(1)].has_lazy_match(sbuf, spos + 2, lazy_len2, cfg.lazy_match_depth2)
                {
                    let encoded_match_len = match match_len.cmp(&match_len_expected) {
                        std::cmp::Ordering::Greater => match_len - match_len_min,
                        std::cmp::Ordering::Less    => match_len - match_len_min + 1,
                        std::cmp::Ordering::Equal   => 0,
                    } as u8;
                    match_items.push(MatchItem::Match {
                        symbol: 257 + roid as u16 * 5 + std::cmp::min(4, encoded_match_len as u16),
                        mtf_context,
                        mtf_unlikely,
                        robitlen,
                        robits,
                        encoded_match_len,
                    });
                    self.buckets.nocheck_mut()[shc!(-1)].update(sbuf, spos, reduced_offset, match_len);
                    spos += match_len;
                    self.after_literal = false;
                    self.words.nocheck_mut()[shw!(-3)] = sw!(-1);
                    continue;
                }
            }
            self.buckets.nocheck_mut()[shc!(-1)].update(sbuf, spos, 0, 0);

            // encode as symbol
            if last_word_expected == sw!(1) {
                match_items.push(MatchItem::Symbol {symbol: 256, mtf_context, mtf_unlikely});
                spos += 2;
                self.after_literal = false;
            } else {
                match_items.push(MatchItem::Symbol {symbol: sc!(0) as u16, mtf_context, mtf_unlikely});
                spos += 1;
                self.after_literal = true;
                self.words.nocheck_mut()[shw!(-3)] = sw!(-1);
            }
        }

        // encode match_items_len
        bits.put(32, match_items.len() as u64);
        bits.save_u32(tbuf, &mut tpos);

        // perform mtf transform
        for match_item in &mut match_items {
            match match_item {
                &mut MatchItem::Match  {ref mut symbol, mtf_context, mtf_unlikely, ..} |
                &mut MatchItem::Symbol {ref mut symbol, mtf_context, mtf_unlikely, ..} => {
                    *symbol = self.mtfs.nocheck_mut()[mtf_context as usize].encode(*symbol, mtf_unlikely as u16);
                }
            }
        }

        // start Huffman encoding
        let mut huff_weights1 = [0u32; super::mtf::MTF_NUM_SYMBOLS + super::mtf::MTF_NUM_SYMBOLS % 2];
        let mut huff_weights2 = [0u32; super::LZ_MATCH_MAX_LEN + super::LZ_MATCH_MAX_LEN % 2];
        for match_item in &match_items {
            match match_item {
                &MatchItem::Symbol {symbol, ..} => {
                    huff_weights1.nocheck_mut()[symbol as usize] += 1;
                },
                &MatchItem::Match {symbol, encoded_match_len, ..} => {
                    huff_weights1.nocheck_mut()[symbol as usize] += 1;
                    if encoded_match_len >= 4 {
                        huff_weights2.nocheck_mut()[encoded_match_len as usize] += 1;
                    }
                }
            }
        }
        let huff_encoder1 = HuffmanEncoder::from_symbol_weights(&huff_weights1, 15);
        let huff_encoder2 = HuffmanEncoder::from_symbol_weights(&huff_weights2, 15);
        for huff_canonical_lens in &[huff_encoder1.get_canonical_lens(), huff_encoder2.get_canonical_lens()] {
            for i in 0 .. huff_canonical_lens.len() / 2 {
                tbuf.nocheck_mut()[tpos + i] =
                    huff_canonical_lens.nocheck()[i * 2 + 0] * 16 +
                    huff_canonical_lens.nocheck()[i * 2 + 1];
            }
            tpos += huff_canonical_lens.len() / 2;
        }

        for match_item in &match_items {
            match match_item {
                &MatchItem::Symbol {symbol, ..} => {
                    huff_encoder1.encode_to_bits(symbol, &mut bits);
                },
                &MatchItem::Match {symbol, robitlen, robits, encoded_match_len, ..} => {
                    huff_encoder1.encode_to_bits(symbol, &mut bits);
                    bits.put(robitlen, robits as u64);
                    if encoded_match_len >= 4 {
                        huff_encoder2.encode_to_bits(encoded_match_len as u16, &mut bits);
                        bits.save_u32(tbuf, &mut tpos);
                    }
                }
            }
            bits.save_u32(tbuf, &mut tpos);
        }
        bits.save_all(tbuf, &mut tpos);
        return (spos, tpos);
    }
}

impl LZDecoder {
    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut bits: Bits = Default::default();
        let mut spos = spos;
        let mut tpos = 0;

        macro_rules! sc {
            ($off:expr) => (sbuf.nocheck()[(spos as isize + $off as isize) as usize])
        }
        macro_rules! sw {
            ($off:expr) => ((sc!($off - 1) as u16) << 8 | (sc!($off) as u16))
        }
        macro_rules! shc {
            ($off:expr) => (((sc!($off) & 0x7f) | (sc!($off - 1) & 0x40) << 1) as usize)
        }
        macro_rules! shw {
            ($off:expr) => (((sw!($off) & 0x7f7f) | (sc!($off - 2) as u16 & 0x40) << 1) as usize)
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
        bits.load_u32(tbuf, &mut tpos);
        let match_items_len = bits.get(32) as usize;

        // start decoding
        let mut huff_canonical_lens1 = [0u8; super::mtf::MTF_NUM_SYMBOLS + super::mtf::MTF_NUM_SYMBOLS % 2];
        let mut huff_canonical_lens2 = [0u8; super::LZ_MATCH_MAX_LEN + super::LZ_MATCH_MAX_LEN % 2];
        for huff_canonical_lens in [&mut huff_canonical_lens1[..], &mut huff_canonical_lens2[..]].iter_mut() {
            for i in 0 .. huff_canonical_lens.len() / 2 {
                huff_canonical_lens.nocheck_mut()[i * 2 + 0] = tbuf.nocheck()[tpos + i] / 16;
                huff_canonical_lens.nocheck_mut()[i * 2 + 1] = tbuf.nocheck()[tpos + i] % 16;
            }
            tpos += huff_canonical_lens.len() / 2;
        }

        let huff_decoder1 = HuffmanDecoder::from_canonical_lens(&huff_canonical_lens1);
        let huff_decoder2 = HuffmanDecoder::from_canonical_lens(&huff_canonical_lens2);
        for _ in 0 .. match_items_len {
            let last_word_expected = self.words.nocheck()[shw!(-1)];
            let unlikely_symbol = (last_word_expected >> 8) as u16;
            let mtf = &mut self.mtfs.nocheck_mut()[(self.after_literal as usize) << 8 | shc!(-1)];

            bits.load_u32(tbuf, &mut tpos);
            match mtf.decode(huff_decoder1.decode_from_bits(&mut bits), unlikely_symbol) {
                256 => {
                    sw_set!(1, last_word_expected);
                    self.buckets.nocheck_mut()[shc!(-1)].update(spos, 0, 0);
                    spos += 2;
                    self.after_literal = false;
                }
                symbol @ 0 ..= 255 => {
                    sc_set!(0, symbol as u8);
                    self.buckets.nocheck_mut()[shc!(-1)].update(spos, 0, 0);
                    spos += 1;
                    self.words.nocheck_mut()[shw!(-3)] = sw!(-1);
                    self.after_literal = true;
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
                        _ => {
                            bits.load_u32(tbuf, &mut tpos);
                            huff_decoder2.decode_from_bits(&mut bits) as usize
                        }
                    };
                    let (
                        match_pos,
                        match_len_expected,
                        match_len_min,
                    ) = self.buckets.nocheck()[shc!(-1)].get_match_pos_and_match_len(reduced_offset as u16);

                    let match_len = match encoded_match_len {
                        l if l + match_len_min > match_len_expected => l + match_len_min,
                        l if l > 0 => encoded_match_len + match_len_min - 1,
                        _ => match_len_expected,
                    };
                    super::mem::copy_fast(sbuf, match_pos, spos, match_len);
                    self.buckets.nocheck_mut()[shc!(-1)].update(spos, reduced_offset, match_len);
                    spos += match_len;
                    self.words.nocheck_mut()[shw!(-3)] = sw!(-1);
                    self.after_literal = false;
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
