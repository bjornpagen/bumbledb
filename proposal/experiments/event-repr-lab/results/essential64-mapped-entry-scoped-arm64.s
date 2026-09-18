
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100edf29c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>:
100edf29c:     	stp	x28, x27, [sp, #-0x60]!
100edf2a0:     	stp	x26, x25, [sp, #0x10]
100edf2a4:     	stp	x24, x23, [sp, #0x20]
100edf2a8:     	stp	x22, x21, [sp, #0x30]
100edf2ac:     	stp	x20, x19, [sp, #0x40]
100edf2b0:     	stp	x29, x30, [sp, #0x50]
100edf2b4:     	add	x29, sp, #0x50
100edf2b8:     	sub	sp, sp, #0x260
100edf2bc:     	ldr	w8, [x1, #0xf0]
100edf2c0:     	str	x4, [sp, #0x160]
100edf2c4:     	str	x8, [sp]
100edf2c8:     	cmp	x4, x8
100edf2cc:     	b.ne	0x100edf64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100edf2d0:     	mov	x25, x6
100edf2d4:     	mov	x21, x5
100edf2d8:     	mov	x23, x4
100edf2dc:     	mov	x22, x2
100edf2e0:     	mov	x20, x1
100edf2e4:     	mov	x19, x0
100edf2e8:     	ldp	x24, x10, [x29, #0x18]
100edf2ec:     	cbz	x4, 0x100edf3b0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x114>
100edf2f0:     	mov	x11, #0x0               ; =0
100edf2f4:     	lsl	x9, x23, #2
100edf2f8:     	mov	w12, #0x1               ; =1
100edf2fc:     	mov	x13, x9
100edf300:     	mov	x14, x3
100edf304:     	ldr	w15, [x14], #0x4
100edf308:     	cmp	w15, w8
100edf30c:     	b.hs	0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf310:     	lsr	x16, x11, x15
100edf314:     	tbnz	w16, #0x0, 0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf318:     	lsl	x15, x12, x15
100edf31c:     	orr	x11, x15, x11
100edf320:     	subs	x13, x13, #0x4
100edf324:     	b.ne	0x100edf304 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x68>
100edf328:     	str	x7, [sp, #0x160]
100edf32c:     	str	x23, [sp]
100edf330:     	cmp	x7, x23
100edf334:     	b.ne	0x100edf64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100edf338:     	mov	x11, #0x0               ; =0
100edf33c:     	mov	w12, #0x1               ; =1
100edf340:     	mov	x13, x9
100edf344:     	mov	x14, x25
100edf348:     	ldr	w15, [x14], #0x4
100edf34c:     	cmp	w15, w8
100edf350:     	b.hs	0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf354:     	lsr	x16, x11, x15
100edf358:     	tbnz	w16, #0x0, 0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf35c:     	lsl	x15, x12, x15
100edf360:     	orr	x11, x15, x11
100edf364:     	subs	x13, x13, #0x4
100edf368:     	b.ne	0x100edf348 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xac>
100edf36c:     	str	x10, [sp, #0x160]
100edf370:     	str	x23, [sp]
100edf374:     	cmp	x10, x23
100edf378:     	b.ne	0x100edf64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100edf37c:     	mov	x10, #0x0               ; =0
100edf380:     	mov	w11, #0x1               ; =1
100edf384:     	mov	x12, x24
100edf388:     	ldr	w13, [x12], #0x4
100edf38c:     	cmp	w13, w8
100edf390:     	b.hs	0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf394:     	lsr	x14, x10, x13
100edf398:     	tbnz	w14, #0x0, 0x100edf634 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100edf39c:     	lsl	x13, x11, x13
100edf3a0:     	orr	x10, x13, x10
100edf3a4:     	subs	x9, x9, #0x4
100edf3a8:     	b.ne	0x100edf388 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xec>
100edf3ac:     	b	0x100edf3c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x12c>
100edf3b0:     	str	x7, [sp, #0x160]
100edf3b4:     	str	xzr, [sp]
100edf3b8:     	cbnz	x7, 0x100edf64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100edf3bc:     	str	x10, [sp, #0x160]
100edf3c0:     	str	xzr, [sp]
100edf3c4:     	cbnz	x10, 0x100edf64c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100edf3c8:     	ldr	x26, [x29, #0x10]
100edf3cc:     	lsr	x8, x26, x8
100edf3d0:     	str	x8, [sp]
100edf3d4:     	cbnz	x8, 0x100edf670 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d4>
100edf3d8:     	sub	x0, x29, #0xf0
100edf3dc:     	mov	x1, x3
100edf3e0:     	mov	x2, x23
100edf3e4:     	mov	x3, x24
100edf3e8:     	mov	x4, x23
100edf3ec:     	bl	0x100ebd0b8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100edf3f0:     	mov	x0, sp
100edf3f4:     	mov	x1, x25
100edf3f8:     	mov	x2, x23
100edf3fc:     	mov	x3, x24
100edf400:     	mov	x4, x23
100edf404:     	bl	0x100ebd0b8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100edf408:     	ldp	q0, q1, [x29, #-0xf0]
100edf40c:     	stp	q0, q1, [sp, #0x160]
100edf410:     	ldur	q0, [x29, #-0xd0]
100edf414:     	ldp	q1, q2, [sp]
100edf418:     	stp	q0, q1, [sp, #0x180]
100edf41c:     	ldr	q0, [sp, #0x20]
100edf420:     	stp	q2, q0, [sp, #0x1a0]
100edf424:     	mov	w8, #0x4                ; =4
100edf428:     	stp	xzr, x8, [sp]
100edf42c:     	str	xzr, [sp, #0x10]
100edf430:     	cbz	x26, 0x100edf4b8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x21c>
100edf434:     	mov	x27, #0x0               ; =0
100edf438:     	mov	w8, #0x4                ; =4
100edf43c:     	b	0x100edf464 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1c8>
100edf440:     	ldr	x8, [sp, #0x8]
100edf444:     	rbit	x9, x26
100edf448:     	clz	x9, x9
100edf44c:     	str	w9, [x8, x27, lsl #2]
100edf450:     	add	x27, x27, #0x1
100edf454:     	str	x27, [sp, #0x10]
100edf458:     	sub	x9, x26, #0x1
100edf45c:     	ands	x26, x9, x26
100edf460:     	b.eq	0x100edf47c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1e0>
100edf464:     	ldr	x9, [sp]
100edf468:     	cmp	x27, x9
100edf46c:     	b.ne	0x100edf444 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a8>
100edf470:     	mov	x0, sp
100edf474:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100edf478:     	b	0x100edf440 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a4>
100edf47c:     	ldp	x26, x25, [sp]
100edf480:     	cbz	x27, 0x100edf4c4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x228>
100edf484:     	mov	x9, #0x0                ; =0
100edf488:     	mov	x8, #0x0                ; =0
100edf48c:     	mov	w10, #0x1               ; =1
100edf490:     	ldr	w0, [x25, x9, lsl #2]
100edf494:     	cmp	x23, x0
100edf498:     	b.ls	0x100edf698 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3fc>
100edf49c:     	ldr	w11, [x24, x0, lsl #2]
100edf4a0:     	lsl	x11, x10, x11
100edf4a4:     	orr	x8, x11, x8
100edf4a8:     	add	x9, x9, #0x1
100edf4ac:     	cmp	x27, x9
100edf4b0:     	b.ne	0x100edf490 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1f4>
100edf4b4:     	b	0x100edf4c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100edf4b8:     	mov	x8, #0x0                ; =0
100edf4bc:     	mov	w25, #0x4               ; =4
100edf4c0:     	b	0x100edf4c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100edf4c4:     	mov	x8, #0x0                ; =0
100edf4c8:     	mov	x9, sp
100edf4cc:     	add	x23, x9, #0xc8
100edf4d0:     	movi.2d	v0, #0000000000000000
100edf4d4:     	stp	q0, q0, [x23, #0x60]
100edf4d8:     	mov	x9, sp
100edf4dc:     	stp	q0, q0, [x23, #0x40]
100edf4e0:     	stur	q0, [x9, #0xf8]
100edf4e4:     	stur	q0, [x9, #0xe8]
100edf4e8:     	stur	q0, [x9, #0xd8]
100edf4ec:     	stur	q0, [x9, #0xc8]
100edf4f0:     	ldrb	w9, [x20, #0xf6]
100edf4f4:     	ldrb	w10, [x20, #0xf7]
100edf4f8:     	stp	xzr, xzr, [sp, #0xb0]
100edf4fc:     	ldp	q0, q1, [sp, #0x160]
100edf500:     	ldp	q2, q3, [sp, #0x180]
100edf504:     	stp	q1, q2, [sp, #0x20]
100edf508:     	ldp	q1, q2, [sp, #0x1a0]
100edf50c:     	stp	q1, q2, [sp, #0x50]
100edf510:     	str	q3, [sp, #0x40]
100edf514:     	str	xzr, [sp, #0x148]
100edf518:     	str	x8, [sp, #0xc0]
100edf51c:     	adrp	x8, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100edf520:     	add	x8, x8, #0x9b8
100edf524:     	stp	x8, xzr, [sp, #0x70]
100edf528:     	stp	xzr, xzr, [sp, #0x80]
100edf52c:     	strb	w9, [sp, #0x150]
100edf530:     	ldr	q1, [x20]
100edf534:     	stp	q1, q0, [sp]
100edf538:     	strb	w10, [sp, #0x151]
100edf53c:     	mov	w8, #0x8                ; =8
100edf540:     	stp	x8, xzr, [sp, #0x90]
100edf544:     	stp	xzr, x8, [sp, #0xa0]
100edf548:     	cbz	x26, 0x100edf554 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x2b8>
100edf54c:     	mov	x0, x25
100edf550:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edf554:     	stur	w22, [x29, #-0xe0]
100edf558:     	stp	xzr, xzr, [x29, #-0xf0]
100edf55c:     	sub	x0, x29, #0x88
100edf560:     	sub	x1, x29, #0xf0
100edf564:     	mov	x2, x20
100edf568:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100edf56c:     	add	x22, sp, #0x160
100edf570:     	ldur	x8, [x29, #-0x78]
100edf574:     	ldur	q0, [x22, #0xc8]
100edf578:     	stur	q0, [x29, #-0x70]
100edf57c:     	stur	x8, [x29, #-0x60]
100edf580:     	str	x8, [sp, #0x170]
100edf584:     	str	q0, [sp, #0x160]
100edf588:     	stur	w21, [x29, #-0xe0]
100edf58c:     	stp	xzr, xzr, [x29, #-0xf0]
100edf590:     	sub	x0, x29, #0x88
100edf594:     	sub	x1, x29, #0xf0
100edf598:     	mov	x2, x20
100edf59c:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100edf5a0:     	ldur	x8, [x29, #-0x78]
100edf5a4:     	ldur	q0, [x22, #0xc8]
100edf5a8:     	stur	q0, [x22, #0x18]
100edf5ac:     	str	x8, [sp, #0x188]
100edf5b0:     	ldp	q0, q1, [sp, #0x160]
100edf5b4:     	ldr	q2, [sp, #0x180]
100edf5b8:     	stp	q1, q2, [x29, #-0xe0]
100edf5bc:     	stur	q0, [x29, #-0xf0]
100edf5c0:     	mov	x0, sp
100edf5c4:     	sub	x2, x29, #0xf0
100edf5c8:     	mov	x1, x20
100edf5cc:     	bl	0x1008a30bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
100edf5d0:     	ldp	q0, q1, [x23, #0x40]
100edf5d4:     	stur	q0, [x19, #0x48]
100edf5d8:     	stur	q1, [x19, #0x58]
100edf5dc:     	ldp	q0, q1, [x23, #0x60]
100edf5e0:     	stur	q0, [x19, #0x68]
100edf5e4:     	stur	q1, [x19, #0x78]
100edf5e8:     	ldp	q0, q1, [x23]
100edf5ec:     	stur	q0, [x19, #0x8]
100edf5f0:     	stur	q1, [x19, #0x18]
100edf5f4:     	ldp	q0, q1, [x23, #0x20]
100edf5f8:     	stur	q0, [x19, #0x28]
100edf5fc:     	ldr	x8, [x23, #0x80]
100edf600:     	str	x8, [x19, #0x88]
100edf604:     	stur	q1, [x19, #0x38]
100edf608:     	str	w0, [x19]
100edf60c:     	mov	x0, sp
100edf610:     	bl	0x10091aa64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100edf614:     	add	sp, sp, #0x260
100edf618:     	ldp	x29, x30, [sp, #0x50]
100edf61c:     	ldp	x20, x19, [sp, #0x40]
100edf620:     	ldp	x22, x21, [sp, #0x30]
100edf624:     	ldp	x24, x23, [sp, #0x20]
100edf628:     	ldp	x26, x25, [sp, #0x10]
100edf62c:     	ldp	x28, x27, [sp], #0x60
100edf630:     	ret
100edf634:     	adrp	x0, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edf638:     	add	x0, x0, #0x57f
100edf63c:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
100edf640:     	add	x2, x2, #0x9a8
100edf644:     	mov	w1, #0x2f               ; =47
100edf648:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100edf64c:     	adrp	x3, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edf650:     	add	x3, x3, #0x569
100edf654:     	adrp	x5, 0x101798000 <dyld_stub_binder+0x101798000>
100edf658:     	add	x5, x5, #0x990
100edf65c:     	add	x1, sp, #0x160
100edf660:     	mov	x2, sp
100edf664:     	mov	w0, #0x0                ; =0
100edf668:     	mov	w4, #0x2d               ; =45
100edf66c:     	bl	0x101506cb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100edf670:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100edf674:     	add	x2, x2, #0x268
100edf678:     	adrp	x3, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edf67c:     	add	x3, x3, #0x894
100edf680:     	adrp	x5, 0x101799000 <dyld_stub_binder+0x101799000>
100edf684:     	add	x5, x5, #0x18
100edf688:     	mov	x1, sp
100edf68c:     	mov	w0, #0x0                ; =0
100edf690:     	mov	w4, #0x57               ; =87
100edf694:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100edf698:     	adrp	x2, 0x10175b000 <dyld_stub_binder+0x10175b000>
100edf69c:     	add	x2, x2, #0x438
100edf6a0:     	mov	x1, x23
100edf6a4:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100edf6a8:     	brk	#0x1
100edf6ac:     	mov	x19, x0
100edf6b0:     	sub	x0, x29, #0xf0
100edf6b4:     	bl	0x10091954c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100edf6b8:     	mov	x0, x19
100edf6bc:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edf6c0:     	mov	x19, x0
100edf6c4:     	mov	x0, sp
100edf6c8:     	bl	0x10091aa64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100edf6cc:     	mov	x0, x19
100edf6d0:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edf6d4:     	mov	x19, x0
100edf6d8:     	ldr	x8, [sp]
100edf6dc:     	cbz	x8, 0x100edf6e8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x44c>
100edf6e0:     	ldr	x0, [sp, #0x8]
100edf6e4:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edf6e8:     	add	x0, sp, #0x160
100edf6ec:     	bl	0x1008fdf00 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100edf6f0:     	mov	x0, x19
100edf6f4:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edf6f8:     	mov	x19, x0
100edf6fc:     	add	x0, sp, #0x160
100edf700:     	bl	0x1008fdf00 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100edf704:     	cbnz	x26, 0x100edf710 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x474>
100edf708:     	mov	x0, x19
100edf70c:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edf710:     	mov	x0, x25
100edf714:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edf718:     	mov	x0, x19
100edf71c:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
