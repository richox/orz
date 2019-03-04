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
benchmark for large text: [enwik8](http://mattmahoney.net/dc/text):

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| **orz -l4** | 28,694,399      | 5.83s       | 0.61s       |
| **orz -l3** | 28,895,950      | 4.72s       | 0.62s       |
| bzip2       | 29,008,758      | 7.11s       | 4.04s       |
| **orz -l2** | 29,244,585      | 3.48s       | 0.61s       |
| **orz -l1** | 29,795,044      | 2.83s       | 0.62s       |
| zstd -15    | 29,882,879      | 22.1s       | 0.3s        |
| **orz -l0** | 30,567,914      | 2.39s       | 0.61s       |
| zstd -12    | 31,106,827      | 12.2s       | 0.3s        |
| zstd -9     | 31,834,628      | 5.31s       | 0.27s       |
| brotli -6   | 32,446,572      | 6.14s       | 0.37s       |
| zstd -6     | 33,144,064      | 1.98s       | 0.29s       |
| zstd -3     | 35,745,324      | 0.77s       | 0.27s       |
| lzfse       | 36,157,828      | 1.9s        | 0.27s       |
| gzip        | 36,548,933      | 4.28s       | 0.37s       |
| brotli -3   | 36,685,022      | 1.3s        | 0.42s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
