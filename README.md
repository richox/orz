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
| **orz -l4** | 28,392,133      | 5.93s       | 0.68s       |
| **orz -l3** | 28,599,408      | 4.75s       | 0.62s       |
| **orz -l2** | 28,958,614      | 3.62s       | 0.7s        |
| bzip2       | 29,008,758      | 6.94s       | 4.05s       |
| **orz -l1** | 29,491,891      | 2.93s       | 0.66s       |
| zstd -15    | 29,882,879      | 21.5s       | 0.29s       |
| **orz -l0** | 30,209,045      | 2.66s       | 0.66s       |
| zstd -12    | 31,106,827      | 11.3s       | 0.26s       |
| zstd -9     | 31,834,628      | 4.89s       | 0.27s       |
| brotli -6   | 32,446,572      | 5.94s       | 0.39s       |
| zstd -6     | 33,144,064      | 1.88s       | 0.29s       |
| zstd -3     | 35,745,324      | 0.75s       | 0.27s       |
| lzfse       | 36,157,828      | 1.85s       | 0.27s       |
| gzip        | 36,548,933      | 4.16s       | 0.36s       |
| brotli -3   | 36,685,022      | 1.29s       | 0.44s       |

reference:
1. zstandard: https://github.com/facebook/zstd
2. brotli: https://github.com/google/brotli
3. lzfse: https://github.com/lzfse/lzfse
