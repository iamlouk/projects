.PHONY: test test-c

TEST_FILE := ../README.md

test: test-c

test-c: base64-c
	@echo "Testing base64-c.c:"
	./base64-c < $(TEST_FILE) | base64 --decode | diff -s -q $(TEST_FILE) -

base64-c: ./base64.c
	gcc -Wall -Wextra -O2 -march=native -o $@ $<




