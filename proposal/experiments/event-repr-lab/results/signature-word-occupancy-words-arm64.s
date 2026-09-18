
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d01624 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy>:
100d01624:     	sub	sp, sp, #0x30
100d01628:     	stp	x29, x30, [sp, #0x20]
100d0162c:     	add	x29, sp, #0x20
100d01630:     	str	x1, [sp, #0x8]
100d01634:     	stur	x3, [x29, #-0x8]
100d01638:     	cmp	x1, x3
100d0163c:     	b.ne	0x100d016c8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xa4>
100d01640:     	str	x1, [sp, #0x10]
100d01644:     	stur	x5, [x29, #-0x8]
100d01648:     	cmp	x1, x5
100d0164c:     	b.ne	0x100d016e4 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xc0>
100d01650:     	mov	w8, #0x0                ; =0
100d01654:     	mov	w9, #0x2                ; =2
100d01658:     	mov	w10, #0x4               ; =4
100d0165c:     	mov	w11, #0x8               ; =8
100d01660:     	cbz	x1, 0x100d016b8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x94>
100d01664:     	ldr	x12, [x0], #0x8
100d01668:     	ldr	x13, [x2], #0x8
100d0166c:     	ldr	x14, [x4], #0x8
100d01670:     	bic	x15, x12, x13
100d01674:     	tst	x15, x14
100d01678:     	csel	w16, wzr, w9, eq
100d0167c:     	and	x12, x13, x12
100d01680:     	bics	xzr, x12, x14
100d01684:     	csel	w13, wzr, w10, eq
100d01688:     	tst	x12, x14
100d0168c:     	csel	w12, wzr, w11, eq
100d01690:     	bics	xzr, x15, x14
100d01694:     	cinc	w14, w16, ne
100d01698:     	orr	w12, w13, w12
100d0169c:     	orr	w12, w14, w12
100d016a0:     	orr	w8, w12, w8
100d016a4:     	and	w12, w8, #0xff
100d016a8:     	sub	x1, x1, #0x1
100d016ac:     	cmp	w12, #0xf
100d016b0:     	b.ne	0x100d01660 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x3c>
100d016b4:     	mov	w8, #0xf                ; =15
100d016b8:     	mov	x0, x8
100d016bc:     	ldp	x29, x30, [sp, #0x20]
100d016c0:     	add	sp, sp, #0x30
100d016c4:     	ret
100d016c8:     	adrp	x5, 0x101505000 <dyld_stub_binder+0x101505000>
100d016cc:     	add	x5, x5, #0x740
100d016d0:     	add	x1, sp, #0x8
100d016d4:     	sub	x2, x29, #0x8
100d016d8:     	mov	w0, #0x0                ; =0
100d016dc:     	mov	x3, #0x0                ; =0
100d016e0:     	bl	0x101281eb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d016e4:     	adrp	x5, 0x101505000 <dyld_stub_binder+0x101505000>
100d016e8:     	add	x5, x5, #0x758
100d016ec:     	add	x1, sp, #0x10
100d016f0:     	sub	x2, x29, #0x8
100d016f4:     	mov	w0, #0x0                ; =0
100d016f8:     	mov	x3, #0x0                ; =0
100d016fc:     	bl	0x101281eb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
