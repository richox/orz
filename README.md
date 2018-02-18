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
| orz e0 | 31157845        | 2.46        | 0.94        |
| orz e1 | 30665287        | 2.79        | 0.94        |
| orz e2 | 30209595        | 3.32        | 0.95        |
| orz e3 | 29911544        | 4.07        | 0.94        |
| orz e4 | 29705704        | 4.77        | 0.93        |
| bzip2  | 29008758        | 9.2         | 5.86        |
| xz     | 26375764        | 73.87       | 1.96        |
