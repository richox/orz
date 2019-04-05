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
benchmark for 100MB of large text (enwik8, see http://mattmahoney.net/dc/text):

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| xz -5       | 26,521,436      | 53.53s      | 1.36s       |
| zstd -19    | 27,102,464      | 51.45s      | 0.23s       |
| **orz -l3** | 27,552,928      | 5.19s       | 0.90s       |
| **orz -l2** | 27,677,132      | 4.60s       | 0.91s       |
| xz -4       | 27,929,124      | 38.46s      | 1.42s       |
| **orz -l1** | 28,035,112      | 3.61s       | 0.89s       |
| **orz -l0** | 28,813,275      | 2.86s       | 0.90s       |
| bzip2       | 29,008,758      | 6.86s       | 3.86s       |
| brotli -9   | 29,685,672      | 36.63s      | 0.28s       |
| zstd -18    | 29,882,879      | 21.06s      | 0.21s       |
| brotli -8   | 30,326,580      | 19.08s      | 0.26s       |
| brotli -7   | 31,057,759      | 10.62s      | 0.27s       |
| xz -3       | 31,233,128      | 30.80s      | 1.54s       |
| zstd -17    | 31,377,355      | 9.30s       | 0.16s       |
| lzfse       | 36,157,828      | 1.72s       | 0.18s       |
| gzip -9     | 36,475,804      | 5.29s       | 0.32s       |
| gzip -6     | 36,548,933      | 4.20s       | 0.33s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
