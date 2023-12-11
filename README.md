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
|   xz -6   |  26,375,764   |  71.601s  |  1.563s   |
|**orz -l2**|  27,100,084   |  8.151s   |  1.299s   |
|**orz -l1**|  27,381,156   |  6.812s   |  1.314s   |
| zstd -19  |  27,659,082   |  52.844s  |  0.244s   |
|**orz -l0**|  28,025,726   |  5.630s   |  1.327s   |
| bzip2 -9  |  29,008,758   |  8.385s   |  3.922s   |
| brotli -9 |  29,685,672   |  35.458s  |  0.340s   |
| brotli -8 |  30,326,580   |  20.501s  |  0.311s   |
| zstd -15  |  30,328,568   |  23.030s  |  0.195s   |
| brotli -7 |  31,057,759   |  12.267s  |  0.307s   |
| zstd -11  |  31,230,229   |  8.992s   |  0.206s   |
|   lzfse   |  36,157,828   |  1.976s   |  0.176s   |
|  gzip -6  |  36,518,322   |  4.948s   |  0.672s   |

benchmark for 400MB of text data of Global Data Compression Competition (TS40.txt, see https://globalcompetition.compression.ru/#leaderboards):
|   name    |compressed size|encode time|decode time|
|-----------|---------------|-----------|-----------|
|   xz -6   |  108,677,096  | 335.738s  |  5.887s   |
| bzip2 -9  |  109,502,210  |  35.331s  |  15.986s  |
|**orz -l2**|  111,844,429  |  31.955s  |  4.890s   |
| zstd -19  |  112,679,835  | 252.155s  |  1.050s   |
|**orz -l1**|  113,065,821  |  26.168s  |  4.799s   |
|**orz -l0**|  116,003,142  |  20.172s  |  4.785s   |
| zstd -15  |  123,100,586  | 110.805s  |  0.878s   |
| brotli -9 |  124,453,389  | 144.100s  |  1.422s   |
| brotli -8 |  126,791,079  |  78.620s  |  1.281s   |
| zstd -11  |  127,940,149  |  40.962s  |  0.827s   |
| brotli -7 |  129,425,945  |  45.338s  |  1.245s   |
|  gzip -6  |  146,656,915  |  25.237s  |  2.662s   |
|   lzfse   |  147,579,002  |  8.220s   |  0.832s   |
