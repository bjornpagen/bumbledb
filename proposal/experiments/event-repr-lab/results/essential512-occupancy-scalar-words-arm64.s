
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b22c00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_>:
100b22c00:     	stp	d15, d14, [sp, #-0xa0]!
100b22c04:     	stp	d13, d12, [sp, #0x10]
100b22c08:     	stp	d11, d10, [sp, #0x20]
100b22c0c:     	stp	d9, d8, [sp, #0x30]
100b22c10:     	stp	x28, x27, [sp, #0x40]
100b22c14:     	stp	x26, x25, [sp, #0x50]
100b22c18:     	stp	x24, x23, [sp, #0x60]
100b22c1c:     	stp	x22, x21, [sp, #0x70]
100b22c20:     	stp	x20, x19, [sp, #0x80]
100b22c24:     	stp	x29, x30, [sp, #0x90]
100b22c28:     	add	x29, sp, #0x90
100b22c2c:     	sub	sp, sp, #0x230
100b22c30:     	ldr	w8, [x3, #0x10]
100b22c34:     	str	x8, [sp, #0xe0]
100b22c38:     	cbz	w8, 0x100b22c6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x6c>
100b22c3c:     	mov	x21, x5
100b22c40:     	mov	x23, x4
100b22c44:     	mov	x24, x3
100b22c48:     	mov	x26, x2
100b22c4c:     	mov	x27, x1
100b22c50:     	mov	x28, x0
100b22c54:     	mov	x0, x4
100b22c58:     	mov	x1, x3
100b22c5c:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b22c60:     	cbz	x0, 0x100b22c74 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x74>
100b22c64:     	ldrb	w27, [x0]
100b22c68:     	b	0x100b2341c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x81c>
100b22c6c:     	mov	w27, #0x0               ; =0
100b22c70:     	b	0x100b2341c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x81c>
100b22c74:     	ldr	x8, [x21]
100b22c78:     	add	x8, x8, #0x1
100b22c7c:     	str	x8, [x21]
100b22c80:     	ldr	x8, [x28, #0x30]
100b22c84:     	ldr	x1, [x28, #0x40]
100b22c88:     	ldr	x9, [x24]
100b22c8c:     	ldr	x10, [sp, #0xe0]
100b22c90:     	lsr	x0, x10, #1
100b22c94:     	cmn	x8, #0x1
100b22c98:     	b.eq	0x100b22cf4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0xf4>
100b22c9c:     	cmp	x1, x0
100b22ca0:     	b.ls	0x100b23470 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x870>
100b22ca4:     	ldr	w8, [x24, #0x28]
100b22ca8:     	lsr	x12, x8, #1
100b22cac:     	cmp	x1, x12
100b22cb0:     	b.ls	0x100b2345c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x85c>
100b22cb4:     	ldr	w10, [x24, #0x40]
100b22cb8:     	lsr	x11, x10, #1
100b22cbc:     	cmp	x1, x11
100b22cc0:     	b.ls	0x100b2346c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x86c>
100b22cc4:     	ldr	x13, [x28, #0x38]
100b22cc8:     	lsl	x12, x12, #4
100b22ccc:     	ldr	x12, [x13, x12]
100b22cd0:     	ldr	x14, [x24, #0x18]
100b22cd4:     	lsl	x15, x0, #4
100b22cd8:     	ldr	x15, [x13, x15]
100b22cdc:     	bic	x12, x12, x14
100b22ce0:     	bic	x9, x15, x9
100b22ce4:     	orr	x9, x12, x9
100b22ce8:     	add	x11, x13, x11, lsl #4
100b22cec:     	stp	x10, x8, [sp, #0xc0]
100b22cf0:     	b	0x100b22d50 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x150>
100b22cf4:     	ldr	x8, [x28, #0x48]
100b22cf8:     	cmp	x8, x0
100b22cfc:     	b.ls	0x100b23494 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x894>
100b22d00:     	ldr	w10, [x24, #0x28]
100b22d04:     	str	x10, [sp, #0xc8]
100b22d08:     	lsr	x11, x10, #1
100b22d0c:     	cmp	x8, x11
100b22d10:     	b.ls	0x100b2347c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x87c>
100b22d14:     	ldr	w10, [x24, #0x40]
100b22d18:     	str	x10, [sp, #0xc0]
100b22d1c:     	lsr	x10, x10, #1
100b22d20:     	cmp	x8, x10
100b22d24:     	b.ls	0x100b23490 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x890>
100b22d28:     	add	x8, x1, x11, lsl #5
100b22d2c:     	ldr	x8, [x8, #0x18]
100b22d30:     	ldr	x11, [x24, #0x18]
100b22d34:     	bic	x8, x8, x11
100b22d38:     	add	x11, x1, x0, lsl #5
100b22d3c:     	ldr	x11, [x11, #0x18]
100b22d40:     	bic	x9, x11, x9
100b22d44:     	orr	x9, x8, x9
100b22d48:     	add	x8, x1, x10, lsl #5
100b22d4c:     	add	x11, x8, #0x18
100b22d50:     	ldr	x8, [x11]
100b22d54:     	mov	x19, x24
100b22d58:     	ldr	x10, [x19, #0x30]!
100b22d5c:     	bic	x8, x8, x10
100b22d60:     	orr	x20, x8, x9
100b22d64:     	fmov	d0, x20
100b22d68:     	cnt.8b	v0, v0
100b22d6c:     	addv.8b	b0, v0
100b22d70:     	fmov	x8, d0
100b22d74:     	cmp	x8, #0xa
100b22d78:     	str	x28, [sp, #0xd8]
100b22d7c:     	b.hs	0x100b22df4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x1f4>
100b22d80:     	str	x24, [sp, #0x18]
100b22d84:     	ldr	x8, [x21, #0x10]
100b22d88:     	add	x8, x8, #0x1
100b22d8c:     	str	x8, [x21, #0x10]
100b22d90:     	mov	w26, #0x4               ; =4
100b22d94:     	stp	xzr, x26, [x29, #-0xc0]
100b22d98:     	stur	xzr, [x29, #-0xb0]
100b22d9c:     	mov	w24, #0x1               ; =1
100b22da0:     	str	x23, [sp, #0x8]
100b22da4:     	str	x21, [sp, #0xd0]
100b22da8:     	mov	x22, #0x0               ; =0
100b22dac:     	cbz	x20, 0x100b23054 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x454>
100b22db0:     	mov	w8, #0x4                ; =4
100b22db4:     	b	0x100b22ddc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x1dc>
100b22db8:     	ldur	x8, [x29, #-0xb8]
100b22dbc:     	rbit	x9, x20
100b22dc0:     	clz	x9, x9
100b22dc4:     	str	w9, [x8, x22, lsl #2]
100b22dc8:     	add	x22, x22, #0x1
100b22dcc:     	stur	x22, [x29, #-0xb0]
100b22dd0:     	sub	x9, x20, #0x1
100b22dd4:     	ands	x20, x9, x20
100b22dd8:     	b.eq	0x100b22f28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x328>
100b22ddc:     	ldur	x9, [x29, #-0xc0]
100b22de0:     	cmp	x22, x9
100b22de4:     	b.ne	0x100b22dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x1bc>
100b22de8:     	sub	x0, x29, #0xc0
100b22dec:     	bl	0x101282ddc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b22df0:     	b	0x100b22db8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x1b8>
100b22df4:     	mov	x28, x21
100b22df8:     	mov	x9, #0x0                ; =0
100b22dfc:     	sub	x25, x29, #0xf8
100b22e00:     	lsl	x10, x26, #2
100b22e04:     	cmp	x10, x9
100b22e08:     	b.eq	0x100b23450 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x850>
100b22e0c:     	ldr	w8, [x27, x9]
100b22e10:     	lsr	x11, x20, x8
100b22e14:     	add	x9, x9, #0x4
100b22e18:     	tbz	w11, #0x0, 0x100b22e04 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x204>
100b22e1c:     	ldr	q0, [x24]
100b22e20:     	stur	q0, [x29, #-0xc0]
100b22e24:     	ldr	x9, [x24, #0x10]
100b22e28:     	stur	x9, [x29, #-0xb0]
100b22e2c:     	sub	x0, x29, #0xf8
100b22e30:     	sub	x1, x29, #0xc0
100b22e34:     	ldr	x21, [sp, #0xd8]
100b22e38:     	mov	x2, x21
100b22e3c:     	mov	x20, x8
100b22e40:     	mov	x3, x20
100b22e44:     	mov	w4, #0x0                ; =0
100b22e48:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22e4c:     	ldr	q0, [x25]
100b22e50:     	ldur	x8, [x29, #-0xe8]
100b22e54:     	str	x8, [sp, #0x1a0]
100b22e58:     	stur	q0, [x29, #-0xe0]
100b22e5c:     	stur	x8, [x29, #-0xd0]
100b22e60:     	str	q0, [sp, #0xf0]
100b22e64:     	str	x8, [sp, #0x100]
100b22e68:     	ldur	q0, [x24, #0x18]
100b22e6c:     	stur	q0, [x29, #-0xc0]
100b22e70:     	ldr	x8, [x24, #0x28]
100b22e74:     	stur	x8, [x29, #-0xb0]
100b22e78:     	sub	x0, x29, #0xf8
100b22e7c:     	sub	x1, x29, #0xc0
100b22e80:     	mov	x2, x21
100b22e84:     	mov	x3, x20
100b22e88:     	mov	w4, #0x0                ; =0
100b22e8c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22e90:     	ldr	q0, [x25]
100b22e94:     	ldur	x8, [x29, #-0xe8]
100b22e98:     	str	x8, [sp, #0x1a0]
100b22e9c:     	stur	q0, [x29, #-0xe0]
100b22ea0:     	stur	x8, [x29, #-0xd0]
100b22ea4:     	add	x9, sp, #0x9
100b22ea8:     	stur	q0, [x9, #0xff]
100b22eac:     	str	x8, [sp, #0x118]
100b22eb0:     	ldr	q0, [x19]
100b22eb4:     	stur	q0, [x29, #-0xc0]
100b22eb8:     	ldr	x8, [x19, #0x10]
100b22ebc:     	stur	x8, [x29, #-0xb0]
100b22ec0:     	sub	x0, x29, #0xf8
100b22ec4:     	sub	x1, x29, #0xc0
100b22ec8:     	mov	x2, x21
100b22ecc:     	mov	x22, x20
100b22ed0:     	mov	x3, x20
100b22ed4:     	mov	w4, #0x0                ; =0
100b22ed8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22edc:     	ldr	q0, [x25]
100b22ee0:     	ldur	x8, [x29, #-0xe8]
100b22ee4:     	str	x8, [sp, #0x1a0]
100b22ee8:     	stur	q0, [x29, #-0xe0]
100b22eec:     	str	q0, [sp, #0x120]
100b22ef0:     	str	x8, [sp, #0x130]
100b22ef4:     	add	x3, sp, #0xf0
100b22ef8:     	mov	x0, x21
100b22efc:     	mov	x1, x27
100b22f00:     	mov	x2, x26
100b22f04:     	mov	x4, x23
100b22f08:     	mov	x5, x28
100b22f0c:     	bl	0x100b22c00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_>
100b22f10:     	mov	x25, x23
100b22f14:     	and	w8, w0, #0xff
100b22f18:     	cmp	w8, #0xf
100b22f1c:     	b.ne	0x100b22f38 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x338>
100b22f20:     	mov	w27, #0xf               ; =15
100b22f24:     	b	0x100b2304c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x44c>
100b22f28:     	ldp	x8, x26, [x29, #-0xc0]
100b22f2c:     	cmp	x8, #0x0
100b22f30:     	cset	w8, eq
100b22f34:     	b	0x100b23058 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x458>
100b22f38:     	mov	x23, x0
100b22f3c:     	ldr	q0, [x24]
100b22f40:     	stur	q0, [x29, #-0xc0]
100b22f44:     	ldr	x8, [x24, #0x10]
100b22f48:     	stur	x8, [x29, #-0xb0]
100b22f4c:     	sub	x0, x29, #0xf8
100b22f50:     	sub	x1, x29, #0xc0
100b22f54:     	mov	x2, x21
100b22f58:     	mov	x20, x22
100b22f5c:     	mov	x3, x20
100b22f60:     	mov	w4, #0x1                ; =1
100b22f64:     	sub	x22, x29, #0xf8
100b22f68:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22f6c:     	ldr	q0, [x22]
100b22f70:     	str	q0, [sp, #0x1b0]
100b22f74:     	ldur	x8, [x29, #-0xe8]
100b22f78:     	str	q0, [sp, #0x190]
100b22f7c:     	stur	q0, [x29, #-0xe0]
100b22f80:     	stur	x8, [x29, #-0xd0]
100b22f84:     	ldur	q0, [x29, #-0xe0]
100b22f88:     	str	x8, [sp, #0x150]
100b22f8c:     	str	q0, [sp, #0x140]
100b22f90:     	ldur	q0, [x24, #0x18]
100b22f94:     	stur	q0, [x29, #-0xc0]
100b22f98:     	ldur	x8, [x24, #0x28]
100b22f9c:     	stur	x8, [x29, #-0xb0]
100b22fa0:     	sub	x0, x29, #0xf8
100b22fa4:     	sub	x1, x29, #0xc0
100b22fa8:     	mov	x2, x21
100b22fac:     	mov	x3, x20
100b22fb0:     	mov	w4, #0x1                ; =1
100b22fb4:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22fb8:     	ldr	q0, [x22]
100b22fbc:     	str	q0, [sp, #0x1b0]
100b22fc0:     	ldur	x8, [x29, #-0xe8]
100b22fc4:     	str	q0, [sp, #0x190]
100b22fc8:     	stur	q0, [x29, #-0xe0]
100b22fcc:     	stur	x8, [x29, #-0xd0]
100b22fd0:     	ldur	q0, [x29, #-0xe0]
100b22fd4:     	str	x8, [sp, #0x168]
100b22fd8:     	add	x8, sp, #0x59
100b22fdc:     	stur	q0, [x8, #0xff]
100b22fe0:     	ldr	q0, [x19]
100b22fe4:     	stur	q0, [x29, #-0xc0]
100b22fe8:     	ldr	x8, [x19, #0x10]
100b22fec:     	stur	x8, [x29, #-0xb0]
100b22ff0:     	sub	x0, x29, #0xf8
100b22ff4:     	sub	x1, x29, #0xc0
100b22ff8:     	mov	x2, x21
100b22ffc:     	mov	x3, x20
100b23000:     	mov	w4, #0x1                ; =1
100b23004:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23008:     	ldr	q0, [x22]
100b2300c:     	str	q0, [sp, #0x1b0]
100b23010:     	ldur	x8, [x29, #-0xe8]
100b23014:     	str	q0, [sp, #0x190]
100b23018:     	stur	q0, [x29, #-0xe0]
100b2301c:     	stur	x8, [x29, #-0xd0]
100b23020:     	ldur	q0, [x29, #-0xe0]
100b23024:     	str	x8, [sp, #0x180]
100b23028:     	str	q0, [sp, #0x170]
100b2302c:     	add	x3, sp, #0x140
100b23030:     	mov	x0, x21
100b23034:     	mov	x1, x27
100b23038:     	mov	x2, x26
100b2303c:     	mov	x4, x25
100b23040:     	mov	x5, x28
100b23044:     	bl	0x100b22c00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_>
100b23048:     	orr	w27, w0, w23
100b2304c:     	mov	x0, x25
100b23050:     	b	0x100b23410 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x810>
100b23054:     	mov	w8, #0x1                ; =1
100b23058:     	str	w8, [sp, #0x14]
100b2305c:     	mov	w27, #0x0               ; =0
100b23060:     	mov	x21, #0x0               ; =0
100b23064:     	ldr	x8, [sp, #0x18]
100b23068:     	ldr	x10, [x8, #0x8]
100b2306c:     	ldr	x9, [x8, #0x20]
100b23070:     	stp	x9, x10, [sp, #0xb0]
100b23074:     	ldr	x8, [x8, #0x38]
100b23078:     	str	x8, [sp, #0xa8]
100b2307c:     	ldr	x8, [sp, #0xd0]
100b23080:     	ldr	x8, [x8, #0x8]
100b23084:     	str	x8, [sp, #0xe8]
100b23088:     	and	x8, x22, #0xfffffffffffffffe
100b2308c:     	neg	x8, x8
100b23090:     	str	x8, [sp, #0x88]
100b23094:     	mov	w23, #0x2               ; =2
100b23098:     	adrp	x8, 0x10131c000 <GCC_except_table9305+0x108>
100b2309c:     	ldr	q0, [x8, #0x240]
100b230a0:     	str	q0, [sp, #0x90]
100b230a4:     	mov	w8, #0x4                ; =4
100b230a8:     	dup.2d	v1, x8
100b230ac:     	mov	w8, #0x8                ; =8
100b230b0:     	dup.2d	v0, x8
100b230b4:     	stp	q0, q1, [sp, #0x50]
100b230b8:     	mov	w8, #0xc                ; =12
100b230bc:     	dup.2d	v1, x8
100b230c0:     	mov	w8, #0x10               ; =16
100b230c4:     	dup.2d	v0, x8
100b230c8:     	stp	q0, q1, [sp, #0x30]
100b230cc:     	adrp	x8, 0x10131c000 <GCC_except_table9305+0x108>
100b230d0:     	ldr	q0, [x8, #0x260]
100b230d4:     	str	q0, [sp, #0x20]
100b230d8:     	mov	w8, #0x3f               ; =63
100b230dc:     	dup.2d	v0, x8
100b230e0:     	str	q0, [sp, #0x70]
100b230e4:     	movi.2s	v8, #0x3f
100b230e8:     	b	0x100b23100 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x500>
100b230ec:     	add	x21, x21, #0x1
100b230f0:     	and	x8, x22, #0x3f
100b230f4:     	lsr	x8, x21, x8
100b230f8:     	ldr	x28, [sp, #0xd8]
100b230fc:     	cbnz	x8, 0x100b233f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x7f8>
100b23100:     	ldr	x9, [sp, #0xe8]
100b23104:     	add	x9, x9, #0x1
100b23108:     	ldr	x8, [sp, #0xd0]
100b2310c:     	str	x9, [sp, #0xe8]
100b23110:     	str	x9, [x8, #0x8]
100b23114:     	mov	x25, x22
100b23118:     	cbz	x22, 0x100b2338c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x78c>
100b2311c:     	cmp	x22, #0x1
100b23120:     	b.ne	0x100b23130 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x530>
100b23124:     	mov	x8, #0x0                ; =0
100b23128:     	mov	x25, #0x0               ; =0
100b2312c:     	b	0x100b2336c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x76c>
100b23130:     	dup.2d	v0, x21
100b23134:     	cmp	x22, #0x10
100b23138:     	b.hs	0x100b23148 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x548>
100b2313c:     	mov	x9, #0x0                ; =0
100b23140:     	mov	x25, #0x0               ; =0
100b23144:     	b	0x100b232f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x6f8>
100b23148:     	movi.2d	v1, #0000000000000000
100b2314c:     	add	x8, x26, #0x20
100b23150:     	movi.2d	v2, #0000000000000000
100b23154:     	and	x9, x22, #0x1ffffffffffffff0
100b23158:     	ldr	q4, [sp, #0x90]
100b2315c:     	ldp	q6, q15, [sp, #0x20]
100b23160:     	movi.2d	v3, #0000000000000000
100b23164:     	movi.2d	v7, #0000000000000000
100b23168:     	movi.2d	v16, #0000000000000000
100b2316c:     	movi.2d	v5, #0000000000000000
100b23170:     	movi.2d	v18, #0000000000000000
100b23174:     	movi.2d	v17, #0000000000000000
100b23178:     	ldp	q13, q12, [sp, #0x50]
100b2317c:     	ldr	q14, [sp, #0x40]
100b23180:     	mov	w10, #0x3f              ; =63
100b23184:     	movi.4s	v8, #0x3f
100b23188:     	add.2d	v19, v4, v12
100b2318c:     	add.2d	v20, v6, v12
100b23190:     	add.2d	v21, v4, v13
100b23194:     	add.2d	v22, v6, v13
100b23198:     	add.2d	v23, v4, v14
100b2319c:     	add.2d	v24, v6, v14
100b231a0:     	ldp	q25, q26, [x8, #-0x20]
100b231a4:     	dup.2d	v27, x10
100b231a8:     	ldp	q28, q29, [x8], #0x40
100b231ac:     	and.16b	v30, v6, v27
100b231b0:     	and.16b	v31, v4, v27
100b231b4:     	and.16b	v20, v20, v27
100b231b8:     	and.16b	v19, v19, v27
100b231bc:     	and.16b	v22, v22, v27
100b231c0:     	and.16b	v21, v21, v27
100b231c4:     	and.16b	v24, v24, v27
100b231c8:     	and.16b	v23, v23, v27
100b231cc:     	neg.2d	v27, v31
100b231d0:     	ushl.2d	v27, v0, v27
100b231d4:     	neg.2d	v30, v30
100b231d8:     	ushl.2d	v30, v0, v30
100b231dc:     	neg.2d	v19, v19
100b231e0:     	ushl.2d	v19, v0, v19
100b231e4:     	neg.2d	v20, v20
100b231e8:     	ushl.2d	v20, v0, v20
100b231ec:     	neg.2d	v21, v21
100b231f0:     	ushl.2d	v21, v0, v21
100b231f4:     	neg.2d	v22, v22
100b231f8:     	ushl.2d	v22, v0, v22
100b231fc:     	neg.2d	v23, v23
100b23200:     	ushl.2d	v23, v0, v23
100b23204:     	neg.2d	v24, v24
100b23208:     	ushl.2d	v24, v0, v24
100b2320c:     	dup.2d	v31, x24
100b23210:     	and.16b	v30, v30, v31
100b23214:     	and.16b	v27, v27, v31
100b23218:     	and.16b	v20, v20, v31
100b2321c:     	and.16b	v19, v19, v31
100b23220:     	and.16b	v22, v22, v31
100b23224:     	and.16b	v21, v21, v31
100b23228:     	and.16b	v24, v24, v31
100b2322c:     	and.16b	v23, v23, v31
100b23230:     	and.16b	v25, v25, v8
100b23234:     	and.16b	v26, v26, v8
100b23238:     	and.16b	v28, v28, v8
100b2323c:     	and.16b	v29, v29, v8
100b23240:     	ushll2.2d	v31, v25, #0x0
100b23244:     	ushll.2d	v25, v25, #0x0
100b23248:     	ushll2.2d	v9, v26, #0x0
100b2324c:     	ushll.2d	v26, v26, #0x0
100b23250:     	ushll2.2d	v10, v28, #0x0
100b23254:     	ushll.2d	v28, v28, #0x0
100b23258:     	ushll2.2d	v11, v29, #0x0
100b2325c:     	ushll.2d	v29, v29, #0x0
100b23260:     	ushl.2d	v25, v27, v25
100b23264:     	ushl.2d	v27, v30, v31
100b23268:     	ushl.2d	v19, v19, v26
100b2326c:     	ushl.2d	v20, v20, v9
100b23270:     	ushl.2d	v21, v21, v28
100b23274:     	ushl.2d	v22, v22, v10
100b23278:     	ushl.2d	v23, v23, v29
100b2327c:     	ushl.2d	v24, v24, v11
100b23280:     	orr.16b	v3, v27, v3
100b23284:     	orr.16b	v2, v25, v2
100b23288:     	orr.16b	v16, v20, v16
100b2328c:     	orr.16b	v7, v19, v7
100b23290:     	orr.16b	v18, v22, v18
100b23294:     	orr.16b	v5, v21, v5
100b23298:     	orr.16b	v1, v24, v1
100b2329c:     	orr.16b	v17, v23, v17
100b232a0:     	add.2d	v6, v6, v15
100b232a4:     	add.2d	v4, v4, v15
100b232a8:     	subs	x9, x9, #0x10
100b232ac:     	b.ne	0x100b23188 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x588>
100b232b0:     	orr.16b	v2, v7, v2
100b232b4:     	orr.16b	v3, v16, v3
100b232b8:     	orr.16b	v3, v18, v3
100b232bc:     	orr.16b	v2, v5, v2
100b232c0:     	orr.16b	v2, v17, v2
100b232c4:     	orr.16b	v1, v1, v3
100b232c8:     	orr.16b	v1, v2, v1
100b232cc:     	mov	d2, v1[1]
100b232d0:     	orr.8b	v1, v1, v2
100b232d4:     	fmov	x25, d1
100b232d8:     	and	x8, x22, #0x1ffffffffffffff0
100b232dc:     	cmp	x22, x8
100b232e0:     	movi.2s	v8, #0x3f
100b232e4:     	b.eq	0x100b2338c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x78c>
100b232e8:     	and	x9, x22, #0x1ffffffffffffff0
100b232ec:     	and	x8, x22, #0x1ffffffffffffff0
100b232f0:     	and	x10, x22, #0xe
100b232f4:     	cbz	x10, 0x100b2336c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x76c>
100b232f8:     	fmov	d1, x25
100b232fc:     	dup.2d	v2, x9
100b23300:     	ldr	q3, [sp, #0x90]
100b23304:     	orr.16b	v2, v2, v3
100b23308:     	ldr	x8, [sp, #0x88]
100b2330c:     	add	x8, x8, x9
100b23310:     	add	x9, x26, x9, lsl #2
100b23314:     	ldr	q6, [sp, #0x70]
100b23318:     	ldr	d3, [x9], #0x8
100b2331c:     	and.16b	v4, v2, v6
100b23320:     	neg.2d	v4, v4
100b23324:     	ushl.2d	v4, v0, v4
100b23328:     	dup.2d	v5, x24
100b2332c:     	and.16b	v4, v4, v5
100b23330:     	and.8b	v3, v3, v8
100b23334:     	ushll.2d	v3, v3, #0x0
100b23338:     	ushl.2d	v3, v4, v3
100b2333c:     	orr.16b	v1, v3, v1
100b23340:     	dup.2d	v3, x23
100b23344:     	add.2d	v2, v2, v3
100b23348:     	adds	x8, x8, #0x2
100b2334c:     	b.ne	0x100b23318 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x718>
100b23350:     	mov	d0, v1[1]
100b23354:     	orr.8b	v0, v1, v0
100b23358:     	fmov	x25, d0
100b2335c:     	and	x8, x22, #0x1ffffffffffffffe
100b23360:     	and	x9, x22, #0x1ffffffffffffffe
100b23364:     	cmp	x22, x9
100b23368:     	b.eq	0x100b2338c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x78c>
100b2336c:     	ldr	w9, [x26, x8, lsl #2]
100b23370:     	lsr	x10, x21, x8
100b23374:     	and	x10, x10, #0x1
100b23378:     	lsl	x9, x10, x9
100b2337c:     	orr	x25, x9, x25
100b23380:     	add	x8, x8, #0x1
100b23384:     	cmp	x22, x8
100b23388:     	b.ne	0x100b2336c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x76c>
100b2338c:     	ldr	x8, [sp, #0xb8]
100b23390:     	orr	x2, x8, x25
100b23394:     	mov	x0, x28
100b23398:     	ldr	x1, [sp, #0xe0]
100b2339c:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b233a0:     	mov	x19, x0
100b233a4:     	ldr	x8, [sp, #0xb0]
100b233a8:     	orr	x2, x8, x25
100b233ac:     	mov	x0, x28
100b233b0:     	ldr	x1, [sp, #0xc8]
100b233b4:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b233b8:     	mov	x20, x0
100b233bc:     	ldr	x8, [sp, #0xa8]
100b233c0:     	orr	x2, x8, x25
100b233c4:     	mov	x0, x28
100b233c8:     	ldr	x1, [sp, #0xc0]
100b233cc:     	bl	0x100b8ebe8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b233d0:     	tbz	w19, #0x0, 0x100b230ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x4ec>
100b233d4:     	cmp	w20, #0x0
100b233d8:     	csel	w8, w23, wzr, ne
100b233dc:     	orr	w8, w8, w0
100b233e0:     	lsl	w8, w24, w8
100b233e4:     	orr	w27, w8, w27
100b233e8:     	and	w8, w27, #0xff
100b233ec:     	cmp	w8, #0xf
100b233f0:     	b.ne	0x100b230ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x4ec>
100b233f4:     	mov	w27, #0xf               ; =15
100b233f8:     	ldr	w8, [sp, #0x14]
100b233fc:     	tbnz	w8, #0x0, 0x100b23408 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x808>
100b23400:     	mov	x0, x26
100b23404:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23408:     	ldr	x24, [sp, #0x18]
100b2340c:     	ldr	x0, [sp, #0x8]
100b23410:     	mov	x1, x24
100b23414:     	mov	x2, x27
100b23418:     	bl	0x100c293a0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b2341c:     	mov	x0, x27
100b23420:     	add	sp, sp, #0x230
100b23424:     	ldp	x29, x30, [sp, #0x90]
100b23428:     	ldp	x20, x19, [sp, #0x80]
100b2342c:     	ldp	x22, x21, [sp, #0x70]
100b23430:     	ldp	x24, x23, [sp, #0x60]
100b23434:     	ldp	x26, x25, [sp, #0x50]
100b23438:     	ldp	x28, x27, [sp, #0x40]
100b2343c:     	ldp	d9, d8, [sp, #0x30]
100b23440:     	ldp	d11, d10, [sp, #0x20]
100b23444:     	ldp	d13, d12, [sp, #0x10]
100b23448:     	ldp	d15, d14, [sp], #0xa0
100b2344c:     	ret
100b23450:     	adrp	x0, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b23454:     	add	x0, x0, #0xb78
100b23458:     	bl	0x101282074 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b2345c:     	mov	x0, x12
100b23460:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23464:     	add	x2, x2, #0x368
100b23468:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2346c:     	mov	x0, x11
100b23470:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23474:     	add	x2, x2, #0x368
100b23478:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2347c:     	mov	x0, x11
100b23480:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23484:     	add	x2, x2, #0x350
100b23488:     	mov	x1, x8
100b2348c:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23490:     	mov	x0, x10
100b23494:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23498:     	add	x2, x2, #0x350
100b2349c:     	mov	x1, x8
100b234a0:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b234a4:     	mov	x19, x0
100b234a8:     	ldur	x8, [x29, #-0xc0]
100b234ac:     	cbz	x8, 0x100b234cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x8cc>
100b234b0:     	ldur	x26, [x29, #-0xb8]
100b234b4:     	b	0x100b234c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x8c4>
100b234b8:     	mov	x19, x0
100b234bc:     	ldr	w8, [sp, #0x14]
100b234c0:     	tbnz	w8, #0x0, 0x100b234cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb0_EB8_+0x8cc>
100b234c4:     	mov	x0, x26
100b234c8:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b234cc:     	mov	x0, x19
100b234d0:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
