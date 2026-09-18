
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba1a34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>:
100ba1a34:     	sub	sp, sp, #0x50
100ba1a38:     	stp	x24, x23, [sp, #0x10]
100ba1a3c:     	stp	x22, x21, [sp, #0x20]
100ba1a40:     	stp	x20, x19, [sp, #0x30]
100ba1a44:     	stp	x29, x30, [sp, #0x40]
100ba1a48:     	add	x29, sp, #0x40
100ba1a4c:     	stp	x3, x5, [sp]
100ba1a50:     	cmp	x3, x5
100ba1a54:     	b.ne	0x100ba2068 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x634>
100ba1a58:     	and	x8, x1, #0xff
100ba1a5c:     	mov	x21, x4
100ba1a60:     	mov	x19, x3
100ba1a64:     	mov	x22, x2
100ba1a68:     	mov	x20, x0
100ba1a6c:     	adrp	x9, 0x10119d000 <dyld_stub_binder+0x10119d000>
100ba1a70:     	add	x9, x9, #0x36a
100ba1a74:     	adr	x10, 0x100ba1a84 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x50>
100ba1a78:     	ldrb	w11, [x9, x8]
100ba1a7c:     	add	x10, x10, x11, lsl #2
100ba1a80:     	br	x10
100ba1a84:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1a88:     	lsl	x21, x19, #3
100ba1a8c:     	mov	x0, x21
100ba1a90:     	mov	w1, #0x1                ; =1
100ba1a94:     	bl	0x1011099a4 <dyld_stub_binder+0x1011099a4>
100ba1a98:     	cbnz	x0, 0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba1a9c:     	b	0x100ba2090 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100ba1aa0:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1aa4:     	lsl	x23, x19, #3
100ba1aa8:     	mov	x0, x23
100ba1aac:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1ab0:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1ab4:     	cmp	x19, #0x8
100ba1ab8:     	b.hs	0x100ba1cf0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2bc>
100ba1abc:     	mov	x8, #0x0                ; =0
100ba1ac0:     	b	0x100ba20b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x67c>
100ba1ac4:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1ac8:     	lsl	x23, x19, #3
100ba1acc:     	mov	x0, x23
100ba1ad0:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1ad4:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1ad8:     	cmp	x19, #0x8
100ba1adc:     	b.hs	0x100ba1d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x304>
100ba1ae0:     	mov	x8, #0x0                ; =0
100ba1ae4:     	b	0x100ba20d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x69c>
100ba1ae8:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1aec:     	lsl	x23, x19, #3
100ba1af0:     	mov	x0, x23
100ba1af4:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1af8:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1afc:     	cmp	x19, #0x8
100ba1b00:     	b.hs	0x100ba1d80 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x34c>
100ba1b04:     	mov	x8, #0x0                ; =0
100ba1b08:     	b	0x100ba20f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6bc>
100ba1b0c:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1b10:     	lsl	x21, x19, #3
100ba1b14:     	mov	x0, x21
100ba1b18:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1b1c:     	cbz	x0, 0x100ba2090 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100ba1b20:     	mov	x23, x0
100ba1b24:     	mov	x1, x22
100ba1b28:     	mov	x2, x21
100ba1b2c:     	b	0x100ba1c9c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x268>
100ba1b30:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1b34:     	lsl	x23, x19, #3
100ba1b38:     	mov	x0, x23
100ba1b3c:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1b40:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1b44:     	cmp	x19, #0x8
100ba1b48:     	b.hs	0x100ba1dc8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x394>
100ba1b4c:     	mov	x8, #0x0                ; =0
100ba1b50:     	b	0x100ba2110 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6dc>
100ba1b54:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1b58:     	lsl	x23, x19, #3
100ba1b5c:     	mov	x0, x23
100ba1b60:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1b64:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1b68:     	cmp	x19, #0x8
100ba1b6c:     	b.hs	0x100ba1e20 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3ec>
100ba1b70:     	mov	x8, #0x0                ; =0
100ba1b74:     	b	0x100ba2130 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6fc>
100ba1b78:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1b7c:     	lsl	x21, x19, #3
100ba1b80:     	mov	x0, x21
100ba1b84:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1b88:     	cbz	x0, 0x100ba2090 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100ba1b8c:     	mov	x22, x0
100ba1b90:     	mov	w1, #0xff               ; =255
100ba1b94:     	mov	x2, x21
100ba1b98:     	bl	0x101109bb4 <dyld_stub_binder+0x101109bb4>
100ba1b9c:     	mov	x0, x22
100ba1ba0:     	b	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba1ba4:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1ba8:     	lsl	x21, x19, #3
100ba1bac:     	mov	x0, x21
100ba1bb0:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1bb4:     	cbz	x0, 0x100ba2090 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100ba1bb8:     	cmp	x19, #0x8
100ba1bbc:     	b.hs	0x100ba1e68 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x434>
100ba1bc0:     	mov	x8, #0x0                ; =0
100ba1bc4:     	b	0x100ba2150 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x71c>
100ba1bc8:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1bcc:     	lsl	x23, x19, #3
100ba1bd0:     	mov	x0, x23
100ba1bd4:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1bd8:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1bdc:     	cmp	x19, #0x8
100ba1be0:     	b.hs	0x100ba1ea4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x470>
100ba1be4:     	mov	x8, #0x0                ; =0
100ba1be8:     	b	0x100ba216c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x738>
100ba1bec:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1bf0:     	lsl	x23, x19, #3
100ba1bf4:     	mov	x0, x23
100ba1bf8:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1bfc:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1c00:     	cmp	x19, #0x8
100ba1c04:     	b.hs	0x100ba1efc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4c8>
100ba1c08:     	mov	x8, #0x0                ; =0
100ba1c0c:     	b	0x100ba2190 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x75c>
100ba1c10:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1c14:     	lsl	x22, x19, #3
100ba1c18:     	mov	x0, x22
100ba1c1c:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1c20:     	cbz	x0, 0x100ba209c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x668>
100ba1c24:     	cmp	x19, #0x8
100ba1c28:     	b.hs	0x100ba1f54 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x520>
100ba1c2c:     	mov	x8, #0x0                ; =0
100ba1c30:     	b	0x100ba21b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x780>
100ba1c34:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1c38:     	lsl	x23, x19, #3
100ba1c3c:     	mov	x0, x23
100ba1c40:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1c44:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1c48:     	cmp	x19, #0x8
100ba1c4c:     	b.hs	0x100ba1f90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x55c>
100ba1c50:     	mov	x8, #0x0                ; =0
100ba1c54:     	b	0x100ba21d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x79c>
100ba1c58:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1c5c:     	lsl	x23, x19, #3
100ba1c60:     	mov	x0, x23
100ba1c64:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1c68:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1c6c:     	cmp	x19, #0x8
100ba1c70:     	b.hs	0x100ba1fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5a4>
100ba1c74:     	mov	x8, #0x0                ; =0
100ba1c78:     	b	0x100ba21f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7bc>
100ba1c7c:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1c80:     	lsl	x22, x19, #3
100ba1c84:     	mov	x0, x22
100ba1c88:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1c8c:     	cbz	x0, 0x100ba209c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x668>
100ba1c90:     	mov	x23, x0
100ba1c94:     	mov	x1, x21
100ba1c98:     	mov	x2, x22
100ba1c9c:     	bl	0x101109b9c <dyld_stub_binder+0x101109b9c>
100ba1ca0:     	mov	x0, x23
100ba1ca4:     	b	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba1ca8:     	cbz	x19, 0x100ba1ccc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100ba1cac:     	lsl	x23, x19, #3
100ba1cb0:     	mov	x0, x23
100ba1cb4:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba1cb8:     	cbz	x0, 0x100ba2084 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100ba1cbc:     	cmp	x19, #0x8
100ba1cc0:     	b.hs	0x100ba2020 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5ec>
100ba1cc4:     	mov	x8, #0x0                ; =0
100ba1cc8:     	b	0x100ba2210 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7dc>
100ba1ccc:     	mov	w0, #0x8                ; =8
100ba1cd0:     	stp	x19, x0, [x20]
100ba1cd4:     	str	x19, [x20, #0x10]
100ba1cd8:     	ldp	x29, x30, [sp, #0x40]
100ba1cdc:     	ldp	x20, x19, [sp, #0x30]
100ba1ce0:     	ldp	x22, x21, [sp, #0x20]
100ba1ce4:     	ldp	x24, x23, [sp, #0x10]
100ba1ce8:     	add	sp, sp, #0x50
100ba1cec:     	ret
100ba1cf0:     	and	x8, x19, #0xffffffffffffff8
100ba1cf4:     	add	x9, x22, #0x20
100ba1cf8:     	add	x10, x0, #0x20
100ba1cfc:     	add	x11, x21, #0x20
100ba1d00:     	and	x12, x19, #0xffffffffffffff8
100ba1d04:     	ldp	q0, q1, [x9, #-0x20]
100ba1d08:     	ldp	q2, q3, [x9], #0x40
100ba1d0c:     	ldp	q4, q5, [x11, #-0x20]
100ba1d10:     	ldp	q6, q7, [x11], #0x40
100ba1d14:     	orr.16b	v0, v4, v0
100ba1d18:     	orr.16b	v1, v5, v1
100ba1d1c:     	orr.16b	v2, v6, v2
100ba1d20:     	orr.16b	v3, v7, v3
100ba1d24:     	stp	q0, q1, [x10, #-0x20]
100ba1d28:     	stp	q2, q3, [x10], #0x40
100ba1d2c:     	subs	x12, x12, #0x8
100ba1d30:     	b.ne	0x100ba1d04 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2d0>
100ba1d34:     	b	0x100ba20a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba1d38:     	and	x8, x19, #0xffffffffffffff8
100ba1d3c:     	add	x9, x22, #0x20
100ba1d40:     	add	x10, x0, #0x20
100ba1d44:     	add	x11, x21, #0x20
100ba1d48:     	and	x12, x19, #0xffffffffffffff8
100ba1d4c:     	ldp	q0, q1, [x9, #-0x20]
100ba1d50:     	ldp	q2, q3, [x9], #0x40
100ba1d54:     	ldp	q4, q5, [x11, #-0x20]
100ba1d58:     	ldp	q6, q7, [x11], #0x40
100ba1d5c:     	orn.16b	v0, v4, v0
100ba1d60:     	orn.16b	v1, v5, v1
100ba1d64:     	orn.16b	v2, v6, v2
100ba1d68:     	orn.16b	v3, v7, v3
100ba1d6c:     	stp	q0, q1, [x10, #-0x20]
100ba1d70:     	stp	q2, q3, [x10], #0x40
100ba1d74:     	subs	x12, x12, #0x8
100ba1d78:     	b.ne	0x100ba1d4c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x318>
100ba1d7c:     	b	0x100ba20c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x694>
100ba1d80:     	and	x8, x19, #0xffffffffffffff8
100ba1d84:     	add	x9, x22, #0x20
100ba1d88:     	add	x10, x0, #0x20
100ba1d8c:     	add	x11, x21, #0x20
100ba1d90:     	and	x12, x19, #0xffffffffffffff8
100ba1d94:     	ldp	q0, q1, [x9, #-0x20]
100ba1d98:     	ldp	q2, q3, [x9], #0x40
100ba1d9c:     	ldp	q4, q5, [x11, #-0x20]
100ba1da0:     	ldp	q6, q7, [x11], #0x40
100ba1da4:     	bic.16b	v0, v0, v4
100ba1da8:     	bic.16b	v1, v1, v5
100ba1dac:     	bic.16b	v2, v2, v6
100ba1db0:     	bic.16b	v3, v3, v7
100ba1db4:     	stp	q0, q1, [x10, #-0x20]
100ba1db8:     	stp	q2, q3, [x10], #0x40
100ba1dbc:     	subs	x12, x12, #0x8
100ba1dc0:     	b.ne	0x100ba1d94 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x360>
100ba1dc4:     	b	0x100ba20e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b4>
100ba1dc8:     	and	x8, x19, #0xffffffffffffff8
100ba1dcc:     	add	x9, x22, #0x20
100ba1dd0:     	add	x10, x0, #0x20
100ba1dd4:     	add	x11, x21, #0x20
100ba1dd8:     	and	x12, x19, #0xffffffffffffff8
100ba1ddc:     	ldp	q0, q1, [x9, #-0x20]
100ba1de0:     	ldp	q2, q3, [x9], #0x40
100ba1de4:     	ldp	q4, q5, [x11, #-0x20]
100ba1de8:     	ldp	q6, q7, [x11], #0x40
100ba1dec:     	eor.16b	v0, v0, v4
100ba1df0:     	eor.16b	v1, v1, v5
100ba1df4:     	eor.16b	v2, v2, v6
100ba1df8:     	eor.16b	v3, v3, v7
100ba1dfc:     	mvn.16b	v0, v0
100ba1e00:     	mvn.16b	v1, v1
100ba1e04:     	mvn.16b	v2, v2
100ba1e08:     	mvn.16b	v3, v3
100ba1e0c:     	stp	q0, q1, [x10, #-0x20]
100ba1e10:     	stp	q2, q3, [x10], #0x40
100ba1e14:     	subs	x12, x12, #0x8
100ba1e18:     	b.ne	0x100ba1ddc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3a8>
100ba1e1c:     	b	0x100ba2108 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d4>
100ba1e20:     	and	x8, x19, #0xffffffffffffff8
100ba1e24:     	add	x9, x22, #0x20
100ba1e28:     	add	x10, x0, #0x20
100ba1e2c:     	add	x11, x21, #0x20
100ba1e30:     	and	x12, x19, #0xffffffffffffff8
100ba1e34:     	ldp	q0, q1, [x9, #-0x20]
100ba1e38:     	ldp	q2, q3, [x9], #0x40
100ba1e3c:     	ldp	q4, q5, [x11, #-0x20]
100ba1e40:     	ldp	q6, q7, [x11], #0x40
100ba1e44:     	bic.16b	v0, v4, v0
100ba1e48:     	bic.16b	v1, v5, v1
100ba1e4c:     	bic.16b	v2, v6, v2
100ba1e50:     	bic.16b	v3, v7, v3
100ba1e54:     	stp	q0, q1, [x10, #-0x20]
100ba1e58:     	stp	q2, q3, [x10], #0x40
100ba1e5c:     	subs	x12, x12, #0x8
100ba1e60:     	b.ne	0x100ba1e34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x400>
100ba1e64:     	b	0x100ba2128 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f4>
100ba1e68:     	and	x8, x19, #0xffffffffffffff8
100ba1e6c:     	add	x9, x22, #0x20
100ba1e70:     	add	x10, x0, #0x20
100ba1e74:     	and	x11, x19, #0xffffffffffffff8
100ba1e78:     	ldp	q0, q1, [x9, #-0x20]
100ba1e7c:     	ldp	q2, q3, [x9], #0x40
100ba1e80:     	mvn.16b	v0, v0
100ba1e84:     	mvn.16b	v1, v1
100ba1e88:     	mvn.16b	v2, v2
100ba1e8c:     	mvn.16b	v3, v3
100ba1e90:     	stp	q0, q1, [x10, #-0x20]
100ba1e94:     	stp	q2, q3, [x10], #0x40
100ba1e98:     	subs	x11, x11, #0x8
100ba1e9c:     	b.ne	0x100ba1e78 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x444>
100ba1ea0:     	b	0x100ba2148 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x714>
100ba1ea4:     	and	x8, x19, #0xffffffffffffff8
100ba1ea8:     	add	x9, x22, #0x20
100ba1eac:     	add	x10, x0, #0x20
100ba1eb0:     	add	x11, x21, #0x20
100ba1eb4:     	and	x12, x19, #0xffffffffffffff8
100ba1eb8:     	ldp	q0, q1, [x9, #-0x20]
100ba1ebc:     	ldp	q2, q3, [x9], #0x40
100ba1ec0:     	ldp	q4, q5, [x11, #-0x20]
100ba1ec4:     	ldp	q6, q7, [x11], #0x40
100ba1ec8:     	and.16b	v0, v4, v0
100ba1ecc:     	and.16b	v1, v5, v1
100ba1ed0:     	and.16b	v2, v6, v2
100ba1ed4:     	and.16b	v3, v7, v3
100ba1ed8:     	mvn.16b	v0, v0
100ba1edc:     	mvn.16b	v1, v1
100ba1ee0:     	mvn.16b	v2, v2
100ba1ee4:     	mvn.16b	v3, v3
100ba1ee8:     	stp	q0, q1, [x10, #-0x20]
100ba1eec:     	stp	q2, q3, [x10], #0x40
100ba1ef0:     	subs	x12, x12, #0x8
100ba1ef4:     	b.ne	0x100ba1eb8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x484>
100ba1ef8:     	b	0x100ba2164 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x730>
100ba1efc:     	and	x8, x19, #0xffffffffffffff8
100ba1f00:     	add	x9, x22, #0x20
100ba1f04:     	add	x10, x0, #0x20
100ba1f08:     	add	x11, x21, #0x20
100ba1f0c:     	and	x12, x19, #0xffffffffffffff8
100ba1f10:     	ldp	q0, q1, [x9, #-0x20]
100ba1f14:     	ldp	q2, q3, [x9], #0x40
100ba1f18:     	ldp	q4, q5, [x11, #-0x20]
100ba1f1c:     	ldp	q6, q7, [x11], #0x40
100ba1f20:     	orr.16b	v0, v4, v0
100ba1f24:     	orr.16b	v1, v5, v1
100ba1f28:     	orr.16b	v2, v6, v2
100ba1f2c:     	orr.16b	v3, v7, v3
100ba1f30:     	mvn.16b	v0, v0
100ba1f34:     	mvn.16b	v1, v1
100ba1f38:     	mvn.16b	v2, v2
100ba1f3c:     	mvn.16b	v3, v3
100ba1f40:     	stp	q0, q1, [x10, #-0x20]
100ba1f44:     	stp	q2, q3, [x10], #0x40
100ba1f48:     	subs	x12, x12, #0x8
100ba1f4c:     	b.ne	0x100ba1f10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4dc>
100ba1f50:     	b	0x100ba2188 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100ba1f54:     	and	x8, x19, #0xffffffffffffff8
100ba1f58:     	add	x9, x21, #0x20
100ba1f5c:     	add	x10, x0, #0x20
100ba1f60:     	and	x11, x19, #0xffffffffffffff8
100ba1f64:     	ldp	q0, q1, [x9, #-0x20]
100ba1f68:     	ldp	q2, q3, [x9], #0x40
100ba1f6c:     	mvn.16b	v0, v0
100ba1f70:     	mvn.16b	v1, v1
100ba1f74:     	mvn.16b	v2, v2
100ba1f78:     	mvn.16b	v3, v3
100ba1f7c:     	stp	q0, q1, [x10, #-0x20]
100ba1f80:     	stp	q2, q3, [x10], #0x40
100ba1f84:     	subs	x11, x11, #0x8
100ba1f88:     	b.ne	0x100ba1f64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x530>
100ba1f8c:     	b	0x100ba21ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100ba1f90:     	and	x8, x19, #0xffffffffffffff8
100ba1f94:     	add	x9, x22, #0x20
100ba1f98:     	add	x10, x0, #0x20
100ba1f9c:     	add	x11, x21, #0x20
100ba1fa0:     	and	x12, x19, #0xffffffffffffff8
100ba1fa4:     	ldp	q0, q1, [x9, #-0x20]
100ba1fa8:     	ldp	q2, q3, [x9], #0x40
100ba1fac:     	ldp	q4, q5, [x11, #-0x20]
100ba1fb0:     	ldp	q6, q7, [x11], #0x40
100ba1fb4:     	orn.16b	v0, v0, v4
100ba1fb8:     	orn.16b	v1, v1, v5
100ba1fbc:     	orn.16b	v2, v2, v6
100ba1fc0:     	orn.16b	v3, v3, v7
100ba1fc4:     	stp	q0, q1, [x10, #-0x20]
100ba1fc8:     	stp	q2, q3, [x10], #0x40
100ba1fcc:     	subs	x12, x12, #0x8
100ba1fd0:     	b.ne	0x100ba1fa4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x570>
100ba1fd4:     	b	0x100ba21c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x794>
100ba1fd8:     	and	x8, x19, #0xffffffffffffff8
100ba1fdc:     	add	x9, x22, #0x20
100ba1fe0:     	add	x10, x0, #0x20
100ba1fe4:     	add	x11, x21, #0x20
100ba1fe8:     	and	x12, x19, #0xffffffffffffff8
100ba1fec:     	ldp	q0, q1, [x9, #-0x20]
100ba1ff0:     	ldp	q2, q3, [x9], #0x40
100ba1ff4:     	ldp	q4, q5, [x11, #-0x20]
100ba1ff8:     	ldp	q6, q7, [x11], #0x40
100ba1ffc:     	eor.16b	v0, v4, v0
100ba2000:     	eor.16b	v1, v5, v1
100ba2004:     	eor.16b	v2, v6, v2
100ba2008:     	eor.16b	v3, v7, v3
100ba200c:     	stp	q0, q1, [x10, #-0x20]
100ba2010:     	stp	q2, q3, [x10], #0x40
100ba2014:     	subs	x12, x12, #0x8
100ba2018:     	b.ne	0x100ba1fec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5b8>
100ba201c:     	b	0x100ba21e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b4>
100ba2020:     	and	x8, x19, #0xffffffffffffff8
100ba2024:     	add	x9, x22, #0x20
100ba2028:     	add	x10, x0, #0x20
100ba202c:     	add	x11, x21, #0x20
100ba2030:     	and	x12, x19, #0xffffffffffffff8
100ba2034:     	ldp	q0, q1, [x9, #-0x20]
100ba2038:     	ldp	q2, q3, [x9], #0x40
100ba203c:     	ldp	q4, q5, [x11, #-0x20]
100ba2040:     	ldp	q6, q7, [x11], #0x40
100ba2044:     	and.16b	v0, v4, v0
100ba2048:     	and.16b	v1, v5, v1
100ba204c:     	and.16b	v2, v6, v2
100ba2050:     	and.16b	v3, v7, v3
100ba2054:     	stp	q0, q1, [x10, #-0x20]
100ba2058:     	stp	q2, q3, [x10], #0x40
100ba205c:     	subs	x12, x12, #0x8
100ba2060:     	b.ne	0x100ba2034 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x600>
100ba2064:     	b	0x100ba2208 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d4>
100ba2068:     	adrp	x5, 0x101377000 <dyld_stub_binder+0x101377000>
100ba206c:     	add	x5, x5, #0x7a8
100ba2070:     	mov	x1, sp
100ba2074:     	add	x2, sp, #0x8
100ba2078:     	mov	w0, #0x0                ; =0
100ba207c:     	mov	x3, #0x0                ; =0
100ba2080:     	bl	0x1011015b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100ba2084:     	mov	w0, #0x8                ; =8
100ba2088:     	mov	x1, x23
100ba208c:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba2090:     	mov	w0, #0x8                ; =8
100ba2094:     	mov	x1, x21
100ba2098:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba209c:     	mov	w0, #0x8                ; =8
100ba20a0:     	mov	x1, x22
100ba20a4:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba20a8:     	cmp	x19, x8
100ba20ac:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba20b0:     	ldr	x9, [x22, x8, lsl #3]
100ba20b4:     	ldr	x10, [x21, x8, lsl #3]
100ba20b8:     	orr	x9, x10, x9
100ba20bc:     	str	x9, [x0, x8, lsl #3]
100ba20c0:     	add	x8, x8, #0x1
100ba20c4:     	b	0x100ba20a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba20c8:     	cmp	x19, x8
100ba20cc:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba20d0:     	ldr	x9, [x22, x8, lsl #3]
100ba20d4:     	ldr	x10, [x21, x8, lsl #3]
100ba20d8:     	orn	x9, x10, x9
100ba20dc:     	str	x9, [x0, x8, lsl #3]
100ba20e0:     	add	x8, x8, #0x1
100ba20e4:     	b	0x100ba20c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x694>
100ba20e8:     	cmp	x19, x8
100ba20ec:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba20f0:     	ldr	x9, [x22, x8, lsl #3]
100ba20f4:     	ldr	x10, [x21, x8, lsl #3]
100ba20f8:     	bic	x9, x9, x10
100ba20fc:     	str	x9, [x0, x8, lsl #3]
100ba2100:     	add	x8, x8, #0x1
100ba2104:     	b	0x100ba20e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b4>
100ba2108:     	cmp	x19, x8
100ba210c:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba2110:     	ldr	x9, [x22, x8, lsl #3]
100ba2114:     	ldr	x10, [x21, x8, lsl #3]
100ba2118:     	eon	x9, x9, x10
100ba211c:     	str	x9, [x0, x8, lsl #3]
100ba2120:     	add	x8, x8, #0x1
100ba2124:     	b	0x100ba2108 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d4>
100ba2128:     	cmp	x19, x8
100ba212c:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba2130:     	ldr	x9, [x22, x8, lsl #3]
100ba2134:     	ldr	x10, [x21, x8, lsl #3]
100ba2138:     	bic	x9, x10, x9
100ba213c:     	str	x9, [x0, x8, lsl #3]
100ba2140:     	add	x8, x8, #0x1
100ba2144:     	b	0x100ba2128 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f4>
100ba2148:     	cmp	x19, x8
100ba214c:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba2150:     	ldr	x9, [x22, x8, lsl #3]
100ba2154:     	mvn	x9, x9
100ba2158:     	str	x9, [x0, x8, lsl #3]
100ba215c:     	add	x8, x8, #0x1
100ba2160:     	b	0x100ba2148 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x714>
100ba2164:     	cmp	x19, x8
100ba2168:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba216c:     	ldr	x9, [x22, x8, lsl #3]
100ba2170:     	ldr	x10, [x21, x8, lsl #3]
100ba2174:     	and	x9, x10, x9
100ba2178:     	mvn	x9, x9
100ba217c:     	str	x9, [x0, x8, lsl #3]
100ba2180:     	add	x8, x8, #0x1
100ba2184:     	b	0x100ba2164 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x730>
100ba2188:     	cmp	x19, x8
100ba218c:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba2190:     	ldr	x9, [x22, x8, lsl #3]
100ba2194:     	ldr	x10, [x21, x8, lsl #3]
100ba2198:     	orr	x9, x10, x9
100ba219c:     	mvn	x9, x9
100ba21a0:     	str	x9, [x0, x8, lsl #3]
100ba21a4:     	add	x8, x8, #0x1
100ba21a8:     	b	0x100ba2188 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100ba21ac:     	cmp	x19, x8
100ba21b0:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba21b4:     	ldr	x9, [x21, x8, lsl #3]
100ba21b8:     	mvn	x9, x9
100ba21bc:     	str	x9, [x0, x8, lsl #3]
100ba21c0:     	add	x8, x8, #0x1
100ba21c4:     	b	0x100ba21ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100ba21c8:     	cmp	x19, x8
100ba21cc:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba21d0:     	ldr	x9, [x22, x8, lsl #3]
100ba21d4:     	ldr	x10, [x21, x8, lsl #3]
100ba21d8:     	orn	x9, x9, x10
100ba21dc:     	str	x9, [x0, x8, lsl #3]
100ba21e0:     	add	x8, x8, #0x1
100ba21e4:     	b	0x100ba21c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x794>
100ba21e8:     	cmp	x19, x8
100ba21ec:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba21f0:     	ldr	x9, [x22, x8, lsl #3]
100ba21f4:     	ldr	x10, [x21, x8, lsl #3]
100ba21f8:     	eor	x9, x10, x9
100ba21fc:     	str	x9, [x0, x8, lsl #3]
100ba2200:     	add	x8, x8, #0x1
100ba2204:     	b	0x100ba21e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b4>
100ba2208:     	cmp	x19, x8
100ba220c:     	b.eq	0x100ba1cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100ba2210:     	ldr	x9, [x22, x8, lsl #3]
100ba2214:     	ldr	x10, [x21, x8, lsl #3]
100ba2218:     	and	x9, x10, x9
100ba221c:     	str	x9, [x0, x8, lsl #3]
100ba2220:     	add	x8, x8, #0x1
100ba2224:     	b	0x100ba2208 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d4>
100ba2228:     	nop
100ba222c:     	nop
100ba2230:     	nop
100ba2234:     	nop
100ba2238:     	nop
100ba223c:     	nop
