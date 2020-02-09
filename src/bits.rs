use super::auxility::ByteSliceExt;

/// 64-bit bit-queue (FIFO). Adds from the least-significant end.
/// Consumes from the most-significant, bounded by len.
#[derive(Clone, Copy, Default)]
pub struct Bits {
    value: u64,
    len: u8,
}

impl Bits {
    /// Get most significant len bits.
    pub fn peek(&self, len: u8) -> u64 {
        debug_assert!(len <= 64);
        return self.value >> (self.len - len);
    }

    /// Consume len bits.
    pub fn get(&mut self, len: u8) -> u64 {
        debug_assert!(len <= 64);
        debug_assert!(self.len - len <= 64);
        let value = self.peek(len);
        self.value ^= value << (self.len - len);
        self.len -= len;
        return value;
    }

    /// Add len bits.
    pub fn put(&mut self, len: u8, value: u64) {
        debug_assert!(len <= 64 - self.len);
        self.value = self.value << len ^ value;
        self.len += len;
    }

    /// Add 4 bytes from a buffer. Big-endian. Updates pos.
    pub unsafe fn load_u32(&mut self, buf: &[u8], pos: &mut usize) {
        if self.len <= 32 {
            self.put(32, buf.read_forward::<u32>(pos).to_be() as u64);
        }
    }
    
    /// Save 4 bytes to a buffer. Big-endian. Updates pos.
    pub unsafe fn save_u32(&mut self, buf: &mut [u8], pos: &mut usize) {
        if self.len >= 32 {
            buf.write_forward(pos, (self.get(32) as u32).to_be());
        }
    }

    /// Save all as bytes to a buffer. Pads self to byte-align. Big-endian.
    pub unsafe fn save_all(&mut self, buf: &mut [u8], pos: &mut usize) {
        self.put(8 - self.len % 8, 0);
        while self.len > 0 {
            buf.write_forward(pos, self.get(8) as u8);
        }
    }
}
