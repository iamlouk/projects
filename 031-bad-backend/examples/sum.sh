#!/bin/bash
set -ex
clear
clear
ninja -C ./build -j 4
/home/lou/proj/llvm-backend/build/bin/opt \
	-debug -disable-output \
	-load-pass-plugin=./build/libBadCodeGen.so \
	-passes=bad-codegen ./examples/sum.ll
