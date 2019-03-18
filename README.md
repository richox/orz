Orz
===
this is a general purpose data compressor written in rust.

orz is mainly base on an optimized ROLZ (reduced offset Lempel-Ziv) dictionary compressor. symbols and matches are then encoded by an order-0 static huffman encoder. for better compression, there is a simplified order-1 MTF model before huffman coding.

with the great ROLZ algorithm, orz is more powerful than traditional LZ77 compressors like old gzip, zstandard from Facebook, lzfse from Apple, and brotli from Google. in our benchmark with large text (enwik8, test data of Hutter Prize), we can see that orz is faster and compressing better than other LZ77 ones, while decompression is still fast enough.

orz is completely implemented in rust. thanks to the wonderful rust compiler, we implemented orz in less than 1,000 lines of code, and the running speed is still as fast as C/C++.

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz --tag v1.2.0

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
| **orz -l4** | 28,255,600      | 5.90s       | 0.59s       |
| **orz -l3** | 28,433,390      | 4.77s       | 0.60s       |
| **orz -l2** | 28,751,813      | 3.69s       | 0.60s       |
| bzip2       | 29,008,758      | 6.88s       | 3.66s       |
| **orz -l1** | 29,285,630      | 2.97s       | 0.61s       |
| brotli -9   | 29,685,672      | 37.02s      | 0.29s       |
| zstd -15    | 29,882,879      | 21.22s      | 0.19s       |
| **orz -l0** | 30,053,328      | 2.55s       | 0.60s       |
| zstd -12    | 31,106,827      | 11.24s      | 0.18s       |
| xz -3       | 31,233,128      | 31.18s      | 1.54s       |
| zstd -9     | 31,834,628      | 4.72s       | 0.16s       |
| xz -2       | 31,989,048      | 14.67s      | 1.63s       |
| brotli -6   | 32,446,572      | 5.45s       | 0.29s       |
| xz -1       | 33,276,380      | 7.21s       | 1.77s       |
| lzfse       | 36,157,828      | 1.74s       | 0.19s       |
| gzip        | 36,548,933      | 4.23s       | 0.33s       |
| brotli -3   | 36,685,022      | 1.19s       | 0.31s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
