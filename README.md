introduction
============
Orz: an optimized ROLZ data-compressor written in rust.

main features:

<table border="0" class="xxx">
    <tr>
        <td><b>lightweight</b></td>
        <td>implemented in less than 1,000 lines of code.</td>
    </tr>
    <tr>
        <td><b>fast</b></td>
        <td>~30% faster than gzip in slowest mode, 3 times faster in fastest mode.</td>
    </tr>
    <tr>
        <td><b>powerful</b></td>
        <td>compression ratio is ~20% better than gzip.</td>
    </tr>
</table>

installation
============
you can install orz with cargo:

    cargo +nightly install --git https://github.com/richox/orz --tag v0.2.0

usage
=====

for compression:

    orz encode <source-file-input> <compressed-file-output>

for decompression:

    orz decode <compressed-file-input> <source-file-output>

for more details, see `orz --help`

benchmarks
==========
benchmark for [enwik8](http://mattmahoney.net/dc/text):

    CPU: Intel(R) Xeon(R) CPU E5-2630 v4 @ 2.20GHz
    MEM: 128GB
    OS:  Linux 3.10.0-514.16.1.el7.x86_64

| name        | compressed size | encode time | decode time |
|-------------|-----------------|-------------|-------------|
| gzip        | 36518322        | 7.39        | 1.17        |
| lzfse       | 36157828        | 5.63        | 0.39        |
| zstd -3     | 35745324        | 1.24        | 0.38        |
| zstd -6     | 33353407        | 2.34        | 0.38        |
| zstd -9     | 32061946        | 3.95        | 0.36        |
| **orz -l0** | 31155905        | 2.34        | 0.72        |
| **orz -l1** | 30577494        | 2.63        | 0.71        |
| **orz -l2** | 30110658        | 3.11        | 0.69        |
| **orz -l3** | 29768362        | 3.85        | 0.7         |
| **orz -l4** | 29575585        | 4.66        | 0.7         |
| bzip2       | 29008758        | 12.05       | 4.89        |
