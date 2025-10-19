use crate::{
    huffman::{HuffmanDecoding, HuffmanEncoding, HuffmanTable},
    mem::{BytesConstPtrExt, BytesMutPtrExt},
};

pub struct Encoder<'a> {
    output: &'a mut [u8],
    output_pos: usize,
    buffer: BitBuffer,
}

impl<'a> Encoder<'a> {
    pub fn new(output: &'a mut [u8], output_pos: usize) -> Self {
        Encoder {
            output,
            output_pos,
            buffer: BitBuffer::default(),
        }
    }

    pub fn encode_varint(&mut self, mut v: u32) {
        loop {
            self.reserve_32bits();
            let has_next = v > 0b01;
            let bits = v & 0b01 | (has_next as u32) << 1;
            self.buffer.put(2, bits as u64);
            v >>= 1;
            if !has_next {
                break;
            }
        }
    }

    pub fn encode_raw_bits(&mut self, bits: u32, bits_len: u8) {
        self.reserve_32bits();
        self.buffer.put(bits_len, bits as u64);
    }

    pub fn encode_huffman_table(&mut self, huffman_table: &HuffmanTable) {
        assert!(!huffman_table.code_lens.is_empty());

        // write max code len
        let max_code_len = *huffman_table.code_lens.iter().max().unwrap();
        self.encode_varint(max_code_len as u32);

        // write huffman table
        let mut last_sym = usize::MAX;
        for (sym, &code_len) in huffman_table.code_lens.iter().enumerate() {
            if code_len > 0 {
                self.encode_varint((sym - last_sym) as u32);
                self.encode_varint((max_code_len - code_len) as u32);
                last_sym = sym;
            }
        }
        self.encode_varint(0); // to identify the end of the huffman table
    }

    pub fn encode_huffman_sym(&mut self, encoding: &HuffmanEncoding, sym: u16) {
        self.reserve_32bits();
        let (bits, code_len) = encoding.encodings[sym as usize];
        self.buffer.put(code_len as u8, bits as u64);
    }

    pub fn finish_into_output_pos(mut self) -> usize {
        self.reserve_32bits();
        if self.buffer.len > 0 {
            self.buffer.put(32 - self.buffer.len, 0);
            self.output_pos = self.buffer.save_all(self.output, self.output_pos);
        }
        self.output_pos
    }

    fn reserve_32bits(&mut self) {
        if self.buffer.len >= 32 {
            self.output_pos = self.buffer.save_u32(self.output, self.output_pos);
        }
    }
}

pub struct Decoder<'a> {
    input: &'a [u8],
    input_pos: usize,
    buffer: BitBuffer,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &[u8], input_pos: usize) -> Decoder<'_> {
        Decoder {
            input,
            input_pos,
            buffer: BitBuffer::default(),
        }
    }

    pub fn decode_varint(&mut self) -> u32 {
        let mut v = 0u32;
        for bits_len in (0..).step_by(1) {
            self.reserve_32bits();
            let bits = self.buffer.get(2) as u32;
            let has_next = bits > 0b01;
            v |= (bits & 0b01) << bits_len;
            if !has_next {
                break;
            }
        }
        v
    }

    pub fn decode_raw_bits(&mut self, bits_len: u8) -> u32 {
        self.reserve_32bits();
        self.buffer.get(bits_len) as u32
    }

    pub fn decode_huffman_table(&mut self) -> HuffmanTable {
        // read max code len
        let max_code_len = self.decode_varint() as u8;

        // read huffman table
        let mut huffman_table = vec![];
        loop {
            let sym_delta = self.decode_varint();
            if sym_delta == 0 {
                break;
            }
            for _ in 1..sym_delta {
                huffman_table.push(0);
            }
            huffman_table.push(max_code_len - self.decode_varint() as u8);
        }
        HuffmanTable::new(huffman_table, max_code_len)
    }

    pub fn decode_huffman_sym(&mut self, decoding: &HuffmanDecoding) -> u16 {
        self.reserve_32bits();
        let peeked = self.buffer.peek(decoding.max_code_len);
        let (sym, code_len) = decoding.decodings[peeked as usize];
        self.buffer.skip(code_len as u8);
        sym
    }

    fn reserve_32bits(&mut self) {
        if self.buffer.len < 32 {
            self.input_pos = self.buffer.load_u32(self.input, self.input_pos);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BitBuffer {
    value: u64,
    len: u8,
}

impl BitBuffer {
    #[inline]
    fn peek(&self, len: u8) -> u64 {
        (self.value >> (self.len - len)) & ((1 << len) - 1)
    }

    #[inline]
    fn skip(&mut self, len: u8) {
        self.len -= len;
    }

    #[inline]
    fn get(&mut self, len: u8) -> u64 {
        let value = self.peek(len);
        self.skip(len);
        value
    }

    #[inline]
    fn put(&mut self, len: u8, value: u64) {
        self.value = self.value << len ^ value;
        self.len += len;
    }

    #[inline]
    fn load_u32(&mut self, buf: &[u8], mut pos: usize) -> usize {
        if self.len <= 32 {
            self.put(32, buf.as_ptr().get::<u32>(pos).swap_bytes() as u64);
            pos += 4;
        }
        pos
    }

    #[inline]
    fn save_u32(&mut self, buf: &mut [u8], mut pos: usize) -> usize {
        if self.len >= 32 {
            buf.as_mut_ptr()
                .put(pos, (self.get(32) as u32).swap_bytes());
            pos += 4;
        }
        pos
    }

    #[inline]
    fn save_all(&mut self, buf: &mut [u8], mut pos: usize) -> usize {
        while self.len > 0 {
            buf.as_mut_ptr().put(pos, self.peek(8) as u8);
            pos += 1;
            self.len -= self.len.min(8);
        }
        pos
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::huffman::HuffmanTable;

    #[test]
    fn test_coder_with_huffman() {
        let input = b"i can can a can into a can, can you can a can into a can?";

        // test compute huffman table
        let mut weights = [0; 256];
        for &b in input {
            weights[b as usize] += 1;
        }
        let huffman_table = HuffmanTable::new_from_sym_weights(&weights, 15);
        let huff = HuffmanEncoding::from_huffman_table(&huffman_table);

        // test encoding
        let mut encoded = vec![0; 1024];
        let mut encoder = Encoder::new(&mut encoded, 0);
        encoder.encode_varint(input.len() as u32);
        encoder.encode_huffman_table(&huffman_table);
        for &b in input {
            encoder.encode_huffman_sym(&huff, b as u16);
        }
        let output_pos = encoder.finish_into_output_pos();
        encoded.truncate(output_pos);

        println!("input.len() = {}", input.len());
        println!("encoded.len() = {}", encoded.len());

        // test decoding
        let mut decoded = vec![];
        let mut decoder = Decoder::new(&encoded, 0);
        let num_syms = decoder.decode_varint();

        let huffman_table = decoder.decode_huffman_table();
        let huff = HuffmanDecoding::from_huffman_table(&huffman_table);

        for _ in 0..num_syms {
            decoded.push(decoder.decode_huffman_sym(&huff) as u8);
        }
        assert_eq!(
            String::from_utf8(decoded),
            String::from_utf8(input.to_vec())
        );
    }
}
