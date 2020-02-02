CFLAGS := ${CFLAGS} -Wall -O3
LDFLAGS := ${LDFLAGS} -L ../target/release -lorz

orz-cffi-demo: orz.h ../target/release/liborz.a
	${CC} ${CFLAGS} ${LDFLAGS} -o $@ $@.c
orz.h:
	(cd .. && cbindgen --crate orz -o cffi/orz.h)
