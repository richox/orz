pub unsafe fn llcp_fast(buf: &[u8], p1: usize, p2: usize, max_len: usize) -> usize {
    let p1 = buf.as_ptr() as usize + p1;
    let p2 = buf.as_ptr() as usize + p2;
    let mut l = 0;

    // keep max_len=4n+2+1, so (l + 3 < max_len) is always true
    while l + 4 <= max_len && *((p1 + l) as *const u32) == *((p2 + l) as *const u32) {
        l += 4;
    }
    l += (*((p1 + l) as *const u16) == *((p2 + l) as *const u16)) as usize * 2;
    l += (*((p1 + l) as *const  u8) == *((p2 + l) as *const  u8)) as usize;
    return l;
}

pub unsafe fn memeq_hack_fast(buf: &[u8], p1: usize, p2: usize, len: usize) -> bool {
    // this function requires buf[p1+len + 0..3] == buf[p2+len + 0..3]
    if len > 0 {
        let p1 = buf.as_ptr() as usize + p1;
        let p2 = buf.as_ptr() as usize + p2;
        for l in (0 .. len).step_by(4) {
            if *((p1 + l) as *const u32) != *((p2 + l) as *const u32) {
                return false;
            }
        }
    }
    return true;
}

pub unsafe fn copy_fast(buf: &mut [u8], psrc: usize, pdst: usize, len: usize) {
    let mut psrc = buf.as_ptr() as usize + psrc;
    let mut pdst = buf.as_ptr() as usize + pdst;
    let r = pdst + len;

    while pdst - psrc < 4 {
        *(pdst as *mut u32) = *(psrc as *const u32);
        pdst += pdst - psrc;
    }
    while pdst < r {
        *((pdst + 0) as *mut u32) = *((psrc + 0) as *const u32);
        *((pdst + 4) as *mut u32) = *((psrc + 4) as *const u32);
        psrc += 8;
        pdst += 8;
    }
}
