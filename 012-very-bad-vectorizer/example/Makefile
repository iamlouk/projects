BUILDDIR = ../../../llvm/llvm-project/build

CC  = $(BUILDDIR)/bin/clang -mcpu=neoverse-v1 -O1 -Wall -Wextra -std=c11
OPT = $(BUILDDIR)/bin/opt -load-pass-plugin=../llvm-project/build/lib/VBVectorizer.so
LLC = $(BUILDDIR)/bin/llc

.PHONY: all clean

all: axb_test axb_test_opt
clean:
	rm -f *.ll
	rm -f *.s
	rm -f ./axb_test ./axb_test_opt

# %.o: %.c
# 	$(CC) $(CFLAGS) -c -o $@ $<
#
# %.s: %.c
# 	$(CC) $(CFLAGS) -S -o $@ $<

%.ll: %.c
	$(CC) $(CFLAGS) -S -emit-llvm -o $@ $<

%.s: %.ll
	$(LLC) $< -o $@

axb_opt.s: axb.ll
	$(OPT) -passes=vbv,dce -S -o axb_vectorized.ll axb.ll
	$(LLC) axb_vectorized.ll -o axb_opt.s

axb_test: ./axb.s ./axb_main.s
	aarch64-linux-gnu-gcc -march=armv8-a+sve -static -o $@ $^

axb_test_opt: ./axb_opt.s ./axb_main.s
	aarch64-linux-gnu-gcc -march=armv8-a+sve -static -o $@ $^


