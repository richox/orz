Orz
===
orz -- a general purpose data compressor written in rust.

[![LICENSE](https://img.shields.io/badge/license-MIT-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE)
[![Build Status](https://travis-ci.org/richox/orz.svg?branch=master)](https://travis-ci.org/richox/orz)

orz is an optimized ROLZ (reduced offset Lempel-Ziv) general purpose data compressor. input data is encoded as ROLZ-matches (reduced-offsets and match lengths), 2-byte words, and single bytes. then all encoded symbols is processed with a Move-to-Front transformer and a static huffman coder.

benefited from the ROLZ algorithm, orz compresses times faster than many other LZ-based compressors which has same compression ratio, and decompression speed is still very acceptable.

orz is completely implemented in rust. clone the repo and run `cargo build --release` to have an executable orz binary.

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz --tag v1.5.0

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for 100MB of Large Text Compression Benchmark (enwik8, see http://mattmahoney.net/dc/text.html):

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| xz -5       | 26,521,436      | 54.29s      | 1.37s       |
| zstd -19    | 26,970,792      | 57.75s      | 0.24s       |
| **orz -l3** | 27,085,309      | 5.79s       | 1.04s       |
| **orz -l2** | 27,219,940      | 5.10s       | 1.04s       |
| **orz -l1** | 27,470,377      | 4.37s       | 1.03s       |
| **orz -l0** | 27,877,713      | 3.67s       | 1.04s       |
| xz -4       | 27,929,124      | 38.75s      | 1.46s       |
| bzip2       | 29,008,758      | 6.87s       | 3.86s       |
| zstd -18    | 29,563,105      | 26.49s      | 0.20s       |
| brotli -9   | 29,685,672      | 37.17s      | 0.31s       |
| brotli -8   | 30,326,580      | 19.24s      | 0.29s       |
| zstd -17    | 31,016,135      | 10.02s      | 0.19s       |
| brotli -7   | 31,057,759      | 10.68s      | 0.27s       |
| xz -3       | 31,233,128      | 31.01s      | 1.58s       |
| lzfse       | 36,157,828      | 1.77s       | 0.19s       |
| gzip -9     | 36,475,804      | 5.20s       | 0.33s       |
| gzip -6     | 36,548,933      | 4.16s       | 0.35s       |
