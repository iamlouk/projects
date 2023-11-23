section .text
	global interceptor_template
	interceptor_template:
		push rax
		push rdi
		push rsi
		push rdx
		push rcx
		push r8
		push r9
		mov rdi, 0x1122334455667788
		mov rax, 0x1122334455667788
		call rax
		pop r9
		pop r8
		pop rcx
		pop rdx
		pop rsi
		pop rdi
		pop rax
