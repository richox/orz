Orz
===
this is a general purpose data compressor written in rust.

orz is mainly base on an optimized ROLZ (reduced offset Lempel-Ziv) dictionary compressor. symbols and matches are then encoded by an order-0 static huffman encoder. for better compression, there is a simplified order-1 MTF model before huffman coding.

with the great ROLZ algorithm, orz is more powerful than traditional LZ77 compressors like old gzip, zstandard from Facebook, lzfse from Apple, and brotli from Google. in our benchmark with large text (enwik8, test data of Hutter Prize), we can see that orz is faster and compressing better than other LZ77 ones, while decompression is still fast enough.

orz is completely implemented in rust. thanks to the wonderful rust compiler, we implemented orz in less than 1,000 lines of code, and the running speed is still as fast as C/C++.

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz --tag v1.0.0

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

| name         | compressed size | encode time | decode time |
|--------------|-----------------|-------------|-------------|
| brotli -3    | 36,685,022      | 2.16s       | 0.57s       |
| gzip         | 36,518,322      | 7.33s       | 1.18s       |
| LZFSE        | 36,157,828      | 3.38s       | 0.39s       |
| ZStandard -3 | 35,745,324      | 1.2s        | 0.37s       |
| ZStandard -6 | 33,353,407      | 2.34s       | 0.37s       |
| brotli -6    | 32,446,572      | 8.08s       | 0.5s        |
| ZStandard -9 | 32,061,946      | 4.0s        | 0.38s       |
| **orz -l0**  | 30,788,239      | 2.53s       | 0.76s       |
| **orz -l1**  | 30,323,561      | 2.88s       | 0.75s       |
| **orz -l2**  | 29,991,780      | 3.35s       | 0.75s       |
| **orz -l3**  | 29,761,976      | 3.92s       | 0.73s       |
| **orz -l4**  | 29,611,080      | 4.6s        | 0.74s       |
| bzip2        | 29,008,758      | 11.9s       | 4.91s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
