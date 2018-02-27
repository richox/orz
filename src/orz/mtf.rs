const MTF_INIT_ARRAY: [u8; 256] = include!("constants/MTF_INIT_ARRAY.txt");
const MTF_NEXT_ARRAY: [u8; 256] = include!("constants/MTF_NEXT_ARRAY.txt");

pub struct MTFEncoder {
    mtf_array: [u8; 256],
    rev_array: [u8; 256],
}

pub struct MTFDecoder {
    mtf_array: [u8; 256],
}

impl MTFEncoder {
    pub fn new() -> MTFEncoder {
        let mut rev_array = [0u8; 256];
        for i in 0..256 {
            rev_array[MTF_INIT_ARRAY[i] as usize] = i as u8;
        }
        return MTFEncoder {
            mtf_array: MTF_INIT_ARRAY,
            rev_array: rev_array,
        };
    }

    pub fn encode(&mut self, s: u8) -> u8 {
        unsafe {
            let t = *self.rev_array.get_unchecked(s as usize);
            let next_t = *MTF_NEXT_ARRAY.get_unchecked(t as usize);
            let next_s = *self.mtf_array.get_unchecked(next_t as usize);
            self.rev_array.swap(s as usize, next_s as usize);
            self.mtf_array.swap(t as usize, next_t as usize);
            return t;
        }
    }
}

impl MTFDecoder {
    pub fn new() -> MTFDecoder {
        return MTFDecoder {
            mtf_array: MTF_INIT_ARRAY,
        };
    }

    pub fn decode(&mut self, t: u8) -> u8 {
        unsafe {
            let s = *self.mtf_array.get_unchecked(t as usize) as u8;
            let next_t = *MTF_NEXT_ARRAY.get_unchecked(t as usize);
            self.mtf_array.swap(t as usize, next_t as usize);
            return s;
        }
    }
}
