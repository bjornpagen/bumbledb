
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d05164 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy>:
100d05164:     	sub	sp, sp, #0x30
100d05168:     	stp	x29, x30, [sp, #0x20]
100d0516c:     	add	x29, sp, #0x20
100d05170:     	str	x1, [sp, #0x8]
100d05174:     	stur	x3, [x29, #-0x8]
100d05178:     	cmp	x1, x3
100d0517c:     	b.ne	0x100d05208 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xa4>
100d05180:     	str	x1, [sp, #0x10]
100d05184:     	stur	x5, [x29, #-0x8]
100d05188:     	cmp	x1, x5
100d0518c:     	b.ne	0x100d05224 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0xc0>
100d05190:     	mov	w8, #0x0                ; =0
100d05194:     	mov	w9, #0x2                ; =2
100d05198:     	mov	w10, #0x4               ; =4
100d0519c:     	mov	w11, #0x8               ; =8
100d051a0:     	cbz	x1, 0x100d051f8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x94>
100d051a4:     	ldr	x12, [x0], #0x8
100d051a8:     	ldr	x13, [x2], #0x8
100d051ac:     	ldr	x14, [x4], #0x8
100d051b0:     	bic	x15, x12, x13
100d051b4:     	tst	x15, x14
100d051b8:     	csel	w16, wzr, w9, eq
100d051bc:     	and	x12, x13, x12
100d051c0:     	bics	xzr, x12, x14
100d051c4:     	csel	w13, wzr, w10, eq
100d051c8:     	tst	x12, x14
100d051cc:     	csel	w12, wzr, w11, eq
100d051d0:     	bics	xzr, x15, x14
100d051d4:     	cinc	w14, w16, ne
100d051d8:     	orr	w12, w13, w12
100d051dc:     	orr	w12, w14, w12
100d051e0:     	orr	w8, w12, w8
100d051e4:     	and	w12, w8, #0xff
100d051e8:     	sub	x1, x1, #0x1
100d051ec:     	cmp	w12, #0xf
100d051f0:     	b.ne	0x100d051a0 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9signature14word_occupancy+0x3c>
100d051f4:     	mov	w8, #0xf                ; =15
100d051f8:     	mov	x0, x8
100d051fc:     	ldp	x29, x30, [sp, #0x20]
100d05200:     	add	sp, sp, #0x30
100d05204:     	ret
100d05208:     	adrp	x5, 0x10150d000 <dyld_stub_binder+0x10150d000>
100d0520c:     	add	x5, x5, #0xac8
100d05210:     	add	x1, sp, #0x8
100d05214:     	sub	x2, x29, #0x8
100d05218:     	mov	w0, #0x0                ; =0
100d0521c:     	mov	x3, #0x0                ; =0
100d05220:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d05224:     	adrp	x5, 0x10150d000 <dyld_stub_binder+0x10150d000>
100d05228:     	add	x5, x5, #0xae0
100d0522c:     	add	x1, sp, #0x10
100d05230:     	sub	x2, x29, #0x8
100d05234:     	mov	w0, #0x0                ; =0
100d05238:     	mov	x3, #0x0                ; =0
100d0523c:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
