.PHONY: test clean test-ref test-c test-rs test-pystdlib test-go test-java test-hare test-c-decode

# Some big file?
TEST_FILE := /tmp/big-fat-random-file.data

test: test-c test-ref test-rs test-pystdlib test-hare test-c-decode

clean:
	rm -f $(TEST_FILE)
	rm -rf *.class ./__pycache__
	rm -f base64-*

$(TEST_FILE):
	head -c 1G < /dev/urandom > $@

test-ref: $(TEST_FILE)
	@echo "Testing coreutils base64:"
	time base64 < $(TEST_FILE)                     | base64 --decode | diff -s -q $(TEST_FILE) -

test-c: base64-c $(TEST_FILE)
	@echo "Testing base64.c:"
	time ./base64-c < $(TEST_FILE)                 | base64 --decode | diff -s -q $(TEST_FILE) -

test-c-decode: base64-c
	@echo "Testing base64.c decoding:"
	./base64-c < ./base64.c | ./base64-c --decode | diff -s -q ./base64.c -

test-rs: base64-rs $(TEST_FILE)
	@echo "Testing base64.rs:"
	time ./base64-rs < $(TEST_FILE)                | base64 --decode | diff -s -q $(TEST_FILE) -

test-pystdlib: ./base64.py $(TEST_FILE)
	@echo "Tesing base64.py (using stdlib):"
	time python ./base64.py < $(TEST_FILE)         | base64 --decode | diff -s -q $(TEST_FILE) -

test-go: base64-go $(TEST_FILE)
	@echo "Tesing base64.go:"
	time ./base64-go < $(TEST_FILE)                | base64 --decode | diff -s -q $(TEST_FILE) -

test-java: base64.class $(TEST_FILE)
	@echo "Tesing base64.java:"
	time java --class-path . base64 < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

test-hare: base64-ha $(TEST_FILE)
	@echo "Tesing base64.ha:"
	time ./base64-ha < $(TEST_FILE)                | base64 --decode | diff -s -q $(TEST_FILE) -

base64-c: ./base64.c
	gcc -Wall -Wextra -O2 -march=native -o $@ $<

base64-rs: ./base64.rs
	rustc -C opt-level=2 -C debuginfo=2 --edition 2021 $< -o $@

base64-go: ./base64.go
	go build -o $@ $<

base64.class: ./base64.java
	javac -Xdiags:verbose -g --release 22 $<

base64-ha: ./base64.ha
	hare build -o $@ $<

