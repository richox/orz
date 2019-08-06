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

|   name    |compressed size|encode time|decode time|
|-----------|---------------|-----------|-----------|
|    xz     |  26,375,764   |  54.447s  |  1.295s   |
| zstd -19  |  26,960,368   |  57.745s  |  0.239s   |
|**orz -l3**|  27,085,611   |  5.699s   |  1.022s   |
|**orz -l2**|  27,219,370   |  5.048s   |  1.019s   |
|**orz -l1**|  27,471,482   |  4.313s   |  1.010s   |
|**orz -l0**|  27,881,216   |  3.649s   |  1.009s   |
|   bzip2   |  29,008,758   |  6.705s   |  3.264s   |
| zstd -15  |  29,543,910   |  26.929s  |  0.198s   |
| brotli -9 |  29,685,672   |  36.261s  |  0.281s   |
| brotli -8 |  30,326,580   |  18.551s  |  0.270s   |
| zstd -11  |  31,011,103   |  10.231s  |  0.177s   |
| brotli -7 |  31,057,759   |  10.375s  |  0.263s   |
|   lzfse   |  36,157,828   |  1.706s   |  0.166s   |
|   gzip    |  36,548,933   |  4.248s   |  0.335s   |
