#!/usr/bin/env python3

import sys
import os
import time
import subprocess
import filecmp
import tempfile
import terminaltables

bench_input_filename = sys.argv[1]

def run_bench(name, encode_cmd, decode_cmd):
    print("running benchmark for %(name)s ..." % locals(), file=sys.stderr)

    ## encode
    input_filename = bench_input_filename
    output_filename = tempfile.mktemp()
    t1 = time.time()
    encode_proc = subprocess.call(encode_cmd % locals(), shell=True)
    t2 = time.time()

    ## decode
    input_filename = output_filename
    output_filename = tempfile.mktemp()
    t3 = time.time()
    decode_proc = subprocess.call(decode_cmd % locals(), shell=True)
    t4 = time.time()

    ## validate
    if not filecmp.cmp(output_filename, bench_input_filename, shallow=False):
        raise RuntimeError("running bench for '%(name)s' failed: bad decoding result" % locals())

    ## collect statistic info
    encoded_file_size = os.path.getsize(input_filename)
    encode_time_secs = round(t2 - t1, 3)
    decode_time_secs = round(t4 - t3, 3)
    return (name, encoded_file_size, encode_time_secs, decode_time_secs)

if __name__ == "__main__":
    table_data = [("name", "compressed size", "encode time", "decode time")]
    table_data.append(run_bench("orz e0",
            "target/release/orz e0 %(input_filename)s %(output_filename)s 2>/dev/null",
            "target/release/orz d  %(input_filename)s %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz e1",
            "target/release/orz e1 %(input_filename)s %(output_filename)s 2>/dev/null",
            "target/release/orz d  %(input_filename)s %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz e2",
            "target/release/orz e2 %(input_filename)s %(output_filename)s 2>/dev/null",
            "target/release/orz d  %(input_filename)s %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz e3",
            "target/release/orz e3 %(input_filename)s %(output_filename)s 2>/dev/null",
            "target/release/orz d  %(input_filename)s %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("orz e4",
            "target/release/orz e4 %(input_filename)s %(output_filename)s 2>/dev/null",
            "target/release/orz d  %(input_filename)s %(output_filename)s 2>/dev/null"))
    table_data.append(run_bench("gzip",
            "gzip    < %(input_filename)s >%(output_filename)s",
            "gzip -d < %(input_filename)s >%(output_filename)s"))
    table_data.append(run_bench("bzip2",
            "bzip2    < %(input_filename)s >%(output_filename)s",
            "bzip2 -d < %(input_filename)s >%(output_filename)s"))
    table_data.append(run_bench("xz",
            "xz    < %(input_filename)s >%(output_filename)s",
            "xz -d < %(input_filename)s >%(output_filename)s"))

    table_data[1:] = sorted(table_data[1:], key = lambda row: -row[1])
    table = terminaltables.GithubFlavoredMarkdownTable(table_data)
    print(table.table)
