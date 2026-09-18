
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100edf720 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>:
100edf720:     	stp	x28, x27, [sp, #-0x60]!
100edf724:     	stp	x26, x25, [sp, #0x10]
100edf728:     	stp	x24, x23, [sp, #0x20]
100edf72c:     	stp	x22, x21, [sp, #0x30]
100edf730:     	stp	x20, x19, [sp, #0x40]
100edf734:     	stp	x29, x30, [sp, #0x50]
100edf738:     	add	x29, sp, #0x50
100edf73c:     	sub	sp, sp, #0x260
100edf740:     	ldr	w8, [x1, #0xf0]
100edf744:     	str	x4, [sp, #0x160]
100edf748:     	str	x8, [sp]
100edf74c:     	cmp	x4, x8
100edf750:     	b.ne	0x100edfad0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100edf754:     	mov	x25, x6
100edf758:     	mov	x21, x5
100edf75c:     	mov	x23, x4
100edf760:     	mov	x22, x2
100edf764:     	mov	x20, x1
100edf768:     	mov	x19, x0
100edf76c:     	ldp	x24, x10, [x29, #0x18]
100edf770:     	cbz	x4, 0x100edf834 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x114>
100edf774:     	mov	x11, #0x0               ; =0
100edf778:     	lsl	x9, x23, #2
100edf77c:     	mov	w12, #0x1               ; =1
100edf780:     	mov	x13, x9
100edf784:     	mov	x14, x3
100edf788:     	ldr	w15, [x14], #0x4
100edf78c:     	cmp	w15, w8
100edf790:     	b.hs	0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf794:     	lsr	x16, x11, x15
100edf798:     	tbnz	w16, #0x0, 0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf79c:     	lsl	x15, x12, x15
100edf7a0:     	orr	x11, x15, x11
100edf7a4:     	subs	x13, x13, #0x4
100edf7a8:     	b.ne	0x100edf788 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x68>
100edf7ac:     	str	x7, [sp, #0x160]
100edf7b0:     	str	x23, [sp]
100edf7b4:     	cmp	x7, x23
100edf7b8:     	b.ne	0x100edfad0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100edf7bc:     	mov	x11, #0x0               ; =0
100edf7c0:     	mov	w12, #0x1               ; =1
100edf7c4:     	mov	x13, x9
100edf7c8:     	mov	x14, x25
100edf7cc:     	ldr	w15, [x14], #0x4
100edf7d0:     	cmp	w15, w8
100edf7d4:     	b.hs	0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf7d8:     	lsr	x16, x11, x15
100edf7dc:     	tbnz	w16, #0x0, 0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf7e0:     	lsl	x15, x12, x15
100edf7e4:     	orr	x11, x15, x11
100edf7e8:     	subs	x13, x13, #0x4
100edf7ec:     	b.ne	0x100edf7cc <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xac>
100edf7f0:     	str	x10, [sp, #0x160]
100edf7f4:     	str	x23, [sp]
100edf7f8:     	cmp	x10, x23
100edf7fc:     	b.ne	0x100edfad0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100edf800:     	mov	x10, #0x0               ; =0
100edf804:     	mov	w11, #0x1               ; =1
100edf808:     	mov	x12, x24
100edf80c:     	ldr	w13, [x12], #0x4
100edf810:     	cmp	w13, w8
100edf814:     	b.hs	0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf818:     	lsr	x14, x10, x13
100edf81c:     	tbnz	w14, #0x0, 0x100edfab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
100edf820:     	lsl	x13, x11, x13
100edf824:     	orr	x10, x13, x10
100edf828:     	subs	x9, x9, #0x4
100edf82c:     	b.ne	0x100edf80c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xec>
100edf830:     	b	0x100edf84c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x12c>
100edf834:     	str	x7, [sp, #0x160]
100edf838:     	str	xzr, [sp]
100edf83c:     	cbnz	x7, 0x100edfad0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100edf840:     	str	x10, [sp, #0x160]
100edf844:     	str	xzr, [sp]
100edf848:     	cbnz	x10, 0x100edfad0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
100edf84c:     	ldr	x26, [x29, #0x10]
100edf850:     	lsr	x8, x26, x8
100edf854:     	str	x8, [sp]
100edf858:     	cbnz	x8, 0x100edfaf4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d4>
100edf85c:     	sub	x0, x29, #0xf0
100edf860:     	mov	x1, x3
100edf864:     	mov	x2, x23
100edf868:     	mov	x3, x24
100edf86c:     	mov	x4, x23
100edf870:     	bl	0x100ebd0b8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100edf874:     	mov	x0, sp
100edf878:     	mov	x1, x25
100edf87c:     	mov	x2, x23
100edf880:     	mov	x3, x24
100edf884:     	mov	x4, x23
100edf888:     	bl	0x100ebd0b8 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100edf88c:     	ldp	q0, q1, [x29, #-0xf0]
100edf890:     	stp	q0, q1, [sp, #0x160]
100edf894:     	ldur	q0, [x29, #-0xd0]
100edf898:     	ldp	q1, q2, [sp]
100edf89c:     	stp	q0, q1, [sp, #0x180]
100edf8a0:     	ldr	q0, [sp, #0x20]
100edf8a4:     	stp	q2, q0, [sp, #0x1a0]
100edf8a8:     	mov	w8, #0x4                ; =4
100edf8ac:     	stp	xzr, x8, [sp]
100edf8b0:     	str	xzr, [sp, #0x10]
100edf8b4:     	cbz	x26, 0x100edf93c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x21c>
100edf8b8:     	mov	x27, #0x0               ; =0
100edf8bc:     	mov	w8, #0x4                ; =4
100edf8c0:     	b	0x100edf8e8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1c8>
100edf8c4:     	ldr	x8, [sp, #0x8]
100edf8c8:     	rbit	x9, x26
100edf8cc:     	clz	x9, x9
100edf8d0:     	str	w9, [x8, x27, lsl #2]
100edf8d4:     	add	x27, x27, #0x1
100edf8d8:     	str	x27, [sp, #0x10]
100edf8dc:     	sub	x9, x26, #0x1
100edf8e0:     	ands	x26, x9, x26
100edf8e4:     	b.eq	0x100edf900 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1e0>
100edf8e8:     	ldr	x9, [sp]
100edf8ec:     	cmp	x27, x9
100edf8f0:     	b.ne	0x100edf8c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a8>
100edf8f4:     	mov	x0, sp
100edf8f8:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100edf8fc:     	b	0x100edf8c4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a4>
100edf900:     	ldp	x26, x25, [sp]
100edf904:     	cbz	x27, 0x100edf948 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x228>
100edf908:     	mov	x9, #0x0                ; =0
100edf90c:     	mov	x8, #0x0                ; =0
100edf910:     	mov	w10, #0x1               ; =1
100edf914:     	ldr	w0, [x25, x9, lsl #2]
100edf918:     	cmp	x23, x0
100edf91c:     	b.ls	0x100edfb1c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3fc>
100edf920:     	ldr	w11, [x24, x0, lsl #2]
100edf924:     	lsl	x11, x10, x11
100edf928:     	orr	x8, x11, x8
100edf92c:     	add	x9, x9, #0x1
100edf930:     	cmp	x27, x9
100edf934:     	b.ne	0x100edf914 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1f4>
100edf938:     	b	0x100edf94c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100edf93c:     	mov	x8, #0x0                ; =0
100edf940:     	mov	w25, #0x4               ; =4
100edf944:     	b	0x100edf94c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
100edf948:     	mov	x8, #0x0                ; =0
100edf94c:     	mov	x9, sp
100edf950:     	add	x23, x9, #0xc8
100edf954:     	movi.2d	v0, #0000000000000000
100edf958:     	stp	q0, q0, [x23, #0x60]
100edf95c:     	mov	x9, sp
100edf960:     	stp	q0, q0, [x23, #0x40]
100edf964:     	stur	q0, [x9, #0xf8]
100edf968:     	stur	q0, [x9, #0xe8]
100edf96c:     	stur	q0, [x9, #0xd8]
100edf970:     	stur	q0, [x9, #0xc8]
100edf974:     	ldrb	w9, [x20, #0xf6]
100edf978:     	ldrb	w10, [x20, #0xf7]
100edf97c:     	stp	xzr, xzr, [sp, #0xb0]
100edf980:     	ldp	q0, q1, [sp, #0x160]
100edf984:     	ldp	q2, q3, [sp, #0x180]
100edf988:     	stp	q1, q2, [sp, #0x20]
100edf98c:     	ldp	q1, q2, [sp, #0x1a0]
100edf990:     	stp	q1, q2, [sp, #0x50]
100edf994:     	str	q3, [sp, #0x40]
100edf998:     	str	xzr, [sp, #0x148]
100edf99c:     	str	x8, [sp, #0xc0]
100edf9a0:     	adrp	x8, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100edf9a4:     	add	x8, x8, #0x9b8
100edf9a8:     	stp	x8, xzr, [sp, #0x70]
100edf9ac:     	stp	xzr, xzr, [sp, #0x80]
100edf9b0:     	strb	w9, [sp, #0x150]
100edf9b4:     	ldr	q1, [x20]
100edf9b8:     	stp	q1, q0, [sp]
100edf9bc:     	strb	w10, [sp, #0x151]
100edf9c0:     	mov	w8, #0x8                ; =8
100edf9c4:     	stp	x8, xzr, [sp, #0x90]
100edf9c8:     	stp	xzr, x8, [sp, #0xa0]
100edf9cc:     	cbz	x26, 0x100edf9d8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x2b8>
100edf9d0:     	mov	x0, x25
100edf9d4:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edf9d8:     	stur	w22, [x29, #-0xe0]
100edf9dc:     	stp	xzr, xzr, [x29, #-0xf0]
100edf9e0:     	sub	x0, x29, #0x88
100edf9e4:     	sub	x1, x29, #0xf0
100edf9e8:     	mov	x2, x20
100edf9ec:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100edf9f0:     	add	x22, sp, #0x160
100edf9f4:     	ldur	x8, [x29, #-0x78]
100edf9f8:     	ldur	q0, [x22, #0xc8]
100edf9fc:     	stur	q0, [x29, #-0x70]
100edfa00:     	stur	x8, [x29, #-0x60]
100edfa04:     	str	x8, [sp, #0x170]
100edfa08:     	str	q0, [sp, #0x160]
100edfa0c:     	stur	w21, [x29, #-0xe0]
100edfa10:     	stp	xzr, xzr, [x29, #-0xf0]
100edfa14:     	sub	x0, x29, #0x88
100edfa18:     	sub	x1, x29, #0xf0
100edfa1c:     	mov	x2, x20
100edfa20:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100edfa24:     	ldur	x8, [x29, #-0x78]
100edfa28:     	ldur	q0, [x22, #0xc8]
100edfa2c:     	stur	q0, [x22, #0x18]
100edfa30:     	str	x8, [sp, #0x188]
100edfa34:     	ldp	q0, q1, [sp, #0x160]
100edfa38:     	ldr	q2, [sp, #0x180]
100edfa3c:     	stp	q1, q2, [x29, #-0xe0]
100edfa40:     	stur	q0, [x29, #-0xf0]
100edfa44:     	mov	x0, sp
100edfa48:     	sub	x2, x29, #0xf0
100edfa4c:     	mov	x1, x20
100edfa50:     	bl	0x1008a4678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
100edfa54:     	ldp	q0, q1, [x23, #0x40]
100edfa58:     	stur	q0, [x19, #0x48]
100edfa5c:     	stur	q1, [x19, #0x58]
100edfa60:     	ldp	q0, q1, [x23, #0x60]
100edfa64:     	stur	q0, [x19, #0x68]
100edfa68:     	stur	q1, [x19, #0x78]
100edfa6c:     	ldp	q0, q1, [x23]
100edfa70:     	stur	q0, [x19, #0x8]
100edfa74:     	stur	q1, [x19, #0x18]
100edfa78:     	ldp	q0, q1, [x23, #0x20]
100edfa7c:     	stur	q0, [x19, #0x28]
100edfa80:     	ldr	x8, [x23, #0x80]
100edfa84:     	str	x8, [x19, #0x88]
100edfa88:     	stur	q1, [x19, #0x38]
100edfa8c:     	str	w0, [x19]
100edfa90:     	mov	x0, sp
100edfa94:     	bl	0x10091aa64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100edfa98:     	add	sp, sp, #0x260
100edfa9c:     	ldp	x29, x30, [sp, #0x50]
100edfaa0:     	ldp	x20, x19, [sp, #0x40]
100edfaa4:     	ldp	x22, x21, [sp, #0x30]
100edfaa8:     	ldp	x24, x23, [sp, #0x20]
100edfaac:     	ldp	x26, x25, [sp, #0x10]
100edfab0:     	ldp	x28, x27, [sp], #0x60
100edfab4:     	ret
100edfab8:     	adrp	x0, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edfabc:     	add	x0, x0, #0x57f
100edfac0:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
100edfac4:     	add	x2, x2, #0x9a8
100edfac8:     	mov	w1, #0x2f               ; =47
100edfacc:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100edfad0:     	adrp	x3, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edfad4:     	add	x3, x3, #0x569
100edfad8:     	adrp	x5, 0x101798000 <dyld_stub_binder+0x101798000>
100edfadc:     	add	x5, x5, #0x990
100edfae0:     	add	x1, sp, #0x160
100edfae4:     	mov	x2, sp
100edfae8:     	mov	w0, #0x0                ; =0
100edfaec:     	mov	w4, #0x2d               ; =45
100edfaf0:     	bl	0x101506cb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100edfaf4:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
100edfaf8:     	add	x2, x2, #0x268
100edfafc:     	adrp	x3, 0x1015d2000 <dyld_stub_binder+0x1015d2000>
100edfb00:     	add	x3, x3, #0x894
100edfb04:     	adrp	x5, 0x101799000 <dyld_stub_binder+0x101799000>
100edfb08:     	add	x5, x5, #0x18
100edfb0c:     	mov	x1, sp
100edfb10:     	mov	w0, #0x0                ; =0
100edfb14:     	mov	w4, #0x57               ; =87
100edfb18:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100edfb1c:     	adrp	x2, 0x10175b000 <dyld_stub_binder+0x10175b000>
100edfb20:     	add	x2, x2, #0x438
100edfb24:     	mov	x1, x23
100edfb28:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100edfb2c:     	brk	#0x1
100edfb30:     	mov	x19, x0
100edfb34:     	sub	x0, x29, #0xf0
100edfb38:     	bl	0x10091954c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100edfb3c:     	mov	x0, x19
100edfb40:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edfb44:     	mov	x19, x0
100edfb48:     	mov	x0, sp
100edfb4c:     	bl	0x10091aa64 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100edfb50:     	mov	x0, x19
100edfb54:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edfb58:     	mov	x19, x0
100edfb5c:     	ldr	x8, [sp]
100edfb60:     	cbz	x8, 0x100edfb6c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x44c>
100edfb64:     	ldr	x0, [sp, #0x8]
100edfb68:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edfb6c:     	add	x0, sp, #0x160
100edfb70:     	bl	0x1008fdf00 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100edfb74:     	mov	x0, x19
100edfb78:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edfb7c:     	mov	x19, x0
100edfb80:     	add	x0, sp, #0x160
100edfb84:     	bl	0x1008fdf00 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100edfb88:     	cbnz	x26, 0x100edfb94 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x474>
100edfb8c:     	mov	x0, x19
100edfb90:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
100edfb94:     	mov	x0, x25
100edfb98:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100edfb9c:     	mov	x0, x19
100edfba0:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
