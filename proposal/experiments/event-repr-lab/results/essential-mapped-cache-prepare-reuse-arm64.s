
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001007ae5ac <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_>:
1007ae5ac:     	sub	sp, sp, #0xa0
1007ae5b0:     	stp	x28, x27, [sp, #0x40]
1007ae5b4:     	stp	x26, x25, [sp, #0x50]
1007ae5b8:     	stp	x24, x23, [sp, #0x60]
1007ae5bc:     	stp	x22, x21, [sp, #0x70]
1007ae5c0:     	stp	x20, x19, [sp, #0x80]
1007ae5c4:     	stp	x29, x30, [sp, #0x90]
1007ae5c8:     	add	x29, sp, #0x90
1007ae5cc:     	ldr	w24, [x2, #0x10]
1007ae5d0:     	cmp	w24, #0x2
1007ae5d4:     	b.lo	0x1007ae87c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
1007ae5d8:     	mov	x19, x4
1007ae5dc:     	mov	x20, x0
1007ae5e0:     	ldr	x8, [x0, #0x8]
1007ae5e4:     	cbz	x8, 0x1007ae5f4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x48>
1007ae5e8:     	ldr	x0, [x20]
1007ae5ec:     	mov	x6, x8
1007ae5f0:     	b	0x1007ae6c0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x114>
1007ae5f4:     	sub	x8, x6, #0x1
1007ae5f8:     	eor	x9, x6, x8
1007ae5fc:     	cmp	x9, x8
1007ae600:     	b.ls	0x1007ae904 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x358>
1007ae604:     	lsl	x21, x6, #6
1007ae608:     	lsr	x8, x6, #58
1007ae60c:     	mov	x9, #-0x7               ; =-7
1007ae610:     	movk	x9, #0x7fff, lsl #48
1007ae614:     	cmp	x21, x9
1007ae618:     	ccmp	x8, #0x0, #0x0, lo
1007ae61c:     	b.eq	0x1007ae624 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x78>
1007ae620:     	bl	0x1013b9b90 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1007ae624:     	cbz	x21, 0x1007ae688 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xdc>
1007ae628:     	mov	x0, x21
1007ae62c:     	mov	x22, x7
1007ae630:     	mov	x26, x5
1007ae634:     	mov	x23, x3
1007ae638:     	mov	x27, x2
1007ae63c:     	mov	x25, x1
1007ae640:     	mov	x28, x6
1007ae644:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
1007ae648:     	mov	x6, x28
1007ae64c:     	mov	x1, x25
1007ae650:     	mov	x2, x27
1007ae654:     	mov	x3, x23
1007ae658:     	mov	x5, x26
1007ae65c:     	mov	x7, x22
1007ae660:     	cbz	x0, 0x1007ae91c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x370>
1007ae664:     	cmp	x6, #0x3
1007ae668:     	b.hi	0x1007ae694 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xe8>
1007ae66c:     	mov	w8, #0xff               ; =255
1007ae670:     	mov	x9, x0
1007ae674:     	mov	x10, x6
1007ae678:     	strb	w8, [x9], #0x40
1007ae67c:     	subs	x10, x10, #0x1
1007ae680:     	b.ne	0x1007ae678 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xcc>
1007ae684:     	b	0x1007ae6bc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x110>
1007ae688:     	mov	w0, #0x8                ; =8
1007ae68c:     	cmp	x6, #0x3
1007ae690:     	b.ls	0x1007ae66c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xc0>
1007ae694:     	and	x8, x6, #0x3fffffffffffffc
1007ae698:     	add	x9, x0, #0x80
1007ae69c:     	mov	w10, #0xff              ; =255
1007ae6a0:     	sturb	w10, [x9, #-0x80]
1007ae6a4:     	sturb	w10, [x9, #-0x40]
1007ae6a8:     	strb	w10, [x9]
1007ae6ac:     	strb	w10, [x9, #0x40]
1007ae6b0:     	add	x9, x9, #0x100
1007ae6b4:     	subs	x8, x8, #0x4
1007ae6b8:     	b.ne	0x1007ae6a0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xf4>
1007ae6bc:     	stp	x0, x6, [x20]
1007ae6c0:     	ldp	x11, x12, [x2]
1007ae6c4:     	ldr	w10, [x2, #0x14]
1007ae6c8:     	mov	x8, #0xa9c5             ; =43461
1007ae6cc:     	movk	x8, #0x2e62, lsl #16
1007ae6d0:     	movk	x8, #0x7aea, lsl #32
1007ae6d4:     	movk	x8, #0xf135, lsl #48
1007ae6d8:     	madd	x9, x24, x8, x11
1007ae6dc:     	madd	x9, x9, x8, x12
1007ae6e0:     	madd	x9, x9, x8, x19
1007ae6e4:     	mul	x8, x9, x8
1007ae6e8:     	sub	x9, x6, #0x1
1007ae6ec:     	and	x8, x9, x8, ror #44
1007ae6f0:     	add	x28, x0, x8, lsl #6
1007ae6f4:     	ldrb	w8, [x28]
1007ae6f8:     	cmp	w8, #0xff
1007ae6fc:     	str	w10, [sp, #0x1c]
1007ae700:     	stp	x12, x11, [sp, #0x8]
1007ae704:     	b.ne	0x1007ae724 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x178>
1007ae708:     	ldr	x8, [x7, #0x58]
1007ae70c:     	add	x8, x8, #0x1
1007ae710:     	str	x8, [x7, #0x58]
1007ae714:     	add	x8, x28, #0x10
1007ae718:     	str	x8, [sp]
1007ae71c:     	add	x25, x28, #0x18
1007ae720:     	b	0x1007ae7e8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
1007ae724:     	ldr	x9, [x28, #0x38]
1007ae728:     	cmp	x9, x19
1007ae72c:     	b.ne	0x1007ae764 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
1007ae730:     	ldr	x9, [x28, #0x20]
1007ae734:     	cmp	x9, x11
1007ae738:     	b.ne	0x1007ae764 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
1007ae73c:     	ldr	x9, [x28, #0x28]
1007ae740:     	cmp	x9, x12
1007ae744:     	b.ne	0x1007ae764 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
1007ae748:     	ldr	w9, [x28, #0x30]
1007ae74c:     	cmp	w9, w24
1007ae750:     	b.ne	0x1007ae764 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
1007ae754:     	ldr	x8, [x7, #0x50]
1007ae758:     	add	x8, x8, #0x1
1007ae75c:     	str	x8, [x7, #0x50]
1007ae760:     	b	0x1007ae87c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
1007ae764:     	ldr	x9, [x7, #0x58]
1007ae768:     	add	x9, x9, #0x1
1007ae76c:     	str	x9, [x7, #0x58]
1007ae770:     	mov	x25, x28
1007ae774:     	ldr	x9, [x25, #0x18]!
1007ae778:     	mov	w10, #0xff              ; =255
1007ae77c:     	strb	w10, [x28]
1007ae780:     	lsl	x10, x9, #3
1007ae784:     	cmp	w8, #0x2
1007ae788:     	csel	x10, x10, xzr, eq
1007ae78c:     	ldr	x11, [x20, #0x10]
1007ae790:     	sub	x10, x11, x10
1007ae794:     	mov	x11, x28
1007ae798:     	ldr	x0, [x11, #0x10]!
1007ae79c:     	str	x11, [sp]
1007ae7a0:     	cmp	w8, #0x2
1007ae7a4:     	ldr	x8, [x7, #0x60]
1007ae7a8:     	add	x8, x8, #0x1
1007ae7ac:     	str	x8, [x7, #0x60]
1007ae7b0:     	str	x10, [x20, #0x10]
1007ae7b4:     	ccmp	x9, #0x0, #0x4, hs
1007ae7b8:     	b.eq	0x1007ae7e8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
1007ae7bc:     	mov	x21, x7
1007ae7c0:     	mov	x26, x5
1007ae7c4:     	mov	x22, x3
1007ae7c8:     	mov	x27, x2
1007ae7cc:     	mov	x23, x1
1007ae7d0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007ae7d4:     	mov	x1, x23
1007ae7d8:     	mov	x2, x27
1007ae7dc:     	mov	x3, x22
1007ae7e0:     	mov	x5, x26
1007ae7e4:     	mov	x7, x21
1007ae7e8:     	add	x0, sp, #0x20
1007ae7ec:     	mov	x4, x19
1007ae7f0:     	mov	x6, x7
1007ae7f4:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007ae7f8:     	ldr	x8, [sp, #0x20]
1007ae7fc:     	cmn	x8, #0x1
1007ae800:     	b.eq	0x1007ae818 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x26c>
1007ae804:     	cmn	x8, #0x2
1007ae808:     	b.ne	0x1007ae89c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2f0>
1007ae80c:     	mov	w27, #0x0               ; =0
1007ae810:     	ldrb	w26, [sp, #0x28]
1007ae814:     	b	0x1007ae81c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x270>
1007ae818:     	mov	w27, #0x1               ; =1
1007ae81c:     	mov	x21, #0x0               ; =0
1007ae820:     	ldr	x8, [x20, #0x10]
1007ae824:     	add	x8, x8, x21
1007ae828:     	str	x8, [x20, #0x10]
1007ae82c:     	ldrb	w8, [x28]
1007ae830:     	cmp	w8, #0x2
1007ae834:     	b.ne	0x1007ae854 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
1007ae838:     	ldr	x8, [x25]
1007ae83c:     	cbz	x8, 0x1007ae854 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
1007ae840:     	ldr	x8, [sp]
1007ae844:     	ldr	x0, [x8]
1007ae848:     	mov	x20, x9
1007ae84c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007ae850:     	mov	x9, x20
1007ae854:     	strb	w27, [x28]
1007ae858:     	strb	w26, [x28, #0x1]
1007ae85c:     	str	w24, [x28, #0x4]
1007ae860:     	stp	x9, x22, [x28, #0x8]
1007ae864:     	ldp	x8, x9, [sp, #0x8]
1007ae868:     	stp	x23, x9, [x28, #0x18]
1007ae86c:     	str	x8, [x28, #0x28]
1007ae870:     	ldr	w8, [sp, #0x1c]
1007ae874:     	stp	w24, w8, [x28, #0x30]
1007ae878:     	str	x19, [x28, #0x38]
1007ae87c:     	ldp	x29, x30, [sp, #0x90]
1007ae880:     	ldp	x20, x19, [sp, #0x80]
1007ae884:     	ldp	x22, x21, [sp, #0x70]
1007ae888:     	ldp	x24, x23, [sp, #0x60]
1007ae88c:     	ldp	x26, x25, [sp, #0x50]
1007ae890:     	ldp	x28, x27, [sp, #0x40]
1007ae894:     	add	sp, sp, #0xa0
1007ae898:     	ret
1007ae89c:     	ldp	x27, x23, [sp, #0x28]
1007ae8a0:     	ldr	x9, [sp, #0x38]
1007ae8a4:     	lsl	x21, x23, #3
1007ae8a8:     	cmp	x8, x23
1007ae8ac:     	b.ls	0x1007ae8e0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x334>
1007ae8b0:     	mov	x26, x9
1007ae8b4:     	cbz	x23, 0x1007ae8ec <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x340>
1007ae8b8:     	mov	x0, x27
1007ae8bc:     	mov	x1, x21
1007ae8c0:     	bl	0x1013c2b14 <dyld_stub_binder+0x1013c2b14>
1007ae8c4:     	mov	x22, x0
1007ae8c8:     	mov	x9, x26
1007ae8cc:     	cbnz	x0, 0x1007ae8fc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x350>
1007ae8d0:     	mov	w0, #0x8                ; =8
1007ae8d4:     	mov	x1, x21
1007ae8d8:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007ae8dc:     	brk	#0x1
1007ae8e0:     	mov	x22, x27
1007ae8e4:     	mov	w27, #0x2               ; =2
1007ae8e8:     	b	0x1007ae820 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
1007ae8ec:     	mov	x0, x27
1007ae8f0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007ae8f4:     	mov	w22, #0x8               ; =8
1007ae8f8:     	mov	x9, x26
1007ae8fc:     	mov	w27, #0x2               ; =2
1007ae900:     	b	0x1007ae820 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
1007ae904:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007ae908:     	add	x0, x0, #0x2ef
1007ae90c:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007ae910:     	add	x2, x2, #0xd90
1007ae914:     	mov	w1, #0x2c               ; =44
1007ae918:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1007ae91c:     	mov	w0, #0x8                ; =8
1007ae920:     	mov	x1, x21
1007ae924:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007ae928:     	mov	x19, x0
1007ae92c:     	mov	x0, x27
1007ae930:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007ae934:     	mov	x0, x19
1007ae938:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
		...
