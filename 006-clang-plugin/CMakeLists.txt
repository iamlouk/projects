cmake_minimum_required(VERSION 3.13.4)

project(helloworldplugin)

find_package(LLVM REQUIRED CONFIG) # Set $LLVM_DIR to help with finding this package

list(APPEND CMAKE_MODULE_PATH ${LLVM_DIR})
include(AddLLVM)
include(HandleLLVMOptions)

find_package(Clang REQUIRED)

include_directories("${LLVM_INCLUDE_DIR}" "${CLANG_INCLUDE_DIRS}")
add_definitions("${LLVM_DEFINITIONS}")
link_directories("${LLVM_LIBRARY_DIR}")

add_llvm_library(helloworldplugin MODULE helloworldplugin.cpp PLUGIN_TOOL clang)

