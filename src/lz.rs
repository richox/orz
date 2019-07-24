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
            words:         Vec<(u8, u8)>,
            after_literal: bool,
        }

        impl $CoderType {
            pub fn new() -> $CoderType {
                return $CoderType {
                    buckets:       (0 .. 256).map(|_| <$BucketType>::new()).collect(),
                    mtfs:          (0 .. 512).map(|_| MTFCoder::new()).collect(),
                    words:         vec![(0, 0); 32768],
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

const WORD_SYMBOL: u16 = super::mtf::MTF_NUM_SYMBOLS as u16 - 1;

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

        let sc  = |pos| (sbuf.nc()[pos]);
        let sw  = |pos| (sbuf.nc()[pos - 1], sbuf.nc()[pos]);
        let shc = |pos| sc(pos) as usize & 0x7f | (sc(pos - 1) as usize & 0x40) << 1;
        let shw = |pos| sc(pos) as usize & 0x7f | shc(pos - 1) << 7;

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let last_word_expected = self.words.nc()[shw(spos - 1)];
            let mtf_context = (self.after_literal as u16) << 8 | shc(spos - 1) as u16;
            let mtf_unlikely = last_word_expected.0;

            // encode as match
            let mut lazy_match_rets = (false, false);
            let match_result = self.buckets.nc()[shc(spos - 1)].find_match(sbuf, spos, cfg.match_depth);
            if let Some(MatchResult {reduced_offset, match_len, match_len_expected, match_len_min}) = match_result {
                let (roid, robitlen, robits) = LZ_ROID_ENCODING_ARRAY.nc()[reduced_offset as usize];
                let lazy_len1 = match_len + 1 + (robitlen < 8) as usize;
                let lazy_len2 = lazy_len1 - (self.words.nc()[shw(spos - 1)] == sw(spos + 1)) as usize;

                lazy_match_rets.0 =
                    self.buckets.nc()[shc(spos + 0)].has_lazy_match(sbuf, spos + 1, lazy_len1, cfg.lazy_match_depth1);
                lazy_match_rets.1 = !lazy_match_rets.0 &&
                    self.buckets.nc()[shc(spos + 1)].has_lazy_match(sbuf, spos + 2, lazy_len2, cfg.lazy_match_depth2);

                if !lazy_match_rets.0 && !lazy_match_rets.1 {
                    let encoded_match_len = match match_len.cmp(&match_len_expected) {
                        std::cmp::Ordering::Greater => match_len - match_len_min,
                        std::cmp::Ordering::Less    => match_len - match_len_min + 1,
                        std::cmp::Ordering::Equal   => 0,
                    } as u8;
                    match_items.push(MatchItem::Match {
                        symbol: 256 + roid as u16 * 5 + std::cmp::min(4, encoded_match_len as u16),
                        mtf_context,
                        mtf_unlikely,
                        robitlen,
                        robits,
                        encoded_match_len,
                    });
                    self.buckets.nc_mut()[shc(spos - 1)].update(sbuf, spos, reduced_offset, match_len);
                    spos += match_len;
                    self.after_literal = false;
                    self.words.nc_mut()[shw(spos - 3)] = sw(spos - 1);
                    continue;
                }
            }
            self.buckets.nc_mut()[shc(spos - 1)].update(sbuf, spos, 0, 0);

            // encode as symbol
            if !lazy_match_rets.0 && last_word_expected == sw(spos + 1) {
                match_items.push(MatchItem::Symbol {symbol: WORD_SYMBOL, mtf_context, mtf_unlikely});
                spos += 2;
                self.after_literal = false;
            } else {
                match_items.push(MatchItem::Symbol {symbol: sc(spos) as u16, mtf_context, mtf_unlikely});
                spos += 1;
                self.after_literal = true;
                self.words.nc_mut()[shw(spos - 3)] = sw(spos - 1);
            }
        }

        // encode match_items_len
        bits.put(32, std::cmp::min(spos, sbuf.len()) as u64);
        bits.put(32, match_items.len() as u64);
        bits.save_u32(tbuf, &mut tpos);
        bits.save_u32(tbuf, &mut tpos);

        // perform mtf transform
        match_items.iter_mut().for_each(|match_item| match match_item {
            &mut MatchItem::Match  {ref mut symbol, mtf_context, mtf_unlikely, ..} |
            &mut MatchItem::Symbol {ref mut symbol, mtf_context, mtf_unlikely, ..} => {
                *symbol = self.mtfs.nc_mut()[mtf_context as usize].encode(*symbol, mtf_unlikely as u16);
            }
        });

        // start Huffman encoding
        let mut huff_weights1 = [0u32; super::mtf::MTF_NUM_SYMBOLS];
        let mut huff_weights2 = [0u32; super::LZ_MATCH_MAX_LEN];
        match_items.iter().for_each(|match_item| match match_item {
            &MatchItem::Symbol {symbol, ..} => {
                huff_weights1.nc_mut()[symbol as usize] += 1;
            },
            &MatchItem::Match {symbol, encoded_match_len, ..} => {
                huff_weights1.nc_mut()[symbol as usize] += 1;
                huff_weights2.nc_mut()[encoded_match_len as usize] += (encoded_match_len >= 4) as u32;
            }
        });

        let huff_encoder1 = HuffmanEncoder::new(&huff_weights1, 15, tbuf, &mut tpos);
        let huff_encoder2 = HuffmanEncoder::new(&huff_weights2, 15, tbuf, &mut tpos);
        match_items.iter().for_each(|match_item| match match_item {
            &MatchItem::Symbol {symbol, ..} => {
                huff_encoder1.encode_to_bits(symbol, &mut bits);
                bits.save_u32(tbuf, &mut tpos);
            },
            &MatchItem::Match {symbol, robitlen, robits, encoded_match_len, ..} => {
                huff_encoder1.encode_to_bits(symbol, &mut bits);
                bits.put(robitlen, robits as u64);
                bits.save_u32(tbuf, &mut tpos);
                if encoded_match_len >= 4 {
                    huff_encoder2.encode_to_bits(encoded_match_len as u16, &mut bits);
                    bits.save_u32(tbuf, &mut tpos);
                }
            }
        });
        bits.save_all(tbuf, &mut tpos);
        return (spos, tpos);
    }
}

impl LZDecoder {
    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let mut bits: Bits = Default::default();
        let mut spos = spos;
        let mut tpos = 0;

        let sbuf_unsafe = std::slice::from_raw_parts_mut(sbuf.as_ptr() as *mut u8, 0);
        let sc  = |pos| (sbuf_unsafe.nc()[pos as usize]);
        let sw  = |pos| (sbuf_unsafe.nc()[pos as usize - 1], sbuf_unsafe.nc()[pos as usize]);
        let shc = |pos| sc(pos) as usize & 0x7f | (sc(pos - 1) as usize & 0x40) << 1;
        let shw = |pos| sc(pos) as usize & 0x7f | shc(pos - 1) << 7;

        // decode sbuf_len/match_items_len
        bits.load_u32(tbuf, &mut tpos);
        bits.load_u32(tbuf, &mut tpos);
        let sbuf_len = bits.get(32) as usize;
        let match_items_len = bits.get(32) as usize;

        // start decoding
        let huff_decoder1 = HuffmanDecoder::new(super::mtf::MTF_NUM_SYMBOLS, tbuf, &mut tpos);
        let huff_decoder2 = HuffmanDecoder::new(super::LZ_MATCH_MAX_LEN, tbuf, &mut tpos);
        for _ in 0 .. match_items_len {
            let last_word_expected = self.words.nc()[shw(spos - 1)];
            let mtf = &mut self.mtfs.nc_mut()[(self.after_literal as usize) << 8 | shc(spos - 1)];
            let mtf_unlikely = last_word_expected.0;

            bits.load_u32(tbuf, &mut tpos);
            match mtf.decode(huff_decoder1.decode_from_bits(&mut bits), mtf_unlikely as u16) {
                WORD_SYMBOL => {
                    sbuf.nc_mut()[spos + 0] = last_word_expected.0;
                    sbuf.nc_mut()[spos + 1] = last_word_expected.1;
                    self.buckets.nc_mut()[shc(spos - 1)].update(sbuf, spos, 0, 0);
                    spos += 2;
                    self.after_literal = false;
                }
                symbol @ 0 ..= 255 => {
                    sbuf.nc_mut()[spos] = symbol as u8;
                    self.buckets.nc_mut()[shc(spos - 1)].update(sbuf, spos, 0, 0);
                    spos += 1;
                    self.words.nc_mut()[shw(spos - 3)] = sw(spos - 1);
                    self.after_literal = true;
                }
                roid_plus_256 if roid_plus_256 as usize - 256 < super::LZ_ROID_SIZE * 5 => {
                    let encoded_roid_match_len = roid_plus_256 - 256;

                    // get reduced offset
                    let roid     = encoded_roid_match_len as usize / 5;
                    let robase   = LZ_ROID_DECODING_ARRAY.nc()[roid].0;
                    let robitlen = LZ_ROID_DECODING_ARRAY.nc()[roid].1;
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
                    ) = self.buckets.nc()[shc(spos - 1)].get_match_pos_and_match_len(reduced_offset as u16);

                    let match_len = match encoded_match_len {
                        l if l + match_len_min > match_len_expected => l + match_len_min,
                        l if l > 0 => encoded_match_len + match_len_min - 1,
                        _ => match_len_expected,
                    };
                    super::mem::copy_fast(sbuf, match_pos, spos, match_len);
                    self.buckets.nc_mut()[shc(spos - 1)].update(sbuf, spos, reduced_offset, match_len);
                    spos += match_len;
                    self.words.nc_mut()[shw(spos - 3)] = sw(spos - 1);
                    self.after_literal = false;
                }
                _ => Err(())?
            }
        }
        return Ok((std::cmp::min(spos, sbuf_len), std::cmp::min(tpos, tbuf.len())));
    }
}
