# Hello World!

Projects to small for their own repository or just small little things I want to try out.

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

Tags: __*C*__, __*Performance*__

Somehow, [the iSH iOS app](https://github.com/ish-app/ish) manages to have a quite good emulator (still quite slow) on a platform where code generation is impossible because of the restrictions iOS enforces on paging. I knew of computed `goto`s, but have never used them before. This is my take on a bytecode VM that uses the threaded dispatch technique used in _iSH_, but without the assembly. Using that technique indeed increases performance by up to 2x for my simple testcase.

### 008: [LLVM IR Pass](./008-llvm-ir-pass)

Tags: __*C++*__, __*LLVM*__, __*Compiler*__

Similar setup as with project #6, but this time there is a `run.sh` script as well. The pass plugin contains two passes: one is a very simple strength reduction (`x / y -> x >> log2(y)` if `y` is a power of two), the other is actually a de-optimization implemented for fun/practice not actually useful. When it sees the integer formula `y = (x*(x+1))/2` it replaces that by a loop that sums up all numbers from zero to `x` ([known as gaussian summation](https://de.wikipedia.org/wiki/Gau%C3%9Fsche_Summenformel)). LLVM is actually able to reverse that optimization again!

### 009: [Lexer and parser for binary operations](./009-interpreter) *(unfinished/abandoned, see rhall instead)*

Tags: __*Rust*__, __*Lexer*__, __*Parser*__

This will be a multi-day project. The first step was a lexer and a simple parser for the most basic expressions.

### 010: [CAs in taichi-python](./010-taichi-ca)

Tags: __*CAs*__, __*Python*__, __*GPUs*__

I accidentally stumbled across [Taichi Lang](https://github.com/taichi-dev/taichi/) and was fascinated by it. As someone interested in compilers and GPU programming, the way this project works is really cool. It uses python's reflection/inspection/meta-programming in order to embed itself as a DSL into regular python code and JITs kernels. The way it works becomes clearer and clearer the more you read up on the limitations and design choices of the language. If it had proper reduction support it could be very very useful for a lot of things I did in C before. In order to play around with it I wrote a small cellular automata.

### 011: [Jump'n'Run](./011-jump-n-run) *(unfinished/abandoned)*

Tags: __*HTML*__, __*JS*__, __*Game*__

A simple Jump&Run in plain vanilla JS using HTML5 canvas. Images from [this asset pack](https://jesse-m.itch.io/jungle-pack) where used.

### 012: [Very Bad Vectorizer](./012-very-bad-vectorizer)

Tags: __*C++*__, __*LLVM*__

The worst but maybe simplest vectorizer you have ever seen. It is a auto-vectorizer for LLVM and has it's own [README](./012-very-bad-vectorizer/README.md).

### 013: [Rust Macro for Matching Enum Variants](./013-enum-match)

Tags: __*Rust*__, __*Macros*__

A procedural Rust macro that can be `#[derive()]`d on any enum and adds a `fn match_variants(self: &Self, other: &Self) -> bool` function. This function returns true if the enums are of the same variant, but they can hold different values in their fields.

### 015: [Shitty SVE Instruction Emulator](./015-shitty-sve-ie)

Tags: __*SVE*__, __*C*__, __*AArch64*__

SVE (Arm's Scalable Vector Extensions) is cool, but hardware supporting it is hard to get a.t.m.. This project is a proof of concept for a `LD_PRELOAD` and `SIGILL`-handler based instruction emulator for AArch64 hardware without SVE for SVE instructions. It only supports 5 instructions in order to run a super simple vectorized vector-add operation. The VL/VScale can be changed. It is super slow (especially for me as I do not have AArch64 hardware and had to test inside of a `qemu-system-aarch64` VM (Btw: I learned that user-space `qemu-aarch` does not correctly emulate the `mcontext_t` struct).

The directory also contains me experimenting with how the ARM Scalable Matrix Extensions could be used in the future. Once a stable version of gcc or clang support ARM SME, I would like to come back to this project and add support for SME in the library.

### 016: [Adaptive Radix Tree](./016-art)

Tags: __*C++*__, __*ART*__, __*Data Structures*__

A bad, unoptimized implementation of a [Adaptive Radix Tree](https://db.in.tum.de/~leis/papers/ART.pdf).

### 017: [QEMU Plugin](./017-qemu+dwarf)

Tags: __*DWARF*__, __*libelfin*__, __*QEMU*__

This is a very simple very basic QEMU [TCG Plugin](https://qemu.readthedocs.io/en/latest/devel/tcg-plugins.html) for user mode emulation. Like everything here it is only a proof-of-concept, and though it works, it could be improved in a lot of ways. It uses [libelfin](https://github.com/aclements/libelfin) to parse the debug information of the emulated binary, tracks how often every translation block was executed, maps translation blocks to source file line numbers, and prints the line numbers of the most executed pieces of code at the end of the emulation.

### 018: [Go SSH App](./018-ssh-snake)

Tags: __*Golang*__, __*SSH*__

Golang is a cool language for networking applications because of builtin channels and the great standard library. There is a SSH server and client implementation for Go ([golang.org/x/crypto/ssh](https://pkg.go.dev/golang.org/x/crypto/ssh)) which can be used for basically anything, not just remote terminals. This project contains a SSH server that one can connect to (e.g. via `ssh -p 2022 localhost`) where the different users can play a snake-like game on the same plane.

### 019: [Loop-Control GCC plugin](./019-gcc-loop-plugin)

Tags: __*GCC*__, __*GIMPLE*__, __*C*__

This is a plugin for GCC 13 that injects two function calls into every loop: `__gcclc_loop_preheader` is called every time the preheader of a loop is executed (so once before it is entered), and `__gcclc_loop_header` is called every time the loop header is executed (so once per iteration). The user can then link a runtime library to the compiled code containing those two functions (a super simple one is provided). If `__gcclc_loop_header` returns `0`, the loop is exited, regardless of what the actual loop condition says. This is obviously completely useless, unless maybe to really annoy someone and make them question reality. The Makefile is hopefully simple enough to see how to use it.

### 020: [CSON](./020-cson)

Tags: __*C*__, __*JSON*__

A simple JSON parser written in pure good-old C. I have not written plain C in a while, I already had forgotten how `fread` and so on works :D.

### 021: [Rust-JSON](./021-rust-json)

Tags: __*Rust*__, __*JSON*__, __*Macros*__

Yet another Rust Macro/Metaprogramming experiment. This time: `#[derive(JSONifyable)]`, a derive macro (and trait) that automatically generates a `to_json` method for structs. Look at the test in `src/lib.rs` for how it is applied.

### 022: [Self-Patching Tracing Library](./022-calltracer)

Tags: __*C*__, __*Linux*__, __*amd64*__

This is a fucked-up experiment that nobody should use. This small library can patch (AT RUNTIME!) a x86 binary on Linux and inject calls to a tracer function for selected functions. The binary needs to be compiled with `-fpatchable-function-entry=42`, but the patching itself happens at runtime! The tracer function can be linked in, but a default (weak symbol) is provided that prints `"intercepted: <name>"` and counts the number of calls to the traced function. Functions to trace can be selected using `LSP_TO_PATCH=<pattern>|<pattern>|...`.

### 023: [Port2Proto](./023-proto2port) *(unfinished)*

Tags: __*Networking*__, __*Linux*__, __*Go*__/__*C*__

Some firewalls only allow traffic for a very select amount of ports (e.g. 80/443), not only from the outside in (thats obviously needed for security!), but also from the inside out (which is annoying). The idea behind this mini-PoC (which I might actually use for once) is a server that listens on a specified port (e.g. 80 because it's almost always allowed, its for HTTP), and then redirects the incoming connections to a *different* port depending on the protocol (e.g. HTTP -> 8080, SSH -> 2022, ...). [This inspired me as well](https://blog.cloudflare.com/sockmap-tcp-splicing-of-the-future/).

### 024: [Command-Line Tetris In Rust](./024-rustris)

Tags: __*Rust*__, __*Game*__

Command line tetris.

### 025: [Rhall](./025-rhall)

Tags: __*Rust*__, __*Interpreter*__

A tiny super bad interpreter for a [Dhall](https://github.com/dhall-lang/dhall-lang)-like language subset, but with recursion! The prototype works, but is missing a lot of futures. I might come back to it to improve it and fix a known bug in the type checker. All [examples](./025-rhall/examples) work.

### 026: [YAC: Yet Another Compression](./026-compress)

Tags: __*Rust*__, __*Huffmann-Encoding*__, __*LZ77*__

A sliding-window compression and decompression in Rust using Huffmann encoding and LZ77. It is inspired by [how the *deflate* algorithm works](https://www.zlib.net/feldspar.html), but is super super shitty and crap and not at all how it actually works.

### 027: [SystemC](./027-testing-systemc)

Tags: __*SystemC*__

Refreshing some super basic System-C skills.

### 028: [Python JIT](./028-python-jit)

Tags: __*LLVM*__, __*Python*__, __*JIT*__

A `@jit` Python decorator that causes the given function to become a DSL and be jit-ted. Only a very very basic prototype.


### 029: [Game Of Living](./029-gol)

Tags: __*CAs*__, __*Rust*__

A [cellular automata inspired by this](https://www.youtube.com/watch?v=WMJ1H3Ai-qs). I played a lot with CAs a few years ago, and wanted to try out some new rules here.

### 030: [MNIST Classifier](./030-mnist/) *(unfinished)*

Tags: __*MNIST*__, __*DL*__

A neural network from scratch in Rust. This might get abandoned pretty soon, it is very far from being done.

### 031: [Expression Templates In Rust](./031-rust-exprtmpl)

Tags: __*Rust*__, __*ExprTmpl*__

A proof-of-concept for expression templates in Rust.

### 032: [Base64](./032-base64)

Tags: __*Base64*__, __*Encoding*__, __*Java*__, __*Hare*__, __*C*__, __*Go*__

A mini-benchmark of base64 encoding (and decoding in C) in different languages. Are there more optimizations possible? Maybe I will come back to this one. To my surprise, "chunking" does not help with performance. More possible future features: Allow whitespace when decoding, add newlines in the output when encoding. The [Hare](https://harelang.org/) implementation is a bit disappointingly slow, clearly QBE is not even close to 75% of GCC/LLVM as it claims, and I don't think this need some super-fancy compiler opt..

### 033: [BARE](./033-bare/)

Tags: __*Go*__, __*Encoding*__

Some fun (nothing useful) with implementing parts of the [BARE](https://datatracker.ietf.org/doc/draft-devault-bare/) message format and its schema language.

### 034: [A WebSocket Server Protocol Impl.](./034-websocket)

Tags: __*Hare*__, __*Go*__, __*WebSocket*__

This is a from-scratch implementation of the [WebSocket protocol (RFC6455)](https://datatracker.ietf.org/doc/html/rfc6455), only using the stdlib, in both Golang and the Hare programming language. The Golang version is a simple echo service, the Hare impl. is more flexible and implements a basic chatroom. It is only a simple subset of the server side of the protocol. For anything serious, there is [golang.org/x/net/websocket](https://pkg.go.dev/golang.org/x/net/websocket), which I used to test my server against. The directory also contains a simple TCP echo server in Hare. *TODO*: Use [hare-ev](https://git.sr.ht/~sircmpwn/hare-ev) to avoid blocking IO in the websocket server?

### 035: [FUSE version of a overlay FS](./035-fuse-overlayfs/)

Tags: __*C*__, __*FUSE*__

In a past job, I worked with union/overlay file systems like [overlayfs](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/fs/overlayfs). They are extremely useful sometimes. This project is a extremely incomplete (at the time of writing, creating and deleting files and directories works, but no read/write of actual files) and shitty version of such a file system where you have a underlying and a overlay directory. The underlying one is never changed, all changes are represented in the overlay dir.

### 036: [A B-Tree in HARE](./036-b-tree/)

Tags: __*Hare*__, __*B-Tree*__

Working B-Tree with variable key and value sizes, but currently without persistence or deletion. I plan on maybe extending this impl. to a basic persistent key-value store. There are memory leaks etc. in the currnet impl and I know that it is shit, but it was fun to implement a copy-on-write B-Tree and I designed it so that it can be extended for persistence etc.. Also, node-splitting and all of that works.

### 037: [SIMD stuff](./037-simd-stuff/)

Tags: __*C*__, __*SIMD*__

Just some experiments with using SIMD to parse text, specifically big integers in base 10.

### 038: [Shitty C](./038-shittyc/)

Tags: __*C*__, __*Rust*__, __*Compiler*__

A extremely shitty compiler for a C subset. Very few features for now, but some basic stuff works. Needs a lot more stuff to become even close to useful and more tests. The lexer works. The preprocessor has support for `#include` and `#define FOO` (but not for macros with macro parameters). The parser and typechecker is decent (no globals except functions, no `goto`/`continue`/`break`) but far from done. The ISel/CodeGen is extremely unfinished but the basic design should work. It works on the AST directly, there is no IR, and has for now no proper reg. allocation.

## 039: [Syntax Highlighter](./039-syntax-highlight/example.html)

Tags: __*JS*__, __*Deno*__

A project I started mostly to try out the Deno JS runtime, do a bit of TypeScript, and to play with regular expressions. Currently, only C is supported, and the output must be HTML.

## 040: [CHIP-8 Emulator](./040-chip8/)

Tags: __*Chip-8*__, __*Rust*__, __*Emulator*__

A [Chip-8](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM) emulator for retro games in rust. The emulator logic is separated from the UI, there is a `crossterm`-based terminal UI. Tetris and [this test ROM](https://github.com/corax89/chip8-test-rom/tree/master) work. To play, run: `cd ./040-chip8/ && cargo run -- ./roms/tetris.ch8`. Another UI in a sub-crate provides a WASM based browser interface.

## 041: [MD2HTML](./041-md2html/)

Tags: __*JS*__, __*Deno*__, __*Markdown*__

A extremely simple and shitty markdown subset parser and transpiler targeting HTML.

## Future Ideas:

- A REDIS clone (using epoll etc), maybe in Rust?
- A [QBE](https://c9x.me/compile/) to/from LLVM-IR or to/from GCC GIMPLE translation?
- A small simple compiler backend: ISel, Register Allocation, ...
- A Hare libgccjit backend using [hare::parse](https://docs.harelang.org/hare/parse) as frontend?
- A websocket-based shared canvas for [louknr.net](https://louknr.net).
- A super super simple linker? (for ELF, but with only the most basic features possible?)
- A simplified make clone?
- Something like [simdjson](https://github.com/simdjson/simdjson) but with AArch64 SVE?
- Something like a compressed bitmap ([see here](https://github.com/RoaringBitmap/RoaringBitmap))?
- A mini OS for the RISC-V virt `QEMU` platform?
