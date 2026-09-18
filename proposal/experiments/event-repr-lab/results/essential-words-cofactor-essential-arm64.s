
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba2798 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>:
100ba2798:     	stp	d15, d14, [sp, #-0xa0]!
100ba279c:     	stp	d13, d12, [sp, #0x10]
100ba27a0:     	stp	d11, d10, [sp, #0x20]
100ba27a4:     	stp	d9, d8, [sp, #0x30]
100ba27a8:     	stp	x28, x27, [sp, #0x40]
100ba27ac:     	stp	x26, x25, [sp, #0x50]
100ba27b0:     	stp	x24, x23, [sp, #0x60]
100ba27b4:     	stp	x22, x21, [sp, #0x70]
100ba27b8:     	stp	x20, x19, [sp, #0x80]
100ba27bc:     	stp	x29, x30, [sp, #0x90]
100ba27c0:     	add	x29, sp, #0x90
100ba27c4:     	sub	sp, sp, #0x280
100ba27c8:     	cmp	w4, w3
100ba27cc:     	b.hs	0x100ba31d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa40>
100ba27d0:     	mov	x24, x5
100ba27d4:     	mov	x23, x4
100ba27d8:     	mov	x25, x2
100ba27dc:     	mov	x22, x1
100ba27e0:     	mov	x21, x0
100ba27e4:     	sub	w8, w3, #0x1
100ba27e8:     	and	w28, w8, #0x3f
100ba27ec:     	mov	w9, #0x1                ; =1
100ba27f0:     	lsl	x27, x9, x8
100ba27f4:     	lsr	x8, x27, #6
100ba27f8:     	cmp	w28, #0x6
100ba27fc:     	cinc	x19, x8, lo
100ba2800:     	cbz	x19, 0x100ba28d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x140>
100ba2804:     	lsl	x26, x19, #3
100ba2808:     	mov	x0, x26
100ba280c:     	mov	w1, #0x1                ; =1
100ba2810:     	bl	0x1011099a4 <dyld_stub_binder+0x1011099a4>
100ba2814:     	cbz	x0, 0x100ba3230 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa98>
100ba2818:     	mov	x20, x0
100ba281c:     	cmp	w23, #0x5
100ba2820:     	str	x27, [sp, #0x208]
100ba2824:     	b.ls	0x100ba28e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x150>
100ba2828:     	add	w8, w23, #0x3a
100ba282c:     	and	w26, w8, #0x3f
100ba2830:     	cmp	w26, #0x3f
100ba2834:     	b.eq	0x100ba3200 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa68>
100ba2838:     	mov	w9, #0x1                ; =1
100ba283c:     	lsl	x23, x9, x8
100ba2840:     	mov	w9, #0x2                ; =2
100ba2844:     	lsl	x2, x9, x8
100ba2848:     	neg	x8, x2
100ba284c:     	and	x8, x25, x8
100ba2850:     	lsr	x9, x19, x26
100ba2854:     	sub	x10, x23, #0x1
100ba2858:     	tst	x19, x10
100ba285c:     	cinc	x9, x9, ne
100ba2860:     	cmp	x19, #0x0
100ba2864:     	csel	x9, xzr, x9, eq
100ba2868:     	add	x10, x26, #0x1
100ba286c:     	lsr	x8, x8, x10
100ba2870:     	cmp	x8, x9
100ba2874:     	csel	x25, x8, x9, lo
100ba2878:     	cbz	x25, 0x100ba316c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba287c:     	mov	w8, w24
100ba2880:     	lsl	x0, x8, x26
100ba2884:     	adds	x1, x0, x23
100ba2888:     	b.hs	0x100ba31f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100ba288c:     	cmp	x1, x2
100ba2890:     	b.hi	0x100ba31f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100ba2894:     	mov	x24, #0x0               ; =0
100ba2898:     	lsl	x27, x2, #3
100ba289c:     	add	x22, x22, x0, lsl #3
100ba28a0:     	lsl	x8, x24, x26
100ba28a4:     	sub	x9, x19, x8
100ba28a8:     	cmp	x23, x9
100ba28ac:     	csel	x0, x23, x9, lo
100ba28b0:     	b.hi	0x100ba31c4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa2c>
100ba28b4:     	add	x24, x24, #0x1
100ba28b8:     	lsl	x2, x0, #3
100ba28bc:     	add	x0, x20, x8, lsl #3
100ba28c0:     	mov	x1, x22
100ba28c4:     	bl	0x101109b9c <dyld_stub_binder+0x101109b9c>
100ba28c8:     	add	x22, x22, x27
100ba28cc:     	cmp	x25, x24
100ba28d0:     	b.ne	0x100ba28a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x108>
100ba28d4:     	b	0x100ba316c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba28d8:     	mov	w20, #0x8               ; =8
100ba28dc:     	cmp	w23, #0x5
100ba28e0:     	str	x27, [sp, #0x208]
100ba28e4:     	b.hi	0x100ba2828 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x90>
100ba28e8:     	cbz	x25, 0x100ba316c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba28ec:     	mov	x9, #0x0                ; =0
100ba28f0:     	mov	w8, w23
100ba28f4:     	dup.2d	v7, x8
100ba28f8:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba28fc:     	ldr	q0, [x10, #0x540]
100ba2900:     	ushl.2d	v0, v0, v7
100ba2904:     	stur	q0, [x29, #-0xb0]
100ba2908:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba290c:     	ldr	q0, [x10, #0x570]
100ba2910:     	ushl.2d	v0, v0, v7
100ba2914:     	stur	q0, [x29, #-0xc0]
100ba2918:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba291c:     	ldr	q0, [x10, #0x580]
100ba2920:     	ushl.2d	v0, v0, v7
100ba2924:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2928:     	ldr	q1, [x10, #0x590]
100ba292c:     	ushl.2d	v1, v1, v7
100ba2930:     	mov	w10, #0x3e              ; =62
100ba2934:     	dup.2d	v2, x10
100ba2938:     	and.16b	v3, v0, v2
100ba293c:     	and.16b	v0, v1, v2
100ba2940:     	stp	q0, q3, [x29, #-0xe0]
100ba2944:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2948:     	ldr	q0, [x10, #0x560]
100ba294c:     	ushl.2d	v0, v0, v7
100ba2950:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2954:     	ldr	q1, [x10, #0x5a0]
100ba2958:     	ushl.2d	v1, v1, v7
100ba295c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2960:     	ldr	q3, [x10, #0x5b0]
100ba2964:     	ushl.2d	v3, v3, v7
100ba2968:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba296c:     	ldr	q4, [x10, #0x5c0]
100ba2970:     	ushl.2d	v4, v4, v7
100ba2974:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2978:     	ldr	q16, [x10, #0x5d0]
100ba297c:     	ushl.2d	v16, v16, v7
100ba2980:     	and.16b	v5, v1, v2
100ba2984:     	and.16b	v1, v3, v2
100ba2988:     	stp	q1, q5, [x29, #-0x100]
100ba298c:     	and.16b	v3, v4, v2
100ba2990:     	and.16b	v1, v16, v2
100ba2994:     	stp	q1, q3, [sp, #0x190]
100ba2998:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba299c:     	ldr	q3, [x10, #0x5e0]
100ba29a0:     	ushl.2d	v3, v3, v7
100ba29a4:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29a8:     	ldr	q4, [x10, #0x5f0]
100ba29ac:     	ushl.2d	v4, v4, v7
100ba29b0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29b4:     	ldr	q16, [x10, #0x600]
100ba29b8:     	ushl.2d	v16, v16, v7
100ba29bc:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29c0:     	ldr	q17, [x10, #0x610]
100ba29c4:     	ushl.2d	v17, v17, v7
100ba29c8:     	and.16b	v5, v3, v2
100ba29cc:     	and.16b	v1, v4, v2
100ba29d0:     	stp	q1, q5, [sp, #0x170]
100ba29d4:     	and.16b	v3, v16, v2
100ba29d8:     	and.16b	v1, v17, v2
100ba29dc:     	stp	q1, q3, [sp, #0x150]
100ba29e0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29e4:     	ldr	q3, [x10, #0x620]
100ba29e8:     	ushl.2d	v3, v3, v7
100ba29ec:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29f0:     	ldr	q4, [x10, #0x630]
100ba29f4:     	ushl.2d	v4, v4, v7
100ba29f8:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba29fc:     	ldr	q16, [x10, #0x640]
100ba2a00:     	ushl.2d	v16, v16, v7
100ba2a04:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a08:     	ldr	q17, [x10, #0x650]
100ba2a0c:     	ushl.2d	v17, v17, v7
100ba2a10:     	and.16b	v5, v3, v2
100ba2a14:     	and.16b	v1, v4, v2
100ba2a18:     	stp	q5, q1, [sp, #0x110]
100ba2a1c:     	and.16b	v3, v16, v2
100ba2a20:     	and.16b	v1, v17, v2
100ba2a24:     	stp	q3, q1, [sp, #0x130]
100ba2a28:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a2c:     	ldr	q3, [x10, #0x660]
100ba2a30:     	ushl.2d	v3, v3, v7
100ba2a34:     	and.16b	v1, v3, v2
100ba2a38:     	str	q1, [sp, #0x100]
100ba2a3c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a40:     	ldr	q3, [x10, #0x680]
100ba2a44:     	ushl.2d	v3, v3, v7
100ba2a48:     	and.16b	v1, v3, v2
100ba2a4c:     	str	q1, [sp, #0xf0]
100ba2a50:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a54:     	ldr	q3, [x10, #0x690]
100ba2a58:     	ushl.2d	v3, v3, v7
100ba2a5c:     	and.16b	v1, v3, v2
100ba2a60:     	str	q1, [sp, #0xe0]
100ba2a64:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a68:     	ldr	q3, [x10, #0x6b0]
100ba2a6c:     	ushl.2d	v3, v3, v7
100ba2a70:     	and.16b	v1, v3, v2
100ba2a74:     	str	q1, [sp, #0xd0]
100ba2a78:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a7c:     	ldr	q3, [x10, #0x6c0]
100ba2a80:     	ushl.2d	v3, v3, v7
100ba2a84:     	and.16b	v1, v3, v2
100ba2a88:     	str	q1, [sp, #0xc0]
100ba2a8c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2a90:     	ldr	q3, [x10, #0x6e0]
100ba2a94:     	ushl.2d	v3, v3, v7
100ba2a98:     	and.16b	v1, v3, v2
100ba2a9c:     	str	q1, [sp, #0xb0]
100ba2aa0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2aa4:     	ldr	q3, [x10, #0x6f0]
100ba2aa8:     	ushl.2d	v3, v3, v7
100ba2aac:     	and.16b	v1, v3, v2
100ba2ab0:     	str	q1, [sp, #0xa0]
100ba2ab4:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2ab8:     	ldr	q3, [x10, #0x710]
100ba2abc:     	ushl.2d	v3, v3, v7
100ba2ac0:     	mov	w10, #0x1e              ; =30
100ba2ac4:     	dup.2d	v4, x10
100ba2ac8:     	and.16b	v1, v3, v4
100ba2acc:     	str	q1, [sp, #0x90]
100ba2ad0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2ad4:     	ldr	q3, [x10, #0x360]
100ba2ad8:     	ushl.2d	v3, v3, v7
100ba2adc:     	mov	w10, #0x2f              ; =47
100ba2ae0:     	dup.2d	v4, x10
100ba2ae4:     	and.16b	v1, v3, v4
100ba2ae8:     	str	q1, [sp, #0x70]
100ba2aec:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2af0:     	ldr	q3, [x10, #0x720]
100ba2af4:     	ushl.2d	v3, v3, v7
100ba2af8:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2afc:     	ldr	q4, [x10, #0x730]
100ba2b00:     	ushl.2d	v4, v4, v7
100ba2b04:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2b08:     	ldr	q16, [x10, #0x740]
100ba2b0c:     	ushl.2d	v16, v16, v7
100ba2b10:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2b14:     	ldr	q17, [x10, #0x750]
100ba2b18:     	ushl.2d	v17, v17, v7
100ba2b1c:     	and.16b	v1, v3, v2
100ba2b20:     	str	q1, [sp, #0x80]
100ba2b24:     	and.16b	v26, v4, v2
100ba2b28:     	and.16b	v27, v16, v2
100ba2b2c:     	and.16b	v28, v17, v2
100ba2b30:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b34:     	ldr	q2, [x10, #0x580]
100ba2b38:     	ushl.2d	v2, v2, v7
100ba2b3c:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b40:     	ldr	q3, [x10, #0x760]
100ba2b44:     	ushl.2d	v3, v3, v7
100ba2b48:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b4c:     	ldr	q4, [x10, #0x750]
100ba2b50:     	ushl.2d	v4, v4, v7
100ba2b54:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b58:     	ldr	q16, [x10, #0x740]
100ba2b5c:     	ushl.2d	v23, v16, v7
100ba2b60:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b64:     	ldr	q17, [x10, #0x730]
100ba2b68:     	ushl.2d	v16, v17, v7
100ba2b6c:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b70:     	ldr	q18, [x10, #0x720]
100ba2b74:     	ushl.2d	v17, v18, v7
100ba2b78:     	adrp	x10, 0x101195000 <GCC_except_table8962+0x60>
100ba2b7c:     	ldr	q19, [x10, #0x710]
100ba2b80:     	ushl.2d	v18, v19, v7
100ba2b84:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2b88:     	ldr	q20, [x10, #0x2f0]
100ba2b8c:     	ushl.2d	v20, v20, v7
100ba2b90:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2b94:     	ldr	q21, [x10, #0x300]
100ba2b98:     	ushl.2d	v21, v21, v7
100ba2b9c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2ba0:     	ldr	q22, [x10, #0x1f0]
100ba2ba4:     	ushl.2d	v22, v22, v7
100ba2ba8:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bac:     	ldr	q5, [x10, #0x310]
100ba2bb0:     	ushl.2d	v5, v5, v7
100ba2bb4:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bb8:     	ldr	q6, [x10, #0x320]
100ba2bbc:     	ushl.2d	v6, v6, v7
100ba2bc0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bc4:     	ldr	q24, [x10, #0x330]
100ba2bc8:     	ushl.2d	v24, v24, v7
100ba2bcc:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bd0:     	ldr	q25, [x10, #0x340]
100ba2bd4:     	ushl.2d	v25, v25, v7
100ba2bd8:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bdc:     	ldr	q1, [x10, #0x350]
100ba2be0:     	ushl.2d	v1, v1, v7
100ba2be4:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2be8:     	ldr	q29, [x10, #0x670]
100ba2bec:     	ushl.2d	v29, v29, v7
100ba2bf0:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2bf4:     	ldr	q30, [x10, #0x390]
100ba2bf8:     	ushl.2d	v30, v30, v7
100ba2bfc:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c00:     	ldr	q31, [x10, #0x6a0]
100ba2c04:     	ushl.2d	v31, v31, v7
100ba2c08:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c0c:     	ldr	q8, [x10, #0x380]
100ba2c10:     	ushl.2d	v8, v8, v7
100ba2c14:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c18:     	ldr	q9, [x10, #0x6d0]
100ba2c1c:     	ushl.2d	v9, v9, v7
100ba2c20:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c24:     	ldr	q10, [x10, #0x370]
100ba2c28:     	ushl.2d	v10, v10, v7
100ba2c2c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c30:     	ldr	q11, [x10, #0x700]
100ba2c34:     	ushl.2d	v11, v11, v7
100ba2c38:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c3c:     	ldr	q15, [x10, #0x760]
100ba2c40:     	ushl.2d	v15, v15, v7
100ba2c44:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c48:     	ldr	q14, [x10, #0x770]
100ba2c4c:     	ushl.2d	v14, v14, v7
100ba2c50:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c54:     	ldr	q13, [x10, #0x780]
100ba2c58:     	ushl.2d	v13, v13, v7
100ba2c5c:     	adrp	x10, 0x101196000 <dyld_stub_binder+0x101196000>
100ba2c60:     	ldr	q12, [x10, #0x790]
100ba2c64:     	ushl.2d	v12, v12, v7
100ba2c68:     	mov	w10, #0x3f              ; =63
100ba2c6c:     	dup.2d	v7, x10
100ba2c70:     	and.16b	v19, v23, v7
100ba2c74:     	and.16b	v16, v16, v7
100ba2c78:     	and.16b	v17, v17, v7
100ba2c7c:     	and.16b	v18, v18, v7
100ba2c80:     	and.16b	v20, v20, v7
100ba2c84:     	and.16b	v23, v21, v7
100ba2c88:     	mov.16b	v21, v20
100ba2c8c:     	and.16b	v20, v22, v7
100ba2c90:     	mov.16b	v22, v23
100ba2c94:     	and.16b	v5, v5, v7
100ba2c98:     	and.16b	v6, v6, v7
100ba2c9c:     	str	q6, [sp, #0x1f0]
100ba2ca0:     	and.16b	v6, v24, v7
100ba2ca4:     	str	q6, [sp, #0x1e0]
100ba2ca8:     	and.16b	v6, v25, v7
100ba2cac:     	and.16b	v1, v1, v7
100ba2cb0:     	stp	q1, q6, [sp, #0x1c0]
100ba2cb4:     	and.16b	v6, v29, v7
100ba2cb8:     	and.16b	v1, v30, v7
100ba2cbc:     	stp	q1, q6, [sp, #0x50]
100ba2cc0:     	and.16b	v6, v31, v7
100ba2cc4:     	and.16b	v1, v8, v7
100ba2cc8:     	stp	q1, q6, [sp, #0x30]
100ba2ccc:     	mov.16b	v8, v20
100ba2cd0:     	and.16b	v6, v9, v7
100ba2cd4:     	mov.16b	v9, v5
100ba2cd8:     	and.16b	v10, v10, v7
100ba2cdc:     	and.16b	v29, v11, v7
100ba2ce0:     	and.16b	v1, v15, v7
100ba2ce4:     	stp	q1, q6, [sp, #0x10]
100ba2ce8:     	and.16b	v1, v14, v7
100ba2cec:     	str	q1, [sp]
100ba2cf0:     	and.16b	v30, v13, v7
100ba2cf4:     	and.16b	v31, v12, v7
100ba2cf8:     	ldp	q1, q6, [x29, #-0xc0]
100ba2cfc:     	neg.2d	v5, v6
100ba2d00:     	neg.2d	v6, v1
100ba2d04:     	ldp	q1, q7, [x29, #-0xe0]
100ba2d08:     	neg.2d	v24, v7
100ba2d0c:     	neg.2d	v25, v1
100ba2d10:     	ldp	q1, q7, [x29, #-0x100]
100ba2d14:     	neg.2d	v11, v7
100ba2d18:     	neg.2d	v1, v1
100ba2d1c:     	ldr	q7, [sp, #0x1a0]
100ba2d20:     	neg.2d	v7, v7
100ba2d24:     	stur	q7, [x29, #-0xb0]
100ba2d28:     	ldr	q7, [sp, #0x190]
100ba2d2c:     	neg.2d	v7, v7
100ba2d30:     	stur	q7, [x29, #-0xc0]
100ba2d34:     	ldr	q7, [sp, #0x180]
100ba2d38:     	neg.2d	v7, v7
100ba2d3c:     	stur	q7, [x29, #-0xd0]
100ba2d40:     	ldr	q7, [sp, #0x170]
100ba2d44:     	neg.2d	v7, v7
100ba2d48:     	stur	q7, [x29, #-0xe0]
100ba2d4c:     	ldr	q7, [sp, #0x160]
100ba2d50:     	neg.2d	v7, v7
100ba2d54:     	stur	q7, [x29, #-0xf0]
100ba2d58:     	ldr	q7, [sp, #0x150]
100ba2d5c:     	neg.2d	v7, v7
100ba2d60:     	stur	q7, [x29, #-0x100]
100ba2d64:     	ldr	q7, [sp, #0x110]
100ba2d68:     	neg.2d	v7, v7
100ba2d6c:     	str	q7, [sp, #0x1a0]
100ba2d70:     	mov	w10, #0x1               ; =1
100ba2d74:     	ldr	q7, [sp, #0x120]
100ba2d78:     	neg.2d	v7, v7
100ba2d7c:     	str	q7, [sp, #0x190]
100ba2d80:     	lsl	x10, x10, x23
100ba2d84:     	ldr	q7, [sp, #0x130]
100ba2d88:     	neg.2d	v7, v7
100ba2d8c:     	str	q7, [sp, #0x180]
100ba2d90:     	mov	x11, #-0x1              ; =-1
100ba2d94:     	ldr	q7, [sp, #0x140]
100ba2d98:     	neg.2d	v7, v7
100ba2d9c:     	str	q7, [sp, #0x170]
100ba2da0:     	lsl	x10, x11, x10
100ba2da4:     	ldr	q7, [sp, #0x100]
100ba2da8:     	neg.2d	v7, v7
100ba2dac:     	str	q7, [sp, #0x160]
100ba2db0:     	add	x11, x22, x25, lsl #3
100ba2db4:     	ldr	q7, [sp, #0xf0]
100ba2db8:     	neg.2d	v7, v7
100ba2dbc:     	str	q7, [sp, #0x150]
100ba2dc0:     	mov	w12, w24
100ba2dc4:     	ldr	q7, [sp, #0xe0]
100ba2dc8:     	neg.2d	v7, v7
100ba2dcc:     	str	q7, [sp, #0x140]
100ba2dd0:     	lsl	x12, x12, x8
100ba2dd4:     	ldr	q7, [sp, #0xd0]
100ba2dd8:     	neg.2d	v7, v7
100ba2ddc:     	str	q7, [sp, #0x130]
100ba2de0:     	mov	w13, #0x20              ; =32
100ba2de4:     	ldr	q7, [sp, #0xc0]
100ba2de8:     	neg.2d	v7, v7
100ba2dec:     	str	q7, [sp, #0x120]
100ba2df0:     	lsr	x13, x13, x8
100ba2df4:     	ldr	q7, [sp, #0xb0]
100ba2df8:     	neg.2d	v7, v7
100ba2dfc:     	str	q7, [sp, #0x110]
100ba2e00:     	and	x14, x13, #0x38
100ba2e04:     	ldr	q7, [sp, #0xa0]
100ba2e08:     	neg.2d	v7, v7
100ba2e0c:     	str	q7, [sp, #0x100]
100ba2e10:     	ldr	q7, [sp, #0x90]
100ba2e14:     	neg.2d	v7, v7
100ba2e18:     	str	q7, [sp, #0xf0]
100ba2e1c:     	ldr	q7, [sp, #0x80]
100ba2e20:     	neg.2d	v7, v7
100ba2e24:     	str	q7, [sp, #0xe0]
100ba2e28:     	neg.2d	v7, v26
100ba2e2c:     	str	q7, [sp, #0xd0]
100ba2e30:     	neg.2d	v7, v27
100ba2e34:     	str	q7, [sp, #0xc0]
100ba2e38:     	neg.2d	v7, v28
100ba2e3c:     	str	q7, [sp, #0xb0]
100ba2e40:     	str	q1, [sp, #0x1b0]
100ba2e44:     	ldr	x15, [x22]
100ba2e48:     	lsr	x15, x15, x12
100ba2e4c:     	cmp	w23, #0x2
100ba2e50:     	b.ls	0x100ba2e60 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6c8>
100ba2e54:     	mov	x17, #0x0               ; =0
100ba2e58:     	mov	x16, #0x0               ; =0
100ba2e5c:     	b	0x100ba310c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x974>
100ba2e60:     	dup.2d	v12, x15
100ba2e64:     	ushl.2d	v7, v12, v5
100ba2e68:     	ushl.2d	v23, v12, v6
100ba2e6c:     	ushl.2d	v26, v12, v24
100ba2e70:     	ushl.2d	v27, v12, v25
100ba2e74:     	dup.2d	v13, x10
100ba2e78:     	bic.16b	v7, v7, v13
100ba2e7c:     	bic.16b	v28, v23, v13
100ba2e80:     	bic.16b	v14, v26, v13
100ba2e84:     	bic.16b	v15, v27, v13
100ba2e88:     	ushl.2d	v23, v7, v0
100ba2e8c:     	ushl.2d	v26, v28, v2
100ba2e90:     	ushl.2d	v27, v14, v3
100ba2e94:     	ushl.2d	v28, v15, v4
100ba2e98:     	cmp	x14, #0x8
100ba2e9c:     	b.eq	0x100ba30e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba2ea0:     	ushl.2d	v7, v12, v11
100ba2ea4:     	ushl.2d	v14, v12, v1
100ba2ea8:     	ldur	q20, [x29, #-0xb0]
100ba2eac:     	ushl.2d	v15, v12, v20
100ba2eb0:     	ldur	q20, [x29, #-0xc0]
100ba2eb4:     	ushl.2d	v20, v12, v20
100ba2eb8:     	bic.16b	v7, v7, v13
100ba2ebc:     	bic.16b	v14, v14, v13
100ba2ec0:     	bic.16b	v15, v15, v13
100ba2ec4:     	bic.16b	v20, v20, v13
100ba2ec8:     	ushl.2d	v7, v7, v19
100ba2ecc:     	ushl.2d	v14, v14, v16
100ba2ed0:     	ushl.2d	v15, v15, v17
100ba2ed4:     	ushl.2d	v20, v20, v18
100ba2ed8:     	orr.16b	v23, v7, v23
100ba2edc:     	orr.16b	v26, v14, v26
100ba2ee0:     	orr.16b	v27, v15, v27
100ba2ee4:     	orr.16b	v28, v20, v28
100ba2ee8:     	cmp	x14, #0x10
100ba2eec:     	b.eq	0x100ba30e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba2ef0:     	ldp	q20, q7, [x29, #-0xe0]
100ba2ef4:     	ushl.2d	v7, v12, v7
100ba2ef8:     	ushl.2d	v20, v12, v20
100ba2efc:     	ldp	q15, q14, [x29, #-0x100]
100ba2f00:     	ushl.2d	v14, v12, v14
100ba2f04:     	ushl.2d	v15, v12, v15
100ba2f08:     	bic.16b	v7, v7, v13
100ba2f0c:     	bic.16b	v20, v20, v13
100ba2f10:     	bic.16b	v14, v14, v13
100ba2f14:     	bic.16b	v15, v15, v13
100ba2f18:     	ushl.2d	v7, v7, v21
100ba2f1c:     	ushl.2d	v20, v20, v22
100ba2f20:     	ushl.2d	v14, v14, v8
100ba2f24:     	ushl.2d	v15, v15, v9
100ba2f28:     	orr.16b	v23, v7, v23
100ba2f2c:     	orr.16b	v26, v20, v26
100ba2f30:     	orr.16b	v27, v14, v27
100ba2f34:     	orr.16b	v28, v15, v28
100ba2f38:     	cmp	x14, #0x18
100ba2f3c:     	b.eq	0x100ba30e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba2f40:     	ldr	q1, [sp, #0x1a0]
100ba2f44:     	ushl.2d	v7, v12, v1
100ba2f48:     	ldr	q1, [sp, #0x190]
100ba2f4c:     	ushl.2d	v20, v12, v1
100ba2f50:     	ldr	q1, [sp, #0x180]
100ba2f54:     	ushl.2d	v14, v12, v1
100ba2f58:     	ldr	q1, [sp, #0x170]
100ba2f5c:     	ushl.2d	v15, v12, v1
100ba2f60:     	bic.16b	v7, v7, v13
100ba2f64:     	bic.16b	v20, v20, v13
100ba2f68:     	bic.16b	v14, v14, v13
100ba2f6c:     	bic.16b	v15, v15, v13
100ba2f70:     	ldr	q1, [sp, #0x1f0]
100ba2f74:     	ushl.2d	v7, v7, v1
100ba2f78:     	ldr	q1, [sp, #0x1e0]
100ba2f7c:     	ushl.2d	v20, v20, v1
100ba2f80:     	ldr	q1, [sp, #0x1d0]
100ba2f84:     	ushl.2d	v14, v14, v1
100ba2f88:     	ldr	q1, [sp, #0x1c0]
100ba2f8c:     	ushl.2d	v15, v15, v1
100ba2f90:     	orr.16b	v23, v7, v23
100ba2f94:     	orr.16b	v26, v20, v26
100ba2f98:     	orr.16b	v27, v14, v27
100ba2f9c:     	orr.16b	v28, v15, v28
100ba2fa0:     	cmp	x14, #0x20
100ba2fa4:     	b.eq	0x100ba30e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x94c>
100ba2fa8:     	ldr	q1, [sp, #0x160]
100ba2fac:     	ushl.2d	v7, v12, v1
100ba2fb0:     	bic.16b	v7, v7, v13
100ba2fb4:     	ldr	q1, [sp, #0x60]
100ba2fb8:     	ushl.2d	v7, v7, v1
100ba2fbc:     	ldr	q1, [sp, #0x150]
100ba2fc0:     	ushl.2d	v20, v12, v1
100ba2fc4:     	bic.16b	v20, v20, v13
100ba2fc8:     	ldr	q1, [sp, #0x50]
100ba2fcc:     	ushl.2d	v20, v20, v1
100ba2fd0:     	orr.16b	v1, v20, v7
100ba2fd4:     	str	q1, [sp, #0x90]
100ba2fd8:     	ldr	q1, [sp, #0x140]
100ba2fdc:     	ushl.2d	v20, v12, v1
100ba2fe0:     	bic.16b	v20, v20, v13
100ba2fe4:     	ldr	q1, [sp, #0x40]
100ba2fe8:     	ushl.2d	v20, v20, v1
100ba2fec:     	ldr	q1, [sp, #0x130]
100ba2ff0:     	ushl.2d	v14, v12, v1
100ba2ff4:     	bic.16b	v14, v14, v13
100ba2ff8:     	ldr	q1, [sp, #0x30]
100ba2ffc:     	ushl.2d	v14, v14, v1
100ba3000:     	orr.16b	v1, v14, v20
100ba3004:     	str	q1, [sp, #0x80]
100ba3008:     	ldr	q1, [sp, #0x120]
100ba300c:     	ushl.2d	v14, v12, v1
100ba3010:     	bic.16b	v14, v14, v13
100ba3014:     	ldr	q1, [sp, #0x20]
100ba3018:     	ushl.2d	v14, v14, v1
100ba301c:     	ldp	q1, q7, [sp, #0x100]
100ba3020:     	ushl.2d	v15, v12, v7
100ba3024:     	bic.16b	v15, v15, v13
100ba3028:     	ushl.2d	v15, v15, v10
100ba302c:     	orr.16b	v14, v15, v14
100ba3030:     	ushl.2d	v15, v12, v1
100ba3034:     	bic.16b	v15, v15, v13
100ba3038:     	ushl.2d	v15, v15, v29
100ba303c:     	str	q0, [sp, #0xa0]
100ba3040:     	mov.16b	v7, v18
100ba3044:     	mov.16b	v18, v21
100ba3048:     	ldr	q0, [sp, #0xf0]
100ba304c:     	ushl.2d	v21, v12, v0
100ba3050:     	bic.16b	v21, v21, v13
100ba3054:     	mov.16b	v1, v22
100ba3058:     	ldr	q22, [sp, #0x70]
100ba305c:     	ushl.2d	v21, v21, v22
100ba3060:     	orr.16b	v21, v21, v15
100ba3064:     	ldr	q0, [sp, #0xe0]
100ba3068:     	ushl.2d	v15, v12, v0
100ba306c:     	ldr	q0, [sp, #0xd0]
100ba3070:     	ushl.2d	v22, v12, v0
100ba3074:     	mov.16b	v0, v8
100ba3078:     	ldp	q20, q8, [sp, #0xb0]
100ba307c:     	ushl.2d	v8, v12, v8
100ba3080:     	ushl.2d	v12, v12, v20
100ba3084:     	bic.16b	v15, v15, v13
100ba3088:     	bic.16b	v22, v22, v13
100ba308c:     	bic.16b	v8, v8, v13
100ba3090:     	bic.16b	v12, v12, v13
100ba3094:     	ldr	q13, [sp, #0x10]
100ba3098:     	ushl.2d	v13, v15, v13
100ba309c:     	orr.16b	v21, v21, v13
100ba30a0:     	orr.16b	v23, v21, v23
100ba30a4:     	ldr	q21, [sp]
100ba30a8:     	ushl.2d	v21, v22, v21
100ba30ac:     	mov.16b	v22, v1
100ba30b0:     	orr.16b	v21, v14, v21
100ba30b4:     	orr.16b	v26, v21, v26
100ba30b8:     	ushl.2d	v21, v8, v30
100ba30bc:     	mov.16b	v8, v0
100ba30c0:     	ldp	q0, q1, [sp, #0x80]
100ba30c4:     	orr.16b	v20, v0, v21
100ba30c8:     	mov.16b	v21, v18
100ba30cc:     	mov.16b	v18, v7
100ba30d0:     	ldr	q0, [sp, #0xa0]
100ba30d4:     	orr.16b	v27, v20, v27
100ba30d8:     	ushl.2d	v20, v12, v31
100ba30dc:     	orr.16b	v7, v1, v20
100ba30e0:     	orr.16b	v28, v7, v28
100ba30e4:     	ldr	q1, [sp, #0x1b0]
100ba30e8:     	orr.16b	v7, v26, v23
100ba30ec:     	orr.16b	v20, v28, v27
100ba30f0:     	orr.16b	v7, v20, v7
100ba30f4:     	mov	d20, v7[1]
100ba30f8:     	orr.8b	v7, v7, v20
100ba30fc:     	fmov	x16, d7
100ba3100:     	and	x17, x13, #0x38
100ba3104:     	cmp	x13, x14
100ba3108:     	b.eq	0x100ba313c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9a4>
100ba310c:     	lsl	x0, x17, #1
100ba3110:     	lsl	x1, x17, x8
100ba3114:     	add	x17, x17, #0x1
100ba3118:     	lsl	x2, x0, x8
100ba311c:     	and	x2, x2, #0x3e
100ba3120:     	lsr	x2, x15, x2
100ba3124:     	bic	x2, x2, x10
100ba3128:     	lsl	x1, x2, x1
100ba312c:     	orr	x16, x1, x16
100ba3130:     	add	x0, x0, #0x2
100ba3134:     	cmp	x13, x17
100ba3138:     	b.ne	0x100ba3110 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x978>
100ba313c:     	lsr	x0, x9, #1
100ba3140:     	cmp	x0, x19
100ba3144:     	b.hs	0x100ba321c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa84>
100ba3148:     	ubfiz	x15, x9, #5, #1
100ba314c:     	add	x9, x9, #0x1
100ba3150:     	add	x22, x22, #0x8
100ba3154:     	ldr	x17, [x20, x0, lsl #3]
100ba3158:     	lsl	x15, x16, x15
100ba315c:     	orr	x15, x17, x15
100ba3160:     	str	x15, [x20, x0, lsl #3]
100ba3164:     	cmp	x22, x11
100ba3168:     	b.ne	0x100ba2e44 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6ac>
100ba316c:     	cmp	w28, #0x6
100ba3170:     	b.hs	0x100ba318c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9f4>
100ba3174:     	mov	x8, #-0x1               ; =-1
100ba3178:     	ldr	x9, [sp, #0x208]
100ba317c:     	lsl	x8, x8, x9
100ba3180:     	ldr	x9, [x20]
100ba3184:     	bic	x8, x9, x8
100ba3188:     	str	x8, [x20]
100ba318c:     	stp	x19, x20, [x21]
100ba3190:     	str	x19, [x21, #0x10]
100ba3194:     	add	sp, sp, #0x280
100ba3198:     	ldp	x29, x30, [sp, #0x90]
100ba319c:     	ldp	x20, x19, [sp, #0x80]
100ba31a0:     	ldp	x22, x21, [sp, #0x70]
100ba31a4:     	ldp	x24, x23, [sp, #0x60]
100ba31a8:     	ldp	x26, x25, [sp, #0x50]
100ba31ac:     	ldp	x28, x27, [sp, #0x40]
100ba31b0:     	ldp	d9, d8, [sp, #0x30]
100ba31b4:     	ldp	d11, d10, [sp, #0x20]
100ba31b8:     	ldp	d13, d12, [sp, #0x10]
100ba31bc:     	ldp	d15, d14, [sp], #0xa0
100ba31c0:     	ret
100ba31c4:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba31c8:     	add	x2, x2, #0x8c8
100ba31cc:     	mov	x1, x23
100ba31d0:     	bl	0x101101a78 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100ba31d4:     	b	0x100ba322c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba31d8:     	adrp	x0, 0x1011c2000 <dyld_stub_binder+0x1011c2000>
100ba31dc:     	add	x0, x0, #0x1ed
100ba31e0:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba31e4:     	add	x2, x2, #0x880
100ba31e8:     	mov	w1, #0x22               ; =34
100ba31ec:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba31f0:     	adrp	x3, 0x101377000 <dyld_stub_binder+0x101377000>
100ba31f4:     	add	x3, x3, #0x8e0
100ba31f8:     	bl	0x101101614 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100ba31fc:     	b	0x100ba322c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba3200:     	adrp	x0, 0x10119e000 <dyld_stub_binder+0x10119e000>
100ba3204:     	add	x0, x0, #0x791
100ba3208:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba320c:     	add	x2, x2, #0x8b0
100ba3210:     	mov	w1, #0x37               ; =55
100ba3214:     	bl	0x101101574 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba3218:     	b	0x100ba322c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba321c:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba3220:     	add	x2, x2, #0x898
100ba3224:     	mov	x1, x19
100ba3228:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ba322c:     	brk	#0x1
100ba3230:     	mov	w0, #0x8                ; =8
100ba3234:     	mov	x1, x26
100ba3238:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba323c:     	cbz	x19, 0x100ba3250 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xab8>
100ba3240:     	mov	x19, x0
100ba3244:     	mov	x0, x20
100ba3248:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100ba324c:     	mov	x0, x19
100ba3250:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
