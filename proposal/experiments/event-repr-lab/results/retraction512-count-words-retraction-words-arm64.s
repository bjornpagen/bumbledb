
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010e9860 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>:
1010e9860:     	sub	sp, sp, #0x1e0
1010e9864:     	stp	x28, x27, [sp, #0x180]
1010e9868:     	stp	x26, x25, [sp, #0x190]
1010e986c:     	stp	x24, x23, [sp, #0x1a0]
1010e9870:     	stp	x22, x21, [sp, #0x1b0]
1010e9874:     	stp	x20, x19, [sp, #0x1c0]
1010e9878:     	stp	x29, x30, [sp, #0x1d0]
1010e987c:     	add	x29, sp, #0x1d0
1010e9880:     	ldr	w8, [x3, #0x10]
1010e9884:     	cbz	w8, 0x1010e99b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
1010e9888:     	mov	x19, x3
1010e988c:     	ldr	w9, [x3, #0x28]
1010e9890:     	cbz	w9, 0x1010e99b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
1010e9894:     	mov	x20, x4
1010e9898:     	ldr	x10, [x4, #0x18]
1010e989c:     	cbz	x10, 0x1010e99c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
1010e98a0:     	mov	x10, #0x0               ; =0
1010e98a4:     	mov	x15, #0xa9c5            ; =43461
1010e98a8:     	movk	x15, #0x2e62, lsl #16
1010e98ac:     	movk	x15, #0x7aea, lsl #32
1010e98b0:     	movk	x15, #0xf135, lsl #48
1010e98b4:     	ldp	x11, x12, [x19]
1010e98b8:     	madd	x13, x8, x15, x11
1010e98bc:     	mov	x14, #0x6332            ; =25394
1010e98c0:     	movk	x14, #0x6ed3, lsl #16
1010e98c4:     	movk	x14, #0x765a, lsl #32
1010e98c8:     	movk	x14, #0x284f, lsl #48
1010e98cc:     	mul	x14, x14, x15
1010e98d0:     	madd	x13, x13, x15, x14
1010e98d4:     	add	x13, x13, x12
1010e98d8:     	madd	x16, x13, x15, x9
1010e98dc:     	ldp	x13, x14, [x19, #0x18]
1010e98e0:     	madd	x16, x16, x15, x13
1010e98e4:     	madd	x16, x16, x15, x14
1010e98e8:     	mul	x15, x16, x15
1010e98ec:     	ror	x3, x15, #0x2c
1010e98f0:     	lsr	x17, x3, #57
1010e98f4:     	ldp	x16, x15, [x20]
1010e98f8:     	dup.8b	v0, w17
1010e98fc:     	movi.2d	v1, #0xffffffffffffffff
1010e9900:     	mov	w17, #0x38              ; =56
1010e9904:     	and	x3, x3, x15
1010e9908:     	ldr	d2, [x16, x3]
1010e990c:     	cmeq.8b	v3, v2, v0
1010e9910:     	fmov	x4, d3
1010e9914:     	ands	x4, x4, #0x8080808080808080
1010e9918:     	b.eq	0x1010e9988 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
1010e991c:     	rbit	x5, x4
1010e9920:     	clz	x5, x5
1010e9924:     	add	x5, x3, x5, lsr #3
1010e9928:     	and	x5, x5, x15
1010e992c:     	mneg	x5, x5, x17
1010e9930:     	add	x5, x16, x5
1010e9934:     	ldur	x6, [x5, #-0x38]
1010e9938:     	cmp	x11, x6
1010e993c:     	b.ne	0x1010e997c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
1010e9940:     	ldur	x6, [x5, #-0x30]
1010e9944:     	cmp	x12, x6
1010e9948:     	b.ne	0x1010e997c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
1010e994c:     	ldur	w6, [x5, #-0x28]
1010e9950:     	cmp	w8, w6
1010e9954:     	b.ne	0x1010e997c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
1010e9958:     	ldur	x6, [x5, #-0x20]
1010e995c:     	cmp	x13, x6
1010e9960:     	b.ne	0x1010e997c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
1010e9964:     	ldur	x6, [x5, #-0x18]
1010e9968:     	cmp	x14, x6
1010e996c:     	b.ne	0x1010e997c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
1010e9970:     	ldur	w6, [x5, #-0x10]
1010e9974:     	cmp	w9, w6
1010e9978:     	b.eq	0x1010e9ad0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x270>
1010e997c:     	sub	x5, x4, #0x2
1010e9980:     	ands	x4, x5, x4
1010e9984:     	b.ne	0x1010e991c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
1010e9988:     	cmeq.8b	v2, v2, v1
1010e998c:     	fmov	x4, d2
1010e9990:     	cbnz	x4, 0x1010e99c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
1010e9994:     	add	x10, x10, #0x8
1010e9998:     	add	x3, x3, x10
1010e999c:     	and	x3, x3, x15
1010e99a0:     	ldr	d2, [x16, x3]
1010e99a4:     	cmeq.8b	v3, v2, v0
1010e99a8:     	fmov	x4, d3
1010e99ac:     	ands	x4, x4, #0x8080808080808080
1010e99b0:     	b.ne	0x1010e991c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
1010e99b4:     	b	0x1010e9988 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
1010e99b8:     	mov	x23, #0x0               ; =0
1010e99bc:     	b	0x1010ea04c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
1010e99c0:     	mov	x22, x1
1010e99c4:     	mov	x24, x2
1010e99c8:     	mov	x23, x0
1010e99cc:     	mov	x1, x19
1010e99d0:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e99d4:     	mov	x21, x0
1010e99d8:     	fmov	d0, x21
1010e99dc:     	cnt.8b	v0, v0
1010e99e0:     	addv.8b	b0, v0
1010e99e4:     	fmov	x26, d0
1010e99e8:     	cmp	x26, #0xa
1010e99ec:     	b.hs	0x1010e9ad8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x278>
1010e99f0:     	strb	wzr, [sp, #0x120]
1010e99f4:     	movi.2d	v0, #0000000000000000
1010e99f8:     	stp	q0, q0, [sp, #0x100]
1010e99fc:     	stp	q0, q0, [sp, #0xe0]
1010e9a00:     	str	q0, [sp, #0xd0]
1010e9a04:     	sub	x0, x29, #0xa0
1010e9a08:     	add	x4, sp, #0xd0
1010e9a0c:     	mov	x1, x23
1010e9a10:     	mov	x2, x19
1010e9a14:     	mov	x3, x21
1010e9a18:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
1010e9a1c:     	ldp	x25, x22, [x29, #-0xa0]
1010e9a20:     	ldur	q0, [x29, #-0x90]
1010e9a24:     	str	q0, [sp, #0xb0]
1010e9a28:     	str	q0, [sp, #0x90]
1010e9a2c:     	str	q0, [sp]
1010e9a30:     	str	q0, [sp, #0x70]
1010e9a34:     	sub	x0, x29, #0xa0
1010e9a38:     	add	x2, x19, #0x18
1010e9a3c:     	add	x4, sp, #0xd0
1010e9a40:     	mov	x1, x23
1010e9a44:     	mov	x3, x21
1010e9a48:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
1010e9a4c:     	ldp	x24, x21, [x29, #-0xa0]
1010e9a50:     	ldur	q0, [x29, #-0x90]
1010e9a54:     	stp	x25, x22, [x29, #-0xa0]
1010e9a58:     	ldr	q1, [sp, #0x70]
1010e9a5c:     	stur	q1, [x29, #-0x90]
1010e9a60:     	stp	x24, x21, [x29, #-0x80]
1010e9a64:     	stur	q0, [x29, #-0x70]
1010e9a68:     	mov	w8, #0x1                ; =1
1010e9a6c:     	lsl	x10, x8, x26
1010e9a70:     	cmp	x26, #0x6
1010e9a74:     	cset	w8, lo
1010e9a78:     	mov	x9, #-0x1               ; =-1
1010e9a7c:     	lsl	x11, x9, x10
1010e9a80:     	csinv	x9, x9, x11, hs
1010e9a84:     	lsr	x10, x10, #6
1010e9a88:     	cinc	x13, x10, lo
1010e9a8c:     	cbz	x13, 0x1010e9d98 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x538>
1010e9a90:     	mov	x12, #0x0               ; =0
1010e9a94:     	ldp	x0, x11, [x29, #-0x90]
1010e9a98:     	sub	x14, x12, w22, uxtb
1010e9a9c:     	cmn	x24, #0x2
1010e9aa0:     	b.ne	0x1010e9d0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4ac>
1010e9aa4:     	cmn	x25, #0x2
1010e9aa8:     	b.ne	0x1010e9d40 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4e0>
1010e9aac:     	tst	w21, #0x1
1010e9ab0:     	csel	x8, x14, xzr, ne
1010e9ab4:     	and	x8, x8, x9
1010e9ab8:     	fmov	d0, x8
1010e9abc:     	cnt.8b	v0, v0
1010e9ac0:     	addv.8b	b0, v0
1010e9ac4:     	fmov	x8, d0
1010e9ac8:     	mul	x23, x8, x13
1010e9acc:     	b	0x1010ea03c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
1010e9ad0:     	ldur	x23, [x5, #-0x8]
1010e9ad4:     	b	0x1010ea04c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
1010e9ad8:     	mov	x10, #0x0               ; =0
1010e9adc:     	add	x27, sp, #0xd0
1010e9ae0:     	lsl	x11, x24, #2
1010e9ae4:     	mov	x8, x22
1010e9ae8:     	cmp	x11, x10
1010e9aec:     	b.eq	0x1010ea070 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x810>
1010e9af0:     	ldr	w9, [x8, x10]
1010e9af4:     	lsr	x12, x21, x9
1010e9af8:     	add	x10, x10, #0x4
1010e9afc:     	tbz	w12, #0x0, 0x1010e9ae8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x288>
1010e9b00:     	ldr	q0, [x19]
1010e9b04:     	str	q0, [sp, #0xb0]
1010e9b08:     	ldr	x10, [x19, #0x10]
1010e9b0c:     	str	x10, [sp, #0xc0]
1010e9b10:     	add	x0, sp, #0x70
1010e9b14:     	mov	x21, x8
1010e9b18:     	add	x1, sp, #0xb0
1010e9b1c:     	mov	x22, x23
1010e9b20:     	mov	x2, x23
1010e9b24:     	mov	x23, x9
1010e9b28:     	mov	x3, x23
1010e9b2c:     	mov	w4, #0x0                ; =0
1010e9b30:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e9b34:     	ldr	q0, [sp, #0x70]
1010e9b38:     	str	q0, [sp, #0x50]
1010e9b3c:     	ldr	x8, [sp, #0x80]
1010e9b40:     	str	q0, [sp, #0x30]
1010e9b44:     	str	q0, [sp, #0x90]
1010e9b48:     	str	x8, [sp, #0xa0]
1010e9b4c:     	ldr	q0, [sp, #0x90]
1010e9b50:     	str	x8, [sp, #0xe0]
1010e9b54:     	str	q0, [sp, #0xd0]
1010e9b58:     	ldur	q0, [x19, #0x18]
1010e9b5c:     	str	q0, [sp, #0xb0]
1010e9b60:     	ldur	x8, [x19, #0x28]
1010e9b64:     	str	x8, [sp, #0xc0]
1010e9b68:     	add	x0, sp, #0x70
1010e9b6c:     	add	x1, sp, #0xb0
1010e9b70:     	mov	x2, x22
1010e9b74:     	mov	x3, x23
1010e9b78:     	mov	w4, #0x0                ; =0
1010e9b7c:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e9b80:     	ldr	q0, [sp, #0x70]
1010e9b84:     	str	q0, [sp, #0x50]
1010e9b88:     	ldr	x8, [sp, #0x80]
1010e9b8c:     	str	q0, [sp, #0x30]
1010e9b90:     	str	q0, [sp, #0x90]
1010e9b94:     	str	x8, [sp, #0xa0]
1010e9b98:     	ldr	q0, [sp, #0x90]
1010e9b9c:     	str	x8, [sp, #0xf8]
1010e9ba0:     	stur	q0, [x27, #0x18]
1010e9ba4:     	ldp	q0, q1, [sp, #0xd0]
1010e9ba8:     	ldr	q2, [sp, #0xf0]
1010e9bac:     	stp	q1, q2, [x29, #-0x90]
1010e9bb0:     	stur	q0, [x29, #-0xa0]
1010e9bb4:     	ldp	q0, q1, [x29, #-0xa0]
1010e9bb8:     	ldur	q2, [x29, #-0x80]
1010e9bbc:     	stp	q1, q2, [sp, #0x10]
1010e9bc0:     	str	q0, [sp]
1010e9bc4:     	mov	x1, sp
1010e9bc8:     	mov	x0, x22
1010e9bcc:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e9bd0:     	mov	x25, x0
1010e9bd4:     	mov	x3, sp
1010e9bd8:     	mov	x0, x22
1010e9bdc:     	mov	x1, x21
1010e9be0:     	mov	x2, x24
1010e9be4:     	mov	x4, x20
1010e9be8:     	bl	0x1010e9860 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
1010e9bec:     	fmov	d0, x25
1010e9bf0:     	cnt.8b	v0, v0
1010e9bf4:     	addv.8b	b0, v0
1010e9bf8:     	fmov	w8, s0
1010e9bfc:     	sub	w8, w8, w26
1010e9c00:     	mvn	w8, w8
1010e9c04:     	lsl	x25, x0, x8
1010e9c08:     	ldr	q0, [x19]
1010e9c0c:     	str	q0, [sp, #0xb0]
1010e9c10:     	ldr	x8, [x19, #0x10]
1010e9c14:     	str	x8, [sp, #0xc0]
1010e9c18:     	add	x0, sp, #0x70
1010e9c1c:     	add	x1, sp, #0xb0
1010e9c20:     	mov	x2, x22
1010e9c24:     	mov	x3, x23
1010e9c28:     	mov	w4, #0x1                ; =1
1010e9c2c:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e9c30:     	ldr	q0, [sp, #0x70]
1010e9c34:     	str	q0, [sp, #0x50]
1010e9c38:     	ldr	x8, [sp, #0x80]
1010e9c3c:     	str	q0, [sp, #0x30]
1010e9c40:     	str	q0, [sp, #0x90]
1010e9c44:     	str	x8, [sp, #0xa0]
1010e9c48:     	ldr	q0, [sp, #0x90]
1010e9c4c:     	str	x8, [sp, #0xe0]
1010e9c50:     	str	q0, [sp, #0xd0]
1010e9c54:     	ldur	q0, [x19, #0x18]
1010e9c58:     	str	q0, [sp, #0xb0]
1010e9c5c:     	ldur	x8, [x19, #0x28]
1010e9c60:     	str	x8, [sp, #0xc0]
1010e9c64:     	add	x0, sp, #0x70
1010e9c68:     	add	x1, sp, #0xb0
1010e9c6c:     	mov	x2, x22
1010e9c70:     	mov	x3, x23
1010e9c74:     	mov	w4, #0x1                ; =1
1010e9c78:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e9c7c:     	ldr	q0, [sp, #0x70]
1010e9c80:     	str	q0, [sp, #0x50]
1010e9c84:     	ldr	x8, [sp, #0x80]
1010e9c88:     	str	q0, [sp, #0x30]
1010e9c8c:     	str	q0, [sp, #0x90]
1010e9c90:     	str	x8, [sp, #0xa0]
1010e9c94:     	ldr	q0, [sp, #0x90]
1010e9c98:     	str	x8, [sp, #0xf8]
1010e9c9c:     	stur	q0, [x27, #0x18]
1010e9ca0:     	ldp	q0, q1, [sp, #0xd0]
1010e9ca4:     	ldr	q2, [sp, #0xf0]
1010e9ca8:     	stp	q1, q2, [x29, #-0x90]
1010e9cac:     	stur	q0, [x29, #-0xa0]
1010e9cb0:     	ldp	q0, q1, [x29, #-0xa0]
1010e9cb4:     	ldur	q2, [x29, #-0x80]
1010e9cb8:     	stp	q1, q2, [sp, #0x10]
1010e9cbc:     	str	q0, [sp]
1010e9cc0:     	mov	x1, sp
1010e9cc4:     	mov	x0, x22
1010e9cc8:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e9ccc:     	mov	x23, x0
1010e9cd0:     	mov	x3, sp
1010e9cd4:     	mov	x0, x22
1010e9cd8:     	mov	x1, x21
1010e9cdc:     	mov	x2, x24
1010e9ce0:     	mov	x4, x20
1010e9ce4:     	bl	0x1010e9860 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
1010e9ce8:     	fmov	d0, x23
1010e9cec:     	cnt.8b	v0, v0
1010e9cf0:     	addv.8b	b0, v0
1010e9cf4:     	fmov	w8, s0
1010e9cf8:     	sub	w8, w8, w26
1010e9cfc:     	mvn	w8, w8
1010e9d00:     	lsl	x8, x0, x8
1010e9d04:     	add	x23, x8, x25
1010e9d08:     	b	0x1010ea03c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
1010e9d0c:     	ldp	x15, x12, [x29, #-0x70]
1010e9d10:     	sub	x16, x13, #0x1
1010e9d14:     	cmn	x25, #0x2
1010e9d18:     	b.ne	0x1010e9d64 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x504>
1010e9d1c:     	mov	x0, x15
1010e9d20:     	cmp	x15, x16
1010e9d24:     	b.ls	0x1010ea07c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1010e9d28:     	and	x9, x9, x14
1010e9d2c:     	cmp	x13, #0x8
1010e9d30:     	b.hs	0x1010e9da0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x540>
1010e9d34:     	mov	x23, #0x0               ; =0
1010e9d38:     	mov	x13, #0x0               ; =0
1010e9d3c:     	b	0x1010e9e28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5c8>
1010e9d40:     	sub	x12, x13, #0x1
1010e9d44:     	cmp	x0, x12
1010e9d48:     	tbz	w21, #0x0, 0x1010e9d94 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x534>
1010e9d4c:     	b.ls	0x1010ea07c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1010e9d50:     	cmp	x13, #0x8
1010e9d54:     	b.hs	0x1010e9f60 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x700>
1010e9d58:     	mov	x23, #0x0               ; =0
1010e9d5c:     	mov	x13, #0x0               ; =0
1010e9d60:     	b	0x1010e9fe8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x788>
1010e9d64:     	cmp	x15, x16
1010e9d68:     	csel	x14, x15, x16, lo
1010e9d6c:     	cmp	x0, x14
1010e9d70:     	b.ls	0x1010ea07c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1010e9d74:     	mov	x0, x15
1010e9d78:     	cmp	x15, x14
1010e9d7c:     	b.eq	0x1010ea07c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1010e9d80:     	cmp	x13, #0x8
1010e9d84:     	b.hs	0x1010e9e60 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x600>
1010e9d88:     	mov	x23, #0x0               ; =0
1010e9d8c:     	mov	x15, #0x0               ; =0
1010e9d90:     	b	0x1010e9f14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6b4>
1010e9d94:     	b.ls	0x1010ea07c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1010e9d98:     	mov	x23, #0x0               ; =0
1010e9d9c:     	b	0x1010ea01c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1010e9da0:     	dup.2d	v0, x12
1010e9da4:     	dup.2d	v1, x9
1010e9da8:     	ldp	q3, q2, [x21, #0x20]
1010e9dac:     	eor.16b	v2, v2, v0
1010e9db0:     	and.16b	v2, v2, v1
1010e9db4:     	cnt.16b	v2, v2
1010e9db8:     	movi.16b	v4, #0x1
1010e9dbc:     	movi.2d	v5, #0000000000000000
1010e9dc0:     	udot.4s	v5, v4, v2
1010e9dc4:     	eor.16b	v2, v3, v0
1010e9dc8:     	and.16b	v2, v2, v1
1010e9dcc:     	cnt.16b	v2, v2
1010e9dd0:     	movi.2d	v3, #0000000000000000
1010e9dd4:     	udot.4s	v3, v4, v2
1010e9dd8:     	ldp	q6, q2, [x21]
1010e9ddc:     	eor.16b	v2, v2, v0
1010e9de0:     	and.16b	v2, v2, v1
1010e9de4:     	cnt.16b	v2, v2
1010e9de8:     	movi.2d	v7, #0000000000000000
1010e9dec:     	udot.4s	v7, v4, v2
1010e9df0:     	movi.2d	v2, #0000000000000000
1010e9df4:     	uaddlp.2d	v7, v7
1010e9df8:     	eor.16b	v0, v6, v0
1010e9dfc:     	and.16b	v0, v0, v1
1010e9e00:     	cnt.16b	v0, v0
1010e9e04:     	udot.4s	v2, v4, v0
1010e9e08:     	uadalp.2d	v7, v2
1010e9e0c:     	uadalp.2d	v7, v3
1010e9e10:     	uadalp.2d	v7, v5
1010e9e14:     	addp.2d	d0, v7
1010e9e18:     	fmov	x23, d0
1010e9e1c:     	cmp	x13, #0x8
1010e9e20:     	b.eq	0x1010ea02c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
1010e9e24:     	mov	w13, #0x8               ; =8
1010e9e28:     	add	x11, x21, x13, lsl #3
1010e9e2c:     	add	x8, x10, x8
1010e9e30:     	sub	x8, x8, x13
1010e9e34:     	ldr	x10, [x11], #0x8
1010e9e38:     	eor	x10, x10, x12
1010e9e3c:     	and	x10, x10, x9
1010e9e40:     	fmov	d0, x10
1010e9e44:     	cnt.8b	v0, v0
1010e9e48:     	addv.8b	b0, v0
1010e9e4c:     	fmov	x10, d0
1010e9e50:     	add	x23, x10, x23
1010e9e54:     	subs	x8, x8, #0x1
1010e9e58:     	b.ne	0x1010e9e34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5d4>
1010e9e5c:     	b	0x1010ea02c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
1010e9e60:     	dup.2d	v0, x11
1010e9e64:     	ldp	q2, q1, [x22, #0x20]
1010e9e68:     	eor.16b	v1, v1, v0
1010e9e6c:     	dup.2d	v3, x12
1010e9e70:     	ldp	q5, q4, [x21, #0x20]
1010e9e74:     	eor.16b	v4, v4, v3
1010e9e78:     	and.16b	v1, v1, v4
1010e9e7c:     	dup.2d	v4, x9
1010e9e80:     	and.16b	v1, v1, v4
1010e9e84:     	cnt.16b	v1, v1
1010e9e88:     	movi.16b	v6, #0x1
1010e9e8c:     	movi.2d	v7, #0000000000000000
1010e9e90:     	udot.4s	v7, v6, v1
1010e9e94:     	eor.16b	v1, v2, v0
1010e9e98:     	eor.16b	v2, v5, v3
1010e9e9c:     	and.16b	v1, v1, v2
1010e9ea0:     	and.16b	v1, v1, v4
1010e9ea4:     	cnt.16b	v1, v1
1010e9ea8:     	movi.2d	v2, #0000000000000000
1010e9eac:     	udot.4s	v2, v6, v1
1010e9eb0:     	ldp	q5, q1, [x22]
1010e9eb4:     	eor.16b	v1, v1, v0
1010e9eb8:     	ldp	q17, q16, [x21]
1010e9ebc:     	eor.16b	v16, v16, v3
1010e9ec0:     	and.16b	v1, v1, v16
1010e9ec4:     	and.16b	v1, v1, v4
1010e9ec8:     	cnt.16b	v1, v1
1010e9ecc:     	movi.2d	v16, #0000000000000000
1010e9ed0:     	udot.4s	v16, v6, v1
1010e9ed4:     	eor.16b	v0, v5, v0
1010e9ed8:     	movi.2d	v1, #0000000000000000
1010e9edc:     	uaddlp.2d	v5, v16
1010e9ee0:     	eor.16b	v3, v17, v3
1010e9ee4:     	and.16b	v0, v0, v3
1010e9ee8:     	and.16b	v0, v0, v4
1010e9eec:     	cnt.16b	v0, v0
1010e9ef0:     	udot.4s	v1, v6, v0
1010e9ef4:     	uadalp.2d	v5, v1
1010e9ef8:     	uadalp.2d	v5, v2
1010e9efc:     	uadalp.2d	v5, v7
1010e9f00:     	addp.2d	d0, v5
1010e9f04:     	fmov	x23, d0
1010e9f08:     	cmp	x13, #0x8
1010e9f0c:     	b.eq	0x1010ea01c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1010e9f10:     	mov	w15, #0x8               ; =8
1010e9f14:     	lsl	x14, x15, #3
1010e9f18:     	add	x13, x21, x14
1010e9f1c:     	add	x14, x22, x14
1010e9f20:     	add	x8, x10, x8
1010e9f24:     	sub	x8, x8, x15
1010e9f28:     	ldr	x10, [x14], #0x8
1010e9f2c:     	eor	x10, x10, x11
1010e9f30:     	ldr	x15, [x13], #0x8
1010e9f34:     	eor	x15, x15, x12
1010e9f38:     	and	x10, x10, x15
1010e9f3c:     	and	x10, x10, x9
1010e9f40:     	fmov	d0, x10
1010e9f44:     	cnt.8b	v0, v0
1010e9f48:     	addv.8b	b0, v0
1010e9f4c:     	fmov	x10, d0
1010e9f50:     	add	x23, x10, x23
1010e9f54:     	subs	x8, x8, #0x1
1010e9f58:     	b.ne	0x1010e9f28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6c8>
1010e9f5c:     	b	0x1010ea01c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1010e9f60:     	dup.2d	v0, x11
1010e9f64:     	dup.2d	v1, x9
1010e9f68:     	ldp	q3, q2, [x22, #0x20]
1010e9f6c:     	eor.16b	v2, v2, v0
1010e9f70:     	and.16b	v2, v2, v1
1010e9f74:     	cnt.16b	v2, v2
1010e9f78:     	movi.16b	v4, #0x1
1010e9f7c:     	movi.2d	v5, #0000000000000000
1010e9f80:     	udot.4s	v5, v4, v2
1010e9f84:     	eor.16b	v2, v3, v0
1010e9f88:     	and.16b	v2, v2, v1
1010e9f8c:     	cnt.16b	v2, v2
1010e9f90:     	movi.2d	v3, #0000000000000000
1010e9f94:     	udot.4s	v3, v4, v2
1010e9f98:     	ldp	q6, q2, [x22]
1010e9f9c:     	eor.16b	v2, v2, v0
1010e9fa0:     	and.16b	v2, v2, v1
1010e9fa4:     	cnt.16b	v2, v2
1010e9fa8:     	movi.2d	v7, #0000000000000000
1010e9fac:     	udot.4s	v7, v4, v2
1010e9fb0:     	movi.2d	v2, #0000000000000000
1010e9fb4:     	uaddlp.2d	v7, v7
1010e9fb8:     	eor.16b	v0, v6, v0
1010e9fbc:     	and.16b	v0, v0, v1
1010e9fc0:     	cnt.16b	v0, v0
1010e9fc4:     	udot.4s	v2, v4, v0
1010e9fc8:     	uadalp.2d	v7, v2
1010e9fcc:     	uadalp.2d	v7, v3
1010e9fd0:     	uadalp.2d	v7, v5
1010e9fd4:     	addp.2d	d0, v7
1010e9fd8:     	fmov	x23, d0
1010e9fdc:     	cmp	x13, #0x8
1010e9fe0:     	b.eq	0x1010ea01c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1010e9fe4:     	mov	w13, #0x8               ; =8
1010e9fe8:     	add	x12, x22, x13, lsl #3
1010e9fec:     	add	x8, x10, x8
1010e9ff0:     	sub	x8, x8, x13
1010e9ff4:     	ldr	x10, [x12], #0x8
1010e9ff8:     	eor	x10, x10, x11
1010e9ffc:     	and	x10, x10, x9
1010ea000:     	fmov	d0, x10
1010ea004:     	cnt.8b	v0, v0
1010ea008:     	addv.8b	b0, v0
1010ea00c:     	fmov	x10, d0
1010ea010:     	add	x23, x10, x23
1010ea014:     	subs	x8, x8, #0x1
1010ea018:     	b.ne	0x1010e9ff4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x794>
1010ea01c:     	cmp	x25, #0x1
1010ea020:     	b.lt	0x1010ea02c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
1010ea024:     	mov	x0, x22
1010ea028:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010ea02c:     	cmp	x24, #0x1
1010ea030:     	b.lt	0x1010ea03c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
1010ea034:     	mov	x0, x21
1010ea038:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010ea03c:     	mov	x0, x20
1010ea040:     	mov	x1, x19
1010ea044:     	mov	x2, x23
1010ea048:     	bl	0x10121d47c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
1010ea04c:     	mov	x0, x23
1010ea050:     	ldp	x29, x30, [sp, #0x1d0]
1010ea054:     	ldp	x20, x19, [sp, #0x1c0]
1010ea058:     	ldp	x22, x21, [sp, #0x1b0]
1010ea05c:     	ldp	x24, x23, [sp, #0x1a0]
1010ea060:     	ldp	x26, x25, [sp, #0x190]
1010ea064:     	ldp	x28, x27, [sp, #0x180]
1010ea068:     	add	sp, sp, #0x1e0
1010ea06c:     	ret
1010ea070:     	adrp	x0, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010ea074:     	add	x0, x0, #0xf80
1010ea078:     	bl	0x1018c1834 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1010ea07c:     	adrp	x2, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010ea080:     	add	x2, x2, #0x2f0
1010ea084:     	mov	x1, x0
1010ea088:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010ea08c:     	brk	#0x1
1010ea090:     	mov	x19, x0
1010ea094:     	sub	x0, x29, #0xa0
1010ea098:     	bl	0x100b14790 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej2_EBK_>
1010ea09c:     	mov	x0, x19
1010ea0a0:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
1010ea0a4:     	mov	x19, x0
1010ea0a8:     	cmp	x25, #0x1
1010ea0ac:     	b.lt	0x1010ea0b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x858>
1010ea0b0:     	mov	x0, x22
1010ea0b4:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010ea0b8:     	mov	x0, x19
1010ea0bc:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
