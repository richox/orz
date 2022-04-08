use super::byteslice::ByteSliceExt;

// assume max_len = 16n
pub unsafe fn memlcp_fast(buf: &[u8], p1: usize, p2: usize, max_len: usize) -> usize {
    for l in (0..max_len).step_by(16) {
        let num_equ_bits = if cfg!(target_endian = "little") {
            (buf.read::<u128>(p1 + l) ^ buf.read::<u128>(p2 + l)).trailing_zeros()
        } else {
            (buf.read::<u128>(p1 + l) ^ buf.read::<u128>(p2 + l)).leading_zeros()
        };

        if num_equ_bits < 128 {
            return l + num_equ_bits as usize / 8;
        }
    }
    max_len
}

// this function requires buf[p1+len + 0..3] == buf[p2+len + 0..3]
pub unsafe fn memequal_fast(buf: &[u8], p1: usize, p2: usize, len: usize) -> bool {
    (0..len)
        .step_by(4)
        .all(|i| buf.read::<[u8; 4]>(p1 + i) == buf.read::<[u8; 4]>(p2 + i))
}

// with sentinels
pub unsafe fn memcopy_fast(buf: &mut [u8], psrc: usize, pdst: usize, len: usize) {
    let mut pdst_nonoverlap = pdst;
    while pdst_nonoverlap - psrc < 4 {
        buf.write::<u32>(pdst_nonoverlap, buf.read(psrc));
        pdst_nonoverlap += pdst_nonoverlap - psrc;
    }
    (0..len)
        .step_by(4)
        .for_each(|i| buf.write::<[u8; 4]>(pdst_nonoverlap + i, buf.read(psrc + i)));
}
