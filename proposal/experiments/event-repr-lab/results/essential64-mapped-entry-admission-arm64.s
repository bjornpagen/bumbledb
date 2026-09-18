
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010ae29c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>:
1010ae29c:     	stp	x28, x27, [sp, #-0x60]!
1010ae2a0:     	stp	x26, x25, [sp, #0x10]
1010ae2a4:     	stp	x24, x23, [sp, #0x20]
1010ae2a8:     	stp	x22, x21, [sp, #0x30]
1010ae2ac:     	stp	x20, x19, [sp, #0x40]
1010ae2b0:     	stp	x29, x30, [sp, #0x50]
1010ae2b4:     	add	x29, sp, #0x50
1010ae2b8:     	sub	sp, sp, #0x260
1010ae2bc:     	ldr	w8, [x1, #0xf0]
1010ae2c0:     	str	x4, [sp, #0x160]
1010ae2c4:     	str	x8, [sp]
1010ae2c8:     	cmp	x4, x8
1010ae2cc:     	b.ne	0x1010ae64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
1010ae2d0:     	mov	x25, x6
1010ae2d4:     	mov	x21, x5
1010ae2d8:     	mov	x23, x4
1010ae2dc:     	mov	x22, x2
1010ae2e0:     	mov	x20, x1
1010ae2e4:     	mov	x19, x0
1010ae2e8:     	ldp	x24, x10, [x29, #0x18]
1010ae2ec:     	cbz	x4, 0x1010ae3b0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x114>
1010ae2f0:     	mov	x11, #0x0               ; =0
1010ae2f4:     	lsl	x9, x23, #2
1010ae2f8:     	mov	w12, #0x1               ; =1
1010ae2fc:     	mov	x13, x9
1010ae300:     	mov	x14, x3
1010ae304:     	ldr	w15, [x14], #0x4
1010ae308:     	cmp	w15, w8
1010ae30c:     	b.hs	0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae310:     	lsr	x16, x11, x15
1010ae314:     	tbnz	w16, #0x0, 0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae318:     	lsl	x15, x12, x15
1010ae31c:     	orr	x11, x15, x11
1010ae320:     	subs	x13, x13, #0x4
1010ae324:     	b.ne	0x1010ae304 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x68>
1010ae328:     	str	x7, [sp, #0x160]
1010ae32c:     	str	x23, [sp]
1010ae330:     	cmp	x7, x23
1010ae334:     	b.ne	0x1010ae64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
1010ae338:     	mov	x11, #0x0               ; =0
1010ae33c:     	mov	w12, #0x1               ; =1
1010ae340:     	mov	x13, x9
1010ae344:     	mov	x14, x25
1010ae348:     	ldr	w15, [x14], #0x4
1010ae34c:     	cmp	w15, w8
1010ae350:     	b.hs	0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae354:     	lsr	x16, x11, x15
1010ae358:     	tbnz	w16, #0x0, 0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae35c:     	lsl	x15, x12, x15
1010ae360:     	orr	x11, x15, x11
1010ae364:     	subs	x13, x13, #0x4
1010ae368:     	b.ne	0x1010ae348 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xac>
1010ae36c:     	str	x10, [sp, #0x160]
1010ae370:     	str	x23, [sp]
1010ae374:     	cmp	x10, x23
1010ae378:     	b.ne	0x1010ae64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
1010ae37c:     	mov	x10, #0x0               ; =0
1010ae380:     	mov	w11, #0x1               ; =1
1010ae384:     	mov	x12, x24
1010ae388:     	ldr	w13, [x12], #0x4
1010ae38c:     	cmp	w13, w8
1010ae390:     	b.hs	0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae394:     	lsr	x14, x10, x13
1010ae398:     	tbnz	w14, #0x0, 0x1010ae634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
1010ae39c:     	lsl	x13, x11, x13
1010ae3a0:     	orr	x10, x13, x10
1010ae3a4:     	subs	x9, x9, #0x4
1010ae3a8:     	b.ne	0x1010ae388 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xec>
1010ae3ac:     	b	0x1010ae3c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x12c>
1010ae3b0:     	str	x7, [sp, #0x160]
1010ae3b4:     	str	xzr, [sp]
1010ae3b8:     	cbnz	x7, 0x1010ae64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
1010ae3bc:     	str	x10, [sp, #0x160]
1010ae3c0:     	str	xzr, [sp]
1010ae3c4:     	cbnz	x10, 0x1010ae64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
1010ae3c8:     	ldr	x26, [x29, #0x10]
1010ae3cc:     	lsr	x8, x26, x8
1010ae3d0:     	str	x8, [sp]
1010ae3d4:     	cbnz	x8, 0x1010ae670 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d4>
1010ae3d8:     	sub	x0, x29, #0xf0
1010ae3dc:     	mov	x1, x3
1010ae3e0:     	mov	x2, x23
1010ae3e4:     	mov	x3, x24
1010ae3e8:     	mov	x4, x23
1010ae3ec:     	bl	0x10108c338 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
1010ae3f0:     	mov	x0, sp
1010ae3f4:     	mov	x1, x25
1010ae3f8:     	mov	x2, x23
1010ae3fc:     	mov	x3, x24
1010ae400:     	mov	x4, x23
1010ae404:     	bl	0x10108c338 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
1010ae408:     	ldp	q0, q1, [x29, #-0xf0]
1010ae40c:     	stp	q0, q1, [sp, #0x160]
1010ae410:     	ldur	q0, [x29, #-0xd0]
1010ae414:     	ldp	q1, q2, [sp]
1010ae418:     	stp	q0, q1, [sp, #0x180]
1010ae41c:     	ldr	q0, [sp, #0x20]
1010ae420:     	stp	q2, q0, [sp, #0x1a0]
1010ae424:     	mov	w8, #0x4                ; =4
1010ae428:     	stp	xzr, x8, [sp]
1010ae42c:     	str	xzr, [sp, #0x10]
1010ae430:     	cbz	x26, 0x1010ae4b8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x21c>
1010ae434:     	mov	x27, #0x0               ; =0
1010ae438:     	mov	w8, #0x4                ; =4
1010ae43c:     	b	0x1010ae464 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1c8>
1010ae440:     	ldr	x8, [sp, #0x8]
1010ae444:     	rbit	x9, x26
1010ae448:     	clz	x9, x9
1010ae44c:     	str	w9, [x8, x27, lsl #2]
1010ae450:     	add	x27, x27, #0x1
1010ae454:     	str	x27, [sp, #0x10]
1010ae458:     	sub	x9, x26, #0x1
1010ae45c:     	ands	x26, x9, x26
1010ae460:     	b.eq	0x1010ae47c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1e0>
1010ae464:     	ldr	x9, [sp]
1010ae468:     	cmp	x27, x9
1010ae46c:     	b.ne	0x1010ae444 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a8>
1010ae470:     	mov	x0, sp
1010ae474:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1010ae478:     	b	0x1010ae440 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a4>
1010ae47c:     	ldp	x26, x25, [sp]
1010ae480:     	cbz	x27, 0x1010ae4c4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x228>
1010ae484:     	mov	x9, #0x0                ; =0
1010ae488:     	mov	x8, #0x0                ; =0
1010ae48c:     	mov	w10, #0x1               ; =1
1010ae490:     	ldr	w0, [x25, x9, lsl #2]
1010ae494:     	cmp	x23, x0
1010ae498:     	b.ls	0x1010ae698 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3fc>
1010ae49c:     	ldr	w11, [x24, x0, lsl #2]
1010ae4a0:     	lsl	x11, x10, x11
1010ae4a4:     	orr	x8, x11, x8
1010ae4a8:     	add	x9, x9, #0x1
1010ae4ac:     	cmp	x27, x9
1010ae4b0:     	b.ne	0x1010ae490 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1f4>
1010ae4b4:     	b	0x1010ae4c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
1010ae4b8:     	mov	x8, #0x0                ; =0
1010ae4bc:     	mov	w25, #0x4               ; =4
1010ae4c0:     	b	0x1010ae4c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
1010ae4c4:     	mov	x8, #0x0                ; =0
1010ae4c8:     	mov	x9, sp
1010ae4cc:     	add	x23, x9, #0xc8
1010ae4d0:     	movi.2d	v0, #0000000000000000
1010ae4d4:     	stp	q0, q0, [x23, #0x60]
1010ae4d8:     	mov	x9, sp
1010ae4dc:     	stp	q0, q0, [x23, #0x40]
1010ae4e0:     	stur	q0, [x9, #0xf8]
1010ae4e4:     	stur	q0, [x9, #0xe8]
1010ae4e8:     	stur	q0, [x9, #0xd8]
1010ae4ec:     	stur	q0, [x9, #0xc8]
1010ae4f0:     	ldrb	w9, [x20, #0xf6]
1010ae4f4:     	ldrb	w10, [x20, #0xf7]
1010ae4f8:     	stp	xzr, xzr, [sp, #0xb0]
1010ae4fc:     	ldp	q0, q1, [sp, #0x160]
1010ae500:     	ldp	q2, q3, [sp, #0x180]
1010ae504:     	stp	q1, q2, [sp, #0x20]
1010ae508:     	ldp	q1, q2, [sp, #0x1a0]
1010ae50c:     	stp	q1, q2, [sp, #0x50]
1010ae510:     	str	q3, [sp, #0x40]
1010ae514:     	str	xzr, [sp, #0x148]
1010ae518:     	str	x8, [sp, #0xc0]
1010ae51c:     	adrp	x8, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1010ae520:     	add	x8, x8, #0xab0
1010ae524:     	stp	x8, xzr, [sp, #0x70]
1010ae528:     	stp	xzr, xzr, [sp, #0x80]
1010ae52c:     	strb	w9, [sp, #0x150]
1010ae530:     	ldr	q1, [x20]
1010ae534:     	stp	q1, q0, [sp]
1010ae538:     	strb	w10, [sp, #0x151]
1010ae53c:     	mov	w8, #0x8                ; =8
1010ae540:     	stp	x8, xzr, [sp, #0x90]
1010ae544:     	stp	xzr, x8, [sp, #0xa0]
1010ae548:     	cbz	x26, 0x1010ae554 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x2b8>
1010ae54c:     	mov	x0, x25
1010ae550:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010ae554:     	stur	w22, [x29, #-0xe0]
1010ae558:     	stp	xzr, xzr, [x29, #-0xf0]
1010ae55c:     	sub	x0, x29, #0x88
1010ae560:     	sub	x1, x29, #0xf0
1010ae564:     	mov	x2, x20
1010ae568:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1010ae56c:     	add	x22, sp, #0x160
1010ae570:     	ldur	x8, [x29, #-0x78]
1010ae574:     	ldur	q0, [x22, #0xc8]
1010ae578:     	stur	q0, [x29, #-0x70]
1010ae57c:     	stur	x8, [x29, #-0x60]
1010ae580:     	str	x8, [sp, #0x170]
1010ae584:     	str	q0, [sp, #0x160]
1010ae588:     	stur	w21, [x29, #-0xe0]
1010ae58c:     	stp	xzr, xzr, [x29, #-0xf0]
1010ae590:     	sub	x0, x29, #0x88
1010ae594:     	sub	x1, x29, #0xf0
1010ae598:     	mov	x2, x20
1010ae59c:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1010ae5a0:     	ldur	x8, [x29, #-0x78]
1010ae5a4:     	ldur	q0, [x22, #0xc8]
1010ae5a8:     	stur	q0, [x22, #0x18]
1010ae5ac:     	str	x8, [sp, #0x188]
1010ae5b0:     	ldp	q0, q1, [sp, #0x160]
1010ae5b4:     	ldr	q2, [sp, #0x180]
1010ae5b8:     	stp	q1, q2, [x29, #-0xe0]
1010ae5bc:     	stur	q0, [x29, #-0xf0]
1010ae5c0:     	mov	x0, sp
1010ae5c4:     	sub	x2, x29, #0xf0
1010ae5c8:     	mov	x1, x20
1010ae5cc:     	bl	0x100994150 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
1010ae5d0:     	ldp	q0, q1, [x23, #0x40]
1010ae5d4:     	stur	q0, [x19, #0x48]
1010ae5d8:     	stur	q1, [x19, #0x58]
1010ae5dc:     	ldp	q0, q1, [x23, #0x60]
1010ae5e0:     	stur	q0, [x19, #0x68]
1010ae5e4:     	stur	q1, [x19, #0x78]
1010ae5e8:     	ldp	q0, q1, [x23]
1010ae5ec:     	stur	q0, [x19, #0x8]
1010ae5f0:     	stur	q1, [x19, #0x18]
1010ae5f4:     	ldp	q0, q1, [x23, #0x20]
1010ae5f8:     	stur	q0, [x19, #0x28]
1010ae5fc:     	ldr	x8, [x23, #0x80]
1010ae600:     	str	x8, [x19, #0x88]
1010ae604:     	stur	q1, [x19, #0x38]
1010ae608:     	str	w0, [x19]
1010ae60c:     	mov	x0, sp
1010ae610:     	bl	0x100a2043c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
1010ae614:     	add	sp, sp, #0x260
1010ae618:     	ldp	x29, x30, [sp, #0x50]
1010ae61c:     	ldp	x20, x19, [sp, #0x40]
1010ae620:     	ldp	x22, x21, [sp, #0x30]
1010ae624:     	ldp	x24, x23, [sp, #0x20]
1010ae628:     	ldp	x26, x25, [sp, #0x10]
1010ae62c:     	ldp	x28, x27, [sp], #0x60
1010ae630:     	ret
1010ae634:     	adrp	x0, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010ae638:     	add	x0, x0, #0x4ef
1010ae63c:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010ae640:     	add	x2, x2, #0x868
1010ae644:     	mov	w1, #0x2f               ; =47
1010ae648:     	bl	0x1016e73bc <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1010ae64c:     	adrp	x3, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010ae650:     	add	x3, x3, #0x4d9
1010ae654:     	adrp	x5, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010ae658:     	add	x5, x5, #0x850
1010ae65c:     	add	x1, sp, #0x160
1010ae660:     	mov	x2, sp
1010ae664:     	mov	w0, #0x0                ; =0
1010ae668:     	mov	w4, #0x2d               ; =45
1010ae66c:     	bl	0x1016e73f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1010ae670:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1010ae674:     	add	x2, x2, #0x358
1010ae678:     	adrp	x3, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010ae67c:     	add	x3, x3, #0x804
1010ae680:     	adrp	x5, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010ae684:     	add	x5, x5, #0xed8
1010ae688:     	mov	x1, sp
1010ae68c:     	mov	w0, #0x0                ; =0
1010ae690:     	mov	w4, #0x57               ; =87
1010ae694:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1010ae698:     	adrp	x2, 0x101950000 <dyld_stub_binder+0x101950000>
1010ae69c:     	add	x2, x2, #0x178
1010ae6a0:     	mov	x1, x23
1010ae6a4:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010ae6a8:     	brk	#0x1
1010ae6ac:     	mov	x19, x0
1010ae6b0:     	sub	x0, x29, #0xf0
1010ae6b4:     	bl	0x100a1ef24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
1010ae6b8:     	mov	x0, x19
1010ae6bc:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010ae6c0:     	mov	x19, x0
1010ae6c4:     	mov	x0, sp
1010ae6c8:     	bl	0x100a2043c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
1010ae6cc:     	mov	x0, x19
1010ae6d0:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010ae6d4:     	mov	x19, x0
1010ae6d8:     	ldr	x8, [sp]
1010ae6dc:     	cbz	x8, 0x1010ae6e8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x44c>
1010ae6e0:     	ldr	x0, [sp, #0x8]
1010ae6e4:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010ae6e8:     	add	x0, sp, #0x160
1010ae6ec:     	bl	0x100a024ac <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
1010ae6f0:     	mov	x0, x19
1010ae6f4:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010ae6f8:     	mov	x19, x0
1010ae6fc:     	add	x0, sp, #0x160
1010ae700:     	bl	0x100a024ac <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
1010ae704:     	cbnz	x26, 0x1010ae710 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x474>
1010ae708:     	mov	x0, x19
1010ae70c:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010ae710:     	mov	x0, x25
1010ae714:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010ae718:     	mov	x0, x19
1010ae71c:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
