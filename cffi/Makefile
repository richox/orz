CFLAGS := ${CFLAGS} -Wall -O3
LDFLAGS := ${LDFLAGS} -L ../target/release -lorz
CBINDGEN := cbindgen --crate orz

orz_cffi_demo: orz.h ../target/release/liborz.a
	${CC} ${CFLAGS} ${LDFLAGS} -o $@ $@.c

orz.h: cbindgen.toml
	cd .. && ${CBINDGEN} -o cffi/$@ -c cffi/$<

clean:
	rm -f orz.h orz_cffi_demo orz_cffi_demo.exe

.PHONY: clean
