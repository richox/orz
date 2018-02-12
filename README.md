introduction
============
Orz: an optimized ROLZ data-compressor written in rust

installation
============
you can install orz with cargo:

    cargo install --git https://github.com/richox/orz

benchmarks
==========
benchmark for enwik8:

| name   | compressed size | encode time | decode time |
|--------|-----------------|-------------|-------------|
| gzip   | 36548933        | 5.83        | 0.47        |
| orz e0 | 31157845        | 2.61        | 1.08        |
| orz e1 | 30665287        | 3.12        | 1.0         |
| orz e2 | 30209595        | 3.72        | 0.99        |
| orz e3 | 29911544        | 4.49        | 0.98        |
| orz e4 | 29705704        | 5.31        | 0.96        |
| bzip2  | 29008758        | 9.47        | 6.32        |
| xz     | 26375764        | 78.33       | 2.0         |
