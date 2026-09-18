
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a211e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>:
100a211e4:     	sub	sp, sp, #0xa0
100a211e8:     	stp	x28, x27, [sp, #0x40]
100a211ec:     	stp	x26, x25, [sp, #0x50]
100a211f0:     	stp	x24, x23, [sp, #0x60]
100a211f4:     	stp	x22, x21, [sp, #0x70]
100a211f8:     	stp	x20, x19, [sp, #0x80]
100a211fc:     	stp	x29, x30, [sp, #0x90]
100a21200:     	add	x29, sp, #0x90
100a21204:     	mov	x19, x2
100a21208:     	ldr	w8, [x0, #0xa8]
100a2120c:     	lsr	x8, x1, x8
100a21210:     	cbnz	x8, 0x100a21650 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x46c>
100a21214:     	mov	x28, x1
100a21218:     	fmov	d0, x28
100a2121c:     	cnt.8b	v0, v0
100a21220:     	addv.8b	b0, v0
100a21224:     	fmov	x22, d0
100a21228:     	cmp	x22, #0x15
100a2122c:     	b.hs	0x100a2166c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x488>
100a21230:     	mov	w8, #0x1                ; =1
100a21234:     	lsl	x8, x8, x22
100a21238:     	ldr	x9, [x19, #0x10]
100a2123c:     	str	x9, [sp, #0x10]
100a21240:     	lsr	x10, x8, #6
100a21244:     	cmp	x22, #0x6
100a21248:     	cinc	x10, x10, lo
100a2124c:     	str	x10, [sp, #0x28]
100a21250:     	cmp	x9, x10
100a21254:     	b.ne	0x100a21688 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4a4>
100a21258:     	cmp	x22, #0x6
100a2125c:     	b.hs	0x100a21280 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x9c>
100a21260:     	cbz	x9, 0x100a216d8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4f4>
100a21264:     	ldr	x9, [x19, #0x8]
100a21268:     	mov	x10, #-0x1              ; =-1
100a2126c:     	lsl	x8, x10, x8
100a21270:     	ldr	x10, [x9]
100a21274:     	bic	x8, x10, x8
100a21278:     	str	x8, [x9]
100a2127c:     	cbz	x28, 0x100a21454 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x270>
100a21280:     	mov	w23, #0x0               ; =0
100a21284:     	str	x0, [sp, #0x8]
100a21288:     	ldrb	w27, [x0, #0xac]
100a2128c:     	mov	w20, #0x1               ; =1
100a21290:     	b	0x100a212a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100a21294:     	add	w23, w23, #0x1
100a21298:     	cmp	w23, w22
100a2129c:     	b.hs	0x100a21448 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x264>
100a212a0:     	tbz	w27, #0x0, 0x100a212c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xdc>
100a212a4:     	ldp	x24, x25, [x19, #0x8]
100a212a8:     	mov	x0, x24
100a212ac:     	mov	x1, x25
100a212b0:     	mov	x2, x23
100a212b4:     	bl	0x100ba3c10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>
100a212b8:     	tbz	w0, #0x0, 0x100a21294 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100a212bc:     	b	0x100a21358 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x174>
100a212c0:     	ldp	x24, x25, [x19, #0x8]
100a212c4:     	add	x0, sp, #0x10
100a212c8:     	mov	x1, x24
100a212cc:     	mov	x2, x25
100a212d0:     	mov	x3, x22
100a212d4:     	mov	x4, x23
100a212d8:     	mov	w5, #0x0                ; =0
100a212dc:     	bl	0x100b534e0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a212e0:     	add	x0, sp, #0x28
100a212e4:     	mov	x1, x24
100a212e8:     	mov	x2, x25
100a212ec:     	mov	x3, x22
100a212f0:     	mov	x4, x23
100a212f4:     	mov	w5, #0x1                ; =1
100a212f8:     	bl	0x100b534e0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a212fc:     	ldr	x8, [sp, #0x20]
100a21300:     	ldr	x9, [sp, #0x38]
100a21304:     	cmp	x8, x9
100a21308:     	b.ne	0x100a21330 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x14c>
100a2130c:     	ldr	x1, [sp, #0x30]
100a21310:     	ldr	x0, [sp, #0x18]
100a21314:     	lsl	x2, x8, #3
100a21318:     	bl	0x10110d210 <dyld_stub_binder+0x10110d210>
100a2131c:     	cmp	w0, #0x0
100a21320:     	cset	w21, eq
100a21324:     	ldr	x8, [sp, #0x28]
100a21328:     	cbnz	x8, 0x100a2133c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x158>
100a2132c:     	b	0x100a21344 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100a21330:     	mov	w21, #0x0               ; =0
100a21334:     	ldr	x8, [sp, #0x28]
100a21338:     	cbz	x8, 0x100a21344 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x160>
100a2133c:     	ldr	x0, [sp, #0x30]
100a21340:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21344:     	ldr	x8, [sp, #0x10]
100a21348:     	cbz	x8, 0x100a21354 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x170>
100a2134c:     	ldr	x0, [sp, #0x18]
100a21350:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21354:     	tbz	w21, #0x0, 0x100a21294 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xb0>
100a21358:     	mov	w8, #0x4                ; =4
100a2135c:     	stp	xzr, x8, [sp, #0x28]
100a21360:     	str	xzr, [sp, #0x38]
100a21364:     	mov	x26, #0x0               ; =0
100a21368:     	cbz	x28, 0x100a216a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4c4>
100a2136c:     	mov	w8, #0x4                ; =4
100a21370:     	mov	x21, x28
100a21374:     	b	0x100a21398 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1b4>
100a21378:     	rbit	x9, x21
100a2137c:     	clz	x9, x9
100a21380:     	str	w9, [x8, x26, lsl #2]
100a21384:     	add	x26, x26, #0x1
100a21388:     	str	x26, [sp, #0x38]
100a2138c:     	sub	x9, x21, #0x1
100a21390:     	ands	x21, x9, x21
100a21394:     	b.eq	0x100a213b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1d0>
100a21398:     	ldr	x9, [sp, #0x28]
100a2139c:     	cmp	x26, x9
100a213a0:     	b.ne	0x100a21378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100a213a4:     	add	x0, sp, #0x28
100a213a8:     	bl	0x101105b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100a213ac:     	ldr	x8, [sp, #0x30]
100a213b0:     	b	0x100a21378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x194>
100a213b4:     	ldp	x9, x8, [sp, #0x28]
100a213b8:     	mov	w0, w23
100a213bc:     	cmp	x26, x0
100a213c0:     	b.ls	0x100a216b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4d4>
100a213c4:     	ldr	w26, [x8, x0, lsl #2]
100a213c8:     	cbz	x9, 0x100a213d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x1f0>
100a213cc:     	mov	x0, x8
100a213d0:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a213d4:     	tbz	w27, #0x0, 0x100a213f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x214>
100a213d8:     	add	x0, sp, #0x28
100a213dc:     	mov	x1, x24
100a213e0:     	mov	x2, x25
100a213e4:     	mov	x3, x22
100a213e8:     	mov	x4, x23
100a213ec:     	mov	w5, #0x0                ; =0
100a213f0:     	bl	0x100ba4ad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a213f4:     	b	0x100a21414 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x230>
100a213f8:     	add	x0, sp, #0x28
100a213fc:     	mov	x1, x24
100a21400:     	mov	x2, x25
100a21404:     	mov	x3, x22
100a21408:     	mov	x4, x23
100a2140c:     	mov	w5, #0x0                ; =0
100a21410:     	bl	0x100b534e0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a21414:     	ldr	x8, [x19]
100a21418:     	cbz	x8, 0x100a21424 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x240>
100a2141c:     	mov	x0, x24
100a21420:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21424:     	lsl	x8, x20, x26
100a21428:     	bic	x28, x28, x8
100a2142c:     	ldur	q0, [sp, #0x28]
100a21430:     	str	q0, [x19]
100a21434:     	ldr	x8, [sp, #0x38]
100a21438:     	str	x8, [x19, #0x10]
100a2143c:     	sub	w22, w22, #0x1
100a21440:     	cmp	w23, w22
100a21444:     	b.lo	0x100a212a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0xbc>
100a21448:     	cmp	w22, #0xa
100a2144c:     	b.hs	0x100a2147c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x298>
100a21450:     	ldr	x0, [sp, #0x8]
100a21454:     	mov	x1, x28
100a21458:     	mov	x2, x19
100a2145c:     	ldp	x29, x30, [sp, #0x90]
100a21460:     	ldp	x20, x19, [sp, #0x80]
100a21464:     	ldp	x22, x21, [sp, #0x70]
100a21468:     	ldp	x24, x23, [sp, #0x60]
100a2146c:     	ldp	x26, x25, [sp, #0x50]
100a21470:     	ldp	x28, x27, [sp, #0x40]
100a21474:     	add	sp, sp, #0xa0
100a21478:     	b	0x100a1fc0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E11exact_tableB6_>
100a2147c:     	ldr	x0, [sp, #0x8]
100a21480:     	mov	x1, x28
100a21484:     	bl	0x100a18fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a21488:     	mov	w8, #0x4                ; =4
100a2148c:     	stp	xzr, x8, [sp, #0x28]
100a21490:     	str	xzr, [sp, #0x38]
100a21494:     	cbz	x28, 0x100a216f0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x50c>
100a21498:     	mov	x23, x0
100a2149c:     	mov	x20, #0x0               ; =0
100a214a0:     	mov	w8, #0x4                ; =4
100a214a4:     	mov	w9, #0x1                ; =1
100a214a8:     	mov	x24, x28
100a214ac:     	b	0x100a214d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2f0>
100a214b0:     	rbit	x9, x24
100a214b4:     	clz	x9, x9
100a214b8:     	str	w9, [x8, x20]
100a214bc:     	str	x25, [sp, #0x38]
100a214c0:     	sub	x10, x24, #0x1
100a214c4:     	add	x20, x20, #0x4
100a214c8:     	add	x9, x25, #0x1
100a214cc:     	ands	x24, x10, x24
100a214d0:     	b.eq	0x100a214f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x314>
100a214d4:     	mov	x25, x9
100a214d8:     	sub	x9, x9, #0x1
100a214dc:     	ldr	x10, [sp, #0x28]
100a214e0:     	cmp	x9, x10
100a214e4:     	b.ne	0x100a214b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2cc>
100a214e8:     	add	x0, sp, #0x28
100a214ec:     	bl	0x101105b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100a214f0:     	ldr	x8, [sp, #0x30]
100a214f4:     	b	0x100a214b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x2cc>
100a214f8:     	ldp	x24, x0, [sp, #0x28]
100a214fc:     	cbz	x25, 0x100a2151c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x338>
100a21500:     	mov	x25, #0x0               ; =0
100a21504:     	ldr	w8, [x0, x25, lsl #2]
100a21508:     	cmp	w8, w23
100a2150c:     	b.eq	0x100a21530 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x34c>
100a21510:     	add	x25, x25, #0x1
100a21514:     	subs	x20, x20, #0x4
100a21518:     	b.ne	0x100a21504 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x320>
100a2151c:     	mov	x20, x0
100a21520:     	adrp	x0, 0x101373000 <dyld_stub_binder+0x101373000>
100a21524:     	add	x0, x0, #0xe88
100a21528:     	bl	0x101104df4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100a2152c:     	b	0x100a216ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100a21530:     	cbz	x24, 0x100a21538 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x354>
100a21534:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21538:     	ldp	x24, x26, [x19, #0x8]
100a2153c:     	ldr	x8, [sp, #0x8]
100a21540:     	ldrb	w8, [x8, #0xac]
100a21544:     	tbz	w8, #0x0, 0x100a2156c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x388>
100a21548:     	add	x0, sp, #0x28
100a2154c:     	mov	x1, x24
100a21550:     	mov	x2, x26
100a21554:     	mov	x3, x22
100a21558:     	mov	x4, x25
100a2155c:     	mov	w5, #0x0                ; =0
100a21560:     	bl	0x100ba4ad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a21564:     	ldr	x21, [sp, #0x8]
100a21568:     	b	0x100a2158c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3a8>
100a2156c:     	add	x0, sp, #0x28
100a21570:     	mov	x1, x24
100a21574:     	mov	x2, x26
100a21578:     	mov	x3, x22
100a2157c:     	mov	x4, x25
100a21580:     	mov	w5, #0x0                ; =0
100a21584:     	bl	0x100b534e0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a21588:     	ldr	x21, [sp, #0x8]
100a2158c:     	mov	w8, #0x1                ; =1
100a21590:     	lsl	x20, x8, x23
100a21594:     	bic	x1, x28, x20
100a21598:     	add	x2, sp, #0x28
100a2159c:     	mov	x0, x21
100a215a0:     	bl	0x100a211e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a215a4:     	mov	x27, x0
100a215a8:     	ldrb	w8, [x21, #0xac]
100a215ac:     	tbz	w8, #0x0, 0x100a215d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x3f0>
100a215b0:     	add	x0, sp, #0x28
100a215b4:     	mov	x1, x24
100a215b8:     	mov	x2, x26
100a215bc:     	mov	x3, x22
100a215c0:     	mov	x4, x25
100a215c4:     	mov	w5, #0x1                ; =1
100a215c8:     	bl	0x100ba4ad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a215cc:     	ldr	x21, [sp, #0x8]
100a215d0:     	b	0x100a215f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x410>
100a215d4:     	add	x0, sp, #0x28
100a215d8:     	mov	x1, x24
100a215dc:     	mov	x2, x26
100a215e0:     	mov	x3, x22
100a215e4:     	mov	x4, x25
100a215e8:     	mov	w5, #0x1                ; =1
100a215ec:     	bl	0x100b534e0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a215f0:     	ldr	x21, [sp, #0x8]
100a215f4:     	bic	x1, x28, x20
100a215f8:     	add	x2, sp, #0x28
100a215fc:     	mov	x0, x21
100a21600:     	bl	0x100a211e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a21604:     	mov	x3, x0
100a21608:     	mov	x0, x21
100a2160c:     	mov	x1, x23
100a21610:     	mov	x2, x27
100a21614:     	bl	0x100a217cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100a21618:     	ldr	x8, [x19]
100a2161c:     	cbz	x8, 0x100a21630 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x44c>
100a21620:     	mov	x19, x0
100a21624:     	mov	x0, x24
100a21628:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a2162c:     	mov	x0, x19
100a21630:     	ldp	x29, x30, [sp, #0x90]
100a21634:     	ldp	x20, x19, [sp, #0x80]
100a21638:     	ldp	x22, x21, [sp, #0x70]
100a2163c:     	ldp	x24, x23, [sp, #0x60]
100a21640:     	ldp	x26, x25, [sp, #0x50]
100a21644:     	ldp	x28, x27, [sp, #0x40]
100a21648:     	add	sp, sp, #0xa0
100a2164c:     	ret
100a21650:     	adrp	x0, 0x1011c1000 <dyld_stub_binder+0x1011c1000>
100a21654:     	add	x0, x0, #0x61d
100a21658:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a2165c:     	add	x2, x2, #0xeb8
100a21660:     	mov	w1, #0x33               ; =51
100a21664:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a21668:     	b	0x100a216ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100a2166c:     	adrp	x0, 0x1011c1000 <dyld_stub_binder+0x1011c1000>
100a21670:     	add	x0, x0, #0x5fb
100a21674:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a21678:     	add	x2, x2, #0xe40
100a2167c:     	mov	w1, #0x45               ; =69
100a21680:     	bl	0x101104bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100a21684:     	b	0x100a216ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100a21688:     	adrp	x5, 0x101373000 <dyld_stub_binder+0x101373000>
100a2168c:     	add	x5, x5, #0xe58
100a21690:     	add	x1, sp, #0x10
100a21694:     	add	x2, sp, #0x28
100a21698:     	mov	w0, #0x0                ; =0
100a2169c:     	mov	x3, #0x0                ; =0
100a216a0:     	bl	0x101104c30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100a216a4:     	b	0x100a216ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100a216a8:     	mov	w22, #0x1               ; =1
100a216ac:     	mov	w20, #0x4               ; =4
100a216b0:     	mov	w0, w23
100a216b4:     	b	0x100a216c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x4e0>
100a216b8:     	mov	x20, x8
100a216bc:     	cmp	x9, #0x0
100a216c0:     	cset	w22, eq
100a216c4:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a216c8:     	add	x2, x2, #0xea0
100a216cc:     	mov	x1, x26
100a216d0:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a216d4:     	b	0x100a216ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x508>
100a216d8:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a216dc:     	add	x2, x2, #0xe70
100a216e0:     	mov	x0, #0x0                ; =0
100a216e4:     	mov	x1, #0x0                ; =0
100a216e8:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a216ec:     	brk	#0x1
100a216f0:     	mov	x24, #0x0               ; =0
100a216f4:     	mov	w20, #0x4               ; =4
100a216f8:     	b	0x100a21520 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x33c>
100a216fc:     	mov	x20, x0
100a21700:     	ldr	x8, [sp, #0x28]
100a21704:     	cbz	x8, 0x100a2171c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x538>
100a21708:     	ldr	x0, [sp, #0x30]
100a2170c:     	b	0x100a21798 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5b4>
100a21710:     	mov	x20, x0
100a21714:     	ldr	x8, [sp, #0x10]
100a21718:     	cbnz	x8, 0x100a2172c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x548>
100a2171c:     	mov	x0, x20
100a21720:     	ldr	x8, [x19]
100a21724:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a21728:     	b	0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a2172c:     	ldr	x0, [sp, #0x18]
100a21730:     	b	0x100a21798 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5b4>
100a21734:     	mov	x21, x0
100a21738:     	cbz	x24, 0x100a2175c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x578>
100a2173c:     	mov	x0, x20
100a21740:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21744:     	mov	x0, x21
100a21748:     	ldr	x8, [x19]
100a2174c:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a21750:     	b	0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a21754:     	mov	x21, x0
100a21758:     	tbz	w22, #0x0, 0x100a2173c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x558>
100a2175c:     	mov	x0, x21
100a21760:     	ldr	x8, [x19]
100a21764:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a21768:     	b	0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a2176c:     	ldr	x8, [x19]
100a21770:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a21774:     	b	0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a21778:     	ldr	x8, [sp, #0x28]
100a2177c:     	cbnz	x8, 0x100a2178c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a8>
100a21780:     	ldr	x8, [x19]
100a21784:     	cbnz	x8, 0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a21788:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
100a2178c:     	ldr	x8, [sp, #0x30]
100a21790:     	mov	x20, x0
100a21794:     	mov	x0, x8
100a21798:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a2179c:     	mov	x0, x20
100a217a0:     	ldr	x8, [x19]
100a217a4:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a217a8:     	b	0x100a217b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5d0>
100a217ac:     	ldr	x8, [x19]
100a217b0:     	cbz	x8, 0x100a21788 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_+0x5a4>
100a217b4:     	ldr	x8, [x19, #0x8]
100a217b8:     	mov	x19, x0
100a217bc:     	mov	x0, x8
100a217c0:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a217c4:     	mov	x0, x19
100a217c8:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
