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
| gzip   | 36548933        | 5.872       | 0.469       |
| orz e0 | 31157845        | 2.802       | 1.022       |
| orz e1 | 30665287        | 3.084       | 0.956       |
| orz e2 | 30209595        | 3.817       | 0.994       |
| orz e3 | 29911544        | 4.447       | 0.958       |
| orz e4 | 29705704        | 5.199       | 1.012       |
| bzip2  | 29008758        | 9.773       | 6.385       |
| xz     | 26375764        | 78.533      | 2.027       |
