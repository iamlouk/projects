# CC = aarch64-linux-gnu-gcc
CC = gcc
CFLAGS = -march=armv8-a+sve -Wall -Wextra -fPIC

.PHONY: run-test-in-qemu run-test clean

libshittysveie.so: shittysveie.c
	$(CC) $(CFLAGS) -g -O1 -shared -o $@ $^

test/add_vecs: test/add_vecs.c test/sve_add_vecs.s
	$(CC) $(CFLAGS) -g -O1 -static-libgcc -o $@ $^

run-test-in-qemu: libshittysveie.so test/add_vecs
	qemu-aarch64 \
		-L /usr/aarch64-linux-gnu/ \
		-E LD_PRELOAD=$(realpath libshittysveie.so) \
		-cpu max,sve=off \
		test/add_vecs

run-test: libshittysveie.so test/add_vecs
	LD_PRELOAD=$(realpath libshittysveie.so) ./test/add_vecs

clean:
	rm -rf ./libshittysveie.so ./test/add_vecs
	rm -rf ./*.core

