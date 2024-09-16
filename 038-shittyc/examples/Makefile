CC = riscv64-elf-gcc
CFLAGS = -march=rv64g -O1 -Wall -Wextra

.PHONY: all

all: add.s fibs.s

%.s: %.c
	$(CC) $(CFLAGS) -S -o $@ $<
