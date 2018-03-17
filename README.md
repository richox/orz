introduction
============
Orz: an optimized ROLZ data-compressor written in rust

installation
============
you can install orz with cargo:

    cargo +nightly install --git https://github.com/richox/orz

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for [enwik8](http://mattmahoney.net/dc/text):

    CPU: Intel(R) Xeon(R) CPU E5-2630 v4 @ 2.20GHz
    MEM: 128GB
    OS:  Linux 3.10.0-514.16.1.el7.x86_64

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| gzip        | 36518322        | 7.38        | 1.17        |
| lzfse       | 36157828        | 5.59        | 0.39        |
| zstd -3     | 35745324        | 1.23        | 0.37        |
| zstd -6     | 33353407        | 2.45        | 0.36        |
| zstd -9     | 32061946        | 3.94        | 0.35        |
| **orz -l0** | 31155905        | 2.42        | 0.73        |
| **orz -l1** | 30577494        | 2.68        | 0.71        |
| **orz -l2** | 30110658        | 3.17        | 0.69        |
| **orz -l3** | 29768362        | 3.86        | 0.7         |
| **orz -l4** | 29575585        | 4.68        | 0.7         |
| bzip2       | 29008758        | 11.81       | 5.08        |
