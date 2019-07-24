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
| xz -5       | 26,521,436      | 104.81s     | 2.39s       |
| zstd -19    | 26,960,368      | 105.50s     | 0.42s       |
| **orz -l3** | 27,148,974      | 9.90s       | 1.65s       |
| **orz -l2** | 27,317,490      | 8.60s       | 1.69s       |
| **orz -l1** | 27,730,608      | 7.20s       | 1.76s       |
| xz -4       | 27,929,124      | 81.65s      | 2.54s       |
| **orz -l0** | 28,541,618      | 5.59s       | 1.65s       |
| bzip2       | 29,008,758      | 11.10s      | 5.43s       |
| zstd -18    | 29,543,910      | 52.38s      | 0.30s       |
| brotli -9   | 29,685,672      | 57.88s      | 0.47s       |
| brotli -8   | 30,326,580      | 32.85s      | 0.45s       |
| zstd -17    | 31,011,103      | 18.46s      | 0.28s       |
| brotli -7   | 31,057,759      | 18.32s      | 0.42s       |
| xz -3       | 31,233,128      | 57.22s      | 2.72s       |
| lzfse       | 36,157,828      | 4.87s       | 0.27s       |
| gzip -9     | 36,445,241      | 8.60s       | 0.96s       |
| gzip -6     | 36,518,322      | 6.87s       | 0.96s       |
