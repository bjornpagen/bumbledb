
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000101278ca0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>:
101278ca0:     	sub	sp, sp, #0x1e0
101278ca4:     	stp	x28, x27, [sp, #0x180]
101278ca8:     	stp	x26, x25, [sp, #0x190]
101278cac:     	stp	x24, x23, [sp, #0x1a0]
101278cb0:     	stp	x22, x21, [sp, #0x1b0]
101278cb4:     	stp	x20, x19, [sp, #0x1c0]
101278cb8:     	stp	x29, x30, [sp, #0x1d0]
101278cbc:     	add	x29, sp, #0x1d0
101278cc0:     	ldr	w8, [x3, #0x10]
101278cc4:     	cbz	w8, 0x101278df8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
101278cc8:     	mov	x19, x3
101278ccc:     	ldr	w9, [x3, #0x28]
101278cd0:     	cbz	w9, 0x101278df8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
101278cd4:     	mov	x20, x4
101278cd8:     	ldr	x10, [x4, #0x18]
101278cdc:     	cbz	x10, 0x101278e00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
101278ce0:     	mov	x10, #0x0               ; =0
101278ce4:     	mov	x15, #0xa9c5            ; =43461
101278ce8:     	movk	x15, #0x2e62, lsl #16
101278cec:     	movk	x15, #0x7aea, lsl #32
101278cf0:     	movk	x15, #0xf135, lsl #48
101278cf4:     	ldp	x11, x12, [x19]
101278cf8:     	madd	x13, x8, x15, x11
101278cfc:     	mov	x14, #0x6332            ; =25394
101278d00:     	movk	x14, #0x6ed3, lsl #16
101278d04:     	movk	x14, #0x765a, lsl #32
101278d08:     	movk	x14, #0x284f, lsl #48
101278d0c:     	mul	x14, x14, x15
101278d10:     	madd	x13, x13, x15, x14
101278d14:     	add	x13, x13, x12
101278d18:     	madd	x16, x13, x15, x9
101278d1c:     	ldp	x13, x14, [x19, #0x18]
101278d20:     	madd	x16, x16, x15, x13
101278d24:     	madd	x16, x16, x15, x14
101278d28:     	mul	x15, x16, x15
101278d2c:     	ror	x3, x15, #0x2c
101278d30:     	lsr	x17, x3, #57
101278d34:     	ldp	x16, x15, [x20]
101278d38:     	dup.8b	v0, w17
101278d3c:     	movi.2d	v1, #0xffffffffffffffff
101278d40:     	mov	w17, #0x38              ; =56
101278d44:     	and	x3, x3, x15
101278d48:     	ldr	d2, [x16, x3]
101278d4c:     	cmeq.8b	v3, v2, v0
101278d50:     	fmov	x4, d3
101278d54:     	ands	x4, x4, #0x8080808080808080
101278d58:     	b.eq	0x101278dc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
101278d5c:     	rbit	x5, x4
101278d60:     	clz	x5, x5
101278d64:     	add	x5, x3, x5, lsr #3
101278d68:     	and	x5, x5, x15
101278d6c:     	mneg	x5, x5, x17
101278d70:     	add	x5, x16, x5
101278d74:     	ldur	x6, [x5, #-0x38]
101278d78:     	cmp	x11, x6
101278d7c:     	b.ne	0x101278dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
101278d80:     	ldur	x6, [x5, #-0x30]
101278d84:     	cmp	x12, x6
101278d88:     	b.ne	0x101278dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
101278d8c:     	ldur	w6, [x5, #-0x28]
101278d90:     	cmp	w8, w6
101278d94:     	b.ne	0x101278dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
101278d98:     	ldur	x6, [x5, #-0x20]
101278d9c:     	cmp	x13, x6
101278da0:     	b.ne	0x101278dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
101278da4:     	ldur	x6, [x5, #-0x18]
101278da8:     	cmp	x14, x6
101278dac:     	b.ne	0x101278dbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
101278db0:     	ldur	w6, [x5, #-0x10]
101278db4:     	cmp	w9, w6
101278db8:     	b.eq	0x101278f10 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x270>
101278dbc:     	sub	x5, x4, #0x2
101278dc0:     	ands	x4, x5, x4
101278dc4:     	b.ne	0x101278d5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
101278dc8:     	cmeq.8b	v2, v2, v1
101278dcc:     	fmov	x4, d2
101278dd0:     	cbnz	x4, 0x101278e00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
101278dd4:     	add	x10, x10, #0x8
101278dd8:     	add	x3, x3, x10
101278ddc:     	and	x3, x3, x15
101278de0:     	ldr	d2, [x16, x3]
101278de4:     	cmeq.8b	v3, v2, v0
101278de8:     	fmov	x4, d3
101278dec:     	ands	x4, x4, #0x8080808080808080
101278df0:     	b.ne	0x101278d5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
101278df4:     	b	0x101278dc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
101278df8:     	mov	x23, #0x0               ; =0
101278dfc:     	b	0x10127948c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
101278e00:     	mov	x22, x1
101278e04:     	mov	x24, x2
101278e08:     	mov	x23, x0
101278e0c:     	mov	x1, x19
101278e10:     	bl	0x100c5406c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
101278e14:     	mov	x21, x0
101278e18:     	fmov	d0, x21
101278e1c:     	cnt.8b	v0, v0
101278e20:     	addv.8b	b0, v0
101278e24:     	fmov	x26, d0
101278e28:     	cmp	x26, #0xa
101278e2c:     	b.hs	0x101278f18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x278>
101278e30:     	strb	wzr, [sp, #0x120]
101278e34:     	movi.2d	v0, #0000000000000000
101278e38:     	stp	q0, q0, [sp, #0x100]
101278e3c:     	stp	q0, q0, [sp, #0xe0]
101278e40:     	str	q0, [sp, #0xd0]
101278e44:     	sub	x0, x29, #0xa0
101278e48:     	add	x4, sp, #0xd0
101278e4c:     	mov	x1, x23
101278e50:     	mov	x2, x19
101278e54:     	mov	x3, x21
101278e58:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
101278e5c:     	ldp	x25, x22, [x29, #-0xa0]
101278e60:     	ldur	q0, [x29, #-0x90]
101278e64:     	str	q0, [sp, #0xb0]
101278e68:     	str	q0, [sp, #0x90]
101278e6c:     	str	q0, [sp]
101278e70:     	str	q0, [sp, #0x70]
101278e74:     	sub	x0, x29, #0xa0
101278e78:     	add	x2, x19, #0x18
101278e7c:     	add	x4, sp, #0xd0
101278e80:     	mov	x1, x23
101278e84:     	mov	x3, x21
101278e88:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
101278e8c:     	ldp	x24, x21, [x29, #-0xa0]
101278e90:     	ldur	q0, [x29, #-0x90]
101278e94:     	stp	x25, x22, [x29, #-0xa0]
101278e98:     	ldr	q1, [sp, #0x70]
101278e9c:     	stur	q1, [x29, #-0x90]
101278ea0:     	stp	x24, x21, [x29, #-0x80]
101278ea4:     	stur	q0, [x29, #-0x70]
101278ea8:     	mov	w8, #0x1                ; =1
101278eac:     	lsl	x10, x8, x26
101278eb0:     	cmp	x26, #0x6
101278eb4:     	cset	w8, lo
101278eb8:     	mov	x9, #-0x1               ; =-1
101278ebc:     	lsl	x11, x9, x10
101278ec0:     	csinv	x9, x9, x11, hs
101278ec4:     	lsr	x10, x10, #6
101278ec8:     	cinc	x13, x10, lo
101278ecc:     	cbz	x13, 0x1012791d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x538>
101278ed0:     	mov	x12, #0x0               ; =0
101278ed4:     	ldp	x0, x11, [x29, #-0x90]
101278ed8:     	sub	x14, x12, w22, uxtb
101278edc:     	cmn	x24, #0x2
101278ee0:     	b.ne	0x10127914c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4ac>
101278ee4:     	cmn	x25, #0x2
101278ee8:     	b.ne	0x101279180 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4e0>
101278eec:     	tst	w21, #0x1
101278ef0:     	csel	x8, x14, xzr, ne
101278ef4:     	and	x8, x8, x9
101278ef8:     	fmov	d0, x8
101278efc:     	cnt.8b	v0, v0
101278f00:     	addv.8b	b0, v0
101278f04:     	fmov	x8, d0
101278f08:     	mul	x23, x8, x13
101278f0c:     	b	0x10127947c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
101278f10:     	ldur	x23, [x5, #-0x8]
101278f14:     	b	0x10127948c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
101278f18:     	mov	x10, #0x0               ; =0
101278f1c:     	add	x27, sp, #0xd0
101278f20:     	lsl	x11, x24, #2
101278f24:     	mov	x8, x22
101278f28:     	cmp	x11, x10
101278f2c:     	b.eq	0x1012794b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x810>
101278f30:     	ldr	w9, [x8, x10]
101278f34:     	lsr	x12, x21, x9
101278f38:     	add	x10, x10, #0x4
101278f3c:     	tbz	w12, #0x0, 0x101278f28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x288>
101278f40:     	ldr	q0, [x19]
101278f44:     	str	q0, [sp, #0xb0]
101278f48:     	ldr	x10, [x19, #0x10]
101278f4c:     	str	x10, [sp, #0xc0]
101278f50:     	add	x0, sp, #0x70
101278f54:     	mov	x21, x8
101278f58:     	add	x1, sp, #0xb0
101278f5c:     	mov	x22, x23
101278f60:     	mov	x2, x23
101278f64:     	mov	x23, x9
101278f68:     	mov	x3, x23
101278f6c:     	mov	w4, #0x0                ; =0
101278f70:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
101278f74:     	ldr	q0, [sp, #0x70]
101278f78:     	str	q0, [sp, #0x50]
101278f7c:     	ldr	x8, [sp, #0x80]
101278f80:     	str	q0, [sp, #0x30]
101278f84:     	str	q0, [sp, #0x90]
101278f88:     	str	x8, [sp, #0xa0]
101278f8c:     	ldr	q0, [sp, #0x90]
101278f90:     	str	x8, [sp, #0xe0]
101278f94:     	str	q0, [sp, #0xd0]
101278f98:     	ldur	q0, [x19, #0x18]
101278f9c:     	str	q0, [sp, #0xb0]
101278fa0:     	ldur	x8, [x19, #0x28]
101278fa4:     	str	x8, [sp, #0xc0]
101278fa8:     	add	x0, sp, #0x70
101278fac:     	add	x1, sp, #0xb0
101278fb0:     	mov	x2, x22
101278fb4:     	mov	x3, x23
101278fb8:     	mov	w4, #0x0                ; =0
101278fbc:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
101278fc0:     	ldr	q0, [sp, #0x70]
101278fc4:     	str	q0, [sp, #0x50]
101278fc8:     	ldr	x8, [sp, #0x80]
101278fcc:     	str	q0, [sp, #0x30]
101278fd0:     	str	q0, [sp, #0x90]
101278fd4:     	str	x8, [sp, #0xa0]
101278fd8:     	ldr	q0, [sp, #0x90]
101278fdc:     	str	x8, [sp, #0xf8]
101278fe0:     	stur	q0, [x27, #0x18]
101278fe4:     	ldp	q0, q1, [sp, #0xd0]
101278fe8:     	ldr	q2, [sp, #0xf0]
101278fec:     	stp	q1, q2, [x29, #-0x90]
101278ff0:     	stur	q0, [x29, #-0xa0]
101278ff4:     	ldp	q0, q1, [x29, #-0xa0]
101278ff8:     	ldur	q2, [x29, #-0x80]
101278ffc:     	stp	q1, q2, [sp, #0x10]
101279000:     	str	q0, [sp]
101279004:     	mov	x1, sp
101279008:     	mov	x0, x22
10127900c:     	bl	0x100c5406c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
101279010:     	mov	x25, x0
101279014:     	mov	x3, sp
101279018:     	mov	x0, x22
10127901c:     	mov	x1, x21
101279020:     	mov	x2, x24
101279024:     	mov	x4, x20
101279028:     	bl	0x101278ca0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
10127902c:     	fmov	d0, x25
101279030:     	cnt.8b	v0, v0
101279034:     	addv.8b	b0, v0
101279038:     	fmov	w8, s0
10127903c:     	sub	w8, w8, w26
101279040:     	mvn	w8, w8
101279044:     	lsl	x25, x0, x8
101279048:     	ldr	q0, [x19]
10127904c:     	str	q0, [sp, #0xb0]
101279050:     	ldr	x8, [x19, #0x10]
101279054:     	str	x8, [sp, #0xc0]
101279058:     	add	x0, sp, #0x70
10127905c:     	add	x1, sp, #0xb0
101279060:     	mov	x2, x22
101279064:     	mov	x3, x23
101279068:     	mov	w4, #0x1                ; =1
10127906c:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
101279070:     	ldr	q0, [sp, #0x70]
101279074:     	str	q0, [sp, #0x50]
101279078:     	ldr	x8, [sp, #0x80]
10127907c:     	str	q0, [sp, #0x30]
101279080:     	str	q0, [sp, #0x90]
101279084:     	str	x8, [sp, #0xa0]
101279088:     	ldr	q0, [sp, #0x90]
10127908c:     	str	x8, [sp, #0xe0]
101279090:     	str	q0, [sp, #0xd0]
101279094:     	ldur	q0, [x19, #0x18]
101279098:     	str	q0, [sp, #0xb0]
10127909c:     	ldur	x8, [x19, #0x28]
1012790a0:     	str	x8, [sp, #0xc0]
1012790a4:     	add	x0, sp, #0x70
1012790a8:     	add	x1, sp, #0xb0
1012790ac:     	mov	x2, x22
1012790b0:     	mov	x3, x23
1012790b4:     	mov	w4, #0x1                ; =1
1012790b8:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1012790bc:     	ldr	q0, [sp, #0x70]
1012790c0:     	str	q0, [sp, #0x50]
1012790c4:     	ldr	x8, [sp, #0x80]
1012790c8:     	str	q0, [sp, #0x30]
1012790cc:     	str	q0, [sp, #0x90]
1012790d0:     	str	x8, [sp, #0xa0]
1012790d4:     	ldr	q0, [sp, #0x90]
1012790d8:     	str	x8, [sp, #0xf8]
1012790dc:     	stur	q0, [x27, #0x18]
1012790e0:     	ldp	q0, q1, [sp, #0xd0]
1012790e4:     	ldr	q2, [sp, #0xf0]
1012790e8:     	stp	q1, q2, [x29, #-0x90]
1012790ec:     	stur	q0, [x29, #-0xa0]
1012790f0:     	ldp	q0, q1, [x29, #-0xa0]
1012790f4:     	ldur	q2, [x29, #-0x80]
1012790f8:     	stp	q1, q2, [sp, #0x10]
1012790fc:     	str	q0, [sp]
101279100:     	mov	x1, sp
101279104:     	mov	x0, x22
101279108:     	bl	0x100c5406c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
10127910c:     	mov	x23, x0
101279110:     	mov	x3, sp
101279114:     	mov	x0, x22
101279118:     	mov	x1, x21
10127911c:     	mov	x2, x24
101279120:     	mov	x4, x20
101279124:     	bl	0x101278ca0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
101279128:     	fmov	d0, x23
10127912c:     	cnt.8b	v0, v0
101279130:     	addv.8b	b0, v0
101279134:     	fmov	w8, s0
101279138:     	sub	w8, w8, w26
10127913c:     	mvn	w8, w8
101279140:     	lsl	x8, x0, x8
101279144:     	add	x23, x8, x25
101279148:     	b	0x10127947c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
10127914c:     	ldp	x15, x12, [x29, #-0x70]
101279150:     	sub	x16, x13, #0x1
101279154:     	cmn	x25, #0x2
101279158:     	b.ne	0x1012791a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x504>
10127915c:     	mov	x0, x15
101279160:     	cmp	x15, x16
101279164:     	b.ls	0x1012794bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
101279168:     	and	x9, x9, x14
10127916c:     	cmp	x13, #0x8
101279170:     	b.hs	0x1012791e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x540>
101279174:     	mov	x23, #0x0               ; =0
101279178:     	mov	x13, #0x0               ; =0
10127917c:     	b	0x101279268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5c8>
101279180:     	sub	x12, x13, #0x1
101279184:     	cmp	x0, x12
101279188:     	tbz	w21, #0x0, 0x1012791d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x534>
10127918c:     	b.ls	0x1012794bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
101279190:     	cmp	x13, #0x8
101279194:     	b.hs	0x1012793a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x700>
101279198:     	mov	x23, #0x0               ; =0
10127919c:     	mov	x13, #0x0               ; =0
1012791a0:     	b	0x101279428 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x788>
1012791a4:     	cmp	x15, x16
1012791a8:     	csel	x14, x15, x16, lo
1012791ac:     	cmp	x0, x14
1012791b0:     	b.ls	0x1012794bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1012791b4:     	mov	x0, x15
1012791b8:     	cmp	x15, x14
1012791bc:     	b.eq	0x1012794bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1012791c0:     	cmp	x13, #0x8
1012791c4:     	b.hs	0x1012792a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x600>
1012791c8:     	mov	x23, #0x0               ; =0
1012791cc:     	mov	x15, #0x0               ; =0
1012791d0:     	b	0x101279354 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6b4>
1012791d4:     	b.ls	0x1012794bc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
1012791d8:     	mov	x23, #0x0               ; =0
1012791dc:     	b	0x10127945c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1012791e0:     	dup.2d	v0, x12
1012791e4:     	dup.2d	v1, x9
1012791e8:     	ldp	q3, q2, [x21, #0x20]
1012791ec:     	eor.16b	v2, v2, v0
1012791f0:     	and.16b	v2, v2, v1
1012791f4:     	cnt.16b	v2, v2
1012791f8:     	movi.16b	v4, #0x1
1012791fc:     	movi.2d	v5, #0000000000000000
101279200:     	udot.4s	v5, v4, v2
101279204:     	eor.16b	v2, v3, v0
101279208:     	and.16b	v2, v2, v1
10127920c:     	cnt.16b	v2, v2
101279210:     	movi.2d	v3, #0000000000000000
101279214:     	udot.4s	v3, v4, v2
101279218:     	ldp	q6, q2, [x21]
10127921c:     	eor.16b	v2, v2, v0
101279220:     	and.16b	v2, v2, v1
101279224:     	cnt.16b	v2, v2
101279228:     	movi.2d	v7, #0000000000000000
10127922c:     	udot.4s	v7, v4, v2
101279230:     	movi.2d	v2, #0000000000000000
101279234:     	uaddlp.2d	v7, v7
101279238:     	eor.16b	v0, v6, v0
10127923c:     	and.16b	v0, v0, v1
101279240:     	cnt.16b	v0, v0
101279244:     	udot.4s	v2, v4, v0
101279248:     	uadalp.2d	v7, v2
10127924c:     	uadalp.2d	v7, v3
101279250:     	uadalp.2d	v7, v5
101279254:     	addp.2d	d0, v7
101279258:     	fmov	x23, d0
10127925c:     	cmp	x13, #0x8
101279260:     	b.eq	0x10127946c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
101279264:     	mov	w13, #0x8               ; =8
101279268:     	add	x11, x21, x13, lsl #3
10127926c:     	add	x8, x10, x8
101279270:     	sub	x8, x8, x13
101279274:     	ldr	x10, [x11], #0x8
101279278:     	eor	x10, x10, x12
10127927c:     	and	x10, x10, x9
101279280:     	fmov	d0, x10
101279284:     	cnt.8b	v0, v0
101279288:     	addv.8b	b0, v0
10127928c:     	fmov	x10, d0
101279290:     	add	x23, x10, x23
101279294:     	subs	x8, x8, #0x1
101279298:     	b.ne	0x101279274 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5d4>
10127929c:     	b	0x10127946c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
1012792a0:     	dup.2d	v0, x11
1012792a4:     	ldp	q2, q1, [x22, #0x20]
1012792a8:     	eor.16b	v1, v1, v0
1012792ac:     	dup.2d	v3, x12
1012792b0:     	ldp	q5, q4, [x21, #0x20]
1012792b4:     	eor.16b	v4, v4, v3
1012792b8:     	and.16b	v1, v1, v4
1012792bc:     	dup.2d	v4, x9
1012792c0:     	and.16b	v1, v1, v4
1012792c4:     	cnt.16b	v1, v1
1012792c8:     	movi.16b	v6, #0x1
1012792cc:     	movi.2d	v7, #0000000000000000
1012792d0:     	udot.4s	v7, v6, v1
1012792d4:     	eor.16b	v1, v2, v0
1012792d8:     	eor.16b	v2, v5, v3
1012792dc:     	and.16b	v1, v1, v2
1012792e0:     	and.16b	v1, v1, v4
1012792e4:     	cnt.16b	v1, v1
1012792e8:     	movi.2d	v2, #0000000000000000
1012792ec:     	udot.4s	v2, v6, v1
1012792f0:     	ldp	q5, q1, [x22]
1012792f4:     	eor.16b	v1, v1, v0
1012792f8:     	ldp	q17, q16, [x21]
1012792fc:     	eor.16b	v16, v16, v3
101279300:     	and.16b	v1, v1, v16
101279304:     	and.16b	v1, v1, v4
101279308:     	cnt.16b	v1, v1
10127930c:     	movi.2d	v16, #0000000000000000
101279310:     	udot.4s	v16, v6, v1
101279314:     	eor.16b	v0, v5, v0
101279318:     	movi.2d	v1, #0000000000000000
10127931c:     	uaddlp.2d	v5, v16
101279320:     	eor.16b	v3, v17, v3
101279324:     	and.16b	v0, v0, v3
101279328:     	and.16b	v0, v0, v4
10127932c:     	cnt.16b	v0, v0
101279330:     	udot.4s	v1, v6, v0
101279334:     	uadalp.2d	v5, v1
101279338:     	uadalp.2d	v5, v2
10127933c:     	uadalp.2d	v5, v7
101279340:     	addp.2d	d0, v5
101279344:     	fmov	x23, d0
101279348:     	cmp	x13, #0x8
10127934c:     	b.eq	0x10127945c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
101279350:     	mov	w15, #0x8               ; =8
101279354:     	lsl	x14, x15, #3
101279358:     	add	x13, x21, x14
10127935c:     	add	x14, x22, x14
101279360:     	add	x8, x10, x8
101279364:     	sub	x8, x8, x15
101279368:     	ldr	x10, [x14], #0x8
10127936c:     	eor	x10, x10, x11
101279370:     	ldr	x15, [x13], #0x8
101279374:     	eor	x15, x15, x12
101279378:     	and	x10, x10, x15
10127937c:     	and	x10, x10, x9
101279380:     	fmov	d0, x10
101279384:     	cnt.8b	v0, v0
101279388:     	addv.8b	b0, v0
10127938c:     	fmov	x10, d0
101279390:     	add	x23, x10, x23
101279394:     	subs	x8, x8, #0x1
101279398:     	b.ne	0x101279368 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6c8>
10127939c:     	b	0x10127945c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
1012793a0:     	dup.2d	v0, x11
1012793a4:     	dup.2d	v1, x9
1012793a8:     	ldp	q3, q2, [x22, #0x20]
1012793ac:     	eor.16b	v2, v2, v0
1012793b0:     	and.16b	v2, v2, v1
1012793b4:     	cnt.16b	v2, v2
1012793b8:     	movi.16b	v4, #0x1
1012793bc:     	movi.2d	v5, #0000000000000000
1012793c0:     	udot.4s	v5, v4, v2
1012793c4:     	eor.16b	v2, v3, v0
1012793c8:     	and.16b	v2, v2, v1
1012793cc:     	cnt.16b	v2, v2
1012793d0:     	movi.2d	v3, #0000000000000000
1012793d4:     	udot.4s	v3, v4, v2
1012793d8:     	ldp	q6, q2, [x22]
1012793dc:     	eor.16b	v2, v2, v0
1012793e0:     	and.16b	v2, v2, v1
1012793e4:     	cnt.16b	v2, v2
1012793e8:     	movi.2d	v7, #0000000000000000
1012793ec:     	udot.4s	v7, v4, v2
1012793f0:     	movi.2d	v2, #0000000000000000
1012793f4:     	uaddlp.2d	v7, v7
1012793f8:     	eor.16b	v0, v6, v0
1012793fc:     	and.16b	v0, v0, v1
101279400:     	cnt.16b	v0, v0
101279404:     	udot.4s	v2, v4, v0
101279408:     	uadalp.2d	v7, v2
10127940c:     	uadalp.2d	v7, v3
101279410:     	uadalp.2d	v7, v5
101279414:     	addp.2d	d0, v7
101279418:     	fmov	x23, d0
10127941c:     	cmp	x13, #0x8
101279420:     	b.eq	0x10127945c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
101279424:     	mov	w13, #0x8               ; =8
101279428:     	add	x12, x22, x13, lsl #3
10127942c:     	add	x8, x10, x8
101279430:     	sub	x8, x8, x13
101279434:     	ldr	x10, [x12], #0x8
101279438:     	eor	x10, x10, x11
10127943c:     	and	x10, x10, x9
101279440:     	fmov	d0, x10
101279444:     	cnt.8b	v0, v0
101279448:     	addv.8b	b0, v0
10127944c:     	fmov	x10, d0
101279450:     	add	x23, x10, x23
101279454:     	subs	x8, x8, #0x1
101279458:     	b.ne	0x101279434 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x794>
10127945c:     	cmp	x25, #0x1
101279460:     	b.lt	0x10127946c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
101279464:     	mov	x0, x22
101279468:     	bl	0x101a77278 <dyld_stub_binder+0x101a77278>
10127946c:     	cmp	x24, #0x1
101279470:     	b.lt	0x10127947c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
101279474:     	mov	x0, x21
101279478:     	bl	0x101a77278 <dyld_stub_binder+0x101a77278>
10127947c:     	mov	x0, x20
101279480:     	mov	x1, x19
101279484:     	mov	x2, x23
101279488:     	bl	0x1013af77c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
10127948c:     	mov	x0, x23
101279490:     	ldp	x29, x30, [sp, #0x1d0]
101279494:     	ldp	x20, x19, [sp, #0x1c0]
101279498:     	ldp	x22, x21, [sp, #0x1b0]
10127949c:     	ldp	x24, x23, [sp, #0x1a0]
1012794a0:     	ldp	x26, x25, [sp, #0x190]
1012794a4:     	ldp	x28, x27, [sp, #0x180]
1012794a8:     	add	sp, sp, #0x1e0
1012794ac:     	ret
1012794b0:     	adrp	x0, 0x101cf6000 <dyld_stub_binder+0x101cf6000>
1012794b4:     	add	x0, x0, #0x220
1012794b8:     	bl	0x101a6eef4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1012794bc:     	adrp	x2, 0x101cf5000 <dyld_stub_binder+0x101cf5000>
1012794c0:     	add	x2, x2, #0x590
1012794c4:     	mov	x1, x0
1012794c8:     	bl	0x101a6ee5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012794cc:     	brk	#0x1
1012794d0:     	mov	x19, x0
1012794d4:     	sub	x0, x29, #0xa0
1012794d8:     	bl	0x100c04d5c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej2_EBK_>
1012794dc:     	mov	x0, x19
1012794e0:     	bl	0x101a770c8 <dyld_stub_binder+0x101a770c8>
1012794e4:     	mov	x19, x0
1012794e8:     	cmp	x25, #0x1
1012794ec:     	b.lt	0x1012794f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x858>
1012794f0:     	mov	x0, x22
1012794f4:     	bl	0x101a77278 <dyld_stub_binder+0x101a77278>
1012794f8:     	mov	x0, x19
1012794fc:     	bl	0x101a770c8 <dyld_stub_binder+0x101a770c8>
