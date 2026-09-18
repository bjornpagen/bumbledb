
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bcb378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>:
100bcb378:     	sub	sp, sp, #0x60
100bcb37c:     	stp	x26, x25, [sp, #0x10]
100bcb380:     	stp	x24, x23, [sp, #0x20]
100bcb384:     	stp	x22, x21, [sp, #0x30]
100bcb388:     	stp	x20, x19, [sp, #0x40]
100bcb38c:     	stp	x29, x30, [sp, #0x50]
100bcb390:     	add	x29, sp, #0x50
100bcb394:     	cbz	w1, 0x100bcb53c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1c4>
100bcb398:     	ldr	x8, [x4, #0x18]
100bcb39c:     	cbz	x8, 0x100bcb46c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bcb3a0:     	mov	x8, #0x0                ; =0
100bcb3a4:     	mov	w9, w1
100bcb3a8:     	mov	x10, #0xa9c5            ; =43461
100bcb3ac:     	movk	x10, #0x2e62, lsl #16
100bcb3b0:     	movk	x10, #0x7aea, lsl #32
100bcb3b4:     	movk	x10, #0xf135, lsl #48
100bcb3b8:     	mul	x9, x9, x10
100bcb3bc:     	add	x9, x9, w2, uxtw
100bcb3c0:     	mul	x9, x9, x10
100bcb3c4:     	add	x9, x9, w3, uxtw
100bcb3c8:     	mul	x9, x9, x10
100bcb3cc:     	ror	x11, x9, #0x2c
100bcb3d0:     	lsr	x12, x11, #57
100bcb3d4:     	ldp	x10, x9, [x4]
100bcb3d8:     	dup.8b	v0, w12
100bcb3dc:     	movi.2d	v1, #0xffffffffffffffff
100bcb3e0:     	and	x11, x11, x9
100bcb3e4:     	ldr	d2, [x10, x11]
100bcb3e8:     	cmeq.8b	v3, v2, v0
100bcb3ec:     	fmov	x12, d3
100bcb3f0:     	ands	x12, x12, #0x8080808080808080
100bcb3f4:     	b.eq	0x100bcb43c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bcb3f8:     	rbit	x13, x12
100bcb3fc:     	clz	x13, x13
100bcb400:     	add	x13, x11, x13, lsr #3
100bcb404:     	and	x13, x13, x9
100bcb408:     	sub	x13, x10, x13, lsl #4
100bcb40c:     	ldur	w14, [x13, #-0x10]
100bcb410:     	cmp	w1, w14
100bcb414:     	b.ne	0x100bcb430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bcb418:     	ldur	w14, [x13, #-0xc]
100bcb41c:     	cmp	w2, w14
100bcb420:     	b.ne	0x100bcb430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bcb424:     	ldur	w14, [x13, #-0x8]
100bcb428:     	cmp	w3, w14
100bcb42c:     	b.eq	0x100bcb544 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1cc>
100bcb430:     	sub	x13, x12, #0x2
100bcb434:     	ands	x12, x13, x12
100bcb438:     	b.ne	0x100bcb3f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bcb43c:     	cmeq.8b	v2, v2, v1
100bcb440:     	fmov	x12, d2
100bcb444:     	cbnz	x12, 0x100bcb46c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bcb448:     	add	x8, x8, #0x8
100bcb44c:     	add	x11, x11, x8
100bcb450:     	and	x11, x11, x9
100bcb454:     	ldr	d2, [x10, x11]
100bcb458:     	cmeq.8b	v3, v2, v0
100bcb45c:     	fmov	x12, d3
100bcb460:     	ands	x12, x12, #0x8080808080808080
100bcb464:     	b.ne	0x100bcb3f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bcb468:     	b	0x100bcb43c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bcb46c:     	tbnz	w1, #0x1, 0x100bcb54c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1d4>
100bcb470:     	ldr	x10, [x0, #0xc8]
100bcb474:     	tbnz	w2, #0x1, 0x100bcb56c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1f4>
100bcb478:     	ldr	x8, [x0, #0xc8]
100bcb47c:     	cmp	x8, x10
100bcb480:     	csel	x10, x8, x10, lo
100bcb484:     	tbnz	w3, #0x1, 0x100bcb594 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x21c>
100bcb488:     	ldr	x8, [x0, #0xc8]
100bcb48c:     	cmp	x8, x10
100bcb490:     	csel	x21, x8, x10, lo
100bcb494:     	cmp	x21, x8
100bcb498:     	b.ne	0x100bcb5c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x24c>
100bcb49c:     	lsr	w9, w1, #2
100bcb4a0:     	ldr	x8, [x0, #0x58]
100bcb4a4:     	cmp	x8, x9
100bcb4a8:     	b.ls	0x100bcb7e8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x470>
100bcb4ac:     	ldr	x12, [x0, #0xd8]
100bcb4b0:     	tst	w1, #0x1
100bcb4b4:     	csel	x13, xzr, x12, eq
100bcb4b8:     	lsr	w10, w2, #2
100bcb4bc:     	cmp	x8, x10
100bcb4c0:     	b.ls	0x100bcb7fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x484>
100bcb4c4:     	lsr	w11, w3, #2
100bcb4c8:     	cmp	x8, x11
100bcb4cc:     	b.ls	0x100bcb810 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x498>
100bcb4d0:     	ldr	x8, [x0, #0x50]
100bcb4d4:     	ldr	x9, [x8, x9, lsl #3]
100bcb4d8:     	ldr	x10, [x8, x10, lsl #3]
100bcb4dc:     	eor	x9, x13, x9
100bcb4e0:     	tst	w2, #0x1
100bcb4e4:     	csel	x13, xzr, x12, eq
100bcb4e8:     	eor	x10, x10, x13
100bcb4ec:     	ldr	x8, [x8, x11, lsl #3]
100bcb4f0:     	tst	w3, #0x1
100bcb4f4:     	csel	x11, xzr, x12, eq
100bcb4f8:     	eor	x8, x8, x11
100bcb4fc:     	bic	x11, x9, x10
100bcb500:     	tst	x11, x8
100bcb504:     	mov	w12, #0x2               ; =2
100bcb508:     	csel	w12, wzr, w12, eq
100bcb50c:     	and	x9, x10, x9
100bcb510:     	bics	xzr, x9, x8
100bcb514:     	mov	w10, #0x4               ; =4
100bcb518:     	csel	w10, wzr, w10, eq
100bcb51c:     	tst	x9, x8
100bcb520:     	mov	w9, #0x8                ; =8
100bcb524:     	csel	w9, wzr, w9, eq
100bcb528:     	bics	xzr, x11, x8
100bcb52c:     	cinc	w8, w12, ne
100bcb530:     	orr	w9, w10, w9
100bcb534:     	orr	w20, w8, w9
100bcb538:     	b	0x100bcb79c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x424>
100bcb53c:     	mov	w20, #0x0               ; =0
100bcb540:     	b	0x100bcb7b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bcb544:     	ldurb	w20, [x13, #-0x4]
100bcb548:     	b	0x100bcb7b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bcb54c:     	lsr	w8, w1, #2
100bcb550:     	ldr	x9, [x0, #0x40]
100bcb554:     	cmp	x9, x8
100bcb558:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb55c:     	ldr	x9, [x0, #0x38]
100bcb560:     	lsl	x8, x8, #4
100bcb564:     	ldr	w10, [x9, x8]
100bcb568:     	tbz	w2, #0x1, 0x100bcb478 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x100>
100bcb56c:     	lsr	w8, w2, #2
100bcb570:     	ldr	x9, [x0, #0x40]
100bcb574:     	cmp	x9, x8
100bcb578:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb57c:     	ldr	x9, [x0, #0x38]
100bcb580:     	lsl	x8, x8, #4
100bcb584:     	ldr	w8, [x9, x8]
100bcb588:     	cmp	x8, x10
100bcb58c:     	csel	x10, x8, x10, lo
100bcb590:     	tbz	w3, #0x1, 0x100bcb488 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x110>
100bcb594:     	lsr	w8, w3, #2
100bcb598:     	ldr	x9, [x0, #0x40]
100bcb59c:     	cmp	x9, x8
100bcb5a0:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb5a4:     	ldr	x9, [x0, #0x38]
100bcb5a8:     	lsl	x8, x8, #4
100bcb5ac:     	ldr	w9, [x9, x8]
100bcb5b0:     	ldr	x8, [x0, #0xc8]
100bcb5b4:     	cmp	x9, x10
100bcb5b8:     	csel	x21, x9, x10, lo
100bcb5bc:     	cmp	x21, x8
100bcb5c0:     	b.eq	0x100bcb49c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x124>
100bcb5c4:     	mov	x8, x1
100bcb5c8:     	tbz	w1, #0x1, 0x100bcb600 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bcb5cc:     	lsr	w8, w1, #2
100bcb5d0:     	ldr	x9, [x0, #0x40]
100bcb5d4:     	cmp	x9, x8
100bcb5d8:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb5dc:     	ldr	x9, [x0, #0x38]
100bcb5e0:     	add	x9, x9, x8, lsl #4
100bcb5e4:     	ldr	w10, [x9]
100bcb5e8:     	mov	x8, x1
100bcb5ec:     	cmp	x21, x10
100bcb5f0:     	b.ne	0x100bcb600 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bcb5f4:     	ldr	w8, [x9, #0x4]
100bcb5f8:     	and	w9, w1, #0x1
100bcb5fc:     	eor	w8, w8, w9
100bcb600:     	mov	x9, x2
100bcb604:     	tbz	w2, #0x1, 0x100bcb63c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bcb608:     	lsr	w9, w2, #2
100bcb60c:     	ldr	x10, [x0, #0x40]
100bcb610:     	cmp	x10, x9
100bcb614:     	b.ls	0x100bcb824 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4ac>
100bcb618:     	ldr	x10, [x0, #0x38]
100bcb61c:     	add	x10, x10, x9, lsl #4
100bcb620:     	ldr	w11, [x10]
100bcb624:     	mov	x9, x2
100bcb628:     	cmp	x21, x11
100bcb62c:     	b.ne	0x100bcb63c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bcb630:     	ldr	w9, [x10, #0x4]
100bcb634:     	and	w10, w2, #0x1
100bcb638:     	eor	w9, w9, w10
100bcb63c:     	mov	x22, x1
100bcb640:     	mov	x10, x3
100bcb644:     	tbz	w3, #0x1, 0x100bcb67c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bcb648:     	lsr	w10, w3, #2
100bcb64c:     	ldr	x1, [x0, #0x40]
100bcb650:     	cmp	x1, x10
100bcb654:     	b.ls	0x100bcb838 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4c0>
100bcb658:     	ldr	x11, [x0, #0x38]
100bcb65c:     	add	x11, x11, x10, lsl #4
100bcb660:     	ldr	w12, [x11]
100bcb664:     	mov	x10, x3
100bcb668:     	cmp	x21, x12
100bcb66c:     	b.ne	0x100bcb67c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bcb670:     	ldr	w10, [x11, #0x4]
100bcb674:     	and	w11, w3, #0x1
100bcb678:     	eor	w10, w10, w11
100bcb67c:     	mov	x23, x2
100bcb680:     	mov	x24, x3
100bcb684:     	mov	x25, x0
100bcb688:     	mov	x1, x8
100bcb68c:     	mov	x2, x9
100bcb690:     	mov	x3, x10
100bcb694:     	mov	x19, x4
100bcb698:     	bl	0x100bcb378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bcb69c:     	and	w8, w0, #0xff
100bcb6a0:     	cmp	w8, #0xf
100bcb6a4:     	b.ne	0x100bcb6b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x340>
100bcb6a8:     	mov	w20, #0xf               ; =15
100bcb6ac:     	mov	x4, x19
100bcb6b0:     	mov	x3, x24
100bcb6b4:     	b	0x100bcb794 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x41c>
100bcb6b8:     	mov	x20, x0
100bcb6bc:     	mov	x1, x22
100bcb6c0:     	mov	x10, x24
100bcb6c4:     	mov	x11, x23
100bcb6c8:     	mov	x0, x25
100bcb6cc:     	tbz	w22, #0x1, 0x100bcb708 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bcb6d0:     	mov	x9, x22
100bcb6d4:     	lsr	w8, w22, #2
100bcb6d8:     	ldr	x1, [x0, #0x40]
100bcb6dc:     	cmp	x1, x8
100bcb6e0:     	b.ls	0x100bcb848 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4d0>
100bcb6e4:     	ldr	x12, [x0, #0x38]
100bcb6e8:     	add	x8, x12, x8, lsl #4
100bcb6ec:     	ldr	w12, [x8]
100bcb6f0:     	mov	x1, x9
100bcb6f4:     	cmp	x21, x12
100bcb6f8:     	b.ne	0x100bcb708 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bcb6fc:     	ldr	w8, [x8, #0x8]
100bcb700:     	and	w9, w9, #0x1
100bcb704:     	eor	w1, w8, w9
100bcb708:     	mov	x2, x11
100bcb70c:     	tbz	w11, #0x1, 0x100bcb744 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bcb710:     	lsr	w8, w11, #2
100bcb714:     	ldr	x9, [x0, #0x40]
100bcb718:     	cmp	x9, x8
100bcb71c:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb720:     	ldr	x9, [x0, #0x38]
100bcb724:     	add	x8, x9, x8, lsl #4
100bcb728:     	ldr	w9, [x8]
100bcb72c:     	mov	x2, x11
100bcb730:     	cmp	x21, x9
100bcb734:     	b.ne	0x100bcb744 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bcb738:     	ldr	w8, [x8, #0x8]
100bcb73c:     	and	w9, w11, #0x1
100bcb740:     	eor	w2, w8, w9
100bcb744:     	mov	x3, x10
100bcb748:     	tbz	w10, #0x1, 0x100bcb780 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bcb74c:     	lsr	w8, w10, #2
100bcb750:     	ldr	x9, [x0, #0x40]
100bcb754:     	cmp	x9, x8
100bcb758:     	b.ls	0x100bcb7d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bcb75c:     	ldr	x9, [x0, #0x38]
100bcb760:     	add	x8, x9, x8, lsl #4
100bcb764:     	ldr	w9, [x8]
100bcb768:     	mov	x3, x10
100bcb76c:     	cmp	x21, x9
100bcb770:     	b.ne	0x100bcb780 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bcb774:     	ldr	w8, [x8, #0x8]
100bcb778:     	and	w9, w10, #0x1
100bcb77c:     	eor	w3, w8, w9
100bcb780:     	mov	x4, x19
100bcb784:     	bl	0x100bcb378 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bcb788:     	mov	x3, x24
100bcb78c:     	mov	x4, x19
100bcb790:     	orr	w20, w0, w20
100bcb794:     	mov	x2, x23
100bcb798:     	mov	x1, x22
100bcb79c:     	stp	w1, w2, [sp, #0x4]
100bcb7a0:     	str	w3, [sp, #0xc]
100bcb7a4:     	add	x1, sp, #0x4
100bcb7a8:     	mov	x0, x4
100bcb7ac:     	mov	x2, x20
100bcb7b0:     	bl	0x100c2f0ac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bcb7b4:     	mov	x0, x20
100bcb7b8:     	ldp	x29, x30, [sp, #0x50]
100bcb7bc:     	ldp	x20, x19, [sp, #0x40]
100bcb7c0:     	ldp	x22, x21, [sp, #0x30]
100bcb7c4:     	ldp	x24, x23, [sp, #0x20]
100bcb7c8:     	ldp	x26, x25, [sp, #0x10]
100bcb7cc:     	add	sp, sp, #0x60
100bcb7d0:     	ret
100bcb7d4:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb7d8:     	add	x2, x2, #0x650
100bcb7dc:     	mov	x0, x8
100bcb7e0:     	mov	x1, x9
100bcb7e4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb7e8:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb7ec:     	add	x2, x2, #0x8d8
100bcb7f0:     	mov	x0, x9
100bcb7f4:     	mov	x1, x8
100bcb7f8:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb7fc:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb800:     	add	x2, x2, #0x8d8
100bcb804:     	mov	x0, x10
100bcb808:     	mov	x1, x8
100bcb80c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb810:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb814:     	add	x2, x2, #0x8d8
100bcb818:     	mov	x0, x11
100bcb81c:     	mov	x1, x8
100bcb820:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb824:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb828:     	add	x2, x2, #0x650
100bcb82c:     	mov	x0, x9
100bcb830:     	mov	x1, x10
100bcb834:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb838:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb83c:     	add	x2, x2, #0x650
100bcb840:     	mov	x0, x10
100bcb844:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bcb848:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bcb84c:     	add	x2, x2, #0x650
100bcb850:     	mov	x0, x8
100bcb854:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
