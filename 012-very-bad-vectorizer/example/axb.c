#include <stdlib.h>
#include <arm_sve.h>

extern void add_vecs_naive(size_t n, float * restrict dst,
		const float * restrict a, const float * restrict b) {
	for (size_t i = 0; i < n; i++) {
		dst[i] = a[i] + b[i];
	}
}

extern void add_vecs_acle(size_t n, float * restrict dst,
		const float * restrict a, const float * restrict b) {
	size_t vl = svcntw();
	for (size_t i = 0; i < n; i += vl) {
		svbool_t mask = svwhilelt_b32(i, n);
		svfloat32_t a_vec = svld1_f32(mask, &a[i]);
		svfloat32_t b_vec = svld1_f32(mask, &b[i]);
		svfloat32_t res = svadd_f32_m(mask, a_vec, b_vec);
        svst1_f32(mask, &dst[i], res);
	}
}

