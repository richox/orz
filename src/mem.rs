use std::hint::likely;
use std::mem::MaybeUninit;
use std::simd::cmp::SimdPartialEq;

pub trait BytesConstPtrExt {
    fn get<T: Copy>(&self, offset: usize) -> T;
}

pub trait BytesMutPtrExt {
    fn put<T: Copy>(&self, offset: usize, value: T);
}

impl BytesConstPtrExt for *const u8 {
    fn get<T: Copy>(&self, offset: usize) -> T {
        let mut t = unsafe { MaybeUninit::uninit().assume_init() };
        unsafe {
            let p = self.wrapping_add(offset);
            std::ptr::copy_nonoverlapping(p, &mut t as *mut T as *mut u8, size_of::<T>());
        }
        t
    }
}

impl BytesMutPtrExt for *mut u8 {
    fn put<T: Copy>(&self, offset: usize, value: T) {
        unsafe {
            let p = self.wrapping_add(offset);
            std::ptr::copy_nonoverlapping(&value as *const T as *const u8, p, size_of::<T>());
        }
    }
}

// requires max_len = 16n
#[inline(always)]
pub unsafe fn mem_fast_common_prefix(
    buf: *const u8,
    p1: usize,
    p2: usize,
    max_len: usize,
) -> usize {
    for l in (0..max_len).step_by(16) {
        let bits1 = buf.get::<std::simd::u8x16>(p1 + l);
        let bits2 = buf.get::<std::simd::u8x16>(p2 + l);
        let cmp = bits1.simd_ne(bits2);
        if likely(cmp.any()) {
            return l + cmp.first_set().unwrap();
        }
    }
    max_len
}

// requires len = 4n, otherwise trailing bytes are ignored
#[inline(always)]
pub unsafe fn mem_fast_equal(
    buf: *const u8,
    p1: usize,
    p2: usize,
    len: usize,
    p2_last_dword: u32,
) -> bool {
    // first check the last 4 bytes of longest match (likely to be unequal for a
    // failed match) then perform full comparison
    p2_last_dword == buf.get(p1 + len - 4)
        && (0..len - 4).step_by(4).rev().all(|l| {
            let bits1: u32 = buf.get(p1 + l);
            let bits2: u32 = buf.get(p2 + l);
            bits1 == bits2
        })
}

// with max_match_len sentinel bytes at the end
#[inline(always)]
pub unsafe fn mem_fast_copy(buf: *mut u8, psrc: usize, pdst: usize, len: usize) {
    let mut pdst = pdst;

    // handle most common and simple cases
    if len == 4 && psrc + 4 <= pdst {
        buf.put(pdst, buf.cast_const().get::<u32>(psrc));
        return;
    }

    // handle overlapping
    while pdst - psrc < 4 {
        buf.put(pdst, buf.cast_const().get::<u32>(psrc));
        pdst += pdst - psrc;
    }

    for l in (0..len).step_by(4) {
        buf.put(pdst + l, buf.cast_const().get::<u32>(psrc + l));
    }
}
