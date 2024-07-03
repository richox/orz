pub trait BytesConstPtrExt {
    fn get<T: Default + Copy>(&self, offset: usize, num_bytes: usize) -> T;
}

pub trait BytesMutPtrExt {
    fn put<T: Default + Copy>(&self, offset: usize, value: T, num_bytes: usize);
}

impl BytesConstPtrExt for *const u8 {
    fn get<T: Default + Copy>(&self, offset: usize, num_bytes: usize) -> T {
        let mut t = T::default();
        unsafe {
            let p = self.wrapping_add(offset);
            std::ptr::copy_nonoverlapping(p, &mut t as *mut T as *mut u8, num_bytes);
        }
        t
    }
}

impl BytesMutPtrExt for *mut u8 {
    fn put<T: Default + Copy>(&self, offset: usize, value: T, num_bytes: usize) {
        unsafe {
            let p = self.wrapping_add(offset);
            std::ptr::copy_nonoverlapping(&value as *const T as *const u8, p, num_bytes);
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
        let bits1: u128 = buf.get::<u128>(p1 + l, 16);
        let bits2: u128 = buf.get::<u128>(p2 + l, 16);
        let bitmask = bits1 ^ bits2;
        let num_equal_bits = if cfg!(target_endian = "little") {
            bitmask.trailing_zeros()
        } else {
            bitmask.leading_zeros()
        };

        if num_equal_bits < 128 {
            return l + num_equal_bits as usize / 8;
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
    let p1_last_dword: u32 = buf.get(p1 + len - 4, 4);

    // first check the last 4 bytes of longest match (likely to be unequal for a failed match)
    // then perform full comparison
    p1_last_dword == p2_last_dword
        && (0..len - 4).step_by(4).rev().all(|l| {
            let bits1: u32 = buf.get(p1 + l, 4);
            let bits2: u32 = buf.get(p2 + l, 4);
            bits1 == bits2
        })
}

// with max_match_len sentinel bytes at the end
#[inline(always)]
pub unsafe fn mem_fast_copy(buf: *mut u8, psrc: usize, pdst: usize, len: usize) {
    let mut pdst = pdst;

    // handle most common and simple cases
    if len == 4 && psrc + 4 <= pdst {
        buf.put(pdst, buf.cast_const().get::<u32>(psrc, 4), 4);
        return;
    }

    // handle overlapping
    while pdst - psrc < 4 {
        buf.put(pdst, buf.cast_const().get::<u32>(psrc, 4), 4);
        pdst += pdst - psrc;
    }

    for l in (0..len).step_by(4) {
        buf.put(pdst + l, buf.cast_const().get::<u32>(psrc + l, 4), 4);
    }
}
