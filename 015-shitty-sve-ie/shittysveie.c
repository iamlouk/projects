#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <signal.h>
#include <ucontext.h>
#include <assert.h>
#include <stdbool.h>

static uint64_t sve_vector_bits = 2048;
static void *sve_regs[32];
static uint8_t *sve_predicate_regs[16]; // one byte per actual bit because lazy!

static inline void helper_pred_set_zero(uint32_t pred) {
	for (uint64_t i = 0; i < sve_vector_bits / 8; i++)
		sve_predicate_regs[pred][i] = 0;
}

static inline void helper_pred_set_bit(uint32_t pred, uint32_t bitindex, bool val) {
	sve_predicate_regs[pred][bitindex] = val == true ? 1 : 0;
}

static inline bool helper_pred_get_bit(uint32_t pred, uint32_t bitindex) {
	return sve_predicate_regs[pred][bitindex] == 1;
}

static inline uint32_t helper_size_enc(uint32_t raw_size) {
	switch (raw_size) {
	case 0x0: return 1; // byte
	case 0x1: return 2; // half-word
	case 0x2: return 4; // word
	case 0x3: return 8; // double-word
	default: __builtin_unreachable(); return 0;
	}
}

static inline int32_t helper_sign_extend(int32_t imm, uint32_t bits) {
	int32_t m = 1U << (bits - 1);
	return (imm ^ m) - m;
}

static const uint32_t SVE_CNT_BITS_MASK = 0xff30fc00;
static const uint32_t SVE_CNT_BITS = 0x420e000;
static void emulate_sve_cnt(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t xd = inst & 0x1f,
		 pattern = (inst >> 5) & 0x1f,
		 imm = ((inst >> 16) & 0xf) + 1,
		 size = helper_size_enc((inst >> 22) & 0x3);
	assert(pattern == 0x1f && imm == 1 && "other variants are unimplemented");
	mctx->regs[xd] = sve_vector_bits / 8 / size;
}

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
	for (uint64_t i = 0; i < sve_vector_bits / 8; i += size)
		helper_pred_set_bit(pd, i, true);
}

// Only the 32bit variant of 'LD1W (scalar plus immediate)'
static const uint32_t SVE_LD1W_BITS_MASK = 0xfff0e000;
static const uint32_t SVE_LD1W_BITS = 0xa540a000;
static void emulate_sve_ld1w(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
		 rn = (inst >> 5) & 0x1f,
		 pg = (inst >> 10) & 0x7;
	int32_t imm = helper_sign_extend((inst >> 16) & 0xf, 4) * (sve_vector_bits / 8);
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bits / 8 / size;
	for (int32_t i = 0; i < nelms; i++) {
		if (!helper_pred_get_bit(pg, i * size)) continue;
		sve_reg[i] = base[imm + i];
	}
}

// Only the 32bit variant of FADD (vectors, unpredicated)
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
	int32_t nelms = sve_vector_bits / 8 / size;
	float *Zd = (float*)sve_regs[zd],
		*Zn = (float*)sve_regs[zn],
		*Zm = (float*)sve_regs[zm];
	for (int32_t i = 0; i < nelms; i++)
		Zd[i] = Zn[i] + Zm[i];
}

// Only the 32bit variant of 'ST1W (scalar plus immediate)'
static const uint32_t SVE_ST1W_BITS_MASK = 0xff90e000;
static const uint32_t SVE_ST1W_BITS = 0xe500e000;
static void emulate_sve_st1w(uint32_t inst, ucontext_t *uctx, mcontext_t *mctx) {
	(void) uctx;
	uint32_t zt = inst & 0x1f,
		 rn = (inst >> 5) & 0x1f,
		 pg = (inst >> 10) & 0x7,
		 size_raw = (inst >> 21) & 0x3;
	int32_t imm = helper_sign_extend((inst >> 16) & 0xf, 4) * (sve_vector_bits / 8);
	assert(size_raw == 0x2);
	uint32_t *base = (uint32_t*)(rn == 31 ? mctx->sp : mctx->regs[rn]);
	uint32_t *sve_reg = sve_regs[zt];
	int32_t size = 4, nelms = sve_vector_bits / 8 / size;
	for (int32_t i = 0; i < nelms; i++) {
		if (!helper_pred_get_bit(pg, i * size)) continue;
		base[imm + i] = sve_reg[i];
	}
}

static void handler(int signal, siginfo_t *siginfo, void *ucontext) {
	(void) siginfo;
	assert(signal == SIGILL);
	ucontext_t *uctx = (ucontext_t*)ucontext;
	mcontext_t *mctx = &uctx->uc_mcontext;
	uint32_t inst = *(uint32_t*)mctx->pc;
	// fprintf(stderr, "SIGILL (pc: %#llx, fault_address: %#llx, inst: %#x)!\n",
	//		mctx->pc, mctx->fault_address, inst);

	if ((inst & SVE_CNT_BITS_MASK) == SVE_CNT_BITS) {
		emulate_sve_cnt(inst, uctx, mctx);
	} else if ((inst & SVE_PTRUE_BITS_MASK) == SVE_PTRUE_BITS) {
		emulate_sve_ptrue(inst, uctx, mctx);
	} else if ((inst & SVE_LD1W_BITS_MASK) == SVE_LD1W_BITS) {
		emulate_sve_ld1w(inst, uctx, mctx);
	} else if ((inst & SVE_FADD_VECS_UNPRED_BITS_MASK) == SVE_FADD_VECS_UNPRED_BITS) {
		emulate_sve_fadd_vecs_unpred(inst, uctx, mctx);
	} else if ((inst & SVE_ST1W_BITS_MASK) == SVE_ST1W_BITS) {
		emulate_sve_st1w(inst, uctx, mctx);
	} else {
		fprintf(stderr, "unkown instruction, cannot emulate.\n");
		abort();
	}

	mctx->pc += 4;
}

/*
 * Init function of this library.
 * It mostly serves to register a signal handler
 * for SIGILL (illegal instruction).
 */
__attribute__((constructor)) static void init() {
	// Init SVE registers:
	for (int i = 0; i < 32; i++)
		sve_regs[i] = calloc(1, sve_vector_bits / 8);
	for (int i = 0; i < 16; i++)
		sve_predicate_regs[i] = calloc(1, sve_vector_bits / 8);


	struct sigaction act = {
		.sa_flags = SA_SIGINFO,
		.sa_sigaction = &handler,
	};

	if (sigaction(SIGILL, &act, NULL) == -1) {
		perror("sigaction");
		exit(EXIT_FAILURE);
	}
}


