#include <stdio.h>
#include <string.h>
#include "orz.h"

int main(int argc, char** argv) {
    LZCfg cfg;
    cfg.match_depth = 48;
    cfg.lazy_match_depth1 = 32;
    cfg.lazy_match_depth2 = 16;

    if (argc != 4) {
        fprintf(stderr, "usage: orz-cffi-demo encode/decode <input-file> <output-file>\n");
        return -1;
    }

    if (strcmp(argv[1], "encode") == 0) {
        const Stat* stat = orz_encode_path(argv[2], argv[3], &cfg);
        fprintf(stderr, "%lld => %lld\n", stat->source_size, stat->target_size);
        orz_free_stat((Stat*) stat);

    } else if (strcmp(argv[1], "decode") == 0) {
        const Stat* stat = orz_decode_path(argv[2], argv[3]);
        fprintf(stderr, "%lld <= %lld\n", stat->source_size, stat->target_size);
        orz_free_stat((Stat*) stat);

    } else {
        fprintf(stderr, "invalid operation: %s\n", argv[1]);
        return -1;
    }
    return 0;
}
