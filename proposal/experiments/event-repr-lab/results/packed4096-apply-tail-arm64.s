
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>:
1006d3600:     	stp	d15, d14, [sp, #-0x90]!
1006d3604:     	stp	d13, d12, [sp, #0x10]
1006d3608:     	stp	d11, d10, [sp, #0x20]
1006d360c:     	stp	d9, d8, [sp, #0x30]
1006d3610:     	stp	x26, x25, [sp, #0x40]
1006d3614:     	stp	x24, x23, [sp, #0x50]
1006d3618:     	stp	x22, x21, [sp, #0x60]
1006d361c:     	stp	x20, x19, [sp, #0x70]
1006d3620:     	stp	x29, x30, [sp, #0x80]
1006d3624:     	add	x29, sp, #0x80
1006d3628:     	sub	sp, sp, #0x610
1006d362c:     	ldr	xzr, [sp]
1006d3630:     	mov	x20, x3
1006d3634:     	mov	x23, x2
1006d3638:     	mov	x22, x1
1006d363c:     	mov	x19, x0
1006d3640:     	mov	w1, w2
1006d3644:     	mov	w2, w3
1006d3648:     	mov	x0, x22
1006d364c:     	bl	0x1007989d8 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
1006d3650:     	cmp	x0, #0x1
1006d3654:     	b.ne	0x1006d3660 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x60>
1006d3658:     	mov	x21, x1
1006d365c:     	b	0x1006d3d00 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x700>
1006d3660:     	add	x25, x19, #0x108
1006d3664:     	mov	w8, #0x9                ; =9
1006d3668:     	and	w8, w22, w8
1006d366c:     	lsr	w9, w22, #1
1006d3670:     	bfi	w8, w9, #2, #1
1006d3674:     	and	w9, w9, #0x2
1006d3678:     	orr	w8, w8, w9
1006d367c:     	nop
1006d3680:     	cmp	w23, w20
1006d3684:     	csel	w24, w23, w20, hi
1006d3688:     	csel	w23, w20, w23, hi
1006d368c:     	csel	w20, w8, w22, hi
1006d3690:     	ldr	x8, [x19, #0xb8]
1006d3694:     	cbz	x8, 0x1006d3764 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x164>
1006d3698:     	mov	x8, #0x0                ; =0
1006d369c:     	and	x9, x20, #0xff
1006d36a0:     	mov	x10, #0xa9c5            ; =43461
1006d36a4:     	movk	x10, #0x2e62, lsl #16
1006d36a8:     	movk	x10, #0x7aea, lsl #32
1006d36ac:     	movk	x10, #0xf135, lsl #48
1006d36b0:     	mul	x9, x9, x10
1006d36b4:     	add	x9, x9, w23, uxtw
1006d36b8:     	mul	x9, x9, x10
1006d36bc:     	add	x9, x9, w24, uxtw
1006d36c0:     	mul	x9, x9, x10
1006d36c4:     	ror	x11, x9, #0x2c
1006d36c8:     	lsr	x12, x11, #57
1006d36cc:     	ldp	x10, x9, [x19, #0xa0]
1006d36d0:     	dup.8b	v0, w12
1006d36d4:     	movi.2d	v1, #0xffffffffffffffff
1006d36d8:     	and	x11, x11, x9
1006d36dc:     	ldr	d2, [x10, x11]
1006d36e0:     	cmeq.8b	v3, v2, v0
1006d36e4:     	fmov	x12, d3
1006d36e8:     	ands	x12, x12, #0x8080808080808080
1006d36ec:     	b.eq	0x1006d3734 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x134>
1006d36f0:     	rbit	x13, x12
1006d36f4:     	clz	x13, x13
1006d36f8:     	add	x13, x11, x13, lsr #3
1006d36fc:     	and	x13, x13, x9
1006d3700:     	sub	x13, x10, x13, lsl #4
1006d3704:     	ldurb	w14, [x13, #-0xc]
1006d3708:     	cmp	w14, w20, uxtb
1006d370c:     	b.ne	0x1006d3728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x128>
1006d3710:     	ldur	w14, [x13, #-0x10]
1006d3714:     	cmp	w23, w14
1006d3718:     	b.ne	0x1006d3728 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x128>
1006d371c:     	ldur	w14, [x13, #-0x8]
1006d3720:     	cmp	w24, w14
1006d3724:     	b.eq	0x1006d3b18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x518>
1006d3728:     	sub	x13, x12, #0x2
1006d372c:     	ands	x12, x13, x12
1006d3730:     	b.ne	0x1006d36f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0xf0>
1006d3734:     	cmeq.8b	v2, v2, v1
1006d3738:     	fmov	x12, d2
1006d373c:     	cbnz	x12, 0x1006d3764 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x164>
1006d3740:     	add	x8, x8, #0x8
1006d3744:     	add	x11, x11, x8
1006d3748:     	and	x11, x11, x9
1006d374c:     	ldr	d2, [x10, x11]
1006d3750:     	cmeq.8b	v3, v2, v0
1006d3754:     	fmov	x12, d3
1006d3758:     	ands	x12, x12, #0x8080808080808080
1006d375c:     	b.ne	0x1006d36f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0xf0>
1006d3760:     	b	0x1006d3734 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x134>
1006d3764:     	tbnz	w23, #0x1, 0x1006d3b20 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x520>
1006d3768:     	ldr	x8, [x19, #0xc8]
1006d376c:     	tbnz	w24, #0x1, 0x1006d3b40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x540>
1006d3770:     	ldr	x9, [x19, #0xc8]
1006d3774:     	cmp	x9, x8
1006d3778:     	csel	x21, x9, x8, lo
1006d377c:     	cmp	x21, x9
1006d3780:     	b.ne	0x1006d3b70 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x570>
1006d3784:     	lsr	w0, w23, #2
1006d3788:     	ldr	x21, [x19, #0x58]
1006d378c:     	cmp	x21, x0
1006d3790:     	b.ls	0x1006d3d3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x73c>
1006d3794:     	ldr	x22, [x19, #0x50]
1006d3798:     	add	x1, x22, x0, lsl #9
1006d379c:     	tbz	w23, #0x0, 0x1006d3934 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x334>
1006d37a0:     	ldp	q1, q0, [x1, #0x1e0]
1006d37a4:     	str	q0, [sp]
1006d37a8:     	ldp	q3, q2, [x1, #0x1c0]
1006d37ac:     	ldp	q5, q4, [x1, #0x1a0]
1006d37b0:     	ldp	q7, q6, [x1, #0x180]
1006d37b4:     	ldp	q17, q16, [x1, #0x160]
1006d37b8:     	ldp	q19, q18, [x1, #0x140]
1006d37bc:     	ldp	q21, q20, [x1, #0x120]
1006d37c0:     	ldp	q23, q22, [x1, #0x100]
1006d37c4:     	ldp	q25, q24, [x1, #0xe0]
1006d37c8:     	ldp	q27, q26, [x1, #0xc0]
1006d37cc:     	ldp	q29, q28, [x1, #0xa0]
1006d37d0:     	ldp	q31, q30, [x1, #0x80]
1006d37d4:     	ldp	q9, q8, [x1, #0x60]
1006d37d8:     	ldp	q11, q10, [x1, #0x20]
1006d37dc:     	ldp	q13, q12, [x1]
1006d37e0:     	ldur	q14, [x19, #0xd8]
1006d37e4:     	ldur	q15, [x19, #0xe8]
1006d37e8:     	eor.16b	v13, v14, v13
1006d37ec:     	ldur	q14, [x19, #0xf8]
1006d37f0:     	eor.16b	v12, v15, v12
1006d37f4:     	eor.16b	v11, v14, v11
1006d37f8:     	ldp	q14, q15, [x25]
1006d37fc:     	eor.16b	v10, v14, v10
1006d3800:     	ldp	q0, q14, [x1, #0x40]
1006d3804:     	str	q13, [sp, #0x410]
1006d3808:     	str	q12, [sp, #0x420]
1006d380c:     	str	q11, [sp, #0x430]
1006d3810:     	str	q10, [sp, #0x440]
1006d3814:     	eor.16b	v0, v15, v0
1006d3818:     	ldp	q10, q11, [x25, #0x20]
1006d381c:     	eor.16b	v10, v10, v14
1006d3820:     	eor.16b	v9, v11, v9
1006d3824:     	ldp	q11, q12, [x25, #0x40]
1006d3828:     	eor.16b	v8, v11, v8
1006d382c:     	str	q0, [sp, #0x450]
1006d3830:     	str	q10, [sp, #0x460]
1006d3834:     	str	q9, [sp, #0x470]
1006d3838:     	str	q8, [sp, #0x480]
1006d383c:     	eor.16b	v0, v12, v31
1006d3840:     	ldp	q31, q8, [x25, #0x60]
1006d3844:     	eor.16b	v30, v31, v30
1006d3848:     	eor.16b	v29, v8, v29
1006d384c:     	ldp	q31, q8, [x25, #0x80]
1006d3850:     	eor.16b	v28, v31, v28
1006d3854:     	str	q0, [sp, #0x490]
1006d3858:     	str	q30, [sp, #0x4a0]
1006d385c:     	str	q29, [sp, #0x4b0]
1006d3860:     	str	q28, [sp, #0x4c0]
1006d3864:     	eor.16b	v0, v8, v27
1006d3868:     	ldp	q27, q28, [x25, #0xa0]
1006d386c:     	eor.16b	v26, v27, v26
1006d3870:     	eor.16b	v25, v28, v25
1006d3874:     	ldp	q27, q28, [x25, #0xc0]
1006d3878:     	eor.16b	v24, v27, v24
1006d387c:     	str	q0, [sp, #0x4d0]
1006d3880:     	str	q26, [sp, #0x4e0]
1006d3884:     	str	q25, [sp, #0x4f0]
1006d3888:     	str	q24, [sp, #0x500]
1006d388c:     	eor.16b	v0, v28, v23
1006d3890:     	ldp	q23, q24, [x25, #0xe0]
1006d3894:     	eor.16b	v22, v23, v22
1006d3898:     	eor.16b	v21, v24, v21
1006d389c:     	ldp	q23, q24, [x25, #0x100]
1006d38a0:     	eor.16b	v20, v23, v20
1006d38a4:     	str	q0, [sp, #0x510]
1006d38a8:     	str	q22, [sp, #0x520]
1006d38ac:     	str	q21, [sp, #0x530]
1006d38b0:     	str	q20, [sp, #0x540]
1006d38b4:     	eor.16b	v0, v24, v19
1006d38b8:     	ldp	q19, q20, [x25, #0x120]
1006d38bc:     	eor.16b	v18, v19, v18
1006d38c0:     	eor.16b	v17, v20, v17
1006d38c4:     	ldp	q19, q20, [x25, #0x140]
1006d38c8:     	eor.16b	v16, v19, v16
1006d38cc:     	str	q0, [sp, #0x550]
1006d38d0:     	str	q18, [sp, #0x560]
1006d38d4:     	str	q17, [sp, #0x570]
1006d38d8:     	str	q16, [sp, #0x580]
1006d38dc:     	eor.16b	v0, v20, v7
1006d38e0:     	ldp	q7, q16, [x25, #0x160]
1006d38e4:     	eor.16b	v6, v7, v6
1006d38e8:     	eor.16b	v5, v16, v5
1006d38ec:     	ldp	q7, q16, [x25, #0x180]
1006d38f0:     	eor.16b	v4, v7, v4
1006d38f4:     	str	q0, [sp, #0x590]
1006d38f8:     	str	q6, [sp, #0x5a0]
1006d38fc:     	str	q5, [sp, #0x5b0]
1006d3900:     	str	q4, [sp, #0x5c0]
1006d3904:     	eor.16b	v0, v16, v3
1006d3908:     	ldp	q3, q4, [x25, #0x1a0]
1006d390c:     	eor.16b	v2, v3, v2
1006d3910:     	eor.16b	v1, v4, v1
1006d3914:     	ldr	q3, [x25, #0x1c0]
1006d3918:     	ldr	q4, [sp]
1006d391c:     	eor.16b	v3, v3, v4
1006d3920:     	str	q0, [sp, #0x5d0]
1006d3924:     	str	q2, [sp, #0x5e0]
1006d3928:     	str	q1, [sp, #0x5f0]
1006d392c:     	str	q3, [sp, #0x600]
1006d3930:     	add	x1, sp, #0x410
1006d3934:     	add	x0, sp, #0x10
1006d3938:     	mov	w2, #0x200              ; =512
1006d393c:     	bl	0x100ca35d8 <dyld_stub_binder+0x100ca35d8>
1006d3940:     	lsr	w0, w24, #2
1006d3944:     	cmp	x21, x0
1006d3948:     	b.ls	0x1006d3d3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x73c>
1006d394c:     	add	x1, x22, x0, lsl #9
1006d3950:     	tbz	w24, #0x0, 0x1006d3ae8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x4e8>
1006d3954:     	ldp	q1, q0, [x1, #0x1e0]
1006d3958:     	str	q0, [sp]
1006d395c:     	ldp	q3, q2, [x1, #0x1c0]
1006d3960:     	ldp	q5, q4, [x1, #0x1a0]
1006d3964:     	ldp	q7, q6, [x1, #0x180]
1006d3968:     	ldp	q17, q16, [x1, #0x160]
1006d396c:     	ldp	q19, q18, [x1, #0x140]
1006d3970:     	ldp	q21, q20, [x1, #0x120]
1006d3974:     	ldp	q23, q22, [x1, #0x100]
1006d3978:     	ldp	q25, q24, [x1, #0xe0]
1006d397c:     	ldp	q27, q26, [x1, #0xc0]
1006d3980:     	ldp	q29, q28, [x1, #0xa0]
1006d3984:     	ldp	q31, q30, [x1, #0x80]
1006d3988:     	ldp	q9, q8, [x1, #0x60]
1006d398c:     	ldp	q11, q10, [x1, #0x20]
1006d3990:     	ldp	q13, q12, [x1]
1006d3994:     	ldur	q14, [x19, #0xd8]
1006d3998:     	ldur	q15, [x19, #0xe8]
1006d399c:     	eor.16b	v13, v14, v13
1006d39a0:     	ldur	q14, [x19, #0xf8]
1006d39a4:     	eor.16b	v12, v15, v12
1006d39a8:     	eor.16b	v11, v14, v11
1006d39ac:     	ldp	q14, q15, [x25]
1006d39b0:     	eor.16b	v10, v14, v10
1006d39b4:     	ldp	q0, q14, [x1, #0x40]
1006d39b8:     	str	q13, [sp, #0x410]
1006d39bc:     	str	q12, [sp, #0x420]
1006d39c0:     	str	q11, [sp, #0x430]
1006d39c4:     	str	q10, [sp, #0x440]
1006d39c8:     	eor.16b	v0, v15, v0
1006d39cc:     	ldp	q10, q11, [x25, #0x20]
1006d39d0:     	eor.16b	v10, v10, v14
1006d39d4:     	eor.16b	v9, v11, v9
1006d39d8:     	ldp	q11, q12, [x25, #0x40]
1006d39dc:     	eor.16b	v8, v11, v8
1006d39e0:     	str	q0, [sp, #0x450]
1006d39e4:     	str	q10, [sp, #0x460]
1006d39e8:     	str	q9, [sp, #0x470]
1006d39ec:     	str	q8, [sp, #0x480]
1006d39f0:     	eor.16b	v0, v12, v31
1006d39f4:     	ldp	q31, q8, [x25, #0x60]
1006d39f8:     	eor.16b	v30, v31, v30
1006d39fc:     	eor.16b	v29, v8, v29
1006d3a00:     	ldp	q31, q8, [x25, #0x80]
1006d3a04:     	eor.16b	v28, v31, v28
1006d3a08:     	str	q0, [sp, #0x490]
1006d3a0c:     	str	q30, [sp, #0x4a0]
1006d3a10:     	str	q29, [sp, #0x4b0]
1006d3a14:     	str	q28, [sp, #0x4c0]
1006d3a18:     	eor.16b	v0, v8, v27
1006d3a1c:     	ldp	q27, q28, [x25, #0xa0]
1006d3a20:     	eor.16b	v26, v27, v26
1006d3a24:     	eor.16b	v25, v28, v25
1006d3a28:     	ldp	q27, q28, [x25, #0xc0]
1006d3a2c:     	eor.16b	v24, v27, v24
1006d3a30:     	str	q0, [sp, #0x4d0]
1006d3a34:     	str	q26, [sp, #0x4e0]
1006d3a38:     	str	q25, [sp, #0x4f0]
1006d3a3c:     	str	q24, [sp, #0x500]
1006d3a40:     	eor.16b	v0, v28, v23
1006d3a44:     	ldp	q23, q24, [x25, #0xe0]
1006d3a48:     	eor.16b	v22, v23, v22
1006d3a4c:     	eor.16b	v21, v24, v21
1006d3a50:     	ldp	q23, q24, [x25, #0x100]
1006d3a54:     	eor.16b	v20, v23, v20
1006d3a58:     	str	q0, [sp, #0x510]
1006d3a5c:     	str	q22, [sp, #0x520]
1006d3a60:     	str	q21, [sp, #0x530]
1006d3a64:     	str	q20, [sp, #0x540]
1006d3a68:     	eor.16b	v0, v24, v19
1006d3a6c:     	ldp	q19, q20, [x25, #0x120]
1006d3a70:     	eor.16b	v18, v19, v18
1006d3a74:     	eor.16b	v17, v20, v17
1006d3a78:     	ldp	q19, q20, [x25, #0x140]
1006d3a7c:     	eor.16b	v16, v19, v16
1006d3a80:     	str	q0, [sp, #0x550]
1006d3a84:     	str	q18, [sp, #0x560]
1006d3a88:     	str	q17, [sp, #0x570]
1006d3a8c:     	str	q16, [sp, #0x580]
1006d3a90:     	eor.16b	v0, v20, v7
1006d3a94:     	ldp	q7, q16, [x25, #0x160]
1006d3a98:     	eor.16b	v6, v7, v6
1006d3a9c:     	eor.16b	v5, v16, v5
1006d3aa0:     	ldp	q7, q16, [x25, #0x180]
1006d3aa4:     	eor.16b	v4, v7, v4
1006d3aa8:     	str	q0, [sp, #0x590]
1006d3aac:     	str	q6, [sp, #0x5a0]
1006d3ab0:     	str	q5, [sp, #0x5b0]
1006d3ab4:     	str	q4, [sp, #0x5c0]
1006d3ab8:     	eor.16b	v0, v16, v3
1006d3abc:     	ldp	q3, q4, [x25, #0x1a0]
1006d3ac0:     	eor.16b	v2, v3, v2
1006d3ac4:     	eor.16b	v1, v4, v1
1006d3ac8:     	ldr	q3, [x25, #0x1c0]
1006d3acc:     	ldr	q4, [sp]
1006d3ad0:     	eor.16b	v3, v3, v4
1006d3ad4:     	str	q0, [sp, #0x5d0]
1006d3ad8:     	str	q2, [sp, #0x5e0]
1006d3adc:     	str	q1, [sp, #0x5f0]
1006d3ae0:     	str	q3, [sp, #0x600]
1006d3ae4:     	add	x1, sp, #0x410
1006d3ae8:     	add	x0, sp, #0x210
1006d3aec:     	mov	w2, #0x200              ; =512
1006d3af0:     	bl	0x100ca35d8 <dyld_stub_binder+0x100ca35d8>
1006d3af4:     	add	x0, sp, #0x410
1006d3af8:     	add	x2, sp, #0x10
1006d3afc:     	add	x3, sp, #0x210
1006d3b00:     	mov	x1, x20
1006d3b04:     	bl	0x1006d3d58 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E7combineB6_>
1006d3b08:     	add	x1, sp, #0x410
1006d3b0c:     	mov	x0, x19
1006d3b10:     	bl	0x1006d2b64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E4leafB6_>
1006d3b14:     	b	0x1006d3ce0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x6e0>
1006d3b18:     	ldur	w21, [x13, #-0x4]
1006d3b1c:     	b	0x1006d3d00 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x700>
1006d3b20:     	lsr	w0, w23, #2
1006d3b24:     	ldr	x1, [x19, #0x40]
1006d3b28:     	cmp	x1, x0
1006d3b2c:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3b30:     	ldr	x8, [x19, #0x38]
1006d3b34:     	lsl	x9, x0, #4
1006d3b38:     	ldr	w8, [x8, x9]
1006d3b3c:     	tbz	w24, #0x1, 0x1006d3770 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x170>
1006d3b40:     	lsr	w0, w24, #2
1006d3b44:     	ldr	x1, [x19, #0x40]
1006d3b48:     	cmp	x1, x0
1006d3b4c:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3b50:     	ldr	x9, [x19, #0x38]
1006d3b54:     	lsl	x10, x0, #4
1006d3b58:     	ldr	w10, [x9, x10]
1006d3b5c:     	ldr	x9, [x19, #0xc8]
1006d3b60:     	cmp	x10, x8
1006d3b64:     	csel	x21, x10, x8, lo
1006d3b68:     	cmp	x21, x9
1006d3b6c:     	b.eq	0x1006d3784 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x184>
1006d3b70:     	mov	x2, x23
1006d3b74:     	tbz	w23, #0x1, 0x1006d3bac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x5ac>
1006d3b78:     	lsr	w0, w23, #2
1006d3b7c:     	ldr	x1, [x19, #0x40]
1006d3b80:     	cmp	x1, x0
1006d3b84:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3b88:     	ldr	x8, [x19, #0x38]
1006d3b8c:     	add	x8, x8, x0, lsl #4
1006d3b90:     	ldr	w9, [x8]
1006d3b94:     	mov	x2, x23
1006d3b98:     	cmp	x21, x9
1006d3b9c:     	b.ne	0x1006d3bac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x5ac>
1006d3ba0:     	ldr	w8, [x8, #0x4]
1006d3ba4:     	and	w9, w23, #0x1
1006d3ba8:     	eor	w2, w8, w9
1006d3bac:     	mov	x3, x24
1006d3bb0:     	tbz	w24, #0x1, 0x1006d3be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x5e8>
1006d3bb4:     	lsr	w0, w24, #2
1006d3bb8:     	ldr	x1, [x19, #0x40]
1006d3bbc:     	cmp	x1, x0
1006d3bc0:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3bc4:     	ldr	x8, [x19, #0x38]
1006d3bc8:     	add	x8, x8, x0, lsl #4
1006d3bcc:     	ldr	w9, [x8]
1006d3bd0:     	mov	x3, x24
1006d3bd4:     	cmp	x21, x9
1006d3bd8:     	b.ne	0x1006d3be8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x5e8>
1006d3bdc:     	ldr	w8, [x8, #0x4]
1006d3be0:     	and	w9, w24, #0x1
1006d3be4:     	eor	w3, w8, w9
1006d3be8:     	mov	x0, x19
1006d3bec:     	mov	x1, x20
1006d3bf0:     	bl	0x1006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>
1006d3bf4:     	mov	x22, x0
1006d3bf8:     	tbnz	w23, #0x1, 0x1006d3c24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x624>
1006d3bfc:     	ldr	x8, [x19, #0xc8]
1006d3c00:     	mov	x2, x23
1006d3c04:     	cmp	x8, x21
1006d3c08:     	b.eq	0x1006d3c4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x64c>
1006d3c0c:     	tbnz	w24, #0x1, 0x1006d3c74 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x674>
1006d3c10:     	ldr	x8, [x19, #0xc8]
1006d3c14:     	mov	x3, x24
1006d3c18:     	cmp	x8, x21
1006d3c1c:     	b.eq	0x1006d3c9c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x69c>
1006d3c20:     	b	0x1006d3cc0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x6c0>
1006d3c24:     	lsr	w0, w23, #2
1006d3c28:     	ldr	x1, [x19, #0x40]
1006d3c2c:     	cmp	x1, x0
1006d3c30:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3c34:     	ldr	x8, [x19, #0x38]
1006d3c38:     	lsl	x9, x0, #4
1006d3c3c:     	ldr	w8, [x8, x9]
1006d3c40:     	mov	x2, x23
1006d3c44:     	cmp	x8, x21
1006d3c48:     	b.ne	0x1006d3c0c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x60c>
1006d3c4c:     	lsr	w0, w23, #2
1006d3c50:     	ldr	x1, [x19, #0x40]
1006d3c54:     	cmp	x1, x0
1006d3c58:     	b.ls	0x1006d3d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x74c>
1006d3c5c:     	ldr	x8, [x19, #0x38]
1006d3c60:     	add	x8, x8, x0, lsl #4
1006d3c64:     	ldr	w8, [x8, #0x8]
1006d3c68:     	and	w9, w23, #0x1
1006d3c6c:     	eor	w2, w8, w9
1006d3c70:     	tbz	w24, #0x1, 0x1006d3c10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x610>
1006d3c74:     	lsr	w0, w24, #2
1006d3c78:     	ldr	x1, [x19, #0x40]
1006d3c7c:     	cmp	x1, x0
1006d3c80:     	b.ls	0x1006d3d30 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x730>
1006d3c84:     	ldr	x8, [x19, #0x38]
1006d3c88:     	lsl	x9, x0, #4
1006d3c8c:     	ldr	w8, [x8, x9]
1006d3c90:     	mov	x3, x24
1006d3c94:     	cmp	x8, x21
1006d3c98:     	b.ne	0x1006d3cc0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x6c0>
1006d3c9c:     	lsr	w0, w24, #2
1006d3ca0:     	ldr	x1, [x19, #0x40]
1006d3ca4:     	cmp	x1, x0
1006d3ca8:     	b.ls	0x1006d3d4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_+0x74c>
1006d3cac:     	ldr	x8, [x19, #0x38]
1006d3cb0:     	add	x8, x8, x0, lsl #4
1006d3cb4:     	ldr	w8, [x8, #0x8]
1006d3cb8:     	and	w9, w24, #0x1
1006d3cbc:     	eor	w3, w8, w9
1006d3cc0:     	mov	x0, x19
1006d3cc4:     	mov	x1, x20
1006d3cc8:     	bl	0x1006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>
1006d3ccc:     	mov	x3, x0
1006d3cd0:     	mov	x0, x19
1006d3cd4:     	mov	x1, x21
1006d3cd8:     	mov	x2, x22
1006d3cdc:     	bl	0x1006d25f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E2mkB6_>
1006d3ce0:     	mov	x21, x0
1006d3ce4:     	strb	w20, [sp, #0x414]
1006d3ce8:     	str	w23, [sp, #0x410]
1006d3cec:     	str	w24, [sp, #0x418]
1006d3cf0:     	add	x0, x19, #0xa0
1006d3cf4:     	add	x1, sp, #0x410
1006d3cf8:     	mov	x2, x21
1006d3cfc:     	bl	0x1007284d8 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006d3d00:     	mov	x0, x21
1006d3d04:     	add	sp, sp, #0x610
1006d3d08:     	ldp	x29, x30, [sp, #0x80]
1006d3d0c:     	ldp	x20, x19, [sp, #0x70]
1006d3d10:     	ldp	x22, x21, [sp, #0x60]
1006d3d14:     	ldp	x24, x23, [sp, #0x50]
1006d3d18:     	ldp	x26, x25, [sp, #0x40]
1006d3d1c:     	ldp	d9, d8, [sp, #0x30]
1006d3d20:     	ldp	d11, d10, [sp, #0x20]
1006d3d24:     	ldp	d13, d12, [sp, #0x10]
1006d3d28:     	ldp	d15, d14, [sp], #0x90
1006d3d2c:     	ret
1006d3d30:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d3d34:     	add	x2, x2, #0x760
1006d3d38:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d3d3c:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d3d40:     	add	x2, x2, #0x928
1006d3d44:     	mov	x1, x21
1006d3d48:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d3d4c:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d3d50:     	add	x2, x2, #0x910
1006d3d54:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
