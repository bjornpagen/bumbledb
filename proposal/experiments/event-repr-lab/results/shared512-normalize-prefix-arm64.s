
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>:
1012f2368:     	stp	x24, x23, [sp, #-0x40]!
1012f236c:     	stp	x22, x21, [sp, #0x10]
1012f2370:     	stp	x20, x19, [sp, #0x20]
1012f2374:     	stp	x29, x30, [sp, #0x30]
1012f2378:     	add	x29, sp, #0x30
1012f237c:     	mov	x19, x1
1012f2380:     	mov	x20, x0
1012f2384:     	ldr	x8, [x0, #0x40]
1012f2388:     	lsr	w0, w1, #1
1012f238c:     	cmn	x8, #0x1
1012f2390:     	b.eq	0x1012f23bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x54>
1012f2394:     	ldr	x1, [x20, #0x50]
1012f2398:     	cmp	x1, x0
1012f239c:     	b.ls	0x1012f25d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x26c>
1012f23a0:     	ldr	x9, [x20, #0x48]
1012f23a4:     	add	x9, x9, x0, lsl #4
1012f23a8:     	ldr	x9, [x9]
1012f23ac:     	ldr	x10, [x20, #0x180]
1012f23b0:     	bics	xzr, x9, x10
1012f23b4:     	b.ne	0x1012f23e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x7c>
1012f23b8:     	b	0x1012f25b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
1012f23bc:     	ldr	x1, [x20, #0x58]
1012f23c0:     	cmp	x1, x0
1012f23c4:     	b.ls	0x1012f25f0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x288>
1012f23c8:     	ldr	x1, [x20, #0x50]
1012f23cc:     	add	x9, x1, x0, lsl #5
1012f23d0:     	add	x9, x9, #0x18
1012f23d4:     	ldr	x9, [x9]
1012f23d8:     	ldr	x10, [x20, #0x180]
1012f23dc:     	bics	xzr, x9, x10
1012f23e0:     	b.eq	0x1012f25b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
1012f23e4:     	and	w9, w19, #0xfffffffe
1012f23e8:     	ldr	x10, [x20, #0x148]
1012f23ec:     	cbz	x10, 0x1012f2490 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x128>
1012f23f0:     	mov	x10, #0x0               ; =0
1012f23f4:     	mov	x11, #0xa9c5            ; =43461
1012f23f8:     	movk	x11, #0x2e62, lsl #16
1012f23fc:     	movk	x11, #0x7aea, lsl #32
1012f2400:     	movk	x11, #0xf135, lsl #48
1012f2404:     	mul	x11, x9, x11
1012f2408:     	ror	x13, x11, #0x2c
1012f240c:     	lsr	x14, x13, #57
1012f2410:     	ldp	x12, x11, [x20, #0x130]
1012f2414:     	dup.8b	v0, w14
1012f2418:     	movi.2d	v1, #0xffffffffffffffff
1012f241c:     	and	x13, x13, x11
1012f2420:     	ldr	d2, [x12, x13]
1012f2424:     	cmeq.8b	v3, v2, v0
1012f2428:     	fmov	x14, d3
1012f242c:     	ands	x14, x14, #0x8080808080808080
1012f2430:     	b.eq	0x1012f2460 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xf8>
1012f2434:     	rbit	x15, x14
1012f2438:     	clz	x15, x15
1012f243c:     	add	x15, x13, x15, lsr #3
1012f2440:     	and	x15, x15, x11
1012f2444:     	sub	x15, x12, x15, lsl #3
1012f2448:     	ldur	w16, [x15, #-0x8]
1012f244c:     	cmp	w9, w16
1012f2450:     	b.eq	0x1012f24ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x144>
1012f2454:     	sub	x15, x14, #0x2
1012f2458:     	ands	x14, x15, x14
1012f245c:     	b.ne	0x1012f2434 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xcc>
1012f2460:     	cmeq.8b	v2, v2, v1
1012f2464:     	fmov	x14, d2
1012f2468:     	cbnz	x14, 0x1012f2490 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x128>
1012f246c:     	add	x10, x10, #0x8
1012f2470:     	add	x13, x13, x10
1012f2474:     	and	x13, x13, x11
1012f2478:     	ldr	d2, [x12, x13]
1012f247c:     	cmeq.8b	v3, v2, v0
1012f2480:     	fmov	x14, d3
1012f2484:     	ands	x14, x14, #0x8080808080808080
1012f2488:     	b.ne	0x1012f2434 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xcc>
1012f248c:     	b	0x1012f2460 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xf8>
1012f2490:     	cmn	x8, #0x1
1012f2494:     	b.eq	0x1012f24bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x154>
1012f2498:     	cmp	x1, x0
1012f249c:     	b.ls	0x1012f25d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x26c>
1012f24a0:     	ldr	x8, [x20, #0x48]
1012f24a4:     	add	x8, x8, x0, lsl #4
1012f24a8:     	b	0x1012f24d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x168>
1012f24ac:     	ldur	w8, [x15, #-0x4]
1012f24b0:     	and	w9, w19, #0x1
1012f24b4:     	eor	w19, w8, w9
1012f24b8:     	b	0x1012f25b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
1012f24bc:     	ldr	x8, [x20, #0x58]
1012f24c0:     	cmp	x8, x0
1012f24c4:     	b.ls	0x1012f25fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x294>
1012f24c8:     	add	x8, x1, x0, lsl #5
1012f24cc:     	add	x8, x8, #0x18
1012f24d0:     	ldr	x8, [x8]
1012f24d4:     	ldp	x9, x10, [x20, #0x100]
1012f24d8:     	lsl	x10, x10, #2
1012f24dc:     	cbz	x10, 0x1012f25c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x260>
1012f24e0:     	ldr	w21, [x9], #0x4
1012f24e4:     	lsr	x11, x8, x21
1012f24e8:     	sub	x10, x10, #0x4
1012f24ec:     	tbz	w11, #0x0, 0x1012f24dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x174>
1012f24f0:     	and	w1, w19, #0xfffffffe
1012f24f4:     	mov	x0, x20
1012f24f8:     	mov	x2, x21
1012f24fc:     	mov	w3, #0x0                ; =0
1012f2500:     	bl	0x10130517c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
1012f2504:     	mov	x22, x0
1012f2508:     	and	w1, w19, #0xfffffffe
1012f250c:     	mov	x0, x20
1012f2510:     	mov	x2, x21
1012f2514:     	mov	w3, #0x1                ; =1
1012f2518:     	bl	0x10130517c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
1012f251c:     	mov	x24, x0
1012f2520:     	mov	x0, x20
1012f2524:     	mov	x1, x22
1012f2528:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
1012f252c:     	mov	x23, x0
1012f2530:     	mov	x0, x20
1012f2534:     	mov	x1, x24
1012f2538:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
1012f253c:     	ldr	x1, [x20, #0x120]
1012f2540:     	cmp	x1, x21
1012f2544:     	b.ls	0x1012f25e0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x278>
1012f2548:     	mov	x22, x0
1012f254c:     	ldr	x8, [x20, #0x118]
1012f2550:     	ldr	w21, [x8, x21, lsl #2]
1012f2554:     	mov	x0, x20
1012f2558:     	mov	w1, #0x4                ; =4
1012f255c:     	mov	x2, x23
1012f2560:     	mov	x3, x21
1012f2564:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012f2568:     	mov	x23, x0
1012f256c:     	mov	x0, x20
1012f2570:     	mov	w1, #0x8                ; =8
1012f2574:     	mov	x2, x22
1012f2578:     	mov	x3, x21
1012f257c:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012f2580:     	mov	x3, x0
1012f2584:     	mov	x0, x20
1012f2588:     	mov	w1, #0xe                ; =14
1012f258c:     	mov	x2, x23
1012f2590:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012f2594:     	mov	x21, x0
1012f2598:     	add	x0, x20, #0x130
1012f259c:     	and	w1, w19, #0xfffffffe
1012f25a0:     	mov	x2, x21
1012f25a4:     	bl	0x1013b46ac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapmmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
1012f25a8:     	and	w8, w19, #0x1
1012f25ac:     	eor	w19, w21, w8
1012f25b0:     	mov	x0, x19
1012f25b4:     	ldp	x29, x30, [sp, #0x30]
1012f25b8:     	ldp	x20, x19, [sp, #0x20]
1012f25bc:     	ldp	x22, x21, [sp, #0x10]
1012f25c0:     	ldp	x24, x23, [sp], #0x40
1012f25c4:     	ret
1012f25c8:     	adrp	x0, 0x101d2a000 <dyld_stub_binder+0x101d2a000>
1012f25cc:     	add	x0, x0, #0xe08
1012f25d0:     	bl	0x101a662f4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1012f25d4:     	adrp	x2, 0x101d2e000 <dyld_stub_binder+0x101d2e000>
1012f25d8:     	add	x2, x2, #0xa48
1012f25dc:     	bl	0x101a6625c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012f25e0:     	adrp	x2, 0x101d2a000 <dyld_stub_binder+0x101d2a000>
1012f25e4:     	add	x2, x2, #0xe20
1012f25e8:     	mov	x0, x21
1012f25ec:     	bl	0x101a6625c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012f25f0:     	adrp	x2, 0x101d2e000 <dyld_stub_binder+0x101d2e000>
1012f25f4:     	add	x2, x2, #0xa30
1012f25f8:     	bl	0x101a6625c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012f25fc:     	adrp	x2, 0x101d2e000 <dyld_stub_binder+0x101d2e000>
1012f2600:     	add	x2, x2, #0xa30
1012f2604:     	mov	x1, x8
1012f2608:     	bl	0x101a6625c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
