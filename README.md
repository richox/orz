Orz
===
this is a general purpose data compressor written in rust.

orz is mainly base on an optimized ROLZ (reduced offset Lempel-Ziv) dictionary compressor. symbols and matches are then encoded by an order-0 static huffman encoder. for better compression, there is a simplified order-1 MTF model before huffman coding.

with the great ROLZ algorithm, orz is more powerful than traditional LZ77 compressors like old gzip, zstandard from Facebook, lzfse from Apple, and brotli from Google. in our benchmark with large text (enwik8, test data of Hutter Prize), we can see that orz is faster and compressing better than other LZ77 ones, while decompression is still fast enough.

orz is completely implemented in rust. thanks to the wonderful rust compiler, we implemented orz in less than 1,000 lines of code, and the running speed is still as fast as C/C++.

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz --tag v1.1.0

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for large text: [enwik8](http://mattmahoney.net/dc/text):

    CPU: Intel(R) Xeon(R) CPU E5-2630 v4 @ 2.20GHz
    MEM: 128GB
    OS:  Linux 3.10.0-514.16.1.el7.x86_64

| name          | compressed size | encode time | decode time |
|---------------|-----------------|-------------|-------------|
| bzip2         | 29,008,758      | 6.89s       | 4.11s       |
| **orz -l4**   | 29,464,782      | 3.3s        | 0.55s       |
| **orz -l3**   | 29,619,560      | 2.82s       | 0.55s       |
| **orz -l2**   | 29,846,453      | 2.67s       | 0.59s       |
| zstandard -15 | 29,882,879      | 21.9s       | 0.28s       |
| **orz -l1**   | 30,174,484      | 2.18s       | 0.54s       |
| **orz -l0**   | 30,638,905      | 1.91s       | 0.57s       |
| zstandard -12 | 31,106,827      | 11.9s       | 0.28s       |
| zstandard -9  | 31,834,628      | 5.07s       | 0.26s       |
| brotli -6     | 32,446,572      | 6.09s       | 0.36s       |
| zstandard -6  | 33,144,064      | 1.93s       | 0.26s       |
| zstandard -3  | 35,745,324      | 0.76s       | 0.28s       |
| lzfse         | 36,157,828      | 1.85s       | 0.27s       |
| gzip          | 36,548,933      | 4.25s       | 0.38s       |
| brotli -3     | 36,685,022      | 1.3s        | 0.44s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
