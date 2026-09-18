
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ceb264 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy>:
100ceb264:     	sub	sp, sp, #0x30
100ceb268:     	stp	x29, x30, [sp, #0x20]
100ceb26c:     	add	x29, sp, #0x20
100ceb270:     	str	x1, [sp, #0x8]
100ceb274:     	stur	x3, [x29, #-0x8]
100ceb278:     	cmp	x1, x3
100ceb27c:     	b.ne	0x100ceb308 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xa4>
100ceb280:     	str	x1, [sp, #0x10]
100ceb284:     	stur	x5, [x29, #-0x8]
100ceb288:     	cmp	x1, x5
100ceb28c:     	b.ne	0x100ceb324 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xc0>
100ceb290:     	mov	w8, #0x0                ; =0
100ceb294:     	mov	w9, #0x2                ; =2
100ceb298:     	mov	w10, #0x4               ; =4
100ceb29c:     	mov	w11, #0x8               ; =8
100ceb2a0:     	cbz	x1, 0x100ceb2f8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x94>
100ceb2a4:     	ldr	x12, [x0], #0x8
100ceb2a8:     	ldr	x13, [x2], #0x8
100ceb2ac:     	ldr	x14, [x4], #0x8
100ceb2b0:     	bic	x15, x12, x13
100ceb2b4:     	tst	x15, x14
100ceb2b8:     	csel	w16, wzr, w9, eq
100ceb2bc:     	and	x12, x13, x12
100ceb2c0:     	bics	xzr, x12, x14
100ceb2c4:     	csel	w13, wzr, w10, eq
100ceb2c8:     	tst	x12, x14
100ceb2cc:     	csel	w12, wzr, w11, eq
100ceb2d0:     	bics	xzr, x15, x14
100ceb2d4:     	cinc	w14, w16, ne
100ceb2d8:     	orr	w12, w13, w12
100ceb2dc:     	orr	w12, w14, w12
100ceb2e0:     	orr	w8, w12, w8
100ceb2e4:     	and	w12, w8, #0xff
100ceb2e8:     	sub	x1, x1, #0x1
100ceb2ec:     	cmp	w12, #0xf
100ceb2f0:     	b.ne	0x100ceb2a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x3c>
100ceb2f4:     	mov	w8, #0xf                ; =15
100ceb2f8:     	mov	x0, x8
100ceb2fc:     	ldp	x29, x30, [sp, #0x20]
100ceb300:     	add	sp, sp, #0x30
100ceb304:     	ret
100ceb308:     	adrp	x5, 0x1014e5000 <dyld_stub_binder+0x1014e5000>
100ceb30c:     	add	x5, x5, #0x108
100ceb310:     	add	x1, sp, #0x8
100ceb314:     	sub	x2, x29, #0x8
100ceb318:     	mov	w0, #0x0                ; =0
100ceb31c:     	mov	x3, #0x0                ; =0
100ceb320:     	bl	0x1012683b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100ceb324:     	adrp	x5, 0x1014e5000 <dyld_stub_binder+0x1014e5000>
100ceb328:     	add	x5, x5, #0x120
100ceb32c:     	add	x1, sp, #0x10
100ceb330:     	sub	x2, x29, #0x8
100ceb334:     	mov	w0, #0x0                ; =0
100ceb338:     	mov	x3, #0x0                ; =0
100ceb33c:     	bl	0x1012683b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
