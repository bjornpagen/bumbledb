
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010059d500 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>:
10059d500:     	sub	sp, sp, #0x50
10059d504:     	stp	x24, x23, [sp, #0x10]
10059d508:     	stp	x22, x21, [sp, #0x20]
10059d50c:     	stp	x20, x19, [sp, #0x30]
10059d510:     	stp	x29, x30, [sp, #0x40]
10059d514:     	add	x29, sp, #0x40
10059d518:     	mov	x20, x3
10059d51c:     	mov	x23, x2
10059d520:     	mov	x22, x1
10059d524:     	mov	x19, x0
10059d528:     	mov	w1, w2
10059d52c:     	mov	w2, w3
10059d530:     	mov	x0, x22
10059d534:     	bl	0x100670e68 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
10059d538:     	cmp	x0, #0x1
10059d53c:     	b.ne	0x10059d548 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x48>
10059d540:     	mov	x21, x1
10059d544:     	b	0x10059dbfc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6fc>
10059d548:     	mov	w8, #0x9                ; =9
10059d54c:     	and	w8, w22, w8
10059d550:     	lsr	w9, w22, #1
10059d554:     	bfi	w8, w9, #2, #1
10059d558:     	and	w9, w9, #0x2
10059d55c:     	orr	w8, w8, w9
10059d560:     	cmp	w23, w20
10059d564:     	csel	w24, w23, w20, hi
10059d568:     	csel	w23, w20, w23, hi
10059d56c:     	csel	w20, w8, w22, hi
10059d570:     	ldr	x8, [x19, #0xb8]
10059d574:     	cbz	x8, 0x10059d644 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x144>
10059d578:     	mov	x8, #0x0                ; =0
10059d57c:     	and	x9, x20, #0xff
10059d580:     	mov	x10, #0xa9c5            ; =43461
10059d584:     	movk	x10, #0x2e62, lsl #16
10059d588:     	movk	x10, #0x7aea, lsl #32
10059d58c:     	movk	x10, #0xf135, lsl #48
10059d590:     	mul	x9, x9, x10
10059d594:     	add	x9, x9, w23, uxtw
10059d598:     	mul	x9, x9, x10
10059d59c:     	add	x9, x9, w24, uxtw
10059d5a0:     	mul	x9, x9, x10
10059d5a4:     	ror	x11, x9, #0x2c
10059d5a8:     	lsr	x12, x11, #57
10059d5ac:     	ldp	x10, x9, [x19, #0xa0]
10059d5b0:     	dup.8b	v0, w12
10059d5b4:     	movi.2d	v1, #0xffffffffffffffff
10059d5b8:     	and	x11, x11, x9
10059d5bc:     	ldr	d2, [x10, x11]
10059d5c0:     	cmeq.8b	v3, v2, v0
10059d5c4:     	fmov	x12, d3
10059d5c8:     	ands	x12, x12, #0x8080808080808080
10059d5cc:     	b.eq	0x10059d614 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x114>
10059d5d0:     	rbit	x13, x12
10059d5d4:     	clz	x13, x13
10059d5d8:     	add	x13, x11, x13, lsr #3
10059d5dc:     	and	x13, x13, x9
10059d5e0:     	sub	x13, x10, x13, lsl #4
10059d5e4:     	ldurb	w14, [x13, #-0xc]
10059d5e8:     	cmp	w14, w20, uxtb
10059d5ec:     	b.ne	0x10059d608 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x108>
10059d5f0:     	ldur	w14, [x13, #-0x10]
10059d5f4:     	cmp	w23, w14
10059d5f8:     	b.ne	0x10059d608 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x108>
10059d5fc:     	ldur	w14, [x13, #-0x8]
10059d600:     	cmp	w24, w14
10059d604:     	b.eq	0x10059d6e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1e4>
10059d608:     	sub	x13, x12, #0x2
10059d60c:     	ands	x12, x13, x12
10059d610:     	b.ne	0x10059d5d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0xd0>
10059d614:     	cmeq.8b	v2, v2, v1
10059d618:     	fmov	x12, d2
10059d61c:     	cbnz	x12, 0x10059d644 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x144>
10059d620:     	add	x8, x8, #0x8
10059d624:     	add	x11, x11, x8
10059d628:     	and	x11, x11, x9
10059d62c:     	ldr	d2, [x10, x11]
10059d630:     	cmeq.8b	v3, v2, v0
10059d634:     	fmov	x12, d3
10059d638:     	ands	x12, x12, #0x8080808080808080
10059d63c:     	b.ne	0x10059d5d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0xd0>
10059d640:     	b	0x10059d614 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x114>
10059d644:     	tbnz	w23, #0x1, 0x10059d6ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1ec>
10059d648:     	ldr	x8, [x19, #0xc8]
10059d64c:     	tbnz	w24, #0x1, 0x10059d70c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x20c>
10059d650:     	ldr	x9, [x19, #0xc8]
10059d654:     	cmp	x9, x8
10059d658:     	csel	x21, x9, x8, lo
10059d65c:     	cmp	x21, x9
10059d660:     	b.ne	0x10059d73c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x23c>
10059d664:     	lsr	w0, w23, #2
10059d668:     	ldr	x1, [x19, #0x58]
10059d66c:     	cmp	x1, x0
10059d670:     	b.ls	0x10059dc24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x724>
10059d674:     	lsr	w8, w24, #2
10059d678:     	cmp	x1, x8
10059d67c:     	b.ls	0x10059dc30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x730>
10059d680:     	ldr	x10, [x19, #0xd8]
10059d684:     	tst	w23, #0x1
10059d688:     	csel	x9, xzr, x10, eq
10059d68c:     	ldr	x11, [x19, #0x50]
10059d690:     	ldr	x12, [x11, x0, lsl #3]
10059d694:     	eor	x9, x12, x9
10059d698:     	ldr	x8, [x11, x8, lsl #3]
10059d69c:     	tst	w24, #0x1
10059d6a0:     	csel	x10, xzr, x10, eq
10059d6a4:     	eor	x8, x8, x10
10059d6a8:     	and	x1, x20, #0xff
10059d6ac:     	adrp	x10, 0x100be2000 <dyld_stub_binder+0x100be2000>
10059d6b0:     	add	x10, x10, #0x576
10059d6b4:     	adr	x11, 0x10059d6c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1c4>
10059d6b8:     	ldrh	w12, [x10, x1, lsl #1]
10059d6bc:     	add	x11, x11, x12, lsl #2
10059d6c0:     	br	x11
10059d6c4:     	mov	x10, #0x0               ; =0
10059d6c8:     	and	x11, x8, x9
10059d6cc:     	eor	x12, x8, x9
10059d6d0:     	bic	x13, x9, x8
10059d6d4:     	bic	x14, x8, x9
10059d6d8:     	orr	x15, x8, x9
10059d6dc:     	mov	w16, #0x1               ; =1
10059d6e0:     	b	0x10059d8b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b8>
10059d6e4:     	ldur	w21, [x13, #-0x4]
10059d6e8:     	b	0x10059dbfc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6fc>
10059d6ec:     	lsr	w0, w23, #2
10059d6f0:     	ldr	x1, [x19, #0x40]
10059d6f4:     	cmp	x1, x0
10059d6f8:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d6fc:     	ldr	x8, [x19, #0x38]
10059d700:     	lsl	x9, x0, #4
10059d704:     	ldr	w8, [x8, x9]
10059d708:     	tbz	w24, #0x1, 0x10059d650 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x150>
10059d70c:     	lsr	w0, w24, #2
10059d710:     	ldr	x1, [x19, #0x40]
10059d714:     	cmp	x1, x0
10059d718:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d71c:     	ldr	x9, [x19, #0x38]
10059d720:     	lsl	x10, x0, #4
10059d724:     	ldr	w10, [x9, x10]
10059d728:     	ldr	x9, [x19, #0xc8]
10059d72c:     	cmp	x10, x8
10059d730:     	csel	x21, x10, x8, lo
10059d734:     	cmp	x21, x9
10059d738:     	b.eq	0x10059d664 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x164>
10059d73c:     	mov	x2, x23
10059d740:     	tbz	w23, #0x1, 0x10059d778 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x278>
10059d744:     	lsr	w0, w23, #2
10059d748:     	ldr	x1, [x19, #0x40]
10059d74c:     	cmp	x1, x0
10059d750:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d754:     	ldr	x8, [x19, #0x38]
10059d758:     	add	x8, x8, x0, lsl #4
10059d75c:     	ldr	w9, [x8]
10059d760:     	mov	x2, x23
10059d764:     	cmp	x21, x9
10059d768:     	b.ne	0x10059d778 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x278>
10059d76c:     	ldr	w8, [x8, #0x4]
10059d770:     	and	w9, w23, #0x1
10059d774:     	eor	w2, w8, w9
10059d778:     	mov	x3, x24
10059d77c:     	tbz	w24, #0x1, 0x10059d7b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2b4>
10059d780:     	lsr	w0, w24, #2
10059d784:     	ldr	x1, [x19, #0x40]
10059d788:     	cmp	x1, x0
10059d78c:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d790:     	ldr	x8, [x19, #0x38]
10059d794:     	add	x8, x8, x0, lsl #4
10059d798:     	ldr	w9, [x8]
10059d79c:     	mov	x3, x24
10059d7a0:     	cmp	x21, x9
10059d7a4:     	b.ne	0x10059d7b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2b4>
10059d7a8:     	ldr	w8, [x8, #0x4]
10059d7ac:     	and	w9, w24, #0x1
10059d7b0:     	eor	w3, w8, w9
10059d7b4:     	mov	x0, x19
10059d7b8:     	mov	x1, x20
10059d7bc:     	bl	0x10059d500 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>
10059d7c0:     	mov	x22, x0
10059d7c4:     	tbnz	w23, #0x1, 0x10059d82c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x32c>
10059d7c8:     	ldr	x8, [x19, #0xc8]
10059d7cc:     	mov	x2, x23
10059d7d0:     	cmp	x8, x21
10059d7d4:     	b.ne	0x10059d854 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x354>
10059d7d8:     	lsr	w0, w23, #2
10059d7dc:     	ldr	x1, [x19, #0x40]
10059d7e0:     	cmp	x1, x0
10059d7e4:     	b.ls	0x10059dc40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x740>
10059d7e8:     	ldr	x8, [x19, #0x38]
10059d7ec:     	add	x8, x8, x0, lsl #4
10059d7f0:     	ldr	w8, [x8, #0x8]
10059d7f4:     	and	w9, w23, #0x1
10059d7f8:     	eor	w2, w8, w9
10059d7fc:     	tbz	w24, #0x1, 0x10059d858 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x358>
10059d800:     	lsr	w0, w24, #2
10059d804:     	ldr	x1, [x19, #0x40]
10059d808:     	cmp	x1, x0
10059d80c:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d810:     	ldr	x8, [x19, #0x38]
10059d814:     	lsl	x9, x0, #4
10059d818:     	ldr	w8, [x8, x9]
10059d81c:     	mov	x3, x24
10059d820:     	cmp	x8, x21
10059d824:     	b.ne	0x10059d88c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x38c>
10059d828:     	b	0x10059d868 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x368>
10059d82c:     	lsr	w0, w23, #2
10059d830:     	ldr	x1, [x19, #0x40]
10059d834:     	cmp	x1, x0
10059d838:     	b.ls	0x10059dc18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
10059d83c:     	ldr	x8, [x19, #0x38]
10059d840:     	lsl	x9, x0, #4
10059d844:     	ldr	w8, [x8, x9]
10059d848:     	mov	x2, x23
10059d84c:     	cmp	x8, x21
10059d850:     	b.eq	0x10059d7d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2d8>
10059d854:     	tbnz	w24, #0x1, 0x10059d800 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x300>
10059d858:     	ldr	x8, [x19, #0xc8]
10059d85c:     	mov	x3, x24
10059d860:     	cmp	x8, x21
10059d864:     	b.ne	0x10059d88c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x38c>
10059d868:     	lsr	w0, w24, #2
10059d86c:     	ldr	x1, [x19, #0x40]
10059d870:     	cmp	x1, x0
10059d874:     	b.ls	0x10059dc40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x740>
10059d878:     	ldr	x8, [x19, #0x38]
10059d87c:     	add	x8, x8, x0, lsl #4
10059d880:     	ldr	w8, [x8, #0x8]
10059d884:     	and	w9, w24, #0x1
10059d888:     	eor	w3, w8, w9
10059d88c:     	mov	x0, x19
10059d890:     	mov	x1, x20
10059d894:     	bl	0x10059d500 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>
10059d898:     	mov	x3, x0
10059d89c:     	mov	x0, x19
10059d8a0:     	mov	x1, x21
10059d8a4:     	mov	x2, x22
10059d8a8:     	bl	0x10059c8b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E2mkB6_>
10059d8ac:     	b	0x10059dbdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6dc>
10059d8b0:     	eor	w16, w16, #0xf
10059d8b4:     	mvn	x10, x10
10059d8b8:     	and	w17, w16, #0xff
10059d8bc:     	cmp	w17, #0x7
10059d8c0:     	b.le	0x10059d8e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3e0>
10059d8c4:     	cmp	w17, #0xb
10059d8c8:     	b.gt	0x10059d8fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3fc>
10059d8cc:     	cmp	w17, #0x8
10059d8d0:     	b.eq	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059d8d4:     	cmp	w17, #0xa
10059d8d8:     	b.ne	0x10059d8b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
10059d8dc:     	b	0x10059db90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
10059d8e0:     	cmp	w17, #0x2
10059d8e4:     	b.eq	0x10059dba8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a8>
10059d8e8:     	cmp	w17, #0x4
10059d8ec:     	b.eq	0x10059dab0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5b0>
10059d8f0:     	cmp	w17, #0x6
10059d8f4:     	b.ne	0x10059d8b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
10059d8f8:     	b	0x10059da50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x550>
10059d8fc:     	cmp	w17, #0xc
10059d900:     	b.eq	0x10059db98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
10059d904:     	cmp	w17, #0xe
10059d908:     	b.ne	0x10059d8b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
10059d90c:     	mov	x11, x15
10059d910:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059d914:     	orr	x1, x8, x9
10059d918:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059d91c:     	mov	x10, #0x0               ; =0
10059d920:     	eor	x11, x8, x9
10059d924:     	bic	x12, x9, x8
10059d928:     	and	x9, x8, x9
10059d92c:     	mov	w13, #0xb               ; =11
10059d930:     	b	0x10059d93c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x43c>
10059d934:     	eor	w13, w13, #0xf
10059d938:     	mvn	x10, x10
10059d93c:     	and	w14, w13, #0xff
10059d940:     	cmp	w14, #0x7
10059d944:     	b.gt	0x10059d95c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x45c>
10059d948:     	cmp	w14, #0x4
10059d94c:     	b.eq	0x10059dba0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a0>
10059d950:     	cmp	w14, #0x6
10059d954:     	b.ne	0x10059d934 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x434>
10059d958:     	b	0x10059db04 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x604>
10059d95c:     	cmp	w14, #0x8
10059d960:     	b.eq	0x10059db70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
10059d964:     	cmp	w14, #0xa
10059d968:     	b.ne	0x10059d934 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x434>
10059d96c:     	b	0x10059dbb4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
10059d970:     	bic	x1, x9, x8
10059d974:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059d978:     	mov	x1, x9
10059d97c:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059d980:     	mov	x10, #0x0               ; =0
10059d984:     	eor	x11, x8, x9
10059d988:     	and	x8, x8, x9
10059d98c:     	mov	w9, #0x9                ; =9
10059d990:     	and	w12, w9, #0xff
10059d994:     	cmp	w12, #0x6
10059d998:     	b.eq	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059d99c:     	cmp	w12, #0x8
10059d9a0:     	b.eq	0x10059db90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
10059d9a4:     	eor	w9, w9, #0xf
10059d9a8:     	mvn	x10, x10
10059d9ac:     	and	w12, w9, #0xff
10059d9b0:     	cmp	w12, #0x6
10059d9b4:     	b.ne	0x10059d99c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x49c>
10059d9b8:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059d9bc:     	bic	x1, x8, x9
10059d9c0:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059d9c4:     	mov	x10, #0x0               ; =0
10059d9c8:     	orr	x12, x8, x9
10059d9cc:     	and	x13, x8, x9
10059d9d0:     	eor	x11, x8, x9
10059d9d4:     	bic	x14, x9, x8
10059d9d8:     	bic	x15, x8, x9
10059d9dc:     	mov	w16, #0xf               ; =15
10059d9e0:     	b	0x10059d9ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4ec>
10059d9e4:     	eor	w16, w16, #0xf
10059d9e8:     	mvn	x10, x10
10059d9ec:     	and	w17, w16, #0xff
10059d9f0:     	cmp	w17, #0x7
10059d9f4:     	b.gt	0x10059da10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x510>
10059d9f8:     	cmp	w17, #0x3
10059d9fc:     	b.gt	0x10059da2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x52c>
10059da00:     	cbz	w17, 0x10059dbcc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6cc>
10059da04:     	cmp	w17, #0x2
10059da08:     	b.ne	0x10059d9e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
10059da0c:     	b	0x10059d90c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x40c>
10059da10:     	cmp	w17, #0xb
10059da14:     	b.gt	0x10059da40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x540>
10059da18:     	cmp	w17, #0x8
10059da1c:     	b.eq	0x10059dab0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5b0>
10059da20:     	cmp	w17, #0xa
10059da24:     	b.ne	0x10059d9e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
10059da28:     	b	0x10059db90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
10059da2c:     	cmp	w17, #0x4
10059da30:     	b.eq	0x10059dba8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a8>
10059da34:     	cmp	w17, #0x6
10059da38:     	b.ne	0x10059d9e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
10059da3c:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059da40:     	cmp	w17, #0xc
10059da44:     	b.eq	0x10059db98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
10059da48:     	cmp	w17, #0xe
10059da4c:     	b.ne	0x10059d9e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
10059da50:     	mov	x11, x12
10059da54:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059da58:     	mov	x10, #0x0               ; =0
10059da5c:     	and	x12, x8, x9
10059da60:     	eor	x13, x8, x9
10059da64:     	bic	x11, x9, x8
10059da68:     	mov	w14, #0x3               ; =3
10059da6c:     	b	0x10059da78 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x578>
10059da70:     	eor	w14, w14, #0xf
10059da74:     	mvn	x10, x10
10059da78:     	and	w15, w14, #0xff
10059da7c:     	cmp	w15, #0x7
10059da80:     	b.le	0x10059daa0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5a0>
10059da84:     	cmp	w15, #0x8
10059da88:     	b.eq	0x10059da50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x550>
10059da8c:     	cmp	w15, #0xa
10059da90:     	b.eq	0x10059db90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
10059da94:     	cmp	w15, #0xc
10059da98:     	b.ne	0x10059da70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x570>
10059da9c:     	b	0x10059db98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
10059daa0:     	cmp	w15, #0x4
10059daa4:     	b.eq	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059daa8:     	cmp	w15, #0x6
10059daac:     	b.ne	0x10059da70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x570>
10059dab0:     	mov	x11, x13
10059dab4:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059dab8:     	and	x8, x8, x9
10059dabc:     	mvn	x1, x8
10059dac0:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059dac4:     	mov	x10, #0x0               ; =0
10059dac8:     	eor	x11, x8, x9
10059dacc:     	and	x9, x8, x9
10059dad0:     	mov	w12, #0x5               ; =5
10059dad4:     	and	w13, w12, #0xff
10059dad8:     	cmp	w13, #0x6
10059dadc:     	b.eq	0x10059db04 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x604>
10059dae0:     	cmp	w13, #0x8
10059dae4:     	b.eq	0x10059db70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
10059dae8:     	cmp	w13, #0xa
10059daec:     	b.eq	0x10059dbb4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
10059daf0:     	eor	w12, w12, #0xf
10059daf4:     	mvn	x10, x10
10059daf8:     	and	w13, w12, #0xff
10059dafc:     	cmp	w13, #0x6
10059db00:     	b.ne	0x10059dae0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5e0>
10059db04:     	mov	x9, x11
10059db08:     	b	0x10059db70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
10059db0c:     	mov	x10, #0x0               ; =0
10059db10:     	and	x11, x8, x9
10059db14:     	eor	x12, x8, x9
10059db18:     	bic	x13, x9, x8
10059db1c:     	bic	x14, x8, x9
10059db20:     	mov	w15, #0xd               ; =13
10059db24:     	b	0x10059db30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x630>
10059db28:     	eor	w15, w15, #0xf
10059db2c:     	mvn	x10, x10
10059db30:     	and	w16, w15, #0xff
10059db34:     	cmp	w16, #0x7
10059db38:     	b.gt	0x10059db58 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x658>
10059db3c:     	cmp	w16, #0x2
10059db40:     	b.eq	0x10059dbbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6bc>
10059db44:     	cmp	w16, #0x4
10059db48:     	b.eq	0x10059dbc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6c4>
10059db4c:     	cmp	w16, #0x6
10059db50:     	b.ne	0x10059db28 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x628>
10059db54:     	b	0x10059dba0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a0>
10059db58:     	cmp	w16, #0x8
10059db5c:     	b.eq	0x10059dbb0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b0>
10059db60:     	cmp	w16, #0xa
10059db64:     	b.eq	0x10059dbb4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
10059db68:     	cmp	w16, #0xc
10059db6c:     	b.ne	0x10059db28 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x628>
10059db70:     	eor	x1, x9, x10
10059db74:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059db78:     	eor	x1, x8, x9
10059db7c:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059db80:     	mov	x1, x8
10059db84:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059db88:     	and	x1, x8, x9
10059db8c:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059db90:     	mov	x11, x8
10059db94:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059db98:     	mov	x11, x9
10059db9c:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059dba0:     	eor	x1, x12, x10
10059dba4:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059dba8:     	mov	x11, x14
10059dbac:     	b	0x10059dbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
10059dbb0:     	mov	x8, x11
10059dbb4:     	eor	x1, x8, x10
10059dbb8:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059dbbc:     	eor	x1, x14, x10
10059dbc0:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059dbc4:     	eor	x1, x13, x10
10059dbc8:     	b	0x10059dbd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
10059dbcc:     	mov	x11, #0x0               ; =0
10059dbd0:     	eor	x1, x11, x10
10059dbd4:     	mov	x0, x19
10059dbd8:     	bl	0x10059ce0c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E4leafB6_>
10059dbdc:     	mov	x21, x0
10059dbe0:     	strb	w20, [sp, #0x8]
10059dbe4:     	str	w23, [sp, #0x4]
10059dbe8:     	str	w24, [sp, #0xc]
10059dbec:     	add	x0, x19, #0xa0
10059dbf0:     	add	x1, sp, #0x4
10059dbf4:     	mov	x2, x21
10059dbf8:     	bl	0x1005f2878 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
10059dbfc:     	mov	x0, x21
10059dc00:     	ldp	x29, x30, [sp, #0x40]
10059dc04:     	ldp	x20, x19, [sp, #0x30]
10059dc08:     	ldp	x22, x21, [sp, #0x20]
10059dc0c:     	ldp	x24, x23, [sp, #0x10]
10059dc10:     	add	sp, sp, #0x50
10059dc14:     	ret
10059dc18:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059dc1c:     	add	x2, x2, #0x100
10059dc20:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059dc24:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059dc28:     	add	x2, x2, #0x2c8
10059dc2c:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059dc30:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059dc34:     	add	x2, x2, #0x2c8
10059dc38:     	mov	x0, x8
10059dc3c:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059dc40:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059dc44:     	add	x2, x2, #0x2b0
10059dc48:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059dc4c:     	nop
10059dc50:     	nop
10059dc54:     	nop
10059dc58:     	nop
10059dc5c:     	nop
10059dc60:     	nop
10059dc64:     	nop
10059dc68:     	nop
10059dc6c:     	nop
10059dc70:     	nop
10059dc74:     	nop
10059dc78:     	nop
10059dc7c:     	nop
