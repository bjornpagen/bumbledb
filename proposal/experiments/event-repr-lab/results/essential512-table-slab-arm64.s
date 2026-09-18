
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b99064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>:
100b99064:     	sub	sp, sp, #0xa0
100b99068:     	stp	x28, x27, [sp, #0x40]
100b9906c:     	stp	x26, x25, [sp, #0x50]
100b99070:     	stp	x24, x23, [sp, #0x60]
100b99074:     	stp	x22, x21, [sp, #0x70]
100b99078:     	stp	x20, x19, [sp, #0x80]
100b9907c:     	stp	x29, x30, [sp, #0x90]
100b99080:     	add	x29, sp, #0x90
100b99084:     	mov	x19, x2
100b99088:     	ldr	w8, [x0, #0xe0]
100b9908c:     	lsr	x8, x1, x8
100b99090:     	cbnz	x8, 0x100b994d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x46c>
100b99094:     	mov	x28, x1
100b99098:     	fmov	d0, x28
100b9909c:     	cnt.8b	v0, v0
100b990a0:     	addv.8b	b0, v0
100b990a4:     	fmov	x22, d0
100b990a8:     	cmp	x22, #0x15
100b990ac:     	b.hs	0x100b994ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x488>
100b990b0:     	mov	w8, #0x1                ; =1
100b990b4:     	lsl	x8, x8, x22
100b990b8:     	ldr	x9, [x19, #0x10]
100b990bc:     	str	x9, [sp, #0x10]
100b990c0:     	lsr	x10, x8, #6
100b990c4:     	cmp	x22, #0x6
100b990c8:     	cinc	x10, x10, lo
100b990cc:     	str	x10, [sp, #0x28]
100b990d0:     	cmp	x9, x10
100b990d4:     	b.ne	0x100b99508 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4a4>
100b990d8:     	cmp	x22, #0x6
100b990dc:     	b.hs	0x100b99100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x9c>
100b990e0:     	cbz	x9, 0x100b99558 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4f4>
100b990e4:     	ldr	x9, [x19, #0x8]
100b990e8:     	mov	x10, #-0x1              ; =-1
100b990ec:     	lsl	x8, x10, x8
100b990f0:     	ldr	x10, [x9]
100b990f4:     	bic	x8, x10, x8
100b990f8:     	str	x8, [x9]
100b990fc:     	cbz	x28, 0x100b992d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x270>
100b99100:     	mov	w23, #0x0               ; =0
100b99104:     	str	x0, [sp, #0x8]
100b99108:     	ldrb	w27, [x0, #0xe4]
100b9910c:     	mov	w20, #0x1               ; =1
100b99110:     	b	0x100b99120 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100b99114:     	add	w23, w23, #0x1
100b99118:     	cmp	w23, w22
100b9911c:     	b.hs	0x100b992c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x264>
100b99120:     	tbz	w27, #0x0, 0x100b99140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xdc>
100b99124:     	ldp	x24, x25, [x19, #0x8]
100b99128:     	mov	x0, x24
100b9912c:     	mov	x1, x25
100b99130:     	mov	x2, x23
100b99134:     	bl	0x100d15210 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>
100b99138:     	tbz	w0, #0x0, 0x100b99114 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100b9913c:     	b	0x100b991d8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x174>
100b99140:     	ldp	x24, x25, [x19, #0x8]
100b99144:     	add	x0, sp, #0x10
100b99148:     	mov	x1, x24
100b9914c:     	mov	x2, x25
100b99150:     	mov	x3, x22
100b99154:     	mov	x4, x23
100b99158:     	mov	w5, #0x0                ; =0
100b9915c:     	bl	0x100cc55a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100b99160:     	add	x0, sp, #0x28
100b99164:     	mov	x1, x24
100b99168:     	mov	x2, x25
100b9916c:     	mov	x3, x22
100b99170:     	mov	x4, x23
100b99174:     	mov	w5, #0x1                ; =1
100b99178:     	bl	0x100cc55a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100b9917c:     	ldr	x8, [sp, #0x20]
100b99180:     	ldr	x9, [sp, #0x38]
100b99184:     	cmp	x8, x9
100b99188:     	b.ne	0x100b991b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x14c>
100b9918c:     	ldr	x1, [sp, #0x30]
100b99190:     	ldr	x0, [sp, #0x18]
100b99194:     	lsl	x2, x8, #3
100b99198:     	bl	0x101284d90 <dyld_stub_binder+0x101284d90>
100b9919c:     	cmp	w0, #0x0
100b991a0:     	cset	w21, eq
100b991a4:     	ldr	x8, [sp, #0x28]
100b991a8:     	cbnz	x8, 0x100b991bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x158>
100b991ac:     	b	0x100b991c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100b991b0:     	mov	w21, #0x0               ; =0
100b991b4:     	ldr	x8, [sp, #0x28]
100b991b8:     	cbz	x8, 0x100b991c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100b991bc:     	ldr	x0, [sp, #0x30]
100b991c0:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b991c4:     	ldr	x8, [sp, #0x10]
100b991c8:     	cbz	x8, 0x100b991d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x170>
100b991cc:     	ldr	x0, [sp, #0x18]
100b991d0:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b991d4:     	tbz	w21, #0x0, 0x100b99114 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100b991d8:     	mov	w8, #0x4                ; =4
100b991dc:     	stp	xzr, x8, [sp, #0x28]
100b991e0:     	str	xzr, [sp, #0x38]
100b991e4:     	mov	x26, #0x0               ; =0
100b991e8:     	cbz	x28, 0x100b99528 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4c4>
100b991ec:     	mov	w8, #0x4                ; =4
100b991f0:     	mov	x21, x28
100b991f4:     	b	0x100b99218 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1b4>
100b991f8:     	rbit	x9, x21
100b991fc:     	clz	x9, x9
100b99200:     	str	w9, [x8, x26, lsl #2]
100b99204:     	add	x26, x26, #0x1
100b99208:     	str	x26, [sp, #0x38]
100b9920c:     	sub	x9, x21, #0x1
100b99210:     	ands	x21, x9, x21
100b99214:     	b.eq	0x100b99234 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1d0>
100b99218:     	ldr	x9, [sp, #0x28]
100b9921c:     	cmp	x26, x9
100b99220:     	b.ne	0x100b991f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100b99224:     	add	x0, sp, #0x28
100b99228:     	bl	0x10127d69c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b9922c:     	ldr	x8, [sp, #0x30]
100b99230:     	b	0x100b991f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100b99234:     	ldp	x9, x8, [sp, #0x28]
100b99238:     	mov	w0, w23
100b9923c:     	cmp	x26, x0
100b99240:     	b.ls	0x100b99538 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4d4>
100b99244:     	ldr	w26, [x8, x0, lsl #2]
100b99248:     	cbz	x9, 0x100b99254 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1f0>
100b9924c:     	mov	x0, x8
100b99250:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b99254:     	tbz	w27, #0x0, 0x100b99278 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x214>
100b99258:     	add	x0, sp, #0x28
100b9925c:     	mov	x1, x24
100b99260:     	mov	x2, x25
100b99264:     	mov	x3, x22
100b99268:     	mov	x4, x23
100b9926c:     	mov	w5, #0x0                ; =0
100b99270:     	bl	0x100d16098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b99274:     	b	0x100b99294 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x230>
100b99278:     	add	x0, sp, #0x28
100b9927c:     	mov	x1, x24
100b99280:     	mov	x2, x25
100b99284:     	mov	x3, x22
100b99288:     	mov	x4, x23
100b9928c:     	mov	w5, #0x0                ; =0
100b99290:     	bl	0x100cc55a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100b99294:     	ldr	x8, [x19]
100b99298:     	cbz	x8, 0x100b992a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x240>
100b9929c:     	mov	x0, x24
100b992a0:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b992a4:     	lsl	x8, x20, x26
100b992a8:     	bic	x28, x28, x8
100b992ac:     	ldur	q0, [sp, #0x28]
100b992b0:     	str	q0, [x19]
100b992b4:     	ldr	x8, [sp, #0x38]
100b992b8:     	str	x8, [x19, #0x10]
100b992bc:     	sub	w22, w22, #0x1
100b992c0:     	cmp	w23, w22
100b992c4:     	b.lo	0x100b99120 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100b992c8:     	cmp	w22, #0xa
100b992cc:     	b.hs	0x100b992fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x298>
100b992d0:     	ldr	x0, [sp, #0x8]
100b992d4:     	mov	x1, x28
100b992d8:     	mov	x2, x19
100b992dc:     	ldp	x29, x30, [sp, #0x90]
100b992e0:     	ldp	x20, x19, [sp, #0x80]
100b992e4:     	ldp	x22, x21, [sp, #0x70]
100b992e8:     	ldp	x24, x23, [sp, #0x60]
100b992ec:     	ldp	x26, x25, [sp, #0x50]
100b992f0:     	ldp	x28, x27, [sp, #0x40]
100b992f4:     	add	sp, sp, #0xa0
100b992f8:     	b	0x100b97a74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E11exact_tableB6_>
100b992fc:     	ldr	x0, [sp, #0x8]
100b99300:     	mov	x1, x28
100b99304:     	bl	0x100b8d6c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100b99308:     	mov	w8, #0x4                ; =4
100b9930c:     	stp	xzr, x8, [sp, #0x28]
100b99310:     	str	xzr, [sp, #0x38]
100b99314:     	cbz	x28, 0x100b99570 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x50c>
100b99318:     	mov	x23, x0
100b9931c:     	mov	x20, #0x0               ; =0
100b99320:     	mov	w8, #0x4                ; =4
100b99324:     	mov	w9, #0x1                ; =1
100b99328:     	mov	x24, x28
100b9932c:     	b	0x100b99354 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2f0>
100b99330:     	rbit	x9, x24
100b99334:     	clz	x9, x9
100b99338:     	str	w9, [x8, x20]
100b9933c:     	str	x25, [sp, #0x38]
100b99340:     	sub	x10, x24, #0x1
100b99344:     	add	x20, x20, #0x4
100b99348:     	add	x9, x25, #0x1
100b9934c:     	ands	x24, x10, x24
100b99350:     	b.eq	0x100b99378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x314>
100b99354:     	mov	x25, x9
100b99358:     	sub	x9, x9, #0x1
100b9935c:     	ldr	x10, [sp, #0x28]
100b99360:     	cmp	x9, x10
100b99364:     	b.ne	0x100b99330 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2cc>
100b99368:     	add	x0, sp, #0x28
100b9936c:     	bl	0x10127d69c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b99370:     	ldr	x8, [sp, #0x30]
100b99374:     	b	0x100b99330 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2cc>
100b99378:     	ldp	x24, x0, [sp, #0x28]
100b9937c:     	cbz	x25, 0x100b9939c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x338>
100b99380:     	mov	x25, #0x0               ; =0
100b99384:     	ldr	w8, [x0, x25, lsl #2]
100b99388:     	cmp	w8, w23
100b9938c:     	b.eq	0x100b993b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x34c>
100b99390:     	add	x25, x25, #0x1
100b99394:     	subs	x20, x20, #0x4
100b99398:     	b.ne	0x100b99384 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x320>
100b9939c:     	mov	x20, x0
100b993a0:     	adrp	x0, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b993a4:     	add	x0, x0, #0x768
100b993a8:     	bl	0x10127c934 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b993ac:     	b	0x100b9956c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100b993b0:     	cbz	x24, 0x100b993b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x354>
100b993b4:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b993b8:     	ldp	x24, x26, [x19, #0x8]
100b993bc:     	ldr	x8, [sp, #0x8]
100b993c0:     	ldrb	w8, [x8, #0xe4]
100b993c4:     	tbz	w8, #0x0, 0x100b993ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x388>
100b993c8:     	add	x0, sp, #0x28
100b993cc:     	mov	x1, x24
100b993d0:     	mov	x2, x26
100b993d4:     	mov	x3, x22
100b993d8:     	mov	x4, x25
100b993dc:     	mov	w5, #0x0                ; =0
100b993e0:     	bl	0x100d16098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b993e4:     	ldr	x21, [sp, #0x8]
100b993e8:     	b	0x100b9940c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3a8>
100b993ec:     	add	x0, sp, #0x28
100b993f0:     	mov	x1, x24
100b993f4:     	mov	x2, x26
100b993f8:     	mov	x3, x22
100b993fc:     	mov	x4, x25
100b99400:     	mov	w5, #0x0                ; =0
100b99404:     	bl	0x100cc55a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100b99408:     	ldr	x21, [sp, #0x8]
100b9940c:     	mov	w8, #0x1                ; =1
100b99410:     	lsl	x20, x8, x23
100b99414:     	bic	x1, x28, x20
100b99418:     	add	x2, sp, #0x28
100b9941c:     	mov	x0, x21
100b99420:     	bl	0x100b99064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100b99424:     	mov	x27, x0
100b99428:     	ldrb	w8, [x21, #0xe4]
100b9942c:     	tbz	w8, #0x0, 0x100b99454 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3f0>
100b99430:     	add	x0, sp, #0x28
100b99434:     	mov	x1, x24
100b99438:     	mov	x2, x26
100b9943c:     	mov	x3, x22
100b99440:     	mov	x4, x25
100b99444:     	mov	w5, #0x1                ; =1
100b99448:     	bl	0x100d16098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b9944c:     	ldr	x21, [sp, #0x8]
100b99450:     	b	0x100b99474 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x410>
100b99454:     	add	x0, sp, #0x28
100b99458:     	mov	x1, x24
100b9945c:     	mov	x2, x26
100b99460:     	mov	x3, x22
100b99464:     	mov	x4, x25
100b99468:     	mov	w5, #0x1                ; =1
100b9946c:     	bl	0x100cc55a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100b99470:     	ldr	x21, [sp, #0x8]
100b99474:     	bic	x1, x28, x20
100b99478:     	add	x2, sp, #0x28
100b9947c:     	mov	x0, x21
100b99480:     	bl	0x100b99064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100b99484:     	mov	x3, x0
100b99488:     	mov	x0, x21
100b9948c:     	mov	x1, x23
100b99490:     	mov	x2, x27
100b99494:     	bl	0x100b9964c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100b99498:     	ldr	x8, [x19]
100b9949c:     	cbz	x8, 0x100b994b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x44c>
100b994a0:     	mov	x19, x0
100b994a4:     	mov	x0, x24
100b994a8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b994ac:     	mov	x0, x19
100b994b0:     	ldp	x29, x30, [sp, #0x90]
100b994b4:     	ldp	x20, x19, [sp, #0x80]
100b994b8:     	ldp	x22, x21, [sp, #0x70]
100b994bc:     	ldp	x24, x23, [sp, #0x60]
100b994c0:     	ldp	x26, x25, [sp, #0x50]
100b994c4:     	ldp	x28, x27, [sp, #0x40]
100b994c8:     	add	sp, sp, #0xa0
100b994cc:     	ret
100b994d0:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100b994d4:     	add	x0, x0, #0x5d8
100b994d8:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b994dc:     	add	x2, x2, #0x798
100b994e0:     	mov	w1, #0x33               ; =51
100b994e4:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b994e8:     	b	0x100b9956c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100b994ec:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100b994f0:     	add	x0, x0, #0x5b6
100b994f4:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b994f8:     	add	x2, x2, #0x720
100b994fc:     	mov	w1, #0x45               ; =69
100b99500:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b99504:     	b	0x100b9956c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100b99508:     	adrp	x5, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b9950c:     	add	x5, x5, #0x738
100b99510:     	add	x1, sp, #0x10
100b99514:     	add	x2, sp, #0x28
100b99518:     	mov	w0, #0x0                ; =0
100b9951c:     	mov	x3, #0x0                ; =0
100b99520:     	bl	0x10127c770 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100b99524:     	b	0x100b9956c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100b99528:     	mov	w22, #0x1               ; =1
100b9952c:     	mov	w20, #0x4               ; =4
100b99530:     	mov	w0, w23
100b99534:     	b	0x100b99544 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4e0>
100b99538:     	mov	x20, x8
100b9953c:     	cmp	x9, #0x0
100b99540:     	cset	w22, eq
100b99544:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b99548:     	add	x2, x2, #0x780
100b9954c:     	mov	x1, x26
100b99550:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b99554:     	b	0x100b9956c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100b99558:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b9955c:     	add	x2, x2, #0x750
100b99560:     	mov	x0, #0x0                ; =0
100b99564:     	mov	x1, #0x0                ; =0
100b99568:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b9956c:     	brk	#0x1
100b99570:     	mov	x24, #0x0               ; =0
100b99574:     	mov	w20, #0x4               ; =4
100b99578:     	b	0x100b993a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x33c>
100b9957c:     	mov	x20, x0
100b99580:     	ldr	x8, [sp, #0x28]
100b99584:     	cbz	x8, 0x100b9959c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x538>
100b99588:     	ldr	x0, [sp, #0x30]
100b9958c:     	b	0x100b99618 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5b4>
100b99590:     	mov	x20, x0
100b99594:     	ldr	x8, [sp, #0x10]
100b99598:     	cbnz	x8, 0x100b995ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x548>
100b9959c:     	mov	x0, x20
100b995a0:     	ldr	x8, [x19]
100b995a4:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b995a8:     	b	0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b995ac:     	ldr	x0, [sp, #0x18]
100b995b0:     	b	0x100b99618 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5b4>
100b995b4:     	mov	x21, x0
100b995b8:     	cbz	x24, 0x100b995dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x578>
100b995bc:     	mov	x0, x20
100b995c0:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b995c4:     	mov	x0, x21
100b995c8:     	ldr	x8, [x19]
100b995cc:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b995d0:     	b	0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b995d4:     	mov	x21, x0
100b995d8:     	tbz	w22, #0x0, 0x100b995bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x558>
100b995dc:     	mov	x0, x21
100b995e0:     	ldr	x8, [x19]
100b995e4:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b995e8:     	b	0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b995ec:     	ldr	x8, [x19]
100b995f0:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b995f4:     	b	0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b995f8:     	ldr	x8, [sp, #0x28]
100b995fc:     	cbnz	x8, 0x100b9960c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a8>
100b99600:     	ldr	x8, [x19]
100b99604:     	cbnz	x8, 0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b99608:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
100b9960c:     	ldr	x8, [sp, #0x30]
100b99610:     	mov	x20, x0
100b99614:     	mov	x0, x8
100b99618:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b9961c:     	mov	x0, x20
100b99620:     	ldr	x8, [x19]
100b99624:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b99628:     	b	0x100b99634 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100b9962c:     	ldr	x8, [x19]
100b99630:     	cbz	x8, 0x100b99608 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100b99634:     	ldr	x8, [x19, #0x8]
100b99638:     	mov	x19, x0
100b9963c:     	mov	x0, x8
100b99640:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b99644:     	mov	x0, x19
100b99648:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
