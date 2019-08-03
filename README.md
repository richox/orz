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
| xz -5       | 26,521,436      | 51.68s      | 1.28s       |
| zstd -19    | 26,960,368      | 56.85s      | 0.23s       |
| **orz -l3** | 27,085,611      | 5.54s       | 0.99s       |
| **orz -l2** | 27,219,370      | 4.89s       | 0.99s       |
| **orz -l1** | 27,471,482      | 4.14s       | 0.98s       |
| **orz -l0** | 27,881,216      | 3.47s       | 0.98s       |
| xz -4       | 27,929,124      | 37.09s      | 1.39s       |
| bzip2       | 29,008,758      | 6.70s       | 3.43s       |
| zstd -18    | 29,543,910      | 26.61s      | 0.20s       |
| brotli -9   | 29,685,672      | 35.85s      | 0.27s       |
| brotli -8   | 30,326,580      | 18.30s      | 0.27s       |
| zstd -17    | 31,011,103      | 10.01s      | 0.18s       |
| brotli -7   | 31,057,759      | 10.20s      | 0.25s       |
| xz -3       | 31,233,128      | 34.87s      | 1.51s       |
| lzfse       | 36,157,828      | 1.69s       | 0.17s       |
| gzip -9     | 36,475,804      | 5.28s       | 0.32s       |
| gzip -6     | 36,548,933      | 4.26s       | 0.34s       |
