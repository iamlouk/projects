## VBV: The Very Bad Vectorizer

LLVM build:

```sh

git clone https://github.com/llvm/llvm-project.git
mkdir ./build
cd ./build
cmake -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ \
	-DLLVM_ENABLE_PROJECTS="clang;lld;clang-tools-extra;compiler-rt" \
	-DLLVM_INSTALL_UTILS=On -DLLVM_TARGETS_TO_BUILD="AArch64;X86" \
	-DCMAKE_INSTALL_PREFIX="/opt/llvm" -DLLVM_DEFAULT_TARGET_TRIPLE="aarch64-linux-gnu" \
	-DCMAKE_BUILD_TYPE=RelWithDebInfo -DLLVM_OPTIMIZED_TABLEGEN=On -DLLVM_ENABLE_ASSERTIONS=On \
	-DLLVM_PARALLEL_LINK_JOBS=1 -DLLVM_BUILD_LLVM_DYLIB=On -DLLVM_LINK_LLVM_DYLIB=On \
	-DLLVM_DYLIB_COMPONENTS=all -DLLVM_ENABLE_LTO=Off -DLLVM_USE_LINKER=/usr/bin/ld.lld \
	../llvm

```

Notes on the build:
- The default way of statically linking all LLVM tools like clang results in disk usage of > 100GB for a debug build.
- Using GNU binutils and GCC and the GLIBC for cross compilation is still much easier than using the equivalent LLVM tools.
- To many parallel linking tasks can consume a lot of main memory.

Also required (for libc and linker etc.): `aarch64-linux-gnu-gcc` etc.

The `./VBV` directory has to be placed in `./llvm-project/llvm/lib/Transforms/` and `./llvm-project/llvm/lib/Transforms/CMakeLists.txt`
has to be patched like this:

```diff
diff --git a/llvm/lib/Transforms/CMakeLists.txt b/llvm/lib/Transforms/CMakeLists.txt
index dda5f6de11e3..489dd160369e 100644
--- a/llvm/lib/Transforms/CMakeLists.txt
+++ b/llvm/lib/Transforms/CMakeLists.txt
@@ -9,3 +9,4 @@ add_subdirectory(Hello)
 add_subdirectory(ObjCARC)
 add_subdirectory(Coroutines)
 add_subdirectory(CFGuard)
+add_subdirectory(VBV)
```

In order to test the code, fix the path to your LLVM build in the `Makefile` of the `test` directory and then run `make`.
The test program `axb_main_opt` checks if the vectorization was correct (run it using `qemu-aarch64` if you are not
running on a AArch64 system or one that does not support SVE).

