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
benchmark for large text ([enwik8](http://mattmahoney.net/dc/text)):

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| **orz -l3** | 28,025,473      | 4.68s       | 0.71s       |
| **orz -l2** | 28,133,276      | 4.13s       | 0.71s       |
| **orz -l1** | 28,464,107      | 3.30s       | 0.71s       |
| bzip2       | 29,008,758      | 6.82s       | 3.74s       |
| **orz -l0** | 29,220,761      | 2.71s       | 0.72s       |
| brotli -9   | 29,685,672      | 36.69s      | 0.27s       |
| zstd -15    | 29,882,879      | 20.95s      | 0.19s       |
| zstd -12    | 31,106,827      | 11.11s      | 0.17s       |
| xz -3       | 31,233,128      | 30.76s      | 1.51s       |
| zstd -9     | 31,834,628      | 4.60s       | 0.18s       |
| xz -2       | 31,989,048      | 14.32s      | 1.59s       |
| brotli -6   | 32,446,572      | 5.33s       | 0.27s       |
| xz -1       | 33,276,380      | 7.13s       | 1.70s       |
| lzfse       | 36,157,828      | 1.75s       | 0.18s       |
| gzip        | 36,548,933      | 4.11s       | 0.34s       |
| brotli -3   | 36,685,022      | 1.17s       | 0.32s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
