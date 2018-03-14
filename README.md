introduction
============
Orz: an optimized ROLZ data-compressor written in rust

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz

benchmarks
==========
benchmark for [enwik8](http://mattmahoney.net/dc/text):

    CPU: Intel(R) Xeon(R) CPU E5-2630 v4 @ 2.20GHz
    MEM: 128GB
    OS:  Linux 3.10.0-514.16.1.el7.x86_64

| name    | compressed size | encode time | decode time |
|---------|-----------------|-------------|-------------|
| gzip    | 36518322        | 7.32        | 1.17        |
| lzfse   | 36157828        | 5.57        | 0.4         |
| zstd -3 | 35745324        | 1.24        | 0.37        |
| zstd -6 | 33353407        | 2.35        | 0.37        |
| zstd -9 | 32061946        | 4.0         | 0.36        |
| orz -l0 | 31158133        | 2.42        | 0.74        |
| orz -l1 | 30665438        | 2.63        | 0.7         |
| orz -l2 | 30206933        | 3.08        | 0.69        |
| orz -l3 | 29909397        | 3.65        | 0.68        |
| orz -l4 | 29700165        | 4.35        | 0.68        |
| bzip2   | 29008758        | 12.02       | 5.09        |
