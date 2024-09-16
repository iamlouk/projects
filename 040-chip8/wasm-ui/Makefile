.PHONY: all clean

all: ./lib.rs ../lib.rs
	rm -rf ./pkg
	wasm-pack build --target web
	exec python -m http.server -d . 8000
	open "http://localhost:8000/index.html"

clean:
	cargo clean
	rm -rf ./pkg/

