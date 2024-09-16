#include <stdlib.h>

extern void add_vecs(size_t n, float * restrict dst,
		const float * restrict a, const float * restrict b) {
	for (size_t i = 0; i < n; i++) {
		dst[i] = a[i] + b[i];
	}
}

