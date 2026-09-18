
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010e89a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_>:
1010e89a8:     	sub	sp, sp, #0x1e0
1010e89ac:     	stp	x28, x27, [sp, #0x180]
1010e89b0:     	stp	x26, x25, [sp, #0x190]
1010e89b4:     	stp	x24, x23, [sp, #0x1a0]
1010e89b8:     	stp	x22, x21, [sp, #0x1b0]
1010e89bc:     	stp	x20, x19, [sp, #0x1c0]
1010e89c0:     	stp	x29, x30, [sp, #0x1d0]
1010e89c4:     	add	x29, sp, #0x1d0
1010e89c8:     	ldr	w8, [x3, #0x10]
1010e89cc:     	cbz	w8, 0x1010e8b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x158>
1010e89d0:     	mov	x19, x3
1010e89d4:     	ldr	w9, [x3, #0x28]
1010e89d8:     	cbz	w9, 0x1010e8b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x158>
1010e89dc:     	mov	x20, x4
1010e89e0:     	ldr	x10, [x4, #0x18]
1010e89e4:     	cbz	x10, 0x1010e8b08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x160>
1010e89e8:     	mov	x10, #0x0               ; =0
1010e89ec:     	mov	x15, #0xa9c5            ; =43461
1010e89f0:     	movk	x15, #0x2e62, lsl #16
1010e89f4:     	movk	x15, #0x7aea, lsl #32
1010e89f8:     	movk	x15, #0xf135, lsl #48
1010e89fc:     	ldp	x11, x12, [x19]
1010e8a00:     	madd	x13, x8, x15, x11
1010e8a04:     	mov	x14, #0x6332            ; =25394
1010e8a08:     	movk	x14, #0x6ed3, lsl #16
1010e8a0c:     	movk	x14, #0x765a, lsl #32
1010e8a10:     	movk	x14, #0x284f, lsl #48
1010e8a14:     	mul	x14, x14, x15
1010e8a18:     	madd	x13, x13, x15, x14
1010e8a1c:     	add	x13, x13, x12
1010e8a20:     	madd	x16, x13, x15, x9
1010e8a24:     	ldp	x13, x14, [x19, #0x18]
1010e8a28:     	madd	x16, x16, x15, x13
1010e8a2c:     	madd	x16, x16, x15, x14
1010e8a30:     	mul	x15, x16, x15
1010e8a34:     	ror	x3, x15, #0x2c
1010e8a38:     	lsr	x17, x3, #57
1010e8a3c:     	ldp	x16, x15, [x20]
1010e8a40:     	dup.8b	v0, w17
1010e8a44:     	movi.2d	v1, #0xffffffffffffffff
1010e8a48:     	mov	w17, #0x38              ; =56
1010e8a4c:     	and	x3, x3, x15
1010e8a50:     	ldr	d2, [x16, x3]
1010e8a54:     	cmeq.8b	v3, v2, v0
1010e8a58:     	fmov	x4, d3
1010e8a5c:     	ands	x4, x4, #0x8080808080808080
1010e8a60:     	b.eq	0x1010e8ad0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x128>
1010e8a64:     	rbit	x5, x4
1010e8a68:     	clz	x5, x5
1010e8a6c:     	add	x5, x3, x5, lsr #3
1010e8a70:     	and	x5, x5, x15
1010e8a74:     	mneg	x5, x5, x17
1010e8a78:     	add	x5, x16, x5
1010e8a7c:     	ldur	x6, [x5, #-0x38]
1010e8a80:     	cmp	x11, x6
1010e8a84:     	b.ne	0x1010e8ac4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x11c>
1010e8a88:     	ldur	x6, [x5, #-0x30]
1010e8a8c:     	cmp	x12, x6
1010e8a90:     	b.ne	0x1010e8ac4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x11c>
1010e8a94:     	ldur	w6, [x5, #-0x28]
1010e8a98:     	cmp	w8, w6
1010e8a9c:     	b.ne	0x1010e8ac4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x11c>
1010e8aa0:     	ldur	x6, [x5, #-0x20]
1010e8aa4:     	cmp	x13, x6
1010e8aa8:     	b.ne	0x1010e8ac4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x11c>
1010e8aac:     	ldur	x6, [x5, #-0x18]
1010e8ab0:     	cmp	x14, x6
1010e8ab4:     	b.ne	0x1010e8ac4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x11c>
1010e8ab8:     	ldur	w6, [x5, #-0x10]
1010e8abc:     	cmp	w9, w6
1010e8ac0:     	b.eq	0x1010e8c14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x26c>
1010e8ac4:     	sub	x5, x4, #0x2
1010e8ac8:     	ands	x4, x5, x4
1010e8acc:     	b.ne	0x1010e8a64 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0xbc>
1010e8ad0:     	cmeq.8b	v2, v2, v1
1010e8ad4:     	fmov	x4, d2
1010e8ad8:     	cbnz	x4, 0x1010e8b08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x160>
1010e8adc:     	add	x10, x10, #0x8
1010e8ae0:     	add	x3, x3, x10
1010e8ae4:     	and	x3, x3, x15
1010e8ae8:     	ldr	d2, [x16, x3]
1010e8aec:     	cmeq.8b	v3, v2, v0
1010e8af0:     	fmov	x4, d3
1010e8af4:     	ands	x4, x4, #0x8080808080808080
1010e8af8:     	b.ne	0x1010e8a64 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0xbc>
1010e8afc:     	b	0x1010e8ad0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x128>
1010e8b00:     	mov	x23, #0x0               ; =0
1010e8b04:     	b	0x1010e8f80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5d8>
1010e8b08:     	mov	x22, x1
1010e8b0c:     	mov	x24, x2
1010e8b10:     	mov	x23, x0
1010e8b14:     	mov	x1, x19
1010e8b18:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e8b1c:     	mov	x21, x0
1010e8b20:     	fmov	d0, x21
1010e8b24:     	cnt.8b	v0, v0
1010e8b28:     	addv.8b	b0, v0
1010e8b2c:     	fmov	x26, d0
1010e8b30:     	cmp	x26, #0x7
1010e8b34:     	b.hs	0x1010e8c1c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x274>
1010e8b38:     	strb	wzr, [sp, #0x120]
1010e8b3c:     	movi.2d	v0, #0000000000000000
1010e8b40:     	stp	q0, q0, [sp, #0x100]
1010e8b44:     	stp	q0, q0, [sp, #0xe0]
1010e8b48:     	str	q0, [sp, #0xd0]
1010e8b4c:     	sub	x0, x29, #0xa0
1010e8b50:     	add	x4, sp, #0xd0
1010e8b54:     	mov	x1, x23
1010e8b58:     	mov	x2, x19
1010e8b5c:     	mov	x3, x21
1010e8b60:     	bl	0x1000059dc <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm6_EB9_>
1010e8b64:     	ldp	x25, x22, [x29, #-0xa0]
1010e8b68:     	ldur	q0, [x29, #-0x90]
1010e8b6c:     	str	q0, [sp, #0xb0]
1010e8b70:     	str	q0, [sp, #0x90]
1010e8b74:     	str	q0, [sp]
1010e8b78:     	str	q0, [sp, #0x70]
1010e8b7c:     	sub	x0, x29, #0xa0
1010e8b80:     	add	x2, x19, #0x18
1010e8b84:     	add	x4, sp, #0xd0
1010e8b88:     	mov	x1, x23
1010e8b8c:     	mov	x3, x21
1010e8b90:     	bl	0x1000059dc <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm6_EB9_>
1010e8b94:     	ldp	x24, x21, [x29, #-0xa0]
1010e8b98:     	ldur	q0, [x29, #-0x90]
1010e8b9c:     	stp	x25, x22, [x29, #-0xa0]
1010e8ba0:     	ldr	q1, [sp, #0x70]
1010e8ba4:     	stur	q1, [x29, #-0x90]
1010e8ba8:     	stp	x24, x21, [x29, #-0x80]
1010e8bac:     	stur	q0, [x29, #-0x70]
1010e8bb0:     	mov	w8, #0x1                ; =1
1010e8bb4:     	lsl	x8, x8, x26
1010e8bb8:     	mov	x9, #-0x1               ; =-1
1010e8bbc:     	lsl	x10, x9, x8
1010e8bc0:     	cmp	x26, #0x6
1010e8bc4:     	csinv	x9, x9, x10, eq
1010e8bc8:     	lsr	x8, x8, #6
1010e8bcc:     	cinc	x8, x8, ne
1010e8bd0:     	cbz	x8, 0x1010e8f4c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5a4>
1010e8bd4:     	mov	x11, #0x0               ; =0
1010e8bd8:     	ldp	x0, x10, [x29, #-0x90]
1010e8bdc:     	sub	x12, x11, w22, uxtb
1010e8be0:     	cmn	x24, #0x2
1010e8be4:     	b.ne	0x1010e8e50 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x4a8>
1010e8be8:     	cmn	x25, #0x2
1010e8bec:     	b.ne	0x1010e8ea4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x4fc>
1010e8bf0:     	tst	w21, #0x1
1010e8bf4:     	csel	x10, x12, xzr, ne
1010e8bf8:     	and	x9, x10, x9
1010e8bfc:     	fmov	d0, x9
1010e8c00:     	cnt.8b	v0, v0
1010e8c04:     	addv.8b	b0, v0
1010e8c08:     	fmov	x9, d0
1010e8c0c:     	mul	x23, x9, x8
1010e8c10:     	b	0x1010e8f70 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5c8>
1010e8c14:     	ldur	x23, [x5, #-0x8]
1010e8c18:     	b	0x1010e8f80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5d8>
1010e8c1c:     	mov	x10, #0x0               ; =0
1010e8c20:     	add	x27, sp, #0xd0
1010e8c24:     	lsl	x11, x24, #2
1010e8c28:     	mov	x8, x22
1010e8c2c:     	cmp	x11, x10
1010e8c30:     	b.eq	0x1010e8fa4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5fc>
1010e8c34:     	ldr	w9, [x8, x10]
1010e8c38:     	lsr	x12, x21, x9
1010e8c3c:     	add	x10, x10, #0x4
1010e8c40:     	tbz	w12, #0x0, 0x1010e8c2c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x284>
1010e8c44:     	ldr	q0, [x19]
1010e8c48:     	str	q0, [sp, #0xb0]
1010e8c4c:     	ldr	x10, [x19, #0x10]
1010e8c50:     	str	x10, [sp, #0xc0]
1010e8c54:     	add	x0, sp, #0x70
1010e8c58:     	mov	x21, x8
1010e8c5c:     	add	x1, sp, #0xb0
1010e8c60:     	mov	x22, x23
1010e8c64:     	mov	x2, x23
1010e8c68:     	mov	x23, x9
1010e8c6c:     	mov	x3, x23
1010e8c70:     	mov	w4, #0x0                ; =0
1010e8c74:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e8c78:     	ldr	q0, [sp, #0x70]
1010e8c7c:     	str	q0, [sp, #0x50]
1010e8c80:     	ldr	x8, [sp, #0x80]
1010e8c84:     	str	q0, [sp, #0x30]
1010e8c88:     	str	q0, [sp, #0x90]
1010e8c8c:     	str	x8, [sp, #0xa0]
1010e8c90:     	ldr	q0, [sp, #0x90]
1010e8c94:     	str	x8, [sp, #0xe0]
1010e8c98:     	str	q0, [sp, #0xd0]
1010e8c9c:     	ldur	q0, [x19, #0x18]
1010e8ca0:     	str	q0, [sp, #0xb0]
1010e8ca4:     	ldur	x8, [x19, #0x28]
1010e8ca8:     	str	x8, [sp, #0xc0]
1010e8cac:     	add	x0, sp, #0x70
1010e8cb0:     	add	x1, sp, #0xb0
1010e8cb4:     	mov	x2, x22
1010e8cb8:     	mov	x3, x23
1010e8cbc:     	mov	w4, #0x0                ; =0
1010e8cc0:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e8cc4:     	ldr	q0, [sp, #0x70]
1010e8cc8:     	str	q0, [sp, #0x50]
1010e8ccc:     	ldr	x8, [sp, #0x80]
1010e8cd0:     	str	q0, [sp, #0x30]
1010e8cd4:     	str	q0, [sp, #0x90]
1010e8cd8:     	str	x8, [sp, #0xa0]
1010e8cdc:     	ldr	q0, [sp, #0x90]
1010e8ce0:     	str	x8, [sp, #0xf8]
1010e8ce4:     	stur	q0, [x27, #0x18]
1010e8ce8:     	ldp	q0, q1, [sp, #0xd0]
1010e8cec:     	ldr	q2, [sp, #0xf0]
1010e8cf0:     	stp	q1, q2, [x29, #-0x90]
1010e8cf4:     	stur	q0, [x29, #-0xa0]
1010e8cf8:     	ldp	q0, q1, [x29, #-0xa0]
1010e8cfc:     	ldur	q2, [x29, #-0x80]
1010e8d00:     	stp	q1, q2, [sp, #0x10]
1010e8d04:     	str	q0, [sp]
1010e8d08:     	mov	x1, sp
1010e8d0c:     	mov	x0, x22
1010e8d10:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e8d14:     	mov	x25, x0
1010e8d18:     	mov	x3, sp
1010e8d1c:     	mov	x0, x22
1010e8d20:     	mov	x1, x21
1010e8d24:     	mov	x2, x24
1010e8d28:     	mov	x4, x20
1010e8d2c:     	bl	0x1010e89a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_>
1010e8d30:     	fmov	d0, x25
1010e8d34:     	cnt.8b	v0, v0
1010e8d38:     	addv.8b	b0, v0
1010e8d3c:     	fmov	w8, s0
1010e8d40:     	sub	w8, w8, w26
1010e8d44:     	mvn	w8, w8
1010e8d48:     	lsl	x25, x0, x8
1010e8d4c:     	ldr	q0, [x19]
1010e8d50:     	str	q0, [sp, #0xb0]
1010e8d54:     	ldr	x8, [x19, #0x10]
1010e8d58:     	str	x8, [sp, #0xc0]
1010e8d5c:     	add	x0, sp, #0x70
1010e8d60:     	add	x1, sp, #0xb0
1010e8d64:     	mov	x2, x22
1010e8d68:     	mov	x3, x23
1010e8d6c:     	mov	w4, #0x1                ; =1
1010e8d70:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e8d74:     	ldr	q0, [sp, #0x70]
1010e8d78:     	str	q0, [sp, #0x50]
1010e8d7c:     	ldr	x8, [sp, #0x80]
1010e8d80:     	str	q0, [sp, #0x30]
1010e8d84:     	str	q0, [sp, #0x90]
1010e8d88:     	str	x8, [sp, #0xa0]
1010e8d8c:     	ldr	q0, [sp, #0x90]
1010e8d90:     	str	x8, [sp, #0xe0]
1010e8d94:     	str	q0, [sp, #0xd0]
1010e8d98:     	ldur	q0, [x19, #0x18]
1010e8d9c:     	str	q0, [sp, #0xb0]
1010e8da0:     	ldur	x8, [x19, #0x28]
1010e8da4:     	str	x8, [sp, #0xc0]
1010e8da8:     	add	x0, sp, #0x70
1010e8dac:     	add	x1, sp, #0xb0
1010e8db0:     	mov	x2, x22
1010e8db4:     	mov	x3, x23
1010e8db8:     	mov	w4, #0x1                ; =1
1010e8dbc:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e8dc0:     	ldr	q0, [sp, #0x70]
1010e8dc4:     	str	q0, [sp, #0x50]
1010e8dc8:     	ldr	x8, [sp, #0x80]
1010e8dcc:     	str	q0, [sp, #0x30]
1010e8dd0:     	str	q0, [sp, #0x90]
1010e8dd4:     	str	x8, [sp, #0xa0]
1010e8dd8:     	ldr	q0, [sp, #0x90]
1010e8ddc:     	str	x8, [sp, #0xf8]
1010e8de0:     	stur	q0, [x27, #0x18]
1010e8de4:     	ldp	q0, q1, [sp, #0xd0]
1010e8de8:     	ldr	q2, [sp, #0xf0]
1010e8dec:     	stp	q1, q2, [x29, #-0x90]
1010e8df0:     	stur	q0, [x29, #-0xa0]
1010e8df4:     	ldp	q0, q1, [x29, #-0xa0]
1010e8df8:     	ldur	q2, [x29, #-0x80]
1010e8dfc:     	stp	q1, q2, [sp, #0x10]
1010e8e00:     	str	q0, [sp]
1010e8e04:     	mov	x1, sp
1010e8e08:     	mov	x0, x22
1010e8e0c:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e8e10:     	mov	x23, x0
1010e8e14:     	mov	x3, sp
1010e8e18:     	mov	x0, x22
1010e8e1c:     	mov	x1, x21
1010e8e20:     	mov	x2, x24
1010e8e24:     	mov	x4, x20
1010e8e28:     	bl	0x1010e89a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_>
1010e8e2c:     	fmov	d0, x23
1010e8e30:     	cnt.8b	v0, v0
1010e8e34:     	addv.8b	b0, v0
1010e8e38:     	fmov	w8, s0
1010e8e3c:     	sub	w8, w8, w26
1010e8e40:     	mvn	w8, w8
1010e8e44:     	lsl	x8, x0, x8
1010e8e48:     	add	x23, x8, x25
1010e8e4c:     	b	0x1010e8f70 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5c8>
1010e8e50:     	ldp	x13, x11, [x29, #-0x70]
1010e8e54:     	sub	x14, x8, #0x1
1010e8e58:     	cmn	x25, #0x2
1010e8e5c:     	b.ne	0x1010e8ee8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x540>
1010e8e60:     	mov	x0, x13
1010e8e64:     	cmp	x13, x14
1010e8e68:     	b.ls	0x1010e8fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x608>
1010e8e6c:     	mov	x23, #0x0               ; =0
1010e8e70:     	and	x9, x12, x9
1010e8e74:     	mov	x10, x21
1010e8e78:     	ldr	x12, [x10], #0x8
1010e8e7c:     	eor	x12, x12, x11
1010e8e80:     	and	x12, x12, x9
1010e8e84:     	fmov	d0, x12
1010e8e88:     	cnt.8b	v0, v0
1010e8e8c:     	addv.8b	b0, v0
1010e8e90:     	fmov	x12, d0
1010e8e94:     	add	x23, x12, x23
1010e8e98:     	subs	x8, x8, #0x1
1010e8e9c:     	b.ne	0x1010e8e78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x4d0>
1010e8ea0:     	b	0x1010e8f60 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5b8>
1010e8ea4:     	sub	x11, x8, #0x1
1010e8ea8:     	cmp	x0, x11
1010e8eac:     	tbz	w21, #0x0, 0x1010e8f48 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5a0>
1010e8eb0:     	b.ls	0x1010e8fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x608>
1010e8eb4:     	mov	x23, #0x0               ; =0
1010e8eb8:     	mov	x11, x22
1010e8ebc:     	ldr	x12, [x11], #0x8
1010e8ec0:     	eor	x12, x12, x10
1010e8ec4:     	and	x12, x12, x9
1010e8ec8:     	fmov	d0, x12
1010e8ecc:     	cnt.8b	v0, v0
1010e8ed0:     	addv.8b	b0, v0
1010e8ed4:     	fmov	x12, d0
1010e8ed8:     	add	x23, x12, x23
1010e8edc:     	subs	x8, x8, #0x1
1010e8ee0:     	b.ne	0x1010e8ebc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x514>
1010e8ee4:     	b	0x1010e8f50 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5a8>
1010e8ee8:     	cmp	x13, x14
1010e8eec:     	csel	x12, x13, x14, lo
1010e8ef0:     	cmp	x0, x12
1010e8ef4:     	b.ls	0x1010e8fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x608>
1010e8ef8:     	mov	x0, x13
1010e8efc:     	cmp	x13, x12
1010e8f00:     	b.eq	0x1010e8fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x608>
1010e8f04:     	mov	x23, #0x0               ; =0
1010e8f08:     	mov	x12, x22
1010e8f0c:     	mov	x13, x21
1010e8f10:     	ldr	x14, [x12], #0x8
1010e8f14:     	eor	x14, x14, x10
1010e8f18:     	ldr	x15, [x13], #0x8
1010e8f1c:     	eor	x15, x15, x11
1010e8f20:     	and	x14, x14, x15
1010e8f24:     	and	x14, x14, x9
1010e8f28:     	fmov	d0, x14
1010e8f2c:     	cnt.8b	v0, v0
1010e8f30:     	addv.8b	b0, v0
1010e8f34:     	fmov	x14, d0
1010e8f38:     	add	x23, x14, x23
1010e8f3c:     	subs	x8, x8, #0x1
1010e8f40:     	b.ne	0x1010e8f10 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x568>
1010e8f44:     	b	0x1010e8f50 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5a8>
1010e8f48:     	b.ls	0x1010e8fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x608>
1010e8f4c:     	mov	x23, #0x0               ; =0
1010e8f50:     	cmp	x25, #0x1
1010e8f54:     	b.lt	0x1010e8f60 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5b8>
1010e8f58:     	mov	x0, x22
1010e8f5c:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e8f60:     	cmp	x24, #0x1
1010e8f64:     	b.lt	0x1010e8f70 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x5c8>
1010e8f68:     	mov	x0, x21
1010e8f6c:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e8f70:     	mov	x0, x20
1010e8f74:     	mov	x1, x19
1010e8f78:     	mov	x2, x23
1010e8f7c:     	bl	0x10121d47c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
1010e8f80:     	mov	x0, x23
1010e8f84:     	ldp	x29, x30, [sp, #0x1d0]
1010e8f88:     	ldp	x20, x19, [sp, #0x1c0]
1010e8f8c:     	ldp	x22, x21, [sp, #0x1b0]
1010e8f90:     	ldp	x24, x23, [sp, #0x1a0]
1010e8f94:     	ldp	x26, x25, [sp, #0x190]
1010e8f98:     	ldp	x28, x27, [sp, #0x180]
1010e8f9c:     	add	sp, sp, #0x1e0
1010e8fa0:     	ret
1010e8fa4:     	adrp	x0, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010e8fa8:     	add	x0, x0, #0xf80
1010e8fac:     	bl	0x1018c1834 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1010e8fb0:     	adrp	x2, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010e8fb4:     	add	x2, x2, #0x2f0
1010e8fb8:     	mov	x1, x0
1010e8fbc:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010e8fc0:     	brk	#0x1
1010e8fc4:     	mov	x19, x0
1010e8fc8:     	sub	x0, x29, #0xa0
1010e8fcc:     	bl	0x100b14790 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej2_EBK_>
1010e8fd0:     	mov	x0, x19
1010e8fd4:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
1010e8fd8:     	mov	x19, x0
1010e8fdc:     	cmp	x25, #0x1
1010e8fe0:     	b.lt	0x1010e8fec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb1_EB8_+0x644>
1010e8fe4:     	mov	x0, x22
1010e8fe8:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e8fec:     	mov	x0, x19
1010e8ff0:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
