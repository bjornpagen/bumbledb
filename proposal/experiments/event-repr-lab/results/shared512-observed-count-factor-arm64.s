
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012f0240 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_>:
1012f0240:     	sub	sp, sp, #0x80
1012f0244:     	stp	x28, x27, [sp, #0x20]
1012f0248:     	stp	x26, x25, [sp, #0x30]
1012f024c:     	stp	x24, x23, [sp, #0x40]
1012f0250:     	stp	x22, x21, [sp, #0x50]
1012f0254:     	stp	x20, x19, [sp, #0x60]
1012f0258:     	stp	x29, x30, [sp, #0x70]
1012f025c:     	add	x29, sp, #0x70
1012f0260:     	str	w1, [sp, #0xc]
1012f0264:     	mov	x20, x0
1012f0268:     	ldrb	w8, [x0, #0x1a5]
1012f026c:     	ldr	w3, [x0, #0x1a0]
1012f0270:     	cmp	w3, #0x1
1012f0274:     	csel	w8, wzr, w8, eq
1012f0278:     	tbz	w8, #0x0, 0x1012f0320 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0xe0>
1012f027c:     	ldr	x8, [x20, #0x138]
1012f0280:     	cbz	x8, 0x1012f0320 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0xe0>
1012f0284:     	ldr	w9, [x20, #0x140]
1012f0288:     	cbz	w9, 0x1012f048c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x24c>
1012f028c:     	ldr	w9, [x20, #0x144]
1012f0290:     	ldr	x10, [x20, #0x40]
1012f0294:     	ldr	w11, [sp, #0xc]
1012f0298:     	lsr	w0, w11, #1
1012f029c:     	cmn	x10, #0x1
1012f02a0:     	b.eq	0x1012f02bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x7c>
1012f02a4:     	ldr	x1, [x20, #0x50]
1012f02a8:     	cmp	x1, x0
1012f02ac:     	b.ls	0x1012f04a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x264>
1012f02b0:     	ldr	x10, [x20, #0x48]
1012f02b4:     	add	x10, x10, x0, lsl #4
1012f02b8:     	b	0x1012f02d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x94>
1012f02bc:     	ldr	x1, [x20, #0x58]
1012f02c0:     	cmp	x1, x0
1012f02c4:     	b.ls	0x1012f04dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x29c>
1012f02c8:     	ldr	x10, [x20, #0x50]
1012f02cc:     	add	x10, x10, x0, lsl #5
1012f02d0:     	add	x10, x10, #0x18
1012f02d4:     	and	x11, x9, #0x3f
1012f02d8:     	mov	x12, #-0x1              ; =-1
1012f02dc:     	lsl	x12, x12, x9
1012f02e0:     	mvn	x13, x12
1012f02e4:     	ldr	x10, [x10]
1012f02e8:     	lsl	x11, x13, x11
1012f02ec:     	tst	x10, x11
1012f02f0:     	mov	w11, #0x2               ; =2
1012f02f4:     	csel	x11, xzr, x11, eq
1012f02f8:     	bics	xzr, x10, x12
1012f02fc:     	cinc	x11, x11, ne
1012f0300:     	ubfiz	x12, x9, #1, #5
1012f0304:     	lsl	x12, x13, x12
1012f0308:     	tst	x10, x12
1012f030c:     	mov	w10, #0x4               ; =4
1012f0310:     	csel	x10, xzr, x10, eq
1012f0314:     	orr	x11, x11, x10
1012f0318:     	cmp	x11, #0x7
1012f031c:     	b.ne	0x1012f0350 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x110>
1012f0320:     	ldp	x1, x2, [x20, #0x100]
1012f0324:     	ldrb	w5, [x20, #0x1a4]
1012f0328:     	mov	x0, x20
1012f032c:     	ldr	w4, [sp, #0xc]
1012f0330:     	ldp	x29, x30, [sp, #0x70]
1012f0334:     	ldp	x20, x19, [sp, #0x60]
1012f0338:     	ldp	x22, x21, [sp, #0x50]
1012f033c:     	ldp	x24, x23, [sp, #0x40]
1012f0340:     	ldp	x26, x25, [sp, #0x30]
1012f0344:     	ldp	x28, x27, [sp, #0x20]
1012f0348:     	add	sp, sp, #0x80
1012f034c:     	b	0x100c53640 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5countKm9_EB6_>
1012f0350:     	mov	x21, #0x0               ; =0
1012f0354:     	mov	x26, #0x0               ; =0
1012f0358:     	fmov	d0, x11
1012f035c:     	cnt.8b	v0, v0
1012f0360:     	fmov	w10, s0
1012f0364:     	eor	w10, w10, #0x3
1012f0368:     	str	w10, [sp, #0x8]
1012f036c:     	mul	w9, w10, w9
1012f0370:     	and	w10, w9, #0x3f
1012f0374:     	str	x10, [sp]
1012f0378:     	ldr	x25, [x20, #0x130]
1012f037c:     	mov	w10, #0x28              ; =40
1012f0380:     	madd	x19, x8, x10, x25
1012f0384:     	ldp	x22, x23, [x20, #0x100]
1012f0388:     	mov	x8, #-0x1               ; =-1
1012f038c:     	lsl	x28, x8, x9
1012f0390:     	ldrb	w24, [x20, #0x1a4]
1012f0394:     	mov	x27, x11
1012f0398:     	ldr	w3, [x25, x11, lsl #2]
1012f039c:     	mov	x0, x20
1012f03a0:     	mov	x1, x22
1012f03a4:     	mov	x2, x23
1012f03a8:     	ldr	w4, [sp, #0xc]
1012f03ac:     	mov	x5, x24
1012f03b0:     	bl	0x100c53640 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5countKm9_EB6_>
1012f03b4:     	bics	x8, x0, x28
1012f03b8:     	str	x8, [sp, #0x10]
1012f03bc:     	b.ne	0x1012f0464 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x224>
1012f03c0:     	mov	x9, #0x0                ; =0
1012f03c4:     	mov	x11, #0x0               ; =0
1012f03c8:     	add	x8, x25, #0x28
1012f03cc:     	ldr	x10, [sp]
1012f03d0:     	lsr	x10, x0, x10
1012f03d4:     	ldr	x12, [x25, #0x20]
1012f03d8:     	mov	w13, #0x1               ; =1
1012f03dc:     	ldr	w14, [sp, #0x8]
1012f03e0:     	b	0x1012f03fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x1bc>
1012f03e4:     	lsr	w14, w14, #1
1012f03e8:     	mul	x11, x12, x11
1012f03ec:     	umulh	x15, x12, x12
1012f03f0:     	add	x15, x15, x11
1012f03f4:     	add	x11, x15, x11
1012f03f8:     	mul	x12, x12, x12
1012f03fc:     	tbz	w14, #0x0, 0x1012f03e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x1a4>
1012f0400:     	umulh	x15, x13, x12
1012f0404:     	madd	x15, x13, x11, x15
1012f0408:     	madd	x9, x9, x12, x15
1012f040c:     	mul	x13, x13, x12
1012f0410:     	cmp	w14, #0x1
1012f0414:     	b.ne	0x1012f03e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x1a4>
1012f0418:     	umulh	x11, x13, x10
1012f041c:     	madd	x9, x9, x10, x11
1012f0420:     	mul	x10, x13, x10
1012f0424:     	adds	x21, x10, x21
1012f0428:     	adc	x26, x9, x26
1012f042c:     	mov	x25, x8
1012f0430:     	cmp	x8, x19
1012f0434:     	mov	x11, x27
1012f0438:     	b.ne	0x1012f0394 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x154>
1012f043c:     	cbnz	x26, 0x1012f04b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E14observed_countB6_+0x270>
1012f0440:     	mov	x0, x21
1012f0444:     	ldp	x29, x30, [sp, #0x70]
1012f0448:     	ldp	x20, x19, [sp, #0x60]
1012f044c:     	ldp	x22, x21, [sp, #0x50]
1012f0450:     	ldp	x24, x23, [sp, #0x40]
1012f0454:     	ldp	x26, x25, [sp, #0x30]
1012f0458:     	ldp	x28, x27, [sp, #0x20]
1012f045c:     	add	sp, sp, #0x80
1012f0460:     	ret
1012f0464:     	adrp	x2, 0x101bf5000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1e2f>
1012f0468:     	add	x2, x2, #0x3f0
1012f046c:     	adrp	x3, 0x101b53000 <dyld_stub_binder+0x101b53000>
1012f0470:     	add	x3, x3, #0xdee
1012f0474:     	adrp	x5, 0x101d32000 <dyld_stub_binder+0x101d32000>
1012f0478:     	add	x5, x5, #0xf28
1012f047c:     	add	x1, sp, #0x10
1012f0480:     	mov	w0, #0x0                ; =0
1012f0484:     	mov	w4, #0x37               ; =55
1012f0488:     	bl	0x101a6ed60 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012f048c:     	adrp	x0, 0x101b53000 <dyld_stub_binder+0x101b53000>
1012f0490:     	add	x0, x0, #0xdc9
1012f0494:     	adrp	x2, 0x101d32000 <dyld_stub_binder+0x101d32000>
1012f0498:     	add	x2, x2, #0xf10
1012f049c:     	mov	w1, #0x25               ; =37
1012f04a0:     	bl	0x101a6efa4 <__RNvNtCs4sDCw1iE1MS_4core6option13expect_failed>
1012f04a4:     	adrp	x2, 0x101d36000 <dyld_stub_binder+0x101d36000>
1012f04a8:     	add	x2, x2, #0xbf8
1012f04ac:     	bl	0x101a6ee5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012f04b0:     	mov	w8, #0x2                ; =2
1012f04b4:     	strb	w8, [sp, #0x1f]
1012f04b8:     	adrp	x0, 0x101b53000 <dyld_stub_binder+0x101b53000>
1012f04bc:     	add	x0, x0, #0xe09
1012f04c0:     	adrp	x3, 0x101d32000 <dyld_stub_binder+0x101d32000>
1012f04c4:     	add	x3, x3, #0xdf8
1012f04c8:     	adrp	x4, 0x101d32000 <dyld_stub_binder+0x101d32000>
1012f04cc:     	add	x4, x4, #0xf40
1012f04d0:     	add	x2, sp, #0x1f
1012f04d4:     	mov	w1, #0x2d               ; =45
1012f04d8:     	bl	0x101a6efd4 <__RNvNtCs4sDCw1iE1MS_4core6result13unwrap_failed>
1012f04dc:     	adrp	x2, 0x101d36000 <dyld_stub_binder+0x101d36000>
1012f04e0:     	add	x2, x2, #0xbe0
1012f04e4:     	bl	0x101a6ee5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
