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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/build.rs");
    let out_dir = std::env::var("OUT_DIR")?;
    let out_dir_path = std::path::Path::new(&out_dir);

    // generete LZ_ROID_ENCODING/DECODING_ARRAY
    let get_extra_bitlen = |i| i / 2;
    let lz_roid_encs = generate_extra_bits_enc(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bitlen);
    let lz_roid_decs = generate_extra_bits_dec(LZ_MF_BUCKET_ITEM_SIZE, &get_extra_bitlen);
    assert_eq!(lz_roid_encs.len(), LZ_MF_BUCKET_ITEM_SIZE);
    assert_eq!(lz_roid_decs.len(), LZ_ROID_SIZE);
    write!(std::fs::File::create(&out_dir_path.join("LZ_ROID_ENCODING_ARRAY.txt"))?, "{:?}", lz_roid_encs)?;
    write!(std::fs::File::create(&out_dir_path.join("LZ_ROID_DECODING_ARRAY.txt"))?, "{:?}", lz_roid_decs)?;

    // generate MTF_NEXT_ARRAY
    write!(std::fs::File::create(&out_dir_path.join("MTF_NEXT_ARRAY.txt"))?, "{:.0?}", [vec![0.0; 2], (2 .. MTF_NUM_SYMBOLS)
        .map(|i| i as f64)
        .map(|i| i.powf(1.0 - 0.08 * i / MTF_NUM_SYMBOLS as f64).trunc())
        .collect()
    ].concat())?;

    return Ok(());
}
