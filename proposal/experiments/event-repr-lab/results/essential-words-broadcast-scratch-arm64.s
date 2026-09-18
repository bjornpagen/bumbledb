
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d1e694 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>:
100d1e694:     	sub	sp, sp, #0xc0
100d1e698:     	stp	x28, x27, [sp, #0x60]
100d1e69c:     	stp	x26, x25, [sp, #0x70]
100d1e6a0:     	stp	x24, x23, [sp, #0x80]
100d1e6a4:     	stp	x22, x21, [sp, #0x90]
100d1e6a8:     	stp	x20, x19, [sp, #0xa0]
100d1e6ac:     	stp	x29, x30, [sp, #0xb0]
100d1e6b0:     	add	x29, sp, #0xb0
100d1e6b4:     	cmp	w3, #0xb
100d1e6b8:     	b.hi	0x100d1ea1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x388>
100d1e6bc:     	mov	x24, x4
100d1e6c0:     	mov	x22, x3
100d1e6c4:     	cmp	w4, w3
100d1e6c8:     	b.hi	0x100d1ea1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x388>
100d1e6cc:     	mov	x23, x2
100d1e6d0:     	mov	w8, #0x1                ; =1
100d1e6d4:     	lsl	x8, x8, x22
100d1e6d8:     	lsr	x8, x8, #6
100d1e6dc:     	cmp	w22, #0x6
100d1e6e0:     	cinc	x8, x8, lo
100d1e6e4:     	stp	x2, x8, [sp, #0x50]
100d1e6e8:     	cmp	x2, x8
100d1e6ec:     	b.ne	0x100d1ea34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3a0>
100d1e6f0:     	mov	x28, x1
100d1e6f4:     	mov	x21, x0
100d1e6f8:     	add	w8, w22, #0x1
100d1e6fc:     	mov	w9, #0x1                ; =1
100d1e700:     	lsl	x26, x9, x8
100d1e704:     	mov	w9, #0x3e               ; =62
100d1e708:     	lsr	x8, x9, x8
100d1e70c:     	and	x8, x8, #0x1
100d1e710:     	adds	x27, x8, x26, lsr #6
100d1e714:     	b.eq	0x100d1e764 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0xd0>
100d1e718:     	lsl	x25, x27, #3
100d1e71c:     	mov	x0, x25
100d1e720:     	mov	w1, #0x1                ; =1
100d1e724:     	bl	0x101290724 <dyld_stub_binder+0x101290724>
100d1e728:     	cbz	x0, 0x100d1ea64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3d0>
100d1e72c:     	mov	x20, x0
100d1e730:     	cmp	w24, #0x5
100d1e734:     	b.ls	0x100d1e880 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x1ec>
100d1e738:     	sub	w12, w24, #0x6
100d1e73c:     	mov	w8, #0x2                ; =2
100d1e740:     	lsl	x19, x8, x12
100d1e744:     	add	x8, x12, #0x1
100d1e748:     	lsr	x8, x27, x8
100d1e74c:     	sub	x9, x19, #0x1
100d1e750:     	tst	x27, x9
100d1e754:     	cinc	x8, x8, ne
100d1e758:     	str	x27, [sp, #0x20]
100d1e75c:     	cbnz	x23, 0x100d1e788 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0xf4>
100d1e760:     	b	0x100d1e8e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1e764:     	cmp	w24, #0x5
100d1e768:     	b.ls	0x100d1e908 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x274>
100d1e76c:     	str	xzr, [sp, #0x20]
100d1e770:     	mov	x8, #0x0                ; =0
100d1e774:     	sub	w12, w24, #0x6
100d1e778:     	mov	w9, #0x2                ; =2
100d1e77c:     	lsl	x19, x9, x12
100d1e780:     	mov	w20, #0x8               ; =8
100d1e784:     	cbz	x23, 0x100d1e8e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1e788:     	mov	w9, #0x1                ; =1
100d1e78c:     	lsl	x24, x9, x12
100d1e790:     	lsr	x9, x23, x12
100d1e794:     	mov	x10, #0xfffffffffffffff ; =1152921504606846975
100d1e798:     	add	x10, x24, x10
100d1e79c:     	tst	x10, x23
100d1e7a0:     	cinc	x9, x9, ne
100d1e7a4:     	cmp	x9, x8
100d1e7a8:     	csel	x8, x9, x8, lo
100d1e7ac:     	str	x8, [sp, #0x48]
100d1e7b0:     	cbz	x8, 0x100d1e8e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1e7b4:     	stp	x26, x22, [sp, #0x8]
100d1e7b8:     	str	x23, [sp, #0x40]
100d1e7bc:     	str	x21, [sp, #0x18]
100d1e7c0:     	mov	x21, #0x0               ; =0
100d1e7c4:     	add	x8, x12, #0x1
100d1e7c8:     	stp	x28, x8, [sp, #0x30]
100d1e7cc:     	mov	w8, #0x8                ; =8
100d1e7d0:     	lsl	x8, x8, x12
100d1e7d4:     	str	x8, [sp, #0x28]
100d1e7d8:     	ldp	x8, x10, [sp, #0x38]
100d1e7dc:     	lsl	x8, x21, x8
100d1e7e0:     	mov	x23, x27
100d1e7e4:     	sub	x11, x27, x8
100d1e7e8:     	cmp	x19, x11
100d1e7ec:     	csel	x28, x19, x11, lo
100d1e7f0:     	lsl	x9, x21, x12
100d1e7f4:     	sub	x10, x10, x9
100d1e7f8:     	cmp	x24, x10
100d1e7fc:     	csel	x27, x24, x10, lo
100d1e800:     	cmp	x11, x24
100d1e804:     	b.lo	0x100d1e9c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x334>
100d1e808:     	cmp	x24, x10
100d1e80c:     	b.hi	0x100d1e9e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x354>
100d1e810:     	mov	x26, x12
100d1e814:     	mov	x22, x20
100d1e818:     	add	x20, x20, x8, lsl #3
100d1e81c:     	ldp	x2, x8, [sp, #0x28]
100d1e820:     	add	x25, x8, x9, lsl #3
100d1e824:     	mov	x0, x20
100d1e828:     	mov	x1, x25
100d1e82c:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d1e830:     	sub	x8, x28, x24
100d1e834:     	cmp	x8, x27
100d1e838:     	b.ne	0x100d1e9f8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x364>
100d1e83c:     	add	x21, x21, #0x1
100d1e840:     	lsl	x2, x27, #3
100d1e844:     	add	x0, x20, x24, lsl #3
100d1e848:     	mov	x1, x25
100d1e84c:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d1e850:     	ldr	x8, [sp, #0x48]
100d1e854:     	cmp	x8, x21
100d1e858:     	mov	x20, x22
100d1e85c:     	mov	x27, x23
100d1e860:     	mov	x12, x26
100d1e864:     	b.ne	0x100d1e7d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x144>
100d1e868:     	ldp	x22, x21, [sp, #0x10]
100d1e86c:     	ldr	x26, [sp, #0x8]
100d1e870:     	ldr	x19, [sp, #0x20]
100d1e874:     	cmp	w22, #0x5
100d1e878:     	b.lo	0x100d1e8ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x258>
100d1e87c:     	b	0x100d1e918 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1e880:     	mov	w8, #0x1                ; =1
100d1e884:     	lsl	w8, w8, w24
100d1e888:     	cmp	w24, #0x5
100d1e88c:     	b.ne	0x100d1e940 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2ac>
100d1e890:     	mov	x9, #0x0                ; =0
100d1e894:     	mov	x10, #0x0               ; =0
100d1e898:     	lsr	x0, x10, #1
100d1e89c:     	cmp	x0, x23
100d1e8a0:     	b.hs	0x100d1ea50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3bc>
100d1e8a4:     	ldr	x11, [x28, x0, lsl #3]
100d1e8a8:     	and	x12, x9, #0x20
100d1e8ac:     	lsr	x11, x11, x12
100d1e8b0:     	mov	w11, w11
100d1e8b4:     	lsl	x12, x11, x8
100d1e8b8:     	orr	x11, x12, x11
100d1e8bc:     	str	x11, [x20, x10, lsl #3]
100d1e8c0:     	add	x10, x10, #0x1
100d1e8c4:     	add	x9, x9, #0x20
100d1e8c8:     	subs	x25, x25, #0x8
100d1e8cc:     	b.ne	0x100d1e898 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x204>
100d1e8d0:     	mov	x19, x27
100d1e8d4:     	cmp	w22, #0x5
100d1e8d8:     	b.lo	0x100d1e8ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x258>
100d1e8dc:     	b	0x100d1e918 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1e8e0:     	ldr	x19, [sp, #0x20]
100d1e8e4:     	cmp	w22, #0x5
100d1e8e8:     	b.hs	0x100d1e918 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1e8ec:     	cbz	x27, 0x100d1ea70 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3dc>
100d1e8f0:     	mov	x8, #-0x1               ; =-1
100d1e8f4:     	lsl	x8, x8, x26
100d1e8f8:     	ldr	x9, [x20]
100d1e8fc:     	bic	x8, x9, x8
100d1e900:     	str	x8, [x20]
100d1e904:     	b	0x100d1e918 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1e908:     	mov	x19, #0x0               ; =0
100d1e90c:     	mov	w20, #0x8               ; =8
100d1e910:     	cmp	w22, #0x5
100d1e914:     	b.lo	0x100d1ea70 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3dc>
100d1e918:     	stp	x19, x20, [x21]
100d1e91c:     	str	x27, [x21, #0x10]
100d1e920:     	ldp	x29, x30, [sp, #0xb0]
100d1e924:     	ldp	x20, x19, [sp, #0xa0]
100d1e928:     	ldp	x22, x21, [sp, #0x90]
100d1e92c:     	ldp	x24, x23, [sp, #0x80]
100d1e930:     	ldp	x26, x25, [sp, #0x70]
100d1e934:     	ldp	x28, x27, [sp, #0x60]
100d1e938:     	add	sp, sp, #0xc0
100d1e93c:     	ret
100d1e940:     	mov	x9, #0x0                ; =0
100d1e944:     	mov	x10, #0x0               ; =0
100d1e948:     	b	0x100d1e968 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2d4>
100d1e94c:     	lsl	x12, x11, x8
100d1e950:     	orr	x11, x12, x11
100d1e954:     	str	x11, [x20, x10, lsl #3]
100d1e958:     	add	x9, x9, #0x20
100d1e95c:     	add	x10, x10, #0x1
100d1e960:     	subs	x25, x25, #0x8
100d1e964:     	b.eq	0x100d1e8d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x23c>
100d1e968:     	lsr	x0, x10, #1
100d1e96c:     	cmp	x0, x23
100d1e970:     	b.hs	0x100d1ea50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3bc>
100d1e974:     	ldr	x11, [x28, x0, lsl #3]
100d1e978:     	and	x12, x9, #0x20
100d1e97c:     	lsr	x11, x11, x12
100d1e980:     	bfi	x11, x11, #16, #48
100d1e984:     	and	x11, x11, #0xffff0000ffff
100d1e988:     	cmp	w24, #0x3
100d1e98c:     	b.hi	0x100d1e94c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1e990:     	orr	x11, x11, x11, lsl #8
100d1e994:     	and	x11, x11, #0xff00ff00ff00ff
100d1e998:     	cmp	w24, #0x3
100d1e99c:     	b.eq	0x100d1e94c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1e9a0:     	orr	x11, x11, x11, lsl #4
100d1e9a4:     	and	x11, x11, #0xf0f0f0f0f0f0f0f
100d1e9a8:     	cmp	w24, #0x1
100d1e9ac:     	b.hi	0x100d1e94c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1e9b0:     	orr	x11, x11, x11, lsl #2
100d1e9b4:     	and	x11, x11, #0x3333333333333333
100d1e9b8:     	cbnz	w24, 0x100d1e94c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1e9bc:     	orr	x11, x11, x11, lsl #1
100d1e9c0:     	and	x11, x11, #0x5555555555555555
100d1e9c4:     	b	0x100d1e94c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1e9c8:     	adrp	x3, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e9cc:     	add	x3, x3, #0xbe0
100d1e9d0:     	mov	x0, #0x0                ; =0
100d1e9d4:     	mov	x1, x24
100d1e9d8:     	mov	x2, x28
100d1e9dc:     	ldr	x19, [sp, #0x20]
100d1e9e0:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d1e9e4:     	b	0x100d1ea84 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1e9e8:     	ldr	x19, [sp, #0x20]
100d1e9ec:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e9f0:     	add	x2, x2, #0xbb0
100d1e9f4:     	b	0x100d1ea0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x378>
100d1e9f8:     	mov	x24, x8
100d1e9fc:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1ea00:     	add	x2, x2, #0xbc8
100d1ea04:     	mov	x20, x22
100d1ea08:     	ldr	x19, [sp, #0x20]
100d1ea0c:     	mov	x0, x24
100d1ea10:     	mov	x1, x27
100d1ea14:     	bl	0x1012887b8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d1ea18:     	b	0x100d1ea84 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1ea1c:     	adrp	x0, 0x10134f000 <dyld_stub_binder+0x10134f000>
100d1ea20:     	add	x0, x0, #0xfef
100d1ea24:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1ea28:     	add	x2, x2, #0xb50
100d1ea2c:     	mov	w1, #0x31               ; =49
100d1ea30:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d1ea34:     	adrp	x5, 0x101510000 <dyld_stub_binder+0x101510000>
100d1ea38:     	add	x5, x5, #0xb68
100d1ea3c:     	add	x1, sp, #0x50
100d1ea40:     	add	x2, sp, #0x58
100d1ea44:     	mov	w0, #0x0                ; =0
100d1ea48:     	mov	x3, #0x0                ; =0
100d1ea4c:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d1ea50:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1ea54:     	add	x2, x2, #0xb80
100d1ea58:     	mov	x1, x23
100d1ea5c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1ea60:     	b	0x100d1ea84 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1ea64:     	mov	w0, #0x8                ; =8
100d1ea68:     	mov	x1, x25
100d1ea6c:     	bl	0x101287c24 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d1ea70:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1ea74:     	add	x2, x2, #0xb98
100d1ea78:     	mov	x0, #0x0                ; =0
100d1ea7c:     	mov	x1, #0x0                ; =0
100d1ea80:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1ea84:     	brk	#0x1
100d1ea88:     	b	0x100d1ea90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3fc>
100d1ea8c:     	cbz	x19, 0x100d1eaa0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x40c>
100d1ea90:     	mov	x19, x0
100d1ea94:     	mov	x0, x20
100d1ea98:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100d1ea9c:     	mov	x0, x19
100d1eaa0:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
