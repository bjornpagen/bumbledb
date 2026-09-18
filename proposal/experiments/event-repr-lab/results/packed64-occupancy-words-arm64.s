
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bc77b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>:
100bc77b8:     	sub	sp, sp, #0x60
100bc77bc:     	stp	x26, x25, [sp, #0x10]
100bc77c0:     	stp	x24, x23, [sp, #0x20]
100bc77c4:     	stp	x22, x21, [sp, #0x30]
100bc77c8:     	stp	x20, x19, [sp, #0x40]
100bc77cc:     	stp	x29, x30, [sp, #0x50]
100bc77d0:     	add	x29, sp, #0x50
100bc77d4:     	cbz	w1, 0x100bc797c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1c4>
100bc77d8:     	ldr	x8, [x4, #0x18]
100bc77dc:     	cbz	x8, 0x100bc78ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bc77e0:     	mov	x8, #0x0                ; =0
100bc77e4:     	mov	w9, w1
100bc77e8:     	mov	x10, #0xa9c5            ; =43461
100bc77ec:     	movk	x10, #0x2e62, lsl #16
100bc77f0:     	movk	x10, #0x7aea, lsl #32
100bc77f4:     	movk	x10, #0xf135, lsl #48
100bc77f8:     	mul	x9, x9, x10
100bc77fc:     	add	x9, x9, w2, uxtw
100bc7800:     	mul	x9, x9, x10
100bc7804:     	add	x9, x9, w3, uxtw
100bc7808:     	mul	x9, x9, x10
100bc780c:     	ror	x11, x9, #0x2c
100bc7810:     	lsr	x12, x11, #57
100bc7814:     	ldp	x10, x9, [x4]
100bc7818:     	dup.8b	v0, w12
100bc781c:     	movi.2d	v1, #0xffffffffffffffff
100bc7820:     	and	x11, x11, x9
100bc7824:     	ldr	d2, [x10, x11]
100bc7828:     	cmeq.8b	v3, v2, v0
100bc782c:     	fmov	x12, d3
100bc7830:     	ands	x12, x12, #0x8080808080808080
100bc7834:     	b.eq	0x100bc787c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bc7838:     	rbit	x13, x12
100bc783c:     	clz	x13, x13
100bc7840:     	add	x13, x11, x13, lsr #3
100bc7844:     	and	x13, x13, x9
100bc7848:     	sub	x13, x10, x13, lsl #4
100bc784c:     	ldur	w14, [x13, #-0x10]
100bc7850:     	cmp	w1, w14
100bc7854:     	b.ne	0x100bc7870 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bc7858:     	ldur	w14, [x13, #-0xc]
100bc785c:     	cmp	w2, w14
100bc7860:     	b.ne	0x100bc7870 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xb8>
100bc7864:     	ldur	w14, [x13, #-0x8]
100bc7868:     	cmp	w3, w14
100bc786c:     	b.eq	0x100bc7984 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1cc>
100bc7870:     	sub	x13, x12, #0x2
100bc7874:     	ands	x12, x13, x12
100bc7878:     	b.ne	0x100bc7838 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bc787c:     	cmeq.8b	v2, v2, v1
100bc7880:     	fmov	x12, d2
100bc7884:     	cbnz	x12, 0x100bc78ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xf4>
100bc7888:     	add	x8, x8, #0x8
100bc788c:     	add	x11, x11, x8
100bc7890:     	and	x11, x11, x9
100bc7894:     	ldr	d2, [x10, x11]
100bc7898:     	cmeq.8b	v3, v2, v0
100bc789c:     	fmov	x12, d3
100bc78a0:     	ands	x12, x12, #0x8080808080808080
100bc78a4:     	b.ne	0x100bc7838 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x80>
100bc78a8:     	b	0x100bc787c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0xc4>
100bc78ac:     	tbnz	w1, #0x1, 0x100bc798c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1d4>
100bc78b0:     	ldr	x10, [x0, #0xc8]
100bc78b4:     	tbnz	w2, #0x1, 0x100bc79ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x1f4>
100bc78b8:     	ldr	x8, [x0, #0xc8]
100bc78bc:     	cmp	x8, x10
100bc78c0:     	csel	x10, x8, x10, lo
100bc78c4:     	tbnz	w3, #0x1, 0x100bc79d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x21c>
100bc78c8:     	ldr	x8, [x0, #0xc8]
100bc78cc:     	cmp	x8, x10
100bc78d0:     	csel	x21, x8, x10, lo
100bc78d4:     	cmp	x21, x8
100bc78d8:     	b.ne	0x100bc7a04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x24c>
100bc78dc:     	lsr	w9, w1, #2
100bc78e0:     	ldr	x8, [x0, #0x58]
100bc78e4:     	cmp	x8, x9
100bc78e8:     	b.ls	0x100bc7c28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x470>
100bc78ec:     	ldr	x12, [x0, #0xd8]
100bc78f0:     	tst	w1, #0x1
100bc78f4:     	csel	x13, xzr, x12, eq
100bc78f8:     	lsr	w10, w2, #2
100bc78fc:     	cmp	x8, x10
100bc7900:     	b.ls	0x100bc7c3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x484>
100bc7904:     	lsr	w11, w3, #2
100bc7908:     	cmp	x8, x11
100bc790c:     	b.ls	0x100bc7c50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x498>
100bc7910:     	ldr	x8, [x0, #0x50]
100bc7914:     	ldr	x9, [x8, x9, lsl #3]
100bc7918:     	ldr	x10, [x8, x10, lsl #3]
100bc791c:     	eor	x9, x13, x9
100bc7920:     	tst	w2, #0x1
100bc7924:     	csel	x13, xzr, x12, eq
100bc7928:     	eor	x10, x10, x13
100bc792c:     	ldr	x8, [x8, x11, lsl #3]
100bc7930:     	tst	w3, #0x1
100bc7934:     	csel	x11, xzr, x12, eq
100bc7938:     	eor	x8, x8, x11
100bc793c:     	bic	x11, x9, x10
100bc7940:     	tst	x11, x8
100bc7944:     	mov	w12, #0x2               ; =2
100bc7948:     	csel	w12, wzr, w12, eq
100bc794c:     	and	x9, x10, x9
100bc7950:     	bics	xzr, x9, x8
100bc7954:     	mov	w10, #0x4               ; =4
100bc7958:     	csel	w10, wzr, w10, eq
100bc795c:     	tst	x9, x8
100bc7960:     	mov	w9, #0x8                ; =8
100bc7964:     	csel	w9, wzr, w9, eq
100bc7968:     	bics	xzr, x11, x8
100bc796c:     	cinc	w8, w12, ne
100bc7970:     	orr	w9, w10, w9
100bc7974:     	orr	w20, w8, w9
100bc7978:     	b	0x100bc7bdc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x424>
100bc797c:     	mov	w20, #0x0               ; =0
100bc7980:     	b	0x100bc7bf4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bc7984:     	ldurb	w20, [x13, #-0x4]
100bc7988:     	b	0x100bc7bf4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x43c>
100bc798c:     	lsr	w8, w1, #2
100bc7990:     	ldr	x9, [x0, #0x40]
100bc7994:     	cmp	x9, x8
100bc7998:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc799c:     	ldr	x9, [x0, #0x38]
100bc79a0:     	lsl	x8, x8, #4
100bc79a4:     	ldr	w10, [x9, x8]
100bc79a8:     	tbz	w2, #0x1, 0x100bc78b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x100>
100bc79ac:     	lsr	w8, w2, #2
100bc79b0:     	ldr	x9, [x0, #0x40]
100bc79b4:     	cmp	x9, x8
100bc79b8:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc79bc:     	ldr	x9, [x0, #0x38]
100bc79c0:     	lsl	x8, x8, #4
100bc79c4:     	ldr	w8, [x9, x8]
100bc79c8:     	cmp	x8, x10
100bc79cc:     	csel	x10, x8, x10, lo
100bc79d0:     	tbz	w3, #0x1, 0x100bc78c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x110>
100bc79d4:     	lsr	w8, w3, #2
100bc79d8:     	ldr	x9, [x0, #0x40]
100bc79dc:     	cmp	x9, x8
100bc79e0:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc79e4:     	ldr	x9, [x0, #0x38]
100bc79e8:     	lsl	x8, x8, #4
100bc79ec:     	ldr	w9, [x9, x8]
100bc79f0:     	ldr	x8, [x0, #0xc8]
100bc79f4:     	cmp	x9, x10
100bc79f8:     	csel	x21, x9, x10, lo
100bc79fc:     	cmp	x21, x8
100bc7a00:     	b.eq	0x100bc78dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x124>
100bc7a04:     	mov	x8, x1
100bc7a08:     	tbz	w1, #0x1, 0x100bc7a40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bc7a0c:     	lsr	w8, w1, #2
100bc7a10:     	ldr	x9, [x0, #0x40]
100bc7a14:     	cmp	x9, x8
100bc7a18:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc7a1c:     	ldr	x9, [x0, #0x38]
100bc7a20:     	add	x9, x9, x8, lsl #4
100bc7a24:     	ldr	w10, [x9]
100bc7a28:     	mov	x8, x1
100bc7a2c:     	cmp	x21, x10
100bc7a30:     	b.ne	0x100bc7a40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x288>
100bc7a34:     	ldr	w8, [x9, #0x4]
100bc7a38:     	and	w9, w1, #0x1
100bc7a3c:     	eor	w8, w8, w9
100bc7a40:     	mov	x9, x2
100bc7a44:     	tbz	w2, #0x1, 0x100bc7a7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bc7a48:     	lsr	w9, w2, #2
100bc7a4c:     	ldr	x10, [x0, #0x40]
100bc7a50:     	cmp	x10, x9
100bc7a54:     	b.ls	0x100bc7c64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4ac>
100bc7a58:     	ldr	x10, [x0, #0x38]
100bc7a5c:     	add	x10, x10, x9, lsl #4
100bc7a60:     	ldr	w11, [x10]
100bc7a64:     	mov	x9, x2
100bc7a68:     	cmp	x21, x11
100bc7a6c:     	b.ne	0x100bc7a7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x2c4>
100bc7a70:     	ldr	w9, [x10, #0x4]
100bc7a74:     	and	w10, w2, #0x1
100bc7a78:     	eor	w9, w9, w10
100bc7a7c:     	mov	x22, x1
100bc7a80:     	mov	x10, x3
100bc7a84:     	tbz	w3, #0x1, 0x100bc7abc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bc7a88:     	lsr	w10, w3, #2
100bc7a8c:     	ldr	x1, [x0, #0x40]
100bc7a90:     	cmp	x1, x10
100bc7a94:     	b.ls	0x100bc7c78 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4c0>
100bc7a98:     	ldr	x11, [x0, #0x38]
100bc7a9c:     	add	x11, x11, x10, lsl #4
100bc7aa0:     	ldr	w12, [x11]
100bc7aa4:     	mov	x10, x3
100bc7aa8:     	cmp	x21, x12
100bc7aac:     	b.ne	0x100bc7abc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x304>
100bc7ab0:     	ldr	w10, [x11, #0x4]
100bc7ab4:     	and	w11, w3, #0x1
100bc7ab8:     	eor	w10, w10, w11
100bc7abc:     	mov	x23, x2
100bc7ac0:     	mov	x24, x3
100bc7ac4:     	mov	x25, x0
100bc7ac8:     	mov	x1, x8
100bc7acc:     	mov	x2, x9
100bc7ad0:     	mov	x3, x10
100bc7ad4:     	mov	x19, x4
100bc7ad8:     	bl	0x100bc77b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bc7adc:     	and	w8, w0, #0xff
100bc7ae0:     	cmp	w8, #0xf
100bc7ae4:     	b.ne	0x100bc7af8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x340>
100bc7ae8:     	mov	w20, #0xf               ; =15
100bc7aec:     	mov	x4, x19
100bc7af0:     	mov	x3, x24
100bc7af4:     	b	0x100bc7bd4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x41c>
100bc7af8:     	mov	x20, x0
100bc7afc:     	mov	x1, x22
100bc7b00:     	mov	x10, x24
100bc7b04:     	mov	x11, x23
100bc7b08:     	mov	x0, x25
100bc7b0c:     	tbz	w22, #0x1, 0x100bc7b48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bc7b10:     	mov	x9, x22
100bc7b14:     	lsr	w8, w22, #2
100bc7b18:     	ldr	x1, [x0, #0x40]
100bc7b1c:     	cmp	x1, x8
100bc7b20:     	b.ls	0x100bc7c88 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x4d0>
100bc7b24:     	ldr	x12, [x0, #0x38]
100bc7b28:     	add	x8, x12, x8, lsl #4
100bc7b2c:     	ldr	w12, [x8]
100bc7b30:     	mov	x1, x9
100bc7b34:     	cmp	x21, x12
100bc7b38:     	b.ne	0x100bc7b48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x390>
100bc7b3c:     	ldr	w8, [x8, #0x8]
100bc7b40:     	and	w9, w9, #0x1
100bc7b44:     	eor	w1, w8, w9
100bc7b48:     	mov	x2, x11
100bc7b4c:     	tbz	w11, #0x1, 0x100bc7b84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bc7b50:     	lsr	w8, w11, #2
100bc7b54:     	ldr	x9, [x0, #0x40]
100bc7b58:     	cmp	x9, x8
100bc7b5c:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc7b60:     	ldr	x9, [x0, #0x38]
100bc7b64:     	add	x8, x9, x8, lsl #4
100bc7b68:     	ldr	w9, [x8]
100bc7b6c:     	mov	x2, x11
100bc7b70:     	cmp	x21, x9
100bc7b74:     	b.ne	0x100bc7b84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x3cc>
100bc7b78:     	ldr	w8, [x8, #0x8]
100bc7b7c:     	and	w9, w11, #0x1
100bc7b80:     	eor	w2, w8, w9
100bc7b84:     	mov	x3, x10
100bc7b88:     	tbz	w10, #0x1, 0x100bc7bc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bc7b8c:     	lsr	w8, w10, #2
100bc7b90:     	ldr	x9, [x0, #0x40]
100bc7b94:     	cmp	x9, x8
100bc7b98:     	b.ls	0x100bc7c14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x45c>
100bc7b9c:     	ldr	x9, [x0, #0x38]
100bc7ba0:     	add	x8, x9, x8, lsl #4
100bc7ba4:     	ldr	w9, [x8]
100bc7ba8:     	mov	x3, x10
100bc7bac:     	cmp	x21, x9
100bc7bb0:     	b.ne	0x100bc7bc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_+0x408>
100bc7bb4:     	ldr	w8, [x8, #0x8]
100bc7bb8:     	and	w9, w10, #0x1
100bc7bbc:     	eor	w3, w8, w9
100bc7bc0:     	mov	x4, x19
100bc7bc4:     	bl	0x100bc77b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E9occupancyB6_>
100bc7bc8:     	mov	x3, x24
100bc7bcc:     	mov	x4, x19
100bc7bd0:     	orr	w20, w0, w20
100bc7bd4:     	mov	x2, x23
100bc7bd8:     	mov	x1, x22
100bc7bdc:     	stp	w1, w2, [sp, #0x4]
100bc7be0:     	str	w3, [sp, #0xc]
100bc7be4:     	add	x1, sp, #0x4
100bc7be8:     	mov	x0, x4
100bc7bec:     	mov	x2, x20
100bc7bf0:     	bl	0x100c2b4ec <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bc7bf4:     	mov	x0, x20
100bc7bf8:     	ldp	x29, x30, [sp, #0x50]
100bc7bfc:     	ldp	x20, x19, [sp, #0x40]
100bc7c00:     	ldp	x22, x21, [sp, #0x30]
100bc7c04:     	ldp	x24, x23, [sp, #0x20]
100bc7c08:     	ldp	x26, x25, [sp, #0x10]
100bc7c0c:     	add	sp, sp, #0x60
100bc7c10:     	ret
100bc7c14:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c18:     	add	x2, x2, #0x2f8
100bc7c1c:     	mov	x0, x8
100bc7c20:     	mov	x1, x9
100bc7c24:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c28:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c2c:     	add	x2, x2, #0x580
100bc7c30:     	mov	x0, x9
100bc7c34:     	mov	x1, x8
100bc7c38:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c3c:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c40:     	add	x2, x2, #0x580
100bc7c44:     	mov	x0, x10
100bc7c48:     	mov	x1, x8
100bc7c4c:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c50:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c54:     	add	x2, x2, #0x580
100bc7c58:     	mov	x0, x11
100bc7c5c:     	mov	x1, x8
100bc7c60:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c64:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c68:     	add	x2, x2, #0x2f8
100bc7c6c:     	mov	x0, x9
100bc7c70:     	mov	x1, x10
100bc7c74:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c78:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c7c:     	add	x2, x2, #0x2f8
100bc7c80:     	mov	x0, x10
100bc7c84:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc7c88:     	adrp	x2, 0x101501000 <dyld_stub_binder+0x101501000>
100bc7c8c:     	add	x2, x2, #0x2f8
100bc7c90:     	mov	x0, x8
100bc7c94:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
