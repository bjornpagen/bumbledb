
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006cc540 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op>:
1006cc540:     	stp	x28, x27, [sp, #-0x60]!
1006cc544:     	stp	x26, x25, [sp, #0x10]
1006cc548:     	stp	x24, x23, [sp, #0x20]
1006cc54c:     	stp	x22, x21, [sp, #0x30]
1006cc550:     	stp	x20, x19, [sp, #0x40]
1006cc554:     	stp	x29, x30, [sp, #0x50]
1006cc558:     	add	x29, sp, #0x50
1006cc55c:     	sub	sp, sp, #0xc30
1006cc560:     	ldr	xzr, [sp]
1006cc564:     	mov	x21, x3
1006cc568:     	mov	x24, x2
1006cc56c:     	mov	x22, x1
1006cc570:     	mov	x19, x0
1006cc574:     	mov	w1, w2
1006cc578:     	mov	w2, w3
1006cc57c:     	mov	x0, x22
1006cc580:     	bl	0x1007989d8 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
1006cc584:     	cmp	x0, #0x1
1006cc588:     	b.ne	0x1006cc594 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x54>
1006cc58c:     	mov	x23, x1
1006cc590:     	b	0x1006ccf40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa00>
1006cc594:     	mov	w8, #0x9                ; =9
1006cc598:     	and	w8, w22, w8
1006cc59c:     	lsr	w9, w22, #1
1006cc5a0:     	bfi	w8, w9, #2, #1
1006cc5a4:     	and	w9, w9, #0x2
1006cc5a8:     	orr	w8, w8, w9
1006cc5ac:     	cmp	w24, w21
1006cc5b0:     	csel	w26, w24, w21, hi
1006cc5b4:     	csel	w21, w21, w24, hi
1006cc5b8:     	csel	w22, w8, w22, hi
1006cc5bc:     	ldr	x8, [x19, #0x100]
1006cc5c0:     	cbz	x8, 0x1006cc690 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x150>
1006cc5c4:     	mov	x8, #0x0                ; =0
1006cc5c8:     	and	x9, x22, #0xff
1006cc5cc:     	mov	x10, #0xa9c5            ; =43461
1006cc5d0:     	movk	x10, #0x2e62, lsl #16
1006cc5d4:     	movk	x10, #0x7aea, lsl #32
1006cc5d8:     	movk	x10, #0xf135, lsl #48
1006cc5dc:     	mul	x9, x9, x10
1006cc5e0:     	add	x9, x9, w21, uxtw
1006cc5e4:     	mul	x9, x9, x10
1006cc5e8:     	add	x9, x9, w26, uxtw
1006cc5ec:     	mul	x9, x9, x10
1006cc5f0:     	ror	x11, x9, #0x2c
1006cc5f4:     	lsr	x12, x11, #57
1006cc5f8:     	ldp	x10, x9, [x19, #0xe8]
1006cc5fc:     	dup.8b	v0, w12
1006cc600:     	movi.2d	v1, #0xffffffffffffffff
1006cc604:     	and	x11, x11, x9
1006cc608:     	ldr	d2, [x10, x11]
1006cc60c:     	cmeq.8b	v3, v2, v0
1006cc610:     	fmov	x12, d3
1006cc614:     	ands	x12, x12, #0x8080808080808080
1006cc618:     	b.eq	0x1006cc660 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x120>
1006cc61c:     	rbit	x13, x12
1006cc620:     	clz	x13, x13
1006cc624:     	add	x13, x11, x13, lsr #3
1006cc628:     	and	x13, x13, x9
1006cc62c:     	sub	x13, x10, x13, lsl #4
1006cc630:     	ldurb	w14, [x13, #-0xc]
1006cc634:     	cmp	w14, w22, uxtb
1006cc638:     	b.ne	0x1006cc654 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x114>
1006cc63c:     	ldur	w14, [x13, #-0x10]
1006cc640:     	cmp	w21, w14
1006cc644:     	b.ne	0x1006cc654 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x114>
1006cc648:     	ldur	w14, [x13, #-0x8]
1006cc64c:     	cmp	w26, w14
1006cc650:     	b.eq	0x1006cc790 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x250>
1006cc654:     	sub	x13, x12, #0x2
1006cc658:     	ands	x12, x13, x12
1006cc65c:     	b.ne	0x1006cc61c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xdc>
1006cc660:     	cmeq.8b	v2, v2, v1
1006cc664:     	fmov	x12, d2
1006cc668:     	cbnz	x12, 0x1006cc690 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x150>
1006cc66c:     	add	x8, x8, #0x8
1006cc670:     	add	x11, x11, x8
1006cc674:     	and	x11, x11, x9
1006cc678:     	ldr	d2, [x10, x11]
1006cc67c:     	cmeq.8b	v3, v2, v0
1006cc680:     	fmov	x12, d3
1006cc684:     	ands	x12, x12, #0x8080808080808080
1006cc688:     	b.ne	0x1006cc61c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xdc>
1006cc68c:     	b	0x1006cc660 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x120>
1006cc690:     	tbnz	w21, #0x1, 0x1006cc798 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x258>
1006cc694:     	ldr	x8, [x19, #0x110]
1006cc698:     	tbnz	w26, #0x1, 0x1006cc7b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x278>
1006cc69c:     	ldr	x0, [x19, #0x110]
1006cc6a0:     	cmp	x0, x8
1006cc6a4:     	csel	x23, x0, x8, lo
1006cc6a8:     	cmp	x23, x0
1006cc6ac:     	b.ne	0x1006cc7e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x2a8>
1006cc6b0:     	lsr	w8, w21, #2
1006cc6b4:     	ldr	x1, [x19, #0xa0]
1006cc6b8:     	cmp	x1, x8
1006cc6bc:     	b.ls	0x1006ccf9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa5c>
1006cc6c0:     	ldr	x10, [x19, #0x98]
1006cc6c4:     	ldr	x9, [x10, x8, lsl #3]
1006cc6c8:     	tbz	w21, #0x0, 0x1006cc6e4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1a4>
1006cc6cc:     	ldr	x8, [x19, #0x58]
1006cc6d0:     	cmp	x0, x8
1006cc6d4:     	b.hs	0x1006ccfac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa6c>
1006cc6d8:     	ldr	x8, [x19, #0x50]
1006cc6dc:     	ldr	x8, [x8, x0, lsl #3]
1006cc6e0:     	eor	x9, x8, x9
1006cc6e4:     	lsr	w8, w26, #2
1006cc6e8:     	cmp	x1, x8
1006cc6ec:     	b.ls	0x1006ccf9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa5c>
1006cc6f0:     	ldr	x10, [x10, x8, lsl #3]
1006cc6f4:     	tbz	w26, #0x0, 0x1006cc710 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1d0>
1006cc6f8:     	ldr	x1, [x19, #0x58]
1006cc6fc:     	cmp	x0, x1
1006cc700:     	b.hs	0x1006ccfbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa7c>
1006cc704:     	ldr	x8, [x19, #0x50]
1006cc708:     	ldr	x8, [x8, x0, lsl #3]
1006cc70c:     	eor	x10, x8, x10
1006cc710:     	mov	x8, #0x0                ; =0
1006cc714:     	mov	x11, x22
1006cc718:     	b	0x1006cc724 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1e4>
1006cc71c:     	eor	w11, w11, #0xf
1006cc720:     	mvn	x8, x8
1006cc724:     	and	w12, w11, #0xff
1006cc728:     	cmp	w12, #0x7
1006cc72c:     	b.gt	0x1006cc748 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x208>
1006cc730:     	cmp	w12, #0x3
1006cc734:     	b.gt	0x1006cc764 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x224>
1006cc738:     	cbz	w12, 0x1006ccf08 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9c8>
1006cc73c:     	cmp	w12, #0x2
1006cc740:     	b.ne	0x1006cc71c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1dc>
1006cc744:     	b	0x1006ccef0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9b0>
1006cc748:     	cmp	w12, #0xb
1006cc74c:     	b.gt	0x1006cc778 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x238>
1006cc750:     	cmp	w12, #0x8
1006cc754:     	b.eq	0x1006ccf00 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9c0>
1006cc758:     	cmp	w12, #0xa
1006cc75c:     	b.ne	0x1006cc71c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1dc>
1006cc760:     	b	0x1006ccef8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9b8>
1006cc764:     	cmp	w12, #0x4
1006cc768:     	b.eq	0x1006ccf10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d0>
1006cc76c:     	cmp	w12, #0x6
1006cc770:     	b.ne	0x1006cc71c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1dc>
1006cc774:     	b	0x1006ccee8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9a8>
1006cc778:     	cmp	w12, #0xc
1006cc77c:     	b.eq	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006cc780:     	cmp	w12, #0xe
1006cc784:     	b.ne	0x1006cc71c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x1dc>
1006cc788:     	orr	x9, x10, x9
1006cc78c:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006cc790:     	ldur	w23, [x13, #-0x4]
1006cc794:     	b	0x1006ccf40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa00>
1006cc798:     	lsr	w0, w21, #2
1006cc79c:     	ldr	x1, [x19, #0x70]
1006cc7a0:     	cmp	x1, x0
1006cc7a4:     	b.ls	0x1006ccf90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa50>
1006cc7a8:     	ldr	x8, [x19, #0x68]
1006cc7ac:     	lsl	x9, x0, #4
1006cc7b0:     	ldr	w8, [x8, x9]
1006cc7b4:     	tbz	w26, #0x1, 0x1006cc69c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x15c>
1006cc7b8:     	lsr	w0, w26, #2
1006cc7bc:     	ldr	x1, [x19, #0x70]
1006cc7c0:     	cmp	x1, x0
1006cc7c4:     	b.ls	0x1006ccf90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa50>
1006cc7c8:     	ldr	x9, [x19, #0x68]
1006cc7cc:     	lsl	x10, x0, #4
1006cc7d0:     	ldr	w9, [x9, x10]
1006cc7d4:     	ldr	x0, [x19, #0x110]
1006cc7d8:     	cmp	x9, x8
1006cc7dc:     	csel	x23, x9, x8, lo
1006cc7e0:     	cmp	x23, x0
1006cc7e4:     	b.eq	0x1006cc6b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x170>
1006cc7e8:     	str	xzr, [sp, #0x28]
1006cc7ec:     	str	wzr, [sp, #0x30]
1006cc7f0:     	str	xzr, [sp, #0x38]
1006cc7f4:     	str	wzr, [sp, #0x40]
1006cc7f8:     	str	xzr, [sp, #0x48]
1006cc7fc:     	str	wzr, [sp, #0x50]
1006cc800:     	str	xzr, [sp, #0x58]
1006cc804:     	str	wzr, [sp, #0x60]
1006cc808:     	str	xzr, [sp, #0x68]
1006cc80c:     	str	wzr, [sp, #0x70]
1006cc810:     	str	xzr, [sp, #0x78]
1006cc814:     	str	wzr, [sp, #0x80]
1006cc818:     	str	xzr, [sp, #0x88]
1006cc81c:     	str	wzr, [sp, #0x90]
1006cc820:     	str	xzr, [sp, #0x98]
1006cc824:     	str	wzr, [sp, #0xa0]
1006cc828:     	str	xzr, [sp, #0xa8]
1006cc82c:     	str	wzr, [sp, #0xb0]
1006cc830:     	str	xzr, [sp, #0xb8]
1006cc834:     	str	wzr, [sp, #0xc0]
1006cc838:     	str	xzr, [sp, #0xc8]
1006cc83c:     	str	wzr, [sp, #0xd0]
1006cc840:     	str	xzr, [sp, #0xd8]
1006cc844:     	str	wzr, [sp, #0xe0]
1006cc848:     	str	xzr, [sp, #0xe8]
1006cc84c:     	str	wzr, [sp, #0xf0]
1006cc850:     	str	xzr, [sp, #0xf8]
1006cc854:     	str	wzr, [sp, #0x100]
1006cc858:     	str	xzr, [sp, #0x108]
1006cc85c:     	str	wzr, [sp, #0x110]
1006cc860:     	str	xzr, [sp, #0x118]
1006cc864:     	str	wzr, [sp, #0x120]
1006cc868:     	str	xzr, [sp, #0x128]
1006cc86c:     	str	wzr, [sp, #0x130]
1006cc870:     	str	xzr, [sp, #0x138]
1006cc874:     	str	wzr, [sp, #0x140]
1006cc878:     	str	xzr, [sp, #0x148]
1006cc87c:     	str	wzr, [sp, #0x150]
1006cc880:     	str	xzr, [sp, #0x158]
1006cc884:     	str	wzr, [sp, #0x160]
1006cc888:     	str	xzr, [sp, #0x168]
1006cc88c:     	str	wzr, [sp, #0x170]
1006cc890:     	str	xzr, [sp, #0x178]
1006cc894:     	str	wzr, [sp, #0x180]
1006cc898:     	str	xzr, [sp, #0x188]
1006cc89c:     	str	wzr, [sp, #0x190]
1006cc8a0:     	str	xzr, [sp, #0x198]
1006cc8a4:     	str	wzr, [sp, #0x1a0]
1006cc8a8:     	str	xzr, [sp, #0x1a8]
1006cc8ac:     	str	wzr, [sp, #0x1b0]
1006cc8b0:     	str	xzr, [sp, #0x1b8]
1006cc8b4:     	str	wzr, [sp, #0x1c0]
1006cc8b8:     	str	xzr, [sp, #0x1c8]
1006cc8bc:     	str	wzr, [sp, #0x1d0]
1006cc8c0:     	str	xzr, [sp, #0x1d8]
1006cc8c4:     	str	wzr, [sp, #0x1e0]
1006cc8c8:     	str	xzr, [sp, #0x1e8]
1006cc8cc:     	str	wzr, [sp, #0x1f0]
1006cc8d0:     	str	xzr, [sp, #0x1f8]
1006cc8d4:     	str	wzr, [sp, #0x200]
1006cc8d8:     	str	xzr, [sp, #0x208]
1006cc8dc:     	str	wzr, [sp, #0x210]
1006cc8e0:     	str	xzr, [sp, #0x218]
1006cc8e4:     	str	wzr, [sp, #0x220]
1006cc8e8:     	str	xzr, [sp, #0x228]
1006cc8ec:     	str	wzr, [sp, #0x230]
1006cc8f0:     	str	xzr, [sp, #0x238]
1006cc8f4:     	str	wzr, [sp, #0x240]
1006cc8f8:     	str	xzr, [sp, #0x248]
1006cc8fc:     	str	wzr, [sp, #0x250]
1006cc900:     	str	xzr, [sp, #0x258]
1006cc904:     	str	wzr, [sp, #0x260]
1006cc908:     	str	xzr, [sp, #0x268]
1006cc90c:     	str	wzr, [sp, #0x270]
1006cc910:     	str	xzr, [sp, #0x278]
1006cc914:     	str	wzr, [sp, #0x280]
1006cc918:     	str	xzr, [sp, #0x288]
1006cc91c:     	str	wzr, [sp, #0x290]
1006cc920:     	str	xzr, [sp, #0x298]
1006cc924:     	str	wzr, [sp, #0x2a0]
1006cc928:     	str	xzr, [sp, #0x2a8]
1006cc92c:     	str	wzr, [sp, #0x2b0]
1006cc930:     	str	xzr, [sp, #0x2b8]
1006cc934:     	str	wzr, [sp, #0x2c0]
1006cc938:     	str	xzr, [sp, #0x2c8]
1006cc93c:     	str	wzr, [sp, #0x2d0]
1006cc940:     	str	xzr, [sp, #0x2d8]
1006cc944:     	str	wzr, [sp, #0x2e0]
1006cc948:     	str	xzr, [sp, #0x2e8]
1006cc94c:     	str	wzr, [sp, #0x2f0]
1006cc950:     	str	xzr, [sp, #0x2f8]
1006cc954:     	str	wzr, [sp, #0x300]
1006cc958:     	str	xzr, [sp, #0x308]
1006cc95c:     	str	wzr, [sp, #0x310]
1006cc960:     	str	xzr, [sp, #0x318]
1006cc964:     	str	wzr, [sp, #0x320]
1006cc968:     	str	xzr, [sp, #0x328]
1006cc96c:     	str	wzr, [sp, #0x330]
1006cc970:     	str	xzr, [sp, #0x338]
1006cc974:     	str	wzr, [sp, #0x340]
1006cc978:     	str	xzr, [sp, #0x348]
1006cc97c:     	str	wzr, [sp, #0x350]
1006cc980:     	str	xzr, [sp, #0x358]
1006cc984:     	str	wzr, [sp, #0x360]
1006cc988:     	str	xzr, [sp, #0x368]
1006cc98c:     	str	wzr, [sp, #0x370]
1006cc990:     	str	xzr, [sp, #0x378]
1006cc994:     	str	wzr, [sp, #0x380]
1006cc998:     	str	xzr, [sp, #0x388]
1006cc99c:     	str	wzr, [sp, #0x390]
1006cc9a0:     	str	xzr, [sp, #0x398]
1006cc9a4:     	str	wzr, [sp, #0x3a0]
1006cc9a8:     	str	xzr, [sp, #0x3a8]
1006cc9ac:     	str	wzr, [sp, #0x3b0]
1006cc9b0:     	str	xzr, [sp, #0x3b8]
1006cc9b4:     	str	wzr, [sp, #0x3c0]
1006cc9b8:     	str	xzr, [sp, #0x3c8]
1006cc9bc:     	str	wzr, [sp, #0x3d0]
1006cc9c0:     	str	xzr, [sp, #0x3d8]
1006cc9c4:     	str	wzr, [sp, #0x3e0]
1006cc9c8:     	str	xzr, [sp, #0x3e8]
1006cc9cc:     	str	wzr, [sp, #0x3f0]
1006cc9d0:     	str	xzr, [sp, #0x3f8]
1006cc9d4:     	str	wzr, [sp, #0x400]
1006cc9d8:     	str	xzr, [sp, #0x408]
1006cc9dc:     	str	wzr, [sp, #0x410]
1006cc9e0:     	str	xzr, [sp, #0x418]
1006cc9e4:     	str	wzr, [sp, #0x420]
1006cc9e8:     	str	xzr, [sp, #0x428]
1006cc9ec:     	str	wzr, [sp, #0x430]
1006cc9f0:     	str	xzr, [sp, #0x438]
1006cc9f4:     	str	wzr, [sp, #0x440]
1006cc9f8:     	str	xzr, [sp, #0x448]
1006cc9fc:     	str	wzr, [sp, #0x450]
1006cca00:     	str	xzr, [sp, #0x458]
1006cca04:     	str	wzr, [sp, #0x460]
1006cca08:     	str	xzr, [sp, #0x468]
1006cca0c:     	str	wzr, [sp, #0x470]
1006cca10:     	str	xzr, [sp, #0x478]
1006cca14:     	str	wzr, [sp, #0x480]
1006cca18:     	str	xzr, [sp, #0x488]
1006cca1c:     	str	wzr, [sp, #0x490]
1006cca20:     	str	xzr, [sp, #0x498]
1006cca24:     	str	wzr, [sp, #0x4a0]
1006cca28:     	str	xzr, [sp, #0x4a8]
1006cca2c:     	str	wzr, [sp, #0x4b0]
1006cca30:     	str	xzr, [sp, #0x4b8]
1006cca34:     	str	wzr, [sp, #0x4c0]
1006cca38:     	str	xzr, [sp, #0x4c8]
1006cca3c:     	str	wzr, [sp, #0x4d0]
1006cca40:     	str	xzr, [sp, #0x4d8]
1006cca44:     	str	wzr, [sp, #0x4e0]
1006cca48:     	str	xzr, [sp, #0x4e8]
1006cca4c:     	str	wzr, [sp, #0x4f0]
1006cca50:     	str	xzr, [sp, #0x4f8]
1006cca54:     	str	wzr, [sp, #0x500]
1006cca58:     	str	xzr, [sp, #0x508]
1006cca5c:     	str	wzr, [sp, #0x510]
1006cca60:     	str	xzr, [sp, #0x518]
1006cca64:     	str	wzr, [sp, #0x520]
1006cca68:     	str	xzr, [sp, #0x528]
1006cca6c:     	str	wzr, [sp, #0x530]
1006cca70:     	str	xzr, [sp, #0x538]
1006cca74:     	str	wzr, [sp, #0x540]
1006cca78:     	str	xzr, [sp, #0x548]
1006cca7c:     	str	wzr, [sp, #0x550]
1006cca80:     	str	xzr, [sp, #0x558]
1006cca84:     	str	wzr, [sp, #0x560]
1006cca88:     	str	xzr, [sp, #0x568]
1006cca8c:     	str	wzr, [sp, #0x570]
1006cca90:     	str	xzr, [sp, #0x578]
1006cca94:     	str	wzr, [sp, #0x580]
1006cca98:     	str	xzr, [sp, #0x588]
1006cca9c:     	str	wzr, [sp, #0x590]
1006ccaa0:     	str	xzr, [sp, #0x598]
1006ccaa4:     	str	wzr, [sp, #0x5a0]
1006ccaa8:     	str	xzr, [sp, #0x5a8]
1006ccaac:     	str	wzr, [sp, #0x5b0]
1006ccab0:     	str	xzr, [sp, #0x5b8]
1006ccab4:     	str	wzr, [sp, #0x5c0]
1006ccab8:     	str	xzr, [sp, #0x5c8]
1006ccabc:     	str	wzr, [sp, #0x5d0]
1006ccac0:     	str	xzr, [sp, #0x5d8]
1006ccac4:     	str	wzr, [sp, #0x5e0]
1006ccac8:     	str	xzr, [sp, #0x5e8]
1006ccacc:     	str	wzr, [sp, #0x5f0]
1006ccad0:     	str	xzr, [sp, #0x5f8]
1006ccad4:     	str	wzr, [sp, #0x600]
1006ccad8:     	str	xzr, [sp, #0x608]
1006ccadc:     	str	wzr, [sp, #0x610]
1006ccae0:     	str	xzr, [sp, #0x618]
1006ccae4:     	str	wzr, [sp, #0x620]
1006ccae8:     	str	xzr, [sp, #0x628]
1006ccaec:     	str	wzr, [sp, #0x630]
1006ccaf0:     	str	xzr, [sp, #0x638]
1006ccaf4:     	str	wzr, [sp, #0x640]
1006ccaf8:     	str	xzr, [sp, #0x648]
1006ccafc:     	str	wzr, [sp, #0x650]
1006ccb00:     	str	xzr, [sp, #0x658]
1006ccb04:     	str	wzr, [sp, #0x660]
1006ccb08:     	str	xzr, [sp, #0x668]
1006ccb0c:     	str	wzr, [sp, #0x670]
1006ccb10:     	str	xzr, [sp, #0x678]
1006ccb14:     	str	wzr, [sp, #0x680]
1006ccb18:     	str	xzr, [sp, #0x688]
1006ccb1c:     	str	wzr, [sp, #0x690]
1006ccb20:     	str	xzr, [sp, #0x698]
1006ccb24:     	str	wzr, [sp, #0x6a0]
1006ccb28:     	str	xzr, [sp, #0x6a8]
1006ccb2c:     	str	wzr, [sp, #0x6b0]
1006ccb30:     	str	xzr, [sp, #0x6b8]
1006ccb34:     	str	wzr, [sp, #0x6c0]
1006ccb38:     	str	xzr, [sp, #0x6c8]
1006ccb3c:     	str	wzr, [sp, #0x6d0]
1006ccb40:     	str	xzr, [sp, #0x6d8]
1006ccb44:     	str	wzr, [sp, #0x6e0]
1006ccb48:     	str	xzr, [sp, #0x6e8]
1006ccb4c:     	str	wzr, [sp, #0x6f0]
1006ccb50:     	str	xzr, [sp, #0x6f8]
1006ccb54:     	str	wzr, [sp, #0x700]
1006ccb58:     	str	xzr, [sp, #0x708]
1006ccb5c:     	str	wzr, [sp, #0x710]
1006ccb60:     	str	xzr, [sp, #0x718]
1006ccb64:     	str	wzr, [sp, #0x720]
1006ccb68:     	str	xzr, [sp, #0x728]
1006ccb6c:     	str	wzr, [sp, #0x730]
1006ccb70:     	str	xzr, [sp, #0x738]
1006ccb74:     	str	wzr, [sp, #0x740]
1006ccb78:     	str	xzr, [sp, #0x748]
1006ccb7c:     	str	wzr, [sp, #0x750]
1006ccb80:     	str	xzr, [sp, #0x758]
1006ccb84:     	str	wzr, [sp, #0x760]
1006ccb88:     	str	xzr, [sp, #0x768]
1006ccb8c:     	str	wzr, [sp, #0x770]
1006ccb90:     	str	xzr, [sp, #0x778]
1006ccb94:     	str	wzr, [sp, #0x780]
1006ccb98:     	str	xzr, [sp, #0x788]
1006ccb9c:     	str	wzr, [sp, #0x790]
1006ccba0:     	str	xzr, [sp, #0x798]
1006ccba4:     	str	wzr, [sp, #0x7a0]
1006ccba8:     	str	xzr, [sp, #0x7a8]
1006ccbac:     	str	wzr, [sp, #0x7b0]
1006ccbb0:     	str	xzr, [sp, #0x7b8]
1006ccbb4:     	str	wzr, [sp, #0x7c0]
1006ccbb8:     	str	xzr, [sp, #0x7c8]
1006ccbbc:     	str	wzr, [sp, #0x7d0]
1006ccbc0:     	str	xzr, [sp, #0x7d8]
1006ccbc4:     	str	wzr, [sp, #0x7e0]
1006ccbc8:     	str	xzr, [sp, #0x7e8]
1006ccbcc:     	str	wzr, [sp, #0x7f0]
1006ccbd0:     	str	xzr, [sp, #0x7f8]
1006ccbd4:     	str	wzr, [sp, #0x800]
1006ccbd8:     	str	xzr, [sp, #0x808]
1006ccbdc:     	str	wzr, [sp, #0x810]
1006ccbe0:     	str	xzr, [sp, #0x818]
1006ccbe4:     	str	wzr, [sp, #0x820]
1006ccbe8:     	str	xzr, [sp, #0x828]
1006ccbec:     	str	wzr, [sp, #0x830]
1006ccbf0:     	str	xzr, [sp, #0x838]
1006ccbf4:     	str	wzr, [sp, #0x840]
1006ccbf8:     	str	xzr, [sp, #0x848]
1006ccbfc:     	str	wzr, [sp, #0x850]
1006ccc00:     	str	xzr, [sp, #0x858]
1006ccc04:     	str	wzr, [sp, #0x860]
1006ccc08:     	str	xzr, [sp, #0x868]
1006ccc0c:     	str	wzr, [sp, #0x870]
1006ccc10:     	str	xzr, [sp, #0x878]
1006ccc14:     	str	wzr, [sp, #0x880]
1006ccc18:     	str	xzr, [sp, #0x888]
1006ccc1c:     	str	wzr, [sp, #0x890]
1006ccc20:     	str	xzr, [sp, #0x898]
1006ccc24:     	str	wzr, [sp, #0x8a0]
1006ccc28:     	str	xzr, [sp, #0x8a8]
1006ccc2c:     	str	wzr, [sp, #0x8b0]
1006ccc30:     	str	xzr, [sp, #0x8b8]
1006ccc34:     	str	wzr, [sp, #0x8c0]
1006ccc38:     	str	xzr, [sp, #0x8c8]
1006ccc3c:     	str	wzr, [sp, #0x8d0]
1006ccc40:     	str	xzr, [sp, #0x8d8]
1006ccc44:     	str	wzr, [sp, #0x8e0]
1006ccc48:     	str	xzr, [sp, #0x8e8]
1006ccc4c:     	str	wzr, [sp, #0x8f0]
1006ccc50:     	str	xzr, [sp, #0x8f8]
1006ccc54:     	str	wzr, [sp, #0x900]
1006ccc58:     	str	xzr, [sp, #0x908]
1006ccc5c:     	str	wzr, [sp, #0x910]
1006ccc60:     	str	xzr, [sp, #0x918]
1006ccc64:     	str	wzr, [sp, #0x920]
1006ccc68:     	str	xzr, [sp, #0x928]
1006ccc6c:     	str	wzr, [sp, #0x930]
1006ccc70:     	str	xzr, [sp, #0x938]
1006ccc74:     	str	wzr, [sp, #0x940]
1006ccc78:     	str	xzr, [sp, #0x948]
1006ccc7c:     	str	wzr, [sp, #0x950]
1006ccc80:     	str	xzr, [sp, #0x958]
1006ccc84:     	str	wzr, [sp, #0x960]
1006ccc88:     	str	xzr, [sp, #0x968]
1006ccc8c:     	str	wzr, [sp, #0x970]
1006ccc90:     	str	xzr, [sp, #0x978]
1006ccc94:     	str	wzr, [sp, #0x980]
1006ccc98:     	str	xzr, [sp, #0x988]
1006ccc9c:     	str	wzr, [sp, #0x990]
1006ccca0:     	str	xzr, [sp, #0x998]
1006ccca4:     	str	wzr, [sp, #0x9a0]
1006ccca8:     	str	xzr, [sp, #0x9a8]
1006cccac:     	str	wzr, [sp, #0x9b0]
1006cccb0:     	str	xzr, [sp, #0x9b8]
1006cccb4:     	str	wzr, [sp, #0x9c0]
1006cccb8:     	str	xzr, [sp, #0x9c8]
1006cccbc:     	str	wzr, [sp, #0x9d0]
1006cccc0:     	str	xzr, [sp, #0x9d8]
1006cccc4:     	str	wzr, [sp, #0x9e0]
1006cccc8:     	str	xzr, [sp, #0x9e8]
1006ccccc:     	str	wzr, [sp, #0x9f0]
1006cccd0:     	str	xzr, [sp, #0x9f8]
1006cccd4:     	str	wzr, [sp, #0xa00]
1006cccd8:     	str	xzr, [sp, #0xa08]
1006cccdc:     	str	wzr, [sp, #0xa10]
1006ccce0:     	str	xzr, [sp, #0xa18]
1006ccce4:     	str	wzr, [sp, #0xa20]
1006ccce8:     	str	xzr, [sp, #0xa28]
1006cccec:     	str	wzr, [sp, #0xa30]
1006cccf0:     	str	xzr, [sp, #0xa38]
1006cccf4:     	str	wzr, [sp, #0xa40]
1006cccf8:     	str	xzr, [sp, #0xa48]
1006cccfc:     	str	wzr, [sp, #0xa50]
1006ccd00:     	str	xzr, [sp, #0xa58]
1006ccd04:     	str	wzr, [sp, #0xa60]
1006ccd08:     	str	xzr, [sp, #0xa68]
1006ccd0c:     	str	wzr, [sp, #0xa70]
1006ccd10:     	str	xzr, [sp, #0xa78]
1006ccd14:     	str	wzr, [sp, #0xa80]
1006ccd18:     	str	xzr, [sp, #0xa88]
1006ccd1c:     	str	wzr, [sp, #0xa90]
1006ccd20:     	str	xzr, [sp, #0xa98]
1006ccd24:     	str	wzr, [sp, #0xaa0]
1006ccd28:     	str	xzr, [sp, #0xaa8]
1006ccd2c:     	str	wzr, [sp, #0xab0]
1006ccd30:     	str	xzr, [sp, #0xab8]
1006ccd34:     	str	wzr, [sp, #0xac0]
1006ccd38:     	str	xzr, [sp, #0xac8]
1006ccd3c:     	str	wzr, [sp, #0xad0]
1006ccd40:     	str	xzr, [sp, #0xad8]
1006ccd44:     	str	wzr, [sp, #0xae0]
1006ccd48:     	str	xzr, [sp, #0xae8]
1006ccd4c:     	str	wzr, [sp, #0xaf0]
1006ccd50:     	str	xzr, [sp, #0xaf8]
1006ccd54:     	str	wzr, [sp, #0xb00]
1006ccd58:     	str	xzr, [sp, #0xb08]
1006ccd5c:     	str	wzr, [sp, #0xb10]
1006ccd60:     	str	xzr, [sp, #0xb18]
1006ccd64:     	str	wzr, [sp, #0xb20]
1006ccd68:     	str	xzr, [sp, #0xb28]
1006ccd6c:     	str	wzr, [sp, #0xb30]
1006ccd70:     	str	xzr, [sp, #0xb38]
1006ccd74:     	str	wzr, [sp, #0xb40]
1006ccd78:     	str	xzr, [sp, #0xb48]
1006ccd7c:     	str	wzr, [sp, #0xb50]
1006ccd80:     	str	xzr, [sp, #0xb58]
1006ccd84:     	str	wzr, [sp, #0xb60]
1006ccd88:     	str	xzr, [sp, #0xb68]
1006ccd8c:     	str	wzr, [sp, #0xb70]
1006ccd90:     	str	xzr, [sp, #0xb78]
1006ccd94:     	str	wzr, [sp, #0xb80]
1006ccd98:     	str	xzr, [sp, #0xb88]
1006ccd9c:     	str	wzr, [sp, #0xb90]
1006ccda0:     	str	xzr, [sp, #0xb98]
1006ccda4:     	str	wzr, [sp, #0xba0]
1006ccda8:     	str	xzr, [sp, #0xba8]
1006ccdac:     	str	wzr, [sp, #0xbb0]
1006ccdb0:     	str	xzr, [sp, #0xbb8]
1006ccdb4:     	str	wzr, [sp, #0xbc0]
1006ccdb8:     	str	xzr, [sp, #0xbc8]
1006ccdbc:     	str	wzr, [sp, #0xbd0]
1006ccdc0:     	str	xzr, [sp, #0xbd8]
1006ccdc4:     	str	wzr, [sp, #0xbe0]
1006ccdc8:     	str	xzr, [sp, #0xbe8]
1006ccdcc:     	str	wzr, [sp, #0xbf0]
1006ccdd0:     	str	xzr, [sp, #0xbf8]
1006ccdd4:     	str	wzr, [sp, #0xc00]
1006ccdd8:     	str	xzr, [sp, #0xc08]
1006ccddc:     	str	wzr, [sp, #0xc10]
1006ccde0:     	str	xzr, [sp, #0xc18]
1006ccde4:     	str	wzr, [sp, #0xc20]
1006ccde8:     	add	x20, sp, #0x28
1006ccdec:     	add	x3, sp, #0x28
1006ccdf0:     	mov	x0, x19
1006ccdf4:     	mov	x1, x21
1006ccdf8:     	mov	x2, x23
1006ccdfc:     	bl	0x1006c8780 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411branches_at>
1006cce00:     	mov	x25, x0
1006cce04:     	add	x3, sp, #0x428
1006cce08:     	mov	x0, x19
1006cce0c:     	mov	x1, x26
1006cce10:     	mov	x2, x23
1006cce14:     	bl	0x1006c8780 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411branches_at>
1006cce18:     	cbz	x25, 0x1006cceb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x978>
1006cce1c:     	cbz	x0, 0x1006cceb8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x978>
1006cce20:     	str	x23, [sp, #0x8]
1006cce24:     	stp	w21, w26, [sp, #0x10]
1006cce28:     	mov	x24, #0x0               ; =0
1006cce2c:     	add	x9, x20, x25, lsl #4
1006cce30:     	lsl	x8, x0, #4
1006cce34:     	stp	x9, x8, [sp, #0x18]
1006cce38:     	add	x8, sp, #0x428
1006cce3c:     	add	x25, x8, #0x8
1006cce40:     	add	x20, sp, #0x28
1006cce44:     	add	x23, sp, #0x828
1006cce48:     	b	0x1006cce58 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x918>
1006cce4c:     	ldr	x8, [sp, #0x18]
1006cce50:     	cmp	x20, x8
1006cce54:     	b.eq	0x1006ccec0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x980>
1006cce58:     	mov	x28, x20
1006cce5c:     	add	x20, x20, #0x10
1006cce60:     	ldr	x26, [sp, #0x20]
1006cce64:     	mov	x27, x25
1006cce68:     	b	0x1006cce78 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x938>
1006cce6c:     	add	x27, x27, #0x10
1006cce70:     	subs	x26, x26, #0x10
1006cce74:     	b.eq	0x1006cce4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x90c>
1006cce78:     	ldr	x8, [x28]
1006cce7c:     	ldur	x9, [x27, #-0x8]
1006cce80:     	ands	x21, x9, x8
1006cce84:     	b.eq	0x1006cce6c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x92c>
1006cce88:     	ldr	w2, [x28, #0x8]
1006cce8c:     	ldr	w3, [x27]
1006cce90:     	mov	x0, x19
1006cce94:     	mov	x1, x22
1006cce98:     	bl	0x1006cc540 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op>
1006cce9c:     	cmp	x24, #0x3f
1006ccea0:     	b.hi	0x1006ccf7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa3c>
1006ccea4:     	add	x8, x23, x24, lsl #4
1006ccea8:     	str	x21, [x8]
1006cceac:     	str	w0, [x8, #0x8]
1006cceb0:     	add	x24, x24, #0x1
1006cceb4:     	b	0x1006cce6c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x92c>
1006cceb8:     	mov	x24, #0x0               ; =0
1006ccebc:     	b	0x1006cced0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x990>
1006ccec0:     	cmp	x24, #0x41
1006ccec4:     	b.hs	0x1006ccf64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0xa24>
1006ccec8:     	ldp	w21, w26, [sp, #0x10]
1006ccecc:     	ldr	x23, [sp, #0x8]
1006cced0:     	add	x2, sp, #0x828
1006cced4:     	mov	x0, x19
1006cced8:     	mov	x1, x23
1006ccedc:     	mov	x3, x24
1006ccee0:     	bl	0x1006cb1cc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block642mk>
1006ccee4:     	b	0x1006ccf20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9e0>
1006ccee8:     	eor	x9, x10, x9
1006cceec:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006ccef0:     	bic	x9, x10, x9
1006ccef4:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006ccef8:     	mov	x9, x10
1006ccefc:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006ccf00:     	and	x9, x10, x9
1006ccf04:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006ccf08:     	mov	x9, #0x0                ; =0
1006ccf0c:     	b	0x1006ccf14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block646raw_op+0x9d4>
1006ccf10:     	bic	x9, x9, x10
1006ccf14:     	eor	x1, x9, x8
1006ccf18:     	mov	x0, x19
1006ccf1c:     	bl	0x1006cbe4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block644leaf>
1006ccf20:     	mov	x23, x0
1006ccf24:     	strb	w22, [sp, #0x82c]
1006ccf28:     	str	w21, [sp, #0x828]
1006ccf2c:     	str	w26, [sp, #0x830]
1006ccf30:     	add	x0, x19, #0xe8
1006ccf34:     	add	x1, sp, #0x828
1006ccf38:     	mov	x2, x23
1006ccf3c:     	bl	0x1007284d8 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006ccf40:     	mov	x0, x23
1006ccf44:     	add	sp, sp, #0xc30
1006ccf48:     	ldp	x29, x30, [sp, #0x50]
1006ccf4c:     	ldp	x20, x19, [sp, #0x40]
1006ccf50:     	ldp	x22, x21, [sp, #0x30]
1006ccf54:     	ldp	x24, x23, [sp, #0x20]
1006ccf58:     	ldp	x26, x25, [sp, #0x10]
1006ccf5c:     	ldp	x28, x27, [sp], #0x60
1006ccf60:     	ret
1006ccf64:     	adrp	x3, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccf68:     	add	x3, x3, #0x610
1006ccf6c:     	mov	x0, #0x0                ; =0
1006ccf70:     	mov	x1, x24
1006ccf74:     	mov	w2, #0x40               ; =64
1006ccf78:     	bl	0x100c9afd4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
1006ccf7c:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccf80:     	add	x2, x2, #0x5f8
1006ccf84:     	mov	x0, x24
1006ccf88:     	mov	w1, #0x40               ; =64
1006ccf8c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006ccf90:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccf94:     	add	x2, x2, #0x238
1006ccf98:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006ccf9c:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccfa0:     	add	x2, x2, #0x568
1006ccfa4:     	mov	x0, x8
1006ccfa8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006ccfac:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccfb0:     	add	x2, x2, #0x580
1006ccfb4:     	mov	x1, x8
1006ccfb8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006ccfbc:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ccfc0:     	add	x2, x2, #0x580
1006ccfc4:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
