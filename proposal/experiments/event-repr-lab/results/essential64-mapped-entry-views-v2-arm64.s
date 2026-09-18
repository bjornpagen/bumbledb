
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d7e638 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>:
100d7e638:     	stp	x28, x27, [sp, #-0x60]!
100d7e63c:     	stp	x26, x25, [sp, #0x10]
100d7e640:     	stp	x24, x23, [sp, #0x20]
100d7e644:     	stp	x22, x21, [sp, #0x30]
100d7e648:     	stp	x20, x19, [sp, #0x40]
100d7e64c:     	stp	x29, x30, [sp, #0x50]
100d7e650:     	add	x29, sp, #0x50
100d7e654:     	sub	sp, sp, #0x1e0
100d7e658:     	ldr	w8, [x1, #0xe0]
100d7e65c:     	str	x4, [sp, #0xe0]
100d7e660:     	str	x8, [sp]
100d7e664:     	cmp	x4, x8
100d7e668:     	b.ne	0x100d7ea10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d8>
100d7e66c:     	mov	x25, x6
100d7e670:     	mov	x21, x5
100d7e674:     	mov	x23, x4
100d7e678:     	mov	x22, x2
100d7e67c:     	mov	x20, x1
100d7e680:     	mov	x19, x0
100d7e684:     	ldp	x24, x10, [x29, #0x18]
100d7e688:     	cbz	x4, 0x100d7e74c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x114>
100d7e68c:     	mov	x11, #0x0               ; =0
100d7e690:     	lsl	x9, x23, #2
100d7e694:     	mov	w12, #0x1               ; =1
100d7e698:     	mov	x13, x9
100d7e69c:     	mov	x14, x3
100d7e6a0:     	ldr	w15, [x14], #0x4
100d7e6a4:     	cmp	w15, w8
100d7e6a8:     	b.hs	0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e6ac:     	lsr	x16, x11, x15
100d7e6b0:     	tbnz	w16, #0x0, 0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e6b4:     	lsl	x15, x12, x15
100d7e6b8:     	orr	x11, x15, x11
100d7e6bc:     	subs	x13, x13, #0x4
100d7e6c0:     	b.ne	0x100d7e6a0 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x68>
100d7e6c4:     	str	x7, [sp, #0xe0]
100d7e6c8:     	str	x23, [sp]
100d7e6cc:     	cmp	x7, x23
100d7e6d0:     	b.ne	0x100d7ea10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d8>
100d7e6d4:     	mov	x11, #0x0               ; =0
100d7e6d8:     	mov	w12, #0x1               ; =1
100d7e6dc:     	mov	x13, x9
100d7e6e0:     	mov	x14, x25
100d7e6e4:     	ldr	w15, [x14], #0x4
100d7e6e8:     	cmp	w15, w8
100d7e6ec:     	b.hs	0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e6f0:     	lsr	x16, x11, x15
100d7e6f4:     	tbnz	w16, #0x0, 0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e6f8:     	lsl	x15, x12, x15
100d7e6fc:     	orr	x11, x15, x11
100d7e700:     	subs	x13, x13, #0x4
100d7e704:     	b.ne	0x100d7e6e4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xac>
100d7e708:     	str	x10, [sp, #0xe0]
100d7e70c:     	str	x23, [sp]
100d7e710:     	cmp	x10, x23
100d7e714:     	b.ne	0x100d7ea10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d8>
100d7e718:     	mov	x10, #0x0               ; =0
100d7e71c:     	mov	w11, #0x1               ; =1
100d7e720:     	mov	x12, x24
100d7e724:     	ldr	w13, [x12], #0x4
100d7e728:     	cmp	w13, w8
100d7e72c:     	b.hs	0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e730:     	lsr	x14, x10, x13
100d7e734:     	tbnz	w14, #0x0, 0x100d7e9f8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3c0>
100d7e738:     	lsl	x13, x11, x13
100d7e73c:     	orr	x10, x13, x10
100d7e740:     	subs	x9, x9, #0x4
100d7e744:     	b.ne	0x100d7e724 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0xec>
100d7e748:     	b	0x100d7e764 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x12c>
100d7e74c:     	str	x7, [sp, #0xe0]
100d7e750:     	str	xzr, [sp]
100d7e754:     	cbnz	x7, 0x100d7ea10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d8>
100d7e758:     	str	x10, [sp, #0xe0]
100d7e75c:     	str	xzr, [sp]
100d7e760:     	cbnz	x10, 0x100d7ea10 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3d8>
100d7e764:     	ldr	x26, [x29, #0x10]
100d7e768:     	lsr	x8, x26, x8
100d7e76c:     	str	x8, [sp]
100d7e770:     	cbnz	x8, 0x100d7ea34 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3fc>
100d7e774:     	sub	x0, x29, #0xf0
100d7e778:     	mov	x1, x3
100d7e77c:     	mov	x2, x23
100d7e780:     	mov	x3, x24
100d7e784:     	mov	x4, x23
100d7e788:     	bl	0x100d33418 <__RNvMs0_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d7e78c:     	mov	x0, sp
100d7e790:     	mov	x1, x25
100d7e794:     	mov	x2, x23
100d7e798:     	mov	x3, x24
100d7e79c:     	mov	x4, x23
100d7e7a0:     	bl	0x100d33418 <__RNvMs0_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_3Map7compose>
100d7e7a4:     	ldp	q0, q1, [x29, #-0xf0]
100d7e7a8:     	stp	q0, q1, [sp, #0xe0]
100d7e7ac:     	ldur	q0, [x29, #-0xd0]
100d7e7b0:     	ldp	q1, q2, [sp]
100d7e7b4:     	stp	q0, q1, [sp, #0x100]
100d7e7b8:     	ldr	q0, [sp, #0x20]
100d7e7bc:     	stp	q2, q0, [sp, #0x120]
100d7e7c0:     	mov	w8, #0x4                ; =4
100d7e7c4:     	stp	xzr, x8, [sp]
100d7e7c8:     	str	xzr, [sp, #0x10]
100d7e7cc:     	cbz	x26, 0x100d7e854 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x21c>
100d7e7d0:     	mov	x27, #0x0               ; =0
100d7e7d4:     	mov	w8, #0x4                ; =4
100d7e7d8:     	b	0x100d7e800 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1c8>
100d7e7dc:     	ldr	x8, [sp, #0x8]
100d7e7e0:     	rbit	x9, x26
100d7e7e4:     	clz	x9, x9
100d7e7e8:     	str	w9, [x8, x27, lsl #2]
100d7e7ec:     	add	x27, x27, #0x1
100d7e7f0:     	str	x27, [sp, #0x10]
100d7e7f4:     	sub	x9, x26, #0x1
100d7e7f8:     	ands	x26, x9, x26
100d7e7fc:     	b.eq	0x100d7e818 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1e0>
100d7e800:     	ldr	x9, [sp]
100d7e804:     	cmp	x27, x9
100d7e808:     	b.ne	0x100d7e7e0 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a8>
100d7e80c:     	mov	x0, sp
100d7e810:     	bl	0x1013b5b5c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100d7e814:     	b	0x100d7e7dc <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1a4>
100d7e818:     	ldp	x26, x25, [sp]
100d7e81c:     	cbz	x27, 0x100d7e860 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x228>
100d7e820:     	mov	x9, #0x0                ; =0
100d7e824:     	mov	x8, #0x0                ; =0
100d7e828:     	mov	w10, #0x1               ; =1
100d7e82c:     	ldr	w0, [x25, x9, lsl #2]
100d7e830:     	cmp	x23, x0
100d7e834:     	b.ls	0x100d7ea5c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x424>
100d7e838:     	ldr	w11, [x24, x0, lsl #2]
100d7e83c:     	lsl	x11, x10, x11
100d7e840:     	orr	x8, x11, x8
100d7e844:     	add	x9, x9, #0x1
100d7e848:     	cmp	x27, x9
100d7e84c:     	b.ne	0x100d7e82c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x1f4>
100d7e850:     	b	0x100d7e864 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100d7e854:     	mov	x8, #0x0                ; =0
100d7e858:     	mov	w25, #0x4               ; =4
100d7e85c:     	b	0x100d7e864 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x22c>
100d7e860:     	mov	x8, #0x0                ; =0
100d7e864:     	mov	x23, sp
100d7e868:     	movi.2d	v0, #0000000000000000
100d7e86c:     	stur	q0, [x23, #0xc8]
100d7e870:     	stur	q0, [x23, #0xb8]
100d7e874:     	stur	q0, [x23, #0xa8]
100d7e878:     	stur	q0, [x23, #0x98]
100d7e87c:     	stur	q0, [x23, #0x88]
100d7e880:     	ldrb	w9, [x20, #0xe6]
100d7e884:     	ldp	q0, q1, [sp, #0x120]
100d7e888:     	stp	q0, q1, [sp, #0x40]
100d7e88c:     	ldp	q0, q1, [sp, #0xe0]
100d7e890:     	stp	q0, q1, [sp]
100d7e894:     	ldp	q0, q1, [sp, #0x100]
100d7e898:     	stp	xzr, x8, [sp, #0x78]
100d7e89c:     	adrp	x8, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100d7e8a0:     	add	x8, x8, #0xb30
100d7e8a4:     	stp	q0, q1, [sp, #0x20]
100d7e8a8:     	stp	x8, xzr, [sp, #0x60]
100d7e8ac:     	str	xzr, [sp, #0x70]
100d7e8b0:     	strb	w9, [sp, #0xd8]
100d7e8b4:     	cbz	x26, 0x100d7e8c0 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x288>
100d7e8b8:     	mov	x0, x25
100d7e8bc:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e8c0:     	stur	w22, [x29, #-0xe0]
100d7e8c4:     	stp	xzr, xzr, [x29, #-0xf0]
100d7e8c8:     	sub	x0, x29, #0x88
100d7e8cc:     	sub	x1, x29, #0xf0
100d7e8d0:     	mov	x2, x20
100d7e8d4:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100d7e8d8:     	sub	x22, x29, #0x88
100d7e8dc:     	ldur	x8, [x29, #-0x78]
100d7e8e0:     	ldr	q0, [x22]
100d7e8e4:     	stur	q0, [x29, #-0x70]
100d7e8e8:     	stur	x8, [x29, #-0x60]
100d7e8ec:     	str	x8, [sp, #0xf0]
100d7e8f0:     	str	q0, [sp, #0xe0]
100d7e8f4:     	stur	w21, [x29, #-0xe0]
100d7e8f8:     	stp	xzr, xzr, [x29, #-0xf0]
100d7e8fc:     	sub	x0, x29, #0x88
100d7e900:     	sub	x1, x29, #0xf0
100d7e904:     	mov	x2, x20
100d7e908:     	bl	0x1007b92f4 <__RINvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB5_10Restricted9normalizeKm1_EBb_>
100d7e90c:     	ldur	x8, [x29, #-0x78]
100d7e910:     	ldr	q0, [x22]
100d7e914:     	stur	q0, [x23, #0xf8]
100d7e918:     	str	x8, [sp, #0x108]
100d7e91c:     	ldp	q0, q1, [sp, #0xe0]
100d7e920:     	ldr	q2, [sp, #0x100]
100d7e924:     	stp	q1, q2, [x29, #-0xe0]
100d7e928:     	stur	q0, [x29, #-0xf0]
100d7e92c:     	mov	x0, sp
100d7e930:     	sub	x2, x29, #0xf0
100d7e934:     	mov	x1, x20
100d7e938:     	bl	0x10075eabc <__RINvMs2_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
100d7e93c:     	mov	x8, sp
100d7e940:     	ldur	q0, [x8, #0xc8]
100d7e944:     	stur	q0, [x19, #0x48]
100d7e948:     	ldur	q0, [x8, #0x88]
100d7e94c:     	stur	q0, [x19, #0x8]
100d7e950:     	ldur	q0, [x8, #0x98]
100d7e954:     	stur	q0, [x19, #0x18]
100d7e958:     	ldur	q0, [x8, #0xa8]
100d7e95c:     	stur	q0, [x19, #0x28]
100d7e960:     	ldur	q0, [x8, #0xb8]
100d7e964:     	stur	q0, [x19, #0x38]
100d7e968:     	str	w0, [x19]
100d7e96c:     	ldr	x8, [sp]
100d7e970:     	cbz	x8, 0x100d7e97c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x344>
100d7e974:     	ldr	x0, [sp, #0x8]
100d7e978:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e97c:     	ldr	x8, [sp, #0x18]
100d7e980:     	cbz	x8, 0x100d7e98c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x354>
100d7e984:     	ldr	x0, [sp, #0x20]
100d7e988:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e98c:     	ldr	x8, [sp, #0x30]
100d7e990:     	cbz	x8, 0x100d7e99c <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x364>
100d7e994:     	ldr	x0, [sp, #0x38]
100d7e998:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e99c:     	ldr	x8, [sp, #0x48]
100d7e9a0:     	cbz	x8, 0x100d7e9ac <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x374>
100d7e9a4:     	ldr	x0, [sp, #0x50]
100d7e9a8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e9ac:     	ldr	x9, [sp, #0x68]
100d7e9b0:     	cbz	x9, 0x100d7e9d8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3a0>
100d7e9b4:     	lsl	x8, x9, #6
100d7e9b8:     	sub	x8, x8, x9, lsl #3
100d7e9bc:     	add	x9, x8, x9
100d7e9c0:     	cmn	x9, #0x41
100d7e9c4:     	b.eq	0x100d7e9d8 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x3a0>
100d7e9c8:     	ldr	x9, [sp, #0x60]
100d7e9cc:     	sub	x8, x9, x8
100d7e9d0:     	sub	x0, x8, #0x38
100d7e9d4:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7e9d8:     	add	sp, sp, #0x1e0
100d7e9dc:     	ldp	x29, x30, [sp, #0x50]
100d7e9e0:     	ldp	x20, x19, [sp, #0x40]
100d7e9e4:     	ldp	x22, x21, [sp, #0x30]
100d7e9e8:     	ldp	x24, x23, [sp, #0x20]
100d7e9ec:     	ldp	x26, x25, [sp, #0x10]
100d7e9f0:     	ldp	x28, x27, [sp], #0x60
100d7e9f4:     	ret
100d7e9f8:     	adrp	x0, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7e9fc:     	add	x0, x0, #0x563
100d7ea00:     	adrp	x2, 0x10163f000 <dyld_stub_binder+0x10163f000>
100d7ea04:     	add	x2, x2, #0xd60
100d7ea08:     	mov	w1, #0x2f               ; =47
100d7ea0c:     	bl	0x1013b4bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d7ea10:     	adrp	x3, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7ea14:     	add	x3, x3, #0x54d
100d7ea18:     	adrp	x5, 0x10163f000 <dyld_stub_binder+0x10163f000>
100d7ea1c:     	add	x5, x5, #0xd48
100d7ea20:     	add	x1, sp, #0xe0
100d7ea24:     	mov	x2, sp
100d7ea28:     	mov	w0, #0x0                ; =0
100d7ea2c:     	mov	w4, #0x2d               ; =45
100d7ea30:     	bl	0x1013b4c30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d7ea34:     	adrp	x2, 0x10151c000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e3f>
100d7ea38:     	add	x2, x2, #0x3e0
100d7ea3c:     	adrp	x3, 0x10147c000 <dyld_stub_binder+0x10147c000>
100d7ea40:     	add	x3, x3, #0xada
100d7ea44:     	adrp	x5, 0x101640000 <dyld_stub_binder+0x101640000>
100d7ea48:     	add	x5, x5, #0x678
100d7ea4c:     	mov	x1, sp
100d7ea50:     	mov	w0, #0x0                ; =0
100d7ea54:     	mov	w4, #0x57               ; =87
100d7ea58:     	bl	0x1013b4c60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100d7ea5c:     	adrp	x2, 0x101603000 <dyld_stub_binder+0x101603000>
100d7ea60:     	add	x2, x2, #0xa8
100d7ea64:     	mov	x1, x23
100d7ea68:     	bl	0x1013b4d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d7ea6c:     	brk	#0x1
100d7ea70:     	mov	x19, x0
100d7ea74:     	sub	x0, x29, #0xf0
100d7ea78:     	bl	0x100824a0c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtCs23EhFSy3h49_8bumbledb4plan7planner9JoinOrderEBH_>
100d7ea7c:     	mov	x0, x19
100d7ea80:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ea84:     	mov	x19, x0
100d7ea88:     	mov	x0, sp
100d7ea8c:     	bl	0x100825f24 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product7ProductEBJ_>
100d7ea90:     	mov	x0, x19
100d7ea94:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ea98:     	mov	x19, x0
100d7ea9c:     	ldr	x8, [sp]
100d7eaa0:     	cbz	x8, 0x100d7eaac <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x474>
100d7eaa4:     	ldr	x0, [sp, #0x8]
100d7eaa8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7eaac:     	add	x0, sp, #0xe0
100d7eab0:     	bl	0x10080a39c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d7eab4:     	mov	x0, x19
100d7eab8:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7eabc:     	mov	x19, x0
100d7eac0:     	add	x0, sp, #0xe0
100d7eac4:     	bl	0x10080a39c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product3Mapj2_EBK_>
100d7eac8:     	cbnz	x26, 0x100d7ead4 <__RNvMs3_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_+0x49c>
100d7eacc:     	mov	x0, x19
100d7ead0:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
100d7ead4:     	mov	x0, x25
100d7ead8:     	bl	0x1013bd178 <dyld_stub_binder+0x1013bd178>
100d7eadc:     	mov	x0, x19
100d7eae0:     	bl	0x1013bcfc8 <dyld_stub_binder+0x1013bcfc8>
