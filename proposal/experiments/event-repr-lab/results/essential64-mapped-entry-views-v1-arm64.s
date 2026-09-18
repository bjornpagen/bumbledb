
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d6739c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_>:
100d6739c:     	stp	x28, x27, [sp, #-0x60]!
100d673a0:     	stp	x26, x25, [sp, #0x10]
100d673a4:     	stp	x24, x23, [sp, #0x20]
100d673a8:     	stp	x22, x21, [sp, #0x30]
100d673ac:     	stp	x20, x19, [sp, #0x40]
100d673b0:     	stp	x29, x30, [sp, #0x50]
100d673b4:     	add	x29, sp, #0x50
100d673b8:     	sub	sp, sp, #0x1d0
100d673bc:     	ldr	w8, [x0, #0xe0]
100d673c0:     	str	x3, [sp, #0xd0]
100d673c4:     	str	x8, [sp]
100d673c8:     	cmp	x3, x8
100d673cc:     	b.ne	0x100d67744 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3a8>
100d673d0:     	mov	x24, x7
100d673d4:     	mov	x25, x5
100d673d8:     	mov	x20, x4
100d673dc:     	mov	x22, x3
100d673e0:     	mov	x21, x1
100d673e4:     	mov	x19, x0
100d673e8:     	ldp	x23, x10, [x29, #0x10]
100d673ec:     	cbz	x3, 0x100d674b0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x114>
100d673f0:     	mov	x11, #0x0               ; =0
100d673f4:     	lsl	x9, x22, #2
100d673f8:     	mov	w12, #0x1               ; =1
100d673fc:     	mov	x13, x9
100d67400:     	mov	x14, x2
100d67404:     	ldr	w15, [x14], #0x4
100d67408:     	cmp	w15, w8
100d6740c:     	b.hs	0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d67410:     	lsr	x16, x11, x15
100d67414:     	tbnz	w16, #0x0, 0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d67418:     	lsl	x15, x12, x15
100d6741c:     	orr	x11, x15, x11
100d67420:     	subs	x13, x13, #0x4
100d67424:     	b.ne	0x100d67404 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x68>
100d67428:     	str	x6, [sp, #0xd0]
100d6742c:     	str	x22, [sp]
100d67430:     	cmp	x6, x22
100d67434:     	b.ne	0x100d67744 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3a8>
100d67438:     	mov	x11, #0x0               ; =0
100d6743c:     	mov	w12, #0x1               ; =1
100d67440:     	mov	x13, x9
100d67444:     	mov	x14, x25
100d67448:     	ldr	w15, [x14], #0x4
100d6744c:     	cmp	w15, w8
100d67450:     	b.hs	0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d67454:     	lsr	x16, x11, x15
100d67458:     	tbnz	w16, #0x0, 0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d6745c:     	lsl	x15, x12, x15
100d67460:     	orr	x11, x15, x11
100d67464:     	subs	x13, x13, #0x4
100d67468:     	b.ne	0x100d67448 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0xac>
100d6746c:     	str	x10, [sp, #0xd0]
100d67470:     	str	x22, [sp]
100d67474:     	cmp	x10, x22
100d67478:     	b.ne	0x100d67744 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3a8>
100d6747c:     	mov	x10, #0x0               ; =0
100d67480:     	mov	w11, #0x1               ; =1
100d67484:     	mov	x12, x23
100d67488:     	ldr	w13, [x12], #0x4
100d6748c:     	cmp	w13, w8
100d67490:     	b.hs	0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d67494:     	lsr	x14, x10, x13
100d67498:     	tbnz	w14, #0x0, 0x100d6772c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x390>
100d6749c:     	lsl	x13, x11, x13
100d674a0:     	orr	x10, x13, x10
100d674a4:     	subs	x9, x9, #0x4
100d674a8:     	b.ne	0x100d67488 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0xec>
100d674ac:     	b	0x100d674c8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x12c>
100d674b0:     	str	x6, [sp, #0xd0]
100d674b4:     	str	xzr, [sp]
100d674b8:     	cbnz	x6, 0x100d67744 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3a8>
100d674bc:     	str	x10, [sp, #0xd0]
100d674c0:     	str	xzr, [sp]
100d674c4:     	cbnz	x10, 0x100d67744 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3a8>
100d674c8:     	lsr	x8, x24, x8
100d674cc:     	str	x8, [sp]
100d674d0:     	cbnz	x8, 0x100d67768 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3cc>
100d674d4:     	sub	x0, x29, #0xf0
100d674d8:     	mov	x1, x2
100d674dc:     	mov	x2, x22
100d674e0:     	mov	x3, x23
100d674e4:     	mov	x4, x22
100d674e8:     	bl	0x100d91eb4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB4_3Map7compose>
100d674ec:     	mov	x0, sp
100d674f0:     	mov	x1, x25
100d674f4:     	mov	x2, x22
100d674f8:     	mov	x3, x23
100d674fc:     	mov	x4, x22
100d67500:     	bl	0x100d91eb4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB4_3Map7compose>
100d67504:     	ldp	q0, q1, [x29, #-0xf0]
100d67508:     	stp	q0, q1, [sp, #0xd0]
100d6750c:     	ldur	q0, [x29, #-0xd0]
100d67510:     	ldp	q1, q2, [sp]
100d67514:     	stp	q0, q1, [sp, #0xf0]
100d67518:     	ldr	q0, [sp, #0x20]
100d6751c:     	stp	q2, q0, [sp, #0x110]
100d67520:     	mov	w8, #0x4                ; =4
100d67524:     	stp	xzr, x8, [sp]
100d67528:     	str	xzr, [sp, #0x10]
100d6752c:     	cbz	x24, 0x100d675b4 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x218>
100d67530:     	mov	x26, #0x0               ; =0
100d67534:     	mov	w8, #0x4                ; =4
100d67538:     	b	0x100d67560 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x1c4>
100d6753c:     	ldr	x8, [sp, #0x8]
100d67540:     	rbit	x9, x24
100d67544:     	clz	x9, x9
100d67548:     	str	w9, [x8, x26, lsl #2]
100d6754c:     	add	x26, x26, #0x1
100d67550:     	str	x26, [sp, #0x10]
100d67554:     	sub	x9, x24, #0x1
100d67558:     	ands	x24, x9, x24
100d6755c:     	b.eq	0x100d67578 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x1dc>
100d67560:     	ldr	x9, [sp]
100d67564:     	cmp	x26, x9
100d67568:     	b.ne	0x100d67540 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x1a4>
100d6756c:     	mov	x0, sp
100d67570:     	bl	0x1013af1dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d67574:     	b	0x100d6753c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x1a0>
100d67578:     	ldp	x25, x24, [sp]
100d6757c:     	cbz	x26, 0x100d675c4 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x228>
100d67580:     	mov	x9, #0x0                ; =0
100d67584:     	mov	x8, #0x0                ; =0
100d67588:     	mov	w10, #0x1               ; =1
100d6758c:     	ldr	w0, [x24, x9, lsl #2]
100d67590:     	cmp	x22, x0
100d67594:     	b.ls	0x100d67790 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x3f4>
100d67598:     	ldr	w11, [x23, x0, lsl #2]
100d6759c:     	lsl	x11, x10, x11
100d675a0:     	orr	x8, x11, x8
100d675a4:     	add	x9, x9, #0x1
100d675a8:     	cmp	x26, x9
100d675ac:     	b.ne	0x100d6758c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x1f0>
100d675b0:     	b	0x100d675c8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x22c>
100d675b4:     	mov	x25, #0x0               ; =0
100d675b8:     	mov	x8, #0x0                ; =0
100d675bc:     	mov	w24, #0x4               ; =4
100d675c0:     	b	0x100d675c8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x22c>
100d675c4:     	mov	x8, #0x0                ; =0
100d675c8:     	mov	x22, sp
100d675cc:     	movi.2d	v0, #0000000000000000
100d675d0:     	stur	q0, [x22, #0xb8]
100d675d4:     	stur	q0, [x22, #0xa8]
100d675d8:     	stur	q0, [x22, #0x98]
100d675dc:     	stur	q0, [x22, #0x88]
100d675e0:     	ldp	q0, q1, [sp, #0x110]
100d675e4:     	stp	q0, q1, [sp, #0x40]
100d675e8:     	ldp	q0, q1, [sp, #0xd0]
100d675ec:     	stp	q0, q1, [sp]
100d675f0:     	ldp	q0, q1, [sp, #0xf0]
100d675f4:     	stp	q0, q1, [sp, #0x20]
100d675f8:     	str	xzr, [sp, #0xc8]
100d675fc:     	stp	xzr, x8, [sp, #0x78]
100d67600:     	adrp	x8, 0x101515000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x208f>
100d67604:     	add	x8, x8, #0x8e0
100d67608:     	stp	x8, xzr, [sp, #0x60]
100d6760c:     	str	xzr, [sp, #0x70]
100d67610:     	cbz	x25, 0x100d6761c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x280>
100d67614:     	mov	x0, x24
100d67618:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d6761c:     	stur	w21, [x29, #-0xe0]
100d67620:     	stp	xzr, xzr, [x29, #-0xf0]
100d67624:     	sub	x0, x29, #0x88
100d67628:     	sub	x1, x29, #0xf0
100d6762c:     	mov	x2, x19
100d67630:     	bl	0x100004ff8 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB3_10Restricted9normalizeKm1_EB9_>
100d67634:     	sub	x21, x29, #0x88
100d67638:     	ldur	x8, [x29, #-0x78]
100d6763c:     	ldr	q0, [x21]
100d67640:     	stur	q0, [x29, #-0x70]
100d67644:     	stur	x8, [x29, #-0x60]
100d67648:     	str	x8, [sp, #0xe0]
100d6764c:     	str	q0, [sp, #0xd0]
100d67650:     	stur	w20, [x29, #-0xe0]
100d67654:     	stp	xzr, xzr, [x29, #-0xf0]
100d67658:     	sub	x0, x29, #0x88
100d6765c:     	sub	x1, x29, #0xf0
100d67660:     	mov	x2, x19
100d67664:     	bl	0x100004ff8 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB3_10Restricted9normalizeKm1_EB9_>
100d67668:     	ldur	x8, [x29, #-0x78]
100d6766c:     	ldr	q0, [x21]
100d67670:     	stur	q0, [x22, #0xe8]
100d67674:     	str	x8, [sp, #0xf8]
100d67678:     	ldp	q0, q1, [sp, #0xd0]
100d6767c:     	ldr	q2, [sp, #0xf0]
100d67680:     	stp	q1, q2, [x29, #-0xe0]
100d67684:     	stur	q0, [x29, #-0xf0]
100d67688:     	mov	x0, sp
100d6768c:     	sub	x2, x29, #0xf0
100d67690:     	mov	x1, x19
100d67694:     	bl	0x100751f38 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
100d67698:     	mov	x19, x0
100d6769c:     	ldr	x8, [sp]
100d676a0:     	cbz	x8, 0x100d676ac <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x310>
100d676a4:     	ldr	x0, [sp, #0x8]
100d676a8:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d676ac:     	ldr	x8, [sp, #0x18]
100d676b0:     	cbz	x8, 0x100d676bc <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x320>
100d676b4:     	ldr	x0, [sp, #0x20]
100d676b8:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d676bc:     	ldr	x8, [sp, #0x30]
100d676c0:     	cbz	x8, 0x100d676cc <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x330>
100d676c4:     	ldr	x0, [sp, #0x38]
100d676c8:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d676cc:     	ldr	x8, [sp, #0x48]
100d676d0:     	cbz	x8, 0x100d676dc <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x340>
100d676d4:     	ldr	x0, [sp, #0x50]
100d676d8:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d676dc:     	ldr	x9, [sp, #0x68]
100d676e0:     	cbz	x9, 0x100d67708 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x36c>
100d676e4:     	lsl	x8, x9, #6
100d676e8:     	sub	x8, x8, x9, lsl #3
100d676ec:     	add	x9, x8, x9
100d676f0:     	cmn	x9, #0x41
100d676f4:     	b.eq	0x100d67708 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x36c>
100d676f8:     	ldr	x9, [sp, #0x60]
100d676fc:     	sub	x8, x9, x8
100d67700:     	sub	x0, x8, #0x38
100d67704:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67708:     	mov	x0, x19
100d6770c:     	add	sp, sp, #0x1d0
100d67710:     	ldp	x29, x30, [sp, #0x50]
100d67714:     	ldp	x20, x19, [sp, #0x40]
100d67718:     	ldp	x22, x21, [sp, #0x30]
100d6771c:     	ldp	x24, x23, [sp, #0x20]
100d67720:     	ldp	x26, x25, [sp, #0x10]
100d67724:     	ldp	x28, x27, [sp], #0x60
100d67728:     	ret
100d6772c:     	adrp	x0, 0x101475000 <dyld_stub_binder+0x101475000>
100d67730:     	add	x0, x0, #0xc47
100d67734:     	adrp	x2, 0x101638000 <dyld_stub_binder+0x101638000>
100d67738:     	add	x2, x2, #0xcd0
100d6773c:     	mov	w1, #0x2f               ; =47
100d67740:     	bl	0x1013ae274 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d67744:     	adrp	x3, 0x101475000 <dyld_stub_binder+0x101475000>
100d67748:     	add	x3, x3, #0xc31
100d6774c:     	adrp	x5, 0x101638000 <dyld_stub_binder+0x101638000>
100d67750:     	add	x5, x5, #0xcb8
100d67754:     	add	x1, sp, #0xd0
100d67758:     	mov	x2, sp
100d6775c:     	mov	w0, #0x0                ; =0
100d67760:     	mov	w4, #0x2d               ; =45
100d67764:     	bl	0x1013ae2b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d67768:     	adrp	x2, 0x101515000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x208f>
100d6776c:     	add	x2, x2, #0x190
100d67770:     	adrp	x3, 0x101475000 <dyld_stub_binder+0x101475000>
100d67774:     	add	x3, x3, #0x6a1
100d67778:     	adrp	x5, 0x101638000 <dyld_stub_binder+0x101638000>
100d6777c:     	add	x5, x5, #0x248
100d67780:     	mov	x1, sp
100d67784:     	mov	w0, #0x0                ; =0
100d67788:     	mov	w4, #0x57               ; =87
100d6778c:     	bl	0x1013ae2e0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d67790:     	adrp	x2, 0x1015fa000 <dyld_stub_binder+0x1015fa000>
100d67794:     	add	x2, x2, #0xf58
100d67798:     	mov	x1, x22
100d6779c:     	bl	0x1013ae3dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d677a0:     	brk	#0x1
100d677a4:     	mov	x19, x0
100d677a8:     	sub	x0, x29, #0xf0
100d677ac:     	bl	0x10082480c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d677b0:     	mov	x0, x19
100d677b4:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d677b8:     	mov	x19, x0
100d677bc:     	mov	x0, sp
100d677c0:     	bl	0x100825d24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d677c4:     	mov	x0, x19
100d677c8:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d677cc:     	mov	x19, x0
100d677d0:     	ldr	x8, [sp]
100d677d4:     	cbz	x8, 0x100d677e0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x444>
100d677d8:     	ldr	x0, [sp, #0x8]
100d677dc:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d677e0:     	add	x0, sp, #0xd0
100d677e4:     	bl	0x10080a19c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d677e8:     	mov	x0, x19
100d677ec:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d677f0:     	mov	x19, x0
100d677f4:     	add	x0, sp, #0xd0
100d677f8:     	bl	0x10080a19c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d677fc:     	cbnz	x25, 0x100d67808 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E14mapped_productBb_+0x46c>
100d67800:     	mov	x0, x19
100d67804:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67808:     	mov	x0, x24
100d6780c:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67810:     	mov	x0, x19
100d67814:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
