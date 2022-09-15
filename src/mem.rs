#[inline(always)]
pub unsafe fn mem_get<T: Copy>(buf: *const u8, pos: usize) -> T {
    *(buf.add(pos) as *const T)
}

#[inline(always)]
pub unsafe fn mem_put<T: Copy>(buf: *mut u8, pos: usize, value: T) {
    *(buf.add(pos) as *mut T) = value
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
        let bits1: u128 = mem_get(buf, p1 + l);
        let bits2: u128 = mem_get(buf, p2 + l);
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
    let p1_last_dword: u32 = mem_get(buf, p1 + len - 4);

    // first check the last 4 bytes of longest match (likely to be unequal for a failed match)
    // then perform full comparison
    p1_last_dword == p2_last_dword
        && (0..len - 4).step_by(4).rev().all(|l| {
            let bits1: u32 = mem_get(buf, p1 + l);
            let bits2: u32 = mem_get(buf, p2 + l);
            bits1 == bits2
        })
}

// with max_match_len sentinel bytes at the end
#[inline(always)]
pub unsafe fn mem_fast_copy(buf: *mut u8, psrc: usize, pdst: usize, len: usize) {
    let mut pdst = pdst;

    // handle most common and simple cases
    if len == 4 && psrc + 4 <= pdst {
        mem_put(buf, pdst, mem_get::<u32>(buf, psrc));
        return;
    }

    // handle overlapping
    while pdst - psrc < 4 {
        mem_put(buf, pdst, mem_get::<u32>(buf, psrc));
        pdst += pdst - psrc;
    }

    for l in (0..len).step_by(4) {
        mem_put(buf, pdst + l, mem_get::<u32>(buf, psrc + l));
    }
}
