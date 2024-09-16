#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>

extern void sve_vec_add(uint64_t n, float * restrict a, float * restrict b);

void classic_vec_add(uint64_t n, float *a, float *b) {
	for (uint64_t i = 0; i < n; i++)
		a[i] += b[i];
}

int main() {
	srand(42);
	uint64_t n = 20000;
	float *a1 = malloc(n * sizeof(float));
	float *a2 = malloc(n * sizeof(float));
	float *b = malloc(n * sizeof(float));

	for (uint64_t i = 0; i < n; i++) {
		float x = (float)(rand() % 100 - 50),
		      y = (float)(rand() % 50 - 25);
		a1[i] = a2[i] = x;
		b[i] = y;
	}

	printf("start test...\n");
	sve_vec_add(n, a1, b);
	classic_vec_add(n, a2, b);

	for (uint64_t i = 0; i < n; i++)
		if (a1[i] != a2[i])
			return EXIT_FAILURE;

	printf("success!\n");
	free(a1);
	free(a2);
	free(b);
	return EXIT_SUCCESS;
}


