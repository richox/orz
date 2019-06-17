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
| xz -5       | 26,521,436      | 105.66s     | 2.37s       |
| zstd -19    | 26,960,368      | 103.19s     | 0.38s       |
| **orz -l3** | 27,523,734      | 8.47s       | 1.68s       |
| **orz -l2** | 27,647,744      | 7.36s       | 1.85s       |
| xz -4       | 27,929,124      | 80.01s      | 2.42s       |
| **orz -l1** | 28,005,806      | 6.06s       | 1.62s       |
| **orz -l0** | 28,780,465      | 4.86s       | 1.62s       |
| bzip2       | 29,008,758      | 11.38s      | 6.13s       |
| zstd -18    | 29,543,910      | 51.05s      | 0.30s       |
| brotli -9   | 29,685,672      | 61.37s      | 0.49s       |
| brotli -8   | 30,326,580      | 32.38s      | 0.45s       |
| zstd -17    | 31,011,103      | 18.88s      | 0.27s       |
| brotli -7   | 31,057,759      | 18.66s      | 0.43s       |
| xz -3       | 31,233,128      | 64.29s      | 2.68s       |
| lzfse       | 36,157,828      | 4.94s       | 0.25s       |
| gzip -9     | 36,445,241      | 8.52s       | 0.92s       |
| gzip -6     | 36,518,322      | 6.87s       | 0.93s       |
