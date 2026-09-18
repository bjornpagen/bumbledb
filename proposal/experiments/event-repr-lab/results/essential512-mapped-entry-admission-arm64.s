
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010ae720 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>:
1010ae720:     	stp	x28, x27, [sp, #-0x60]!
1010ae724:     	stp	x26, x25, [sp, #0x10]
1010ae728:     	stp	x24, x23, [sp, #0x20]
1010ae72c:     	stp	x22, x21, [sp, #0x30]
1010ae730:     	stp	x20, x19, [sp, #0x40]
1010ae734:     	stp	x29, x30, [sp, #0x50]
1010ae738:     	add	x29, sp, #0x50
1010ae73c:     	sub	sp, sp, #0x260
1010ae740:     	ldr	w8, [x1, #0xf0]
1010ae744:     	str	x4, [sp, #0x160]
1010ae748:     	str	x8, [sp]
1010ae74c:     	cmp	x4, x8
1010ae750:     	b.ne	0x1010aead0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
1010ae754:     	mov	x25, x6
1010ae758:     	mov	x21, x5
1010ae75c:     	mov	x23, x4
1010ae760:     	mov	x22, x2
1010ae764:     	mov	x20, x1
1010ae768:     	mov	x19, x0
1010ae76c:     	ldp	x24, x10, [x29, #0x18]
1010ae770:     	cbz	x4, 0x1010ae834 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x114>
1010ae774:     	mov	x11, #0x0               ; =0
1010ae778:     	lsl	x9, x23, #2
1010ae77c:     	mov	w12, #0x1               ; =1
1010ae780:     	mov	x13, x9
1010ae784:     	mov	x14, x3
1010ae788:     	ldr	w15, [x14], #0x4
1010ae78c:     	cmp	w15, w8
1010ae790:     	b.hs	0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae794:     	lsr	x16, x11, x15
1010ae798:     	tbnz	w16, #0x0, 0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae79c:     	lsl	x15, x12, x15
1010ae7a0:     	orr	x11, x15, x11
1010ae7a4:     	subs	x13, x13, #0x4
1010ae7a8:     	b.ne	0x1010ae788 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x68>
1010ae7ac:     	str	x7, [sp, #0x160]
1010ae7b0:     	str	x23, [sp]
1010ae7b4:     	cmp	x7, x23
1010ae7b8:     	b.ne	0x1010aead0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
1010ae7bc:     	mov	x11, #0x0               ; =0
1010ae7c0:     	mov	w12, #0x1               ; =1
1010ae7c4:     	mov	x13, x9
1010ae7c8:     	mov	x14, x25
1010ae7cc:     	ldr	w15, [x14], #0x4
1010ae7d0:     	cmp	w15, w8
1010ae7d4:     	b.hs	0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae7d8:     	lsr	x16, x11, x15
1010ae7dc:     	tbnz	w16, #0x0, 0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae7e0:     	lsl	x15, x12, x15
1010ae7e4:     	orr	x11, x15, x11
1010ae7e8:     	subs	x13, x13, #0x4
1010ae7ec:     	b.ne	0x1010ae7cc <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xac>
1010ae7f0:     	str	x10, [sp, #0x160]
1010ae7f4:     	str	x23, [sp]
1010ae7f8:     	cmp	x10, x23
1010ae7fc:     	b.ne	0x1010aead0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
1010ae800:     	mov	x10, #0x0               ; =0
1010ae804:     	mov	w11, #0x1               ; =1
1010ae808:     	mov	x12, x24
1010ae80c:     	ldr	w13, [x12], #0x4
1010ae810:     	cmp	w13, w8
1010ae814:     	b.hs	0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae818:     	lsr	x14, x10, x13
1010ae81c:     	tbnz	w14, #0x0, 0x1010aeab8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x398>
1010ae820:     	lsl	x13, x11, x13
1010ae824:     	orr	x10, x13, x10
1010ae828:     	subs	x9, x9, #0x4
1010ae82c:     	b.ne	0x1010ae80c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0xec>
1010ae830:     	b	0x1010ae84c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x12c>
1010ae834:     	str	x7, [sp, #0x160]
1010ae838:     	str	xzr, [sp]
1010ae83c:     	cbnz	x7, 0x1010aead0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
1010ae840:     	str	x10, [sp, #0x160]
1010ae844:     	str	xzr, [sp]
1010ae848:     	cbnz	x10, 0x1010aead0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3b0>
1010ae84c:     	ldr	x26, [x29, #0x10]
1010ae850:     	lsr	x8, x26, x8
1010ae854:     	str	x8, [sp]
1010ae858:     	cbnz	x8, 0x1010aeaf4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3d4>
1010ae85c:     	sub	x0, x29, #0xf0
1010ae860:     	mov	x1, x3
1010ae864:     	mov	x2, x23
1010ae868:     	mov	x3, x24
1010ae86c:     	mov	x4, x23
1010ae870:     	bl	0x10108c338 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
1010ae874:     	mov	x0, sp
1010ae878:     	mov	x1, x25
1010ae87c:     	mov	x2, x23
1010ae880:     	mov	x3, x24
1010ae884:     	mov	x4, x23
1010ae888:     	bl	0x10108c338 <__RNvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
1010ae88c:     	ldp	q0, q1, [x29, #-0xf0]
1010ae890:     	stp	q0, q1, [sp, #0x160]
1010ae894:     	ldur	q0, [x29, #-0xd0]
1010ae898:     	ldp	q1, q2, [sp]
1010ae89c:     	stp	q0, q1, [sp, #0x180]
1010ae8a0:     	ldr	q0, [sp, #0x20]
1010ae8a4:     	stp	q2, q0, [sp, #0x1a0]
1010ae8a8:     	mov	w8, #0x4                ; =4
1010ae8ac:     	stp	xzr, x8, [sp]
1010ae8b0:     	str	xzr, [sp, #0x10]
1010ae8b4:     	cbz	x26, 0x1010ae93c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x21c>
1010ae8b8:     	mov	x27, #0x0               ; =0
1010ae8bc:     	mov	w8, #0x4                ; =4
1010ae8c0:     	b	0x1010ae8e8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1c8>
1010ae8c4:     	ldr	x8, [sp, #0x8]
1010ae8c8:     	rbit	x9, x26
1010ae8cc:     	clz	x9, x9
1010ae8d0:     	str	w9, [x8, x27, lsl #2]
1010ae8d4:     	add	x27, x27, #0x1
1010ae8d8:     	str	x27, [sp, #0x10]
1010ae8dc:     	sub	x9, x26, #0x1
1010ae8e0:     	ands	x26, x9, x26
1010ae8e4:     	b.eq	0x1010ae900 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1e0>
1010ae8e8:     	ldr	x9, [sp]
1010ae8ec:     	cmp	x27, x9
1010ae8f0:     	b.ne	0x1010ae8c8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a8>
1010ae8f4:     	mov	x0, sp
1010ae8f8:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1010ae8fc:     	b	0x1010ae8c4 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1a4>
1010ae900:     	ldp	x26, x25, [sp]
1010ae904:     	cbz	x27, 0x1010ae948 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x228>
1010ae908:     	mov	x9, #0x0                ; =0
1010ae90c:     	mov	x8, #0x0                ; =0
1010ae910:     	mov	w10, #0x1               ; =1
1010ae914:     	ldr	w0, [x25, x9, lsl #2]
1010ae918:     	cmp	x23, x0
1010ae91c:     	b.ls	0x1010aeb1c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x3fc>
1010ae920:     	ldr	w11, [x24, x0, lsl #2]
1010ae924:     	lsl	x11, x10, x11
1010ae928:     	orr	x8, x11, x8
1010ae92c:     	add	x9, x9, #0x1
1010ae930:     	cmp	x27, x9
1010ae934:     	b.ne	0x1010ae914 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x1f4>
1010ae938:     	b	0x1010ae94c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
1010ae93c:     	mov	x8, #0x0                ; =0
1010ae940:     	mov	w25, #0x4               ; =4
1010ae944:     	b	0x1010ae94c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x22c>
1010ae948:     	mov	x8, #0x0                ; =0
1010ae94c:     	mov	x9, sp
1010ae950:     	add	x23, x9, #0xc8
1010ae954:     	movi.2d	v0, #0000000000000000
1010ae958:     	stp	q0, q0, [x23, #0x60]
1010ae95c:     	mov	x9, sp
1010ae960:     	stp	q0, q0, [x23, #0x40]
1010ae964:     	stur	q0, [x9, #0xf8]
1010ae968:     	stur	q0, [x9, #0xe8]
1010ae96c:     	stur	q0, [x9, #0xd8]
1010ae970:     	stur	q0, [x9, #0xc8]
1010ae974:     	ldrb	w9, [x20, #0xf6]
1010ae978:     	ldrb	w10, [x20, #0xf7]
1010ae97c:     	stp	xzr, xzr, [sp, #0xb0]
1010ae980:     	ldp	q0, q1, [sp, #0x160]
1010ae984:     	ldp	q2, q3, [sp, #0x180]
1010ae988:     	stp	q1, q2, [sp, #0x20]
1010ae98c:     	ldp	q1, q2, [sp, #0x1a0]
1010ae990:     	stp	q1, q2, [sp, #0x50]
1010ae994:     	str	q3, [sp, #0x40]
1010ae998:     	str	xzr, [sp, #0x148]
1010ae99c:     	str	x8, [sp, #0xc0]
1010ae9a0:     	adrp	x8, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1010ae9a4:     	add	x8, x8, #0xab0
1010ae9a8:     	stp	x8, xzr, [sp, #0x70]
1010ae9ac:     	stp	xzr, xzr, [sp, #0x80]
1010ae9b0:     	strb	w9, [sp, #0x150]
1010ae9b4:     	ldr	q1, [x20]
1010ae9b8:     	stp	q1, q0, [sp]
1010ae9bc:     	strb	w10, [sp, #0x151]
1010ae9c0:     	mov	w8, #0x8                ; =8
1010ae9c4:     	stp	x8, xzr, [sp, #0x90]
1010ae9c8:     	stp	xzr, x8, [sp, #0xa0]
1010ae9cc:     	cbz	x26, 0x1010ae9d8 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x2b8>
1010ae9d0:     	mov	x0, x25
1010ae9d4:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010ae9d8:     	stur	w22, [x29, #-0xe0]
1010ae9dc:     	stp	xzr, xzr, [x29, #-0xf0]
1010ae9e0:     	sub	x0, x29, #0x88
1010ae9e4:     	sub	x1, x29, #0xf0
1010ae9e8:     	mov	x2, x20
1010ae9ec:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1010ae9f0:     	add	x22, sp, #0x160
1010ae9f4:     	ldur	x8, [x29, #-0x78]
1010ae9f8:     	ldur	q0, [x22, #0xc8]
1010ae9fc:     	stur	q0, [x29, #-0x70]
1010aea00:     	stur	x8, [x29, #-0x60]
1010aea04:     	str	x8, [sp, #0x170]
1010aea08:     	str	q0, [sp, #0x160]
1010aea0c:     	stur	w21, [x29, #-0xe0]
1010aea10:     	stp	xzr, xzr, [x29, #-0xf0]
1010aea14:     	sub	x0, x29, #0x88
1010aea18:     	sub	x1, x29, #0xf0
1010aea1c:     	mov	x2, x20
1010aea20:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1010aea24:     	ldur	x8, [x29, #-0x78]
1010aea28:     	ldur	q0, [x22, #0xc8]
1010aea2c:     	stur	q0, [x22, #0x18]
1010aea30:     	str	x8, [sp, #0x188]
1010aea34:     	ldp	q0, q1, [sp, #0x160]
1010aea38:     	ldr	q2, [sp, #0x180]
1010aea3c:     	stp	q1, q2, [x29, #-0xe0]
1010aea40:     	stur	q0, [x29, #-0xf0]
1010aea44:     	mov	x0, sp
1010aea48:     	sub	x2, x29, #0xf0
1010aea4c:     	mov	x1, x20
1010aea50:     	bl	0x10099570c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1010aea54:     	ldp	q0, q1, [x23, #0x40]
1010aea58:     	stur	q0, [x19, #0x48]
1010aea5c:     	stur	q1, [x19, #0x58]
1010aea60:     	ldp	q0, q1, [x23, #0x60]
1010aea64:     	stur	q0, [x19, #0x68]
1010aea68:     	stur	q1, [x19, #0x78]
1010aea6c:     	ldp	q0, q1, [x23]
1010aea70:     	stur	q0, [x19, #0x8]
1010aea74:     	stur	q1, [x19, #0x18]
1010aea78:     	ldp	q0, q1, [x23, #0x20]
1010aea7c:     	stur	q0, [x19, #0x28]
1010aea80:     	ldr	x8, [x23, #0x80]
1010aea84:     	str	x8, [x19, #0x88]
1010aea88:     	stur	q1, [x19, #0x38]
1010aea8c:     	str	w0, [x19]
1010aea90:     	mov	x0, sp
1010aea94:     	bl	0x100a2043c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
1010aea98:     	add	sp, sp, #0x260
1010aea9c:     	ldp	x29, x30, [sp, #0x50]
1010aeaa0:     	ldp	x20, x19, [sp, #0x40]
1010aeaa4:     	ldp	x22, x21, [sp, #0x30]
1010aeaa8:     	ldp	x24, x23, [sp, #0x20]
1010aeaac:     	ldp	x26, x25, [sp, #0x10]
1010aeab0:     	ldp	x28, x27, [sp], #0x60
1010aeab4:     	ret
1010aeab8:     	adrp	x0, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010aeabc:     	add	x0, x0, #0x4ef
1010aeac0:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010aeac4:     	add	x2, x2, #0x868
1010aeac8:     	mov	w1, #0x2f               ; =47
1010aeacc:     	bl	0x1016e73bc <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1010aead0:     	adrp	x3, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010aead4:     	add	x3, x3, #0x4d9
1010aead8:     	adrp	x5, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010aeadc:     	add	x5, x5, #0x850
1010aeae0:     	add	x1, sp, #0x160
1010aeae4:     	mov	x2, sp
1010aeae8:     	mov	w0, #0x0                ; =0
1010aeaec:     	mov	w4, #0x2d               ; =45
1010aeaf0:     	bl	0x1016e73f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1010aeaf4:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1010aeaf8:     	add	x2, x2, #0x358
1010aeafc:     	adrp	x3, 0x1017bb000 <dyld_stub_binder+0x1017bb000>
1010aeb00:     	add	x3, x3, #0x804
1010aeb04:     	adrp	x5, 0x10198d000 <dyld_stub_binder+0x10198d000>
1010aeb08:     	add	x5, x5, #0xed8
1010aeb0c:     	mov	x1, sp
1010aeb10:     	mov	w0, #0x0                ; =0
1010aeb14:     	mov	w4, #0x57               ; =87
1010aeb18:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1010aeb1c:     	adrp	x2, 0x101950000 <dyld_stub_binder+0x101950000>
1010aeb20:     	add	x2, x2, #0x178
1010aeb24:     	mov	x1, x23
1010aeb28:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010aeb2c:     	brk	#0x1
1010aeb30:     	mov	x19, x0
1010aeb34:     	sub	x0, x29, #0xf0
1010aeb38:     	bl	0x100a1ef24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
1010aeb3c:     	mov	x0, x19
1010aeb40:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010aeb44:     	mov	x19, x0
1010aeb48:     	mov	x0, sp
1010aeb4c:     	bl	0x100a2043c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
1010aeb50:     	mov	x0, x19
1010aeb54:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010aeb58:     	mov	x19, x0
1010aeb5c:     	ldr	x8, [sp]
1010aeb60:     	cbz	x8, 0x1010aeb6c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x44c>
1010aeb64:     	ldr	x0, [sp, #0x8]
1010aeb68:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010aeb6c:     	add	x0, sp, #0x160
1010aeb70:     	bl	0x100a024ac <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
1010aeb74:     	mov	x0, x19
1010aeb78:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010aeb7c:     	mov	x19, x0
1010aeb80:     	add	x0, sp, #0x160
1010aeb84:     	bl	0x100a024ac <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
1010aeb88:     	cbnz	x26, 0x1010aeb94 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_+0x474>
1010aeb8c:     	mov	x0, x19
1010aeb90:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
1010aeb94:     	mov	x0, x25
1010aeb98:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1010aeb9c:     	mov	x0, x19
1010aeba0:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
