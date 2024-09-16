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

### 007: [Bytecode-VM comparing threaded jumps and a jump table](./007-mini-vm) *(semi-finished)*

Tags: __*C*__, __*performance*__

Somehow, [the iSH iOS app](https://github.com/ish-app/ish) manages to have a quite good emulator (still quite slow) on a platform where code generation is impossible because of the restrictions iOS enforces on paging. I knew of computed `goto`s, but have never used them before. This is my take on a bytecode VM that uses the threaded dispatch technique used in _iSH_, but without the assembly. Using that technique indeed increases performance by up to 2x for my simple testcase.

### 008: [LLVM IR Pass](./008-llvm-ir-pass)

Tags: __*C++*__, __*LLVM*__, __*Compiler*__

Similar setup as with project #6, but this time there is a `run.sh` script as well. The pass plugin contains two passes: one is a very simple strength reduction (`x / y -> x >> log2(y)` if `y` is a power of two), the other is actually a de-optimization implemented for fun/practice not actually useful. When it sees the integer formula `y = (x*(x+1))/2` it replaces that by a loop that sums up all numbers from zero to `x` ([known as gaussian summation](https://de.wikipedia.org/wiki/Gau%C3%9Fsche_Summenformel)). LLVM is actually able to reverse that optimization again!

### 009: [Lexer and parser for binary operations](./009-interpreter)

Tags: __*Rust*__, __*Lexer*__, __*Parser*__

This will be a multi-day project. The first step was a lexer and a simple parser for the most basic expressions.

### 010: [CAs in taichi-python](./010-taichi-ca)

Tags: __*CAs*__, __*Python*__, __*GPUs*__

I accidentally stumbled across [Taichi Lang](https://github.com/taichi-dev/taichi/) and was fascinated by it. As someone interested in compilers and GPU programming, the way this project works is really cool. It uses python's reflection/inspection/meta-programming in order to embed itself as a DSL into regular python code and JITs kernels. The way it works becomes clearer and clearer the more you read up on the limitations and design choices of the language. If it had proper reduction support it could be very very useful for a lot of things I did in C before. In order to play around with it I wrote a small cellular automata.

### 011: [Jump'n'Run](./011-jump-n-run) *(unfinished)*

Tags: __*HTML*__, __*JS*__, __*Game*__

A simple Jump&Run in plain vanilla JS using HTML5 canvas. Images from [this asset pack](https://jesse-m.itch.io/jungle-pack) where used.

### 012: [Very Bad Vectorizer](./012-very-bad-vectorizer)

Tags: __*C++*__, __*LLVM*__

This is probably one of the best ones yet. It is a auto-vectorizer for LLVM and has it's own [README](./012-very-bad-vectorizer/README.md).

### 013: [Rust Macro for Matching Enum Variants](./013-enum-match)

Tags: __*Rust*__, __*Macros*__

A procedural Rust macro that can be `#[derive()]`d on any enum and adds a `fn match_variants(self: &Self, other: &Self) -> bool` function. This function
returns true if the enums are of the same variant, but they can hold different values in their fields.


