	.arch armv8-a+sve
	.text
	.align 2
	.p2align 4,,11
	.global sve_vec_add
	.type sve_vec_add, %function
sve_vec_add:
	.cfi_startproc
	cbz     x0, .L1
	mov     x3, 0
	cntw    x4
	whilelo p0.s, xzr, x0
.L3:
	ld1w    z1.s, p0/z, [x1, x3, lsl 2]
	ld1w    z0.s, p0/z, [x2, x3, lsl 2]
	fadd    z0.s, z0.s, z1.s
	st1w    z0.s, p0, [x1, x3, lsl 2]
	add     x3, x3, x4
	whilelo p0.s, x3, x0
	b.any   .L3
.L1:
	ret
	.cfi_endproc
	.size sve_vec_add, .-sve_vec_add
	.section .note.GNU-stack,"",@progbits

