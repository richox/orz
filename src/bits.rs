use super::auxility::ByteSliceExt;

#[derive(Clone, Copy, Default)]
pub struct Bits {
    value: u64,
    len: u8,
}

impl Bits {
    pub fn peek(&self, len: u8) -> u64 {
        return self.value >> (self.len - len);
    }

    pub fn get(&mut self, len: u8) -> u64 {
        let value = self.peek(len);
        self.value ^= value << (self.len - len);
        self.len -= len;
        return value;
    }

    pub fn put(&mut self, len: u8, value: u64) {
        self.value = self.value << len ^ value;
        self.len += len;
    }

    pub unsafe fn load_u32(&mut self, buf: &[u8], pos: &mut usize) {
        if self.len <= 32 {
            self.put(32, buf.read::<u32>(*pos).to_be() as u64);
            *pos += 4;
        }
    }
    pub unsafe fn save_u32(&mut self, buf: &mut [u8], pos: &mut usize) {
        if self.len >= 32 {
            buf.write::<u32>(*pos, (self.get(32) as u32).to_be());
            *pos += 4;
        }
    }

    pub unsafe fn save_all(&mut self, buf: &mut [u8], pos: &mut usize) {
        self.put(8 - self.len % 8, 0);
        while self.len > 0 {
            *buf.get_unchecked_mut(*pos) = self.get(8) as u8;
            *pos += 1;
        }
    }
}
