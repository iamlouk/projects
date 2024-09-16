#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <assert.h>
#include <string.h>

enum vm_opcodes {
	VM_NOP      , /*                                             */
	VM_LD       , /* reg[op0] = memory[reg[op1]]                 */
	VM_ST       , /* memory[reg[op0]] = reg[op1]                 */
	VM_LDI      , /* reg[op0] = op1                              */
	VM_LDLI     , /* reg[op0] = <16 next bits>                   */
	VM_ADD      , /* reg[op0] += reg[op1]                        */
	VM_MUL      ,
	VM_SUB      ,
	VM_DIV      ,
	VM_SHL      ,
	VM_ADDI     ,
	VM_SUBI     ,
	VM_JMP      , /* pc = <16 next bits>                         */
	VM_JREL     , /* pc += int8_t(op0|(op1 << 4))                */
	VM_JIND     , /* pc = reg[op0] + op1                         */
	VM_BEQ      , /* pc = <16 next bits> if reg[op0] == reg[op1] */
	VM_BNE      , /* pc = <16 next bits> if reg[op0] != reg[op1] */
	VM_BLT      , /* pc = <16 next bits> if reg[op0]  < reg[op1] */
	VM_BLE      , /* pc = <16 next bits> if reg[op0] <= reg[op1] */
	VM_HLT      , /* stop VM.                                    */
	VM_IO       , /*                                             */
};

static const uint8_t IO_IN = 0x0, IO_OUT = 0x1;

typedef union {
	uint16_t raw;
	struct {
		uint8_t  opcode;
		uint8_t  ops;
	} __attribute__ ((__packed__));
} vm_inst_t;

#define VM_INST(OP, OP0, OP1) { .opcode = OP, .ops = ((OP1 << 4) | OP0) }
#define VM_OP0(inst) ((uint8_t)((inst).ops & 0xf))
#define VM_OP1(inst) ((uint8_t)(((inst).ops >> 4) & 0xf))

static uint16_t regs[16] = {0};
static uint16_t memory[(2 << 16)] = {0};

static const vm_inst_t *code = NULL;
static uint16_t pc = 0;

static inline void vm_op_nop  (vm_inst_t ins) { pc += 1; };
static inline void vm_op_ld   (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] = memory[regs[VM_OP1(ins)]]; };
static inline void vm_op_st   (vm_inst_t ins) { pc += 1; memory[regs[VM_OP0(ins)]] = regs[VM_OP1(ins)]; };
static inline void vm_op_ldi  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] = VM_OP1(ins); };
static inline void vm_op_ldli (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] = *((uint16_t*)code + pc); pc += 1; };
static inline void vm_op_add  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] += regs[VM_OP1(ins)]; };
static inline void vm_op_mul  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] *= regs[VM_OP1(ins)]; };
static inline void vm_op_sub  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] -= regs[VM_OP1(ins)]; };
static inline void vm_op_div  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] /= regs[VM_OP1(ins)]; };
static inline void vm_op_shl  (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] <<= regs[VM_OP1(ins)];};
static inline void vm_op_addi (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] += VM_OP1(ins); };
static inline void vm_op_subi (vm_inst_t ins) { pc += 1; regs[VM_OP0(ins)] -= VM_OP1(ins); };
static inline void vm_op_jmp  (vm_inst_t ins) { pc = *((uint16_t*)code + pc + 1); }
static inline void vm_op_jrel (vm_inst_t ins) { assert(0); }
static inline void vm_op_jind (vm_inst_t ins) { assert(0); }
static inline void vm_op_beq  (vm_inst_t ins) {
	if (regs[VM_OP0(ins)] == regs[VM_OP1(ins)]) vm_op_jmp(ins);
	else                                        pc += 2;
}
static inline void vm_op_bne  (vm_inst_t ins) { assert(0); }
static inline void vm_op_blt  (vm_inst_t ins) {
	if (regs[VM_OP0(ins)]  < regs[VM_OP1(ins)]) vm_op_jmp(ins);
	else                                        pc += 2;
}
static inline void vm_op_ble  (vm_inst_t ins) { assert(0); }
static inline void vm_op_hlt  (vm_inst_t ins) { exit(EXIT_SUCCESS); };
static inline void vm_op_io   (vm_inst_t ins) {
	pc += 1;
	if (VM_OP1(ins) == IO_OUT)
		printf("%hd\n", regs[VM_OP0(ins)]);
	else if (VM_OP1(ins) == IO_IN)
		scanf("\n%hd\n", &regs[VM_OP0(ins)]);
	else
		assert(0);
};

static void run_vm() {
#define USE_COMPUTED_GOTO 0
#if USE_COMPUTED_GOTO
	static void* jump_table[] = {
		[VM_NOP]  = &&LABEL_vm_op_nop,
		[VM_LD]   = &&LABEL_vm_op_ld,
		[VM_ST]   = &&LABEL_vm_op_st,
		[VM_LDI]  = &&LABEL_vm_op_ldi,
		[VM_LDLI] = &&LABEL_vm_op_ldli,
		[VM_ADD]  = &&LABEL_vm_op_add,
		[VM_MUL]  = &&LABEL_vm_op_mul,
		[VM_SUB]  = &&LABEL_vm_op_sub,
		[VM_DIV]  = &&LABEL_vm_op_div,
		[VM_SHL]  = &&LABEL_vm_op_shl,
		[VM_ADDI] = &&LABEL_vm_op_addi,
		[VM_SUBI] = &&LABEL_vm_op_subi,
		[VM_JMP]  = &&LABEL_vm_op_jmp,
		[VM_JREL] = &&LABEL_vm_op_jrel,
		[VM_JIND] = &&LABEL_vm_op_jind,
		[VM_BEQ]  = &&LABEL_vm_op_beq,
		[VM_BNE]  = &&LABEL_vm_op_bne,
		[VM_BLT]  = &&LABEL_vm_op_blt,
		[VM_BLE]  = &&LABEL_vm_op_ble,
		[VM_HLT]  = &&LABEL_vm_op_hlt,
		[VM_IO]   = &&LABEL_vm_op_io,
	};

	#define VM_OP_ENTRY(opc, func) \
		LABEL_ ## func: \
			func(code[pc]); \
			goto *jump_table[code[pc].opcode];

	goto *jump_table[code[pc].opcode];
	for (;;) {
#else
	#define VM_OP_ENTRY(opc, func) \
		case opc: \
			func(ins); \
			break;
	for (;;) {
		vm_inst_t ins = code[pc];
		switch ((enum vm_opcodes)ins.opcode) {
#endif

		VM_OP_ENTRY(VM_NOP, vm_op_nop);
		VM_OP_ENTRY(VM_LD, vm_op_ld);
		VM_OP_ENTRY(VM_ST, vm_op_st);
		VM_OP_ENTRY(VM_LDI, vm_op_ldi);
		VM_OP_ENTRY(VM_LDLI, vm_op_ldli);
		VM_OP_ENTRY(VM_ADD, vm_op_add);
		VM_OP_ENTRY(VM_MUL, vm_op_mul);
		VM_OP_ENTRY(VM_SUB, vm_op_sub);
		VM_OP_ENTRY(VM_DIV, vm_op_div);
		VM_OP_ENTRY(VM_SHL, vm_op_shl);
		VM_OP_ENTRY(VM_ADDI, vm_op_addi);
		VM_OP_ENTRY(VM_SUBI, vm_op_subi);
		VM_OP_ENTRY(VM_JMP, vm_op_jmp);
		VM_OP_ENTRY(VM_JREL, vm_op_jrel);
		VM_OP_ENTRY(VM_JIND, vm_op_jind);
		VM_OP_ENTRY(VM_BEQ, vm_op_beq);
		VM_OP_ENTRY(VM_BNE, vm_op_bne);
		VM_OP_ENTRY(VM_BLT, vm_op_blt);
		VM_OP_ENTRY(VM_BLE, vm_op_ble);
		VM_OP_ENTRY(VM_HLT, vm_op_hlt);
		VM_OP_ENTRY(VM_IO, vm_op_io);
	
#if USE_COMPUTED_GOTO
	}
#else
		}
	}
#endif
}

// Example program: countdown
static const vm_inst_t example_program_count[] = {
	[  0] = VM_INST(VM_LDI, 0x0, 10),
	[  1] = VM_INST(VM_IO, 0x0, IO_OUT),
	[  2] = VM_INST(VM_NOP, 0, 0),
	[  3] = VM_INST(VM_SUBI, 0x0, 1),
	[  4] = VM_INST(VM_LDI, 0x1, 0),
	[  5] = VM_INST(VM_BEQ, 0x0, 0x1),
	[  6] = { .raw = 10 },
	[  7] = VM_INST(VM_JMP, 0, 0),
	[  8] = { .raw = 1 },
	[ 10] = VM_INST(VM_HLT, 0, 0)
};

// Example program: fibonacci numbers
static const vm_inst_t example_program_fibs[] = {
	[  0] = VM_INST(VM_LDI, 0x0, 10),   // r0 = n
	[  1] = VM_INST(VM_LDI, 0xa, 1),   // ra = 1
	[  2] = VM_INST(VM_LDI, 0xb, 1),   // rb = 1
	[  3] = VM_INST(VM_LDI, 0x1, 0),   // r1 = 0
	[  4] = VM_INST(VM_BEQ, 0x1, 0x0), // if r1 == r0 -> goto end
	[  5] = { .raw = 50 },
	[  6] = VM_INST(VM_SUBI, 0x0, 1),  // r0 -= 1
	[  7] = VM_INST(VM_LDI, 0x1, 0),   // r1 = 0
	[  8] = VM_INST(VM_ADD, 0x1, 0xa), // r1 += ra
	[  9] = VM_INST(VM_ADD, 0xa, 0xb), // ra += rb
	[ 10] = VM_INST(VM_LDI, 0xb, 0),   // rb = 0
	[ 11] = VM_INST(VM_ADD, 0xb, 0x1), // rb += r1
	[ 12] = VM_INST(VM_IO, 0xb, IO_OUT),
	[ 13] = VM_INST(VM_JMP, 0x0, 0x0),
	[ 14] = { .raw = 3 },

	[50] = VM_INST(VM_HLT, 0x0, 0x0)
};

// Example program: something that computes something for a benchmark
static const vm_inst_t example_program_benchmark[] = {
	[  0] = VM_INST(VM_NOP, 0, 0),
	[  1] = VM_INST(VM_LDLI, 0xa, 0),
	[  2] = { .raw = 8192 },
	[  3] = VM_INST(VM_LDI, 0x0, 0),
	[  4] = VM_INST(VM_BEQ, 0x0, 0xa),
	[  5] = { .raw = 100 },
	[  6] = VM_INST(VM_NOP, 0xc, 0),
	[  7] = VM_INST(VM_NOP, 0xc, 0xa),
	[  8] = VM_INST(VM_LDLI, 0xb, 0),
	[  9] = { .raw = 8192 },
	[ 10] = VM_INST(VM_BEQ, 0x0, 0xb),
	[ 11] = { .raw = 22 },
	[ 12] = VM_INST(VM_NOP, 0xc, 0xb),
	[ 13] = VM_INST(VM_NOP, 0xc, 0xa),
	[ 14] = VM_INST(VM_NOP, 0xc, 0xc),
	[ 15] = VM_INST(VM_NOP, 0xc, 8),
	[ 16] = VM_INST(VM_LDI, 0x1, 1),
	[ 17] = VM_INST(VM_SUB, 0xb, 0x1),
	[ 18] = VM_INST(VM_JMP, 0, 0),
	[ 19] = { .raw = 10 },
	[ 20] = VM_INST(VM_NOP, 0, 0),
	[ 21] = VM_INST(VM_NOP, 0, 0),
	[ 22] = VM_INST(VM_NOP, 0, 0),
	[ 23] = VM_INST(VM_SUBI, 0xa, 1),
	[ 24] = VM_INST(VM_JMP, 0, 0),
	[ 25] = { .raw = 4 },
	[100] = VM_INST(VM_HLT, 0, 0)
};

int main(int argc, const char *argv[]) {
	assert(sizeof(vm_inst_t) == sizeof(uint16_t));

	code = example_program_benchmark;
	run_vm();

}


