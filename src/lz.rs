use std::{cmp::Ordering, error::Error};

use unchecked_index::UncheckedIndex;

use crate::{
    LZ_CHUNK_SIZE, LZ_LENID_SIZE, LZ_MATCH_MAX_LEN, LZ_MF_BUCKET_ITEM_SIZE, LZ_ROID_SIZE,
    SYMRANK_NUM_SYMBOLS,
    coder::{Decoder, Encoder},
    huffman::{HuffmanDecoding, HuffmanEncoding, HuffmanTable},
    matchfinder::{Bucket, BucketMatcher, MatchInfo},
    mem::{BytesConstPtrExt, BytesMutPtrExt, mem_fast_copy},
    symrank::SymRankCoder,
    unchecked,
};

const LZ_ROID_ENCODING_ARRAY: [(u8, u8, u16); LZ_MF_BUCKET_ITEM_SIZE] =
    include!(concat!(env!("OUT_DIR"), "/", "LZ_ROID_ENCODING_ARRAY.txt"));
const LZ_ROID_DECODING_ARRAY: [(u16, u8); LZ_ROID_SIZE] =
    include!(concat!(env!("OUT_DIR"), "/", "LZ_ROID_DECODING_ARRAY.txt"));

const WORD_SYMBOL: u16 = SYMRANK_NUM_SYMBOLS as u16 - 1;

/// Limpel-Ziv matching options.
#[repr(C)]
pub struct LZCfg {
    pub match_depth: usize,
    pub lazy_match_depth1: usize,
    pub lazy_match_depth2: usize,
}

impl LZCfg {
    pub fn new(match_depth: usize, lazy_match_depth1: usize, lazy_match_depth2: usize) -> Self {
        Self {
            match_depth,
            lazy_match_depth1,
            lazy_match_depth2,
        }
    }
}

struct LZContext {
    buckets: UncheckedIndex<Vec<Bucket>>,
    symranks: UncheckedIndex<Vec<SymRankCoder>>,
    words: UncheckedIndex<Vec<[u8; 2]>>,
    first_block: bool,
    after_literal: bool,
}

impl LZContext {
    pub fn new() -> Self {
        Self {
            buckets: unchecked!(vec![Bucket::new(); 256]),
            symranks: unchecked!(vec![SymRankCoder::new(); 512]),
            words: unchecked!(vec![[0, 0]; 32768]),
            first_block: true,
            after_literal: true,
        }
    }
}

pub struct LZEncoder {
    ctx: LZContext,
    bucket_matchers: UncheckedIndex<Vec<BucketMatcher>>,
}

impl LZEncoder {
    pub fn new() -> Self {
        Self {
            ctx: LZContext::new(),
            bucket_matchers: unchecked!(vec![BucketMatcher::new(); 256]),
        }
    }

    pub fn forward(&mut self, forward_len: usize) {
        for i in 0..self.bucket_matchers.len() {
            self.ctx.buckets[i].forward(forward_len);
            self.bucket_matchers[i].forward(&self.ctx.buckets[i]);
        }
    }

    pub unsafe fn encode(
        &mut self,
        cfg: &LZCfg,
        sbuf: &[u8],
        tbuf: &mut [u8],
        spos: usize,
    ) -> (usize, usize) {
        unsafe {
            let roid_encoding_array = &unchecked!(&LZ_ROID_ENCODING_ARRAY);
            let sbuf = &unchecked!(sbuf);
            let tbuf = &mut unchecked!(tbuf);

            enum MatchItem {
                Match {
                    symbol: u16,
                    symrank_context: u16,
                    symrank_unlikely: u8,
                    robitlen: u8,
                    robits: u16,
                    encoded_match_len: u8,
                    after_literal: bool,
                },
                Symbol {
                    symbol: u16,
                    symrank_context: u16,
                    symrank_unlikely: u8,
                    after_literal: bool,
                },
            }
            let mut encoder: Encoder = Encoder::new(tbuf, 0);
            let mut spos = spos;
            let mut match_items = Vec::with_capacity(LZ_CHUNK_SIZE);

            // start Lempel-Ziv encoding
            while spos < sbuf.len() && match_items.len() < match_items.capacity() {
                let last_word_expected = self.ctx.words[hash2(sbuf, spos - 1)];
                let last_word_matched = sbuf.as_ptr().get::<[u8; 2]>(spos) == last_word_expected;
                let symrank_context =
                    hash1(sbuf, spos - 1) as u16 | (self.ctx.after_literal as u16) << 8;
                let symrank_unlikely = last_word_expected[0];

                // encode as match
                let mut lazy_match_id = 0;
                let m = self.bucket_matchers[hash1(sbuf, spos - 1)].find_match(
                    &self.ctx.buckets[hash1(sbuf, spos - 1)],
                    sbuf,
                    spos,
                    cfg.match_depth,
                );

                if m.match_len > 0 {
                    let (roid, robitlen, robits) = roid_encoding_array[m.reduced_offset as usize];

                    // find lazy match
                    if m.match_len < LZ_MATCH_MAX_LEN / 2 {
                        let lazy_len1 = m.match_len + 1 + (robitlen < 8) as usize;
                        let lazy_len2 = lazy_len1 - last_word_matched as usize;
                        let has_lazy_match = |pos, lazy_len, match_depth| {
                            let lazy_bucket_matcher = &self.bucket_matchers[hash1(sbuf, pos)];
                            let lazy_bucket = &self.ctx.buckets[hash1(sbuf, pos)];
                            lazy_bucket_matcher.has_lazy_match(
                                lazy_bucket,
                                sbuf,
                                pos + 1,
                                lazy_len,
                                match_depth,
                            )
                        };
                        lazy_match_id = match () {
                            _ if has_lazy_match(spos, lazy_len1, cfg.lazy_match_depth1) => 1,
                            _ if has_lazy_match(spos + 1, lazy_len2, cfg.lazy_match_depth2) => 2,
                            _ => 0,
                        };
                    }

                    if lazy_match_id == 0 {
                        let encoded_match_len = match m.match_len.cmp(&m.match_len_expected) {
                            Ordering::Greater => m.match_len - m.match_len_min,
                            Ordering::Less => m.match_len - m.match_len_min + 1,
                            Ordering::Equal => 0,
                        } as u8;
                        let lenid = std::cmp::min(LZ_LENID_SIZE as u8 - 1, encoded_match_len);
                        let encoded_roid_lenid =
                            256 + roid as u16 * LZ_LENID_SIZE as u16 + lenid as u16;
                        match_items.push(MatchItem::Match {
                            symbol: encoded_roid_lenid,
                            symrank_context,
                            symrank_unlikely,
                            robitlen,
                            robits,
                            encoded_match_len,
                            after_literal: self.ctx.after_literal,
                        });

                        self.ctx.buckets[hash1(sbuf, spos - 1)].update(
                            spos,
                            m.reduced_offset,
                            m.match_len,
                        );
                        self.bucket_matchers[hash1(sbuf, spos - 1)].update(
                            &self.ctx.buckets[hash1(sbuf, spos - 1)],
                            sbuf,
                            spos,
                        );
                        spos += m.match_len;
                        self.ctx.after_literal = false;
                        self.ctx.words[hash2(sbuf, spos - 3)] = sbuf.as_ptr().get(spos - 2);
                        continue;
                    }
                }
                self.ctx.buckets[hash1(sbuf, spos - 1)].update(spos, 0, 0);
                self.bucket_matchers[hash1(sbuf, spos - 1)].update(
                    &self.ctx.buckets[hash1(sbuf, spos - 1)],
                    sbuf,
                    spos,
                );

                // encode as symbol
                if spos + 1 < sbuf.len() && lazy_match_id != 1 && last_word_matched {
                    match_items.push(MatchItem::Symbol {
                        symbol: WORD_SYMBOL,
                        symrank_context,
                        symrank_unlikely,
                        after_literal: self.ctx.after_literal,
                    });
                    spos += 2;
                    self.ctx.after_literal = false;
                } else {
                    match_items.push(MatchItem::Symbol {
                        symbol: sbuf[spos] as u16,
                        symrank_context,
                        symrank_unlikely,
                        after_literal: self.ctx.after_literal,
                    });
                    spos += 1;
                    self.ctx.after_literal = true;
                    self.ctx.words[hash2(sbuf, spos - 3)] = sbuf.as_ptr().get(spos - 2);
                }
            }

            // init symrank array
            if self.ctx.first_block {
                let symbol_counts = &mut [0; SYMRANK_NUM_SYMBOLS];
                match_items.iter().for_each(|match_item| match match_item {
                    &MatchItem::Match { symbol, .. } | &MatchItem::Symbol { symbol, .. } => {
                        symbol_counts[symbol as usize] += 1;
                    }
                });

                let mut last_symbol = u16::MAX;
                let vs = (0..SYMRANK_NUM_SYMBOLS)
                    .map(|i| (-symbol_counts[i], i))
                    .collect::<std::collections::BTreeSet<_>>()
                    .iter()
                    .map(|(_count, i)| {
                        let symbol = *i as u16;
                        if symbol == last_symbol + 1 {
                            encoder.encode_raw_bits(0, 1);
                        } else {
                            encoder.encode_raw_bits(1, 1);
                            encoder.encode_raw_bits(symbol as u32, 9);
                        }
                        last_symbol = symbol;
                        symbol
                    })
                    .collect::<Vec<_>>();

                self.ctx
                    .symranks
                    .iter_mut()
                    .for_each(|symrank| symrank.init(&vs));
                self.ctx.first_block = false;
            }

            // encode match_items_len
            encoder.encode_varint(std::cmp::min(spos, sbuf.len()) as u32);
            encoder.encode_varint(match_items.len() as u32);

            // start Huffman encoding
            let mut huff_weights1 = unchecked!([[0u32; SYMRANK_NUM_SYMBOLS]; 2]);
            let mut huff_weights2 = unchecked!([0u32; LZ_MATCH_MAX_LEN]);
            for match_item in &mut match_items {
                match match_item {
                    &mut MatchItem::Match {
                        ref mut symbol,
                        symrank_context,
                        symrank_unlikely,
                        encoded_match_len,
                        after_literal,
                        ..
                    } => {
                        let symrank = &mut self.ctx.symranks[symrank_context as usize];
                        let encoded_symbol = symrank.encode(*symbol, symrank_unlikely as u16);
                        huff_weights1[after_literal as usize][encoded_symbol as usize] += 1;
                        if encoded_match_len as usize >= LZ_LENID_SIZE - 1 {
                            huff_weights2[encoded_match_len as usize] += 1;
                        }
                        *symbol = encoded_symbol;
                    }
                    &mut MatchItem::Symbol {
                        ref mut symbol,
                        symrank_context,
                        symrank_unlikely,
                        after_literal,
                        ..
                    } => {
                        let symrank = &mut self.ctx.symranks[symrank_context as usize];
                        let encoded_symbol = symrank.encode(*symbol, symrank_unlikely as u16);
                        huff_weights1[after_literal as usize][encoded_symbol as usize] += 1;
                        *symbol = encoded_symbol;
                    }
                }
            }
            let huff_table1 = [
                HuffmanTable::new_from_sym_weights(&huff_weights1[0][..], 15),
                HuffmanTable::new_from_sym_weights(&huff_weights1[1][..], 15),
            ];
            let huff_table2 = HuffmanTable::new_from_sym_weights(&huff_weights2[..], 15);
            encoder.encode_huffman_table(&huff_table1[0]);
            encoder.encode_huffman_table(&huff_table1[1]);
            encoder.encode_huffman_table(&huff_table2);
            let huff1 = [
                HuffmanEncoding::from_huffman_table(&huff_table1[0]),
                HuffmanEncoding::from_huffman_table(&huff_table1[1]),
            ];
            let huff2 = HuffmanEncoding::from_huffman_table(&huff_table2);

            match_items.iter().for_each(|match_item| match *match_item {
                MatchItem::Symbol {
                    symbol,
                    after_literal,
                    ..
                } => {
                    encoder.encode_huffman_sym(&huff1[after_literal as usize], symbol);
                }
                MatchItem::Match {
                    symbol,
                    robitlen,
                    robits,
                    encoded_match_len,
                    after_literal,
                    ..
                } => {
                    encoder.encode_huffman_sym(&huff1[after_literal as usize], symbol);
                    encoder.encode_raw_bits(robits as u32, robitlen);
                    if encoded_match_len as usize >= LZ_LENID_SIZE - 1 {
                        encoder.encode_huffman_sym(&huff2, encoded_match_len as u16);
                    }
                }
            });

            (spos, encoder.finish_into_output_pos())
        }
    }
}

pub struct LZDecoder {
    ctx: LZContext,
}

impl LZDecoder {
    pub fn new() -> Self {
        Self {
            ctx: LZContext::new(),
        }
    }

    pub fn forward(&mut self, forward_len: usize) {
        self.ctx
            .buckets
            .iter_mut()
            .for_each(|bucket| bucket.forward(forward_len));
    }

    pub unsafe fn decode(
        &mut self,
        tbuf: &[u8],
        sbuf: &mut [u8],
        spos: usize,
    ) -> Result<usize, Box<dyn Error>> {
        unsafe {
            let roid_decoding_array = unchecked!(&LZ_ROID_DECODING_ARRAY);

            let mut decoder: Decoder = Decoder::new(tbuf, 0);
            let mut spos = spos;

            // init symrank array
            if self.ctx.first_block {
                let mut last_symbol = u16::MAX;
                let vs = (0..SYMRANK_NUM_SYMBOLS)
                    .map(|_| {
                        let symbol = if decoder.decode_raw_bits(1) == 0 {
                            last_symbol + 1
                        } else {
                            decoder.decode_raw_bits(9) as u16
                        };
                        last_symbol = symbol;
                        symbol
                    })
                    .collect::<Vec<_>>();
                self.ctx
                    .symranks
                    .iter_mut()
                    .for_each(|symrank| symrank.init(&vs));
                self.ctx.first_block = false;
            }

            // decode sbuf_len/match_items_len
            let sbuf = std::slice::from_raw_parts_mut(sbuf.as_ptr() as *mut u8, 0);
            let sbuf_len = decoder.decode_varint() as usize;
            let match_items_len = decoder.decode_varint() as usize;

            // start decoding
            let huff_table1 = [
                decoder.decode_huffman_table(),
                decoder.decode_huffman_table(),
            ];
            let huff_table2 = decoder.decode_huffman_table();
            let huff1 = [
                HuffmanDecoding::from_huffman_table(&huff_table1[0]),
                HuffmanDecoding::from_huffman_table(&huff_table1[1]),
            ];
            let huff2 = HuffmanDecoding::from_huffman_table(&huff_table2);

            for _ in 0..match_items_len {
                let last_word_expected = self.ctx.words[hash2(sbuf, spos - 1)];
                let symrank_context =
                    hash1(sbuf, spos - 1) as u16 | (self.ctx.after_literal as u16) << 8;
                let symrank = &mut self.ctx.symranks[symrank_context as usize];
                let symrank_unlikely = last_word_expected[0];

                let symbol = decoder.decode_huffman_sym(&huff1[self.ctx.after_literal as usize]);
                if !(0..=SYMRANK_NUM_SYMBOLS as u16).contains(&symbol) {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData).into());
                }

                match symrank.decode(symbol, symrank_unlikely as u16) {
                    WORD_SYMBOL => {
                        self.ctx.buckets[hash1(sbuf, spos - 1)].update(spos, 0, 0);
                        self.ctx.after_literal = false;
                        sbuf.as_mut_ptr().put(spos, last_word_expected);
                        spos += 2;
                    }
                    symbol @ 0..=255 => {
                        self.ctx.buckets[hash1(sbuf, spos - 1)].update(spos, 0, 0);
                        self.ctx.after_literal = true;
                        sbuf.as_mut_ptr().put(spos, symbol as u8);
                        spos += 1;
                        self.ctx.words[hash2(sbuf, spos - 3)] = sbuf.as_ptr().get(spos - 2);
                    }
                    encoded_roid_lenid => {
                        let (roid, lenid) = (
                            ((encoded_roid_lenid - 256) / LZ_LENID_SIZE as u16) as u8,
                            ((encoded_roid_lenid - 256) % LZ_LENID_SIZE as u16) as u8,
                        );

                        // get reduced offset
                        let (robase, robitlen) = roid_decoding_array[roid as usize];
                        let reduced_offset = robase + decoder.decode_raw_bits(robitlen) as u16;

                        // get match_pos/match_len
                        let MatchInfo {
                            match_pos,
                            match_len_expected,
                            match_len_min,
                        } = self.ctx.buckets[hash1(sbuf, spos - 1)].get_match_info(reduced_offset);

                        let encoded_match_len = if lenid == LZ_LENID_SIZE as u8 - 1 {
                            decoder.decode_huffman_sym(&huff2) as usize
                        } else {
                            lenid as usize
                        };
                        let match_len = match encoded_match_len {
                            l if l + match_len_min > match_len_expected => l + match_len_min,
                            l if l > 0 => encoded_match_len + match_len_min - 1,
                            _ => match_len_expected,
                        };
                        self.ctx.buckets[hash1(sbuf, spos - 1)].update(
                            spos,
                            reduced_offset,
                            match_len,
                        );
                        self.ctx.after_literal = false;

                        mem_fast_copy(sbuf.as_mut_ptr(), match_pos, spos, match_len);
                        spos += match_len;
                        self.ctx.words[hash2(sbuf, spos - 3)] = sbuf.as_ptr().get(spos - 2);
                    }
                }
            }
            Ok(std::cmp::min(spos, sbuf_len))
        }
    }
}

#[inline]
unsafe fn hash1(buf: &[u8], pos: usize) -> usize {
    unsafe {
        let ptr = buf.as_ptr();
        ptr.add(pos).read_unaligned() as usize & 0x7f
            | (ptr.add(pos - 1).read_unaligned().is_ascii_alphanumeric() as usize) << 7
    }
}

#[inline]
unsafe fn hash2(buf: &[u8], pos: usize) -> usize {
    unsafe {
        let ptr = buf.as_ptr();
        ptr.add(pos).read_unaligned() as usize & 0x7f | hash1(buf, pos - 1) << 7
    }
}
