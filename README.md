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

    cargo install --git https://github.com/richox/orz --tag v1.6.0

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
|   xz -6   |  26,375,764   | 100.013s  |  2.236s   |
| zstd -19  |  26,960,368   |  97.262s  |  0.355s   |
|**orz -l3**|  27,085,621   |  10.861s  |  1.898s   |
|**orz -l2**|  27,219,380   |  9.684s   |  1.856s   |
|**orz -l1**|  27,471,492   |  8.505s   |  1.828s   |
|**orz -l0**|  27,881,226   |  7.222s   |  1.821s   |
| bzip2 -9  |  29,008,758   |  11.253s  |  5.975s   |
| zstd -15  |  29,543,910   |  46.876s  |  0.254s   |
| brotli -9 |  29,685,672   |  55.985s  |  0.465s   |
| brotli -8 |  30,326,580   |  30.885s  |  0.440s   |
| zstd -11  |  31,011,103   |  17.575s  |  0.215s   |
| brotli -7 |  31,057,759   |  17.642s  |  0.422s   |
|   lzfse   |  36,157,828   |  2.902s   |  0.252s   |
|  gzip -6  |  36,518,322   |  6.454s   |  0.984s   |
