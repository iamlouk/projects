.PHONY: clean check

clean:
	rm -f ./*.o ./*.s ./btree

check: $(shell find . -name '*.ha')
	hare test .
