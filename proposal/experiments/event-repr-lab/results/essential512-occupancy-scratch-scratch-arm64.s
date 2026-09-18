
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b26998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_>:
100b26998:     	stp	x28, x27, [sp, #-0x60]!
100b2699c:     	stp	x26, x25, [sp, #0x10]
100b269a0:     	stp	x24, x23, [sp, #0x20]
100b269a4:     	stp	x22, x21, [sp, #0x30]
100b269a8:     	stp	x20, x19, [sp, #0x40]
100b269ac:     	stp	x29, x30, [sp, #0x50]
100b269b0:     	add	x29, sp, #0x50
100b269b4:     	sub	sp, sp, #0x270
100b269b8:     	str	x6, [sp, #0x98]
100b269bc:     	ldr	w19, [x3, #0x10]
100b269c0:     	cbz	w19, 0x100b269f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x5c>
100b269c4:     	mov	x21, x5
100b269c8:     	mov	x22, x4
100b269cc:     	mov	x20, x3
100b269d0:     	mov	x24, x2
100b269d4:     	mov	x25, x1
100b269d8:     	mov	x26, x0
100b269dc:     	mov	x0, x4
100b269e0:     	mov	x1, x3
100b269e4:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b269e8:     	cbz	x0, 0x100b269fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x64>
100b269ec:     	ldrb	w23, [x0]
100b269f0:     	b	0x100b272fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x964>
100b269f4:     	mov	w23, #0x0               ; =0
100b269f8:     	b	0x100b272fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x964>
100b269fc:     	ldr	x8, [x21]
100b26a00:     	add	x8, x8, #0x1
100b26a04:     	str	x8, [x21]
100b26a08:     	mov	x10, x26
100b26a0c:     	ldr	x8, [x10, #0x30]!
100b26a10:     	ldr	x1, [x10, #0x10]
100b26a14:     	ldr	x9, [x20]
100b26a18:     	lsr	x0, x19, #1
100b26a1c:     	cmn	x8, #0x1
100b26a20:     	b.eq	0x100b26a7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xe4>
100b26a24:     	cmp	x1, x0
100b26a28:     	b.ls	0x100b27458 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xac0>
100b26a2c:     	ldr	w8, [x20, #0x28]
100b26a30:     	lsr	x8, x8, #1
100b26a34:     	cmp	x1, x8
100b26a38:     	b.ls	0x100b27444 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xaac>
100b26a3c:     	mov	x13, x10
100b26a40:     	ldr	w10, [x20, #0x40]
100b26a44:     	lsr	x10, x10, #1
100b26a48:     	cmp	x1, x10
100b26a4c:     	b.ls	0x100b27454 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xabc>
100b26a50:     	ldr	x11, [x13, #0x8]
100b26a54:     	lsl	x8, x8, #4
100b26a58:     	ldr	x8, [x11, x8]
100b26a5c:     	ldr	x12, [x20, #0x18]
100b26a60:     	bic	x8, x8, x12
100b26a64:     	lsl	x12, x0, #4
100b26a68:     	ldr	x12, [x11, x12]
100b26a6c:     	bic	x9, x12, x9
100b26a70:     	orr	x8, x8, x9
100b26a74:     	add	x9, x11, x10, lsl #4
100b26a78:     	b	0x100b26ad4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x13c>
100b26a7c:     	ldr	x8, [x10, #0x18]
100b26a80:     	cmp	x8, x0
100b26a84:     	b.ls	0x100b2747c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xae4>
100b26a88:     	mov	x13, x10
100b26a8c:     	ldr	w10, [x20, #0x28]
100b26a90:     	lsr	x11, x10, #1
100b26a94:     	cmp	x8, x11
100b26a98:     	b.ls	0x100b27464 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xacc>
100b26a9c:     	ldr	w10, [x20, #0x40]
100b26aa0:     	lsr	x10, x10, #1
100b26aa4:     	cmp	x8, x10
100b26aa8:     	b.ls	0x100b27478 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xae0>
100b26aac:     	add	x8, x1, x11, lsl #5
100b26ab0:     	ldr	x8, [x8, #0x18]
100b26ab4:     	ldr	x11, [x20, #0x18]
100b26ab8:     	bic	x8, x8, x11
100b26abc:     	add	x11, x1, x0, lsl #5
100b26ac0:     	ldr	x11, [x11, #0x18]
100b26ac4:     	bic	x9, x11, x9
100b26ac8:     	orr	x8, x8, x9
100b26acc:     	add	x9, x1, x10, lsl #5
100b26ad0:     	add	x9, x9, #0x18
100b26ad4:     	stp	x22, x20, [sp, #0x20]
100b26ad8:     	ldr	x9, [x9]
100b26adc:     	mov	x19, x20
100b26ae0:     	ldr	x10, [x19, #0x30]!
100b26ae4:     	bic	x9, x9, x10
100b26ae8:     	orr	x11, x9, x8
100b26aec:     	fmov	d0, x11
100b26af0:     	cnt.8b	v0, v0
100b26af4:     	addv.8b	b0, v0
100b26af8:     	fmov	x9, d0
100b26afc:     	cmp	x9, #0xa
100b26b00:     	b.hs	0x100b26f00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x568>
100b26b04:     	mov	x22, #0x0               ; =0
100b26b08:     	ldr	x8, [x21, #0x10]
100b26b0c:     	add	x8, x8, #0x1
100b26b10:     	str	x8, [x21, #0x10]
100b26b14:     	strh	wzr, [sp, #0xa0]
100b26b18:     	strh	wzr, [sp, #0xc0]
100b26b1c:     	strh	wzr, [sp, #0xe0]
100b26b20:     	add	x8, sp, #0x100
100b26b24:     	add	x25, x8, #0x10
100b26b28:     	ldr	x8, [x20, #0x40]
100b26b2c:     	ldp	q0, q1, [x20]
100b26b30:     	stp	q0, q1, [sp, #0x110]
100b26b34:     	ldp	q0, q1, [x20, #0x20]
100b26b38:     	stp	q0, q1, [sp, #0x130]
100b26b3c:     	stp	x8, xzr, [sp, #0x150]
100b26b40:     	mov	w19, #0x1               ; =1
100b26b44:     	lsl	x8, x19, x9
100b26b48:     	stp	x8, x9, [sp, #0x8]
100b26b4c:     	lsr	x8, x8, #6
100b26b50:     	cmp	x9, #0x6
100b26b54:     	cinc	x8, x8, lo
100b26b58:     	stp	x25, x8, [sp, #0x40]
100b26b5c:     	lsl	x8, x8, #3
100b26b60:     	str	x8, [sp, #0x18]
100b26b64:     	ldp	x8, x27, [x21, #0x28]
100b26b68:     	str	x8, [sp, #0x50]
100b26b6c:     	ldr	x8, [x21, #0x20]
100b26b70:     	stp	x8, x13, [sp, #0x30]
100b26b74:     	mov	x9, x21
100b26b78:     	str	x9, [sp, #0x60]
100b26b7c:     	ldr	x23, [x9, #0x40]
100b26b80:     	mov	x26, x13
100b26b84:     	str	x11, [sp, #0x88]
100b26b88:     	b	0x100b26bd4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x23c>
100b26b8c:     	and	w10, w28, #0x1
100b26b90:     	add	x27, x27, #0x1
100b26b94:     	ldr	x9, [sp, #0x60]
100b26b98:     	str	x27, [x9, #0x30]
100b26b9c:     	ldr	x11, [sp, #0x88]
100b26ba0:     	mov	x20, x21
100b26ba4:     	add	x9, sp, #0xa0
100b26ba8:     	add	x9, x9, x22, lsl #5
100b26bac:     	strb	w8, [x9]
100b26bb0:     	str	w10, [sp, #0x7c]
100b26bb4:     	strb	w10, [x9, #0x1]
100b26bb8:     	add	x22, x22, #0x1
100b26bbc:     	mov	x21, x20
100b26bc0:     	ldp	x8, x10, [sp, #0x68]
100b26bc4:     	stp	x20, x8, [x9, #0x8]
100b26bc8:     	str	x10, [x9, #0x18]
100b26bcc:     	cmp	x22, #0x3
100b26bd0:     	b.eq	0x100b27030 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x698>
100b26bd4:     	mov	w8, #0x18               ; =24
100b26bd8:     	madd	x8, x22, x8, x25
100b26bdc:     	ldp	x24, x20, [x8]
100b26be0:     	ldr	w28, [x8, #0x10]
100b26be4:     	stur	x11, [x29, #-0xa0]
100b26be8:     	add	x0, sp, #0x160
100b26bec:     	mov	x1, x26
100b26bf0:     	mov	x2, x28
100b26bf4:     	bl	0x100c86bac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b26bf8:     	ldr	w8, [sp, #0x160]
100b26bfc:     	cbz	w8, 0x100b26b8c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x1f4>
100b26c00:     	str	x27, [sp, #0x58]
100b26c04:     	str	x22, [sp, #0x80]
100b26c08:     	cmp	w8, #0x1
100b26c0c:     	b.ne	0x100b273e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa48>
100b26c10:     	ldp	x8, x27, [sp, #0x168]
100b26c14:     	str	x8, [sp, #0x90]
100b26c18:     	ldr	x25, [sp, #0x178]
100b26c1c:     	bics	x8, x24, x25
100b26c20:     	str	x8, [sp, #0x160]
100b26c24:     	ldr	x10, [sp, #0x88]
100b26c28:     	ldr	x21, [sp, #0x60]
100b26c2c:     	b.ne	0x100b27320 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x988>
100b26c30:     	bics	x8, x20, x24
100b26c34:     	str	x8, [sp, #0x160]
100b26c38:     	b.ne	0x100b27340 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x9a8>
100b26c3c:     	orr	x8, x24, x10
100b26c40:     	bics	x8, x25, x8
100b26c44:     	str	x8, [sp, #0x160]
100b26c48:     	b.ne	0x100b27360 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x9c8>
100b26c4c:     	ands	x8, x24, x10
100b26c50:     	str	x8, [sp, #0x160]
100b26c54:     	b.ne	0x100b27380 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x9e8>
100b26c58:     	ldr	x8, [sp, #0x98]
100b26c5c:     	ldr	x9, [sp, #0x80]
100b26c60:     	add	x26, x8, x9, lsl #6
100b26c64:     	cbz	x24, 0x100b26d88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x3f0>
100b26c68:     	mov	w8, #0x0                ; =0
100b26c6c:     	b	0x100b26ca4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x30c>
100b26c70:     	cmp	x9, #0x0
100b26c74:     	cset	w4, ne
100b26c78:     	ldr	x0, [sp, #0x90]
100b26c7c:     	mov	x1, x27
100b26c80:     	mov	x5, x26
100b26c84:     	bl	0x100d20d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into>
100b26c88:     	mov	w8, #0x1                ; =1
100b26c8c:     	add	x23, x23, #0x1
100b26c90:     	str	x23, [x21, #0x40]
100b26c94:     	bic	x25, x25, x22
100b26c98:     	cmp	x22, x24
100b26c9c:     	eor	x24, x22, x24
100b26ca0:     	b.eq	0x100b26d6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x3d4>
100b26ca4:     	fmov	d0, x25
100b26ca8:     	cnt.8b	v0, v0
100b26cac:     	addv.8b	b0, v0
100b26cb0:     	fmov	x2, d0
100b26cb4:     	and	w9, w2, #0x3e
100b26cb8:     	lsl	x10, x19, x2
100b26cbc:     	lsr	x10, x10, #6
100b26cc0:     	cmp	w9, #0x6
100b26cc4:     	cinc	x1, x10, lo
100b26cc8:     	sub	w9, w2, #0x1
100b26ccc:     	and	w10, w9, #0x3f
100b26cd0:     	lsl	x9, x19, x9
100b26cd4:     	lsr	x9, x9, #6
100b26cd8:     	cmp	w10, #0x6
100b26cdc:     	cinc	x6, x9, lo
100b26ce0:     	cmp	x1, #0x8
100b26ce4:     	ccmp	x6, #0x8, #0x2, ls
100b26ce8:     	b.hi	0x100b272c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x930>
100b26cec:     	neg	x9, x24
100b26cf0:     	and	x22, x24, x9
100b26cf4:     	sub	x9, x22, #0x1
100b26cf8:     	and	x9, x9, x25
100b26cfc:     	fmov	d0, x9
100b26d00:     	cnt.8b	v0, v0
100b26d04:     	addv.8b	b0, v0
100b26d08:     	fmov	w3, s0
100b26d0c:     	and	x9, x22, x20
100b26d10:     	ands	w8, w8, #0xff
100b26d14:     	b.eq	0x100b26c70 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x2d8>
100b26d18:     	cmp	w8, #0x1
100b26d1c:     	b.ne	0x100b26d58 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x3c0>
100b26d20:     	cmp	x9, #0x0
100b26d24:     	cset	w4, ne
100b26d28:     	ldr	x8, [sp, #0x98]
100b26d2c:     	add	x5, x8, #0xc0
100b26d30:     	mov	x0, x26
100b26d34:     	bl	0x100d20d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into>
100b26d38:     	mov	w8, #0x2                ; =2
100b26d3c:     	add	x23, x23, #0x1
100b26d40:     	str	x23, [x21, #0x40]
100b26d44:     	bic	x25, x25, x22
100b26d48:     	cmp	x22, x24
100b26d4c:     	eor	x24, x22, x24
100b26d50:     	b.ne	0x100b26ca4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x30c>
100b26d54:     	b	0x100b26d6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x3d4>
100b26d58:     	cmp	x9, #0x0
100b26d5c:     	cset	w4, ne
100b26d60:     	ldr	x8, [sp, #0x98]
100b26d64:     	add	x0, x8, #0xc0
100b26d68:     	b	0x100b26c80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x2e8>
100b26d6c:     	mvn	x9, x25
100b26d70:     	ldr	x10, [sp, #0x88]
100b26d74:     	mov	x12, x28
100b26d78:     	ands	x20, x9, x10
100b26d7c:     	ldr	x24, [sp, #0x98]
100b26d80:     	b.ne	0x100b26e3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x4a4>
100b26d84:     	b	0x100b26da0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x408>
100b26d88:     	mov	x12, x28
100b26d8c:     	mov	w8, #0x0                ; =0
100b26d90:     	mvn	x9, x25
100b26d94:     	ands	x20, x9, x10
100b26d98:     	ldr	x24, [sp, #0x98]
100b26d9c:     	b.ne	0x100b26e3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x4a4>
100b26da0:     	stur	x25, [x29, #-0x80]
100b26da4:     	ldr	x11, [sp, #0x88]
100b26da8:     	cmp	x25, x11
100b26dac:     	b.ne	0x100b273a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa08>
100b26db0:     	sbfx	x20, x12, #0, #1
100b26db4:     	cbz	w8, 0x100b26ed0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x538>
100b26db8:     	cmp	w8, #0x2
100b26dbc:     	ldr	x22, [sp, #0x80]
100b26dc0:     	ldr	x25, [sp, #0x40]
100b26dc4:     	ldr	x27, [sp, #0x58]
100b26dc8:     	b.ne	0x100b26dec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x454>
100b26dcc:     	ldr	x8, [sp, #0x48]
100b26dd0:     	cmp	x8, #0x8
100b26dd4:     	b.hi	0x100b273bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa24>
100b26dd8:     	add	x1, x24, #0xc0
100b26ddc:     	mov	x0, x26
100b26de0:     	ldr	x2, [sp, #0x18]
100b26de4:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100b26de8:     	ldr	x11, [sp, #0x88]
100b26dec:     	ldr	x8, [sp, #0x50]
100b26df0:     	add	x8, x8, #0x1
100b26df4:     	str	x8, [sp, #0x50]
100b26df8:     	str	x8, [x21, #0x28]
100b26dfc:     	mov	w8, #0x2                ; =2
100b26e00:     	ldr	x26, [sp, #0x38]
100b26e04:     	ldr	w10, [sp, #0x7c]
100b26e08:     	b	0x100b26ba4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x20c>
100b26e0c:     	ldr	x0, [sp, #0x90]
100b26e10:     	mov	x1, x27
100b26e14:     	mov	x4, x26
100b26e18:     	bl	0x100d21824 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into>
100b26e1c:     	mov	w8, #0x1                ; =1
100b26e20:     	add	x23, x23, #0x1
100b26e24:     	str	x23, [x21, #0x40]
100b26e28:     	orr	x25, x22, x25
100b26e2c:     	cmp	x22, x20
100b26e30:     	eor	x20, x22, x20
100b26e34:     	mov	x12, x28
100b26e38:     	b.eq	0x100b26da0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x408>
100b26e3c:     	fmov	d0, x25
100b26e40:     	cnt.8b	v0, v0
100b26e44:     	addv.8b	b0, v0
100b26e48:     	fmov	x2, d0
100b26e4c:     	and	w9, w2, #0x3e
100b26e50:     	lsl	x10, x19, x2
100b26e54:     	lsr	x10, x10, #6
100b26e58:     	cmp	w9, #0x6
100b26e5c:     	cinc	x1, x10, lo
100b26e60:     	add	w9, w2, #0x1
100b26e64:     	and	w10, w9, #0x3f
100b26e68:     	lsl	x9, x19, x9
100b26e6c:     	lsr	x9, x9, #6
100b26e70:     	cmp	w10, #0x6
100b26e74:     	cinc	x5, x9, lo
100b26e78:     	cmp	x1, #0x8
100b26e7c:     	ccmp	x5, #0x8, #0x2, ls
100b26e80:     	b.hi	0x100b272c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x930>
100b26e84:     	neg	x9, x20
100b26e88:     	and	x22, x20, x9
100b26e8c:     	sub	x9, x22, #0x1
100b26e90:     	and	x9, x9, x25
100b26e94:     	fmov	d0, x9
100b26e98:     	cnt.8b	v0, v0
100b26e9c:     	addv.8b	b0, v0
100b26ea0:     	fmov	w3, s0
100b26ea4:     	ands	w8, w8, #0xff
100b26ea8:     	b.eq	0x100b26e0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x474>
100b26eac:     	cmp	w8, #0x1
100b26eb0:     	b.ne	0x100b26ec8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x530>
100b26eb4:     	add	x4, x24, #0xc0
100b26eb8:     	mov	x0, x26
100b26ebc:     	bl	0x100d21824 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into>
100b26ec0:     	mov	w8, #0x2                ; =2
100b26ec4:     	b	0x100b26e20 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x488>
100b26ec8:     	add	x0, x24, #0xc0
100b26ecc:     	b	0x100b26e14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x47c>
100b26ed0:     	ldp	x8, x26, [sp, #0x30]
100b26ed4:     	add	x8, x8, #0x1
100b26ed8:     	str	x8, [sp, #0x30]
100b26edc:     	str	x8, [x21, #0x20]
100b26ee0:     	mov	w8, #0x1                ; =1
100b26ee4:     	ldr	x9, [sp, #0x90]
100b26ee8:     	stp	x9, x27, [sp, #0x68]
100b26eec:     	ldr	x22, [sp, #0x80]
100b26ef0:     	ldr	x25, [sp, #0x40]
100b26ef4:     	ldr	x27, [sp, #0x58]
100b26ef8:     	ldr	w10, [sp, #0x7c]
100b26efc:     	b	0x100b26ba4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x20c>
100b26f00:     	mov	x8, #0x0                ; =0
100b26f04:     	add	x20, sp, #0x180
100b26f08:     	lsl	x9, x24, #2
100b26f0c:     	cmp	x9, x8
100b26f10:     	b.eq	0x100b273d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa3c>
100b26f14:     	ldr	w23, [x25, x8]
100b26f18:     	lsr	x10, x11, x23
100b26f1c:     	add	x8, x8, #0x4
100b26f20:     	tbz	w10, #0x0, 0x100b26f0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x574>
100b26f24:     	ldr	x22, [sp, #0x28]
100b26f28:     	ldr	q0, [x22]
100b26f2c:     	str	q0, [sp, #0x100]
100b26f30:     	ldr	x8, [x22, #0x10]
100b26f34:     	str	x8, [sp, #0x110]
100b26f38:     	add	x0, sp, #0x160
100b26f3c:     	add	x1, sp, #0x100
100b26f40:     	mov	x2, x26
100b26f44:     	mov	x3, x23
100b26f48:     	mov	w4, #0x0                ; =0
100b26f4c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26f50:     	ldr	q0, [sp, #0x160]
100b26f54:     	ldr	x8, [sp, #0x170]
100b26f58:     	stur	x8, [x29, #-0x90]
100b26f5c:     	str	q0, [sp, #0xa0]
100b26f60:     	str	x8, [sp, #0xb0]
100b26f64:     	str	q0, [sp, #0x180]
100b26f68:     	str	x8, [sp, #0x190]
100b26f6c:     	ldur	q0, [x22, #0x18]
100b26f70:     	str	q0, [sp, #0x100]
100b26f74:     	ldr	x8, [x22, #0x28]
100b26f78:     	str	x8, [sp, #0x110]
100b26f7c:     	add	x0, sp, #0x160
100b26f80:     	add	x1, sp, #0x100
100b26f84:     	mov	x2, x26
100b26f88:     	mov	x3, x23
100b26f8c:     	mov	w4, #0x0                ; =0
100b26f90:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26f94:     	ldr	q0, [sp, #0x160]
100b26f98:     	ldr	x8, [sp, #0x170]
100b26f9c:     	stur	x8, [x29, #-0x90]
100b26fa0:     	str	q0, [sp, #0xa0]
100b26fa4:     	str	x8, [sp, #0xb0]
100b26fa8:     	stur	q0, [x20, #0x18]
100b26fac:     	str	x8, [sp, #0x1a8]
100b26fb0:     	ldr	q0, [x19]
100b26fb4:     	str	q0, [sp, #0x100]
100b26fb8:     	ldr	x8, [x19, #0x10]
100b26fbc:     	str	x8, [sp, #0x110]
100b26fc0:     	add	x0, sp, #0x160
100b26fc4:     	add	x1, sp, #0x100
100b26fc8:     	mov	x2, x26
100b26fcc:     	mov	x27, x23
100b26fd0:     	mov	x3, x23
100b26fd4:     	mov	w4, #0x0                ; =0
100b26fd8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26fdc:     	ldr	q0, [sp, #0x160]
100b26fe0:     	ldr	x8, [sp, #0x170]
100b26fe4:     	stur	x8, [x29, #-0x90]
100b26fe8:     	str	q0, [sp, #0xa0]
100b26fec:     	str	q0, [sp, #0x1b0]
100b26ff0:     	str	x8, [sp, #0x1c0]
100b26ff4:     	add	x3, sp, #0x180
100b26ff8:     	mov	x0, x26
100b26ffc:     	mov	x1, x25
100b27000:     	mov	x2, x24
100b27004:     	ldr	x28, [sp, #0x20]
100b27008:     	mov	x4, x28
100b2700c:     	mov	x5, x21
100b27010:     	ldr	x20, [sp, #0x98]
100b27014:     	mov	x6, x20
100b27018:     	bl	0x100b26998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_>
100b2701c:     	and	w8, w0, #0xff
100b27020:     	cmp	w8, #0xf
100b27024:     	b.ne	0x100b271b4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x81c>
100b27028:     	mov	w23, #0xf               ; =15
100b2702c:     	b	0x100b272f0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x958>
100b27030:     	mov	x8, #-0x1               ; =-1
100b27034:     	ldp	x9, x10, [sp, #0x8]
100b27038:     	lsl	x9, x8, x9
100b2703c:     	cmp	x10, #0x6
100b27040:     	csinv	x10, x8, x9, hs
100b27044:     	ldr	x6, [sp, #0x48]
100b27048:     	cbz	x6, 0x100b272e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x948>
100b2704c:     	mov	w23, #0x0               ; =0
100b27050:     	mov	x0, #0x0                ; =0
100b27054:     	ldrb	w11, [sp, #0xa0]
100b27058:     	ldp	x13, x1, [sp, #0xb0]
100b2705c:     	ldr	x12, [sp, #0xa8]
100b27060:     	ldrb	w8, [sp, #0xa1]
100b27064:     	neg	x14, x8
100b27068:     	ldrb	w15, [sp, #0xc0]
100b2706c:     	ldp	x17, x8, [sp, #0xd0]
100b27070:     	ldr	x16, [sp, #0xc8]
100b27074:     	ldrb	w9, [sp, #0xc1]
100b27078:     	neg	x2, x9
100b2707c:     	ldrb	w4, [sp, #0xe0]
100b27080:     	ldp	x25, x9, [sp, #0xf0]
100b27084:     	ldrb	w3, [sp, #0xe1]
100b27088:     	neg	x5, x3
100b2708c:     	ldr	x21, [sp, #0x60]
100b27090:     	ldr	x3, [x21, #0x18]
100b27094:     	add	x6, x6, x3
100b27098:     	add	x3, x3, #0x1
100b2709c:     	mov	w7, #0x2                ; =2
100b270a0:     	mov	w19, #0x4               ; =4
100b270a4:     	mov	w20, #0x8               ; =8
100b270a8:     	ldr	x24, [sp, #0xe8]
100b270ac:     	mov	x22, x14
100b270b0:     	cbz	w11, 0x100b270e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x748>
100b270b4:     	cmp	w11, #0x2
100b270b8:     	b.ne	0x100b270cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x734>
100b270bc:     	ldr	x22, [sp, #0x98]
100b270c0:     	cmp	x0, #0x8
100b270c4:     	b.lo	0x100b270d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x740>
100b270c8:     	b	0x100b273f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa60>
100b270cc:     	mov	x22, x13
100b270d0:     	cmp	x0, x1
100b270d4:     	b.hs	0x100b2740c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa74>
100b270d8:     	ldr	x22, [x22, x0, lsl #3]
100b270dc:     	eor	x22, x22, x12
100b270e0:     	mov	x26, x2
100b270e4:     	cbz	w15, 0x100b27118 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x780>
100b270e8:     	cmp	w15, #0x2
100b270ec:     	b.ne	0x100b27104 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x76c>
100b270f0:     	ldr	x26, [sp, #0x98]
100b270f4:     	add	x26, x26, #0x40
100b270f8:     	cmp	x0, #0x8
100b270fc:     	b.lo	0x100b27110 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x778>
100b27100:     	b	0x100b273f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa60>
100b27104:     	mov	x26, x17
100b27108:     	cmp	x0, x8
100b2710c:     	b.hs	0x100b2741c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa84>
100b27110:     	ldr	x26, [x26, x0, lsl #3]
100b27114:     	eor	x26, x26, x16
100b27118:     	mov	x27, x5
100b2711c:     	cbz	w4, 0x100b27150 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x7b8>
100b27120:     	cmp	w4, #0x2
100b27124:     	b.ne	0x100b2713c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x7a4>
100b27128:     	ldr	x27, [sp, #0x98]
100b2712c:     	add	x27, x27, #0x80
100b27130:     	cmp	x0, #0x8
100b27134:     	b.lo	0x100b27148 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x7b0>
100b27138:     	b	0x100b273f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa60>
100b2713c:     	mov	x27, x25
100b27140:     	cmp	x0, x9
100b27144:     	b.hs	0x100b27430 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0xa98>
100b27148:     	ldr	x27, [x27, x0, lsl #3]
100b2714c:     	eor	x27, x27, x24
100b27150:     	and	x22, x22, x10
100b27154:     	bic	x28, x22, x26
100b27158:     	bics	xzr, x28, x27
100b2715c:     	cset	w30, ne
100b27160:     	tst	x27, x28
100b27164:     	csel	w28, wzr, w7, eq
100b27168:     	and	x22, x26, x22
100b2716c:     	bics	xzr, x22, x27
100b27170:     	csel	w26, wzr, w19, eq
100b27174:     	tst	x27, x22
100b27178:     	csel	w22, wzr, w20, eq
100b2717c:     	orr	w23, w23, w30
100b27180:     	orr	w26, w28, w26
100b27184:     	orr	w23, w23, w26
100b27188:     	orr	w23, w23, w22
100b2718c:     	and	w22, w23, #0xff
100b27190:     	cmp	w22, #0xf
100b27194:     	b.eq	0x100b272e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x950>
100b27198:     	add	x0, x0, #0x1
100b2719c:     	add	x3, x3, #0x1
100b271a0:     	ldr	x22, [sp, #0x48]
100b271a4:     	cmp	x22, x0
100b271a8:     	b.ne	0x100b270ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x714>
100b271ac:     	str	x6, [x21, #0x18]
100b271b0:     	b	0x100b272f0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x958>
100b271b4:     	mov	x23, x0
100b271b8:     	ldr	q0, [x22]
100b271bc:     	str	q0, [sp, #0x100]
100b271c0:     	ldr	x8, [x22, #0x10]
100b271c4:     	str	x8, [sp, #0x110]
100b271c8:     	add	x0, sp, #0x160
100b271cc:     	add	x1, sp, #0x100
100b271d0:     	mov	x2, x26
100b271d4:     	mov	x3, x27
100b271d8:     	mov	w4, #0x1                ; =1
100b271dc:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b271e0:     	ldr	q0, [sp, #0x160]
100b271e4:     	stur	q0, [x29, #-0x80]
100b271e8:     	ldr	x8, [sp, #0x170]
100b271ec:     	stur	q0, [x29, #-0xa0]
100b271f0:     	str	q0, [sp, #0xa0]
100b271f4:     	str	x8, [sp, #0xb0]
100b271f8:     	ldr	q0, [sp, #0xa0]
100b271fc:     	stur	x8, [x29, #-0xe0]
100b27200:     	stur	q0, [x29, #-0xf0]
100b27204:     	ldur	q0, [x22, #0x18]
100b27208:     	str	q0, [sp, #0x100]
100b2720c:     	ldur	x8, [x22, #0x28]
100b27210:     	str	x8, [sp, #0x110]
100b27214:     	add	x0, sp, #0x160
100b27218:     	add	x1, sp, #0x100
100b2721c:     	mov	x2, x26
100b27220:     	mov	x3, x27
100b27224:     	mov	w4, #0x1                ; =1
100b27228:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2722c:     	ldr	q0, [sp, #0x160]
100b27230:     	stur	q0, [x29, #-0x80]
100b27234:     	ldr	x8, [sp, #0x170]
100b27238:     	stur	q0, [x29, #-0xa0]
100b2723c:     	str	q0, [sp, #0xa0]
100b27240:     	str	x8, [sp, #0xb0]
100b27244:     	ldr	q0, [sp, #0xa0]
100b27248:     	stur	x8, [x29, #-0xc8]
100b2724c:     	add	x8, sp, #0x180
100b27250:     	stur	q0, [x8, #0x68]
100b27254:     	ldr	q0, [x19]
100b27258:     	str	q0, [sp, #0x100]
100b2725c:     	ldr	x8, [x19, #0x10]
100b27260:     	str	x8, [sp, #0x110]
100b27264:     	add	x0, sp, #0x160
100b27268:     	add	x1, sp, #0x100
100b2726c:     	mov	x2, x26
100b27270:     	mov	x3, x27
100b27274:     	mov	w4, #0x1                ; =1
100b27278:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2727c:     	ldr	q0, [sp, #0x160]
100b27280:     	stur	q0, [x29, #-0x80]
100b27284:     	ldr	x8, [sp, #0x170]
100b27288:     	stur	q0, [x29, #-0xa0]
100b2728c:     	str	q0, [sp, #0xa0]
100b27290:     	str	x8, [sp, #0xb0]
100b27294:     	ldr	q0, [sp, #0xa0]
100b27298:     	stur	x8, [x29, #-0xb0]
100b2729c:     	stur	q0, [x29, #-0xc0]
100b272a0:     	sub	x3, x29, #0xf0
100b272a4:     	mov	x0, x26
100b272a8:     	mov	x1, x25
100b272ac:     	mov	x2, x24
100b272b0:     	mov	x4, x28
100b272b4:     	mov	x5, x21
100b272b8:     	mov	x6, x20
100b272bc:     	bl	0x100b26998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_>
100b272c0:     	orr	w23, w0, w23
100b272c4:     	b	0x100b272f0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x958>
100b272c8:     	adrp	x0, 0x10132c000 <dyld_stub_binder+0x10132c000>
100b272cc:     	add	x0, x0, #0xbf0
100b272d0:     	adrp	x2, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b272d4:     	add	x2, x2, #0x238
100b272d8:     	mov	w1, #0x2b               ; =43
100b272dc:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b272e0:     	mov	w23, #0x0               ; =0
100b272e4:     	b	0x100b272f0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_+0x958>
100b272e8:     	mov	w23, #0xf               ; =15
100b272ec:     	str	x3, [x21, #0x18]
100b272f0:     	ldp	x0, x1, [sp, #0x20]
100b272f4:     	mov	x2, x23
100b272f8:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b272fc:     	mov	x0, x23
100b27300:     	add	sp, sp, #0x270
100b27304:     	ldp	x29, x30, [sp, #0x50]
100b27308:     	ldp	x20, x19, [sp, #0x40]
100b2730c:     	ldp	x22, x21, [sp, #0x30]
100b27310:     	ldp	x24, x23, [sp, #0x20]
100b27314:     	ldp	x26, x25, [sp, #0x10]
100b27318:     	ldp	x28, x27, [sp], #0x60
100b2731c:     	ret
100b27320:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b27324:     	add	x2, x2, #0x788
100b27328:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b2732c:     	add	x5, x5, #0x3e8
100b27330:     	add	x1, sp, #0x160
100b27334:     	mov	w0, #0x0                ; =0
100b27338:     	mov	x3, #0x0                ; =0
100b2733c:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b27340:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b27344:     	add	x2, x2, #0x788
100b27348:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b2734c:     	add	x5, x5, #0x3d0
100b27350:     	add	x1, sp, #0x160
100b27354:     	mov	w0, #0x0                ; =0
100b27358:     	mov	x3, #0x0                ; =0
100b2735c:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b27360:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b27364:     	add	x2, x2, #0x788
100b27368:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b2736c:     	add	x5, x5, #0x3b8
100b27370:     	add	x1, sp, #0x160
100b27374:     	mov	w0, #0x0                ; =0
100b27378:     	mov	x3, #0x0                ; =0
100b2737c:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b27380:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b27384:     	add	x2, x2, #0x788
100b27388:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b2738c:     	add	x5, x5, #0x3a0
100b27390:     	add	x1, sp, #0x160
100b27394:     	mov	w0, #0x0                ; =0
100b27398:     	mov	x3, #0x0                ; =0
100b2739c:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b273a0:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b273a4:     	add	x5, x5, #0x370
100b273a8:     	sub	x1, x29, #0x80
100b273ac:     	sub	x2, x29, #0xa0
100b273b0:     	mov	w0, #0x0                ; =0
100b273b4:     	mov	x3, #0x0                ; =0
100b273b8:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b273bc:     	adrp	x3, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b273c0:     	add	x3, x3, #0x388
100b273c4:     	mov	x0, #0x0                ; =0
100b273c8:     	mov	w1, #0x9                ; =9
100b273cc:     	mov	w2, #0x8                ; =8
100b273d0:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100b273d4:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b273d8:     	add	x0, x0, #0xd90
100b273dc:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b273e0:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100b273e4:     	add	x0, x0, #0xd49
100b273e8:     	adrp	x2, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b273ec:     	add	x2, x2, #0x400
100b273f0:     	mov	w1, #0xc9               ; =201
100b273f4:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b273f8:     	str	x3, [x21, #0x18]
100b273fc:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b27400:     	add	x2, x2, #0x6f0
100b27404:     	mov	w1, #0x8                ; =8
100b27408:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2740c:     	str	x3, [x21, #0x18]
100b27410:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b27414:     	add	x2, x2, #0x6d8
100b27418:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2741c:     	str	x3, [x21, #0x18]
100b27420:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b27424:     	add	x2, x2, #0x6d8
100b27428:     	mov	x1, x8
100b2742c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b27430:     	str	x3, [x21, #0x18]
100b27434:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b27438:     	add	x2, x2, #0x6d8
100b2743c:     	mov	x1, x9
100b27440:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b27444:     	mov	x0, x8
100b27448:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b2744c:     	add	x2, x2, #0x6c0
100b27450:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b27454:     	mov	x0, x10
100b27458:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b2745c:     	add	x2, x2, #0x6c0
100b27460:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b27464:     	mov	x0, x11
100b27468:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b2746c:     	add	x2, x2, #0x6a8
100b27470:     	mov	x1, x8
100b27474:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b27478:     	mov	x0, x10
100b2747c:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b27480:     	add	x2, x2, #0x6a8
100b27484:     	mov	x1, x8
100b27488:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
