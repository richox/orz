#![allow(dead_code)]

use std::io::Write;

pub const LZ_MF_BUCKET_ITEM_SIZE: usize = 3070;
pub const LZ_ROID_SIZE: usize = 21;

pub fn generate() {
    let fenc_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_ENCODING_ARRAY.txt");
    let fdec_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_DECODING_ARRAY.txt");
    let mut fenc = std::io::BufWriter::new(std::fs::File::create(&fenc_dest_path).unwrap());
    let mut fdec = std::io::BufWriter::new(std::fs::File::create(&fdec_dest_path).unwrap());

    let get_extra_bit_len = |i| i / 2;
    let encs = super::generate_extra_bit_codes_enc(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bit_len);
    let decs = super::generate_extra_bit_codes_dec(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bit_len);
    write!(fenc, "{:?}", encs).unwrap();
    write!(fdec, "{:?}", decs).unwrap();
    assert_eq!(encs.len(), LZ_MF_BUCKET_ITEM_SIZE);
    assert_eq!(decs.len(), LZ_ROID_SIZE);
}
