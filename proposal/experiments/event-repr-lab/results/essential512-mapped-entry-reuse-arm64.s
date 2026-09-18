
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d93860 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>:
100d93860:     	stp	x28, x27, [sp, #-0x60]!
100d93864:     	stp	x26, x25, [sp, #0x10]
100d93868:     	stp	x24, x23, [sp, #0x20]
100d9386c:     	stp	x22, x21, [sp, #0x30]
100d93870:     	stp	x20, x19, [sp, #0x40]
100d93874:     	stp	x29, x30, [sp, #0x50]
100d93878:     	add	x29, sp, #0x50
100d9387c:     	sub	sp, sp, #0x260
100d93880:     	ldr	w8, [x1, #0xf0]
100d93884:     	str	x4, [sp, #0x160]
100d93888:     	str	x8, [sp]
100d9388c:     	cmp	x4, x8
100d93890:     	b.ne	0x100d93c10 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100d93894:     	mov	x25, x6
100d93898:     	mov	x21, x5
100d9389c:     	mov	x23, x4
100d938a0:     	mov	x22, x2
100d938a4:     	mov	x20, x1
100d938a8:     	mov	x19, x0
100d938ac:     	ldp	x24, x10, [x29, #0x18]
100d938b0:     	cbz	x4, 0x100d93974 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x114>
100d938b4:     	mov	x11, #0x0               ; =0
100d938b8:     	lsl	x9, x23, #2
100d938bc:     	mov	w12, #0x1               ; =1
100d938c0:     	mov	x13, x9
100d938c4:     	mov	x14, x3
100d938c8:     	ldr	w15, [x14], #0x4
100d938cc:     	cmp	w15, w8
100d938d0:     	b.hs	0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d938d4:     	lsr	x16, x11, x15
100d938d8:     	tbnz	w16, #0x0, 0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d938dc:     	lsl	x15, x12, x15
100d938e0:     	orr	x11, x15, x11
100d938e4:     	subs	x13, x13, #0x4
100d938e8:     	b.ne	0x100d938c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x68>
100d938ec:     	str	x7, [sp, #0x160]
100d938f0:     	str	x23, [sp]
100d938f4:     	cmp	x7, x23
100d938f8:     	b.ne	0x100d93c10 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100d938fc:     	mov	x11, #0x0               ; =0
100d93900:     	mov	w12, #0x1               ; =1
100d93904:     	mov	x13, x9
100d93908:     	mov	x14, x25
100d9390c:     	ldr	w15, [x14], #0x4
100d93910:     	cmp	w15, w8
100d93914:     	b.hs	0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d93918:     	lsr	x16, x11, x15
100d9391c:     	tbnz	w16, #0x0, 0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d93920:     	lsl	x15, x12, x15
100d93924:     	orr	x11, x15, x11
100d93928:     	subs	x13, x13, #0x4
100d9392c:     	b.ne	0x100d9390c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xac>
100d93930:     	str	x10, [sp, #0x160]
100d93934:     	str	x23, [sp]
100d93938:     	cmp	x10, x23
100d9393c:     	b.ne	0x100d93c10 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100d93940:     	mov	x10, #0x0               ; =0
100d93944:     	mov	w11, #0x1               ; =1
100d93948:     	mov	x12, x24
100d9394c:     	ldr	w13, [x12], #0x4
100d93950:     	cmp	w13, w8
100d93954:     	b.hs	0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d93958:     	lsr	x14, x10, x13
100d9395c:     	tbnz	w14, #0x0, 0x100d93bf8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100d93960:     	lsl	x13, x11, x13
100d93964:     	orr	x10, x13, x10
100d93968:     	subs	x9, x9, #0x4
100d9396c:     	b.ne	0x100d9394c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xec>
100d93970:     	b	0x100d9398c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x12c>
100d93974:     	str	x7, [sp, #0x160]
100d93978:     	str	xzr, [sp]
100d9397c:     	cbnz	x7, 0x100d93c10 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100d93980:     	str	x10, [sp, #0x160]
100d93984:     	str	xzr, [sp]
100d93988:     	cbnz	x10, 0x100d93c10 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100d9398c:     	ldr	x26, [x29, #0x10]
100d93990:     	lsr	x8, x26, x8
100d93994:     	str	x8, [sp]
100d93998:     	cbnz	x8, 0x100d93c34 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d4>
100d9399c:     	sub	x0, x29, #0xf0
100d939a0:     	mov	x1, x3
100d939a4:     	mov	x2, x23
100d939a8:     	mov	x3, x24
100d939ac:     	mov	x4, x23
100d939b0:     	bl	0x100d711f8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d939b4:     	mov	x0, sp
100d939b8:     	mov	x1, x25
100d939bc:     	mov	x2, x23
100d939c0:     	mov	x3, x24
100d939c4:     	mov	x4, x23
100d939c8:     	bl	0x100d711f8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d939cc:     	ldp	q0, q1, [x29, #-0xf0]
100d939d0:     	stp	q0, q1, [sp, #0x160]
100d939d4:     	ldur	q0, [x29, #-0xd0]
100d939d8:     	ldp	q1, q2, [sp]
100d939dc:     	stp	q0, q1, [sp, #0x180]
100d939e0:     	ldr	q0, [sp, #0x20]
100d939e4:     	stp	q2, q0, [sp, #0x1a0]
100d939e8:     	mov	w8, #0x4                ; =4
100d939ec:     	stp	xzr, x8, [sp]
100d939f0:     	str	xzr, [sp, #0x10]
100d939f4:     	cbz	x26, 0x100d93a7c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x21c>
100d939f8:     	mov	x27, #0x0               ; =0
100d939fc:     	mov	w8, #0x4                ; =4
100d93a00:     	b	0x100d93a28 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1c8>
100d93a04:     	ldr	x8, [sp, #0x8]
100d93a08:     	rbit	x9, x26
100d93a0c:     	clz	x9, x9
100d93a10:     	str	w9, [x8, x27, lsl #2]
100d93a14:     	add	x27, x27, #0x1
100d93a18:     	str	x27, [sp, #0x10]
100d93a1c:     	sub	x9, x26, #0x1
100d93a20:     	ands	x26, x9, x26
100d93a24:     	b.eq	0x100d93a40 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1e0>
100d93a28:     	ldr	x9, [sp]
100d93a2c:     	cmp	x27, x9
100d93a30:     	b.ne	0x100d93a08 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a8>
100d93a34:     	mov	x0, sp
100d93a38:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d93a3c:     	b	0x100d93a04 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a4>
100d93a40:     	ldp	x26, x25, [sp]
100d93a44:     	cbz	x27, 0x100d93a88 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x228>
100d93a48:     	mov	x9, #0x0                ; =0
100d93a4c:     	mov	x8, #0x0                ; =0
100d93a50:     	mov	w10, #0x1               ; =1
100d93a54:     	ldr	w0, [x25, x9, lsl #2]
100d93a58:     	cmp	x23, x0
100d93a5c:     	b.ls	0x100d93c5c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3fc>
100d93a60:     	ldr	w11, [x24, x0, lsl #2]
100d93a64:     	lsl	x11, x10, x11
100d93a68:     	orr	x8, x11, x8
100d93a6c:     	add	x9, x9, #0x1
100d93a70:     	cmp	x27, x9
100d93a74:     	b.ne	0x100d93a54 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1f4>
100d93a78:     	b	0x100d93a8c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100d93a7c:     	mov	x8, #0x0                ; =0
100d93a80:     	mov	w25, #0x4               ; =4
100d93a84:     	b	0x100d93a8c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100d93a88:     	mov	x8, #0x0                ; =0
100d93a8c:     	mov	x9, sp
100d93a90:     	add	x23, x9, #0xc8
100d93a94:     	movi.2d	v0, #0000000000000000
100d93a98:     	stp	q0, q0, [x23, #0x60]
100d93a9c:     	mov	x9, sp
100d93aa0:     	stp	q0, q0, [x23, #0x40]
100d93aa4:     	stur	q0, [x9, #0xf8]
100d93aa8:     	stur	q0, [x9, #0xe8]
100d93aac:     	stur	q0, [x9, #0xd8]
100d93ab0:     	stur	q0, [x9, #0xc8]
100d93ab4:     	ldrb	w9, [x20, #0xf6]
100d93ab8:     	ldrb	w10, [x20, #0xf7]
100d93abc:     	stp	xzr, xzr, [sp, #0xb0]
100d93ac0:     	ldp	q0, q1, [sp, #0x160]
100d93ac4:     	ldp	q2, q3, [sp, #0x180]
100d93ac8:     	stp	q1, q2, [sp, #0x20]
100d93acc:     	ldp	q1, q2, [sp, #0x1a0]
100d93ad0:     	stp	q1, q2, [sp, #0x50]
100d93ad4:     	str	q3, [sp, #0x40]
100d93ad8:     	str	xzr, [sp, #0x148]
100d93adc:     	str	x8, [sp, #0xc0]
100d93ae0:     	adrp	x8, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100d93ae4:     	add	x8, x8, #0x7c8
100d93ae8:     	stp	x8, xzr, [sp, #0x70]
100d93aec:     	stp	xzr, xzr, [sp, #0x80]
100d93af0:     	strb	w9, [sp, #0x150]
100d93af4:     	ldr	q1, [x20]
100d93af8:     	stp	q1, q0, [sp]
100d93afc:     	strb	w10, [sp, #0x151]
100d93b00:     	mov	w8, #0x8                ; =8
100d93b04:     	stp	x8, xzr, [sp, #0x90]
100d93b08:     	stp	xzr, x8, [sp, #0xa0]
100d93b0c:     	cbz	x26, 0x100d93b18 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x2b8>
100d93b10:     	mov	x0, x25
100d93b14:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93b18:     	stur	w22, [x29, #-0xe0]
100d93b1c:     	stp	xzr, xzr, [x29, #-0xf0]
100d93b20:     	sub	x0, x29, #0x88
100d93b24:     	sub	x1, x29, #0xf0
100d93b28:     	mov	x2, x20
100d93b2c:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d93b30:     	add	x22, sp, #0x160
100d93b34:     	ldur	x8, [x29, #-0x78]
100d93b38:     	ldur	q0, [x22, #0xc8]
100d93b3c:     	stur	q0, [x29, #-0x70]
100d93b40:     	stur	x8, [x29, #-0x60]
100d93b44:     	str	x8, [sp, #0x170]
100d93b48:     	str	q0, [sp, #0x160]
100d93b4c:     	stur	w21, [x29, #-0xe0]
100d93b50:     	stp	xzr, xzr, [x29, #-0xf0]
100d93b54:     	sub	x0, x29, #0x88
100d93b58:     	sub	x1, x29, #0xf0
100d93b5c:     	mov	x2, x20
100d93b60:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d93b64:     	ldur	x8, [x29, #-0x78]
100d93b68:     	ldur	q0, [x22, #0xc8]
100d93b6c:     	stur	q0, [x22, #0x18]
100d93b70:     	str	x8, [sp, #0x188]
100d93b74:     	ldp	q0, q1, [sp, #0x160]
100d93b78:     	ldr	q2, [sp, #0x180]
100d93b7c:     	stp	q1, q2, [x29, #-0xe0]
100d93b80:     	stur	q0, [x29, #-0xf0]
100d93b84:     	mov	x0, sp
100d93b88:     	sub	x2, x29, #0xf0
100d93b8c:     	mov	x1, x20
100d93b90:     	bl	0x1007b37fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
100d93b94:     	ldp	q0, q1, [x23, #0x40]
100d93b98:     	stur	q0, [x19, #0x48]
100d93b9c:     	stur	q1, [x19, #0x58]
100d93ba0:     	ldp	q0, q1, [x23, #0x60]
100d93ba4:     	stur	q0, [x19, #0x68]
100d93ba8:     	stur	q1, [x19, #0x78]
100d93bac:     	ldp	q0, q1, [x23]
100d93bb0:     	stur	q0, [x19, #0x8]
100d93bb4:     	stur	q1, [x19, #0x18]
100d93bb8:     	ldp	q0, q1, [x23, #0x20]
100d93bbc:     	stur	q0, [x19, #0x28]
100d93bc0:     	ldr	x8, [x23, #0x80]
100d93bc4:     	str	x8, [x19, #0x88]
100d93bc8:     	stur	q1, [x19, #0x38]
100d93bcc:     	str	w0, [x19]
100d93bd0:     	mov	x0, sp
100d93bd4:     	bl	0x100828b64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d93bd8:     	add	sp, sp, #0x260
100d93bdc:     	ldp	x29, x30, [sp, #0x50]
100d93be0:     	ldp	x20, x19, [sp, #0x40]
100d93be4:     	ldp	x22, x21, [sp, #0x30]
100d93be8:     	ldp	x24, x23, [sp, #0x20]
100d93bec:     	ldp	x26, x25, [sp, #0x10]
100d93bf0:     	ldp	x28, x27, [sp], #0x60
100d93bf4:     	ret
100d93bf8:     	adrp	x0, 0x101482000 <dyld_stub_binder+0x101482000>
100d93bfc:     	add	x0, x0, #0x407
100d93c00:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
100d93c04:     	add	x2, x2, #0x6a8
100d93c08:     	mov	w1, #0x2f               ; =47
100d93c0c:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d93c10:     	adrp	x3, 0x101482000 <dyld_stub_binder+0x101482000>
100d93c14:     	add	x3, x3, #0x3f1
100d93c18:     	adrp	x5, 0x101644000 <dyld_stub_binder+0x101644000>
100d93c1c:     	add	x5, x5, #0x690
100d93c20:     	add	x1, sp, #0x160
100d93c24:     	mov	x2, sp
100d93c28:     	mov	w0, #0x0                ; =0
100d93c2c:     	mov	w4, #0x2d               ; =45
100d93c30:     	bl	0x1013ba230 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d93c34:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100d93c38:     	add	x2, x2, #0x78
100d93c3c:     	adrp	x3, 0x101482000 <dyld_stub_binder+0x101482000>
100d93c40:     	add	x3, x3, #0x71c
100d93c44:     	adrp	x5, 0x101644000 <dyld_stub_binder+0x101644000>
100d93c48:     	add	x5, x5, #0xd18
100d93c4c:     	mov	x1, sp
100d93c50:     	mov	w0, #0x0                ; =0
100d93c54:     	mov	w4, #0x57               ; =87
100d93c58:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d93c5c:     	adrp	x2, 0x101607000 <dyld_stub_binder+0x101607000>
100d93c60:     	add	x2, x2, #0x168
100d93c64:     	mov	x1, x23
100d93c68:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d93c6c:     	brk	#0x1
100d93c70:     	mov	x19, x0
100d93c74:     	sub	x0, x29, #0xf0
100d93c78:     	bl	0x10082764c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d93c7c:     	mov	x0, x19
100d93c80:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93c84:     	mov	x19, x0
100d93c88:     	mov	x0, sp
100d93c8c:     	bl	0x100828b64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d93c90:     	mov	x0, x19
100d93c94:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93c98:     	mov	x19, x0
100d93c9c:     	ldr	x8, [sp]
100d93ca0:     	cbz	x8, 0x100d93cac <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x44c>
100d93ca4:     	ldr	x0, [sp, #0x8]
100d93ca8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93cac:     	add	x0, sp, #0x160
100d93cb0:     	bl	0x10080cfdc <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d93cb4:     	mov	x0, x19
100d93cb8:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93cbc:     	mov	x19, x0
100d93cc0:     	add	x0, sp, #0x160
100d93cc4:     	bl	0x10080cfdc <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d93cc8:     	cbnz	x26, 0x100d93cd4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x474>
100d93ccc:     	mov	x0, x19
100d93cd0:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
100d93cd4:     	mov	x0, x25
100d93cd8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100d93cdc:     	mov	x0, x19
100d93ce0:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
