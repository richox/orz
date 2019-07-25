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
| xz -5       | 26,521,436      | 54.38s      | 1.38s       |
| zstd -19    | 26,970,792      | 58.16s      | 0.24s       |
| **orz -l3** | 27,113,462      | 6.21s       | 1.11s       |
| **orz -l2** | 27,283,020      | 5.17s       | 1.10s       |
| **orz -l1** | 27,693,225      | 4.07s       | 1.10s       |
| xz -4       | 27,929,124      | 39.26s      | 1.40s       |
| **orz -l0** | 28,500,436      | 3.26s       | 1.11s       |
| bzip2       | 29,008,758      | 6.92s       | 3.95s       |
| zstd -18    | 29,563,105      | 26.68s      | 0.20s       |
| brotli -9   | 29,685,672      | 37.07s      | 0.29s       |
| brotli -8   | 30,326,580      | 19.24s      | 0.27s       |
| zstd -17    | 31,016,135      | 10.00s      | 0.18s       |
| brotli -7   | 31,057,759      | 10.71s      | 0.26s       |
| xz -3       | 31,233,128      | 31.47s      | 1.60s       |
| lzfse       | 36,157,828      | 1.74s       | 0.18s       |
| gzip -9     | 36,475,804      | 5.36s       | 0.33s       |
| gzip -6     | 36,548,933      | 4.21s       | 0.33s       |
