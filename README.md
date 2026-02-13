Orz
===
orz -- a general purpose data compressor written in the crab-lang.

[![LICENSE](https://img.shields.io/badge/license-MIT-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE-MIT)
[![LICENSE](https://img.shields.io/badge/license-APACHE-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE-APACHE)
[![Enwik8 Benchmark](https://github.com/richox/orz/actions/workflows/enwik8-benchmark.yml/badge.svg?branch=master)](https://github.com/richox/orz/actions/workflows/enwik8-benchmark.yml)

orz is an optimized ROLZ (reduced offset Lempel-Ziv) general purpose data compressor. input data is encoded as ROLZ-matches (reduced-offsets and match lengths), 2-byte words, and single bytes. then all encoded symbols are processed with a symbol ranking (aka Move-to-Front) transformer and a static huffman coder.

benefited from the ROLZ algorithm, orz compresses times faster than many other LZ-based compressors which has same compression ratio, and decompression speed is still very acceptable.

orz is completely implemented in the crab-lang. clone the repo and run `cargo build --release` to have an executable orz binary.

installation
============
you can install orz with cargo:

    cargo install orz --git https://github.com/richox/orz --tag v1.6.2

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for 100MB of Large Text Compression Benchmark (enwik8, see http://mattmahoney.net/dc/text.html):

(for latest enwik8 benchmark result, see [github actions](https://github.com/richox/orz/actions/workflows/enwik8-benchmark.yml))

|   name    |compressed size|encode time|decode time|
|-----------|---------------|-----------|-----------|
|   xz -6   |  26,665,156   |  44.936s  |  0.812s   |
|**orz -l2**|  26,892,825   |  3.360s   |  0.578s   |
| zstd -19  |  26,944,223   |  45.985s  |  0.085s   |
|**orz -l1**|  27,217,825   |  2.503s   |  0.588s   |
|**orz -l0**|  27,898,433   |  1.773s   |  0.603s   |
| bzip2 -9  |  29,008,758   |  4.279s   |  1.795s   |
| zstd -15  |  29,544,526   |  22.144s  |  0.084s   |
| brotli -9 |  29,685,672   |  9.562s   |  0.131s   |
| brotli -8 |  30,326,580   |  6.004s   |  0.139s   |
| zstd -10  |  30,697,508   |  1.958s   |  0.082s   |
| brotli -7 |  31,057,759   |  3.753s   |  0.147s   |
|   lzfse   |  36,157,828   |  0.878s   |  0.080s   |
|  gzip -6  |  36,548,933   |  2.441s   |  0.107s   |
