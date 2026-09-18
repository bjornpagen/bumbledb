
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d933dc <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>:
100d933dc:     	stp	x28, x27, [sp, #-0x60]!
100d933e0:     	stp	x26, x25, [sp, #0x10]
100d933e4:     	stp	x24, x23, [sp, #0x20]
100d933e8:     	stp	x22, x21, [sp, #0x30]
100d933ec:     	stp	x20, x19, [sp, #0x40]
100d933f0:     	stp	x29, x30, [sp, #0x50]
100d933f4:     	add	x29, sp, #0x50
100d933f8:     	sub	sp, sp, #0x260
100d933fc:     	ldr	w8, [x1, #0xf0]
100d93400:     	str	x4, [sp, #0x160]
100d93404:     	str	x8, [sp]
100d93408:     	cmp	x4, x8
100d9340c:     	b.ne	0x100d9378c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100d93410:     	mov	x25, x6
100d93414:     	mov	x21, x5
100d93418:     	mov	x23, x4
100d9341c:     	mov	x22, x2
100d93420:     	mov	x20, x1
100d93424:     	mov	x19, x0
100d93428:     	ldp	x24, x10, [x29, #0x18]
100d9342c:     	cbz	x4, 0x100d934f0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x114>
100d93430:     	mov	x11, #0x0               ; =0
100d93434:     	lsl	x9, x23, #2
100d93438:     	mov	w12, #0x1               ; =1
100d9343c:     	mov	x13, x9
100d93440:     	mov	x14, x3
100d93444:     	ldr	w15, [x14], #0x4
100d93448:     	cmp	w15, w8
100d9344c:     	b.hs	0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d93450:     	lsr	x16, x11, x15
100d93454:     	tbnz	w16, #0x0, 0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d93458:     	lsl	x15, x12, x15
100d9345c:     	orr	x11, x15, x11
100d93460:     	subs	x13, x13, #0x4
100d93464:     	b.ne	0x100d93444 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x68>
100d93468:     	str	x7, [sp, #0x160]
100d9346c:     	str	x23, [sp]
100d93470:     	cmp	x7, x23
100d93474:     	b.ne	0x100d9378c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100d93478:     	mov	x11, #0x0               ; =0
100d9347c:     	mov	w12, #0x1               ; =1
100d93480:     	mov	x13, x9
100d93484:     	mov	x14, x25
100d93488:     	ldr	w15, [x14], #0x4
100d9348c:     	cmp	w15, w8
100d93490:     	b.hs	0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d93494:     	lsr	x16, x11, x15
100d93498:     	tbnz	w16, #0x0, 0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d9349c:     	lsl	x15, x12, x15
100d934a0:     	orr	x11, x15, x11
100d934a4:     	subs	x13, x13, #0x4
100d934a8:     	b.ne	0x100d93488 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xac>
100d934ac:     	str	x10, [sp, #0x160]
100d934b0:     	str	x23, [sp]
100d934b4:     	cmp	x10, x23
100d934b8:     	b.ne	0x100d9378c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100d934bc:     	mov	x10, #0x0               ; =0
100d934c0:     	mov	w11, #0x1               ; =1
100d934c4:     	mov	x12, x24
100d934c8:     	ldr	w13, [x12], #0x4
100d934cc:     	cmp	w13, w8
100d934d0:     	b.hs	0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d934d4:     	lsr	x14, x10, x13
100d934d8:     	tbnz	w14, #0x0, 0x100d93774 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x398>
100d934dc:     	lsl	x13, x11, x13
100d934e0:     	orr	x10, x13, x10
100d934e4:     	subs	x9, x9, #0x4
100d934e8:     	b.ne	0x100d934c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xec>
100d934ec:     	b	0x100d93508 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x12c>
100d934f0:     	str	x7, [sp, #0x160]
100d934f4:     	str	xzr, [sp]
100d934f8:     	cbnz	x7, 0x100d9378c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100d934fc:     	str	x10, [sp, #0x160]
100d93500:     	str	xzr, [sp]
100d93504:     	cbnz	x10, 0x100d9378c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3b0>
100d93508:     	ldr	x26, [x29, #0x10]
100d9350c:     	lsr	x8, x26, x8
100d93510:     	str	x8, [sp]
100d93514:     	cbnz	x8, 0x100d937b0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d4>
100d93518:     	sub	x0, x29, #0xf0
100d9351c:     	mov	x1, x3
100d93520:     	mov	x2, x23
100d93524:     	mov	x3, x24
100d93528:     	mov	x4, x23
100d9352c:     	bl	0x100d711f8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d93530:     	mov	x0, sp
100d93534:     	mov	x1, x25
100d93538:     	mov	x2, x23
100d9353c:     	mov	x3, x24
100d93540:     	mov	x4, x23
100d93544:     	bl	0x100d711f8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d93548:     	ldp	q0, q1, [x29, #-0xf0]
100d9354c:     	stp	q0, q1, [sp, #0x160]
100d93550:     	ldur	q0, [x29, #-0xd0]
100d93554:     	ldp	q1, q2, [sp]
100d93558:     	stp	q0, q1, [sp, #0x180]
100d9355c:     	ldr	q0, [sp, #0x20]
100d93560:     	stp	q2, q0, [sp, #0x1a0]
100d93564:     	mov	w8, #0x4                ; =4
100d93568:     	stp	xzr, x8, [sp]
100d9356c:     	str	xzr, [sp, #0x10]
100d93570:     	cbz	x26, 0x100d935f8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x21c>
100d93574:     	mov	x27, #0x0               ; =0
100d93578:     	mov	w8, #0x4                ; =4
100d9357c:     	b	0x100d935a4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1c8>
100d93580:     	ldr	x8, [sp, #0x8]
100d93584:     	rbit	x9, x26
100d93588:     	clz	x9, x9
100d9358c:     	str	w9, [x8, x27, lsl #2]
100d93590:     	add	x27, x27, #0x1
100d93594:     	str	x27, [sp, #0x10]
100d93598:     	sub	x9, x26, #0x1
100d9359c:     	ands	x26, x9, x26
100d935a0:     	b.eq	0x100d935bc <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1e0>
100d935a4:     	ldr	x9, [sp]
100d935a8:     	cmp	x27, x9
100d935ac:     	b.ne	0x100d93584 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a8>
100d935b0:     	mov	x0, sp
100d935b4:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d935b8:     	b	0x100d93580 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a4>
100d935bc:     	ldp	x26, x25, [sp]
100d935c0:     	cbz	x27, 0x100d93604 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x228>
100d935c4:     	mov	x9, #0x0                ; =0
100d935c8:     	mov	x8, #0x0                ; =0
100d935cc:     	mov	w10, #0x1               ; =1
100d935d0:     	ldr	w0, [x25, x9, lsl #2]
100d935d4:     	cmp	x23, x0
100d935d8:     	b.ls	0x100d937d8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3fc>
100d935dc:     	ldr	w11, [x24, x0, lsl #2]
100d935e0:     	lsl	x11, x10, x11
100d935e4:     	orr	x8, x11, x8
100d935e8:     	add	x9, x9, #0x1
100d935ec:     	cmp	x27, x9
100d935f0:     	b.ne	0x100d935d0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1f4>
100d935f4:     	b	0x100d93608 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100d935f8:     	mov	x8, #0x0                ; =0
100d935fc:     	mov	w25, #0x4               ; =4
100d93600:     	b	0x100d93608 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100d93604:     	mov	x8, #0x0                ; =0
100d93608:     	mov	x9, sp
100d9360c:     	add	x23, x9, #0xc8
100d93610:     	movi.2d	v0, #0000000000000000
100d93614:     	stp	q0, q0, [x23, #0x60]
100d93618:     	mov	x9, sp
100d9361c:     	stp	q0, q0, [x23, #0x40]
100d93620:     	stur	q0, [x9, #0xf8]
100d93624:     	stur	q0, [x9, #0xe8]
100d93628:     	stur	q0, [x9, #0xd8]
100d9362c:     	stur	q0, [x9, #0xc8]
100d93630:     	ldrb	w9, [x20, #0xf6]
100d93634:     	ldrb	w10, [x20, #0xf7]
100d93638:     	stp	xzr, xzr, [sp, #0xb0]
100d9363c:     	ldp	q0, q1, [sp, #0x160]
100d93640:     	ldp	q2, q3, [sp, #0x180]
100d93644:     	stp	q1, q2, [sp, #0x20]
100d93648:     	ldp	q1, q2, [sp, #0x1a0]
100d9364c:     	stp	q1, q2, [sp, #0x50]
100d93650:     	str	q3, [sp, #0x40]
100d93654:     	str	xzr, [sp, #0x148]
100d93658:     	str	x8, [sp, #0xc0]
100d9365c:     	adrp	x8, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100d93660:     	add	x8, x8, #0x7c8
100d93664:     	stp	x8, xzr, [sp, #0x70]
100d93668:     	stp	xzr, xzr, [sp, #0x80]
100d9366c:     	strb	w9, [sp, #0x150]
100d93670:     	ldr	q1, [x20]
100d93674:     	stp	q1, q0, [sp]
100d93678:     	strb	w10, [sp, #0x151]
100d9367c:     	mov	w8, #0x8                ; =8
100d93680:     	stp	x8, xzr, [sp, #0x90]
100d93684:     	stp	xzr, x8, [sp, #0xa0]
100d93688:     	cbz	x26, 0x100d93694 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x2b8>
100d9368c:     	mov	x0, x25
100d93690:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93694:     	stur	w22, [x29, #-0xe0]
100d93698:     	stp	xzr, xzr, [x29, #-0xf0]
100d9369c:     	sub	x0, x29, #0x88
100d936a0:     	sub	x1, x29, #0xf0
100d936a4:     	mov	x2, x20
100d936a8:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d936ac:     	add	x22, sp, #0x160
100d936b0:     	ldur	x8, [x29, #-0x78]
100d936b4:     	ldur	q0, [x22, #0xc8]
100d936b8:     	stur	q0, [x29, #-0x70]
100d936bc:     	stur	x8, [x29, #-0x60]
100d936c0:     	str	x8, [sp, #0x170]
100d936c4:     	str	q0, [sp, #0x160]
100d936c8:     	stur	w21, [x29, #-0xe0]
100d936cc:     	stp	xzr, xzr, [x29, #-0xf0]
100d936d0:     	sub	x0, x29, #0x88
100d936d4:     	sub	x1, x29, #0xf0
100d936d8:     	mov	x2, x20
100d936dc:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d936e0:     	ldur	x8, [x29, #-0x78]
100d936e4:     	ldur	q0, [x22, #0xc8]
100d936e8:     	stur	q0, [x22, #0x18]
100d936ec:     	str	x8, [sp, #0x188]
100d936f0:     	ldp	q0, q1, [sp, #0x160]
100d936f4:     	ldr	q2, [sp, #0x180]
100d936f8:     	stp	q1, q2, [x29, #-0xe0]
100d936fc:     	stur	q0, [x29, #-0xf0]
100d93700:     	mov	x0, sp
100d93704:     	sub	x2, x29, #0xf0
100d93708:     	mov	x1, x20
100d9370c:     	bl	0x1007b2240 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
100d93710:     	ldp	q0, q1, [x23, #0x40]
100d93714:     	stur	q0, [x19, #0x48]
100d93718:     	stur	q1, [x19, #0x58]
100d9371c:     	ldp	q0, q1, [x23, #0x60]
100d93720:     	stur	q0, [x19, #0x68]
100d93724:     	stur	q1, [x19, #0x78]
100d93728:     	ldp	q0, q1, [x23]
100d9372c:     	stur	q0, [x19, #0x8]
100d93730:     	stur	q1, [x19, #0x18]
100d93734:     	ldp	q0, q1, [x23, #0x20]
100d93738:     	stur	q0, [x19, #0x28]
100d9373c:     	ldr	x8, [x23, #0x80]
100d93740:     	str	x8, [x19, #0x88]
100d93744:     	stur	q1, [x19, #0x38]
100d93748:     	str	w0, [x19]
100d9374c:     	mov	x0, sp
100d93750:     	bl	0x100828b64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d93754:     	add	sp, sp, #0x260
100d93758:     	ldp	x29, x30, [sp, #0x50]
100d9375c:     	ldp	x20, x19, [sp, #0x40]
100d93760:     	ldp	x22, x21, [sp, #0x30]
100d93764:     	ldp	x24, x23, [sp, #0x20]
100d93768:     	ldp	x26, x25, [sp, #0x10]
100d9376c:     	ldp	x28, x27, [sp], #0x60
100d93770:     	ret
100d93774:     	adrp	x0, 0x101482000 <dyld_stub_binder+0x101482000>
100d93778:     	add	x0, x0, #0x407
100d9377c:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
100d93780:     	add	x2, x2, #0x6a8
100d93784:     	mov	w1, #0x2f               ; =47
100d93788:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d9378c:     	adrp	x3, 0x101482000 <dyld_stub_binder+0x101482000>
100d93790:     	add	x3, x3, #0x3f1
100d93794:     	adrp	x5, 0x101644000 <dyld_stub_binder+0x101644000>
100d93798:     	add	x5, x5, #0x690
100d9379c:     	add	x1, sp, #0x160
100d937a0:     	mov	x2, sp
100d937a4:     	mov	w0, #0x0                ; =0
100d937a8:     	mov	w4, #0x2d               ; =45
100d937ac:     	bl	0x1013ba230 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d937b0:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100d937b4:     	add	x2, x2, #0x78
100d937b8:     	adrp	x3, 0x101482000 <dyld_stub_binder+0x101482000>
100d937bc:     	add	x3, x3, #0x71c
100d937c0:     	adrp	x5, 0x101644000 <dyld_stub_binder+0x101644000>
100d937c4:     	add	x5, x5, #0xd18
100d937c8:     	mov	x1, sp
100d937cc:     	mov	w0, #0x0                ; =0
100d937d0:     	mov	w4, #0x57               ; =87
100d937d4:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d937d8:     	adrp	x2, 0x101607000 <dyld_stub_binder+0x101607000>
100d937dc:     	add	x2, x2, #0x168
100d937e0:     	mov	x1, x23
100d937e4:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d937e8:     	brk	#0x1
100d937ec:     	mov	x19, x0
100d937f0:     	sub	x0, x29, #0xf0
100d937f4:     	bl	0x10082764c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d937f8:     	mov	x0, x19
100d937fc:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93800:     	mov	x19, x0
100d93804:     	mov	x0, sp
100d93808:     	bl	0x100828b64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d9380c:     	mov	x0, x19
100d93810:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93814:     	mov	x19, x0
100d93818:     	ldr	x8, [sp]
100d9381c:     	cbz	x8, 0x100d93828 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x44c>
100d93820:     	ldr	x0, [sp, #0x8]
100d93824:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93828:     	add	x0, sp, #0x160
100d9382c:     	bl	0x10080cfdc <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d93830:     	mov	x0, x19
100d93834:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93838:     	mov	x19, x0
100d9383c:     	add	x0, sp, #0x160
100d93840:     	bl	0x10080cfdc <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d93844:     	cbnz	x26, 0x100d93850 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x474>
100d93848:     	mov	x0, x19
100d9384c:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93850:     	mov	x0, x25
100d93854:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93858:     	mov	x0, x19
100d9385c:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
