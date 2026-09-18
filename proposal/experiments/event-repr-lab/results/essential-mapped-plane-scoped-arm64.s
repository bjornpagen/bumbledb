
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>:
100d06968:     	sub	sp, sp, #0x1e0
100d0696c:     	stp	d15, d14, [sp, #0x140]
100d06970:     	stp	d13, d12, [sp, #0x150]
100d06974:     	stp	d11, d10, [sp, #0x160]
100d06978:     	stp	d9, d8, [sp, #0x170]
100d0697c:     	stp	x28, x27, [sp, #0x180]
100d06980:     	stp	x26, x25, [sp, #0x190]
100d06984:     	stp	x24, x23, [sp, #0x1a0]
100d06988:     	stp	x22, x21, [sp, #0x1b0]
100d0698c:     	stp	x20, x19, [sp, #0x1c0]
100d06990:     	stp	x29, x30, [sp, #0x1d0]
100d06994:     	add	x29, sp, #0x1d0
100d06998:     	str	x6, [sp, #0x98]
100d0699c:     	mov	x24, x5
100d069a0:     	mov	x20, x4
100d069a4:     	mov	x27, x3
100d069a8:     	mov	x25, x2
100d069ac:     	mov	x23, x1
100d069b0:     	mov	x28, x0
100d069b4:     	str	x4, [sp, #0xb8]
100d069b8:     	ldr	x21, [x2]
100d069bc:     	ldp	x22, x26, [x3, #0x8]
100d069c0:     	cbz	x21, 0x100d06a04 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9c>
100d069c4:     	mov	x8, #0x0                ; =0
100d069c8:     	mov	w9, #0x1                ; =1
100d069cc:     	mov	x10, x21
100d069d0:     	rbit	x11, x10
100d069d4:     	clz	x0, x11
100d069d8:     	cmp	x0, x26
100d069dc:     	b.hs	0x100d07778 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe10>
100d069e0:     	ldr	w11, [x22, x0, lsl #2]
100d069e4:     	lsl	x11, x9, x11
100d069e8:     	orr	x8, x11, x8
100d069ec:     	sub	x11, x10, #0x1
100d069f0:     	ands	x10, x11, x10
100d069f4:     	b.ne	0x100d069d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x68>
100d069f8:     	ands	x8, x8, x20
100d069fc:     	stur	x8, [x29, #-0xb8]
100d06a00:     	b.ne	0x100d076b0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd48>
100d06a04:     	ldr	w2, [x25, #0x10]
100d06a08:     	sub	x0, x29, #0xb8
100d06a0c:     	add	x1, x23, #0x40
100d06a10:     	str	x2, [sp, #0x78]
100d06a14:     	bl	0x100ee9280 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100d06a18:     	ldur	w8, [x29, #-0xb8]
100d06a1c:     	cbz	w8, 0x100d06aa4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x13c>
100d06a20:     	cmp	w8, #0x1
100d06a24:     	str	x20, [sp, #0x60]
100d06a28:     	b.ne	0x100d06abc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x154>
100d06a2c:     	ldp	x23, x8, [x29, #-0xb0]
100d06a30:     	str	x8, [sp, #0xa0]
100d06a34:     	ldur	x27, [x29, #-0xa0]
100d06a38:     	mov	w8, #0x4                ; =4
100d06a3c:     	stp	xzr, x8, [x29, #-0xb8]
100d06a40:     	stur	xzr, [x29, #-0xa8]
100d06a44:     	str	x28, [sp, #0x58]
100d06a48:     	cbz	x21, 0x100d06de0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x478>
100d06a4c:     	mov	x28, #0x0               ; =0
100d06a50:     	mov	w8, #0x4                ; =4
100d06a54:     	mov	w9, #0x1                ; =1
100d06a58:     	b	0x100d06a84 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x11c>
100d06a5c:     	ldur	x8, [x29, #-0xb0]
100d06a60:     	rbit	x9, x21
100d06a64:     	clz	x9, x9
100d06a68:     	str	w9, [x8, x28]
100d06a6c:     	stur	x19, [x29, #-0xa8]
100d06a70:     	sub	x10, x21, #0x1
100d06a74:     	add	x28, x28, #0x4
100d06a78:     	add	x9, x19, #0x1
100d06a7c:     	ands	x21, x10, x21
100d06a80:     	b.eq	0x100d06cc8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x360>
100d06a84:     	mov	x19, x9
100d06a88:     	sub	x9, x9, #0x1
100d06a8c:     	ldur	x10, [x29, #-0xb8]
100d06a90:     	cmp	x9, x10
100d06a94:     	b.ne	0x100d06a60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf8>
100d06a98:     	sub	x0, x29, #0xb8
100d06a9c:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d06aa0:     	b	0x100d06a5c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf4>
100d06aa4:     	ldr	x8, [sp, #0x78]
100d06aa8:     	and	w8, w8, #0x1
100d06aac:     	strb	w8, [x28, #0x8]
100d06ab0:     	mov	x8, #-0x2               ; =-2
100d06ab4:     	str	x8, [x28]
100d06ab8:     	b	0x100d07680 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100d06abc:     	ldp	w19, w8, [x29, #-0xb4]
100d06ac0:     	ldur	w9, [x29, #-0xac]
100d06ac4:     	ldr	x11, [sp, #0x98]
100d06ac8:     	ldr	x10, [x11, #0x38]
100d06acc:     	add	x10, x10, #0x1
100d06ad0:     	str	x10, [x11, #0x38]
100d06ad4:     	tbz	w24, #0x0, 0x100d06d88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x420>
100d06ad8:     	lsr	x10, x21, x19
100d06adc:     	and	x11, x10, #0x1
100d06ae0:     	stur	x11, [x29, #-0xb8]
100d06ae4:     	tbnz	w10, #0x0, 0x100d07714 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdac>
100d06ae8:     	ldr	x10, [sp, #0x78]
100d06aec:     	and	w10, w10, #0x1
100d06af0:     	eor	w8, w8, w10
100d06af4:     	eor	w20, w9, w10
100d06af8:     	stur	w8, [x29, #-0xa8]
100d06afc:     	ldr	x24, [x25, #0x8]
100d06b00:     	stp	x21, x24, [x29, #-0xb8]
100d06b04:     	add	x0, sp, #0xc0
100d06b08:     	sub	x1, x29, #0xb8
100d06b0c:     	mov	x2, x23
100d06b10:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d06b14:     	stur	w20, [x29, #-0xa8]
100d06b18:     	stp	x21, x24, [x29, #-0xb8]
100d06b1c:     	add	x0, sp, #0xd8
100d06b20:     	sub	x1, x29, #0xb8
100d06b24:     	mov	x2, x23
100d06b28:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100d06b2c:     	sub	x0, x29, #0xe0
100d06b30:     	add	x2, sp, #0xc0
100d06b34:     	mov	x1, x23
100d06b38:     	mov	x3, x27
100d06b3c:     	ldr	x21, [sp, #0x60]
100d06b40:     	mov	x4, x21
100d06b44:     	mov	w5, #0x1                ; =1
100d06b48:     	ldr	x20, [sp, #0x98]
100d06b4c:     	mov	x6, x20
100d06b50:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100d06b54:     	sub	x0, x29, #0xb8
100d06b58:     	add	x2, sp, #0xd8
100d06b5c:     	mov	x1, x23
100d06b60:     	mov	x3, x27
100d06b64:     	mov	x4, x21
100d06b68:     	mov	w5, #0x1                ; =1
100d06b6c:     	mov	x6, x20
100d06b70:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100d06b74:     	cmp	x26, x19
100d06b78:     	b.ls	0x100d07808 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xea0>
100d06b7c:     	ldr	w21, [x22, x19, lsl #2]
100d06b80:     	ldr	x10, [sp, #0x60]
100d06b84:     	lsr	x8, x10, x21
100d06b88:     	and	x9, x8, #0x1
100d06b8c:     	stur	x9, [x29, #-0xc0]
100d06b90:     	tbz	w8, #0x0, 0x100d07734 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdcc>
100d06b94:     	fmov	d0, x10
100d06b98:     	cnt.8b	v0, v0
100d06b9c:     	addv.8b	b0, v0
100d06ba0:     	fmov	x8, d0
100d06ba4:     	and	x9, x8, #0x3f
100d06ba8:     	mov	w10, #0x1               ; =1
100d06bac:     	lsl	x8, x10, x8
100d06bb0:     	lsr	x8, x8, #6
100d06bb4:     	cmp	x9, #0x6
100d06bb8:     	cinc	x20, x8, lo
100d06bbc:     	cbz	x20, 0x100d07420 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xab8>
100d06bc0:     	lsl	x19, x20, #3
100d06bc4:     	mov	x0, x19
100d06bc8:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
100d06bcc:     	cbz	x0, 0x100d07830 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xec8>
100d06bd0:     	mov	x9, #0x0                ; =0
100d06bd4:     	and	x8, x21, #0x3f
100d06bd8:     	mov	x10, #-0x1              ; =-1
100d06bdc:     	lsl	x8, x10, x8
100d06be0:     	ldr	x10, [sp, #0x60]
100d06be4:     	bic	x8, x10, x8
100d06be8:     	fmov	d0, x8
100d06bec:     	cnt.8b	v0, v0
100d06bf0:     	addv.8b	b0, v0
100d06bf4:     	fmov	x10, d0
100d06bf8:     	add	w8, w10, #0x3a
100d06bfc:     	mov	w11, #0x1               ; =1
100d06c00:     	lsl	x11, x11, x8
100d06c04:     	ldp	x12, x13, [x29, #-0xe0]
100d06c08:     	ldp	x1, x14, [x29, #-0xd0]
100d06c0c:     	sub	x15, x9, w13, uxtb
100d06c10:     	ldp	x21, x17, [x29, #-0xb8]
100d06c14:     	ldp	x16, x2, [x29, #-0xa8]
100d06c18:     	adrp	x3, 0x1015b2000 <dyld_stub_binder+0x1015b2000>
100d06c1c:     	add	x3, x3, #0x648
100d06c20:     	mov	x8, #0x0                ; =0
100d06c24:     	b	0x100d06c48 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100d06c28:     	tst	w17, #0x1
100d06c2c:     	csel	x6, x4, x9, ne
100d06c30:     	bic	x4, x5, x4
100d06c34:     	orr	x4, x6, x4
100d06c38:     	str	x4, [x0, x8, lsl #3]
100d06c3c:     	add	x8, x8, #0x1
100d06c40:     	cmp	x20, x8
100d06c44:     	b.eq	0x100d06cc0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x358>
100d06c48:     	cmp	x10, #0x6
100d06c4c:     	b.hs	0x100d06c68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x300>
100d06c50:     	ldr	x4, [x3, x10, lsl #3]
100d06c54:     	mvn	x4, x4
100d06c58:     	mov	x5, x15
100d06c5c:     	cmn	x12, #0x2
100d06c60:     	b.ne	0x100d06c7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x314>
100d06c64:     	b	0x100d06c8c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100d06c68:     	tst	x8, x11
100d06c6c:     	csetm	x4, ne
100d06c70:     	mov	x5, x15
100d06c74:     	cmn	x12, #0x2
100d06c78:     	b.eq	0x100d06c8c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100d06c7c:     	cmp	x8, x1
100d06c80:     	b.hs	0x100d077e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe78>
100d06c84:     	ldr	x5, [x13, x8, lsl #3]
100d06c88:     	eor	x5, x14, x5
100d06c8c:     	cmn	x21, #0x2
100d06c90:     	b.eq	0x100d06c28 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2c0>
100d06c94:     	cmp	x8, x16
100d06c98:     	b.hs	0x100d077d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe6c>
100d06c9c:     	ldr	x6, [x17, x8, lsl #3]
100d06ca0:     	eor	x6, x2, x6
100d06ca4:     	and	x6, x6, x4
100d06ca8:     	bic	x4, x5, x4
100d06cac:     	orr	x4, x6, x4
100d06cb0:     	str	x4, [x0, x8, lsl #3]
100d06cb4:     	add	x8, x8, #0x1
100d06cb8:     	cmp	x20, x8
100d06cbc:     	b.ne	0x100d06c48 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100d06cc0:     	mov	x8, x20
100d06cc4:     	b	0x100d0742c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xac4>
100d06cc8:     	ldp	x8, x21, [x29, #-0xb8]
100d06ccc:     	str	x8, [sp, #0x40]
100d06cd0:     	str	x21, [sp, #0x30]
100d06cd4:     	cbz	x19, 0x100d06e48 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e0>
100d06cd8:     	ldr	x8, [x25, #0x8]
100d06cdc:     	str	x8, [sp, #0x80]
100d06ce0:     	mov	x24, #-0x1              ; =-1
100d06ce4:     	mov	x19, x23
100d06ce8:     	b	0x100d06d14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x3ac>
100d06cec:     	bic	x27, x27, x23
100d06cf0:     	ldr	x9, [sp, #0x98]
100d06cf4:     	ldr	x8, [x9, #0x20]
100d06cf8:     	add	x8, x8, #0x1
100d06cfc:     	str	x8, [x9, #0x20]
100d06d00:     	mov	x24, x20
100d06d04:     	mov	x23, x25
100d06d08:     	mov	x19, x25
100d06d0c:     	subs	x28, x28, #0x4
100d06d10:     	b.eq	0x100d06e4c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e4>
100d06d14:     	ldr	w8, [x21], #0x4
100d06d18:     	mov	w9, #0x1                ; =1
100d06d1c:     	lsl	x23, x9, x8
100d06d20:     	sub	x9, x23, #0x1
100d06d24:     	and	x9, x9, x27
100d06d28:     	fmov	d0, x9
100d06d2c:     	cnt.8b	v0, v0
100d06d30:     	addv.8b	b0, v0
100d06d34:     	fmov	w4, s0
100d06d38:     	fmov	d0, x27
100d06d3c:     	cnt.8b	v0, v0
100d06d40:     	addv.8b	b0, v0
100d06d44:     	fmov	w3, s0
100d06d48:     	ldr	x9, [sp, #0x80]
100d06d4c:     	lsr	x8, x9, x8
100d06d50:     	sub	x0, x29, #0xb8
100d06d54:     	and	w5, w8, #0x1
100d06d58:     	mov	x1, x19
100d06d5c:     	ldr	x2, [sp, #0xa0]
100d06d60:     	bl	0x100f906d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100d06d64:     	ldp	x20, x25, [x29, #-0xb8]
100d06d68:     	ldur	x8, [x29, #-0xa8]
100d06d6c:     	str	x8, [sp, #0xa0]
100d06d70:     	sub	x8, x24, #0x1
100d06d74:     	cmn	x8, #0x3
100d06d78:     	b.hi	0x100d06cec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100d06d7c:     	mov	x0, x19
100d06d80:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d06d84:     	b	0x100d06cec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100d06d88:     	mov	w8, #0x4                ; =4
100d06d8c:     	stp	xzr, x8, [x29, #-0xb8]
100d06d90:     	stur	xzr, [x29, #-0xa8]
100d06d94:     	mov	x26, #0x0               ; =0
100d06d98:     	cbz	x20, 0x100d0708c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x724>
100d06d9c:     	mov	w8, #0x4                ; =4
100d06da0:     	b	0x100d06dc8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x460>
100d06da4:     	ldur	x8, [x29, #-0xb0]
100d06da8:     	rbit	x9, x20
100d06dac:     	clz	x9, x9
100d06db0:     	str	w9, [x8, x26, lsl #2]
100d06db4:     	add	x26, x26, #0x1
100d06db8:     	stur	x26, [x29, #-0xa8]
100d06dbc:     	sub	x9, x20, #0x1
100d06dc0:     	ands	x20, x9, x20
100d06dc4:     	b.eq	0x100d06df0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x488>
100d06dc8:     	ldur	x9, [x29, #-0xb8]
100d06dcc:     	cmp	x26, x9
100d06dd0:     	b.ne	0x100d06da8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x440>
100d06dd4:     	sub	x0, x29, #0xb8
100d06dd8:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d06ddc:     	b	0x100d06da4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x43c>
100d06de0:     	mov	x19, #-0x1              ; =-1
100d06de4:     	mov	x21, #0x0               ; =0
100d06de8:     	cbnz	x27, 0x100d06e6c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x504>
100d06dec:     	b	0x100d06e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100d06df0:     	ldp	x20, x19, [x29, #-0xb8]
100d06df4:     	cbz	x26, 0x100d074a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb40>
100d06df8:     	lsl	x22, x26, #2
100d06dfc:     	mov	x0, x22
100d06e00:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
100d06e04:     	cbz	x0, 0x100d07840 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xed8>
100d06e08:     	mov	x24, x0
100d06e0c:     	mov	x8, #0x0                ; =0
100d06e10:     	ldp	x9, x1, [x27, #0x20]
100d06e14:     	ldr	w0, [x19, x8, lsl #2]
100d06e18:     	cmp	x1, x0
100d06e1c:     	b.ls	0x100d077c4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe5c>
100d06e20:     	ldr	w10, [x9, x0, lsl #2]
100d06e24:     	str	w10, [x24, x8, lsl #2]
100d06e28:     	add	x8, x8, #0x1
100d06e2c:     	cmp	x26, x8
100d06e30:     	b.ne	0x100d06e14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4ac>
100d06e34:     	cbz	x20, 0x100d06e40 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100d06e38:     	mov	x0, x19
100d06e3c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d06e40:     	ldr	x20, [sp, #0x60]
100d06e44:     	b	0x100d07090 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x728>
100d06e48:     	mov	x20, #-0x1              ; =-1
100d06e4c:     	ldr	x8, [sp, #0x40]
100d06e50:     	cbz	x8, 0x100d06e5c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4f4>
100d06e54:     	ldr	x0, [sp, #0x30]
100d06e58:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d06e5c:     	mov	x19, x20
100d06e60:     	ldp	x28, x20, [sp, #0x58]
100d06e64:     	mov	x21, #0x0               ; =0
100d06e68:     	cbz	x27, 0x100d06e9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100d06e6c:     	mov	w8, #0x1                ; =1
100d06e70:     	mov	x9, x27
100d06e74:     	rbit	x10, x9
100d06e78:     	clz	x0, x10
100d06e7c:     	cmp	x0, x26
100d06e80:     	b.hs	0x100d07788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe20>
100d06e84:     	ldr	w10, [x22, x0, lsl #2]
100d06e88:     	lsl	x10, x8, x10
100d06e8c:     	orr	x21, x10, x21
100d06e90:     	sub	x10, x9, #0x1
100d06e94:     	ands	x9, x10, x9
100d06e98:     	b.ne	0x100d06e74 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x50c>
100d06e9c:     	stur	x21, [x29, #-0xe0]
100d06ea0:     	bics	x8, x21, x20
100d06ea4:     	stur	x8, [x29, #-0xb8]
100d06ea8:     	b.ne	0x100d076d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd68>
100d06eac:     	str	x19, [sp, #0x80]
100d06eb0:     	mov	w25, #0x4               ; =4
100d06eb4:     	stp	xzr, x25, [x29, #-0xb8]
100d06eb8:     	stur	xzr, [x29, #-0xa8]
100d06ebc:     	mov	x19, #0x0               ; =0
100d06ec0:     	cbz	x21, 0x100d07020 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6b8>
100d06ec4:     	mov	w8, #0x4                ; =4
100d06ec8:     	mov	x20, x21
100d06ecc:     	b	0x100d06ef0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x588>
100d06ed0:     	rbit	x9, x20
100d06ed4:     	clz	x9, x9
100d06ed8:     	str	w9, [x8, x19, lsl #2]
100d06edc:     	add	x19, x19, #0x1
100d06ee0:     	stur	x19, [x29, #-0xa8]
100d06ee4:     	sub	x9, x20, #0x1
100d06ee8:     	ands	x20, x9, x20
100d06eec:     	b.eq	0x100d06f0c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5a4>
100d06ef0:     	ldur	x9, [x29, #-0xb8]
100d06ef4:     	cmp	x19, x9
100d06ef8:     	b.ne	0x100d06ed0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100d06efc:     	sub	x0, x29, #0xb8
100d06f00:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d06f04:     	ldur	x8, [x29, #-0xb0]
100d06f08:     	b	0x100d06ed0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100d06f0c:     	ldp	x8, x25, [x29, #-0xb8]
100d06f10:     	cmp	x8, #0x0
100d06f14:     	cset	w8, eq
100d06f18:     	str	w8, [sp, #0x40]
100d06f1c:     	mov	w8, #0x4                ; =4
100d06f20:     	stp	xzr, x8, [x29, #-0xb8]
100d06f24:     	stur	xzr, [x29, #-0xa8]
100d06f28:     	cbz	x27, 0x100d07038 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6d0>
100d06f2c:     	str	x23, [sp, #0x30]
100d06f30:     	mov	x20, #0x0               ; =0
100d06f34:     	mov	w8, #0x4                ; =4
100d06f38:     	b	0x100d06f5c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5f4>
100d06f3c:     	rbit	x9, x27
100d06f40:     	clz	x9, x9
100d06f44:     	str	w9, [x8, x28, lsl #2]
100d06f48:     	add	x20, x28, #0x1
100d06f4c:     	stur	x20, [x29, #-0xa8]
100d06f50:     	sub	x9, x27, #0x1
100d06f54:     	ands	x27, x9, x27
100d06f58:     	b.eq	0x100d06f7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x614>
100d06f5c:     	mov	x28, x20
100d06f60:     	ldur	x9, [x29, #-0xb8]
100d06f64:     	cmp	x20, x9
100d06f68:     	b.ne	0x100d06f3c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100d06f6c:     	sub	x0, x29, #0xb8
100d06f70:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d06f74:     	ldur	x8, [x29, #-0xb0]
100d06f78:     	b	0x100d06f3c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100d06f7c:     	ldp	x8, x24, [x29, #-0xb8]
100d06f80:     	cbz	x20, 0x100d07078 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x710>
100d06f84:     	str	x8, [sp, #0x20]
100d06f88:     	lsl	x0, x20, #2
100d06f8c:     	mov	x23, x0
100d06f90:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
100d06f94:     	cbz	x0, 0x100d07820 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xeb8>
100d06f98:     	mov	x27, x0
100d06f9c:     	cbz	x19, 0x100d06fec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x684>
100d06fa0:     	mov	x9, #0x0                ; =0
100d06fa4:     	lsl	x8, x19, #2
100d06fa8:     	b	0x100d06fbc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x654>
100d06fac:     	str	w10, [x27, x9, lsl #2]
100d06fb0:     	cmp	x9, x28
100d06fb4:     	add	x9, x9, #0x1
100d06fb8:     	b.eq	0x100d06ffc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x694>
100d06fbc:     	ldr	w0, [x24, x9, lsl #2]
100d06fc0:     	cmp	x26, x0
100d06fc4:     	b.ls	0x100d0779c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe34>
100d06fc8:     	mov	x10, #0x0               ; =0
100d06fcc:     	ldr	w11, [x22, x0, lsl #2]
100d06fd0:     	mov	x12, x8
100d06fd4:     	ldr	w13, [x25, x10, lsl #2]
100d06fd8:     	cmp	w13, w11
100d06fdc:     	b.eq	0x100d06fac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x644>
100d06fe0:     	add	x10, x10, #0x1
100d06fe4:     	subs	x12, x12, #0x4
100d06fe8:     	b.ne	0x100d06fd4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x66c>
100d06fec:     	adrp	x0, 0x10175a000 <dyld_stub_binder+0x10175a000>
100d06ff0:     	add	x0, x0, #0xe0
100d06ff4:     	bl	0x101506e74 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100d06ff8:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d06ffc:     	ldr	x19, [sp, #0x80]
100d07000:     	ldr	x8, [sp, #0x20]
100d07004:     	ldr	x28, [sp, #0x58]
100d07008:     	cbz	x8, 0x100d07014 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100d0700c:     	mov	x0, x24
100d07010:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07014:     	mov	x24, x27
100d07018:     	ldr	x23, [sp, #0x30]
100d0701c:     	b	0x100d07044 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6dc>
100d07020:     	mov	w8, #0x1                ; =1
100d07024:     	str	w8, [sp, #0x40]
100d07028:     	mov	w8, #0x4                ; =4
100d0702c:     	stp	xzr, x8, [x29, #-0xb8]
100d07030:     	stur	xzr, [x29, #-0xa8]
100d07034:     	cbnz	x27, 0x100d06f2c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5c4>
100d07038:     	mov	x20, #0x0               ; =0
100d0703c:     	mov	w24, #0x4               ; =4
100d07040:     	ldr	x19, [sp, #0x80]
100d07044:     	mov	x8, #0x0                ; =0
100d07048:     	lsl	x9, x20, #2
100d0704c:     	str	x24, [sp, #0x30]
100d07050:     	cbz	x9, 0x100d074e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb78>
100d07054:     	ldr	w10, [x24, x8, lsl #2]
100d07058:     	sub	x9, x9, #0x4
100d0705c:     	cmp	x8, x10
100d07060:     	add	x8, x8, #0x1
100d07064:     	b.eq	0x100d07050 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6e8>
100d07068:     	cmn	x19, #0x1
100d0706c:     	b.eq	0x100d07468 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb00>
100d07070:     	ldr	x1, [sp, #0xa0]
100d07074:     	b	0x100d074bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100d07078:     	mov	w27, #0x4               ; =4
100d0707c:     	ldr	x19, [sp, #0x80]
100d07080:     	ldr	x28, [sp, #0x58]
100d07084:     	cbnz	x8, 0x100d0700c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6a4>
100d07088:     	b	0x100d07014 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100d0708c:     	mov	w24, #0x4               ; =4
100d07090:     	str	x28, [sp, #0x58]
100d07094:     	fmov	d0, x20
100d07098:     	cnt.8b	v0, v0
100d0709c:     	addv.8b	b0, v0
100d070a0:     	fmov	x8, d0
100d070a4:     	mov	w9, #0x1                ; =1
100d070a8:     	lsl	x20, x9, x8
100d070ac:     	ldr	x9, [sp, #0x98]
100d070b0:     	ldr	x8, [x9, #0x40]
100d070b4:     	add	x8, x8, x20
100d070b8:     	str	x8, [x9, #0x40]
100d070bc:     	add	x8, x20, #0x3f
100d070c0:     	lsr	x22, x8, #6
100d070c4:     	lsl	x19, x22, #3
100d070c8:     	mov	x0, x19
100d070cc:     	mov	w1, #0x1                ; =1
100d070d0:     	bl	0x10150f0e4 <dyld_stub_binder+0x10150f0e4>
100d070d4:     	cbz	x0, 0x100d077f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe90>
100d070d8:     	mov	x21, x0
100d070dc:     	mov	x19, #0x0               ; =0
100d070e0:     	ldr	x25, [x25, #0x8]
100d070e4:     	and	x8, x26, #0xfffffffffffffffe
100d070e8:     	neg	x8, x8
100d070ec:     	str	x8, [sp, #0x98]
100d070f0:     	mov	w28, #0x1               ; =1
100d070f4:     	adrp	x8, 0x1015a7000 <GCC_except_table9672+0xc>
100d070f8:     	ldr	q0, [x8, #0x700]
100d070fc:     	str	q0, [sp, #0xa0]
100d07100:     	mov	w8, #0x2                ; =2
100d07104:     	dup.2d	v0, x8
100d07108:     	str	q0, [sp, #0x80]
100d0710c:     	mov	w8, #0x4                ; =4
100d07110:     	dup.2d	v1, x8
100d07114:     	mov	w8, #0x8                ; =8
100d07118:     	dup.2d	v0, x8
100d0711c:     	stp	q0, q1, [sp, #0x30]
100d07120:     	mov	w8, #0xc                ; =12
100d07124:     	dup.2d	v1, x8
100d07128:     	mov	w8, #0x10               ; =16
100d0712c:     	dup.2d	v0, x8
100d07130:     	stp	q0, q1, [sp, #0x10]
100d07134:     	adrp	x8, 0x1015a7000 <GCC_except_table9672+0xc>
100d07138:     	ldr	q0, [x8, #0x720]
100d0713c:     	str	q0, [sp]
100d07140:     	mov	w27, #0x3f              ; =63
100d07144:     	dup.2d	v0, x27
100d07148:     	str	q0, [sp, #0x60]
100d0714c:     	movi.2s	v8, #0x3f
100d07150:     	b	0x100d07160 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7f8>
100d07154:     	add	x19, x19, #0x1
100d07158:     	cmp	x19, x20
100d0715c:     	b.eq	0x100d07408 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaa0>
100d07160:     	mov	x2, x25
100d07164:     	cbz	x26, 0x100d073d8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa70>
100d07168:     	cmp	x26, #0x1
100d0716c:     	b.ne	0x100d0717c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x814>
100d07170:     	mov	x9, #0x0                ; =0
100d07174:     	mov	x8, #0x0                ; =0
100d07178:     	b	0x100d073b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100d0717c:     	dup.2d	v0, x19
100d07180:     	cmp	x26, #0x10
100d07184:     	b.hs	0x100d07194 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x82c>
100d07188:     	mov	x10, #0x0               ; =0
100d0718c:     	mov	x8, #0x0                ; =0
100d07190:     	b	0x100d07340 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9d8>
100d07194:     	movi.2d	v1, #0000000000000000
100d07198:     	add	x8, x24, #0x20
100d0719c:     	movi.2d	v2, #0000000000000000
100d071a0:     	and	x9, x26, #0xfffffffffffffff0
100d071a4:     	ldr	q4, [sp, #0xa0]
100d071a8:     	ldp	q6, q15, [sp]
100d071ac:     	movi.2d	v3, #0000000000000000
100d071b0:     	movi.2d	v7, #0000000000000000
100d071b4:     	movi.2d	v16, #0000000000000000
100d071b8:     	movi.2d	v5, #0000000000000000
100d071bc:     	movi.2d	v18, #0000000000000000
100d071c0:     	movi.2d	v17, #0000000000000000
100d071c4:     	ldp	q13, q12, [sp, #0x30]
100d071c8:     	ldr	q14, [sp, #0x20]
100d071cc:     	movi.4s	v8, #0x3f
100d071d0:     	add.2d	v19, v4, v12
100d071d4:     	add.2d	v20, v6, v12
100d071d8:     	add.2d	v21, v4, v13
100d071dc:     	add.2d	v22, v6, v13
100d071e0:     	add.2d	v23, v4, v14
100d071e4:     	add.2d	v24, v6, v14
100d071e8:     	ldp	q25, q26, [x8, #-0x20]
100d071ec:     	dup.2d	v27, x27
100d071f0:     	ldp	q28, q29, [x8], #0x40
100d071f4:     	and.16b	v30, v6, v27
100d071f8:     	and.16b	v31, v4, v27
100d071fc:     	and.16b	v20, v20, v27
100d07200:     	and.16b	v19, v19, v27
100d07204:     	and.16b	v22, v22, v27
100d07208:     	and.16b	v21, v21, v27
100d0720c:     	and.16b	v24, v24, v27
100d07210:     	and.16b	v23, v23, v27
100d07214:     	neg.2d	v27, v31
100d07218:     	ushl.2d	v27, v0, v27
100d0721c:     	neg.2d	v30, v30
100d07220:     	ushl.2d	v30, v0, v30
100d07224:     	neg.2d	v19, v19
100d07228:     	ushl.2d	v19, v0, v19
100d0722c:     	neg.2d	v20, v20
100d07230:     	ushl.2d	v20, v0, v20
100d07234:     	neg.2d	v21, v21
100d07238:     	ushl.2d	v21, v0, v21
100d0723c:     	neg.2d	v22, v22
100d07240:     	ushl.2d	v22, v0, v22
100d07244:     	neg.2d	v23, v23
100d07248:     	ushl.2d	v23, v0, v23
100d0724c:     	neg.2d	v24, v24
100d07250:     	ushl.2d	v24, v0, v24
100d07254:     	dup.2d	v31, x28
100d07258:     	and.16b	v30, v30, v31
100d0725c:     	and.16b	v27, v27, v31
100d07260:     	and.16b	v20, v20, v31
100d07264:     	and.16b	v19, v19, v31
100d07268:     	and.16b	v22, v22, v31
100d0726c:     	and.16b	v21, v21, v31
100d07270:     	and.16b	v24, v24, v31
100d07274:     	and.16b	v23, v23, v31
100d07278:     	and.16b	v25, v25, v8
100d0727c:     	and.16b	v26, v26, v8
100d07280:     	and.16b	v28, v28, v8
100d07284:     	and.16b	v29, v29, v8
100d07288:     	ushll2.2d	v31, v25, #0x0
100d0728c:     	ushll.2d	v25, v25, #0x0
100d07290:     	ushll2.2d	v9, v26, #0x0
100d07294:     	ushll.2d	v26, v26, #0x0
100d07298:     	ushll2.2d	v10, v28, #0x0
100d0729c:     	ushll.2d	v28, v28, #0x0
100d072a0:     	ushll2.2d	v11, v29, #0x0
100d072a4:     	ushll.2d	v29, v29, #0x0
100d072a8:     	ushl.2d	v25, v27, v25
100d072ac:     	ushl.2d	v27, v30, v31
100d072b0:     	ushl.2d	v19, v19, v26
100d072b4:     	ushl.2d	v20, v20, v9
100d072b8:     	ushl.2d	v21, v21, v28
100d072bc:     	ushl.2d	v22, v22, v10
100d072c0:     	ushl.2d	v23, v23, v29
100d072c4:     	ushl.2d	v24, v24, v11
100d072c8:     	orr.16b	v3, v27, v3
100d072cc:     	orr.16b	v2, v25, v2
100d072d0:     	orr.16b	v16, v20, v16
100d072d4:     	orr.16b	v7, v19, v7
100d072d8:     	orr.16b	v18, v22, v18
100d072dc:     	orr.16b	v5, v21, v5
100d072e0:     	orr.16b	v1, v24, v1
100d072e4:     	orr.16b	v17, v23, v17
100d072e8:     	add.2d	v6, v6, v15
100d072ec:     	add.2d	v4, v4, v15
100d072f0:     	subs	x9, x9, #0x10
100d072f4:     	b.ne	0x100d071d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x868>
100d072f8:     	orr.16b	v2, v7, v2
100d072fc:     	orr.16b	v3, v16, v3
100d07300:     	orr.16b	v3, v18, v3
100d07304:     	orr.16b	v2, v5, v2
100d07308:     	orr.16b	v2, v17, v2
100d0730c:     	orr.16b	v1, v1, v3
100d07310:     	orr.16b	v1, v2, v1
100d07314:     	mov	d2, v1[1]
100d07318:     	orr.8b	v1, v1, v2
100d0731c:     	fmov	x8, d1
100d07320:     	and	x9, x26, #0xfffffffffffffff0
100d07324:     	cmp	x26, x9
100d07328:     	movi.2s	v8, #0x3f
100d0732c:     	b.eq	0x100d073d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100d07330:     	and	x10, x26, #0xfffffffffffffff0
100d07334:     	and	x9, x26, #0xfffffffffffffff0
100d07338:     	and	x11, x26, #0xe
100d0733c:     	cbz	x11, 0x100d073b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100d07340:     	fmov	d1, x8
100d07344:     	dup.2d	v2, x10
100d07348:     	ldr	q3, [sp, #0xa0]
100d0734c:     	orr.16b	v2, v2, v3
100d07350:     	ldr	x8, [sp, #0x98]
100d07354:     	add	x8, x8, x10
100d07358:     	add	x9, x24, x10, lsl #2
100d0735c:     	ldr	q6, [sp, #0x80]
100d07360:     	ldr	q7, [sp, #0x60]
100d07364:     	ldr	d3, [x9], #0x8
100d07368:     	and.16b	v4, v2, v7
100d0736c:     	neg.2d	v4, v4
100d07370:     	ushl.2d	v4, v0, v4
100d07374:     	dup.2d	v5, x28
100d07378:     	and.16b	v4, v4, v5
100d0737c:     	and.8b	v3, v3, v8
100d07380:     	ushll.2d	v3, v3, #0x0
100d07384:     	ushl.2d	v3, v4, v3
100d07388:     	orr.16b	v1, v3, v1
100d0738c:     	add.2d	v2, v2, v6
100d07390:     	adds	x8, x8, #0x2
100d07394:     	b.ne	0x100d07364 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9fc>
100d07398:     	mov	d0, v1[1]
100d0739c:     	orr.8b	v0, v1, v0
100d073a0:     	fmov	x8, d0
100d073a4:     	and	x9, x26, #0xfffffffffffffffe
100d073a8:     	and	x10, x26, #0xfffffffffffffffe
100d073ac:     	cmp	x26, x10
100d073b0:     	b.eq	0x100d073d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100d073b4:     	ldr	w10, [x24, x9, lsl #2]
100d073b8:     	lsr	x11, x19, x9
100d073bc:     	and	x11, x11, #0x1
100d073c0:     	lsl	x10, x11, x10
100d073c4:     	orr	x8, x10, x8
100d073c8:     	add	x9, x9, #0x1
100d073cc:     	cmp	x26, x9
100d073d0:     	b.ne	0x100d073b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100d073d4:     	orr	x2, x8, x25
100d073d8:     	mov	x0, x23
100d073dc:     	ldr	x1, [sp, #0x78]
100d073e0:     	bl	0x100dec3b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100d073e4:     	cbz	w0, 0x100d07154 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100d073e8:     	lsr	x0, x19, #6
100d073ec:     	cmp	x0, x22
100d073f0:     	b.hs	0x100d077b0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe48>
100d073f4:     	lsl	x8, x28, x19
100d073f8:     	ldr	x9, [x21, x0, lsl #3]
100d073fc:     	orr	x8, x9, x8
100d07400:     	str	x8, [x21, x0, lsl #3]
100d07404:     	b	0x100d07154 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100d07408:     	ldr	x8, [sp, #0x58]
100d0740c:     	stp	x22, x21, [x8]
100d07410:     	stp	x22, xzr, [x8, #0x10]
100d07414:     	cbz	x26, 0x100d07680 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100d07418:     	mov	x0, x24
100d0741c:     	b	0x100d0767c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100d07420:     	mov	x8, #0x0                ; =0
100d07424:     	ldur	x21, [x29, #-0xb8]
100d07428:     	mov	w0, #0x8                ; =8
100d0742c:     	ldr	x10, [sp, #0x98]
100d07430:     	ldr	x9, [x10, #0x48]
100d07434:     	add	x9, x9, x20
100d07438:     	str	x9, [x10, #0x48]
100d0743c:     	stp	x8, x0, [x28]
100d07440:     	stp	x20, xzr, [x28, #0x10]
100d07444:     	cmp	x21, #0x1
100d07448:     	b.lt	0x100d07454 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaec>
100d0744c:     	ldur	x0, [x29, #-0xb0]
100d07450:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07454:     	ldur	x8, [x29, #-0xe0]
100d07458:     	cmp	x8, #0x1
100d0745c:     	b.lt	0x100d07680 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100d07460:     	ldur	x0, [x29, #-0xd8]
100d07464:     	b	0x100d0767c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100d07468:     	ldr	x1, [sp, #0xa0]
100d0746c:     	cbz	x1, 0x100d074b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb4c>
100d07470:     	lsl	x27, x1, #3
100d07474:     	mov	x0, x27
100d07478:     	mov	x19, x1
100d0747c:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
100d07480:     	cbz	x0, 0x100d07850 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xee8>
100d07484:     	mov	x26, x0
100d07488:     	mov	x1, x23
100d0748c:     	mov	x2, x27
100d07490:     	bl	0x10150f2dc <dyld_stub_binder+0x10150f2dc>
100d07494:     	cmn	x19, #0x1
100d07498:     	b.eq	0x100d07758 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdf0>
100d0749c:     	mov	x1, x19
100d074a0:     	mov	x23, x26
100d074a4:     	b	0x100d074bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100d074a8:     	mov	w24, #0x4               ; =4
100d074ac:     	cbnz	x20, 0x100d06e38 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d0>
100d074b0:     	b	0x100d06e40 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100d074b4:     	mov	x19, #0x0               ; =0
100d074b8:     	mov	w23, #0x8               ; =8
100d074bc:     	mov	x0, x23
100d074c0:     	mov	x2, x24
100d074c4:     	mov	x3, x20
100d074c8:     	bl	0x100f90180 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>
100d074cc:     	str	x19, [sp, #0x80]
100d074d0:     	ldr	x9, [sp, #0x98]
100d074d4:     	ldr	x8, [x9, #0x28]
100d074d8:     	add	x8, x8, #0x1
100d074dc:     	str	x8, [x9, #0x28]
100d074e0:     	mov	w8, #0x4                ; =4
100d074e4:     	stp	xzr, x8, [x29, #-0xb8]
100d074e8:     	stur	xzr, [x29, #-0xa8]
100d074ec:     	ldr	x8, [sp, #0x60]
100d074f0:     	bics	x22, x8, x21
100d074f4:     	b.eq	0x100d07600 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc98>
100d074f8:     	mov	x24, x23
100d074fc:     	mov	x19, #0x0               ; =0
100d07500:     	mov	w8, #0x4                ; =4
100d07504:     	mov	w9, #0x1                ; =1
100d07508:     	b	0x100d07530 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbc8>
100d0750c:     	rbit	x9, x22
100d07510:     	clz	x9, x9
100d07514:     	str	w9, [x8, x19]
100d07518:     	stur	x23, [x29, #-0xa8]
100d0751c:     	sub	x10, x22, #0x1
100d07520:     	add	x19, x19, #0x4
100d07524:     	add	x9, x23, #0x1
100d07528:     	ands	x22, x10, x22
100d0752c:     	b.eq	0x100d07554 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbec>
100d07530:     	mov	x23, x9
100d07534:     	sub	x9, x9, #0x1
100d07538:     	ldur	x10, [x29, #-0xb8]
100d0753c:     	cmp	x9, x10
100d07540:     	b.ne	0x100d0750c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100d07544:     	sub	x0, x29, #0xb8
100d07548:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d0754c:     	ldur	x8, [x29, #-0xb0]
100d07550:     	b	0x100d0750c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100d07554:     	ldp	x8, x28, [x29, #-0xb8]
100d07558:     	str	x8, [sp, #0x20]
100d0755c:     	str	x28, [sp, #0x10]
100d07560:     	cbz	x23, 0x100d07608 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca0>
100d07564:     	mov	w27, #0x1               ; =1
100d07568:     	mov	x1, x24
100d0756c:     	b	0x100d07598 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc30>
100d07570:     	orr	x21, x22, x21
100d07574:     	stur	x21, [x29, #-0xe0]
100d07578:     	ldr	x9, [sp, #0x98]
100d0757c:     	ldr	x8, [x9, #0x30]
100d07580:     	add	x8, x8, #0x1
100d07584:     	str	x8, [x9, #0x30]
100d07588:     	str	x26, [sp, #0x80]
100d0758c:     	mov	x1, x24
100d07590:     	subs	x19, x19, #0x4
100d07594:     	b.eq	0x100d0760c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca4>
100d07598:     	ldr	w8, [x28], #0x4
100d0759c:     	lsl	x22, x27, x8
100d075a0:     	sub	x8, x22, #0x1
100d075a4:     	and	x8, x8, x21
100d075a8:     	fmov	d0, x8
100d075ac:     	cnt.8b	v0, v0
100d075b0:     	addv.8b	b0, v0
100d075b4:     	fmov	w4, s0
100d075b8:     	fmov	d0, x21
100d075bc:     	cnt.8b	v0, v0
100d075c0:     	addv.8b	b0, v0
100d075c4:     	fmov	w3, s0
100d075c8:     	sub	x0, x29, #0xb8
100d075cc:     	mov	x23, x1
100d075d0:     	ldr	x2, [sp, #0xa0]
100d075d4:     	bl	0x100f91194 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100d075d8:     	ldp	x26, x24, [x29, #-0xb8]
100d075dc:     	ldur	x8, [x29, #-0xa8]
100d075e0:     	str	x8, [sp, #0xa0]
100d075e4:     	ldr	x8, [sp, #0x80]
100d075e8:     	sub	x8, x8, #0x1
100d075ec:     	cmn	x8, #0x3
100d075f0:     	b.hi	0x100d07570 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100d075f4:     	mov	x0, x23
100d075f8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d075fc:     	b	0x100d07570 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100d07600:     	ldr	x19, [sp, #0x80]
100d07604:     	b	0x100d07628 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcc0>
100d07608:     	ldr	x26, [sp, #0x80]
100d0760c:     	ldr	x8, [sp, #0x20]
100d07610:     	cbz	x8, 0x100d0761c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcb4>
100d07614:     	ldr	x0, [sp, #0x10]
100d07618:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d0761c:     	mov	x19, x26
100d07620:     	mov	x23, x24
100d07624:     	ldr	x28, [sp, #0x58]
100d07628:     	ldr	x24, [sp, #0x30]
100d0762c:     	ldr	x8, [sp, #0x60]
100d07630:     	cmp	x21, x8
100d07634:     	b.ne	0x100d076f4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd8c>
100d07638:     	cmn	x19, #0x1
100d0763c:     	b.ne	0x100d07650 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xce8>
100d07640:     	ldr	x9, [sp, #0x98]
100d07644:     	ldr	x8, [x9, #0x18]
100d07648:     	add	x8, x8, #0x1
100d0764c:     	str	x8, [x9, #0x18]
100d07650:     	ldr	x8, [sp, #0x78]
100d07654:     	sbfx	x8, x8, #0, #1
100d07658:     	stp	x19, x23, [x28]
100d0765c:     	ldr	x9, [sp, #0xa0]
100d07660:     	stp	x9, x8, [x28, #0x10]
100d07664:     	cbz	x20, 0x100d07670 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd08>
100d07668:     	mov	x0, x24
100d0766c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07670:     	ldr	w8, [sp, #0x40]
100d07674:     	tbnz	w8, #0x0, 0x100d07680 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100d07678:     	mov	x0, x25
100d0767c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07680:     	ldp	x29, x30, [sp, #0x1d0]
100d07684:     	ldp	x20, x19, [sp, #0x1c0]
100d07688:     	ldp	x22, x21, [sp, #0x1b0]
100d0768c:     	ldp	x24, x23, [sp, #0x1a0]
100d07690:     	ldp	x26, x25, [sp, #0x190]
100d07694:     	ldp	x28, x27, [sp, #0x180]
100d07698:     	ldp	d9, d8, [sp, #0x170]
100d0769c:     	ldp	d11, d10, [sp, #0x160]
100d076a0:     	ldp	d13, d12, [sp, #0x150]
100d076a4:     	ldp	d15, d14, [sp, #0x140]
100d076a8:     	add	sp, sp, #0x1e0
100d076ac:     	ret
100d076b0:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100d076b4:     	add	x2, x2, #0x268
100d076b8:     	adrp	x5, 0x101758000 <dyld_stub_binder+0x101758000>
100d076bc:     	add	x5, x5, #0x340
100d076c0:     	sub	x1, x29, #0xb8
100d076c4:     	mov	w0, #0x0                ; =0
100d076c8:     	mov	x3, #0x0                ; =0
100d076cc:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d076d0:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100d076d4:     	add	x2, x2, #0x268
100d076d8:     	adrp	x5, 0x101758000 <dyld_stub_binder+0x101758000>
100d076dc:     	add	x5, x5, #0x2e0
100d076e0:     	sub	x1, x29, #0xb8
100d076e4:     	mov	w0, #0x0                ; =0
100d076e8:     	mov	x3, #0x0                ; =0
100d076ec:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d076f0:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d076f4:     	adrp	x5, 0x101758000 <dyld_stub_binder+0x101758000>
100d076f8:     	add	x5, x5, #0x2c8
100d076fc:     	sub	x1, x29, #0xe0
100d07700:     	add	x2, sp, #0xb8
100d07704:     	mov	w0, #0x0                ; =0
100d07708:     	mov	x3, #0x0                ; =0
100d0770c:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d07710:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07714:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100d07718:     	add	x2, x2, #0x268
100d0771c:     	adrp	x5, 0x101758000 <dyld_stub_binder+0x101758000>
100d07720:     	add	x5, x5, #0x328
100d07724:     	sub	x1, x29, #0xb8
100d07728:     	mov	w0, #0x0                ; =0
100d0772c:     	mov	x3, #0x0                ; =0
100d07730:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d07734:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100d07738:     	add	x2, x2, #0x268
100d0773c:     	adrp	x5, 0x101758000 <dyld_stub_binder+0x101758000>
100d07740:     	add	x5, x5, #0x310
100d07744:     	sub	x1, x29, #0xc0
100d07748:     	mov	w0, #0x1                ; =1
100d0774c:     	mov	x3, #0x0                ; =0
100d07750:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d07754:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07758:     	adrp	x0, 0x10166e000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xa98>
100d0775c:     	add	x0, x0, #0xd29
100d07760:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
100d07764:     	add	x2, x2, #0x498
100d07768:     	mov	x23, x26
100d0776c:     	mov	w1, #0x28               ; =40
100d07770:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d07774:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07778:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
100d0777c:     	add	x2, x2, #0x978
100d07780:     	mov	x1, x26
100d07784:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d07788:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
100d0778c:     	add	x2, x2, #0x978
100d07790:     	mov	x1, x26
100d07794:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d07798:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d0779c:     	adrp	x2, 0x10175a000 <dyld_stub_binder+0x10175a000>
100d077a0:     	add	x2, x2, #0x470
100d077a4:     	mov	x1, x26
100d077a8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d077ac:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d077b0:     	adrp	x2, 0x101756000 <dyld_stub_binder+0x101756000>
100d077b4:     	add	x2, x2, #0x5c0
100d077b8:     	mov	x1, x22
100d077bc:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d077c0:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d077c4:     	adrp	x2, 0x10175a000 <dyld_stub_binder+0x10175a000>
100d077c8:     	add	x2, x2, #0xf8
100d077cc:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d077d0:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d077d4:     	mov	x19, x0
100d077d8:     	mov	x1, x16
100d077dc:     	b	0x100d077e4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe7c>
100d077e0:     	mov	x19, x0
100d077e4:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
100d077e8:     	add	x2, x2, #0xc18
100d077ec:     	mov	x0, x8
100d077f0:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d077f4:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d077f8:     	mov	w0, #0x8                ; =8
100d077fc:     	mov	x1, x19
100d07800:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d07804:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07808:     	adrp	x2, 0x101758000 <dyld_stub_binder+0x101758000>
100d0780c:     	add	x2, x2, #0x2f8
100d07810:     	mov	x0, x19
100d07814:     	mov	x1, x26
100d07818:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d0781c:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07820:     	mov	w0, #0x4                ; =4
100d07824:     	mov	x1, x23
100d07828:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d0782c:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07830:     	mov	w0, #0x8                ; =8
100d07834:     	mov	x1, x19
100d07838:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d0783c:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07840:     	mov	w0, #0x4                ; =4
100d07844:     	mov	x1, x22
100d07848:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d0784c:     	b	0x100d07860 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100d07850:     	mov	x19, #-0x1              ; =-1
100d07854:     	mov	w0, #0x8                ; =8
100d07858:     	mov	x1, x27
100d0785c:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d07860:     	brk	#0x1
100d07864:     	mov	x28, x0
100d07868:     	b	0x100d07894 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf2c>
100d0786c:     	mov	x28, x0
100d07870:     	b	0x100d079cc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1064>
100d07874:     	mov	x28, x0
100d07878:     	b	0x100d079ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100d0787c:     	mov	x28, x0
100d07880:     	b	0x100d07988 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1020>
100d07884:     	b	0x100d07924 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfbc>
100d07888:     	mov	x28, x0
100d0788c:     	mov	x0, x24
100d07890:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07894:     	cbz	x20, 0x100d07a1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100d07898:     	mov	x0, x19
100d0789c:     	b	0x100d07a18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100d078a0:     	b	0x100d0797c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1014>
100d078a4:     	mov	x28, x0
100d078a8:     	ldur	x8, [x29, #-0xb8]
100d078ac:     	cbnz	x8, 0x100d078b8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf50>
100d078b0:     	mov	x23, x24
100d078b4:     	b	0x100d0794c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100d078b8:     	ldur	x0, [x29, #-0xb0]
100d078bc:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d078c0:     	mov	x23, x24
100d078c4:     	b	0x100d0794c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100d078c8:     	mov	x28, x0
100d078cc:     	mov	x0, x19
100d078d0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d078d4:     	b	0x100d0799c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1034>
100d078d8:     	mov	x28, x0
100d078dc:     	ldur	x8, [x29, #-0xb8]
100d078e0:     	cbnz	x8, 0x100d078f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf88>
100d078e4:     	ldr	x23, [sp, #0x30]
100d078e8:     	ldr	x19, [sp, #0x80]
100d078ec:     	b	0x100d079f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100d078f0:     	ldur	x24, [x29, #-0xb0]
100d078f4:     	ldr	x23, [sp, #0x30]
100d078f8:     	ldr	x19, [sp, #0x80]
100d078fc:     	b	0x100d079f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100d07900:     	mov	x28, x0
100d07904:     	ldur	x8, [x29, #-0xb8]
100d07908:     	cbnz	x8, 0x100d07914 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfac>
100d0790c:     	ldr	x19, [sp, #0x80]
100d07910:     	b	0x100d07a08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100d07914:     	ldur	x0, [x29, #-0xb0]
100d07918:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d0791c:     	ldr	x19, [sp, #0x80]
100d07920:     	b	0x100d07a08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100d07924:     	mov	x28, x0
100d07928:     	ldur	x8, [x29, #-0xb8]
100d0792c:     	cbz	x8, 0x100d07a1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100d07930:     	ldur	x0, [x29, #-0xb0]
100d07934:     	b	0x100d07a18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100d07938:     	mov	x28, x0
100d0793c:     	ldr	x8, [sp, #0x20]
100d07940:     	cbz	x8, 0x100d0794c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100d07944:     	ldr	x0, [sp, #0x10]
100d07948:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d0794c:     	ldr	x19, [sp, #0x80]
100d07950:     	ldr	x24, [sp, #0x30]
100d07954:     	cbnz	x20, 0x100d079f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100d07958:     	b	0x100d079f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100d0795c:     	mov	x28, x0
100d07960:     	ldr	x8, [sp, #0x40]
100d07964:     	cbz	x8, 0x100d07970 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1008>
100d07968:     	ldr	x0, [sp, #0x30]
100d0796c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07970:     	mov	x23, x19
100d07974:     	mov	x19, x24
100d07978:     	b	0x100d07a08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100d0797c:     	mov	x28, x0
100d07980:     	mov	x0, x21
100d07984:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07988:     	cbz	x26, 0x100d07a1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100d0798c:     	mov	x0, x24
100d07990:     	b	0x100d07a18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100d07994:     	mov	x28, x0
100d07998:     	ldur	x21, [x29, #-0xb8]
100d0799c:     	cmp	x21, #0x1
100d079a0:     	b.lt	0x100d079ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100d079a4:     	ldur	x0, [x29, #-0xb0]
100d079a8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d079ac:     	ldur	x8, [x29, #-0xe0]
100d079b0:     	cmp	x8, #0x1
100d079b4:     	b.lt	0x100d07a1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100d079b8:     	ldur	x0, [x29, #-0xd8]
100d079bc:     	b	0x100d07a18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100d079c0:     	mov	x28, x0
100d079c4:     	mov	x0, x27
100d079c8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d079cc:     	ldr	x23, [sp, #0x30]
100d079d0:     	ldr	x19, [sp, #0x80]
100d079d4:     	ldr	x8, [sp, #0x20]
100d079d8:     	cbnz	x8, 0x100d079f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100d079dc:     	b	0x100d079f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100d079e0:     	mov	x28, x0
100d079e4:     	b	0x100d07a08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100d079e8:     	mov	x28, x0
100d079ec:     	cbz	x20, 0x100d079f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100d079f0:     	mov	x0, x24
100d079f4:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d079f8:     	ldr	w8, [sp, #0x40]
100d079fc:     	tbnz	w8, #0x0, 0x100d07a08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100d07a00:     	mov	x0, x25
100d07a04:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07a08:     	sub	x8, x19, #0x1
100d07a0c:     	cmn	x8, #0x3
100d07a10:     	b.hi	0x100d07a1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100d07a14:     	mov	x0, x23
100d07a18:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100d07a1c:     	mov	x0, x28
100d07a20:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
