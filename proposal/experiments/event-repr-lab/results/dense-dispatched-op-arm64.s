
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100653330 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_>:
100653330:     	sub	sp, sp, #0x60
100653334:     	stp	x24, x23, [sp, #0x20]
100653338:     	stp	x22, x21, [sp, #0x30]
10065333c:     	stp	x20, x19, [sp, #0x40]
100653340:     	stp	x29, x30, [sp, #0x50]
100653344:     	add	x29, sp, #0x50
100653348:     	mov	x20, x3
10065334c:     	mov	x21, x2
100653350:     	mov	x22, x1
100653354:     	mov	x19, x0
100653358:     	mov	x0, x1
10065335c:     	mov	x1, x2
100653360:     	mov	x2, x3
100653364:     	bl	0x1004f62e4 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
100653368:     	tbnz	w0, #0x0, 0x10065386c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x53c>
10065336c:     	and	w8, w20, #0x1
100653370:     	bfi	w8, w21, #1, #1
100653374:     	and	w9, w22, #0xff
100653378:     	lsr	w10, w9, w8
10065337c:     	ands	w22, w10, #0x1
100653380:     	eor	w10, w8, #0x1
100653384:     	lsr	w10, w9, w10
100653388:     	eor	w11, w8, #0x2
10065338c:     	lsr	w11, w9, w11
100653390:     	ubfiz	w11, w11, #2, #1
100653394:     	bfi	w11, w10, #1, #1
100653398:     	eor	w8, w8, #0x3
10065339c:     	lsr	w8, w9, w8
1006533a0:     	bfi	w11, w8, #3, #1
1006533a4:     	orr	w8, w11, w22
1006533a8:     	eor	w8, w8, #0xf
1006533ac:     	csel	w10, w11, w8, eq
1006533b0:     	cbz	w10, 0x1006533dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0xac>
1006533b4:     	cmp	w10, #0xa
1006533b8:     	b.eq	0x1006533cc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x9c>
1006533bc:     	cmp	w10, #0xc
1006533c0:     	b.ne	0x1006533e4 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0xb4>
1006533c4:     	and	x8, x21, #0xfffffffffffffffe
1006533c8:     	b	0x1006533d0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0xa0>
1006533cc:     	and	x8, x20, #0xfffffffffffffffe
1006533d0:     	and	x9, x22, #0xff
1006533d4:     	orr	x1, x8, x9
1006533d8:     	b	0x10065386c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x53c>
1006533dc:     	and	x1, x22, #0xff
1006533e0:     	b	0x10065386c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x53c>
1006533e4:     	lsr	x0, x21, #1
1006533e8:     	ldr	x1, [x19, #0x40]
1006533ec:     	cmp	x0, x1
1006533f0:     	b.hs	0x1006537f0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4c0>
1006533f4:     	lsr	x8, x20, #1
1006533f8:     	cmp	x8, x1
1006533fc:     	b.hs	0x1006537fc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4cc>
100653400:     	ldr	x9, [x19, #0x38]
100653404:     	mov	w11, #0x18              ; =24
100653408:     	madd	x12, x0, x11, x9
10065340c:     	madd	x9, x8, x11, x9
100653410:     	ldp	x23, x8, [x12, #0x8]
100653414:     	ldp	x24, x9, [x9, #0x8]
100653418:     	and	w10, w10, #0xff
10065341c:     	cmp	w10, #0x7
100653420:     	b.gt	0x100653474 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x144>
100653424:     	cmp	w10, #0x2
100653428:     	b.eq	0x100653568 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x238>
10065342c:     	cmp	w10, #0x4
100653430:     	b.eq	0x1006534c4 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x194>
100653434:     	cmp	w10, #0x6
100653438:     	b.ne	0x1006537d8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4a8>
10065343c:     	cmp	x9, x8
100653440:     	csel	x21, x9, x8, lo
100653444:     	lsr	x8, x21, #60
100653448:     	cbnz	x8, 0x1006535b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x280>
10065344c:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
100653450:     	lsl	x20, x21, #3
100653454:     	mov	x0, x20
100653458:     	mov	w1, #0x8                ; =8
10065345c:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
100653460:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
100653464:     	cmp	x21, #0x8
100653468:     	b.hs	0x100653648 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x318>
10065346c:     	mov	x8, #0x0                ; =0
100653470:     	b	0x1006538b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x580>
100653474:     	cmp	w10, #0x8
100653478:     	b.eq	0x1006535a0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x270>
10065347c:     	cmp	w10, #0xc
100653480:     	b.eq	0x1006534fc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x1cc>
100653484:     	cmp	w10, #0xe
100653488:     	b.ne	0x1006537d8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4a8>
10065348c:     	cmp	x9, x8
100653490:     	csel	x21, x9, x8, lo
100653494:     	lsr	x8, x21, #60
100653498:     	cbnz	x8, 0x1006535b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x280>
10065349c:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
1006534a0:     	lsl	x20, x21, #3
1006534a4:     	mov	x0, x20
1006534a8:     	mov	w1, #0x8                ; =8
1006534ac:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1006534b0:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
1006534b4:     	cmp	x21, #0x8
1006534b8:     	b.hs	0x1006536ac <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x37c>
1006534bc:     	mov	x8, #0x0                ; =0
1006534c0:     	b	0x100653818 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4e8>
1006534c4:     	cmp	x9, x8
1006534c8:     	csel	x21, x9, x8, lo
1006534cc:     	lsr	x8, x21, #60
1006534d0:     	cbnz	x8, 0x1006535b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x280>
1006534d4:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
1006534d8:     	lsl	x20, x21, #3
1006534dc:     	mov	x0, x20
1006534e0:     	mov	w1, #0x8                ; =8
1006534e4:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1006534e8:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
1006534ec:     	cmp	x21, #0x8
1006534f0:     	b.hs	0x1006535e4 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2b4>
1006534f4:     	mov	x8, #0x0                ; =0
1006534f8:     	b	0x100653890 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x560>
1006534fc:     	cmp	x9, x8
100653500:     	csel	x21, x9, x8, lo
100653504:     	lsr	x8, x21, #60
100653508:     	cbnz	x8, 0x1006535b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x280>
10065350c:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
100653510:     	lsl	x20, x21, #3
100653514:     	mov	x0, x20
100653518:     	mov	w1, #0x8                ; =8
10065351c:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
100653520:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
100653524:     	mov	x8, #0x0                ; =0
100653528:     	cmp	x21, #0x8
10065352c:     	b.lo	0x100653840 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x510>
100653530:     	sub	x9, x23, x0
100653534:     	cmn	x9, #0x40
100653538:     	b.hi	0x100653840 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x510>
10065353c:     	and	x8, x21, #0xffffffffffffff8
100653540:     	add	x9, x23, #0x20
100653544:     	add	x10, x0, #0x20
100653548:     	and	x11, x21, #0xffffffffffffff8
10065354c:     	ldp	q0, q1, [x9, #-0x20]
100653550:     	ldp	q2, q3, [x9], #0x40
100653554:     	stp	q0, q1, [x10, #-0x20]
100653558:     	stp	q2, q3, [x10], #0x40
10065355c:     	subs	x11, x11, #0x8
100653560:     	b.ne	0x10065354c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x21c>
100653564:     	b	0x100653838 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x508>
100653568:     	cmp	x9, x8
10065356c:     	csel	x21, x9, x8, lo
100653570:     	lsr	x8, x21, #60
100653574:     	cbnz	x8, 0x1006535b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x280>
100653578:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
10065357c:     	lsl	x20, x21, #3
100653580:     	mov	x0, x20
100653584:     	mov	w1, #0x8                ; =8
100653588:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
10065358c:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
100653590:     	cmp	x21, #0x8
100653594:     	b.hs	0x100653710 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x3e0>
100653598:     	mov	x8, #0x0                ; =0
10065359c:     	b	0x1006538d0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5a0>
1006535a0:     	cmp	x9, x8
1006535a4:     	csel	x21, x9, x8, lo
1006535a8:     	lsr	x8, x21, #60
1006535ac:     	cbz	x8, 0x1006535b4 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x284>
1006535b0:     	bl	0x1009ec010 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1006535b4:     	cbz	x21, 0x1006535dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2ac>
1006535b8:     	lsl	x20, x21, #3
1006535bc:     	mov	x0, x20
1006535c0:     	mov	w1, #0x8                ; =8
1006535c4:     	bl	0x100942910 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1006535c8:     	cbz	x0, 0x10065380c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4dc>
1006535cc:     	cmp	x21, #0x8
1006535d0:     	b.hs	0x100653774 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x444>
1006535d4:     	mov	x8, #0x0                ; =0
1006535d8:     	b	0x1006538f0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5c0>
1006535dc:     	mov	w0, #0x8                ; =8
1006535e0:     	b	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
1006535e4:     	mov	x8, #0x0                ; =0
1006535e8:     	sub	x9, x23, x0
1006535ec:     	cmn	x9, #0x40
1006535f0:     	b.hi	0x100653890 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x560>
1006535f4:     	sub	x9, x24, x0
1006535f8:     	cmn	x9, #0x40
1006535fc:     	b.hi	0x100653890 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x560>
100653600:     	and	x8, x21, #0xffffffffffffff8
100653604:     	add	x9, x23, #0x20
100653608:     	add	x10, x24, #0x20
10065360c:     	add	x11, x0, #0x20
100653610:     	and	x12, x21, #0xffffffffffffff8
100653614:     	ldp	q0, q1, [x9, #-0x20]
100653618:     	ldp	q2, q3, [x9], #0x40
10065361c:     	ldp	q4, q5, [x10, #-0x20]
100653620:     	ldp	q6, q7, [x10], #0x40
100653624:     	bic.16b	v0, v0, v4
100653628:     	bic.16b	v1, v1, v5
10065362c:     	bic.16b	v2, v2, v6
100653630:     	bic.16b	v3, v3, v7
100653634:     	stp	q0, q1, [x11, #-0x20]
100653638:     	stp	q2, q3, [x11], #0x40
10065363c:     	subs	x12, x12, #0x8
100653640:     	b.ne	0x100653614 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x2e4>
100653644:     	b	0x100653888 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x558>
100653648:     	mov	x8, #0x0                ; =0
10065364c:     	sub	x9, x23, x0
100653650:     	cmn	x9, #0x40
100653654:     	b.hi	0x1006538b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x580>
100653658:     	sub	x9, x24, x0
10065365c:     	cmn	x9, #0x40
100653660:     	b.hi	0x1006538b0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x580>
100653664:     	and	x8, x21, #0xffffffffffffff8
100653668:     	add	x9, x23, #0x20
10065366c:     	add	x10, x24, #0x20
100653670:     	add	x11, x0, #0x20
100653674:     	and	x12, x21, #0xffffffffffffff8
100653678:     	ldp	q0, q1, [x9, #-0x20]
10065367c:     	ldp	q2, q3, [x9], #0x40
100653680:     	ldp	q4, q5, [x10, #-0x20]
100653684:     	ldp	q6, q7, [x10], #0x40
100653688:     	eor.16b	v0, v4, v0
10065368c:     	eor.16b	v1, v5, v1
100653690:     	eor.16b	v2, v6, v2
100653694:     	eor.16b	v3, v7, v3
100653698:     	stp	q0, q1, [x11, #-0x20]
10065369c:     	stp	q2, q3, [x11], #0x40
1006536a0:     	subs	x12, x12, #0x8
1006536a4:     	b.ne	0x100653678 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x348>
1006536a8:     	b	0x1006538a8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x578>
1006536ac:     	mov	x8, #0x0                ; =0
1006536b0:     	sub	x9, x23, x0
1006536b4:     	cmn	x9, #0x40
1006536b8:     	b.hi	0x100653818 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4e8>
1006536bc:     	sub	x9, x24, x0
1006536c0:     	cmn	x9, #0x40
1006536c4:     	b.hi	0x100653818 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4e8>
1006536c8:     	and	x8, x21, #0xffffffffffffff8
1006536cc:     	add	x9, x23, #0x20
1006536d0:     	add	x10, x24, #0x20
1006536d4:     	add	x11, x0, #0x20
1006536d8:     	and	x12, x21, #0xffffffffffffff8
1006536dc:     	ldp	q0, q1, [x9, #-0x20]
1006536e0:     	ldp	q2, q3, [x9], #0x40
1006536e4:     	ldp	q4, q5, [x10, #-0x20]
1006536e8:     	ldp	q6, q7, [x10], #0x40
1006536ec:     	orr.16b	v0, v4, v0
1006536f0:     	orr.16b	v1, v5, v1
1006536f4:     	orr.16b	v2, v6, v2
1006536f8:     	orr.16b	v3, v7, v3
1006536fc:     	stp	q0, q1, [x11, #-0x20]
100653700:     	stp	q2, q3, [x11], #0x40
100653704:     	subs	x12, x12, #0x8
100653708:     	b.ne	0x1006536dc <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x3ac>
10065370c:     	b	0x10065382c <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4fc>
100653710:     	mov	x8, #0x0                ; =0
100653714:     	sub	x9, x23, x0
100653718:     	cmn	x9, #0x40
10065371c:     	b.hi	0x1006538d0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5a0>
100653720:     	sub	x9, x24, x0
100653724:     	cmn	x9, #0x40
100653728:     	b.hi	0x1006538d0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5a0>
10065372c:     	and	x8, x21, #0xffffffffffffff8
100653730:     	add	x9, x23, #0x20
100653734:     	add	x10, x24, #0x20
100653738:     	add	x11, x0, #0x20
10065373c:     	and	x12, x21, #0xffffffffffffff8
100653740:     	ldp	q0, q1, [x9, #-0x20]
100653744:     	ldp	q2, q3, [x9], #0x40
100653748:     	ldp	q4, q5, [x10, #-0x20]
10065374c:     	ldp	q6, q7, [x10], #0x40
100653750:     	bic.16b	v0, v4, v0
100653754:     	bic.16b	v1, v5, v1
100653758:     	bic.16b	v2, v6, v2
10065375c:     	bic.16b	v3, v7, v3
100653760:     	stp	q0, q1, [x11, #-0x20]
100653764:     	stp	q2, q3, [x11], #0x40
100653768:     	subs	x12, x12, #0x8
10065376c:     	b.ne	0x100653740 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x410>
100653770:     	b	0x1006538c8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x598>
100653774:     	mov	x8, #0x0                ; =0
100653778:     	sub	x9, x23, x0
10065377c:     	cmn	x9, #0x40
100653780:     	b.hi	0x1006538f0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5c0>
100653784:     	sub	x9, x24, x0
100653788:     	cmn	x9, #0x40
10065378c:     	b.hi	0x1006538f0 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5c0>
100653790:     	and	x8, x21, #0xffffffffffffff8
100653794:     	add	x9, x23, #0x20
100653798:     	add	x10, x24, #0x20
10065379c:     	add	x11, x0, #0x20
1006537a0:     	and	x12, x21, #0xffffffffffffff8
1006537a4:     	ldp	q0, q1, [x9, #-0x20]
1006537a8:     	ldp	q2, q3, [x9], #0x40
1006537ac:     	ldp	q4, q5, [x10, #-0x20]
1006537b0:     	ldp	q6, q7, [x10], #0x40
1006537b4:     	and.16b	v0, v4, v0
1006537b8:     	and.16b	v1, v5, v1
1006537bc:     	and.16b	v2, v6, v2
1006537c0:     	and.16b	v3, v7, v3
1006537c4:     	stp	q0, q1, [x11, #-0x20]
1006537c8:     	stp	q2, q3, [x11], #0x40
1006537cc:     	subs	x12, x12, #0x8
1006537d0:     	b.ne	0x1006537a4 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x474>
1006537d4:     	b	0x1006538e8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5b8>
1006537d8:     	adrp	x0, 0x100b12000 <dyld_stub_binder+0x100b12000>
1006537dc:     	add	x0, x0, #0x122
1006537e0:     	adrp	x2, 0x100c32000 <dyld_stub_binder+0x100c32000>
1006537e4:     	add	x2, x2, #0xbd8
1006537e8:     	mov	w1, #0xa7               ; =167
1006537ec:     	bl	0x1009ec674 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1006537f0:     	adrp	x2, 0x100c33000 <dyld_stub_binder+0x100c33000>
1006537f4:     	add	x2, x2, #0xe40
1006537f8:     	bl	0x1009ec7dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006537fc:     	adrp	x2, 0x100c33000 <dyld_stub_binder+0x100c33000>
100653800:     	add	x2, x2, #0xe58
100653804:     	mov	x0, x8
100653808:     	bl	0x1009ec7dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10065380c:     	mov	w0, #0x8                ; =8
100653810:     	mov	x1, x20
100653814:     	bl	0x1009ebfe4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100653818:     	ldr	x9, [x23, x8, lsl #3]
10065381c:     	ldr	x10, [x24, x8, lsl #3]
100653820:     	orr	x9, x10, x9
100653824:     	str	x9, [x0, x8, lsl #3]
100653828:     	add	x8, x8, #0x1
10065382c:     	cmp	x21, x8
100653830:     	b.ne	0x100653818 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x4e8>
100653834:     	b	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
100653838:     	cmp	x21, x8
10065383c:     	b.eq	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
100653840:     	ldr	x9, [x23, x8, lsl #3]
100653844:     	str	x9, [x0, x8, lsl #3]
100653848:     	add	x8, x8, #0x1
10065384c:     	b	0x100653838 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x508>
100653850:     	stp	x21, x0, [sp, #0x8]
100653854:     	str	x21, [sp, #0x18]
100653858:     	add	x1, sp, #0x8
10065385c:     	mov	x0, x19
100653860:     	bl	0x1004934d4 <__RNvMs2_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EE6internB9_>
100653864:     	and	x8, x22, #0xff
100653868:     	eor	x1, x0, x8
10065386c:     	mov	x0, x1
100653870:     	ldp	x29, x30, [sp, #0x50]
100653874:     	ldp	x20, x19, [sp, #0x40]
100653878:     	ldp	x22, x21, [sp, #0x30]
10065387c:     	ldp	x24, x23, [sp, #0x20]
100653880:     	add	sp, sp, #0x60
100653884:     	ret
100653888:     	cmp	x21, x8
10065388c:     	b.eq	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
100653890:     	ldr	x9, [x23, x8, lsl #3]
100653894:     	ldr	x10, [x24, x8, lsl #3]
100653898:     	bic	x9, x9, x10
10065389c:     	str	x9, [x0, x8, lsl #3]
1006538a0:     	add	x8, x8, #0x1
1006538a4:     	b	0x100653888 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x558>
1006538a8:     	cmp	x21, x8
1006538ac:     	b.eq	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
1006538b0:     	ldr	x9, [x23, x8, lsl #3]
1006538b4:     	ldr	x10, [x24, x8, lsl #3]
1006538b8:     	eor	x9, x10, x9
1006538bc:     	str	x9, [x0, x8, lsl #3]
1006538c0:     	add	x8, x8, #0x1
1006538c4:     	b	0x1006538a8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x578>
1006538c8:     	cmp	x21, x8
1006538cc:     	b.eq	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
1006538d0:     	ldr	x9, [x23, x8, lsl #3]
1006538d4:     	ldr	x10, [x24, x8, lsl #3]
1006538d8:     	bic	x9, x10, x9
1006538dc:     	str	x9, [x0, x8, lsl #3]
1006538e0:     	add	x8, x8, #0x1
1006538e4:     	b	0x1006538c8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x598>
1006538e8:     	cmp	x21, x8
1006538ec:     	b.eq	0x100653850 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x520>
1006538f0:     	ldr	x9, [x23, x8, lsl #3]
1006538f4:     	ldr	x10, [x24, x8, lsl #3]
1006538f8:     	and	x9, x10, x9
1006538fc:     	str	x9, [x0, x8, lsl #3]
100653900:     	add	x8, x8, #0x1
100653904:     	b	0x1006538e8 <__RNvXs3_NtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteINtB5_6FiniteINtB5_5DenseKb1_EENtNtB7_7carrier7Carrier2opB9_+0x5b8>
