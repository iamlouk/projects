#define _XOPEN_SOURCE
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <arm_sve.h>

/*
__attribute__((unused))
static bool parse_rfc3339(const char *raw, struct tm *tm) {

    return true;
}
*/

#ifdef TEST
static const struct {
    const char *example;
    uint64_t unixepoch;
} examples[] = {
    {
        .example = "2006-01-02T15:04:05",
        .unixepoch = 1136239445,
    }
};

#define COL_RED     "\033[0;31m"
#define COL_GREEN   "\033[0;32m"
#define COL_YELLOW  "\033[0;33m"
#define COL_BLUE    "\033[0;34m"
#define COL_CYAN    "\033[0;36m"
#define COL_GREY    "\033[0;2m"
#define COL_RESET   "\033[0m"
int main() {
    size_t num_tests = sizeof(examples) / sizeof(examples[0]);
    for (size_t i = 0; i < num_tests; i += 1) {
        fprintf(stderr, "test#%0ld: '%s' -> ", i + 1, examples[i].example);
        const char *RFC3339 = "%Y-%m-%dT%H:%M:%S";
        struct tm tm;
        memset(&tm, 0, sizeof(tm));
        strptime(examples[i].example, RFC3339, &tm);
        uint64_t unixepoch = mktime(&tm);
        if (unixepoch != examples[i].unixepoch) {
            fprintf(stderr, COL_RED "failure!" COL_RESET " (expexted=%ld, got=%ld)\n",
                examples[i].unixepoch, unixepoch);
            return EXIT_FAILURE;
        }

        fprintf(stderr, COL_GREEN "success!" COL_RESET "\n");
    }
    fprintf(stderr, COL_GREEN "success!" COL_RESET "\n");
    return EXIT_SUCCESS;
}
#endif
