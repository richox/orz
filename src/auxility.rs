pub trait UncheckedSliceExt<T> {
    unsafe fn nc<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self>;
    unsafe fn nc_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self>;
}

impl<T> UncheckedSliceExt<T> for [T] {
    unsafe fn nc<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self> {
        unchecked_index::unchecked_index(self)
    }

    unsafe fn nc_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self> {
        unchecked_index::unchecked_index(self)
    }
}
