#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <assert.h>

static int get_prio(char item) {
	assert(('a' <= item && item <= 'z') || ('A' <= item && item <= 'Z'));
	if ('a' <= item && item <= 'z') return (item - 'a' + 1);
	else                            return (item - 'A' + 27);
}


static int counts[26*2] = {};

int main() {
	char *buf = NULL;
	size_t buflen = 0;

	int64_t sum = 0;
	while (!feof(stdin) && !ferror(stdin)) {
		ssize_t len = getline(&buf, &buflen, stdin);
		if (len <= 0)
			break;

		len -= 1;
		assert(buf[len] == '\n');
		assert(len % 2 == 0);
		if (len == 0)
			break;

		buf[len] = '\0';

		// first halve:
		for (int i = 0; i < len / 2; i++) {
			int prio = get_prio(buf[i]);
			assert(0 <= prio - 1 && prio - 1 < 2*26);
			counts[prio-1] = 1;
		}

		// second halve:
		for (int i = len / 2; i < len; i++) {
			int prio = get_prio(buf[i]);
			assert(0 <= prio - 1 && prio - 1 < 2*26);
			if (counts[prio-1] == 1) {
				counts[prio-1] += 1;
				sum += prio;
			}
		}
		// cleanup:
		for (int i = 0; i < 2*26; i++)
			counts[i] = 0;
	}

	if (ferror(stdin))
		perror("getline");

	printf("sum: %ld\n", sum);
	free(buf);
	return EXIT_SUCCESS;
}

