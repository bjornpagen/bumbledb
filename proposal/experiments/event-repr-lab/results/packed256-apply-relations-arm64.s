
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>:
1005a0400:     	sub	sp, sp, #0x60
1005a0404:     	stp	x24, x23, [sp, #0x20]
1005a0408:     	stp	x22, x21, [sp, #0x30]
1005a040c:     	stp	x20, x19, [sp, #0x40]
1005a0410:     	stp	x29, x30, [sp, #0x50]
1005a0414:     	add	x29, sp, #0x50
1005a0418:     	mov	x20, x3
1005a041c:     	mov	x23, x2
1005a0420:     	mov	x22, x1
1005a0424:     	mov	x19, x0
1005a0428:     	mov	w1, w2
1005a042c:     	mov	w2, w3
1005a0430:     	mov	x0, x22
1005a0434:     	bl	0x100670e68 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
1005a0438:     	cmp	x0, #0x1
1005a043c:     	b.ne	0x1005a0448 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x48>
1005a0440:     	mov	x21, x1
1005a0444:     	b	0x1005a14f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10f8>
1005a0448:     	mov	w8, #0x9                ; =9
1005a044c:     	and	w8, w22, w8
1005a0450:     	lsr	w9, w22, #1
1005a0454:     	bfi	w8, w9, #2, #1
1005a0458:     	and	w9, w9, #0x2
1005a045c:     	orr	w8, w8, w9
1005a0460:     	cmp	w23, w20
1005a0464:     	csel	w24, w23, w20, hi
1005a0468:     	csel	w23, w20, w23, hi
1005a046c:     	csel	w20, w8, w22, hi
1005a0470:     	ldr	x8, [x19, #0xb8]
1005a0474:     	cbz	x8, 0x1005a0544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x144>
1005a0478:     	mov	x8, #0x0                ; =0
1005a047c:     	and	x9, x20, #0xff
1005a0480:     	mov	x10, #0xa9c5            ; =43461
1005a0484:     	movk	x10, #0x2e62, lsl #16
1005a0488:     	movk	x10, #0x7aea, lsl #32
1005a048c:     	movk	x10, #0xf135, lsl #48
1005a0490:     	mul	x9, x9, x10
1005a0494:     	add	x9, x9, w23, uxtw
1005a0498:     	mul	x9, x9, x10
1005a049c:     	add	x9, x9, w24, uxtw
1005a04a0:     	mul	x9, x9, x10
1005a04a4:     	ror	x11, x9, #0x2c
1005a04a8:     	lsr	x12, x11, #57
1005a04ac:     	ldp	x10, x9, [x19, #0xa0]
1005a04b0:     	dup.8b	v0, w12
1005a04b4:     	movi.2d	v1, #0xffffffffffffffff
1005a04b8:     	and	x11, x11, x9
1005a04bc:     	ldr	d2, [x10, x11]
1005a04c0:     	cmeq.8b	v3, v2, v0
1005a04c4:     	fmov	x12, d3
1005a04c8:     	ands	x12, x12, #0x8080808080808080
1005a04cc:     	b.eq	0x1005a0514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x114>
1005a04d0:     	rbit	x13, x12
1005a04d4:     	clz	x13, x13
1005a04d8:     	add	x13, x11, x13, lsr #3
1005a04dc:     	and	x13, x13, x9
1005a04e0:     	sub	x13, x10, x13, lsl #4
1005a04e4:     	ldurb	w14, [x13, #-0xc]
1005a04e8:     	cmp	w14, w20, uxtb
1005a04ec:     	b.ne	0x1005a0508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x108>
1005a04f0:     	ldur	w14, [x13, #-0x10]
1005a04f4:     	cmp	w23, w14
1005a04f8:     	b.ne	0x1005a0508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x108>
1005a04fc:     	ldur	w14, [x13, #-0x8]
1005a0500:     	cmp	w24, w14
1005a0504:     	b.eq	0x1005a0630 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x230>
1005a0508:     	sub	x13, x12, #0x2
1005a050c:     	ands	x12, x13, x12
1005a0510:     	b.ne	0x1005a04d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd0>
1005a0514:     	cmeq.8b	v2, v2, v1
1005a0518:     	fmov	x12, d2
1005a051c:     	cbnz	x12, 0x1005a0544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x144>
1005a0520:     	add	x8, x8, #0x8
1005a0524:     	add	x11, x11, x8
1005a0528:     	and	x11, x11, x9
1005a052c:     	ldr	d2, [x10, x11]
1005a0530:     	cmeq.8b	v3, v2, v0
1005a0534:     	fmov	x12, d3
1005a0538:     	ands	x12, x12, #0x8080808080808080
1005a053c:     	b.ne	0x1005a04d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd0>
1005a0540:     	b	0x1005a0514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x114>
1005a0544:     	tbnz	w23, #0x1, 0x1005a0638 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x238>
1005a0548:     	ldr	x8, [x19, #0xc8]
1005a054c:     	tbnz	w24, #0x1, 0x1005a0658 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x258>
1005a0550:     	ldr	x9, [x19, #0xc8]
1005a0554:     	cmp	x9, x8
1005a0558:     	csel	x21, x9, x8, lo
1005a055c:     	cmp	x21, x9
1005a0560:     	b.ne	0x1005a0688 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x288>
1005a0564:     	lsr	w8, w23, #2
1005a0568:     	ldr	x1, [x19, #0x58]
1005a056c:     	cmp	x1, x8
1005a0570:     	b.ls	0x1005a1520 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1120>
1005a0574:     	lsr	w0, w24, #2
1005a0578:     	cmp	x1, x0
1005a057c:     	b.ls	0x1005a1530 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1130>
1005a0580:     	ldr	x9, [x19, #0x50]
1005a0584:     	add	x8, x9, x8, lsl #5
1005a0588:     	ldp	q0, q2, [x8]
1005a058c:     	and	w8, w23, #0x1
1005a0590:     	fmov	s1, w8
1005a0594:     	movi.2d	v3, #0000000000000000
1005a0598:     	cmeq.4s	v1, v1, v3
1005a059c:     	dup.4s	v4, v1[0]
1005a05a0:     	ldur	q1, [x19, #0xd8]
1005a05a4:     	eor.16b	v1, v1, v0
1005a05a8:     	bit.16b	v1, v0, v4
1005a05ac:     	ldur	q0, [x19, #0xe8]
1005a05b0:     	eor.16b	v0, v0, v2
1005a05b4:     	bit.16b	v0, v2, v4
1005a05b8:     	mov.d	x8, v0[1]
1005a05bc:     	mov.d	x13, v1[1]
1005a05c0:     	add	x9, x9, x0, lsl #5
1005a05c4:     	ldp	q4, q2, [x9]
1005a05c8:     	and	w9, w24, #0x1
1005a05cc:     	fmov	s5, w9
1005a05d0:     	cmeq.4s	v3, v5, v3
1005a05d4:     	dup.4s	v3, v3[0]
1005a05d8:     	ldur	q5, [x19, #0xe8]
1005a05dc:     	eor.16b	v5, v5, v2
1005a05e0:     	bif.16b	v2, v5, v3
1005a05e4:     	ldur	q5, [x19, #0xd8]
1005a05e8:     	eor.16b	v5, v5, v4
1005a05ec:     	mov.d	x9, v2[1]
1005a05f0:     	bsl.16b	v3, v4, v5
1005a05f4:     	mov.d	x14, v3[1]
1005a05f8:     	and	x16, x20, #0xff
1005a05fc:     	fmov	x10, d0
1005a0600:     	fmov	x15, d1
1005a0604:     	fmov	x11, d2
1005a0608:     	fmov	x12, d3
1005a060c:     	adrp	x17, 0x100be2000 <dyld_stub_binder+0x100be2000>
1005a0610:     	add	x17, x17, #0x596
1005a0614:     	adr	x0, 0x1005a0624 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x224>
1005a0618:     	ldrh	w1, [x17, x16, lsl #1]
1005a061c:     	add	x0, x0, x1, lsl #2
1005a0620:     	br	x0
1005a0624:     	movi.2d	v0, #0000000000000000
1005a0628:     	stp	q0, q0, [sp]
1005a062c:     	b	0x1005a14cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1005a0630:     	ldur	w21, [x13, #-0x4]
1005a0634:     	b	0x1005a14f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10f8>
1005a0638:     	lsr	w0, w23, #2
1005a063c:     	ldr	x1, [x19, #0x40]
1005a0640:     	cmp	x1, x0
1005a0644:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a0648:     	ldr	x8, [x19, #0x38]
1005a064c:     	lsl	x9, x0, #4
1005a0650:     	ldr	w8, [x8, x9]
1005a0654:     	tbz	w24, #0x1, 0x1005a0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x150>
1005a0658:     	lsr	w0, w24, #2
1005a065c:     	ldr	x1, [x19, #0x40]
1005a0660:     	cmp	x1, x0
1005a0664:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a0668:     	ldr	x9, [x19, #0x38]
1005a066c:     	lsl	x10, x0, #4
1005a0670:     	ldr	w10, [x9, x10]
1005a0674:     	ldr	x9, [x19, #0xc8]
1005a0678:     	cmp	x10, x8
1005a067c:     	csel	x21, x10, x8, lo
1005a0680:     	cmp	x21, x9
1005a0684:     	b.eq	0x1005a0564 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x164>
1005a0688:     	mov	x2, x23
1005a068c:     	tbz	w23, #0x1, 0x1005a06c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x2c4>
1005a0690:     	lsr	w0, w23, #2
1005a0694:     	ldr	x1, [x19, #0x40]
1005a0698:     	cmp	x1, x0
1005a069c:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a06a0:     	ldr	x8, [x19, #0x38]
1005a06a4:     	add	x8, x8, x0, lsl #4
1005a06a8:     	ldr	w9, [x8]
1005a06ac:     	mov	x2, x23
1005a06b0:     	cmp	x21, x9
1005a06b4:     	b.ne	0x1005a06c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x2c4>
1005a06b8:     	ldr	w8, [x8, #0x4]
1005a06bc:     	and	w9, w23, #0x1
1005a06c0:     	eor	w2, w8, w9
1005a06c4:     	mov	x3, x24
1005a06c8:     	tbz	w24, #0x1, 0x1005a0700 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x300>
1005a06cc:     	lsr	w0, w24, #2
1005a06d0:     	ldr	x1, [x19, #0x40]
1005a06d4:     	cmp	x1, x0
1005a06d8:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a06dc:     	ldr	x8, [x19, #0x38]
1005a06e0:     	add	x8, x8, x0, lsl #4
1005a06e4:     	ldr	w9, [x8]
1005a06e8:     	mov	x3, x24
1005a06ec:     	cmp	x21, x9
1005a06f0:     	b.ne	0x1005a0700 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x300>
1005a06f4:     	ldr	w8, [x8, #0x4]
1005a06f8:     	and	w9, w24, #0x1
1005a06fc:     	eor	w3, w8, w9
1005a0700:     	mov	x0, x19
1005a0704:     	mov	x1, x20
1005a0708:     	bl	0x1005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1005a070c:     	mov	x22, x0
1005a0710:     	tbnz	w23, #0x1, 0x1005a0778 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x378>
1005a0714:     	ldr	x8, [x19, #0xc8]
1005a0718:     	mov	x2, x23
1005a071c:     	cmp	x8, x21
1005a0720:     	b.ne	0x1005a07a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3a0>
1005a0724:     	lsr	w0, w23, #2
1005a0728:     	ldr	x1, [x19, #0x40]
1005a072c:     	cmp	x1, x0
1005a0730:     	b.ls	0x1005a153c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x113c>
1005a0734:     	ldr	x8, [x19, #0x38]
1005a0738:     	add	x8, x8, x0, lsl #4
1005a073c:     	ldr	w8, [x8, #0x8]
1005a0740:     	and	w9, w23, #0x1
1005a0744:     	eor	w2, w8, w9
1005a0748:     	tbz	w24, #0x1, 0x1005a07a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3a4>
1005a074c:     	lsr	w0, w24, #2
1005a0750:     	ldr	x1, [x19, #0x40]
1005a0754:     	cmp	x1, x0
1005a0758:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a075c:     	ldr	x8, [x19, #0x38]
1005a0760:     	lsl	x9, x0, #4
1005a0764:     	ldr	w8, [x8, x9]
1005a0768:     	mov	x3, x24
1005a076c:     	cmp	x8, x21
1005a0770:     	b.ne	0x1005a07d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3d8>
1005a0774:     	b	0x1005a07b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3b4>
1005a0778:     	lsr	w0, w23, #2
1005a077c:     	ldr	x1, [x19, #0x40]
1005a0780:     	cmp	x1, x0
1005a0784:     	b.ls	0x1005a1514 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1114>
1005a0788:     	ldr	x8, [x19, #0x38]
1005a078c:     	lsl	x9, x0, #4
1005a0790:     	ldr	w8, [x8, x9]
1005a0794:     	mov	x2, x23
1005a0798:     	cmp	x8, x21
1005a079c:     	b.eq	0x1005a0724 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x324>
1005a07a0:     	tbnz	w24, #0x1, 0x1005a074c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x34c>
1005a07a4:     	ldr	x8, [x19, #0xc8]
1005a07a8:     	mov	x3, x24
1005a07ac:     	cmp	x8, x21
1005a07b0:     	b.ne	0x1005a07d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x3d8>
1005a07b4:     	lsr	w0, w24, #2
1005a07b8:     	ldr	x1, [x19, #0x40]
1005a07bc:     	cmp	x1, x0
1005a07c0:     	b.ls	0x1005a153c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x113c>
1005a07c4:     	ldr	x8, [x19, #0x38]
1005a07c8:     	add	x8, x8, x0, lsl #4
1005a07cc:     	ldr	w8, [x8, #0x8]
1005a07d0:     	and	w9, w24, #0x1
1005a07d4:     	eor	w3, w8, w9
1005a07d8:     	mov	x0, x19
1005a07dc:     	mov	x1, x20
1005a07e0:     	bl	0x1005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1005a07e4:     	mov	x3, x0
1005a07e8:     	mov	x0, x19
1005a07ec:     	mov	x1, x21
1005a07f0:     	mov	x2, x22
1005a07f4:     	bl	0x10059f65c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E2mkB6_>
1005a07f8:     	b	0x1005a14d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10d8>
1005a07fc:     	orr.16b	v1, v3, v1
1005a0800:     	orr.16b	v0, v2, v0
1005a0804:     	b	0x1005a0ad8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1005a0808:     	stp	q3, q2, [sp]
1005a080c:     	b	0x1005a14cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1005a0810:     	mov	x16, #0x0               ; =0
1005a0814:     	mov	x17, x20
1005a0818:     	b	0x1005a0824 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x424>
1005a081c:     	eor	w17, w17, #0xf
1005a0820:     	mvn	x16, x16
1005a0824:     	and	w0, w17, #0xff
1005a0828:     	cmp	w0, #0x7
1005a082c:     	b.le	0x1005a084c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x44c>
1005a0830:     	cmp	w0, #0x8
1005a0834:     	b.eq	0x1005a0dcc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9cc>
1005a0838:     	cmp	w0, #0xa
1005a083c:     	b.eq	0x1005a0dd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1005a0840:     	cmp	w0, #0xc
1005a0844:     	b.ne	0x1005a081c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x41c>
1005a0848:     	b	0x1005a0dc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9c4>
1005a084c:     	cmp	w0, #0x4
1005a0850:     	b.eq	0x1005a0dd4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d4>
1005a0854:     	cmp	w0, #0x6
1005a0858:     	b.ne	0x1005a081c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x41c>
1005a085c:     	eor	x12, x12, x15
1005a0860:     	b	0x1005a0dd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1005a0864:     	mov	x16, #0x0               ; =0
1005a0868:     	mov	x17, x20
1005a086c:     	b	0x1005a0878 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x478>
1005a0870:     	eor	w17, w17, #0xf
1005a0874:     	mvn	x16, x16
1005a0878:     	and	w0, w17, #0xff
1005a087c:     	cmp	w0, #0x7
1005a0880:     	b.gt	0x1005a0898 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x498>
1005a0884:     	cmp	w0, #0x4
1005a0888:     	b.eq	0x1005a0cc0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c0>
1005a088c:     	cmp	w0, #0x6
1005a0890:     	b.ne	0x1005a0870 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x470>
1005a0894:     	b	0x1005a0cb0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8b0>
1005a0898:     	cmp	w0, #0x8
1005a089c:     	b.eq	0x1005a0cb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8b8>
1005a08a0:     	cmp	w0, #0xa
1005a08a4:     	b.ne	0x1005a0870 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x470>
1005a08a8:     	b	0x1005a0cc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1005a08ac:     	and.16b	v1, v3, v1
1005a08b0:     	and.16b	v0, v2, v0
1005a08b4:     	b	0x1005a0ad8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1005a08b8:     	mov	x16, #0x0               ; =0
1005a08bc:     	mov	x17, x20
1005a08c0:     	b	0x1005a08cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4cc>
1005a08c4:     	eor	w17, w17, #0xf
1005a08c8:     	mvn	x16, x16
1005a08cc:     	and	w0, w17, #0xff
1005a08d0:     	cmp	w0, #0x7
1005a08d4:     	b.gt	0x1005a08f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4f0>
1005a08d8:     	cmp	w0, #0x3
1005a08dc:     	b.gt	0x1005a090c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x50c>
1005a08e0:     	cbz	w0, 0x1005a12a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xea4>
1005a08e4:     	cmp	w0, #0x2
1005a08e8:     	b.ne	0x1005a08c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1005a08ec:     	b	0x1005a128c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe8c>
1005a08f0:     	cmp	w0, #0xb
1005a08f4:     	b.gt	0x1005a0920 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x520>
1005a08f8:     	cmp	w0, #0x8
1005a08fc:     	b.eq	0x1005a129c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe9c>
1005a0900:     	cmp	w0, #0xa
1005a0904:     	b.ne	0x1005a08c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1005a0908:     	b	0x1005a1294 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe94>
1005a090c:     	cmp	w0, #0x4
1005a0910:     	b.eq	0x1005a12ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeac>
1005a0914:     	cmp	w0, #0x6
1005a0918:     	b.ne	0x1005a08c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1005a091c:     	b	0x1005a1284 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe84>
1005a0920:     	cmp	w0, #0xc
1005a0924:     	b.eq	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a0928:     	cmp	w0, #0xe
1005a092c:     	b.ne	0x1005a08c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x4c4>
1005a0930:     	orr	x15, x12, x15
1005a0934:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a0938:     	bic.16b	v1, v3, v1
1005a093c:     	bic.16b	v0, v2, v0
1005a0940:     	b	0x1005a0ad8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1005a0944:     	eor.16b	v1, v3, v1
1005a0948:     	eor.16b	v0, v2, v0
1005a094c:     	b	0x1005a0ad8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1005a0950:     	mov	x16, #0x0               ; =0
1005a0954:     	mov	x17, x20
1005a0958:     	b	0x1005a0964 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x564>
1005a095c:     	eor	w17, w17, #0xf
1005a0960:     	mvn	x16, x16
1005a0964:     	and	w0, w17, #0xff
1005a0968:     	cmp	w0, #0x7
1005a096c:     	b.le	0x1005a098c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x58c>
1005a0970:     	cmp	w0, #0xb
1005a0974:     	b.gt	0x1005a09a8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5a8>
1005a0978:     	cmp	w0, #0x8
1005a097c:     	b.eq	0x1005a1098 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc98>
1005a0980:     	cmp	w0, #0xa
1005a0984:     	b.ne	0x1005a095c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1005a0988:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a098c:     	cmp	w0, #0x2
1005a0990:     	b.eq	0x1005a10a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xca0>
1005a0994:     	cmp	w0, #0x4
1005a0998:     	b.eq	0x1005a10a8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xca8>
1005a099c:     	cmp	w0, #0x6
1005a09a0:     	b.ne	0x1005a095c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1005a09a4:     	b	0x1005a1090 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc90>
1005a09a8:     	cmp	w0, #0xc
1005a09ac:     	b.eq	0x1005a1088 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc88>
1005a09b0:     	cmp	w0, #0xe
1005a09b4:     	b.ne	0x1005a095c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x55c>
1005a09b8:     	orr	x12, x12, x15
1005a09bc:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a09c0:     	bic.16b	v1, v1, v3
1005a09c4:     	bic.16b	v0, v0, v2
1005a09c8:     	b	0x1005a0ad8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6d8>
1005a09cc:     	mov	x16, #0x0               ; =0
1005a09d0:     	mov	x17, x20
1005a09d4:     	b	0x1005a09e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5e0>
1005a09d8:     	eor	w17, w17, #0xf
1005a09dc:     	mvn	x16, x16
1005a09e0:     	and	w0, w17, #0xff
1005a09e4:     	cmp	w0, #0x7
1005a09e8:     	b.gt	0x1005a0a08 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x608>
1005a09ec:     	cmp	w0, #0x2
1005a09f0:     	b.eq	0x1005a0f14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb14>
1005a09f4:     	cmp	w0, #0x4
1005a09f8:     	b.eq	0x1005a0f04 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb04>
1005a09fc:     	cmp	w0, #0x6
1005a0a00:     	b.ne	0x1005a09d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5d8>
1005a0a04:     	b	0x1005a0f0c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb0c>
1005a0a08:     	cmp	w0, #0x8
1005a0a0c:     	b.eq	0x1005a0efc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xafc>
1005a0a10:     	cmp	w0, #0xa
1005a0a14:     	b.eq	0x1005a0f18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1005a0a18:     	cmp	w0, #0xc
1005a0a1c:     	b.ne	0x1005a09d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x5d8>
1005a0a20:     	mov	x12, x15
1005a0a24:     	b	0x1005a0f18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1005a0a28:     	mov	x16, #0x0               ; =0
1005a0a2c:     	mov	x17, x20
1005a0a30:     	and	w0, w17, #0xff
1005a0a34:     	cmp	w0, #0x6
1005a0a38:     	b.eq	0x1005a0a60 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x660>
1005a0a3c:     	cmp	w0, #0x8
1005a0a40:     	b.eq	0x1005a0bb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7b8>
1005a0a44:     	cmp	w0, #0xa
1005a0a48:     	b.eq	0x1005a0bbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7bc>
1005a0a4c:     	eor	w17, w17, #0xf
1005a0a50:     	mvn	x16, x16
1005a0a54:     	and	w0, w17, #0xff
1005a0a58:     	cmp	w0, #0x6
1005a0a5c:     	b.ne	0x1005a0a3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x63c>
1005a0a60:     	eor	x12, x12, x15
1005a0a64:     	mov	x15, #0x0               ; =0
1005a0a68:     	mov	w17, #0x5               ; =5
1005a0a6c:     	and	w0, w17, #0xff
1005a0a70:     	cmp	w0, #0xa
1005a0a74:     	b.ne	0x1005a0bd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7d0>
1005a0a78:     	b	0x1005a0c18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1005a0a7c:     	mov	x16, #0x0               ; =0
1005a0a80:     	mov	x17, x20
1005a0a84:     	and	w0, w17, #0xff
1005a0a88:     	cmp	w0, #0x6
1005a0a8c:     	b.eq	0x1005a0aac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6ac>
1005a0a90:     	cmp	w0, #0x8
1005a0a94:     	b.eq	0x1005a0ae0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6e0>
1005a0a98:     	eor	w17, w17, #0xf
1005a0a9c:     	mvn	x16, x16
1005a0aa0:     	and	w0, w17, #0xff
1005a0aa4:     	cmp	w0, #0x6
1005a0aa8:     	b.ne	0x1005a0a90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x690>
1005a0aac:     	eor	x12, x12, x15
1005a0ab0:     	mov	x15, #0x0               ; =0
1005a0ab4:     	mov	w17, #0x9               ; =9
1005a0ab8:     	and	w0, w17, #0xff
1005a0abc:     	cmp	w0, #0x8
1005a0ac0:     	b.ne	0x1005a0af8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6f8>
1005a0ac4:     	b	0x1005a0b14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x714>
1005a0ac8:     	and.16b	v1, v3, v1
1005a0acc:     	and.16b	v0, v2, v0
1005a0ad0:     	mvn.16b	v1, v1
1005a0ad4:     	mvn.16b	v0, v0
1005a0ad8:     	stp	q1, q0, [sp]
1005a0adc:     	b	0x1005a14cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1005a0ae0:     	and	x12, x12, x15
1005a0ae4:     	mov	x15, #0x0               ; =0
1005a0ae8:     	mov	w17, #0x9               ; =9
1005a0aec:     	and	w0, w17, #0xff
1005a0af0:     	cmp	w0, #0x8
1005a0af4:     	b.eq	0x1005a0b14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x714>
1005a0af8:     	cmp	w0, #0x6
1005a0afc:     	b.eq	0x1005a0b30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x730>
1005a0b00:     	eor	w17, w17, #0xf
1005a0b04:     	mvn	x15, x15
1005a0b08:     	and	w0, w17, #0xff
1005a0b0c:     	cmp	w0, #0x8
1005a0b10:     	b.ne	0x1005a0af8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x6f8>
1005a0b14:     	and	x13, x14, x13
1005a0b18:     	mov	x14, #0x0               ; =0
1005a0b1c:     	mov	w17, #0x9               ; =9
1005a0b20:     	and	w0, w17, #0xff
1005a0b24:     	cmp	w0, #0x8
1005a0b28:     	b.ne	0x1005a0b48 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x748>
1005a0b2c:     	b	0x1005a0b64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x764>
1005a0b30:     	eor	x13, x14, x13
1005a0b34:     	mov	x14, #0x0               ; =0
1005a0b38:     	mov	w17, #0x9               ; =9
1005a0b3c:     	and	w0, w17, #0xff
1005a0b40:     	cmp	w0, #0x8
1005a0b44:     	b.eq	0x1005a0b64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x764>
1005a0b48:     	cmp	w0, #0x6
1005a0b4c:     	b.eq	0x1005a0b80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x780>
1005a0b50:     	eor	w17, w17, #0xf
1005a0b54:     	mvn	x14, x14
1005a0b58:     	and	w0, w17, #0xff
1005a0b5c:     	cmp	w0, #0x8
1005a0b60:     	b.ne	0x1005a0b48 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x748>
1005a0b64:     	and	x10, x11, x10
1005a0b68:     	mov	x11, #0x0               ; =0
1005a0b6c:     	mov	w17, #0x9               ; =9
1005a0b70:     	and	w0, w17, #0xff
1005a0b74:     	cmp	w0, #0x8
1005a0b78:     	b.ne	0x1005a0b98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x798>
1005a0b7c:     	b	0x1005a105c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1005a0b80:     	eor	x10, x11, x10
1005a0b84:     	mov	x11, #0x0               ; =0
1005a0b88:     	mov	w17, #0x9               ; =9
1005a0b8c:     	and	w0, w17, #0xff
1005a0b90:     	cmp	w0, #0x8
1005a0b94:     	b.eq	0x1005a105c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1005a0b98:     	cmp	w0, #0x6
1005a0b9c:     	b.eq	0x1005a0ef4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaf4>
1005a0ba0:     	eor	w17, w17, #0xf
1005a0ba4:     	mvn	x11, x11
1005a0ba8:     	and	w0, w17, #0xff
1005a0bac:     	cmp	w0, #0x8
1005a0bb0:     	b.ne	0x1005a0b98 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x798>
1005a0bb4:     	b	0x1005a105c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1005a0bb8:     	and	x12, x12, x15
1005a0bbc:     	mov	x15, #0x0               ; =0
1005a0bc0:     	mov	w17, #0x5               ; =5
1005a0bc4:     	and	w0, w17, #0xff
1005a0bc8:     	cmp	w0, #0xa
1005a0bcc:     	b.eq	0x1005a0c18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1005a0bd0:     	cmp	w0, #0x8
1005a0bd4:     	b.eq	0x1005a0c14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x814>
1005a0bd8:     	cmp	w0, #0x6
1005a0bdc:     	b.eq	0x1005a0bf8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7f8>
1005a0be0:     	eor	w17, w17, #0xf
1005a0be4:     	mvn	x15, x15
1005a0be8:     	and	w0, w17, #0xff
1005a0bec:     	cmp	w0, #0xa
1005a0bf0:     	b.ne	0x1005a0bd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x7d0>
1005a0bf4:     	b	0x1005a0c18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x818>
1005a0bf8:     	eor	x14, x14, x13
1005a0bfc:     	mov	x13, #0x0               ; =0
1005a0c00:     	mov	w17, #0x5               ; =5
1005a0c04:     	and	w0, w17, #0xff
1005a0c08:     	cmp	w0, #0xa
1005a0c0c:     	b.ne	0x1005a0c2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x82c>
1005a0c10:     	b	0x1005a0c74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1005a0c14:     	and	x14, x14, x13
1005a0c18:     	mov	x13, #0x0               ; =0
1005a0c1c:     	mov	w17, #0x5               ; =5
1005a0c20:     	and	w0, w17, #0xff
1005a0c24:     	cmp	w0, #0xa
1005a0c28:     	b.eq	0x1005a0c74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1005a0c2c:     	cmp	w0, #0x8
1005a0c30:     	b.eq	0x1005a0c70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x870>
1005a0c34:     	cmp	w0, #0x6
1005a0c38:     	b.eq	0x1005a0c54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x854>
1005a0c3c:     	eor	w17, w17, #0xf
1005a0c40:     	mvn	x13, x13
1005a0c44:     	and	w0, w17, #0xff
1005a0c48:     	cmp	w0, #0xa
1005a0c4c:     	b.ne	0x1005a0c2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x82c>
1005a0c50:     	b	0x1005a0c74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x874>
1005a0c54:     	eor	x11, x11, x10
1005a0c58:     	mov	x10, #0x0               ; =0
1005a0c5c:     	mov	w17, #0x5               ; =5
1005a0c60:     	and	w0, w17, #0xff
1005a0c64:     	cmp	w0, #0xa
1005a0c68:     	b.ne	0x1005a0c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x888>
1005a0c6c:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a0c70:     	and	x11, x11, x10
1005a0c74:     	mov	x10, #0x0               ; =0
1005a0c78:     	mov	w17, #0x5               ; =5
1005a0c7c:     	and	w0, w17, #0xff
1005a0c80:     	cmp	w0, #0xa
1005a0c84:     	b.eq	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a0c88:     	cmp	w0, #0x8
1005a0c8c:     	b.eq	0x1005a124c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1005a0c90:     	cmp	w0, #0x6
1005a0c94:     	b.eq	0x1005a1244 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1005a0c98:     	eor	w17, w17, #0xf
1005a0c9c:     	mvn	x10, x10
1005a0ca0:     	and	w0, w17, #0xff
1005a0ca4:     	cmp	w0, #0xa
1005a0ca8:     	b.ne	0x1005a0c88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x888>
1005a0cac:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a0cb0:     	eor	x12, x12, x15
1005a0cb4:     	b	0x1005a0cc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1005a0cb8:     	and	x12, x12, x15
1005a0cbc:     	b	0x1005a0cc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8c4>
1005a0cc0:     	bic	x12, x15, x12
1005a0cc4:     	mov	x15, #0x0               ; =0
1005a0cc8:     	mov	w17, #0xb               ; =11
1005a0ccc:     	b	0x1005a0cd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d8>
1005a0cd0:     	eor	w17, w17, #0xf
1005a0cd4:     	mvn	x15, x15
1005a0cd8:     	and	w0, w17, #0xff
1005a0cdc:     	cmp	w0, #0x7
1005a0ce0:     	b.gt	0x1005a0cf8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8f8>
1005a0ce4:     	cmp	w0, #0x4
1005a0ce8:     	b.eq	0x1005a0d1c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x91c>
1005a0cec:     	cmp	w0, #0x6
1005a0cf0:     	b.ne	0x1005a0cd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d0>
1005a0cf4:     	b	0x1005a0d0c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x90c>
1005a0cf8:     	cmp	w0, #0x8
1005a0cfc:     	b.eq	0x1005a0d14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x914>
1005a0d00:     	cmp	w0, #0xa
1005a0d04:     	b.ne	0x1005a0cd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x8d0>
1005a0d08:     	b	0x1005a0d20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1005a0d0c:     	eor	x14, x14, x13
1005a0d10:     	b	0x1005a0d20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1005a0d14:     	and	x14, x14, x13
1005a0d18:     	b	0x1005a0d20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x920>
1005a0d1c:     	bic	x14, x13, x14
1005a0d20:     	mov	x13, #0x0               ; =0
1005a0d24:     	mov	w17, #0xb               ; =11
1005a0d28:     	b	0x1005a0d34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x934>
1005a0d2c:     	eor	w17, w17, #0xf
1005a0d30:     	mvn	x13, x13
1005a0d34:     	and	w0, w17, #0xff
1005a0d38:     	cmp	w0, #0x7
1005a0d3c:     	b.gt	0x1005a0d54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x954>
1005a0d40:     	cmp	w0, #0x4
1005a0d44:     	b.eq	0x1005a0d78 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x978>
1005a0d48:     	cmp	w0, #0x6
1005a0d4c:     	b.ne	0x1005a0d2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x92c>
1005a0d50:     	b	0x1005a0d68 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x968>
1005a0d54:     	cmp	w0, #0x8
1005a0d58:     	b.eq	0x1005a0d70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x970>
1005a0d5c:     	cmp	w0, #0xa
1005a0d60:     	b.ne	0x1005a0d2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x92c>
1005a0d64:     	b	0x1005a0d7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1005a0d68:     	eor	x11, x11, x10
1005a0d6c:     	b	0x1005a0d7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1005a0d70:     	and	x11, x11, x10
1005a0d74:     	b	0x1005a0d7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x97c>
1005a0d78:     	bic	x11, x10, x11
1005a0d7c:     	mov	x10, #0x0               ; =0
1005a0d80:     	mov	w17, #0xb               ; =11
1005a0d84:     	b	0x1005a0d90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x990>
1005a0d88:     	eor	w17, w17, #0xf
1005a0d8c:     	mvn	x10, x10
1005a0d90:     	and	w0, w17, #0xff
1005a0d94:     	cmp	w0, #0x7
1005a0d98:     	b.gt	0x1005a0db0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9b0>
1005a0d9c:     	cmp	w0, #0x4
1005a0da0:     	b.eq	0x1005a1254 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe54>
1005a0da4:     	cmp	w0, #0x6
1005a0da8:     	b.ne	0x1005a0d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x988>
1005a0dac:     	b	0x1005a1244 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1005a0db0:     	cmp	w0, #0x8
1005a0db4:     	b.eq	0x1005a124c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1005a0db8:     	cmp	w0, #0xa
1005a0dbc:     	b.ne	0x1005a0d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x988>
1005a0dc0:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a0dc4:     	mov	x12, x15
1005a0dc8:     	b	0x1005a0dd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1005a0dcc:     	and	x12, x12, x15
1005a0dd0:     	b	0x1005a0dd8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9d8>
1005a0dd4:     	bic	x12, x15, x12
1005a0dd8:     	mov	x15, #0x0               ; =0
1005a0ddc:     	mov	w17, #0x3               ; =3
1005a0de0:     	b	0x1005a0dec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9ec>
1005a0de4:     	eor	w17, w17, #0xf
1005a0de8:     	mvn	x15, x15
1005a0dec:     	and	w0, w17, #0xff
1005a0df0:     	cmp	w0, #0x7
1005a0df4:     	b.le	0x1005a0e14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa14>
1005a0df8:     	cmp	w0, #0xc
1005a0dfc:     	b.eq	0x1005a0e40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1005a0e00:     	cmp	w0, #0xa
1005a0e04:     	b.eq	0x1005a0e34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa34>
1005a0e08:     	cmp	w0, #0x8
1005a0e0c:     	b.ne	0x1005a0de4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9e4>
1005a0e10:     	b	0x1005a0e2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa2c>
1005a0e14:     	cmp	w0, #0x4
1005a0e18:     	b.eq	0x1005a0e3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa3c>
1005a0e1c:     	cmp	w0, #0x6
1005a0e20:     	b.ne	0x1005a0de4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x9e4>
1005a0e24:     	eor	x13, x14, x13
1005a0e28:     	b	0x1005a0e40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1005a0e2c:     	and	x13, x14, x13
1005a0e30:     	b	0x1005a0e40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1005a0e34:     	mov	x13, x14
1005a0e38:     	b	0x1005a0e40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa40>
1005a0e3c:     	bic	x13, x13, x14
1005a0e40:     	mov	x14, #0x0               ; =0
1005a0e44:     	mov	w17, #0x3               ; =3
1005a0e48:     	b	0x1005a0e54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa54>
1005a0e4c:     	eor	w17, w17, #0xf
1005a0e50:     	mvn	x14, x14
1005a0e54:     	and	w0, w17, #0xff
1005a0e58:     	cmp	w0, #0x7
1005a0e5c:     	b.le	0x1005a0e7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa7c>
1005a0e60:     	cmp	w0, #0xc
1005a0e64:     	b.eq	0x1005a0ea8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1005a0e68:     	cmp	w0, #0xa
1005a0e6c:     	b.eq	0x1005a0e9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa9c>
1005a0e70:     	cmp	w0, #0x8
1005a0e74:     	b.ne	0x1005a0e4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa4c>
1005a0e78:     	b	0x1005a0e94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa94>
1005a0e7c:     	cmp	w0, #0x4
1005a0e80:     	b.eq	0x1005a0ea4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa4>
1005a0e84:     	cmp	w0, #0x6
1005a0e88:     	b.ne	0x1005a0e4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xa4c>
1005a0e8c:     	eor	x10, x11, x10
1005a0e90:     	b	0x1005a0ea8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1005a0e94:     	and	x10, x11, x10
1005a0e98:     	b	0x1005a0ea8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1005a0e9c:     	mov	x10, x11
1005a0ea0:     	b	0x1005a0ea8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaa8>
1005a0ea4:     	bic	x10, x10, x11
1005a0ea8:     	mov	x11, #0x0               ; =0
1005a0eac:     	mov	w17, #0x3               ; =3
1005a0eb0:     	b	0x1005a0ebc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xabc>
1005a0eb4:     	eor	w17, w17, #0xf
1005a0eb8:     	mvn	x11, x11
1005a0ebc:     	and	w0, w17, #0xff
1005a0ec0:     	cmp	w0, #0x7
1005a0ec4:     	b.le	0x1005a0ee4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xae4>
1005a0ec8:     	cmp	w0, #0xc
1005a0ecc:     	b.eq	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a0ed0:     	cmp	w0, #0xa
1005a0ed4:     	b.eq	0x1005a106c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc6c>
1005a0ed8:     	cmp	w0, #0x8
1005a0edc:     	b.ne	0x1005a0eb4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xab4>
1005a0ee0:     	b	0x1005a105c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc5c>
1005a0ee4:     	cmp	w0, #0x4
1005a0ee8:     	b.eq	0x1005a1064 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc64>
1005a0eec:     	cmp	w0, #0x6
1005a0ef0:     	b.ne	0x1005a0eb4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xab4>
1005a0ef4:     	eor	x8, x9, x8
1005a0ef8:     	b	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a0efc:     	and	x12, x12, x15
1005a0f00:     	b	0x1005a0f18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1005a0f04:     	bic	x12, x15, x12
1005a0f08:     	b	0x1005a0f18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1005a0f0c:     	eor	x12, x12, x15
1005a0f10:     	b	0x1005a0f18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb18>
1005a0f14:     	bic	x12, x12, x15
1005a0f18:     	mov	x15, #0x0               ; =0
1005a0f1c:     	mov	w17, #0xd               ; =13
1005a0f20:     	b	0x1005a0f2c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb2c>
1005a0f24:     	eor	w17, w17, #0xf
1005a0f28:     	mvn	x15, x15
1005a0f2c:     	and	w0, w17, #0xff
1005a0f30:     	cmp	w0, #0x7
1005a0f34:     	b.gt	0x1005a0f54 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb54>
1005a0f38:     	cmp	w0, #0x2
1005a0f3c:     	b.eq	0x1005a0f8c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb8c>
1005a0f40:     	cmp	w0, #0x4
1005a0f44:     	b.eq	0x1005a0f7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb7c>
1005a0f48:     	cmp	w0, #0x6
1005a0f4c:     	b.ne	0x1005a0f24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb24>
1005a0f50:     	b	0x1005a0f84 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb84>
1005a0f54:     	cmp	w0, #0xc
1005a0f58:     	b.eq	0x1005a0f90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1005a0f5c:     	cmp	w0, #0xa
1005a0f60:     	b.eq	0x1005a0f74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb74>
1005a0f64:     	cmp	w0, #0x8
1005a0f68:     	b.ne	0x1005a0f24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb24>
1005a0f6c:     	and	x13, x14, x13
1005a0f70:     	b	0x1005a0f90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1005a0f74:     	mov	x13, x14
1005a0f78:     	b	0x1005a0f90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1005a0f7c:     	bic	x13, x13, x14
1005a0f80:     	b	0x1005a0f90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1005a0f84:     	eor	x13, x14, x13
1005a0f88:     	b	0x1005a0f90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb90>
1005a0f8c:     	bic	x13, x14, x13
1005a0f90:     	mov	x14, #0x0               ; =0
1005a0f94:     	mov	w17, #0xd               ; =13
1005a0f98:     	b	0x1005a0fa4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xba4>
1005a0f9c:     	eor	w17, w17, #0xf
1005a0fa0:     	mvn	x14, x14
1005a0fa4:     	and	w0, w17, #0xff
1005a0fa8:     	cmp	w0, #0x7
1005a0fac:     	b.gt	0x1005a0fcc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbcc>
1005a0fb0:     	cmp	w0, #0x2
1005a0fb4:     	b.eq	0x1005a1004 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc04>
1005a0fb8:     	cmp	w0, #0x4
1005a0fbc:     	b.eq	0x1005a0ff4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbf4>
1005a0fc0:     	cmp	w0, #0x6
1005a0fc4:     	b.ne	0x1005a0f9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb9c>
1005a0fc8:     	b	0x1005a0ffc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbfc>
1005a0fcc:     	cmp	w0, #0xc
1005a0fd0:     	b.eq	0x1005a1008 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1005a0fd4:     	cmp	w0, #0xa
1005a0fd8:     	b.eq	0x1005a0fec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xbec>
1005a0fdc:     	cmp	w0, #0x8
1005a0fe0:     	b.ne	0x1005a0f9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xb9c>
1005a0fe4:     	and	x10, x11, x10
1005a0fe8:     	b	0x1005a1008 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1005a0fec:     	mov	x10, x11
1005a0ff0:     	b	0x1005a1008 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1005a0ff4:     	bic	x10, x10, x11
1005a0ff8:     	b	0x1005a1008 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1005a0ffc:     	eor	x10, x11, x10
1005a1000:     	b	0x1005a1008 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc08>
1005a1004:     	bic	x10, x11, x10
1005a1008:     	mov	x11, #0x0               ; =0
1005a100c:     	mov	w17, #0xd               ; =13
1005a1010:     	b	0x1005a101c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc1c>
1005a1014:     	eor	w17, w17, #0xf
1005a1018:     	mvn	x11, x11
1005a101c:     	and	w0, w17, #0xff
1005a1020:     	cmp	w0, #0x7
1005a1024:     	b.gt	0x1005a1044 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc44>
1005a1028:     	cmp	w0, #0x2
1005a102c:     	b.eq	0x1005a1074 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc74>
1005a1030:     	cmp	w0, #0x4
1005a1034:     	b.eq	0x1005a1064 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc64>
1005a1038:     	cmp	w0, #0x6
1005a103c:     	b.ne	0x1005a1014 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc14>
1005a1040:     	b	0x1005a0ef4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xaf4>
1005a1044:     	cmp	w0, #0xc
1005a1048:     	b.eq	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a104c:     	cmp	w0, #0xa
1005a1050:     	b.eq	0x1005a106c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc6c>
1005a1054:     	cmp	w0, #0x8
1005a1058:     	b.ne	0x1005a1014 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc14>
1005a105c:     	and	x8, x9, x8
1005a1060:     	b	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a1064:     	bic	x8, x8, x9
1005a1068:     	b	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a106c:     	mov	x8, x9
1005a1070:     	b	0x1005a1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xc78>
1005a1074:     	bic	x8, x9, x8
1005a1078:     	eor	x9, x10, x14
1005a107c:     	eor	x10, x13, x15
1005a1080:     	eor	x12, x12, x16
1005a1084:     	b	0x1005a14c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10c0>
1005a1088:     	mov	x12, x15
1005a108c:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a1090:     	eor	x12, x12, x15
1005a1094:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a1098:     	and	x12, x12, x15
1005a109c:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a10a0:     	bic	x12, x12, x15
1005a10a4:     	b	0x1005a10ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcac>
1005a10a8:     	bic	x12, x15, x12
1005a10ac:     	mov	x15, #0x0               ; =0
1005a10b0:     	mov	w17, #0x1               ; =1
1005a10b4:     	b	0x1005a10c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcc0>
1005a10b8:     	eor	w17, w17, #0xf
1005a10bc:     	mvn	x15, x15
1005a10c0:     	and	w0, w17, #0xff
1005a10c4:     	cmp	w0, #0x7
1005a10c8:     	b.le	0x1005a10e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xce8>
1005a10cc:     	cmp	w0, #0xb
1005a10d0:     	b.gt	0x1005a1104 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd04>
1005a10d4:     	cmp	w0, #0x8
1005a10d8:     	b.eq	0x1005a112c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd2c>
1005a10dc:     	cmp	w0, #0xa
1005a10e0:     	b.ne	0x1005a10b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1005a10e4:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a10e8:     	cmp	w0, #0x2
1005a10ec:     	b.eq	0x1005a1134 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd34>
1005a10f0:     	cmp	w0, #0x4
1005a10f4:     	b.eq	0x1005a113c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd3c>
1005a10f8:     	cmp	w0, #0x6
1005a10fc:     	b.ne	0x1005a10b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1005a1100:     	b	0x1005a1124 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd24>
1005a1104:     	cmp	w0, #0xc
1005a1108:     	b.eq	0x1005a111c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd1c>
1005a110c:     	cmp	w0, #0xe
1005a1110:     	b.ne	0x1005a10b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xcb8>
1005a1114:     	orr	x14, x14, x13
1005a1118:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a111c:     	mov	x14, x13
1005a1120:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a1124:     	eor	x14, x14, x13
1005a1128:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a112c:     	and	x14, x14, x13
1005a1130:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a1134:     	bic	x14, x14, x13
1005a1138:     	b	0x1005a1140 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd40>
1005a113c:     	bic	x14, x13, x14
1005a1140:     	mov	x13, #0x0               ; =0
1005a1144:     	mov	w17, #0x1               ; =1
1005a1148:     	b	0x1005a1154 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd54>
1005a114c:     	eor	w17, w17, #0xf
1005a1150:     	mvn	x13, x13
1005a1154:     	and	w0, w17, #0xff
1005a1158:     	cmp	w0, #0x7
1005a115c:     	b.le	0x1005a117c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd7c>
1005a1160:     	cmp	w0, #0xb
1005a1164:     	b.gt	0x1005a1198 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd98>
1005a1168:     	cmp	w0, #0x8
1005a116c:     	b.eq	0x1005a11c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdc0>
1005a1170:     	cmp	w0, #0xa
1005a1174:     	b.ne	0x1005a114c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1005a1178:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a117c:     	cmp	w0, #0x2
1005a1180:     	b.eq	0x1005a11c8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdc8>
1005a1184:     	cmp	w0, #0x4
1005a1188:     	b.eq	0x1005a11d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd0>
1005a118c:     	cmp	w0, #0x6
1005a1190:     	b.ne	0x1005a114c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1005a1194:     	b	0x1005a11b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdb8>
1005a1198:     	cmp	w0, #0xc
1005a119c:     	b.eq	0x1005a11b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdb0>
1005a11a0:     	cmp	w0, #0xe
1005a11a4:     	b.ne	0x1005a114c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xd4c>
1005a11a8:     	orr	x11, x11, x10
1005a11ac:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a11b0:     	mov	x11, x10
1005a11b4:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a11b8:     	eor	x11, x11, x10
1005a11bc:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a11c0:     	and	x11, x11, x10
1005a11c4:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a11c8:     	bic	x11, x11, x10
1005a11cc:     	b	0x1005a11d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xdd4>
1005a11d0:     	bic	x11, x10, x11
1005a11d4:     	mov	x10, #0x0               ; =0
1005a11d8:     	mov	w17, #0x1               ; =1
1005a11dc:     	b	0x1005a11e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde8>
1005a11e0:     	eor	w17, w17, #0xf
1005a11e4:     	mvn	x10, x10
1005a11e8:     	and	w0, w17, #0xff
1005a11ec:     	cmp	w0, #0x7
1005a11f0:     	b.le	0x1005a1210 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe10>
1005a11f4:     	cmp	w0, #0xb
1005a11f8:     	b.gt	0x1005a122c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe2c>
1005a11fc:     	cmp	w0, #0x8
1005a1200:     	b.eq	0x1005a124c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe4c>
1005a1204:     	cmp	w0, #0xa
1005a1208:     	b.ne	0x1005a11e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1005a120c:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a1210:     	cmp	w0, #0x2
1005a1214:     	b.eq	0x1005a1264 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe64>
1005a1218:     	cmp	w0, #0x4
1005a121c:     	b.eq	0x1005a1254 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe54>
1005a1220:     	cmp	w0, #0x6
1005a1224:     	b.ne	0x1005a11e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1005a1228:     	b	0x1005a1244 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe44>
1005a122c:     	cmp	w0, #0xc
1005a1230:     	b.eq	0x1005a125c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe5c>
1005a1234:     	cmp	w0, #0xe
1005a1238:     	b.ne	0x1005a11e0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xde0>
1005a123c:     	orr	x9, x9, x8
1005a1240:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a1244:     	eor	x9, x9, x8
1005a1248:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a124c:     	and	x9, x9, x8
1005a1250:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a1254:     	bic	x9, x8, x9
1005a1258:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a125c:     	mov	x9, x8
1005a1260:     	b	0x1005a1268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xe68>
1005a1264:     	bic	x9, x9, x8
1005a1268:     	eor	x8, x11, x13
1005a126c:     	eor	x11, x14, x15
1005a1270:     	eor	x12, x12, x16
1005a1274:     	stp	x12, x11, [sp]
1005a1278:     	eor	x9, x9, x10
1005a127c:     	stp	x8, x9, [sp, #0x10]
1005a1280:     	b	0x1005a14cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10cc>
1005a1284:     	eor	x15, x12, x15
1005a1288:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a128c:     	bic	x15, x12, x15
1005a1290:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a1294:     	mov	x15, x12
1005a1298:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a129c:     	and	x15, x12, x15
1005a12a0:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a12a4:     	mov	x15, #0x0               ; =0
1005a12a8:     	b	0x1005a12b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xeb0>
1005a12ac:     	bic	x15, x15, x12
1005a12b0:     	mov	x12, #0x0               ; =0
1005a12b4:     	mov	w17, #0xf               ; =15
1005a12b8:     	b	0x1005a12c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xec4>
1005a12bc:     	eor	w17, w17, #0xf
1005a12c0:     	mvn	x12, x12
1005a12c4:     	and	w0, w17, #0xff
1005a12c8:     	cmp	w0, #0x7
1005a12cc:     	b.gt	0x1005a12e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xee8>
1005a12d0:     	cmp	w0, #0x3
1005a12d4:     	b.gt	0x1005a1304 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf04>
1005a12d8:     	cbz	w0, 0x1005a1350 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf50>
1005a12dc:     	cmp	w0, #0x2
1005a12e0:     	b.ne	0x1005a12bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1005a12e4:     	b	0x1005a1338 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf38>
1005a12e8:     	cmp	w0, #0xb
1005a12ec:     	b.gt	0x1005a1318 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf18>
1005a12f0:     	cmp	w0, #0x8
1005a12f4:     	b.eq	0x1005a1348 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf48>
1005a12f8:     	cmp	w0, #0xa
1005a12fc:     	b.ne	0x1005a12bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1005a1300:     	b	0x1005a1340 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf40>
1005a1304:     	cmp	w0, #0x4
1005a1308:     	b.eq	0x1005a1358 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf58>
1005a130c:     	cmp	w0, #0x6
1005a1310:     	b.ne	0x1005a12bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1005a1314:     	b	0x1005a1330 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf30>
1005a1318:     	cmp	w0, #0xc
1005a131c:     	b.eq	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1320:     	cmp	w0, #0xe
1005a1324:     	b.ne	0x1005a12bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xebc>
1005a1328:     	orr	x13, x14, x13
1005a132c:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1330:     	eor	x13, x14, x13
1005a1334:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1338:     	bic	x13, x14, x13
1005a133c:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1340:     	mov	x13, x14
1005a1344:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1348:     	and	x13, x14, x13
1005a134c:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1350:     	mov	x13, #0x0               ; =0
1005a1354:     	b	0x1005a135c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf5c>
1005a1358:     	bic	x13, x13, x14
1005a135c:     	mov	x14, #0x0               ; =0
1005a1360:     	mov	w17, #0xf               ; =15
1005a1364:     	b	0x1005a1370 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf70>
1005a1368:     	eor	w17, w17, #0xf
1005a136c:     	mvn	x14, x14
1005a1370:     	and	w0, w17, #0xff
1005a1374:     	cmp	w0, #0x7
1005a1378:     	b.gt	0x1005a1394 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf94>
1005a137c:     	cmp	w0, #0x3
1005a1380:     	b.gt	0x1005a13b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfb0>
1005a1384:     	cbz	w0, 0x1005a13fc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xffc>
1005a1388:     	cmp	w0, #0x2
1005a138c:     	b.ne	0x1005a1368 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1005a1390:     	b	0x1005a13e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfe4>
1005a1394:     	cmp	w0, #0xb
1005a1398:     	b.gt	0x1005a13c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfc4>
1005a139c:     	cmp	w0, #0x8
1005a13a0:     	b.eq	0x1005a13f4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xff4>
1005a13a4:     	cmp	w0, #0xa
1005a13a8:     	b.ne	0x1005a1368 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1005a13ac:     	b	0x1005a13ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfec>
1005a13b0:     	cmp	w0, #0x4
1005a13b4:     	b.eq	0x1005a1404 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1004>
1005a13b8:     	cmp	w0, #0x6
1005a13bc:     	b.ne	0x1005a1368 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1005a13c0:     	b	0x1005a13dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xfdc>
1005a13c4:     	cmp	w0, #0xc
1005a13c8:     	b.eq	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13cc:     	cmp	w0, #0xe
1005a13d0:     	b.ne	0x1005a1368 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0xf68>
1005a13d4:     	orr	x10, x11, x10
1005a13d8:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13dc:     	eor	x10, x11, x10
1005a13e0:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13e4:     	bic	x10, x11, x10
1005a13e8:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13ec:     	mov	x10, x11
1005a13f0:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13f4:     	and	x10, x11, x10
1005a13f8:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a13fc:     	mov	x10, #0x0               ; =0
1005a1400:     	b	0x1005a1408 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1008>
1005a1404:     	bic	x10, x10, x11
1005a1408:     	mov	x11, #0x0               ; =0
1005a140c:     	mov	w17, #0xf               ; =15
1005a1410:     	b	0x1005a141c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x101c>
1005a1414:     	eor	w17, w17, #0xf
1005a1418:     	mvn	x11, x11
1005a141c:     	and	w0, w17, #0xff
1005a1420:     	cmp	w0, #0x7
1005a1424:     	b.gt	0x1005a1440 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1040>
1005a1428:     	cmp	w0, #0x3
1005a142c:     	b.gt	0x1005a145c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x105c>
1005a1430:     	cbz	w0, 0x1005a14a8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10a8>
1005a1434:     	cmp	w0, #0x2
1005a1438:     	b.ne	0x1005a1414 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1005a143c:     	b	0x1005a1490 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1090>
1005a1440:     	cmp	w0, #0xb
1005a1444:     	b.gt	0x1005a1470 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1070>
1005a1448:     	cmp	w0, #0x8
1005a144c:     	b.eq	0x1005a14a0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10a0>
1005a1450:     	cmp	w0, #0xa
1005a1454:     	b.ne	0x1005a1414 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1005a1458:     	b	0x1005a1498 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1098>
1005a145c:     	cmp	w0, #0x4
1005a1460:     	b.eq	0x1005a14b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b0>
1005a1464:     	cmp	w0, #0x6
1005a1468:     	b.ne	0x1005a1414 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1005a146c:     	b	0x1005a1488 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1088>
1005a1470:     	cmp	w0, #0xc
1005a1474:     	b.eq	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a1478:     	cmp	w0, #0xe
1005a147c:     	b.ne	0x1005a1414 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x1014>
1005a1480:     	orr	x8, x9, x8
1005a1484:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a1488:     	eor	x8, x9, x8
1005a148c:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a1490:     	bic	x8, x9, x8
1005a1494:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a1498:     	mov	x8, x9
1005a149c:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a14a0:     	and	x8, x9, x8
1005a14a4:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a14a8:     	mov	x8, #0x0                ; =0
1005a14ac:     	b	0x1005a14b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_+0x10b4>
1005a14b0:     	bic	x8, x8, x9
1005a14b4:     	eor	x9, x10, x14
1005a14b8:     	eor	x10, x13, x12
1005a14bc:     	eor	x12, x15, x16
1005a14c0:     	stp	x12, x10, [sp]
1005a14c4:     	eor	x8, x8, x11
1005a14c8:     	stp	x9, x8, [sp, #0x10]
1005a14cc:     	mov	x1, sp
1005a14d0:     	mov	x0, x19
1005a14d4:     	bl	0x10059fbd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E4leafB6_>
1005a14d8:     	mov	x21, x0
1005a14dc:     	strb	w20, [sp, #0x4]
1005a14e0:     	str	w23, [sp]
1005a14e4:     	str	w24, [sp, #0x8]
1005a14e8:     	add	x0, x19, #0xa0
1005a14ec:     	mov	x1, sp
1005a14f0:     	mov	x2, x21
1005a14f4:     	bl	0x1005f2878 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1005a14f8:     	mov	x0, x21
1005a14fc:     	ldp	x29, x30, [sp, #0x50]
1005a1500:     	ldp	x20, x19, [sp, #0x40]
1005a1504:     	ldp	x22, x21, [sp, #0x30]
1005a1508:     	ldp	x24, x23, [sp, #0x20]
1005a150c:     	add	sp, sp, #0x60
1005a1510:     	ret
1005a1514:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a1518:     	add	x2, x2, #0x100
1005a151c:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1005a1520:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a1524:     	add	x2, x2, #0x2c8
1005a1528:     	mov	x0, x8
1005a152c:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1005a1530:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a1534:     	add	x2, x2, #0x2c8
1005a1538:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1005a153c:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a1540:     	add	x2, x2, #0x2b0
1005a1544:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1005a1548:     	nop
1005a154c:     	nop
1005a1550:     	nop
1005a1554:     	nop
1005a1558:     	nop
1005a155c:     	nop
1005a1560:     	nop
1005a1564:     	nop
1005a1568:     	nop
1005a156c:     	nop
1005a1570:     	nop
1005a1574:     	nop
1005a1578:     	nop
1005a157c:     	nop
