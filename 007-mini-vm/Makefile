CC = clang
CFLAGS = -Wall -Wextra -std=c11 -g -O0 -fsanitize=address -Wno-unused-parameter

.PHONY: all clean

all: minivm

minivm: minivm.c
	$(CC) $(CFLAGS) -o $@ $^

clean:
	rm -rf *.o
	rm -rf minivm

