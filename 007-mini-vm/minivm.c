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
static inline void vm_op_ldli (vm_inst_t ins) { assert(0); };
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
	if (regs[VM_OP0(ins)] == regs[VM_OP1(ins)])
		vm_op_jmp(ins);
	else
		pc += 2;
}
static inline void vm_op_bne  (vm_inst_t ins) { assert(0); }
static inline void vm_op_blt  (vm_inst_t ins) { assert(0); }
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
	static void* jump_table[] = {
		[VM_NOP] = &&lbl_nop,
		[VM_LD] = &&lbl_ld,
		[VM_ST] = &&lbl_st,
		[VM_LDI] = &&lbl_ldi,
		[VM_LDLI] = &&lbl_ldli,
		[VM_ADD] = &&lbl_add,
		[VM_MUL] = &&lbl_mul,
		[VM_SUB] = &&lbl_sub,
		[VM_DIV] = &&lbl_div,
		[VM_SHL] = &&lbl_shl,
		[VM_ADDI] = &&lbl_addi,
		[VM_SUBI] = &&lbl_subi,
		[VM_JMP] = &&lbl_jmp,
		[VM_JREL] = &&lbl_jrel,
		[VM_JIND] = &&lbl_jind,
		[VM_BEQ] = &&lbl_beq,
		[VM_BNE] = &&lbl_bne,
		[VM_BLT] = &&lbl_blt,
		[VM_BLE] = &&lbl_ble,
		[VM_HLT] = &&lbl_hlt,
		[VM_IO] = &&lbl_io,
	};

	#define COMPUTED_GOTO() goto *jump_table[code[pc].opcode]

	COMPUTED_GOTO();
	for(;;) {
	lbl_nop:
		vm_op_nop(code[pc]);
		COMPUTED_GOTO();
	lbl_ld:
		vm_op_ld(code[pc]);
		COMPUTED_GOTO();
	lbl_st:
		vm_op_st(code[pc]);
		COMPUTED_GOTO();
	lbl_ldi:
		vm_op_ldi(code[pc]);
		COMPUTED_GOTO();
	lbl_ldli:
		vm_op_ldli(code[pc]);
		COMPUTED_GOTO();
	lbl_add:
		vm_op_add(code[pc]);
		COMPUTED_GOTO();
	lbl_mul:
		vm_op_mul(code[pc]);
		COMPUTED_GOTO();
	lbl_sub:
		vm_op_sub(code[pc]);
		COMPUTED_GOTO();
	lbl_div:
		vm_op_div(code[pc]);
		COMPUTED_GOTO();
	lbl_shl:
		vm_op_shl(code[pc]);
		COMPUTED_GOTO();
	lbl_addi:
		vm_op_addi(code[pc]);
		COMPUTED_GOTO();
	lbl_subi:
		vm_op_subi(code[pc]);
		COMPUTED_GOTO();
	lbl_jmp:
		vm_op_jmp(code[pc]);
		COMPUTED_GOTO();
	lbl_jrel:
		vm_op_jrel(code[pc]);
		COMPUTED_GOTO();
	lbl_jind:
		vm_op_jind(code[pc]);
		COMPUTED_GOTO();
	lbl_beq:
		vm_op_beq(code[pc]);
		COMPUTED_GOTO();
	lbl_bne:
		vm_op_bne(code[pc]);
		COMPUTED_GOTO();
	lbl_blt:
		vm_op_blt(code[pc]);
		COMPUTED_GOTO();
	lbl_ble:
		vm_op_ble(code[pc]);
		COMPUTED_GOTO();
	lbl_hlt:
		vm_op_hlt(code[pc]);
		COMPUTED_GOTO();
	lbl_io:
		vm_op_io(code[pc]);
		COMPUTED_GOTO();
	}
}

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

// Example program:
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

int main(int argc, const char *argv[]) {
	assert(sizeof(vm_inst_t) == sizeof(uint16_t));

	code = example_program_fibs;
	run_vm();

}


