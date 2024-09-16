#!/bin/bash

LLVM_BIN_DIR="/opt/llvm/bin"
PLUGIN="../build/helloworldplugin.so"

$LLVM_BIN_DIR/clang -fplugin=$PLUGIN ./example.c

