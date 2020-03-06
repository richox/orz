pub trait ByteSliceExt {
    unsafe fn read<T>(&self, offset: usize) -> T;
    unsafe fn write<T>(&mut self, offset: usize, value: T);
    unsafe fn read_forward<T>(&self, offset: &mut usize) -> T;
    unsafe fn write_forward<T>(&mut self, offset: &mut usize, value: T);
}

impl ByteSliceExt for [u8] {
    unsafe fn read<T>(&self, offset: usize) -> T {
        return std::ptr::read_unaligned(self.as_ptr().add(offset) as *const T);
    }

    unsafe fn write<T>(&mut self, offset: usize, value: T) {
        std::ptr::write_unaligned(self.as_mut_ptr().add(offset) as *mut T, value);
    }

    unsafe fn read_forward<T>(&self, offset: &mut usize) -> T {
        let t = std::ptr::read_unaligned(self.as_ptr().add(*offset) as *const T);
        std::ptr::write(offset, *offset + std::mem::size_of::<T>());
        return t;
    }

    unsafe fn write_forward<T>(&mut self, offset: &mut usize, value: T) {
        std::ptr::write_unaligned(self.as_mut_ptr().add(*offset) as *mut T, value);
        std::ptr::write(offset, *offset + std::mem::size_of::<T>());
    }
}
