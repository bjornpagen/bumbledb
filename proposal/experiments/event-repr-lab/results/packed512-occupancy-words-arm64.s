
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bd85d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>:
100bd85d0:     	sub	sp, sp, #0x90
100bd85d4:     	stp	x28, x27, [sp, #0x30]
100bd85d8:     	stp	x26, x25, [sp, #0x40]
100bd85dc:     	stp	x24, x23, [sp, #0x50]
100bd85e0:     	stp	x22, x21, [sp, #0x60]
100bd85e4:     	stp	x20, x19, [sp, #0x70]
100bd85e8:     	stp	x29, x30, [sp, #0x80]
100bd85ec:     	add	x29, sp, #0x80
100bd85f0:     	cbz	w1, 0x100bd89e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x414>
100bd85f4:     	ldr	x8, [x4, #0x18]
100bd85f8:     	cbz	x8, 0x100bd86c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bd85fc:     	mov	x8, #0x0                ; =0
100bd8600:     	mov	w9, w1
100bd8604:     	mov	x10, #0xa9c5            ; =43461
100bd8608:     	movk	x10, #0x2e62, lsl #16
100bd860c:     	movk	x10, #0x7aea, lsl #32
100bd8610:     	movk	x10, #0xf135, lsl #48
100bd8614:     	mul	x9, x9, x10
100bd8618:     	add	x9, x9, w2, uxtw
100bd861c:     	mul	x9, x9, x10
100bd8620:     	add	x9, x9, w3, uxtw
100bd8624:     	mul	x9, x9, x10
100bd8628:     	ror	x11, x9, #0x2c
100bd862c:     	lsr	x12, x11, #57
100bd8630:     	ldp	x10, x9, [x4]
100bd8634:     	dup.8b	v0, w12
100bd8638:     	movi.2d	v1, #0xffffffffffffffff
100bd863c:     	and	x11, x11, x9
100bd8640:     	ldr	d2, [x10, x11]
100bd8644:     	cmeq.8b	v3, v2, v0
100bd8648:     	fmov	x12, d3
100bd864c:     	ands	x12, x12, #0x8080808080808080
100bd8650:     	b.eq	0x100bd8698 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bd8654:     	rbit	x13, x12
100bd8658:     	clz	x13, x13
100bd865c:     	add	x13, x11, x13, lsr #3
100bd8660:     	and	x13, x13, x9
100bd8664:     	sub	x13, x10, x13, lsl #4
100bd8668:     	ldur	w14, [x13, #-0x10]
100bd866c:     	cmp	w1, w14
100bd8670:     	b.ne	0x100bd868c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bd8674:     	ldur	w14, [x13, #-0xc]
100bd8678:     	cmp	w2, w14
100bd867c:     	b.ne	0x100bd868c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bd8680:     	ldur	w14, [x13, #-0x8]
100bd8684:     	cmp	w3, w14
100bd8688:     	b.eq	0x100bd89ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x41c>
100bd868c:     	sub	x13, x12, #0x2
100bd8690:     	ands	x12, x13, x12
100bd8694:     	b.ne	0x100bd8654 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bd8698:     	cmeq.8b	v2, v2, v1
100bd869c:     	fmov	x12, d2
100bd86a0:     	cbnz	x12, 0x100bd86c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bd86a4:     	add	x8, x8, #0x8
100bd86a8:     	add	x11, x11, x8
100bd86ac:     	and	x11, x11, x9
100bd86b0:     	ldr	d2, [x10, x11]
100bd86b4:     	cmeq.8b	v3, v2, v0
100bd86b8:     	fmov	x12, d3
100bd86bc:     	ands	x12, x12, #0x8080808080808080
100bd86c0:     	b.ne	0x100bd8654 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bd86c4:     	b	0x100bd8698 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bd86c8:     	tbnz	w1, #0x1, 0x100bd89f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x424>
100bd86cc:     	ldr	x10, [x0, #0xc8]
100bd86d0:     	tbnz	w2, #0x1, 0x100bd8a14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x444>
100bd86d4:     	ldr	x8, [x0, #0xc8]
100bd86d8:     	cmp	x8, x10
100bd86dc:     	csel	x10, x8, x10, lo
100bd86e0:     	tbnz	w3, #0x1, 0x100bd8a3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x46c>
100bd86e4:     	ldr	x8, [x0, #0xc8]
100bd86e8:     	cmp	x8, x10
100bd86ec:     	csel	x21, x8, x10, lo
100bd86f0:     	cmp	x21, x8
100bd86f4:     	b.ne	0x100bd8a6c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x49c>
100bd86f8:     	lsr	w9, w1, #2
100bd86fc:     	ldr	x8, [x0, #0x58]
100bd8700:     	cmp	x8, x9
100bd8704:     	b.ls	0x100bd8cd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bd8708:     	ldr	x23, [x0, #0x50]
100bd870c:     	add	x9, x23, x9, lsl #6
100bd8710:     	ldp	x7, x5, [x9]
100bd8714:     	ldp	x16, x14, [x9, #0x10]
100bd8718:     	ldp	x13, x12, [x9, #0x20]
100bd871c:     	ldp	x11, x10, [x9, #0x30]
100bd8720:     	tbz	w1, #0x0, 0x100bd8754 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x184>
100bd8724:     	ldp	x9, x15, [x0, #0xd8]
100bd8728:     	eor	x7, x9, x7
100bd872c:     	eor	x5, x15, x5
100bd8730:     	ldp	x9, x15, [x0, #0xe8]
100bd8734:     	eor	x16, x9, x16
100bd8738:     	eor	x14, x15, x14
100bd873c:     	ldp	x9, x15, [x0, #0xf8]
100bd8740:     	eor	x13, x9, x13
100bd8744:     	eor	x12, x15, x12
100bd8748:     	ldp	x9, x15, [x0, #0x108]
100bd874c:     	eor	x11, x9, x11
100bd8750:     	eor	x10, x15, x10
100bd8754:     	lsr	w9, w2, #2
100bd8758:     	cmp	x8, x9
100bd875c:     	b.ls	0x100bd8cd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bd8760:     	add	x9, x23, x9, lsl #6
100bd8764:     	ldp	x24, x22, [x9]
100bd8768:     	ldp	x21, x20, [x9, #0x10]
100bd876c:     	ldp	x19, x6, [x9, #0x20]
100bd8770:     	ldp	x17, x15, [x9, #0x30]
100bd8774:     	tbz	w2, #0x0, 0x100bd87a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x1d8>
100bd8778:     	ldp	x9, x25, [x0, #0xd8]
100bd877c:     	eor	x24, x9, x24
100bd8780:     	eor	x22, x25, x22
100bd8784:     	ldp	x9, x25, [x0, #0xe8]
100bd8788:     	eor	x21, x9, x21
100bd878c:     	eor	x20, x25, x20
100bd8790:     	ldp	x9, x25, [x0, #0xf8]
100bd8794:     	eor	x19, x9, x19
100bd8798:     	eor	x6, x25, x6
100bd879c:     	ldp	x9, x25, [x0, #0x108]
100bd87a0:     	eor	x17, x9, x17
100bd87a4:     	eor	x15, x25, x15
100bd87a8:     	lsr	w9, w3, #2
100bd87ac:     	cmp	x8, x9
100bd87b0:     	b.ls	0x100bd8cd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bd87b4:     	stp	x15, x10, [sp]
100bd87b8:     	add	x8, x23, x9, lsl #6
100bd87bc:     	ldp	x30, x28, [x8]
100bd87c0:     	ldp	x27, x26, [x8, #0x10]
100bd87c4:     	ldp	x25, x23, [x8, #0x20]
100bd87c8:     	ldp	x9, x8, [x8, #0x30]
100bd87cc:     	stp	x11, x12, [sp, #0x10]
100bd87d0:     	mov	x15, x13
100bd87d4:     	tbz	w3, #0x0, 0x100bd8808 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x238>
100bd87d8:     	ldp	x10, x11, [x0, #0xd8]
100bd87dc:     	eor	x30, x10, x30
100bd87e0:     	eor	x28, x11, x28
100bd87e4:     	ldp	x10, x11, [x0, #0xe8]
100bd87e8:     	eor	x27, x10, x27
100bd87ec:     	eor	x26, x11, x26
100bd87f0:     	ldp	x10, x11, [x0, #0xf8]
100bd87f4:     	eor	x25, x10, x25
100bd87f8:     	eor	x23, x11, x23
100bd87fc:     	ldp	x10, x11, [x0, #0x108]
100bd8800:     	eor	x9, x10, x9
100bd8804:     	eor	x8, x11, x8
100bd8808:     	bic	x10, x7, x24
100bd880c:     	tst	x10, x30
100bd8810:     	mov	w0, #0x2                ; =2
100bd8814:     	csel	w11, wzr, w0, eq
100bd8818:     	and	x24, x24, x7
100bd881c:     	bics	xzr, x24, x30
100bd8820:     	mov	w7, #0x4                ; =4
100bd8824:     	csel	w12, wzr, w7, eq
100bd8828:     	tst	x24, x30
100bd882c:     	mov	w24, #0x8               ; =8
100bd8830:     	csel	w13, wzr, w24, eq
100bd8834:     	bics	xzr, x10, x30
100bd8838:     	cinc	w10, w11, ne
100bd883c:     	orr	w11, w12, w13
100bd8840:     	orr	w30, w10, w11
100bd8844:     	cmp	w30, #0xf
100bd8848:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd884c:     	bic	x10, x5, x22
100bd8850:     	tst	x10, x28
100bd8854:     	csel	w11, wzr, w0, eq
100bd8858:     	and	x12, x22, x5
100bd885c:     	bics	xzr, x12, x28
100bd8860:     	csel	w13, wzr, w7, eq
100bd8864:     	tst	x12, x28
100bd8868:     	csel	w12, wzr, w24, eq
100bd886c:     	bics	xzr, x10, x28
100bd8870:     	cinc	w10, w11, ne
100bd8874:     	orr	w11, w13, w12
100bd8878:     	orr	w10, w10, w11
100bd887c:     	orr	w5, w10, w30
100bd8880:     	cmp	w5, #0xf
100bd8884:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd8888:     	bic	x10, x16, x21
100bd888c:     	tst	x10, x27
100bd8890:     	csel	w11, wzr, w0, eq
100bd8894:     	and	x12, x21, x16
100bd8898:     	bics	xzr, x12, x27
100bd889c:     	mov	w16, #0x4               ; =4
100bd88a0:     	csel	w13, wzr, w16, eq
100bd88a4:     	tst	x12, x27
100bd88a8:     	mov	w7, #0x8                ; =8
100bd88ac:     	csel	w12, wzr, w7, eq
100bd88b0:     	bics	xzr, x10, x27
100bd88b4:     	cinc	w10, w11, ne
100bd88b8:     	orr	w11, w13, w12
100bd88bc:     	orr	w10, w10, w11
100bd88c0:     	orr	w5, w10, w5
100bd88c4:     	cmp	w5, #0xf
100bd88c8:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd88cc:     	bic	x10, x14, x20
100bd88d0:     	tst	x10, x26
100bd88d4:     	csel	w11, wzr, w0, eq
100bd88d8:     	and	x12, x20, x14
100bd88dc:     	bics	xzr, x12, x26
100bd88e0:     	csel	w13, wzr, w16, eq
100bd88e4:     	tst	x12, x26
100bd88e8:     	csel	w12, wzr, w7, eq
100bd88ec:     	bics	xzr, x10, x26
100bd88f0:     	cinc	w10, w11, ne
100bd88f4:     	orr	w11, w13, w12
100bd88f8:     	orr	w10, w10, w11
100bd88fc:     	orr	w16, w10, w5
100bd8900:     	cmp	w16, #0xf
100bd8904:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd8908:     	bic	x10, x15, x19
100bd890c:     	tst	x10, x25
100bd8910:     	mov	w14, #0x2               ; =2
100bd8914:     	csel	w11, wzr, w14, eq
100bd8918:     	and	x12, x19, x15
100bd891c:     	bics	xzr, x12, x25
100bd8920:     	mov	w13, #0x4               ; =4
100bd8924:     	csel	w5, wzr, w13, eq
100bd8928:     	tst	x12, x25
100bd892c:     	mov	w0, #0x8                ; =8
100bd8930:     	csel	w12, wzr, w0, eq
100bd8934:     	bics	xzr, x10, x25
100bd8938:     	cinc	w10, w11, ne
100bd893c:     	orr	w11, w5, w12
100bd8940:     	orr	w10, w10, w11
100bd8944:     	orr	w16, w10, w16
100bd8948:     	cmp	w16, #0xf
100bd894c:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd8950:     	ldr	x12, [sp, #0x18]
100bd8954:     	bic	x10, x12, x6
100bd8958:     	tst	x10, x23
100bd895c:     	csel	w11, wzr, w14, eq
100bd8960:     	and	x12, x6, x12
100bd8964:     	bics	xzr, x12, x23
100bd8968:     	csel	w13, wzr, w13, eq
100bd896c:     	tst	x12, x23
100bd8970:     	csel	w12, wzr, w0, eq
100bd8974:     	bics	xzr, x10, x23
100bd8978:     	cinc	w10, w11, ne
100bd897c:     	orr	w11, w13, w12
100bd8980:     	orr	w10, w10, w11
100bd8984:     	orr	w13, w10, w16
100bd8988:     	cmp	w13, #0xf
100bd898c:     	b.eq	0x100bd89dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bd8990:     	ldr	x11, [sp, #0x10]
100bd8994:     	bic	x10, x11, x17
100bd8998:     	tst	x10, x9
100bd899c:     	mov	w12, #0x2               ; =2
100bd89a0:     	csel	w16, wzr, w12, eq
100bd89a4:     	and	x14, x17, x11
100bd89a8:     	bics	xzr, x14, x9
100bd89ac:     	mov	w11, #0x4               ; =4
100bd89b0:     	csel	w17, wzr, w11, eq
100bd89b4:     	tst	x14, x9
100bd89b8:     	mov	w14, #0x8               ; =8
100bd89bc:     	csel	w0, wzr, w14, eq
100bd89c0:     	bics	xzr, x10, x9
100bd89c4:     	cinc	w9, w16, ne
100bd89c8:     	orr	w10, w17, w0
100bd89cc:     	orr	w9, w9, w10
100bd89d0:     	orr	w9, w9, w13
100bd89d4:     	cmp	w9, #0xf
100bd89d8:     	b.ne	0x100bd8c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6b0>
100bd89dc:     	mov	w20, #0xf               ; =15
100bd89e0:     	b	0x100bd8c44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bd89e4:     	mov	w20, #0x0               ; =0
100bd89e8:     	b	0x100bd8c5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bd89ec:     	ldurb	w20, [x13, #-0x4]
100bd89f0:     	b	0x100bd8c5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bd89f4:     	lsr	w8, w1, #2
100bd89f8:     	ldr	x9, [x0, #0x40]
100bd89fc:     	cmp	x9, x8
100bd8a00:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8a04:     	ldr	x9, [x0, #0x38]
100bd8a08:     	lsl	x8, x8, #4
100bd8a0c:     	ldr	w10, [x9, x8]
100bd8a10:     	tbz	w2, #0x1, 0x100bd86d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x104>
100bd8a14:     	lsr	w8, w2, #2
100bd8a18:     	ldr	x9, [x0, #0x40]
100bd8a1c:     	cmp	x9, x8
100bd8a20:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8a24:     	ldr	x9, [x0, #0x38]
100bd8a28:     	lsl	x8, x8, #4
100bd8a2c:     	ldr	w8, [x9, x8]
100bd8a30:     	cmp	x8, x10
100bd8a34:     	csel	x10, x8, x10, lo
100bd8a38:     	tbz	w3, #0x1, 0x100bd86e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x114>
100bd8a3c:     	lsr	w8, w3, #2
100bd8a40:     	ldr	x9, [x0, #0x40]
100bd8a44:     	cmp	x9, x8
100bd8a48:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8a4c:     	ldr	x9, [x0, #0x38]
100bd8a50:     	lsl	x8, x8, #4
100bd8a54:     	ldr	w9, [x9, x8]
100bd8a58:     	ldr	x8, [x0, #0xc8]
100bd8a5c:     	cmp	x9, x10
100bd8a60:     	csel	x21, x9, x10, lo
100bd8a64:     	cmp	x21, x8
100bd8a68:     	b.eq	0x100bd86f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x128>
100bd8a6c:     	mov	x8, x1
100bd8a70:     	tbz	w1, #0x1, 0x100bd8aa8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bd8a74:     	lsr	w8, w1, #2
100bd8a78:     	ldr	x9, [x0, #0x40]
100bd8a7c:     	cmp	x9, x8
100bd8a80:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8a84:     	ldr	x9, [x0, #0x38]
100bd8a88:     	add	x9, x9, x8, lsl #4
100bd8a8c:     	ldr	w10, [x9]
100bd8a90:     	mov	x8, x1
100bd8a94:     	cmp	x21, x10
100bd8a98:     	b.ne	0x100bd8aa8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bd8a9c:     	ldr	w8, [x9, #0x4]
100bd8aa0:     	and	w9, w1, #0x1
100bd8aa4:     	eor	w8, w8, w9
100bd8aa8:     	mov	x9, x2
100bd8aac:     	tbz	w2, #0x1, 0x100bd8ae4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bd8ab0:     	lsr	w9, w2, #2
100bd8ab4:     	ldr	x10, [x0, #0x40]
100bd8ab8:     	cmp	x10, x9
100bd8abc:     	b.ls	0x100bd8ce4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x714>
100bd8ac0:     	ldr	x10, [x0, #0x38]
100bd8ac4:     	add	x10, x10, x9, lsl #4
100bd8ac8:     	ldr	w11, [x10]
100bd8acc:     	mov	x9, x2
100bd8ad0:     	cmp	x21, x11
100bd8ad4:     	b.ne	0x100bd8ae4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bd8ad8:     	ldr	w9, [x10, #0x4]
100bd8adc:     	and	w10, w2, #0x1
100bd8ae0:     	eor	w9, w9, w10
100bd8ae4:     	mov	x22, x1
100bd8ae8:     	mov	x10, x3
100bd8aec:     	tbz	w3, #0x1, 0x100bd8b24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bd8af0:     	lsr	w10, w3, #2
100bd8af4:     	ldr	x1, [x0, #0x40]
100bd8af8:     	cmp	x1, x10
100bd8afc:     	b.ls	0x100bd8cf8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x728>
100bd8b00:     	ldr	x11, [x0, #0x38]
100bd8b04:     	add	x11, x11, x10, lsl #4
100bd8b08:     	ldr	w12, [x11]
100bd8b0c:     	mov	x10, x3
100bd8b10:     	cmp	x21, x12
100bd8b14:     	b.ne	0x100bd8b24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bd8b18:     	ldr	w10, [x11, #0x4]
100bd8b1c:     	and	w11, w3, #0x1
100bd8b20:     	eor	w10, w10, w11
100bd8b24:     	mov	x23, x2
100bd8b28:     	mov	x24, x3
100bd8b2c:     	mov	x25, x0
100bd8b30:     	mov	x1, x8
100bd8b34:     	mov	x2, x9
100bd8b38:     	mov	x3, x10
100bd8b3c:     	mov	x19, x4
100bd8b40:     	bl	0x100bd85d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bd8b44:     	and	w8, w0, #0xff
100bd8b48:     	cmp	w8, #0xf
100bd8b4c:     	b.ne	0x100bd8b60 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x590>
100bd8b50:     	mov	w20, #0xf               ; =15
100bd8b54:     	mov	x4, x19
100bd8b58:     	mov	x3, x24
100bd8b5c:     	b	0x100bd8c3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x66c>
100bd8b60:     	mov	x20, x0
100bd8b64:     	mov	x1, x22
100bd8b68:     	mov	x10, x24
100bd8b6c:     	mov	x11, x23
100bd8b70:     	mov	x0, x25
100bd8b74:     	tbz	w22, #0x1, 0x100bd8bb0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bd8b78:     	mov	x9, x22
100bd8b7c:     	lsr	w8, w22, #2
100bd8b80:     	ldr	x1, [x0, #0x40]
100bd8b84:     	cmp	x1, x8
100bd8b88:     	b.ls	0x100bd8d08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x738>
100bd8b8c:     	ldr	x12, [x0, #0x38]
100bd8b90:     	add	x8, x12, x8, lsl #4
100bd8b94:     	ldr	w12, [x8]
100bd8b98:     	mov	x1, x9
100bd8b9c:     	cmp	x21, x12
100bd8ba0:     	b.ne	0x100bd8bb0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bd8ba4:     	ldr	w8, [x8, #0x8]
100bd8ba8:     	and	w9, w9, #0x1
100bd8bac:     	eor	w1, w8, w9
100bd8bb0:     	mov	x2, x11
100bd8bb4:     	tbz	w11, #0x1, 0x100bd8bec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bd8bb8:     	lsr	w8, w11, #2
100bd8bbc:     	ldr	x9, [x0, #0x40]
100bd8bc0:     	cmp	x9, x8
100bd8bc4:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8bc8:     	ldr	x9, [x0, #0x38]
100bd8bcc:     	add	x8, x9, x8, lsl #4
100bd8bd0:     	ldr	w9, [x8]
100bd8bd4:     	mov	x2, x11
100bd8bd8:     	cmp	x21, x9
100bd8bdc:     	b.ne	0x100bd8bec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bd8be0:     	ldr	w8, [x8, #0x8]
100bd8be4:     	and	w9, w11, #0x1
100bd8be8:     	eor	w2, w8, w9
100bd8bec:     	mov	x3, x10
100bd8bf0:     	tbz	w10, #0x1, 0x100bd8c28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bd8bf4:     	lsr	w8, w10, #2
100bd8bf8:     	ldr	x9, [x0, #0x40]
100bd8bfc:     	cmp	x9, x8
100bd8c00:     	b.ls	0x100bd8cbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bd8c04:     	ldr	x9, [x0, #0x38]
100bd8c08:     	add	x8, x9, x8, lsl #4
100bd8c0c:     	ldr	w9, [x8]
100bd8c10:     	mov	x3, x10
100bd8c14:     	cmp	x21, x9
100bd8c18:     	b.ne	0x100bd8c28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bd8c1c:     	ldr	w8, [x8, #0x8]
100bd8c20:     	and	w9, w10, #0x1
100bd8c24:     	eor	w3, w8, w9
100bd8c28:     	mov	x4, x19
100bd8c2c:     	bl	0x100bd85d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bd8c30:     	mov	x3, x24
100bd8c34:     	mov	x4, x19
100bd8c38:     	orr	w20, w0, w20
100bd8c3c:     	mov	x2, x23
100bd8c40:     	mov	x1, x22
100bd8c44:     	stp	w1, w2, [sp, #0x24]
100bd8c48:     	str	w3, [sp, #0x2c]
100bd8c4c:     	add	x1, sp, #0x24
100bd8c50:     	mov	x0, x4
100bd8c54:     	mov	x2, x20
100bd8c58:     	bl	0x100c2b4ec <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bd8c5c:     	mov	x0, x20
100bd8c60:     	ldp	x29, x30, [sp, #0x80]
100bd8c64:     	ldp	x20, x19, [sp, #0x70]
100bd8c68:     	ldp	x22, x21, [sp, #0x60]
100bd8c6c:     	ldp	x24, x23, [sp, #0x50]
100bd8c70:     	ldp	x26, x25, [sp, #0x40]
100bd8c74:     	ldp	x28, x27, [sp, #0x30]
100bd8c78:     	add	sp, sp, #0x90
100bd8c7c:     	ret
100bd8c80:     	ldp	x15, x13, [sp]
100bd8c84:     	bic	x10, x13, x15
100bd8c88:     	tst	x10, x8
100bd8c8c:     	csel	w12, wzr, w12, eq
100bd8c90:     	and	x13, x15, x13
100bd8c94:     	bics	xzr, x13, x8
100bd8c98:     	csel	w11, wzr, w11, eq
100bd8c9c:     	tst	x13, x8
100bd8ca0:     	csel	w13, wzr, w14, eq
100bd8ca4:     	bics	xzr, x10, x8
100bd8ca8:     	cinc	w8, w12, ne
100bd8cac:     	orr	w10, w11, w13
100bd8cb0:     	orr	w8, w8, w10
100bd8cb4:     	orr	w20, w8, w9
100bd8cb8:     	b	0x100bd8c44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bd8cbc:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bd8cc0:     	add	x2, x2, #0x2f8
100bd8cc4:     	mov	x0, x8
100bd8cc8:     	mov	x1, x9
100bd8ccc:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bd8cd0:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bd8cd4:     	add	x2, x2, #0x580
100bd8cd8:     	mov	x0, x9
100bd8cdc:     	mov	x1, x8
100bd8ce0:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bd8ce4:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bd8ce8:     	add	x2, x2, #0x2f8
100bd8cec:     	mov	x0, x9
100bd8cf0:     	mov	x1, x10
100bd8cf4:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bd8cf8:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bd8cfc:     	add	x2, x2, #0x2f8
100bd8d00:     	mov	x0, x10
100bd8d04:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bd8d08:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bd8d0c:     	add	x2, x2, #0x2f8
100bd8d10:     	mov	x0, x8
100bd8d14:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
