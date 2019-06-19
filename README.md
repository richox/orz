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
| xz -5       | 26,521,436      | 107.56s     | 2.41s       |
| zstd -19    | 26,960,368      | 106.72s     | 0.40s       |
| **orz -l3** | 27,523,734      | 7.93s       | 1.63s       |
| **orz -l2** | 27,647,744      | 7.45s       | 1.54s       |
| xz -4       | 27,929,124      | 81.89s      | 2.53s       |
| **orz -l1** | 28,005,806      | 5.95s       | 1.64s       |
| **orz -l0** | 28,780,465      | 4.68s       | 1.76s       |
| bzip2       | 29,008,758      | 11.28s      | 6.03s       |
| zstd -18    | 29,543,910      | 54.58s      | 0.35s       |
| brotli -9   | 29,685,672      | 63.09s      | 0.54s       |
| brotli -8   | 30,326,580      | 35.86s      | 0.48s       |
| zstd -17    | 31,011,103      | 21.27s      | 0.29s       |
| brotli -7   | 31,057,759      | 19.66s      | 0.47s       |
| xz -3       | 31,233,128      | 63.24s      | 2.72s       |
| lzfse       | 36,157,828      | 4.96s       | 0.25s       |
| gzip -9     | 36,445,241      | 8.54s       | 0.94s       |
| gzip -6     | 36,518,322      | 6.89s       | 0.94s       |
