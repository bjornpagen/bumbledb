
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>:
1006d7c80:     	sub	sp, sp, #0x60
1006d7c84:     	stp	x24, x23, [sp, #0x20]
1006d7c88:     	stp	x22, x21, [sp, #0x30]
1006d7c8c:     	stp	x20, x19, [sp, #0x40]
1006d7c90:     	stp	x29, x30, [sp, #0x50]
1006d7c94:     	add	x29, sp, #0x50
1006d7c98:     	mov	x20, x3
1006d7c9c:     	mov	x23, x2
1006d7ca0:     	mov	x22, x1
1006d7ca4:     	mov	x19, x0
1006d7ca8:     	mov	w1, w2
1006d7cac:     	mov	w2, w3
1006d7cb0:     	mov	x0, x22
1006d7cb4:     	bl	0x1007989d8 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
1006d7cb8:     	cmp	x0, #0x1
1006d7cbc:     	b.ne	0x1006d7cc8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x48>
1006d7cc0:     	mov	x21, x1
1006d7cc4:     	b	0x1006d8d78 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10f8>
1006d7cc8:     	mov	w8, #0x9                ; =9
1006d7ccc:     	and	w8, w22, w8
1006d7cd0:     	lsr	w9, w22, #1
1006d7cd4:     	bfi	w8, w9, #2, #1
1006d7cd8:     	and	w9, w9, #0x2
1006d7cdc:     	orr	w8, w8, w9
1006d7ce0:     	cmp	w23, w20
1006d7ce4:     	csel	w24, w23, w20, hi
1006d7ce8:     	csel	w23, w20, w23, hi
1006d7cec:     	csel	w20, w8, w22, hi
1006d7cf0:     	ldr	x8, [x19, #0xb8]
1006d7cf4:     	cbz	x8, 0x1006d7dc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x144>
1006d7cf8:     	mov	x8, #0x0                ; =0
1006d7cfc:     	and	x9, x20, #0xff
1006d7d00:     	mov	x10, #0xa9c5            ; =43461
1006d7d04:     	movk	x10, #0x2e62, lsl #16
1006d7d08:     	movk	x10, #0x7aea, lsl #32
1006d7d0c:     	movk	x10, #0xf135, lsl #48
1006d7d10:     	mul	x9, x9, x10
1006d7d14:     	add	x9, x9, w23, uxtw
1006d7d18:     	mul	x9, x9, x10
1006d7d1c:     	add	x9, x9, w24, uxtw
1006d7d20:     	mul	x9, x9, x10
1006d7d24:     	ror	x11, x9, #0x2c
1006d7d28:     	lsr	x12, x11, #57
1006d7d2c:     	ldp	x10, x9, [x19, #0xa0]
1006d7d30:     	dup.8b	v0, w12
1006d7d34:     	movi.2d	v1, #0xffffffffffffffff
1006d7d38:     	and	x11, x11, x9
1006d7d3c:     	ldr	d2, [x10, x11]
1006d7d40:     	cmeq.8b	v3, v2, v0
1006d7d44:     	fmov	x12, d3
1006d7d48:     	ands	x12, x12, #0x8080808080808080
1006d7d4c:     	b.eq	0x1006d7d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x114>
1006d7d50:     	rbit	x13, x12
1006d7d54:     	clz	x13, x13
1006d7d58:     	add	x13, x11, x13, lsr #3
1006d7d5c:     	and	x13, x13, x9
1006d7d60:     	sub	x13, x10, x13, lsl #4
1006d7d64:     	ldurb	w14, [x13, #-0xc]
1006d7d68:     	cmp	w14, w20, uxtb
1006d7d6c:     	b.ne	0x1006d7d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x108>
1006d7d70:     	ldur	w14, [x13, #-0x10]
1006d7d74:     	cmp	w23, w14
1006d7d78:     	b.ne	0x1006d7d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x108>
1006d7d7c:     	ldur	w14, [x13, #-0x8]
1006d7d80:     	cmp	w24, w14
1006d7d84:     	b.eq	0x1006d7eb0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x230>
1006d7d88:     	sub	x13, x12, #0x2
1006d7d8c:     	ands	x12, x13, x12
1006d7d90:     	b.ne	0x1006d7d50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd0>
1006d7d94:     	cmeq.8b	v2, v2, v1
1006d7d98:     	fmov	x12, d2
1006d7d9c:     	cbnz	x12, 0x1006d7dc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x144>
1006d7da0:     	add	x8, x8, #0x8
1006d7da4:     	add	x11, x11, x8
1006d7da8:     	and	x11, x11, x9
1006d7dac:     	ldr	d2, [x10, x11]
1006d7db0:     	cmeq.8b	v3, v2, v0
1006d7db4:     	fmov	x12, d3
1006d7db8:     	ands	x12, x12, #0x8080808080808080
1006d7dbc:     	b.ne	0x1006d7d50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd0>
1006d7dc0:     	b	0x1006d7d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x114>
1006d7dc4:     	tbnz	w23, #0x1, 0x1006d7eb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x238>
1006d7dc8:     	ldr	x8, [x19, #0xc8]
1006d7dcc:     	tbnz	w24, #0x1, 0x1006d7ed8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x258>
1006d7dd0:     	ldr	x9, [x19, #0xc8]
1006d7dd4:     	cmp	x9, x8
1006d7dd8:     	csel	x21, x9, x8, lo
1006d7ddc:     	cmp	x21, x9
1006d7de0:     	b.ne	0x1006d7f08 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x288>
1006d7de4:     	lsr	w8, w23, #2
1006d7de8:     	ldr	x1, [x19, #0x58]
1006d7dec:     	cmp	x1, x8
1006d7df0:     	b.ls	0x1006d8da0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1120>
1006d7df4:     	lsr	w0, w24, #2
1006d7df8:     	cmp	x1, x0
1006d7dfc:     	b.ls	0x1006d8db0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1130>
1006d7e00:     	ldr	x9, [x19, #0x50]
1006d7e04:     	add	x8, x9, x8, lsl #5
1006d7e08:     	ldp	q0, q2, [x8]
1006d7e0c:     	and	w8, w23, #0x1
1006d7e10:     	fmov	s1, w8
1006d7e14:     	movi.2d	v3, #0000000000000000
1006d7e18:     	cmeq.4s	v1, v1, v3
1006d7e1c:     	dup.4s	v4, v1[0]
1006d7e20:     	ldur	q1, [x19, #0xd8]
1006d7e24:     	eor.16b	v1, v1, v0
1006d7e28:     	bit.16b	v1, v0, v4
1006d7e2c:     	ldur	q0, [x19, #0xe8]
1006d7e30:     	eor.16b	v0, v0, v2
1006d7e34:     	bit.16b	v0, v2, v4
1006d7e38:     	mov.d	x8, v0[1]
1006d7e3c:     	mov.d	x13, v1[1]
1006d7e40:     	add	x9, x9, x0, lsl #5
1006d7e44:     	ldp	q4, q2, [x9]
1006d7e48:     	and	w9, w24, #0x1
1006d7e4c:     	fmov	s5, w9
1006d7e50:     	cmeq.4s	v3, v5, v3
1006d7e54:     	dup.4s	v3, v3[0]
1006d7e58:     	ldur	q5, [x19, #0xe8]
1006d7e5c:     	eor.16b	v5, v5, v2
1006d7e60:     	bif.16b	v2, v5, v3
1006d7e64:     	ldur	q5, [x19, #0xd8]
1006d7e68:     	eor.16b	v5, v5, v4
1006d7e6c:     	mov.d	x9, v2[1]
1006d7e70:     	bsl.16b	v3, v4, v5
1006d7e74:     	mov.d	x14, v3[1]
1006d7e78:     	and	x16, x20, #0xff
1006d7e7c:     	fmov	x10, d0
1006d7e80:     	fmov	x15, d1
1006d7e84:     	fmov	x11, d2
1006d7e88:     	fmov	x12, d3
1006d7e8c:     	adrp	x17, 0x100d14000 <dyld_stub_binder+0x100d14000>
1006d7e90:     	add	x17, x17, #0x7c6
1006d7e94:     	adr	x0, 0x1006d7ea4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x224>
1006d7e98:     	ldrh	w1, [x17, x16, lsl #1]
1006d7e9c:     	add	x0, x0, x1, lsl #2
1006d7ea0:     	br	x0
1006d7ea4:     	movi.2d	v0, #0000000000000000
1006d7ea8:     	stp	q0, q0, [sp]
1006d7eac:     	b	0x1006d8d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1006d7eb0:     	ldur	w21, [x13, #-0x4]
1006d7eb4:     	b	0x1006d8d78 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10f8>
1006d7eb8:     	lsr	w0, w23, #2
1006d7ebc:     	ldr	x1, [x19, #0x40]
1006d7ec0:     	cmp	x1, x0
1006d7ec4:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d7ec8:     	ldr	x8, [x19, #0x38]
1006d7ecc:     	lsl	x9, x0, #4
1006d7ed0:     	ldr	w8, [x8, x9]
1006d7ed4:     	tbz	w24, #0x1, 0x1006d7dd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x150>
1006d7ed8:     	lsr	w0, w24, #2
1006d7edc:     	ldr	x1, [x19, #0x40]
1006d7ee0:     	cmp	x1, x0
1006d7ee4:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d7ee8:     	ldr	x9, [x19, #0x38]
1006d7eec:     	lsl	x10, x0, #4
1006d7ef0:     	ldr	w10, [x9, x10]
1006d7ef4:     	ldr	x9, [x19, #0xc8]
1006d7ef8:     	cmp	x10, x8
1006d7efc:     	csel	x21, x10, x8, lo
1006d7f00:     	cmp	x21, x9
1006d7f04:     	b.eq	0x1006d7de4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x164>
1006d7f08:     	mov	x2, x23
1006d7f0c:     	tbz	w23, #0x1, 0x1006d7f44 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x2c4>
1006d7f10:     	lsr	w0, w23, #2
1006d7f14:     	ldr	x1, [x19, #0x40]
1006d7f18:     	cmp	x1, x0
1006d7f1c:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d7f20:     	ldr	x8, [x19, #0x38]
1006d7f24:     	add	x8, x8, x0, lsl #4
1006d7f28:     	ldr	w9, [x8]
1006d7f2c:     	mov	x2, x23
1006d7f30:     	cmp	x21, x9
1006d7f34:     	b.ne	0x1006d7f44 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x2c4>
1006d7f38:     	ldr	w8, [x8, #0x4]
1006d7f3c:     	and	w9, w23, #0x1
1006d7f40:     	eor	w2, w8, w9
1006d7f44:     	mov	x3, x24
1006d7f48:     	tbz	w24, #0x1, 0x1006d7f80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x300>
1006d7f4c:     	lsr	w0, w24, #2
1006d7f50:     	ldr	x1, [x19, #0x40]
1006d7f54:     	cmp	x1, x0
1006d7f58:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d7f5c:     	ldr	x8, [x19, #0x38]
1006d7f60:     	add	x8, x8, x0, lsl #4
1006d7f64:     	ldr	w9, [x8]
1006d7f68:     	mov	x3, x24
1006d7f6c:     	cmp	x21, x9
1006d7f70:     	b.ne	0x1006d7f80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x300>
1006d7f74:     	ldr	w8, [x8, #0x4]
1006d7f78:     	and	w9, w24, #0x1
1006d7f7c:     	eor	w3, w8, w9
1006d7f80:     	mov	x0, x19
1006d7f84:     	mov	x1, x20
1006d7f88:     	bl	0x1006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1006d7f8c:     	mov	x22, x0
1006d7f90:     	tbnz	w23, #0x1, 0x1006d7ff8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x378>
1006d7f94:     	ldr	x8, [x19, #0xc8]
1006d7f98:     	mov	x2, x23
1006d7f9c:     	cmp	x8, x21
1006d7fa0:     	b.ne	0x1006d8020 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3a0>
1006d7fa4:     	lsr	w0, w23, #2
1006d7fa8:     	ldr	x1, [x19, #0x40]
1006d7fac:     	cmp	x1, x0
1006d7fb0:     	b.ls	0x1006d8dbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x113c>
1006d7fb4:     	ldr	x8, [x19, #0x38]
1006d7fb8:     	add	x8, x8, x0, lsl #4
1006d7fbc:     	ldr	w8, [x8, #0x8]
1006d7fc0:     	and	w9, w23, #0x1
1006d7fc4:     	eor	w2, w8, w9
1006d7fc8:     	tbz	w24, #0x1, 0x1006d8024 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3a4>
1006d7fcc:     	lsr	w0, w24, #2
1006d7fd0:     	ldr	x1, [x19, #0x40]
1006d7fd4:     	cmp	x1, x0
1006d7fd8:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d7fdc:     	ldr	x8, [x19, #0x38]
1006d7fe0:     	lsl	x9, x0, #4
1006d7fe4:     	ldr	w8, [x8, x9]
1006d7fe8:     	mov	x3, x24
1006d7fec:     	cmp	x8, x21
1006d7ff0:     	b.ne	0x1006d8058 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3d8>
1006d7ff4:     	b	0x1006d8034 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3b4>
1006d7ff8:     	lsr	w0, w23, #2
1006d7ffc:     	ldr	x1, [x19, #0x40]
1006d8000:     	cmp	x1, x0
1006d8004:     	b.ls	0x1006d8d94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1006d8008:     	ldr	x8, [x19, #0x38]
1006d800c:     	lsl	x9, x0, #4
1006d8010:     	ldr	w8, [x8, x9]
1006d8014:     	mov	x2, x23
1006d8018:     	cmp	x8, x21
1006d801c:     	b.eq	0x1006d7fa4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x324>
1006d8020:     	tbnz	w24, #0x1, 0x1006d7fcc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x34c>
1006d8024:     	ldr	x8, [x19, #0xc8]
1006d8028:     	mov	x3, x24
1006d802c:     	cmp	x8, x21
1006d8030:     	b.ne	0x1006d8058 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3d8>
1006d8034:     	lsr	w0, w24, #2
1006d8038:     	ldr	x1, [x19, #0x40]
1006d803c:     	cmp	x1, x0
1006d8040:     	b.ls	0x1006d8dbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x113c>
1006d8044:     	ldr	x8, [x19, #0x38]
1006d8048:     	add	x8, x8, x0, lsl #4
1006d804c:     	ldr	w8, [x8, #0x8]
1006d8050:     	and	w9, w24, #0x1
1006d8054:     	eor	w3, w8, w9
1006d8058:     	mov	x0, x19
1006d805c:     	mov	x1, x20
1006d8060:     	bl	0x1006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1006d8064:     	mov	x3, x0
1006d8068:     	mov	x0, x19
1006d806c:     	mov	x1, x21
1006d8070:     	mov	x2, x22
1006d8074:     	bl	0x1006d6edc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E2mkB6_>
1006d8078:     	b	0x1006d8d58 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10d8>
1006d807c:     	orr.16b	v1, v3, v1
1006d8080:     	orr.16b	v0, v2, v0
1006d8084:     	b	0x1006d8358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1006d8088:     	stp	q3, q2, [sp]
1006d808c:     	b	0x1006d8d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1006d8090:     	mov	x16, #0x0               ; =0
1006d8094:     	mov	x17, x20
1006d8098:     	b	0x1006d80a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x424>
1006d809c:     	eor	w17, w17, #0xf
1006d80a0:     	mvn	x16, x16
1006d80a4:     	and	w0, w17, #0xff
1006d80a8:     	cmp	w0, #0x7
1006d80ac:     	b.le	0x1006d80cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x44c>
1006d80b0:     	cmp	w0, #0x8
1006d80b4:     	b.eq	0x1006d864c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9cc>
1006d80b8:     	cmp	w0, #0xa
1006d80bc:     	b.eq	0x1006d8658 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1006d80c0:     	cmp	w0, #0xc
1006d80c4:     	b.ne	0x1006d809c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x41c>
1006d80c8:     	b	0x1006d8644 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9c4>
1006d80cc:     	cmp	w0, #0x4
1006d80d0:     	b.eq	0x1006d8654 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d4>
1006d80d4:     	cmp	w0, #0x6
1006d80d8:     	b.ne	0x1006d809c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x41c>
1006d80dc:     	eor	x12, x12, x15
1006d80e0:     	b	0x1006d8658 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1006d80e4:     	mov	x16, #0x0               ; =0
1006d80e8:     	mov	x17, x20
1006d80ec:     	b	0x1006d80f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x478>
1006d80f0:     	eor	w17, w17, #0xf
1006d80f4:     	mvn	x16, x16
1006d80f8:     	and	w0, w17, #0xff
1006d80fc:     	cmp	w0, #0x7
1006d8100:     	b.gt	0x1006d8118 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x498>
1006d8104:     	cmp	w0, #0x4
1006d8108:     	b.eq	0x1006d8540 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c0>
1006d810c:     	cmp	w0, #0x6
1006d8110:     	b.ne	0x1006d80f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x470>
1006d8114:     	b	0x1006d8530 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8b0>
1006d8118:     	cmp	w0, #0x8
1006d811c:     	b.eq	0x1006d8538 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8b8>
1006d8120:     	cmp	w0, #0xa
1006d8124:     	b.ne	0x1006d80f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x470>
1006d8128:     	b	0x1006d8544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1006d812c:     	and.16b	v1, v3, v1
1006d8130:     	and.16b	v0, v2, v0
1006d8134:     	b	0x1006d8358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1006d8138:     	mov	x16, #0x0               ; =0
1006d813c:     	mov	x17, x20
1006d8140:     	b	0x1006d814c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4cc>
1006d8144:     	eor	w17, w17, #0xf
1006d8148:     	mvn	x16, x16
1006d814c:     	and	w0, w17, #0xff
1006d8150:     	cmp	w0, #0x7
1006d8154:     	b.gt	0x1006d8170 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4f0>
1006d8158:     	cmp	w0, #0x3
1006d815c:     	b.gt	0x1006d818c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x50c>
1006d8160:     	cbz	w0, 0x1006d8b24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xea4>
1006d8164:     	cmp	w0, #0x2
1006d8168:     	b.ne	0x1006d8144 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1006d816c:     	b	0x1006d8b0c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe8c>
1006d8170:     	cmp	w0, #0xb
1006d8174:     	b.gt	0x1006d81a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x520>
1006d8178:     	cmp	w0, #0x8
1006d817c:     	b.eq	0x1006d8b1c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe9c>
1006d8180:     	cmp	w0, #0xa
1006d8184:     	b.ne	0x1006d8144 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1006d8188:     	b	0x1006d8b14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe94>
1006d818c:     	cmp	w0, #0x4
1006d8190:     	b.eq	0x1006d8b2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeac>
1006d8194:     	cmp	w0, #0x6
1006d8198:     	b.ne	0x1006d8144 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1006d819c:     	b	0x1006d8b04 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe84>
1006d81a0:     	cmp	w0, #0xc
1006d81a4:     	b.eq	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d81a8:     	cmp	w0, #0xe
1006d81ac:     	b.ne	0x1006d8144 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1006d81b0:     	orr	x15, x12, x15
1006d81b4:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d81b8:     	bic.16b	v1, v3, v1
1006d81bc:     	bic.16b	v0, v2, v0
1006d81c0:     	b	0x1006d8358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1006d81c4:     	eor.16b	v1, v3, v1
1006d81c8:     	eor.16b	v0, v2, v0
1006d81cc:     	b	0x1006d8358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1006d81d0:     	mov	x16, #0x0               ; =0
1006d81d4:     	mov	x17, x20
1006d81d8:     	b	0x1006d81e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x564>
1006d81dc:     	eor	w17, w17, #0xf
1006d81e0:     	mvn	x16, x16
1006d81e4:     	and	w0, w17, #0xff
1006d81e8:     	cmp	w0, #0x7
1006d81ec:     	b.le	0x1006d820c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x58c>
1006d81f0:     	cmp	w0, #0xb
1006d81f4:     	b.gt	0x1006d8228 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5a8>
1006d81f8:     	cmp	w0, #0x8
1006d81fc:     	b.eq	0x1006d8918 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc98>
1006d8200:     	cmp	w0, #0xa
1006d8204:     	b.ne	0x1006d81dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1006d8208:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d820c:     	cmp	w0, #0x2
1006d8210:     	b.eq	0x1006d8920 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xca0>
1006d8214:     	cmp	w0, #0x4
1006d8218:     	b.eq	0x1006d8928 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xca8>
1006d821c:     	cmp	w0, #0x6
1006d8220:     	b.ne	0x1006d81dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1006d8224:     	b	0x1006d8910 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc90>
1006d8228:     	cmp	w0, #0xc
1006d822c:     	b.eq	0x1006d8908 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc88>
1006d8230:     	cmp	w0, #0xe
1006d8234:     	b.ne	0x1006d81dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1006d8238:     	orr	x12, x12, x15
1006d823c:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d8240:     	bic.16b	v1, v1, v3
1006d8244:     	bic.16b	v0, v0, v2
1006d8248:     	b	0x1006d8358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1006d824c:     	mov	x16, #0x0               ; =0
1006d8250:     	mov	x17, x20
1006d8254:     	b	0x1006d8260 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5e0>
1006d8258:     	eor	w17, w17, #0xf
1006d825c:     	mvn	x16, x16
1006d8260:     	and	w0, w17, #0xff
1006d8264:     	cmp	w0, #0x7
1006d8268:     	b.gt	0x1006d8288 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x608>
1006d826c:     	cmp	w0, #0x2
1006d8270:     	b.eq	0x1006d8794 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb14>
1006d8274:     	cmp	w0, #0x4
1006d8278:     	b.eq	0x1006d8784 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb04>
1006d827c:     	cmp	w0, #0x6
1006d8280:     	b.ne	0x1006d8258 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5d8>
1006d8284:     	b	0x1006d878c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb0c>
1006d8288:     	cmp	w0, #0x8
1006d828c:     	b.eq	0x1006d877c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xafc>
1006d8290:     	cmp	w0, #0xa
1006d8294:     	b.eq	0x1006d8798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1006d8298:     	cmp	w0, #0xc
1006d829c:     	b.ne	0x1006d8258 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5d8>
1006d82a0:     	mov	x12, x15
1006d82a4:     	b	0x1006d8798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1006d82a8:     	mov	x16, #0x0               ; =0
1006d82ac:     	mov	x17, x20
1006d82b0:     	and	w0, w17, #0xff
1006d82b4:     	cmp	w0, #0x6
1006d82b8:     	b.eq	0x1006d82e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x660>
1006d82bc:     	cmp	w0, #0x8
1006d82c0:     	b.eq	0x1006d8438 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7b8>
1006d82c4:     	cmp	w0, #0xa
1006d82c8:     	b.eq	0x1006d843c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7bc>
1006d82cc:     	eor	w17, w17, #0xf
1006d82d0:     	mvn	x16, x16
1006d82d4:     	and	w0, w17, #0xff
1006d82d8:     	cmp	w0, #0x6
1006d82dc:     	b.ne	0x1006d82bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x63c>
1006d82e0:     	eor	x12, x12, x15
1006d82e4:     	mov	x15, #0x0               ; =0
1006d82e8:     	mov	w17, #0x5               ; =5
1006d82ec:     	and	w0, w17, #0xff
1006d82f0:     	cmp	w0, #0xa
1006d82f4:     	b.ne	0x1006d8450 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7d0>
1006d82f8:     	b	0x1006d8498 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1006d82fc:     	mov	x16, #0x0               ; =0
1006d8300:     	mov	x17, x20
1006d8304:     	and	w0, w17, #0xff
1006d8308:     	cmp	w0, #0x6
1006d830c:     	b.eq	0x1006d832c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6ac>
1006d8310:     	cmp	w0, #0x8
1006d8314:     	b.eq	0x1006d8360 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6e0>
1006d8318:     	eor	w17, w17, #0xf
1006d831c:     	mvn	x16, x16
1006d8320:     	and	w0, w17, #0xff
1006d8324:     	cmp	w0, #0x6
1006d8328:     	b.ne	0x1006d8310 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x690>
1006d832c:     	eor	x12, x12, x15
1006d8330:     	mov	x15, #0x0               ; =0
1006d8334:     	mov	w17, #0x9               ; =9
1006d8338:     	and	w0, w17, #0xff
1006d833c:     	cmp	w0, #0x8
1006d8340:     	b.ne	0x1006d8378 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6f8>
1006d8344:     	b	0x1006d8394 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x714>
1006d8348:     	and.16b	v1, v3, v1
1006d834c:     	and.16b	v0, v2, v0
1006d8350:     	mvn.16b	v1, v1
1006d8354:     	mvn.16b	v0, v0
1006d8358:     	stp	q1, q0, [sp]
1006d835c:     	b	0x1006d8d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1006d8360:     	and	x12, x12, x15
1006d8364:     	mov	x15, #0x0               ; =0
1006d8368:     	mov	w17, #0x9               ; =9
1006d836c:     	and	w0, w17, #0xff
1006d8370:     	cmp	w0, #0x8
1006d8374:     	b.eq	0x1006d8394 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x714>
1006d8378:     	cmp	w0, #0x6
1006d837c:     	b.eq	0x1006d83b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x730>
1006d8380:     	eor	w17, w17, #0xf
1006d8384:     	mvn	x15, x15
1006d8388:     	and	w0, w17, #0xff
1006d838c:     	cmp	w0, #0x8
1006d8390:     	b.ne	0x1006d8378 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6f8>
1006d8394:     	and	x13, x14, x13
1006d8398:     	mov	x14, #0x0               ; =0
1006d839c:     	mov	w17, #0x9               ; =9
1006d83a0:     	and	w0, w17, #0xff
1006d83a4:     	cmp	w0, #0x8
1006d83a8:     	b.ne	0x1006d83c8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x748>
1006d83ac:     	b	0x1006d83e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x764>
1006d83b0:     	eor	x13, x14, x13
1006d83b4:     	mov	x14, #0x0               ; =0
1006d83b8:     	mov	w17, #0x9               ; =9
1006d83bc:     	and	w0, w17, #0xff
1006d83c0:     	cmp	w0, #0x8
1006d83c4:     	b.eq	0x1006d83e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x764>
1006d83c8:     	cmp	w0, #0x6
1006d83cc:     	b.eq	0x1006d8400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x780>
1006d83d0:     	eor	w17, w17, #0xf
1006d83d4:     	mvn	x14, x14
1006d83d8:     	and	w0, w17, #0xff
1006d83dc:     	cmp	w0, #0x8
1006d83e0:     	b.ne	0x1006d83c8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x748>
1006d83e4:     	and	x10, x11, x10
1006d83e8:     	mov	x11, #0x0               ; =0
1006d83ec:     	mov	w17, #0x9               ; =9
1006d83f0:     	and	w0, w17, #0xff
1006d83f4:     	cmp	w0, #0x8
1006d83f8:     	b.ne	0x1006d8418 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x798>
1006d83fc:     	b	0x1006d88dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1006d8400:     	eor	x10, x11, x10
1006d8404:     	mov	x11, #0x0               ; =0
1006d8408:     	mov	w17, #0x9               ; =9
1006d840c:     	and	w0, w17, #0xff
1006d8410:     	cmp	w0, #0x8
1006d8414:     	b.eq	0x1006d88dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1006d8418:     	cmp	w0, #0x6
1006d841c:     	b.eq	0x1006d8774 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaf4>
1006d8420:     	eor	w17, w17, #0xf
1006d8424:     	mvn	x11, x11
1006d8428:     	and	w0, w17, #0xff
1006d842c:     	cmp	w0, #0x8
1006d8430:     	b.ne	0x1006d8418 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x798>
1006d8434:     	b	0x1006d88dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1006d8438:     	and	x12, x12, x15
1006d843c:     	mov	x15, #0x0               ; =0
1006d8440:     	mov	w17, #0x5               ; =5
1006d8444:     	and	w0, w17, #0xff
1006d8448:     	cmp	w0, #0xa
1006d844c:     	b.eq	0x1006d8498 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1006d8450:     	cmp	w0, #0x8
1006d8454:     	b.eq	0x1006d8494 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x814>
1006d8458:     	cmp	w0, #0x6
1006d845c:     	b.eq	0x1006d8478 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7f8>
1006d8460:     	eor	w17, w17, #0xf
1006d8464:     	mvn	x15, x15
1006d8468:     	and	w0, w17, #0xff
1006d846c:     	cmp	w0, #0xa
1006d8470:     	b.ne	0x1006d8450 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7d0>
1006d8474:     	b	0x1006d8498 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1006d8478:     	eor	x14, x14, x13
1006d847c:     	mov	x13, #0x0               ; =0
1006d8480:     	mov	w17, #0x5               ; =5
1006d8484:     	and	w0, w17, #0xff
1006d8488:     	cmp	w0, #0xa
1006d848c:     	b.ne	0x1006d84ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x82c>
1006d8490:     	b	0x1006d84f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1006d8494:     	and	x14, x14, x13
1006d8498:     	mov	x13, #0x0               ; =0
1006d849c:     	mov	w17, #0x5               ; =5
1006d84a0:     	and	w0, w17, #0xff
1006d84a4:     	cmp	w0, #0xa
1006d84a8:     	b.eq	0x1006d84f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1006d84ac:     	cmp	w0, #0x8
1006d84b0:     	b.eq	0x1006d84f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x870>
1006d84b4:     	cmp	w0, #0x6
1006d84b8:     	b.eq	0x1006d84d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x854>
1006d84bc:     	eor	w17, w17, #0xf
1006d84c0:     	mvn	x13, x13
1006d84c4:     	and	w0, w17, #0xff
1006d84c8:     	cmp	w0, #0xa
1006d84cc:     	b.ne	0x1006d84ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x82c>
1006d84d0:     	b	0x1006d84f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1006d84d4:     	eor	x11, x11, x10
1006d84d8:     	mov	x10, #0x0               ; =0
1006d84dc:     	mov	w17, #0x5               ; =5
1006d84e0:     	and	w0, w17, #0xff
1006d84e4:     	cmp	w0, #0xa
1006d84e8:     	b.ne	0x1006d8508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x888>
1006d84ec:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d84f0:     	and	x11, x11, x10
1006d84f4:     	mov	x10, #0x0               ; =0
1006d84f8:     	mov	w17, #0x5               ; =5
1006d84fc:     	and	w0, w17, #0xff
1006d8500:     	cmp	w0, #0xa
1006d8504:     	b.eq	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8508:     	cmp	w0, #0x8
1006d850c:     	b.eq	0x1006d8acc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1006d8510:     	cmp	w0, #0x6
1006d8514:     	b.eq	0x1006d8ac4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1006d8518:     	eor	w17, w17, #0xf
1006d851c:     	mvn	x10, x10
1006d8520:     	and	w0, w17, #0xff
1006d8524:     	cmp	w0, #0xa
1006d8528:     	b.ne	0x1006d8508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x888>
1006d852c:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8530:     	eor	x12, x12, x15
1006d8534:     	b	0x1006d8544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1006d8538:     	and	x12, x12, x15
1006d853c:     	b	0x1006d8544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1006d8540:     	bic	x12, x15, x12
1006d8544:     	mov	x15, #0x0               ; =0
1006d8548:     	mov	w17, #0xb               ; =11
1006d854c:     	b	0x1006d8558 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d8>
1006d8550:     	eor	w17, w17, #0xf
1006d8554:     	mvn	x15, x15
1006d8558:     	and	w0, w17, #0xff
1006d855c:     	cmp	w0, #0x7
1006d8560:     	b.gt	0x1006d8578 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8f8>
1006d8564:     	cmp	w0, #0x4
1006d8568:     	b.eq	0x1006d859c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x91c>
1006d856c:     	cmp	w0, #0x6
1006d8570:     	b.ne	0x1006d8550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d0>
1006d8574:     	b	0x1006d858c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x90c>
1006d8578:     	cmp	w0, #0x8
1006d857c:     	b.eq	0x1006d8594 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x914>
1006d8580:     	cmp	w0, #0xa
1006d8584:     	b.ne	0x1006d8550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d0>
1006d8588:     	b	0x1006d85a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1006d858c:     	eor	x14, x14, x13
1006d8590:     	b	0x1006d85a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1006d8594:     	and	x14, x14, x13
1006d8598:     	b	0x1006d85a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1006d859c:     	bic	x14, x13, x14
1006d85a0:     	mov	x13, #0x0               ; =0
1006d85a4:     	mov	w17, #0xb               ; =11
1006d85a8:     	b	0x1006d85b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x934>
1006d85ac:     	eor	w17, w17, #0xf
1006d85b0:     	mvn	x13, x13
1006d85b4:     	and	w0, w17, #0xff
1006d85b8:     	cmp	w0, #0x7
1006d85bc:     	b.gt	0x1006d85d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x954>
1006d85c0:     	cmp	w0, #0x4
1006d85c4:     	b.eq	0x1006d85f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x978>
1006d85c8:     	cmp	w0, #0x6
1006d85cc:     	b.ne	0x1006d85ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x92c>
1006d85d0:     	b	0x1006d85e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x968>
1006d85d4:     	cmp	w0, #0x8
1006d85d8:     	b.eq	0x1006d85f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x970>
1006d85dc:     	cmp	w0, #0xa
1006d85e0:     	b.ne	0x1006d85ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x92c>
1006d85e4:     	b	0x1006d85fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1006d85e8:     	eor	x11, x11, x10
1006d85ec:     	b	0x1006d85fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1006d85f0:     	and	x11, x11, x10
1006d85f4:     	b	0x1006d85fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1006d85f8:     	bic	x11, x10, x11
1006d85fc:     	mov	x10, #0x0               ; =0
1006d8600:     	mov	w17, #0xb               ; =11
1006d8604:     	b	0x1006d8610 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x990>
1006d8608:     	eor	w17, w17, #0xf
1006d860c:     	mvn	x10, x10
1006d8610:     	and	w0, w17, #0xff
1006d8614:     	cmp	w0, #0x7
1006d8618:     	b.gt	0x1006d8630 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9b0>
1006d861c:     	cmp	w0, #0x4
1006d8620:     	b.eq	0x1006d8ad4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe54>
1006d8624:     	cmp	w0, #0x6
1006d8628:     	b.ne	0x1006d8608 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x988>
1006d862c:     	b	0x1006d8ac4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1006d8630:     	cmp	w0, #0x8
1006d8634:     	b.eq	0x1006d8acc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1006d8638:     	cmp	w0, #0xa
1006d863c:     	b.ne	0x1006d8608 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x988>
1006d8640:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8644:     	mov	x12, x15
1006d8648:     	b	0x1006d8658 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1006d864c:     	and	x12, x12, x15
1006d8650:     	b	0x1006d8658 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1006d8654:     	bic	x12, x15, x12
1006d8658:     	mov	x15, #0x0               ; =0
1006d865c:     	mov	w17, #0x3               ; =3
1006d8660:     	b	0x1006d866c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9ec>
1006d8664:     	eor	w17, w17, #0xf
1006d8668:     	mvn	x15, x15
1006d866c:     	and	w0, w17, #0xff
1006d8670:     	cmp	w0, #0x7
1006d8674:     	b.le	0x1006d8694 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa14>
1006d8678:     	cmp	w0, #0xc
1006d867c:     	b.eq	0x1006d86c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1006d8680:     	cmp	w0, #0xa
1006d8684:     	b.eq	0x1006d86b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa34>
1006d8688:     	cmp	w0, #0x8
1006d868c:     	b.ne	0x1006d8664 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9e4>
1006d8690:     	b	0x1006d86ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa2c>
1006d8694:     	cmp	w0, #0x4
1006d8698:     	b.eq	0x1006d86bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa3c>
1006d869c:     	cmp	w0, #0x6
1006d86a0:     	b.ne	0x1006d8664 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9e4>
1006d86a4:     	eor	x13, x14, x13
1006d86a8:     	b	0x1006d86c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1006d86ac:     	and	x13, x14, x13
1006d86b0:     	b	0x1006d86c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1006d86b4:     	mov	x13, x14
1006d86b8:     	b	0x1006d86c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1006d86bc:     	bic	x13, x13, x14
1006d86c0:     	mov	x14, #0x0               ; =0
1006d86c4:     	mov	w17, #0x3               ; =3
1006d86c8:     	b	0x1006d86d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa54>
1006d86cc:     	eor	w17, w17, #0xf
1006d86d0:     	mvn	x14, x14
1006d86d4:     	and	w0, w17, #0xff
1006d86d8:     	cmp	w0, #0x7
1006d86dc:     	b.le	0x1006d86fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa7c>
1006d86e0:     	cmp	w0, #0xc
1006d86e4:     	b.eq	0x1006d8728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1006d86e8:     	cmp	w0, #0xa
1006d86ec:     	b.eq	0x1006d871c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa9c>
1006d86f0:     	cmp	w0, #0x8
1006d86f4:     	b.ne	0x1006d86cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa4c>
1006d86f8:     	b	0x1006d8714 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa94>
1006d86fc:     	cmp	w0, #0x4
1006d8700:     	b.eq	0x1006d8724 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa4>
1006d8704:     	cmp	w0, #0x6
1006d8708:     	b.ne	0x1006d86cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa4c>
1006d870c:     	eor	x10, x11, x10
1006d8710:     	b	0x1006d8728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1006d8714:     	and	x10, x11, x10
1006d8718:     	b	0x1006d8728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1006d871c:     	mov	x10, x11
1006d8720:     	b	0x1006d8728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1006d8724:     	bic	x10, x10, x11
1006d8728:     	mov	x11, #0x0               ; =0
1006d872c:     	mov	w17, #0x3               ; =3
1006d8730:     	b	0x1006d873c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xabc>
1006d8734:     	eor	w17, w17, #0xf
1006d8738:     	mvn	x11, x11
1006d873c:     	and	w0, w17, #0xff
1006d8740:     	cmp	w0, #0x7
1006d8744:     	b.le	0x1006d8764 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xae4>
1006d8748:     	cmp	w0, #0xc
1006d874c:     	b.eq	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d8750:     	cmp	w0, #0xa
1006d8754:     	b.eq	0x1006d88ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc6c>
1006d8758:     	cmp	w0, #0x8
1006d875c:     	b.ne	0x1006d8734 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xab4>
1006d8760:     	b	0x1006d88dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1006d8764:     	cmp	w0, #0x4
1006d8768:     	b.eq	0x1006d88e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc64>
1006d876c:     	cmp	w0, #0x6
1006d8770:     	b.ne	0x1006d8734 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xab4>
1006d8774:     	eor	x8, x9, x8
1006d8778:     	b	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d877c:     	and	x12, x12, x15
1006d8780:     	b	0x1006d8798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1006d8784:     	bic	x12, x15, x12
1006d8788:     	b	0x1006d8798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1006d878c:     	eor	x12, x12, x15
1006d8790:     	b	0x1006d8798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1006d8794:     	bic	x12, x12, x15
1006d8798:     	mov	x15, #0x0               ; =0
1006d879c:     	mov	w17, #0xd               ; =13
1006d87a0:     	b	0x1006d87ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb2c>
1006d87a4:     	eor	w17, w17, #0xf
1006d87a8:     	mvn	x15, x15
1006d87ac:     	and	w0, w17, #0xff
1006d87b0:     	cmp	w0, #0x7
1006d87b4:     	b.gt	0x1006d87d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb54>
1006d87b8:     	cmp	w0, #0x2
1006d87bc:     	b.eq	0x1006d880c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb8c>
1006d87c0:     	cmp	w0, #0x4
1006d87c4:     	b.eq	0x1006d87fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb7c>
1006d87c8:     	cmp	w0, #0x6
1006d87cc:     	b.ne	0x1006d87a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb24>
1006d87d0:     	b	0x1006d8804 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb84>
1006d87d4:     	cmp	w0, #0xc
1006d87d8:     	b.eq	0x1006d8810 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1006d87dc:     	cmp	w0, #0xa
1006d87e0:     	b.eq	0x1006d87f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb74>
1006d87e4:     	cmp	w0, #0x8
1006d87e8:     	b.ne	0x1006d87a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb24>
1006d87ec:     	and	x13, x14, x13
1006d87f0:     	b	0x1006d8810 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1006d87f4:     	mov	x13, x14
1006d87f8:     	b	0x1006d8810 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1006d87fc:     	bic	x13, x13, x14
1006d8800:     	b	0x1006d8810 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1006d8804:     	eor	x13, x14, x13
1006d8808:     	b	0x1006d8810 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1006d880c:     	bic	x13, x14, x13
1006d8810:     	mov	x14, #0x0               ; =0
1006d8814:     	mov	w17, #0xd               ; =13
1006d8818:     	b	0x1006d8824 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xba4>
1006d881c:     	eor	w17, w17, #0xf
1006d8820:     	mvn	x14, x14
1006d8824:     	and	w0, w17, #0xff
1006d8828:     	cmp	w0, #0x7
1006d882c:     	b.gt	0x1006d884c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbcc>
1006d8830:     	cmp	w0, #0x2
1006d8834:     	b.eq	0x1006d8884 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc04>
1006d8838:     	cmp	w0, #0x4
1006d883c:     	b.eq	0x1006d8874 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbf4>
1006d8840:     	cmp	w0, #0x6
1006d8844:     	b.ne	0x1006d881c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb9c>
1006d8848:     	b	0x1006d887c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbfc>
1006d884c:     	cmp	w0, #0xc
1006d8850:     	b.eq	0x1006d8888 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1006d8854:     	cmp	w0, #0xa
1006d8858:     	b.eq	0x1006d886c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbec>
1006d885c:     	cmp	w0, #0x8
1006d8860:     	b.ne	0x1006d881c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb9c>
1006d8864:     	and	x10, x11, x10
1006d8868:     	b	0x1006d8888 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1006d886c:     	mov	x10, x11
1006d8870:     	b	0x1006d8888 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1006d8874:     	bic	x10, x10, x11
1006d8878:     	b	0x1006d8888 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1006d887c:     	eor	x10, x11, x10
1006d8880:     	b	0x1006d8888 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1006d8884:     	bic	x10, x11, x10
1006d8888:     	mov	x11, #0x0               ; =0
1006d888c:     	mov	w17, #0xd               ; =13
1006d8890:     	b	0x1006d889c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc1c>
1006d8894:     	eor	w17, w17, #0xf
1006d8898:     	mvn	x11, x11
1006d889c:     	and	w0, w17, #0xff
1006d88a0:     	cmp	w0, #0x7
1006d88a4:     	b.gt	0x1006d88c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc44>
1006d88a8:     	cmp	w0, #0x2
1006d88ac:     	b.eq	0x1006d88f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc74>
1006d88b0:     	cmp	w0, #0x4
1006d88b4:     	b.eq	0x1006d88e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc64>
1006d88b8:     	cmp	w0, #0x6
1006d88bc:     	b.ne	0x1006d8894 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc14>
1006d88c0:     	b	0x1006d8774 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaf4>
1006d88c4:     	cmp	w0, #0xc
1006d88c8:     	b.eq	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d88cc:     	cmp	w0, #0xa
1006d88d0:     	b.eq	0x1006d88ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc6c>
1006d88d4:     	cmp	w0, #0x8
1006d88d8:     	b.ne	0x1006d8894 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc14>
1006d88dc:     	and	x8, x9, x8
1006d88e0:     	b	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d88e4:     	bic	x8, x8, x9
1006d88e8:     	b	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d88ec:     	mov	x8, x9
1006d88f0:     	b	0x1006d88f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1006d88f4:     	bic	x8, x9, x8
1006d88f8:     	eor	x9, x10, x14
1006d88fc:     	eor	x10, x13, x15
1006d8900:     	eor	x12, x12, x16
1006d8904:     	b	0x1006d8d40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10c0>
1006d8908:     	mov	x12, x15
1006d890c:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d8910:     	eor	x12, x12, x15
1006d8914:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d8918:     	and	x12, x12, x15
1006d891c:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d8920:     	bic	x12, x12, x15
1006d8924:     	b	0x1006d892c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1006d8928:     	bic	x12, x15, x12
1006d892c:     	mov	x15, #0x0               ; =0
1006d8930:     	mov	w17, #0x1               ; =1
1006d8934:     	b	0x1006d8940 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcc0>
1006d8938:     	eor	w17, w17, #0xf
1006d893c:     	mvn	x15, x15
1006d8940:     	and	w0, w17, #0xff
1006d8944:     	cmp	w0, #0x7
1006d8948:     	b.le	0x1006d8968 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xce8>
1006d894c:     	cmp	w0, #0xb
1006d8950:     	b.gt	0x1006d8984 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd04>
1006d8954:     	cmp	w0, #0x8
1006d8958:     	b.eq	0x1006d89ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd2c>
1006d895c:     	cmp	w0, #0xa
1006d8960:     	b.ne	0x1006d8938 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1006d8964:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d8968:     	cmp	w0, #0x2
1006d896c:     	b.eq	0x1006d89b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd34>
1006d8970:     	cmp	w0, #0x4
1006d8974:     	b.eq	0x1006d89bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd3c>
1006d8978:     	cmp	w0, #0x6
1006d897c:     	b.ne	0x1006d8938 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1006d8980:     	b	0x1006d89a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd24>
1006d8984:     	cmp	w0, #0xc
1006d8988:     	b.eq	0x1006d899c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd1c>
1006d898c:     	cmp	w0, #0xe
1006d8990:     	b.ne	0x1006d8938 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1006d8994:     	orr	x14, x14, x13
1006d8998:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d899c:     	mov	x14, x13
1006d89a0:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d89a4:     	eor	x14, x14, x13
1006d89a8:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d89ac:     	and	x14, x14, x13
1006d89b0:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d89b4:     	bic	x14, x14, x13
1006d89b8:     	b	0x1006d89c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1006d89bc:     	bic	x14, x13, x14
1006d89c0:     	mov	x13, #0x0               ; =0
1006d89c4:     	mov	w17, #0x1               ; =1
1006d89c8:     	b	0x1006d89d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd54>
1006d89cc:     	eor	w17, w17, #0xf
1006d89d0:     	mvn	x13, x13
1006d89d4:     	and	w0, w17, #0xff
1006d89d8:     	cmp	w0, #0x7
1006d89dc:     	b.le	0x1006d89fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd7c>
1006d89e0:     	cmp	w0, #0xb
1006d89e4:     	b.gt	0x1006d8a18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd98>
1006d89e8:     	cmp	w0, #0x8
1006d89ec:     	b.eq	0x1006d8a40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdc0>
1006d89f0:     	cmp	w0, #0xa
1006d89f4:     	b.ne	0x1006d89cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1006d89f8:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d89fc:     	cmp	w0, #0x2
1006d8a00:     	b.eq	0x1006d8a48 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdc8>
1006d8a04:     	cmp	w0, #0x4
1006d8a08:     	b.eq	0x1006d8a50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd0>
1006d8a0c:     	cmp	w0, #0x6
1006d8a10:     	b.ne	0x1006d89cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1006d8a14:     	b	0x1006d8a38 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdb8>
1006d8a18:     	cmp	w0, #0xc
1006d8a1c:     	b.eq	0x1006d8a30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdb0>
1006d8a20:     	cmp	w0, #0xe
1006d8a24:     	b.ne	0x1006d89cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1006d8a28:     	orr	x11, x11, x10
1006d8a2c:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d8a30:     	mov	x11, x10
1006d8a34:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d8a38:     	eor	x11, x11, x10
1006d8a3c:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d8a40:     	and	x11, x11, x10
1006d8a44:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d8a48:     	bic	x11, x11, x10
1006d8a4c:     	b	0x1006d8a54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1006d8a50:     	bic	x11, x10, x11
1006d8a54:     	mov	x10, #0x0               ; =0
1006d8a58:     	mov	w17, #0x1               ; =1
1006d8a5c:     	b	0x1006d8a68 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde8>
1006d8a60:     	eor	w17, w17, #0xf
1006d8a64:     	mvn	x10, x10
1006d8a68:     	and	w0, w17, #0xff
1006d8a6c:     	cmp	w0, #0x7
1006d8a70:     	b.le	0x1006d8a90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe10>
1006d8a74:     	cmp	w0, #0xb
1006d8a78:     	b.gt	0x1006d8aac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe2c>
1006d8a7c:     	cmp	w0, #0x8
1006d8a80:     	b.eq	0x1006d8acc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1006d8a84:     	cmp	w0, #0xa
1006d8a88:     	b.ne	0x1006d8a60 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1006d8a8c:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8a90:     	cmp	w0, #0x2
1006d8a94:     	b.eq	0x1006d8ae4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe64>
1006d8a98:     	cmp	w0, #0x4
1006d8a9c:     	b.eq	0x1006d8ad4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe54>
1006d8aa0:     	cmp	w0, #0x6
1006d8aa4:     	b.ne	0x1006d8a60 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1006d8aa8:     	b	0x1006d8ac4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1006d8aac:     	cmp	w0, #0xc
1006d8ab0:     	b.eq	0x1006d8adc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe5c>
1006d8ab4:     	cmp	w0, #0xe
1006d8ab8:     	b.ne	0x1006d8a60 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1006d8abc:     	orr	x9, x9, x8
1006d8ac0:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8ac4:     	eor	x9, x9, x8
1006d8ac8:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8acc:     	and	x9, x9, x8
1006d8ad0:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8ad4:     	bic	x9, x8, x9
1006d8ad8:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8adc:     	mov	x9, x8
1006d8ae0:     	b	0x1006d8ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1006d8ae4:     	bic	x9, x9, x8
1006d8ae8:     	eor	x8, x11, x13
1006d8aec:     	eor	x11, x14, x15
1006d8af0:     	eor	x12, x12, x16
1006d8af4:     	stp	x12, x11, [sp]
1006d8af8:     	eor	x9, x9, x10
1006d8afc:     	stp	x8, x9, [sp, #0x10]
1006d8b00:     	b	0x1006d8d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1006d8b04:     	eor	x15, x12, x15
1006d8b08:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d8b0c:     	bic	x15, x12, x15
1006d8b10:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d8b14:     	mov	x15, x12
1006d8b18:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d8b1c:     	and	x15, x12, x15
1006d8b20:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d8b24:     	mov	x15, #0x0               ; =0
1006d8b28:     	b	0x1006d8b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1006d8b2c:     	bic	x15, x15, x12
1006d8b30:     	mov	x12, #0x0               ; =0
1006d8b34:     	mov	w17, #0xf               ; =15
1006d8b38:     	b	0x1006d8b44 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xec4>
1006d8b3c:     	eor	w17, w17, #0xf
1006d8b40:     	mvn	x12, x12
1006d8b44:     	and	w0, w17, #0xff
1006d8b48:     	cmp	w0, #0x7
1006d8b4c:     	b.gt	0x1006d8b68 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xee8>
1006d8b50:     	cmp	w0, #0x3
1006d8b54:     	b.gt	0x1006d8b84 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf04>
1006d8b58:     	cbz	w0, 0x1006d8bd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf50>
1006d8b5c:     	cmp	w0, #0x2
1006d8b60:     	b.ne	0x1006d8b3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1006d8b64:     	b	0x1006d8bb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf38>
1006d8b68:     	cmp	w0, #0xb
1006d8b6c:     	b.gt	0x1006d8b98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf18>
1006d8b70:     	cmp	w0, #0x8
1006d8b74:     	b.eq	0x1006d8bc8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf48>
1006d8b78:     	cmp	w0, #0xa
1006d8b7c:     	b.ne	0x1006d8b3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1006d8b80:     	b	0x1006d8bc0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf40>
1006d8b84:     	cmp	w0, #0x4
1006d8b88:     	b.eq	0x1006d8bd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf58>
1006d8b8c:     	cmp	w0, #0x6
1006d8b90:     	b.ne	0x1006d8b3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1006d8b94:     	b	0x1006d8bb0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf30>
1006d8b98:     	cmp	w0, #0xc
1006d8b9c:     	b.eq	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8ba0:     	cmp	w0, #0xe
1006d8ba4:     	b.ne	0x1006d8b3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1006d8ba8:     	orr	x13, x14, x13
1006d8bac:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bb0:     	eor	x13, x14, x13
1006d8bb4:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bb8:     	bic	x13, x14, x13
1006d8bbc:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bc0:     	mov	x13, x14
1006d8bc4:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bc8:     	and	x13, x14, x13
1006d8bcc:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bd0:     	mov	x13, #0x0               ; =0
1006d8bd4:     	b	0x1006d8bdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1006d8bd8:     	bic	x13, x13, x14
1006d8bdc:     	mov	x14, #0x0               ; =0
1006d8be0:     	mov	w17, #0xf               ; =15
1006d8be4:     	b	0x1006d8bf0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf70>
1006d8be8:     	eor	w17, w17, #0xf
1006d8bec:     	mvn	x14, x14
1006d8bf0:     	and	w0, w17, #0xff
1006d8bf4:     	cmp	w0, #0x7
1006d8bf8:     	b.gt	0x1006d8c14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf94>
1006d8bfc:     	cmp	w0, #0x3
1006d8c00:     	b.gt	0x1006d8c30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfb0>
1006d8c04:     	cbz	w0, 0x1006d8c7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xffc>
1006d8c08:     	cmp	w0, #0x2
1006d8c0c:     	b.ne	0x1006d8be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1006d8c10:     	b	0x1006d8c64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfe4>
1006d8c14:     	cmp	w0, #0xb
1006d8c18:     	b.gt	0x1006d8c44 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfc4>
1006d8c1c:     	cmp	w0, #0x8
1006d8c20:     	b.eq	0x1006d8c74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xff4>
1006d8c24:     	cmp	w0, #0xa
1006d8c28:     	b.ne	0x1006d8be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1006d8c2c:     	b	0x1006d8c6c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfec>
1006d8c30:     	cmp	w0, #0x4
1006d8c34:     	b.eq	0x1006d8c84 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1004>
1006d8c38:     	cmp	w0, #0x6
1006d8c3c:     	b.ne	0x1006d8be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1006d8c40:     	b	0x1006d8c5c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfdc>
1006d8c44:     	cmp	w0, #0xc
1006d8c48:     	b.eq	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c4c:     	cmp	w0, #0xe
1006d8c50:     	b.ne	0x1006d8be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1006d8c54:     	orr	x10, x11, x10
1006d8c58:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c5c:     	eor	x10, x11, x10
1006d8c60:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c64:     	bic	x10, x11, x10
1006d8c68:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c6c:     	mov	x10, x11
1006d8c70:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c74:     	and	x10, x11, x10
1006d8c78:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c7c:     	mov	x10, #0x0               ; =0
1006d8c80:     	b	0x1006d8c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1006d8c84:     	bic	x10, x10, x11
1006d8c88:     	mov	x11, #0x0               ; =0
1006d8c8c:     	mov	w17, #0xf               ; =15
1006d8c90:     	b	0x1006d8c9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x101c>
1006d8c94:     	eor	w17, w17, #0xf
1006d8c98:     	mvn	x11, x11
1006d8c9c:     	and	w0, w17, #0xff
1006d8ca0:     	cmp	w0, #0x7
1006d8ca4:     	b.gt	0x1006d8cc0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1040>
1006d8ca8:     	cmp	w0, #0x3
1006d8cac:     	b.gt	0x1006d8cdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x105c>
1006d8cb0:     	cbz	w0, 0x1006d8d28 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10a8>
1006d8cb4:     	cmp	w0, #0x2
1006d8cb8:     	b.ne	0x1006d8c94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1006d8cbc:     	b	0x1006d8d10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1090>
1006d8cc0:     	cmp	w0, #0xb
1006d8cc4:     	b.gt	0x1006d8cf0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1070>
1006d8cc8:     	cmp	w0, #0x8
1006d8ccc:     	b.eq	0x1006d8d20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10a0>
1006d8cd0:     	cmp	w0, #0xa
1006d8cd4:     	b.ne	0x1006d8c94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1006d8cd8:     	b	0x1006d8d18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1098>
1006d8cdc:     	cmp	w0, #0x4
1006d8ce0:     	b.eq	0x1006d8d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b0>
1006d8ce4:     	cmp	w0, #0x6
1006d8ce8:     	b.ne	0x1006d8c94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1006d8cec:     	b	0x1006d8d08 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1088>
1006d8cf0:     	cmp	w0, #0xc
1006d8cf4:     	b.eq	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8cf8:     	cmp	w0, #0xe
1006d8cfc:     	b.ne	0x1006d8c94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1006d8d00:     	orr	x8, x9, x8
1006d8d04:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d08:     	eor	x8, x9, x8
1006d8d0c:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d10:     	bic	x8, x9, x8
1006d8d14:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d18:     	mov	x8, x9
1006d8d1c:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d20:     	and	x8, x9, x8
1006d8d24:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d28:     	mov	x8, #0x0                ; =0
1006d8d2c:     	b	0x1006d8d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1006d8d30:     	bic	x8, x8, x9
1006d8d34:     	eor	x9, x10, x14
1006d8d38:     	eor	x10, x13, x12
1006d8d3c:     	eor	x12, x15, x16
1006d8d40:     	stp	x12, x10, [sp]
1006d8d44:     	eor	x8, x8, x11
1006d8d48:     	stp	x9, x8, [sp, #0x10]
1006d8d4c:     	mov	x1, sp
1006d8d50:     	mov	x0, x19
1006d8d54:     	bl	0x1006d7450 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E4leafB6_>
1006d8d58:     	mov	x21, x0
1006d8d5c:     	strb	w20, [sp, #0x4]
1006d8d60:     	str	w23, [sp]
1006d8d64:     	str	w24, [sp, #0x8]
1006d8d68:     	add	x0, x19, #0xa0
1006d8d6c:     	mov	x1, sp
1006d8d70:     	mov	x2, x21
1006d8d74:     	bl	0x1007284d8 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006d8d78:     	mov	x0, x21
1006d8d7c:     	ldp	x29, x30, [sp, #0x50]
1006d8d80:     	ldp	x20, x19, [sp, #0x40]
1006d8d84:     	ldp	x22, x21, [sp, #0x30]
1006d8d88:     	ldp	x24, x23, [sp, #0x20]
1006d8d8c:     	add	sp, sp, #0x60
1006d8d90:     	ret
1006d8d94:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d8d98:     	add	x2, x2, #0x760
1006d8d9c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d8da0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d8da4:     	add	x2, x2, #0x928
1006d8da8:     	mov	x0, x8
1006d8dac:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d8db0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d8db4:     	add	x2, x2, #0x928
1006d8db8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d8dbc:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d8dc0:     	add	x2, x2, #0x910
1006d8dc4:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d8dc8:     	nop
1006d8dcc:     	nop
1006d8dd0:     	nop
1006d8dd4:     	nop
1006d8dd8:     	nop
1006d8ddc:     	nop
1006d8de0:     	nop
1006d8de4:     	nop
1006d8de8:     	nop
1006d8dec:     	nop
1006d8df0:     	nop
1006d8df4:     	nop
1006d8df8:     	nop
1006d8dfc:     	nop
