#ifndef VECTORCLASS_SVE_H
#define VECTORCLASS_SVE_H

#include <cstdint>
#include <cstddef>
#include <cassert>
#include <cmath>
#ifndef __ARM_FEATURE_SVE
#error "SVE not available!"
#endif
#include <arm_sve.h>

#if !defined(VECTORCLASS_SVE_H_SETUP)
#define VECTORCLASS_SVE_H_SETUP
/* Figure out the vector length at compile time. */
#if defined(__ARM_FEATURE_SVE_BITS) && __ARM_FEATURE_SVE_BITS != 0
#define SVE_VECTOR_BITS (__ARM_FEATURE_SVE_BITS)
#elif defined(VECTORCLASS_SVE_ASSUME_BITS)
#define SVE_VECTOR_BITS (VECTORCLASS_SVE_ASSUME_BITS)
#else
#error "SVE vector length is unknown: __ARM_FEATURE_SVE_BITS and VECTORCLASS_SVE_ASSUME_BITS unset"
#endif
__attribute__((constructor)) static void check_sve_vl() {
	assert(SVE_VECTOR_BITS == svcntb() * 8 && "SVE vector length unexpected, recompile!?");
}
/* For amd64 compatibility (ignore options for now, it's a hint anyways): */
static inline void _mm_prefetch(const char *p, int _opt) {
	(void) _opt; svprfb(svptrue_b8(), p, SV_PLDL1KEEP);
}
/* Dummy functions that Agner vectorclass has (here to suppress compiler messages): */
static void no_subnormals() {}
/* Attributed variants of sv*_t that have a fixed, compile-time known size. */
typedef svbool_t    fixed_svbool_t    __attribute__((arm_sve_vector_bits(SVE_VECTOR_BITS)));
typedef svfloat32_t fixed_svfloat32_t __attribute__((arm_sve_vector_bits(SVE_VECTOR_BITS)));
typedef svfloat64_t fixed_svfloat64_t __attribute__((arm_sve_vector_bits(SVE_VECTOR_BITS)));
typedef svint32_t   fixed_svint32_t   __attribute__((arm_sve_vector_bits(SVE_VECTOR_BITS)));
typedef svint64_t   fixed_svint64_t   __attribute__((arm_sve_vector_bits(SVE_VECTOR_BITS)));
/* Do something fucked-up: include this file into itself! */
/* 32-bit float: */
#undef VECTORCLASS_SVE_H
#define scalar_t float
#define vector_t fixed_svfloat32_t
#define SVE_SUFFIX _f32
#define SVE_BYTE_SUFFIX _b32
#define VEC Vecf32
#define MASK Mask32
#define PTRUE svptrue_b32
#include "vectorclass_sve.h"
#undef PTRUE
#undef MASK
#undef VEC
#undef SVE_BYTE_SUFFIX
#undef SVE_SUFFIX
#undef vector_t
#undef scalar_t
/* 64-bit float (double): */
#undef VECTORCLASS_SVE_H
#define scalar_t double
#define vector_t fixed_svfloat64_t
#define SVE_SUFFIX _f64
#define SVE_BYTE_SUFFIX _b32
#define VEC Vecf64
#define MASK Mask64
#define PTRUE svptrue_b64
#include "vectorclass_sve.h"
#undef PTRUE
#undef MASK
#undef VEC
#undef SVE_BYTE_SUFFIX
#undef SVE_SUFFIX
#undef vector_t
#undef scalar_t
/* 32-bit int: */
#undef VECTORCLASS_SVE_H
#define scalar_t int32_t
#define vector_t fixed_svint32_t
#define SVE_SUFFIX _s32
#define SVE_BYTE_SUFFIX _b32
#define FPVEC Vecf32
#define VEC Veci32
#define MASK Mask32
#define PTRUE svptrue_b32
#include "vectorclass_sve.h"
static inline FPVEC to_float(const VEC &vec) {
	return FPVEC(svcvt_f32_s32_x(PTRUE(), vec.vec));
}
#undef PTRUE
#undef MASK
#undef VEC
#undef FPVEC
#undef SVE_BYTE_SUFFIX
#undef SVE_SUFFIX
#undef vector_t
#undef scalar_t
/* 64-bit int (long): */
#undef VECTORCLASS_SVE_H
#define scalar_t int64_t
#define vector_t fixed_svint64_t
#define SVE_SUFFIX _s64
#define SVE_BYTE_SUFFIX _b32
#define FPVEC Vecf64
#define VEC Veci64
#define MASK Mask64
#define PTRUE svptrue_b64
#include "vectorclass_sve.h"
static inline FPVEC to_double(const VEC &vec) {
	return FPVEC(svcvt_f64_s64_x(PTRUE(), vec.vec));
}
#undef PTRUE
#undef MASK
#undef VEC
#undef FPVEC
#undef SVE_BYTE_SUFFIX
#undef SVE_SUFFIX
#undef vector_t
#undef scalar_t

#else /* if defined(VECTORCLASS_SVE_H_SETUP) */

/* Preprocessor-Magic: Two-times redirection needed to resolve suffix, as
 * it is a macro itself: */
#define SVINTR(name, ...) SVINTR_NX1(name, SVE_SUFFIX, __VA_ARGS__)
#define SVINTR_NX1(name, suffix, ...) SVINTR_NX2(name, suffix, __VA_ARGS__)
#define SVINTR_NX2(name, suffix, ...) (sv ## name ## suffix)(__VA_ARGS__)
#define SVINTR_X(name, ...) SVINTR_X_NX1(name, SVE_SUFFIX, __VA_ARGS__)
#define SVINTR_X_NX1(name, suffix, ...) SVINTR_X_NX2(name, suffix, __VA_ARGS__)
#define SVINTR_X_NX2(name, suffix, ...) (sv ## name ## suffix ## _x)(__VA_ARGS__)

#ifndef FPVEC
class MASK {
public:
	fixed_svbool_t mask;
	static constexpr size_t size = sizeof(vector_t) / sizeof(scalar_t);

	MASK(bool b): mask(b ? fixed_svbool_t(PTRUE()) : fixed_svbool_t(svpfalse())) {}
	MASK(fixed_svbool_t mask): mask(mask) {}
	MASK(): MASK(false) {}
	MASK(const MASK &other): mask(other.mask) {}

	inline MASK &operator = (const MASK other) { mask = other.mask; return *this; }

	inline MASK operator || (const MASK rhs) { return svorn_z(PTRUE(), mask, rhs.mask); }
	inline MASK operator && (const MASK rhs) { return svmov_z(mask, rhs.mask); }
	inline MASK operator ! () { return svnot_b_z(PTRUE(), mask); }
	inline MASK operator != (const MASK rhs) { return sveor_z(PTRUE(), mask, rhs.mask); }
	inline MASK operator == (const MASK rhs) { return !(*this != rhs); }
};

static inline bool horizontal_or(const MASK m) {
	return svptest_any(PTRUE(), m.mask);
}
static inline bool horizontal_and(const MASK m) {
	return !svptest_any(PTRUE(), svnot_z(PTRUE(), m.mask));
}
#endif

class VEC {
public:
	vector_t vec;
	static constexpr size_t size = sizeof(vector_t) / sizeof(scalar_t);
	using scalar = scalar_t;

	VEC(scalar_t s){ vec = SVINTR(dup, s); }
	VEC(vector_t v): vec(v) {}
	VEC(): VEC(0) {}
	VEC(const VEC &vec): vec(vec.vec) {}

	/* For compatibility with x86 stuff: */
	VEC(scalar_t x1, scalar_t x2, scalar_t x3, scalar_t x4) {
		assert(size == 4);
		scalar_t tmp[] = { x1, x2, x3, x4 };
		load(&tmp[0]);
	}
	VEC(scalar_t x1, scalar_t x2, scalar_t x3, scalar_t x4,
			scalar_t x5, scalar_t x6, scalar_t x7, scalar_t x8) {
		assert(size == 8);
		scalar_t tmp[] = { x1, x2, x3, x4, x5, x6, x7, x8 };
		load(&tmp[0]);
	}

	inline VEC &load(const scalar_t *p) {
		vec = svld1(PTRUE(), p); return *this;
	}
	inline void store(scalar_t *p) const {
		svst1(PTRUE(), p, vec);
	}

	inline VEC &load_a(const scalar_t *p) { return load(p); }
	inline void store_a(scalar_t *p) const { store(p); }

	VEC &operator = (const VEC other) {
		vec = other.vec; return *this;
	}
};

static inline VEC operator - (const VEC op) {
	return SVINTR_X(neg, PTRUE(), op.vec);
}

static inline VEC operator + (const VEC lhs, const VEC rhs) {
	return SVINTR_X(add, PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC operator + (const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(add_n, PTRUE(), lhs.vec, rhs);
}
static inline VEC operator + (const scalar_t lhs, const VEC rhs) {
	return rhs + lhs;
}

static inline VEC operator - (const VEC lhs, const VEC rhs) {
	return SVINTR_X(sub, PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC operator - (const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(sub_n, PTRUE(), lhs.vec, rhs);
}

static inline VEC operator * (const VEC lhs, const VEC rhs) {
	return SVINTR_X(mul, PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC operator * (const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(mul_n, PTRUE(), lhs.vec, rhs);
}
static inline VEC operator * (const scalar_t lhs, const VEC rhs) {
	return rhs * lhs;
}

static inline VEC operator / (const VEC lhs, const VEC rhs) {
	return SVINTR_X(div, PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC operator / (const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(div_n, PTRUE(), lhs.vec, rhs);
}

static inline VEC &operator += (VEC &lhs, const VEC rhs) { return (lhs = lhs + rhs); }
static inline VEC &operator += (VEC &lhs, const scalar_t rhs) { return (lhs = lhs + rhs); }
static inline VEC &operator -= (VEC &lhs, const VEC rhs) { return (lhs = lhs - rhs); }
static inline VEC &operator -= (VEC &lhs, const scalar_t rhs) { return (lhs = lhs - rhs); }
static inline VEC &operator *= (VEC &lhs, const VEC rhs) { return (lhs = lhs * rhs); }
static inline VEC &operator *= (VEC &lhs, const scalar_t rhs) { return (lhs = lhs * rhs); }

static inline VEC abs(const VEC op) {
	return svabs_x(PTRUE(), op.vec);
}

#if !defined(FPVEC)
static inline VEC sqrt(const VEC op) {
	return svsqrt_x(PTRUE(), op.vec);
}
#endif

static inline VEC min(const VEC lhs, const VEC rhs) {
	return svmin_x(PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC min(const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(min_n, PTRUE(), lhs.vec, rhs);
}
static inline VEC min(const scalar_t lhs, const VEC rhs) {
	return SVINTR_X(max_n, PTRUE(), rhs.vec, lhs);
}

static inline VEC max(const VEC lhs, const VEC rhs) {
	return svmax_x(PTRUE(), lhs.vec, rhs.vec);
}
static inline VEC max(const VEC lhs, const scalar_t rhs) {
	return SVINTR_X(max_n, PTRUE(), lhs.vec, rhs);
}
static inline VEC max(const scalar_t lhs, const VEC rhs) {
	return SVINTR_X(min_n, PTRUE(), rhs.vec, lhs);
}

static inline MASK operator > (const VEC lhs, const VEC rhs) {
	return svcmpgt(PTRUE(), lhs.vec, rhs.vec);
}
static inline MASK operator > (const VEC lhs, const scalar_t rhs) {
	return SVINTR(cmpgt_n, PTRUE(), lhs.vec, rhs);
}

static inline MASK operator < (const VEC lhs, const VEC rhs) {
	return svcmplt(PTRUE(), lhs.vec, rhs.vec);
}
static inline MASK operator < (const VEC lhs, const scalar_t rhs) {
	return SVINTR(cmplt_n, PTRUE(), lhs.vec, rhs);
}

static inline MASK operator >= (const VEC lhs, const VEC rhs) {
	return svcmpge(PTRUE(), lhs.vec, rhs.vec);
}
static inline MASK operator >= (const VEC lhs, const scalar_t rhs) {
	return SVINTR(cmpge_n, PTRUE(), lhs.vec, rhs);
}

static inline MASK operator <= (const VEC lhs, const VEC rhs) {
	return svcmple(PTRUE(), lhs.vec, rhs.vec);
}
static inline MASK operator <= (const VEC lhs, const scalar_t rhs) {
	return SVINTR(cmple_n, PTRUE(), lhs.vec, rhs);
}

/* Lets hope for some instcombine on this... (if not, thats a TODO for the compiler team) */
static inline MASK operator > (const scalar_t lhs, const VEC rhs) { return !(rhs <= lhs); }
static inline MASK operator < (const scalar_t lhs, const VEC rhs) { return !(rhs >= lhs); }
static inline MASK operator >= (const scalar_t lhs, const VEC rhs) { return !(rhs < lhs); }
static inline MASK operator <= (const scalar_t lhs, const VEC rhs) { return !(rhs > lhs); }

static inline VEC select(MASK mask, const VEC a, const VEC b) {
	return svsel(mask.mask, a.vec, b.vec);
}
static inline VEC select(MASK mask, const VEC a, const scalar_t b) {
	return svsel(mask.mask, a.vec, VEC(b).vec);
}
static inline VEC select(MASK mask, const scalar_t a, const VEC b) {
	return svsel(mask.mask, VEC(a).vec, b.vec);
}
static inline VEC select(MASK mask, const scalar_t a, const scalar_t b) {
	return svsel(mask.mask, VEC(a).vec, VEC(b).vec);
}

#if defined(FPVEC)
static inline VEC truncate_to_int(const FPVEC vec) {
	return SVINTR_X(cvt, PTRUE(), vec.vec);
}
#endif

#undef SVINTR
#undef SVINTR_NX1
#undef SVINTR_NX2
#undef SVINTR_X
#undef SVINTR_X_NX1
#undef SVINTR_X_NX2

#endif /* !defined(VECTORCLASS_SVE_H_SETUP) */
#endif /* VECTORCLASS_SVE_H */
