#include <stdlib.h>
#include <stdio.h>
#include <time.h>

extern void add_vecs_naive(size_t n, float * restrict dst,
		const float * restrict a, const float * restrict b);

extern void add_vecs_acle(size_t n, float * restrict dst,
		const float * restrict a, const float * restrict b);

int main(int argc, const char *argv[]) {
	srand((int)time(NULL));
	size_t N = argc == 1 ? 1000 : atoi(argv[1]);
	float *res1 = (float*)malloc(N * sizeof(float)),
		  *res2 = (float*)malloc(N * sizeof(float)),
		  *a = (float*)malloc(N * sizeof(float)),
		  *b = (float*)malloc(N * sizeof(float));

	for (size_t i = 0; i < N; i++) {
		a[i] = (float)(rand() % 100 - 50);
		b[i] = (float)(rand() % 100 - 50);
	}

	add_vecs_acle(N, res1, a, b);
	add_vecs_naive(N, res2, a, b);

	for (size_t i = 0; i < N; i++)
		if (res1[i] != res2[i])
			return EXIT_FAILURE;

	fprintf(stderr, "success!\n");
	return EXIT_SUCCESS;
}

