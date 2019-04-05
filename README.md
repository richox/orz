Orz
===
this is a general purpose data compressor written in rust.

[![LICENSE](https://img.shields.io/badge/license-MIT-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE)
[![Build Status](https://travis-ci.org/richox/orz.svg?branch=master)](https://travis-ci.org/richox/orz)

orz is mainly base on an optimized ROLZ (reduced offset Lempel-Ziv) dictionary compressor. symbols and matches are then encoded by an order-0 static huffman encoder. for better compression, there is a simplified order-1 MTF model before huffman coding.

with the great ROLZ algorithm, orz is more powerful than traditional LZ77 compressors like old gzip, zstandard from Facebook, lzfse from Apple, and brotli from Google. in our benchmark with large text (enwik8, test data of Hutter Prize), we can see that orz is faster and compressing better than other LZ77 ones, while decompression is still fast enough.

orz is completely implemented in rust. thanks to the wonderful rust compiler, we implemented orz in about 1,000 lines of code, and the running speed is still as fast as C/C++.

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz --tag v1.3.0

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for large text (first 30,000,000 bytes of enwik8, see http://mattmahoney.net/dc/text):

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| xz -5       | 8,048,216       | 15.46s      | 0.41s       |
| zstd -19    | 8,188,985       | 14.98s      | 0.07s       |
| **orz -l3** | 8,348,933       | 1.48s       | 0.27s       |
| **orz -l2** | 8,383,834       | 1.33s       | 0.28s       |
| xz -4       | 8,431,296       | 11.32s      | 0.43s       |
| **orz -l1** | 8,487,735       | 1.07s       | 0.27s       |
| **orz -l0** | 8,719,611       | 0.85s       | 0.27s       |
| bzip2       | 8,741,833       | 2.05s       | 1.16s       |
| brotli -9   | 8,989,105       | 10.12s      | 0.09s       |
| zstd -18    | 9,009,720       | 6.23s       | 0.06s       |
| brotli -8   | 9,168,056       | 5.65s       | 0.08s       |
| brotli -7   | 9,374,298       | 3.35s       | 0.09s       |
| xz -3       | 9,412,204       | 9.15s       | 0.46s       |
| zstd -17    | 9,457,738       | 2.79s       | 0.05s       |
| lzfse       | 10,905,067      | 0.53s       | 0.06s       |
| gzip -9     | 11,002,632      | 1.53s       | 0.10s       |
| gzip -6     | 11,025,348      | 1.23s       | 0.10s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
