
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>:
100ebf6e8:     	sub	sp, sp, #0x1e0
100ebf6ec:     	stp	d15, d14, [sp, #0x140]
100ebf6f0:     	stp	d13, d12, [sp, #0x150]
100ebf6f4:     	stp	d11, d10, [sp, #0x160]
100ebf6f8:     	stp	d9, d8, [sp, #0x170]
100ebf6fc:     	stp	x28, x27, [sp, #0x180]
100ebf700:     	stp	x26, x25, [sp, #0x190]
100ebf704:     	stp	x24, x23, [sp, #0x1a0]
100ebf708:     	stp	x22, x21, [sp, #0x1b0]
100ebf70c:     	stp	x20, x19, [sp, #0x1c0]
100ebf710:     	stp	x29, x30, [sp, #0x1d0]
100ebf714:     	add	x29, sp, #0x1d0
100ebf718:     	str	x6, [sp, #0x98]
100ebf71c:     	mov	x24, x5
100ebf720:     	mov	x20, x4
100ebf724:     	mov	x27, x3
100ebf728:     	mov	x25, x2
100ebf72c:     	mov	x23, x1
100ebf730:     	mov	x28, x0
100ebf734:     	str	x4, [sp, #0xb8]
100ebf738:     	ldr	x21, [x2]
100ebf73c:     	ldp	x22, x26, [x3, #0x8]
100ebf740:     	cbz	x21, 0x100ebf784 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9c>
100ebf744:     	mov	x8, #0x0                ; =0
100ebf748:     	mov	w9, #0x1                ; =1
100ebf74c:     	mov	x10, x21
100ebf750:     	rbit	x11, x10
100ebf754:     	clz	x0, x11
100ebf758:     	cmp	x0, x26
100ebf75c:     	b.hs	0x100ec04f8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe10>
100ebf760:     	ldr	w11, [x22, x0, lsl #2]
100ebf764:     	lsl	x11, x9, x11
100ebf768:     	orr	x8, x11, x8
100ebf76c:     	sub	x11, x10, #0x1
100ebf770:     	ands	x10, x11, x10
100ebf774:     	b.ne	0x100ebf750 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x68>
100ebf778:     	ands	x8, x8, x20
100ebf77c:     	stur	x8, [x29, #-0xb8]
100ebf780:     	b.ne	0x100ec0430 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd48>
100ebf784:     	ldr	w2, [x25, #0x10]
100ebf788:     	sub	x0, x29, #0xb8
100ebf78c:     	add	x1, x23, #0x40
100ebf790:     	str	x2, [sp, #0x78]
100ebf794:     	bl	0x1010b8900 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100ebf798:     	ldur	w8, [x29, #-0xb8]
100ebf79c:     	cbz	w8, 0x100ebf824 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x13c>
100ebf7a0:     	cmp	w8, #0x1
100ebf7a4:     	str	x20, [sp, #0x60]
100ebf7a8:     	b.ne	0x100ebf83c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x154>
100ebf7ac:     	ldp	x23, x8, [x29, #-0xb0]
100ebf7b0:     	str	x8, [sp, #0xa0]
100ebf7b4:     	ldur	x27, [x29, #-0xa0]
100ebf7b8:     	mov	w8, #0x4                ; =4
100ebf7bc:     	stp	xzr, x8, [x29, #-0xb8]
100ebf7c0:     	stur	xzr, [x29, #-0xa8]
100ebf7c4:     	str	x28, [sp, #0x58]
100ebf7c8:     	cbz	x21, 0x100ebfb60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x478>
100ebf7cc:     	mov	x28, #0x0               ; =0
100ebf7d0:     	mov	w8, #0x4                ; =4
100ebf7d4:     	mov	w9, #0x1                ; =1
100ebf7d8:     	b	0x100ebf804 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x11c>
100ebf7dc:     	ldur	x8, [x29, #-0xb0]
100ebf7e0:     	rbit	x9, x21
100ebf7e4:     	clz	x9, x9
100ebf7e8:     	str	w9, [x8, x28]
100ebf7ec:     	stur	x19, [x29, #-0xa8]
100ebf7f0:     	sub	x10, x21, #0x1
100ebf7f4:     	add	x28, x28, #0x4
100ebf7f8:     	add	x9, x19, #0x1
100ebf7fc:     	ands	x21, x10, x21
100ebf800:     	b.eq	0x100ebfa48 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x360>
100ebf804:     	mov	x19, x9
100ebf808:     	sub	x9, x9, #0x1
100ebf80c:     	ldur	x10, [x29, #-0xb8]
100ebf810:     	cmp	x9, x10
100ebf814:     	b.ne	0x100ebf7e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf8>
100ebf818:     	sub	x0, x29, #0xb8
100ebf81c:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100ebf820:     	b	0x100ebf7dc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf4>
100ebf824:     	ldr	x8, [sp, #0x78]
100ebf828:     	and	w8, w8, #0x1
100ebf82c:     	strb	w8, [x28, #0x8]
100ebf830:     	mov	x8, #-0x2               ; =-2
100ebf834:     	str	x8, [x28]
100ebf838:     	b	0x100ec0400 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100ebf83c:     	ldp	w19, w8, [x29, #-0xb4]
100ebf840:     	ldur	w9, [x29, #-0xac]
100ebf844:     	ldr	x11, [sp, #0x98]
100ebf848:     	ldr	x10, [x11, #0x38]
100ebf84c:     	add	x10, x10, #0x1
100ebf850:     	str	x10, [x11, #0x38]
100ebf854:     	tbz	w24, #0x0, 0x100ebfb08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x420>
100ebf858:     	lsr	x10, x21, x19
100ebf85c:     	and	x11, x10, #0x1
100ebf860:     	stur	x11, [x29, #-0xb8]
100ebf864:     	tbnz	w10, #0x0, 0x100ec0494 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdac>
100ebf868:     	ldr	x10, [sp, #0x78]
100ebf86c:     	and	w10, w10, #0x1
100ebf870:     	eor	w8, w8, w10
100ebf874:     	eor	w20, w9, w10
100ebf878:     	stur	w8, [x29, #-0xa8]
100ebf87c:     	ldr	x24, [x25, #0x8]
100ebf880:     	stp	x21, x24, [x29, #-0xb8]
100ebf884:     	add	x0, sp, #0xc0
100ebf888:     	sub	x1, x29, #0xb8
100ebf88c:     	mov	x2, x23
100ebf890:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100ebf894:     	stur	w20, [x29, #-0xa8]
100ebf898:     	stp	x21, x24, [x29, #-0xb8]
100ebf89c:     	add	x0, sp, #0xd8
100ebf8a0:     	sub	x1, x29, #0xb8
100ebf8a4:     	mov	x2, x23
100ebf8a8:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100ebf8ac:     	sub	x0, x29, #0xe0
100ebf8b0:     	add	x2, sp, #0xc0
100ebf8b4:     	mov	x1, x23
100ebf8b8:     	mov	x3, x27
100ebf8bc:     	ldr	x21, [sp, #0x60]
100ebf8c0:     	mov	x4, x21
100ebf8c4:     	mov	w5, #0x1                ; =1
100ebf8c8:     	ldr	x20, [sp, #0x98]
100ebf8cc:     	mov	x6, x20
100ebf8d0:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100ebf8d4:     	sub	x0, x29, #0xb8
100ebf8d8:     	add	x2, sp, #0xd8
100ebf8dc:     	mov	x1, x23
100ebf8e0:     	mov	x3, x27
100ebf8e4:     	mov	x4, x21
100ebf8e8:     	mov	w5, #0x1                ; =1
100ebf8ec:     	mov	x6, x20
100ebf8f0:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100ebf8f4:     	cmp	x26, x19
100ebf8f8:     	b.ls	0x100ec0588 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xea0>
100ebf8fc:     	ldr	w21, [x22, x19, lsl #2]
100ebf900:     	ldr	x10, [sp, #0x60]
100ebf904:     	lsr	x8, x10, x21
100ebf908:     	and	x9, x8, #0x1
100ebf90c:     	stur	x9, [x29, #-0xc0]
100ebf910:     	tbz	w8, #0x0, 0x100ec04b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdcc>
100ebf914:     	fmov	d0, x10
100ebf918:     	cnt.8b	v0, v0
100ebf91c:     	addv.8b	b0, v0
100ebf920:     	fmov	x8, d0
100ebf924:     	and	x9, x8, #0x3f
100ebf928:     	mov	w10, #0x1               ; =1
100ebf92c:     	lsl	x8, x10, x8
100ebf930:     	lsr	x8, x8, #6
100ebf934:     	cmp	x9, #0x6
100ebf938:     	cinc	x20, x8, lo
100ebf93c:     	cbz	x20, 0x100ec01a0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xab8>
100ebf940:     	lsl	x19, x20, #3
100ebf944:     	mov	x0, x19
100ebf948:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
100ebf94c:     	cbz	x0, 0x100ec05b0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xec8>
100ebf950:     	mov	x9, #0x0                ; =0
100ebf954:     	and	x8, x21, #0x3f
100ebf958:     	mov	x10, #-0x1              ; =-1
100ebf95c:     	lsl	x8, x10, x8
100ebf960:     	ldr	x10, [sp, #0x60]
100ebf964:     	bic	x8, x10, x8
100ebf968:     	fmov	d0, x8
100ebf96c:     	cnt.8b	v0, v0
100ebf970:     	addv.8b	b0, v0
100ebf974:     	fmov	x10, d0
100ebf978:     	add	w8, w10, #0x3a
100ebf97c:     	mov	w11, #0x1               ; =1
100ebf980:     	lsl	x11, x11, x8
100ebf984:     	ldp	x12, x13, [x29, #-0xe0]
100ebf988:     	ldp	x1, x14, [x29, #-0xd0]
100ebf98c:     	sub	x15, x9, w13, uxtb
100ebf990:     	ldp	x21, x17, [x29, #-0xb8]
100ebf994:     	ldp	x16, x2, [x29, #-0xa8]
100ebf998:     	adrp	x3, 0x10179b000 <dyld_stub_binder+0x10179b000>
100ebf99c:     	add	x3, x3, #0x558
100ebf9a0:     	mov	x8, #0x0                ; =0
100ebf9a4:     	b	0x100ebf9c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100ebf9a8:     	tst	w17, #0x1
100ebf9ac:     	csel	x6, x4, x9, ne
100ebf9b0:     	bic	x4, x5, x4
100ebf9b4:     	orr	x4, x6, x4
100ebf9b8:     	str	x4, [x0, x8, lsl #3]
100ebf9bc:     	add	x8, x8, #0x1
100ebf9c0:     	cmp	x20, x8
100ebf9c4:     	b.eq	0x100ebfa40 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x358>
100ebf9c8:     	cmp	x10, #0x6
100ebf9cc:     	b.hs	0x100ebf9e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x300>
100ebf9d0:     	ldr	x4, [x3, x10, lsl #3]
100ebf9d4:     	mvn	x4, x4
100ebf9d8:     	mov	x5, x15
100ebf9dc:     	cmn	x12, #0x2
100ebf9e0:     	b.ne	0x100ebf9fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x314>
100ebf9e4:     	b	0x100ebfa0c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100ebf9e8:     	tst	x8, x11
100ebf9ec:     	csetm	x4, ne
100ebf9f0:     	mov	x5, x15
100ebf9f4:     	cmn	x12, #0x2
100ebf9f8:     	b.eq	0x100ebfa0c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100ebf9fc:     	cmp	x8, x1
100ebfa00:     	b.hs	0x100ec0560 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe78>
100ebfa04:     	ldr	x5, [x13, x8, lsl #3]
100ebfa08:     	eor	x5, x14, x5
100ebfa0c:     	cmn	x21, #0x2
100ebfa10:     	b.eq	0x100ebf9a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2c0>
100ebfa14:     	cmp	x8, x16
100ebfa18:     	b.hs	0x100ec0554 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe6c>
100ebfa1c:     	ldr	x6, [x17, x8, lsl #3]
100ebfa20:     	eor	x6, x2, x6
100ebfa24:     	and	x6, x6, x4
100ebfa28:     	bic	x4, x5, x4
100ebfa2c:     	orr	x4, x6, x4
100ebfa30:     	str	x4, [x0, x8, lsl #3]
100ebfa34:     	add	x8, x8, #0x1
100ebfa38:     	cmp	x20, x8
100ebfa3c:     	b.ne	0x100ebf9c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100ebfa40:     	mov	x8, x20
100ebfa44:     	b	0x100ec01ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xac4>
100ebfa48:     	ldp	x8, x21, [x29, #-0xb8]
100ebfa4c:     	str	x8, [sp, #0x40]
100ebfa50:     	str	x21, [sp, #0x30]
100ebfa54:     	cbz	x19, 0x100ebfbc8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e0>
100ebfa58:     	ldr	x8, [x25, #0x8]
100ebfa5c:     	str	x8, [sp, #0x80]
100ebfa60:     	mov	x24, #-0x1              ; =-1
100ebfa64:     	mov	x19, x23
100ebfa68:     	b	0x100ebfa94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x3ac>
100ebfa6c:     	bic	x27, x27, x23
100ebfa70:     	ldr	x9, [sp, #0x98]
100ebfa74:     	ldr	x8, [x9, #0x20]
100ebfa78:     	add	x8, x8, #0x1
100ebfa7c:     	str	x8, [x9, #0x20]
100ebfa80:     	mov	x24, x20
100ebfa84:     	mov	x23, x25
100ebfa88:     	mov	x19, x25
100ebfa8c:     	subs	x28, x28, #0x4
100ebfa90:     	b.eq	0x100ebfbcc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e4>
100ebfa94:     	ldr	w8, [x21], #0x4
100ebfa98:     	mov	w9, #0x1                ; =1
100ebfa9c:     	lsl	x23, x9, x8
100ebfaa0:     	sub	x9, x23, #0x1
100ebfaa4:     	and	x9, x9, x27
100ebfaa8:     	fmov	d0, x9
100ebfaac:     	cnt.8b	v0, v0
100ebfab0:     	addv.8b	b0, v0
100ebfab4:     	fmov	w4, s0
100ebfab8:     	fmov	d0, x27
100ebfabc:     	cnt.8b	v0, v0
100ebfac0:     	addv.8b	b0, v0
100ebfac4:     	fmov	w3, s0
100ebfac8:     	ldr	x9, [sp, #0x80]
100ebfacc:     	lsr	x8, x9, x8
100ebfad0:     	sub	x0, x29, #0xb8
100ebfad4:     	and	w5, w8, #0x1
100ebfad8:     	mov	x1, x19
100ebfadc:     	ldr	x2, [sp, #0xa0]
100ebfae0:     	bl	0x101169958 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100ebfae4:     	ldp	x20, x25, [x29, #-0xb8]
100ebfae8:     	ldur	x8, [x29, #-0xa8]
100ebfaec:     	str	x8, [sp, #0xa0]
100ebfaf0:     	sub	x8, x24, #0x1
100ebfaf4:     	cmn	x8, #0x3
100ebfaf8:     	b.hi	0x100ebfa6c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100ebfafc:     	mov	x0, x19
100ebfb00:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ebfb04:     	b	0x100ebfa6c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100ebfb08:     	mov	w8, #0x4                ; =4
100ebfb0c:     	stp	xzr, x8, [x29, #-0xb8]
100ebfb10:     	stur	xzr, [x29, #-0xa8]
100ebfb14:     	mov	x26, #0x0               ; =0
100ebfb18:     	cbz	x20, 0x100ebfe0c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x724>
100ebfb1c:     	mov	w8, #0x4                ; =4
100ebfb20:     	b	0x100ebfb48 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x460>
100ebfb24:     	ldur	x8, [x29, #-0xb0]
100ebfb28:     	rbit	x9, x20
100ebfb2c:     	clz	x9, x9
100ebfb30:     	str	w9, [x8, x26, lsl #2]
100ebfb34:     	add	x26, x26, #0x1
100ebfb38:     	stur	x26, [x29, #-0xa8]
100ebfb3c:     	sub	x9, x20, #0x1
100ebfb40:     	ands	x20, x9, x20
100ebfb44:     	b.eq	0x100ebfb70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x488>
100ebfb48:     	ldur	x9, [x29, #-0xb8]
100ebfb4c:     	cmp	x26, x9
100ebfb50:     	b.ne	0x100ebfb28 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x440>
100ebfb54:     	sub	x0, x29, #0xb8
100ebfb58:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100ebfb5c:     	b	0x100ebfb24 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x43c>
100ebfb60:     	mov	x19, #-0x1              ; =-1
100ebfb64:     	mov	x21, #0x0               ; =0
100ebfb68:     	cbnz	x27, 0x100ebfbec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x504>
100ebfb6c:     	b	0x100ebfc1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100ebfb70:     	ldp	x20, x19, [x29, #-0xb8]
100ebfb74:     	cbz	x26, 0x100ec0228 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb40>
100ebfb78:     	lsl	x22, x26, #2
100ebfb7c:     	mov	x0, x22
100ebfb80:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
100ebfb84:     	cbz	x0, 0x100ec05c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xed8>
100ebfb88:     	mov	x24, x0
100ebfb8c:     	mov	x8, #0x0                ; =0
100ebfb90:     	ldp	x9, x1, [x27, #0x20]
100ebfb94:     	ldr	w0, [x19, x8, lsl #2]
100ebfb98:     	cmp	x1, x0
100ebfb9c:     	b.ls	0x100ec0544 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe5c>
100ebfba0:     	ldr	w10, [x9, x0, lsl #2]
100ebfba4:     	str	w10, [x24, x8, lsl #2]
100ebfba8:     	add	x8, x8, #0x1
100ebfbac:     	cmp	x26, x8
100ebfbb0:     	b.ne	0x100ebfb94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4ac>
100ebfbb4:     	cbz	x20, 0x100ebfbc0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100ebfbb8:     	mov	x0, x19
100ebfbbc:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ebfbc0:     	ldr	x20, [sp, #0x60]
100ebfbc4:     	b	0x100ebfe10 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x728>
100ebfbc8:     	mov	x20, #-0x1              ; =-1
100ebfbcc:     	ldr	x8, [sp, #0x40]
100ebfbd0:     	cbz	x8, 0x100ebfbdc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4f4>
100ebfbd4:     	ldr	x0, [sp, #0x30]
100ebfbd8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ebfbdc:     	mov	x19, x20
100ebfbe0:     	ldp	x28, x20, [sp, #0x58]
100ebfbe4:     	mov	x21, #0x0               ; =0
100ebfbe8:     	cbz	x27, 0x100ebfc1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100ebfbec:     	mov	w8, #0x1                ; =1
100ebfbf0:     	mov	x9, x27
100ebfbf4:     	rbit	x10, x9
100ebfbf8:     	clz	x0, x10
100ebfbfc:     	cmp	x0, x26
100ebfc00:     	b.hs	0x100ec0508 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe20>
100ebfc04:     	ldr	w10, [x22, x0, lsl #2]
100ebfc08:     	lsl	x10, x8, x10
100ebfc0c:     	orr	x21, x10, x21
100ebfc10:     	sub	x10, x9, #0x1
100ebfc14:     	ands	x9, x10, x9
100ebfc18:     	b.ne	0x100ebfbf4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x50c>
100ebfc1c:     	stur	x21, [x29, #-0xe0]
100ebfc20:     	bics	x8, x21, x20
100ebfc24:     	stur	x8, [x29, #-0xb8]
100ebfc28:     	b.ne	0x100ec0450 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd68>
100ebfc2c:     	str	x19, [sp, #0x80]
100ebfc30:     	mov	w25, #0x4               ; =4
100ebfc34:     	stp	xzr, x25, [x29, #-0xb8]
100ebfc38:     	stur	xzr, [x29, #-0xa8]
100ebfc3c:     	mov	x19, #0x0               ; =0
100ebfc40:     	cbz	x21, 0x100ebfda0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6b8>
100ebfc44:     	mov	w8, #0x4                ; =4
100ebfc48:     	mov	x20, x21
100ebfc4c:     	b	0x100ebfc70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x588>
100ebfc50:     	rbit	x9, x20
100ebfc54:     	clz	x9, x9
100ebfc58:     	str	w9, [x8, x19, lsl #2]
100ebfc5c:     	add	x19, x19, #0x1
100ebfc60:     	stur	x19, [x29, #-0xa8]
100ebfc64:     	sub	x9, x20, #0x1
100ebfc68:     	ands	x20, x9, x20
100ebfc6c:     	b.eq	0x100ebfc8c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5a4>
100ebfc70:     	ldur	x9, [x29, #-0xb8]
100ebfc74:     	cmp	x19, x9
100ebfc78:     	b.ne	0x100ebfc50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100ebfc7c:     	sub	x0, x29, #0xb8
100ebfc80:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100ebfc84:     	ldur	x8, [x29, #-0xb0]
100ebfc88:     	b	0x100ebfc50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100ebfc8c:     	ldp	x8, x25, [x29, #-0xb8]
100ebfc90:     	cmp	x8, #0x0
100ebfc94:     	cset	w8, eq
100ebfc98:     	str	w8, [sp, #0x40]
100ebfc9c:     	mov	w8, #0x4                ; =4
100ebfca0:     	stp	xzr, x8, [x29, #-0xb8]
100ebfca4:     	stur	xzr, [x29, #-0xa8]
100ebfca8:     	cbz	x27, 0x100ebfdb8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6d0>
100ebfcac:     	str	x23, [sp, #0x30]
100ebfcb0:     	mov	x20, #0x0               ; =0
100ebfcb4:     	mov	w8, #0x4                ; =4
100ebfcb8:     	b	0x100ebfcdc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5f4>
100ebfcbc:     	rbit	x9, x27
100ebfcc0:     	clz	x9, x9
100ebfcc4:     	str	w9, [x8, x28, lsl #2]
100ebfcc8:     	add	x20, x28, #0x1
100ebfccc:     	stur	x20, [x29, #-0xa8]
100ebfcd0:     	sub	x9, x27, #0x1
100ebfcd4:     	ands	x27, x9, x27
100ebfcd8:     	b.eq	0x100ebfcfc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x614>
100ebfcdc:     	mov	x28, x20
100ebfce0:     	ldur	x9, [x29, #-0xb8]
100ebfce4:     	cmp	x20, x9
100ebfce8:     	b.ne	0x100ebfcbc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100ebfcec:     	sub	x0, x29, #0xb8
100ebfcf0:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100ebfcf4:     	ldur	x8, [x29, #-0xb0]
100ebfcf8:     	b	0x100ebfcbc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100ebfcfc:     	ldp	x8, x24, [x29, #-0xb8]
100ebfd00:     	cbz	x20, 0x100ebfdf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x710>
100ebfd04:     	str	x8, [sp, #0x20]
100ebfd08:     	lsl	x0, x20, #2
100ebfd0c:     	mov	x23, x0
100ebfd10:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
100ebfd14:     	cbz	x0, 0x100ec05a0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xeb8>
100ebfd18:     	mov	x27, x0
100ebfd1c:     	cbz	x19, 0x100ebfd6c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x684>
100ebfd20:     	mov	x9, #0x0                ; =0
100ebfd24:     	lsl	x8, x19, #2
100ebfd28:     	b	0x100ebfd3c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x654>
100ebfd2c:     	str	w10, [x27, x9, lsl #2]
100ebfd30:     	cmp	x9, x28
100ebfd34:     	add	x9, x9, #0x1
100ebfd38:     	b.eq	0x100ebfd7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x694>
100ebfd3c:     	ldr	w0, [x24, x9, lsl #2]
100ebfd40:     	cmp	x26, x0
100ebfd44:     	b.ls	0x100ec051c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe34>
100ebfd48:     	mov	x10, #0x0               ; =0
100ebfd4c:     	ldr	w11, [x22, x0, lsl #2]
100ebfd50:     	mov	x12, x8
100ebfd54:     	ldr	w13, [x25, x10, lsl #2]
100ebfd58:     	cmp	w13, w11
100ebfd5c:     	b.eq	0x100ebfd2c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x644>
100ebfd60:     	add	x10, x10, #0x1
100ebfd64:     	subs	x12, x12, #0x4
100ebfd68:     	b.ne	0x100ebfd54 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x66c>
100ebfd6c:     	adrp	x0, 0x10194e000 <dyld_stub_binder+0x10194e000>
100ebfd70:     	add	x0, x0, #0xbf8
100ebfd74:     	bl	0x1016e75b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100ebfd78:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ebfd7c:     	ldr	x19, [sp, #0x80]
100ebfd80:     	ldr	x8, [sp, #0x20]
100ebfd84:     	ldr	x28, [sp, #0x58]
100ebfd88:     	cbz	x8, 0x100ebfd94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100ebfd8c:     	mov	x0, x24
100ebfd90:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ebfd94:     	mov	x24, x27
100ebfd98:     	ldr	x23, [sp, #0x30]
100ebfd9c:     	b	0x100ebfdc4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6dc>
100ebfda0:     	mov	w8, #0x1                ; =1
100ebfda4:     	str	w8, [sp, #0x40]
100ebfda8:     	mov	w8, #0x4                ; =4
100ebfdac:     	stp	xzr, x8, [x29, #-0xb8]
100ebfdb0:     	stur	xzr, [x29, #-0xa8]
100ebfdb4:     	cbnz	x27, 0x100ebfcac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5c4>
100ebfdb8:     	mov	x20, #0x0               ; =0
100ebfdbc:     	mov	w24, #0x4               ; =4
100ebfdc0:     	ldr	x19, [sp, #0x80]
100ebfdc4:     	mov	x8, #0x0                ; =0
100ebfdc8:     	lsl	x9, x20, #2
100ebfdcc:     	str	x24, [sp, #0x30]
100ebfdd0:     	cbz	x9, 0x100ec0260 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb78>
100ebfdd4:     	ldr	w10, [x24, x8, lsl #2]
100ebfdd8:     	sub	x9, x9, #0x4
100ebfddc:     	cmp	x8, x10
100ebfde0:     	add	x8, x8, #0x1
100ebfde4:     	b.eq	0x100ebfdd0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6e8>
100ebfde8:     	cmn	x19, #0x1
100ebfdec:     	b.eq	0x100ec01e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb00>
100ebfdf0:     	ldr	x1, [sp, #0xa0]
100ebfdf4:     	b	0x100ec023c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100ebfdf8:     	mov	w27, #0x4               ; =4
100ebfdfc:     	ldr	x19, [sp, #0x80]
100ebfe00:     	ldr	x28, [sp, #0x58]
100ebfe04:     	cbnz	x8, 0x100ebfd8c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6a4>
100ebfe08:     	b	0x100ebfd94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100ebfe0c:     	mov	w24, #0x4               ; =4
100ebfe10:     	str	x28, [sp, #0x58]
100ebfe14:     	fmov	d0, x20
100ebfe18:     	cnt.8b	v0, v0
100ebfe1c:     	addv.8b	b0, v0
100ebfe20:     	fmov	x8, d0
100ebfe24:     	mov	w9, #0x1                ; =1
100ebfe28:     	lsl	x20, x9, x8
100ebfe2c:     	ldr	x9, [sp, #0x98]
100ebfe30:     	ldr	x8, [x9, #0x40]
100ebfe34:     	add	x8, x8, x20
100ebfe38:     	str	x8, [x9, #0x40]
100ebfe3c:     	add	x8, x20, #0x3f
100ebfe40:     	lsr	x22, x8, #6
100ebfe44:     	lsl	x19, x22, #3
100ebfe48:     	mov	x0, x19
100ebfe4c:     	mov	w1, #0x1                ; =1
100ebfe50:     	bl	0x1016ef824 <dyld_stub_binder+0x1016ef824>
100ebfe54:     	cbz	x0, 0x100ec0578 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe90>
100ebfe58:     	mov	x21, x0
100ebfe5c:     	mov	x19, #0x0               ; =0
100ebfe60:     	ldr	x25, [x25, #0x8]
100ebfe64:     	and	x8, x26, #0xfffffffffffffffe
100ebfe68:     	neg	x8, x8
100ebfe6c:     	str	x8, [sp, #0x98]
100ebfe70:     	mov	w28, #0x1               ; =1
100ebfe74:     	adrp	x8, 0x101790000 <GCC_except_table10048+0x4>
100ebfe78:     	ldr	q0, [x8, #0x140]
100ebfe7c:     	str	q0, [sp, #0xa0]
100ebfe80:     	mov	w8, #0x2                ; =2
100ebfe84:     	dup.2d	v0, x8
100ebfe88:     	str	q0, [sp, #0x80]
100ebfe8c:     	mov	w8, #0x4                ; =4
100ebfe90:     	dup.2d	v1, x8
100ebfe94:     	mov	w8, #0x8                ; =8
100ebfe98:     	dup.2d	v0, x8
100ebfe9c:     	stp	q0, q1, [sp, #0x30]
100ebfea0:     	mov	w8, #0xc                ; =12
100ebfea4:     	dup.2d	v1, x8
100ebfea8:     	mov	w8, #0x10               ; =16
100ebfeac:     	dup.2d	v0, x8
100ebfeb0:     	stp	q0, q1, [sp, #0x10]
100ebfeb4:     	adrp	x8, 0x101790000 <GCC_except_table10048+0x4>
100ebfeb8:     	ldr	q0, [x8, #0x130]
100ebfebc:     	str	q0, [sp]
100ebfec0:     	mov	w27, #0x3f              ; =63
100ebfec4:     	dup.2d	v0, x27
100ebfec8:     	str	q0, [sp, #0x60]
100ebfecc:     	movi.2s	v8, #0x3f
100ebfed0:     	b	0x100ebfee0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7f8>
100ebfed4:     	add	x19, x19, #0x1
100ebfed8:     	cmp	x19, x20
100ebfedc:     	b.eq	0x100ec0188 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaa0>
100ebfee0:     	mov	x2, x25
100ebfee4:     	cbz	x26, 0x100ec0158 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa70>
100ebfee8:     	cmp	x26, #0x1
100ebfeec:     	b.ne	0x100ebfefc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x814>
100ebfef0:     	mov	x9, #0x0                ; =0
100ebfef4:     	mov	x8, #0x0                ; =0
100ebfef8:     	b	0x100ec0134 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100ebfefc:     	dup.2d	v0, x19
100ebff00:     	cmp	x26, #0x10
100ebff04:     	b.hs	0x100ebff14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x82c>
100ebff08:     	mov	x10, #0x0               ; =0
100ebff0c:     	mov	x8, #0x0                ; =0
100ebff10:     	b	0x100ec00c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9d8>
100ebff14:     	movi.2d	v1, #0000000000000000
100ebff18:     	add	x8, x24, #0x20
100ebff1c:     	movi.2d	v2, #0000000000000000
100ebff20:     	and	x9, x26, #0xfffffffffffffff0
100ebff24:     	ldr	q4, [sp, #0xa0]
100ebff28:     	ldp	q6, q15, [sp]
100ebff2c:     	movi.2d	v3, #0000000000000000
100ebff30:     	movi.2d	v7, #0000000000000000
100ebff34:     	movi.2d	v16, #0000000000000000
100ebff38:     	movi.2d	v5, #0000000000000000
100ebff3c:     	movi.2d	v18, #0000000000000000
100ebff40:     	movi.2d	v17, #0000000000000000
100ebff44:     	ldp	q13, q12, [sp, #0x30]
100ebff48:     	ldr	q14, [sp, #0x20]
100ebff4c:     	movi.4s	v8, #0x3f
100ebff50:     	add.2d	v19, v4, v12
100ebff54:     	add.2d	v20, v6, v12
100ebff58:     	add.2d	v21, v4, v13
100ebff5c:     	add.2d	v22, v6, v13
100ebff60:     	add.2d	v23, v4, v14
100ebff64:     	add.2d	v24, v6, v14
100ebff68:     	ldp	q25, q26, [x8, #-0x20]
100ebff6c:     	dup.2d	v27, x27
100ebff70:     	ldp	q28, q29, [x8], #0x40
100ebff74:     	and.16b	v30, v6, v27
100ebff78:     	and.16b	v31, v4, v27
100ebff7c:     	and.16b	v20, v20, v27
100ebff80:     	and.16b	v19, v19, v27
100ebff84:     	and.16b	v22, v22, v27
100ebff88:     	and.16b	v21, v21, v27
100ebff8c:     	and.16b	v24, v24, v27
100ebff90:     	and.16b	v23, v23, v27
100ebff94:     	neg.2d	v27, v31
100ebff98:     	ushl.2d	v27, v0, v27
100ebff9c:     	neg.2d	v30, v30
100ebffa0:     	ushl.2d	v30, v0, v30
100ebffa4:     	neg.2d	v19, v19
100ebffa8:     	ushl.2d	v19, v0, v19
100ebffac:     	neg.2d	v20, v20
100ebffb0:     	ushl.2d	v20, v0, v20
100ebffb4:     	neg.2d	v21, v21
100ebffb8:     	ushl.2d	v21, v0, v21
100ebffbc:     	neg.2d	v22, v22
100ebffc0:     	ushl.2d	v22, v0, v22
100ebffc4:     	neg.2d	v23, v23
100ebffc8:     	ushl.2d	v23, v0, v23
100ebffcc:     	neg.2d	v24, v24
100ebffd0:     	ushl.2d	v24, v0, v24
100ebffd4:     	dup.2d	v31, x28
100ebffd8:     	and.16b	v30, v30, v31
100ebffdc:     	and.16b	v27, v27, v31
100ebffe0:     	and.16b	v20, v20, v31
100ebffe4:     	and.16b	v19, v19, v31
100ebffe8:     	and.16b	v22, v22, v31
100ebffec:     	and.16b	v21, v21, v31
100ebfff0:     	and.16b	v24, v24, v31
100ebfff4:     	and.16b	v23, v23, v31
100ebfff8:     	and.16b	v25, v25, v8
100ebfffc:     	and.16b	v26, v26, v8
100ec0000:     	and.16b	v28, v28, v8
100ec0004:     	and.16b	v29, v29, v8
100ec0008:     	ushll2.2d	v31, v25, #0x0
100ec000c:     	ushll.2d	v25, v25, #0x0
100ec0010:     	ushll2.2d	v9, v26, #0x0
100ec0014:     	ushll.2d	v26, v26, #0x0
100ec0018:     	ushll2.2d	v10, v28, #0x0
100ec001c:     	ushll.2d	v28, v28, #0x0
100ec0020:     	ushll2.2d	v11, v29, #0x0
100ec0024:     	ushll.2d	v29, v29, #0x0
100ec0028:     	ushl.2d	v25, v27, v25
100ec002c:     	ushl.2d	v27, v30, v31
100ec0030:     	ushl.2d	v19, v19, v26
100ec0034:     	ushl.2d	v20, v20, v9
100ec0038:     	ushl.2d	v21, v21, v28
100ec003c:     	ushl.2d	v22, v22, v10
100ec0040:     	ushl.2d	v23, v23, v29
100ec0044:     	ushl.2d	v24, v24, v11
100ec0048:     	orr.16b	v3, v27, v3
100ec004c:     	orr.16b	v2, v25, v2
100ec0050:     	orr.16b	v16, v20, v16
100ec0054:     	orr.16b	v7, v19, v7
100ec0058:     	orr.16b	v18, v22, v18
100ec005c:     	orr.16b	v5, v21, v5
100ec0060:     	orr.16b	v1, v24, v1
100ec0064:     	orr.16b	v17, v23, v17
100ec0068:     	add.2d	v6, v6, v15
100ec006c:     	add.2d	v4, v4, v15
100ec0070:     	subs	x9, x9, #0x10
100ec0074:     	b.ne	0x100ebff50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x868>
100ec0078:     	orr.16b	v2, v7, v2
100ec007c:     	orr.16b	v3, v16, v3
100ec0080:     	orr.16b	v3, v18, v3
100ec0084:     	orr.16b	v2, v5, v2
100ec0088:     	orr.16b	v2, v17, v2
100ec008c:     	orr.16b	v1, v1, v3
100ec0090:     	orr.16b	v1, v2, v1
100ec0094:     	mov	d2, v1[1]
100ec0098:     	orr.8b	v1, v1, v2
100ec009c:     	fmov	x8, d1
100ec00a0:     	and	x9, x26, #0xfffffffffffffff0
100ec00a4:     	cmp	x26, x9
100ec00a8:     	movi.2s	v8, #0x3f
100ec00ac:     	b.eq	0x100ec0154 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100ec00b0:     	and	x10, x26, #0xfffffffffffffff0
100ec00b4:     	and	x9, x26, #0xfffffffffffffff0
100ec00b8:     	and	x11, x26, #0xe
100ec00bc:     	cbz	x11, 0x100ec0134 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100ec00c0:     	fmov	d1, x8
100ec00c4:     	dup.2d	v2, x10
100ec00c8:     	ldr	q3, [sp, #0xa0]
100ec00cc:     	orr.16b	v2, v2, v3
100ec00d0:     	ldr	x8, [sp, #0x98]
100ec00d4:     	add	x8, x8, x10
100ec00d8:     	add	x9, x24, x10, lsl #2
100ec00dc:     	ldr	q6, [sp, #0x80]
100ec00e0:     	ldr	q7, [sp, #0x60]
100ec00e4:     	ldr	d3, [x9], #0x8
100ec00e8:     	and.16b	v4, v2, v7
100ec00ec:     	neg.2d	v4, v4
100ec00f0:     	ushl.2d	v4, v0, v4
100ec00f4:     	dup.2d	v5, x28
100ec00f8:     	and.16b	v4, v4, v5
100ec00fc:     	and.8b	v3, v3, v8
100ec0100:     	ushll.2d	v3, v3, #0x0
100ec0104:     	ushl.2d	v3, v4, v3
100ec0108:     	orr.16b	v1, v3, v1
100ec010c:     	add.2d	v2, v2, v6
100ec0110:     	adds	x8, x8, #0x2
100ec0114:     	b.ne	0x100ec00e4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9fc>
100ec0118:     	mov	d0, v1[1]
100ec011c:     	orr.8b	v0, v1, v0
100ec0120:     	fmov	x8, d0
100ec0124:     	and	x9, x26, #0xfffffffffffffffe
100ec0128:     	and	x10, x26, #0xfffffffffffffffe
100ec012c:     	cmp	x26, x10
100ec0130:     	b.eq	0x100ec0154 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100ec0134:     	ldr	w10, [x24, x9, lsl #2]
100ec0138:     	lsr	x11, x19, x9
100ec013c:     	and	x11, x11, #0x1
100ec0140:     	lsl	x10, x11, x10
100ec0144:     	orr	x8, x10, x8
100ec0148:     	add	x9, x9, #0x1
100ec014c:     	cmp	x26, x9
100ec0150:     	b.ne	0x100ec0134 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100ec0154:     	orr	x2, x8, x25
100ec0158:     	mov	x0, x23
100ec015c:     	ldr	x1, [sp, #0x78]
100ec0160:     	bl	0x100fa5938 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100ec0164:     	cbz	w0, 0x100ebfed4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100ec0168:     	lsr	x0, x19, #6
100ec016c:     	cmp	x0, x22
100ec0170:     	b.hs	0x100ec0530 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe48>
100ec0174:     	lsl	x8, x28, x19
100ec0178:     	ldr	x9, [x21, x0, lsl #3]
100ec017c:     	orr	x8, x9, x8
100ec0180:     	str	x8, [x21, x0, lsl #3]
100ec0184:     	b	0x100ebfed4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100ec0188:     	ldr	x8, [sp, #0x58]
100ec018c:     	stp	x22, x21, [x8]
100ec0190:     	stp	x22, xzr, [x8, #0x10]
100ec0194:     	cbz	x26, 0x100ec0400 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100ec0198:     	mov	x0, x24
100ec019c:     	b	0x100ec03fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100ec01a0:     	mov	x8, #0x0                ; =0
100ec01a4:     	ldur	x21, [x29, #-0xb8]
100ec01a8:     	mov	w0, #0x8                ; =8
100ec01ac:     	ldr	x10, [sp, #0x98]
100ec01b0:     	ldr	x9, [x10, #0x48]
100ec01b4:     	add	x9, x9, x20
100ec01b8:     	str	x9, [x10, #0x48]
100ec01bc:     	stp	x8, x0, [x28]
100ec01c0:     	stp	x20, xzr, [x28, #0x10]
100ec01c4:     	cmp	x21, #0x1
100ec01c8:     	b.lt	0x100ec01d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaec>
100ec01cc:     	ldur	x0, [x29, #-0xb0]
100ec01d0:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec01d4:     	ldur	x8, [x29, #-0xe0]
100ec01d8:     	cmp	x8, #0x1
100ec01dc:     	b.lt	0x100ec0400 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100ec01e0:     	ldur	x0, [x29, #-0xd8]
100ec01e4:     	b	0x100ec03fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100ec01e8:     	ldr	x1, [sp, #0xa0]
100ec01ec:     	cbz	x1, 0x100ec0234 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb4c>
100ec01f0:     	lsl	x27, x1, #3
100ec01f4:     	mov	x0, x27
100ec01f8:     	mov	x19, x1
100ec01fc:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
100ec0200:     	cbz	x0, 0x100ec05d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xee8>
100ec0204:     	mov	x26, x0
100ec0208:     	mov	x1, x23
100ec020c:     	mov	x2, x27
100ec0210:     	bl	0x1016efa1c <dyld_stub_binder+0x1016efa1c>
100ec0214:     	cmn	x19, #0x1
100ec0218:     	b.eq	0x100ec04d8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdf0>
100ec021c:     	mov	x1, x19
100ec0220:     	mov	x23, x26
100ec0224:     	b	0x100ec023c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100ec0228:     	mov	w24, #0x4               ; =4
100ec022c:     	cbnz	x20, 0x100ebfbb8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d0>
100ec0230:     	b	0x100ebfbc0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100ec0234:     	mov	x19, #0x0               ; =0
100ec0238:     	mov	w23, #0x8               ; =8
100ec023c:     	mov	x0, x23
100ec0240:     	mov	x2, x24
100ec0244:     	mov	x3, x20
100ec0248:     	bl	0x101169400 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>
100ec024c:     	str	x19, [sp, #0x80]
100ec0250:     	ldr	x9, [sp, #0x98]
100ec0254:     	ldr	x8, [x9, #0x28]
100ec0258:     	add	x8, x8, #0x1
100ec025c:     	str	x8, [x9, #0x28]
100ec0260:     	mov	w8, #0x4                ; =4
100ec0264:     	stp	xzr, x8, [x29, #-0xb8]
100ec0268:     	stur	xzr, [x29, #-0xa8]
100ec026c:     	ldr	x8, [sp, #0x60]
100ec0270:     	bics	x22, x8, x21
100ec0274:     	b.eq	0x100ec0380 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc98>
100ec0278:     	mov	x24, x23
100ec027c:     	mov	x19, #0x0               ; =0
100ec0280:     	mov	w8, #0x4                ; =4
100ec0284:     	mov	w9, #0x1                ; =1
100ec0288:     	b	0x100ec02b0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbc8>
100ec028c:     	rbit	x9, x22
100ec0290:     	clz	x9, x9
100ec0294:     	str	w9, [x8, x19]
100ec0298:     	stur	x23, [x29, #-0xa8]
100ec029c:     	sub	x10, x22, #0x1
100ec02a0:     	add	x19, x19, #0x4
100ec02a4:     	add	x9, x23, #0x1
100ec02a8:     	ands	x22, x10, x22
100ec02ac:     	b.eq	0x100ec02d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbec>
100ec02b0:     	mov	x23, x9
100ec02b4:     	sub	x9, x9, #0x1
100ec02b8:     	ldur	x10, [x29, #-0xb8]
100ec02bc:     	cmp	x9, x10
100ec02c0:     	b.ne	0x100ec028c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100ec02c4:     	sub	x0, x29, #0xb8
100ec02c8:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100ec02cc:     	ldur	x8, [x29, #-0xb0]
100ec02d0:     	b	0x100ec028c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100ec02d4:     	ldp	x8, x28, [x29, #-0xb8]
100ec02d8:     	str	x8, [sp, #0x20]
100ec02dc:     	str	x28, [sp, #0x10]
100ec02e0:     	cbz	x23, 0x100ec0388 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca0>
100ec02e4:     	mov	w27, #0x1               ; =1
100ec02e8:     	mov	x1, x24
100ec02ec:     	b	0x100ec0318 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc30>
100ec02f0:     	orr	x21, x22, x21
100ec02f4:     	stur	x21, [x29, #-0xe0]
100ec02f8:     	ldr	x9, [sp, #0x98]
100ec02fc:     	ldr	x8, [x9, #0x30]
100ec0300:     	add	x8, x8, #0x1
100ec0304:     	str	x8, [x9, #0x30]
100ec0308:     	str	x26, [sp, #0x80]
100ec030c:     	mov	x1, x24
100ec0310:     	subs	x19, x19, #0x4
100ec0314:     	b.eq	0x100ec038c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca4>
100ec0318:     	ldr	w8, [x28], #0x4
100ec031c:     	lsl	x22, x27, x8
100ec0320:     	sub	x8, x22, #0x1
100ec0324:     	and	x8, x8, x21
100ec0328:     	fmov	d0, x8
100ec032c:     	cnt.8b	v0, v0
100ec0330:     	addv.8b	b0, v0
100ec0334:     	fmov	w4, s0
100ec0338:     	fmov	d0, x21
100ec033c:     	cnt.8b	v0, v0
100ec0340:     	addv.8b	b0, v0
100ec0344:     	fmov	w3, s0
100ec0348:     	sub	x0, x29, #0xb8
100ec034c:     	mov	x23, x1
100ec0350:     	ldr	x2, [sp, #0xa0]
100ec0354:     	bl	0x10116a414 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100ec0358:     	ldp	x26, x24, [x29, #-0xb8]
100ec035c:     	ldur	x8, [x29, #-0xa8]
100ec0360:     	str	x8, [sp, #0xa0]
100ec0364:     	ldr	x8, [sp, #0x80]
100ec0368:     	sub	x8, x8, #0x1
100ec036c:     	cmn	x8, #0x3
100ec0370:     	b.hi	0x100ec02f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100ec0374:     	mov	x0, x23
100ec0378:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec037c:     	b	0x100ec02f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100ec0380:     	ldr	x19, [sp, #0x80]
100ec0384:     	b	0x100ec03a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcc0>
100ec0388:     	ldr	x26, [sp, #0x80]
100ec038c:     	ldr	x8, [sp, #0x20]
100ec0390:     	cbz	x8, 0x100ec039c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcb4>
100ec0394:     	ldr	x0, [sp, #0x10]
100ec0398:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec039c:     	mov	x19, x26
100ec03a0:     	mov	x23, x24
100ec03a4:     	ldr	x28, [sp, #0x58]
100ec03a8:     	ldr	x24, [sp, #0x30]
100ec03ac:     	ldr	x8, [sp, #0x60]
100ec03b0:     	cmp	x21, x8
100ec03b4:     	b.ne	0x100ec0474 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd8c>
100ec03b8:     	cmn	x19, #0x1
100ec03bc:     	b.ne	0x100ec03d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xce8>
100ec03c0:     	ldr	x9, [sp, #0x98]
100ec03c4:     	ldr	x8, [x9, #0x18]
100ec03c8:     	add	x8, x8, #0x1
100ec03cc:     	str	x8, [x9, #0x18]
100ec03d0:     	ldr	x8, [sp, #0x78]
100ec03d4:     	sbfx	x8, x8, #0, #1
100ec03d8:     	stp	x19, x23, [x28]
100ec03dc:     	ldr	x9, [sp, #0xa0]
100ec03e0:     	stp	x9, x8, [x28, #0x10]
100ec03e4:     	cbz	x20, 0x100ec03f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd08>
100ec03e8:     	mov	x0, x24
100ec03ec:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec03f0:     	ldr	w8, [sp, #0x40]
100ec03f4:     	tbnz	w8, #0x0, 0x100ec0400 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100ec03f8:     	mov	x0, x25
100ec03fc:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0400:     	ldp	x29, x30, [sp, #0x1d0]
100ec0404:     	ldp	x20, x19, [sp, #0x1c0]
100ec0408:     	ldp	x22, x21, [sp, #0x1b0]
100ec040c:     	ldp	x24, x23, [sp, #0x1a0]
100ec0410:     	ldp	x26, x25, [sp, #0x190]
100ec0414:     	ldp	x28, x27, [sp, #0x180]
100ec0418:     	ldp	d9, d8, [sp, #0x170]
100ec041c:     	ldp	d11, d10, [sp, #0x160]
100ec0420:     	ldp	d13, d12, [sp, #0x150]
100ec0424:     	ldp	d15, d14, [sp, #0x140]
100ec0428:     	add	sp, sp, #0x1e0
100ec042c:     	ret
100ec0430:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
100ec0434:     	add	x2, x2, #0x358
100ec0438:     	adrp	x5, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec043c:     	add	x5, x5, #0xcc0
100ec0440:     	sub	x1, x29, #0xb8
100ec0444:     	mov	w0, #0x0                ; =0
100ec0448:     	mov	x3, #0x0                ; =0
100ec044c:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100ec0450:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
100ec0454:     	add	x2, x2, #0x358
100ec0458:     	adrp	x5, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec045c:     	add	x5, x5, #0xc60
100ec0460:     	sub	x1, x29, #0xb8
100ec0464:     	mov	w0, #0x0                ; =0
100ec0468:     	mov	x3, #0x0                ; =0
100ec046c:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100ec0470:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0474:     	adrp	x5, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec0478:     	add	x5, x5, #0xc48
100ec047c:     	sub	x1, x29, #0xe0
100ec0480:     	add	x2, sp, #0xb8
100ec0484:     	mov	w0, #0x0                ; =0
100ec0488:     	mov	x3, #0x0                ; =0
100ec048c:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100ec0490:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0494:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
100ec0498:     	add	x2, x2, #0x358
100ec049c:     	adrp	x5, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec04a0:     	add	x5, x5, #0xca8
100ec04a4:     	sub	x1, x29, #0xb8
100ec04a8:     	mov	w0, #0x0                ; =0
100ec04ac:     	mov	x3, #0x0                ; =0
100ec04b0:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100ec04b4:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
100ec04b8:     	add	x2, x2, #0x358
100ec04bc:     	adrp	x5, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec04c0:     	add	x5, x5, #0xc90
100ec04c4:     	sub	x1, x29, #0xc0
100ec04c8:     	mov	w0, #0x1                ; =1
100ec04cc:     	mov	x3, #0x0                ; =0
100ec04d0:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100ec04d4:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec04d8:     	adrp	x0, 0x101857000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x9a8>
100ec04dc:     	add	x0, x0, #0xe19
100ec04e0:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
100ec04e4:     	add	x2, x2, #0x358
100ec04e8:     	mov	x23, x26
100ec04ec:     	mov	w1, #0x28               ; =40
100ec04f0:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ec04f4:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec04f8:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
100ec04fc:     	add	x2, x2, #0x838
100ec0500:     	mov	x1, x26
100ec0504:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec0508:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
100ec050c:     	add	x2, x2, #0x838
100ec0510:     	mov	x1, x26
100ec0514:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec0518:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec051c:     	adrp	x2, 0x10194f000 <dyld_stub_binder+0x10194f000>
100ec0520:     	add	x2, x2, #0x30
100ec0524:     	mov	x1, x26
100ec0528:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec052c:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0530:     	adrp	x2, 0x10194a000 <dyld_stub_binder+0x10194a000>
100ec0534:     	add	x2, x2, #0xef8
100ec0538:     	mov	x1, x22
100ec053c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec0540:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0544:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100ec0548:     	add	x2, x2, #0xc10
100ec054c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec0550:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0554:     	mov	x19, x0
100ec0558:     	mov	x1, x16
100ec055c:     	b	0x100ec0564 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe7c>
100ec0560:     	mov	x19, x0
100ec0564:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100ec0568:     	add	x2, x2, #0x598
100ec056c:     	mov	x0, x8
100ec0570:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec0574:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0578:     	mov	w0, #0x8                ; =8
100ec057c:     	mov	x1, x19
100ec0580:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ec0584:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec0588:     	adrp	x2, 0x10194c000 <dyld_stub_binder+0x10194c000>
100ec058c:     	add	x2, x2, #0xc78
100ec0590:     	mov	x0, x19
100ec0594:     	mov	x1, x26
100ec0598:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ec059c:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec05a0:     	mov	w0, #0x4                ; =4
100ec05a4:     	mov	x1, x23
100ec05a8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ec05ac:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec05b0:     	mov	w0, #0x8                ; =8
100ec05b4:     	mov	x1, x19
100ec05b8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ec05bc:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec05c0:     	mov	w0, #0x4                ; =4
100ec05c4:     	mov	x1, x22
100ec05c8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ec05cc:     	b	0x100ec05e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100ec05d0:     	mov	x19, #-0x1              ; =-1
100ec05d4:     	mov	w0, #0x8                ; =8
100ec05d8:     	mov	x1, x27
100ec05dc:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ec05e0:     	brk	#0x1
100ec05e4:     	mov	x28, x0
100ec05e8:     	b	0x100ec0614 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf2c>
100ec05ec:     	mov	x28, x0
100ec05f0:     	b	0x100ec074c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1064>
100ec05f4:     	mov	x28, x0
100ec05f8:     	b	0x100ec072c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100ec05fc:     	mov	x28, x0
100ec0600:     	b	0x100ec0708 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1020>
100ec0604:     	b	0x100ec06a4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfbc>
100ec0608:     	mov	x28, x0
100ec060c:     	mov	x0, x24
100ec0610:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0614:     	cbz	x20, 0x100ec079c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100ec0618:     	mov	x0, x19
100ec061c:     	b	0x100ec0798 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100ec0620:     	b	0x100ec06fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1014>
100ec0624:     	mov	x28, x0
100ec0628:     	ldur	x8, [x29, #-0xb8]
100ec062c:     	cbnz	x8, 0x100ec0638 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf50>
100ec0630:     	mov	x23, x24
100ec0634:     	b	0x100ec06cc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100ec0638:     	ldur	x0, [x29, #-0xb0]
100ec063c:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0640:     	mov	x23, x24
100ec0644:     	b	0x100ec06cc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100ec0648:     	mov	x28, x0
100ec064c:     	mov	x0, x19
100ec0650:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0654:     	b	0x100ec071c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1034>
100ec0658:     	mov	x28, x0
100ec065c:     	ldur	x8, [x29, #-0xb8]
100ec0660:     	cbnz	x8, 0x100ec0670 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf88>
100ec0664:     	ldr	x23, [sp, #0x30]
100ec0668:     	ldr	x19, [sp, #0x80]
100ec066c:     	b	0x100ec0778 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100ec0670:     	ldur	x24, [x29, #-0xb0]
100ec0674:     	ldr	x23, [sp, #0x30]
100ec0678:     	ldr	x19, [sp, #0x80]
100ec067c:     	b	0x100ec0770 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100ec0680:     	mov	x28, x0
100ec0684:     	ldur	x8, [x29, #-0xb8]
100ec0688:     	cbnz	x8, 0x100ec0694 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfac>
100ec068c:     	ldr	x19, [sp, #0x80]
100ec0690:     	b	0x100ec0788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100ec0694:     	ldur	x0, [x29, #-0xb0]
100ec0698:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec069c:     	ldr	x19, [sp, #0x80]
100ec06a0:     	b	0x100ec0788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100ec06a4:     	mov	x28, x0
100ec06a8:     	ldur	x8, [x29, #-0xb8]
100ec06ac:     	cbz	x8, 0x100ec079c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100ec06b0:     	ldur	x0, [x29, #-0xb0]
100ec06b4:     	b	0x100ec0798 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100ec06b8:     	mov	x28, x0
100ec06bc:     	ldr	x8, [sp, #0x20]
100ec06c0:     	cbz	x8, 0x100ec06cc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100ec06c4:     	ldr	x0, [sp, #0x10]
100ec06c8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec06cc:     	ldr	x19, [sp, #0x80]
100ec06d0:     	ldr	x24, [sp, #0x30]
100ec06d4:     	cbnz	x20, 0x100ec0770 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100ec06d8:     	b	0x100ec0778 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100ec06dc:     	mov	x28, x0
100ec06e0:     	ldr	x8, [sp, #0x40]
100ec06e4:     	cbz	x8, 0x100ec06f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1008>
100ec06e8:     	ldr	x0, [sp, #0x30]
100ec06ec:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec06f0:     	mov	x23, x19
100ec06f4:     	mov	x19, x24
100ec06f8:     	b	0x100ec0788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100ec06fc:     	mov	x28, x0
100ec0700:     	mov	x0, x21
100ec0704:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0708:     	cbz	x26, 0x100ec079c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100ec070c:     	mov	x0, x24
100ec0710:     	b	0x100ec0798 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100ec0714:     	mov	x28, x0
100ec0718:     	ldur	x21, [x29, #-0xb8]
100ec071c:     	cmp	x21, #0x1
100ec0720:     	b.lt	0x100ec072c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100ec0724:     	ldur	x0, [x29, #-0xb0]
100ec0728:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec072c:     	ldur	x8, [x29, #-0xe0]
100ec0730:     	cmp	x8, #0x1
100ec0734:     	b.lt	0x100ec079c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100ec0738:     	ldur	x0, [x29, #-0xd8]
100ec073c:     	b	0x100ec0798 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100ec0740:     	mov	x28, x0
100ec0744:     	mov	x0, x27
100ec0748:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec074c:     	ldr	x23, [sp, #0x30]
100ec0750:     	ldr	x19, [sp, #0x80]
100ec0754:     	ldr	x8, [sp, #0x20]
100ec0758:     	cbnz	x8, 0x100ec0770 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100ec075c:     	b	0x100ec0778 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100ec0760:     	mov	x28, x0
100ec0764:     	b	0x100ec0788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100ec0768:     	mov	x28, x0
100ec076c:     	cbz	x20, 0x100ec0778 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100ec0770:     	mov	x0, x24
100ec0774:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0778:     	ldr	w8, [sp, #0x40]
100ec077c:     	tbnz	w8, #0x0, 0x100ec0788 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100ec0780:     	mov	x0, x25
100ec0784:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec0788:     	sub	x8, x19, #0x1
100ec078c:     	cmn	x8, #0x3
100ec0790:     	b.hi	0x100ec079c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100ec0794:     	mov	x0, x23
100ec0798:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100ec079c:     	mov	x0, x28
100ec07a0:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
