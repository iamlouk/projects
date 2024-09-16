.PHONY: test clean test-ref test-c test-rs test-pystdlib test-go

# Some big file?
TEST_FILE := /tmp/big-fat-random-file.data

test: test-c test-ref test-rs test-pystdlib

clean:
	rm -rf $(TEST_FILE)

$(TEST_FILE):
	head -c 1G < /dev/urandom > $@

test-ref: $(TEST_FILE)
	@echo "Testing coreutils base64:"
	time base64 < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-c: base64-c $(TEST_FILE)
	@echo "Testing base64.c:"
	time ./base64-c < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-rs: base64-rs $(TEST_FILE)
	@echo "Testing base64.rs:"
	time ./base64-rs < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-pystdlib: ./base64.py $(TEST_FILE)
	@echo "Tesing base64.py (using stdlib):"
	time python ./base64.py < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-go: base64-go $(TEST_FILE)
	@echo "Tesing base64.go:"
	time ./base64-go < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

base64-c: ./base64.c
	gcc -Wall -Wextra -O2 -march=native -o $@ $<

base64-rs: ./base64.rs
	rustc -C opt-level=2 -C debuginfo=2 --edition 2021 $< -o $@

base64-go: ./base64.go
	go build -o $@ $<
