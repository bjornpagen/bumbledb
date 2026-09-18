
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d7eae4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>:
100d7eae4:     	stp	x28, x27, [sp, #-0x60]!
100d7eae8:     	stp	x26, x25, [sp, #0x10]
100d7eaec:     	stp	x24, x23, [sp, #0x20]
100d7eaf0:     	stp	x22, x21, [sp, #0x30]
100d7eaf4:     	stp	x20, x19, [sp, #0x40]
100d7eaf8:     	stp	x29, x30, [sp, #0x50]
100d7eafc:     	add	x29, sp, #0x50
100d7eb00:     	sub	sp, sp, #0x1e0
100d7eb04:     	ldr	w8, [x1, #0xe0]
100d7eb08:     	str	x4, [sp, #0xe0]
100d7eb0c:     	str	x8, [sp]
100d7eb10:     	cmp	x4, x8
100d7eb14:     	b.ne	0x100d7eebc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d8>
100d7eb18:     	mov	x25, x6
100d7eb1c:     	mov	x21, x5
100d7eb20:     	mov	x23, x4
100d7eb24:     	mov	x22, x2
100d7eb28:     	mov	x20, x1
100d7eb2c:     	mov	x19, x0
100d7eb30:     	ldp	x24, x10, [x29, #0x18]
100d7eb34:     	cbz	x4, 0x100d7ebf8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x114>
100d7eb38:     	mov	x11, #0x0               ; =0
100d7eb3c:     	lsl	x9, x23, #2
100d7eb40:     	mov	w12, #0x1               ; =1
100d7eb44:     	mov	x13, x9
100d7eb48:     	mov	x14, x3
100d7eb4c:     	ldr	w15, [x14], #0x4
100d7eb50:     	cmp	w15, w8
100d7eb54:     	b.hs	0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7eb58:     	lsr	x16, x11, x15
100d7eb5c:     	tbnz	w16, #0x0, 0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7eb60:     	lsl	x15, x12, x15
100d7eb64:     	orr	x11, x15, x11
100d7eb68:     	subs	x13, x13, #0x4
100d7eb6c:     	b.ne	0x100d7eb4c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x68>
100d7eb70:     	str	x7, [sp, #0xe0]
100d7eb74:     	str	x23, [sp]
100d7eb78:     	cmp	x7, x23
100d7eb7c:     	b.ne	0x100d7eebc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d8>
100d7eb80:     	mov	x11, #0x0               ; =0
100d7eb84:     	mov	w12, #0x1               ; =1
100d7eb88:     	mov	x13, x9
100d7eb8c:     	mov	x14, x25
100d7eb90:     	ldr	w15, [x14], #0x4
100d7eb94:     	cmp	w15, w8
100d7eb98:     	b.hs	0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7eb9c:     	lsr	x16, x11, x15
100d7eba0:     	tbnz	w16, #0x0, 0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7eba4:     	lsl	x15, x12, x15
100d7eba8:     	orr	x11, x15, x11
100d7ebac:     	subs	x13, x13, #0x4
100d7ebb0:     	b.ne	0x100d7eb90 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xac>
100d7ebb4:     	str	x10, [sp, #0xe0]
100d7ebb8:     	str	x23, [sp]
100d7ebbc:     	cmp	x10, x23
100d7ebc0:     	b.ne	0x100d7eebc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d8>
100d7ebc4:     	mov	x10, #0x0               ; =0
100d7ebc8:     	mov	w11, #0x1               ; =1
100d7ebcc:     	mov	x12, x24
100d7ebd0:     	ldr	w13, [x12], #0x4
100d7ebd4:     	cmp	w13, w8
100d7ebd8:     	b.hs	0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7ebdc:     	lsr	x14, x10, x13
100d7ebe0:     	tbnz	w14, #0x0, 0x100d7eea4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3c0>
100d7ebe4:     	lsl	x13, x11, x13
100d7ebe8:     	orr	x10, x13, x10
100d7ebec:     	subs	x9, x9, #0x4
100d7ebf0:     	b.ne	0x100d7ebd0 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xec>
100d7ebf4:     	b	0x100d7ec10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x12c>
100d7ebf8:     	str	x7, [sp, #0xe0]
100d7ebfc:     	str	xzr, [sp]
100d7ec00:     	cbnz	x7, 0x100d7eebc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d8>
100d7ec04:     	str	x10, [sp, #0xe0]
100d7ec08:     	str	xzr, [sp]
100d7ec0c:     	cbnz	x10, 0x100d7eebc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d8>
100d7ec10:     	ldr	x26, [x29, #0x10]
100d7ec14:     	lsr	x8, x26, x8
100d7ec18:     	str	x8, [sp]
100d7ec1c:     	cbnz	x8, 0x100d7eee0 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3fc>
100d7ec20:     	sub	x0, x29, #0xf0
100d7ec24:     	mov	x1, x3
100d7ec28:     	mov	x2, x23
100d7ec2c:     	mov	x3, x24
100d7ec30:     	mov	x4, x23
100d7ec34:     	bl	0x100d33418 <__RNvMs0_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d7ec38:     	mov	x0, sp
100d7ec3c:     	mov	x1, x25
100d7ec40:     	mov	x2, x23
100d7ec44:     	mov	x3, x24
100d7ec48:     	mov	x4, x23
100d7ec4c:     	bl	0x100d33418 <__RNvMs0_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d7ec50:     	ldp	q0, q1, [x29, #-0xf0]
100d7ec54:     	stp	q0, q1, [sp, #0xe0]
100d7ec58:     	ldur	q0, [x29, #-0xd0]
100d7ec5c:     	ldp	q1, q2, [sp]
100d7ec60:     	stp	q0, q1, [sp, #0x100]
100d7ec64:     	ldr	q0, [sp, #0x20]
100d7ec68:     	stp	q2, q0, [sp, #0x120]
100d7ec6c:     	mov	w8, #0x4                ; =4
100d7ec70:     	stp	xzr, x8, [sp]
100d7ec74:     	str	xzr, [sp, #0x10]
100d7ec78:     	cbz	x26, 0x100d7ed00 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x21c>
100d7ec7c:     	mov	x27, #0x0               ; =0
100d7ec80:     	mov	w8, #0x4                ; =4
100d7ec84:     	b	0x100d7ecac <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1c8>
100d7ec88:     	ldr	x8, [sp, #0x8]
100d7ec8c:     	rbit	x9, x26
100d7ec90:     	clz	x9, x9
100d7ec94:     	str	w9, [x8, x27, lsl #2]
100d7ec98:     	add	x27, x27, #0x1
100d7ec9c:     	str	x27, [sp, #0x10]
100d7eca0:     	sub	x9, x26, #0x1
100d7eca4:     	ands	x26, x9, x26
100d7eca8:     	b.eq	0x100d7ecc4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1e0>
100d7ecac:     	ldr	x9, [sp]
100d7ecb0:     	cmp	x27, x9
100d7ecb4:     	b.ne	0x100d7ec8c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a8>
100d7ecb8:     	mov	x0, sp
100d7ecbc:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d7ecc0:     	b	0x100d7ec88 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a4>
100d7ecc4:     	ldp	x26, x25, [sp]
100d7ecc8:     	cbz	x27, 0x100d7ed0c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x228>
100d7eccc:     	mov	x9, #0x0                ; =0
100d7ecd0:     	mov	x8, #0x0                ; =0
100d7ecd4:     	mov	w10, #0x1               ; =1
100d7ecd8:     	ldr	w0, [x25, x9, lsl #2]
100d7ecdc:     	cmp	x23, x0
100d7ece0:     	b.ls	0x100d7ef08 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x424>
100d7ece4:     	ldr	w11, [x24, x0, lsl #2]
100d7ece8:     	lsl	x11, x10, x11
100d7ecec:     	orr	x8, x11, x8
100d7ecf0:     	add	x9, x9, #0x1
100d7ecf4:     	cmp	x27, x9
100d7ecf8:     	b.ne	0x100d7ecd8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1f4>
100d7ecfc:     	b	0x100d7ed10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100d7ed00:     	mov	x8, #0x0                ; =0
100d7ed04:     	mov	w25, #0x4               ; =4
100d7ed08:     	b	0x100d7ed10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100d7ed0c:     	mov	x8, #0x0                ; =0
100d7ed10:     	mov	x23, sp
100d7ed14:     	movi.2d	v0, #0000000000000000
100d7ed18:     	stur	q0, [x23, #0xc8]
100d7ed1c:     	stur	q0, [x23, #0xb8]
100d7ed20:     	stur	q0, [x23, #0xa8]
100d7ed24:     	stur	q0, [x23, #0x98]
100d7ed28:     	stur	q0, [x23, #0x88]
100d7ed2c:     	ldrb	w9, [x20, #0xe6]
100d7ed30:     	ldp	q0, q1, [sp, #0x120]
100d7ed34:     	stp	q0, q1, [sp, #0x40]
100d7ed38:     	ldp	q0, q1, [sp, #0xe0]
100d7ed3c:     	stp	q0, q1, [sp]
100d7ed40:     	ldp	q0, q1, [sp, #0x100]
100d7ed44:     	stp	xzr, x8, [sp, #0x78]
100d7ed48:     	adrp	x8, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100d7ed4c:     	add	x8, x8, #0xb30
100d7ed50:     	stp	q0, q1, [sp, #0x20]
100d7ed54:     	stp	x8, xzr, [sp, #0x60]
100d7ed58:     	str	xzr, [sp, #0x70]
100d7ed5c:     	strb	w9, [sp, #0xd8]
100d7ed60:     	cbz	x26, 0x100d7ed6c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x288>
100d7ed64:     	mov	x0, x25
100d7ed68:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ed6c:     	stur	w22, [x29, #-0xe0]
100d7ed70:     	stp	xzr, xzr, [x29, #-0xf0]
100d7ed74:     	sub	x0, x29, #0x88
100d7ed78:     	sub	x1, x29, #0xf0
100d7ed7c:     	mov	x2, x20
100d7ed80:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100d7ed84:     	sub	x22, x29, #0x88
100d7ed88:     	ldur	x8, [x29, #-0x78]
100d7ed8c:     	ldr	q0, [x22]
100d7ed90:     	stur	q0, [x29, #-0x70]
100d7ed94:     	stur	x8, [x29, #-0x60]
100d7ed98:     	str	x8, [sp, #0xf0]
100d7ed9c:     	str	q0, [sp, #0xe0]
100d7eda0:     	stur	w21, [x29, #-0xe0]
100d7eda4:     	stp	xzr, xzr, [x29, #-0xf0]
100d7eda8:     	sub	x0, x29, #0x88
100d7edac:     	sub	x1, x29, #0xf0
100d7edb0:     	mov	x2, x20
100d7edb4:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100d7edb8:     	ldur	x8, [x29, #-0x78]
100d7edbc:     	ldr	q0, [x22]
100d7edc0:     	stur	q0, [x23, #0xf8]
100d7edc4:     	str	x8, [sp, #0x108]
100d7edc8:     	ldp	q0, q1, [sp, #0xe0]
100d7edcc:     	ldr	q2, [sp, #0x100]
100d7edd0:     	stp	q1, q2, [x29, #-0xe0]
100d7edd4:     	stur	q0, [x29, #-0xf0]
100d7edd8:     	mov	x0, sp
100d7eddc:     	sub	x2, x29, #0xf0
100d7ede0:     	mov	x1, x20
100d7ede4:     	bl	0x10075f7a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
100d7ede8:     	mov	x8, sp
100d7edec:     	ldur	q0, [x8, #0xc8]
100d7edf0:     	stur	q0, [x19, #0x48]
100d7edf4:     	ldur	q0, [x8, #0x88]
100d7edf8:     	stur	q0, [x19, #0x8]
100d7edfc:     	ldur	q0, [x8, #0x98]
100d7ee00:     	stur	q0, [x19, #0x18]
100d7ee04:     	ldur	q0, [x8, #0xa8]
100d7ee08:     	stur	q0, [x19, #0x28]
100d7ee0c:     	ldur	q0, [x8, #0xb8]
100d7ee10:     	stur	q0, [x19, #0x38]
100d7ee14:     	str	w0, [x19]
100d7ee18:     	ldr	x8, [sp]
100d7ee1c:     	cbz	x8, 0x100d7ee28 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x344>
100d7ee20:     	ldr	x0, [sp, #0x8]
100d7ee24:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ee28:     	ldr	x8, [sp, #0x18]
100d7ee2c:     	cbz	x8, 0x100d7ee38 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x354>
100d7ee30:     	ldr	x0, [sp, #0x20]
100d7ee34:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ee38:     	ldr	x8, [sp, #0x30]
100d7ee3c:     	cbz	x8, 0x100d7ee48 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x364>
100d7ee40:     	ldr	x0, [sp, #0x38]
100d7ee44:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ee48:     	ldr	x8, [sp, #0x48]
100d7ee4c:     	cbz	x8, 0x100d7ee58 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x374>
100d7ee50:     	ldr	x0, [sp, #0x50]
100d7ee54:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ee58:     	ldr	x9, [sp, #0x68]
100d7ee5c:     	cbz	x9, 0x100d7ee84 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3a0>
100d7ee60:     	lsl	x8, x9, #6
100d7ee64:     	sub	x8, x8, x9, lsl #3
100d7ee68:     	add	x9, x8, x9
100d7ee6c:     	cmn	x9, #0x41
100d7ee70:     	b.eq	0x100d7ee84 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3a0>
100d7ee74:     	ldr	x9, [sp, #0x60]
100d7ee78:     	sub	x8, x9, x8
100d7ee7c:     	sub	x0, x8, #0x38
100d7ee80:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ee84:     	add	sp, sp, #0x1e0
100d7ee88:     	ldp	x29, x30, [sp, #0x50]
100d7ee8c:     	ldp	x20, x19, [sp, #0x40]
100d7ee90:     	ldp	x22, x21, [sp, #0x30]
100d7ee94:     	ldp	x24, x23, [sp, #0x20]
100d7ee98:     	ldp	x26, x25, [sp, #0x10]
100d7ee9c:     	ldp	x28, x27, [sp], #0x60
100d7eea0:     	ret
100d7eea4:     	adrp	x0, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7eea8:     	add	x0, x0, #0x563
100d7eeac:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100d7eeb0:     	add	x2, x2, #0xd60
100d7eeb4:     	mov	w1, #0x2f               ; =47
100d7eeb8:     	bl	0x1013b4bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d7eebc:     	adrp	x3, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7eec0:     	add	x3, x3, #0x54d
100d7eec4:     	adrp	x5, 0x10163f000 <dyld_stub_binder+0x10163f000>
100d7eec8:     	add	x5, x5, #0xd48
100d7eecc:     	add	x1, sp, #0xe0
100d7eed0:     	mov	x2, sp
100d7eed4:     	mov	w0, #0x0                ; =0
100d7eed8:     	mov	w4, #0x2d               ; =45
100d7eedc:     	bl	0x1013b4c30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d7eee0:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100d7eee4:     	add	x2, x2, #0x3e0
100d7eee8:     	adrp	x3, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7eeec:     	add	x3, x3, #0xada
100d7eef0:     	adrp	x5, 0x101640000 <dyld_stub_binder+0x101640000>
100d7eef4:     	add	x5, x5, #0x678
100d7eef8:     	mov	x1, sp
100d7eefc:     	mov	w0, #0x0                ; =0
100d7ef00:     	mov	w4, #0x57               ; =87
100d7ef04:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d7ef08:     	adrp	x2, 0x101603000 <dyld_stub_binder+0x101603000>
100d7ef0c:     	add	x2, x2, #0xa8
100d7ef10:     	mov	x1, x23
100d7ef14:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d7ef18:     	brk	#0x1
100d7ef1c:     	mov	x19, x0
100d7ef20:     	sub	x0, x29, #0xf0
100d7ef24:     	bl	0x100824a0c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d7ef28:     	mov	x0, x19
100d7ef2c:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ef30:     	mov	x19, x0
100d7ef34:     	mov	x0, sp
100d7ef38:     	bl	0x100825f24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d7ef3c:     	mov	x0, x19
100d7ef40:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ef44:     	mov	x19, x0
100d7ef48:     	ldr	x8, [sp]
100d7ef4c:     	cbz	x8, 0x100d7ef58 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x474>
100d7ef50:     	ldr	x0, [sp, #0x8]
100d7ef54:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ef58:     	add	x0, sp, #0xe0
100d7ef5c:     	bl	0x10080a39c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d7ef60:     	mov	x0, x19
100d7ef64:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ef68:     	mov	x19, x0
100d7ef6c:     	add	x0, sp, #0xe0
100d7ef70:     	bl	0x10080a39c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d7ef74:     	cbnz	x26, 0x100d7ef80 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x49c>
100d7ef78:     	mov	x0, x19
100d7ef7c:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ef80:     	mov	x0, x25
100d7ef84:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7ef88:     	mov	x0, x19
100d7ef8c:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
