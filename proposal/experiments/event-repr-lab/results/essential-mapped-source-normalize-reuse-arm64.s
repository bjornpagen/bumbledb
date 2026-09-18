
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010074f7b4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_>:
10074f7b4:     	sub	sp, sp, #0x90
10074f7b8:     	stp	x28, x27, [sp, #0x30]
10074f7bc:     	stp	x26, x25, [sp, #0x40]
10074f7c0:     	stp	x24, x23, [sp, #0x50]
10074f7c4:     	stp	x22, x21, [sp, #0x60]
10074f7c8:     	stp	x20, x19, [sp, #0x70]
10074f7cc:     	stp	x29, x30, [sp, #0x80]
10074f7d0:     	add	x29, sp, #0x80
10074f7d4:     	mov	x24, x1
10074f7d8:     	mov	x19, x0
10074f7dc:     	ldr	x23, [x1]
10074f7e0:     	cbz	x23, 0x10074f824 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x70>
10074f7e4:     	mov	x20, x3
10074f7e8:     	mov	x22, x2
10074f7ec:     	ldr	x8, [x3, #0x70]
10074f7f0:     	add	x8, x8, #0x1
10074f7f4:     	str	x8, [x3, #0x70]
10074f7f8:     	ldr	w21, [x24, #0x10]
10074f7fc:     	ldr	x8, [x2, #0x40]
10074f800:     	lsr	x0, x21, #1
10074f804:     	cmn	x8, #0x1
10074f808:     	b.eq	0x10074f838 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x84>
10074f80c:     	ldr	x1, [x22, #0x50]
10074f810:     	cmp	x1, x0
10074f814:     	b.ls	0x10074fa60 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
10074f818:     	ldr	x8, [x22, #0x48]
10074f81c:     	add	x8, x8, x0, lsl #4
10074f820:     	b	0x10074f850 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x9c>
10074f824:     	ldr	q0, [x24]
10074f828:     	str	q0, [x19]
10074f82c:     	ldr	x8, [x24, #0x10]
10074f830:     	str	x8, [x19, #0x10]
10074f834:     	b	0x10074f9d8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x224>
10074f838:     	ldr	x1, [x22, #0x58]
10074f83c:     	cmp	x1, x0
10074f840:     	b.ls	0x10074fa6c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
10074f844:     	ldr	x8, [x22, #0x50]
10074f848:     	add	x8, x8, x0, lsl #5
10074f84c:     	add	x8, x8, #0x18
10074f850:     	mov	x26, #0x0               ; =0
10074f854:     	ldr	x8, [x8]
10074f858:     	bic	x25, x8, x23
10074f85c:     	mov	w8, #0x4                ; =4
10074f860:     	stp	xzr, x8, [sp, #0x18]
10074f864:     	str	xzr, [sp, #0x28]
10074f868:     	mov	w9, #0x1                ; =1
10074f86c:     	b	0x10074f894 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xe0>
10074f870:     	rbit	x9, x23
10074f874:     	clz	x9, x9
10074f878:     	str	w9, [x8, x26]
10074f87c:     	str	x28, [sp, #0x28]
10074f880:     	sub	x10, x23, #0x1
10074f884:     	add	x26, x26, #0x4
10074f888:     	add	x9, x28, #0x1
10074f88c:     	ands	x23, x10, x23
10074f890:     	b.eq	0x10074f8b8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x104>
10074f894:     	mov	x28, x9
10074f898:     	sub	x9, x9, #0x1
10074f89c:     	ldr	x10, [sp, #0x18]
10074f8a0:     	cmp	x9, x10
10074f8a4:     	b.ne	0x10074f870 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
10074f8a8:     	add	x0, sp, #0x18
10074f8ac:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10074f8b0:     	ldr	x8, [sp, #0x20]
10074f8b4:     	b	0x10074f870 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
10074f8b8:     	ldp	x8, x27, [sp, #0x18]
10074f8bc:     	str	x8, [sp, #0x10]
10074f8c0:     	cbz	x28, 0x10074f96c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
10074f8c4:     	ldr	x28, [x24, #0x8]
10074f8c8:     	ldr	x24, [x20, #0x78]
10074f8cc:     	mov	x23, x27
10074f8d0:     	adrp	x8, 0x101645000 <dyld_stub_binder+0x101645000>
10074f8d4:     	add	x8, x8, #0x2e0
10074f8d8:     	str	x8, [sp, #0x8]
10074f8dc:     	b	0x10074f8e8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x134>
10074f8e0:     	subs	x26, x26, #0x4
10074f8e4:     	b.eq	0x10074f96c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
10074f8e8:     	ldr	w2, [x23], #0x4
10074f8ec:     	ldr	x8, [x22, #0x40]
10074f8f0:     	lsr	w0, w21, #1
10074f8f4:     	cmn	x8, #0x1
10074f8f8:     	b.eq	0x10074f920 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x16c>
10074f8fc:     	ldr	x1, [x22, #0x50]
10074f900:     	cmp	x1, x0
10074f904:     	b.ls	0x10074fa54 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2a0>
10074f908:     	ldr	x8, [x22, #0x48]
10074f90c:     	add	x8, x8, x0, lsl #4
10074f910:     	ldr	x8, [x8]
10074f914:     	lsr	x8, x8, x2
10074f918:     	tbnz	w8, #0x0, 0x10074f944 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x190>
10074f91c:     	b	0x10074f8e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
10074f920:     	ldr	x1, [x22, #0x58]
10074f924:     	cmp	x1, x0
10074f928:     	b.ls	0x10074fa48 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x294>
10074f92c:     	ldr	x8, [x22, #0x50]
10074f930:     	add	x8, x8, x0, lsl #5
10074f934:     	add	x8, x8, #0x18
10074f938:     	ldr	x8, [x8]
10074f93c:     	lsr	x8, x8, x2
10074f940:     	tbz	w8, #0x0, 0x10074f8e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
10074f944:     	and	x8, x2, #0x3f
10074f948:     	lsr	x8, x28, x8
10074f94c:     	and	w3, w8, #0x1
10074f950:     	mov	x0, x22
10074f954:     	mov	x1, x21
10074f958:     	bl	0x100ca02d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8cofactorB6_>
10074f95c:     	mov	x21, x0
10074f960:     	add	x24, x24, #0x1
10074f964:     	str	x24, [x20, #0x78]
10074f968:     	b	0x10074f8e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
10074f96c:     	ldr	x8, [sp, #0x10]
10074f970:     	cbz	x8, 0x10074f97c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1c8>
10074f974:     	mov	x0, x27
10074f978:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
10074f97c:     	ldr	x8, [x22, #0x40]
10074f980:     	lsr	w0, w21, #1
10074f984:     	cmn	x8, #0x1
10074f988:     	b.eq	0x10074f9f8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x244>
10074f98c:     	ldr	x1, [x22, #0x50]
10074f990:     	cmp	x1, x0
10074f994:     	b.ls	0x10074fa60 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
10074f998:     	ldr	x8, [x22, #0x48]
10074f99c:     	add	x8, x8, x0, lsl #4
10074f9a0:     	ldr	x8, [x8]
10074f9a4:     	bics	x9, x8, x25
10074f9a8:     	str	x9, [sp, #0x18]
10074f9ac:     	b.ne	0x10074fa20 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x26c>
10074f9b0:     	bic	x8, x25, x8
10074f9b4:     	fmov	d0, x8
10074f9b8:     	cnt.8b	v0, v0
10074f9bc:     	addv.8b	b0, v0
10074f9c0:     	ldr	x8, [x20, #0x80]
10074f9c4:     	fmov	x9, d0
10074f9c8:     	add	x8, x8, x9
10074f9cc:     	str	x8, [x20, #0x80]
10074f9d0:     	str	w21, [x19, #0x10]
10074f9d4:     	stp	xzr, xzr, [x19]
10074f9d8:     	ldp	x29, x30, [sp, #0x80]
10074f9dc:     	ldp	x20, x19, [sp, #0x70]
10074f9e0:     	ldp	x22, x21, [sp, #0x60]
10074f9e4:     	ldp	x24, x23, [sp, #0x50]
10074f9e8:     	ldp	x26, x25, [sp, #0x40]
10074f9ec:     	ldp	x28, x27, [sp, #0x30]
10074f9f0:     	add	sp, sp, #0x90
10074f9f4:     	ret
10074f9f8:     	ldr	x1, [x22, #0x58]
10074f9fc:     	cmp	x1, x0
10074fa00:     	b.ls	0x10074fa6c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
10074fa04:     	ldr	x8, [x22, #0x50]
10074fa08:     	add	x8, x8, x0, lsl #5
10074fa0c:     	add	x8, x8, #0x18
10074fa10:     	ldr	x8, [x8]
10074fa14:     	bics	x9, x8, x25
10074fa18:     	str	x9, [sp, #0x18]
10074fa1c:     	b.eq	0x10074f9b0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1fc>
10074fa20:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
10074fa24:     	add	x2, x2, #0x78
10074fa28:     	adrp	x3, 0x101461000 <dyld_stub_binder+0x101461000>
10074fa2c:     	add	x3, x3, #0x185
10074fa30:     	adrp	x5, 0x1015fd000 <dyld_stub_binder+0x1015fd000>
10074fa34:     	add	x5, x5, #0xed8
10074fa38:     	add	x1, sp, #0x18
10074fa3c:     	mov	w0, #0x0                ; =0
10074fa40:     	mov	w4, #0x43               ; =67
10074fa44:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
10074fa48:     	adrp	x8, 0x101645000 <dyld_stub_binder+0x101645000>
10074fa4c:     	add	x8, x8, #0x2c8
10074fa50:     	str	x8, [sp, #0x8]
10074fa54:     	ldr	x2, [sp, #0x8]
10074fa58:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10074fa5c:     	brk	#0x1
10074fa60:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
10074fa64:     	add	x2, x2, #0x2e0
10074fa68:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10074fa6c:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
10074fa70:     	add	x2, x2, #0x2c8
10074fa74:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10074fa78:     	b	0x10074fa90 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2dc>
10074fa7c:     	mov	x19, x0
10074fa80:     	ldr	x8, [sp, #0x18]
10074fa84:     	cbz	x8, 0x10074faa4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
10074fa88:     	ldr	x27, [sp, #0x20]
10074fa8c:     	b	0x10074fa9c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2e8>
10074fa90:     	mov	x19, x0
10074fa94:     	ldr	x8, [sp, #0x10]
10074fa98:     	cbz	x8, 0x10074faa4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
10074fa9c:     	mov	x0, x27
10074faa0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
10074faa4:     	mov	x0, x19
10074faa8:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
