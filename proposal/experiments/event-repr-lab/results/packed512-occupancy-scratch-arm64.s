
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bdc190 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>:
100bdc190:     	sub	sp, sp, #0x90
100bdc194:     	stp	x28, x27, [sp, #0x30]
100bdc198:     	stp	x26, x25, [sp, #0x40]
100bdc19c:     	stp	x24, x23, [sp, #0x50]
100bdc1a0:     	stp	x22, x21, [sp, #0x60]
100bdc1a4:     	stp	x20, x19, [sp, #0x70]
100bdc1a8:     	stp	x29, x30, [sp, #0x80]
100bdc1ac:     	add	x29, sp, #0x80
100bdc1b0:     	cbz	w1, 0x100bdc5a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x414>
100bdc1b4:     	ldr	x8, [x4, #0x18]
100bdc1b8:     	cbz	x8, 0x100bdc288 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bdc1bc:     	mov	x8, #0x0                ; =0
100bdc1c0:     	mov	w9, w1
100bdc1c4:     	mov	x10, #0xa9c5            ; =43461
100bdc1c8:     	movk	x10, #0x2e62, lsl #16
100bdc1cc:     	movk	x10, #0x7aea, lsl #32
100bdc1d0:     	movk	x10, #0xf135, lsl #48
100bdc1d4:     	mul	x9, x9, x10
100bdc1d8:     	add	x9, x9, w2, uxtw
100bdc1dc:     	mul	x9, x9, x10
100bdc1e0:     	add	x9, x9, w3, uxtw
100bdc1e4:     	mul	x9, x9, x10
100bdc1e8:     	ror	x11, x9, #0x2c
100bdc1ec:     	lsr	x12, x11, #57
100bdc1f0:     	ldp	x10, x9, [x4]
100bdc1f4:     	dup.8b	v0, w12
100bdc1f8:     	movi.2d	v1, #0xffffffffffffffff
100bdc1fc:     	and	x11, x11, x9
100bdc200:     	ldr	d2, [x10, x11]
100bdc204:     	cmeq.8b	v3, v2, v0
100bdc208:     	fmov	x12, d3
100bdc20c:     	ands	x12, x12, #0x8080808080808080
100bdc210:     	b.eq	0x100bdc258 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bdc214:     	rbit	x13, x12
100bdc218:     	clz	x13, x13
100bdc21c:     	add	x13, x11, x13, lsr #3
100bdc220:     	and	x13, x13, x9
100bdc224:     	sub	x13, x10, x13, lsl #4
100bdc228:     	ldur	w14, [x13, #-0x10]
100bdc22c:     	cmp	w1, w14
100bdc230:     	b.ne	0x100bdc24c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bdc234:     	ldur	w14, [x13, #-0xc]
100bdc238:     	cmp	w2, w14
100bdc23c:     	b.ne	0x100bdc24c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bdc240:     	ldur	w14, [x13, #-0x8]
100bdc244:     	cmp	w3, w14
100bdc248:     	b.eq	0x100bdc5ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x41c>
100bdc24c:     	sub	x13, x12, #0x2
100bdc250:     	ands	x12, x13, x12
100bdc254:     	b.ne	0x100bdc214 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bdc258:     	cmeq.8b	v2, v2, v1
100bdc25c:     	fmov	x12, d2
100bdc260:     	cbnz	x12, 0x100bdc288 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bdc264:     	add	x8, x8, #0x8
100bdc268:     	add	x11, x11, x8
100bdc26c:     	and	x11, x11, x9
100bdc270:     	ldr	d2, [x10, x11]
100bdc274:     	cmeq.8b	v3, v2, v0
100bdc278:     	fmov	x12, d3
100bdc27c:     	ands	x12, x12, #0x8080808080808080
100bdc280:     	b.ne	0x100bdc214 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bdc284:     	b	0x100bdc258 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bdc288:     	tbnz	w1, #0x1, 0x100bdc5b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x424>
100bdc28c:     	ldr	x10, [x0, #0xc8]
100bdc290:     	tbnz	w2, #0x1, 0x100bdc5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x444>
100bdc294:     	ldr	x8, [x0, #0xc8]
100bdc298:     	cmp	x8, x10
100bdc29c:     	csel	x10, x8, x10, lo
100bdc2a0:     	tbnz	w3, #0x1, 0x100bdc5fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x46c>
100bdc2a4:     	ldr	x8, [x0, #0xc8]
100bdc2a8:     	cmp	x8, x10
100bdc2ac:     	csel	x21, x8, x10, lo
100bdc2b0:     	cmp	x21, x8
100bdc2b4:     	b.ne	0x100bdc62c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x49c>
100bdc2b8:     	lsr	w9, w1, #2
100bdc2bc:     	ldr	x8, [x0, #0x58]
100bdc2c0:     	cmp	x8, x9
100bdc2c4:     	b.ls	0x100bdc890 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bdc2c8:     	ldr	x23, [x0, #0x50]
100bdc2cc:     	add	x9, x23, x9, lsl #6
100bdc2d0:     	ldp	x7, x5, [x9]
100bdc2d4:     	ldp	x16, x14, [x9, #0x10]
100bdc2d8:     	ldp	x13, x12, [x9, #0x20]
100bdc2dc:     	ldp	x11, x10, [x9, #0x30]
100bdc2e0:     	tbz	w1, #0x0, 0x100bdc314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x184>
100bdc2e4:     	ldp	x9, x15, [x0, #0xd8]
100bdc2e8:     	eor	x7, x9, x7
100bdc2ec:     	eor	x5, x15, x5
100bdc2f0:     	ldp	x9, x15, [x0, #0xe8]
100bdc2f4:     	eor	x16, x9, x16
100bdc2f8:     	eor	x14, x15, x14
100bdc2fc:     	ldp	x9, x15, [x0, #0xf8]
100bdc300:     	eor	x13, x9, x13
100bdc304:     	eor	x12, x15, x12
100bdc308:     	ldp	x9, x15, [x0, #0x108]
100bdc30c:     	eor	x11, x9, x11
100bdc310:     	eor	x10, x15, x10
100bdc314:     	lsr	w9, w2, #2
100bdc318:     	cmp	x8, x9
100bdc31c:     	b.ls	0x100bdc890 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bdc320:     	add	x9, x23, x9, lsl #6
100bdc324:     	ldp	x24, x22, [x9]
100bdc328:     	ldp	x21, x20, [x9, #0x10]
100bdc32c:     	ldp	x19, x6, [x9, #0x20]
100bdc330:     	ldp	x17, x15, [x9, #0x30]
100bdc334:     	tbz	w2, #0x0, 0x100bdc368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x1d8>
100bdc338:     	ldp	x9, x25, [x0, #0xd8]
100bdc33c:     	eor	x24, x9, x24
100bdc340:     	eor	x22, x25, x22
100bdc344:     	ldp	x9, x25, [x0, #0xe8]
100bdc348:     	eor	x21, x9, x21
100bdc34c:     	eor	x20, x25, x20
100bdc350:     	ldp	x9, x25, [x0, #0xf8]
100bdc354:     	eor	x19, x9, x19
100bdc358:     	eor	x6, x25, x6
100bdc35c:     	ldp	x9, x25, [x0, #0x108]
100bdc360:     	eor	x17, x9, x17
100bdc364:     	eor	x15, x25, x15
100bdc368:     	lsr	w9, w3, #2
100bdc36c:     	cmp	x8, x9
100bdc370:     	b.ls	0x100bdc890 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bdc374:     	stp	x15, x10, [sp]
100bdc378:     	add	x8, x23, x9, lsl #6
100bdc37c:     	ldp	x30, x28, [x8]
100bdc380:     	ldp	x27, x26, [x8, #0x10]
100bdc384:     	ldp	x25, x23, [x8, #0x20]
100bdc388:     	ldp	x9, x8, [x8, #0x30]
100bdc38c:     	stp	x11, x12, [sp, #0x10]
100bdc390:     	mov	x15, x13
100bdc394:     	tbz	w3, #0x0, 0x100bdc3c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x238>
100bdc398:     	ldp	x10, x11, [x0, #0xd8]
100bdc39c:     	eor	x30, x10, x30
100bdc3a0:     	eor	x28, x11, x28
100bdc3a4:     	ldp	x10, x11, [x0, #0xe8]
100bdc3a8:     	eor	x27, x10, x27
100bdc3ac:     	eor	x26, x11, x26
100bdc3b0:     	ldp	x10, x11, [x0, #0xf8]
100bdc3b4:     	eor	x25, x10, x25
100bdc3b8:     	eor	x23, x11, x23
100bdc3bc:     	ldp	x10, x11, [x0, #0x108]
100bdc3c0:     	eor	x9, x10, x9
100bdc3c4:     	eor	x8, x11, x8
100bdc3c8:     	bic	x10, x7, x24
100bdc3cc:     	tst	x10, x30
100bdc3d0:     	mov	w0, #0x2                ; =2
100bdc3d4:     	csel	w11, wzr, w0, eq
100bdc3d8:     	and	x24, x24, x7
100bdc3dc:     	bics	xzr, x24, x30
100bdc3e0:     	mov	w7, #0x4                ; =4
100bdc3e4:     	csel	w12, wzr, w7, eq
100bdc3e8:     	tst	x24, x30
100bdc3ec:     	mov	w24, #0x8               ; =8
100bdc3f0:     	csel	w13, wzr, w24, eq
100bdc3f4:     	bics	xzr, x10, x30
100bdc3f8:     	cinc	w10, w11, ne
100bdc3fc:     	orr	w11, w12, w13
100bdc400:     	orr	w30, w10, w11
100bdc404:     	cmp	w30, #0xf
100bdc408:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc40c:     	bic	x10, x5, x22
100bdc410:     	tst	x10, x28
100bdc414:     	csel	w11, wzr, w0, eq
100bdc418:     	and	x12, x22, x5
100bdc41c:     	bics	xzr, x12, x28
100bdc420:     	csel	w13, wzr, w7, eq
100bdc424:     	tst	x12, x28
100bdc428:     	csel	w12, wzr, w24, eq
100bdc42c:     	bics	xzr, x10, x28
100bdc430:     	cinc	w10, w11, ne
100bdc434:     	orr	w11, w13, w12
100bdc438:     	orr	w10, w10, w11
100bdc43c:     	orr	w5, w10, w30
100bdc440:     	cmp	w5, #0xf
100bdc444:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc448:     	bic	x10, x16, x21
100bdc44c:     	tst	x10, x27
100bdc450:     	csel	w11, wzr, w0, eq
100bdc454:     	and	x12, x21, x16
100bdc458:     	bics	xzr, x12, x27
100bdc45c:     	mov	w16, #0x4               ; =4
100bdc460:     	csel	w13, wzr, w16, eq
100bdc464:     	tst	x12, x27
100bdc468:     	mov	w7, #0x8                ; =8
100bdc46c:     	csel	w12, wzr, w7, eq
100bdc470:     	bics	xzr, x10, x27
100bdc474:     	cinc	w10, w11, ne
100bdc478:     	orr	w11, w13, w12
100bdc47c:     	orr	w10, w10, w11
100bdc480:     	orr	w5, w10, w5
100bdc484:     	cmp	w5, #0xf
100bdc488:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc48c:     	bic	x10, x14, x20
100bdc490:     	tst	x10, x26
100bdc494:     	csel	w11, wzr, w0, eq
100bdc498:     	and	x12, x20, x14
100bdc49c:     	bics	xzr, x12, x26
100bdc4a0:     	csel	w13, wzr, w16, eq
100bdc4a4:     	tst	x12, x26
100bdc4a8:     	csel	w12, wzr, w7, eq
100bdc4ac:     	bics	xzr, x10, x26
100bdc4b0:     	cinc	w10, w11, ne
100bdc4b4:     	orr	w11, w13, w12
100bdc4b8:     	orr	w10, w10, w11
100bdc4bc:     	orr	w16, w10, w5
100bdc4c0:     	cmp	w16, #0xf
100bdc4c4:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc4c8:     	bic	x10, x15, x19
100bdc4cc:     	tst	x10, x25
100bdc4d0:     	mov	w14, #0x2               ; =2
100bdc4d4:     	csel	w11, wzr, w14, eq
100bdc4d8:     	and	x12, x19, x15
100bdc4dc:     	bics	xzr, x12, x25
100bdc4e0:     	mov	w13, #0x4               ; =4
100bdc4e4:     	csel	w5, wzr, w13, eq
100bdc4e8:     	tst	x12, x25
100bdc4ec:     	mov	w0, #0x8                ; =8
100bdc4f0:     	csel	w12, wzr, w0, eq
100bdc4f4:     	bics	xzr, x10, x25
100bdc4f8:     	cinc	w10, w11, ne
100bdc4fc:     	orr	w11, w5, w12
100bdc500:     	orr	w10, w10, w11
100bdc504:     	orr	w16, w10, w16
100bdc508:     	cmp	w16, #0xf
100bdc50c:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc510:     	ldr	x12, [sp, #0x18]
100bdc514:     	bic	x10, x12, x6
100bdc518:     	tst	x10, x23
100bdc51c:     	csel	w11, wzr, w14, eq
100bdc520:     	and	x12, x6, x12
100bdc524:     	bics	xzr, x12, x23
100bdc528:     	csel	w13, wzr, w13, eq
100bdc52c:     	tst	x12, x23
100bdc530:     	csel	w12, wzr, w0, eq
100bdc534:     	bics	xzr, x10, x23
100bdc538:     	cinc	w10, w11, ne
100bdc53c:     	orr	w11, w13, w12
100bdc540:     	orr	w10, w10, w11
100bdc544:     	orr	w13, w10, w16
100bdc548:     	cmp	w13, #0xf
100bdc54c:     	b.eq	0x100bdc59c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bdc550:     	ldr	x11, [sp, #0x10]
100bdc554:     	bic	x10, x11, x17
100bdc558:     	tst	x10, x9
100bdc55c:     	mov	w12, #0x2               ; =2
100bdc560:     	csel	w16, wzr, w12, eq
100bdc564:     	and	x14, x17, x11
100bdc568:     	bics	xzr, x14, x9
100bdc56c:     	mov	w11, #0x4               ; =4
100bdc570:     	csel	w17, wzr, w11, eq
100bdc574:     	tst	x14, x9
100bdc578:     	mov	w14, #0x8               ; =8
100bdc57c:     	csel	w0, wzr, w14, eq
100bdc580:     	bics	xzr, x10, x9
100bdc584:     	cinc	w9, w16, ne
100bdc588:     	orr	w10, w17, w0
100bdc58c:     	orr	w9, w9, w10
100bdc590:     	orr	w9, w9, w13
100bdc594:     	cmp	w9, #0xf
100bdc598:     	b.ne	0x100bdc840 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6b0>
100bdc59c:     	mov	w20, #0xf               ; =15
100bdc5a0:     	b	0x100bdc804 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bdc5a4:     	mov	w20, #0x0               ; =0
100bdc5a8:     	b	0x100bdc81c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bdc5ac:     	ldurb	w20, [x13, #-0x4]
100bdc5b0:     	b	0x100bdc81c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bdc5b4:     	lsr	w8, w1, #2
100bdc5b8:     	ldr	x9, [x0, #0x40]
100bdc5bc:     	cmp	x9, x8
100bdc5c0:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc5c4:     	ldr	x9, [x0, #0x38]
100bdc5c8:     	lsl	x8, x8, #4
100bdc5cc:     	ldr	w10, [x9, x8]
100bdc5d0:     	tbz	w2, #0x1, 0x100bdc294 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x104>
100bdc5d4:     	lsr	w8, w2, #2
100bdc5d8:     	ldr	x9, [x0, #0x40]
100bdc5dc:     	cmp	x9, x8
100bdc5e0:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc5e4:     	ldr	x9, [x0, #0x38]
100bdc5e8:     	lsl	x8, x8, #4
100bdc5ec:     	ldr	w8, [x9, x8]
100bdc5f0:     	cmp	x8, x10
100bdc5f4:     	csel	x10, x8, x10, lo
100bdc5f8:     	tbz	w3, #0x1, 0x100bdc2a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x114>
100bdc5fc:     	lsr	w8, w3, #2
100bdc600:     	ldr	x9, [x0, #0x40]
100bdc604:     	cmp	x9, x8
100bdc608:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc60c:     	ldr	x9, [x0, #0x38]
100bdc610:     	lsl	x8, x8, #4
100bdc614:     	ldr	w9, [x9, x8]
100bdc618:     	ldr	x8, [x0, #0xc8]
100bdc61c:     	cmp	x9, x10
100bdc620:     	csel	x21, x9, x10, lo
100bdc624:     	cmp	x21, x8
100bdc628:     	b.eq	0x100bdc2b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x128>
100bdc62c:     	mov	x8, x1
100bdc630:     	tbz	w1, #0x1, 0x100bdc668 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bdc634:     	lsr	w8, w1, #2
100bdc638:     	ldr	x9, [x0, #0x40]
100bdc63c:     	cmp	x9, x8
100bdc640:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc644:     	ldr	x9, [x0, #0x38]
100bdc648:     	add	x9, x9, x8, lsl #4
100bdc64c:     	ldr	w10, [x9]
100bdc650:     	mov	x8, x1
100bdc654:     	cmp	x21, x10
100bdc658:     	b.ne	0x100bdc668 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bdc65c:     	ldr	w8, [x9, #0x4]
100bdc660:     	and	w9, w1, #0x1
100bdc664:     	eor	w8, w8, w9
100bdc668:     	mov	x9, x2
100bdc66c:     	tbz	w2, #0x1, 0x100bdc6a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bdc670:     	lsr	w9, w2, #2
100bdc674:     	ldr	x10, [x0, #0x40]
100bdc678:     	cmp	x10, x9
100bdc67c:     	b.ls	0x100bdc8a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x714>
100bdc680:     	ldr	x10, [x0, #0x38]
100bdc684:     	add	x10, x10, x9, lsl #4
100bdc688:     	ldr	w11, [x10]
100bdc68c:     	mov	x9, x2
100bdc690:     	cmp	x21, x11
100bdc694:     	b.ne	0x100bdc6a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bdc698:     	ldr	w9, [x10, #0x4]
100bdc69c:     	and	w10, w2, #0x1
100bdc6a0:     	eor	w9, w9, w10
100bdc6a4:     	mov	x22, x1
100bdc6a8:     	mov	x10, x3
100bdc6ac:     	tbz	w3, #0x1, 0x100bdc6e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bdc6b0:     	lsr	w10, w3, #2
100bdc6b4:     	ldr	x1, [x0, #0x40]
100bdc6b8:     	cmp	x1, x10
100bdc6bc:     	b.ls	0x100bdc8b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x728>
100bdc6c0:     	ldr	x11, [x0, #0x38]
100bdc6c4:     	add	x11, x11, x10, lsl #4
100bdc6c8:     	ldr	w12, [x11]
100bdc6cc:     	mov	x10, x3
100bdc6d0:     	cmp	x21, x12
100bdc6d4:     	b.ne	0x100bdc6e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bdc6d8:     	ldr	w10, [x11, #0x4]
100bdc6dc:     	and	w11, w3, #0x1
100bdc6e0:     	eor	w10, w10, w11
100bdc6e4:     	mov	x23, x2
100bdc6e8:     	mov	x24, x3
100bdc6ec:     	mov	x25, x0
100bdc6f0:     	mov	x1, x8
100bdc6f4:     	mov	x2, x9
100bdc6f8:     	mov	x3, x10
100bdc6fc:     	mov	x19, x4
100bdc700:     	bl	0x100bdc190 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bdc704:     	and	w8, w0, #0xff
100bdc708:     	cmp	w8, #0xf
100bdc70c:     	b.ne	0x100bdc720 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x590>
100bdc710:     	mov	w20, #0xf               ; =15
100bdc714:     	mov	x4, x19
100bdc718:     	mov	x3, x24
100bdc71c:     	b	0x100bdc7fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x66c>
100bdc720:     	mov	x20, x0
100bdc724:     	mov	x1, x22
100bdc728:     	mov	x10, x24
100bdc72c:     	mov	x11, x23
100bdc730:     	mov	x0, x25
100bdc734:     	tbz	w22, #0x1, 0x100bdc770 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bdc738:     	mov	x9, x22
100bdc73c:     	lsr	w8, w22, #2
100bdc740:     	ldr	x1, [x0, #0x40]
100bdc744:     	cmp	x1, x8
100bdc748:     	b.ls	0x100bdc8c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x738>
100bdc74c:     	ldr	x12, [x0, #0x38]
100bdc750:     	add	x8, x12, x8, lsl #4
100bdc754:     	ldr	w12, [x8]
100bdc758:     	mov	x1, x9
100bdc75c:     	cmp	x21, x12
100bdc760:     	b.ne	0x100bdc770 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bdc764:     	ldr	w8, [x8, #0x8]
100bdc768:     	and	w9, w9, #0x1
100bdc76c:     	eor	w1, w8, w9
100bdc770:     	mov	x2, x11
100bdc774:     	tbz	w11, #0x1, 0x100bdc7ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bdc778:     	lsr	w8, w11, #2
100bdc77c:     	ldr	x9, [x0, #0x40]
100bdc780:     	cmp	x9, x8
100bdc784:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc788:     	ldr	x9, [x0, #0x38]
100bdc78c:     	add	x8, x9, x8, lsl #4
100bdc790:     	ldr	w9, [x8]
100bdc794:     	mov	x2, x11
100bdc798:     	cmp	x21, x9
100bdc79c:     	b.ne	0x100bdc7ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bdc7a0:     	ldr	w8, [x8, #0x8]
100bdc7a4:     	and	w9, w11, #0x1
100bdc7a8:     	eor	w2, w8, w9
100bdc7ac:     	mov	x3, x10
100bdc7b0:     	tbz	w10, #0x1, 0x100bdc7e8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bdc7b4:     	lsr	w8, w10, #2
100bdc7b8:     	ldr	x9, [x0, #0x40]
100bdc7bc:     	cmp	x9, x8
100bdc7c0:     	b.ls	0x100bdc87c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bdc7c4:     	ldr	x9, [x0, #0x38]
100bdc7c8:     	add	x8, x9, x8, lsl #4
100bdc7cc:     	ldr	w9, [x8]
100bdc7d0:     	mov	x3, x10
100bdc7d4:     	cmp	x21, x9
100bdc7d8:     	b.ne	0x100bdc7e8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bdc7dc:     	ldr	w8, [x8, #0x8]
100bdc7e0:     	and	w9, w10, #0x1
100bdc7e4:     	eor	w3, w8, w9
100bdc7e8:     	mov	x4, x19
100bdc7ec:     	bl	0x100bdc190 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bdc7f0:     	mov	x3, x24
100bdc7f4:     	mov	x4, x19
100bdc7f8:     	orr	w20, w0, w20
100bdc7fc:     	mov	x2, x23
100bdc800:     	mov	x1, x22
100bdc804:     	stp	w1, w2, [sp, #0x24]
100bdc808:     	str	w3, [sp, #0x2c]
100bdc80c:     	add	x1, sp, #0x24
100bdc810:     	mov	x0, x4
100bdc814:     	mov	x2, x20
100bdc818:     	bl	0x100c2f0ac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bdc81c:     	mov	x0, x20
100bdc820:     	ldp	x29, x30, [sp, #0x80]
100bdc824:     	ldp	x20, x19, [sp, #0x70]
100bdc828:     	ldp	x22, x21, [sp, #0x60]
100bdc82c:     	ldp	x24, x23, [sp, #0x50]
100bdc830:     	ldp	x26, x25, [sp, #0x40]
100bdc834:     	ldp	x28, x27, [sp, #0x30]
100bdc838:     	add	sp, sp, #0x90
100bdc83c:     	ret
100bdc840:     	ldp	x15, x13, [sp]
100bdc844:     	bic	x10, x13, x15
100bdc848:     	tst	x10, x8
100bdc84c:     	csel	w12, wzr, w12, eq
100bdc850:     	and	x13, x15, x13
100bdc854:     	bics	xzr, x13, x8
100bdc858:     	csel	w11, wzr, w11, eq
100bdc85c:     	tst	x13, x8
100bdc860:     	csel	w13, wzr, w14, eq
100bdc864:     	bics	xzr, x10, x8
100bdc868:     	cinc	w8, w12, ne
100bdc86c:     	orr	w10, w11, w13
100bdc870:     	orr	w8, w8, w10
100bdc874:     	orr	w20, w8, w9
100bdc878:     	b	0x100bdc804 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bdc87c:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bdc880:     	add	x2, x2, #0x650
100bdc884:     	mov	x0, x8
100bdc888:     	mov	x1, x9
100bdc88c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bdc890:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bdc894:     	add	x2, x2, #0x8d8
100bdc898:     	mov	x0, x9
100bdc89c:     	mov	x1, x8
100bdc8a0:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bdc8a4:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bdc8a8:     	add	x2, x2, #0x650
100bdc8ac:     	mov	x0, x9
100bdc8b0:     	mov	x1, x10
100bdc8b4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bdc8b8:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bdc8bc:     	add	x2, x2, #0x650
100bdc8c0:     	mov	x0, x10
100bdc8c4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bdc8c8:     	adrp	x2, 0x101509000 <dyld_stub_binder+0x101509000>
100bdc8cc:     	add	x2, x2, #0x650
100bdc8d0:     	mov	x0, x8
100bdc8d4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
