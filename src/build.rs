use std::io::Write;

pub const LZ_MF_BUCKET_ITEM_SIZE: usize = 3070;
pub const LZ_ROID_SIZE: usize = 21;
pub const LZ_LENID_SIZE: usize = 6;
pub const MTF_NUM_SYMBOLS: usize = 256 + LZ_ROID_SIZE * LZ_LENID_SIZE + 1;

#[allow(dead_code)]
fn generate_extra_bits_enc(count: usize, get_extra_bitlen: &dyn Fn(usize) -> usize) -> Vec<(usize, usize, usize)> {
    let mut encs = vec![];
    let mut base = 0;
    let mut current_id = 0;

    while base < count {
        let bit_len = get_extra_bitlen(current_id);
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

#[allow(dead_code)]
fn generate_extra_bits_dec(count: usize, get_extra_bitlen: &dyn Fn(usize) -> usize) -> Vec<(usize, usize)> {
    let mut decs = vec![];
    let mut base = 0;
    let mut current_id = 0;

    while base < count {
        let bit_len = get_extra_bitlen(current_id);
        decs.push((base, bit_len));
        current_id += 1;
        base += 1 << bit_len;
    }
    return decs;
}

#[allow(dead_code)]
fn main() {
    println!("cargo:rerun-if-changed=src/build.rs");

    // generete LZ_ROID_ENCODING/DECODING_ARRAY
    let rolzenc_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_ENCODING_ARRAY.txt");
    let rolzdec_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_DECODING_ARRAY.txt");
    let mtfnext_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("MTF_NEXT_ARRAY.txt");
    let mut frolzenc = std::io::BufWriter::new(std::fs::File::create(&rolzenc_dest_path).unwrap());
    let mut frolzdec = std::io::BufWriter::new(std::fs::File::create(&rolzdec_dest_path).unwrap());
    let mut fmtfnext = std::io::BufWriter::new(std::fs::File::create(&mtfnext_dest_path).unwrap());

    let get_extra_bitlen = |i| i / 2;
    let rolzencs = generate_extra_bits_enc(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bitlen);
    let rolzdecs = generate_extra_bits_dec(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bitlen);
    write!(frolzenc, "{:?}", rolzencs).unwrap();
    write!(frolzdec, "{:?}", rolzdecs).unwrap();
    assert_eq!(rolzencs.len(), LZ_MF_BUCKET_ITEM_SIZE);
    assert_eq!(rolzdecs.len(), LZ_ROID_SIZE);

    // generate MTF_NEXT_ARRAY
    write!(fmtfnext, "{:?}", (0 .. MTF_NUM_SYMBOLS)
        .map(|i| (i as f64 * 0.9999).powf(1.0 - 0.08 * i as f64 / MTF_NUM_SYMBOLS as f64) as usize)
        .collect::<Vec<_>>()).unwrap();
}
