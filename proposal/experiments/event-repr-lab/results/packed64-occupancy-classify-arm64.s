
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bb26b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>:
100bb26b8:     	sub	sp, sp, #0x60
100bb26bc:     	stp	x26, x25, [sp, #0x10]
100bb26c0:     	stp	x24, x23, [sp, #0x20]
100bb26c4:     	stp	x22, x21, [sp, #0x30]
100bb26c8:     	stp	x20, x19, [sp, #0x40]
100bb26cc:     	stp	x29, x30, [sp, #0x50]
100bb26d0:     	add	x29, sp, #0x50
100bb26d4:     	cbz	w1, 0x100bb287c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1c4>
100bb26d8:     	ldr	x8, [x4, #0x18]
100bb26dc:     	cbz	x8, 0x100bb27ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bb26e0:     	mov	x8, #0x0                ; =0
100bb26e4:     	mov	w9, w1
100bb26e8:     	mov	x10, #0xa9c5            ; =43461
100bb26ec:     	movk	x10, #0x2e62, lsl #16
100bb26f0:     	movk	x10, #0x7aea, lsl #32
100bb26f4:     	movk	x10, #0xf135, lsl #48
100bb26f8:     	mul	x9, x9, x10
100bb26fc:     	add	x9, x9, w2, uxtw
100bb2700:     	mul	x9, x9, x10
100bb2704:     	add	x9, x9, w3, uxtw
100bb2708:     	mul	x9, x9, x10
100bb270c:     	ror	x11, x9, #0x2c
100bb2710:     	lsr	x12, x11, #57
100bb2714:     	ldp	x10, x9, [x4]
100bb2718:     	dup.8b	v0, w12
100bb271c:     	movi.2d	v1, #0xffffffffffffffff
100bb2720:     	and	x11, x11, x9
100bb2724:     	ldr	d2, [x10, x11]
100bb2728:     	cmeq.8b	v3, v2, v0
100bb272c:     	fmov	x12, d3
100bb2730:     	ands	x12, x12, #0x8080808080808080
100bb2734:     	b.eq	0x100bb277c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bb2738:     	rbit	x13, x12
100bb273c:     	clz	x13, x13
100bb2740:     	add	x13, x11, x13, lsr #3
100bb2744:     	and	x13, x13, x9
100bb2748:     	sub	x13, x10, x13, lsl #4
100bb274c:     	ldur	w14, [x13, #-0x10]
100bb2750:     	cmp	w1, w14
100bb2754:     	b.ne	0x100bb2770 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bb2758:     	ldur	w14, [x13, #-0xc]
100bb275c:     	cmp	w2, w14
100bb2760:     	b.ne	0x100bb2770 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bb2764:     	ldur	w14, [x13, #-0x8]
100bb2768:     	cmp	w3, w14
100bb276c:     	b.eq	0x100bb2884 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1cc>
100bb2770:     	sub	x13, x12, #0x2
100bb2774:     	ands	x12, x13, x12
100bb2778:     	b.ne	0x100bb2738 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bb277c:     	cmeq.8b	v2, v2, v1
100bb2780:     	fmov	x12, d2
100bb2784:     	cbnz	x12, 0x100bb27ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bb2788:     	add	x8, x8, #0x8
100bb278c:     	add	x11, x11, x8
100bb2790:     	and	x11, x11, x9
100bb2794:     	ldr	d2, [x10, x11]
100bb2798:     	cmeq.8b	v3, v2, v0
100bb279c:     	fmov	x12, d3
100bb27a0:     	ands	x12, x12, #0x8080808080808080
100bb27a4:     	b.ne	0x100bb2738 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bb27a8:     	b	0x100bb277c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bb27ac:     	tbnz	w1, #0x1, 0x100bb288c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1d4>
100bb27b0:     	ldr	x10, [x0, #0xc8]
100bb27b4:     	tbnz	w2, #0x1, 0x100bb28ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1f4>
100bb27b8:     	ldr	x8, [x0, #0xc8]
100bb27bc:     	cmp	x8, x10
100bb27c0:     	csel	x10, x8, x10, lo
100bb27c4:     	tbnz	w3, #0x1, 0x100bb28d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x21c>
100bb27c8:     	ldr	x8, [x0, #0xc8]
100bb27cc:     	cmp	x8, x10
100bb27d0:     	csel	x21, x8, x10, lo
100bb27d4:     	cmp	x21, x8
100bb27d8:     	b.ne	0x100bb2904 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x24c>
100bb27dc:     	lsr	w9, w1, #2
100bb27e0:     	ldr	x8, [x0, #0x58]
100bb27e4:     	cmp	x8, x9
100bb27e8:     	b.ls	0x100bb2b28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x470>
100bb27ec:     	ldr	x12, [x0, #0xd8]
100bb27f0:     	tst	w1, #0x1
100bb27f4:     	csel	x13, xzr, x12, eq
100bb27f8:     	lsr	w10, w2, #2
100bb27fc:     	cmp	x8, x10
100bb2800:     	b.ls	0x100bb2b3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x484>
100bb2804:     	lsr	w11, w3, #2
100bb2808:     	cmp	x8, x11
100bb280c:     	b.ls	0x100bb2b50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x498>
100bb2810:     	ldr	x8, [x0, #0x50]
100bb2814:     	ldr	x9, [x8, x9, lsl #3]
100bb2818:     	ldr	x10, [x8, x10, lsl #3]
100bb281c:     	eor	x9, x13, x9
100bb2820:     	tst	w2, #0x1
100bb2824:     	csel	x13, xzr, x12, eq
100bb2828:     	eor	x10, x10, x13
100bb282c:     	ldr	x8, [x8, x11, lsl #3]
100bb2830:     	tst	w3, #0x1
100bb2834:     	csel	x11, xzr, x12, eq
100bb2838:     	eor	x8, x8, x11
100bb283c:     	bic	x11, x9, x10
100bb2840:     	tst	x11, x8
100bb2844:     	mov	w12, #0x2               ; =2
100bb2848:     	csel	w12, wzr, w12, eq
100bb284c:     	and	x9, x10, x9
100bb2850:     	bics	xzr, x9, x8
100bb2854:     	mov	w10, #0x4               ; =4
100bb2858:     	csel	w10, wzr, w10, eq
100bb285c:     	tst	x9, x8
100bb2860:     	mov	w9, #0x8                ; =8
100bb2864:     	csel	w9, wzr, w9, eq
100bb2868:     	bics	xzr, x11, x8
100bb286c:     	cinc	w8, w12, ne
100bb2870:     	orr	w9, w10, w9
100bb2874:     	orr	w20, w8, w9
100bb2878:     	b	0x100bb2adc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x424>
100bb287c:     	mov	w20, #0x0               ; =0
100bb2880:     	b	0x100bb2af4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bb2884:     	ldurb	w20, [x13, #-0x4]
100bb2888:     	b	0x100bb2af4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bb288c:     	lsr	w8, w1, #2
100bb2890:     	ldr	x9, [x0, #0x40]
100bb2894:     	cmp	x9, x8
100bb2898:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb289c:     	ldr	x9, [x0, #0x38]
100bb28a0:     	lsl	x8, x8, #4
100bb28a4:     	ldr	w10, [x9, x8]
100bb28a8:     	tbz	w2, #0x1, 0x100bb27b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x100>
100bb28ac:     	lsr	w8, w2, #2
100bb28b0:     	ldr	x9, [x0, #0x40]
100bb28b4:     	cmp	x9, x8
100bb28b8:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb28bc:     	ldr	x9, [x0, #0x38]
100bb28c0:     	lsl	x8, x8, #4
100bb28c4:     	ldr	w8, [x9, x8]
100bb28c8:     	cmp	x8, x10
100bb28cc:     	csel	x10, x8, x10, lo
100bb28d0:     	tbz	w3, #0x1, 0x100bb27c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x110>
100bb28d4:     	lsr	w8, w3, #2
100bb28d8:     	ldr	x9, [x0, #0x40]
100bb28dc:     	cmp	x9, x8
100bb28e0:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb28e4:     	ldr	x9, [x0, #0x38]
100bb28e8:     	lsl	x8, x8, #4
100bb28ec:     	ldr	w9, [x9, x8]
100bb28f0:     	ldr	x8, [x0, #0xc8]
100bb28f4:     	cmp	x9, x10
100bb28f8:     	csel	x21, x9, x10, lo
100bb28fc:     	cmp	x21, x8
100bb2900:     	b.eq	0x100bb27dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x124>
100bb2904:     	mov	x8, x1
100bb2908:     	tbz	w1, #0x1, 0x100bb2940 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bb290c:     	lsr	w8, w1, #2
100bb2910:     	ldr	x9, [x0, #0x40]
100bb2914:     	cmp	x9, x8
100bb2918:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb291c:     	ldr	x9, [x0, #0x38]
100bb2920:     	add	x9, x9, x8, lsl #4
100bb2924:     	ldr	w10, [x9]
100bb2928:     	mov	x8, x1
100bb292c:     	cmp	x21, x10
100bb2930:     	b.ne	0x100bb2940 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bb2934:     	ldr	w8, [x9, #0x4]
100bb2938:     	and	w9, w1, #0x1
100bb293c:     	eor	w8, w8, w9
100bb2940:     	mov	x9, x2
100bb2944:     	tbz	w2, #0x1, 0x100bb297c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bb2948:     	lsr	w9, w2, #2
100bb294c:     	ldr	x10, [x0, #0x40]
100bb2950:     	cmp	x10, x9
100bb2954:     	b.ls	0x100bb2b64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4ac>
100bb2958:     	ldr	x10, [x0, #0x38]
100bb295c:     	add	x10, x10, x9, lsl #4
100bb2960:     	ldr	w11, [x10]
100bb2964:     	mov	x9, x2
100bb2968:     	cmp	x21, x11
100bb296c:     	b.ne	0x100bb297c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bb2970:     	ldr	w9, [x10, #0x4]
100bb2974:     	and	w10, w2, #0x1
100bb2978:     	eor	w9, w9, w10
100bb297c:     	mov	x22, x1
100bb2980:     	mov	x10, x3
100bb2984:     	tbz	w3, #0x1, 0x100bb29bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bb2988:     	lsr	w10, w3, #2
100bb298c:     	ldr	x1, [x0, #0x40]
100bb2990:     	cmp	x1, x10
100bb2994:     	b.ls	0x100bb2b78 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4c0>
100bb2998:     	ldr	x11, [x0, #0x38]
100bb299c:     	add	x11, x11, x10, lsl #4
100bb29a0:     	ldr	w12, [x11]
100bb29a4:     	mov	x10, x3
100bb29a8:     	cmp	x21, x12
100bb29ac:     	b.ne	0x100bb29bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bb29b0:     	ldr	w10, [x11, #0x4]
100bb29b4:     	and	w11, w3, #0x1
100bb29b8:     	eor	w10, w10, w11
100bb29bc:     	mov	x23, x2
100bb29c0:     	mov	x24, x3
100bb29c4:     	mov	x25, x0
100bb29c8:     	mov	x1, x8
100bb29cc:     	mov	x2, x9
100bb29d0:     	mov	x3, x10
100bb29d4:     	mov	x19, x4
100bb29d8:     	bl	0x100bb26b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bb29dc:     	and	w8, w0, #0xff
100bb29e0:     	cmp	w8, #0xf
100bb29e4:     	b.ne	0x100bb29f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x340>
100bb29e8:     	mov	w20, #0xf               ; =15
100bb29ec:     	mov	x4, x19
100bb29f0:     	mov	x3, x24
100bb29f4:     	b	0x100bb2ad4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x41c>
100bb29f8:     	mov	x20, x0
100bb29fc:     	mov	x1, x22
100bb2a00:     	mov	x10, x24
100bb2a04:     	mov	x11, x23
100bb2a08:     	mov	x0, x25
100bb2a0c:     	tbz	w22, #0x1, 0x100bb2a48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bb2a10:     	mov	x9, x22
100bb2a14:     	lsr	w8, w22, #2
100bb2a18:     	ldr	x1, [x0, #0x40]
100bb2a1c:     	cmp	x1, x8
100bb2a20:     	b.ls	0x100bb2b88 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4d0>
100bb2a24:     	ldr	x12, [x0, #0x38]
100bb2a28:     	add	x8, x12, x8, lsl #4
100bb2a2c:     	ldr	w12, [x8]
100bb2a30:     	mov	x1, x9
100bb2a34:     	cmp	x21, x12
100bb2a38:     	b.ne	0x100bb2a48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bb2a3c:     	ldr	w8, [x8, #0x8]
100bb2a40:     	and	w9, w9, #0x1
100bb2a44:     	eor	w1, w8, w9
100bb2a48:     	mov	x2, x11
100bb2a4c:     	tbz	w11, #0x1, 0x100bb2a84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bb2a50:     	lsr	w8, w11, #2
100bb2a54:     	ldr	x9, [x0, #0x40]
100bb2a58:     	cmp	x9, x8
100bb2a5c:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb2a60:     	ldr	x9, [x0, #0x38]
100bb2a64:     	add	x8, x9, x8, lsl #4
100bb2a68:     	ldr	w9, [x8]
100bb2a6c:     	mov	x2, x11
100bb2a70:     	cmp	x21, x9
100bb2a74:     	b.ne	0x100bb2a84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bb2a78:     	ldr	w8, [x8, #0x8]
100bb2a7c:     	and	w9, w11, #0x1
100bb2a80:     	eor	w2, w8, w9
100bb2a84:     	mov	x3, x10
100bb2a88:     	tbz	w10, #0x1, 0x100bb2ac0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bb2a8c:     	lsr	w8, w10, #2
100bb2a90:     	ldr	x9, [x0, #0x40]
100bb2a94:     	cmp	x9, x8
100bb2a98:     	b.ls	0x100bb2b14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bb2a9c:     	ldr	x9, [x0, #0x38]
100bb2aa0:     	add	x8, x9, x8, lsl #4
100bb2aa4:     	ldr	w9, [x8]
100bb2aa8:     	mov	x3, x10
100bb2aac:     	cmp	x21, x9
100bb2ab0:     	b.ne	0x100bb2ac0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bb2ab4:     	ldr	w8, [x8, #0x8]
100bb2ab8:     	and	w9, w10, #0x1
100bb2abc:     	eor	w3, w8, w9
100bb2ac0:     	mov	x4, x19
100bb2ac4:     	bl	0x100bb26b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bb2ac8:     	mov	x3, x24
100bb2acc:     	mov	x4, x19
100bb2ad0:     	orr	w20, w0, w20
100bb2ad4:     	mov	x2, x23
100bb2ad8:     	mov	x1, x22
100bb2adc:     	stp	w1, w2, [sp, #0x4]
100bb2ae0:     	str	w3, [sp, #0xc]
100bb2ae4:     	add	x1, sp, #0x4
100bb2ae8:     	mov	x0, x4
100bb2aec:     	mov	x2, x20
100bb2af0:     	bl	0x100c18e30 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bb2af4:     	mov	x0, x20
100bb2af8:     	ldp	x29, x30, [sp, #0x50]
100bb2afc:     	ldp	x20, x19, [sp, #0x40]
100bb2b00:     	ldp	x22, x21, [sp, #0x30]
100bb2b04:     	ldp	x24, x23, [sp, #0x20]
100bb2b08:     	ldp	x26, x25, [sp, #0x10]
100bb2b0c:     	add	sp, sp, #0x60
100bb2b10:     	ret
100bb2b14:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bb2b18:     	add	x2, x2, #0xe88
100bb2b1c:     	mov	x0, x8
100bb2b20:     	mov	x1, x9
100bb2b24:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b28:     	adrp	x2, 0x1014e1000 <dyld_stub_binder+0x1014e1000>
100bb2b2c:     	add	x2, x2, #0x110
100bb2b30:     	mov	x0, x9
100bb2b34:     	mov	x1, x8
100bb2b38:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b3c:     	adrp	x2, 0x1014e1000 <dyld_stub_binder+0x1014e1000>
100bb2b40:     	add	x2, x2, #0x110
100bb2b44:     	mov	x0, x10
100bb2b48:     	mov	x1, x8
100bb2b4c:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b50:     	adrp	x2, 0x1014e1000 <dyld_stub_binder+0x1014e1000>
100bb2b54:     	add	x2, x2, #0x110
100bb2b58:     	mov	x0, x11
100bb2b5c:     	mov	x1, x8
100bb2b60:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b64:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bb2b68:     	add	x2, x2, #0xe88
100bb2b6c:     	mov	x0, x9
100bb2b70:     	mov	x1, x10
100bb2b74:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b78:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bb2b7c:     	add	x2, x2, #0xe88
100bb2b80:     	mov	x0, x10
100bb2b84:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bb2b88:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bb2b8c:     	add	x2, x2, #0xe88
100bb2b90:     	mov	x0, x8
100bb2b94:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
