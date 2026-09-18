
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d1dbd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>:
100d1dbd8:     	stp	d15, d14, [sp, #-0xa0]!
100d1dbdc:     	stp	d13, d12, [sp, #0x10]
100d1dbe0:     	stp	d11, d10, [sp, #0x20]
100d1dbe4:     	stp	d9, d8, [sp, #0x30]
100d1dbe8:     	stp	x28, x27, [sp, #0x40]
100d1dbec:     	stp	x26, x25, [sp, #0x50]
100d1dbf0:     	stp	x24, x23, [sp, #0x60]
100d1dbf4:     	stp	x22, x21, [sp, #0x70]
100d1dbf8:     	stp	x20, x19, [sp, #0x80]
100d1dbfc:     	stp	x29, x30, [sp, #0x90]
100d1dc00:     	add	x29, sp, #0x90
100d1dc04:     	sub	sp, sp, #0x280
100d1dc08:     	cmp	w4, w3
100d1dc0c:     	b.hs	0x100d1e618 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa40>
100d1dc10:     	mov	x24, x5
100d1dc14:     	mov	x23, x4
100d1dc18:     	mov	x25, x2
100d1dc1c:     	mov	x22, x1
100d1dc20:     	mov	x21, x0
100d1dc24:     	sub	w8, w3, #0x1
100d1dc28:     	and	w28, w8, #0x3f
100d1dc2c:     	mov	w9, #0x1                ; =1
100d1dc30:     	lsl	x27, x9, x8
100d1dc34:     	lsr	x8, x27, #6
100d1dc38:     	cmp	w28, #0x6
100d1dc3c:     	cinc	x19, x8, lo
100d1dc40:     	cbz	x19, 0x100d1dd18 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x140>
100d1dc44:     	lsl	x26, x19, #3
100d1dc48:     	mov	x0, x26
100d1dc4c:     	mov	w1, #0x1                ; =1
100d1dc50:     	bl	0x101290724 <dyld_stub_binder+0x101290724>
100d1dc54:     	cbz	x0, 0x100d1e670 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa98>
100d1dc58:     	mov	x20, x0
100d1dc5c:     	cmp	w23, #0x5
100d1dc60:     	str	x27, [sp, #0x208]
100d1dc64:     	b.ls	0x100d1dd28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x150>
100d1dc68:     	add	w8, w23, #0x3a
100d1dc6c:     	and	w26, w8, #0x3f
100d1dc70:     	cmp	w26, #0x3f
100d1dc74:     	b.eq	0x100d1e640 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa68>
100d1dc78:     	mov	w9, #0x1                ; =1
100d1dc7c:     	lsl	x23, x9, x8
100d1dc80:     	mov	w9, #0x2                ; =2
100d1dc84:     	lsl	x2, x9, x8
100d1dc88:     	neg	x8, x2
100d1dc8c:     	and	x8, x25, x8
100d1dc90:     	lsr	x9, x19, x26
100d1dc94:     	sub	x10, x23, #0x1
100d1dc98:     	tst	x19, x10
100d1dc9c:     	cinc	x9, x9, ne
100d1dca0:     	cmp	x19, #0x0
100d1dca4:     	csel	x9, xzr, x9, eq
100d1dca8:     	add	x10, x26, #0x1
100d1dcac:     	lsr	x8, x8, x10
100d1dcb0:     	cmp	x8, x9
100d1dcb4:     	csel	x25, x8, x9, lo
100d1dcb8:     	cbz	x25, 0x100d1e5ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1dcbc:     	mov	w8, w24
100d1dcc0:     	lsl	x0, x8, x26
100d1dcc4:     	adds	x1, x0, x23
100d1dcc8:     	b.hs	0x100d1e630 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d1dccc:     	cmp	x1, x2
100d1dcd0:     	b.hi	0x100d1e630 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d1dcd4:     	mov	x24, #0x0               ; =0
100d1dcd8:     	lsl	x27, x2, #3
100d1dcdc:     	add	x22, x22, x0, lsl #3
100d1dce0:     	lsl	x8, x24, x26
100d1dce4:     	sub	x9, x19, x8
100d1dce8:     	cmp	x23, x9
100d1dcec:     	csel	x0, x23, x9, lo
100d1dcf0:     	b.hi	0x100d1e604 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa2c>
100d1dcf4:     	add	x24, x24, #0x1
100d1dcf8:     	lsl	x2, x0, #3
100d1dcfc:     	add	x0, x20, x8, lsl #3
100d1dd00:     	mov	x1, x22
100d1dd04:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d1dd08:     	add	x22, x22, x27
100d1dd0c:     	cmp	x25, x24
100d1dd10:     	b.ne	0x100d1dce0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x108>
100d1dd14:     	b	0x100d1e5ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1dd18:     	mov	w20, #0x8               ; =8
100d1dd1c:     	cmp	w23, #0x5
100d1dd20:     	str	x27, [sp, #0x208]
100d1dd24:     	b.hi	0x100d1dc68 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x90>
100d1dd28:     	cbz	x25, 0x100d1e5ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1dd2c:     	mov	x9, #0x0                ; =0
100d1dd30:     	mov	w8, w23
100d1dd34:     	dup.2d	v7, x8
100d1dd38:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1dd3c:     	ldr	q0, [x10, #0x720]
100d1dd40:     	ushl.2d	v0, v0, v7
100d1dd44:     	stur	q0, [x29, #-0xb0]
100d1dd48:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dd4c:     	ldr	q0, [x10, #0x870]
100d1dd50:     	ushl.2d	v0, v0, v7
100d1dd54:     	stur	q0, [x29, #-0xc0]
100d1dd58:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dd5c:     	ldr	q0, [x10, #0x880]
100d1dd60:     	ushl.2d	v0, v0, v7
100d1dd64:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dd68:     	ldr	q1, [x10, #0x890]
100d1dd6c:     	ushl.2d	v1, v1, v7
100d1dd70:     	mov	w10, #0x3e              ; =62
100d1dd74:     	dup.2d	v2, x10
100d1dd78:     	and.16b	v3, v0, v2
100d1dd7c:     	and.16b	v0, v1, v2
100d1dd80:     	stp	q0, q3, [x29, #-0xe0]
100d1dd84:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1dd88:     	ldr	q0, [x10, #0x740]
100d1dd8c:     	ushl.2d	v0, v0, v7
100d1dd90:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dd94:     	ldr	q1, [x10, #0x8a0]
100d1dd98:     	ushl.2d	v1, v1, v7
100d1dd9c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dda0:     	ldr	q3, [x10, #0x8b0]
100d1dda4:     	ushl.2d	v3, v3, v7
100d1dda8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1ddac:     	ldr	q4, [x10, #0x8c0]
100d1ddb0:     	ushl.2d	v4, v4, v7
100d1ddb4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1ddb8:     	ldr	q16, [x10, #0x8d0]
100d1ddbc:     	ushl.2d	v16, v16, v7
100d1ddc0:     	and.16b	v5, v1, v2
100d1ddc4:     	and.16b	v1, v3, v2
100d1ddc8:     	stp	q1, q5, [x29, #-0x100]
100d1ddcc:     	and.16b	v3, v4, v2
100d1ddd0:     	and.16b	v1, v16, v2
100d1ddd4:     	stp	q1, q3, [sp, #0x190]
100d1ddd8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dddc:     	ldr	q3, [x10, #0x8e0]
100d1dde0:     	ushl.2d	v3, v3, v7
100d1dde4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dde8:     	ldr	q4, [x10, #0x8f0]
100d1ddec:     	ushl.2d	v4, v4, v7
100d1ddf0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1ddf4:     	ldr	q16, [x10, #0x900]
100d1ddf8:     	ushl.2d	v16, v16, v7
100d1ddfc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de00:     	ldr	q17, [x10, #0x910]
100d1de04:     	ushl.2d	v17, v17, v7
100d1de08:     	and.16b	v5, v3, v2
100d1de0c:     	and.16b	v1, v4, v2
100d1de10:     	stp	q1, q5, [sp, #0x170]
100d1de14:     	and.16b	v3, v16, v2
100d1de18:     	and.16b	v1, v17, v2
100d1de1c:     	stp	q1, q3, [sp, #0x150]
100d1de20:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de24:     	ldr	q3, [x10, #0x920]
100d1de28:     	ushl.2d	v3, v3, v7
100d1de2c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de30:     	ldr	q4, [x10, #0x930]
100d1de34:     	ushl.2d	v4, v4, v7
100d1de38:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de3c:     	ldr	q16, [x10, #0x940]
100d1de40:     	ushl.2d	v16, v16, v7
100d1de44:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de48:     	ldr	q17, [x10, #0x950]
100d1de4c:     	ushl.2d	v17, v17, v7
100d1de50:     	and.16b	v5, v3, v2
100d1de54:     	and.16b	v1, v4, v2
100d1de58:     	stp	q5, q1, [sp, #0x110]
100d1de5c:     	and.16b	v3, v16, v2
100d1de60:     	and.16b	v1, v17, v2
100d1de64:     	stp	q3, q1, [sp, #0x130]
100d1de68:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de6c:     	ldr	q3, [x10, #0x960]
100d1de70:     	ushl.2d	v3, v3, v7
100d1de74:     	and.16b	v1, v3, v2
100d1de78:     	str	q1, [sp, #0x100]
100d1de7c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de80:     	ldr	q3, [x10, #0x980]
100d1de84:     	ushl.2d	v3, v3, v7
100d1de88:     	and.16b	v1, v3, v2
100d1de8c:     	str	q1, [sp, #0xf0]
100d1de90:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1de94:     	ldr	q3, [x10, #0x990]
100d1de98:     	ushl.2d	v3, v3, v7
100d1de9c:     	and.16b	v1, v3, v2
100d1dea0:     	str	q1, [sp, #0xe0]
100d1dea4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dea8:     	ldr	q3, [x10, #0x9b0]
100d1deac:     	ushl.2d	v3, v3, v7
100d1deb0:     	and.16b	v1, v3, v2
100d1deb4:     	str	q1, [sp, #0xd0]
100d1deb8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1debc:     	ldr	q3, [x10, #0x9c0]
100d1dec0:     	ushl.2d	v3, v3, v7
100d1dec4:     	and.16b	v1, v3, v2
100d1dec8:     	str	q1, [sp, #0xc0]
100d1decc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1ded0:     	ldr	q3, [x10, #0x9e0]
100d1ded4:     	ushl.2d	v3, v3, v7
100d1ded8:     	and.16b	v1, v3, v2
100d1dedc:     	str	q1, [sp, #0xb0]
100d1dee0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dee4:     	ldr	q3, [x10, #0x9f0]
100d1dee8:     	ushl.2d	v3, v3, v7
100d1deec:     	and.16b	v1, v3, v2
100d1def0:     	str	q1, [sp, #0xa0]
100d1def4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1def8:     	ldr	q3, [x10, #0xa10]
100d1defc:     	ushl.2d	v3, v3, v7
100d1df00:     	mov	w10, #0x1e              ; =30
100d1df04:     	dup.2d	v4, x10
100d1df08:     	and.16b	v1, v3, v4
100d1df0c:     	str	q1, [sp, #0x90]
100d1df10:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1df14:     	ldr	q3, [x10, #0x660]
100d1df18:     	ushl.2d	v3, v3, v7
100d1df1c:     	mov	w10, #0x2f              ; =47
100d1df20:     	dup.2d	v4, x10
100d1df24:     	and.16b	v1, v3, v4
100d1df28:     	str	q1, [sp, #0x70]
100d1df2c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1df30:     	ldr	q3, [x10, #0xa20]
100d1df34:     	ushl.2d	v3, v3, v7
100d1df38:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1df3c:     	ldr	q4, [x10, #0xa30]
100d1df40:     	ushl.2d	v4, v4, v7
100d1df44:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1df48:     	ldr	q16, [x10, #0xa40]
100d1df4c:     	ushl.2d	v16, v16, v7
100d1df50:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1df54:     	ldr	q17, [x10, #0xa50]
100d1df58:     	ushl.2d	v17, v17, v7
100d1df5c:     	and.16b	v1, v3, v2
100d1df60:     	str	q1, [sp, #0x80]
100d1df64:     	and.16b	v26, v4, v2
100d1df68:     	and.16b	v27, v16, v2
100d1df6c:     	and.16b	v28, v17, v2
100d1df70:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1df74:     	ldr	q2, [x10, #0x760]
100d1df78:     	ushl.2d	v2, v2, v7
100d1df7c:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1df80:     	ldr	q3, [x10, #0x940]
100d1df84:     	ushl.2d	v3, v3, v7
100d1df88:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1df8c:     	ldr	q4, [x10, #0x930]
100d1df90:     	ushl.2d	v4, v4, v7
100d1df94:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1df98:     	ldr	q16, [x10, #0x920]
100d1df9c:     	ushl.2d	v23, v16, v7
100d1dfa0:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1dfa4:     	ldr	q17, [x10, #0x910]
100d1dfa8:     	ushl.2d	v16, v17, v7
100d1dfac:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1dfb0:     	ldr	q18, [x10, #0x900]
100d1dfb4:     	ushl.2d	v17, v18, v7
100d1dfb8:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d1dfbc:     	ldr	q19, [x10, #0x8f0]
100d1dfc0:     	ushl.2d	v18, v19, v7
100d1dfc4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dfc8:     	ldr	q20, [x10, #0x5f0]
100d1dfcc:     	ushl.2d	v20, v20, v7
100d1dfd0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dfd4:     	ldr	q21, [x10, #0x600]
100d1dfd8:     	ushl.2d	v21, v21, v7
100d1dfdc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dfe0:     	ldr	q22, [x10, #0x4f0]
100d1dfe4:     	ushl.2d	v22, v22, v7
100d1dfe8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dfec:     	ldr	q5, [x10, #0x610]
100d1dff0:     	ushl.2d	v5, v5, v7
100d1dff4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1dff8:     	ldr	q6, [x10, #0x620]
100d1dffc:     	ushl.2d	v6, v6, v7
100d1e000:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e004:     	ldr	q24, [x10, #0x630]
100d1e008:     	ushl.2d	v24, v24, v7
100d1e00c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e010:     	ldr	q25, [x10, #0x640]
100d1e014:     	ushl.2d	v25, v25, v7
100d1e018:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e01c:     	ldr	q1, [x10, #0x650]
100d1e020:     	ushl.2d	v1, v1, v7
100d1e024:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e028:     	ldr	q29, [x10, #0x970]
100d1e02c:     	ushl.2d	v29, v29, v7
100d1e030:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e034:     	ldr	q30, [x10, #0x690]
100d1e038:     	ushl.2d	v30, v30, v7
100d1e03c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e040:     	ldr	q31, [x10, #0x9a0]
100d1e044:     	ushl.2d	v31, v31, v7
100d1e048:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e04c:     	ldr	q8, [x10, #0x680]
100d1e050:     	ushl.2d	v8, v8, v7
100d1e054:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e058:     	ldr	q9, [x10, #0x9d0]
100d1e05c:     	ushl.2d	v9, v9, v7
100d1e060:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e064:     	ldr	q10, [x10, #0x670]
100d1e068:     	ushl.2d	v10, v10, v7
100d1e06c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e070:     	ldr	q11, [x10, #0xa00]
100d1e074:     	ushl.2d	v11, v11, v7
100d1e078:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e07c:     	ldr	q15, [x10, #0xa60]
100d1e080:     	ushl.2d	v15, v15, v7
100d1e084:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e088:     	ldr	q14, [x10, #0xa70]
100d1e08c:     	ushl.2d	v14, v14, v7
100d1e090:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e094:     	ldr	q13, [x10, #0xa80]
100d1e098:     	ushl.2d	v13, v13, v7
100d1e09c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d1e0a0:     	ldr	q12, [x10, #0xa90]
100d1e0a4:     	ushl.2d	v12, v12, v7
100d1e0a8:     	mov	w10, #0x3f              ; =63
100d1e0ac:     	dup.2d	v7, x10
100d1e0b0:     	and.16b	v19, v23, v7
100d1e0b4:     	and.16b	v16, v16, v7
100d1e0b8:     	and.16b	v17, v17, v7
100d1e0bc:     	and.16b	v18, v18, v7
100d1e0c0:     	and.16b	v20, v20, v7
100d1e0c4:     	and.16b	v23, v21, v7
100d1e0c8:     	mov.16b	v21, v20
100d1e0cc:     	and.16b	v20, v22, v7
100d1e0d0:     	mov.16b	v22, v23
100d1e0d4:     	and.16b	v5, v5, v7
100d1e0d8:     	and.16b	v6, v6, v7
100d1e0dc:     	str	q6, [sp, #0x1f0]
100d1e0e0:     	and.16b	v6, v24, v7
100d1e0e4:     	str	q6, [sp, #0x1e0]
100d1e0e8:     	and.16b	v6, v25, v7
100d1e0ec:     	and.16b	v1, v1, v7
100d1e0f0:     	stp	q1, q6, [sp, #0x1c0]
100d1e0f4:     	and.16b	v6, v29, v7
100d1e0f8:     	and.16b	v1, v30, v7
100d1e0fc:     	stp	q1, q6, [sp, #0x50]
100d1e100:     	and.16b	v6, v31, v7
100d1e104:     	and.16b	v1, v8, v7
100d1e108:     	stp	q1, q6, [sp, #0x30]
100d1e10c:     	mov.16b	v8, v20
100d1e110:     	and.16b	v6, v9, v7
100d1e114:     	mov.16b	v9, v5
100d1e118:     	and.16b	v10, v10, v7
100d1e11c:     	and.16b	v29, v11, v7
100d1e120:     	and.16b	v1, v15, v7
100d1e124:     	stp	q1, q6, [sp, #0x10]
100d1e128:     	and.16b	v1, v14, v7
100d1e12c:     	str	q1, [sp]
100d1e130:     	and.16b	v30, v13, v7
100d1e134:     	and.16b	v31, v12, v7
100d1e138:     	ldp	q1, q6, [x29, #-0xc0]
100d1e13c:     	neg.2d	v5, v6
100d1e140:     	neg.2d	v6, v1
100d1e144:     	ldp	q1, q7, [x29, #-0xe0]
100d1e148:     	neg.2d	v24, v7
100d1e14c:     	neg.2d	v25, v1
100d1e150:     	ldp	q1, q7, [x29, #-0x100]
100d1e154:     	neg.2d	v11, v7
100d1e158:     	neg.2d	v1, v1
100d1e15c:     	ldr	q7, [sp, #0x1a0]
100d1e160:     	neg.2d	v7, v7
100d1e164:     	stur	q7, [x29, #-0xb0]
100d1e168:     	ldr	q7, [sp, #0x190]
100d1e16c:     	neg.2d	v7, v7
100d1e170:     	stur	q7, [x29, #-0xc0]
100d1e174:     	ldr	q7, [sp, #0x180]
100d1e178:     	neg.2d	v7, v7
100d1e17c:     	stur	q7, [x29, #-0xd0]
100d1e180:     	ldr	q7, [sp, #0x170]
100d1e184:     	neg.2d	v7, v7
100d1e188:     	stur	q7, [x29, #-0xe0]
100d1e18c:     	ldr	q7, [sp, #0x160]
100d1e190:     	neg.2d	v7, v7
100d1e194:     	stur	q7, [x29, #-0xf0]
100d1e198:     	ldr	q7, [sp, #0x150]
100d1e19c:     	neg.2d	v7, v7
100d1e1a0:     	stur	q7, [x29, #-0x100]
100d1e1a4:     	ldr	q7, [sp, #0x110]
100d1e1a8:     	neg.2d	v7, v7
100d1e1ac:     	str	q7, [sp, #0x1a0]
100d1e1b0:     	mov	w10, #0x1               ; =1
100d1e1b4:     	ldr	q7, [sp, #0x120]
100d1e1b8:     	neg.2d	v7, v7
100d1e1bc:     	str	q7, [sp, #0x190]
100d1e1c0:     	lsl	x10, x10, x23
100d1e1c4:     	ldr	q7, [sp, #0x130]
100d1e1c8:     	neg.2d	v7, v7
100d1e1cc:     	str	q7, [sp, #0x180]
100d1e1d0:     	mov	x11, #-0x1              ; =-1
100d1e1d4:     	ldr	q7, [sp, #0x140]
100d1e1d8:     	neg.2d	v7, v7
100d1e1dc:     	str	q7, [sp, #0x170]
100d1e1e0:     	lsl	x10, x11, x10
100d1e1e4:     	ldr	q7, [sp, #0x100]
100d1e1e8:     	neg.2d	v7, v7
100d1e1ec:     	str	q7, [sp, #0x160]
100d1e1f0:     	add	x11, x22, x25, lsl #3
100d1e1f4:     	ldr	q7, [sp, #0xf0]
100d1e1f8:     	neg.2d	v7, v7
100d1e1fc:     	str	q7, [sp, #0x150]
100d1e200:     	mov	w12, w24
100d1e204:     	ldr	q7, [sp, #0xe0]
100d1e208:     	neg.2d	v7, v7
100d1e20c:     	str	q7, [sp, #0x140]
100d1e210:     	lsl	x12, x12, x8
100d1e214:     	ldr	q7, [sp, #0xd0]
100d1e218:     	neg.2d	v7, v7
100d1e21c:     	str	q7, [sp, #0x130]
100d1e220:     	mov	w13, #0x20              ; =32
100d1e224:     	ldr	q7, [sp, #0xc0]
100d1e228:     	neg.2d	v7, v7
100d1e22c:     	str	q7, [sp, #0x120]
100d1e230:     	lsr	x13, x13, x8
100d1e234:     	ldr	q7, [sp, #0xb0]
100d1e238:     	neg.2d	v7, v7
100d1e23c:     	str	q7, [sp, #0x110]
100d1e240:     	and	x14, x13, #0x38
100d1e244:     	ldr	q7, [sp, #0xa0]
100d1e248:     	neg.2d	v7, v7
100d1e24c:     	str	q7, [sp, #0x100]
100d1e250:     	ldr	q7, [sp, #0x90]
100d1e254:     	neg.2d	v7, v7
100d1e258:     	str	q7, [sp, #0xf0]
100d1e25c:     	ldr	q7, [sp, #0x80]
100d1e260:     	neg.2d	v7, v7
100d1e264:     	str	q7, [sp, #0xe0]
100d1e268:     	neg.2d	v7, v26
100d1e26c:     	str	q7, [sp, #0xd0]
100d1e270:     	neg.2d	v7, v27
100d1e274:     	str	q7, [sp, #0xc0]
100d1e278:     	neg.2d	v7, v28
100d1e27c:     	str	q7, [sp, #0xb0]
100d1e280:     	str	q1, [sp, #0x1b0]
100d1e284:     	ldr	x15, [x22]
100d1e288:     	lsr	x15, x15, x12
100d1e28c:     	cmp	w23, #0x2
100d1e290:     	b.ls	0x100d1e2a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6c8>
100d1e294:     	mov	x17, #0x0               ; =0
100d1e298:     	mov	x16, #0x0               ; =0
100d1e29c:     	b	0x100d1e54c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x974>
100d1e2a0:     	dup.2d	v12, x15
100d1e2a4:     	ushl.2d	v7, v12, v5
100d1e2a8:     	ushl.2d	v23, v12, v6
100d1e2ac:     	ushl.2d	v26, v12, v24
100d1e2b0:     	ushl.2d	v27, v12, v25
100d1e2b4:     	dup.2d	v13, x10
100d1e2b8:     	bic.16b	v7, v7, v13
100d1e2bc:     	bic.16b	v28, v23, v13
100d1e2c0:     	bic.16b	v14, v26, v13
100d1e2c4:     	bic.16b	v15, v27, v13
100d1e2c8:     	ushl.2d	v23, v7, v0
100d1e2cc:     	ushl.2d	v26, v28, v2
100d1e2d0:     	ushl.2d	v27, v14, v3
100d1e2d4:     	ushl.2d	v28, v15, v4
100d1e2d8:     	cmp	x14, #0x8
100d1e2dc:     	b.eq	0x100d1e528 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1e2e0:     	ushl.2d	v7, v12, v11
100d1e2e4:     	ushl.2d	v14, v12, v1
100d1e2e8:     	ldur	q20, [x29, #-0xb0]
100d1e2ec:     	ushl.2d	v15, v12, v20
100d1e2f0:     	ldur	q20, [x29, #-0xc0]
100d1e2f4:     	ushl.2d	v20, v12, v20
100d1e2f8:     	bic.16b	v7, v7, v13
100d1e2fc:     	bic.16b	v14, v14, v13
100d1e300:     	bic.16b	v15, v15, v13
100d1e304:     	bic.16b	v20, v20, v13
100d1e308:     	ushl.2d	v7, v7, v19
100d1e30c:     	ushl.2d	v14, v14, v16
100d1e310:     	ushl.2d	v15, v15, v17
100d1e314:     	ushl.2d	v20, v20, v18
100d1e318:     	orr.16b	v23, v7, v23
100d1e31c:     	orr.16b	v26, v14, v26
100d1e320:     	orr.16b	v27, v15, v27
100d1e324:     	orr.16b	v28, v20, v28
100d1e328:     	cmp	x14, #0x10
100d1e32c:     	b.eq	0x100d1e528 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1e330:     	ldp	q20, q7, [x29, #-0xe0]
100d1e334:     	ushl.2d	v7, v12, v7
100d1e338:     	ushl.2d	v20, v12, v20
100d1e33c:     	ldp	q15, q14, [x29, #-0x100]
100d1e340:     	ushl.2d	v14, v12, v14
100d1e344:     	ushl.2d	v15, v12, v15
100d1e348:     	bic.16b	v7, v7, v13
100d1e34c:     	bic.16b	v20, v20, v13
100d1e350:     	bic.16b	v14, v14, v13
100d1e354:     	bic.16b	v15, v15, v13
100d1e358:     	ushl.2d	v7, v7, v21
100d1e35c:     	ushl.2d	v20, v20, v22
100d1e360:     	ushl.2d	v14, v14, v8
100d1e364:     	ushl.2d	v15, v15, v9
100d1e368:     	orr.16b	v23, v7, v23
100d1e36c:     	orr.16b	v26, v20, v26
100d1e370:     	orr.16b	v27, v14, v27
100d1e374:     	orr.16b	v28, v15, v28
100d1e378:     	cmp	x14, #0x18
100d1e37c:     	b.eq	0x100d1e528 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1e380:     	ldr	q1, [sp, #0x1a0]
100d1e384:     	ushl.2d	v7, v12, v1
100d1e388:     	ldr	q1, [sp, #0x190]
100d1e38c:     	ushl.2d	v20, v12, v1
100d1e390:     	ldr	q1, [sp, #0x180]
100d1e394:     	ushl.2d	v14, v12, v1
100d1e398:     	ldr	q1, [sp, #0x170]
100d1e39c:     	ushl.2d	v15, v12, v1
100d1e3a0:     	bic.16b	v7, v7, v13
100d1e3a4:     	bic.16b	v20, v20, v13
100d1e3a8:     	bic.16b	v14, v14, v13
100d1e3ac:     	bic.16b	v15, v15, v13
100d1e3b0:     	ldr	q1, [sp, #0x1f0]
100d1e3b4:     	ushl.2d	v7, v7, v1
100d1e3b8:     	ldr	q1, [sp, #0x1e0]
100d1e3bc:     	ushl.2d	v20, v20, v1
100d1e3c0:     	ldr	q1, [sp, #0x1d0]
100d1e3c4:     	ushl.2d	v14, v14, v1
100d1e3c8:     	ldr	q1, [sp, #0x1c0]
100d1e3cc:     	ushl.2d	v15, v15, v1
100d1e3d0:     	orr.16b	v23, v7, v23
100d1e3d4:     	orr.16b	v26, v20, v26
100d1e3d8:     	orr.16b	v27, v14, v27
100d1e3dc:     	orr.16b	v28, v15, v28
100d1e3e0:     	cmp	x14, #0x20
100d1e3e4:     	b.eq	0x100d1e524 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x94c>
100d1e3e8:     	ldr	q1, [sp, #0x160]
100d1e3ec:     	ushl.2d	v7, v12, v1
100d1e3f0:     	bic.16b	v7, v7, v13
100d1e3f4:     	ldr	q1, [sp, #0x60]
100d1e3f8:     	ushl.2d	v7, v7, v1
100d1e3fc:     	ldr	q1, [sp, #0x150]
100d1e400:     	ushl.2d	v20, v12, v1
100d1e404:     	bic.16b	v20, v20, v13
100d1e408:     	ldr	q1, [sp, #0x50]
100d1e40c:     	ushl.2d	v20, v20, v1
100d1e410:     	orr.16b	v1, v20, v7
100d1e414:     	str	q1, [sp, #0x90]
100d1e418:     	ldr	q1, [sp, #0x140]
100d1e41c:     	ushl.2d	v20, v12, v1
100d1e420:     	bic.16b	v20, v20, v13
100d1e424:     	ldr	q1, [sp, #0x40]
100d1e428:     	ushl.2d	v20, v20, v1
100d1e42c:     	ldr	q1, [sp, #0x130]
100d1e430:     	ushl.2d	v14, v12, v1
100d1e434:     	bic.16b	v14, v14, v13
100d1e438:     	ldr	q1, [sp, #0x30]
100d1e43c:     	ushl.2d	v14, v14, v1
100d1e440:     	orr.16b	v1, v14, v20
100d1e444:     	str	q1, [sp, #0x80]
100d1e448:     	ldr	q1, [sp, #0x120]
100d1e44c:     	ushl.2d	v14, v12, v1
100d1e450:     	bic.16b	v14, v14, v13
100d1e454:     	ldr	q1, [sp, #0x20]
100d1e458:     	ushl.2d	v14, v14, v1
100d1e45c:     	ldp	q1, q7, [sp, #0x100]
100d1e460:     	ushl.2d	v15, v12, v7
100d1e464:     	bic.16b	v15, v15, v13
100d1e468:     	ushl.2d	v15, v15, v10
100d1e46c:     	orr.16b	v14, v15, v14
100d1e470:     	ushl.2d	v15, v12, v1
100d1e474:     	bic.16b	v15, v15, v13
100d1e478:     	ushl.2d	v15, v15, v29
100d1e47c:     	str	q0, [sp, #0xa0]
100d1e480:     	mov.16b	v7, v18
100d1e484:     	mov.16b	v18, v21
100d1e488:     	ldr	q0, [sp, #0xf0]
100d1e48c:     	ushl.2d	v21, v12, v0
100d1e490:     	bic.16b	v21, v21, v13
100d1e494:     	mov.16b	v1, v22
100d1e498:     	ldr	q22, [sp, #0x70]
100d1e49c:     	ushl.2d	v21, v21, v22
100d1e4a0:     	orr.16b	v21, v21, v15
100d1e4a4:     	ldr	q0, [sp, #0xe0]
100d1e4a8:     	ushl.2d	v15, v12, v0
100d1e4ac:     	ldr	q0, [sp, #0xd0]
100d1e4b0:     	ushl.2d	v22, v12, v0
100d1e4b4:     	mov.16b	v0, v8
100d1e4b8:     	ldp	q20, q8, [sp, #0xb0]
100d1e4bc:     	ushl.2d	v8, v12, v8
100d1e4c0:     	ushl.2d	v12, v12, v20
100d1e4c4:     	bic.16b	v15, v15, v13
100d1e4c8:     	bic.16b	v22, v22, v13
100d1e4cc:     	bic.16b	v8, v8, v13
100d1e4d0:     	bic.16b	v12, v12, v13
100d1e4d4:     	ldr	q13, [sp, #0x10]
100d1e4d8:     	ushl.2d	v13, v15, v13
100d1e4dc:     	orr.16b	v21, v21, v13
100d1e4e0:     	orr.16b	v23, v21, v23
100d1e4e4:     	ldr	q21, [sp]
100d1e4e8:     	ushl.2d	v21, v22, v21
100d1e4ec:     	mov.16b	v22, v1
100d1e4f0:     	orr.16b	v21, v14, v21
100d1e4f4:     	orr.16b	v26, v21, v26
100d1e4f8:     	ushl.2d	v21, v8, v30
100d1e4fc:     	mov.16b	v8, v0
100d1e500:     	ldp	q0, q1, [sp, #0x80]
100d1e504:     	orr.16b	v20, v0, v21
100d1e508:     	mov.16b	v21, v18
100d1e50c:     	mov.16b	v18, v7
100d1e510:     	ldr	q0, [sp, #0xa0]
100d1e514:     	orr.16b	v27, v20, v27
100d1e518:     	ushl.2d	v20, v12, v31
100d1e51c:     	orr.16b	v7, v1, v20
100d1e520:     	orr.16b	v28, v7, v28
100d1e524:     	ldr	q1, [sp, #0x1b0]
100d1e528:     	orr.16b	v7, v26, v23
100d1e52c:     	orr.16b	v20, v28, v27
100d1e530:     	orr.16b	v7, v20, v7
100d1e534:     	mov	d20, v7[1]
100d1e538:     	orr.8b	v7, v7, v20
100d1e53c:     	fmov	x16, d7
100d1e540:     	and	x17, x13, #0x38
100d1e544:     	cmp	x13, x14
100d1e548:     	b.eq	0x100d1e57c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9a4>
100d1e54c:     	lsl	x0, x17, #1
100d1e550:     	lsl	x1, x17, x8
100d1e554:     	add	x17, x17, #0x1
100d1e558:     	lsl	x2, x0, x8
100d1e55c:     	and	x2, x2, #0x3e
100d1e560:     	lsr	x2, x15, x2
100d1e564:     	bic	x2, x2, x10
100d1e568:     	lsl	x1, x2, x1
100d1e56c:     	orr	x16, x1, x16
100d1e570:     	add	x0, x0, #0x2
100d1e574:     	cmp	x13, x17
100d1e578:     	b.ne	0x100d1e550 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x978>
100d1e57c:     	lsr	x0, x9, #1
100d1e580:     	cmp	x0, x19
100d1e584:     	b.hs	0x100d1e65c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa84>
100d1e588:     	ubfiz	x15, x9, #5, #1
100d1e58c:     	add	x9, x9, #0x1
100d1e590:     	add	x22, x22, #0x8
100d1e594:     	ldr	x17, [x20, x0, lsl #3]
100d1e598:     	lsl	x15, x16, x15
100d1e59c:     	orr	x15, x17, x15
100d1e5a0:     	str	x15, [x20, x0, lsl #3]
100d1e5a4:     	cmp	x22, x11
100d1e5a8:     	b.ne	0x100d1e284 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6ac>
100d1e5ac:     	cmp	w28, #0x6
100d1e5b0:     	b.hs	0x100d1e5cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9f4>
100d1e5b4:     	mov	x8, #-0x1               ; =-1
100d1e5b8:     	ldr	x9, [sp, #0x208]
100d1e5bc:     	lsl	x8, x8, x9
100d1e5c0:     	ldr	x9, [x20]
100d1e5c4:     	bic	x8, x9, x8
100d1e5c8:     	str	x8, [x20]
100d1e5cc:     	stp	x19, x20, [x21]
100d1e5d0:     	str	x19, [x21, #0x10]
100d1e5d4:     	add	sp, sp, #0x280
100d1e5d8:     	ldp	x29, x30, [sp, #0x90]
100d1e5dc:     	ldp	x20, x19, [sp, #0x80]
100d1e5e0:     	ldp	x22, x21, [sp, #0x70]
100d1e5e4:     	ldp	x24, x23, [sp, #0x60]
100d1e5e8:     	ldp	x26, x25, [sp, #0x50]
100d1e5ec:     	ldp	x28, x27, [sp, #0x40]
100d1e5f0:     	ldp	d9, d8, [sp, #0x30]
100d1e5f4:     	ldp	d11, d10, [sp, #0x20]
100d1e5f8:     	ldp	d13, d12, [sp, #0x10]
100d1e5fc:     	ldp	d15, d14, [sp], #0xa0
100d1e600:     	ret
100d1e604:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e608:     	add	x2, x2, #0xb20
100d1e60c:     	mov	x1, x23
100d1e610:     	bl	0x1012887b8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d1e614:     	b	0x100d1e66c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1e618:     	adrp	x0, 0x10134f000 <dyld_stub_binder+0x10134f000>
100d1e61c:     	add	x0, x0, #0xfcd
100d1e620:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e624:     	add	x2, x2, #0xad8
100d1e628:     	mov	w1, #0x22               ; =34
100d1e62c:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d1e630:     	adrp	x3, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e634:     	add	x3, x3, #0xb38
100d1e638:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d1e63c:     	b	0x100d1e66c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1e640:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100d1e644:     	add	x0, x0, #0xc45
100d1e648:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e64c:     	add	x2, x2, #0xb08
100d1e650:     	mov	w1, #0x37               ; =55
100d1e654:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d1e658:     	b	0x100d1e66c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1e65c:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d1e660:     	add	x2, x2, #0xaf0
100d1e664:     	mov	x1, x19
100d1e668:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1e66c:     	brk	#0x1
100d1e670:     	mov	w0, #0x8                ; =8
100d1e674:     	mov	x1, x26
100d1e678:     	bl	0x101287c24 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d1e67c:     	cbz	x19, 0x100d1e690 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xab8>
100d1e680:     	mov	x19, x0
100d1e684:     	mov	x0, x20
100d1e688:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100d1e68c:     	mov	x0, x19
100d1e690:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
