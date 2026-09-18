
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba1910 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>:
100ba1910:     	stp	x24, x23, [sp, #-0x40]!
100ba1914:     	stp	x22, x21, [sp, #0x10]
100ba1918:     	stp	x20, x19, [sp, #0x20]
100ba191c:     	stp	x29, x30, [sp, #0x30]
100ba1920:     	add	x29, sp, #0x30
100ba1924:     	cmp	w2, #0x6
100ba1928:     	b.hs	0x100ba1964 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x54>
100ba192c:     	mov	w8, #0x1                ; =1
100ba1930:     	lsl	w8, w8, w2
100ba1934:     	lsl	x9, x1, #3
100ba1938:     	adrp	x10, 0x10119f000 <dyld_stub_binder+0x10119f000>
100ba193c:     	add	x10, x10, #0xdd0
100ba1940:     	cbz	x9, 0x100ba19ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100ba1944:     	ldr	x11, [x0], #0x8
100ba1948:     	lsr	x12, x11, x8
100ba194c:     	eor	x11, x12, x11
100ba1950:     	ldr	x12, [x10, w2, uxtw #3]
100ba1954:     	sub	x9, x9, #0x8
100ba1958:     	and	x11, x11, x12
100ba195c:     	cbz	x11, 0x100ba1940 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x30>
100ba1960:     	b	0x100ba19d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc0>
100ba1964:     	add	w9, w2, #0x3a
100ba1968:     	and	w8, w9, #0x3f
100ba196c:     	cmp	w8, #0x3f
100ba1970:     	b.eq	0x100ba1a04 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xf4>
100ba1974:     	mov	w10, #0x2               ; =2
100ba1978:     	lsl	x19, x10, x9
100ba197c:     	mov	w9, #0x1                ; =1
100ba1980:     	lsl	x8, x9, x8
100ba1984:     	neg	x9, x19
100ba1988:     	and	x9, x1, x9
100ba198c:     	cmp	x19, x8
100ba1990:     	b.lo	0x100ba19e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xd4>
100ba1994:     	cmp	x19, x8, lsl #1
100ba1998:     	b.ne	0x100ba19d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc8>
100ba199c:     	lsl	x20, x8, #3
100ba19a0:     	lsl	x21, x19, #3
100ba19a4:     	add	x22, x9, x19
100ba19a8:     	sub	x22, x22, x19
100ba19ac:     	cmp	x19, x22
100ba19b0:     	b.hi	0x100ba19ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100ba19b4:     	add	x23, x0, x21
100ba19b8:     	add	x1, x0, x20
100ba19bc:     	mov	x2, x20
100ba19c0:     	bl	0x101109b90 <dyld_stub_binder+0x101109b90>
100ba19c4:     	mov	x8, x0
100ba19c8:     	mov	x0, x23
100ba19cc:     	cbz	w8, 0x100ba19a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x98>
100ba19d0:     	mov	w0, #0x0                ; =0
100ba19d4:     	b	0x100ba19f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100ba19d8:     	cmp	x19, x9
100ba19dc:     	cset	w0, hi
100ba19e0:     	b	0x100ba19f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100ba19e4:     	cmp	x19, x9
100ba19e8:     	b.ls	0x100ba1a1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x10c>
100ba19ec:     	mov	w0, #0x1                ; =1
100ba19f0:     	ldp	x29, x30, [sp, #0x30]
100ba19f4:     	ldp	x20, x19, [sp, #0x20]
100ba19f8:     	ldp	x22, x21, [sp, #0x10]
100ba19fc:     	ldp	x24, x23, [sp], #0x40
100ba1a00:     	ret
100ba1a04:     	adrp	x0, 0x10119e000 <dyld_stub_binder+0x10119e000>
100ba1a08:     	add	x0, x0, #0x791
100ba1a0c:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba1a10:     	add	x2, x2, #0x790
100ba1a14:     	mov	w1, #0x37               ; =55
100ba1a18:     	bl	0x101101574 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba1a1c:     	adrp	x3, 0x101336000 <dyld_stub_binder+0x101336000>
100ba1a20:     	add	x3, x3, #0xb38
100ba1a24:     	mov	x0, #0x0                ; =0
100ba1a28:     	mov	x1, x8
100ba1a2c:     	mov	x2, x19
100ba1a30:     	bl	0x101101614 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
