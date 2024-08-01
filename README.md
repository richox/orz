Orz
===
orz -- a general purpose data compressor written in the crab-lang.

[![LICENSE](https://img.shields.io/badge/license-MIT-000000.svg)](https://github.com/richox/orz/blob/master/LICENSE)
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
|   xz -6   |  26,665,156   |  69.815s  |  1.309s   |
|**orz -l2**|  26,893,684   |  8.245s   |  1.414s   |
| zstd -19  |  26,942,199   |  62.931s  |  0.239s   |
|**orz -l1**|  27,220,056   |  6.714s   |  1.393s   |
|**orz -l0**|  27,896,572   |  5.209s   |  1.405s   |
| bzip2 -9  |  29,008,758   |  7.417s   |  3.538s   |
| zstd -15  |  29,544,237   |  29.860s  |  0.196s   |
| brotli -9 |  29,685,672   |  36.147s  |  0.285s   |
| brotli -8 |  30,326,580   |  17.989s  |  0.271s   |
| zstd -10  |  30,697,144   |  4.205s   |  0.192s   |
| brotli -7 |  31,057,759   |  11.730s  |  0.267s   |
|   lzfse   |  36,157,828   |  1.762s   |  0.179s   |
|  gzip -6  |  36,548,933   |  4.461s   |  0.357s   |
