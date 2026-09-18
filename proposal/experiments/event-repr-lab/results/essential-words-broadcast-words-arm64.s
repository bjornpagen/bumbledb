
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d1ab54 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>:
100d1ab54:     	sub	sp, sp, #0xc0
100d1ab58:     	stp	x28, x27, [sp, #0x60]
100d1ab5c:     	stp	x26, x25, [sp, #0x70]
100d1ab60:     	stp	x24, x23, [sp, #0x80]
100d1ab64:     	stp	x22, x21, [sp, #0x90]
100d1ab68:     	stp	x20, x19, [sp, #0xa0]
100d1ab6c:     	stp	x29, x30, [sp, #0xb0]
100d1ab70:     	add	x29, sp, #0xb0
100d1ab74:     	cmp	w3, #0xb
100d1ab78:     	b.hi	0x100d1aedc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x388>
100d1ab7c:     	mov	x24, x4
100d1ab80:     	mov	x22, x3
100d1ab84:     	cmp	w4, w3
100d1ab88:     	b.hi	0x100d1aedc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x388>
100d1ab8c:     	mov	x23, x2
100d1ab90:     	mov	w8, #0x1                ; =1
100d1ab94:     	lsl	x8, x8, x22
100d1ab98:     	lsr	x8, x8, #6
100d1ab9c:     	cmp	w22, #0x6
100d1aba0:     	cinc	x8, x8, lo
100d1aba4:     	stp	x2, x8, [sp, #0x50]
100d1aba8:     	cmp	x2, x8
100d1abac:     	b.ne	0x100d1aef4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3a0>
100d1abb0:     	mov	x28, x1
100d1abb4:     	mov	x21, x0
100d1abb8:     	add	w8, w22, #0x1
100d1abbc:     	mov	w9, #0x1                ; =1
100d1abc0:     	lsl	x26, x9, x8
100d1abc4:     	mov	w9, #0x3e               ; =62
100d1abc8:     	lsr	x8, x9, x8
100d1abcc:     	and	x8, x8, #0x1
100d1abd0:     	adds	x27, x8, x26, lsr #6
100d1abd4:     	b.eq	0x100d1ac24 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0xd0>
100d1abd8:     	lsl	x25, x27, #3
100d1abdc:     	mov	x0, x25
100d1abe0:     	mov	w1, #0x1                ; =1
100d1abe4:     	bl	0x10128a2e4 <dyld_stub_binder+0x10128a2e4>
100d1abe8:     	cbz	x0, 0x100d1af24 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3d0>
100d1abec:     	mov	x20, x0
100d1abf0:     	cmp	w24, #0x5
100d1abf4:     	b.ls	0x100d1ad40 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x1ec>
100d1abf8:     	sub	w12, w24, #0x6
100d1abfc:     	mov	w8, #0x2                ; =2
100d1ac00:     	lsl	x19, x8, x12
100d1ac04:     	add	x8, x12, #0x1
100d1ac08:     	lsr	x8, x27, x8
100d1ac0c:     	sub	x9, x19, #0x1
100d1ac10:     	tst	x27, x9
100d1ac14:     	cinc	x8, x8, ne
100d1ac18:     	str	x27, [sp, #0x20]
100d1ac1c:     	cbnz	x23, 0x100d1ac48 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0xf4>
100d1ac20:     	b	0x100d1ada0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1ac24:     	cmp	w24, #0x5
100d1ac28:     	b.ls	0x100d1adc8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x274>
100d1ac2c:     	str	xzr, [sp, #0x20]
100d1ac30:     	mov	x8, #0x0                ; =0
100d1ac34:     	sub	w12, w24, #0x6
100d1ac38:     	mov	w9, #0x2                ; =2
100d1ac3c:     	lsl	x19, x9, x12
100d1ac40:     	mov	w20, #0x8               ; =8
100d1ac44:     	cbz	x23, 0x100d1ada0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1ac48:     	mov	w9, #0x1                ; =1
100d1ac4c:     	lsl	x24, x9, x12
100d1ac50:     	lsr	x9, x23, x12
100d1ac54:     	mov	x10, #0xfffffffffffffff ; =1152921504606846975
100d1ac58:     	add	x10, x24, x10
100d1ac5c:     	tst	x10, x23
100d1ac60:     	cinc	x9, x9, ne
100d1ac64:     	cmp	x9, x8
100d1ac68:     	csel	x8, x9, x8, lo
100d1ac6c:     	str	x8, [sp, #0x48]
100d1ac70:     	cbz	x8, 0x100d1ada0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x24c>
100d1ac74:     	stp	x26, x22, [sp, #0x8]
100d1ac78:     	str	x23, [sp, #0x40]
100d1ac7c:     	str	x21, [sp, #0x18]
100d1ac80:     	mov	x21, #0x0               ; =0
100d1ac84:     	add	x8, x12, #0x1
100d1ac88:     	stp	x28, x8, [sp, #0x30]
100d1ac8c:     	mov	w8, #0x8                ; =8
100d1ac90:     	lsl	x8, x8, x12
100d1ac94:     	str	x8, [sp, #0x28]
100d1ac98:     	ldp	x8, x10, [sp, #0x38]
100d1ac9c:     	lsl	x8, x21, x8
100d1aca0:     	mov	x23, x27
100d1aca4:     	sub	x11, x27, x8
100d1aca8:     	cmp	x19, x11
100d1acac:     	csel	x28, x19, x11, lo
100d1acb0:     	lsl	x9, x21, x12
100d1acb4:     	sub	x10, x10, x9
100d1acb8:     	cmp	x24, x10
100d1acbc:     	csel	x27, x24, x10, lo
100d1acc0:     	cmp	x11, x24
100d1acc4:     	b.lo	0x100d1ae88 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x334>
100d1acc8:     	cmp	x24, x10
100d1accc:     	b.hi	0x100d1aea8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x354>
100d1acd0:     	mov	x26, x12
100d1acd4:     	mov	x22, x20
100d1acd8:     	add	x20, x20, x8, lsl #3
100d1acdc:     	ldp	x2, x8, [sp, #0x28]
100d1ace0:     	add	x25, x8, x9, lsl #3
100d1ace4:     	mov	x0, x20
100d1ace8:     	mov	x1, x25
100d1acec:     	bl	0x10128a4dc <dyld_stub_binder+0x10128a4dc>
100d1acf0:     	sub	x8, x28, x24
100d1acf4:     	cmp	x8, x27
100d1acf8:     	b.ne	0x100d1aeb8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x364>
100d1acfc:     	add	x21, x21, #0x1
100d1ad00:     	lsl	x2, x27, #3
100d1ad04:     	add	x0, x20, x24, lsl #3
100d1ad08:     	mov	x1, x25
100d1ad0c:     	bl	0x10128a4dc <dyld_stub_binder+0x10128a4dc>
100d1ad10:     	ldr	x8, [sp, #0x48]
100d1ad14:     	cmp	x8, x21
100d1ad18:     	mov	x20, x22
100d1ad1c:     	mov	x27, x23
100d1ad20:     	mov	x12, x26
100d1ad24:     	b.ne	0x100d1ac98 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x144>
100d1ad28:     	ldp	x22, x21, [sp, #0x10]
100d1ad2c:     	ldr	x26, [sp, #0x8]
100d1ad30:     	ldr	x19, [sp, #0x20]
100d1ad34:     	cmp	w22, #0x5
100d1ad38:     	b.lo	0x100d1adac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x258>
100d1ad3c:     	b	0x100d1add8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1ad40:     	mov	w8, #0x1                ; =1
100d1ad44:     	lsl	w8, w8, w24
100d1ad48:     	cmp	w24, #0x5
100d1ad4c:     	b.ne	0x100d1ae00 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2ac>
100d1ad50:     	mov	x9, #0x0                ; =0
100d1ad54:     	mov	x10, #0x0               ; =0
100d1ad58:     	lsr	x0, x10, #1
100d1ad5c:     	cmp	x0, x23
100d1ad60:     	b.hs	0x100d1af10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3bc>
100d1ad64:     	ldr	x11, [x28, x0, lsl #3]
100d1ad68:     	and	x12, x9, #0x20
100d1ad6c:     	lsr	x11, x11, x12
100d1ad70:     	mov	w11, w11
100d1ad74:     	lsl	x12, x11, x8
100d1ad78:     	orr	x11, x12, x11
100d1ad7c:     	str	x11, [x20, x10, lsl #3]
100d1ad80:     	add	x10, x10, #0x1
100d1ad84:     	add	x9, x9, #0x20
100d1ad88:     	subs	x25, x25, #0x8
100d1ad8c:     	b.ne	0x100d1ad58 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x204>
100d1ad90:     	mov	x19, x27
100d1ad94:     	cmp	w22, #0x5
100d1ad98:     	b.lo	0x100d1adac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x258>
100d1ad9c:     	b	0x100d1add8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1ada0:     	ldr	x19, [sp, #0x20]
100d1ada4:     	cmp	w22, #0x5
100d1ada8:     	b.hs	0x100d1add8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1adac:     	cbz	x27, 0x100d1af30 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3dc>
100d1adb0:     	mov	x8, #-0x1               ; =-1
100d1adb4:     	lsl	x8, x8, x26
100d1adb8:     	ldr	x9, [x20]
100d1adbc:     	bic	x8, x9, x8
100d1adc0:     	str	x8, [x20]
100d1adc4:     	b	0x100d1add8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x284>
100d1adc8:     	mov	x19, #0x0               ; =0
100d1adcc:     	mov	w20, #0x8               ; =8
100d1add0:     	cmp	w22, #0x5
100d1add4:     	b.lo	0x100d1af30 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3dc>
100d1add8:     	stp	x19, x20, [x21]
100d1addc:     	str	x27, [x21, #0x10]
100d1ade0:     	ldp	x29, x30, [sp, #0xb0]
100d1ade4:     	ldp	x20, x19, [sp, #0xa0]
100d1ade8:     	ldp	x22, x21, [sp, #0x90]
100d1adec:     	ldp	x24, x23, [sp, #0x80]
100d1adf0:     	ldp	x26, x25, [sp, #0x70]
100d1adf4:     	ldp	x28, x27, [sp, #0x60]
100d1adf8:     	add	sp, sp, #0xc0
100d1adfc:     	ret
100d1ae00:     	mov	x9, #0x0                ; =0
100d1ae04:     	mov	x10, #0x0               ; =0
100d1ae08:     	b	0x100d1ae28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2d4>
100d1ae0c:     	lsl	x12, x11, x8
100d1ae10:     	orr	x11, x12, x11
100d1ae14:     	str	x11, [x20, x10, lsl #3]
100d1ae18:     	add	x9, x9, #0x20
100d1ae1c:     	add	x10, x10, #0x1
100d1ae20:     	subs	x25, x25, #0x8
100d1ae24:     	b.eq	0x100d1ad90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x23c>
100d1ae28:     	lsr	x0, x10, #1
100d1ae2c:     	cmp	x0, x23
100d1ae30:     	b.hs	0x100d1af10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3bc>
100d1ae34:     	ldr	x11, [x28, x0, lsl #3]
100d1ae38:     	and	x12, x9, #0x20
100d1ae3c:     	lsr	x11, x11, x12
100d1ae40:     	bfi	x11, x11, #16, #48
100d1ae44:     	and	x11, x11, #0xffff0000ffff
100d1ae48:     	cmp	w24, #0x3
100d1ae4c:     	b.hi	0x100d1ae0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1ae50:     	orr	x11, x11, x11, lsl #8
100d1ae54:     	and	x11, x11, #0xff00ff00ff00ff
100d1ae58:     	cmp	w24, #0x3
100d1ae5c:     	b.eq	0x100d1ae0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1ae60:     	orr	x11, x11, x11, lsl #4
100d1ae64:     	and	x11, x11, #0xf0f0f0f0f0f0f0f
100d1ae68:     	cmp	w24, #0x1
100d1ae6c:     	b.hi	0x100d1ae0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1ae70:     	orr	x11, x11, x11, lsl #2
100d1ae74:     	and	x11, x11, #0x3333333333333333
100d1ae78:     	cbnz	w24, 0x100d1ae0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1ae7c:     	orr	x11, x11, x11, lsl #1
100d1ae80:     	and	x11, x11, #0x5555555555555555
100d1ae84:     	b	0x100d1ae0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x2b8>
100d1ae88:     	adrp	x3, 0x101508000 <dyld_stub_binder+0x101508000>
100d1ae8c:     	add	x3, x3, #0x858
100d1ae90:     	mov	x0, #0x0                ; =0
100d1ae94:     	mov	x1, x24
100d1ae98:     	mov	x2, x28
100d1ae9c:     	ldr	x19, [sp, #0x20]
100d1aea0:     	bl	0x101281f14 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d1aea4:     	b	0x100d1af44 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1aea8:     	ldr	x19, [sp, #0x20]
100d1aeac:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aeb0:     	add	x2, x2, #0x828
100d1aeb4:     	b	0x100d1aecc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x378>
100d1aeb8:     	mov	x24, x8
100d1aebc:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aec0:     	add	x2, x2, #0x840
100d1aec4:     	mov	x20, x22
100d1aec8:     	ldr	x19, [sp, #0x20]
100d1aecc:     	mov	x0, x24
100d1aed0:     	mov	x1, x27
100d1aed4:     	bl	0x101282378 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d1aed8:     	b	0x100d1af44 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1aedc:     	adrp	x0, 0x101349000 <dyld_stub_binder+0x101349000>
100d1aee0:     	add	x0, x0, #0x94f
100d1aee4:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aee8:     	add	x2, x2, #0x7c8
100d1aeec:     	mov	w1, #0x31               ; =49
100d1aef0:     	bl	0x101281fc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d1aef4:     	adrp	x5, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aef8:     	add	x5, x5, #0x7e0
100d1aefc:     	add	x1, sp, #0x50
100d1af00:     	add	x2, sp, #0x58
100d1af04:     	mov	w0, #0x0                ; =0
100d1af08:     	mov	x3, #0x0                ; =0
100d1af0c:     	bl	0x101281eb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d1af10:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1af14:     	add	x2, x2, #0x7f8
100d1af18:     	mov	x1, x23
100d1af1c:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1af20:     	b	0x100d1af44 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3f0>
100d1af24:     	mov	w0, #0x8                ; =8
100d1af28:     	mov	x1, x25
100d1af2c:     	bl	0x1012817e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d1af30:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1af34:     	add	x2, x2, #0x810
100d1af38:     	mov	x0, #0x0                ; =0
100d1af3c:     	mov	x1, #0x0                ; =0
100d1af40:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1af44:     	brk	#0x1
100d1af48:     	b	0x100d1af50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x3fc>
100d1af4c:     	cbz	x19, 0x100d1af60 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast+0x40c>
100d1af50:     	mov	x19, x0
100d1af54:     	mov	x0, x20
100d1af58:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100d1af5c:     	mov	x0, x19
100d1af60:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
