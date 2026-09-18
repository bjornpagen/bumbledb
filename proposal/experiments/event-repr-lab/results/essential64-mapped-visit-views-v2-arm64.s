
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010075eabc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>:
10075eabc:     	sub	sp, sp, #0x1d0
10075eac0:     	stp	x28, x27, [sp, #0x170]
10075eac4:     	stp	x26, x25, [sp, #0x180]
10075eac8:     	stp	x24, x23, [sp, #0x190]
10075eacc:     	stp	x22, x21, [sp, #0x1a0]
10075ead0:     	stp	x20, x19, [sp, #0x1b0]
10075ead4:     	stp	x29, x30, [sp, #0x1c0]
10075ead8:     	add	x29, sp, #0x1c0
10075eadc:     	ldr	w9, [x2, #0x10]
10075eae0:     	cbz	w9, 0x10075f3f4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x938>
10075eae4:     	mov	x27, x2
10075eae8:     	ldr	w10, [x2, #0x28]
10075eaec:     	cbz	w10, 0x10075f3f4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x938>
10075eaf0:     	mov	x20, x1
10075eaf4:     	mov	x26, x0
10075eaf8:     	cmp	w9, #0x1
10075eafc:     	ccmp	w10, #0x1, #0x0, eq
10075eb00:     	b.eq	0x10075ec24 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x168>
10075eb04:     	ldr	x8, [x26, #0x78]
10075eb08:     	cbz	x8, 0x10075ec2c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
10075eb0c:     	mov	x8, #0x0                ; =0
10075eb10:     	mov	x15, #0xa9c5            ; =43461
10075eb14:     	movk	x15, #0x2e62, lsl #16
10075eb18:     	movk	x15, #0x7aea, lsl #32
10075eb1c:     	movk	x15, #0xf135, lsl #48
10075eb20:     	ldp	x11, x12, [x27]
10075eb24:     	madd	x13, x9, x15, x11
10075eb28:     	mov	x14, #0x6332            ; =25394
10075eb2c:     	movk	x14, #0x6ed3, lsl #16
10075eb30:     	movk	x14, #0x765a, lsl #32
10075eb34:     	movk	x14, #0x284f, lsl #48
10075eb38:     	mul	x14, x14, x15
10075eb3c:     	madd	x13, x13, x15, x14
10075eb40:     	add	x13, x13, x12
10075eb44:     	madd	x16, x13, x15, x10
10075eb48:     	ldp	x13, x14, [x27, #0x18]
10075eb4c:     	madd	x16, x16, x15, x13
10075eb50:     	madd	x16, x16, x15, x14
10075eb54:     	mul	x15, x16, x15
10075eb58:     	ror	x0, x15, #0x2c
10075eb5c:     	lsr	x17, x0, #57
10075eb60:     	ldp	x16, x15, [x26, #0x60]
10075eb64:     	dup.8b	v0, w17
10075eb68:     	movi.2d	v1, #0xffffffffffffffff
10075eb6c:     	mov	w17, #0x38              ; =56
10075eb70:     	and	x0, x0, x15
10075eb74:     	ldr	d2, [x16, x0]
10075eb78:     	cmeq.8b	v3, v2, v0
10075eb7c:     	fmov	x1, d3
10075eb80:     	ands	x1, x1, #0x8080808080808080
10075eb84:     	b.eq	0x10075ebf4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x138>
10075eb88:     	rbit	x2, x1
10075eb8c:     	clz	x2, x2
10075eb90:     	add	x2, x0, x2, lsr #3
10075eb94:     	and	x2, x2, x15
10075eb98:     	mneg	x2, x2, x17
10075eb9c:     	add	x2, x16, x2
10075eba0:     	ldur	x3, [x2, #-0x38]
10075eba4:     	cmp	x11, x3
10075eba8:     	b.ne	0x10075ebe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c>
10075ebac:     	ldur	x3, [x2, #-0x30]
10075ebb0:     	cmp	x12, x3
10075ebb4:     	b.ne	0x10075ebe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c>
10075ebb8:     	ldur	w3, [x2, #-0x28]
10075ebbc:     	cmp	w9, w3
10075ebc0:     	b.ne	0x10075ebe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c>
10075ebc4:     	ldur	x3, [x2, #-0x20]
10075ebc8:     	cmp	x13, x3
10075ebcc:     	b.ne	0x10075ebe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c>
10075ebd0:     	ldur	x3, [x2, #-0x18]
10075ebd4:     	cmp	x14, x3
10075ebd8:     	b.ne	0x10075ebe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c>
10075ebdc:     	ldur	w3, [x2, #-0x10]
10075ebe0:     	cmp	w10, w3
10075ebe4:     	b.eq	0x10075eca4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1e8>
10075ebe8:     	sub	x2, x1, #0x2
10075ebec:     	ands	x1, x2, x1
10075ebf0:     	b.ne	0x10075eb88 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcc>
10075ebf4:     	cmeq.8b	v2, v2, v1
10075ebf8:     	fmov	x1, d2
10075ebfc:     	cbnz	x1, 0x10075ec2c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
10075ec00:     	add	x8, x8, #0x8
10075ec04:     	add	x0, x0, x8
10075ec08:     	and	x0, x0, x15
10075ec0c:     	ldr	d2, [x16, x0]
10075ec10:     	cmeq.8b	v3, v2, v0
10075ec14:     	fmov	x1, d3
10075ec18:     	ands	x1, x1, #0x8080808080808080
10075ec1c:     	b.ne	0x10075eb88 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcc>
10075ec20:     	b	0x10075ebf4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x138>
10075ec24:     	mov	w0, #0x1                ; =1
10075ec28:     	b	0x10075f3f8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x93c>
10075ec2c:     	mov	x24, x26
10075ec30:     	ldr	x8, [x24, #0x88]!
10075ec34:     	add	x8, x8, #0x1
10075ec38:     	str	x8, [x24]
10075ec3c:     	mov	w11, #0x8481            ; =33921
10075ec40:     	movk	w11, #0x1e, lsl #16
10075ec44:     	cmp	x8, x11
10075ec48:     	b.hs	0x10075f604 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb48>
10075ec4c:     	ldr	x12, [x27]
10075ec50:     	ldr	x13, [x27, #0x18]
10075ec54:     	ldr	x8, [x20, #0x30]
10075ec58:     	cmn	x8, #0x1
10075ec5c:     	b.eq	0x10075ecb8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1fc>
10075ec60:     	ldr	x1, [x20, #0x40]
10075ec64:     	lsr	x0, x9, #1
10075ec68:     	cmp	x1, x0
10075ec6c:     	b.ls	0x10075f68c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbd0>
10075ec70:     	lsr	x8, x10, #1
10075ec74:     	cmp	x1, x8
10075ec78:     	b.ls	0x10075f688 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbcc>
10075ec7c:     	ldr	x11, [x20, #0x38]
10075ec80:     	lsl	x14, x0, #4
10075ec84:     	ldr	x14, [x11, x14]
10075ec88:     	bic	x19, x14, x12
10075ec8c:     	add	x8, x11, x8, lsl #4
10075ec90:     	ldr	x15, [x8]
10075ec94:     	ldp	x11, x1, [x26, #0x8]
10075ec98:     	mov	x22, #0x0               ; =0
10075ec9c:     	cbnz	x19, 0x10075ecf8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x23c>
10075eca0:     	b	0x10075ed28 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x26c>
10075eca4:     	ldur	w0, [x2, #-0x8]
10075eca8:     	ldr	x8, [x26, #0x90]
10075ecac:     	add	x8, x8, #0x1
10075ecb0:     	str	x8, [x26, #0x90]
10075ecb4:     	b	0x10075f3f8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x93c>
10075ecb8:     	ldr	x1, [x20, #0x48]
10075ecbc:     	lsr	x0, x9, #1
10075ecc0:     	cmp	x1, x0
10075ecc4:     	b.ls	0x10075f6bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
10075ecc8:     	lsr	x8, x10, #1
10075eccc:     	cmp	x1, x8
10075ecd0:     	b.ls	0x10075f6b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbfc>
10075ecd4:     	ldr	x11, [x20, #0x40]
10075ecd8:     	add	x14, x11, x0, lsl #5
10075ecdc:     	ldr	x14, [x14, #0x18]
10075ece0:     	bic	x19, x14, x12
10075ece4:     	add	x8, x11, x8, lsl #5
10075ece8:     	ldr	x15, [x8, #0x18]!
10075ecec:     	ldp	x11, x1, [x26, #0x8]
10075ecf0:     	mov	x22, #0x0               ; =0
10075ecf4:     	cbz	x19, 0x10075ed28 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x26c>
10075ecf8:     	mov	w8, #0x1                ; =1
10075ecfc:     	mov	x14, x19
10075ed00:     	rbit	x16, x14
10075ed04:     	clz	x0, x16
10075ed08:     	cmp	x0, x1
10075ed0c:     	b.hs	0x10075f61c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb60>
10075ed10:     	ldr	w16, [x11, x0, lsl #2]
10075ed14:     	lsl	x16, x8, x16
10075ed18:     	orr	x22, x16, x22
10075ed1c:     	sub	x16, x14, #0x1
10075ed20:     	ands	x14, x16, x14
10075ed24:     	b.ne	0x10075ed00 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x244>
10075ed28:     	ldp	x14, x8, [x26, #0x38]
10075ed2c:     	bic	x15, x15, x13
10075ed30:     	cbz	x15, 0x10075ed68 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2ac>
10075ed34:     	mov	x16, #0x0               ; =0
10075ed38:     	mov	w17, #0x1               ; =1
10075ed3c:     	rbit	x0, x15
10075ed40:     	clz	x0, x0
10075ed44:     	cmp	x0, x8
10075ed48:     	b.hs	0x10075f628 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb6c>
10075ed4c:     	ldr	w0, [x14, x0, lsl #2]
10075ed50:     	lsl	x0, x17, x0
10075ed54:     	orr	x16, x0, x16
10075ed58:     	sub	x0, x15, #0x1
10075ed5c:     	ands	x15, x0, x15
10075ed60:     	b.ne	0x10075ed3c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x280>
10075ed64:     	orr	x22, x16, x22
10075ed68:     	eor	w9, w10, w9
10075ed6c:     	cmp	x12, x13
10075ed70:     	ccmp	w9, #0x1, #0x0, eq
10075ed74:     	b.ne	0x10075ee58 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x39c>
10075ed78:     	ldr	x9, [x27, #0x8]
10075ed7c:     	ldr	x10, [x27, #0x20]
10075ed80:     	cmp	x9, x10
10075ed84:     	b.ne	0x10075ee58 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x39c>
10075ed88:     	mov	x25, x20
10075ed8c:     	mov	w16, #0x4               ; =4
10075ed90:     	stp	xzr, x16, [sp, #0xa0]
10075ed94:     	str	xzr, [sp, #0xb0]
10075ed98:     	mov	x20, #0x0               ; =0
10075ed9c:     	cbz	x19, 0x10075edfc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x340>
10075eda0:     	mov	w8, #0x4                ; =4
10075eda4:     	b	0x10075edc8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x30c>
10075eda8:     	rbit	x9, x19
10075edac:     	clz	x9, x9
10075edb0:     	str	w9, [x8, x20, lsl #2]
10075edb4:     	add	x20, x20, #0x1
10075edb8:     	str	x20, [sp, #0xb0]
10075edbc:     	sub	x9, x19, #0x1
10075edc0:     	ands	x19, x9, x19
10075edc4:     	b.eq	0x10075ede4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x328>
10075edc8:     	ldr	x9, [sp, #0xa0]
10075edcc:     	cmp	x20, x9
10075edd0:     	b.ne	0x10075eda8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2ec>
10075edd4:     	add	x0, sp, #0xa0
10075edd8:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10075eddc:     	ldr	x8, [sp, #0xa8]
10075ede0:     	b	0x10075eda8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2ec>
10075ede4:     	ldp	x9, x16, [sp, #0xa0]
10075ede8:     	ldp	x14, x8, [x26, #0x38]
10075edec:     	ldp	x11, x1, [x26, #0x8]
10075edf0:     	cmp	x9, #0x0
10075edf4:     	cset	w21, eq
10075edf8:     	b	0x10075ee00 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x344>
10075edfc:     	mov	w21, #0x1               ; =1
10075ee00:     	mov	x9, #0x0                ; =0
10075ee04:     	lsl	x10, x20, #2
10075ee08:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10075ee0c:     	add	x2, x2, #0x678
10075ee10:     	adrp	x12, 0x101601000 <dyld_stub_binder+0x101601000>
10075ee14:     	add	x12, x12, #0x690
10075ee18:     	mov	x20, x25
10075ee1c:     	cmp	x10, x9
10075ee20:     	b.eq	0x10075f3f0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x934>
10075ee24:     	ldr	w0, [x16, x9]
10075ee28:     	cmp	x1, x0
10075ee2c:     	b.ls	0x10075f64c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb90>
10075ee30:     	cmp	x8, x0
10075ee34:     	b.ls	0x10075f654 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb98>
10075ee38:     	ldr	w13, [x11, x0, lsl #2]
10075ee3c:     	ldr	w15, [x14, x0, lsl #2]
10075ee40:     	add	x9, x9, #0x4
10075ee44:     	cmp	w13, w15
10075ee48:     	b.eq	0x10075ee1c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x360>
10075ee4c:     	tbnz	w21, #0x0, 0x10075ee58 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x39c>
10075ee50:     	mov	x0, x16
10075ee54:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ee58:     	fmov	d0, x22
10075ee5c:     	cnt.8b	v0, v0
10075ee60:     	addv.8b	b0, v0
10075ee64:     	fmov	x19, d0
10075ee68:     	cmp	x19, #0x7
10075ee6c:     	b.hs	0x10075efe8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x52c>
10075ee70:     	ldr	x8, [x26, #0x98]
10075ee74:     	add	x8, x8, #0x1
10075ee78:     	str	x8, [x26, #0x98]
10075ee7c:     	ldrb	w5, [x26, #0xd8]
10075ee80:     	add	x0, sp, #0x70
10075ee84:     	mov	x1, x20
10075ee88:     	mov	x2, x27
10075ee8c:     	mov	x3, x26
10075ee90:     	mov	x4, x22
10075ee94:     	mov	x6, x24
10075ee98:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
10075ee9c:     	ldrb	w5, [x26, #0xd8]
10075eea0:     	add	x0, sp, #0xa0
10075eea4:     	add	x2, x27, #0x18
10075eea8:     	add	x3, x26, #0x30
10075eeac:     	str	x20, [sp, #0x28]
10075eeb0:     	mov	x1, x20
10075eeb4:     	mov	x4, x22
10075eeb8:     	mov	x6, x24
10075eebc:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
10075eec0:     	mov	w8, #0x1                ; =1
10075eec4:     	lsl	x9, x8, x19
10075eec8:     	lsr	x9, x9, #6
10075eecc:     	cmp	x19, #0x6
10075eed0:     	csinc	x20, x8, x9, eq
10075eed4:     	lsl	x25, x20, #3
10075eed8:     	mov	x0, x25
10075eedc:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
10075eee0:     	cbz	x0, 0x10075f698 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbdc>
10075eee4:     	mov	x23, x0
10075eee8:     	mov	x0, #0x0                ; =0
10075eeec:     	ldp	x19, x25, [sp, #0x70]
10075eef0:     	ldp	x1, x9, [sp, #0x80]
10075eef4:     	sub	x10, x0, w25, uxtb
10075eef8:     	ldp	x21, x8, [sp, #0xa0]
10075eefc:     	ldp	x11, x12, [sp, #0xb0]
10075ef00:     	mov	x28, x20
10075ef04:     	sub	x13, x20, #0x1
10075ef08:     	b	0x10075ef24 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x468>
10075ef0c:     	tst	w8, #0x1
10075ef10:     	csel	x14, x14, xzr, ne
10075ef14:     	str	x14, [x23, x0, lsl #3]
10075ef18:     	cmp	x13, x0
10075ef1c:     	b.eq	0x10075ef78 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4bc>
10075ef20:     	add	x0, x0, #0x1
10075ef24:     	mov	x14, x10
10075ef28:     	cmn	x19, #0x2
10075ef2c:     	b.eq	0x10075ef40 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x484>
10075ef30:     	cmp	x0, x1
10075ef34:     	b.hs	0x10075f63c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb80>
10075ef38:     	ldr	x14, [x25, x0, lsl #3]
10075ef3c:     	eor	x14, x9, x14
10075ef40:     	cmn	x21, #0x2
10075ef44:     	b.eq	0x10075ef0c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x450>
10075ef48:     	cmp	x0, x11
10075ef4c:     	b.hs	0x10075f638 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb7c>
10075ef50:     	ldr	x15, [x8, x0, lsl #3]
10075ef54:     	eor	x15, x12, x15
10075ef58:     	and	x14, x15, x14
10075ef5c:     	str	x14, [x23, x0, lsl #3]
10075ef60:     	cmp	x13, x0
10075ef64:     	b.ne	0x10075ef20 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x464>
10075ef68:     	cmp	x21, #0x1
10075ef6c:     	b.lt	0x10075ef78 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4bc>
10075ef70:     	mov	x0, x8
10075ef74:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ef78:     	cmp	x19, #0x1
10075ef7c:     	b.lt	0x10075ef88 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4cc>
10075ef80:     	mov	x0, x25
10075ef84:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ef88:     	ldr	x8, [x26, #0x80]
10075ef8c:     	mov	w9, #0x4                ; =4
10075ef90:     	stp	xzr, x9, [sp, #0xa0]
10075ef94:     	str	xzr, [sp, #0xb0]
10075ef98:     	ands	x21, x8, x22
10075ef9c:     	b.eq	0x10075f3e8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x92c>
10075efa0:     	mov	x19, #0x0               ; =0
10075efa4:     	mov	w8, #0x4                ; =4
10075efa8:     	b	0x10075efcc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x510>
10075efac:     	rbit	x9, x21
10075efb0:     	clz	x9, x9
10075efb4:     	str	w9, [x8, x19, lsl #2]
10075efb8:     	add	x19, x19, #0x1
10075efbc:     	str	x19, [sp, #0xb0]
10075efc0:     	sub	x9, x21, #0x1
10075efc4:     	ands	x21, x9, x21
10075efc8:     	b.eq	0x10075f180 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6c4>
10075efcc:     	ldr	x9, [sp, #0xa0]
10075efd0:     	cmp	x19, x9
10075efd4:     	b.ne	0x10075efac <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4f0>
10075efd8:     	add	x0, sp, #0xa0
10075efdc:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10075efe0:     	ldr	x8, [sp, #0xa8]
10075efe4:     	b	0x10075efac <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4f0>
10075efe8:     	mov	x0, x20
10075efec:     	mov	x1, x22
10075eff0:     	bl	0x100c9a400 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
10075eff4:     	ldr	q0, [x27]
10075eff8:     	str	q0, [sp, #0x70]
10075effc:     	ldr	x8, [x27, #0x10]
10075f000:     	str	x8, [sp, #0x80]
10075f004:     	mov	w22, w0
10075f008:     	ldr	x1, [x26, #0x28]
10075f00c:     	cmp	x1, x22
10075f010:     	b.ls	0x10075f678 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbbc>
10075f014:     	ldr	x8, [x26, #0x20]
10075f018:     	ldr	w8, [x8, x22, lsl #2]
10075f01c:     	ldr	w9, [sp, #0x80]
10075f020:     	ldr	x10, [x20, #0x30]
10075f024:     	lsr	x0, x9, #1
10075f028:     	cmn	x10, #0x1
10075f02c:     	b.eq	0x10075f388 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x8cc>
10075f030:     	ldr	x1, [x20, #0x40]
10075f034:     	cmp	x1, x0
10075f038:     	b.ls	0x10075f68c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbd0>
10075f03c:     	ldr	x9, [x20, #0x38]
10075f040:     	add	x9, x9, x0, lsl #4
10075f044:     	ldr	x9, [x9]
10075f048:     	mov	w10, #0x1               ; =1
10075f04c:     	lsl	x8, x10, x8
10075f050:     	tst	x9, x8
10075f054:     	b.eq	0x10075f068 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5ac>
10075f058:     	ldp	x9, x10, [sp, #0x70]
10075f05c:     	orr	x9, x9, x8
10075f060:     	bic	x8, x10, x8
10075f064:     	stp	x9, x8, [sp, #0x70]
10075f068:     	sub	x0, x29, #0x70
10075f06c:     	add	x1, sp, #0x70
10075f070:     	mov	x2, x20
10075f074:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075f078:     	ldur	q0, [x29, #-0x70]
10075f07c:     	stur	q0, [x29, #-0x90]
10075f080:     	ldur	x8, [x29, #-0x60]
10075f084:     	stur	q0, [x29, #-0xb0]
10075f088:     	str	q0, [sp, #0x40]
10075f08c:     	str	x8, [sp, #0x50]
10075f090:     	ldr	q0, [sp, #0x40]
10075f094:     	str	x8, [sp, #0xb0]
10075f098:     	str	q0, [sp, #0xa0]
10075f09c:     	ldur	q0, [x27, #0x18]
10075f0a0:     	str	q0, [sp, #0x70]
10075f0a4:     	ldur	x8, [x27, #0x28]
10075f0a8:     	str	x8, [sp, #0x80]
10075f0ac:     	ldr	x1, [x26, #0x58]
10075f0b0:     	cmp	x1, x22
10075f0b4:     	b.ls	0x10075f678 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbbc>
10075f0b8:     	ldr	x8, [x26, #0x50]
10075f0bc:     	ldr	w8, [x8, x22, lsl #2]
10075f0c0:     	ldr	w9, [sp, #0x80]
10075f0c4:     	ldr	x10, [x20, #0x30]
10075f0c8:     	lsr	x0, x9, #1
10075f0cc:     	cmn	x10, #0x1
10075f0d0:     	b.eq	0x10075f3b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x8fc>
10075f0d4:     	ldr	x1, [x20, #0x40]
10075f0d8:     	cmp	x1, x0
10075f0dc:     	b.ls	0x10075f68c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbd0>
10075f0e0:     	ldr	x9, [x20, #0x38]
10075f0e4:     	add	x9, x9, x0, lsl #4
10075f0e8:     	ldr	x9, [x9]
10075f0ec:     	mov	w10, #0x1               ; =1
10075f0f0:     	lsl	x8, x10, x8
10075f0f4:     	tst	x9, x8
10075f0f8:     	b.eq	0x10075f10c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x650>
10075f0fc:     	ldp	x9, x10, [sp, #0x70]
10075f100:     	orr	x9, x9, x8
10075f104:     	bic	x8, x10, x8
10075f108:     	stp	x9, x8, [sp, #0x70]
10075f10c:     	sub	x0, x29, #0x70
10075f110:     	add	x1, sp, #0x70
10075f114:     	mov	x2, x20
10075f118:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075f11c:     	ldur	q0, [x29, #-0x70]
10075f120:     	stur	q0, [x29, #-0x90]
10075f124:     	ldur	x8, [x29, #-0x60]
10075f128:     	stur	q0, [x29, #-0xb0]
10075f12c:     	str	q0, [sp, #0x40]
10075f130:     	str	x8, [sp, #0x50]
10075f134:     	ldr	q0, [sp, #0x40]
10075f138:     	str	x8, [sp, #0xc8]
10075f13c:     	stur	q0, [sp, #0xb8]
10075f140:     	ldp	q0, q1, [sp, #0xa0]
10075f144:     	ldr	q2, [sp, #0xc0]
10075f148:     	stp	q1, q2, [sp, #0x50]
10075f14c:     	str	q0, [sp, #0x40]
10075f150:     	add	x2, sp, #0x40
10075f154:     	mov	x0, x26
10075f158:     	mov	x1, x20
10075f15c:     	bl	0x10075eabc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
10075f160:     	mov	x23, x0
10075f164:     	cmp	w0, #0x1
10075f168:     	b.ne	0x10075f338 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x87c>
10075f16c:     	ldr	x8, [x26, #0x80]
10075f170:     	lsr	x8, x8, x22
10075f174:     	tbz	w8, #0x0, 0x10075f338 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x87c>
10075f178:     	mov	w19, #0x1               ; =1
10075f17c:     	b	0x10075f5dc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb20>
10075f180:     	stp	x27, x26, [sp, #0x10]
10075f184:     	ldp	x9, x8, [sp, #0xa0]
10075f188:     	str	x9, [sp, #0x20]
10075f18c:     	str	x8, [sp, #0x8]
10075f190:     	cbz	x19, 0x10075f418 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x95c>
10075f194:     	add	x9, x8, x19, lsl #2
10075f198:     	str	x9, [sp, #0x30]
10075f19c:     	mov	x19, x8
10075f1a0:     	b	0x10075f1bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x700>
10075f1a4:     	bic	x22, x22, x21
10075f1a8:     	mov	x28, x24
10075f1ac:     	mov	x23, x26
10075f1b0:     	ldr	x8, [sp, #0x30]
10075f1b4:     	cmp	x19, x8
10075f1b8:     	b.eq	0x10075f420 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x964>
10075f1bc:     	ldr	w8, [x19], #0x4
10075f1c0:     	mov	w9, #0x1                ; =1
10075f1c4:     	lsl	x21, x9, x8
10075f1c8:     	sub	x8, x21, #0x1
10075f1cc:     	and	x8, x8, x22
10075f1d0:     	fmov	d0, x8
10075f1d4:     	cnt.8b	v0, v0
10075f1d8:     	addv.8b	b0, v0
10075f1dc:     	fmov	w26, s0
10075f1e0:     	fmov	d0, x22
10075f1e4:     	cnt.8b	v0, v0
10075f1e8:     	addv.8b	b0, v0
10075f1ec:     	fmov	w27, s0
10075f1f0:     	add	x0, sp, #0x70
10075f1f4:     	mov	x1, x23
10075f1f8:     	mov	x2, x28
10075f1fc:     	mov	x3, x27
10075f200:     	mov	x4, x26
10075f204:     	mov	w5, #0x0                ; =0
10075f208:     	bl	0x100e400d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
10075f20c:     	add	x0, sp, #0xa0
10075f210:     	mov	x1, x23
10075f214:     	mov	x2, x28
10075f218:     	mov	x3, x27
10075f21c:     	mov	x4, x26
10075f220:     	mov	w5, #0x1                ; =1
10075f224:     	bl	0x100e400d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
10075f228:     	ldp	x27, x8, [sp, #0x78]
10075f22c:     	ldp	x20, x0, [sp, #0xa0]
10075f230:     	ldr	x9, [sp, #0xb0]
10075f234:     	cmp	x9, x8
10075f238:     	csel	x24, x9, x8, lo
10075f23c:     	cbz	x24, 0x10075f2a0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7e4>
10075f240:     	str	x23, [sp, #0x38]
10075f244:     	mov	x23, x0
10075f248:     	lsl	x25, x24, #3
10075f24c:     	mov	x0, x25
10075f250:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
10075f254:     	cbz	x0, 0x10075f668 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbac>
10075f258:     	mov	x26, x0
10075f25c:     	cmp	x24, #0x8
10075f260:     	mov	x0, x23
10075f264:     	mov	x8, #0x0                ; =0
10075f268:     	b.hs	0x10075f2cc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x810>
10075f26c:     	ldr	x23, [sp, #0x38]
10075f270:     	lsl	x11, x8, #3
10075f274:     	add	x9, x27, x11
10075f278:     	add	x10, x0, x11
10075f27c:     	add	x11, x26, x11
10075f280:     	sub	x8, x24, x8
10075f284:     	ldr	x12, [x10], #0x8
10075f288:     	ldr	x13, [x9], #0x8
10075f28c:     	orr	x12, x13, x12
10075f290:     	str	x12, [x11], #0x8
10075f294:     	subs	x8, x8, #0x1
10075f298:     	b.ne	0x10075f284 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7c8>
10075f29c:     	b	0x10075f2a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7e8>
10075f2a0:     	mov	w26, #0x8               ; =8
10075f2a4:     	cbz	x20, 0x10075f2ac <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7f0>
10075f2a8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f2ac:     	cbz	x28, 0x10075f2b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7fc>
10075f2b0:     	mov	x0, x23
10075f2b4:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f2b8:     	ldr	x8, [sp, #0x70]
10075f2bc:     	cbz	x8, 0x10075f1a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e8>
10075f2c0:     	mov	x0, x27
10075f2c4:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f2c8:     	b	0x10075f1a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e8>
10075f2cc:     	sub	x9, x0, x26
10075f2d0:     	cmn	x9, #0x40
10075f2d4:     	ldr	x23, [sp, #0x38]
10075f2d8:     	b.hi	0x10075f270 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7b4>
10075f2dc:     	sub	x9, x27, x26
10075f2e0:     	cmn	x9, #0x40
10075f2e4:     	b.hi	0x10075f270 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7b4>
10075f2e8:     	and	x8, x24, #0xffffffffffffff8
10075f2ec:     	add	x9, x27, #0x20
10075f2f0:     	add	x10, x0, #0x20
10075f2f4:     	add	x11, x26, #0x20
10075f2f8:     	and	x12, x24, #0xffffffffffffff8
10075f2fc:     	ldp	q0, q1, [x10, #-0x20]
10075f300:     	ldp	q2, q3, [x10], #0x40
10075f304:     	ldp	q4, q5, [x9, #-0x20]
10075f308:     	ldp	q6, q7, [x9], #0x40
10075f30c:     	orr.16b	v0, v4, v0
10075f310:     	orr.16b	v1, v5, v1
10075f314:     	orr.16b	v2, v6, v2
10075f318:     	orr.16b	v3, v7, v3
10075f31c:     	stp	q0, q1, [x11, #-0x20]
10075f320:     	stp	q2, q3, [x11], #0x40
10075f324:     	subs	x12, x12, #0x8
10075f328:     	b.ne	0x10075f2fc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x840>
10075f32c:     	cmp	x24, x8
10075f330:     	b.ne	0x10075f270 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7b4>
10075f334:     	b	0x10075f2a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7e8>
10075f338:     	ldr	q0, [x27]
10075f33c:     	stur	q0, [x29, #-0x70]
10075f340:     	ldr	x8, [x27, #0x10]
10075f344:     	stur	x8, [x29, #-0x60]
10075f348:     	ldr	x1, [x26, #0x28]
10075f34c:     	cmp	x1, x22
10075f350:     	b.ls	0x10075f6a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbec>
10075f354:     	ldr	x8, [x26, #0x20]
10075f358:     	ldr	w8, [x8, x22, lsl #2]
10075f35c:     	ldur	w9, [x29, #-0x60]
10075f360:     	ldr	x10, [x20, #0x30]
10075f364:     	lsr	x0, x9, #1
10075f368:     	cmn	x10, #0x1
10075f36c:     	b.eq	0x10075f454 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x998>
10075f370:     	ldr	x1, [x20, #0x40]
10075f374:     	cmp	x1, x0
10075f378:     	b.ls	0x10075f68c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbd0>
10075f37c:     	ldr	x9, [x20, #0x38]
10075f380:     	add	x9, x9, x0, lsl #4
10075f384:     	b	0x10075f46c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9b0>
10075f388:     	ldr	x1, [x20, #0x48]
10075f38c:     	cmp	x1, x0
10075f390:     	b.ls	0x10075f6bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
10075f394:     	ldr	x9, [x20, #0x40]
10075f398:     	add	x9, x9, x0, lsl #5
10075f39c:     	add	x9, x9, #0x18
10075f3a0:     	ldr	x9, [x9]
10075f3a4:     	mov	w10, #0x1               ; =1
10075f3a8:     	lsl	x8, x10, x8
10075f3ac:     	tst	x9, x8
10075f3b0:     	b.ne	0x10075f058 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x59c>
10075f3b4:     	b	0x10075f068 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5ac>
10075f3b8:     	ldr	x1, [x20, #0x48]
10075f3bc:     	cmp	x1, x0
10075f3c0:     	b.ls	0x10075f6bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
10075f3c4:     	ldr	x9, [x20, #0x40]
10075f3c8:     	add	x9, x9, x0, lsl #5
10075f3cc:     	add	x9, x9, #0x18
10075f3d0:     	ldr	x9, [x9]
10075f3d4:     	mov	w10, #0x1               ; =1
10075f3d8:     	lsl	x8, x10, x8
10075f3dc:     	tst	x9, x8
10075f3e0:     	b.ne	0x10075f0fc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x640>
10075f3e4:     	b	0x10075f10c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x650>
10075f3e8:     	mov	x24, x28
10075f3ec:     	b	0x10075f438 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x97c>
10075f3f0:     	tbz	w21, #0x0, 0x10075f5f4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb38>
10075f3f4:     	mov	w0, #0x0                ; =0
10075f3f8:     	ldp	x29, x30, [sp, #0x1c0]
10075f3fc:     	ldp	x20, x19, [sp, #0x1b0]
10075f400:     	ldp	x22, x21, [sp, #0x1a0]
10075f404:     	ldp	x24, x23, [sp, #0x190]
10075f408:     	ldp	x26, x25, [sp, #0x180]
10075f40c:     	ldp	x28, x27, [sp, #0x170]
10075f410:     	add	sp, sp, #0x1d0
10075f414:     	ret
10075f418:     	mov	x26, x23
10075f41c:     	mov	x24, x28
10075f420:     	ldr	x8, [sp, #0x20]
10075f424:     	cbz	x8, 0x10075f430 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x974>
10075f428:     	ldr	x0, [sp, #0x8]
10075f42c:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f430:     	mov	x23, x26
10075f434:     	ldp	x27, x26, [sp, #0x10]
10075f438:     	stp	x24, x23, [sp, #0xa0]
10075f43c:     	str	x24, [sp, #0xb0]
10075f440:     	add	x2, sp, #0xa0
10075f444:     	ldr	x0, [sp, #0x28]
10075f448:     	mov	x1, x22
10075f44c:     	bl	0x100ca6924 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
10075f450:     	b	0x10075f5d8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb1c>
10075f454:     	ldr	x1, [x20, #0x48]
10075f458:     	cmp	x1, x0
10075f45c:     	b.ls	0x10075f6bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
10075f460:     	ldr	x9, [x20, #0x40]
10075f464:     	add	x9, x9, x0, lsl #5
10075f468:     	add	x9, x9, #0x18
10075f46c:     	ldr	x9, [x9]
10075f470:     	mov	w10, #0x1               ; =1
10075f474:     	lsl	x8, x10, x8
10075f478:     	tst	x9, x8
10075f47c:     	b.eq	0x10075f490 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9d4>
10075f480:     	ldur	q0, [x29, #-0x70]
10075f484:     	dup.2d	v1, x8
10075f488:     	orr.16b	v0, v0, v1
10075f48c:     	stur	q0, [x29, #-0x70]
10075f490:     	sub	x0, x29, #0xb0
10075f494:     	sub	x1, x29, #0x70
10075f498:     	mov	x2, x20
10075f49c:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075f4a0:     	ldur	q0, [x29, #-0xb0]
10075f4a4:     	stur	q0, [x29, #-0xd0]
10075f4a8:     	ldur	x8, [x29, #-0xa0]
10075f4ac:     	str	q0, [sp, #0xd0]
10075f4b0:     	stur	q0, [x29, #-0x90]
10075f4b4:     	stur	x8, [x29, #-0x80]
10075f4b8:     	ldur	q0, [x29, #-0x90]
10075f4bc:     	str	x8, [sp, #0xb0]
10075f4c0:     	str	q0, [sp, #0xa0]
10075f4c4:     	ldur	q0, [x27, #0x18]
10075f4c8:     	stur	q0, [x29, #-0x70]
10075f4cc:     	ldur	x8, [x27, #0x28]
10075f4d0:     	stur	x8, [x29, #-0x60]
10075f4d4:     	ldr	x1, [x26, #0x58]
10075f4d8:     	cmp	x1, x22
10075f4dc:     	b.ls	0x10075f6a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbec>
10075f4e0:     	ldr	x8, [x26, #0x50]
10075f4e4:     	ldr	w8, [x8, x22, lsl #2]
10075f4e8:     	ldur	w9, [x29, #-0x60]
10075f4ec:     	ldr	x10, [x20, #0x30]
10075f4f0:     	lsr	x0, x9, #1
10075f4f4:     	cmn	x10, #0x1
10075f4f8:     	b.eq	0x10075f514 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa58>
10075f4fc:     	ldr	x1, [x20, #0x40]
10075f500:     	cmp	x1, x0
10075f504:     	b.ls	0x10075f68c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbd0>
10075f508:     	ldr	x9, [x20, #0x38]
10075f50c:     	add	x9, x9, x0, lsl #4
10075f510:     	b	0x10075f52c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa70>
10075f514:     	ldr	x1, [x20, #0x48]
10075f518:     	cmp	x1, x0
10075f51c:     	b.ls	0x10075f6bc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
10075f520:     	ldr	x9, [x20, #0x40]
10075f524:     	add	x9, x9, x0, lsl #5
10075f528:     	add	x9, x9, #0x18
10075f52c:     	and	w19, w22, #0x3f
10075f530:     	ldr	x9, [x9]
10075f534:     	mov	w10, #0x1               ; =1
10075f538:     	lsl	x8, x10, x8
10075f53c:     	tst	x9, x8
10075f540:     	b.eq	0x10075f554 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa98>
10075f544:     	ldur	q0, [x29, #-0x70]
10075f548:     	dup.2d	v1, x8
10075f54c:     	orr.16b	v0, v0, v1
10075f550:     	stur	q0, [x29, #-0x70]
10075f554:     	sub	x0, x29, #0xb0
10075f558:     	sub	x1, x29, #0x70
10075f55c:     	mov	x2, x20
10075f560:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075f564:     	ldur	q0, [x29, #-0xb0]
10075f568:     	stur	q0, [x29, #-0xd0]
10075f56c:     	ldur	x8, [x29, #-0xa0]
10075f570:     	str	q0, [sp, #0xd0]
10075f574:     	stur	q0, [x29, #-0x90]
10075f578:     	stur	x8, [x29, #-0x80]
10075f57c:     	ldur	q0, [x29, #-0x90]
10075f580:     	str	x8, [sp, #0xc8]
10075f584:     	stur	q0, [sp, #0xb8]
10075f588:     	ldp	q0, q1, [sp, #0xa0]
10075f58c:     	ldr	q2, [sp, #0xc0]
10075f590:     	stp	q1, q2, [sp, #0x80]
10075f594:     	str	q0, [sp, #0x70]
10075f598:     	add	x2, sp, #0x70
10075f59c:     	mov	x0, x26
10075f5a0:     	mov	x1, x20
10075f5a4:     	bl	0x10075eabc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
10075f5a8:     	mov	x3, x0
10075f5ac:     	ldr	x8, [x26, #0x80]
10075f5b0:     	mov	x0, x20
10075f5b4:     	lsr	x8, x8, x19
10075f5b8:     	tbz	w8, #0x0, 0x10075f5cc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb10>
10075f5bc:     	mov	w1, #0xe                ; =14
10075f5c0:     	mov	x2, x23
10075f5c4:     	bl	0x100ca64c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
10075f5c8:     	b	0x10075f5d8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb1c>
10075f5cc:     	mov	x1, x22
10075f5d0:     	mov	x2, x23
10075f5d4:     	bl	0x100ca6f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
10075f5d8:     	mov	x19, x0
10075f5dc:     	add	x0, x26, #0x60
10075f5e0:     	mov	x1, x27
10075f5e4:     	mov	x2, x19
10075f5e8:     	bl	0x100d3e1bc <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
10075f5ec:     	mov	x0, x19
10075f5f0:     	b	0x10075f3f8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x93c>
10075f5f4:     	mov	x0, x16
10075f5f8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f5fc:     	mov	w0, #0x0                ; =0
10075f600:     	b	0x10075f3f8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x93c>
10075f604:     	adrp	x0, 0x10145b000 <dyld_stub_binder+0x10145b000>
10075f608:     	add	x0, x0, #0x932
10075f60c:     	adrp	x2, 0x1015f9000 <dyld_stub_binder+0x1015f9000>
10075f610:     	add	x2, x2, #0xff8
10075f614:     	mov	w1, #0x51               ; =81
10075f618:     	bl	0x1013b4bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
10075f61c:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
10075f620:     	add	x2, x2, #0xd30
10075f624:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f628:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
10075f62c:     	add	x2, x2, #0xd30
10075f630:     	mov	x1, x8
10075f634:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f638:     	mov	x1, x11
10075f63c:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10075f640:     	add	x2, x2, #0x6a8
10075f644:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f648:     	b	0x10075f6a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbe8>
10075f64c:     	str	x16, [sp, #0x38]
10075f650:     	b	0x10075f660 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xba4>
10075f654:     	str	x16, [sp, #0x38]
10075f658:     	mov	x1, x8
10075f65c:     	mov	x2, x12
10075f660:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f664:     	b	0x10075f6a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbe8>
10075f668:     	mov	w0, #0x8                ; =8
10075f66c:     	mov	x1, x25
10075f670:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
10075f674:     	b	0x10075f6a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbe8>
10075f678:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10075f67c:     	add	x2, x2, #0x6c0
10075f680:     	mov	x0, x22
10075f684:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f688:     	mov	x0, x8
10075f68c:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
10075f690:     	add	x2, x2, #0x40
10075f694:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f698:     	mov	w0, #0x8                ; =8
10075f69c:     	mov	x1, x25
10075f6a0:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
10075f6a4:     	brk	#0x1
10075f6a8:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10075f6ac:     	add	x2, x2, #0x6d8
10075f6b0:     	mov	x0, x22
10075f6b4:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f6b8:     	mov	x0, x8
10075f6bc:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
10075f6c0:     	add	x2, x2, #0x28
10075f6c4:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10075f6c8:     	mov	x19, x0
10075f6cc:     	ldr	x21, [sp, #0xa0]
10075f6d0:     	b	0x10075f774 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcb8>
10075f6d4:     	mov	x19, x0
10075f6d8:     	b	0x10075f784 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcc8>
10075f6dc:     	mov	x19, x0
10075f6e0:     	ldr	x8, [sp, #0xa0]
10075f6e4:     	cbz	x8, 0x10075f7a0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xce4>
10075f6e8:     	ldr	x8, [sp, #0xa8]
10075f6ec:     	b	0x10075f794 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcd8>
10075f6f0:     	mov	x19, x0
10075f6f4:     	cbz	x20, 0x10075f734 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc78>
10075f6f8:     	mov	x0, x23
10075f6fc:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f700:     	b	0x10075f734 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc78>
10075f704:     	str	x23, [sp, #0x38]
10075f708:     	mov	x19, x0
10075f70c:     	ldr	x8, [sp, #0xa0]
10075f710:     	cbz	x8, 0x10075f798 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcdc>
10075f714:     	ldr	x0, [sp, #0xa8]
10075f718:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f71c:     	b	0x10075f798 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcdc>
10075f720:     	str	x23, [sp, #0x38]
10075f724:     	mov	x19, x0
10075f728:     	b	0x10075f744 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc88>
10075f72c:     	str	x23, [sp, #0x38]
10075f730:     	mov	x19, x0
10075f734:     	ldr	x8, [sp, #0x70]
10075f738:     	cbz	x8, 0x10075f744 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc88>
10075f73c:     	ldr	x0, [sp, #0x78]
10075f740:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f744:     	ldr	x8, [sp, #0x20]
10075f748:     	cbz	x8, 0x10075f754 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc98>
10075f74c:     	ldr	x0, [sp, #0x8]
10075f750:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f754:     	cbnz	x28, 0x10075f798 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcdc>
10075f758:     	b	0x10075f7a0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xce4>
10075f75c:     	mov	x19, x0
10075f760:     	tbz	w21, #0x0, 0x10075f798 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcdc>
10075f764:     	b	0x10075f7a0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xce4>
10075f768:     	mov	x19, x0
10075f76c:     	mov	x0, x23
10075f770:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f774:     	cmp	x21, #0x1
10075f778:     	b.lt	0x10075f784 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcc8>
10075f77c:     	ldr	x0, [sp, #0xa8]
10075f780:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f784:     	ldr	x8, [sp, #0x70]
10075f788:     	cmp	x8, #0x1
10075f78c:     	b.lt	0x10075f7a0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xce4>
10075f790:     	ldr	x8, [sp, #0x78]
10075f794:     	str	x8, [sp, #0x38]
10075f798:     	ldr	x0, [sp, #0x38]
10075f79c:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075f7a0:     	mov	x0, x19
10075f7a4:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
