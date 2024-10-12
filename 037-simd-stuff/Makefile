CC = aarch64-linux-gnu-gcc
CFLAGS = -Wall -Wextra -xc -march=armv8-a+sve+sve2 -O2

.PHONY: check-rfc3339 check-bigint clean

clean:
	rm -f ./*-test.*
	rm -f check-base64

#TMP_FILE_RFC3339 := $(shell mktemp "rfc3339-test.XXX")
#check-rfc3339: ./rfc3339.c
#	$(CC) $(CFLAGS) -DTEST=1 -static -o $(TMP_FILE_RFC3339) $<
#	qemu-aarch64 -cpu max,sve128=on $(TMP_FILE_RFC3339)
#	rm $(TMP_FILE_RFC3339)
#
#TMP_FILE_BIGINT := $(shell mktemp "bigint-test.XXX")
#check-bigint: ./bigint.c
#	$(CC) $(CFLAGS) -DTEST=1 -static -o $(TMP_FILE_BIGINT) $<
#	qemu-aarch64 -cpu max,sve256=on $(TMP_FILE_BIGINT)
#	rm $(TMP_FILE_BIGINT)

check-base64: ./base64.c
	$(CC) $(CFLAGS) -DTEST=1 -static -o $@ $<
	qemu-aarch64 -cpu max,sve128=on $@
