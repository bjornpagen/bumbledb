
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d15334 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>:
100d15334:     	sub	sp, sp, #0x50
100d15338:     	stp	x24, x23, [sp, #0x10]
100d1533c:     	stp	x22, x21, [sp, #0x20]
100d15340:     	stp	x20, x19, [sp, #0x30]
100d15344:     	stp	x29, x30, [sp, #0x40]
100d15348:     	add	x29, sp, #0x40
100d1534c:     	stp	x3, x5, [sp]
100d15350:     	cmp	x3, x5
100d15354:     	b.ne	0x100d15968 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x634>
100d15358:     	and	x8, x1, #0xff
100d1535c:     	mov	x21, x4
100d15360:     	mov	x19, x3
100d15364:     	mov	x22, x2
100d15368:     	mov	x20, x0
100d1536c:     	adrp	x9, 0x10131e000 <dyld_stub_binder+0x10131e000>
100d15370:     	add	x9, x9, #0x6ce
100d15374:     	adr	x10, 0x100d15384 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x50>
100d15378:     	ldrb	w11, [x9, x8]
100d1537c:     	add	x10, x10, x11, lsl #2
100d15380:     	br	x10
100d15384:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15388:     	lsl	x21, x19, #3
100d1538c:     	mov	x0, x21
100d15390:     	mov	w1, #0x1                ; =1
100d15394:     	bl	0x101284ba4 <dyld_stub_binder+0x101284ba4>
100d15398:     	cbnz	x0, 0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d1539c:     	b	0x100d15990 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100d153a0:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d153a4:     	lsl	x23, x19, #3
100d153a8:     	mov	x0, x23
100d153ac:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d153b0:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d153b4:     	cmp	x19, #0x8
100d153b8:     	b.hs	0x100d155f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2bc>
100d153bc:     	mov	x8, #0x0                ; =0
100d153c0:     	b	0x100d159b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x67c>
100d153c4:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d153c8:     	lsl	x23, x19, #3
100d153cc:     	mov	x0, x23
100d153d0:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d153d4:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d153d8:     	cmp	x19, #0x8
100d153dc:     	b.hs	0x100d15638 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x304>
100d153e0:     	mov	x8, #0x0                ; =0
100d153e4:     	b	0x100d159d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x69c>
100d153e8:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d153ec:     	lsl	x23, x19, #3
100d153f0:     	mov	x0, x23
100d153f4:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d153f8:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d153fc:     	cmp	x19, #0x8
100d15400:     	b.hs	0x100d15680 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x34c>
100d15404:     	mov	x8, #0x0                ; =0
100d15408:     	b	0x100d159f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6bc>
100d1540c:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15410:     	lsl	x21, x19, #3
100d15414:     	mov	x0, x21
100d15418:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d1541c:     	cbz	x0, 0x100d15990 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100d15420:     	mov	x23, x0
100d15424:     	mov	x1, x22
100d15428:     	mov	x2, x21
100d1542c:     	b	0x100d1559c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x268>
100d15430:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15434:     	lsl	x23, x19, #3
100d15438:     	mov	x0, x23
100d1543c:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15440:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d15444:     	cmp	x19, #0x8
100d15448:     	b.hs	0x100d156c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x394>
100d1544c:     	mov	x8, #0x0                ; =0
100d15450:     	b	0x100d15a10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6dc>
100d15454:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15458:     	lsl	x23, x19, #3
100d1545c:     	mov	x0, x23
100d15460:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15464:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d15468:     	cmp	x19, #0x8
100d1546c:     	b.hs	0x100d15720 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3ec>
100d15470:     	mov	x8, #0x0                ; =0
100d15474:     	b	0x100d15a30 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6fc>
100d15478:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d1547c:     	lsl	x21, x19, #3
100d15480:     	mov	x0, x21
100d15484:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15488:     	cbz	x0, 0x100d15990 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100d1548c:     	mov	x22, x0
100d15490:     	mov	w1, #0xff               ; =255
100d15494:     	mov	x2, x21
100d15498:     	bl	0x101284db4 <dyld_stub_binder+0x101284db4>
100d1549c:     	mov	x0, x22
100d154a0:     	b	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d154a4:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d154a8:     	lsl	x21, x19, #3
100d154ac:     	mov	x0, x21
100d154b0:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d154b4:     	cbz	x0, 0x100d15990 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100d154b8:     	cmp	x19, #0x8
100d154bc:     	b.hs	0x100d15768 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x434>
100d154c0:     	mov	x8, #0x0                ; =0
100d154c4:     	b	0x100d15a50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x71c>
100d154c8:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d154cc:     	lsl	x23, x19, #3
100d154d0:     	mov	x0, x23
100d154d4:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d154d8:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d154dc:     	cmp	x19, #0x8
100d154e0:     	b.hs	0x100d157a4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x470>
100d154e4:     	mov	x8, #0x0                ; =0
100d154e8:     	b	0x100d15a6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x738>
100d154ec:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d154f0:     	lsl	x23, x19, #3
100d154f4:     	mov	x0, x23
100d154f8:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d154fc:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d15500:     	cmp	x19, #0x8
100d15504:     	b.hs	0x100d157fc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4c8>
100d15508:     	mov	x8, #0x0                ; =0
100d1550c:     	b	0x100d15a90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x75c>
100d15510:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15514:     	lsl	x22, x19, #3
100d15518:     	mov	x0, x22
100d1551c:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15520:     	cbz	x0, 0x100d1599c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x668>
100d15524:     	cmp	x19, #0x8
100d15528:     	b.hs	0x100d15854 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x520>
100d1552c:     	mov	x8, #0x0                ; =0
100d15530:     	b	0x100d15ab4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x780>
100d15534:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15538:     	lsl	x23, x19, #3
100d1553c:     	mov	x0, x23
100d15540:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15544:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d15548:     	cmp	x19, #0x8
100d1554c:     	b.hs	0x100d15890 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x55c>
100d15550:     	mov	x8, #0x0                ; =0
100d15554:     	b	0x100d15ad0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x79c>
100d15558:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d1555c:     	lsl	x23, x19, #3
100d15560:     	mov	x0, x23
100d15564:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d15568:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d1556c:     	cmp	x19, #0x8
100d15570:     	b.hs	0x100d158d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5a4>
100d15574:     	mov	x8, #0x0                ; =0
100d15578:     	b	0x100d15af0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7bc>
100d1557c:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d15580:     	lsl	x22, x19, #3
100d15584:     	mov	x0, x22
100d15588:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d1558c:     	cbz	x0, 0x100d1599c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x668>
100d15590:     	mov	x23, x0
100d15594:     	mov	x1, x21
100d15598:     	mov	x2, x22
100d1559c:     	bl	0x101284d9c <dyld_stub_binder+0x101284d9c>
100d155a0:     	mov	x0, x23
100d155a4:     	b	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d155a8:     	cbz	x19, 0x100d155cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x298>
100d155ac:     	lsl	x23, x19, #3
100d155b0:     	mov	x0, x23
100d155b4:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100d155b8:     	cbz	x0, 0x100d15984 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x650>
100d155bc:     	cmp	x19, #0x8
100d155c0:     	b.hs	0x100d15920 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5ec>
100d155c4:     	mov	x8, #0x0                ; =0
100d155c8:     	b	0x100d15b10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7dc>
100d155cc:     	mov	w0, #0x8                ; =8
100d155d0:     	stp	x19, x0, [x20]
100d155d4:     	str	x19, [x20, #0x10]
100d155d8:     	ldp	x29, x30, [sp, #0x40]
100d155dc:     	ldp	x20, x19, [sp, #0x30]
100d155e0:     	ldp	x22, x21, [sp, #0x20]
100d155e4:     	ldp	x24, x23, [sp, #0x10]
100d155e8:     	add	sp, sp, #0x50
100d155ec:     	ret
100d155f0:     	and	x8, x19, #0xffffffffffffff8
100d155f4:     	add	x9, x22, #0x20
100d155f8:     	add	x10, x0, #0x20
100d155fc:     	add	x11, x21, #0x20
100d15600:     	and	x12, x19, #0xffffffffffffff8
100d15604:     	ldp	q0, q1, [x9, #-0x20]
100d15608:     	ldp	q2, q3, [x9], #0x40
100d1560c:     	ldp	q4, q5, [x11, #-0x20]
100d15610:     	ldp	q6, q7, [x11], #0x40
100d15614:     	orr.16b	v0, v4, v0
100d15618:     	orr.16b	v1, v5, v1
100d1561c:     	orr.16b	v2, v6, v2
100d15620:     	orr.16b	v3, v7, v3
100d15624:     	stp	q0, q1, [x10, #-0x20]
100d15628:     	stp	q2, q3, [x10], #0x40
100d1562c:     	subs	x12, x12, #0x8
100d15630:     	b.ne	0x100d15604 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2d0>
100d15634:     	b	0x100d159a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100d15638:     	and	x8, x19, #0xffffffffffffff8
100d1563c:     	add	x9, x22, #0x20
100d15640:     	add	x10, x0, #0x20
100d15644:     	add	x11, x21, #0x20
100d15648:     	and	x12, x19, #0xffffffffffffff8
100d1564c:     	ldp	q0, q1, [x9, #-0x20]
100d15650:     	ldp	q2, q3, [x9], #0x40
100d15654:     	ldp	q4, q5, [x11, #-0x20]
100d15658:     	ldp	q6, q7, [x11], #0x40
100d1565c:     	orn.16b	v0, v4, v0
100d15660:     	orn.16b	v1, v5, v1
100d15664:     	orn.16b	v2, v6, v2
100d15668:     	orn.16b	v3, v7, v3
100d1566c:     	stp	q0, q1, [x10, #-0x20]
100d15670:     	stp	q2, q3, [x10], #0x40
100d15674:     	subs	x12, x12, #0x8
100d15678:     	b.ne	0x100d1564c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x318>
100d1567c:     	b	0x100d159c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x694>
100d15680:     	and	x8, x19, #0xffffffffffffff8
100d15684:     	add	x9, x22, #0x20
100d15688:     	add	x10, x0, #0x20
100d1568c:     	add	x11, x21, #0x20
100d15690:     	and	x12, x19, #0xffffffffffffff8
100d15694:     	ldp	q0, q1, [x9, #-0x20]
100d15698:     	ldp	q2, q3, [x9], #0x40
100d1569c:     	ldp	q4, q5, [x11, #-0x20]
100d156a0:     	ldp	q6, q7, [x11], #0x40
100d156a4:     	bic.16b	v0, v0, v4
100d156a8:     	bic.16b	v1, v1, v5
100d156ac:     	bic.16b	v2, v2, v6
100d156b0:     	bic.16b	v3, v3, v7
100d156b4:     	stp	q0, q1, [x10, #-0x20]
100d156b8:     	stp	q2, q3, [x10], #0x40
100d156bc:     	subs	x12, x12, #0x8
100d156c0:     	b.ne	0x100d15694 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x360>
100d156c4:     	b	0x100d159e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b4>
100d156c8:     	and	x8, x19, #0xffffffffffffff8
100d156cc:     	add	x9, x22, #0x20
100d156d0:     	add	x10, x0, #0x20
100d156d4:     	add	x11, x21, #0x20
100d156d8:     	and	x12, x19, #0xffffffffffffff8
100d156dc:     	ldp	q0, q1, [x9, #-0x20]
100d156e0:     	ldp	q2, q3, [x9], #0x40
100d156e4:     	ldp	q4, q5, [x11, #-0x20]
100d156e8:     	ldp	q6, q7, [x11], #0x40
100d156ec:     	eor.16b	v0, v0, v4
100d156f0:     	eor.16b	v1, v1, v5
100d156f4:     	eor.16b	v2, v2, v6
100d156f8:     	eor.16b	v3, v3, v7
100d156fc:     	mvn.16b	v0, v0
100d15700:     	mvn.16b	v1, v1
100d15704:     	mvn.16b	v2, v2
100d15708:     	mvn.16b	v3, v3
100d1570c:     	stp	q0, q1, [x10, #-0x20]
100d15710:     	stp	q2, q3, [x10], #0x40
100d15714:     	subs	x12, x12, #0x8
100d15718:     	b.ne	0x100d156dc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3a8>
100d1571c:     	b	0x100d15a08 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d4>
100d15720:     	and	x8, x19, #0xffffffffffffff8
100d15724:     	add	x9, x22, #0x20
100d15728:     	add	x10, x0, #0x20
100d1572c:     	add	x11, x21, #0x20
100d15730:     	and	x12, x19, #0xffffffffffffff8
100d15734:     	ldp	q0, q1, [x9, #-0x20]
100d15738:     	ldp	q2, q3, [x9], #0x40
100d1573c:     	ldp	q4, q5, [x11, #-0x20]
100d15740:     	ldp	q6, q7, [x11], #0x40
100d15744:     	bic.16b	v0, v4, v0
100d15748:     	bic.16b	v1, v5, v1
100d1574c:     	bic.16b	v2, v6, v2
100d15750:     	bic.16b	v3, v7, v3
100d15754:     	stp	q0, q1, [x10, #-0x20]
100d15758:     	stp	q2, q3, [x10], #0x40
100d1575c:     	subs	x12, x12, #0x8
100d15760:     	b.ne	0x100d15734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x400>
100d15764:     	b	0x100d15a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f4>
100d15768:     	and	x8, x19, #0xffffffffffffff8
100d1576c:     	add	x9, x22, #0x20
100d15770:     	add	x10, x0, #0x20
100d15774:     	and	x11, x19, #0xffffffffffffff8
100d15778:     	ldp	q0, q1, [x9, #-0x20]
100d1577c:     	ldp	q2, q3, [x9], #0x40
100d15780:     	mvn.16b	v0, v0
100d15784:     	mvn.16b	v1, v1
100d15788:     	mvn.16b	v2, v2
100d1578c:     	mvn.16b	v3, v3
100d15790:     	stp	q0, q1, [x10, #-0x20]
100d15794:     	stp	q2, q3, [x10], #0x40
100d15798:     	subs	x11, x11, #0x8
100d1579c:     	b.ne	0x100d15778 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x444>
100d157a0:     	b	0x100d15a48 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x714>
100d157a4:     	and	x8, x19, #0xffffffffffffff8
100d157a8:     	add	x9, x22, #0x20
100d157ac:     	add	x10, x0, #0x20
100d157b0:     	add	x11, x21, #0x20
100d157b4:     	and	x12, x19, #0xffffffffffffff8
100d157b8:     	ldp	q0, q1, [x9, #-0x20]
100d157bc:     	ldp	q2, q3, [x9], #0x40
100d157c0:     	ldp	q4, q5, [x11, #-0x20]
100d157c4:     	ldp	q6, q7, [x11], #0x40
100d157c8:     	and.16b	v0, v4, v0
100d157cc:     	and.16b	v1, v5, v1
100d157d0:     	and.16b	v2, v6, v2
100d157d4:     	and.16b	v3, v7, v3
100d157d8:     	mvn.16b	v0, v0
100d157dc:     	mvn.16b	v1, v1
100d157e0:     	mvn.16b	v2, v2
100d157e4:     	mvn.16b	v3, v3
100d157e8:     	stp	q0, q1, [x10, #-0x20]
100d157ec:     	stp	q2, q3, [x10], #0x40
100d157f0:     	subs	x12, x12, #0x8
100d157f4:     	b.ne	0x100d157b8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x484>
100d157f8:     	b	0x100d15a64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x730>
100d157fc:     	and	x8, x19, #0xffffffffffffff8
100d15800:     	add	x9, x22, #0x20
100d15804:     	add	x10, x0, #0x20
100d15808:     	add	x11, x21, #0x20
100d1580c:     	and	x12, x19, #0xffffffffffffff8
100d15810:     	ldp	q0, q1, [x9, #-0x20]
100d15814:     	ldp	q2, q3, [x9], #0x40
100d15818:     	ldp	q4, q5, [x11, #-0x20]
100d1581c:     	ldp	q6, q7, [x11], #0x40
100d15820:     	orr.16b	v0, v4, v0
100d15824:     	orr.16b	v1, v5, v1
100d15828:     	orr.16b	v2, v6, v2
100d1582c:     	orr.16b	v3, v7, v3
100d15830:     	mvn.16b	v0, v0
100d15834:     	mvn.16b	v1, v1
100d15838:     	mvn.16b	v2, v2
100d1583c:     	mvn.16b	v3, v3
100d15840:     	stp	q0, q1, [x10, #-0x20]
100d15844:     	stp	q2, q3, [x10], #0x40
100d15848:     	subs	x12, x12, #0x8
100d1584c:     	b.ne	0x100d15810 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4dc>
100d15850:     	b	0x100d15a88 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100d15854:     	and	x8, x19, #0xffffffffffffff8
100d15858:     	add	x9, x21, #0x20
100d1585c:     	add	x10, x0, #0x20
100d15860:     	and	x11, x19, #0xffffffffffffff8
100d15864:     	ldp	q0, q1, [x9, #-0x20]
100d15868:     	ldp	q2, q3, [x9], #0x40
100d1586c:     	mvn.16b	v0, v0
100d15870:     	mvn.16b	v1, v1
100d15874:     	mvn.16b	v2, v2
100d15878:     	mvn.16b	v3, v3
100d1587c:     	stp	q0, q1, [x10, #-0x20]
100d15880:     	stp	q2, q3, [x10], #0x40
100d15884:     	subs	x11, x11, #0x8
100d15888:     	b.ne	0x100d15864 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x530>
100d1588c:     	b	0x100d15aac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100d15890:     	and	x8, x19, #0xffffffffffffff8
100d15894:     	add	x9, x22, #0x20
100d15898:     	add	x10, x0, #0x20
100d1589c:     	add	x11, x21, #0x20
100d158a0:     	and	x12, x19, #0xffffffffffffff8
100d158a4:     	ldp	q0, q1, [x9, #-0x20]
100d158a8:     	ldp	q2, q3, [x9], #0x40
100d158ac:     	ldp	q4, q5, [x11, #-0x20]
100d158b0:     	ldp	q6, q7, [x11], #0x40
100d158b4:     	orn.16b	v0, v0, v4
100d158b8:     	orn.16b	v1, v1, v5
100d158bc:     	orn.16b	v2, v2, v6
100d158c0:     	orn.16b	v3, v3, v7
100d158c4:     	stp	q0, q1, [x10, #-0x20]
100d158c8:     	stp	q2, q3, [x10], #0x40
100d158cc:     	subs	x12, x12, #0x8
100d158d0:     	b.ne	0x100d158a4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x570>
100d158d4:     	b	0x100d15ac8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x794>
100d158d8:     	and	x8, x19, #0xffffffffffffff8
100d158dc:     	add	x9, x22, #0x20
100d158e0:     	add	x10, x0, #0x20
100d158e4:     	add	x11, x21, #0x20
100d158e8:     	and	x12, x19, #0xffffffffffffff8
100d158ec:     	ldp	q0, q1, [x9, #-0x20]
100d158f0:     	ldp	q2, q3, [x9], #0x40
100d158f4:     	ldp	q4, q5, [x11, #-0x20]
100d158f8:     	ldp	q6, q7, [x11], #0x40
100d158fc:     	eor.16b	v0, v4, v0
100d15900:     	eor.16b	v1, v5, v1
100d15904:     	eor.16b	v2, v6, v2
100d15908:     	eor.16b	v3, v7, v3
100d1590c:     	stp	q0, q1, [x10, #-0x20]
100d15910:     	stp	q2, q3, [x10], #0x40
100d15914:     	subs	x12, x12, #0x8
100d15918:     	b.ne	0x100d158ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5b8>
100d1591c:     	b	0x100d15ae8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b4>
100d15920:     	and	x8, x19, #0xffffffffffffff8
100d15924:     	add	x9, x22, #0x20
100d15928:     	add	x10, x0, #0x20
100d1592c:     	add	x11, x21, #0x20
100d15930:     	and	x12, x19, #0xffffffffffffff8
100d15934:     	ldp	q0, q1, [x9, #-0x20]
100d15938:     	ldp	q2, q3, [x9], #0x40
100d1593c:     	ldp	q4, q5, [x11, #-0x20]
100d15940:     	ldp	q6, q7, [x11], #0x40
100d15944:     	and.16b	v0, v4, v0
100d15948:     	and.16b	v1, v5, v1
100d1594c:     	and.16b	v2, v6, v2
100d15950:     	and.16b	v3, v7, v3
100d15954:     	stp	q0, q1, [x10, #-0x20]
100d15958:     	stp	q2, q3, [x10], #0x40
100d1595c:     	subs	x12, x12, #0x8
100d15960:     	b.ne	0x100d15934 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x600>
100d15964:     	b	0x100d15b08 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d4>
100d15968:     	adrp	x5, 0x101500000 <dyld_stub_binder+0x101500000>
100d1596c:     	add	x5, x5, #0x3a0
100d15970:     	mov	x1, sp
100d15974:     	add	x2, sp, #0x8
100d15978:     	mov	w0, #0x0                ; =0
100d1597c:     	mov	x3, #0x0                ; =0
100d15980:     	bl	0x10127c770 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d15984:     	mov	w0, #0x8                ; =8
100d15988:     	mov	x1, x23
100d1598c:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d15990:     	mov	w0, #0x8                ; =8
100d15994:     	mov	x1, x21
100d15998:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d1599c:     	mov	w0, #0x8                ; =8
100d159a0:     	mov	x1, x22
100d159a4:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d159a8:     	cmp	x19, x8
100d159ac:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d159b0:     	ldr	x9, [x22, x8, lsl #3]
100d159b4:     	ldr	x10, [x21, x8, lsl #3]
100d159b8:     	orr	x9, x10, x9
100d159bc:     	str	x9, [x0, x8, lsl #3]
100d159c0:     	add	x8, x8, #0x1
100d159c4:     	b	0x100d159a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100d159c8:     	cmp	x19, x8
100d159cc:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d159d0:     	ldr	x9, [x22, x8, lsl #3]
100d159d4:     	ldr	x10, [x21, x8, lsl #3]
100d159d8:     	orn	x9, x10, x9
100d159dc:     	str	x9, [x0, x8, lsl #3]
100d159e0:     	add	x8, x8, #0x1
100d159e4:     	b	0x100d159c8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x694>
100d159e8:     	cmp	x19, x8
100d159ec:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d159f0:     	ldr	x9, [x22, x8, lsl #3]
100d159f4:     	ldr	x10, [x21, x8, lsl #3]
100d159f8:     	bic	x9, x9, x10
100d159fc:     	str	x9, [x0, x8, lsl #3]
100d15a00:     	add	x8, x8, #0x1
100d15a04:     	b	0x100d159e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b4>
100d15a08:     	cmp	x19, x8
100d15a0c:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15a10:     	ldr	x9, [x22, x8, lsl #3]
100d15a14:     	ldr	x10, [x21, x8, lsl #3]
100d15a18:     	eon	x9, x9, x10
100d15a1c:     	str	x9, [x0, x8, lsl #3]
100d15a20:     	add	x8, x8, #0x1
100d15a24:     	b	0x100d15a08 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d4>
100d15a28:     	cmp	x19, x8
100d15a2c:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15a30:     	ldr	x9, [x22, x8, lsl #3]
100d15a34:     	ldr	x10, [x21, x8, lsl #3]
100d15a38:     	bic	x9, x10, x9
100d15a3c:     	str	x9, [x0, x8, lsl #3]
100d15a40:     	add	x8, x8, #0x1
100d15a44:     	b	0x100d15a28 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f4>
100d15a48:     	cmp	x19, x8
100d15a4c:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15a50:     	ldr	x9, [x22, x8, lsl #3]
100d15a54:     	mvn	x9, x9
100d15a58:     	str	x9, [x0, x8, lsl #3]
100d15a5c:     	add	x8, x8, #0x1
100d15a60:     	b	0x100d15a48 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x714>
100d15a64:     	cmp	x19, x8
100d15a68:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15a6c:     	ldr	x9, [x22, x8, lsl #3]
100d15a70:     	ldr	x10, [x21, x8, lsl #3]
100d15a74:     	and	x9, x10, x9
100d15a78:     	mvn	x9, x9
100d15a7c:     	str	x9, [x0, x8, lsl #3]
100d15a80:     	add	x8, x8, #0x1
100d15a84:     	b	0x100d15a64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x730>
100d15a88:     	cmp	x19, x8
100d15a8c:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15a90:     	ldr	x9, [x22, x8, lsl #3]
100d15a94:     	ldr	x10, [x21, x8, lsl #3]
100d15a98:     	orr	x9, x10, x9
100d15a9c:     	mvn	x9, x9
100d15aa0:     	str	x9, [x0, x8, lsl #3]
100d15aa4:     	add	x8, x8, #0x1
100d15aa8:     	b	0x100d15a88 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100d15aac:     	cmp	x19, x8
100d15ab0:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15ab4:     	ldr	x9, [x21, x8, lsl #3]
100d15ab8:     	mvn	x9, x9
100d15abc:     	str	x9, [x0, x8, lsl #3]
100d15ac0:     	add	x8, x8, #0x1
100d15ac4:     	b	0x100d15aac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100d15ac8:     	cmp	x19, x8
100d15acc:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15ad0:     	ldr	x9, [x22, x8, lsl #3]
100d15ad4:     	ldr	x10, [x21, x8, lsl #3]
100d15ad8:     	orn	x9, x9, x10
100d15adc:     	str	x9, [x0, x8, lsl #3]
100d15ae0:     	add	x8, x8, #0x1
100d15ae4:     	b	0x100d15ac8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x794>
100d15ae8:     	cmp	x19, x8
100d15aec:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15af0:     	ldr	x9, [x22, x8, lsl #3]
100d15af4:     	ldr	x10, [x21, x8, lsl #3]
100d15af8:     	eor	x9, x10, x9
100d15afc:     	str	x9, [x0, x8, lsl #3]
100d15b00:     	add	x8, x8, #0x1
100d15b04:     	b	0x100d15ae8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b4>
100d15b08:     	cmp	x19, x8
100d15b0c:     	b.eq	0x100d155d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x29c>
100d15b10:     	ldr	x9, [x22, x8, lsl #3]
100d15b14:     	ldr	x10, [x21, x8, lsl #3]
100d15b18:     	and	x9, x10, x9
100d15b1c:     	str	x9, [x0, x8, lsl #3]
100d15b20:     	add	x8, x8, #0x1
100d15b24:     	b	0x100d15b08 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d4>
100d15b28:     	nop
100d15b2c:     	nop
100d15b30:     	nop
100d15b34:     	nop
100d15b38:     	nop
100d15b3c:     	nop
