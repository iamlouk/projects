#include <stdlib.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
	for (int i = 0; i < argc; i++) {
		fprintf(stdout, "argv[%d] = '%s'\n", i, argv[i]);
	}
	return EXIT_SUCCESS;
}

