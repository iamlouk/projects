# One Day Projects

Things I programmed on a day or two that are too small for deserving their own repository. Ideas for the future:

- B*-Tree in Rust
- ART (Adaptive Radix Tree) in Rust with path compression
- Some generative art in JS with HTML5-Canvases
- Markdown compiler in python
- A memory allocator where `free()` is given the size of the allocation (using free-bitmaps, ...)

### 001: [Minesweeper](./001-minesweeper)

Tags: __*HTML*__, __*JS*__, __*Game*__, __*Solver*__

A playable version of Minesweeper with a simple automatic solver.

### 002: [Sudoku](./002-sudoku)

Tags: __*HTML*__, __*JS*__, __*Solver*__

A solver for Sudoku loosely based on [this python solver](https://norvig.com/sudoku.html) for the browser with visualization. The solver is very flexible and can not only solve 9x9 but also 16x16 and 25x25 grids.

### 003: [Splay Tree](./003-splay-tree)

Tags: __*C++*__, __*Data Structures*__

A very simple unoptimized and very badly but functioning implementation of a splay tree. A splay tree is self-organizing but not self-balancing: An accessed node gets moved to the root of the tree so that accassing the same elements repeatedly is very fast.

### 004: [Percentiles](./004-percentiles)

Tags: __*Python*__, __*Statistics*__

Calculate percentiles with better time complexity than sorting using _kth element_.

### 005: [Hashmaps in Rust](./005-rust-hashmaps)

Tags: __*Rust*__, __*Data Structures*__

Open and closed addressing hashmaps with some simple optimizations written in Rust.

### 006: [Super Simple Clang Plugin](./006-clang-plugin)

Tags: __*Clang*__, __*LLVM*__, __*Compiler*__

This project was more about building LLVM (14.0) and looking into the build system than the plugin itself. LLVM was build like this: `cmake -G Ninja -DLLVM_ENABLE_PROJECTS="clang;compiler-rt;lld" -DLLVM_TARGETS_TO_BUILD="RISCV" -DCMAKE_INSTALL_PREFIX="/opt/llvm" -DCMAKE_BUILD_TYPE="DEBUG" -DLLVM_OPTIMIZED_TABLEGEN="ON" ../llvm` (with `lld` as a linker and `ninja -j 1` while linking because otherwise, 16GB of main memory are not enough). Build the plugin like this: `cmake -G Ninja -DLLVM_DIR=../../../llvm/llvm-project/llvm -DClang_DIR=../../../llvm/llvm-project/clang ..` (from `./006-clang-plugin/build`). The plugin warns when a variable is assigned to itself (in its own statement).

### 007: [Bytecode-VM comparing threaded jumps and a jump table](./007-mini-vm)

Tags: __*C*__, __*performance*__

Somehow, [the iSH iOS app](https://github.com/ish-app/ish) manages to have a quite good emulator (still quite slow) on a platform where code generation is impossible because of the restrictions iOS enforces on paging. I knew of computed `goto`s, but have never used them before. This is my take on a bytecode VM that uses the threaded dispatch technique used in _iSH_, but without the assembly. Using that technique indeed increases performance by up to 2x for my simple testcase.

### 008: [LLVM IR Pass](./008-llvm-ir-pass)

Tags: __*C++*__, __*LLVM*__, __*Compiler*__

A super basic strength reduction as an LLVM pass. Similar setup as with project #6, but this time there is a `run.sh` script as well.

