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

| name   | compressed size | encode time | decode time |
|--------|-----------------|-------------|-------------|
| gzip   | 36518322        | 7.34        | 1.17        |
| orz e0 | 31158133        | 2.39        | 0.71        |
| orz e1 | 30665438        | 2.63        | 0.7         |
| orz e2 | 30206933        | 3.07        | 0.7         |
| orz e3 | 29909397        | 3.62        | 0.69        |
| orz e4 | 29700165        | 4.31        | 0.69        |
| bzip2  | 29008758        | 11.95       | 4.92        |
| xz     | 26375764        | 69.77       | 2.41        |
