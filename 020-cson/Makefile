CC = gcc
CFLAGS = -Wall -Wextra -g -O2 -fsanitize=undefined,address
LDFLAGS = -fsanitize=undefined,address

.PHONY: test clean all

all:
	@echo "Hello World!"

cson.o: cson.c cson.h Makefile
	$(CC) $(CFLAGS) -c $< -o $@

test-run.o: test-run.c cson.h Makefile
	$(CC) $(CFLAGS) -c $< -o $@

test-run: test-run.o cson.o
	$(CC) $(LDFLAGS) -o $@ $^

test: test-run
	@./test-run

clean:
	rm -rf test-run *.o

