
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d21824 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into>:
100d21824:     	sub	sp, sp, #0xc0
100d21828:     	stp	x28, x27, [sp, #0x60]
100d2182c:     	stp	x26, x25, [sp, #0x70]
100d21830:     	stp	x24, x23, [sp, #0x80]
100d21834:     	stp	x22, x21, [sp, #0x90]
100d21838:     	stp	x20, x19, [sp, #0xa0]
100d2183c:     	stp	x29, x30, [sp, #0xb0]
100d21840:     	add	x29, sp, #0xb0
100d21844:     	str	x4, [sp, #0x40]
100d21848:     	cmp	w2, #0xb
100d2184c:     	b.hi	0x100d21b4c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x328>
100d21850:     	cmp	w3, w2
100d21854:     	b.hi	0x100d21b4c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x328>
100d21858:     	str	x1, [sp, #0x48]
100d2185c:     	mov	w8, #0x1                ; =1
100d21860:     	lsl	x8, x8, x2
100d21864:     	lsr	x8, x8, #6
100d21868:     	cmp	w2, #0x6
100d2186c:     	cinc	x8, x8, lo
100d21870:     	str	x8, [sp, #0x58]
100d21874:     	cmp	x1, x8
100d21878:     	b.ne	0x100d21b64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x340>
100d2187c:     	add	w8, w2, #0x1
100d21880:     	mov	w9, #0x1                ; =1
100d21884:     	lsl	x10, x9, x8
100d21888:     	mov	w9, #0x3e               ; =62
100d2188c:     	lsr	x8, x9, x8
100d21890:     	and	x8, x8, #0x1
100d21894:     	add	x8, x8, x10, lsr #6
100d21898:     	stp	x5, x8, [sp, #0x50]
100d2189c:     	cmp	x5, x8
100d218a0:     	b.ne	0x100d21b80 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x35c>
100d218a4:     	mov	x14, x0
100d218a8:     	cmp	w3, #0x5
100d218ac:     	str	x1, [sp, #0x38]
100d218b0:     	b.ls	0x100d219c4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x1a0>
100d218b4:     	add	w8, w3, #0x3a
100d218b8:     	and	w23, w8, #0x3f
100d218bc:     	cmp	w23, #0x3f
100d218c0:     	b.eq	0x100d21b9c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x378>
100d218c4:     	stp	x10, x5, [sp]
100d218c8:     	str	x2, [sp, #0x10]
100d218cc:     	mov	w9, #0x2                ; =2
100d218d0:     	lsl	x19, x9, x23
100d218d4:     	add	x9, x23, #0x1
100d218d8:     	lsr	x9, x5, x9
100d218dc:     	add	x10, x19, #0x7f
100d218e0:     	tst	x10, x5
100d218e4:     	cinc	x9, x9, ne
100d218e8:     	cmp	x5, #0x0
100d218ec:     	csel	x9, xzr, x9, eq
100d218f0:     	cbz	x1, 0x100d21a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x204>
100d218f4:     	mov	w10, #0x1               ; =1
100d218f8:     	lsl	x24, x10, x8
100d218fc:     	lsr	x8, x1, x23
100d21900:     	mov	x10, #0xfffffffffffffff ; =1152921504606846975
100d21904:     	add	x10, x24, x10
100d21908:     	tst	x10, x1
100d2190c:     	cinc	x8, x8, ne
100d21910:     	cmp	x8, x9
100d21914:     	csel	x8, x8, x9, lo
100d21918:     	str	x8, [sp, #0x30]
100d2191c:     	cbz	x8, 0x100d21a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x204>
100d21920:     	mov	x22, #0x0               ; =0
100d21924:     	mov	w8, #0x8                ; =8
100d21928:     	lsl	x9, x8, x23
100d2192c:     	lsl	x8, x24, #3
100d21930:     	stp	x8, x9, [sp, #0x20]
100d21934:     	lsl	x8, x19, #3
100d21938:     	str	x8, [sp, #0x18]
100d2193c:     	ldr	x25, [sp, #0x8]
100d21940:     	ldr	x26, [sp, #0x40]
100d21944:     	cmp	x19, x25
100d21948:     	csel	x2, x19, x25, lo
100d2194c:     	lsl	x8, x22, x23
100d21950:     	sub	x9, x1, x8
100d21954:     	cmp	x24, x9
100d21958:     	csel	x27, x24, x9, lo
100d2195c:     	subs	x28, x2, x24
100d21960:     	b.lo	0x100d21b10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x2ec>
100d21964:     	cmp	x24, x9
100d21968:     	b.hi	0x100d21b24 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x300>
100d2196c:     	mov	x20, x14
100d21970:     	add	x21, x14, x8, lsl #3
100d21974:     	mov	x0, x26
100d21978:     	mov	x1, x21
100d2197c:     	ldr	x2, [sp, #0x28]
100d21980:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d21984:     	cmp	x28, x27
100d21988:     	b.ne	0x100d21b38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x314>
100d2198c:     	add	x22, x22, #0x1
100d21990:     	lsl	x2, x27, #3
100d21994:     	ldr	x8, [sp, #0x20]
100d21998:     	add	x0, x26, x8
100d2199c:     	mov	x1, x21
100d219a0:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d219a4:     	ldr	x8, [sp, #0x18]
100d219a8:     	add	x26, x26, x8
100d219ac:     	sub	x25, x25, x19
100d219b0:     	ldp	x8, x1, [sp, #0x30]
100d219b4:     	cmp	x8, x22
100d219b8:     	mov	x14, x20
100d219bc:     	b.ne	0x100d21944 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x120>
100d219c0:     	b	0x100d21a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x204>
100d219c4:     	cbz	x5, 0x100d21a5c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x238>
100d219c8:     	stp	x10, x5, [sp]
100d219cc:     	str	x2, [sp, #0x10]
100d219d0:     	mov	w8, #0x1                ; =1
100d219d4:     	lsl	w8, w8, w3
100d219d8:     	cmp	w3, #0x5
100d219dc:     	mov	x9, #0x0                ; =0
100d219e0:     	b.ne	0x100d21a84 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x260>
100d219e4:     	mov	x11, #0x0               ; =0
100d219e8:     	lsl	x10, x5, #5
100d219ec:     	lsr	x0, x11, #1
100d219f0:     	cmp	x0, x1
100d219f4:     	b.hs	0x100d21bb4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x390>
100d219f8:     	ldr	x12, [x14, x0, lsl #3]
100d219fc:     	and	x13, x9, #0x20
100d21a00:     	lsr	x12, x12, x13
100d21a04:     	mov	w12, w12
100d21a08:     	lsl	x13, x12, x8
100d21a0c:     	orr	x12, x13, x12
100d21a10:     	ldr	x13, [sp, #0x40]
100d21a14:     	str	x12, [x13, x11, lsl #3]
100d21a18:     	add	x11, x11, #0x1
100d21a1c:     	add	x9, x9, #0x20
100d21a20:     	cmp	x10, x9
100d21a24:     	b.ne	0x100d219ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x1c8>
100d21a28:     	ldr	x8, [sp, #0x10]
100d21a2c:     	cmp	w8, #0x5
100d21a30:     	b.hs	0x100d21a64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x240>
100d21a34:     	ldr	x8, [sp, #0x8]
100d21a38:     	cbz	x8, 0x100d21bc4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x3a0>
100d21a3c:     	mov	x8, #-0x1               ; =-1
100d21a40:     	ldr	x9, [sp]
100d21a44:     	lsl	x8, x8, x9
100d21a48:     	ldr	x10, [sp, #0x40]
100d21a4c:     	ldr	x9, [x10]
100d21a50:     	bic	x8, x9, x8
100d21a54:     	str	x8, [x10]
100d21a58:     	b	0x100d21a64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x240>
100d21a5c:     	cmp	w2, #0x5
100d21a60:     	b.lo	0x100d21bc4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x3a0>
100d21a64:     	ldp	x29, x30, [sp, #0xb0]
100d21a68:     	ldp	x20, x19, [sp, #0xa0]
100d21a6c:     	ldp	x22, x21, [sp, #0x90]
100d21a70:     	ldp	x24, x23, [sp, #0x80]
100d21a74:     	ldp	x26, x25, [sp, #0x70]
100d21a78:     	ldp	x28, x27, [sp, #0x60]
100d21a7c:     	add	sp, sp, #0xc0
100d21a80:     	ret
100d21a84:     	mov	x10, #0x0               ; =0
100d21a88:     	lsl	x11, x5, #5
100d21a8c:     	b	0x100d21ab0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x28c>
100d21a90:     	lsl	x13, x12, x8
100d21a94:     	orr	x12, x13, x12
100d21a98:     	ldr	x13, [sp, #0x40]
100d21a9c:     	str	x12, [x13, x10, lsl #3]
100d21aa0:     	add	x9, x9, #0x20
100d21aa4:     	add	x10, x10, #0x1
100d21aa8:     	cmp	x11, x9
100d21aac:     	b.eq	0x100d21a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x204>
100d21ab0:     	lsr	x0, x10, #1
100d21ab4:     	cmp	x0, x1
100d21ab8:     	b.hs	0x100d21bb4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x390>
100d21abc:     	ldr	x12, [x14, x0, lsl #3]
100d21ac0:     	and	x13, x9, #0x20
100d21ac4:     	lsr	x12, x12, x13
100d21ac8:     	bfi	x12, x12, #16, #48
100d21acc:     	and	x12, x12, #0xffff0000ffff
100d21ad0:     	cmp	w3, #0x3
100d21ad4:     	b.hi	0x100d21a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x26c>
100d21ad8:     	orr	x12, x12, x12, lsl #8
100d21adc:     	and	x12, x12, #0xff00ff00ff00ff
100d21ae0:     	cmp	w3, #0x3
100d21ae4:     	b.eq	0x100d21a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x26c>
100d21ae8:     	orr	x12, x12, x12, lsl #4
100d21aec:     	and	x12, x12, #0xf0f0f0f0f0f0f0f
100d21af0:     	cmp	w3, #0x1
100d21af4:     	b.hi	0x100d21a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x26c>
100d21af8:     	orr	x12, x12, x12, lsl #2
100d21afc:     	and	x12, x12, #0x3333333333333333
100d21b00:     	cbnz	w3, 0x100d21a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x26c>
100d21b04:     	orr	x12, x12, x12, lsl #1
100d21b08:     	and	x12, x12, #0x5555555555555555
100d21b0c:     	b	0x100d21a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch14broadcast_into+0x26c>
100d21b10:     	adrp	x3, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b14:     	add	x3, x3, #0xf88
100d21b18:     	mov	x0, #0x0                ; =0
100d21b1c:     	mov	x1, x24
100d21b20:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d21b24:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b28:     	add	x2, x2, #0xf58
100d21b2c:     	mov	x0, x24
100d21b30:     	mov	x1, x27
100d21b34:     	bl	0x1012887b8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d21b38:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b3c:     	add	x2, x2, #0xf70
100d21b40:     	mov	x0, x28
100d21b44:     	mov	x1, x27
100d21b48:     	bl	0x1012887b8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d21b4c:     	adrp	x0, 0x10134f000 <dyld_stub_binder+0x10134f000>
100d21b50:     	add	x0, x0, #0xfef
100d21b54:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b58:     	add	x2, x2, #0xec8
100d21b5c:     	mov	w1, #0x31               ; =49
100d21b60:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d21b64:     	adrp	x5, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b68:     	add	x5, x5, #0xee0
100d21b6c:     	add	x1, sp, #0x48
100d21b70:     	add	x2, sp, #0x58
100d21b74:     	mov	w0, #0x0                ; =0
100d21b78:     	mov	x3, #0x0                ; =0
100d21b7c:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d21b80:     	adrp	x5, 0x101510000 <dyld_stub_binder+0x101510000>
100d21b84:     	add	x5, x5, #0xef8
100d21b88:     	add	x1, sp, #0x50
100d21b8c:     	add	x2, sp, #0x58
100d21b90:     	mov	w0, #0x0                ; =0
100d21b94:     	mov	x3, #0x0                ; =0
100d21b98:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d21b9c:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100d21ba0:     	add	x0, x0, #0xc45
100d21ba4:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21ba8:     	add	x2, x2, #0xf28
100d21bac:     	mov	w1, #0x37               ; =55
100d21bb0:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d21bb4:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21bb8:     	add	x2, x2, #0xf10
100d21bbc:     	ldr	x1, [sp, #0x38]
100d21bc0:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d21bc4:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21bc8:     	add	x2, x2, #0xf40
100d21bcc:     	mov	x0, #0x0                ; =0
100d21bd0:     	mov	x1, #0x0                ; =0
100d21bd4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
