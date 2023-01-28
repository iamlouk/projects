QEMU_DIR = $(HOME)/uni/archiv/ma/upstream-qemu

CROSS_CC = /opt/riscv/bin/riscv64-unknown-linux-gnu-gcc
CROSS_CFLAGS = -O1 -gdwarf-4 -static

HOST_CXX = g++
HOST_LIBASAN = $(shell g++ -print-file-name=libasan.so)
HOST_CXX_INCLUDES = -I./libelfin/elf -I./libelfin/dwarf -I$(QEMU_DIR)/include
HOST_CXXFLAGS = $(HOST_CXX_INCLUDES) -Wall -Wextra -Wno-unused-parameter -std=c++17
HOST_LDFLAGS = -L./libelfin/elf -L./libelfin/dwarf -lelf++ -ldwarf++

.PHONY: all run-example clean

all: ./example/hello-world.bin ./liblines.so

./example/hello-world.bin: ./example/hello-world.c
	$(CROSS_CC) $(CROSS_CFLAGS) -o $@ $^

./liblines.so: ./plugin.cc
	$(HOST_CXX) $(HOST_CXXFLAGS) $(HOST_LDFLAGS) -O2 -g -fPIC -shared -Wl,-soname,$@ -o $@ $^

run-example: ./example/hello-world.bin ./liblines.so
	LD_LIBRARY_PATH="./libelfin/elf:./libelfin/dwarf" \
	QEMU_EXEC_BINARY="./example/hello-world.bin" \
	QEMU_EXIT_ADDRESS="000257ac" \
		qemu-riscv64 -plugin ./liblines.so "./example/hello-world.bin"

clean:
	rm -r ./example/*.bin
	rm -r ./liblines.so

