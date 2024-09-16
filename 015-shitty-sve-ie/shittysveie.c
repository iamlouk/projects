#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <signal.h>
#include <ucontext.h>
#include <assert.h>
#include <stdbool.h>
#include <string.h>
#include <errno.h>

#if !defined(__aarch64__)
#error "this is not portable at all"
#endif
static const uint64_t
			PSTATE_N_BIT = 31,
			PSTATE_Z_BIT = 30,
			PSTATE_C_BIT = 29,
			PSTATE_V_BIT = 28;

static uint64_t sve_vector_bits = 1024, sve_vector_bytes = 128; // Can be changed via SVEIE_VL.
static __thread bool sve_initialized = false; // per-thread marker if sve_regs allocated.
static __thread void *sve_regs[32]; // each entry must point to sve_vector_bytes bytes.
static __thread uint16_t *sve_predicate_regs[16]; // uint16_t because 128 / 8 (min. SVE VL).

static _Atomic uint64_t stats_sve_ops = 0;
static _Atomic uint64_t stats_sve_adds = 0;

static inline void helper_pred_set_zero(uint32_t pred) {
	for (uint64_t i = 0; i < sve_vector_bytes / 16; i++)
		sve_predicate_regs[pred][i] = 0;
}

static inline bool helper_pred_is_zero(uint32_t pred) {
	for (uint64_t i = 0; i < sve_vector_bytes / 16; i++)
		if (sve_predicate_regs[pred][i] != 0)
			return false;
	return true;
}

static inline void helper_pred_set_bit(uint32_t pred, uint32_t pos, bool val) {
	uint32_t byte = pos / 16, bit = pos % 16;
	if (val) sve_predicate_regs[pred][byte] |=  (1u << bit);
	else     sve_predicate_regs[pred][byte] &= ~(1u << bit);
}

static inline bool helper_pred_get_bit(uint32_t pred, uint32_t pos) {
	uint32_t byte = pos / 16, bit = pos % 16;
	return (sve_predicate_regs[pred][byte] & (1u << bit)) != 0;
}

#ifdef DEBUG
static void helper_print_pred(uint32_t pred) {
	char buf[1024];
	char *pos = &buf[0];
	for (uint64_t i = 0; i < sve_vector_bytes / 16; i++) {
		for (uint64_t j = 0; j < 16; j++) {
			*(pos++) = sve_predicate_regs[pred][i] & (1u << j) ? '1' : '0';
			if (j == 7 || j == 15) *(pos++) = ' ';
		}
	}
	*pos = '\0';
	fprintf(stderr, "SVE predicate register #%d: %s\n", pred, buf);
}
#endif

static inline uint32_t helper_size_enc(uint32_t raw_size) {
	return 1u << raw_size;
}

static inline int32_t helper_sign_extend(int32_t imm, uint32_t bits) {
	int32_t m = 1U << (bits - 1);
	return (imm ^ m) - m;
}

static inline void helper_set_bit(unsigned long long *x, uint64_t pos, bool val) {
	if (val) *x |=  (1ul << pos);
	else     *x &= ~(1ul << pos);
}

// The cntb/cnth/cntw/cnth instruction:
static const uint32_t SVE_CNT_BITS_MASK = 0xff30fc00;
static const uint32_t SVE_CNT_BITS = 0x420e000;
static void emulate_sve_cnt(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t xd = inst & 0x1f,
		 pattern = (inst >> 5) & 0x1f,
		 imm = ((inst >> 16) & 0xf) + 1,
		 size = helper_size_enc((inst >> 22) & 0x3);
	assert(pattern == 0x1f && imm == 1 && "other variants are unimplemented");
	mctx->regs[xd] = sve_vector_bytes / size;
}

// The ptrue instruction:
static const uint32_t SVE_PTRUE_BITS_MASK = 0xff3ffc10;
static const uint32_t SVE_PTRUE_BITS = 0x2518e000;
static void emulate_sve_ptrue(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	(void) mctx;
	uint32_t pd = inst & 0xf,
		 pattern = (inst >> 5) & 0x1f,
		 size = helper_size_enc((inst >> 22) & 0x3);
	assert(pattern == 0x1f && "other variants are unimplemented");
	helper_pred_set_zero(pd);
	for (uint64_t i = 0; i < sve_vector_bytes; i += size)
		helper_pred_set_bit(pd, i, true);
}

// The whilelo instruction:
static const uint32_t SVE_WHILELO_BITS_MASK = 0xff20ec10;
static const uint32_t SVE_WHILELO_BITS = 0x25200c00;
static void emulate_sve_whilelo(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t pd = inst & 0xf,
			 rn = (inst >> 5) & 0x1f,
			 sf = (inst >> 12) & 0x1,
			 rm = (inst >> 16) & 0x1f,
			 size = helper_size_enc((inst >> 22) & 0x3);
	assert(sf == 1 && "other variants are unimplemented");
	helper_pred_set_zero(pd);
	uint64_t n = rn == 31 ? 0 : mctx->regs[rn],
		 m = rm == 31 ? 0 : mctx->regs[rm];
	uint64_t nelms = sve_vector_bytes / size;
	bool firstactive = n < m, lastactive = false;
	for (uint64_t i = 0; i < nelms; i++) {
		lastactive = (n++) < m;
		helper_pred_set_bit(pd, i * size, lastactive);
	}
	helper_set_bit(&mctx->pstate, PSTATE_N_BIT, firstactive);
	helper_set_bit(&mctx->pstate, PSTATE_Z_BIT, !firstactive);
	helper_set_bit(&mctx->pstate, PSTATE_C_BIT, !lastactive);
	helper_set_bit(&mctx->pstate, PSTATE_V_BIT, 0);
#ifdef DEBUG
	helper_print_pred(pd);
#endif
}

// Only the 32bit variant of 'LD1W (scalar plus immediate)':
static const uint32_t SVE_LD1W_BITS_MASK = 0xfff0e000;
static const uint32_t SVE_LD1W_BITS = 0xa540a000;
static void emulate_sve_ld1w(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
		 rn = (inst >> 5) & 0x1f,
		 pg = (inst >> 10) & 0x7;
	int32_t imm = helper_sign_extend((inst >> 16) & 0xf, 4) * sve_vector_bytes;
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bytes / size;
	for (int32_t i = 0; i < nelms; i++)
		if (helper_pred_get_bit(pg, i * size))
			sve_reg[i] = base[imm + i];
}

// 'LD1W (scalar plus scalar)'
static const uint32_t SVE_LD1W_SPS_BITS_MASK = 0xffe0e000;
static const uint32_t SVE_LD1W_SPS_BITS = 0xa5404000;
static void emulate_sve_ld1w_sps(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
			 rn = (inst >> 5) & 0x1f,
			 pg = (inst >> 10) & 0x7,
			 rm = (inst >> 16) & 0x1f;
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bytes / size;
	for (int32_t i = 0; i < nelms; i++)
		if (helper_pred_get_bit(pg, i * size))
			sve_reg[i] = base[mctx->regs[rm] + i];
}

// Only the 32bit variant of 'FADD (vectors, unpredicated)':
static const uint32_t SVE_FADD_VECS_UNPRED_BITS_MASK = 0xff20fc00;
static const uint32_t SVE_FADD_VECS_UNPRED_BITS = 0x65000000;
static void emulate_sve_fadd_vecs_unpred(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	(void) mctx;
	uint32_t zd = inst & 0x1f,
		 zn = (inst >> 5) & 0x1f,
		 zm = (inst >> 16) & 0x1f,
		 size = helper_size_enc((inst >> 22) & 0x3);
	assert(size == 4 && "other variants are unimplemented");
	int32_t nelms = sve_vector_bytes / size;
	stats_sve_adds += nelms;
	float *Zd = (float*)sve_regs[zd],
		*Zn = (float*)sve_regs[zn],
		*Zm = (float*)sve_regs[zm];
	for (int32_t i = 0; i < nelms; i++)
		Zd[i] = Zn[i] + Zm[i];
}

// Only the 32bit variant of 'ST1W (scalar plus immediate)':
static const uint32_t SVE_ST1W_BITS_MASK = 0xff90e000;
static const uint32_t SVE_ST1W_BITS = 0xe500e000;
static void emulate_sve_st1w(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
		 rn = (inst >> 5) & 0x1f,
		 pg = (inst >> 10) & 0x7,
		 size_raw = (inst >> 21) & 0x3;
	int32_t imm = helper_sign_extend((inst >> 16) & 0xf, 4) * sve_vector_bits;
	assert(size_raw == 0x2 && "other variants are unimplemented");
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bytes / size;
	for (int32_t i = 0; i < nelms; i++)
		if (helper_pred_get_bit(pg, i * size))
			base[imm + i] = sve_reg[i];
}

// 'ST1W (scalar plus scalar)'
static const uint32_t SVE_ST1W_SPS_BITS_MASK = 0xff80e000;
static const uint32_t SVE_ST1W_SPS_BITS = 0xe5004000;
static void emulate_sve_st1w_sps(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
			 rn = (inst >> 5) & 0x1f,
			 pg = (inst >> 10) & 0x7,
			 rm = (inst >> 16) & 0x1f,
			 size_raw = (inst >> 21) & 0x3;
	assert(size_raw == 0x2 && "other variants are unimplemented");
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bytes / size;
	for (int32_t i = 0; i < nelms; i++)
		if (helper_pred_get_bit(pg, i * size))
			base[mctx->regs[rm] + i] = sve_reg[i];
}

// Initialize thread local register values:
static void init_sve() {
	// Init SVE registers:
	sve_initialized = true;
	for (int i = 0; i < 32; i++)
		sve_regs[i] = calloc(1, sve_vector_bytes);
	for (int i = 0; i < 16; i++)
		sve_predicate_regs[i] = calloc(2, sve_vector_bytes / 16);
}

// This function is called after every SIGILL:
static void handler(int signal, siginfo_t *siginfo, void *ucontext) {
	(void) siginfo;
	assert(signal == SIGILL);
	ucontext_t *uctx = (ucontext_t*)ucontext;
	mcontext_t *mctx = &uctx->uc_mcontext;
	uint32_t inst = *(uint32_t*)mctx->pc;

	// We could be on a new thread...
	if (!__builtin_expect(sve_initialized, true))
		init_sve();

	// If I were to ever add some more instructions, this should maybe be changed
	// to a jump-table based on the upper-most few bits.
	if ((inst & SVE_CNT_BITS_MASK) == SVE_CNT_BITS) {
		emulate_sve_cnt(inst, uctx, mctx);
	} else if ((inst & SVE_PTRUE_BITS_MASK) == SVE_PTRUE_BITS) {
		emulate_sve_ptrue(inst, uctx, mctx);
	} else if ((inst & SVE_WHILELO_BITS_MASK) == SVE_WHILELO_BITS) {
		emulate_sve_whilelo(inst, uctx, mctx);
	} else if ((inst & SVE_LD1W_BITS_MASK) == SVE_LD1W_BITS) {
		emulate_sve_ld1w(inst, uctx, mctx);
	} else if ((inst & SVE_LD1W_SPS_BITS_MASK) == SVE_LD1W_SPS_BITS) {
		emulate_sve_ld1w_sps(inst, uctx, mctx);
	} else if ((inst & SVE_FADD_VECS_UNPRED_BITS_MASK) == SVE_FADD_VECS_UNPRED_BITS) {
		emulate_sve_fadd_vecs_unpred(inst, uctx, mctx);
	} else if ((inst & SVE_ST1W_BITS_MASK) == SVE_ST1W_BITS) {
		emulate_sve_st1w(inst, uctx, mctx);
	} else if ((inst & SVE_ST1W_SPS_BITS_MASK) == SVE_ST1W_SPS_BITS) {
		emulate_sve_st1w_sps(inst, uctx, mctx);
	} else {
		fprintf(stderr, "unkown instruction, cannot emulate: %#x (pc: %#llx)\n",
				inst, mctx->pc);
		abort();
	}

	stats_sve_ops += 1;
	mctx->pc += 4;
}

/*
 * Init function of this library. GCC-magic makes it that
 * this function is called when the shared library is loaded
 * into memory (like the constructor of static a C++ object).
 */
__attribute__((constructor)) static void init() {
	const char *envvar = getenv("SVEIE_VL");
	if (envvar != NULL && strlen(envvar) != 0) {
		char *endptr;
		errno = 0;
		int x = strtol(envvar, &endptr, 10);
		if (errno != 0 || *endptr != '\0' || x < 128 || x > 2048 || (x % 128) != 0) {
			fprintf(stderr, "SVEIE_VL: value invalid ('%s')\n", envvar);
			exit(EXIT_FAILURE);
		}
		sve_vector_bits = x;
		sve_vector_bytes = sve_vector_bits / 8;
	}

	// Initialize main thread right away:
	init_sve();

	// Register the SVE handler:
	struct sigaction act = {
		.sa_flags = SA_SIGINFO,
		.sa_sigaction = &handler,
	};
	if (sigaction(SIGILL, &act, NULL) == -1) {
		perror("sigaction");
		exit(EXIT_FAILURE);
	}
}

/*
 * Do some cleanup...
 */
__attribute__((destructor)) static void fini() {
	const char *envvar = getenv("SVEIE_STATS");
	if (envvar != NULL && strcmp(envvar, "1") == 0) {
		fprintf(stderr, "SVEIE_STATS: sve_vl = %lu\n", sve_vector_bits);
		fprintf(stderr, "SVEIE_STATS: sve_ops = %lu\n", stats_sve_ops);
		fprintf(stderr, "SVEIE_STATS: sve_adds = %lu\n", stats_sve_adds);
	}

	// TODO: Cleanup on other threads?
	for (int i = 0; i < 32; i++)
		free(sve_regs[i]);
	for (int i = 0; i < 16; i++)
		free(sve_predicate_regs[i]);
}

