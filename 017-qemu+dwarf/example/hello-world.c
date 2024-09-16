#include <stdlib.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
	int seed = argc > 1 ? atoi(argv[1]) : 123;
	int n    = argc > 2 ? atoi(argv[2]) : 10;
	int m    = argc > 3 ? atoi(argv[3]) : 42;

	srand(seed);
	float *A = malloc(n * m * sizeof(float));
	float *b = malloc(m * sizeof(float));
	float *c = malloc(n * sizeof(float));

	// Initialize...
	for (int i = 0; i < n; i++)
		for (int j = 0; j < m; j++)
			A[i * m + j] = (float)(rand() % 10 - 5) * 0.1;
	for (int j = 0; j < m; j++)
		b[j] = (float)(rand() % 10 - 5) * 0.1;
	for (int i = 0; i < n; i++)
		c[i] = 0.;

	// Calc...
	for (int i = 0; i < n; i++) {
		float x = 0.;
		for (int j = 0; j < m; j++)
			x += A[i * m + j] * b[j];
		c[i] += x;
	}

	printf("result: [");
	for (int i = 0; i < n; i++)
		printf(i == 0 ? "%f" : ", %f", c[i]);
	printf("]\n");


	return EXIT_SUCCESS;
}

