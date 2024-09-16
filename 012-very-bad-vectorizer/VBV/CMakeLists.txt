add_llvm_library(VBVectorizer MODULE BUILDTREE_ONLY
  Vectorize.cpp

  DEPENDS
  intrinsics_gen
  PLUGIN_TOOL
  opt
  )

