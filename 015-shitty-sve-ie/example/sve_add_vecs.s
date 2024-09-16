	.arch armv8-a+sve
	.text
	.align 2
	.p2align 4,,11
	.global sve_vec_add
	.type sve_vec_add, %function
sve_vec_add:
	.cfi_startproc
	cbz x0, .Lret
	cntw x3
	lsl x4, x3, #2
	ptrue p0.s

	b .Lveccond
.Lvecbody:
	ld1w { z0.s }, p0/z, [x1]
	ld1w { z1.s }, p0/z, [x2]
	fadd z0.s, z0.s, z1.s
	st1w { z0.s }, p0, [x1]
	subs x0, x0, x3
	add x1, x1, x4
	add x2, x2, x4
.Lveccond:
	cmp x0, x3
	b.ge .Lvecbody

.Ltail:
	cbz x0, .Lret
.Ltailloop:
	ldr s1, [x1]
	ldr s0, [x2], #4
	fadd s0, s0, s1
	str s0, [x1], #4
	subs x0, x0, #1
	b.ne .Ltailloop
.Lret:
	ret
	.cfi_endproc
	.size sve_vec_add, .-sve_vec_add
	.section .note.GNU-stack,"",@progbits

