CC = gcc
CFLAGS = -Wall -Wextra -O1 -g -D_GNU_SOURCE -DFUSE_USE_VERSION=31 $(shell pkg-config fuse --cflags)
LDFLAGS = $(shell pkg-config fuse --libs)

.PHONY: all clean

all: uolayfs test-uolayfs

clean:
	rm -f ./uolayfs

#uoverlayfs: overlayfs.c
#	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

example-empty: example-empty.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

uolayfs: uolayfs.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

test-uolayfs: uolayfs
	umount ./test/dir || true
	./uolayfs --underlying=./test/underlying --overlayed=./test/overlayed ./test/dir
