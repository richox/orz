#!/usr/bin/env python3

import sys
import os
import subprocess
import filecmp
import tempfile
import terminaltables

bench_input_filename = sys.argv[1]

def run_bench(name, encode_cmd, decode_cmd):
    print("running benchmark for %(name)s ..." % locals(), file=sys.stderr)

    try:
        ## encode
        input_filename = bench_input_filename
        output_filename = tempfile.mktemp()
        t1 = os.times()
        encode_proc = subprocess.call(encode_cmd % locals(), shell=True)
        t2 = os.times()

        ## decode
        input_filename = output_filename
        output_filename = tempfile.mktemp()
        t3 = os.times()
        decode_proc = subprocess.call(decode_cmd % locals(), shell=True)
        t4 = os.times()

        ## validate
        if not filecmp.cmp(output_filename, bench_input_filename, shallow=False):
            raise RuntimeError("running bench for '%(name)s' failed: bad decoding result" % locals())

        ## collect statistic info
        encoded_file_size = os.path.getsize(input_filename)
        encode_time_secs = round((t2[2] + t2[3]) - (t1[2] + t1[3]), 3)
        decode_time_secs = round((t4[2] + t4[3]) - (t3[2] + t3[3]), 3)
        return (name, encoded_file_size, encode_time_secs, decode_time_secs)
    except:
        return (name, -1, -1, -1)


if __name__ == "__main__":
    table_data = [("name", "compressed size", "encode time", "decode time")]
    table_data.append(run_bench("orz -l0",
            "target/release/orz encode -l0 < %(input_filename)s > %(output_filename)s 2>/dev/null",
            "target/release/orz decode     < %(input_filename)s > %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz -l1",
            "target/release/orz encode -l1 < %(input_filename)s > %(output_filename)s 2>/dev/null",
            "target/release/orz decode     < %(input_filename)s > %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz -l2",
            "target/release/orz encode -l2 < %(input_filename)s > %(output_filename)s 2>/dev/null",
            "target/release/orz decode     < %(input_filename)s > %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz -l3",
            "target/release/orz encode -l3 < %(input_filename)s > %(output_filename)s 2>/dev/null",
            "target/release/orz decode     < %(input_filename)s > %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz -l4",
            "target/release/orz encode -l4 < %(input_filename)s > %(output_filename)s 2>/dev/null",
            "target/release/orz decode     < %(input_filename)s > %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("gzip",
            "gzip    < %(input_filename)s >%(output_filename)s",
            "gzip -d < %(input_filename)s >%(output_filename)s"))
    table_data.append(run_bench("bzip2",
            "bzip2    < %(input_filename)s >%(output_filename)s",
            "bzip2 -d < %(input_filename)s >%(output_filename)s"))
    table_data.append(run_bench("lzfse",
            "lzfse -encode < %(input_filename)s > %(output_filename)s",
            "lzfse -decode < %(input_filename)s > %(output_filename)s"))
    table_data.append(run_bench("zstd -3",
            "zstd -3 < %(input_filename)s > %(output_filename)s",
            "zstd -d < %(input_filename)s > %(output_filename)s"))
    table_data.append(run_bench("zstd -6",
            "zstd -6 < %(input_filename)s > %(output_filename)s",
            "zstd -d < %(input_filename)s > %(output_filename)s"))
    table_data.append(run_bench("zstd -9",
            "zstd -9 < %(input_filename)s > %(output_filename)s",
            "zstd -d < %(input_filename)s > %(output_filename)s"))

    table_data[1:] = sorted(table_data[1:], key = lambda row: -row[1])
    table = terminaltables.GithubFlavoredMarkdownTable(table_data)
    print(table.table)
