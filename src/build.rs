mod _constants;

fn main() {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=src/_constants/lz_roid_array.rs");
    println!("cargo:rerun-if-changed=src/_constants/mtf_array.rs");

    _constants::lz_roid_array::generate();
    _constants::mtf_array::generate();
}
