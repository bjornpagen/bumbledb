
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010075f7a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>:
10075f7a8:     	sub	sp, sp, #0x1d0
10075f7ac:     	stp	x28, x27, [sp, #0x170]
10075f7b0:     	stp	x26, x25, [sp, #0x180]
10075f7b4:     	stp	x24, x23, [sp, #0x190]
10075f7b8:     	stp	x22, x21, [sp, #0x1a0]
10075f7bc:     	stp	x20, x19, [sp, #0x1b0]
10075f7c0:     	stp	x29, x30, [sp, #0x1c0]
10075f7c4:     	add	x29, sp, #0x1c0
10075f7c8:     	ldr	w9, [x2, #0x10]
10075f7cc:     	cbz	w9, 0x1007600fc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x954>
10075f7d0:     	mov	x26, x2
10075f7d4:     	ldr	w10, [x2, #0x28]
10075f7d8:     	cbz	w10, 0x1007600fc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x954>
10075f7dc:     	mov	x27, x1
10075f7e0:     	mov	x25, x0
10075f7e4:     	cmp	w9, #0x1
10075f7e8:     	ccmp	w10, #0x1, #0x0, eq
10075f7ec:     	b.eq	0x10075f910 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x168>
10075f7f0:     	ldr	x8, [x25, #0x78]
10075f7f4:     	cbz	x8, 0x10075f918 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
10075f7f8:     	mov	x8, #0x0                ; =0
10075f7fc:     	mov	x15, #0xa9c5            ; =43461
10075f800:     	movk	x15, #0x2e62, lsl #16
10075f804:     	movk	x15, #0x7aea, lsl #32
10075f808:     	movk	x15, #0xf135, lsl #48
10075f80c:     	ldp	x11, x12, [x26]
10075f810:     	madd	x13, x9, x15, x11
10075f814:     	mov	x14, #0x6332            ; =25394
10075f818:     	movk	x14, #0x6ed3, lsl #16
10075f81c:     	movk	x14, #0x765a, lsl #32
10075f820:     	movk	x14, #0x284f, lsl #48
10075f824:     	mul	x14, x14, x15
10075f828:     	madd	x13, x13, x15, x14
10075f82c:     	add	x13, x13, x12
10075f830:     	madd	x16, x13, x15, x10
10075f834:     	ldp	x13, x14, [x26, #0x18]
10075f838:     	madd	x16, x16, x15, x13
10075f83c:     	madd	x16, x16, x15, x14
10075f840:     	mul	x15, x16, x15
10075f844:     	ror	x0, x15, #0x2c
10075f848:     	lsr	x17, x0, #57
10075f84c:     	ldp	x16, x15, [x25, #0x60]
10075f850:     	dup.8b	v0, w17
10075f854:     	movi.2d	v1, #0xffffffffffffffff
10075f858:     	mov	w17, #0x38              ; =56
10075f85c:     	and	x0, x0, x15
10075f860:     	ldr	d2, [x16, x0]
10075f864:     	cmeq.8b	v3, v2, v0
10075f868:     	fmov	x1, d3
10075f86c:     	ands	x1, x1, #0x8080808080808080
10075f870:     	b.eq	0x10075f8e0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x138>
10075f874:     	rbit	x2, x1
10075f878:     	clz	x2, x2
10075f87c:     	add	x2, x0, x2, lsr #3
10075f880:     	and	x2, x2, x15
10075f884:     	mneg	x2, x2, x17
10075f888:     	add	x2, x16, x2
10075f88c:     	ldur	x3, [x2, #-0x38]
10075f890:     	cmp	x11, x3
10075f894:     	b.ne	0x10075f8d4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12c>
10075f898:     	ldur	x3, [x2, #-0x30]
10075f89c:     	cmp	x12, x3
10075f8a0:     	b.ne	0x10075f8d4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12c>
10075f8a4:     	ldur	w3, [x2, #-0x28]
10075f8a8:     	cmp	w9, w3
10075f8ac:     	b.ne	0x10075f8d4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12c>
10075f8b0:     	ldur	x3, [x2, #-0x20]
10075f8b4:     	cmp	x13, x3
10075f8b8:     	b.ne	0x10075f8d4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12c>
10075f8bc:     	ldur	x3, [x2, #-0x18]
10075f8c0:     	cmp	x14, x3
10075f8c4:     	b.ne	0x10075f8d4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12c>
10075f8c8:     	ldur	w3, [x2, #-0x10]
10075f8cc:     	cmp	w10, w3
10075f8d0:     	b.eq	0x10075f990 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1e8>
10075f8d4:     	sub	x2, x1, #0x2
10075f8d8:     	ands	x1, x2, x1
10075f8dc:     	b.ne	0x10075f874 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcc>
10075f8e0:     	cmeq.8b	v2, v2, v1
10075f8e4:     	fmov	x1, d2
10075f8e8:     	cbnz	x1, 0x10075f918 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
10075f8ec:     	add	x8, x8, #0x8
10075f8f0:     	add	x0, x0, x8
10075f8f4:     	and	x0, x0, x15
10075f8f8:     	ldr	d2, [x16, x0]
10075f8fc:     	cmeq.8b	v3, v2, v0
10075f900:     	fmov	x1, d3
10075f904:     	ands	x1, x1, #0x8080808080808080
10075f908:     	b.ne	0x10075f874 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcc>
10075f90c:     	b	0x10075f8e0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x138>
10075f910:     	mov	w0, #0x1                ; =1
10075f914:     	b	0x100760100 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x958>
10075f918:     	mov	x24, x25
10075f91c:     	ldr	x8, [x24, #0x88]!
10075f920:     	add	x8, x8, #0x1
10075f924:     	str	x8, [x24]
10075f928:     	mov	w11, #0x8481            ; =33921
10075f92c:     	movk	w11, #0x1e, lsl #16
10075f930:     	cmp	x8, x11
10075f934:     	b.hs	0x100760310 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb68>
10075f938:     	ldr	x12, [x26]
10075f93c:     	ldr	x13, [x26, #0x18]
10075f940:     	ldr	x8, [x27, #0x30]
10075f944:     	cmn	x8, #0x1
10075f948:     	b.eq	0x10075f9a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1fc>
10075f94c:     	ldr	x1, [x27, #0x40]
10075f950:     	lsr	x0, x9, #1
10075f954:     	cmp	x1, x0
10075f958:     	b.ls	0x100760398 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf0>
10075f95c:     	lsr	x8, x10, #1
10075f960:     	cmp	x1, x8
10075f964:     	b.ls	0x100760394 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbec>
10075f968:     	ldr	x11, [x27, #0x38]
10075f96c:     	lsl	x14, x0, #4
10075f970:     	ldr	x14, [x11, x14]
10075f974:     	bic	x19, x14, x12
10075f978:     	add	x8, x11, x8, lsl #4
10075f97c:     	ldr	x15, [x8]
10075f980:     	ldp	x11, x1, [x25, #0x8]
10075f984:     	mov	x22, #0x0               ; =0
10075f988:     	cbnz	x19, 0x10075f9e4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x23c>
10075f98c:     	b	0x10075fa14 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x26c>
10075f990:     	ldur	w0, [x2, #-0x8]
10075f994:     	ldr	x8, [x25, #0x90]
10075f998:     	add	x8, x8, #0x1
10075f99c:     	str	x8, [x25, #0x90]
10075f9a0:     	b	0x100760100 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x958>
10075f9a4:     	ldr	x1, [x27, #0x48]
10075f9a8:     	lsr	x0, x9, #1
10075f9ac:     	cmp	x1, x0
10075f9b0:     	b.ls	0x1007603b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc10>
10075f9b4:     	lsr	x8, x10, #1
10075f9b8:     	cmp	x1, x8
10075f9bc:     	b.ls	0x1007603b4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc0c>
10075f9c0:     	ldr	x11, [x27, #0x40]
10075f9c4:     	add	x14, x11, x0, lsl #5
10075f9c8:     	ldr	x14, [x14, #0x18]
10075f9cc:     	bic	x19, x14, x12
10075f9d0:     	add	x8, x11, x8, lsl #5
10075f9d4:     	ldr	x15, [x8, #0x18]!
10075f9d8:     	ldp	x11, x1, [x25, #0x8]
10075f9dc:     	mov	x22, #0x0               ; =0
10075f9e0:     	cbz	x19, 0x10075fa14 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x26c>
10075f9e4:     	mov	w8, #0x1                ; =1
10075f9e8:     	mov	x14, x19
10075f9ec:     	rbit	x16, x14
10075f9f0:     	clz	x0, x16
10075f9f4:     	cmp	x0, x1
10075f9f8:     	b.hs	0x100760328 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb80>
10075f9fc:     	ldr	w16, [x11, x0, lsl #2]
10075fa00:     	lsl	x16, x8, x16
10075fa04:     	orr	x22, x16, x22
10075fa08:     	sub	x16, x14, #0x1
10075fa0c:     	ands	x14, x16, x14
10075fa10:     	b.ne	0x10075f9ec <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x244>
10075fa14:     	ldp	x14, x8, [x25, #0x38]
10075fa18:     	bic	x15, x15, x13
10075fa1c:     	cbz	x15, 0x10075fa54 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2ac>
10075fa20:     	mov	x16, #0x0               ; =0
10075fa24:     	mov	w17, #0x1               ; =1
10075fa28:     	rbit	x0, x15
10075fa2c:     	clz	x0, x0
10075fa30:     	cmp	x0, x8
10075fa34:     	b.hs	0x100760334 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb8c>
10075fa38:     	ldr	w0, [x14, x0, lsl #2]
10075fa3c:     	lsl	x0, x17, x0
10075fa40:     	orr	x16, x0, x16
10075fa44:     	sub	x0, x15, #0x1
10075fa48:     	ands	x15, x0, x15
10075fa4c:     	b.ne	0x10075fa28 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x280>
10075fa50:     	orr	x22, x16, x22
10075fa54:     	eor	w9, w10, w9
10075fa58:     	cmp	x12, x13
10075fa5c:     	ccmp	w9, #0x1, #0x0, eq
10075fa60:     	b.ne	0x10075fb3c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x394>
10075fa64:     	ldr	x9, [x26, #0x8]
10075fa68:     	ldr	x10, [x26, #0x20]
10075fa6c:     	cmp	x9, x10
10075fa70:     	b.ne	0x10075fb3c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x394>
10075fa74:     	mov	w16, #0x4               ; =4
10075fa78:     	stp	xzr, x16, [sp, #0xa0]
10075fa7c:     	str	xzr, [sp, #0xb0]
10075fa80:     	mov	x20, #0x0               ; =0
10075fa84:     	cbz	x19, 0x10075fae4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x33c>
10075fa88:     	mov	w8, #0x4                ; =4
10075fa8c:     	b	0x10075fab4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x30c>
10075fa90:     	ldr	x8, [sp, #0xa8]
10075fa94:     	rbit	x9, x19
10075fa98:     	clz	x9, x9
10075fa9c:     	str	w9, [x8, x20, lsl #2]
10075faa0:     	add	x20, x20, #0x1
10075faa4:     	str	x20, [sp, #0xb0]
10075faa8:     	sub	x9, x19, #0x1
10075faac:     	ands	x19, x9, x19
10075fab0:     	b.eq	0x10075facc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x324>
10075fab4:     	ldr	x9, [sp, #0xa0]
10075fab8:     	cmp	x20, x9
10075fabc:     	b.ne	0x10075fa94 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2ec>
10075fac0:     	add	x0, sp, #0xa0
10075fac4:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10075fac8:     	b	0x10075fa90 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2e8>
10075facc:     	ldp	x9, x16, [sp, #0xa0]
10075fad0:     	ldp	x14, x8, [x25, #0x38]
10075fad4:     	ldp	x11, x1, [x25, #0x8]
10075fad8:     	cmp	x9, #0x0
10075fadc:     	cset	w21, eq
10075fae0:     	b	0x10075fae8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x340>
10075fae4:     	mov	w21, #0x1               ; =1
10075fae8:     	mov	x9, #0x0                ; =0
10075faec:     	lsl	x10, x20, #2
10075faf0:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10075faf4:     	add	x2, x2, #0x678
10075faf8:     	adrp	x12, 0x101601000 <dyld_stub_binder+0x101601000>
10075fafc:     	add	x12, x12, #0x690
10075fb00:     	cmp	x10, x9
10075fb04:     	b.eq	0x1007600f8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x950>
10075fb08:     	ldr	w0, [x16, x9]
10075fb0c:     	cmp	x1, x0
10075fb10:     	b.ls	0x100760358 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
10075fb14:     	cmp	x8, x0
10075fb18:     	b.ls	0x100760360 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb8>
10075fb1c:     	ldr	w13, [x11, x0, lsl #2]
10075fb20:     	ldr	w15, [x14, x0, lsl #2]
10075fb24:     	add	x9, x9, #0x4
10075fb28:     	cmp	w13, w15
10075fb2c:     	b.eq	0x10075fb00 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x358>
10075fb30:     	tbnz	w21, #0x0, 0x10075fb3c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x394>
10075fb34:     	mov	x0, x16
10075fb38:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075fb3c:     	fmov	d0, x22
10075fb40:     	cnt.8b	v0, v0
10075fb44:     	addv.8b	b0, v0
10075fb48:     	fmov	x19, d0
10075fb4c:     	cmp	x19, #0xa
10075fb50:     	b.hs	0x10075fcd4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x52c>
10075fb54:     	ldr	x8, [x25, #0x98]
10075fb58:     	add	x8, x8, #0x1
10075fb5c:     	str	x8, [x25, #0x98]
10075fb60:     	ldrb	w5, [x25, #0xd8]
10075fb64:     	add	x0, sp, #0x70
10075fb68:     	mov	x1, x27
10075fb6c:     	mov	x2, x26
10075fb70:     	mov	x3, x25
10075fb74:     	mov	x4, x22
10075fb78:     	mov	x6, x24
10075fb7c:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
10075fb80:     	ldrb	w5, [x25, #0xd8]
10075fb84:     	add	x0, sp, #0xa0
10075fb88:     	add	x2, x26, #0x18
10075fb8c:     	add	x3, x25, #0x30
10075fb90:     	str	x27, [sp, #0x28]
10075fb94:     	mov	x1, x27
10075fb98:     	mov	x4, x22
10075fb9c:     	mov	x6, x24
10075fba0:     	bl	0x100bb7de8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
10075fba4:     	mov	w8, #0x1                ; =1
10075fba8:     	lsl	x8, x8, x19
10075fbac:     	lsr	x8, x8, #6
10075fbb0:     	cmp	x19, #0x6
10075fbb4:     	cinc	x27, x8, lo
10075fbb8:     	cbz	x27, 0x100760080 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x8d8>
10075fbbc:     	lsl	x24, x27, #3
10075fbc0:     	mov	x0, x24
10075fbc4:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
10075fbc8:     	cbz	x0, 0x1007603c4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc1c>
10075fbcc:     	mov	x21, x0
10075fbd0:     	mov	x0, #0x0                ; =0
10075fbd4:     	ldp	x8, x9, [sp, #0x70]
10075fbd8:     	ldp	x1, x10, [sp, #0x80]
10075fbdc:     	sub	x11, x0, w9, uxtb
10075fbe0:     	ldp	x20, x13, [sp, #0xa0]
10075fbe4:     	ldp	x12, x14, [sp, #0xb0]
10075fbe8:     	b	0x10075fc04 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x45c>
10075fbec:     	tst	w13, #0x1
10075fbf0:     	csel	x15, x15, xzr, ne
10075fbf4:     	str	x15, [x21, x0, lsl #3]
10075fbf8:     	add	x0, x0, #0x1
10075fbfc:     	cmp	x27, x0
10075fc00:     	b.eq	0x10075fc4c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
10075fc04:     	mov	x15, x11
10075fc08:     	cmn	x8, #0x2
10075fc0c:     	b.eq	0x10075fc20 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x478>
10075fc10:     	cmp	x0, x1
10075fc14:     	b.hs	0x100760348 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xba0>
10075fc18:     	ldr	x15, [x9, x0, lsl #3]
10075fc1c:     	eor	x15, x10, x15
10075fc20:     	cmn	x20, #0x2
10075fc24:     	b.eq	0x10075fbec <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x444>
10075fc28:     	cmp	x0, x12
10075fc2c:     	b.hs	0x100760344 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb9c>
10075fc30:     	ldr	x16, [x13, x0, lsl #3]
10075fc34:     	eor	x16, x14, x16
10075fc38:     	and	x15, x16, x15
10075fc3c:     	str	x15, [x21, x0, lsl #3]
10075fc40:     	add	x0, x0, #0x1
10075fc44:     	cmp	x27, x0
10075fc48:     	b.ne	0x10075fc04 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x45c>
10075fc4c:     	mov	x24, x27
10075fc50:     	cmp	x20, #0x1
10075fc54:     	b.lt	0x10075fc60 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4b8>
10075fc58:     	ldr	x0, [sp, #0xa8]
10075fc5c:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075fc60:     	ldr	x8, [sp, #0x70]
10075fc64:     	cmp	x8, #0x1
10075fc68:     	b.lt	0x10075fc74 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4cc>
10075fc6c:     	ldr	x0, [sp, #0x78]
10075fc70:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075fc74:     	ldr	x8, [x25, #0x80]
10075fc78:     	mov	w9, #0x4                ; =4
10075fc7c:     	stp	xzr, x9, [sp, #0xa0]
10075fc80:     	str	xzr, [sp, #0xb0]
10075fc84:     	ands	x19, x8, x22
10075fc88:     	b.eq	0x100760144 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x99c>
10075fc8c:     	mov	x20, #0x0               ; =0
10075fc90:     	mov	w8, #0x4                ; =4
10075fc94:     	b	0x10075fcb8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x510>
10075fc98:     	rbit	x9, x19
10075fc9c:     	clz	x9, x9
10075fca0:     	str	w9, [x8, x20, lsl #2]
10075fca4:     	add	x20, x20, #0x1
10075fca8:     	str	x20, [sp, #0xb0]
10075fcac:     	sub	x9, x19, #0x1
10075fcb0:     	ands	x19, x9, x19
10075fcb4:     	b.eq	0x10075fe6c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6c4>
10075fcb8:     	ldr	x9, [sp, #0xa0]
10075fcbc:     	cmp	x20, x9
10075fcc0:     	b.ne	0x10075fc98 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4f0>
10075fcc4:     	add	x0, sp, #0xa0
10075fcc8:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10075fccc:     	ldr	x8, [sp, #0xa8]
10075fcd0:     	b	0x10075fc98 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4f0>
10075fcd4:     	mov	x0, x27
10075fcd8:     	mov	x1, x22
10075fcdc:     	bl	0x100c9a400 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
10075fce0:     	ldr	q0, [x26]
10075fce4:     	str	q0, [sp, #0x70]
10075fce8:     	ldr	x8, [x26, #0x10]
10075fcec:     	str	x8, [sp, #0x80]
10075fcf0:     	mov	w22, w0
10075fcf4:     	ldr	x1, [x25, #0x28]
10075fcf8:     	cmp	x1, x22
10075fcfc:     	b.ls	0x100760384 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbdc>
10075fd00:     	ldr	x8, [x25, #0x20]
10075fd04:     	ldr	w8, [x8, x22, lsl #2]
10075fd08:     	ldr	w9, [sp, #0x80]
10075fd0c:     	ldr	x10, [x27, #0x30]
10075fd10:     	lsr	x0, x9, #1
10075fd14:     	cmn	x10, #0x1
10075fd18:     	b.eq	0x100760098 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x8f0>
10075fd1c:     	ldr	x1, [x27, #0x40]
10075fd20:     	cmp	x1, x0
10075fd24:     	b.ls	0x100760398 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf0>
10075fd28:     	ldr	x9, [x27, #0x38]
10075fd2c:     	add	x9, x9, x0, lsl #4
10075fd30:     	ldr	x9, [x9]
10075fd34:     	mov	w10, #0x1               ; =1
10075fd38:     	lsl	x8, x10, x8
10075fd3c:     	tst	x9, x8
10075fd40:     	b.eq	0x10075fd54 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5ac>
10075fd44:     	ldp	x9, x10, [sp, #0x70]
10075fd48:     	orr	x9, x9, x8
10075fd4c:     	bic	x8, x10, x8
10075fd50:     	stp	x9, x8, [sp, #0x70]
10075fd54:     	sub	x0, x29, #0x70
10075fd58:     	add	x1, sp, #0x70
10075fd5c:     	mov	x2, x27
10075fd60:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075fd64:     	ldur	q0, [x29, #-0x70]
10075fd68:     	stur	q0, [x29, #-0x90]
10075fd6c:     	ldur	x8, [x29, #-0x60]
10075fd70:     	stur	q0, [x29, #-0xb0]
10075fd74:     	str	q0, [sp, #0x40]
10075fd78:     	str	x8, [sp, #0x50]
10075fd7c:     	ldr	q0, [sp, #0x40]
10075fd80:     	str	x8, [sp, #0xb0]
10075fd84:     	str	q0, [sp, #0xa0]
10075fd88:     	ldur	q0, [x26, #0x18]
10075fd8c:     	str	q0, [sp, #0x70]
10075fd90:     	ldur	x8, [x26, #0x28]
10075fd94:     	str	x8, [sp, #0x80]
10075fd98:     	ldr	x1, [x25, #0x58]
10075fd9c:     	cmp	x1, x22
10075fda0:     	b.ls	0x100760384 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbdc>
10075fda4:     	ldr	x8, [x25, #0x50]
10075fda8:     	ldr	w8, [x8, x22, lsl #2]
10075fdac:     	ldr	w9, [sp, #0x80]
10075fdb0:     	ldr	x10, [x27, #0x30]
10075fdb4:     	lsr	x0, x9, #1
10075fdb8:     	cmn	x10, #0x1
10075fdbc:     	b.eq	0x1007600c8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x920>
10075fdc0:     	ldr	x1, [x27, #0x40]
10075fdc4:     	cmp	x1, x0
10075fdc8:     	b.ls	0x100760398 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf0>
10075fdcc:     	ldr	x9, [x27, #0x38]
10075fdd0:     	add	x9, x9, x0, lsl #4
10075fdd4:     	ldr	x9, [x9]
10075fdd8:     	mov	w10, #0x1               ; =1
10075fddc:     	lsl	x8, x10, x8
10075fde0:     	tst	x9, x8
10075fde4:     	b.eq	0x10075fdf8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x650>
10075fde8:     	ldp	x9, x10, [sp, #0x70]
10075fdec:     	orr	x9, x9, x8
10075fdf0:     	bic	x8, x10, x8
10075fdf4:     	stp	x9, x8, [sp, #0x70]
10075fdf8:     	sub	x0, x29, #0x70
10075fdfc:     	add	x1, sp, #0x70
10075fe00:     	mov	x2, x27
10075fe04:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
10075fe08:     	ldur	q0, [x29, #-0x70]
10075fe0c:     	stur	q0, [x29, #-0x90]
10075fe10:     	ldur	x8, [x29, #-0x60]
10075fe14:     	stur	q0, [x29, #-0xb0]
10075fe18:     	str	q0, [sp, #0x40]
10075fe1c:     	str	x8, [sp, #0x50]
10075fe20:     	ldr	q0, [sp, #0x40]
10075fe24:     	str	x8, [sp, #0xc8]
10075fe28:     	stur	q0, [sp, #0xb8]
10075fe2c:     	ldp	q0, q1, [sp, #0xa0]
10075fe30:     	ldr	q2, [sp, #0xc0]
10075fe34:     	stp	q1, q2, [sp, #0x50]
10075fe38:     	str	q0, [sp, #0x40]
10075fe3c:     	add	x2, sp, #0x40
10075fe40:     	mov	x0, x25
10075fe44:     	mov	x1, x27
10075fe48:     	bl	0x10075f7a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
10075fe4c:     	mov	x23, x0
10075fe50:     	cmp	w0, #0x1
10075fe54:     	b.ne	0x100760030 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x888>
10075fe58:     	ldr	x8, [x25, #0x80]
10075fe5c:     	lsr	x8, x8, x22
10075fe60:     	tbz	w8, #0x0, 0x100760030 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x888>
10075fe64:     	mov	w19, #0x1               ; =1
10075fe68:     	b	0x1007602e8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb40>
10075fe6c:     	stp	x26, x25, [sp, #0x10]
10075fe70:     	ldp	x9, x8, [sp, #0xa0]
10075fe74:     	str	x9, [sp, #0x20]
10075fe78:     	str	x8, [sp, #0x8]
10075fe7c:     	cbz	x20, 0x100760120 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x978>
10075fe80:     	mov	x19, x8
10075fe84:     	mov	x25, x27
10075fe88:     	add	x8, x8, x20, lsl #2
10075fe8c:     	str	x8, [sp, #0x30]
10075fe90:     	b	0x10075feb0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x708>
10075fe94:     	bic	x22, x22, x20
10075fe98:     	mov	x24, x25
10075fe9c:     	mov	x21, x26
10075fea0:     	mov	x27, x25
10075fea4:     	ldr	x8, [sp, #0x30]
10075fea8:     	cmp	x19, x8
10075feac:     	b.eq	0x100760128 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x980>
10075feb0:     	ldr	w8, [x19], #0x4
10075feb4:     	mov	w9, #0x1                ; =1
10075feb8:     	lsl	x20, x9, x8
10075febc:     	sub	x8, x20, #0x1
10075fec0:     	and	x8, x8, x22
10075fec4:     	fmov	d0, x8
10075fec8:     	cnt.8b	v0, v0
10075fecc:     	addv.8b	b0, v0
10075fed0:     	fmov	w26, s0
10075fed4:     	fmov	d0, x22
10075fed8:     	cnt.8b	v0, v0
10075fedc:     	addv.8b	b0, v0
10075fee0:     	fmov	w27, s0
10075fee4:     	add	x0, sp, #0x70
10075fee8:     	mov	x1, x21
10075feec:     	mov	x2, x25
10075fef0:     	mov	x3, x27
10075fef4:     	mov	x4, x26
10075fef8:     	mov	w5, #0x0                ; =0
10075fefc:     	str	x21, [sp, #0x38]
10075ff00:     	bl	0x100e400d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
10075ff04:     	add	x0, sp, #0xa0
10075ff08:     	mov	x1, x21
10075ff0c:     	mov	x2, x25
10075ff10:     	mov	x3, x27
10075ff14:     	mov	x4, x26
10075ff18:     	mov	w5, #0x1                ; =1
10075ff1c:     	bl	0x100e400d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
10075ff20:     	ldp	x27, x8, [sp, #0x78]
10075ff24:     	ldp	x23, x28, [sp, #0xa0]
10075ff28:     	ldr	x9, [sp, #0xb0]
10075ff2c:     	cmp	x9, x8
10075ff30:     	csel	x25, x9, x8, lo
10075ff34:     	cbz	x25, 0x10075ff90 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7e8>
10075ff38:     	mov	x21, x24
10075ff3c:     	lsl	x24, x25, #3
10075ff40:     	mov	x0, x24
10075ff44:     	bl	0x1013bd244 <dyld_stub_binder+0x1013bd244>
10075ff48:     	cbz	x0, 0x100760374 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbcc>
10075ff4c:     	mov	x26, x0
10075ff50:     	cmp	x25, #0x8
10075ff54:     	b.hs	0x10075ffc0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x818>
10075ff58:     	mov	x8, #0x0                ; =0
10075ff5c:     	mov	x24, x21
10075ff60:     	lsl	x11, x8, #3
10075ff64:     	add	x9, x27, x11
10075ff68:     	add	x10, x28, x11
10075ff6c:     	add	x11, x26, x11
10075ff70:     	sub	x8, x25, x8
10075ff74:     	ldr	x12, [x10], #0x8
10075ff78:     	ldr	x13, [x9], #0x8
10075ff7c:     	orr	x12, x13, x12
10075ff80:     	str	x12, [x11], #0x8
10075ff84:     	subs	x8, x8, #0x1
10075ff88:     	b.ne	0x10075ff74 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7cc>
10075ff8c:     	b	0x10075ff94 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7ec>
10075ff90:     	mov	w26, #0x8               ; =8
10075ff94:     	cbz	x23, 0x10075ffa0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7f8>
10075ff98:     	mov	x0, x28
10075ff9c:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ffa0:     	cbz	x24, 0x10075ffac <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x804>
10075ffa4:     	ldr	x0, [sp, #0x38]
10075ffa8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ffac:     	ldr	x8, [sp, #0x70]
10075ffb0:     	cbz	x8, 0x10075fe94 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6ec>
10075ffb4:     	mov	x0, x27
10075ffb8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10075ffbc:     	b	0x10075fe94 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6ec>
10075ffc0:     	mov	x8, #0x0                ; =0
10075ffc4:     	sub	x9, x28, x26
10075ffc8:     	cmn	x9, #0x40
10075ffcc:     	mov	x24, x21
10075ffd0:     	b.hi	0x10075ff60 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7b8>
10075ffd4:     	sub	x9, x27, x26
10075ffd8:     	cmn	x9, #0x40
10075ffdc:     	b.hi	0x10075ff60 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7b8>
10075ffe0:     	and	x8, x25, #0xffffffffffffff8
10075ffe4:     	add	x9, x27, #0x20
10075ffe8:     	add	x10, x28, #0x20
10075ffec:     	add	x11, x26, #0x20
10075fff0:     	and	x12, x25, #0xffffffffffffff8
10075fff4:     	ldp	q0, q1, [x10, #-0x20]
10075fff8:     	ldp	q2, q3, [x10], #0x40
10075fffc:     	ldp	q4, q5, [x9, #-0x20]
100760000:     	ldp	q6, q7, [x9], #0x40
100760004:     	orr.16b	v0, v4, v0
100760008:     	orr.16b	v1, v5, v1
10076000c:     	orr.16b	v2, v6, v2
100760010:     	orr.16b	v3, v7, v3
100760014:     	stp	q0, q1, [x11, #-0x20]
100760018:     	stp	q2, q3, [x11], #0x40
10076001c:     	subs	x12, x12, #0x8
100760020:     	b.ne	0x10075fff4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x84c>
100760024:     	cmp	x25, x8
100760028:     	b.ne	0x10075ff60 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7b8>
10076002c:     	b	0x10075ff94 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7ec>
100760030:     	ldr	q0, [x26]
100760034:     	stur	q0, [x29, #-0x70]
100760038:     	ldr	x8, [x26, #0x10]
10076003c:     	stur	x8, [x29, #-0x60]
100760040:     	ldr	x1, [x25, #0x28]
100760044:     	cmp	x1, x22
100760048:     	b.ls	0x1007603a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbfc>
10076004c:     	ldr	x8, [x25, #0x20]
100760050:     	ldr	w8, [x8, x22, lsl #2]
100760054:     	ldur	w9, [x29, #-0x60]
100760058:     	ldr	x10, [x27, #0x30]
10076005c:     	lsr	x0, x9, #1
100760060:     	cmn	x10, #0x1
100760064:     	b.eq	0x100760160 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9b8>
100760068:     	ldr	x1, [x27, #0x40]
10076006c:     	cmp	x1, x0
100760070:     	b.ls	0x100760398 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf0>
100760074:     	ldr	x9, [x27, #0x38]
100760078:     	add	x9, x9, x0, lsl #4
10076007c:     	b	0x100760178 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9d0>
100760080:     	mov	x24, #0x0               ; =0
100760084:     	ldr	x20, [sp, #0xa0]
100760088:     	mov	w21, #0x8               ; =8
10076008c:     	cmp	x20, #0x1
100760090:     	b.ge	0x10075fc58 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4b0>
100760094:     	b	0x10075fc60 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4b8>
100760098:     	ldr	x1, [x27, #0x48]
10076009c:     	cmp	x1, x0
1007600a0:     	b.ls	0x1007603b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc10>
1007600a4:     	ldr	x9, [x27, #0x40]
1007600a8:     	add	x9, x9, x0, lsl #5
1007600ac:     	add	x9, x9, #0x18
1007600b0:     	ldr	x9, [x9]
1007600b4:     	mov	w10, #0x1               ; =1
1007600b8:     	lsl	x8, x10, x8
1007600bc:     	tst	x9, x8
1007600c0:     	b.ne	0x10075fd44 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x59c>
1007600c4:     	b	0x10075fd54 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5ac>
1007600c8:     	ldr	x1, [x27, #0x48]
1007600cc:     	cmp	x1, x0
1007600d0:     	b.ls	0x1007603b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc10>
1007600d4:     	ldr	x9, [x27, #0x40]
1007600d8:     	add	x9, x9, x0, lsl #5
1007600dc:     	add	x9, x9, #0x18
1007600e0:     	ldr	x9, [x9]
1007600e4:     	mov	w10, #0x1               ; =1
1007600e8:     	lsl	x8, x10, x8
1007600ec:     	tst	x9, x8
1007600f0:     	b.ne	0x10075fde8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x640>
1007600f4:     	b	0x10075fdf8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x650>
1007600f8:     	tbz	w21, #0x0, 0x100760300 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb58>
1007600fc:     	mov	w0, #0x0                ; =0
100760100:     	ldp	x29, x30, [sp, #0x1c0]
100760104:     	ldp	x20, x19, [sp, #0x1b0]
100760108:     	ldp	x22, x21, [sp, #0x1a0]
10076010c:     	ldp	x24, x23, [sp, #0x190]
100760110:     	ldp	x26, x25, [sp, #0x180]
100760114:     	ldp	x28, x27, [sp, #0x170]
100760118:     	add	sp, sp, #0x1d0
10076011c:     	ret
100760120:     	mov	x26, x21
100760124:     	mov	x25, x24
100760128:     	ldr	x8, [sp, #0x20]
10076012c:     	cbz	x8, 0x100760138 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x990>
100760130:     	ldr	x0, [sp, #0x8]
100760134:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100760138:     	mov	x24, x25
10076013c:     	mov	x21, x26
100760140:     	ldp	x26, x25, [sp, #0x10]
100760144:     	stp	x24, x21, [sp, #0xa0]
100760148:     	str	x27, [sp, #0xb0]
10076014c:     	add	x2, sp, #0xa0
100760150:     	ldr	x0, [sp, #0x28]
100760154:     	mov	x1, x22
100760158:     	bl	0x100ca9e64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
10076015c:     	b	0x1007602e4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb3c>
100760160:     	ldr	x1, [x27, #0x48]
100760164:     	cmp	x1, x0
100760168:     	b.ls	0x1007603b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc10>
10076016c:     	ldr	x9, [x27, #0x40]
100760170:     	add	x9, x9, x0, lsl #5
100760174:     	add	x9, x9, #0x18
100760178:     	ldr	x9, [x9]
10076017c:     	mov	w10, #0x1               ; =1
100760180:     	lsl	x8, x10, x8
100760184:     	tst	x9, x8
100760188:     	b.eq	0x10076019c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9f4>
10076018c:     	ldur	q0, [x29, #-0x70]
100760190:     	dup.2d	v1, x8
100760194:     	orr.16b	v0, v0, v1
100760198:     	stur	q0, [x29, #-0x70]
10076019c:     	sub	x0, x29, #0xb0
1007601a0:     	sub	x1, x29, #0x70
1007601a4:     	mov	x2, x27
1007601a8:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
1007601ac:     	ldur	q0, [x29, #-0xb0]
1007601b0:     	stur	q0, [x29, #-0xd0]
1007601b4:     	ldur	x8, [x29, #-0xa0]
1007601b8:     	str	q0, [sp, #0xd0]
1007601bc:     	stur	q0, [x29, #-0x90]
1007601c0:     	stur	x8, [x29, #-0x80]
1007601c4:     	ldur	q0, [x29, #-0x90]
1007601c8:     	str	x8, [sp, #0xb0]
1007601cc:     	str	q0, [sp, #0xa0]
1007601d0:     	ldur	q0, [x26, #0x18]
1007601d4:     	stur	q0, [x29, #-0x70]
1007601d8:     	ldur	x8, [x26, #0x28]
1007601dc:     	stur	x8, [x29, #-0x60]
1007601e0:     	ldr	x1, [x25, #0x58]
1007601e4:     	cmp	x1, x22
1007601e8:     	b.ls	0x1007603a4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbfc>
1007601ec:     	ldr	x8, [x25, #0x50]
1007601f0:     	ldr	w8, [x8, x22, lsl #2]
1007601f4:     	ldur	w9, [x29, #-0x60]
1007601f8:     	ldr	x10, [x27, #0x30]
1007601fc:     	lsr	x0, x9, #1
100760200:     	cmn	x10, #0x1
100760204:     	b.eq	0x100760220 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa78>
100760208:     	ldr	x1, [x27, #0x40]
10076020c:     	cmp	x1, x0
100760210:     	b.ls	0x100760398 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf0>
100760214:     	ldr	x9, [x27, #0x38]
100760218:     	add	x9, x9, x0, lsl #4
10076021c:     	b	0x100760238 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa90>
100760220:     	ldr	x1, [x27, #0x48]
100760224:     	cmp	x1, x0
100760228:     	b.ls	0x1007603b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc10>
10076022c:     	ldr	x9, [x27, #0x40]
100760230:     	add	x9, x9, x0, lsl #5
100760234:     	add	x9, x9, #0x18
100760238:     	and	w19, w22, #0x3f
10076023c:     	ldr	x9, [x9]
100760240:     	mov	w10, #0x1               ; =1
100760244:     	lsl	x8, x10, x8
100760248:     	tst	x9, x8
10076024c:     	b.eq	0x100760260 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xab8>
100760250:     	ldur	q0, [x29, #-0x70]
100760254:     	dup.2d	v1, x8
100760258:     	orr.16b	v0, v0, v1
10076025c:     	stur	q0, [x29, #-0x70]
100760260:     	sub	x0, x29, #0xb0
100760264:     	sub	x1, x29, #0x70
100760268:     	mov	x2, x27
10076026c:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100760270:     	ldur	q0, [x29, #-0xb0]
100760274:     	stur	q0, [x29, #-0xd0]
100760278:     	ldur	x8, [x29, #-0xa0]
10076027c:     	str	q0, [sp, #0xd0]
100760280:     	stur	q0, [x29, #-0x90]
100760284:     	stur	x8, [x29, #-0x80]
100760288:     	ldur	q0, [x29, #-0x90]
10076028c:     	str	x8, [sp, #0xc8]
100760290:     	stur	q0, [sp, #0xb8]
100760294:     	ldp	q0, q1, [sp, #0xa0]
100760298:     	ldr	q2, [sp, #0xc0]
10076029c:     	stp	q1, q2, [sp, #0x80]
1007602a0:     	str	q0, [sp, #0x70]
1007602a4:     	add	x2, sp, #0x70
1007602a8:     	mov	x0, x25
1007602ac:     	mov	x1, x27
1007602b0:     	bl	0x10075f7a8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1007602b4:     	mov	x3, x0
1007602b8:     	ldr	x8, [x25, #0x80]
1007602bc:     	mov	x0, x27
1007602c0:     	lsr	x8, x8, x19
1007602c4:     	tbz	w8, #0x0, 0x1007602d8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb30>
1007602c8:     	mov	w1, #0xe                ; =14
1007602cc:     	mov	x2, x23
1007602d0:     	bl	0x100ca9a00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1007602d4:     	b	0x1007602e4 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb3c>
1007602d8:     	mov	x1, x22
1007602dc:     	mov	x2, x23
1007602e0:     	bl	0x100caa44c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
1007602e4:     	mov	x19, x0
1007602e8:     	add	x0, x25, #0x60
1007602ec:     	mov	x1, x26
1007602f0:     	mov	x2, x19
1007602f4:     	bl	0x100d3e1bc <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
1007602f8:     	mov	x0, x19
1007602fc:     	b	0x100760100 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x958>
100760300:     	mov	x0, x16
100760304:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100760308:     	mov	w0, #0x0                ; =0
10076030c:     	b	0x100760100 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x958>
100760310:     	adrp	x0, 0x10145b000 <dyld_stub_binder+0x10145b000>
100760314:     	add	x0, x0, #0x932
100760318:     	adrp	x2, 0x1015f9000 <dyld_stub_binder+0x1015f9000>
10076031c:     	add	x2, x2, #0xff8
100760320:     	mov	w1, #0x51               ; =81
100760324:     	bl	0x1013b4bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100760328:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
10076032c:     	add	x2, x2, #0xd30
100760330:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100760334:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100760338:     	add	x2, x2, #0xd30
10076033c:     	mov	x1, x8
100760340:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100760344:     	mov	x1, x12
100760348:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
10076034c:     	add	x2, x2, #0x6a8
100760350:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100760354:     	b	0x1007603d0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc28>
100760358:     	str	x16, [sp, #0x38]
10076035c:     	b	0x10076036c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbc4>
100760360:     	str	x16, [sp, #0x38]
100760364:     	mov	x1, x8
100760368:     	mov	x2, x12
10076036c:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100760370:     	b	0x1007603d0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc28>
100760374:     	mov	w0, #0x8                ; =8
100760378:     	mov	x1, x24
10076037c:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100760380:     	b	0x1007603d0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc28>
100760384:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
100760388:     	add	x2, x2, #0x6c0
10076038c:     	mov	x0, x22
100760390:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100760394:     	mov	x0, x8
100760398:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
10076039c:     	add	x2, x2, #0x40
1007603a0:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007603a4:     	adrp	x2, 0x101601000 <dyld_stub_binder+0x101601000>
1007603a8:     	add	x2, x2, #0x6d8
1007603ac:     	mov	x0, x22
1007603b0:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007603b4:     	mov	x0, x8
1007603b8:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
1007603bc:     	add	x2, x2, #0x28
1007603c0:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007603c4:     	mov	w0, #0x8                ; =8
1007603c8:     	mov	x1, x24
1007603cc:     	bl	0x1013b4564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007603d0:     	brk	#0x1
1007603d4:     	mov	x19, x0
1007603d8:     	ldr	x20, [sp, #0xa0]
1007603dc:     	b	0x10076048c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xce4>
1007603e0:     	mov	x19, x0
1007603e4:     	b	0x10076049c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcf4>
1007603e8:     	mov	x19, x0
1007603ec:     	ldr	x8, [sp, #0xa0]
1007603f0:     	cbz	x8, 0x1007604b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd10>
1007603f4:     	ldr	x8, [sp, #0xa8]
1007603f8:     	b	0x1007604ac <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd04>
1007603fc:     	mov	x19, x0
100760400:     	cbz	x23, 0x100760444 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc9c>
100760404:     	mov	x0, x28
100760408:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10076040c:     	b	0x100760444 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc9c>
100760410:     	str	x21, [sp, #0x38]
100760414:     	mov	x21, x24
100760418:     	mov	x19, x0
10076041c:     	ldr	x8, [sp, #0xa0]
100760420:     	cbz	x8, 0x10076045c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcb4>
100760424:     	ldr	x8, [sp, #0xa8]
100760428:     	str	x8, [sp, #0x8]
10076042c:     	b	0x100760464 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcbc>
100760430:     	mov	x21, x24
100760434:     	mov	x19, x0
100760438:     	b	0x100760454 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcac>
10076043c:     	mov	x21, x24
100760440:     	mov	x19, x0
100760444:     	ldr	x8, [sp, #0x70]
100760448:     	cbz	x8, 0x100760454 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcac>
10076044c:     	ldr	x0, [sp, #0x78]
100760450:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100760454:     	ldr	x8, [sp, #0x20]
100760458:     	cbnz	x8, 0x100760464 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcbc>
10076045c:     	cbnz	x21, 0x1007604b0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd08>
100760460:     	b	0x1007604b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd10>
100760464:     	ldr	x0, [sp, #0x8]
100760468:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10076046c:     	cbnz	x21, 0x1007604b0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd08>
100760470:     	b	0x1007604b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd10>
100760474:     	mov	x19, x0
100760478:     	tbz	w21, #0x0, 0x1007604b0 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd08>
10076047c:     	b	0x1007604b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd10>
100760480:     	mov	x19, x0
100760484:     	mov	x0, x21
100760488:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10076048c:     	cmp	x20, #0x1
100760490:     	b.lt	0x10076049c <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcf4>
100760494:     	ldr	x0, [sp, #0xa8]
100760498:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
10076049c:     	ldr	x8, [sp, #0x70]
1007604a0:     	cmp	x8, #0x1
1007604a4:     	b.lt	0x1007604b8 <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd10>
1007604a8:     	ldr	x8, [sp, #0x78]
1007604ac:     	str	x8, [sp, #0x38]
1007604b0:     	ldr	x0, [sp, #0x38]
1007604b4:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
1007604b8:     	mov	x0, x19
1007604bc:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
