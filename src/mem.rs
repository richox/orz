use super::auxility::ByteSliceExt;

// assume max_len = 4n+2+1
pub unsafe fn llcp_fast(buf: &[u8], p1: usize, p2: usize, max_len: usize) -> usize {
    let mut l = (0 .. max_len)
        .step_by(4)
        .find(|i| buf.read::<u32>(p1 + i) != buf.read::<u32>(p2 + i))
        .unwrap_or(max_len / 4 * 4);
    l += (buf.read::<u16>(p1 + l) == buf.read::<u16>(p2 + l)) as usize * 2;
    l += (buf.read::<u8> (p1 + l) == buf.read::<u8> (p2 + l)) as usize;
    return l;
}

// this function requires buf[p1+len + 0..3] == buf[p2+len + 0..3]
pub unsafe fn memequ_hack_fast(buf: &[u8], p1: usize, p2: usize, len: usize) -> bool {
    return (0 .. len).step_by(4).all(|i| buf.read::<u32>(p1 + i) == buf.read::<u32>(p2 + i));
}

// assume max_len = 4n+2+1
pub unsafe fn copy_fast(buf: &mut [u8], psrc: usize, pdst: usize, len: usize) {
    let mut pdst_nonoverlap = pdst;
    while pdst_nonoverlap - psrc < 4 {
        buf.write::<u32>(pdst_nonoverlap, buf.read(psrc));
        pdst_nonoverlap += pdst_nonoverlap - psrc;
    }
    (0 .. len).step_by(4).for_each(|i| buf.write::<u32>(pdst_nonoverlap + i, buf.read(psrc + i)));
}
