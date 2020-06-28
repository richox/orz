use super::byteslice::ByteSliceExt;
use super::bits::Bits;
use super::huffman::HuffmanDecoder;
use super::huffman::HuffmanEncoder;
use super::matchfinder::Bucket;
use super::matchfinder::BucketMatcher;
use super::matchfinder::MatchResult;
use super::symrank::SymRankCoder;

const LZ_ROID_ENCODING_ARRAY: [(u8, u8, u16); super::LZ_MF_BUCKET_ITEM_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_ENCODING_ARRAY.txt"));
const LZ_ROID_DECODING_ARRAY: [(u16, u8); super::LZ_ROID_SIZE] = include!(
    concat!(env!("OUT_DIR"), "/", "LZ_ROID_DECODING_ARRAY.txt"));

const WORD_SYMBOL: u16 = super::SYMRANK_NUM_SYMBOLS as u16 - 1;

/// Limpel-Ziv matching options.
#[repr(C)]
pub struct LZCfg {
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}

struct LZContext {
    buckets:       Vec<Bucket>,
    symranks:      Vec<SymRankCoder>,
    words:         Vec<u16>,
    first_block:   bool,
    after_literal: bool,

} impl LZContext {
    pub fn new() -> LZContext {
        return LZContext {
            buckets:       vec![Bucket::new(); 256],
            symranks:      vec![SymRankCoder::new(); 512],
            words:         vec![0; 32768],
            first_block:   true,
            after_literal: true,
        };
    }
}

pub struct LZEncoder {
    ctx: LZContext,
    bucket_matchers: Vec<BucketMatcher>,

} impl LZEncoder {
    pub fn new() -> LZEncoder {
        return LZEncoder {
            ctx: LZContext::new(),
            bucket_matchers: (0..256).map(|_| BucketMatcher::new()).collect(),
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        for i in 0..self.bucket_matchers.len() {
            self.ctx.buckets[i].forward(forward_len);
            self.bucket_matchers[i].forward(&self.ctx.buckets[i]);
        }
    }

    pub unsafe fn encode(&mut self, cfg: &LZCfg, sbuf: &[u8], tbuf: &mut [u8], spos: usize) -> (usize, usize) {
        let roid_encoding_array = &unchecked_index::unchecked_index(&LZ_ROID_ENCODING_ARRAY);
        let sbuf = &unchecked_index::unchecked_index(sbuf);
        let tbuf = &mut unchecked_index::unchecked_index(tbuf);
        let self_bucket_matchers = &mut unchecked_index::unchecked_index(&mut self.bucket_matchers);
        let self_ctx_words = &mut unchecked_index::unchecked_index(&mut self.ctx.words);
        let self_ctx_buckets = &mut unchecked_index::unchecked_index(&mut self.ctx.buckets);
        let self_ctx_symranks = &mut unchecked_index::unchecked_index(&mut self.ctx.symranks);

        enum MatchItem {
            Match  {symbol: u16, symrank_context: u16, symrank_unlikely: u8, robitlen: u8, robits: u16, encoded_match_len: u8},
            Symbol {symbol: u16, symrank_context: u16, symrank_unlikely: u8},
        }
        let mut bits: Bits = Default::default();
        let mut spos = spos;
        let mut tpos = 0;
        let mut match_items = Vec::with_capacity(super::LZ_CHUNK_SIZE);

        let get_word = |pos| u16::from_be_bytes([sbuf[pos], sbuf[pos + 1]]);
        let shc = |pos| sbuf[pos] as usize & 0x7f | (u8::is_ascii_alphanumeric(&sbuf[pos - 1]) as usize) << 7;
        let shw = |pos| sbuf[pos] as usize & 0x7f | shc(pos - 1) << 7;

        // start Lempel-Ziv encoding
        while spos < sbuf.len() && match_items.len() < match_items.capacity() {
            let last_word_expected = self_ctx_words[shw(spos - 1)];
            let last_word_matched = get_word(spos) == last_word_expected;
            let symrank_context = u16::from_be_bytes([self.ctx.after_literal as u8, shc(spos - 1) as u8]);
            let symrank_unlikely = last_word_expected.to_be_bytes()[0];

            // encode as match
            let mut lazy_match_id = 0;
            let match_result = self_bucket_matchers[shc(spos - 1)].find_match(
                &self_ctx_buckets[shc(spos - 1)],
                sbuf,
                spos,
                cfg.match_depth);

            if let MatchResult::Matched {reduced_offset, match_len, match_len_expected, match_len_min} = match_result {
                let (roid, robitlen, robits) = roid_encoding_array[reduced_offset as usize];

                // find lazy match
                if match_len < super::LZ_MATCH_MAX_LEN / 2 {
                    let lazy_len1 = match_len + 1 + (robitlen < 8) as usize;
                    let lazy_len2 = lazy_len1 - last_word_matched as usize;
                    let has_lazy_match = |pos, lazy_len, match_depth| {
                        let lazy_bucket_matcher = &self_bucket_matchers[shc(pos)];
                        let lazy_bucket = &self_ctx_buckets[shc(pos)];
                        return lazy_bucket_matcher.has_lazy_match(lazy_bucket, sbuf, pos + 1, lazy_len, match_depth);
                    };
                    lazy_match_id = match () {
                        _ if has_lazy_match(spos + 0, lazy_len1, cfg.lazy_match_depth1) => 1,
                        _ if has_lazy_match(spos + 1, lazy_len2, cfg.lazy_match_depth2) => 2,
                        _ => 0,
                    };
                }

                if lazy_match_id == 0 {
                    let encoded_match_len = match match_len.cmp(&match_len_expected) {
                        std::cmp::Ordering::Greater => match_len - match_len_min,
                        std::cmp::Ordering::Less    => match_len - match_len_min + 1,
                        std::cmp::Ordering::Equal   => 0,
                    } as u8;
                    let lenid = std::cmp::min(super::LZ_LENID_SIZE as u8 - 1, encoded_match_len);
                    let encoded_roid_lenid = 256 + roid as u16 * super::LZ_LENID_SIZE as u16 + lenid as u16;
                    match_items.push(MatchItem::Match {
                        symbol: encoded_roid_lenid,
                        symrank_context, symrank_unlikely, robitlen, robits, encoded_match_len,
                    });

                    self_ctx_buckets[shc(spos - 1)].update(spos, reduced_offset, match_len);
                    self_bucket_matchers[shc(spos - 1)].update(&self_ctx_buckets[shc(spos - 1)], sbuf, spos);
                    spos += match_len;
                    self.ctx.after_literal = false;
                    self_ctx_words[shw(spos - 3)] = get_word(spos - 2);
                    continue;
                }
            }
            self_ctx_buckets[shc(spos - 1)].update(spos, 0, 0);
            self_bucket_matchers[shc(spos - 1)].update(&self_ctx_buckets[shc(spos - 1)], sbuf, spos);

            // encode as symbol
            if spos + 1 < sbuf.len() && lazy_match_id != 1 && last_word_matched {
                match_items.push(MatchItem::Symbol {symbol: WORD_SYMBOL, symrank_context, symrank_unlikely});
                spos += 2;
                self.ctx.after_literal = false;
            } else {
                match_items.push(MatchItem::Symbol {symbol: sbuf[spos] as u16, symrank_context, symrank_unlikely});
                spos += 1;
                self.ctx.after_literal = true;
                self_ctx_words[shw(spos - 3)] = get_word(spos - 2);
            }
        }

        // init symrank array
        if self.ctx.first_block {
            let symbol_counts = &mut [0; super::SYMRANK_NUM_SYMBOLS];
            match_items.iter().for_each(|match_item| match match_item {
                &MatchItem::Match {symbol, ..} | &MatchItem::Symbol {symbol, ..} => {
                    symbol_counts[symbol as usize] += 1;
                }
            });

            let vs = (0 .. super::SYMRANK_NUM_SYMBOLS)
                .map(|i| (-symbol_counts[i], i))
                .collect::<std::collections::BTreeSet<_>>().iter()
                .map(|(_count, i)| {
                    let symbol = *i as u16;
                    tbuf.write_forward(&mut tpos, symbol.to_le());
                    return symbol;
                })
                .collect::<Vec<_>>();

            self_ctx_symranks.iter_mut().for_each(|symrank| symrank.init(&vs));
            self.ctx.first_block = false;
        }

        // encode match_items_len
        bits.put(32, std::cmp::min(spos, sbuf.len()) as u64);
        bits.put(32, match_items.len() as u64);
        bits.save_u32(tbuf, &mut tpos);
        bits.save_u32(tbuf, &mut tpos);

        // start Huffman encoding
        let mut huff_weights1 = [0u32; super::SYMRANK_NUM_SYMBOLS];
        let mut huff_weights2 = [0u32; super::LZ_MATCH_MAX_LEN];
        match_items.iter_mut().for_each(|match_item| match match_item {
            &mut MatchItem::Match  {ref mut symbol, symrank_context, symrank_unlikely, encoded_match_len, ..} => {
                *symbol = self_ctx_symranks[symrank_context as usize].encode(*symbol, symrank_unlikely as u16);
                unchecked_index::unchecked_index(&mut huff_weights1)[*symbol as usize] += 1;
                unchecked_index::unchecked_index(&mut huff_weights2)[encoded_match_len as usize] +=
                    (encoded_match_len as usize >= super::LZ_LENID_SIZE - 1) as u32;
            }
            &mut MatchItem::Symbol {ref mut symbol, symrank_context, symrank_unlikely, ..} => {
                *symbol = self_ctx_symranks[symrank_context as usize].encode(*symbol, symrank_unlikely as u16);
                unchecked_index::unchecked_index(&mut huff_weights1)[*symbol as usize] += 1;
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
                if encoded_match_len as usize >= super::LZ_LENID_SIZE - 1 {
                    huff_encoder2.encode_to_bits(encoded_match_len as u16, &mut bits);
                    bits.save_u32(tbuf, &mut tpos);
                }
            }
        });
        bits.save_all(tbuf, &mut tpos);
        return (spos, tpos);
    }
}

pub struct LZDecoder {
    ctx: LZContext,

} impl LZDecoder {
    pub fn new() -> LZDecoder {
        return LZDecoder {
            ctx: LZContext::new(),
        };
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.ctx.buckets.iter_mut().for_each(|bucket| bucket.forward(forward_len));
    }

    pub unsafe fn decode(&mut self, tbuf: &[u8], sbuf: &mut [u8], spos: usize) -> Result<(usize, usize), ()> {
        let roid_decoding_array = &unchecked_index::unchecked_index(&LZ_ROID_DECODING_ARRAY);
        let sbuf = &mut unchecked_index::unchecked_index(sbuf);
        let tbuf = &unchecked_index::unchecked_index(tbuf);
        let self_ctx_words = &mut unchecked_index::unchecked_index(&mut self.ctx.words);
        let self_ctx_buckets = &mut unchecked_index::unchecked_index(&mut self.ctx.buckets);
        let self_ctx_symranks = &mut unchecked_index::unchecked_index(&mut self.ctx.symranks);

        let mut bits: Bits = Default::default();
        let mut spos = spos;
        let mut tpos = 0;

        let get_word = |pos| u16::from_be_bytes([sbuf[pos], sbuf[pos + 1]]);
        let shc = |pos| sbuf[pos] as usize & 0x7f | (u8::is_ascii_alphanumeric(&sbuf[pos - 1]) as usize) << 7;
        let shw = |pos| sbuf[pos] as usize & 0x7f | shc(pos - 1) << 7;

        // init symrank array
        if self.ctx.first_block {
            let vs = (0..super::SYMRANK_NUM_SYMBOLS)
                .map(|_| tbuf.read_forward(&mut tpos))
                .map(u16::from_le)
                .collect::<Vec<_>>();
            self_ctx_symranks.iter_mut().for_each(|symrank| symrank.init(&vs));
            self.ctx.first_block = false;
        }

        // decode sbuf_len/match_items_len
        let sbuf = std::slice::from_raw_parts_mut(sbuf.as_ptr() as *mut u8, 0);
        bits.load_u32(tbuf, &mut tpos);
        bits.load_u32(tbuf, &mut tpos);
        let sbuf_len = bits.get(32) as usize;
        let match_items_len = bits.get(32) as usize;

        // start decoding
        let huff_decoder1 = HuffmanDecoder::new(super::SYMRANK_NUM_SYMBOLS, tbuf, &mut tpos);
        let huff_decoder2 = HuffmanDecoder::new(super::LZ_MATCH_MAX_LEN, tbuf, &mut tpos);
        for _ in 0 .. match_items_len {
            let last_word_expected = self_ctx_words[shw(spos - 1)];
            let symrank_context = u16::from_be_bytes([self.ctx.after_literal as u8, shc(spos - 1) as u8]);
            let symrank = &mut self_ctx_symranks[symrank_context as usize];
            let symrank_unlikely = last_word_expected.to_be_bytes()[0];

            bits.load_u32(tbuf, &mut tpos);
            let symbol = huff_decoder1.decode_from_bits(&mut bits);
            if !(0 ..= super::SYMRANK_NUM_SYMBOLS as u16).contains(&symbol) {
                Err(())?;
            }

            match symrank.decode(symbol, symrank_unlikely as u16) {
                WORD_SYMBOL => {
                    self_ctx_buckets[shc(spos - 1)].update(spos, 0, 0);
                    self.ctx.after_literal = false;
                    sbuf.write_forward(&mut spos, last_word_expected.to_be_bytes());
                }
                symbol @ 0 ..= 255 => {
                    self_ctx_buckets[shc(spos - 1)].update(spos, 0, 0);
                    self.ctx.after_literal = true;
                    sbuf.write_forward(&mut spos, symbol as u8);
                    self_ctx_words[shw(spos - 3)] = get_word(spos - 2);
                }
                encoded_roid_lenid @ _ => {
                    let (roid, lenid) = (
                        ((encoded_roid_lenid - 256) / super::LZ_LENID_SIZE as u16) as u8,
                        ((encoded_roid_lenid - 256) % super::LZ_LENID_SIZE as u16) as u8,
                    );

                    // get reduced offset
                    let (robase, robitlen) = roid_decoding_array[roid as usize];
                    let reduced_offset = robase + bits.get(robitlen) as u16;

                    // get match_pos/match_len
                    let match_info = self_ctx_buckets[shc(spos - 1)].get_match_pos_and_match_len(reduced_offset);
                    let encoded_match_len = if lenid == super::LZ_LENID_SIZE as u8 - 1 {
                        bits.load_u32(tbuf, &mut tpos);
                        huff_decoder2.decode_from_bits(&mut bits) as usize
                    } else {
                        lenid as usize
                    };
                    let (match_pos, match_len_expected, match_len_min) = match_info;
                    let match_len = match encoded_match_len {
                        l if l + match_len_min > match_len_expected => l + match_len_min,
                        l if l > 0 => encoded_match_len + match_len_min - 1,
                        _ => match_len_expected,
                    };
                    self_ctx_buckets[shc(spos - 1)].update(spos, reduced_offset, match_len);
                    self.ctx.after_literal = false;

                    super::mem::copy_fast(sbuf, match_pos, spos, match_len);
                    spos += match_len;
                    self_ctx_words[shw(spos - 3)] = get_word(spos - 2);
                }
            }
        }
        return Ok((std::cmp::min(spos, sbuf_len), std::cmp::min(tpos, tbuf.len())));
    }
}
