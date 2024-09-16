CC = aarch64-linux-gnu-gcc
# CC = gcc
CFLAGS = -march=armv8-a+sve -Wall -Wextra -fPIC -g

.PHONY: run-test-in-qemu run-test clean

libshittysveie.so: shittysveie.c
	$(CC) $(CFLAGS) -O3 -shared -o $@ $^

example/add_vecs: example/add_vecs.c example/sve_add_vecs.s
	$(CC) $(CFLAGS) -O1 -static-libgcc -o $@ $^

run-test-in-qemu: libshittysveie.so example/add_vecs
	qemu-aarch64 \
		-L /usr/aarch64-linux-gnu/ \
		-E LD_PRELOAD=$(realpath libshittysveie.so) \
		-cpu max,sve=off \
		example/add_vecs

run-test: libshittysveie.so example/add_vecs
	LD_PRELOAD=$(realpath libshittysveie.so) ./example/add_vecs

clean:
	rm -rf ./libshittysveie.so ./example/add_vecs
	rm -rf ./*.core

