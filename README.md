Orz
===
orz -- a general purpose data compressor written in rust.

[![LICENSE](https://img.shields.io/badge/license-MIT-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE)
[![Build Status](https://travis-ci.org/richox/orz.svg?branch=master)](https://travis-ci.org/richox/orz)

orz is an optimized ROLZ (reduced offset Lempel-Ziv) general purpose data compressor. input data is encoded as ROLZ-matches (reduced-offsets and match lengths), 2-byte words, and single bytes. then all encoded symbols is processed with a symbol ranking (aka Move-to-Front) transformer and a static huffman coder.

benefited from the ROLZ algorithm, orz compresses times faster than many other LZ-based compressors which has same compression ratio, and decompression speed is still very acceptable.

orz is completely implemented in rust. clone the repo and run `cargo build --release` to have an executable orz binary.

installation
============
you can install orz with cargo:

    cargo install orz --git https://github.com/richox/orz --tag v1.6.1

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
|   xz -6   |  26,375,764   |  71.601s  |  1.563s   |
|**orz -l2**|  27,100,084   |  8.151s   |  1.299s   |
|**orz -l1**|  27,381,156   |  6.812s   |  1.314s   |
| zstd -19  |  27,659,082   |  52.844s  |  0.244s   |
|**orz -l0**|  28,025,726   |  5.630s   |  1.327s   |
| bzip2 -9  |  29,008,758   |  8.385s   |  3.922s   |
| brotli -9 |  29,685,672   |  35.458s  |  0.340s   |
| brotli -8 |  30,326,580   |  20.501s  |  0.311s   |
| zstd -15  |  30,328,568   |  23.030s  |  0.195s   |
| brotli -7 |  31,057,759   |  12.267s  |  0.307s   |
| zstd -11  |  31,230,229   |  8.992s   |  0.206s   |
|   lzfse   |  36,157,828   |  1.976s   |  0.176s   |
|  gzip -6  |  36,518,322   |  4.948s   |  0.672s   |
