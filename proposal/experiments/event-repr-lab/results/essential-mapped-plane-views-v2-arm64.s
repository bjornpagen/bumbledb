
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>:
100bb7de8:     	sub	sp, sp, #0x1e0
100bb7dec:     	stp	d15, d14, [sp, #0x140]
100bb7df0:     	stp	d13, d12, [sp, #0x150]
100bb7df4:     	stp	d11, d10, [sp, #0x160]
100bb7df8:     	stp	d9, d8, [sp, #0x170]
100bb7dfc:     	stp	x28, x27, [sp, #0x180]
100bb7e00:     	stp	x26, x25, [sp, #0x190]
100bb7e04:     	stp	x24, x23, [sp, #0x1a0]
100bb7e08:     	stp	x22, x21, [sp, #0x1b0]
100bb7e0c:     	stp	x20, x19, [sp, #0x1c0]
100bb7e10:     	stp	x29, x30, [sp, #0x1d0]
100bb7e14:     	add	x29, sp, #0x1d0
100bb7e18:     	str	x6, [sp, #0x98]
100bb7e1c:     	mov	x24, x5
100bb7e20:     	mov	x20, x4
100bb7e24:     	mov	x27, x3
100bb7e28:     	mov	x25, x2
100bb7e2c:     	mov	x23, x1
100bb7e30:     	mov	x28, x0
100bb7e34:     	str	x4, [sp, #0xb8]
100bb7e38:     	ldr	x21, [x2]
100bb7e3c:     	ldp	x22, x26, [x3, #0x8]
100bb7e40:     	cbz	x21, 0x100bb7e84 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9c>
100bb7e44:     	mov	x8, #0x0                ; =0
100bb7e48:     	mov	w9, #0x1                ; =1
100bb7e4c:     	mov	x10, x21
100bb7e50:     	rbit	x11, x10
100bb7e54:     	clz	x0, x11
100bb7e58:     	cmp	x0, x26
100bb7e5c:     	b.hs	0x100bb8bf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe10>
100bb7e60:     	ldr	w11, [x22, x0, lsl #2]
100bb7e64:     	lsl	x11, x9, x11
100bb7e68:     	orr	x8, x11, x8
100bb7e6c:     	sub	x11, x10, #0x1
100bb7e70:     	ands	x10, x11, x10
100bb7e74:     	b.ne	0x100bb7e50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x68>
100bb7e78:     	ands	x8, x8, x20
100bb7e7c:     	stur	x8, [x29, #-0xb8]
100bb7e80:     	b.ne	0x100bb8b30 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd48>
100bb7e84:     	ldr	w2, [x25, #0x10]
100bb7e88:     	sub	x0, x29, #0xb8
100bb7e8c:     	add	x1, x23, #0x30
100bb7e90:     	str	x2, [sp, #0x78]
100bb7e94:     	bl	0x100d98fac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100bb7e98:     	ldur	w8, [x29, #-0xb8]
100bb7e9c:     	cbz	w8, 0x100bb7f24 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x13c>
100bb7ea0:     	cmp	w8, #0x1
100bb7ea4:     	str	x20, [sp, #0x60]
100bb7ea8:     	b.ne	0x100bb7f3c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x154>
100bb7eac:     	ldp	x23, x8, [x29, #-0xb0]
100bb7eb0:     	str	x8, [sp, #0xa0]
100bb7eb4:     	ldur	x27, [x29, #-0xa0]
100bb7eb8:     	mov	w8, #0x4                ; =4
100bb7ebc:     	stp	xzr, x8, [x29, #-0xb8]
100bb7ec0:     	stur	xzr, [x29, #-0xa8]
100bb7ec4:     	str	x28, [sp, #0x58]
100bb7ec8:     	cbz	x21, 0x100bb8260 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x478>
100bb7ecc:     	mov	x28, #0x0               ; =0
100bb7ed0:     	mov	w8, #0x4                ; =4
100bb7ed4:     	mov	w9, #0x1                ; =1
100bb7ed8:     	b	0x100bb7f04 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x11c>
100bb7edc:     	ldur	x8, [x29, #-0xb0]
100bb7ee0:     	rbit	x9, x21
100bb7ee4:     	clz	x9, x9
100bb7ee8:     	str	w9, [x8, x28]
100bb7eec:     	stur	x19, [x29, #-0xa8]
100bb7ef0:     	sub	x10, x21, #0x1
100bb7ef4:     	add	x28, x28, #0x4
100bb7ef8:     	add	x9, x19, #0x1
100bb7efc:     	ands	x21, x10, x21
100bb7f00:     	b.eq	0x100bb8148 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x360>
100bb7f04:     	mov	x19, x9
100bb7f08:     	sub	x9, x9, #0x1
100bb7f0c:     	ldur	x10, [x29, #-0xb8]
100bb7f10:     	cmp	x9, x10
100bb7f14:     	b.ne	0x100bb7ee0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf8>
100bb7f18:     	sub	x0, x29, #0xb8
100bb7f1c:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bb7f20:     	b	0x100bb7edc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf4>
100bb7f24:     	ldr	x8, [sp, #0x78]
100bb7f28:     	and	w8, w8, #0x1
100bb7f2c:     	strb	w8, [x28, #0x8]
100bb7f30:     	mov	x8, #-0x2               ; =-2
100bb7f34:     	str	x8, [x28]
100bb7f38:     	b	0x100bb8b00 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bb7f3c:     	ldp	w19, w8, [x29, #-0xb4]
100bb7f40:     	ldur	w9, [x29, #-0xac]
100bb7f44:     	ldr	x11, [sp, #0x98]
100bb7f48:     	ldr	x10, [x11, #0x38]
100bb7f4c:     	add	x10, x10, #0x1
100bb7f50:     	str	x10, [x11, #0x38]
100bb7f54:     	tbz	w24, #0x0, 0x100bb8208 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x420>
100bb7f58:     	lsr	x10, x21, x19
100bb7f5c:     	and	x11, x10, #0x1
100bb7f60:     	stur	x11, [x29, #-0xb8]
100bb7f64:     	tbnz	w10, #0x0, 0x100bb8b94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdac>
100bb7f68:     	ldr	x10, [sp, #0x78]
100bb7f6c:     	and	w10, w10, #0x1
100bb7f70:     	eor	w8, w8, w10
100bb7f74:     	eor	w20, w9, w10
100bb7f78:     	stur	w8, [x29, #-0xa8]
100bb7f7c:     	ldr	x24, [x25, #0x8]
100bb7f80:     	stp	x21, x24, [x29, #-0xb8]
100bb7f84:     	add	x0, sp, #0xc0
100bb7f88:     	sub	x1, x29, #0xb8
100bb7f8c:     	mov	x2, x23
100bb7f90:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100bb7f94:     	stur	w20, [x29, #-0xa8]
100bb7f98:     	stp	x21, x24, [x29, #-0xb8]
100bb7f9c:     	add	x0, sp, #0xd8
100bb7fa0:     	sub	x1, x29, #0xb8
100bb7fa4:     	mov	x2, x23
100bb7fa8:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100bb7fac:     	sub	x0, x29, #0xe0
100bb7fb0:     	add	x2, sp, #0xc0
100bb7fb4:     	mov	x1, x23
100bb7fb8:     	mov	x3, x27
100bb7fbc:     	ldr	x21, [sp, #0x60]
100bb7fc0:     	mov	x4, x21
100bb7fc4:     	mov	w5, #0x1                ; =1
100bb7fc8:     	ldr	x20, [sp, #0x98]
100bb7fcc:     	mov	x6, x20
100bb7fd0:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100bb7fd4:     	sub	x0, x29, #0xb8
100bb7fd8:     	add	x2, sp, #0xd8
100bb7fdc:     	mov	x1, x23
100bb7fe0:     	mov	x3, x27
100bb7fe4:     	mov	x4, x21
100bb7fe8:     	mov	w5, #0x1                ; =1
100bb7fec:     	mov	x6, x20
100bb7ff0:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100bb7ff4:     	cmp	x26, x19
100bb7ff8:     	b.ls	0x100bb8c88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xea0>
100bb7ffc:     	ldr	w21, [x22, x19, lsl #2]
100bb8000:     	ldr	x10, [sp, #0x60]
100bb8004:     	lsr	x8, x10, x21
100bb8008:     	and	x9, x8, #0x1
100bb800c:     	stur	x9, [x29, #-0xc0]
100bb8010:     	tbz	w8, #0x0, 0x100bb8bb4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdcc>
100bb8014:     	fmov	d0, x10
100bb8018:     	cnt.8b	v0, v0
100bb801c:     	addv.8b	b0, v0
100bb8020:     	fmov	x8, d0
100bb8024:     	and	x9, x8, #0x3f
100bb8028:     	mov	w10, #0x1               ; =1
100bb802c:     	lsl	x8, x10, x8
100bb8030:     	lsr	x8, x8, #6
100bb8034:     	cmp	x9, #0x6
100bb8038:     	cinc	x20, x8, lo
100bb803c:     	cbz	x20, 0x100bb88a0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xab8>
100bb8040:     	lsl	x19, x20, #3
100bb8044:     	mov	x0, x19
100bb8048:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
100bb804c:     	cbz	x0, 0x100bb8cb0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xec8>
100bb8050:     	mov	x9, #0x0                ; =0
100bb8054:     	and	x8, x21, #0x3f
100bb8058:     	mov	x10, #-0x1              ; =-1
100bb805c:     	lsl	x8, x10, x8
100bb8060:     	ldr	x10, [sp, #0x60]
100bb8064:     	bic	x8, x10, x8
100bb8068:     	fmov	d0, x8
100bb806c:     	cnt.8b	v0, v0
100bb8070:     	addv.8b	b0, v0
100bb8074:     	fmov	x10, d0
100bb8078:     	add	w8, w10, #0x3a
100bb807c:     	mov	w11, #0x1               ; =1
100bb8080:     	lsl	x11, x11, x8
100bb8084:     	ldp	x12, x13, [x29, #-0xe0]
100bb8088:     	ldp	x1, x14, [x29, #-0xd0]
100bb808c:     	sub	x15, x9, w13, uxtb
100bb8090:     	ldp	x21, x17, [x29, #-0xb8]
100bb8094:     	ldp	x16, x2, [x29, #-0xa8]
100bb8098:     	adrp	x3, 0x10145c000 <dyld_stub_binder+0x10145c000>
100bb809c:     	add	x3, x3, #0xb50
100bb80a0:     	mov	x8, #0x0                ; =0
100bb80a4:     	b	0x100bb80c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100bb80a8:     	tst	w17, #0x1
100bb80ac:     	csel	x6, x4, x9, ne
100bb80b0:     	bic	x4, x5, x4
100bb80b4:     	orr	x4, x6, x4
100bb80b8:     	str	x4, [x0, x8, lsl #3]
100bb80bc:     	add	x8, x8, #0x1
100bb80c0:     	cmp	x20, x8
100bb80c4:     	b.eq	0x100bb8140 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x358>
100bb80c8:     	cmp	x10, #0x6
100bb80cc:     	b.hs	0x100bb80e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x300>
100bb80d0:     	ldr	x4, [x3, x10, lsl #3]
100bb80d4:     	mvn	x4, x4
100bb80d8:     	mov	x5, x15
100bb80dc:     	cmn	x12, #0x2
100bb80e0:     	b.ne	0x100bb80fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x314>
100bb80e4:     	b	0x100bb810c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100bb80e8:     	tst	x8, x11
100bb80ec:     	csetm	x4, ne
100bb80f0:     	mov	x5, x15
100bb80f4:     	cmn	x12, #0x2
100bb80f8:     	b.eq	0x100bb810c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100bb80fc:     	cmp	x8, x1
100bb8100:     	b.hs	0x100bb8c60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe78>
100bb8104:     	ldr	x5, [x13, x8, lsl #3]
100bb8108:     	eor	x5, x14, x5
100bb810c:     	cmn	x21, #0x2
100bb8110:     	b.eq	0x100bb80a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2c0>
100bb8114:     	cmp	x8, x16
100bb8118:     	b.hs	0x100bb8c54 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe6c>
100bb811c:     	ldr	x6, [x17, x8, lsl #3]
100bb8120:     	eor	x6, x2, x6
100bb8124:     	and	x6, x6, x4
100bb8128:     	bic	x4, x5, x4
100bb812c:     	orr	x4, x6, x4
100bb8130:     	str	x4, [x0, x8, lsl #3]
100bb8134:     	add	x8, x8, #0x1
100bb8138:     	cmp	x20, x8
100bb813c:     	b.ne	0x100bb80c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100bb8140:     	mov	x8, x20
100bb8144:     	b	0x100bb88ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xac4>
100bb8148:     	ldp	x8, x21, [x29, #-0xb8]
100bb814c:     	str	x8, [sp, #0x40]
100bb8150:     	str	x21, [sp, #0x30]
100bb8154:     	cbz	x19, 0x100bb82c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e0>
100bb8158:     	ldr	x8, [x25, #0x8]
100bb815c:     	str	x8, [sp, #0x80]
100bb8160:     	mov	x24, #-0x1              ; =-1
100bb8164:     	mov	x19, x23
100bb8168:     	b	0x100bb8194 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x3ac>
100bb816c:     	bic	x27, x27, x23
100bb8170:     	ldr	x9, [sp, #0x98]
100bb8174:     	ldr	x8, [x9, #0x20]
100bb8178:     	add	x8, x8, #0x1
100bb817c:     	str	x8, [x9, #0x20]
100bb8180:     	mov	x24, x20
100bb8184:     	mov	x23, x25
100bb8188:     	mov	x19, x25
100bb818c:     	subs	x28, x28, #0x4
100bb8190:     	b.eq	0x100bb82cc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e4>
100bb8194:     	ldr	w8, [x21], #0x4
100bb8198:     	mov	w9, #0x1                ; =1
100bb819c:     	lsl	x23, x9, x8
100bb81a0:     	sub	x9, x23, #0x1
100bb81a4:     	and	x9, x9, x27
100bb81a8:     	fmov	d0, x9
100bb81ac:     	cnt.8b	v0, v0
100bb81b0:     	addv.8b	b0, v0
100bb81b4:     	fmov	w4, s0
100bb81b8:     	fmov	d0, x27
100bb81bc:     	cnt.8b	v0, v0
100bb81c0:     	addv.8b	b0, v0
100bb81c4:     	fmov	w3, s0
100bb81c8:     	ldr	x9, [sp, #0x80]
100bb81cc:     	lsr	x8, x9, x8
100bb81d0:     	sub	x0, x29, #0xb8
100bb81d4:     	and	w5, w8, #0x1
100bb81d8:     	mov	x1, x19
100bb81dc:     	ldr	x2, [sp, #0xa0]
100bb81e0:     	bl	0x100e400d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100bb81e4:     	ldp	x20, x25, [x29, #-0xb8]
100bb81e8:     	ldur	x8, [x29, #-0xa8]
100bb81ec:     	str	x8, [sp, #0xa0]
100bb81f0:     	sub	x8, x24, #0x1
100bb81f4:     	cmn	x8, #0x3
100bb81f8:     	b.hi	0x100bb816c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100bb81fc:     	mov	x0, x19
100bb8200:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8204:     	b	0x100bb816c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100bb8208:     	mov	w8, #0x4                ; =4
100bb820c:     	stp	xzr, x8, [x29, #-0xb8]
100bb8210:     	stur	xzr, [x29, #-0xa8]
100bb8214:     	mov	x26, #0x0               ; =0
100bb8218:     	cbz	x20, 0x100bb850c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x724>
100bb821c:     	mov	w8, #0x4                ; =4
100bb8220:     	b	0x100bb8248 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x460>
100bb8224:     	ldur	x8, [x29, #-0xb0]
100bb8228:     	rbit	x9, x20
100bb822c:     	clz	x9, x9
100bb8230:     	str	w9, [x8, x26, lsl #2]
100bb8234:     	add	x26, x26, #0x1
100bb8238:     	stur	x26, [x29, #-0xa8]
100bb823c:     	sub	x9, x20, #0x1
100bb8240:     	ands	x20, x9, x20
100bb8244:     	b.eq	0x100bb8270 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x488>
100bb8248:     	ldur	x9, [x29, #-0xb8]
100bb824c:     	cmp	x26, x9
100bb8250:     	b.ne	0x100bb8228 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x440>
100bb8254:     	sub	x0, x29, #0xb8
100bb8258:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bb825c:     	b	0x100bb8224 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x43c>
100bb8260:     	mov	x19, #-0x1              ; =-1
100bb8264:     	mov	x21, #0x0               ; =0
100bb8268:     	cbnz	x27, 0x100bb82ec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x504>
100bb826c:     	b	0x100bb831c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100bb8270:     	ldp	x20, x19, [x29, #-0xb8]
100bb8274:     	cbz	x26, 0x100bb8928 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb40>
100bb8278:     	lsl	x22, x26, #2
100bb827c:     	mov	x0, x22
100bb8280:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
100bb8284:     	cbz	x0, 0x100bb8cc0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xed8>
100bb8288:     	mov	x24, x0
100bb828c:     	mov	x8, #0x0                ; =0
100bb8290:     	ldp	x9, x1, [x27, #0x20]
100bb8294:     	ldr	w0, [x19, x8, lsl #2]
100bb8298:     	cmp	x1, x0
100bb829c:     	b.ls	0x100bb8c44 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe5c>
100bb82a0:     	ldr	w10, [x9, x0, lsl #2]
100bb82a4:     	str	w10, [x24, x8, lsl #2]
100bb82a8:     	add	x8, x8, #0x1
100bb82ac:     	cmp	x26, x8
100bb82b0:     	b.ne	0x100bb8294 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4ac>
100bb82b4:     	cbz	x20, 0x100bb82c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100bb82b8:     	mov	x0, x19
100bb82bc:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb82c0:     	ldr	x20, [sp, #0x60]
100bb82c4:     	b	0x100bb8510 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x728>
100bb82c8:     	mov	x20, #-0x1              ; =-1
100bb82cc:     	ldr	x8, [sp, #0x40]
100bb82d0:     	cbz	x8, 0x100bb82dc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4f4>
100bb82d4:     	ldr	x0, [sp, #0x30]
100bb82d8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb82dc:     	mov	x19, x20
100bb82e0:     	ldp	x28, x20, [sp, #0x58]
100bb82e4:     	mov	x21, #0x0               ; =0
100bb82e8:     	cbz	x27, 0x100bb831c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100bb82ec:     	mov	w8, #0x1                ; =1
100bb82f0:     	mov	x9, x27
100bb82f4:     	rbit	x10, x9
100bb82f8:     	clz	x0, x10
100bb82fc:     	cmp	x0, x26
100bb8300:     	b.hs	0x100bb8c08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe20>
100bb8304:     	ldr	w10, [x22, x0, lsl #2]
100bb8308:     	lsl	x10, x8, x10
100bb830c:     	orr	x21, x10, x21
100bb8310:     	sub	x10, x9, #0x1
100bb8314:     	ands	x9, x10, x9
100bb8318:     	b.ne	0x100bb82f4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x50c>
100bb831c:     	stur	x21, [x29, #-0xe0]
100bb8320:     	bics	x8, x21, x20
100bb8324:     	stur	x8, [x29, #-0xb8]
100bb8328:     	b.ne	0x100bb8b50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd68>
100bb832c:     	str	x19, [sp, #0x80]
100bb8330:     	mov	w25, #0x4               ; =4
100bb8334:     	stp	xzr, x25, [x29, #-0xb8]
100bb8338:     	stur	xzr, [x29, #-0xa8]
100bb833c:     	mov	x19, #0x0               ; =0
100bb8340:     	cbz	x21, 0x100bb84a0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6b8>
100bb8344:     	mov	w8, #0x4                ; =4
100bb8348:     	mov	x20, x21
100bb834c:     	b	0x100bb8370 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x588>
100bb8350:     	rbit	x9, x20
100bb8354:     	clz	x9, x9
100bb8358:     	str	w9, [x8, x19, lsl #2]
100bb835c:     	add	x19, x19, #0x1
100bb8360:     	stur	x19, [x29, #-0xa8]
100bb8364:     	sub	x9, x20, #0x1
100bb8368:     	ands	x20, x9, x20
100bb836c:     	b.eq	0x100bb838c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5a4>
100bb8370:     	ldur	x9, [x29, #-0xb8]
100bb8374:     	cmp	x19, x9
100bb8378:     	b.ne	0x100bb8350 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100bb837c:     	sub	x0, x29, #0xb8
100bb8380:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bb8384:     	ldur	x8, [x29, #-0xb0]
100bb8388:     	b	0x100bb8350 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100bb838c:     	ldp	x8, x25, [x29, #-0xb8]
100bb8390:     	cmp	x8, #0x0
100bb8394:     	cset	w8, eq
100bb8398:     	str	w8, [sp, #0x40]
100bb839c:     	mov	w8, #0x4                ; =4
100bb83a0:     	stp	xzr, x8, [x29, #-0xb8]
100bb83a4:     	stur	xzr, [x29, #-0xa8]
100bb83a8:     	cbz	x27, 0x100bb84b8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6d0>
100bb83ac:     	str	x23, [sp, #0x30]
100bb83b0:     	mov	x20, #0x0               ; =0
100bb83b4:     	mov	w8, #0x4                ; =4
100bb83b8:     	b	0x100bb83dc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5f4>
100bb83bc:     	rbit	x9, x27
100bb83c0:     	clz	x9, x9
100bb83c4:     	str	w9, [x8, x28, lsl #2]
100bb83c8:     	add	x20, x28, #0x1
100bb83cc:     	stur	x20, [x29, #-0xa8]
100bb83d0:     	sub	x9, x27, #0x1
100bb83d4:     	ands	x27, x9, x27
100bb83d8:     	b.eq	0x100bb83fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x614>
100bb83dc:     	mov	x28, x20
100bb83e0:     	ldur	x9, [x29, #-0xb8]
100bb83e4:     	cmp	x20, x9
100bb83e8:     	b.ne	0x100bb83bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100bb83ec:     	sub	x0, x29, #0xb8
100bb83f0:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bb83f4:     	ldur	x8, [x29, #-0xb0]
100bb83f8:     	b	0x100bb83bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100bb83fc:     	ldp	x8, x24, [x29, #-0xb8]
100bb8400:     	cbz	x20, 0x100bb84f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x710>
100bb8404:     	str	x8, [sp, #0x20]
100bb8408:     	lsl	x0, x20, #2
100bb840c:     	mov	x23, x0
100bb8410:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
100bb8414:     	cbz	x0, 0x100bb8ca0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xeb8>
100bb8418:     	mov	x27, x0
100bb841c:     	cbz	x19, 0x100bb846c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x684>
100bb8420:     	mov	x9, #0x0                ; =0
100bb8424:     	lsl	x8, x19, #2
100bb8428:     	b	0x100bb843c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x654>
100bb842c:     	str	w10, [x27, x9, lsl #2]
100bb8430:     	cmp	x9, x28
100bb8434:     	add	x9, x9, #0x1
100bb8438:     	b.eq	0x100bb847c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x694>
100bb843c:     	ldr	w0, [x24, x9, lsl #2]
100bb8440:     	cmp	x26, x0
100bb8444:     	b.ls	0x100bb8c1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe34>
100bb8448:     	mov	x10, #0x0               ; =0
100bb844c:     	ldr	w11, [x22, x0, lsl #2]
100bb8450:     	mov	x12, x8
100bb8454:     	ldr	w13, [x25, x10, lsl #2]
100bb8458:     	cmp	w13, w11
100bb845c:     	b.eq	0x100bb842c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x644>
100bb8460:     	add	x10, x10, #0x1
100bb8464:     	subs	x12, x12, #0x4
100bb8468:     	b.ne	0x100bb8454 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x66c>
100bb846c:     	adrp	x0, 0x101601000 <dyld_stub_binder+0x101601000>
100bb8470:     	add	x0, x0, #0xe10
100bb8474:     	bl	0x1013b4df4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100bb8478:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb847c:     	ldr	x19, [sp, #0x80]
100bb8480:     	ldr	x8, [sp, #0x20]
100bb8484:     	ldr	x28, [sp, #0x58]
100bb8488:     	cbz	x8, 0x100bb8494 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100bb848c:     	mov	x0, x24
100bb8490:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8494:     	mov	x24, x27
100bb8498:     	ldr	x23, [sp, #0x30]
100bb849c:     	b	0x100bb84c4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6dc>
100bb84a0:     	mov	w8, #0x1                ; =1
100bb84a4:     	str	w8, [sp, #0x40]
100bb84a8:     	mov	w8, #0x4                ; =4
100bb84ac:     	stp	xzr, x8, [x29, #-0xb8]
100bb84b0:     	stur	xzr, [x29, #-0xa8]
100bb84b4:     	cbnz	x27, 0x100bb83ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5c4>
100bb84b8:     	mov	x20, #0x0               ; =0
100bb84bc:     	mov	w24, #0x4               ; =4
100bb84c0:     	ldr	x19, [sp, #0x80]
100bb84c4:     	mov	x8, #0x0                ; =0
100bb84c8:     	lsl	x9, x20, #2
100bb84cc:     	str	x24, [sp, #0x30]
100bb84d0:     	cbz	x9, 0x100bb8960 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb78>
100bb84d4:     	ldr	w10, [x24, x8, lsl #2]
100bb84d8:     	sub	x9, x9, #0x4
100bb84dc:     	cmp	x8, x10
100bb84e0:     	add	x8, x8, #0x1
100bb84e4:     	b.eq	0x100bb84d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6e8>
100bb84e8:     	cmn	x19, #0x1
100bb84ec:     	b.eq	0x100bb88e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb00>
100bb84f0:     	ldr	x1, [sp, #0xa0]
100bb84f4:     	b	0x100bb893c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100bb84f8:     	mov	w27, #0x4               ; =4
100bb84fc:     	ldr	x19, [sp, #0x80]
100bb8500:     	ldr	x28, [sp, #0x58]
100bb8504:     	cbnz	x8, 0x100bb848c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6a4>
100bb8508:     	b	0x100bb8494 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100bb850c:     	mov	w24, #0x4               ; =4
100bb8510:     	str	x28, [sp, #0x58]
100bb8514:     	fmov	d0, x20
100bb8518:     	cnt.8b	v0, v0
100bb851c:     	addv.8b	b0, v0
100bb8520:     	fmov	x8, d0
100bb8524:     	mov	w9, #0x1                ; =1
100bb8528:     	lsl	x20, x9, x8
100bb852c:     	ldr	x9, [sp, #0x98]
100bb8530:     	ldr	x8, [x9, #0x40]
100bb8534:     	add	x8, x8, x20
100bb8538:     	str	x8, [x9, #0x40]
100bb853c:     	add	x8, x20, #0x3f
100bb8540:     	lsr	x22, x8, #6
100bb8544:     	lsl	x19, x22, #3
100bb8548:     	mov	x0, x19
100bb854c:     	mov	w1, #0x1                ; =1
100bb8550:     	bl	0x1013bd064 <dyld_stub_binder+0x1013bd064>
100bb8554:     	cbz	x0, 0x100bb8c78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe90>
100bb8558:     	mov	x21, x0
100bb855c:     	mov	x19, #0x0               ; =0
100bb8560:     	ldr	x25, [x25, #0x8]
100bb8564:     	and	x8, x26, #0xfffffffffffffffe
100bb8568:     	neg	x8, x8
100bb856c:     	str	x8, [sp, #0x98]
100bb8570:     	mov	w28, #0x1               ; =1
100bb8574:     	adrp	x8, 0x101451000 <GCC_except_table9425+0x14>
100bb8578:     	ldr	q0, [x8, #0xd50]
100bb857c:     	str	q0, [sp, #0xa0]
100bb8580:     	mov	w8, #0x2                ; =2
100bb8584:     	dup.2d	v0, x8
100bb8588:     	str	q0, [sp, #0x80]
100bb858c:     	mov	w8, #0x4                ; =4
100bb8590:     	dup.2d	v1, x8
100bb8594:     	mov	w8, #0x8                ; =8
100bb8598:     	dup.2d	v0, x8
100bb859c:     	stp	q0, q1, [sp, #0x30]
100bb85a0:     	mov	w8, #0xc                ; =12
100bb85a4:     	dup.2d	v1, x8
100bb85a8:     	mov	w8, #0x10               ; =16
100bb85ac:     	dup.2d	v0, x8
100bb85b0:     	stp	q0, q1, [sp, #0x10]
100bb85b4:     	adrp	x8, 0x101451000 <GCC_except_table9425+0x14>
100bb85b8:     	ldr	q0, [x8, #0xd70]
100bb85bc:     	str	q0, [sp]
100bb85c0:     	mov	w27, #0x3f              ; =63
100bb85c4:     	dup.2d	v0, x27
100bb85c8:     	str	q0, [sp, #0x60]
100bb85cc:     	movi.2s	v8, #0x3f
100bb85d0:     	b	0x100bb85e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7f8>
100bb85d4:     	add	x19, x19, #0x1
100bb85d8:     	cmp	x19, x20
100bb85dc:     	b.eq	0x100bb8888 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaa0>
100bb85e0:     	mov	x2, x25
100bb85e4:     	cbz	x26, 0x100bb8858 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa70>
100bb85e8:     	cmp	x26, #0x1
100bb85ec:     	b.ne	0x100bb85fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x814>
100bb85f0:     	mov	x9, #0x0                ; =0
100bb85f4:     	mov	x8, #0x0                ; =0
100bb85f8:     	b	0x100bb8834 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bb85fc:     	dup.2d	v0, x19
100bb8600:     	cmp	x26, #0x10
100bb8604:     	b.hs	0x100bb8614 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x82c>
100bb8608:     	mov	x10, #0x0               ; =0
100bb860c:     	mov	x8, #0x0                ; =0
100bb8610:     	b	0x100bb87c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9d8>
100bb8614:     	movi.2d	v1, #0000000000000000
100bb8618:     	add	x8, x24, #0x20
100bb861c:     	movi.2d	v2, #0000000000000000
100bb8620:     	and	x9, x26, #0xfffffffffffffff0
100bb8624:     	ldr	q4, [sp, #0xa0]
100bb8628:     	ldp	q6, q15, [sp]
100bb862c:     	movi.2d	v3, #0000000000000000
100bb8630:     	movi.2d	v7, #0000000000000000
100bb8634:     	movi.2d	v16, #0000000000000000
100bb8638:     	movi.2d	v5, #0000000000000000
100bb863c:     	movi.2d	v18, #0000000000000000
100bb8640:     	movi.2d	v17, #0000000000000000
100bb8644:     	ldp	q13, q12, [sp, #0x30]
100bb8648:     	ldr	q14, [sp, #0x20]
100bb864c:     	movi.4s	v8, #0x3f
100bb8650:     	add.2d	v19, v4, v12
100bb8654:     	add.2d	v20, v6, v12
100bb8658:     	add.2d	v21, v4, v13
100bb865c:     	add.2d	v22, v6, v13
100bb8660:     	add.2d	v23, v4, v14
100bb8664:     	add.2d	v24, v6, v14
100bb8668:     	ldp	q25, q26, [x8, #-0x20]
100bb866c:     	dup.2d	v27, x27
100bb8670:     	ldp	q28, q29, [x8], #0x40
100bb8674:     	and.16b	v30, v6, v27
100bb8678:     	and.16b	v31, v4, v27
100bb867c:     	and.16b	v20, v20, v27
100bb8680:     	and.16b	v19, v19, v27
100bb8684:     	and.16b	v22, v22, v27
100bb8688:     	and.16b	v21, v21, v27
100bb868c:     	and.16b	v24, v24, v27
100bb8690:     	and.16b	v23, v23, v27
100bb8694:     	neg.2d	v27, v31
100bb8698:     	ushl.2d	v27, v0, v27
100bb869c:     	neg.2d	v30, v30
100bb86a0:     	ushl.2d	v30, v0, v30
100bb86a4:     	neg.2d	v19, v19
100bb86a8:     	ushl.2d	v19, v0, v19
100bb86ac:     	neg.2d	v20, v20
100bb86b0:     	ushl.2d	v20, v0, v20
100bb86b4:     	neg.2d	v21, v21
100bb86b8:     	ushl.2d	v21, v0, v21
100bb86bc:     	neg.2d	v22, v22
100bb86c0:     	ushl.2d	v22, v0, v22
100bb86c4:     	neg.2d	v23, v23
100bb86c8:     	ushl.2d	v23, v0, v23
100bb86cc:     	neg.2d	v24, v24
100bb86d0:     	ushl.2d	v24, v0, v24
100bb86d4:     	dup.2d	v31, x28
100bb86d8:     	and.16b	v30, v30, v31
100bb86dc:     	and.16b	v27, v27, v31
100bb86e0:     	and.16b	v20, v20, v31
100bb86e4:     	and.16b	v19, v19, v31
100bb86e8:     	and.16b	v22, v22, v31
100bb86ec:     	and.16b	v21, v21, v31
100bb86f0:     	and.16b	v24, v24, v31
100bb86f4:     	and.16b	v23, v23, v31
100bb86f8:     	and.16b	v25, v25, v8
100bb86fc:     	and.16b	v26, v26, v8
100bb8700:     	and.16b	v28, v28, v8
100bb8704:     	and.16b	v29, v29, v8
100bb8708:     	ushll2.2d	v31, v25, #0x0
100bb870c:     	ushll.2d	v25, v25, #0x0
100bb8710:     	ushll2.2d	v9, v26, #0x0
100bb8714:     	ushll.2d	v26, v26, #0x0
100bb8718:     	ushll2.2d	v10, v28, #0x0
100bb871c:     	ushll.2d	v28, v28, #0x0
100bb8720:     	ushll2.2d	v11, v29, #0x0
100bb8724:     	ushll.2d	v29, v29, #0x0
100bb8728:     	ushl.2d	v25, v27, v25
100bb872c:     	ushl.2d	v27, v30, v31
100bb8730:     	ushl.2d	v19, v19, v26
100bb8734:     	ushl.2d	v20, v20, v9
100bb8738:     	ushl.2d	v21, v21, v28
100bb873c:     	ushl.2d	v22, v22, v10
100bb8740:     	ushl.2d	v23, v23, v29
100bb8744:     	ushl.2d	v24, v24, v11
100bb8748:     	orr.16b	v3, v27, v3
100bb874c:     	orr.16b	v2, v25, v2
100bb8750:     	orr.16b	v16, v20, v16
100bb8754:     	orr.16b	v7, v19, v7
100bb8758:     	orr.16b	v18, v22, v18
100bb875c:     	orr.16b	v5, v21, v5
100bb8760:     	orr.16b	v1, v24, v1
100bb8764:     	orr.16b	v17, v23, v17
100bb8768:     	add.2d	v6, v6, v15
100bb876c:     	add.2d	v4, v4, v15
100bb8770:     	subs	x9, x9, #0x10
100bb8774:     	b.ne	0x100bb8650 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x868>
100bb8778:     	orr.16b	v2, v7, v2
100bb877c:     	orr.16b	v3, v16, v3
100bb8780:     	orr.16b	v3, v18, v3
100bb8784:     	orr.16b	v2, v5, v2
100bb8788:     	orr.16b	v2, v17, v2
100bb878c:     	orr.16b	v1, v1, v3
100bb8790:     	orr.16b	v1, v2, v1
100bb8794:     	mov	d2, v1[1]
100bb8798:     	orr.8b	v1, v1, v2
100bb879c:     	fmov	x8, d1
100bb87a0:     	and	x9, x26, #0xfffffffffffffff0
100bb87a4:     	cmp	x26, x9
100bb87a8:     	movi.2s	v8, #0x3f
100bb87ac:     	b.eq	0x100bb8854 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100bb87b0:     	and	x10, x26, #0xfffffffffffffff0
100bb87b4:     	and	x9, x26, #0xfffffffffffffff0
100bb87b8:     	and	x11, x26, #0xe
100bb87bc:     	cbz	x11, 0x100bb8834 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bb87c0:     	fmov	d1, x8
100bb87c4:     	dup.2d	v2, x10
100bb87c8:     	ldr	q3, [sp, #0xa0]
100bb87cc:     	orr.16b	v2, v2, v3
100bb87d0:     	ldr	x8, [sp, #0x98]
100bb87d4:     	add	x8, x8, x10
100bb87d8:     	add	x9, x24, x10, lsl #2
100bb87dc:     	ldr	q6, [sp, #0x80]
100bb87e0:     	ldr	q7, [sp, #0x60]
100bb87e4:     	ldr	d3, [x9], #0x8
100bb87e8:     	and.16b	v4, v2, v7
100bb87ec:     	neg.2d	v4, v4
100bb87f0:     	ushl.2d	v4, v0, v4
100bb87f4:     	dup.2d	v5, x28
100bb87f8:     	and.16b	v4, v4, v5
100bb87fc:     	and.8b	v3, v3, v8
100bb8800:     	ushll.2d	v3, v3, #0x0
100bb8804:     	ushl.2d	v3, v4, v3
100bb8808:     	orr.16b	v1, v3, v1
100bb880c:     	add.2d	v2, v2, v6
100bb8810:     	adds	x8, x8, #0x2
100bb8814:     	b.ne	0x100bb87e4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9fc>
100bb8818:     	mov	d0, v1[1]
100bb881c:     	orr.8b	v0, v1, v0
100bb8820:     	fmov	x8, d0
100bb8824:     	and	x9, x26, #0xfffffffffffffffe
100bb8828:     	and	x10, x26, #0xfffffffffffffffe
100bb882c:     	cmp	x26, x10
100bb8830:     	b.eq	0x100bb8854 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100bb8834:     	ldr	w10, [x24, x9, lsl #2]
100bb8838:     	lsr	x11, x19, x9
100bb883c:     	and	x11, x11, #0x1
100bb8840:     	lsl	x10, x11, x10
100bb8844:     	orr	x8, x10, x8
100bb8848:     	add	x9, x9, #0x1
100bb884c:     	cmp	x26, x9
100bb8850:     	b.ne	0x100bb8834 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bb8854:     	orr	x2, x8, x25
100bb8858:     	mov	x0, x23
100bb885c:     	ldr	x1, [sp, #0x78]
100bb8860:     	bl	0x100c9c6b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100bb8864:     	cbz	w0, 0x100bb85d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100bb8868:     	lsr	x0, x19, #6
100bb886c:     	cmp	x0, x22
100bb8870:     	b.hs	0x100bb8c30 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe48>
100bb8874:     	lsl	x8, x28, x19
100bb8878:     	ldr	x9, [x21, x0, lsl #3]
100bb887c:     	orr	x8, x9, x8
100bb8880:     	str	x8, [x21, x0, lsl #3]
100bb8884:     	b	0x100bb85d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100bb8888:     	ldr	x8, [sp, #0x58]
100bb888c:     	stp	x22, x21, [x8]
100bb8890:     	stp	x22, xzr, [x8, #0x10]
100bb8894:     	cbz	x26, 0x100bb8b00 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bb8898:     	mov	x0, x24
100bb889c:     	b	0x100bb8afc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100bb88a0:     	mov	x8, #0x0                ; =0
100bb88a4:     	ldur	x21, [x29, #-0xb8]
100bb88a8:     	mov	w0, #0x8                ; =8
100bb88ac:     	ldr	x10, [sp, #0x98]
100bb88b0:     	ldr	x9, [x10, #0x48]
100bb88b4:     	add	x9, x9, x20
100bb88b8:     	str	x9, [x10, #0x48]
100bb88bc:     	stp	x8, x0, [x28]
100bb88c0:     	stp	x20, xzr, [x28, #0x10]
100bb88c4:     	cmp	x21, #0x1
100bb88c8:     	b.lt	0x100bb88d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaec>
100bb88cc:     	ldur	x0, [x29, #-0xb0]
100bb88d0:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb88d4:     	ldur	x8, [x29, #-0xe0]
100bb88d8:     	cmp	x8, #0x1
100bb88dc:     	b.lt	0x100bb8b00 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bb88e0:     	ldur	x0, [x29, #-0xd8]
100bb88e4:     	b	0x100bb8afc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100bb88e8:     	ldr	x1, [sp, #0xa0]
100bb88ec:     	cbz	x1, 0x100bb8934 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb4c>
100bb88f0:     	lsl	x27, x1, #3
100bb88f4:     	mov	x0, x27
100bb88f8:     	mov	x19, x1
100bb88fc:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
100bb8900:     	cbz	x0, 0x100bb8cd0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xee8>
100bb8904:     	mov	x26, x0
100bb8908:     	mov	x1, x23
100bb890c:     	mov	x2, x27
100bb8910:     	bl	0x1013bd25c <dyld_stub_binder+0x1013bd25c>
100bb8914:     	cmn	x19, #0x1
100bb8918:     	b.eq	0x100bb8bd8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdf0>
100bb891c:     	mov	x1, x19
100bb8920:     	mov	x23, x26
100bb8924:     	b	0x100bb893c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100bb8928:     	mov	w24, #0x4               ; =4
100bb892c:     	cbnz	x20, 0x100bb82b8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d0>
100bb8930:     	b	0x100bb82c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100bb8934:     	mov	x19, #0x0               ; =0
100bb8938:     	mov	w23, #0x8               ; =8
100bb893c:     	mov	x0, x23
100bb8940:     	mov	x2, x24
100bb8944:     	mov	x3, x20
100bb8948:     	bl	0x100e3fb80 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>
100bb894c:     	str	x19, [sp, #0x80]
100bb8950:     	ldr	x9, [sp, #0x98]
100bb8954:     	ldr	x8, [x9, #0x28]
100bb8958:     	add	x8, x8, #0x1
100bb895c:     	str	x8, [x9, #0x28]
100bb8960:     	mov	w8, #0x4                ; =4
100bb8964:     	stp	xzr, x8, [x29, #-0xb8]
100bb8968:     	stur	xzr, [x29, #-0xa8]
100bb896c:     	ldr	x8, [sp, #0x60]
100bb8970:     	bics	x22, x8, x21
100bb8974:     	b.eq	0x100bb8a80 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc98>
100bb8978:     	mov	x24, x23
100bb897c:     	mov	x19, #0x0               ; =0
100bb8980:     	mov	w8, #0x4                ; =4
100bb8984:     	mov	w9, #0x1                ; =1
100bb8988:     	b	0x100bb89b0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbc8>
100bb898c:     	rbit	x9, x22
100bb8990:     	clz	x9, x9
100bb8994:     	str	w9, [x8, x19]
100bb8998:     	stur	x23, [x29, #-0xa8]
100bb899c:     	sub	x10, x22, #0x1
100bb89a0:     	add	x19, x19, #0x4
100bb89a4:     	add	x9, x23, #0x1
100bb89a8:     	ands	x22, x10, x22
100bb89ac:     	b.eq	0x100bb89d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbec>
100bb89b0:     	mov	x23, x9
100bb89b4:     	sub	x9, x9, #0x1
100bb89b8:     	ldur	x10, [x29, #-0xb8]
100bb89bc:     	cmp	x9, x10
100bb89c0:     	b.ne	0x100bb898c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100bb89c4:     	sub	x0, x29, #0xb8
100bb89c8:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bb89cc:     	ldur	x8, [x29, #-0xb0]
100bb89d0:     	b	0x100bb898c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100bb89d4:     	ldp	x8, x28, [x29, #-0xb8]
100bb89d8:     	str	x8, [sp, #0x20]
100bb89dc:     	str	x28, [sp, #0x10]
100bb89e0:     	cbz	x23, 0x100bb8a88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca0>
100bb89e4:     	mov	w27, #0x1               ; =1
100bb89e8:     	mov	x1, x24
100bb89ec:     	b	0x100bb8a18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc30>
100bb89f0:     	orr	x21, x22, x21
100bb89f4:     	stur	x21, [x29, #-0xe0]
100bb89f8:     	ldr	x9, [sp, #0x98]
100bb89fc:     	ldr	x8, [x9, #0x30]
100bb8a00:     	add	x8, x8, #0x1
100bb8a04:     	str	x8, [x9, #0x30]
100bb8a08:     	str	x26, [sp, #0x80]
100bb8a0c:     	mov	x1, x24
100bb8a10:     	subs	x19, x19, #0x4
100bb8a14:     	b.eq	0x100bb8a8c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca4>
100bb8a18:     	ldr	w8, [x28], #0x4
100bb8a1c:     	lsl	x22, x27, x8
100bb8a20:     	sub	x8, x22, #0x1
100bb8a24:     	and	x8, x8, x21
100bb8a28:     	fmov	d0, x8
100bb8a2c:     	cnt.8b	v0, v0
100bb8a30:     	addv.8b	b0, v0
100bb8a34:     	fmov	w4, s0
100bb8a38:     	fmov	d0, x21
100bb8a3c:     	cnt.8b	v0, v0
100bb8a40:     	addv.8b	b0, v0
100bb8a44:     	fmov	w3, s0
100bb8a48:     	sub	x0, x29, #0xb8
100bb8a4c:     	mov	x23, x1
100bb8a50:     	ldr	x2, [sp, #0xa0]
100bb8a54:     	bl	0x100e40b94 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100bb8a58:     	ldp	x26, x24, [x29, #-0xb8]
100bb8a5c:     	ldur	x8, [x29, #-0xa8]
100bb8a60:     	str	x8, [sp, #0xa0]
100bb8a64:     	ldr	x8, [sp, #0x80]
100bb8a68:     	sub	x8, x8, #0x1
100bb8a6c:     	cmn	x8, #0x3
100bb8a70:     	b.hi	0x100bb89f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100bb8a74:     	mov	x0, x23
100bb8a78:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8a7c:     	b	0x100bb89f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100bb8a80:     	ldr	x19, [sp, #0x80]
100bb8a84:     	b	0x100bb8aa8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcc0>
100bb8a88:     	ldr	x26, [sp, #0x80]
100bb8a8c:     	ldr	x8, [sp, #0x20]
100bb8a90:     	cbz	x8, 0x100bb8a9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcb4>
100bb8a94:     	ldr	x0, [sp, #0x10]
100bb8a98:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8a9c:     	mov	x19, x26
100bb8aa0:     	mov	x23, x24
100bb8aa4:     	ldr	x28, [sp, #0x58]
100bb8aa8:     	ldr	x24, [sp, #0x30]
100bb8aac:     	ldr	x8, [sp, #0x60]
100bb8ab0:     	cmp	x21, x8
100bb8ab4:     	b.ne	0x100bb8b74 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd8c>
100bb8ab8:     	cmn	x19, #0x1
100bb8abc:     	b.ne	0x100bb8ad0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xce8>
100bb8ac0:     	ldr	x9, [sp, #0x98]
100bb8ac4:     	ldr	x8, [x9, #0x18]
100bb8ac8:     	add	x8, x8, #0x1
100bb8acc:     	str	x8, [x9, #0x18]
100bb8ad0:     	ldr	x8, [sp, #0x78]
100bb8ad4:     	sbfx	x8, x8, #0, #1
100bb8ad8:     	stp	x19, x23, [x28]
100bb8adc:     	ldr	x9, [sp, #0xa0]
100bb8ae0:     	stp	x9, x8, [x28, #0x10]
100bb8ae4:     	cbz	x20, 0x100bb8af0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd08>
100bb8ae8:     	mov	x0, x24
100bb8aec:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8af0:     	ldr	w8, [sp, #0x40]
100bb8af4:     	tbnz	w8, #0x0, 0x100bb8b00 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bb8af8:     	mov	x0, x25
100bb8afc:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8b00:     	ldp	x29, x30, [sp, #0x1d0]
100bb8b04:     	ldp	x20, x19, [sp, #0x1c0]
100bb8b08:     	ldp	x22, x21, [sp, #0x1b0]
100bb8b0c:     	ldp	x24, x23, [sp, #0x1a0]
100bb8b10:     	ldp	x26, x25, [sp, #0x190]
100bb8b14:     	ldp	x28, x27, [sp, #0x180]
100bb8b18:     	ldp	d9, d8, [sp, #0x170]
100bb8b1c:     	ldp	d11, d10, [sp, #0x160]
100bb8b20:     	ldp	d13, d12, [sp, #0x150]
100bb8b24:     	ldp	d15, d14, [sp, #0x140]
100bb8b28:     	add	sp, sp, #0x1e0
100bb8b2c:     	ret
100bb8b30:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100bb8b34:     	add	x2, x2, #0x3e0
100bb8b38:     	adrp	x5, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8b3c:     	add	x5, x5, #0xe8
100bb8b40:     	sub	x1, x29, #0xb8
100bb8b44:     	mov	w0, #0x0                ; =0
100bb8b48:     	mov	x3, #0x0                ; =0
100bb8b4c:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bb8b50:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100bb8b54:     	add	x2, x2, #0x3e0
100bb8b58:     	adrp	x5, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8b5c:     	add	x5, x5, #0x88
100bb8b60:     	sub	x1, x29, #0xb8
100bb8b64:     	mov	w0, #0x0                ; =0
100bb8b68:     	mov	x3, #0x0                ; =0
100bb8b6c:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bb8b70:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8b74:     	adrp	x5, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8b78:     	add	x5, x5, #0x70
100bb8b7c:     	sub	x1, x29, #0xe0
100bb8b80:     	add	x2, sp, #0xb8
100bb8b84:     	mov	w0, #0x0                ; =0
100bb8b88:     	mov	x3, #0x0                ; =0
100bb8b8c:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bb8b90:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8b94:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100bb8b98:     	add	x2, x2, #0x3e0
100bb8b9c:     	adrp	x5, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8ba0:     	add	x5, x5, #0xd0
100bb8ba4:     	sub	x1, x29, #0xb8
100bb8ba8:     	mov	w0, #0x0                ; =0
100bb8bac:     	mov	x3, #0x0                ; =0
100bb8bb0:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bb8bb4:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100bb8bb8:     	add	x2, x2, #0x3e0
100bb8bbc:     	adrp	x5, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8bc0:     	add	x5, x5, #0xb8
100bb8bc4:     	sub	x1, x29, #0xc0
100bb8bc8:     	mov	w0, #0x1                ; =1
100bb8bcc:     	mov	x3, #0x0                ; =0
100bb8bd0:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bb8bd4:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8bd8:     	adrp	x0, 0x101518000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x920>
100bb8bdc:     	add	x0, x0, #0xea1
100bb8be0:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100bb8be4:     	add	x2, x2, #0xf70
100bb8be8:     	mov	x23, x26
100bb8bec:     	mov	w1, #0x28               ; =40
100bb8bf0:     	bl	0x1013b4d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100bb8bf4:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8bf8:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100bb8bfc:     	add	x2, x2, #0xd30
100bb8c00:     	mov	x1, x26
100bb8c04:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c08:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100bb8c0c:     	add	x2, x2, #0xd30
100bb8c10:     	mov	x1, x26
100bb8c14:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c18:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c1c:     	adrp	x2, 0x101602000 <dyld_stub_binder+0x101602000>
100bb8c20:     	add	x2, x2, #0x140
100bb8c24:     	mov	x1, x26
100bb8c28:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c2c:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c30:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
100bb8c34:     	add	x2, x2, #0x440
100bb8c38:     	mov	x1, x22
100bb8c3c:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c40:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c44:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
100bb8c48:     	add	x2, x2, #0xe28
100bb8c4c:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c50:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c54:     	mov	x19, x0
100bb8c58:     	mov	x1, x16
100bb8c5c:     	b	0x100bb8c64 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe7c>
100bb8c60:     	mov	x19, x0
100bb8c64:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
100bb8c68:     	add	x2, x2, #0x6a8
100bb8c6c:     	mov	x0, x8
100bb8c70:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c74:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c78:     	mov	w0, #0x8                ; =8
100bb8c7c:     	mov	x1, x19
100bb8c80:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bb8c84:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8c88:     	adrp	x2, 0x101600000 <dyld_stub_binder+0x101600000>
100bb8c8c:     	add	x2, x2, #0xa0
100bb8c90:     	mov	x0, x19
100bb8c94:     	mov	x1, x26
100bb8c98:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb8c9c:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8ca0:     	mov	w0, #0x4                ; =4
100bb8ca4:     	mov	x1, x23
100bb8ca8:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bb8cac:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8cb0:     	mov	w0, #0x8                ; =8
100bb8cb4:     	mov	x1, x19
100bb8cb8:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bb8cbc:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8cc0:     	mov	w0, #0x4                ; =4
100bb8cc4:     	mov	x1, x22
100bb8cc8:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bb8ccc:     	b	0x100bb8ce0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bb8cd0:     	mov	x19, #-0x1              ; =-1
100bb8cd4:     	mov	w0, #0x8                ; =8
100bb8cd8:     	mov	x1, x27
100bb8cdc:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bb8ce0:     	brk	#0x1
100bb8ce4:     	mov	x28, x0
100bb8ce8:     	b	0x100bb8d14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf2c>
100bb8cec:     	mov	x28, x0
100bb8cf0:     	b	0x100bb8e4c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1064>
100bb8cf4:     	mov	x28, x0
100bb8cf8:     	b	0x100bb8e2c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100bb8cfc:     	mov	x28, x0
100bb8d00:     	b	0x100bb8e08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1020>
100bb8d04:     	b	0x100bb8da4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfbc>
100bb8d08:     	mov	x28, x0
100bb8d0c:     	mov	x0, x24
100bb8d10:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8d14:     	cbz	x20, 0x100bb8e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bb8d18:     	mov	x0, x19
100bb8d1c:     	b	0x100bb8e98 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bb8d20:     	b	0x100bb8dfc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1014>
100bb8d24:     	mov	x28, x0
100bb8d28:     	ldur	x8, [x29, #-0xb8]
100bb8d2c:     	cbnz	x8, 0x100bb8d38 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf50>
100bb8d30:     	mov	x23, x24
100bb8d34:     	b	0x100bb8dcc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bb8d38:     	ldur	x0, [x29, #-0xb0]
100bb8d3c:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8d40:     	mov	x23, x24
100bb8d44:     	b	0x100bb8dcc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bb8d48:     	mov	x28, x0
100bb8d4c:     	mov	x0, x19
100bb8d50:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8d54:     	b	0x100bb8e1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1034>
100bb8d58:     	mov	x28, x0
100bb8d5c:     	ldur	x8, [x29, #-0xb8]
100bb8d60:     	cbnz	x8, 0x100bb8d70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf88>
100bb8d64:     	ldr	x23, [sp, #0x30]
100bb8d68:     	ldr	x19, [sp, #0x80]
100bb8d6c:     	b	0x100bb8e78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bb8d70:     	ldur	x24, [x29, #-0xb0]
100bb8d74:     	ldr	x23, [sp, #0x30]
100bb8d78:     	ldr	x19, [sp, #0x80]
100bb8d7c:     	b	0x100bb8e70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bb8d80:     	mov	x28, x0
100bb8d84:     	ldur	x8, [x29, #-0xb8]
100bb8d88:     	cbnz	x8, 0x100bb8d94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfac>
100bb8d8c:     	ldr	x19, [sp, #0x80]
100bb8d90:     	b	0x100bb8e88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bb8d94:     	ldur	x0, [x29, #-0xb0]
100bb8d98:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8d9c:     	ldr	x19, [sp, #0x80]
100bb8da0:     	b	0x100bb8e88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bb8da4:     	mov	x28, x0
100bb8da8:     	ldur	x8, [x29, #-0xb8]
100bb8dac:     	cbz	x8, 0x100bb8e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bb8db0:     	ldur	x0, [x29, #-0xb0]
100bb8db4:     	b	0x100bb8e98 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bb8db8:     	mov	x28, x0
100bb8dbc:     	ldr	x8, [sp, #0x20]
100bb8dc0:     	cbz	x8, 0x100bb8dcc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bb8dc4:     	ldr	x0, [sp, #0x10]
100bb8dc8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8dcc:     	ldr	x19, [sp, #0x80]
100bb8dd0:     	ldr	x24, [sp, #0x30]
100bb8dd4:     	cbnz	x20, 0x100bb8e70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bb8dd8:     	b	0x100bb8e78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bb8ddc:     	mov	x28, x0
100bb8de0:     	ldr	x8, [sp, #0x40]
100bb8de4:     	cbz	x8, 0x100bb8df0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1008>
100bb8de8:     	ldr	x0, [sp, #0x30]
100bb8dec:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8df0:     	mov	x23, x19
100bb8df4:     	mov	x19, x24
100bb8df8:     	b	0x100bb8e88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bb8dfc:     	mov	x28, x0
100bb8e00:     	mov	x0, x21
100bb8e04:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e08:     	cbz	x26, 0x100bb8e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bb8e0c:     	mov	x0, x24
100bb8e10:     	b	0x100bb8e98 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bb8e14:     	mov	x28, x0
100bb8e18:     	ldur	x21, [x29, #-0xb8]
100bb8e1c:     	cmp	x21, #0x1
100bb8e20:     	b.lt	0x100bb8e2c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100bb8e24:     	ldur	x0, [x29, #-0xb0]
100bb8e28:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e2c:     	ldur	x8, [x29, #-0xe0]
100bb8e30:     	cmp	x8, #0x1
100bb8e34:     	b.lt	0x100bb8e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bb8e38:     	ldur	x0, [x29, #-0xd8]
100bb8e3c:     	b	0x100bb8e98 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bb8e40:     	mov	x28, x0
100bb8e44:     	mov	x0, x27
100bb8e48:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e4c:     	ldr	x23, [sp, #0x30]
100bb8e50:     	ldr	x19, [sp, #0x80]
100bb8e54:     	ldr	x8, [sp, #0x20]
100bb8e58:     	cbnz	x8, 0x100bb8e70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bb8e5c:     	b	0x100bb8e78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bb8e60:     	mov	x28, x0
100bb8e64:     	b	0x100bb8e88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bb8e68:     	mov	x28, x0
100bb8e6c:     	cbz	x20, 0x100bb8e78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bb8e70:     	mov	x0, x24
100bb8e74:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e78:     	ldr	w8, [sp, #0x40]
100bb8e7c:     	tbnz	w8, #0x0, 0x100bb8e88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bb8e80:     	mov	x0, x25
100bb8e84:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e88:     	sub	x8, x19, #0x1
100bb8e8c:     	cmn	x8, #0x3
100bb8e90:     	b.hi	0x100bb8e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bb8e94:     	mov	x0, x23
100bb8e98:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100bb8e9c:     	mov	x0, x28
100bb8ea0:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
