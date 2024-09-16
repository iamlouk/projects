#!/bin/bash

set -ex

make clean
make sudoku
LSP_TO_PATCH="solve|eliminate_possibilities|print_game" ./sudoku

