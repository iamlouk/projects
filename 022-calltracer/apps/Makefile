CC = gcc
CFLAGS = -no-pie -g -O1 -Wall -Wextra -fno-inline -fpatchable-function-entry=42 -fsanitize=address
LDFLAGS = -no-pie -L.. -fsanitize=address

.PHONY: clean

all: sudoku hello-world

%.o: %.c
	$(CC) $(CFLAGS) $< -c -o $@

sudoku: sudoku.o ../libselfpatch.o
	$(CC) $(LDFLAGS) $^ -o $@

hello-world: hello-world.o ../libselfpatch.o
	$(CC) $(LDFLAGS) $^ -o $@

clean:
	rm -f ./*.o ../libselfpatch.o
	rm -f ./hello-world ./sudoku

