pub trait UncheckedSliceExt<T> {
    unsafe fn nocheck<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self>;
    unsafe fn nocheck_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self>;
}

impl<T> UncheckedSliceExt<T> for [T] {
    unsafe fn nocheck<'a>(&'a self) -> unchecked_index::UncheckedIndex<&'a Self> {
        unchecked_index::unchecked_index(self)
    }

    unsafe fn nocheck_mut<'a>(&'a mut self) -> unchecked_index::UncheckedIndex<&'a mut Self> {
        unchecked_index::unchecked_index(self)
    }
}
