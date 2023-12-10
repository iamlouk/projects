CC = gcc
CFLAGS = $(shell cat ./compile_flags.txt) -O1 -g -fsanitize=address

.PHONY: clean

port2proto: ./proto2port.c ../../020-cson/cson.c
	$(CC) $(CFLAGS) -o $@ $^

clean:
	rm -f port2proto ./*.o

