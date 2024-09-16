.PHONY: test test-c

# Some big file?
TEST_FILE := /usr/lib/libLLVM.so.18.1

test: test-c test-ref test-rs

test-ref:
	@echo "Testing coreutils base64"
	time base64 < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-c: base64-c
	@echo "Testing base64.c:"
	time ./base64-c < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-rs: base64-rs
	@echo "Testing base64.rs:"
	time ./base64-rs < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

base64-c: ./base64.c
	gcc -Wall -Wextra -O2 -march=native -o $@ $<

base64-rs: ./base64.rs
	rustc -C opt-level=2 -C debuginfo=2 --edition 2021 $< -o $@
