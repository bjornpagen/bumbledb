
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b24bd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_>:
100b24bd8:     	stp	x28, x27, [sp, #-0x60]!
100b24bdc:     	stp	x26, x25, [sp, #0x10]
100b24be0:     	stp	x24, x23, [sp, #0x20]
100b24be4:     	stp	x22, x21, [sp, #0x30]
100b24be8:     	stp	x20, x19, [sp, #0x40]
100b24bec:     	stp	x29, x30, [sp, #0x50]
100b24bf0:     	add	x29, sp, #0x50
100b24bf4:     	sub	sp, sp, #0x270
100b24bf8:     	str	x6, [sp, #0x98]
100b24bfc:     	ldr	w19, [x3, #0x10]
100b24c00:     	cbz	w19, 0x100b24c34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x5c>
100b24c04:     	mov	x21, x5
100b24c08:     	mov	x22, x4
100b24c0c:     	mov	x20, x3
100b24c10:     	mov	x24, x2
100b24c14:     	mov	x25, x1
100b24c18:     	mov	x26, x0
100b24c1c:     	mov	x0, x4
100b24c20:     	mov	x1, x3
100b24c24:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b24c28:     	cbz	x0, 0x100b24c3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x64>
100b24c2c:     	ldrb	w22, [x0]
100b24c30:     	b	0x100b25438 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x860>
100b24c34:     	mov	w22, #0x0               ; =0
100b24c38:     	b	0x100b25438 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x860>
100b24c3c:     	ldr	x8, [x21]
100b24c40:     	add	x8, x8, #0x1
100b24c44:     	str	x8, [x21]
100b24c48:     	mov	x10, x26
100b24c4c:     	ldr	x8, [x10, #0x30]!
100b24c50:     	ldr	x1, [x10, #0x10]
100b24c54:     	ldr	x9, [x20]
100b24c58:     	lsr	x0, x19, #1
100b24c5c:     	cmn	x8, #0x1
100b24c60:     	b.eq	0x100b24cbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xe4>
100b24c64:     	cmp	x1, x0
100b24c68:     	b.ls	0x100b256c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xae8>
100b24c6c:     	ldr	w8, [x20, #0x28]
100b24c70:     	lsr	x8, x8, #1
100b24c74:     	cmp	x1, x8
100b24c78:     	b.ls	0x100b256ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xad4>
100b24c7c:     	mov	x13, x10
100b24c80:     	ldr	w10, [x20, #0x40]
100b24c84:     	lsr	x10, x10, #1
100b24c88:     	cmp	x1, x10
100b24c8c:     	b.ls	0x100b256bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xae4>
100b24c90:     	ldr	x11, [x13, #0x8]
100b24c94:     	lsl	x8, x8, #4
100b24c98:     	ldr	x8, [x11, x8]
100b24c9c:     	ldr	x12, [x20, #0x18]
100b24ca0:     	bic	x8, x8, x12
100b24ca4:     	lsl	x12, x0, #4
100b24ca8:     	ldr	x12, [x11, x12]
100b24cac:     	bic	x9, x12, x9
100b24cb0:     	orr	x8, x8, x9
100b24cb4:     	add	x9, x11, x10, lsl #4
100b24cb8:     	b	0x100b24d14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x13c>
100b24cbc:     	ldr	x8, [x10, #0x18]
100b24cc0:     	cmp	x8, x0
100b24cc4:     	b.ls	0x100b256e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb0c>
100b24cc8:     	mov	x13, x10
100b24ccc:     	ldr	w10, [x20, #0x28]
100b24cd0:     	lsr	x11, x10, #1
100b24cd4:     	cmp	x8, x11
100b24cd8:     	b.ls	0x100b256cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xaf4>
100b24cdc:     	ldr	w10, [x20, #0x40]
100b24ce0:     	lsr	x10, x10, #1
100b24ce4:     	cmp	x8, x10
100b24ce8:     	b.ls	0x100b256e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb08>
100b24cec:     	add	x8, x1, x11, lsl #5
100b24cf0:     	ldr	x8, [x8, #0x18]
100b24cf4:     	ldr	x11, [x20, #0x18]
100b24cf8:     	bic	x8, x8, x11
100b24cfc:     	add	x11, x1, x0, lsl #5
100b24d00:     	ldr	x11, [x11, #0x18]
100b24d04:     	bic	x9, x11, x9
100b24d08:     	orr	x8, x8, x9
100b24d0c:     	add	x9, x1, x10, lsl #5
100b24d10:     	add	x9, x9, #0x18
100b24d14:     	str	x22, [sp, #0x20]
100b24d18:     	ldr	x9, [x9]
100b24d1c:     	mov	x19, x20
100b24d20:     	ldr	x10, [x19, #0x30]!
100b24d24:     	bic	x9, x9, x10
100b24d28:     	orr	x11, x9, x8
100b24d2c:     	fmov	d0, x11
100b24d30:     	cnt.8b	v0, v0
100b24d34:     	addv.8b	b0, v0
100b24d38:     	fmov	x9, d0
100b24d3c:     	cmp	x9, #0x7
100b24d40:     	b.hs	0x100b25144 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x56c>
100b24d44:     	mov	x22, #0x0               ; =0
100b24d48:     	ldr	x8, [x21, #0x10]
100b24d4c:     	add	x8, x8, #0x1
100b24d50:     	str	x8, [x21, #0x10]
100b24d54:     	strh	wzr, [sp, #0xa0]
100b24d58:     	strh	wzr, [sp, #0xc0]
100b24d5c:     	strh	wzr, [sp, #0xe0]
100b24d60:     	add	x8, sp, #0x100
100b24d64:     	add	x25, x8, #0x10
100b24d68:     	ldr	x8, [x20, #0x40]
100b24d6c:     	ldp	q0, q1, [x20]
100b24d70:     	stp	q0, q1, [sp, #0x110]
100b24d74:     	ldp	q0, q1, [x20, #0x20]
100b24d78:     	stp	q0, q1, [sp, #0x130]
100b24d7c:     	stp	x8, xzr, [sp, #0x150]
100b24d80:     	mov	w19, #0x1               ; =1
100b24d84:     	lsl	x8, x19, x9
100b24d88:     	stp	x20, x8, [sp, #0x8]
100b24d8c:     	lsr	x8, x8, #6
100b24d90:     	str	x9, [sp, #0x18]
100b24d94:     	cmp	x9, #0x6
100b24d98:     	cinc	x8, x8, ne
100b24d9c:     	str	x8, [sp, #0x30]
100b24da0:     	lsl	x8, x8, #3
100b24da4:     	str	x8, [sp, #0x28]
100b24da8:     	ldp	x8, x27, [x21, #0x28]
100b24dac:     	stp	x25, x8, [sp, #0x48]
100b24db0:     	ldr	x8, [x21, #0x20]
100b24db4:     	stp	x8, x13, [sp, #0x38]
100b24db8:     	mov	x9, x21
100b24dbc:     	str	x9, [sp, #0x60]
100b24dc0:     	ldr	x23, [x9, #0x40]
100b24dc4:     	mov	x26, x13
100b24dc8:     	str	x11, [sp, #0x88]
100b24dcc:     	b	0x100b24e18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x240>
100b24dd0:     	and	w10, w28, #0x1
100b24dd4:     	add	x27, x27, #0x1
100b24dd8:     	ldr	x9, [sp, #0x60]
100b24ddc:     	str	x27, [x9, #0x30]
100b24de0:     	ldr	x11, [sp, #0x88]
100b24de4:     	mov	x20, x21
100b24de8:     	add	x9, sp, #0xa0
100b24dec:     	add	x9, x9, x22, lsl #5
100b24df0:     	strb	w8, [x9]
100b24df4:     	str	w10, [sp, #0x7c]
100b24df8:     	strb	w10, [x9, #0x1]
100b24dfc:     	add	x22, x22, #0x1
100b24e00:     	mov	x21, x20
100b24e04:     	ldp	x8, x10, [sp, #0x68]
100b24e08:     	stp	x20, x8, [x9, #0x8]
100b24e0c:     	str	x10, [x9, #0x18]
100b24e10:     	cmp	x22, #0x3
100b24e14:     	b.eq	0x100b25274 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x69c>
100b24e18:     	mov	w8, #0x18               ; =24
100b24e1c:     	madd	x8, x22, x8, x25
100b24e20:     	ldp	x24, x20, [x8]
100b24e24:     	ldr	w28, [x8, #0x10]
100b24e28:     	stur	x11, [x29, #-0xa0]
100b24e2c:     	add	x0, sp, #0x160
100b24e30:     	mov	x1, x26
100b24e34:     	mov	x2, x28
100b24e38:     	bl	0x100c86bac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b24e3c:     	ldr	w8, [sp, #0x160]
100b24e40:     	cbz	w8, 0x100b24dd0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x1f8>
100b24e44:     	str	x27, [sp, #0x58]
100b24e48:     	str	x22, [sp, #0x80]
100b24e4c:     	cmp	w8, #0x1
100b24e50:     	b.ne	0x100b25694 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xabc>
100b24e54:     	ldp	x8, x27, [sp, #0x168]
100b24e58:     	str	x8, [sp, #0x90]
100b24e5c:     	ldr	x25, [sp, #0x178]
100b24e60:     	bics	x8, x24, x25
100b24e64:     	str	x8, [sp, #0x160]
100b24e68:     	ldr	x10, [sp, #0x88]
100b24e6c:     	ldr	x21, [sp, #0x60]
100b24e70:     	b.ne	0x100b255d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x9fc>
100b24e74:     	bics	x8, x20, x24
100b24e78:     	str	x8, [sp, #0x160]
100b24e7c:     	b.ne	0x100b255f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xa1c>
100b24e80:     	orr	x8, x24, x10
100b24e84:     	bics	x8, x25, x8
100b24e88:     	str	x8, [sp, #0x160]
100b24e8c:     	b.ne	0x100b25614 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xa3c>
100b24e90:     	ands	x8, x24, x10
100b24e94:     	str	x8, [sp, #0x160]
100b24e98:     	b.ne	0x100b25634 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xa5c>
100b24e9c:     	ldr	x8, [sp, #0x98]
100b24ea0:     	ldr	x9, [sp, #0x80]
100b24ea4:     	add	x26, x8, x9, lsl #3
100b24ea8:     	cbz	x24, 0x100b24fcc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x3f4>
100b24eac:     	mov	w8, #0x0                ; =0
100b24eb0:     	b	0x100b24ee8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x310>
100b24eb4:     	cmp	x9, #0x0
100b24eb8:     	cset	w4, ne
100b24ebc:     	ldr	x0, [sp, #0x90]
100b24ec0:     	mov	x1, x27
100b24ec4:     	mov	x5, x26
100b24ec8:     	bl	0x100d20d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into>
100b24ecc:     	mov	w8, #0x1                ; =1
100b24ed0:     	add	x23, x23, #0x1
100b24ed4:     	str	x23, [x21, #0x40]
100b24ed8:     	bic	x25, x25, x22
100b24edc:     	cmp	x22, x24
100b24ee0:     	eor	x24, x22, x24
100b24ee4:     	b.eq	0x100b24fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x3d8>
100b24ee8:     	fmov	d0, x25
100b24eec:     	cnt.8b	v0, v0
100b24ef0:     	addv.8b	b0, v0
100b24ef4:     	fmov	x2, d0
100b24ef8:     	and	w9, w2, #0x3e
100b24efc:     	lsl	x10, x19, x2
100b24f00:     	lsr	x10, x10, #6
100b24f04:     	cmp	w9, #0x6
100b24f08:     	cinc	x1, x10, lo
100b24f0c:     	sub	w9, w2, #0x1
100b24f10:     	and	w10, w9, #0x3f
100b24f14:     	lsl	x9, x19, x9
100b24f18:     	lsr	x9, x9, #6
100b24f1c:     	cmp	w10, #0x6
100b24f20:     	cinc	x6, x9, lo
100b24f24:     	cmp	x1, #0x1
100b24f28:     	ccmp	x6, #0x1, #0x2, ls
100b24f2c:     	b.hi	0x100b2545c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x884>
100b24f30:     	neg	x9, x24
100b24f34:     	and	x22, x24, x9
100b24f38:     	sub	x9, x22, #0x1
100b24f3c:     	and	x9, x9, x25
100b24f40:     	fmov	d0, x9
100b24f44:     	cnt.8b	v0, v0
100b24f48:     	addv.8b	b0, v0
100b24f4c:     	fmov	w3, s0
100b24f50:     	and	x9, x22, x20
100b24f54:     	ands	w8, w8, #0xff
100b24f58:     	b.eq	0x100b24eb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x2dc>
100b24f5c:     	cmp	w8, #0x1
100b24f60:     	b.ne	0x100b24f9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x3c4>
100b24f64:     	cmp	x9, #0x0
100b24f68:     	cset	w4, ne
100b24f6c:     	ldr	x8, [sp, #0x98]
100b24f70:     	add	x5, x8, #0x18
100b24f74:     	mov	x0, x26
100b24f78:     	bl	0x100d20d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into>
100b24f7c:     	mov	w8, #0x2                ; =2
100b24f80:     	add	x23, x23, #0x1
100b24f84:     	str	x23, [x21, #0x40]
100b24f88:     	bic	x25, x25, x22
100b24f8c:     	cmp	x22, x24
100b24f90:     	eor	x24, x22, x24
100b24f94:     	b.ne	0x100b24ee8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x310>
100b24f98:     	b	0x100b24fb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x3d8>
100b24f9c:     	cmp	x9, #0x0
100b24fa0:     	cset	w4, ne
100b24fa4:     	ldr	x8, [sp, #0x98]
100b24fa8:     	add	x0, x8, #0x18
100b24fac:     	b	0x100b24ec4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x2ec>
100b24fb0:     	mvn	x9, x25
100b24fb4:     	ldr	x10, [sp, #0x88]
100b24fb8:     	mov	x12, x28
100b24fbc:     	ands	x20, x9, x10
100b24fc0:     	ldr	x24, [sp, #0x98]
100b24fc4:     	b.ne	0x100b25080 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x4a8>
100b24fc8:     	b	0x100b24fe4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x40c>
100b24fcc:     	mov	x12, x28
100b24fd0:     	mov	w8, #0x0                ; =0
100b24fd4:     	mvn	x9, x25
100b24fd8:     	ands	x20, x9, x10
100b24fdc:     	ldr	x24, [sp, #0x98]
100b24fe0:     	b.ne	0x100b25080 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x4a8>
100b24fe4:     	stur	x25, [x29, #-0x80]
100b24fe8:     	ldr	x11, [sp, #0x88]
100b24fec:     	cmp	x25, x11
100b24ff0:     	b.ne	0x100b25654 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xa7c>
100b24ff4:     	sbfx	x20, x12, #0, #1
100b24ff8:     	cbz	w8, 0x100b25114 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x53c>
100b24ffc:     	cmp	w8, #0x2
100b25000:     	ldr	x22, [sp, #0x80]
100b25004:     	ldr	x25, [sp, #0x48]
100b25008:     	ldr	x27, [sp, #0x58]
100b2500c:     	b.ne	0x100b25030 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x458>
100b25010:     	ldr	x8, [sp, #0x30]
100b25014:     	cmp	x8, #0x1
100b25018:     	b.hi	0x100b25670 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xa98>
100b2501c:     	add	x1, x24, #0x18
100b25020:     	mov	x0, x26
100b25024:     	ldr	x2, [sp, #0x28]
100b25028:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100b2502c:     	ldr	x11, [sp, #0x88]
100b25030:     	ldr	x8, [sp, #0x50]
100b25034:     	add	x8, x8, #0x1
100b25038:     	str	x8, [sp, #0x50]
100b2503c:     	str	x8, [x21, #0x28]
100b25040:     	mov	w8, #0x2                ; =2
100b25044:     	ldr	x26, [sp, #0x40]
100b25048:     	ldr	w10, [sp, #0x7c]
100b2504c:     	b	0x100b24de8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x210>
100b25050:     	ldr	x0, [sp, #0x90]
100b25054:     	mov	x1, x27
100b25058:     	mov	x4, x26
100b2505c:     	bl	0x100d21824 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into>
100b25060:     	mov	w8, #0x1                ; =1
100b25064:     	add	x23, x23, #0x1
100b25068:     	str	x23, [x21, #0x40]
100b2506c:     	orr	x25, x22, x25
100b25070:     	cmp	x22, x20
100b25074:     	eor	x20, x22, x20
100b25078:     	mov	x12, x28
100b2507c:     	b.eq	0x100b24fe4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x40c>
100b25080:     	fmov	d0, x25
100b25084:     	cnt.8b	v0, v0
100b25088:     	addv.8b	b0, v0
100b2508c:     	fmov	x2, d0
100b25090:     	and	w9, w2, #0x3e
100b25094:     	lsl	x10, x19, x2
100b25098:     	lsr	x10, x10, #6
100b2509c:     	cmp	w9, #0x6
100b250a0:     	cinc	x1, x10, lo
100b250a4:     	add	w9, w2, #0x1
100b250a8:     	and	w10, w9, #0x3f
100b250ac:     	lsl	x9, x19, x9
100b250b0:     	lsr	x9, x9, #6
100b250b4:     	cmp	w10, #0x6
100b250b8:     	cinc	x5, x9, lo
100b250bc:     	cmp	x1, #0x1
100b250c0:     	ccmp	x5, #0x1, #0x2, ls
100b250c4:     	b.hi	0x100b2545c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x884>
100b250c8:     	neg	x9, x20
100b250cc:     	and	x22, x20, x9
100b250d0:     	sub	x9, x22, #0x1
100b250d4:     	and	x9, x9, x25
100b250d8:     	fmov	d0, x9
100b250dc:     	cnt.8b	v0, v0
100b250e0:     	addv.8b	b0, v0
100b250e4:     	fmov	w3, s0
100b250e8:     	ands	w8, w8, #0xff
100b250ec:     	b.eq	0x100b25050 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x478>
100b250f0:     	cmp	w8, #0x1
100b250f4:     	b.ne	0x100b2510c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x534>
100b250f8:     	add	x4, x24, #0x18
100b250fc:     	mov	x0, x26
100b25100:     	bl	0x100d21824 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into>
100b25104:     	mov	w8, #0x2                ; =2
100b25108:     	b	0x100b25064 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x48c>
100b2510c:     	add	x0, x24, #0x18
100b25110:     	b	0x100b25058 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x480>
100b25114:     	ldp	x8, x26, [sp, #0x38]
100b25118:     	add	x8, x8, #0x1
100b2511c:     	str	x8, [sp, #0x38]
100b25120:     	str	x8, [x21, #0x20]
100b25124:     	mov	w8, #0x1                ; =1
100b25128:     	ldr	x9, [sp, #0x90]
100b2512c:     	stp	x9, x27, [sp, #0x68]
100b25130:     	ldr	x22, [sp, #0x80]
100b25134:     	ldr	x25, [sp, #0x48]
100b25138:     	ldr	x27, [sp, #0x58]
100b2513c:     	ldr	w10, [sp, #0x7c]
100b25140:     	b	0x100b24de8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x210>
100b25144:     	mov	x28, x20
100b25148:     	mov	x8, #0x0                ; =0
100b2514c:     	add	x20, sp, #0x180
100b25150:     	lsl	x9, x24, #2
100b25154:     	cmp	x9, x8
100b25158:     	b.eq	0x100b25688 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xab0>
100b2515c:     	ldr	w23, [x25, x8]
100b25160:     	lsr	x10, x11, x23
100b25164:     	add	x8, x8, #0x4
100b25168:     	tbz	w10, #0x0, 0x100b25154 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x57c>
100b2516c:     	ldr	q0, [x28]
100b25170:     	str	q0, [sp, #0x100]
100b25174:     	ldr	x8, [x28, #0x10]
100b25178:     	str	x8, [sp, #0x110]
100b2517c:     	add	x0, sp, #0x160
100b25180:     	add	x1, sp, #0x100
100b25184:     	mov	x2, x26
100b25188:     	mov	x3, x23
100b2518c:     	mov	w4, #0x0                ; =0
100b25190:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25194:     	ldr	q0, [sp, #0x160]
100b25198:     	ldr	x8, [sp, #0x170]
100b2519c:     	stur	x8, [x29, #-0x90]
100b251a0:     	str	q0, [sp, #0xa0]
100b251a4:     	str	x8, [sp, #0xb0]
100b251a8:     	str	q0, [sp, #0x180]
100b251ac:     	str	x8, [sp, #0x190]
100b251b0:     	ldur	q0, [x28, #0x18]
100b251b4:     	str	q0, [sp, #0x100]
100b251b8:     	ldr	x8, [x28, #0x28]
100b251bc:     	str	x8, [sp, #0x110]
100b251c0:     	add	x0, sp, #0x160
100b251c4:     	add	x1, sp, #0x100
100b251c8:     	mov	x2, x26
100b251cc:     	mov	x3, x23
100b251d0:     	mov	w4, #0x0                ; =0
100b251d4:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b251d8:     	ldr	q0, [sp, #0x160]
100b251dc:     	ldr	x8, [sp, #0x170]
100b251e0:     	stur	x8, [x29, #-0x90]
100b251e4:     	str	q0, [sp, #0xa0]
100b251e8:     	str	x8, [sp, #0xb0]
100b251ec:     	stur	q0, [x20, #0x18]
100b251f0:     	str	x8, [sp, #0x1a8]
100b251f4:     	ldr	q0, [x19]
100b251f8:     	str	q0, [sp, #0x100]
100b251fc:     	ldr	x8, [x19, #0x10]
100b25200:     	str	x8, [sp, #0x110]
100b25204:     	add	x0, sp, #0x160
100b25208:     	add	x1, sp, #0x100
100b2520c:     	mov	x2, x26
100b25210:     	mov	x27, x23
100b25214:     	mov	x3, x23
100b25218:     	mov	w4, #0x0                ; =0
100b2521c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25220:     	ldr	q0, [sp, #0x160]
100b25224:     	ldr	x8, [sp, #0x170]
100b25228:     	stur	x8, [x29, #-0x90]
100b2522c:     	str	q0, [sp, #0xa0]
100b25230:     	str	q0, [sp, #0x1b0]
100b25234:     	str	x8, [sp, #0x1c0]
100b25238:     	add	x3, sp, #0x180
100b2523c:     	mov	x0, x26
100b25240:     	mov	x1, x25
100b25244:     	mov	x2, x24
100b25248:     	ldr	x20, [sp, #0x20]
100b2524c:     	mov	x4, x20
100b25250:     	mov	x5, x21
100b25254:     	ldr	x22, [sp, #0x98]
100b25258:     	mov	x6, x22
100b2525c:     	bl	0x100b24bd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_>
100b25260:     	and	w8, w0, #0xff
100b25264:     	cmp	w8, #0xf
100b25268:     	b.ne	0x100b25318 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x740>
100b2526c:     	mov	w22, #0xf               ; =15
100b25270:     	b	0x100b25428 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x850>
100b25274:     	mov	x8, #-0x1               ; =-1
100b25278:     	ldp	x9, x10, [sp, #0x10]
100b2527c:     	lsl	x9, x8, x9
100b25280:     	cmp	x10, #0x6
100b25284:     	csinv	x11, x8, x9, eq
100b25288:     	ldr	x8, [sp, #0x30]
100b2528c:     	cbz	x8, 0x100b25474 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x89c>
100b25290:     	ldrb	w4, [sp, #0xa0]
100b25294:     	ldp	x3, x9, [sp, #0xb0]
100b25298:     	ldr	x2, [sp, #0xa8]
100b2529c:     	ldrb	w8, [sp, #0xa1]
100b252a0:     	neg	x12, x8
100b252a4:     	ldrb	w25, [sp, #0xc0]
100b252a8:     	ldp	x17, x8, [sp, #0xd0]
100b252ac:     	ldr	x16, [sp, #0xc8]
100b252b0:     	ldrb	w5, [sp, #0xc1]
100b252b4:     	ldrb	w15, [sp, #0xe0]
100b252b8:     	ldp	x14, x1, [sp, #0xf0]
100b252bc:     	ldr	x13, [sp, #0xe8]
100b252c0:     	ldr	x21, [sp, #0x60]
100b252c4:     	ldr	x7, [x21, #0x18]
100b252c8:     	add	x10, x7, #0x1
100b252cc:     	mov	x19, x12
100b252d0:     	ldrb	w6, [sp, #0xe1]
100b252d4:     	ldr	x0, [sp, #0x98]
100b252d8:     	cbz	w4, 0x100b252f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x720>
100b252dc:     	mov	x19, x0
100b252e0:     	cmp	w4, #0x2
100b252e4:     	b.eq	0x100b252f0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x718>
100b252e8:     	mov	x19, x3
100b252ec:     	cbz	x9, 0x100b256f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb1c>
100b252f0:     	ldr	x19, [x19]
100b252f4:     	eor	x19, x19, x2
100b252f8:     	neg	x5, x5
100b252fc:     	mov	x20, x5
100b25300:     	cbz	w25, 0x100b2548c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8b4>
100b25304:     	cmp	w25, #0x1
100b25308:     	b.ne	0x100b25484 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8ac>
100b2530c:     	cbz	x8, 0x100b256f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb1c>
100b25310:     	ldr	x20, [x17]
100b25314:     	b	0x100b25488 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8b0>
100b25318:     	mov	x23, x0
100b2531c:     	ldr	q0, [x28]
100b25320:     	str	q0, [sp, #0x100]
100b25324:     	ldr	x8, [x28, #0x10]
100b25328:     	str	x8, [sp, #0x110]
100b2532c:     	add	x0, sp, #0x160
100b25330:     	add	x1, sp, #0x100
100b25334:     	mov	x2, x26
100b25338:     	mov	x3, x27
100b2533c:     	mov	w4, #0x1                ; =1
100b25340:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25344:     	ldr	q0, [sp, #0x160]
100b25348:     	stur	q0, [x29, #-0x80]
100b2534c:     	ldr	x8, [sp, #0x170]
100b25350:     	stur	q0, [x29, #-0xa0]
100b25354:     	str	q0, [sp, #0xa0]
100b25358:     	str	x8, [sp, #0xb0]
100b2535c:     	ldr	q0, [sp, #0xa0]
100b25360:     	stur	x8, [x29, #-0xe0]
100b25364:     	stur	q0, [x29, #-0xf0]
100b25368:     	ldur	q0, [x28, #0x18]
100b2536c:     	str	q0, [sp, #0x100]
100b25370:     	ldur	x8, [x28, #0x28]
100b25374:     	str	x8, [sp, #0x110]
100b25378:     	add	x0, sp, #0x160
100b2537c:     	add	x1, sp, #0x100
100b25380:     	mov	x2, x26
100b25384:     	mov	x3, x27
100b25388:     	mov	w4, #0x1                ; =1
100b2538c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25390:     	ldr	q0, [sp, #0x160]
100b25394:     	stur	q0, [x29, #-0x80]
100b25398:     	ldr	x8, [sp, #0x170]
100b2539c:     	stur	q0, [x29, #-0xa0]
100b253a0:     	str	q0, [sp, #0xa0]
100b253a4:     	str	x8, [sp, #0xb0]
100b253a8:     	ldr	q0, [sp, #0xa0]
100b253ac:     	stur	x8, [x29, #-0xc8]
100b253b0:     	add	x8, sp, #0x180
100b253b4:     	stur	q0, [x8, #0x68]
100b253b8:     	ldr	q0, [x19]
100b253bc:     	str	q0, [sp, #0x100]
100b253c0:     	ldr	x8, [x19, #0x10]
100b253c4:     	str	x8, [sp, #0x110]
100b253c8:     	add	x0, sp, #0x160
100b253cc:     	add	x1, sp, #0x100
100b253d0:     	mov	x2, x26
100b253d4:     	mov	x3, x27
100b253d8:     	mov	w4, #0x1                ; =1
100b253dc:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b253e0:     	ldr	q0, [sp, #0x160]
100b253e4:     	stur	q0, [x29, #-0x80]
100b253e8:     	ldr	x8, [sp, #0x170]
100b253ec:     	stur	q0, [x29, #-0xa0]
100b253f0:     	str	q0, [sp, #0xa0]
100b253f4:     	str	x8, [sp, #0xb0]
100b253f8:     	ldr	q0, [sp, #0xa0]
100b253fc:     	stur	x8, [x29, #-0xb0]
100b25400:     	stur	q0, [x29, #-0xc0]
100b25404:     	sub	x3, x29, #0xf0
100b25408:     	mov	x0, x26
100b2540c:     	mov	x1, x25
100b25410:     	mov	x2, x24
100b25414:     	mov	x4, x20
100b25418:     	mov	x5, x21
100b2541c:     	mov	x6, x22
100b25420:     	bl	0x100b24bd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_>
100b25424:     	orr	w22, w0, w23
100b25428:     	mov	x1, x28
100b2542c:     	mov	x0, x20
100b25430:     	mov	x2, x22
100b25434:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b25438:     	mov	x0, x22
100b2543c:     	add	sp, sp, #0x270
100b25440:     	ldp	x29, x30, [sp, #0x50]
100b25444:     	ldp	x20, x19, [sp, #0x40]
100b25448:     	ldp	x22, x21, [sp, #0x30]
100b2544c:     	ldp	x24, x23, [sp, #0x20]
100b25450:     	ldp	x26, x25, [sp, #0x10]
100b25454:     	ldp	x28, x27, [sp], #0x60
100b25458:     	ret
100b2545c:     	adrp	x0, 0x10132c000 <dyld_stub_binder+0x10132c000>
100b25460:     	add	x0, x0, #0xbf0
100b25464:     	adrp	x2, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b25468:     	add	x2, x2, #0x238
100b2546c:     	mov	w1, #0x2b               ; =43
100b25470:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b25474:     	mov	w22, #0x0               ; =0
100b25478:     	ldr	x1, [sp, #0x8]
100b2547c:     	ldr	x0, [sp, #0x20]
100b25480:     	b	0x100b25430 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x858>
100b25484:     	ldr	x20, [x0, #0x8]
100b25488:     	eor	x20, x20, x16
100b2548c:     	neg	x6, x6
100b25490:     	mov	x22, x6
100b25494:     	cbz	w15, 0x100b254b4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8dc>
100b25498:     	cmp	w15, #0x1
100b2549c:     	b.ne	0x100b254ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8d4>
100b254a0:     	cbz	x1, 0x100b25724 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb4c>
100b254a4:     	ldr	x22, [x14]
100b254a8:     	b	0x100b254b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x8d8>
100b254ac:     	ldr	x22, [x0, #0x10]
100b254b0:     	eor	x22, x22, x13
100b254b4:     	and	x19, x19, x11
100b254b8:     	bic	x23, x19, x20
100b254bc:     	tst	x22, x23
100b254c0:     	mov	w24, #0x2               ; =2
100b254c4:     	csel	w24, wzr, w24, eq
100b254c8:     	and	x19, x20, x19
100b254cc:     	bics	xzr, x19, x22
100b254d0:     	mov	w20, #0x4               ; =4
100b254d4:     	csel	w20, wzr, w20, eq
100b254d8:     	tst	x22, x19
100b254dc:     	mov	w19, #0x8               ; =8
100b254e0:     	csel	w19, wzr, w19, eq
100b254e4:     	bics	xzr, x23, x22
100b254e8:     	cinc	w22, w24, ne
100b254ec:     	orr	w19, w20, w19
100b254f0:     	orr	w22, w22, w19
100b254f4:     	cmp	w22, #0xf
100b254f8:     	b.ne	0x100b2550c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x934>
100b254fc:     	ldr	x1, [sp, #0x8]
100b25500:     	ldr	x0, [sp, #0x20]
100b25504:     	str	x10, [x21, #0x18]
100b25508:     	b	0x100b25430 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x858>
100b2550c:     	ldr	x0, [sp, #0x30]
100b25510:     	add	x10, x0, x7
100b25514:     	cmp	x0, #0x1
100b25518:     	ldr	x19, [sp, #0x8]
100b2551c:     	ldr	x0, [sp, #0x20]
100b25520:     	b.eq	0x100b255c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x9f0>
100b25524:     	add	x7, x7, #0x2
100b25528:     	cbz	w4, 0x100b25544 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x96c>
100b2552c:     	cmp	w4, #0x1
100b25530:     	b.ne	0x100b2570c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb34>
100b25534:     	cmp	x9, #0x1
100b25538:     	b.ls	0x100b25738 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb60>
100b2553c:     	ldr	x9, [x3, #0x8]
100b25540:     	eor	x12, x9, x2
100b25544:     	cbz	w25, 0x100b25560 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x988>
100b25548:     	cmp	w25, #0x1
100b2554c:     	b.ne	0x100b2570c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb34>
100b25550:     	cmp	x8, #0x1
100b25554:     	b.ls	0x100b25750 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb78>
100b25558:     	ldr	x8, [x17, #0x8]
100b2555c:     	eor	x5, x8, x16
100b25560:     	cbz	w15, 0x100b2557c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x9a4>
100b25564:     	cmp	w15, #0x1
100b25568:     	b.ne	0x100b2570c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb34>
100b2556c:     	cmp	x1, #0x1
100b25570:     	b.ls	0x100b25768 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0xb90>
100b25574:     	ldr	x8, [x14, #0x8]
100b25578:     	eor	x6, x8, x13
100b2557c:     	and	x8, x12, x11
100b25580:     	bic	x9, x8, x5
100b25584:     	tst	x6, x9
100b25588:     	mov	w11, #0x2               ; =2
100b2558c:     	csel	w11, wzr, w11, eq
100b25590:     	and	x8, x5, x8
100b25594:     	bics	xzr, x8, x6
100b25598:     	mov	w12, #0x4               ; =4
100b2559c:     	csel	w12, wzr, w12, eq
100b255a0:     	tst	x6, x8
100b255a4:     	mov	w8, #0x8                ; =8
100b255a8:     	csel	w8, wzr, w8, eq
100b255ac:     	bics	xzr, x9, x6
100b255b0:     	cinc	w9, w11, ne
100b255b4:     	orr	w8, w12, w8
100b255b8:     	orr	w8, w9, w8
100b255bc:     	orr	w22, w8, w22
100b255c0:     	cmp	w22, #0xf
100b255c4:     	csel	x10, x7, x10, eq
100b255c8:     	mov	x1, x19
100b255cc:     	str	x10, [x21, #0x18]
100b255d0:     	b	0x100b25430 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_+0x858>
100b255d4:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b255d8:     	add	x2, x2, #0x788
100b255dc:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b255e0:     	add	x5, x5, #0x3e8
100b255e4:     	add	x1, sp, #0x160
100b255e8:     	mov	w0, #0x0                ; =0
100b255ec:     	mov	x3, #0x0                ; =0
100b255f0:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b255f4:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b255f8:     	add	x2, x2, #0x788
100b255fc:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b25600:     	add	x5, x5, #0x3d0
100b25604:     	add	x1, sp, #0x160
100b25608:     	mov	w0, #0x0                ; =0
100b2560c:     	mov	x3, #0x0                ; =0
100b25610:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b25614:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b25618:     	add	x2, x2, #0x788
100b2561c:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b25620:     	add	x5, x5, #0x3b8
100b25624:     	add	x1, sp, #0x160
100b25628:     	mov	w0, #0x0                ; =0
100b2562c:     	mov	x3, #0x0                ; =0
100b25630:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b25634:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b25638:     	add	x2, x2, #0x788
100b2563c:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b25640:     	add	x5, x5, #0x3a0
100b25644:     	add	x1, sp, #0x160
100b25648:     	mov	w0, #0x0                ; =0
100b2564c:     	mov	x3, #0x0                ; =0
100b25650:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b25654:     	adrp	x5, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b25658:     	add	x5, x5, #0x370
100b2565c:     	sub	x1, x29, #0x80
100b25660:     	sub	x2, x29, #0xa0
100b25664:     	mov	w0, #0x0                ; =0
100b25668:     	mov	x3, #0x0                ; =0
100b2566c:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b25670:     	adrp	x3, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b25674:     	add	x3, x3, #0x388
100b25678:     	mov	x0, #0x0                ; =0
100b2567c:     	mov	w1, #0x2                ; =2
100b25680:     	mov	w2, #0x1                ; =1
100b25684:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100b25688:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b2568c:     	add	x0, x0, #0xd90
100b25690:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b25694:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100b25698:     	add	x0, x0, #0xd49
100b2569c:     	adrp	x2, 0x1014c7000 <dyld_stub_binder+0x1014c7000>
100b256a0:     	add	x2, x2, #0x400
100b256a4:     	mov	w1, #0xc9               ; =201
100b256a8:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b256ac:     	mov	x0, x8
100b256b0:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b256b4:     	add	x2, x2, #0x6c0
100b256b8:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b256bc:     	mov	x0, x10
100b256c0:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b256c4:     	add	x2, x2, #0x6c0
100b256c8:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b256cc:     	mov	x0, x11
100b256d0:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b256d4:     	add	x2, x2, #0x6a8
100b256d8:     	mov	x1, x8
100b256dc:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b256e0:     	mov	x0, x10
100b256e4:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b256e8:     	add	x2, x2, #0x6a8
100b256ec:     	mov	x1, x8
100b256f0:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b256f4:     	str	x10, [x21, #0x18]
100b256f8:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b256fc:     	add	x2, x2, #0x6d8
100b25700:     	mov	x0, #0x0                ; =0
100b25704:     	mov	x1, #0x0                ; =0
100b25708:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2570c:     	str	x7, [x21, #0x18]
100b25710:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25714:     	add	x2, x2, #0x6f0
100b25718:     	mov	w0, #0x1                ; =1
100b2571c:     	mov	w1, #0x1                ; =1
100b25720:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25724:     	str	x10, [x21, #0x18]
100b25728:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b2572c:     	add	x2, x2, #0x6d8
100b25730:     	mov	x0, #0x0                ; =0
100b25734:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25738:     	str	x7, [x21, #0x18]
100b2573c:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25740:     	add	x2, x2, #0x6d8
100b25744:     	mov	w0, #0x1                ; =1
100b25748:     	mov	x1, x9
100b2574c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25750:     	str	x7, [x21, #0x18]
100b25754:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25758:     	add	x2, x2, #0x6d8
100b2575c:     	mov	w0, #0x1                ; =1
100b25760:     	mov	x1, x8
100b25764:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25768:     	str	x7, [x21, #0x18]
100b2576c:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25770:     	add	x2, x2, #0x6d8
100b25774:     	mov	w0, #0x1                ; =1
100b25778:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2577c:     	nop
