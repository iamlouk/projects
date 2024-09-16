#!/bin/bash

set -e

clang -O0 -S -emit-llvm ./example.c -o ./example.ll

(cd ./build && make)

opt -load-pass-plugin ./build/libHelloWorld.so -passes=hello-world -S ./example.ll

