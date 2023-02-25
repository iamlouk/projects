#include <cstdio>
#include <cstdlib>
#include <cstdint>

#define COL_RED     "\033[0;31m"
#define COL_GREEN   "\033[0;32m"
#define COL_YELLOW  "\033[0;33m"
#define COL_BLUE    "\033[0;34m"
#define COL_CYAN    "\033[0;36m"
#define COL_GREY    "\033[0;2m"
#define COL_RESET   "\033[0m"

extern "C" void
__gcclc_loop_preheader (uint64_t loopid)
{
  fprintf(stderr, COL_YELLOW "lcgcc: " COL_RESET " loop#%lx pre-header executed.\n", loopid);
}

extern "C" uint64_t
__gcclc_loop_header (uint64_t loopid)
{
  fprintf(stderr, COL_GREEN "lcgcc: " COL_RESET " loop#%lx header executed.\n", loopid);
  return 0x1;
}

