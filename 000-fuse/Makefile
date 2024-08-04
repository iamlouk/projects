CC = gcc
CFLAGS = -Wall -Wextra -O1 -g -DFUSE_USE_VERSION=31 $(shell pkg-config fuse --cflags)
LDFLAGS = $(shell pkg-config fuse --libs)


uoverlayfs: overlayfs.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

example-empty: example-empty.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^
