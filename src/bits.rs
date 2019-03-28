pub struct Bits {
    value: u64,
    len: u8,
}

impl Bits {
    pub fn new() -> Bits {
        Bits { value: 0, len: 0 }
    }

    pub fn len(&self) -> u8 {
        self.len
    }

    pub fn peek(&self, len: u8) -> u64 {
        self.value >> (self.len - len)
    }

    pub fn skip(&mut self, len: u8) {
        self.len -= len;
        self.value &= (1 << self.len) - 1;
    }

    pub fn get(&mut self, len: u8) -> u64 {
        let value = self.peek(len);
        self.skip(len);
        value
    }

    pub fn put(&mut self, len: u8, value: u64) {
        self.value = self.value << len | value;
        self.len += len;
    }
}
