CC = clang --target=wasm32
CFLAGS = --no-standard-libraries -Wl,--export-all -Wl,--no-entry -O1

.PHONY: all clean

all: id.wasm

%.wasm: %.c
	$(CC) $(CFLAGS) -o $@ $<

clean:
	rm -f ./*.wasm

