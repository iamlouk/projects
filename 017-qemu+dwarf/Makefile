QEMU_DIR = $(HOME)/uni/archiv/ma/upstream-qemu

CROSS_CC = /opt/riscv/bin/riscv64-unknown-linux-gnu-gcc
CROSS_CFLAGS = -O1 -gdwarf-4 -static

HOST_CXX = g++
HOST_CXXFLAGS = -I./libelfin/elf -I./libelfin/dwarf -I$(QEMU_DIR)/include -Wall -Wextra -Wno-unused-parameter
HOST_LDFLAGS = -L./libelfin/elf -L./libelfin/dwarf -lelf++ -ldwarf++

.PHONY: all run-example

all: ./example/hello-world.riscv ./liblines.so

./example/hello-world.riscv: ./example/hello-world.c
	$(CROSS_CC) $(CROSS_CFLAGS) -o $@ $^

./liblines.so: ./plugin.cc
	$(HOST_CXX) $(HOST_CXXFLAGS) $(HOST_LDFLAGS) -fPIC -shared -Wl,-soname,$@ -o $@ $^

run-example: ./example/hello-world.riscv ./liblines.so
	LD_LIBRARY_PATH="./libelfin/elf:./libelfin/dwarf" \
		$(QEMU_DIR)/build/qemu-riscv64 \
		-plugin ./liblines.so \
		./example/hello-world.riscv

