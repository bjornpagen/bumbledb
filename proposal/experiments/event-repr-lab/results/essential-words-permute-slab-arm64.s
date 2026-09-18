
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d15b40 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>:
100d15b40:     	sub	sp, sp, #0x80
100d15b44:     	stp	x28, x27, [sp, #0x20]
100d15b48:     	stp	x26, x25, [sp, #0x30]
100d15b4c:     	stp	x24, x23, [sp, #0x40]
100d15b50:     	stp	x22, x21, [sp, #0x50]
100d15b54:     	stp	x20, x19, [sp, #0x60]
100d15b58:     	stp	x29, x30, [sp, #0x70]
100d15b5c:     	add	x29, sp, #0x70
100d15b60:     	cmp	x3, #0x15
100d15b64:     	b.hs	0x100d15c68 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x128>
100d15b68:     	mov	x19, x3
100d15b6c:     	mov	x20, x1
100d15b70:     	mov	w8, #0x1                ; =1
100d15b74:     	lsl	x8, x8, x3
100d15b78:     	lsr	x8, x8, #6
100d15b7c:     	cmp	x3, #0x6
100d15b80:     	cinc	x8, x8, lo
100d15b84:     	stp	x1, x8, [sp, #0x10]
100d15b88:     	cmp	x1, x8
100d15b8c:     	b.ne	0x100d15c80 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x140>
100d15b90:     	cbz	x19, 0x100d15bf8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xb8>
100d15b94:     	mov	x21, x2
100d15b98:     	mov	x23, x0
100d15b9c:     	mov	x8, #0x0                ; =0
100d15ba0:     	lsl	x24, x19, #2
100d15ba4:     	add	x25, x2, x24
100d15ba8:     	mov	w9, #0x1                ; =1
100d15bac:     	mov	x10, x24
100d15bb0:     	mov	x11, x2
100d15bb4:     	ldr	w12, [x11], #0x4
100d15bb8:     	cmp	w12, w19
100d15bbc:     	b.hs	0x100d15c50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100d15bc0:     	lsr	x13, x8, x12
100d15bc4:     	tbnz	w13, #0x0, 0x100d15c50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100d15bc8:     	lsl	x12, x9, x12
100d15bcc:     	orr	x8, x12, x8
100d15bd0:     	subs	x10, x10, #0x4
100d15bd4:     	b.ne	0x100d15bb4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x74>
100d15bd8:     	mov	x0, x24
100d15bdc:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15be0:     	cbz	x0, 0x100d15c9c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x15c>
100d15be4:     	mov	x22, x0
100d15be8:     	cmp	x19, #0x8
100d15bec:     	b.hs	0x100d15c18 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xd8>
100d15bf0:     	mov	x8, #0x0                ; =0
100d15bf4:     	b	0x100d15ca8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100d15bf8:     	ldp	x29, x30, [sp, #0x70]
100d15bfc:     	ldp	x20, x19, [sp, #0x60]
100d15c00:     	ldp	x22, x21, [sp, #0x50]
100d15c04:     	ldp	x24, x23, [sp, #0x40]
100d15c08:     	ldp	x26, x25, [sp, #0x30]
100d15c0c:     	ldp	x28, x27, [sp, #0x20]
100d15c10:     	add	sp, sp, #0x80
100d15c14:     	ret
100d15c18:     	and	x8, x19, #0x18
100d15c1c:     	adrp	x9, 0x101316000 <GCC_except_table9261>
100d15c20:     	ldr	q0, [x9, #0x710]
100d15c24:     	adrp	x9, 0x101316000 <GCC_except_table9261>
100d15c28:     	ldr	q1, [x9, #0x810]
100d15c2c:     	stp	q0, q1, [x22]
100d15c30:     	cmp	x8, #0x8
100d15c34:     	b.eq	0x100d15cb0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100d15c38:     	adrp	x9, 0x101316000 <GCC_except_table9261>
100d15c3c:     	ldr	q0, [x9, #0xe50]
100d15c40:     	adrp	x9, 0x101316000 <GCC_except_table9261>
100d15c44:     	ldr	q1, [x9, #0xe60]
100d15c48:     	stp	q0, q1, [x22, #0x20]
100d15c4c:     	b	0x100d15cb0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100d15c50:     	adrp	x0, 0x101343000 <dyld_stub_binder+0x101343000>
100d15c54:     	add	x0, x0, #0xa9c
100d15c58:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d15c5c:     	add	x2, x2, #0x3e8
100d15c60:     	mov	w1, #0x49               ; =73
100d15c64:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d15c68:     	adrp	x0, 0x101343000 <dyld_stub_binder+0x101343000>
100d15c6c:     	add	x0, x0, #0xa72
100d15c70:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d15c74:     	add	x2, x2, #0x3b8
100d15c78:     	mov	w1, #0x2a               ; =42
100d15c7c:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d15c80:     	adrp	x5, 0x101500000 <dyld_stub_binder+0x101500000>
100d15c84:     	add	x5, x5, #0x3d0
100d15c88:     	add	x1, sp, #0x10
100d15c8c:     	add	x2, sp, #0x18
100d15c90:     	mov	w0, #0x0                ; =0
100d15c94:     	mov	x3, #0x0                ; =0
100d15c98:     	bl	0x10127c770 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d15c9c:     	mov	w0, #0x4                ; =4
100d15ca0:     	mov	x1, x24
100d15ca4:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d15ca8:     	str	w8, [x22, x8, lsl #2]
100d15cac:     	add	x8, x8, #0x1
100d15cb0:     	cmp	x19, x8
100d15cb4:     	b.ne	0x100d15ca8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100d15cb8:     	mov	w10, #0x0               ; =0
100d15cbc:     	lsl	x9, x20, #3
100d15cc0:     	add	x8, x23, x9
100d15cc4:     	sub	x9, x9, #0x8
100d15cc8:     	lsr	x11, x9, #3
100d15ccc:     	add	x11, x11, #0x1
100d15cd0:     	and	x12, x11, #0x3ffffffffffffff8
100d15cd4:     	add	x13, x23, x12, lsl #3
100d15cd8:     	adrp	x12, 0x101500000 <dyld_stub_binder+0x101500000>
100d15cdc:     	add	x12, x12, #0x418
100d15ce0:     	stp	x12, x13, [sp]
100d15ce4:     	adrp	x14, 0x101321000 <dyld_stub_binder+0x101321000>
100d15ce8:     	add	x14, x14, #0x3b8
100d15cec:     	mov	w16, #0x1               ; =1
100d15cf0:     	b	0x100d15cfc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1bc>
100d15cf4:     	cmp	x21, x25
100d15cf8:     	b.eq	0x100d16018 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4d8>
100d15cfc:     	mov	x17, #0x0               ; =0
100d15d00:     	mov	x12, x10
100d15d04:     	ldr	w0, [x21], #0x4
100d15d08:     	add	w10, w10, #0x1
100d15d0c:     	mov	x13, x24
100d15d10:     	ldr	w1, [x22, x17, lsl #2]
100d15d14:     	cmp	w1, w12
100d15d18:     	b.eq	0x100d15d2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1ec>
100d15d1c:     	add	x17, x17, #0x1
100d15d20:     	subs	x13, x13, #0x4
100d15d24:     	b.ne	0x100d15d10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1d0>
100d15d28:     	b	0x100d16074 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x534>
100d15d2c:     	cmp	w0, w17
100d15d30:     	b.eq	0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15d34:     	cmp	w0, w17
100d15d38:     	csel	w13, w0, w17, lo
100d15d3c:     	csel	w1, w0, w17, hi
100d15d40:     	cmp	x19, x0
100d15d44:     	b.ls	0x100d16008 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4c8>
100d15d48:     	ldr	w12, [x22, x17, lsl #2]
100d15d4c:     	ldr	w2, [x22, x0, lsl #2]
100d15d50:     	str	w2, [x22, x17, lsl #2]
100d15d54:     	str	w12, [x22, x0, lsl #2]
100d15d58:     	cmp	w1, #0x6
100d15d5c:     	b.hs	0x100d15e48 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x308>
100d15d60:     	cbz	x20, 0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15d64:     	ldr	x12, [x14, w13, uxtw #3]
100d15d68:     	ldr	x17, [x14, w1, uxtw #3]
100d15d6c:     	bic	x17, x17, x12
100d15d70:     	mov	w12, #-0x1              ; =-1
100d15d74:     	lsl	w12, w12, w13
100d15d78:     	lsl	w13, w16, w1
100d15d7c:     	add	w12, w12, w13
100d15d80:     	and	w0, w12, #0x3f
100d15d84:     	mov	x12, x23
100d15d88:     	cmp	x9, #0x38
100d15d8c:     	b.lo	0x100d15e1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100d15d90:     	dup.2d	v0, x0
100d15d94:     	dup.2d	v1, x17
100d15d98:     	neg.2d	v2, v0
100d15d9c:     	add	x13, x23, #0x20
100d15da0:     	and	x1, x11, #0x3ffffffffffffff8
100d15da4:     	ldp	q3, q4, [x13, #-0x20]
100d15da8:     	ldp	q5, q6, [x13]
100d15dac:     	ushl.2d	v7, v3, v2
100d15db0:     	ushl.2d	v16, v4, v2
100d15db4:     	ushl.2d	v17, v5, v2
100d15db8:     	ushl.2d	v18, v6, v2
100d15dbc:     	eor.16b	v7, v7, v3
100d15dc0:     	eor.16b	v16, v16, v4
100d15dc4:     	eor.16b	v17, v17, v5
100d15dc8:     	eor.16b	v18, v18, v6
100d15dcc:     	and.16b	v7, v1, v7
100d15dd0:     	and.16b	v16, v1, v16
100d15dd4:     	and.16b	v17, v1, v17
100d15dd8:     	and.16b	v18, v1, v18
100d15ddc:     	ushl.2d	v19, v7, v0
100d15de0:     	ushl.2d	v20, v16, v0
100d15de4:     	ushl.2d	v21, v17, v0
100d15de8:     	ushl.2d	v22, v18, v0
100d15dec:     	eor3.16b	v3, v3, v19, v7
100d15df0:     	eor3.16b	v4, v4, v20, v16
100d15df4:     	eor3.16b	v5, v5, v21, v17
100d15df8:     	stp	q3, q4, [x13, #-0x20]
100d15dfc:     	eor3.16b	v3, v6, v22, v18
100d15e00:     	stp	q5, q3, [x13], #0x40
100d15e04:     	subs	x1, x1, #0x8
100d15e08:     	b.ne	0x100d15da4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x264>
100d15e0c:     	ldr	x12, [sp, #0x8]
100d15e10:     	and	x13, x11, #0x3ffffffffffffff8
100d15e14:     	cmp	x11, x13
100d15e18:     	b.eq	0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15e1c:     	ldr	x13, [x12]
100d15e20:     	lsr	x15, x13, x0
100d15e24:     	eor	x15, x15, x13
100d15e28:     	and	x15, x17, x15
100d15e2c:     	lsl	x1, x15, x0
100d15e30:     	eor	x13, x13, x15
100d15e34:     	eor	x13, x13, x1
100d15e38:     	str	x13, [x12], #0x8
100d15e3c:     	cmp	x12, x8
100d15e40:     	b.ne	0x100d15e1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100d15e44:     	b	0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15e48:     	cmp	w13, #0x6
100d15e4c:     	b.hs	0x100d15fa0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x460>
100d15e50:     	add	w17, w1, #0x3a
100d15e54:     	and	w12, w17, #0x3f
100d15e58:     	cmp	w12, #0x3f
100d15e5c:     	b.eq	0x100d16058 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x518>
100d15e60:     	cbz	x20, 0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15e64:     	lsl	x0, x16, x17
100d15e68:     	ldr	x4, [x14, w13, uxtw #3]
100d15e6c:     	lsl	w5, w16, w13
100d15e70:     	mov	w13, #0x2               ; =2
100d15e74:     	lsl	x6, x13, x12
100d15e78:     	mov	w13, #0x8               ; =8
100d15e7c:     	lsl	x7, x13, x12
100d15e80:     	lsr	x26, x7, #3
100d15e84:     	dup.2d	v0, x5
100d15e88:     	dup.2d	v1, x4
100d15e8c:     	lsl	x27, x0, #3
100d15e90:     	neg.2d	v2, v0
100d15e94:     	mov	x28, x20
100d15e98:     	mov	x30, x23
100d15e9c:     	b	0x100d15eac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x36c>
100d15ea0:     	add	x30, x30, x17, lsl #3
100d15ea4:     	sub	x28, x28, x17
100d15ea8:     	cbz	x28, 0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15eac:     	cmp	x6, x28
100d15eb0:     	csel	x17, x6, x28, lo
100d15eb4:     	subs	x12, x17, x0
100d15eb8:     	b.lo	0x100d1603c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4fc>
100d15ebc:     	cmp	x12, x26
100d15ec0:     	csel	x13, x12, x26, lo
100d15ec4:     	cmp	x17, x0
100d15ec8:     	ccmp	x30, #0x0, #0x4, ne
100d15ecc:     	b.eq	0x100d15ea0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100d15ed0:     	cmp	x13, #0x4
100d15ed4:     	b.lo	0x100d15f5c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100d15ed8:     	add	x12, x30, x0, lsl #3
100d15edc:     	add	x1, x30, x13, lsl #3
100d15ee0:     	add	x2, x1, x7
100d15ee4:     	cmp	x30, x2
100d15ee8:     	ccmp	x12, x1, #0x2, lo
100d15eec:     	b.lo	0x100d15f5c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100d15ef0:     	and	x2, x13, #0x1ffffffffffffffc
100d15ef4:     	add	x1, x30, #0x10
100d15ef8:     	add	x3, x1, x27
100d15efc:     	and	x12, x13, #0x1ffffffffffffffc
100d15f00:     	ldp	q3, q4, [x1, #-0x10]
100d15f04:     	ushl.2d	v5, v3, v2
100d15f08:     	ushl.2d	v6, v4, v2
100d15f0c:     	ldp	q7, q16, [x3, #-0x10]
100d15f10:     	eor.16b	v5, v5, v7
100d15f14:     	eor.16b	v6, v6, v16
100d15f18:     	and.16b	v5, v5, v1
100d15f1c:     	and.16b	v6, v6, v1
100d15f20:     	ushl.2d	v17, v5, v0
100d15f24:     	ushl.2d	v18, v6, v0
100d15f28:     	eor.16b	v3, v17, v3
100d15f2c:     	eor.16b	v4, v18, v4
100d15f30:     	stp	q3, q4, [x1, #-0x10]
100d15f34:     	eor.16b	v3, v5, v7
100d15f38:     	eor.16b	v4, v6, v16
100d15f3c:     	stp	q3, q4, [x3, #-0x10]
100d15f40:     	add	x3, x3, #0x20
100d15f44:     	add	x1, x1, #0x20
100d15f48:     	subs	x12, x12, #0x4
100d15f4c:     	b.ne	0x100d15f00 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x3c0>
100d15f50:     	cmp	x13, x2
100d15f54:     	b.eq	0x100d15ea0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100d15f58:     	b	0x100d15f60 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x420>
100d15f5c:     	mov	x2, #0x0                ; =0
100d15f60:     	sub	x12, x13, x2
100d15f64:     	add	x13, x30, x2, lsl #3
100d15f68:     	ldr	x1, [x13]
100d15f6c:     	lsr	x2, x1, x5
100d15f70:     	ldr	x3, [x13, x27]
100d15f74:     	eor	x2, x2, x3
100d15f78:     	and	x2, x2, x4
100d15f7c:     	lsl	x15, x2, x5
100d15f80:     	eor	x15, x15, x1
100d15f84:     	str	x15, [x13]
100d15f88:     	eor	x15, x2, x3
100d15f8c:     	str	x15, [x13, x27]
100d15f90:     	add	x13, x13, #0x8
100d15f94:     	subs	x12, x12, #0x1
100d15f98:     	b.ne	0x100d15f68 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x428>
100d15f9c:     	b	0x100d15ea0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100d15fa0:     	cbz	x20, 0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15fa4:     	mov	x17, #0x0               ; =0
100d15fa8:     	add	w12, w13, #0x3a
100d15fac:     	lsl	x12, x16, x12
100d15fb0:     	add	w13, w1, #0x3a
100d15fb4:     	lsl	x13, x16, x13
100d15fb8:     	eor	x1, x13, x12
100d15fbc:     	b	0x100d15fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x49c>
100d15fc0:     	ldr	x2, [x23, x17, lsl #3]
100d15fc4:     	ldr	x3, [x23, x0, lsl #3]
100d15fc8:     	str	x3, [x23, x17, lsl #3]
100d15fcc:     	str	x2, [x23, x0, lsl #3]
100d15fd0:     	add	x17, x17, #0x1
100d15fd4:     	cmp	x20, x17
100d15fd8:     	b.eq	0x100d15cf4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100d15fdc:     	tst	x17, x12
100d15fe0:     	b.eq	0x100d15fd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100d15fe4:     	and	x0, x17, x13
100d15fe8:     	cbnz	x0, 0x100d15fd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100d15fec:     	eor	x0, x1, x17
100d15ff0:     	cmp	x0, x20
100d15ff4:     	b.lo	0x100d15fc0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x480>
100d15ff8:     	mov	x19, x20
100d15ffc:     	adrp	x8, 0x101500000 <dyld_stub_binder+0x101500000>
100d16000:     	add	x8, x8, #0x430
100d16004:     	str	x8, [sp]
100d16008:     	mov	x1, x19
100d1600c:     	ldr	x2, [sp]
100d16010:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d16014:     	b	0x100d16080 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100d16018:     	mov	x0, x22
100d1601c:     	ldp	x29, x30, [sp, #0x70]
100d16020:     	ldp	x20, x19, [sp, #0x60]
100d16024:     	ldp	x22, x21, [sp, #0x50]
100d16028:     	ldp	x24, x23, [sp, #0x40]
100d1602c:     	ldp	x26, x25, [sp, #0x30]
100d16030:     	ldp	x28, x27, [sp, #0x20]
100d16034:     	add	sp, sp, #0x80
100d16038:     	b	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100d1603c:     	adrp	x0, 0x1013dc000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xbd8>
100d16040:     	add	x0, x0, #0xd8b
100d16044:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16048:     	add	x2, x2, #0x460
100d1604c:     	mov	w1, #0x13               ; =19
100d16050:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d16054:     	b	0x100d16080 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100d16058:     	adrp	x0, 0x10131f000 <dyld_stub_binder+0x10131f000>
100d1605c:     	add	x0, x0, #0xaf5
100d16060:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16064:     	add	x2, x2, #0x448
100d16068:     	mov	w1, #0x37               ; =55
100d1606c:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d16070:     	b	0x100d16080 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100d16074:     	adrp	x0, 0x101500000 <dyld_stub_binder+0x101500000>
100d16078:     	add	x0, x0, #0x400
100d1607c:     	bl	0x10127c934 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100d16080:     	brk	#0x1
100d16084:     	mov	x19, x0
100d16088:     	mov	x0, x22
100d1608c:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100d16090:     	mov	x0, x19
100d16094:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
