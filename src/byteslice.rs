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
        let offset = {let old_offset = *offset; *offset += std::mem::size_of::<T>(); old_offset};
        return std::ptr::read_unaligned(self.as_ptr().add(offset) as *const T);
    }

    unsafe fn write_forward<T>(&mut self, offset: &mut usize, value: T) {
        let offset = {let old_offset = *offset; *offset += std::mem::size_of::<T>(); old_offset};
        std::ptr::write_unaligned(self.as_mut_ptr().add(offset) as *mut T, value);
    }
}
