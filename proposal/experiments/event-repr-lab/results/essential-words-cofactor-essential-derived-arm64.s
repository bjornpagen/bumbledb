
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba4ad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>:
100ba4ad8:     	stp	d15, d14, [sp, #-0xa0]!
100ba4adc:     	stp	d13, d12, [sp, #0x10]
100ba4ae0:     	stp	d11, d10, [sp, #0x20]
100ba4ae4:     	stp	d9, d8, [sp, #0x30]
100ba4ae8:     	stp	x28, x27, [sp, #0x40]
100ba4aec:     	stp	x26, x25, [sp, #0x50]
100ba4af0:     	stp	x24, x23, [sp, #0x60]
100ba4af4:     	stp	x22, x21, [sp, #0x70]
100ba4af8:     	stp	x20, x19, [sp, #0x80]
100ba4afc:     	stp	x29, x30, [sp, #0x90]
100ba4b00:     	add	x29, sp, #0x90
100ba4b04:     	sub	sp, sp, #0x280
100ba4b08:     	cmp	w4, w3
100ba4b0c:     	b.hs	0x100ba5518 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa40>
100ba4b10:     	mov	x24, x5
100ba4b14:     	mov	x23, x4
100ba4b18:     	mov	x25, x2
100ba4b1c:     	mov	x22, x1
100ba4b20:     	mov	x21, x0
100ba4b24:     	sub	w8, w3, #0x1
100ba4b28:     	and	w28, w8, #0x3f
100ba4b2c:     	mov	w9, #0x1                ; =1
100ba4b30:     	lsl	x27, x9, x8
100ba4b34:     	lsr	x8, x27, #6
100ba4b38:     	cmp	w28, #0x6
100ba4b3c:     	cinc	x19, x8, lo
100ba4b40:     	cbz	x19, 0x100ba4c18 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x140>
100ba4b44:     	lsl	x26, x19, #3
100ba4b48:     	mov	x0, x26
100ba4b4c:     	mov	w1, #0x1                ; =1
100ba4b50:     	bl	0x10110d024 <dyld_stub_binder+0x10110d024>
100ba4b54:     	cbz	x0, 0x100ba5570 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa98>
100ba4b58:     	mov	x20, x0
100ba4b5c:     	cmp	w23, #0x5
100ba4b60:     	str	x27, [sp, #0x208]
100ba4b64:     	b.ls	0x100ba4c28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x150>
100ba4b68:     	add	w8, w23, #0x3a
100ba4b6c:     	and	w26, w8, #0x3f
100ba4b70:     	cmp	w26, #0x3f
100ba4b74:     	b.eq	0x100ba5540 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa68>
100ba4b78:     	mov	w9, #0x1                ; =1
100ba4b7c:     	lsl	x23, x9, x8
100ba4b80:     	mov	w9, #0x2                ; =2
100ba4b84:     	lsl	x2, x9, x8
100ba4b88:     	neg	x8, x2
100ba4b8c:     	and	x8, x25, x8
100ba4b90:     	lsr	x9, x19, x26
100ba4b94:     	sub	x10, x23, #0x1
100ba4b98:     	tst	x19, x10
100ba4b9c:     	cinc	x9, x9, ne
100ba4ba0:     	cmp	x19, #0x0
100ba4ba4:     	csel	x9, xzr, x9, eq
100ba4ba8:     	add	x10, x26, #0x1
100ba4bac:     	lsr	x8, x8, x10
100ba4bb0:     	cmp	x8, x9
100ba4bb4:     	csel	x25, x8, x9, lo
100ba4bb8:     	cbz	x25, 0x100ba54ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba4bbc:     	mov	w8, w24
100ba4bc0:     	lsl	x0, x8, x26
100ba4bc4:     	adds	x1, x0, x23
100ba4bc8:     	b.hs	0x100ba5530 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100ba4bcc:     	cmp	x1, x2
100ba4bd0:     	b.hi	0x100ba5530 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100ba4bd4:     	mov	x24, #0x0               ; =0
100ba4bd8:     	lsl	x27, x2, #3
100ba4bdc:     	add	x22, x22, x0, lsl #3
100ba4be0:     	lsl	x8, x24, x26
100ba4be4:     	sub	x9, x19, x8
100ba4be8:     	cmp	x23, x9
100ba4bec:     	csel	x0, x23, x9, lo
100ba4bf0:     	b.hi	0x100ba5504 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa2c>
100ba4bf4:     	add	x24, x24, #0x1
100ba4bf8:     	lsl	x2, x0, #3
100ba4bfc:     	add	x0, x20, x8, lsl #3
100ba4c00:     	mov	x1, x22
100ba4c04:     	bl	0x10110d21c <dyld_stub_binder+0x10110d21c>
100ba4c08:     	add	x22, x22, x27
100ba4c0c:     	cmp	x25, x24
100ba4c10:     	b.ne	0x100ba4be0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x108>
100ba4c14:     	b	0x100ba54ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba4c18:     	mov	w20, #0x8               ; =8
100ba4c1c:     	cmp	w23, #0x5
100ba4c20:     	str	x27, [sp, #0x208]
100ba4c24:     	b.hi	0x100ba4b68 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x90>
100ba4c28:     	cbz	x25, 0x100ba54ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100ba4c2c:     	mov	x9, #0x0                ; =0
100ba4c30:     	mov	w8, w23
100ba4c34:     	dup.2d	v7, x8
100ba4c38:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4c3c:     	ldr	q0, [x10, #0xcd0]
100ba4c40:     	ushl.2d	v0, v0, v7
100ba4c44:     	stur	q0, [x29, #-0xb0]
100ba4c48:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4c4c:     	ldr	q0, [x10, #0xd00]
100ba4c50:     	ushl.2d	v0, v0, v7
100ba4c54:     	stur	q0, [x29, #-0xc0]
100ba4c58:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4c5c:     	ldr	q0, [x10, #0xd10]
100ba4c60:     	ushl.2d	v0, v0, v7
100ba4c64:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4c68:     	ldr	q1, [x10, #0xd20]
100ba4c6c:     	ushl.2d	v1, v1, v7
100ba4c70:     	mov	w10, #0x3e              ; =62
100ba4c74:     	dup.2d	v2, x10
100ba4c78:     	and.16b	v3, v0, v2
100ba4c7c:     	and.16b	v0, v1, v2
100ba4c80:     	stp	q0, q3, [x29, #-0xe0]
100ba4c84:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4c88:     	ldr	q0, [x10, #0xcf0]
100ba4c8c:     	ushl.2d	v0, v0, v7
100ba4c90:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4c94:     	ldr	q1, [x10, #0xd30]
100ba4c98:     	ushl.2d	v1, v1, v7
100ba4c9c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ca0:     	ldr	q3, [x10, #0xd40]
100ba4ca4:     	ushl.2d	v3, v3, v7
100ba4ca8:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4cac:     	ldr	q4, [x10, #0xd50]
100ba4cb0:     	ushl.2d	v4, v4, v7
100ba4cb4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4cb8:     	ldr	q16, [x10, #0xd60]
100ba4cbc:     	ushl.2d	v16, v16, v7
100ba4cc0:     	and.16b	v5, v1, v2
100ba4cc4:     	and.16b	v1, v3, v2
100ba4cc8:     	stp	q1, q5, [x29, #-0x100]
100ba4ccc:     	and.16b	v3, v4, v2
100ba4cd0:     	and.16b	v1, v16, v2
100ba4cd4:     	stp	q1, q3, [sp, #0x190]
100ba4cd8:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4cdc:     	ldr	q3, [x10, #0xd70]
100ba4ce0:     	ushl.2d	v3, v3, v7
100ba4ce4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ce8:     	ldr	q4, [x10, #0xd80]
100ba4cec:     	ushl.2d	v4, v4, v7
100ba4cf0:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4cf4:     	ldr	q16, [x10, #0xd90]
100ba4cf8:     	ushl.2d	v16, v16, v7
100ba4cfc:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d00:     	ldr	q17, [x10, #0xda0]
100ba4d04:     	ushl.2d	v17, v17, v7
100ba4d08:     	and.16b	v5, v3, v2
100ba4d0c:     	and.16b	v1, v4, v2
100ba4d10:     	stp	q1, q5, [sp, #0x170]
100ba4d14:     	and.16b	v3, v16, v2
100ba4d18:     	and.16b	v1, v17, v2
100ba4d1c:     	stp	q1, q3, [sp, #0x150]
100ba4d20:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d24:     	ldr	q3, [x10, #0xdb0]
100ba4d28:     	ushl.2d	v3, v3, v7
100ba4d2c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d30:     	ldr	q4, [x10, #0xdc0]
100ba4d34:     	ushl.2d	v4, v4, v7
100ba4d38:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d3c:     	ldr	q16, [x10, #0xdd0]
100ba4d40:     	ushl.2d	v16, v16, v7
100ba4d44:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d48:     	ldr	q17, [x10, #0xde0]
100ba4d4c:     	ushl.2d	v17, v17, v7
100ba4d50:     	and.16b	v5, v3, v2
100ba4d54:     	and.16b	v1, v4, v2
100ba4d58:     	stp	q5, q1, [sp, #0x110]
100ba4d5c:     	and.16b	v3, v16, v2
100ba4d60:     	and.16b	v1, v17, v2
100ba4d64:     	stp	q3, q1, [sp, #0x130]
100ba4d68:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d6c:     	ldr	q3, [x10, #0xdf0]
100ba4d70:     	ushl.2d	v3, v3, v7
100ba4d74:     	and.16b	v1, v3, v2
100ba4d78:     	str	q1, [sp, #0x100]
100ba4d7c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d80:     	ldr	q3, [x10, #0xe10]
100ba4d84:     	ushl.2d	v3, v3, v7
100ba4d88:     	and.16b	v1, v3, v2
100ba4d8c:     	str	q1, [sp, #0xf0]
100ba4d90:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4d94:     	ldr	q3, [x10, #0xe20]
100ba4d98:     	ushl.2d	v3, v3, v7
100ba4d9c:     	and.16b	v1, v3, v2
100ba4da0:     	str	q1, [sp, #0xe0]
100ba4da4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4da8:     	ldr	q3, [x10, #0xe40]
100ba4dac:     	ushl.2d	v3, v3, v7
100ba4db0:     	and.16b	v1, v3, v2
100ba4db4:     	str	q1, [sp, #0xd0]
100ba4db8:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4dbc:     	ldr	q3, [x10, #0xe50]
100ba4dc0:     	ushl.2d	v3, v3, v7
100ba4dc4:     	and.16b	v1, v3, v2
100ba4dc8:     	str	q1, [sp, #0xc0]
100ba4dcc:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4dd0:     	ldr	q3, [x10, #0xe70]
100ba4dd4:     	ushl.2d	v3, v3, v7
100ba4dd8:     	and.16b	v1, v3, v2
100ba4ddc:     	str	q1, [sp, #0xb0]
100ba4de0:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4de4:     	ldr	q3, [x10, #0xe80]
100ba4de8:     	ushl.2d	v3, v3, v7
100ba4dec:     	and.16b	v1, v3, v2
100ba4df0:     	str	q1, [sp, #0xa0]
100ba4df4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4df8:     	ldr	q3, [x10, #0xea0]
100ba4dfc:     	ushl.2d	v3, v3, v7
100ba4e00:     	mov	w10, #0x1e              ; =30
100ba4e04:     	dup.2d	v4, x10
100ba4e08:     	and.16b	v1, v3, v4
100ba4e0c:     	str	q1, [sp, #0x90]
100ba4e10:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4e14:     	ldr	q3, [x10, #0xaf0]
100ba4e18:     	ushl.2d	v3, v3, v7
100ba4e1c:     	mov	w10, #0x2f              ; =47
100ba4e20:     	dup.2d	v4, x10
100ba4e24:     	and.16b	v1, v3, v4
100ba4e28:     	str	q1, [sp, #0x70]
100ba4e2c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4e30:     	ldr	q3, [x10, #0xeb0]
100ba4e34:     	ushl.2d	v3, v3, v7
100ba4e38:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4e3c:     	ldr	q4, [x10, #0xec0]
100ba4e40:     	ushl.2d	v4, v4, v7
100ba4e44:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4e48:     	ldr	q16, [x10, #0xed0]
100ba4e4c:     	ushl.2d	v16, v16, v7
100ba4e50:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4e54:     	ldr	q17, [x10, #0xee0]
100ba4e58:     	ushl.2d	v17, v17, v7
100ba4e5c:     	and.16b	v1, v3, v2
100ba4e60:     	str	q1, [sp, #0x80]
100ba4e64:     	and.16b	v26, v4, v2
100ba4e68:     	and.16b	v27, v16, v2
100ba4e6c:     	and.16b	v28, v17, v2
100ba4e70:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4e74:     	ldr	q2, [x10, #0xd10]
100ba4e78:     	ushl.2d	v2, v2, v7
100ba4e7c:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4e80:     	ldr	q3, [x10, #0xef0]
100ba4e84:     	ushl.2d	v3, v3, v7
100ba4e88:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4e8c:     	ldr	q4, [x10, #0xee0]
100ba4e90:     	ushl.2d	v4, v4, v7
100ba4e94:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4e98:     	ldr	q16, [x10, #0xed0]
100ba4e9c:     	ushl.2d	v23, v16, v7
100ba4ea0:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4ea4:     	ldr	q17, [x10, #0xec0]
100ba4ea8:     	ushl.2d	v16, v17, v7
100ba4eac:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4eb0:     	ldr	q18, [x10, #0xeb0]
100ba4eb4:     	ushl.2d	v17, v18, v7
100ba4eb8:     	adrp	x10, 0x101198000 <GCC_except_table8889+0x8>
100ba4ebc:     	ldr	q19, [x10, #0xea0]
100ba4ec0:     	ushl.2d	v18, v19, v7
100ba4ec4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ec8:     	ldr	q20, [x10, #0xa80]
100ba4ecc:     	ushl.2d	v20, v20, v7
100ba4ed0:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ed4:     	ldr	q21, [x10, #0xa90]
100ba4ed8:     	ushl.2d	v21, v21, v7
100ba4edc:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ee0:     	ldr	q22, [x10, #0x980]
100ba4ee4:     	ushl.2d	v22, v22, v7
100ba4ee8:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4eec:     	ldr	q5, [x10, #0xaa0]
100ba4ef0:     	ushl.2d	v5, v5, v7
100ba4ef4:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4ef8:     	ldr	q6, [x10, #0xab0]
100ba4efc:     	ushl.2d	v6, v6, v7
100ba4f00:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f04:     	ldr	q24, [x10, #0xac0]
100ba4f08:     	ushl.2d	v24, v24, v7
100ba4f0c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f10:     	ldr	q25, [x10, #0xad0]
100ba4f14:     	ushl.2d	v25, v25, v7
100ba4f18:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f1c:     	ldr	q1, [x10, #0xae0]
100ba4f20:     	ushl.2d	v1, v1, v7
100ba4f24:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f28:     	ldr	q29, [x10, #0xe00]
100ba4f2c:     	ushl.2d	v29, v29, v7
100ba4f30:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f34:     	ldr	q30, [x10, #0xb20]
100ba4f38:     	ushl.2d	v30, v30, v7
100ba4f3c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f40:     	ldr	q31, [x10, #0xe30]
100ba4f44:     	ushl.2d	v31, v31, v7
100ba4f48:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f4c:     	ldr	q8, [x10, #0xb10]
100ba4f50:     	ushl.2d	v8, v8, v7
100ba4f54:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f58:     	ldr	q9, [x10, #0xe60]
100ba4f5c:     	ushl.2d	v9, v9, v7
100ba4f60:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f64:     	ldr	q10, [x10, #0xb00]
100ba4f68:     	ushl.2d	v10, v10, v7
100ba4f6c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f70:     	ldr	q11, [x10, #0xe90]
100ba4f74:     	ushl.2d	v11, v11, v7
100ba4f78:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f7c:     	ldr	q15, [x10, #0xef0]
100ba4f80:     	ushl.2d	v15, v15, v7
100ba4f84:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f88:     	ldr	q14, [x10, #0xf00]
100ba4f8c:     	ushl.2d	v14, v14, v7
100ba4f90:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4f94:     	ldr	q13, [x10, #0xf10]
100ba4f98:     	ushl.2d	v13, v13, v7
100ba4f9c:     	adrp	x10, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4fa0:     	ldr	q12, [x10, #0xf20]
100ba4fa4:     	ushl.2d	v12, v12, v7
100ba4fa8:     	mov	w10, #0x3f              ; =63
100ba4fac:     	dup.2d	v7, x10
100ba4fb0:     	and.16b	v19, v23, v7
100ba4fb4:     	and.16b	v16, v16, v7
100ba4fb8:     	and.16b	v17, v17, v7
100ba4fbc:     	and.16b	v18, v18, v7
100ba4fc0:     	and.16b	v20, v20, v7
100ba4fc4:     	and.16b	v23, v21, v7
100ba4fc8:     	mov.16b	v21, v20
100ba4fcc:     	and.16b	v20, v22, v7
100ba4fd0:     	mov.16b	v22, v23
100ba4fd4:     	and.16b	v5, v5, v7
100ba4fd8:     	and.16b	v6, v6, v7
100ba4fdc:     	str	q6, [sp, #0x1f0]
100ba4fe0:     	and.16b	v6, v24, v7
100ba4fe4:     	str	q6, [sp, #0x1e0]
100ba4fe8:     	and.16b	v6, v25, v7
100ba4fec:     	and.16b	v1, v1, v7
100ba4ff0:     	stp	q1, q6, [sp, #0x1c0]
100ba4ff4:     	and.16b	v6, v29, v7
100ba4ff8:     	and.16b	v1, v30, v7
100ba4ffc:     	stp	q1, q6, [sp, #0x50]
100ba5000:     	and.16b	v6, v31, v7
100ba5004:     	and.16b	v1, v8, v7
100ba5008:     	stp	q1, q6, [sp, #0x30]
100ba500c:     	mov.16b	v8, v20
100ba5010:     	and.16b	v6, v9, v7
100ba5014:     	mov.16b	v9, v5
100ba5018:     	and.16b	v10, v10, v7
100ba501c:     	and.16b	v29, v11, v7
100ba5020:     	and.16b	v1, v15, v7
100ba5024:     	stp	q1, q6, [sp, #0x10]
100ba5028:     	and.16b	v1, v14, v7
100ba502c:     	str	q1, [sp]
100ba5030:     	and.16b	v30, v13, v7
100ba5034:     	and.16b	v31, v12, v7
100ba5038:     	ldp	q1, q6, [x29, #-0xc0]
100ba503c:     	neg.2d	v5, v6
100ba5040:     	neg.2d	v6, v1
100ba5044:     	ldp	q1, q7, [x29, #-0xe0]
100ba5048:     	neg.2d	v24, v7
100ba504c:     	neg.2d	v25, v1
100ba5050:     	ldp	q1, q7, [x29, #-0x100]
100ba5054:     	neg.2d	v11, v7
100ba5058:     	neg.2d	v1, v1
100ba505c:     	ldr	q7, [sp, #0x1a0]
100ba5060:     	neg.2d	v7, v7
100ba5064:     	stur	q7, [x29, #-0xb0]
100ba5068:     	ldr	q7, [sp, #0x190]
100ba506c:     	neg.2d	v7, v7
100ba5070:     	stur	q7, [x29, #-0xc0]
100ba5074:     	ldr	q7, [sp, #0x180]
100ba5078:     	neg.2d	v7, v7
100ba507c:     	stur	q7, [x29, #-0xd0]
100ba5080:     	ldr	q7, [sp, #0x170]
100ba5084:     	neg.2d	v7, v7
100ba5088:     	stur	q7, [x29, #-0xe0]
100ba508c:     	ldr	q7, [sp, #0x160]
100ba5090:     	neg.2d	v7, v7
100ba5094:     	stur	q7, [x29, #-0xf0]
100ba5098:     	ldr	q7, [sp, #0x150]
100ba509c:     	neg.2d	v7, v7
100ba50a0:     	stur	q7, [x29, #-0x100]
100ba50a4:     	ldr	q7, [sp, #0x110]
100ba50a8:     	neg.2d	v7, v7
100ba50ac:     	str	q7, [sp, #0x1a0]
100ba50b0:     	mov	w10, #0x1               ; =1
100ba50b4:     	ldr	q7, [sp, #0x120]
100ba50b8:     	neg.2d	v7, v7
100ba50bc:     	str	q7, [sp, #0x190]
100ba50c0:     	lsl	x10, x10, x23
100ba50c4:     	ldr	q7, [sp, #0x130]
100ba50c8:     	neg.2d	v7, v7
100ba50cc:     	str	q7, [sp, #0x180]
100ba50d0:     	mov	x11, #-0x1              ; =-1
100ba50d4:     	ldr	q7, [sp, #0x140]
100ba50d8:     	neg.2d	v7, v7
100ba50dc:     	str	q7, [sp, #0x170]
100ba50e0:     	lsl	x10, x11, x10
100ba50e4:     	ldr	q7, [sp, #0x100]
100ba50e8:     	neg.2d	v7, v7
100ba50ec:     	str	q7, [sp, #0x160]
100ba50f0:     	add	x11, x22, x25, lsl #3
100ba50f4:     	ldr	q7, [sp, #0xf0]
100ba50f8:     	neg.2d	v7, v7
100ba50fc:     	str	q7, [sp, #0x150]
100ba5100:     	mov	w12, w24
100ba5104:     	ldr	q7, [sp, #0xe0]
100ba5108:     	neg.2d	v7, v7
100ba510c:     	str	q7, [sp, #0x140]
100ba5110:     	lsl	x12, x12, x8
100ba5114:     	ldr	q7, [sp, #0xd0]
100ba5118:     	neg.2d	v7, v7
100ba511c:     	str	q7, [sp, #0x130]
100ba5120:     	mov	w13, #0x20              ; =32
100ba5124:     	ldr	q7, [sp, #0xc0]
100ba5128:     	neg.2d	v7, v7
100ba512c:     	str	q7, [sp, #0x120]
100ba5130:     	lsr	x13, x13, x8
100ba5134:     	ldr	q7, [sp, #0xb0]
100ba5138:     	neg.2d	v7, v7
100ba513c:     	str	q7, [sp, #0x110]
100ba5140:     	and	x14, x13, #0x38
100ba5144:     	ldr	q7, [sp, #0xa0]
100ba5148:     	neg.2d	v7, v7
100ba514c:     	str	q7, [sp, #0x100]
100ba5150:     	ldr	q7, [sp, #0x90]
100ba5154:     	neg.2d	v7, v7
100ba5158:     	str	q7, [sp, #0xf0]
100ba515c:     	ldr	q7, [sp, #0x80]
100ba5160:     	neg.2d	v7, v7
100ba5164:     	str	q7, [sp, #0xe0]
100ba5168:     	neg.2d	v7, v26
100ba516c:     	str	q7, [sp, #0xd0]
100ba5170:     	neg.2d	v7, v27
100ba5174:     	str	q7, [sp, #0xc0]
100ba5178:     	neg.2d	v7, v28
100ba517c:     	str	q7, [sp, #0xb0]
100ba5180:     	str	q1, [sp, #0x1b0]
100ba5184:     	ldr	x15, [x22]
100ba5188:     	lsr	x15, x15, x12
100ba518c:     	cmp	w23, #0x2
100ba5190:     	b.ls	0x100ba51a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6c8>
100ba5194:     	mov	x17, #0x0               ; =0
100ba5198:     	mov	x16, #0x0               ; =0
100ba519c:     	b	0x100ba544c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x974>
100ba51a0:     	dup.2d	v12, x15
100ba51a4:     	ushl.2d	v7, v12, v5
100ba51a8:     	ushl.2d	v23, v12, v6
100ba51ac:     	ushl.2d	v26, v12, v24
100ba51b0:     	ushl.2d	v27, v12, v25
100ba51b4:     	dup.2d	v13, x10
100ba51b8:     	bic.16b	v7, v7, v13
100ba51bc:     	bic.16b	v28, v23, v13
100ba51c0:     	bic.16b	v14, v26, v13
100ba51c4:     	bic.16b	v15, v27, v13
100ba51c8:     	ushl.2d	v23, v7, v0
100ba51cc:     	ushl.2d	v26, v28, v2
100ba51d0:     	ushl.2d	v27, v14, v3
100ba51d4:     	ushl.2d	v28, v15, v4
100ba51d8:     	cmp	x14, #0x8
100ba51dc:     	b.eq	0x100ba5428 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba51e0:     	ushl.2d	v7, v12, v11
100ba51e4:     	ushl.2d	v14, v12, v1
100ba51e8:     	ldur	q20, [x29, #-0xb0]
100ba51ec:     	ushl.2d	v15, v12, v20
100ba51f0:     	ldur	q20, [x29, #-0xc0]
100ba51f4:     	ushl.2d	v20, v12, v20
100ba51f8:     	bic.16b	v7, v7, v13
100ba51fc:     	bic.16b	v14, v14, v13
100ba5200:     	bic.16b	v15, v15, v13
100ba5204:     	bic.16b	v20, v20, v13
100ba5208:     	ushl.2d	v7, v7, v19
100ba520c:     	ushl.2d	v14, v14, v16
100ba5210:     	ushl.2d	v15, v15, v17
100ba5214:     	ushl.2d	v20, v20, v18
100ba5218:     	orr.16b	v23, v7, v23
100ba521c:     	orr.16b	v26, v14, v26
100ba5220:     	orr.16b	v27, v15, v27
100ba5224:     	orr.16b	v28, v20, v28
100ba5228:     	cmp	x14, #0x10
100ba522c:     	b.eq	0x100ba5428 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba5230:     	ldp	q20, q7, [x29, #-0xe0]
100ba5234:     	ushl.2d	v7, v12, v7
100ba5238:     	ushl.2d	v20, v12, v20
100ba523c:     	ldp	q15, q14, [x29, #-0x100]
100ba5240:     	ushl.2d	v14, v12, v14
100ba5244:     	ushl.2d	v15, v12, v15
100ba5248:     	bic.16b	v7, v7, v13
100ba524c:     	bic.16b	v20, v20, v13
100ba5250:     	bic.16b	v14, v14, v13
100ba5254:     	bic.16b	v15, v15, v13
100ba5258:     	ushl.2d	v7, v7, v21
100ba525c:     	ushl.2d	v20, v20, v22
100ba5260:     	ushl.2d	v14, v14, v8
100ba5264:     	ushl.2d	v15, v15, v9
100ba5268:     	orr.16b	v23, v7, v23
100ba526c:     	orr.16b	v26, v20, v26
100ba5270:     	orr.16b	v27, v14, v27
100ba5274:     	orr.16b	v28, v15, v28
100ba5278:     	cmp	x14, #0x18
100ba527c:     	b.eq	0x100ba5428 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100ba5280:     	ldr	q1, [sp, #0x1a0]
100ba5284:     	ushl.2d	v7, v12, v1
100ba5288:     	ldr	q1, [sp, #0x190]
100ba528c:     	ushl.2d	v20, v12, v1
100ba5290:     	ldr	q1, [sp, #0x180]
100ba5294:     	ushl.2d	v14, v12, v1
100ba5298:     	ldr	q1, [sp, #0x170]
100ba529c:     	ushl.2d	v15, v12, v1
100ba52a0:     	bic.16b	v7, v7, v13
100ba52a4:     	bic.16b	v20, v20, v13
100ba52a8:     	bic.16b	v14, v14, v13
100ba52ac:     	bic.16b	v15, v15, v13
100ba52b0:     	ldr	q1, [sp, #0x1f0]
100ba52b4:     	ushl.2d	v7, v7, v1
100ba52b8:     	ldr	q1, [sp, #0x1e0]
100ba52bc:     	ushl.2d	v20, v20, v1
100ba52c0:     	ldr	q1, [sp, #0x1d0]
100ba52c4:     	ushl.2d	v14, v14, v1
100ba52c8:     	ldr	q1, [sp, #0x1c0]
100ba52cc:     	ushl.2d	v15, v15, v1
100ba52d0:     	orr.16b	v23, v7, v23
100ba52d4:     	orr.16b	v26, v20, v26
100ba52d8:     	orr.16b	v27, v14, v27
100ba52dc:     	orr.16b	v28, v15, v28
100ba52e0:     	cmp	x14, #0x20
100ba52e4:     	b.eq	0x100ba5424 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x94c>
100ba52e8:     	ldr	q1, [sp, #0x160]
100ba52ec:     	ushl.2d	v7, v12, v1
100ba52f0:     	bic.16b	v7, v7, v13
100ba52f4:     	ldr	q1, [sp, #0x60]
100ba52f8:     	ushl.2d	v7, v7, v1
100ba52fc:     	ldr	q1, [sp, #0x150]
100ba5300:     	ushl.2d	v20, v12, v1
100ba5304:     	bic.16b	v20, v20, v13
100ba5308:     	ldr	q1, [sp, #0x50]
100ba530c:     	ushl.2d	v20, v20, v1
100ba5310:     	orr.16b	v1, v20, v7
100ba5314:     	str	q1, [sp, #0x90]
100ba5318:     	ldr	q1, [sp, #0x140]
100ba531c:     	ushl.2d	v20, v12, v1
100ba5320:     	bic.16b	v20, v20, v13
100ba5324:     	ldr	q1, [sp, #0x40]
100ba5328:     	ushl.2d	v20, v20, v1
100ba532c:     	ldr	q1, [sp, #0x130]
100ba5330:     	ushl.2d	v14, v12, v1
100ba5334:     	bic.16b	v14, v14, v13
100ba5338:     	ldr	q1, [sp, #0x30]
100ba533c:     	ushl.2d	v14, v14, v1
100ba5340:     	orr.16b	v1, v14, v20
100ba5344:     	str	q1, [sp, #0x80]
100ba5348:     	ldr	q1, [sp, #0x120]
100ba534c:     	ushl.2d	v14, v12, v1
100ba5350:     	bic.16b	v14, v14, v13
100ba5354:     	ldr	q1, [sp, #0x20]
100ba5358:     	ushl.2d	v14, v14, v1
100ba535c:     	ldp	q1, q7, [sp, #0x100]
100ba5360:     	ushl.2d	v15, v12, v7
100ba5364:     	bic.16b	v15, v15, v13
100ba5368:     	ushl.2d	v15, v15, v10
100ba536c:     	orr.16b	v14, v15, v14
100ba5370:     	ushl.2d	v15, v12, v1
100ba5374:     	bic.16b	v15, v15, v13
100ba5378:     	ushl.2d	v15, v15, v29
100ba537c:     	str	q0, [sp, #0xa0]
100ba5380:     	mov.16b	v7, v18
100ba5384:     	mov.16b	v18, v21
100ba5388:     	ldr	q0, [sp, #0xf0]
100ba538c:     	ushl.2d	v21, v12, v0
100ba5390:     	bic.16b	v21, v21, v13
100ba5394:     	mov.16b	v1, v22
100ba5398:     	ldr	q22, [sp, #0x70]
100ba539c:     	ushl.2d	v21, v21, v22
100ba53a0:     	orr.16b	v21, v21, v15
100ba53a4:     	ldr	q0, [sp, #0xe0]
100ba53a8:     	ushl.2d	v15, v12, v0
100ba53ac:     	ldr	q0, [sp, #0xd0]
100ba53b0:     	ushl.2d	v22, v12, v0
100ba53b4:     	mov.16b	v0, v8
100ba53b8:     	ldp	q20, q8, [sp, #0xb0]
100ba53bc:     	ushl.2d	v8, v12, v8
100ba53c0:     	ushl.2d	v12, v12, v20
100ba53c4:     	bic.16b	v15, v15, v13
100ba53c8:     	bic.16b	v22, v22, v13
100ba53cc:     	bic.16b	v8, v8, v13
100ba53d0:     	bic.16b	v12, v12, v13
100ba53d4:     	ldr	q13, [sp, #0x10]
100ba53d8:     	ushl.2d	v13, v15, v13
100ba53dc:     	orr.16b	v21, v21, v13
100ba53e0:     	orr.16b	v23, v21, v23
100ba53e4:     	ldr	q21, [sp]
100ba53e8:     	ushl.2d	v21, v22, v21
100ba53ec:     	mov.16b	v22, v1
100ba53f0:     	orr.16b	v21, v14, v21
100ba53f4:     	orr.16b	v26, v21, v26
100ba53f8:     	ushl.2d	v21, v8, v30
100ba53fc:     	mov.16b	v8, v0
100ba5400:     	ldp	q0, q1, [sp, #0x80]
100ba5404:     	orr.16b	v20, v0, v21
100ba5408:     	mov.16b	v21, v18
100ba540c:     	mov.16b	v18, v7
100ba5410:     	ldr	q0, [sp, #0xa0]
100ba5414:     	orr.16b	v27, v20, v27
100ba5418:     	ushl.2d	v20, v12, v31
100ba541c:     	orr.16b	v7, v1, v20
100ba5420:     	orr.16b	v28, v7, v28
100ba5424:     	ldr	q1, [sp, #0x1b0]
100ba5428:     	orr.16b	v7, v26, v23
100ba542c:     	orr.16b	v20, v28, v27
100ba5430:     	orr.16b	v7, v20, v7
100ba5434:     	mov	d20, v7[1]
100ba5438:     	orr.8b	v7, v7, v20
100ba543c:     	fmov	x16, d7
100ba5440:     	and	x17, x13, #0x38
100ba5444:     	cmp	x13, x14
100ba5448:     	b.eq	0x100ba547c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9a4>
100ba544c:     	lsl	x0, x17, #1
100ba5450:     	lsl	x1, x17, x8
100ba5454:     	add	x17, x17, #0x1
100ba5458:     	lsl	x2, x0, x8
100ba545c:     	and	x2, x2, #0x3e
100ba5460:     	lsr	x2, x15, x2
100ba5464:     	bic	x2, x2, x10
100ba5468:     	lsl	x1, x2, x1
100ba546c:     	orr	x16, x1, x16
100ba5470:     	add	x0, x0, #0x2
100ba5474:     	cmp	x13, x17
100ba5478:     	b.ne	0x100ba5450 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x978>
100ba547c:     	lsr	x0, x9, #1
100ba5480:     	cmp	x0, x19
100ba5484:     	b.hs	0x100ba555c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa84>
100ba5488:     	ubfiz	x15, x9, #5, #1
100ba548c:     	add	x9, x9, #0x1
100ba5490:     	add	x22, x22, #0x8
100ba5494:     	ldr	x17, [x20, x0, lsl #3]
100ba5498:     	lsl	x15, x16, x15
100ba549c:     	orr	x15, x17, x15
100ba54a0:     	str	x15, [x20, x0, lsl #3]
100ba54a4:     	cmp	x22, x11
100ba54a8:     	b.ne	0x100ba5184 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6ac>
100ba54ac:     	cmp	w28, #0x6
100ba54b0:     	b.hs	0x100ba54cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9f4>
100ba54b4:     	mov	x8, #-0x1               ; =-1
100ba54b8:     	ldr	x9, [sp, #0x208]
100ba54bc:     	lsl	x8, x8, x9
100ba54c0:     	ldr	x9, [x20]
100ba54c4:     	bic	x8, x9, x8
100ba54c8:     	str	x8, [x20]
100ba54cc:     	stp	x19, x20, [x21]
100ba54d0:     	str	x19, [x21, #0x10]
100ba54d4:     	add	sp, sp, #0x280
100ba54d8:     	ldp	x29, x30, [sp, #0x90]
100ba54dc:     	ldp	x20, x19, [sp, #0x80]
100ba54e0:     	ldp	x22, x21, [sp, #0x70]
100ba54e4:     	ldp	x24, x23, [sp, #0x60]
100ba54e8:     	ldp	x26, x25, [sp, #0x50]
100ba54ec:     	ldp	x28, x27, [sp, #0x40]
100ba54f0:     	ldp	d9, d8, [sp, #0x30]
100ba54f4:     	ldp	d11, d10, [sp, #0x20]
100ba54f8:     	ldp	d13, d12, [sp, #0x10]
100ba54fc:     	ldp	d15, d14, [sp], #0xa0
100ba5500:     	ret
100ba5504:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba5508:     	add	x2, x2, #0x9a0
100ba550c:     	mov	x1, x23
100ba5510:     	bl	0x1011050f8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100ba5514:     	b	0x100ba556c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba5518:     	adrp	x0, 0x1011c5000 <dyld_stub_binder+0x1011c5000>
100ba551c:     	add	x0, x0, #0x9c5
100ba5520:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba5524:     	add	x2, x2, #0x958
100ba5528:     	mov	w1, #0x22               ; =34
100ba552c:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba5530:     	adrp	x3, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba5534:     	add	x3, x3, #0x9b8
100ba5538:     	bl	0x101104c94 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100ba553c:     	b	0x100ba556c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba5540:     	adrp	x0, 0x1011a1000 <dyld_stub_binder+0x1011a1000>
100ba5544:     	add	x0, x0, #0xf21
100ba5548:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba554c:     	add	x2, x2, #0x988
100ba5550:     	mov	w1, #0x37               ; =55
100ba5554:     	bl	0x101104bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba5558:     	b	0x100ba556c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100ba555c:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba5560:     	add	x2, x2, #0x970
100ba5564:     	mov	x1, x19
100ba5568:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ba556c:     	brk	#0x1
100ba5570:     	mov	w0, #0x8                ; =8
100ba5574:     	mov	x1, x26
100ba5578:     	bl	0x101104564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba557c:     	cbz	x19, 0x100ba5590 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xab8>
100ba5580:     	mov	x19, x0
100ba5584:     	mov	x0, x20
100ba5588:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100ba558c:     	mov	x0, x19
100ba5590:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
