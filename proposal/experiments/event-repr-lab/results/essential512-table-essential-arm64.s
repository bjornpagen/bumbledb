
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a1ef04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>:
100a1ef04:     	sub	sp, sp, #0xa0
100a1ef08:     	stp	x28, x27, [sp, #0x40]
100a1ef0c:     	stp	x26, x25, [sp, #0x50]
100a1ef10:     	stp	x24, x23, [sp, #0x60]
100a1ef14:     	stp	x22, x21, [sp, #0x70]
100a1ef18:     	stp	x20, x19, [sp, #0x80]
100a1ef1c:     	stp	x29, x30, [sp, #0x90]
100a1ef20:     	add	x29, sp, #0x90
100a1ef24:     	mov	x19, x2
100a1ef28:     	ldr	w8, [x0, #0x90]
100a1ef2c:     	lsr	x8, x1, x8
100a1ef30:     	cbnz	x8, 0x100a1f450 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x54c>
100a1ef34:     	mov	x28, x1
100a1ef38:     	fmov	d0, x28
100a1ef3c:     	cnt.8b	v0, v0
100a1ef40:     	addv.8b	b0, v0
100a1ef44:     	fmov	x22, d0
100a1ef48:     	cmp	x22, #0x15
100a1ef4c:     	b.hs	0x100a1f46c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x568>
100a1ef50:     	mov	w8, #0x1                ; =1
100a1ef54:     	lsl	x8, x8, x22
100a1ef58:     	ldr	x9, [x19, #0x10]
100a1ef5c:     	str	x9, [sp, #0x8]
100a1ef60:     	lsr	x10, x8, #6
100a1ef64:     	cmp	x22, #0x6
100a1ef68:     	cinc	x10, x10, lo
100a1ef6c:     	str	x10, [sp, #0x20]
100a1ef70:     	cmp	x9, x10
100a1ef74:     	b.ne	0x100a1f488 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x584>
100a1ef78:     	cmp	x22, #0x6
100a1ef7c:     	str	x0, [sp]
100a1ef80:     	b.hs	0x100a1efa4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xa0>
100a1ef84:     	cbz	x9, 0x100a1f4d8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d4>
100a1ef88:     	ldr	x9, [x19, #0x8]
100a1ef8c:     	mov	x10, #-0x1              ; =-1
100a1ef90:     	lsl	x8, x10, x8
100a1ef94:     	ldr	x10, [x9]
100a1ef98:     	bic	x8, x10, x8
100a1ef9c:     	str	x8, [x9]
100a1efa0:     	cbz	x28, 0x100a1f168 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x264>
100a1efa4:     	mov	w23, #0x0               ; =0
100a1efa8:     	ldrb	w27, [x0, #0x94]
100a1efac:     	mov	w20, #0x1               ; =1
100a1efb0:     	b	0x100a1efc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100a1efb4:     	add	w23, w23, #0x1
100a1efb8:     	cmp	w23, w22
100a1efbc:     	b.hs	0x100a1f168 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x264>
100a1efc0:     	tbz	w27, #0x0, 0x100a1efe0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xdc>
100a1efc4:     	ldp	x24, x25, [x19, #0x8]
100a1efc8:     	mov	x0, x24
100a1efcc:     	mov	x1, x25
100a1efd0:     	mov	x2, x23
100a1efd4:     	bl	0x100ba1910 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>
100a1efd8:     	tbz	w0, #0x0, 0x100a1efb4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100a1efdc:     	b	0x100a1f078 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x174>
100a1efe0:     	ldp	x24, x25, [x19, #0x8]
100a1efe4:     	add	x0, sp, #0x8
100a1efe8:     	mov	x1, x24
100a1efec:     	mov	x2, x25
100a1eff0:     	mov	x3, x22
100a1eff4:     	mov	x4, x23
100a1eff8:     	mov	w5, #0x0                ; =0
100a1effc:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a1f000:     	add	x0, sp, #0x20
100a1f004:     	mov	x1, x24
100a1f008:     	mov	x2, x25
100a1f00c:     	mov	x3, x22
100a1f010:     	mov	x4, x23
100a1f014:     	mov	w5, #0x1                ; =1
100a1f018:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a1f01c:     	ldr	x8, [sp, #0x18]
100a1f020:     	ldr	x9, [sp, #0x30]
100a1f024:     	cmp	x8, x9
100a1f028:     	b.ne	0x100a1f050 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x14c>
100a1f02c:     	ldr	x1, [sp, #0x28]
100a1f030:     	ldr	x0, [sp, #0x10]
100a1f034:     	lsl	x2, x8, #3
100a1f038:     	bl	0x101109b90 <dyld_stub_binder+0x101109b90>
100a1f03c:     	cmp	w0, #0x0
100a1f040:     	cset	w21, eq
100a1f044:     	ldr	x8, [sp, #0x20]
100a1f048:     	cbnz	x8, 0x100a1f05c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x158>
100a1f04c:     	b	0x100a1f064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100a1f050:     	mov	w21, #0x0               ; =0
100a1f054:     	ldr	x8, [sp, #0x20]
100a1f058:     	cbz	x8, 0x100a1f064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100a1f05c:     	ldr	x0, [sp, #0x28]
100a1f060:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f064:     	ldr	x8, [sp, #0x8]
100a1f068:     	cbz	x8, 0x100a1f074 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x170>
100a1f06c:     	ldr	x0, [sp, #0x10]
100a1f070:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f074:     	tbz	w21, #0x0, 0x100a1efb4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100a1f078:     	mov	w8, #0x4                ; =4
100a1f07c:     	stp	xzr, x8, [sp, #0x20]
100a1f080:     	str	xzr, [sp, #0x30]
100a1f084:     	mov	x26, #0x0               ; =0
100a1f088:     	cbz	x28, 0x100a1f4a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a1f08c:     	mov	w8, #0x4                ; =4
100a1f090:     	mov	x21, x28
100a1f094:     	b	0x100a1f0b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1b4>
100a1f098:     	rbit	x9, x21
100a1f09c:     	clz	x9, x9
100a1f0a0:     	str	w9, [x8, x26, lsl #2]
100a1f0a4:     	add	x26, x26, #0x1
100a1f0a8:     	str	x26, [sp, #0x30]
100a1f0ac:     	sub	x9, x21, #0x1
100a1f0b0:     	ands	x21, x9, x21
100a1f0b4:     	b.eq	0x100a1f0d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1d0>
100a1f0b8:     	ldr	x9, [sp, #0x20]
100a1f0bc:     	cmp	x26, x9
100a1f0c0:     	b.ne	0x100a1f098 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100a1f0c4:     	add	x0, sp, #0x20
100a1f0c8:     	bl	0x1011024dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100a1f0cc:     	ldr	x8, [sp, #0x28]
100a1f0d0:     	b	0x100a1f098 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100a1f0d4:     	ldp	x9, x8, [sp, #0x20]
100a1f0d8:     	mov	w0, w23
100a1f0dc:     	cmp	x26, x0
100a1f0e0:     	b.ls	0x100a1f4b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5b4>
100a1f0e4:     	ldr	w26, [x8, x0, lsl #2]
100a1f0e8:     	cbz	x9, 0x100a1f0f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1f0>
100a1f0ec:     	mov	x0, x8
100a1f0f0:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f0f4:     	tbz	w27, #0x0, 0x100a1f118 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x214>
100a1f0f8:     	add	x0, sp, #0x20
100a1f0fc:     	mov	x1, x24
100a1f100:     	mov	x2, x25
100a1f104:     	mov	x3, x22
100a1f108:     	mov	x4, x23
100a1f10c:     	mov	w5, #0x0                ; =0
100a1f110:     	bl	0x100ba2798 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a1f114:     	b	0x100a1f134 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x230>
100a1f118:     	add	x0, sp, #0x20
100a1f11c:     	mov	x1, x24
100a1f120:     	mov	x2, x25
100a1f124:     	mov	x3, x22
100a1f128:     	mov	x4, x23
100a1f12c:     	mov	w5, #0x0                ; =0
100a1f130:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a1f134:     	ldr	x8, [x19]
100a1f138:     	cbz	x8, 0x100a1f144 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x240>
100a1f13c:     	mov	x0, x24
100a1f140:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f144:     	lsl	x8, x20, x26
100a1f148:     	bic	x28, x28, x8
100a1f14c:     	ldr	q0, [sp, #0x20]
100a1f150:     	str	q0, [x19]
100a1f154:     	ldr	x8, [sp, #0x30]
100a1f158:     	str	x8, [x19, #0x10]
100a1f15c:     	sub	w22, w22, #0x1
100a1f160:     	cmp	w23, w22
100a1f164:     	b.lo	0x100a1efc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100a1f168:     	cbz	w22, 0x100a1f258 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x354>
100a1f16c:     	cmp	w22, #0xa
100a1f170:     	b.hs	0x100a1f278 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x374>
100a1f174:     	mov	w8, #0x1                ; =1
100a1f178:     	lsl	x8, x8, x22
100a1f17c:     	ldr	x1, [x19, #0x10]
100a1f180:     	sub	x10, x8, #0x1
100a1f184:     	lsr	x0, x10, #6
100a1f188:     	cmp	x0, x1
100a1f18c:     	b.hs	0x100a1f500 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5fc>
100a1f190:     	ldr	x9, [x19, #0x8]
100a1f194:     	ldr	x11, [x9, x0, lsl #3]
100a1f198:     	lsr	x10, x11, x10
100a1f19c:     	and	w20, w10, #0x1
100a1f1a0:     	tbz	w10, #0x0, 0x100a1f234 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x330>
100a1f1a4:     	lsl	x11, x1, #3
100a1f1a8:     	sub	x12, x11, #0x8
100a1f1ac:     	mov	x10, x9
100a1f1b0:     	cmp	x12, #0x38
100a1f1b4:     	b.lo	0x100a1f200 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2fc>
100a1f1b8:     	lsr	x10, x12, #3
100a1f1bc:     	add	x12, x10, #0x1
100a1f1c0:     	and	x13, x12, #0x3ffffffffffffff8
100a1f1c4:     	add	x10, x9, x13, lsl #3
100a1f1c8:     	add	x14, x9, #0x20
100a1f1cc:     	and	x15, x12, #0x3ffffffffffffff8
100a1f1d0:     	ldp	q0, q1, [x14, #-0x20]
100a1f1d4:     	ldp	q2, q3, [x14]
100a1f1d8:     	mvn.16b	v0, v0
100a1f1dc:     	mvn.16b	v1, v1
100a1f1e0:     	mvn.16b	v2, v2
100a1f1e4:     	mvn.16b	v3, v3
100a1f1e8:     	stp	q0, q1, [x14, #-0x20]
100a1f1ec:     	stp	q2, q3, [x14], #0x40
100a1f1f0:     	subs	x15, x15, #0x8
100a1f1f4:     	b.ne	0x100a1f1d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2cc>
100a1f1f8:     	cmp	x12, x13
100a1f1fc:     	b.eq	0x100a1f218 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x314>
100a1f200:     	add	x11, x9, x11
100a1f204:     	ldr	x12, [x10]
100a1f208:     	mvn	x12, x12
100a1f20c:     	str	x12, [x10], #0x8
100a1f210:     	cmp	x10, x11
100a1f214:     	b.ne	0x100a1f204 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x300>
100a1f218:     	cmp	w22, #0x5
100a1f21c:     	b.hi	0x100a1f234 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x330>
100a1f220:     	mov	x10, #-0x1              ; =-1
100a1f224:     	lsl	x8, x10, x8
100a1f228:     	ldr	x10, [x9]
100a1f22c:     	bic	x8, x10, x8
100a1f230:     	str	x8, [x9]
100a1f234:     	ldr	q0, [x19]
100a1f238:     	str	q0, [sp, #0x20]
100a1f23c:     	ldr	x8, [x19, #0x10]
100a1f240:     	stp	x8, x28, [sp, #0x30]
100a1f244:     	add	x1, sp, #0x20
100a1f248:     	ldr	x0, [sp]
100a1f24c:     	bl	0x100a19358 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E6internB6_>
100a1f250:     	eor	w0, w0, w20
100a1f254:     	b	0x100a1f430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x52c>
100a1f258:     	ldr	x8, [x19, #0x10]
100a1f25c:     	cbz	x8, 0x100a1f4ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5e8>
100a1f260:     	ldp	x9, x8, [x19]
100a1f264:     	ldr	w0, [x8]
100a1f268:     	cbz	x9, 0x100a1f430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x52c>
100a1f26c:     	mov	x19, x0
100a1f270:     	mov	x0, x8
100a1f274:     	b	0x100a1f428 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x524>
100a1f278:     	ldr	x20, [sp]
100a1f27c:     	ldp	x0, x1, [x20, #0x8]
100a1f280:     	mov	x2, x28
100a1f284:     	bl	0x100a17d00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a1f288:     	mov	w8, #0x4                ; =4
100a1f28c:     	stp	xzr, x8, [sp, #0x20]
100a1f290:     	str	xzr, [sp, #0x30]
100a1f294:     	cbz	x28, 0x100a1f510 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x60c>
100a1f298:     	mov	x23, x0
100a1f29c:     	mov	x20, #0x0               ; =0
100a1f2a0:     	mov	w8, #0x4                ; =4
100a1f2a4:     	mov	w9, #0x1                ; =1
100a1f2a8:     	mov	x24, x28
100a1f2ac:     	b	0x100a1f2d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3d0>
100a1f2b0:     	rbit	x9, x24
100a1f2b4:     	clz	x9, x9
100a1f2b8:     	str	w9, [x8, x20]
100a1f2bc:     	str	x25, [sp, #0x30]
100a1f2c0:     	sub	x10, x24, #0x1
100a1f2c4:     	add	x20, x20, #0x4
100a1f2c8:     	add	x9, x25, #0x1
100a1f2cc:     	ands	x24, x10, x24
100a1f2d0:     	b.eq	0x100a1f2f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3f4>
100a1f2d4:     	mov	x25, x9
100a1f2d8:     	sub	x9, x9, #0x1
100a1f2dc:     	ldr	x10, [sp, #0x20]
100a1f2e0:     	cmp	x9, x10
100a1f2e4:     	b.ne	0x100a1f2b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3ac>
100a1f2e8:     	add	x0, sp, #0x20
100a1f2ec:     	bl	0x1011024dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100a1f2f0:     	ldr	x8, [sp, #0x28]
100a1f2f4:     	b	0x100a1f2b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3ac>
100a1f2f8:     	ldp	x24, x0, [sp, #0x20]
100a1f2fc:     	cbz	x25, 0x100a1f31c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x418>
100a1f300:     	mov	x25, #0x0               ; =0
100a1f304:     	ldr	w8, [x0, x25, lsl #2]
100a1f308:     	cmp	w8, w23
100a1f30c:     	b.eq	0x100a1f330 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x42c>
100a1f310:     	add	x25, x25, #0x1
100a1f314:     	subs	x20, x20, #0x4
100a1f318:     	b.ne	0x100a1f304 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x400>
100a1f31c:     	mov	x20, x0
100a1f320:     	adrp	x0, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f324:     	add	x0, x0, #0xd98
100a1f328:     	bl	0x101101774 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100a1f32c:     	b	0x100a1f50c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x608>
100a1f330:     	cbz	x24, 0x100a1f338 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x434>
100a1f334:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f338:     	ldp	x24, x26, [x19, #0x8]
100a1f33c:     	ldr	x8, [sp]
100a1f340:     	ldrb	w8, [x8, #0x94]
100a1f344:     	tbz	w8, #0x0, 0x100a1f36c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x468>
100a1f348:     	add	x0, sp, #0x20
100a1f34c:     	mov	x1, x24
100a1f350:     	mov	x2, x26
100a1f354:     	mov	x3, x22
100a1f358:     	mov	x4, x25
100a1f35c:     	mov	w5, #0x0                ; =0
100a1f360:     	bl	0x100ba2798 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a1f364:     	ldr	x21, [sp]
100a1f368:     	b	0x100a1f38c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x488>
100a1f36c:     	add	x0, sp, #0x20
100a1f370:     	mov	x1, x24
100a1f374:     	mov	x2, x26
100a1f378:     	mov	x3, x22
100a1f37c:     	mov	x4, x25
100a1f380:     	mov	w5, #0x0                ; =0
100a1f384:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a1f388:     	ldr	x21, [sp]
100a1f38c:     	mov	w8, #0x1                ; =1
100a1f390:     	lsl	x20, x8, x23
100a1f394:     	bic	x1, x28, x20
100a1f398:     	add	x2, sp, #0x20
100a1f39c:     	mov	x0, x21
100a1f3a0:     	bl	0x100a1ef04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a1f3a4:     	mov	x27, x0
100a1f3a8:     	ldrb	w8, [x21, #0x94]
100a1f3ac:     	tbz	w8, #0x0, 0x100a1f3d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4d0>
100a1f3b0:     	add	x0, sp, #0x20
100a1f3b4:     	mov	x1, x24
100a1f3b8:     	mov	x2, x26
100a1f3bc:     	mov	x3, x22
100a1f3c0:     	mov	x4, x25
100a1f3c4:     	mov	w5, #0x1                ; =1
100a1f3c8:     	bl	0x100ba2798 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a1f3cc:     	ldr	x21, [sp]
100a1f3d0:     	b	0x100a1f3f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4f0>
100a1f3d4:     	add	x0, sp, #0x20
100a1f3d8:     	mov	x1, x24
100a1f3dc:     	mov	x2, x26
100a1f3e0:     	mov	x3, x22
100a1f3e4:     	mov	x4, x25
100a1f3e8:     	mov	w5, #0x1                ; =1
100a1f3ec:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a1f3f0:     	ldr	x21, [sp]
100a1f3f4:     	bic	x1, x28, x20
100a1f3f8:     	add	x2, sp, #0x20
100a1f3fc:     	mov	x0, x21
100a1f400:     	bl	0x100a1ef04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a1f404:     	mov	x3, x0
100a1f408:     	mov	x0, x21
100a1f40c:     	mov	x1, x23
100a1f410:     	mov	x2, x27
100a1f414:     	bl	0x100a1f5ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100a1f418:     	ldr	x8, [x19]
100a1f41c:     	cbz	x8, 0x100a1f430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x52c>
100a1f420:     	mov	x19, x0
100a1f424:     	mov	x0, x24
100a1f428:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f42c:     	mov	x0, x19
100a1f430:     	ldp	x29, x30, [sp, #0x90]
100a1f434:     	ldp	x20, x19, [sp, #0x80]
100a1f438:     	ldp	x22, x21, [sp, #0x70]
100a1f43c:     	ldp	x24, x23, [sp, #0x60]
100a1f440:     	ldp	x26, x25, [sp, #0x50]
100a1f444:     	ldp	x28, x27, [sp, #0x40]
100a1f448:     	add	sp, sp, #0xa0
100a1f44c:     	ret
100a1f450:     	adrp	x0, 0x1011bd000 <dyld_stub_binder+0x1011bd000>
100a1f454:     	add	x0, x0, #0xe47
100a1f458:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f45c:     	add	x2, x2, #0xdc8
100a1f460:     	mov	w1, #0x33               ; =51
100a1f464:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a1f468:     	b	0x100a1f50c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x608>
100a1f46c:     	adrp	x0, 0x1011bd000 <dyld_stub_binder+0x1011bd000>
100a1f470:     	add	x0, x0, #0xe25
100a1f474:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f478:     	add	x2, x2, #0xd38
100a1f47c:     	mov	w1, #0x45               ; =69
100a1f480:     	bl	0x101101574 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100a1f484:     	b	0x100a1f50c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x608>
100a1f488:     	adrp	x5, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f48c:     	add	x5, x5, #0xd50
100a1f490:     	add	x1, sp, #0x8
100a1f494:     	add	x2, sp, #0x20
100a1f498:     	mov	w0, #0x0                ; =0
100a1f49c:     	mov	x3, #0x0                ; =0
100a1f4a0:     	bl	0x1011015b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100a1f4a4:     	b	0x100a1f50c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x608>
100a1f4a8:     	mov	w22, #0x1               ; =1
100a1f4ac:     	mov	w20, #0x4               ; =4
100a1f4b0:     	mov	w0, w23
100a1f4b4:     	b	0x100a1f4c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5c0>
100a1f4b8:     	mov	x20, x8
100a1f4bc:     	cmp	x9, #0x0
100a1f4c0:     	cset	w22, eq
100a1f4c4:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f4c8:     	add	x2, x2, #0xdb0
100a1f4cc:     	mov	x1, x26
100a1f4d0:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1f4d4:     	b	0x100a1f50c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x608>
100a1f4d8:     	mov	x0, #0x0                ; =0
100a1f4dc:     	mov	x1, #0x0                ; =0
100a1f4e0:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f4e4:     	add	x2, x2, #0xd68
100a1f4e8:     	b	0x100a1f508 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x604>
100a1f4ec:     	mov	x0, #0x0                ; =0
100a1f4f0:     	mov	x1, #0x0                ; =0
100a1f4f4:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1f4f8:     	add	x2, x2, #0xd80
100a1f4fc:     	b	0x100a1f508 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x604>
100a1f500:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a1f504:     	add	x2, x2, #0xad0
100a1f508:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1f50c:     	brk	#0x1
100a1f510:     	mov	x24, #0x0               ; =0
100a1f514:     	mov	w20, #0x4               ; =4
100a1f518:     	b	0x100a1f320 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x41c>
100a1f51c:     	mov	x20, x0
100a1f520:     	ldr	x8, [sp, #0x20]
100a1f524:     	cbz	x8, 0x100a1f55c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x658>
100a1f528:     	ldr	x0, [sp, #0x28]
100a1f52c:     	b	0x100a1f5b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6b4>
100a1f530:     	mov	x21, x0
100a1f534:     	cbz	x24, 0x100a1f57c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x678>
100a1f538:     	mov	x0, x20
100a1f53c:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f540:     	mov	x0, x21
100a1f544:     	ldr	x8, [x19]
100a1f548:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f54c:     	b	0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f550:     	mov	x20, x0
100a1f554:     	ldr	x8, [sp, #0x8]
100a1f558:     	cbnz	x8, 0x100a1f56c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x668>
100a1f55c:     	mov	x0, x20
100a1f560:     	ldr	x8, [x19]
100a1f564:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f568:     	b	0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f56c:     	ldr	x0, [sp, #0x10]
100a1f570:     	b	0x100a1f5b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6b4>
100a1f574:     	mov	x21, x0
100a1f578:     	tbz	w22, #0x0, 0x100a1f538 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x634>
100a1f57c:     	mov	x0, x21
100a1f580:     	ldr	x8, [x19]
100a1f584:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f588:     	b	0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f58c:     	ldr	x8, [x19]
100a1f590:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f594:     	b	0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f598:     	ldr	x8, [sp, #0x20]
100a1f59c:     	cbnz	x8, 0x100a1f5ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a8>
100a1f5a0:     	ldr	x8, [x19]
100a1f5a4:     	cbnz	x8, 0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f5a8:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
100a1f5ac:     	ldr	x8, [sp, #0x28]
100a1f5b0:     	mov	x20, x0
100a1f5b4:     	mov	x0, x8
100a1f5b8:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f5bc:     	mov	x0, x20
100a1f5c0:     	ldr	x8, [x19]
100a1f5c4:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f5c8:     	b	0x100a1f5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6d0>
100a1f5cc:     	ldr	x8, [x19]
100a1f5d0:     	cbz	x8, 0x100a1f5a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x6a4>
100a1f5d4:     	ldr	x8, [x19, #0x8]
100a1f5d8:     	mov	x19, x0
100a1f5dc:     	mov	x0, x8
100a1f5e0:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1f5e4:     	mov	x0, x19
100a1f5e8:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
