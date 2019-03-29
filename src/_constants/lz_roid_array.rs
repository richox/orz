#![allow(dead_code)]

use std::io::Write;

pub const LZ_MF_BUCKET_ITEM_SIZE: usize = 2046;
pub const LZ_ROID_SIZE: usize = 22;

pub fn generate() {
    let robitlens = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, /* ... */];
    let fenc_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_ENCODING_ARRAY.txt");
    let fdec_dest_path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("LZ_ROID_DECODING_ARRAY.txt");
    let mut fenc = std::io::BufWriter::new(std::fs::File::create(&fenc_dest_path).unwrap());
    let mut fdec = std::io::BufWriter::new(std::fs::File::create(&fdec_dest_path).unwrap());

    write!(fenc, "[").unwrap();
    write!(fdec, "[").unwrap();

    let mut match_index = 0;
    let mut current_roid = 0;
    while match_index < LZ_MF_BUCKET_ITEM_SIZE {
        let robitlen = if current_roid < robitlens.len() {
            robitlens[current_roid]
        } else {
            robitlens[robitlens.len() - 1]
        };
        write!(fdec, "({}, {}), ", match_index, robitlen).unwrap();

        for current_rest_bits in 0 .. (1 << robitlen) {
            if match_index < LZ_MF_BUCKET_ITEM_SIZE {
                write!(fenc, "({}, {}, {}), ", current_roid, robitlen, current_rest_bits).unwrap();
                match_index += 1;
            }
        }
        current_roid += 1;
    }
    current_roid += current_roid % 2;

    write!(fenc, "]").unwrap();
    write!(fdec, "]").unwrap();
    assert_eq!(current_roid, LZ_ROID_SIZE);
}
