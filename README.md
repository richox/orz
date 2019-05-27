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
| xz -5       | 26,521,436      | 52.41s      | 1.34s       |
| zstd -19    | 27,102,464      | 49.78s      | 0.23s       |
| **orz -l3** | 27,544,332      | 4.78s       | 0.90s       |
| **orz -l2** | 27,666,880      | 4.16s       | 0.90s       |
| xz -4       | 27,929,124      | 37.72s      | 1.41s       |
| **orz -l1** | 28,024,886      | 3.28s       | 0.89s       |
| **orz -l0** | 28,800,210      | 2.65s       | 0.90s       |
| bzip2       | 29,008,758      | 6.79s       | 3.84s       |
| brotli -9   | 29,685,672      | 36.26s      | 0.29s       |
| zstd -18    | 29,882,879      | 20.95s      | 0.20s       |
| brotli -8   | 30,326,580      | 18.62s      | 0.27s       |
| brotli -7   | 31,057,759      | 10.44s      | 0.26s       |
| xz -3       | 31,233,128      | 30.21s      | 1.53s       |
| zstd -17    | 31,377,355      | 10.19s      | 0.16s       |
| lzfse       | 36,157,828      | 1.71s       | 0.18s       |
| gzip -9     | 36,475,804      | 5.20s       | 0.34s       |
| gzip -6     | 36,548,933      | 4.15s       | 0.33s       |

reference:
1. zstd: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
