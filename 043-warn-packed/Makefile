.PHONY: clean test

build/packedwarnings.so: ./lib.cpp
	cd ./build && cmake -G Ninja -DCMAKE_BUILD_TYPE=Debug -DLLVM_DIR=$(llvm-config --cmakedir) ..
	ninja -C ./build

test: build/packedwarnings.so
	clang -fplugin=build/packedwarnings.so -O1 -S -x c -o - - <<< "union utype { struct { unsigned a, b; } __attribute__((packed)); char raw[7]; };"

clean:
	rm -rf ./build
