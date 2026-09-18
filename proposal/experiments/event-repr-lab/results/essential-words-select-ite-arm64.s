
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001016988b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select>:
1016988b4:     	sub	sp, sp, #0x60
1016988b8:     	stp	x24, x23, [sp, #0x20]
1016988bc:     	stp	x22, x21, [sp, #0x30]
1016988c0:     	stp	x20, x19, [sp, #0x40]
1016988c4:     	stp	x29, x30, [sp, #0x50]
1016988c8:     	add	x29, sp, #0x50
1016988cc:     	str	x2, [sp, #0x8]
1016988d0:     	str	x4, [sp, #0x18]
1016988d4:     	cmp	x2, x4
1016988d8:     	b.ne	0x101698980 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0xcc>
1016988dc:     	mov	x19, x2
1016988e0:     	stp	x2, x6, [sp, #0x10]
1016988e4:     	cmp	x2, x6
1016988e8:     	b.ne	0x10169899c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0xe8>
1016988ec:     	mov	x20, x0
1016988f0:     	cbz	x19, 0x101698920 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x6c>
1016988f4:     	mov	x21, x5
1016988f8:     	mov	x22, x3
1016988fc:     	mov	x23, x1
101698900:     	lsl	x24, x19, #3
101698904:     	mov	x0, x24
101698908:     	bl	0x101c29cc4 <dyld_stub_binder+0x101c29cc4>
10169890c:     	cbz	x0, 0x1016989b8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x104>
101698910:     	cmp	x19, #0x8
101698914:     	b.hs	0x10169892c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x78>
101698918:     	mov	x8, #0x0                ; =0
10169891c:     	b	0x1016989c4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x110>
101698920:     	mov	x8, #0x0                ; =0
101698924:     	mov	w0, #0x8                ; =8
101698928:     	b	0x1016989f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x13c>
10169892c:     	and	x8, x19, #0xffffffffffffff8
101698930:     	add	x9, x23, #0x20
101698934:     	add	x10, x0, #0x20
101698938:     	add	x11, x22, #0x20
10169893c:     	add	x12, x21, #0x20
101698940:     	and	x13, x19, #0xffffffffffffff8
101698944:     	ldp	q0, q1, [x9, #-0x20]
101698948:     	ldp	q2, q3, [x9], #0x40
10169894c:     	ldp	q4, q5, [x11, #-0x20]
101698950:     	ldp	q6, q7, [x11], #0x40
101698954:     	ldp	q16, q17, [x12, #-0x20]
101698958:     	ldp	q18, q19, [x12], #0x40
10169895c:     	bsl.16b	v0, v4, v16
101698960:     	bsl.16b	v1, v5, v17
101698964:     	bsl.16b	v2, v6, v18
101698968:     	bsl.16b	v3, v7, v19
10169896c:     	stp	q0, q1, [x10, #-0x20]
101698970:     	stp	q2, q3, [x10], #0x40
101698974:     	subs	x13, x13, #0x8
101698978:     	b.ne	0x101698944 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x90>
10169897c:     	b	0x1016989e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x130>
101698980:     	adrp	x5, 0x101efb000 <dyld_stub_binder+0x101efb000>
101698984:     	add	x5, x5, #0xaf0
101698988:     	add	x1, sp, #0x8
10169898c:     	add	x2, sp, #0x18
101698990:     	mov	w0, #0x0                ; =0
101698994:     	mov	x3, #0x0                ; =0
101698998:     	bl	0x101c216b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
10169899c:     	adrp	x5, 0x101efb000 <dyld_stub_binder+0x101efb000>
1016989a0:     	add	x5, x5, #0xb08
1016989a4:     	add	x1, sp, #0x10
1016989a8:     	add	x2, sp, #0x18
1016989ac:     	mov	w0, #0x0                ; =0
1016989b0:     	mov	x3, #0x0                ; =0
1016989b4:     	bl	0x101c216b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1016989b8:     	mov	w0, #0x8                ; =8
1016989bc:     	mov	x1, x24
1016989c0:     	bl	0x101c20fec <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1016989c4:     	ldr	x9, [x23, x8, lsl #3]
1016989c8:     	ldr	x10, [x22, x8, lsl #3]
1016989cc:     	ldr	x11, [x21, x8, lsl #3]
1016989d0:     	and	x10, x10, x9
1016989d4:     	bic	x9, x11, x9
1016989d8:     	orr	x9, x9, x10
1016989dc:     	str	x9, [x0, x8, lsl #3]
1016989e0:     	add	x8, x8, #0x1
1016989e4:     	cmp	x19, x8
1016989e8:     	b.ne	0x1016989c4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels6select+0x110>
1016989ec:     	mov	x8, x19
1016989f0:     	stp	x8, x0, [x20]
1016989f4:     	str	x19, [x20, #0x10]
1016989f8:     	ldp	x29, x30, [sp, #0x50]
1016989fc:     	ldp	x20, x19, [sp, #0x40]
101698a00:     	ldp	x22, x21, [sp, #0x30]
101698a04:     	ldp	x24, x23, [sp, #0x20]
101698a08:     	add	sp, sp, #0x60
101698a0c:     	ret
