pub mod lz_roid_array;
pub mod mtf_array;

fn generate_extra_bit_codes_enc(count: usize, get_extra_bit_len: &Fn(usize) -> usize) -> Vec<(usize, usize, usize)> {
    let mut encs = vec![];
    let mut base = 0;
    let mut current_id = 0;

    while base < count {
        let bit_len = get_extra_bit_len(current_id);
        for rest_bits in 0 .. (1 << bit_len) {
            if base < count {
                encs.push((current_id, bit_len, rest_bits));
                base += 1;
            }
        }
        current_id += 1;
    }
    return encs;
}

fn generate_extra_bit_codes_dec(count: usize, get_extra_bit_len: &Fn(usize) -> usize) -> Vec<(usize, usize)> {
    let mut decs = vec![];
    let mut base = 0;
    let mut current_id = 0;

    while base < count {
        let bit_len = get_extra_bit_len(current_id);
        decs.push((base, bit_len));
        current_id += 1;
        base += 1 << bit_len;
    }
    return decs;
}
