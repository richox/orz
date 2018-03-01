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

| name   | compressed size | encode time | decode time |
|--------|-----------------|-------------|-------------|
| gzip   | 36548933        | 5.68        | 0.46        |
| orz e0 | 31165286        | 2.55        | 0.9         |
| orz e1 | 30668965        | 2.85        | 0.89        |
| orz e2 | 30213669        | 3.29        | 0.88        |
| orz e3 | 29914316        | 3.88        | 0.88        |
| orz e4 | 29706240        | 4.53        | 0.88        |
| bzip2  | 29008758        | 9.27        | 5.68        |
| xz     | 26375764        | 74.21       | 1.96        |
