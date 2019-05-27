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

    cargo install --git https://github.com/richox/orz --tag v1.4.0

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
| xz -5       | 26,521,436      | 53.53s      | 1.36s       |
| zstd -19    | 27,102,464      | 51.45s      | 0.23s       |
| **orz -l3** | 27,552,928      | 5.19s       | 0.90s       |
| **orz -l2** | 27,677,132      | 4.60s       | 0.91s       |
| xz -4       | 27,929,124      | 38.46s      | 1.42s       |
| **orz -l1** | 28,035,112      | 3.61s       | 0.89s       |
| **orz -l0** | 28,813,275      | 2.86s       | 0.90s       |
| bzip2       | 29,008,758      | 6.86s       | 3.86s       |
| brotli -9   | 29,685,672      | 36.63s      | 0.28s       |
| zstd -18    | 29,882,879      | 21.06s      | 0.21s       |
| brotli -8   | 30,326,580      | 19.08s      | 0.26s       |
| brotli -7   | 31,057,759      | 10.62s      | 0.27s       |
| xz -3       | 31,233,128      | 30.80s      | 1.54s       |
| zstd -17    | 31,377,355      | 9.30s       | 0.16s       |
| lzfse       | 36,157,828      | 1.72s       | 0.18s       |
| gzip -9     | 36,475,804      | 5.29s       | 0.32s       |
| gzip -6     | 36,548,933      | 4.20s       | 0.33s       |

reference:
1. zstd: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
