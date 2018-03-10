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
| gzip   | 36548933        | 5.75        | 0.46        |
| orz e0 | 31158129        | 2.59        | 0.7         |
| orz e1 | 30665434        | 2.89        | 0.7         |
| orz e2 | 30206929        | 3.34        | 0.7         |
| orz e3 | 29909393        | 3.95        | 0.7         |
| orz e4 | 29700161        | 4.57        | 0.7         |
| bzip2  | 29008758        | 9.24        | 5.75        |
| xz     | 26375764        | 73.73       | 1.95        |
