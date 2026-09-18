
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba3c10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>:
100ba3c10:     	stp	x24, x23, [sp, #-0x40]!
100ba3c14:     	stp	x22, x21, [sp, #0x10]
100ba3c18:     	stp	x20, x19, [sp, #0x20]
100ba3c1c:     	stp	x29, x30, [sp, #0x30]
100ba3c20:     	add	x29, sp, #0x30
100ba3c24:     	cmp	w2, #0x6
100ba3c28:     	b.hs	0x100ba3c64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x54>
100ba3c2c:     	mov	w8, #0x1                ; =1
100ba3c30:     	lsl	w8, w8, w2
100ba3c34:     	lsl	x9, x1, #3
100ba3c38:     	adrp	x10, 0x1011a3000 <dyld_stub_binder+0x1011a3000>
100ba3c3c:     	add	x10, x10, #0x570
100ba3c40:     	cbz	x9, 0x100ba3cec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100ba3c44:     	ldr	x11, [x0], #0x8
100ba3c48:     	lsr	x12, x11, x8
100ba3c4c:     	eor	x11, x12, x11
100ba3c50:     	ldr	x12, [x10, w2, uxtw #3]
100ba3c54:     	sub	x9, x9, #0x8
100ba3c58:     	and	x11, x11, x12
100ba3c5c:     	cbz	x11, 0x100ba3c40 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x30>
100ba3c60:     	b	0x100ba3cd0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc0>
100ba3c64:     	add	w9, w2, #0x3a
100ba3c68:     	and	w8, w9, #0x3f
100ba3c6c:     	cmp	w8, #0x3f
100ba3c70:     	b.eq	0x100ba3d04 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xf4>
100ba3c74:     	mov	w10, #0x2               ; =2
100ba3c78:     	lsl	x19, x10, x9
100ba3c7c:     	mov	w9, #0x1                ; =1
100ba3c80:     	lsl	x8, x9, x8
100ba3c84:     	neg	x9, x19
100ba3c88:     	and	x9, x1, x9
100ba3c8c:     	cmp	x19, x8
100ba3c90:     	b.lo	0x100ba3ce4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xd4>
100ba3c94:     	cmp	x19, x8, lsl #1
100ba3c98:     	b.ne	0x100ba3cd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc8>
100ba3c9c:     	lsl	x20, x8, #3
100ba3ca0:     	lsl	x21, x19, #3
100ba3ca4:     	add	x22, x9, x19
100ba3ca8:     	sub	x22, x22, x19
100ba3cac:     	cmp	x19, x22
100ba3cb0:     	b.hi	0x100ba3cec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100ba3cb4:     	add	x23, x0, x21
100ba3cb8:     	add	x1, x0, x20
100ba3cbc:     	mov	x2, x20
100ba3cc0:     	bl	0x10110d210 <dyld_stub_binder+0x10110d210>
100ba3cc4:     	mov	x8, x0
100ba3cc8:     	mov	x0, x23
100ba3ccc:     	cbz	w8, 0x100ba3ca8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x98>
100ba3cd0:     	mov	w0, #0x0                ; =0
100ba3cd4:     	b	0x100ba3cf0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100ba3cd8:     	cmp	x19, x9
100ba3cdc:     	cset	w0, hi
100ba3ce0:     	b	0x100ba3cf0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100ba3ce4:     	cmp	x19, x9
100ba3ce8:     	b.ls	0x100ba3d1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x10c>
100ba3cec:     	mov	w0, #0x1                ; =1
100ba3cf0:     	ldp	x29, x30, [sp, #0x30]
100ba3cf4:     	ldp	x20, x19, [sp, #0x20]
100ba3cf8:     	ldp	x22, x21, [sp, #0x10]
100ba3cfc:     	ldp	x24, x23, [sp], #0x40
100ba3d00:     	ret
100ba3d04:     	adrp	x0, 0x1011a1000 <dyld_stub_binder+0x1011a1000>
100ba3d08:     	add	x0, x0, #0xf21
100ba3d0c:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba3d10:     	add	x2, x2, #0x850
100ba3d14:     	mov	w1, #0x37               ; =55
100ba3d18:     	bl	0x101104bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba3d1c:     	adrp	x3, 0x10133a000 <dyld_stub_binder+0x10133a000>
100ba3d20:     	add	x3, x3, #0xb68
100ba3d24:     	mov	x0, #0x0                ; =0
100ba3d28:     	mov	x1, x8
100ba3d2c:     	mov	x2, x19
100ba3d30:     	bl	0x101104c94 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
