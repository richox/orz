use crate::mem::{mem_get, mem_put};

#[derive(Clone, Copy, Default)]
pub struct BitQueue {
    value: u64,
    len: u8,
}

impl BitQueue {
    pub fn peek(&self, len: u8) -> u64 {
        self.value >> (self.len - len)
    }

    pub fn get(&mut self, len: u8) -> u64 {
        let value = self.peek(len);
        self.len -= len;
        self.value &= !(!0 << self.len);
        value
    }

    pub fn put(&mut self, len: u8, value: u64) {
        self.value = self.value << len ^ value;
        self.len += len;
    }

    pub unsafe fn load_u32(&mut self, buf: &[u8], pos: &mut usize) {
        if self.len <= 32 {
            self.put(32, mem_get::<u32>(buf.as_ptr(), *pos).swap_bytes() as u64);
            *pos += 4;
        }
    }
    pub unsafe fn save_u32(&mut self, buf: &mut [u8], pos: &mut usize) {
        if self.len >= 32 {
            mem_put(buf.as_mut_ptr(), *pos, (self.get(32) as u32).swap_bytes());
            *pos += 4;
        }
    }

    pub unsafe fn save_all(&mut self, buf: &mut [u8], pos: &mut usize) {
        self.put(8 - self.len % 8, 0);
        while self.len > 0 {
            mem_put(buf.as_mut_ptr(), *pos, self.get(8) as u8);
            *pos += 1;
        }
    }
}
