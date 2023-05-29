#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>
#include <dirent.h>
#include <string.h>
#include <errno.h>
#include <assert.h>

#define COL_RED     "\033[0;31m"
#define COL_GREEN   "\033[0;32m"
#define COL_YELLOW  "\033[0;33m"
#define COL_BLUE    "\033[0;34m"
#define COL_CYAN    "\033[0;36m"
#define COL_GREY    "\033[0;2m"
#define COL_RESET   "\033[0m"

#include "cson.h"

static _Noreturn void die(const char msg[])
{
  fprintf(stderr, COL_RED "error: " COL_RESET "%s (%s)\n", msg, strerror(errno));
  exit(EXIT_FAILURE);
}

static size_t read_file(const char *filename, char **contents, char **buf, ssize_t *buf_size)
{
  FILE *f = fopen(filename, "r");
  if (!f)
    die(filename);

  ssize_t size = 0;
  while (!feof(f) && !ferror(f)) {
    ssize_t free_space = *buf_size - size;
    if (free_space < 32) {
      *buf_size += 256;
      *buf = realloc(*buf, *buf_size);
      if (!*buf)
	die("realloc");

      free_space = *buf_size - size;
      assert(free_space > 0);
    }

    size += fread(*buf + size, 1, free_space - 1, f);
  }

  if (ferror(f))
    die(filename);

  (*buf)[size] = '\0';
  *contents = *buf;
  return size;
}

static const struct {
  const char *filename;
} tests[] = {
  { .filename = "./test-files/hello-world.json" }
};

int main(int argc, const char *argv[])
{
  for (int i = 1; i < argc; i++) {
    const char *arg = argv[i];
    if (arg[0] == '-') {
      fprintf(stderr, COL_RED "error: " COL_RESET "unknown option: '%s'\n", arg);
      return EXIT_FAILURE;
    }
  }

  ssize_t buf_size = 1024;
  char *buf = malloc(buf_size);

  size_t successes = 0, failures = 0;
  size_t num_tests = sizeof(tests) / sizeof(tests[0]);
  for (size_t i = 0; i < num_tests; i++) {
    char *contents;
    size_t len = read_file(tests[i].filename, &contents, &buf, &buf_size);

#if 0
    fprintf(stdout, COL_YELLOW "test #%ld: " COL_RESET "input = '%s'\n", i, contents);
#endif

    struct cson *cson = cson_parse(contents, len);
    if (cson->type == CSON_ERROR) {
      failures += 1;
      fprintf(stdout, COL_YELLOW "test #%ld: " COL_RESET "error at %d:%d: %s\n",
              i, cson->line, cson->col, cson->error);
      cson_free(cson);
      continue;
    }

#if 0
    fprintf(stdout, COL_YELLOW "test #%ld: " COL_RESET, i);
    cson_write(cson, stdout);
    fprintf(stdout, "\n");
#endif

    cson_free(cson);
    successes += 1;
  }

  free(buf);

  if (failures > 0) {
    fprintf(stderr, COL_RED "failure: " COL_RESET
            "%ld tests failed (out of %ld)\n", failures, num_tests);
    return EXIT_FAILURE;
  }

  printf(COL_GREEN "success: " COL_RESET "%ld tests executed in total\n", successes);
  return EXIT_SUCCESS;
}

