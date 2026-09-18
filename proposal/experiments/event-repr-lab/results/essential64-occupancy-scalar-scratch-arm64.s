
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b239c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_>:
100b239c0:     	stp	d15, d14, [sp, #-0xa0]!
100b239c4:     	stp	d13, d12, [sp, #0x10]
100b239c8:     	stp	d11, d10, [sp, #0x20]
100b239cc:     	stp	d9, d8, [sp, #0x30]
100b239d0:     	stp	x28, x27, [sp, #0x40]
100b239d4:     	stp	x26, x25, [sp, #0x50]
100b239d8:     	stp	x24, x23, [sp, #0x60]
100b239dc:     	stp	x22, x21, [sp, #0x70]
100b239e0:     	stp	x20, x19, [sp, #0x80]
100b239e4:     	stp	x29, x30, [sp, #0x90]
100b239e8:     	add	x29, sp, #0x90
100b239ec:     	sub	sp, sp, #0x230
100b239f0:     	ldr	w8, [x3, #0x10]
100b239f4:     	str	x8, [sp, #0xe0]
100b239f8:     	cbz	w8, 0x100b23a2c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x6c>
100b239fc:     	mov	x21, x5
100b23a00:     	mov	x23, x4
100b23a04:     	mov	x24, x3
100b23a08:     	mov	x26, x2
100b23a0c:     	mov	x27, x1
100b23a10:     	mov	x28, x0
100b23a14:     	mov	x0, x4
100b23a18:     	mov	x1, x3
100b23a1c:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b23a20:     	cbz	x0, 0x100b23a34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x74>
100b23a24:     	ldrb	w27, [x0]
100b23a28:     	b	0x100b241dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x81c>
100b23a2c:     	mov	w27, #0x0               ; =0
100b23a30:     	b	0x100b241dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x81c>
100b23a34:     	ldr	x8, [x21]
100b23a38:     	add	x8, x8, #0x1
100b23a3c:     	str	x8, [x21]
100b23a40:     	ldr	x8, [x28, #0x30]
100b23a44:     	ldr	x1, [x28, #0x40]
100b23a48:     	ldr	x9, [x24]
100b23a4c:     	ldr	x10, [sp, #0xe0]
100b23a50:     	lsr	x0, x10, #1
100b23a54:     	cmn	x8, #0x1
100b23a58:     	b.eq	0x100b23ab4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0xf4>
100b23a5c:     	cmp	x1, x0
100b23a60:     	b.ls	0x100b24230 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x870>
100b23a64:     	ldr	w8, [x24, #0x28]
100b23a68:     	lsr	x12, x8, #1
100b23a6c:     	cmp	x1, x12
100b23a70:     	b.ls	0x100b2421c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x85c>
100b23a74:     	ldr	w10, [x24, #0x40]
100b23a78:     	lsr	x11, x10, #1
100b23a7c:     	cmp	x1, x11
100b23a80:     	b.ls	0x100b2422c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x86c>
100b23a84:     	ldr	x13, [x28, #0x38]
100b23a88:     	lsl	x12, x12, #4
100b23a8c:     	ldr	x12, [x13, x12]
100b23a90:     	ldr	x14, [x24, #0x18]
100b23a94:     	lsl	x15, x0, #4
100b23a98:     	ldr	x15, [x13, x15]
100b23a9c:     	bic	x12, x12, x14
100b23aa0:     	bic	x9, x15, x9
100b23aa4:     	orr	x9, x12, x9
100b23aa8:     	add	x11, x13, x11, lsl #4
100b23aac:     	stp	x10, x8, [sp, #0xc0]
100b23ab0:     	b	0x100b23b10 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x150>
100b23ab4:     	ldr	x8, [x28, #0x48]
100b23ab8:     	cmp	x8, x0
100b23abc:     	b.ls	0x100b24254 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x894>
100b23ac0:     	ldr	w10, [x24, #0x28]
100b23ac4:     	str	x10, [sp, #0xc8]
100b23ac8:     	lsr	x11, x10, #1
100b23acc:     	cmp	x8, x11
100b23ad0:     	b.ls	0x100b2423c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x87c>
100b23ad4:     	ldr	w10, [x24, #0x40]
100b23ad8:     	str	x10, [sp, #0xc0]
100b23adc:     	lsr	x10, x10, #1
100b23ae0:     	cmp	x8, x10
100b23ae4:     	b.ls	0x100b24250 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x890>
100b23ae8:     	add	x8, x1, x11, lsl #5
100b23aec:     	ldr	x8, [x8, #0x18]
100b23af0:     	ldr	x11, [x24, #0x18]
100b23af4:     	bic	x8, x8, x11
100b23af8:     	add	x11, x1, x0, lsl #5
100b23afc:     	ldr	x11, [x11, #0x18]
100b23b00:     	bic	x9, x11, x9
100b23b04:     	orr	x9, x8, x9
100b23b08:     	add	x8, x1, x10, lsl #5
100b23b0c:     	add	x11, x8, #0x18
100b23b10:     	ldr	x8, [x11]
100b23b14:     	mov	x19, x24
100b23b18:     	ldr	x10, [x19, #0x30]!
100b23b1c:     	bic	x8, x8, x10
100b23b20:     	orr	x20, x8, x9
100b23b24:     	fmov	d0, x20
100b23b28:     	cnt.8b	v0, v0
100b23b2c:     	addv.8b	b0, v0
100b23b30:     	fmov	x8, d0
100b23b34:     	cmp	x8, #0x7
100b23b38:     	str	x28, [sp, #0xd8]
100b23b3c:     	b.hs	0x100b23bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x1f4>
100b23b40:     	str	x24, [sp, #0x18]
100b23b44:     	ldr	x8, [x21, #0x10]
100b23b48:     	add	x8, x8, #0x1
100b23b4c:     	str	x8, [x21, #0x10]
100b23b50:     	mov	w26, #0x4               ; =4
100b23b54:     	stp	xzr, x26, [x29, #-0xc0]
100b23b58:     	stur	xzr, [x29, #-0xb0]
100b23b5c:     	mov	w24, #0x1               ; =1
100b23b60:     	str	x23, [sp, #0x8]
100b23b64:     	str	x21, [sp, #0xd0]
100b23b68:     	mov	x22, #0x0               ; =0
100b23b6c:     	cbz	x20, 0x100b23e14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x454>
100b23b70:     	mov	w8, #0x4                ; =4
100b23b74:     	b	0x100b23b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x1dc>
100b23b78:     	ldur	x8, [x29, #-0xb8]
100b23b7c:     	rbit	x9, x20
100b23b80:     	clz	x9, x9
100b23b84:     	str	w9, [x8, x22, lsl #2]
100b23b88:     	add	x22, x22, #0x1
100b23b8c:     	stur	x22, [x29, #-0xb0]
100b23b90:     	sub	x9, x20, #0x1
100b23b94:     	ands	x20, x9, x20
100b23b98:     	b.eq	0x100b23ce8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x328>
100b23b9c:     	ldur	x9, [x29, #-0xc0]
100b23ba0:     	cmp	x22, x9
100b23ba4:     	b.ne	0x100b23b7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x1bc>
100b23ba8:     	sub	x0, x29, #0xc0
100b23bac:     	bl	0x10128921c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b23bb0:     	b	0x100b23b78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x1b8>
100b23bb4:     	mov	x28, x21
100b23bb8:     	mov	x9, #0x0                ; =0
100b23bbc:     	sub	x25, x29, #0xf8
100b23bc0:     	lsl	x10, x26, #2
100b23bc4:     	cmp	x10, x9
100b23bc8:     	b.eq	0x100b24210 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x850>
100b23bcc:     	ldr	w8, [x27, x9]
100b23bd0:     	lsr	x11, x20, x8
100b23bd4:     	add	x9, x9, #0x4
100b23bd8:     	tbz	w11, #0x0, 0x100b23bc4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x204>
100b23bdc:     	ldr	q0, [x24]
100b23be0:     	stur	q0, [x29, #-0xc0]
100b23be4:     	ldr	x9, [x24, #0x10]
100b23be8:     	stur	x9, [x29, #-0xb0]
100b23bec:     	sub	x0, x29, #0xf8
100b23bf0:     	sub	x1, x29, #0xc0
100b23bf4:     	ldr	x21, [sp, #0xd8]
100b23bf8:     	mov	x2, x21
100b23bfc:     	mov	x20, x8
100b23c00:     	mov	x3, x20
100b23c04:     	mov	w4, #0x0                ; =0
100b23c08:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23c0c:     	ldr	q0, [x25]
100b23c10:     	ldur	x8, [x29, #-0xe8]
100b23c14:     	str	x8, [sp, #0x1a0]
100b23c18:     	stur	q0, [x29, #-0xe0]
100b23c1c:     	stur	x8, [x29, #-0xd0]
100b23c20:     	str	q0, [sp, #0xf0]
100b23c24:     	str	x8, [sp, #0x100]
100b23c28:     	ldur	q0, [x24, #0x18]
100b23c2c:     	stur	q0, [x29, #-0xc0]
100b23c30:     	ldr	x8, [x24, #0x28]
100b23c34:     	stur	x8, [x29, #-0xb0]
100b23c38:     	sub	x0, x29, #0xf8
100b23c3c:     	sub	x1, x29, #0xc0
100b23c40:     	mov	x2, x21
100b23c44:     	mov	x3, x20
100b23c48:     	mov	w4, #0x0                ; =0
100b23c4c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23c50:     	ldr	q0, [x25]
100b23c54:     	ldur	x8, [x29, #-0xe8]
100b23c58:     	str	x8, [sp, #0x1a0]
100b23c5c:     	stur	q0, [x29, #-0xe0]
100b23c60:     	stur	x8, [x29, #-0xd0]
100b23c64:     	add	x9, sp, #0x9
100b23c68:     	stur	q0, [x9, #0xff]
100b23c6c:     	str	x8, [sp, #0x118]
100b23c70:     	ldr	q0, [x19]
100b23c74:     	stur	q0, [x29, #-0xc0]
100b23c78:     	ldr	x8, [x19, #0x10]
100b23c7c:     	stur	x8, [x29, #-0xb0]
100b23c80:     	sub	x0, x29, #0xf8
100b23c84:     	sub	x1, x29, #0xc0
100b23c88:     	mov	x2, x21
100b23c8c:     	mov	x22, x20
100b23c90:     	mov	x3, x20
100b23c94:     	mov	w4, #0x0                ; =0
100b23c98:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23c9c:     	ldr	q0, [x25]
100b23ca0:     	ldur	x8, [x29, #-0xe8]
100b23ca4:     	str	x8, [sp, #0x1a0]
100b23ca8:     	stur	q0, [x29, #-0xe0]
100b23cac:     	str	q0, [sp, #0x120]
100b23cb0:     	str	x8, [sp, #0x130]
100b23cb4:     	add	x3, sp, #0xf0
100b23cb8:     	mov	x0, x21
100b23cbc:     	mov	x1, x27
100b23cc0:     	mov	x2, x26
100b23cc4:     	mov	x4, x23
100b23cc8:     	mov	x5, x28
100b23ccc:     	bl	0x100b239c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_>
100b23cd0:     	mov	x25, x23
100b23cd4:     	and	w8, w0, #0xff
100b23cd8:     	cmp	w8, #0xf
100b23cdc:     	b.ne	0x100b23cf8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x338>
100b23ce0:     	mov	w27, #0xf               ; =15
100b23ce4:     	b	0x100b23e0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x44c>
100b23ce8:     	ldp	x8, x26, [x29, #-0xc0]
100b23cec:     	cmp	x8, #0x0
100b23cf0:     	cset	w8, eq
100b23cf4:     	b	0x100b23e18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x458>
100b23cf8:     	mov	x23, x0
100b23cfc:     	ldr	q0, [x24]
100b23d00:     	stur	q0, [x29, #-0xc0]
100b23d04:     	ldr	x8, [x24, #0x10]
100b23d08:     	stur	x8, [x29, #-0xb0]
100b23d0c:     	sub	x0, x29, #0xf8
100b23d10:     	sub	x1, x29, #0xc0
100b23d14:     	mov	x2, x21
100b23d18:     	mov	x20, x22
100b23d1c:     	mov	x3, x20
100b23d20:     	mov	w4, #0x1                ; =1
100b23d24:     	sub	x22, x29, #0xf8
100b23d28:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23d2c:     	ldr	q0, [x22]
100b23d30:     	str	q0, [sp, #0x1b0]
100b23d34:     	ldur	x8, [x29, #-0xe8]
100b23d38:     	str	q0, [sp, #0x190]
100b23d3c:     	stur	q0, [x29, #-0xe0]
100b23d40:     	stur	x8, [x29, #-0xd0]
100b23d44:     	ldur	q0, [x29, #-0xe0]
100b23d48:     	str	x8, [sp, #0x150]
100b23d4c:     	str	q0, [sp, #0x140]
100b23d50:     	ldur	q0, [x24, #0x18]
100b23d54:     	stur	q0, [x29, #-0xc0]
100b23d58:     	ldur	x8, [x24, #0x28]
100b23d5c:     	stur	x8, [x29, #-0xb0]
100b23d60:     	sub	x0, x29, #0xf8
100b23d64:     	sub	x1, x29, #0xc0
100b23d68:     	mov	x2, x21
100b23d6c:     	mov	x3, x20
100b23d70:     	mov	w4, #0x1                ; =1
100b23d74:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23d78:     	ldr	q0, [x22]
100b23d7c:     	str	q0, [sp, #0x1b0]
100b23d80:     	ldur	x8, [x29, #-0xe8]
100b23d84:     	str	q0, [sp, #0x190]
100b23d88:     	stur	q0, [x29, #-0xe0]
100b23d8c:     	stur	x8, [x29, #-0xd0]
100b23d90:     	ldur	q0, [x29, #-0xe0]
100b23d94:     	str	x8, [sp, #0x168]
100b23d98:     	add	x8, sp, #0x59
100b23d9c:     	stur	q0, [x8, #0xff]
100b23da0:     	ldr	q0, [x19]
100b23da4:     	stur	q0, [x29, #-0xc0]
100b23da8:     	ldr	x8, [x19, #0x10]
100b23dac:     	stur	x8, [x29, #-0xb0]
100b23db0:     	sub	x0, x29, #0xf8
100b23db4:     	sub	x1, x29, #0xc0
100b23db8:     	mov	x2, x21
100b23dbc:     	mov	x3, x20
100b23dc0:     	mov	w4, #0x1                ; =1
100b23dc4:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23dc8:     	ldr	q0, [x22]
100b23dcc:     	str	q0, [sp, #0x1b0]
100b23dd0:     	ldur	x8, [x29, #-0xe8]
100b23dd4:     	str	q0, [sp, #0x190]
100b23dd8:     	stur	q0, [x29, #-0xe0]
100b23ddc:     	stur	x8, [x29, #-0xd0]
100b23de0:     	ldur	q0, [x29, #-0xe0]
100b23de4:     	str	x8, [sp, #0x180]
100b23de8:     	str	q0, [sp, #0x170]
100b23dec:     	add	x3, sp, #0x140
100b23df0:     	mov	x0, x21
100b23df4:     	mov	x1, x27
100b23df8:     	mov	x2, x26
100b23dfc:     	mov	x4, x25
100b23e00:     	mov	x5, x28
100b23e04:     	bl	0x100b239c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_>
100b23e08:     	orr	w27, w0, w23
100b23e0c:     	mov	x0, x25
100b23e10:     	b	0x100b241d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x810>
100b23e14:     	mov	w8, #0x1                ; =1
100b23e18:     	str	w8, [sp, #0x14]
100b23e1c:     	mov	w27, #0x0               ; =0
100b23e20:     	mov	x21, #0x0               ; =0
100b23e24:     	ldr	x8, [sp, #0x18]
100b23e28:     	ldr	x10, [x8, #0x8]
100b23e2c:     	ldr	x9, [x8, #0x20]
100b23e30:     	stp	x9, x10, [sp, #0xb0]
100b23e34:     	ldr	x8, [x8, #0x38]
100b23e38:     	str	x8, [sp, #0xa8]
100b23e3c:     	ldr	x8, [sp, #0xd0]
100b23e40:     	ldr	x8, [x8, #0x8]
100b23e44:     	str	x8, [sp, #0xe8]
100b23e48:     	and	x8, x22, #0xfffffffffffffffe
100b23e4c:     	neg	x8, x8
100b23e50:     	str	x8, [sp, #0x88]
100b23e54:     	mov	w23, #0x2               ; =2
100b23e58:     	adrp	x8, 0x101322000 <GCC_except_table9287>
100b23e5c:     	ldr	q0, [x8, #0x740]
100b23e60:     	str	q0, [sp, #0x90]
100b23e64:     	mov	w8, #0x4                ; =4
100b23e68:     	dup.2d	v1, x8
100b23e6c:     	mov	w8, #0x8                ; =8
100b23e70:     	dup.2d	v0, x8
100b23e74:     	stp	q0, q1, [sp, #0x50]
100b23e78:     	mov	w8, #0xc                ; =12
100b23e7c:     	dup.2d	v1, x8
100b23e80:     	mov	w8, #0x10               ; =16
100b23e84:     	dup.2d	v0, x8
100b23e88:     	stp	q0, q1, [sp, #0x30]
100b23e8c:     	adrp	x8, 0x101322000 <GCC_except_table9287>
100b23e90:     	ldr	q0, [x8, #0x760]
100b23e94:     	str	q0, [sp, #0x20]
100b23e98:     	mov	w8, #0x3f               ; =63
100b23e9c:     	dup.2d	v0, x8
100b23ea0:     	str	q0, [sp, #0x70]
100b23ea4:     	movi.2s	v8, #0x3f
100b23ea8:     	b	0x100b23ec0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x500>
100b23eac:     	add	x21, x21, #0x1
100b23eb0:     	and	x8, x22, #0x3f
100b23eb4:     	lsr	x8, x21, x8
100b23eb8:     	ldr	x28, [sp, #0xd8]
100b23ebc:     	cbnz	x8, 0x100b241b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x7f8>
100b23ec0:     	ldr	x9, [sp, #0xe8]
100b23ec4:     	add	x9, x9, #0x1
100b23ec8:     	ldr	x8, [sp, #0xd0]
100b23ecc:     	str	x9, [sp, #0xe8]
100b23ed0:     	str	x9, [x8, #0x8]
100b23ed4:     	mov	x25, x22
100b23ed8:     	cbz	x22, 0x100b2414c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x78c>
100b23edc:     	cmp	x22, #0x1
100b23ee0:     	b.ne	0x100b23ef0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x530>
100b23ee4:     	mov	x8, #0x0                ; =0
100b23ee8:     	mov	x25, #0x0               ; =0
100b23eec:     	b	0x100b2412c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x76c>
100b23ef0:     	dup.2d	v0, x21
100b23ef4:     	cmp	x22, #0x10
100b23ef8:     	b.hs	0x100b23f08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x548>
100b23efc:     	mov	x9, #0x0                ; =0
100b23f00:     	mov	x25, #0x0               ; =0
100b23f04:     	b	0x100b240b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x6f8>
100b23f08:     	movi.2d	v1, #0000000000000000
100b23f0c:     	add	x8, x26, #0x20
100b23f10:     	movi.2d	v2, #0000000000000000
100b23f14:     	and	x9, x22, #0x1ffffffffffffff0
100b23f18:     	ldr	q4, [sp, #0x90]
100b23f1c:     	ldp	q6, q15, [sp, #0x20]
100b23f20:     	movi.2d	v3, #0000000000000000
100b23f24:     	movi.2d	v7, #0000000000000000
100b23f28:     	movi.2d	v16, #0000000000000000
100b23f2c:     	movi.2d	v5, #0000000000000000
100b23f30:     	movi.2d	v18, #0000000000000000
100b23f34:     	movi.2d	v17, #0000000000000000
100b23f38:     	ldp	q13, q12, [sp, #0x50]
100b23f3c:     	ldr	q14, [sp, #0x40]
100b23f40:     	mov	w10, #0x3f              ; =63
100b23f44:     	movi.4s	v8, #0x3f
100b23f48:     	add.2d	v19, v4, v12
100b23f4c:     	add.2d	v20, v6, v12
100b23f50:     	add.2d	v21, v4, v13
100b23f54:     	add.2d	v22, v6, v13
100b23f58:     	add.2d	v23, v4, v14
100b23f5c:     	add.2d	v24, v6, v14
100b23f60:     	ldp	q25, q26, [x8, #-0x20]
100b23f64:     	dup.2d	v27, x10
100b23f68:     	ldp	q28, q29, [x8], #0x40
100b23f6c:     	and.16b	v30, v6, v27
100b23f70:     	and.16b	v31, v4, v27
100b23f74:     	and.16b	v20, v20, v27
100b23f78:     	and.16b	v19, v19, v27
100b23f7c:     	and.16b	v22, v22, v27
100b23f80:     	and.16b	v21, v21, v27
100b23f84:     	and.16b	v24, v24, v27
100b23f88:     	and.16b	v23, v23, v27
100b23f8c:     	neg.2d	v27, v31
100b23f90:     	ushl.2d	v27, v0, v27
100b23f94:     	neg.2d	v30, v30
100b23f98:     	ushl.2d	v30, v0, v30
100b23f9c:     	neg.2d	v19, v19
100b23fa0:     	ushl.2d	v19, v0, v19
100b23fa4:     	neg.2d	v20, v20
100b23fa8:     	ushl.2d	v20, v0, v20
100b23fac:     	neg.2d	v21, v21
100b23fb0:     	ushl.2d	v21, v0, v21
100b23fb4:     	neg.2d	v22, v22
100b23fb8:     	ushl.2d	v22, v0, v22
100b23fbc:     	neg.2d	v23, v23
100b23fc0:     	ushl.2d	v23, v0, v23
100b23fc4:     	neg.2d	v24, v24
100b23fc8:     	ushl.2d	v24, v0, v24
100b23fcc:     	dup.2d	v31, x24
100b23fd0:     	and.16b	v30, v30, v31
100b23fd4:     	and.16b	v27, v27, v31
100b23fd8:     	and.16b	v20, v20, v31
100b23fdc:     	and.16b	v19, v19, v31
100b23fe0:     	and.16b	v22, v22, v31
100b23fe4:     	and.16b	v21, v21, v31
100b23fe8:     	and.16b	v24, v24, v31
100b23fec:     	and.16b	v23, v23, v31
100b23ff0:     	and.16b	v25, v25, v8
100b23ff4:     	and.16b	v26, v26, v8
100b23ff8:     	and.16b	v28, v28, v8
100b23ffc:     	and.16b	v29, v29, v8
100b24000:     	ushll2.2d	v31, v25, #0x0
100b24004:     	ushll.2d	v25, v25, #0x0
100b24008:     	ushll2.2d	v9, v26, #0x0
100b2400c:     	ushll.2d	v26, v26, #0x0
100b24010:     	ushll2.2d	v10, v28, #0x0
100b24014:     	ushll.2d	v28, v28, #0x0
100b24018:     	ushll2.2d	v11, v29, #0x0
100b2401c:     	ushll.2d	v29, v29, #0x0
100b24020:     	ushl.2d	v25, v27, v25
100b24024:     	ushl.2d	v27, v30, v31
100b24028:     	ushl.2d	v19, v19, v26
100b2402c:     	ushl.2d	v20, v20, v9
100b24030:     	ushl.2d	v21, v21, v28
100b24034:     	ushl.2d	v22, v22, v10
100b24038:     	ushl.2d	v23, v23, v29
100b2403c:     	ushl.2d	v24, v24, v11
100b24040:     	orr.16b	v3, v27, v3
100b24044:     	orr.16b	v2, v25, v2
100b24048:     	orr.16b	v16, v20, v16
100b2404c:     	orr.16b	v7, v19, v7
100b24050:     	orr.16b	v18, v22, v18
100b24054:     	orr.16b	v5, v21, v5
100b24058:     	orr.16b	v1, v24, v1
100b2405c:     	orr.16b	v17, v23, v17
100b24060:     	add.2d	v6, v6, v15
100b24064:     	add.2d	v4, v4, v15
100b24068:     	subs	x9, x9, #0x10
100b2406c:     	b.ne	0x100b23f48 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x588>
100b24070:     	orr.16b	v2, v7, v2
100b24074:     	orr.16b	v3, v16, v3
100b24078:     	orr.16b	v3, v18, v3
100b2407c:     	orr.16b	v2, v5, v2
100b24080:     	orr.16b	v2, v17, v2
100b24084:     	orr.16b	v1, v1, v3
100b24088:     	orr.16b	v1, v2, v1
100b2408c:     	mov	d2, v1[1]
100b24090:     	orr.8b	v1, v1, v2
100b24094:     	fmov	x25, d1
100b24098:     	and	x8, x22, #0x1ffffffffffffff0
100b2409c:     	cmp	x22, x8
100b240a0:     	movi.2s	v8, #0x3f
100b240a4:     	b.eq	0x100b2414c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x78c>
100b240a8:     	and	x9, x22, #0x1ffffffffffffff0
100b240ac:     	and	x8, x22, #0x1ffffffffffffff0
100b240b0:     	and	x10, x22, #0xe
100b240b4:     	cbz	x10, 0x100b2412c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x76c>
100b240b8:     	fmov	d1, x25
100b240bc:     	dup.2d	v2, x9
100b240c0:     	ldr	q3, [sp, #0x90]
100b240c4:     	orr.16b	v2, v2, v3
100b240c8:     	ldr	x8, [sp, #0x88]
100b240cc:     	add	x8, x8, x9
100b240d0:     	add	x9, x26, x9, lsl #2
100b240d4:     	ldr	q6, [sp, #0x70]
100b240d8:     	ldr	d3, [x9], #0x8
100b240dc:     	and.16b	v4, v2, v6
100b240e0:     	neg.2d	v4, v4
100b240e4:     	ushl.2d	v4, v0, v4
100b240e8:     	dup.2d	v5, x24
100b240ec:     	and.16b	v4, v4, v5
100b240f0:     	and.8b	v3, v3, v8
100b240f4:     	ushll.2d	v3, v3, #0x0
100b240f8:     	ushl.2d	v3, v4, v3
100b240fc:     	orr.16b	v1, v3, v1
100b24100:     	dup.2d	v3, x23
100b24104:     	add.2d	v2, v2, v3
100b24108:     	adds	x8, x8, #0x2
100b2410c:     	b.ne	0x100b240d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x718>
100b24110:     	mov	d0, v1[1]
100b24114:     	orr.8b	v0, v1, v0
100b24118:     	fmov	x25, d0
100b2411c:     	and	x8, x22, #0x1ffffffffffffffe
100b24120:     	and	x9, x22, #0x1ffffffffffffffe
100b24124:     	cmp	x22, x9
100b24128:     	b.eq	0x100b2414c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x78c>
100b2412c:     	ldr	w9, [x26, x8, lsl #2]
100b24130:     	lsr	x10, x21, x8
100b24134:     	and	x10, x10, #0x1
100b24138:     	lsl	x9, x10, x9
100b2413c:     	orr	x25, x9, x25
100b24140:     	add	x8, x8, #0x1
100b24144:     	cmp	x22, x8
100b24148:     	b.ne	0x100b2412c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x76c>
100b2414c:     	ldr	x8, [sp, #0xb8]
100b24150:     	orr	x2, x8, x25
100b24154:     	mov	x0, x28
100b24158:     	ldr	x1, [sp, #0xe0]
100b2415c:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b24160:     	mov	x19, x0
100b24164:     	ldr	x8, [sp, #0xb0]
100b24168:     	orr	x2, x8, x25
100b2416c:     	mov	x0, x28
100b24170:     	ldr	x1, [sp, #0xc8]
100b24174:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b24178:     	mov	x20, x0
100b2417c:     	ldr	x8, [sp, #0xa8]
100b24180:     	orr	x2, x8, x25
100b24184:     	mov	x0, x28
100b24188:     	ldr	x1, [sp, #0xc0]
100b2418c:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b24190:     	tbz	w19, #0x0, 0x100b23eac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x4ec>
100b24194:     	cmp	w20, #0x0
100b24198:     	csel	w8, w23, wzr, ne
100b2419c:     	orr	w8, w8, w0
100b241a0:     	lsl	w8, w24, w8
100b241a4:     	orr	w27, w8, w27
100b241a8:     	and	w8, w27, #0xff
100b241ac:     	cmp	w8, #0xf
100b241b0:     	b.ne	0x100b23eac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x4ec>
100b241b4:     	mov	w27, #0xf               ; =15
100b241b8:     	ldr	w8, [sp, #0x14]
100b241bc:     	tbnz	w8, #0x0, 0x100b241c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x808>
100b241c0:     	mov	x0, x26
100b241c4:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b241c8:     	ldr	x24, [sp, #0x18]
100b241cc:     	ldr	x0, [sp, #0x8]
100b241d0:     	mov	x1, x24
100b241d4:     	mov	x2, x27
100b241d8:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b241dc:     	mov	x0, x27
100b241e0:     	add	sp, sp, #0x230
100b241e4:     	ldp	x29, x30, [sp, #0x90]
100b241e8:     	ldp	x20, x19, [sp, #0x80]
100b241ec:     	ldp	x22, x21, [sp, #0x70]
100b241f0:     	ldp	x24, x23, [sp, #0x60]
100b241f4:     	ldp	x26, x25, [sp, #0x50]
100b241f8:     	ldp	x28, x27, [sp, #0x40]
100b241fc:     	ldp	d9, d8, [sp, #0x30]
100b24200:     	ldp	d11, d10, [sp, #0x20]
100b24204:     	ldp	d13, d12, [sp, #0x10]
100b24208:     	ldp	d15, d14, [sp], #0xa0
100b2420c:     	ret
100b24210:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b24214:     	add	x0, x0, #0xd90
100b24218:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b2421c:     	mov	x0, x12
100b24220:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24224:     	add	x2, x2, #0x6c0
100b24228:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2422c:     	mov	x0, x11
100b24230:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24234:     	add	x2, x2, #0x6c0
100b24238:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2423c:     	mov	x0, x11
100b24240:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24244:     	add	x2, x2, #0x6a8
100b24248:     	mov	x1, x8
100b2424c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24250:     	mov	x0, x10
100b24254:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24258:     	add	x2, x2, #0x6a8
100b2425c:     	mov	x1, x8
100b24260:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24264:     	mov	x19, x0
100b24268:     	ldur	x8, [x29, #-0xc0]
100b2426c:     	cbz	x8, 0x100b2428c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x8cc>
100b24270:     	ldur	x26, [x29, #-0xb8]
100b24274:     	b	0x100b24284 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x8c4>
100b24278:     	mov	x19, x0
100b2427c:     	ldr	w8, [sp, #0x14]
100b24280:     	tbnz	w8, #0x0, 0x100b2428c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_+0x8cc>
100b24284:     	mov	x0, x26
100b24288:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b2428c:     	mov	x0, x19
100b24290:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
