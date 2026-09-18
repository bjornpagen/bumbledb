
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b219c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_>:
100b219c0:     	stp	d15, d14, [sp, #-0xa0]!
100b219c4:     	stp	d13, d12, [sp, #0x10]
100b219c8:     	stp	d11, d10, [sp, #0x20]
100b219cc:     	stp	d9, d8, [sp, #0x30]
100b219d0:     	stp	x28, x27, [sp, #0x40]
100b219d4:     	stp	x26, x25, [sp, #0x50]
100b219d8:     	stp	x24, x23, [sp, #0x60]
100b219dc:     	stp	x22, x21, [sp, #0x70]
100b219e0:     	stp	x20, x19, [sp, #0x80]
100b219e4:     	stp	x29, x30, [sp, #0x90]
100b219e8:     	add	x29, sp, #0x90
100b219ec:     	sub	sp, sp, #0x230
100b219f0:     	ldr	w8, [x3, #0x10]
100b219f4:     	str	x8, [sp, #0xe0]
100b219f8:     	cbz	w8, 0x100b21a2c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x6c>
100b219fc:     	mov	x21, x5
100b21a00:     	mov	x23, x4
100b21a04:     	mov	x24, x3
100b21a08:     	mov	x26, x2
100b21a0c:     	mov	x27, x1
100b21a10:     	mov	x28, x0
100b21a14:     	mov	x0, x4
100b21a18:     	mov	x1, x3
100b21a1c:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b21a20:     	cbz	x0, 0x100b21a34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x74>
100b21a24:     	ldrb	w27, [x0]
100b21a28:     	b	0x100b221dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x81c>
100b21a2c:     	mov	w27, #0x0               ; =0
100b21a30:     	b	0x100b221dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x81c>
100b21a34:     	ldr	x8, [x21]
100b21a38:     	add	x8, x8, #0x1
100b21a3c:     	str	x8, [x21]
100b21a40:     	ldr	x8, [x28, #0x30]
100b21a44:     	ldr	x1, [x28, #0x40]
100b21a48:     	ldr	x9, [x24]
100b21a4c:     	ldr	x10, [sp, #0xe0]
100b21a50:     	lsr	x0, x10, #1
100b21a54:     	cmn	x8, #0x1
100b21a58:     	b.eq	0x100b21ab4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0xf4>
100b21a5c:     	cmp	x1, x0
100b21a60:     	b.ls	0x100b22230 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x870>
100b21a64:     	ldr	w8, [x24, #0x28]
100b21a68:     	lsr	x12, x8, #1
100b21a6c:     	cmp	x1, x12
100b21a70:     	b.ls	0x100b2221c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x85c>
100b21a74:     	ldr	w10, [x24, #0x40]
100b21a78:     	lsr	x11, x10, #1
100b21a7c:     	cmp	x1, x11
100b21a80:     	b.ls	0x100b2222c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x86c>
100b21a84:     	ldr	x13, [x28, #0x38]
100b21a88:     	lsl	x12, x12, #4
100b21a8c:     	ldr	x12, [x13, x12]
100b21a90:     	ldr	x14, [x24, #0x18]
100b21a94:     	lsl	x15, x0, #4
100b21a98:     	ldr	x15, [x13, x15]
100b21a9c:     	bic	x12, x12, x14
100b21aa0:     	bic	x9, x15, x9
100b21aa4:     	orr	x9, x12, x9
100b21aa8:     	add	x11, x13, x11, lsl #4
100b21aac:     	stp	x10, x8, [sp, #0xc0]
100b21ab0:     	b	0x100b21b10 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x150>
100b21ab4:     	ldr	x8, [x28, #0x48]
100b21ab8:     	cmp	x8, x0
100b21abc:     	b.ls	0x100b22254 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x894>
100b21ac0:     	ldr	w10, [x24, #0x28]
100b21ac4:     	str	x10, [sp, #0xc8]
100b21ac8:     	lsr	x11, x10, #1
100b21acc:     	cmp	x8, x11
100b21ad0:     	b.ls	0x100b2223c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x87c>
100b21ad4:     	ldr	w10, [x24, #0x40]
100b21ad8:     	str	x10, [sp, #0xc0]
100b21adc:     	lsr	x10, x10, #1
100b21ae0:     	cmp	x8, x10
100b21ae4:     	b.ls	0x100b22250 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x890>
100b21ae8:     	add	x8, x1, x11, lsl #5
100b21aec:     	ldr	x8, [x8, #0x18]
100b21af0:     	ldr	x11, [x24, #0x18]
100b21af4:     	bic	x8, x8, x11
100b21af8:     	add	x11, x1, x0, lsl #5
100b21afc:     	ldr	x11, [x11, #0x18]
100b21b00:     	bic	x9, x11, x9
100b21b04:     	orr	x9, x8, x9
100b21b08:     	add	x8, x1, x10, lsl #5
100b21b0c:     	add	x11, x8, #0x18
100b21b10:     	ldr	x8, [x11]
100b21b14:     	mov	x19, x24
100b21b18:     	ldr	x10, [x19, #0x30]!
100b21b1c:     	bic	x8, x8, x10
100b21b20:     	orr	x20, x8, x9
100b21b24:     	fmov	d0, x20
100b21b28:     	cnt.8b	v0, v0
100b21b2c:     	addv.8b	b0, v0
100b21b30:     	fmov	x8, d0
100b21b34:     	cmp	x8, #0x7
100b21b38:     	str	x28, [sp, #0xd8]
100b21b3c:     	b.hs	0x100b21bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x1f4>
100b21b40:     	str	x24, [sp, #0x18]
100b21b44:     	ldr	x8, [x21, #0x10]
100b21b48:     	add	x8, x8, #0x1
100b21b4c:     	str	x8, [x21, #0x10]
100b21b50:     	mov	w26, #0x4               ; =4
100b21b54:     	stp	xzr, x26, [x29, #-0xc0]
100b21b58:     	stur	xzr, [x29, #-0xb0]
100b21b5c:     	mov	w24, #0x1               ; =1
100b21b60:     	str	x23, [sp, #0x8]
100b21b64:     	str	x21, [sp, #0xd0]
100b21b68:     	mov	x22, #0x0               ; =0
100b21b6c:     	cbz	x20, 0x100b21e14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x454>
100b21b70:     	mov	w8, #0x4                ; =4
100b21b74:     	b	0x100b21b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x1dc>
100b21b78:     	ldur	x8, [x29, #-0xb8]
100b21b7c:     	rbit	x9, x20
100b21b80:     	clz	x9, x9
100b21b84:     	str	w9, [x8, x22, lsl #2]
100b21b88:     	add	x22, x22, #0x1
100b21b8c:     	stur	x22, [x29, #-0xb0]
100b21b90:     	sub	x9, x20, #0x1
100b21b94:     	ands	x20, x9, x20
100b21b98:     	b.eq	0x100b21ce8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x328>
100b21b9c:     	ldur	x9, [x29, #-0xc0]
100b21ba0:     	cmp	x22, x9
100b21ba4:     	b.ne	0x100b21b7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x1bc>
100b21ba8:     	sub	x0, x29, #0xc0
100b21bac:     	bl	0x101282ddc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b21bb0:     	b	0x100b21b78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x1b8>
100b21bb4:     	mov	x28, x21
100b21bb8:     	mov	x9, #0x0                ; =0
100b21bbc:     	sub	x25, x29, #0xf8
100b21bc0:     	lsl	x10, x26, #2
100b21bc4:     	cmp	x10, x9
100b21bc8:     	b.eq	0x100b22210 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x850>
100b21bcc:     	ldr	w8, [x27, x9]
100b21bd0:     	lsr	x11, x20, x8
100b21bd4:     	add	x9, x9, #0x4
100b21bd8:     	tbz	w11, #0x0, 0x100b21bc4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x204>
100b21bdc:     	ldr	q0, [x24]
100b21be0:     	stur	q0, [x29, #-0xc0]
100b21be4:     	ldr	x9, [x24, #0x10]
100b21be8:     	stur	x9, [x29, #-0xb0]
100b21bec:     	sub	x0, x29, #0xf8
100b21bf0:     	sub	x1, x29, #0xc0
100b21bf4:     	ldr	x21, [sp, #0xd8]
100b21bf8:     	mov	x2, x21
100b21bfc:     	mov	x20, x8
100b21c00:     	mov	x3, x20
100b21c04:     	mov	w4, #0x0                ; =0
100b21c08:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21c0c:     	ldr	q0, [x25]
100b21c10:     	ldur	x8, [x29, #-0xe8]
100b21c14:     	str	x8, [sp, #0x1a0]
100b21c18:     	stur	q0, [x29, #-0xe0]
100b21c1c:     	stur	x8, [x29, #-0xd0]
100b21c20:     	str	q0, [sp, #0xf0]
100b21c24:     	str	x8, [sp, #0x100]
100b21c28:     	ldur	q0, [x24, #0x18]
100b21c2c:     	stur	q0, [x29, #-0xc0]
100b21c30:     	ldr	x8, [x24, #0x28]
100b21c34:     	stur	x8, [x29, #-0xb0]
100b21c38:     	sub	x0, x29, #0xf8
100b21c3c:     	sub	x1, x29, #0xc0
100b21c40:     	mov	x2, x21
100b21c44:     	mov	x3, x20
100b21c48:     	mov	w4, #0x0                ; =0
100b21c4c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21c50:     	ldr	q0, [x25]
100b21c54:     	ldur	x8, [x29, #-0xe8]
100b21c58:     	str	x8, [sp, #0x1a0]
100b21c5c:     	stur	q0, [x29, #-0xe0]
100b21c60:     	stur	x8, [x29, #-0xd0]
100b21c64:     	add	x9, sp, #0x9
100b21c68:     	stur	q0, [x9, #0xff]
100b21c6c:     	str	x8, [sp, #0x118]
100b21c70:     	ldr	q0, [x19]
100b21c74:     	stur	q0, [x29, #-0xc0]
100b21c78:     	ldr	x8, [x19, #0x10]
100b21c7c:     	stur	x8, [x29, #-0xb0]
100b21c80:     	sub	x0, x29, #0xf8
100b21c84:     	sub	x1, x29, #0xc0
100b21c88:     	mov	x2, x21
100b21c8c:     	mov	x22, x20
100b21c90:     	mov	x3, x20
100b21c94:     	mov	w4, #0x0                ; =0
100b21c98:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21c9c:     	ldr	q0, [x25]
100b21ca0:     	ldur	x8, [x29, #-0xe8]
100b21ca4:     	str	x8, [sp, #0x1a0]
100b21ca8:     	stur	q0, [x29, #-0xe0]
100b21cac:     	str	q0, [sp, #0x120]
100b21cb0:     	str	x8, [sp, #0x130]
100b21cb4:     	add	x3, sp, #0xf0
100b21cb8:     	mov	x0, x21
100b21cbc:     	mov	x1, x27
100b21cc0:     	mov	x2, x26
100b21cc4:     	mov	x4, x23
100b21cc8:     	mov	x5, x28
100b21ccc:     	bl	0x100b219c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_>
100b21cd0:     	mov	x25, x23
100b21cd4:     	and	w8, w0, #0xff
100b21cd8:     	cmp	w8, #0xf
100b21cdc:     	b.ne	0x100b21cf8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x338>
100b21ce0:     	mov	w27, #0xf               ; =15
100b21ce4:     	b	0x100b21e0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x44c>
100b21ce8:     	ldp	x8, x26, [x29, #-0xc0]
100b21cec:     	cmp	x8, #0x0
100b21cf0:     	cset	w8, eq
100b21cf4:     	b	0x100b21e18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x458>
100b21cf8:     	mov	x23, x0
100b21cfc:     	ldr	q0, [x24]
100b21d00:     	stur	q0, [x29, #-0xc0]
100b21d04:     	ldr	x8, [x24, #0x10]
100b21d08:     	stur	x8, [x29, #-0xb0]
100b21d0c:     	sub	x0, x29, #0xf8
100b21d10:     	sub	x1, x29, #0xc0
100b21d14:     	mov	x2, x21
100b21d18:     	mov	x20, x22
100b21d1c:     	mov	x3, x20
100b21d20:     	mov	w4, #0x1                ; =1
100b21d24:     	sub	x22, x29, #0xf8
100b21d28:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21d2c:     	ldr	q0, [x22]
100b21d30:     	str	q0, [sp, #0x1b0]
100b21d34:     	ldur	x8, [x29, #-0xe8]
100b21d38:     	str	q0, [sp, #0x190]
100b21d3c:     	stur	q0, [x29, #-0xe0]
100b21d40:     	stur	x8, [x29, #-0xd0]
100b21d44:     	ldur	q0, [x29, #-0xe0]
100b21d48:     	str	x8, [sp, #0x150]
100b21d4c:     	str	q0, [sp, #0x140]
100b21d50:     	ldur	q0, [x24, #0x18]
100b21d54:     	stur	q0, [x29, #-0xc0]
100b21d58:     	ldur	x8, [x24, #0x28]
100b21d5c:     	stur	x8, [x29, #-0xb0]
100b21d60:     	sub	x0, x29, #0xf8
100b21d64:     	sub	x1, x29, #0xc0
100b21d68:     	mov	x2, x21
100b21d6c:     	mov	x3, x20
100b21d70:     	mov	w4, #0x1                ; =1
100b21d74:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21d78:     	ldr	q0, [x22]
100b21d7c:     	str	q0, [sp, #0x1b0]
100b21d80:     	ldur	x8, [x29, #-0xe8]
100b21d84:     	str	q0, [sp, #0x190]
100b21d88:     	stur	q0, [x29, #-0xe0]
100b21d8c:     	stur	x8, [x29, #-0xd0]
100b21d90:     	ldur	q0, [x29, #-0xe0]
100b21d94:     	str	x8, [sp, #0x168]
100b21d98:     	add	x8, sp, #0x59
100b21d9c:     	stur	q0, [x8, #0xff]
100b21da0:     	ldr	q0, [x19]
100b21da4:     	stur	q0, [x29, #-0xc0]
100b21da8:     	ldr	x8, [x19, #0x10]
100b21dac:     	stur	x8, [x29, #-0xb0]
100b21db0:     	sub	x0, x29, #0xf8
100b21db4:     	sub	x1, x29, #0xc0
100b21db8:     	mov	x2, x21
100b21dbc:     	mov	x3, x20
100b21dc0:     	mov	w4, #0x1                ; =1
100b21dc4:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b21dc8:     	ldr	q0, [x22]
100b21dcc:     	str	q0, [sp, #0x1b0]
100b21dd0:     	ldur	x8, [x29, #-0xe8]
100b21dd4:     	str	q0, [sp, #0x190]
100b21dd8:     	stur	q0, [x29, #-0xe0]
100b21ddc:     	stur	x8, [x29, #-0xd0]
100b21de0:     	ldur	q0, [x29, #-0xe0]
100b21de4:     	str	x8, [sp, #0x180]
100b21de8:     	str	q0, [sp, #0x170]
100b21dec:     	add	x3, sp, #0x140
100b21df0:     	mov	x0, x21
100b21df4:     	mov	x1, x27
100b21df8:     	mov	x2, x26
100b21dfc:     	mov	x4, x25
100b21e00:     	mov	x5, x28
100b21e04:     	bl	0x100b219c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_>
100b21e08:     	orr	w27, w0, w23
100b21e0c:     	mov	x0, x25
100b21e10:     	b	0x100b221d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x810>
100b21e14:     	mov	w8, #0x1                ; =1
100b21e18:     	str	w8, [sp, #0x14]
100b21e1c:     	mov	w27, #0x0               ; =0
100b21e20:     	mov	x21, #0x0               ; =0
100b21e24:     	ldr	x8, [sp, #0x18]
100b21e28:     	ldr	x10, [x8, #0x8]
100b21e2c:     	ldr	x9, [x8, #0x20]
100b21e30:     	stp	x9, x10, [sp, #0xb0]
100b21e34:     	ldr	x8, [x8, #0x38]
100b21e38:     	str	x8, [sp, #0xa8]
100b21e3c:     	ldr	x8, [sp, #0xd0]
100b21e40:     	ldr	x8, [x8, #0x8]
100b21e44:     	str	x8, [sp, #0xe8]
100b21e48:     	and	x8, x22, #0xfffffffffffffffe
100b21e4c:     	neg	x8, x8
100b21e50:     	str	x8, [sp, #0x88]
100b21e54:     	mov	w23, #0x2               ; =2
100b21e58:     	adrp	x8, 0x10131c000 <GCC_except_table9305+0x108>
100b21e5c:     	ldr	q0, [x8, #0x240]
100b21e60:     	str	q0, [sp, #0x90]
100b21e64:     	mov	w8, #0x4                ; =4
100b21e68:     	dup.2d	v1, x8
100b21e6c:     	mov	w8, #0x8                ; =8
100b21e70:     	dup.2d	v0, x8
100b21e74:     	stp	q0, q1, [sp, #0x50]
100b21e78:     	mov	w8, #0xc                ; =12
100b21e7c:     	dup.2d	v1, x8
100b21e80:     	mov	w8, #0x10               ; =16
100b21e84:     	dup.2d	v0, x8
100b21e88:     	stp	q0, q1, [sp, #0x30]
100b21e8c:     	adrp	x8, 0x10131c000 <GCC_except_table9305+0x108>
100b21e90:     	ldr	q0, [x8, #0x260]
100b21e94:     	str	q0, [sp, #0x20]
100b21e98:     	mov	w8, #0x3f               ; =63
100b21e9c:     	dup.2d	v0, x8
100b21ea0:     	str	q0, [sp, #0x70]
100b21ea4:     	movi.2s	v8, #0x3f
100b21ea8:     	b	0x100b21ec0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x500>
100b21eac:     	add	x21, x21, #0x1
100b21eb0:     	and	x8, x22, #0x3f
100b21eb4:     	lsr	x8, x21, x8
100b21eb8:     	ldr	x28, [sp, #0xd8]
100b21ebc:     	cbnz	x8, 0x100b221b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x7f8>
100b21ec0:     	ldr	x9, [sp, #0xe8]
100b21ec4:     	add	x9, x9, #0x1
100b21ec8:     	ldr	x8, [sp, #0xd0]
100b21ecc:     	str	x9, [sp, #0xe8]
100b21ed0:     	str	x9, [x8, #0x8]
100b21ed4:     	mov	x25, x22
100b21ed8:     	cbz	x22, 0x100b2214c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x78c>
100b21edc:     	cmp	x22, #0x1
100b21ee0:     	b.ne	0x100b21ef0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x530>
100b21ee4:     	mov	x8, #0x0                ; =0
100b21ee8:     	mov	x25, #0x0               ; =0
100b21eec:     	b	0x100b2212c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x76c>
100b21ef0:     	dup.2d	v0, x21
100b21ef4:     	cmp	x22, #0x10
100b21ef8:     	b.hs	0x100b21f08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x548>
100b21efc:     	mov	x9, #0x0                ; =0
100b21f00:     	mov	x25, #0x0               ; =0
100b21f04:     	b	0x100b220b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x6f8>
100b21f08:     	movi.2d	v1, #0000000000000000
100b21f0c:     	add	x8, x26, #0x20
100b21f10:     	movi.2d	v2, #0000000000000000
100b21f14:     	and	x9, x22, #0x1ffffffffffffff0
100b21f18:     	ldr	q4, [sp, #0x90]
100b21f1c:     	ldp	q6, q15, [sp, #0x20]
100b21f20:     	movi.2d	v3, #0000000000000000
100b21f24:     	movi.2d	v7, #0000000000000000
100b21f28:     	movi.2d	v16, #0000000000000000
100b21f2c:     	movi.2d	v5, #0000000000000000
100b21f30:     	movi.2d	v18, #0000000000000000
100b21f34:     	movi.2d	v17, #0000000000000000
100b21f38:     	ldp	q13, q12, [sp, #0x50]
100b21f3c:     	ldr	q14, [sp, #0x40]
100b21f40:     	mov	w10, #0x3f              ; =63
100b21f44:     	movi.4s	v8, #0x3f
100b21f48:     	add.2d	v19, v4, v12
100b21f4c:     	add.2d	v20, v6, v12
100b21f50:     	add.2d	v21, v4, v13
100b21f54:     	add.2d	v22, v6, v13
100b21f58:     	add.2d	v23, v4, v14
100b21f5c:     	add.2d	v24, v6, v14
100b21f60:     	ldp	q25, q26, [x8, #-0x20]
100b21f64:     	dup.2d	v27, x10
100b21f68:     	ldp	q28, q29, [x8], #0x40
100b21f6c:     	and.16b	v30, v6, v27
100b21f70:     	and.16b	v31, v4, v27
100b21f74:     	and.16b	v20, v20, v27
100b21f78:     	and.16b	v19, v19, v27
100b21f7c:     	and.16b	v22, v22, v27
100b21f80:     	and.16b	v21, v21, v27
100b21f84:     	and.16b	v24, v24, v27
100b21f88:     	and.16b	v23, v23, v27
100b21f8c:     	neg.2d	v27, v31
100b21f90:     	ushl.2d	v27, v0, v27
100b21f94:     	neg.2d	v30, v30
100b21f98:     	ushl.2d	v30, v0, v30
100b21f9c:     	neg.2d	v19, v19
100b21fa0:     	ushl.2d	v19, v0, v19
100b21fa4:     	neg.2d	v20, v20
100b21fa8:     	ushl.2d	v20, v0, v20
100b21fac:     	neg.2d	v21, v21
100b21fb0:     	ushl.2d	v21, v0, v21
100b21fb4:     	neg.2d	v22, v22
100b21fb8:     	ushl.2d	v22, v0, v22
100b21fbc:     	neg.2d	v23, v23
100b21fc0:     	ushl.2d	v23, v0, v23
100b21fc4:     	neg.2d	v24, v24
100b21fc8:     	ushl.2d	v24, v0, v24
100b21fcc:     	dup.2d	v31, x24
100b21fd0:     	and.16b	v30, v30, v31
100b21fd4:     	and.16b	v27, v27, v31
100b21fd8:     	and.16b	v20, v20, v31
100b21fdc:     	and.16b	v19, v19, v31
100b21fe0:     	and.16b	v22, v22, v31
100b21fe4:     	and.16b	v21, v21, v31
100b21fe8:     	and.16b	v24, v24, v31
100b21fec:     	and.16b	v23, v23, v31
100b21ff0:     	and.16b	v25, v25, v8
100b21ff4:     	and.16b	v26, v26, v8
100b21ff8:     	and.16b	v28, v28, v8
100b21ffc:     	and.16b	v29, v29, v8
100b22000:     	ushll2.2d	v31, v25, #0x0
100b22004:     	ushll.2d	v25, v25, #0x0
100b22008:     	ushll2.2d	v9, v26, #0x0
100b2200c:     	ushll.2d	v26, v26, #0x0
100b22010:     	ushll2.2d	v10, v28, #0x0
100b22014:     	ushll.2d	v28, v28, #0x0
100b22018:     	ushll2.2d	v11, v29, #0x0
100b2201c:     	ushll.2d	v29, v29, #0x0
100b22020:     	ushl.2d	v25, v27, v25
100b22024:     	ushl.2d	v27, v30, v31
100b22028:     	ushl.2d	v19, v19, v26
100b2202c:     	ushl.2d	v20, v20, v9
100b22030:     	ushl.2d	v21, v21, v28
100b22034:     	ushl.2d	v22, v22, v10
100b22038:     	ushl.2d	v23, v23, v29
100b2203c:     	ushl.2d	v24, v24, v11
100b22040:     	orr.16b	v3, v27, v3
100b22044:     	orr.16b	v2, v25, v2
100b22048:     	orr.16b	v16, v20, v16
100b2204c:     	orr.16b	v7, v19, v7
100b22050:     	orr.16b	v18, v22, v18
100b22054:     	orr.16b	v5, v21, v5
100b22058:     	orr.16b	v1, v24, v1
100b2205c:     	orr.16b	v17, v23, v17
100b22060:     	add.2d	v6, v6, v15
100b22064:     	add.2d	v4, v4, v15
100b22068:     	subs	x9, x9, #0x10
100b2206c:     	b.ne	0x100b21f48 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x588>
100b22070:     	orr.16b	v2, v7, v2
100b22074:     	orr.16b	v3, v16, v3
100b22078:     	orr.16b	v3, v18, v3
100b2207c:     	orr.16b	v2, v5, v2
100b22080:     	orr.16b	v2, v17, v2
100b22084:     	orr.16b	v1, v1, v3
100b22088:     	orr.16b	v1, v2, v1
100b2208c:     	mov	d2, v1[1]
100b22090:     	orr.8b	v1, v1, v2
100b22094:     	fmov	x25, d1
100b22098:     	and	x8, x22, #0x1ffffffffffffff0
100b2209c:     	cmp	x22, x8
100b220a0:     	movi.2s	v8, #0x3f
100b220a4:     	b.eq	0x100b2214c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x78c>
100b220a8:     	and	x9, x22, #0x1ffffffffffffff0
100b220ac:     	and	x8, x22, #0x1ffffffffffffff0
100b220b0:     	and	x10, x22, #0xe
100b220b4:     	cbz	x10, 0x100b2212c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x76c>
100b220b8:     	fmov	d1, x25
100b220bc:     	dup.2d	v2, x9
100b220c0:     	ldr	q3, [sp, #0x90]
100b220c4:     	orr.16b	v2, v2, v3
100b220c8:     	ldr	x8, [sp, #0x88]
100b220cc:     	add	x8, x8, x9
100b220d0:     	add	x9, x26, x9, lsl #2
100b220d4:     	ldr	q6, [sp, #0x70]
100b220d8:     	ldr	d3, [x9], #0x8
100b220dc:     	and.16b	v4, v2, v6
100b220e0:     	neg.2d	v4, v4
100b220e4:     	ushl.2d	v4, v0, v4
100b220e8:     	dup.2d	v5, x24
100b220ec:     	and.16b	v4, v4, v5
100b220f0:     	and.8b	v3, v3, v8
100b220f4:     	ushll.2d	v3, v3, #0x0
100b220f8:     	ushl.2d	v3, v4, v3
100b220fc:     	orr.16b	v1, v3, v1
100b22100:     	dup.2d	v3, x23
100b22104:     	add.2d	v2, v2, v3
100b22108:     	adds	x8, x8, #0x2
100b2210c:     	b.ne	0x100b220d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x718>
100b22110:     	mov	d0, v1[1]
100b22114:     	orr.8b	v0, v1, v0
100b22118:     	fmov	x25, d0
100b2211c:     	and	x8, x22, #0x1ffffffffffffffe
100b22120:     	and	x9, x22, #0x1ffffffffffffffe
100b22124:     	cmp	x22, x9
100b22128:     	b.eq	0x100b2214c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x78c>
100b2212c:     	ldr	w9, [x26, x8, lsl #2]
100b22130:     	lsr	x10, x21, x8
100b22134:     	and	x10, x10, #0x1
100b22138:     	lsl	x9, x10, x9
100b2213c:     	orr	x25, x9, x25
100b22140:     	add	x8, x8, #0x1
100b22144:     	cmp	x22, x8
100b22148:     	b.ne	0x100b2212c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x76c>
100b2214c:     	ldr	x8, [sp, #0xb8]
100b22150:     	orr	x2, x8, x25
100b22154:     	mov	x0, x28
100b22158:     	ldr	x1, [sp, #0xe0]
100b2215c:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b22160:     	mov	x19, x0
100b22164:     	ldr	x8, [sp, #0xb0]
100b22168:     	orr	x2, x8, x25
100b2216c:     	mov	x0, x28
100b22170:     	ldr	x1, [sp, #0xc8]
100b22174:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b22178:     	mov	x20, x0
100b2217c:     	ldr	x8, [sp, #0xa8]
100b22180:     	orr	x2, x8, x25
100b22184:     	mov	x0, x28
100b22188:     	ldr	x1, [sp, #0xc0]
100b2218c:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b22190:     	tbz	w19, #0x0, 0x100b21eac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x4ec>
100b22194:     	cmp	w20, #0x0
100b22198:     	csel	w8, w23, wzr, ne
100b2219c:     	orr	w8, w8, w0
100b221a0:     	lsl	w8, w24, w8
100b221a4:     	orr	w27, w8, w27
100b221a8:     	and	w8, w27, #0xff
100b221ac:     	cmp	w8, #0xf
100b221b0:     	b.ne	0x100b21eac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x4ec>
100b221b4:     	mov	w27, #0xf               ; =15
100b221b8:     	ldr	w8, [sp, #0x14]
100b221bc:     	tbnz	w8, #0x0, 0x100b221c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x808>
100b221c0:     	mov	x0, x26
100b221c4:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b221c8:     	ldr	x24, [sp, #0x18]
100b221cc:     	ldr	x0, [sp, #0x8]
100b221d0:     	mov	x1, x24
100b221d4:     	mov	x2, x27
100b221d8:     	bl	0x100c293a0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b221dc:     	mov	x0, x27
100b221e0:     	add	sp, sp, #0x230
100b221e4:     	ldp	x29, x30, [sp, #0x90]
100b221e8:     	ldp	x20, x19, [sp, #0x80]
100b221ec:     	ldp	x22, x21, [sp, #0x70]
100b221f0:     	ldp	x24, x23, [sp, #0x60]
100b221f4:     	ldp	x26, x25, [sp, #0x50]
100b221f8:     	ldp	x28, x27, [sp, #0x40]
100b221fc:     	ldp	d9, d8, [sp, #0x30]
100b22200:     	ldp	d11, d10, [sp, #0x20]
100b22204:     	ldp	d13, d12, [sp, #0x10]
100b22208:     	ldp	d15, d14, [sp], #0xa0
100b2220c:     	ret
100b22210:     	adrp	x0, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b22214:     	add	x0, x0, #0xb78
100b22218:     	bl	0x101282074 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b2221c:     	mov	x0, x12
100b22220:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22224:     	add	x2, x2, #0x368
100b22228:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2222c:     	mov	x0, x11
100b22230:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22234:     	add	x2, x2, #0x368
100b22238:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2223c:     	mov	x0, x11
100b22240:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22244:     	add	x2, x2, #0x350
100b22248:     	mov	x1, x8
100b2224c:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22250:     	mov	x0, x10
100b22254:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22258:     	add	x2, x2, #0x350
100b2225c:     	mov	x1, x8
100b22260:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22264:     	mov	x19, x0
100b22268:     	ldur	x8, [x29, #-0xc0]
100b2226c:     	cbz	x8, 0x100b2228c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x8cc>
100b22270:     	ldur	x26, [x29, #-0xb8]
100b22274:     	b	0x100b22284 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x8c4>
100b22278:     	mov	x19, x0
100b2227c:     	ldr	w8, [sp, #0x14]
100b22280:     	tbnz	w8, #0x0, 0x100b2228c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb0_EB8_+0x8cc>
100b22284:     	mov	x0, x26
100b22288:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b2228c:     	mov	x0, x19
100b22290:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
