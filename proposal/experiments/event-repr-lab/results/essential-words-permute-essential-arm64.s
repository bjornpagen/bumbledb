
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba2240 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>:
100ba2240:     	sub	sp, sp, #0x80
100ba2244:     	stp	x28, x27, [sp, #0x20]
100ba2248:     	stp	x26, x25, [sp, #0x30]
100ba224c:     	stp	x24, x23, [sp, #0x40]
100ba2250:     	stp	x22, x21, [sp, #0x50]
100ba2254:     	stp	x20, x19, [sp, #0x60]
100ba2258:     	stp	x29, x30, [sp, #0x70]
100ba225c:     	add	x29, sp, #0x70
100ba2260:     	cmp	x3, #0x15
100ba2264:     	b.hs	0x100ba2368 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x128>
100ba2268:     	mov	x19, x3
100ba226c:     	mov	x20, x1
100ba2270:     	mov	w8, #0x1                ; =1
100ba2274:     	lsl	x8, x8, x3
100ba2278:     	lsr	x8, x8, #6
100ba227c:     	cmp	x3, #0x6
100ba2280:     	cinc	x8, x8, lo
100ba2284:     	stp	x1, x8, [sp, #0x10]
100ba2288:     	cmp	x1, x8
100ba228c:     	b.ne	0x100ba2380 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x140>
100ba2290:     	cbz	x19, 0x100ba22f8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xb8>
100ba2294:     	mov	x21, x2
100ba2298:     	mov	x23, x0
100ba229c:     	mov	x8, #0x0                ; =0
100ba22a0:     	lsl	x24, x19, #2
100ba22a4:     	add	x25, x2, x24
100ba22a8:     	mov	w9, #0x1                ; =1
100ba22ac:     	mov	x10, x24
100ba22b0:     	mov	x11, x2
100ba22b4:     	ldr	w12, [x11], #0x4
100ba22b8:     	cmp	w12, w19
100ba22bc:     	b.hs	0x100ba2350 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100ba22c0:     	lsr	x13, x8, x12
100ba22c4:     	tbnz	w13, #0x0, 0x100ba2350 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100ba22c8:     	lsl	x12, x9, x12
100ba22cc:     	orr	x8, x12, x8
100ba22d0:     	subs	x10, x10, #0x4
100ba22d4:     	b.ne	0x100ba22b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x74>
100ba22d8:     	mov	x0, x24
100ba22dc:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100ba22e0:     	cbz	x0, 0x100ba239c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x15c>
100ba22e4:     	mov	x22, x0
100ba22e8:     	cmp	x19, #0x8
100ba22ec:     	b.hs	0x100ba2318 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xd8>
100ba22f0:     	mov	x8, #0x0                ; =0
100ba22f4:     	b	0x100ba23a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100ba22f8:     	ldp	x29, x30, [sp, #0x70]
100ba22fc:     	ldp	x20, x19, [sp, #0x60]
100ba2300:     	ldp	x22, x21, [sp, #0x50]
100ba2304:     	ldp	x24, x23, [sp, #0x40]
100ba2308:     	ldp	x26, x25, [sp, #0x30]
100ba230c:     	ldp	x28, x27, [sp, #0x20]
100ba2310:     	add	sp, sp, #0x80
100ba2314:     	ret
100ba2318:     	and	x8, x19, #0x18
100ba231c:     	adrp	x9, 0x101195000 <GCC_except_table8962+0x60>
100ba2320:     	ldr	q0, [x9, #0x520]
100ba2324:     	adrp	x9, 0x101195000 <GCC_except_table8962+0x60>
100ba2328:     	ldr	q1, [x9, #0x620]
100ba232c:     	stp	q0, q1, [x22]
100ba2330:     	cmp	x8, #0x8
100ba2334:     	b.eq	0x100ba23b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100ba2338:     	adrp	x9, 0x101195000 <GCC_except_table8962+0x60>
100ba233c:     	ldr	q0, [x9, #0xcc0]
100ba2340:     	adrp	x9, 0x101195000 <GCC_except_table8962+0x60>
100ba2344:     	ldr	q1, [x9, #0xcd0]
100ba2348:     	stp	q0, q1, [x22, #0x20]
100ba234c:     	b	0x100ba23b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100ba2350:     	adrp	x0, 0x1011c2000 <dyld_stub_binder+0x1011c2000>
100ba2354:     	add	x0, x0, #0x1a4
100ba2358:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba235c:     	add	x2, x2, #0x7f0
100ba2360:     	mov	w1, #0x49               ; =73
100ba2364:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba2368:     	adrp	x0, 0x1011c2000 <dyld_stub_binder+0x1011c2000>
100ba236c:     	add	x0, x0, #0x17a
100ba2370:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2374:     	add	x2, x2, #0x7c0
100ba2378:     	mov	w1, #0x2a               ; =42
100ba237c:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba2380:     	adrp	x5, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2384:     	add	x5, x5, #0x7d8
100ba2388:     	add	x1, sp, #0x10
100ba238c:     	add	x2, sp, #0x18
100ba2390:     	mov	w0, #0x0                ; =0
100ba2394:     	mov	x3, #0x0                ; =0
100ba2398:     	bl	0x1011015b0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100ba239c:     	mov	w0, #0x4                ; =4
100ba23a0:     	mov	x1, x24
100ba23a4:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba23a8:     	str	w8, [x22, x8, lsl #2]
100ba23ac:     	add	x8, x8, #0x1
100ba23b0:     	cmp	x19, x8
100ba23b4:     	b.ne	0x100ba23a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100ba23b8:     	mov	w10, #0x0               ; =0
100ba23bc:     	lsl	x9, x20, #3
100ba23c0:     	add	x8, x23, x9
100ba23c4:     	sub	x9, x9, #0x8
100ba23c8:     	lsr	x11, x9, #3
100ba23cc:     	add	x11, x11, #0x1
100ba23d0:     	and	x12, x11, #0x3ffffffffffffff8
100ba23d4:     	add	x13, x23, x12, lsl #3
100ba23d8:     	adrp	x12, 0x101377000 <dyld_stub_binder+0x101377000>
100ba23dc:     	add	x12, x12, #0x820
100ba23e0:     	stp	x12, x13, [sp]
100ba23e4:     	adrp	x14, 0x10119f000 <dyld_stub_binder+0x10119f000>
100ba23e8:     	add	x14, x14, #0xdd0
100ba23ec:     	mov	w16, #0x1               ; =1
100ba23f0:     	b	0x100ba23fc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1bc>
100ba23f4:     	cmp	x21, x25
100ba23f8:     	b.eq	0x100ba2718 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4d8>
100ba23fc:     	mov	x17, #0x0               ; =0
100ba2400:     	mov	x12, x10
100ba2404:     	ldr	w0, [x21], #0x4
100ba2408:     	add	w10, w10, #0x1
100ba240c:     	mov	x13, x24
100ba2410:     	ldr	w1, [x22, x17, lsl #2]
100ba2414:     	cmp	w1, w12
100ba2418:     	b.eq	0x100ba242c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1ec>
100ba241c:     	add	x17, x17, #0x1
100ba2420:     	subs	x13, x13, #0x4
100ba2424:     	b.ne	0x100ba2410 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1d0>
100ba2428:     	b	0x100ba2774 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x534>
100ba242c:     	cmp	w0, w17
100ba2430:     	b.eq	0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba2434:     	cmp	w0, w17
100ba2438:     	csel	w13, w0, w17, lo
100ba243c:     	csel	w1, w0, w17, hi
100ba2440:     	cmp	x19, x0
100ba2444:     	b.ls	0x100ba2708 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4c8>
100ba2448:     	ldr	w12, [x22, x17, lsl #2]
100ba244c:     	ldr	w2, [x22, x0, lsl #2]
100ba2450:     	str	w2, [x22, x17, lsl #2]
100ba2454:     	str	w12, [x22, x0, lsl #2]
100ba2458:     	cmp	w1, #0x6
100ba245c:     	b.hs	0x100ba2548 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x308>
100ba2460:     	cbz	x20, 0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba2464:     	ldr	x12, [x14, w13, uxtw #3]
100ba2468:     	ldr	x17, [x14, w1, uxtw #3]
100ba246c:     	bic	x17, x17, x12
100ba2470:     	mov	w12, #-0x1              ; =-1
100ba2474:     	lsl	w12, w12, w13
100ba2478:     	lsl	w13, w16, w1
100ba247c:     	add	w12, w12, w13
100ba2480:     	and	w0, w12, #0x3f
100ba2484:     	mov	x12, x23
100ba2488:     	cmp	x9, #0x38
100ba248c:     	b.lo	0x100ba251c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100ba2490:     	dup.2d	v0, x0
100ba2494:     	dup.2d	v1, x17
100ba2498:     	neg.2d	v2, v0
100ba249c:     	add	x13, x23, #0x20
100ba24a0:     	and	x1, x11, #0x3ffffffffffffff8
100ba24a4:     	ldp	q3, q4, [x13, #-0x20]
100ba24a8:     	ldp	q5, q6, [x13]
100ba24ac:     	ushl.2d	v7, v3, v2
100ba24b0:     	ushl.2d	v16, v4, v2
100ba24b4:     	ushl.2d	v17, v5, v2
100ba24b8:     	ushl.2d	v18, v6, v2
100ba24bc:     	eor.16b	v7, v7, v3
100ba24c0:     	eor.16b	v16, v16, v4
100ba24c4:     	eor.16b	v17, v17, v5
100ba24c8:     	eor.16b	v18, v18, v6
100ba24cc:     	and.16b	v7, v1, v7
100ba24d0:     	and.16b	v16, v1, v16
100ba24d4:     	and.16b	v17, v1, v17
100ba24d8:     	and.16b	v18, v1, v18
100ba24dc:     	ushl.2d	v19, v7, v0
100ba24e0:     	ushl.2d	v20, v16, v0
100ba24e4:     	ushl.2d	v21, v17, v0
100ba24e8:     	ushl.2d	v22, v18, v0
100ba24ec:     	eor3.16b	v3, v3, v19, v7
100ba24f0:     	eor3.16b	v4, v4, v20, v16
100ba24f4:     	eor3.16b	v5, v5, v21, v17
100ba24f8:     	stp	q3, q4, [x13, #-0x20]
100ba24fc:     	eor3.16b	v3, v6, v22, v18
100ba2500:     	stp	q5, q3, [x13], #0x40
100ba2504:     	subs	x1, x1, #0x8
100ba2508:     	b.ne	0x100ba24a4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x264>
100ba250c:     	ldr	x12, [sp, #0x8]
100ba2510:     	and	x13, x11, #0x3ffffffffffffff8
100ba2514:     	cmp	x11, x13
100ba2518:     	b.eq	0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba251c:     	ldr	x13, [x12]
100ba2520:     	lsr	x15, x13, x0
100ba2524:     	eor	x15, x15, x13
100ba2528:     	and	x15, x17, x15
100ba252c:     	lsl	x1, x15, x0
100ba2530:     	eor	x13, x13, x15
100ba2534:     	eor	x13, x13, x1
100ba2538:     	str	x13, [x12], #0x8
100ba253c:     	cmp	x12, x8
100ba2540:     	b.ne	0x100ba251c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100ba2544:     	b	0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba2548:     	cmp	w13, #0x6
100ba254c:     	b.hs	0x100ba26a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x460>
100ba2550:     	add	w17, w1, #0x3a
100ba2554:     	and	w12, w17, #0x3f
100ba2558:     	cmp	w12, #0x3f
100ba255c:     	b.eq	0x100ba2758 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x518>
100ba2560:     	cbz	x20, 0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba2564:     	lsl	x0, x16, x17
100ba2568:     	ldr	x4, [x14, w13, uxtw #3]
100ba256c:     	lsl	w5, w16, w13
100ba2570:     	mov	w13, #0x2               ; =2
100ba2574:     	lsl	x6, x13, x12
100ba2578:     	mov	w13, #0x8               ; =8
100ba257c:     	lsl	x7, x13, x12
100ba2580:     	lsr	x26, x7, #3
100ba2584:     	dup.2d	v0, x5
100ba2588:     	dup.2d	v1, x4
100ba258c:     	lsl	x27, x0, #3
100ba2590:     	neg.2d	v2, v0
100ba2594:     	mov	x28, x20
100ba2598:     	mov	x30, x23
100ba259c:     	b	0x100ba25ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x36c>
100ba25a0:     	add	x30, x30, x17, lsl #3
100ba25a4:     	sub	x28, x28, x17
100ba25a8:     	cbz	x28, 0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba25ac:     	cmp	x6, x28
100ba25b0:     	csel	x17, x6, x28, lo
100ba25b4:     	subs	x12, x17, x0
100ba25b8:     	b.lo	0x100ba273c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4fc>
100ba25bc:     	cmp	x12, x26
100ba25c0:     	csel	x13, x12, x26, lo
100ba25c4:     	cmp	x17, x0
100ba25c8:     	ccmp	x30, #0x0, #0x4, ne
100ba25cc:     	b.eq	0x100ba25a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba25d0:     	cmp	x13, #0x4
100ba25d4:     	b.lo	0x100ba265c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100ba25d8:     	add	x12, x30, x0, lsl #3
100ba25dc:     	add	x1, x30, x13, lsl #3
100ba25e0:     	add	x2, x1, x7
100ba25e4:     	cmp	x30, x2
100ba25e8:     	ccmp	x12, x1, #0x2, lo
100ba25ec:     	b.lo	0x100ba265c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100ba25f0:     	and	x2, x13, #0x1ffffffffffffffc
100ba25f4:     	add	x1, x30, #0x10
100ba25f8:     	add	x3, x1, x27
100ba25fc:     	and	x12, x13, #0x1ffffffffffffffc
100ba2600:     	ldp	q3, q4, [x1, #-0x10]
100ba2604:     	ushl.2d	v5, v3, v2
100ba2608:     	ushl.2d	v6, v4, v2
100ba260c:     	ldp	q7, q16, [x3, #-0x10]
100ba2610:     	eor.16b	v5, v5, v7
100ba2614:     	eor.16b	v6, v6, v16
100ba2618:     	and.16b	v5, v5, v1
100ba261c:     	and.16b	v6, v6, v1
100ba2620:     	ushl.2d	v17, v5, v0
100ba2624:     	ushl.2d	v18, v6, v0
100ba2628:     	eor.16b	v3, v17, v3
100ba262c:     	eor.16b	v4, v18, v4
100ba2630:     	stp	q3, q4, [x1, #-0x10]
100ba2634:     	eor.16b	v3, v5, v7
100ba2638:     	eor.16b	v4, v6, v16
100ba263c:     	stp	q3, q4, [x3, #-0x10]
100ba2640:     	add	x3, x3, #0x20
100ba2644:     	add	x1, x1, #0x20
100ba2648:     	subs	x12, x12, #0x4
100ba264c:     	b.ne	0x100ba2600 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x3c0>
100ba2650:     	cmp	x13, x2
100ba2654:     	b.eq	0x100ba25a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba2658:     	b	0x100ba2660 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x420>
100ba265c:     	mov	x2, #0x0                ; =0
100ba2660:     	sub	x12, x13, x2
100ba2664:     	add	x13, x30, x2, lsl #3
100ba2668:     	ldr	x1, [x13]
100ba266c:     	lsr	x2, x1, x5
100ba2670:     	ldr	x3, [x13, x27]
100ba2674:     	eor	x2, x2, x3
100ba2678:     	and	x2, x2, x4
100ba267c:     	lsl	x15, x2, x5
100ba2680:     	eor	x15, x15, x1
100ba2684:     	str	x15, [x13]
100ba2688:     	eor	x15, x2, x3
100ba268c:     	str	x15, [x13, x27]
100ba2690:     	add	x13, x13, #0x8
100ba2694:     	subs	x12, x12, #0x1
100ba2698:     	b.ne	0x100ba2668 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x428>
100ba269c:     	b	0x100ba25a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba26a0:     	cbz	x20, 0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba26a4:     	mov	x17, #0x0               ; =0
100ba26a8:     	add	w12, w13, #0x3a
100ba26ac:     	lsl	x12, x16, x12
100ba26b0:     	add	w13, w1, #0x3a
100ba26b4:     	lsl	x13, x16, x13
100ba26b8:     	eor	x1, x13, x12
100ba26bc:     	b	0x100ba26dc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x49c>
100ba26c0:     	ldr	x2, [x23, x17, lsl #3]
100ba26c4:     	ldr	x3, [x23, x0, lsl #3]
100ba26c8:     	str	x3, [x23, x17, lsl #3]
100ba26cc:     	str	x2, [x23, x0, lsl #3]
100ba26d0:     	add	x17, x17, #0x1
100ba26d4:     	cmp	x20, x17
100ba26d8:     	b.eq	0x100ba23f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba26dc:     	tst	x17, x12
100ba26e0:     	b.eq	0x100ba26d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100ba26e4:     	and	x0, x17, x13
100ba26e8:     	cbnz	x0, 0x100ba26d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100ba26ec:     	eor	x0, x1, x17
100ba26f0:     	cmp	x0, x20
100ba26f4:     	b.lo	0x100ba26c0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x480>
100ba26f8:     	mov	x19, x20
100ba26fc:     	adrp	x8, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2700:     	add	x8, x8, #0x838
100ba2704:     	str	x8, [sp]
100ba2708:     	mov	x1, x19
100ba270c:     	ldr	x2, [sp]
100ba2710:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ba2714:     	b	0x100ba2780 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba2718:     	mov	x0, x22
100ba271c:     	ldp	x29, x30, [sp, #0x70]
100ba2720:     	ldp	x20, x19, [sp, #0x60]
100ba2724:     	ldp	x22, x21, [sp, #0x50]
100ba2728:     	ldp	x24, x23, [sp, #0x40]
100ba272c:     	ldp	x26, x25, [sp, #0x30]
100ba2730:     	ldp	x28, x27, [sp, #0x20]
100ba2734:     	add	sp, sp, #0x80
100ba2738:     	b	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100ba273c:     	adrp	x0, 0x10125b000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x1590>
100ba2740:     	add	x0, x0, #0x3d3
100ba2744:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2748:     	add	x2, x2, #0x868
100ba274c:     	mov	w1, #0x13               ; =19
100ba2750:     	bl	0x101101574 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba2754:     	b	0x100ba2780 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba2758:     	adrp	x0, 0x10119e000 <dyld_stub_binder+0x10119e000>
100ba275c:     	add	x0, x0, #0x791
100ba2760:     	adrp	x2, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2764:     	add	x2, x2, #0x850
100ba2768:     	mov	w1, #0x37               ; =55
100ba276c:     	bl	0x101101574 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba2770:     	b	0x100ba2780 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba2774:     	adrp	x0, 0x101377000 <dyld_stub_binder+0x101377000>
100ba2778:     	add	x0, x0, #0x808
100ba277c:     	bl	0x101101774 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100ba2780:     	brk	#0x1
100ba2784:     	mov	x19, x0
100ba2788:     	mov	x0, x22
100ba278c:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100ba2790:     	mov	x0, x19
100ba2794:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
