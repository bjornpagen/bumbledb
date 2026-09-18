
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b12d80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_>:
100b12d80:     	stp	d15, d14, [sp, #-0xa0]!
100b12d84:     	stp	d13, d12, [sp, #0x10]
100b12d88:     	stp	d11, d10, [sp, #0x20]
100b12d8c:     	stp	d9, d8, [sp, #0x30]
100b12d90:     	stp	x28, x27, [sp, #0x40]
100b12d94:     	stp	x26, x25, [sp, #0x50]
100b12d98:     	stp	x24, x23, [sp, #0x60]
100b12d9c:     	stp	x22, x21, [sp, #0x70]
100b12da0:     	stp	x20, x19, [sp, #0x80]
100b12da4:     	stp	x29, x30, [sp, #0x90]
100b12da8:     	add	x29, sp, #0x90
100b12dac:     	sub	sp, sp, #0x220
100b12db0:     	ldr	w8, [x3, #0x10]
100b12db4:     	str	x8, [sp, #0xd0]
100b12db8:     	cbz	w8, 0x100b12dec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6c>
100b12dbc:     	mov	x25, x5
100b12dc0:     	mov	x24, x4
100b12dc4:     	mov	x22, x3
100b12dc8:     	mov	x26, x2
100b12dcc:     	mov	x27, x1
100b12dd0:     	mov	x28, x0
100b12dd4:     	mov	x0, x4
100b12dd8:     	mov	x1, x3
100b12ddc:     	bl	0x10065e4b4 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b12de0:     	cbz	x0, 0x100b12df4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x74>
100b12de4:     	ldrb	w27, [x0]
100b12de8:     	b	0x100b13504 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x784>
100b12dec:     	mov	w27, #0x0               ; =0
100b12df0:     	b	0x100b13504 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x784>
100b12df4:     	ldr	x8, [x25]
100b12df8:     	add	x8, x8, #0x1
100b12dfc:     	str	x8, [x25]
100b12e00:     	ldr	x1, [x28, #0x28]
100b12e04:     	ldr	x8, [sp, #0xd0]
100b12e08:     	lsr	x0, x8, #1
100b12e0c:     	cmp	x1, x0
100b12e10:     	b.ls	0x100b13548 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x7c8>
100b12e14:     	ldr	w8, [x22, #0x28]
100b12e18:     	str	x8, [sp, #0xc8]
100b12e1c:     	lsr	x9, x8, #1
100b12e20:     	cmp	x1, x9
100b12e24:     	b.ls	0x100b13544 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x7c4>
100b12e28:     	ldr	w8, [x22, #0x40]
100b12e2c:     	str	x8, [sp, #0xc0]
100b12e30:     	lsr	x8, x8, #1
100b12e34:     	cmp	x1, x8
100b12e38:     	b.ls	0x100b13554 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x7d4>
100b12e3c:     	ldr	x10, [x28, #0x20]
100b12e40:     	add	x9, x10, x9, lsl #5
100b12e44:     	ldr	x9, [x9, #0x18]
100b12e48:     	ldr	x11, [x22, #0x18]
100b12e4c:     	bic	x9, x9, x11
100b12e50:     	add	x11, x10, x0, lsl #5
100b12e54:     	ldr	x11, [x11, #0x18]
100b12e58:     	ldr	x12, [x22]
100b12e5c:     	bic	x11, x11, x12
100b12e60:     	orr	x9, x9, x11
100b12e64:     	ldr	x11, [x22, #0x30]
100b12e68:     	add	x8, x10, x8, lsl #5
100b12e6c:     	ldr	x8, [x8, #0x18]
100b12e70:     	bic	x8, x8, x11
100b12e74:     	orr	x20, x8, x9
100b12e78:     	fmov	d0, x20
100b12e7c:     	cnt.8b	v0, v0
100b12e80:     	addv.8b	b0, v0
100b12e84:     	fmov	x8, d0
100b12e88:     	cmp	x8, #0x7
100b12e8c:     	b.hs	0x100b12ef4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x174>
100b12e90:     	str	x24, [sp, #0x8]
100b12e94:     	str	x22, [sp, #0x18]
100b12e98:     	mov	w26, #0x4               ; =4
100b12e9c:     	stp	xzr, x26, [x29, #-0xc0]
100b12ea0:     	stur	xzr, [x29, #-0xb0]
100b12ea4:     	mov	w24, #0x1               ; =1
100b12ea8:     	mov	x22, #0x0               ; =0
100b12eac:     	cbz	x20, 0x100b1313c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x3bc>
100b12eb0:     	mov	w8, #0x4                ; =4
100b12eb4:     	b	0x100b12edc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x15c>
100b12eb8:     	ldur	x8, [x29, #-0xb8]
100b12ebc:     	rbit	x9, x20
100b12ec0:     	clz	x9, x9
100b12ec4:     	str	w9, [x8, x22, lsl #2]
100b12ec8:     	add	x22, x22, #0x1
100b12ecc:     	stur	x22, [x29, #-0xb0]
100b12ed0:     	sub	x9, x20, #0x1
100b12ed4:     	ands	x20, x9, x20
100b12ed8:     	b.eq	0x100b13018 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x298>
100b12edc:     	ldur	x9, [x29, #-0xc0]
100b12ee0:     	cmp	x22, x9
100b12ee4:     	b.ne	0x100b12ebc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x13c>
100b12ee8:     	sub	x0, x29, #0xc0
100b12eec:     	bl	0x1012692dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b12ef0:     	b	0x100b12eb8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x138>
100b12ef4:     	mov	x9, #0x0                ; =0
100b12ef8:     	sub	x19, x29, #0xf8
100b12efc:     	lsl	x10, x26, #2
100b12f00:     	cmp	x10, x9
100b12f04:     	b.eq	0x100b13538 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x7b8>
100b12f08:     	ldr	w8, [x27, x9]
100b12f0c:     	lsr	x11, x20, x8
100b12f10:     	add	x9, x9, #0x4
100b12f14:     	tbz	w11, #0x0, 0x100b12f00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x180>
100b12f18:     	ldr	q0, [x22]
100b12f1c:     	stur	q0, [x29, #-0xc0]
100b12f20:     	ldr	x9, [x22, #0x10]
100b12f24:     	stur	x9, [x29, #-0xb0]
100b12f28:     	sub	x0, x29, #0xf8
100b12f2c:     	sub	x1, x29, #0xc0
100b12f30:     	mov	x2, x28
100b12f34:     	mov	x20, x8
100b12f38:     	mov	x3, x20
100b12f3c:     	mov	w4, #0x0                ; =0
100b12f40:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b12f44:     	ldr	q0, [x19]
100b12f48:     	ldur	x8, [x29, #-0xe8]
100b12f4c:     	str	x8, [sp, #0x190]
100b12f50:     	stur	q0, [x29, #-0xe0]
100b12f54:     	stur	x8, [x29, #-0xd0]
100b12f58:     	str	q0, [sp, #0xe0]
100b12f5c:     	str	x8, [sp, #0xf0]
100b12f60:     	ldur	q0, [x22, #0x18]
100b12f64:     	stur	q0, [x29, #-0xc0]
100b12f68:     	ldur	x8, [x22, #0x28]
100b12f6c:     	stur	x8, [x29, #-0xb0]
100b12f70:     	sub	x0, x29, #0xf8
100b12f74:     	sub	x1, x29, #0xc0
100b12f78:     	mov	x2, x28
100b12f7c:     	mov	x3, x20
100b12f80:     	mov	w4, #0x0                ; =0
100b12f84:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b12f88:     	ldr	q0, [x19]
100b12f8c:     	ldur	x8, [x29, #-0xe8]
100b12f90:     	str	x8, [sp, #0x190]
100b12f94:     	stur	q0, [x29, #-0xe0]
100b12f98:     	stur	x8, [x29, #-0xd0]
100b12f9c:     	stur	q0, [sp, #0xf8]
100b12fa0:     	str	x8, [sp, #0x108]
100b12fa4:     	ldur	q0, [x22, #0x30]
100b12fa8:     	stur	q0, [x29, #-0xc0]
100b12fac:     	ldur	x8, [x22, #0x40]
100b12fb0:     	stur	x8, [x29, #-0xb0]
100b12fb4:     	sub	x0, x29, #0xf8
100b12fb8:     	sub	x1, x29, #0xc0
100b12fbc:     	mov	x2, x28
100b12fc0:     	mov	x21, x20
100b12fc4:     	mov	x3, x20
100b12fc8:     	mov	w4, #0x0                ; =0
100b12fcc:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b12fd0:     	ldr	q0, [x19]
100b12fd4:     	ldur	x8, [x29, #-0xe8]
100b12fd8:     	str	x8, [sp, #0x190]
100b12fdc:     	stur	q0, [x29, #-0xe0]
100b12fe0:     	str	q0, [sp, #0x110]
100b12fe4:     	str	x8, [sp, #0x120]
100b12fe8:     	add	x3, sp, #0xe0
100b12fec:     	mov	x0, x28
100b12ff0:     	mov	x1, x27
100b12ff4:     	mov	x2, x26
100b12ff8:     	mov	x4, x24
100b12ffc:     	mov	x5, x25
100b13000:     	bl	0x100b12d80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_>
100b13004:     	and	w8, w0, #0xff
100b13008:     	cmp	w8, #0xf
100b1300c:     	b.ne	0x100b13028 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x2a8>
100b13010:     	mov	w27, #0xf               ; =15
100b13014:     	b	0x100b134f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x774>
100b13018:     	ldp	x8, x26, [x29, #-0xc0]
100b1301c:     	cmp	x8, #0x0
100b13020:     	cset	w8, eq
100b13024:     	b	0x100b13140 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x3c0>
100b13028:     	mov	x23, x0
100b1302c:     	ldr	q0, [x22]
100b13030:     	stur	q0, [x29, #-0xc0]
100b13034:     	ldr	x8, [x22, #0x10]
100b13038:     	stur	x8, [x29, #-0xb0]
100b1303c:     	sub	x0, x29, #0xf8
100b13040:     	sub	x1, x29, #0xc0
100b13044:     	mov	x2, x28
100b13048:     	mov	x20, x21
100b1304c:     	mov	x3, x20
100b13050:     	mov	w4, #0x1                ; =1
100b13054:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b13058:     	ldr	q0, [x19]
100b1305c:     	str	q0, [sp, #0x1a0]
100b13060:     	ldur	x8, [x29, #-0xe8]
100b13064:     	str	q0, [sp, #0x180]
100b13068:     	stur	q0, [x29, #-0xe0]
100b1306c:     	stur	x8, [x29, #-0xd0]
100b13070:     	ldur	q0, [x29, #-0xe0]
100b13074:     	str	x8, [sp, #0x140]
100b13078:     	str	q0, [sp, #0x130]
100b1307c:     	ldur	q0, [x22, #0x18]
100b13080:     	stur	q0, [x29, #-0xc0]
100b13084:     	ldur	x8, [x22, #0x28]
100b13088:     	stur	x8, [x29, #-0xb0]
100b1308c:     	sub	x0, x29, #0xf8
100b13090:     	sub	x1, x29, #0xc0
100b13094:     	mov	x2, x28
100b13098:     	mov	x3, x20
100b1309c:     	mov	w4, #0x1                ; =1
100b130a0:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b130a4:     	ldr	q0, [x19]
100b130a8:     	str	q0, [sp, #0x1a0]
100b130ac:     	ldur	x8, [x29, #-0xe8]
100b130b0:     	str	q0, [sp, #0x180]
100b130b4:     	stur	q0, [x29, #-0xe0]
100b130b8:     	stur	x8, [x29, #-0xd0]
100b130bc:     	ldur	q0, [x29, #-0xe0]
100b130c0:     	str	x8, [sp, #0x158]
100b130c4:     	add	x8, sp, #0x49
100b130c8:     	stur	q0, [x8, #0xff]
100b130cc:     	ldur	q0, [x22, #0x30]
100b130d0:     	stur	q0, [x29, #-0xc0]
100b130d4:     	ldur	x8, [x22, #0x40]
100b130d8:     	stur	x8, [x29, #-0xb0]
100b130dc:     	sub	x0, x29, #0xf8
100b130e0:     	sub	x1, x29, #0xc0
100b130e4:     	mov	x2, x28
100b130e8:     	mov	x3, x20
100b130ec:     	mov	w4, #0x1                ; =1
100b130f0:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b130f4:     	ldr	q0, [x19]
100b130f8:     	str	q0, [sp, #0x1a0]
100b130fc:     	ldur	x8, [x29, #-0xe8]
100b13100:     	str	q0, [sp, #0x180]
100b13104:     	stur	q0, [x29, #-0xe0]
100b13108:     	stur	x8, [x29, #-0xd0]
100b1310c:     	ldur	q0, [x29, #-0xe0]
100b13110:     	str	x8, [sp, #0x170]
100b13114:     	str	q0, [sp, #0x160]
100b13118:     	add	x3, sp, #0x130
100b1311c:     	mov	x0, x28
100b13120:     	mov	x1, x27
100b13124:     	mov	x2, x26
100b13128:     	mov	x4, x24
100b1312c:     	mov	x5, x25
100b13130:     	bl	0x100b12d80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_>
100b13134:     	orr	w27, w0, w23
100b13138:     	b	0x100b134f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x774>
100b1313c:     	mov	w8, #0x1                ; =1
100b13140:     	str	w8, [sp, #0x14]
100b13144:     	mov	w27, #0x0               ; =0
100b13148:     	mov	x21, #0x0               ; =0
100b1314c:     	ldr	x8, [sp, #0x18]
100b13150:     	ldr	x10, [x8, #0x8]
100b13154:     	ldr	x9, [x8, #0x20]
100b13158:     	stp	x9, x10, [sp, #0xa8]
100b1315c:     	ldr	x8, [x8, #0x38]
100b13160:     	str	x8, [sp, #0xa0]
100b13164:     	str	x25, [sp, #0xb8]
100b13168:     	ldr	x8, [x25, #0x8]
100b1316c:     	str	x8, [sp, #0xd8]
100b13170:     	and	x8, x22, #0xfffffffffffffffe
100b13174:     	neg	x8, x8
100b13178:     	str	x8, [sp, #0x88]
100b1317c:     	mov	w23, #0x2               ; =2
100b13180:     	adrp	x8, 0x101301000 <dyld_stub_binder+0x101301000>
100b13184:     	ldr	q0, [x8]
100b13188:     	str	q0, [sp, #0x90]
100b1318c:     	mov	w8, #0x4                ; =4
100b13190:     	dup.2d	v1, x8
100b13194:     	mov	w8, #0x8                ; =8
100b13198:     	dup.2d	v0, x8
100b1319c:     	stp	q0, q1, [sp, #0x50]
100b131a0:     	mov	w8, #0xc                ; =12
100b131a4:     	dup.2d	v1, x8
100b131a8:     	mov	w8, #0x10               ; =16
100b131ac:     	dup.2d	v0, x8
100b131b0:     	stp	q0, q1, [sp, #0x30]
100b131b4:     	adrp	x8, 0x101301000 <dyld_stub_binder+0x101301000>
100b131b8:     	ldr	q0, [x8, #0x20]
100b131bc:     	str	q0, [sp, #0x20]
100b131c0:     	mov	w8, #0x3f               ; =63
100b131c4:     	dup.2d	v0, x8
100b131c8:     	str	q0, [sp, #0x70]
100b131cc:     	movi.2s	v8, #0x3f
100b131d0:     	b	0x100b131e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x464>
100b131d4:     	add	x21, x21, #0x1
100b131d8:     	and	x8, x22, #0x3f
100b131dc:     	lsr	x8, x21, x8
100b131e0:     	cbnz	x8, 0x100b134dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x75c>
100b131e4:     	ldr	x9, [sp, #0xd8]
100b131e8:     	add	x9, x9, #0x1
100b131ec:     	ldr	x8, [sp, #0xb8]
100b131f0:     	str	x9, [sp, #0xd8]
100b131f4:     	str	x9, [x8, #0x8]
100b131f8:     	mov	x25, x22
100b131fc:     	cbz	x22, 0x100b13470 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6f0>
100b13200:     	cmp	x22, #0x1
100b13204:     	b.ne	0x100b13214 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x494>
100b13208:     	mov	x8, #0x0                ; =0
100b1320c:     	mov	x25, #0x0               ; =0
100b13210:     	b	0x100b13450 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6d0>
100b13214:     	dup.2d	v0, x21
100b13218:     	cmp	x22, #0x10
100b1321c:     	b.hs	0x100b1322c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x4ac>
100b13220:     	mov	x9, #0x0                ; =0
100b13224:     	mov	x25, #0x0               ; =0
100b13228:     	b	0x100b133dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x65c>
100b1322c:     	movi.2d	v1, #0000000000000000
100b13230:     	add	x8, x26, #0x20
100b13234:     	movi.2d	v2, #0000000000000000
100b13238:     	and	x9, x22, #0x1ffffffffffffff0
100b1323c:     	ldr	q4, [sp, #0x90]
100b13240:     	ldp	q6, q15, [sp, #0x20]
100b13244:     	movi.2d	v3, #0000000000000000
100b13248:     	movi.2d	v7, #0000000000000000
100b1324c:     	movi.2d	v16, #0000000000000000
100b13250:     	movi.2d	v5, #0000000000000000
100b13254:     	movi.2d	v18, #0000000000000000
100b13258:     	movi.2d	v17, #0000000000000000
100b1325c:     	ldp	q13, q12, [sp, #0x50]
100b13260:     	ldr	q14, [sp, #0x40]
100b13264:     	mov	w10, #0x3f              ; =63
100b13268:     	movi.4s	v8, #0x3f
100b1326c:     	add.2d	v19, v4, v12
100b13270:     	add.2d	v20, v6, v12
100b13274:     	add.2d	v21, v4, v13
100b13278:     	add.2d	v22, v6, v13
100b1327c:     	add.2d	v23, v4, v14
100b13280:     	add.2d	v24, v6, v14
100b13284:     	ldp	q25, q26, [x8, #-0x20]
100b13288:     	dup.2d	v27, x10
100b1328c:     	ldp	q28, q29, [x8], #0x40
100b13290:     	and.16b	v30, v6, v27
100b13294:     	and.16b	v31, v4, v27
100b13298:     	and.16b	v20, v20, v27
100b1329c:     	and.16b	v19, v19, v27
100b132a0:     	and.16b	v22, v22, v27
100b132a4:     	and.16b	v21, v21, v27
100b132a8:     	and.16b	v24, v24, v27
100b132ac:     	and.16b	v23, v23, v27
100b132b0:     	neg.2d	v27, v31
100b132b4:     	ushl.2d	v27, v0, v27
100b132b8:     	neg.2d	v30, v30
100b132bc:     	ushl.2d	v30, v0, v30
100b132c0:     	neg.2d	v19, v19
100b132c4:     	ushl.2d	v19, v0, v19
100b132c8:     	neg.2d	v20, v20
100b132cc:     	ushl.2d	v20, v0, v20
100b132d0:     	neg.2d	v21, v21
100b132d4:     	ushl.2d	v21, v0, v21
100b132d8:     	neg.2d	v22, v22
100b132dc:     	ushl.2d	v22, v0, v22
100b132e0:     	neg.2d	v23, v23
100b132e4:     	ushl.2d	v23, v0, v23
100b132e8:     	neg.2d	v24, v24
100b132ec:     	ushl.2d	v24, v0, v24
100b132f0:     	dup.2d	v31, x24
100b132f4:     	and.16b	v30, v30, v31
100b132f8:     	and.16b	v27, v27, v31
100b132fc:     	and.16b	v20, v20, v31
100b13300:     	and.16b	v19, v19, v31
100b13304:     	and.16b	v22, v22, v31
100b13308:     	and.16b	v21, v21, v31
100b1330c:     	and.16b	v24, v24, v31
100b13310:     	and.16b	v23, v23, v31
100b13314:     	and.16b	v25, v25, v8
100b13318:     	and.16b	v26, v26, v8
100b1331c:     	and.16b	v28, v28, v8
100b13320:     	and.16b	v29, v29, v8
100b13324:     	ushll2.2d	v31, v25, #0x0
100b13328:     	ushll.2d	v25, v25, #0x0
100b1332c:     	ushll2.2d	v9, v26, #0x0
100b13330:     	ushll.2d	v26, v26, #0x0
100b13334:     	ushll2.2d	v10, v28, #0x0
100b13338:     	ushll.2d	v28, v28, #0x0
100b1333c:     	ushll2.2d	v11, v29, #0x0
100b13340:     	ushll.2d	v29, v29, #0x0
100b13344:     	ushl.2d	v25, v27, v25
100b13348:     	ushl.2d	v27, v30, v31
100b1334c:     	ushl.2d	v19, v19, v26
100b13350:     	ushl.2d	v20, v20, v9
100b13354:     	ushl.2d	v21, v21, v28
100b13358:     	ushl.2d	v22, v22, v10
100b1335c:     	ushl.2d	v23, v23, v29
100b13360:     	ushl.2d	v24, v24, v11
100b13364:     	orr.16b	v3, v27, v3
100b13368:     	orr.16b	v2, v25, v2
100b1336c:     	orr.16b	v16, v20, v16
100b13370:     	orr.16b	v7, v19, v7
100b13374:     	orr.16b	v18, v22, v18
100b13378:     	orr.16b	v5, v21, v5
100b1337c:     	orr.16b	v1, v24, v1
100b13380:     	orr.16b	v17, v23, v17
100b13384:     	add.2d	v6, v6, v15
100b13388:     	add.2d	v4, v4, v15
100b1338c:     	subs	x9, x9, #0x10
100b13390:     	b.ne	0x100b1326c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x4ec>
100b13394:     	orr.16b	v2, v7, v2
100b13398:     	orr.16b	v3, v16, v3
100b1339c:     	orr.16b	v3, v18, v3
100b133a0:     	orr.16b	v2, v5, v2
100b133a4:     	orr.16b	v2, v17, v2
100b133a8:     	orr.16b	v1, v1, v3
100b133ac:     	orr.16b	v1, v2, v1
100b133b0:     	mov	d2, v1[1]
100b133b4:     	orr.8b	v1, v1, v2
100b133b8:     	fmov	x25, d1
100b133bc:     	and	x8, x22, #0x1ffffffffffffff0
100b133c0:     	cmp	x22, x8
100b133c4:     	movi.2s	v8, #0x3f
100b133c8:     	b.eq	0x100b13470 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6f0>
100b133cc:     	and	x9, x22, #0x1ffffffffffffff0
100b133d0:     	and	x8, x22, #0x1ffffffffffffff0
100b133d4:     	and	x10, x22, #0xe
100b133d8:     	cbz	x10, 0x100b13450 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6d0>
100b133dc:     	fmov	d1, x25
100b133e0:     	dup.2d	v2, x9
100b133e4:     	ldr	q3, [sp, #0x90]
100b133e8:     	orr.16b	v2, v2, v3
100b133ec:     	ldr	x8, [sp, #0x88]
100b133f0:     	add	x8, x8, x9
100b133f4:     	add	x9, x26, x9, lsl #2
100b133f8:     	ldr	q6, [sp, #0x70]
100b133fc:     	ldr	d3, [x9], #0x8
100b13400:     	and.16b	v4, v2, v6
100b13404:     	neg.2d	v4, v4
100b13408:     	ushl.2d	v4, v0, v4
100b1340c:     	dup.2d	v5, x24
100b13410:     	and.16b	v4, v4, v5
100b13414:     	and.8b	v3, v3, v8
100b13418:     	ushll.2d	v3, v3, #0x0
100b1341c:     	ushl.2d	v3, v4, v3
100b13420:     	orr.16b	v1, v3, v1
100b13424:     	dup.2d	v3, x23
100b13428:     	add.2d	v2, v2, v3
100b1342c:     	adds	x8, x8, #0x2
100b13430:     	b.ne	0x100b133fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x67c>
100b13434:     	mov	d0, v1[1]
100b13438:     	orr.8b	v0, v1, v0
100b1343c:     	fmov	x25, d0
100b13440:     	and	x8, x22, #0x1ffffffffffffffe
100b13444:     	and	x9, x22, #0x1ffffffffffffffe
100b13448:     	cmp	x22, x9
100b1344c:     	b.eq	0x100b13470 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6f0>
100b13450:     	ldr	w9, [x26, x8, lsl #2]
100b13454:     	lsr	x10, x21, x8
100b13458:     	and	x10, x10, #0x1
100b1345c:     	lsl	x9, x10, x9
100b13460:     	orr	x25, x9, x25
100b13464:     	add	x8, x8, #0x1
100b13468:     	cmp	x22, x8
100b1346c:     	b.ne	0x100b13450 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x6d0>
100b13470:     	ldr	x8, [sp, #0xb0]
100b13474:     	orr	x2, x8, x25
100b13478:     	mov	x0, x28
100b1347c:     	ldr	x1, [sp, #0xd0]
100b13480:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b13484:     	mov	x19, x0
100b13488:     	ldr	x8, [sp, #0xa8]
100b1348c:     	orr	x2, x8, x25
100b13490:     	mov	x0, x28
100b13494:     	ldr	x1, [sp, #0xc8]
100b13498:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b1349c:     	mov	x20, x0
100b134a0:     	ldr	x8, [sp, #0xa0]
100b134a4:     	orr	x2, x8, x25
100b134a8:     	mov	x0, x28
100b134ac:     	ldr	x1, [sp, #0xc0]
100b134b0:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b134b4:     	tbz	w19, #0x0, 0x100b131d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x454>
100b134b8:     	cmp	w20, #0x0
100b134bc:     	csel	w8, w23, wzr, ne
100b134c0:     	orr	w8, w8, w0
100b134c4:     	lsl	w8, w24, w8
100b134c8:     	orr	w27, w8, w27
100b134cc:     	and	w8, w27, #0xff
100b134d0:     	cmp	w8, #0xf
100b134d4:     	b.ne	0x100b131d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x454>
100b134d8:     	mov	w27, #0xf               ; =15
100b134dc:     	ldr	w8, [sp, #0x14]
100b134e0:     	tbnz	w8, #0x0, 0x100b134ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x76c>
100b134e4:     	mov	x0, x26
100b134e8:     	bl	0x1012708f8 <dyld_stub_binder+0x1012708f8>
100b134ec:     	ldr	x22, [sp, #0x18]
100b134f0:     	ldr	x24, [sp, #0x8]
100b134f4:     	mov	x0, x24
100b134f8:     	mov	x1, x22
100b134fc:     	mov	x2, x27
100b13500:     	bl	0x100c169ec <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b13504:     	mov	x0, x27
100b13508:     	add	sp, sp, #0x220
100b1350c:     	ldp	x29, x30, [sp, #0x90]
100b13510:     	ldp	x20, x19, [sp, #0x80]
100b13514:     	ldp	x22, x21, [sp, #0x70]
100b13518:     	ldp	x24, x23, [sp, #0x60]
100b1351c:     	ldp	x26, x25, [sp, #0x50]
100b13520:     	ldp	x28, x27, [sp, #0x40]
100b13524:     	ldp	d9, d8, [sp, #0x30]
100b13528:     	ldp	d11, d10, [sp, #0x20]
100b1352c:     	ldp	d13, d12, [sp, #0x10]
100b13530:     	ldp	d15, d14, [sp], #0xa0
100b13534:     	ret
100b13538:     	adrp	x0, 0x1014a4000 <dyld_stub_binder+0x1014a4000>
100b1353c:     	add	x0, x0, #0x858
100b13540:     	bl	0x101268574 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b13544:     	mov	x0, x9
100b13548:     	adrp	x2, 0x10149c000 <dyld_stub_binder+0x10149c000>
100b1354c:     	add	x2, x2, #0x18
100b13550:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b13554:     	mov	x0, x8
100b13558:     	adrp	x2, 0x10149c000 <dyld_stub_binder+0x10149c000>
100b1355c:     	add	x2, x2, #0x18
100b13560:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b13564:     	mov	x19, x0
100b13568:     	ldur	x8, [x29, #-0xc0]
100b1356c:     	cbz	x8, 0x100b1358c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x80c>
100b13570:     	ldur	x26, [x29, #-0xb8]
100b13574:     	b	0x100b13584 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x804>
100b13578:     	mov	x19, x0
100b1357c:     	ldr	w8, [sp, #0x14]
100b13580:     	tbnz	w8, #0x0, 0x100b1358c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm6_EB8_+0x80c>
100b13584:     	mov	x0, x26
100b13588:     	bl	0x1012708f8 <dyld_stub_binder+0x1012708f8>
100b1358c:     	mov	x0, x19
100b13590:     	bl	0x101270748 <dyld_stub_binder+0x101270748>
100b13594:     	nop
100b13598:     	nop
100b1359c:     	nop
100b135a0:     	nop
100b135a4:     	nop
100b135a8:     	nop
100b135ac:     	nop
100b135b0:     	nop
100b135b4:     	nop
100b135b8:     	nop
100b135bc:     	nop
