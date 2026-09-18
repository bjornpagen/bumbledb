
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d67818 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_>:
100d67818:     	stp	x28, x27, [sp, #-0x60]!
100d6781c:     	stp	x26, x25, [sp, #0x10]
100d67820:     	stp	x24, x23, [sp, #0x20]
100d67824:     	stp	x22, x21, [sp, #0x30]
100d67828:     	stp	x20, x19, [sp, #0x40]
100d6782c:     	stp	x29, x30, [sp, #0x50]
100d67830:     	add	x29, sp, #0x50
100d67834:     	sub	sp, sp, #0x1d0
100d67838:     	ldr	w8, [x0, #0xe0]
100d6783c:     	str	x3, [sp, #0xd0]
100d67840:     	str	x8, [sp]
100d67844:     	cmp	x3, x8
100d67848:     	b.ne	0x100d67bc0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3a8>
100d6784c:     	mov	x24, x7
100d67850:     	mov	x25, x5
100d67854:     	mov	x20, x4
100d67858:     	mov	x22, x3
100d6785c:     	mov	x21, x1
100d67860:     	mov	x19, x0
100d67864:     	ldp	x23, x10, [x29, #0x10]
100d67868:     	cbz	x3, 0x100d6792c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x114>
100d6786c:     	mov	x11, #0x0               ; =0
100d67870:     	lsl	x9, x22, #2
100d67874:     	mov	w12, #0x1               ; =1
100d67878:     	mov	x13, x9
100d6787c:     	mov	x14, x2
100d67880:     	ldr	w15, [x14], #0x4
100d67884:     	cmp	w15, w8
100d67888:     	b.hs	0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d6788c:     	lsr	x16, x11, x15
100d67890:     	tbnz	w16, #0x0, 0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d67894:     	lsl	x15, x12, x15
100d67898:     	orr	x11, x15, x11
100d6789c:     	subs	x13, x13, #0x4
100d678a0:     	b.ne	0x100d67880 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x68>
100d678a4:     	str	x6, [sp, #0xd0]
100d678a8:     	str	x22, [sp]
100d678ac:     	cmp	x6, x22
100d678b0:     	b.ne	0x100d67bc0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3a8>
100d678b4:     	mov	x11, #0x0               ; =0
100d678b8:     	mov	w12, #0x1               ; =1
100d678bc:     	mov	x13, x9
100d678c0:     	mov	x14, x25
100d678c4:     	ldr	w15, [x14], #0x4
100d678c8:     	cmp	w15, w8
100d678cc:     	b.hs	0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d678d0:     	lsr	x16, x11, x15
100d678d4:     	tbnz	w16, #0x0, 0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d678d8:     	lsl	x15, x12, x15
100d678dc:     	orr	x11, x15, x11
100d678e0:     	subs	x13, x13, #0x4
100d678e4:     	b.ne	0x100d678c4 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0xac>
100d678e8:     	str	x10, [sp, #0xd0]
100d678ec:     	str	x22, [sp]
100d678f0:     	cmp	x10, x22
100d678f4:     	b.ne	0x100d67bc0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3a8>
100d678f8:     	mov	x10, #0x0               ; =0
100d678fc:     	mov	w11, #0x1               ; =1
100d67900:     	mov	x12, x23
100d67904:     	ldr	w13, [x12], #0x4
100d67908:     	cmp	w13, w8
100d6790c:     	b.hs	0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d67910:     	lsr	x14, x10, x13
100d67914:     	tbnz	w14, #0x0, 0x100d67ba8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x390>
100d67918:     	lsl	x13, x11, x13
100d6791c:     	orr	x10, x13, x10
100d67920:     	subs	x9, x9, #0x4
100d67924:     	b.ne	0x100d67904 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0xec>
100d67928:     	b	0x100d67944 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x12c>
100d6792c:     	str	x6, [sp, #0xd0]
100d67930:     	str	xzr, [sp]
100d67934:     	cbnz	x6, 0x100d67bc0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3a8>
100d67938:     	str	x10, [sp, #0xd0]
100d6793c:     	str	xzr, [sp]
100d67940:     	cbnz	x10, 0x100d67bc0 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3a8>
100d67944:     	lsr	x8, x24, x8
100d67948:     	str	x8, [sp]
100d6794c:     	cbnz	x8, 0x100d67be4 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3cc>
100d67950:     	sub	x0, x29, #0xf0
100d67954:     	mov	x1, x2
100d67958:     	mov	x2, x22
100d6795c:     	mov	x3, x23
100d67960:     	mov	x4, x22
100d67964:     	bl	0x100d91eb4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB4_3Map7compose>
100d67968:     	mov	x0, sp
100d6796c:     	mov	x1, x25
100d67970:     	mov	x2, x22
100d67974:     	mov	x3, x23
100d67978:     	mov	x4, x22
100d6797c:     	bl	0x100d91eb4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB4_3Map7compose>
100d67980:     	ldp	q0, q1, [x29, #-0xf0]
100d67984:     	stp	q0, q1, [sp, #0xd0]
100d67988:     	ldur	q0, [x29, #-0xd0]
100d6798c:     	ldp	q1, q2, [sp]
100d67990:     	stp	q0, q1, [sp, #0xf0]
100d67994:     	ldr	q0, [sp, #0x20]
100d67998:     	stp	q2, q0, [sp, #0x110]
100d6799c:     	mov	w8, #0x4                ; =4
100d679a0:     	stp	xzr, x8, [sp]
100d679a4:     	str	xzr, [sp, #0x10]
100d679a8:     	cbz	x24, 0x100d67a30 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x218>
100d679ac:     	mov	x26, #0x0               ; =0
100d679b0:     	mov	w8, #0x4                ; =4
100d679b4:     	b	0x100d679dc <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x1c4>
100d679b8:     	ldr	x8, [sp, #0x8]
100d679bc:     	rbit	x9, x24
100d679c0:     	clz	x9, x9
100d679c4:     	str	w9, [x8, x26, lsl #2]
100d679c8:     	add	x26, x26, #0x1
100d679cc:     	str	x26, [sp, #0x10]
100d679d0:     	sub	x9, x24, #0x1
100d679d4:     	ands	x24, x9, x24
100d679d8:     	b.eq	0x100d679f4 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x1dc>
100d679dc:     	ldr	x9, [sp]
100d679e0:     	cmp	x26, x9
100d679e4:     	b.ne	0x100d679bc <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x1a4>
100d679e8:     	mov	x0, sp
100d679ec:     	bl	0x1013af1dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d679f0:     	b	0x100d679b8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x1a0>
100d679f4:     	ldp	x25, x24, [sp]
100d679f8:     	cbz	x26, 0x100d67a40 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x228>
100d679fc:     	mov	x9, #0x0                ; =0
100d67a00:     	mov	x8, #0x0                ; =0
100d67a04:     	mov	w10, #0x1               ; =1
100d67a08:     	ldr	w0, [x24, x9, lsl #2]
100d67a0c:     	cmp	x22, x0
100d67a10:     	b.ls	0x100d67c0c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x3f4>
100d67a14:     	ldr	w11, [x23, x0, lsl #2]
100d67a18:     	lsl	x11, x10, x11
100d67a1c:     	orr	x8, x11, x8
100d67a20:     	add	x9, x9, #0x1
100d67a24:     	cmp	x26, x9
100d67a28:     	b.ne	0x100d67a08 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x1f0>
100d67a2c:     	b	0x100d67a44 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x22c>
100d67a30:     	mov	x25, #0x0               ; =0
100d67a34:     	mov	x8, #0x0                ; =0
100d67a38:     	mov	w24, #0x4               ; =4
100d67a3c:     	b	0x100d67a44 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x22c>
100d67a40:     	mov	x8, #0x0                ; =0
100d67a44:     	mov	x22, sp
100d67a48:     	movi.2d	v0, #0000000000000000
100d67a4c:     	stur	q0, [x22, #0xb8]
100d67a50:     	stur	q0, [x22, #0xa8]
100d67a54:     	stur	q0, [x22, #0x98]
100d67a58:     	stur	q0, [x22, #0x88]
100d67a5c:     	ldp	q0, q1, [sp, #0x110]
100d67a60:     	stp	q0, q1, [sp, #0x40]
100d67a64:     	ldp	q0, q1, [sp, #0xd0]
100d67a68:     	stp	q0, q1, [sp]
100d67a6c:     	ldp	q0, q1, [sp, #0xf0]
100d67a70:     	stp	q0, q1, [sp, #0x20]
100d67a74:     	str	xzr, [sp, #0xc8]
100d67a78:     	stp	xzr, x8, [sp, #0x78]
100d67a7c:     	adrp	x8, 0x101515000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x208f>
100d67a80:     	add	x8, x8, #0x8e0
100d67a84:     	stp	x8, xzr, [sp, #0x60]
100d67a88:     	str	xzr, [sp, #0x70]
100d67a8c:     	cbz	x25, 0x100d67a98 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x280>
100d67a90:     	mov	x0, x24
100d67a94:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67a98:     	stur	w21, [x29, #-0xe0]
100d67a9c:     	stp	xzr, xzr, [x29, #-0xf0]
100d67aa0:     	sub	x0, x29, #0x88
100d67aa4:     	sub	x1, x29, #0xf0
100d67aa8:     	mov	x2, x19
100d67aac:     	bl	0x100004ff8 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB3_10Restricted9normalizeKm1_EB9_>
100d67ab0:     	sub	x21, x29, #0x88
100d67ab4:     	ldur	x8, [x29, #-0x78]
100d67ab8:     	ldr	q0, [x21]
100d67abc:     	stur	q0, [x29, #-0x70]
100d67ac0:     	stur	x8, [x29, #-0x60]
100d67ac4:     	str	x8, [sp, #0xe0]
100d67ac8:     	str	q0, [sp, #0xd0]
100d67acc:     	stur	w20, [x29, #-0xe0]
100d67ad0:     	stp	xzr, xzr, [x29, #-0xf0]
100d67ad4:     	sub	x0, x29, #0x88
100d67ad8:     	sub	x1, x29, #0xf0
100d67adc:     	mov	x2, x19
100d67ae0:     	bl	0x100004ff8 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB3_10Restricted9normalizeKm1_EB9_>
100d67ae4:     	ldur	x8, [x29, #-0x78]
100d67ae8:     	ldr	q0, [x21]
100d67aec:     	stur	q0, [x22, #0xe8]
100d67af0:     	str	x8, [sp, #0xf8]
100d67af4:     	ldp	q0, q1, [sp, #0xd0]
100d67af8:     	ldr	q2, [sp, #0xf0]
100d67afc:     	stp	q1, q2, [x29, #-0xe0]
100d67b00:     	stur	q0, [x29, #-0xf0]
100d67b04:     	mov	x0, sp
100d67b08:     	sub	x2, x29, #0xf0
100d67b0c:     	mov	x1, x19
100d67b10:     	bl	0x100752c20 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
100d67b14:     	mov	x19, x0
100d67b18:     	ldr	x8, [sp]
100d67b1c:     	cbz	x8, 0x100d67b28 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x310>
100d67b20:     	ldr	x0, [sp, #0x8]
100d67b24:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67b28:     	ldr	x8, [sp, #0x18]
100d67b2c:     	cbz	x8, 0x100d67b38 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x320>
100d67b30:     	ldr	x0, [sp, #0x20]
100d67b34:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67b38:     	ldr	x8, [sp, #0x30]
100d67b3c:     	cbz	x8, 0x100d67b48 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x330>
100d67b40:     	ldr	x0, [sp, #0x38]
100d67b44:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67b48:     	ldr	x8, [sp, #0x48]
100d67b4c:     	cbz	x8, 0x100d67b58 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x340>
100d67b50:     	ldr	x0, [sp, #0x50]
100d67b54:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67b58:     	ldr	x9, [sp, #0x68]
100d67b5c:     	cbz	x9, 0x100d67b84 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x36c>
100d67b60:     	lsl	x8, x9, #6
100d67b64:     	sub	x8, x8, x9, lsl #3
100d67b68:     	add	x9, x8, x9
100d67b6c:     	cmn	x9, #0x41
100d67b70:     	b.eq	0x100d67b84 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x36c>
100d67b74:     	ldr	x9, [sp, #0x60]
100d67b78:     	sub	x8, x9, x8
100d67b7c:     	sub	x0, x8, #0x38
100d67b80:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67b84:     	mov	x0, x19
100d67b88:     	add	sp, sp, #0x1d0
100d67b8c:     	ldp	x29, x30, [sp, #0x50]
100d67b90:     	ldp	x20, x19, [sp, #0x40]
100d67b94:     	ldp	x22, x21, [sp, #0x30]
100d67b98:     	ldp	x24, x23, [sp, #0x20]
100d67b9c:     	ldp	x26, x25, [sp, #0x10]
100d67ba0:     	ldp	x28, x27, [sp], #0x60
100d67ba4:     	ret
100d67ba8:     	adrp	x0, 0x101475000 <dyld_stub_binder+0x101475000>
100d67bac:     	add	x0, x0, #0xc47
100d67bb0:     	adrp	x2, 0x101638000 <dyld_stub_binder+0x101638000>
100d67bb4:     	add	x2, x2, #0xcd0
100d67bb8:     	mov	w1, #0x2f               ; =47
100d67bbc:     	bl	0x1013ae274 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d67bc0:     	adrp	x3, 0x101475000 <dyld_stub_binder+0x101475000>
100d67bc4:     	add	x3, x3, #0xc31
100d67bc8:     	adrp	x5, 0x101638000 <dyld_stub_binder+0x101638000>
100d67bcc:     	add	x5, x5, #0xcb8
100d67bd0:     	add	x1, sp, #0xd0
100d67bd4:     	mov	x2, sp
100d67bd8:     	mov	w0, #0x0                ; =0
100d67bdc:     	mov	w4, #0x2d               ; =45
100d67be0:     	bl	0x1013ae2b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d67be4:     	adrp	x2, 0x101515000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x208f>
100d67be8:     	add	x2, x2, #0x190
100d67bec:     	adrp	x3, 0x101475000 <dyld_stub_binder+0x101475000>
100d67bf0:     	add	x3, x3, #0x6a1
100d67bf4:     	adrp	x5, 0x101638000 <dyld_stub_binder+0x101638000>
100d67bf8:     	add	x5, x5, #0x248
100d67bfc:     	mov	x1, sp
100d67c00:     	mov	w0, #0x0                ; =0
100d67c04:     	mov	w4, #0x57               ; =87
100d67c08:     	bl	0x1013ae2e0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d67c0c:     	adrp	x2, 0x1015fa000 <dyld_stub_binder+0x1015fa000>
100d67c10:     	add	x2, x2, #0xf58
100d67c14:     	mov	x1, x22
100d67c18:     	bl	0x1013ae3dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d67c1c:     	brk	#0x1
100d67c20:     	mov	x19, x0
100d67c24:     	sub	x0, x29, #0xf0
100d67c28:     	bl	0x10082480c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d67c2c:     	mov	x0, x19
100d67c30:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67c34:     	mov	x19, x0
100d67c38:     	mov	x0, sp
100d67c3c:     	bl	0x100825d24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d67c40:     	mov	x0, x19
100d67c44:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67c48:     	mov	x19, x0
100d67c4c:     	ldr	x8, [sp]
100d67c50:     	cbz	x8, 0x100d67c5c <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x444>
100d67c54:     	ldr	x0, [sp, #0x8]
100d67c58:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67c5c:     	add	x0, sp, #0xd0
100d67c60:     	bl	0x10080a19c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d67c64:     	mov	x0, x19
100d67c68:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67c6c:     	mov	x19, x0
100d67c70:     	add	x0, sp, #0xd0
100d67c74:     	bl	0x10080a19c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d67c78:     	cbnz	x25, 0x100d67c84 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E14mapped_productBb_+0x46c>
100d67c7c:     	mov	x0, x19
100d67c80:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67c84:     	mov	x0, x24
100d67c88:     	bl	0x1013b67f8 <dyld_stub_binder+0x1013b67f8>
100d67c8c:     	mov	x0, x19
100d67c90:     	bl	0x1013b6648 <dyld_stub_binder+0x1013b6648>
100d67c94:     	nop
100d67c98:     	nop
100d67c9c:     	nop
100d67ca0:     	nop
100d67ca4:     	nop
100d67ca8:     	nop
100d67cac:     	nop
100d67cb0:     	nop
100d67cb4:     	nop
100d67cb8:     	nop
100d67cbc:     	nop
