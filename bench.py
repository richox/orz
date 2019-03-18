#!/usr/bin/env python3

import sys
import os
import subprocess
import filecmp
import tempfile
import terminaltables

bench_ifn = sys.argv[1]

def run_bench(name, encode_cmd, decode_cmd):
    print("running benchmark for %(name)s ..." % locals(), file=sys.stderr)

    try:
        encode_time_secs_min = 1e300
        decode_time_secs_min = 1e300

        for i in range(3):
            ## encode
            ifn = bench_ifn
            ofn = tempfile.mktemp()
            t1 = os.times()
            encode_proc = subprocess.call(encode_cmd % locals(), shell=True)
            t2 = os.times()
            encode_time_secs_min = min(encode_time_secs_min, t2.children_user - t1.children_user)

            ## decode
            ifn = ofn
            ofn = tempfile.mktemp()
            t3 = os.times()
            decode_proc = subprocess.call(decode_cmd % locals(), shell=True)
            t4 = os.times()
            decode_time_secs_min = min(encode_time_secs_min, t4.children_user - t3.children_user)

            ## validate
            if not filecmp.cmp(ofn, bench_ifn, shallow=False):
                raise RuntimeError("running bench for '%(name)s' failed: bad decoding result" % locals())

        ## collect statistic info
        encoded_file_size = "{:,}".format(os.path.getsize(ifn))
        encode_time_secs = "{:0.02f}s".format(encode_time_secs_min)
        decode_time_secs = "{:0.02f}s".format(decode_time_secs_min)
        return (name, encoded_file_size, encode_time_secs, decode_time_secs)
    except:
        return (name, "-1", "-1s", "-1s")


if __name__ == "__main__":
    table_data = [("name", "compressed size", "encode time", "decode time")]
    table_data.append(run_bench("**orz -l0**",
            "target/release/orz encode --silent -l0 < %(ifn)s > %(ofn)s 2>/dev/null",
            "target/release/orz decode --silent      < %(ifn)s > %(ofn)s 2>/dev/null"))
    table_data.append(run_bench("**orz -l1**",
            "target/release/orz encode --silent -l1 < %(ifn)s > %(ofn)s 2>/dev/null",
            "target/release/orz decode --silent     < %(ifn)s > %(ofn)s 2>/dev/null"))
    table_data.append(run_bench("**orz -l2**",
            "target/release/orz encode --silent -l2 < %(ifn)s > %(ofn)s 2>/dev/null",
            "target/release/orz decode --silent     < %(ifn)s > %(ofn)s 2>/dev/null"))
    table_data.append(run_bench("**orz -l3**",
            "target/release/orz encode --silent -l3 < %(ifn)s > %(ofn)s 2>/dev/null",
            "target/release/orz decode --silent     < %(ifn)s > %(ofn)s 2>/dev/null"))
    table_data.append(run_bench("**orz -l4**",
            "target/release/orz encode --silent -l4 < %(ifn)s > %(ofn)s 2>/dev/null",
            "target/release/orz decode --silent     < %(ifn)s > %(ofn)s 2>/dev/null"))
    table_data.append(run_bench("gzip",
            "gzip    < %(ifn)s >%(ofn)s",
            "gzip -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("bzip2",
            "bzip2    < %(ifn)s >%(ofn)s",
            "bzip2 -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("xz -1",
            "xz -1 < %(ifn)s >%(ofn)s",
            "xz -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("xz -2",
            "xz -2 < %(ifn)s >%(ofn)s",
            "xz -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("xz -3",
            "xz -3 < %(ifn)s >%(ofn)s",
            "xz -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("brotli -3",
            "brotli -3 < %(ifn)s >%(ofn)s",
            "brotli -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("brotli -6",
            "brotli -6 < %(ifn)s >%(ofn)s",
            "brotli -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("brotli -9",
            "brotli -9 < %(ifn)s >%(ofn)s",
            "brotli -d < %(ifn)s >%(ofn)s"))
    table_data.append(run_bench("lzfse",
            "lzfse -encode < %(ifn)s > %(ofn)s",
            "lzfse -decode < %(ifn)s > %(ofn)s"))
    table_data.append(run_bench("zstd -9",
            "zstd -9 < %(ifn)s > %(ofn)s",
            "zstd -d < %(ifn)s > %(ofn)s"))
    table_data.append(run_bench("zstd -12",
            "zstd -12< %(ifn)s > %(ofn)s",
            "zstd -d < %(ifn)s > %(ofn)s"))
    table_data.append(run_bench("zstd -15",
            "zstd -15< %(ifn)s > %(ofn)s",
            "zstd -d < %(ifn)s > %(ofn)s"))

    table_data[1:] = sorted(table_data[1:], key = lambda row: int(row[1].replace(",", "")))
    table = terminaltables.GithubFlavoredMarkdownTable(table_data)
    print(table.table)
