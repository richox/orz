pub trait ToUSize {
    fn to_usize(&self) -> usize;
}

pub trait UncheckedSliceExt<T> {
    unsafe fn xset<I: ToUSize>(&mut self, i: I, value: T);
    unsafe fn xget<I: ToUSize>(&self, i: I) -> &T;
    unsafe fn xget_mut<I: ToUSize>(&mut self, i: I) -> &mut T;
}

pub trait ResultExt<E> {
    fn from_bool(condition: bool, err: Result<(), E>) -> Result<(), E>;
}

impl ToUSize for i8    {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for u8    {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for i16   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for u16   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for i32   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for u32   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for i64   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for u64   {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for isize {fn to_usize(&self) -> usize {*self as usize}}
impl ToUSize for usize {fn to_usize(&self) -> usize {*self as usize}}

impl<T> UncheckedSliceExt<T> for [T] {
    unsafe fn xset<I: ToUSize>(&mut self, i: I, value: T) {
        *self.get_unchecked_mut(i.to_usize()) = value;
    }

    unsafe fn xget<I: ToUSize>(&self, i: I) -> &T {
        self.get_unchecked(i.to_usize())
    }

    unsafe fn xget_mut<I: ToUSize>(&mut self, i: I) -> &mut T {
        self.get_unchecked_mut(i.to_usize())
    }
}

impl<E> ResultExt<E> for Result<(), E> {
    fn from_bool(condition: bool, err: Result<(), E>) -> Result<(), E> {
        if !condition {
            return err;
        }
        Ok(())
    }

}
