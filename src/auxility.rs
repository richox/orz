pub trait UncheckedSliceExt<T> {
    unsafe fn nc<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self>;
    unsafe fn nc_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self>;
}

impl<T> UncheckedSliceExt<T> for [T] {
    #[inline(always)]
    unsafe fn nc<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self> {
        unchecked_index::unchecked_index(self)
    }

    #[inline(always)]
    unsafe fn nc_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self> {
        unchecked_index::unchecked_index(self)
    }
}

pub trait ByteSliceExt {
    unsafe fn read<T>(&self, offset: usize) -> T;
    unsafe fn write<T>(&self, offset: usize, value: T);
}

impl ByteSliceExt for [u8] {
    #[inline(always)]
    unsafe fn read<T>(&self, offset: usize) -> T {
        return std::ptr::read((self.as_ptr() as usize + offset) as *const T);
    }

    #[inline(always)]
    unsafe fn write<T>(&self, offset: usize, value: T) {
        std::ptr::write((self.as_ptr() as usize + offset) as *mut T, value);
    }
}
