
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001007b2240 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>:
1007b2240:     	stp	x28, x27, [sp, #-0x60]!
1007b2244:     	stp	x26, x25, [sp, #0x10]
1007b2248:     	stp	x24, x23, [sp, #0x20]
1007b224c:     	stp	x22, x21, [sp, #0x30]
1007b2250:     	stp	x20, x19, [sp, #0x40]
1007b2254:     	stp	x29, x30, [sp, #0x50]
1007b2258:     	add	x29, sp, #0x50
1007b225c:     	sub	sp, sp, #0x1c0
1007b2260:     	mov	x23, x2
1007b2264:     	mov	x21, x1
1007b2268:     	mov	x28, x0
1007b226c:     	ldrb	w8, [x0, #0x151]
1007b2270:     	str	x0, [sp, #0x88]
1007b2274:     	str	x2, [sp, #0x60]
1007b2278:     	cbz	w8, 0x1007b25a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x360>
1007b227c:     	mov	x27, #0x0               ; =0
1007b2280:     	b	0x1007b229c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5c>
1007b2284:     	ldr	w9, [x26, #0x14]
1007b2288:     	add	x27, x27, #0x18
1007b228c:     	stp	xzr, x20, [x26]
1007b2290:     	stp	w24, w9, [x26, #0x10]
1007b2294:     	cmp	x27, #0x30
1007b2298:     	b.eq	0x1007b25a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x360>
1007b229c:     	add	x26, x23, x27
1007b22a0:     	ldp	x19, x20, [x26]
1007b22a4:     	ldr	w24, [x26, #0x10]
1007b22a8:     	cbz	x19, 0x1007b2284 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x44>
1007b22ac:     	ldr	x8, [x28, #0x138]
1007b22b0:     	add	x8, x8, #0x1
1007b22b4:     	str	x8, [x28, #0x138]
1007b22b8:     	ldur	x8, [x21, #0x40]
1007b22bc:     	lsr	x0, x24, #1
1007b22c0:     	cmn	x8, #0x1
1007b22c4:     	str	w9, [sp, #0x70]
1007b22c8:     	b.eq	0x1007b22e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa4>
1007b22cc:     	ldr	x1, [x21, #0x50]
1007b22d0:     	cmp	x1, x0
1007b22d4:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b22d8:     	ldr	x8, [x21, #0x48]
1007b22dc:     	add	x8, x8, x0, lsl #4
1007b22e0:     	b	0x1007b22fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbc>
1007b22e4:     	ldr	x1, [x21, #0x58]
1007b22e8:     	cmp	x1, x0
1007b22ec:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b22f0:     	ldr	x8, [x21, #0x50]
1007b22f4:     	add	x8, x8, x0, lsl #5
1007b22f8:     	add	x8, x8, #0x18
1007b22fc:     	mov	x25, #0x0               ; =0
1007b2300:     	ldr	x8, [x8]
1007b2304:     	bic	x8, x8, x19
1007b2308:     	str	x8, [sp, #0x78]
1007b230c:     	mov	w8, #0x4                ; =4
1007b2310:     	stp	xzr, x8, [sp, #0xf0]
1007b2314:     	str	xzr, [sp, #0x100]
1007b2318:     	mov	w9, #0x4                ; =4
1007b231c:     	mov	w8, #0x4                ; =4
1007b2320:     	b	0x1007b2348 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x108>
1007b2324:     	rbit	x9, x19
1007b2328:     	clz	x9, x9
1007b232c:     	str	w9, [x8, x25, lsl #2]
1007b2330:     	add	x25, x25, #0x1
1007b2334:     	str	x25, [sp, #0x100]
1007b2338:     	sub	x10, x19, #0x1
1007b233c:     	add	x9, x23, #0x4
1007b2340:     	ands	x19, x10, x19
1007b2344:     	b.eq	0x1007b2368 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x128>
1007b2348:     	mov	x23, x9
1007b234c:     	ldr	x9, [sp, #0xf0]
1007b2350:     	cmp	x25, x9
1007b2354:     	b.ne	0x1007b2324 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe4>
1007b2358:     	add	x0, sp, #0xf0
1007b235c:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b2360:     	ldr	x8, [sp, #0xf8]
1007b2364:     	b	0x1007b2324 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe4>
1007b2368:     	ldp	x9, x8, [sp, #0xf0]
1007b236c:     	str	x9, [sp, #0x80]
1007b2370:     	str	x8, [sp, #0x68]
1007b2374:     	cbz	x25, 0x1007b24e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2a8>
1007b2378:     	ldr	x19, [x28, #0x140]
1007b237c:     	mov	x28, x8
1007b2380:     	b	0x1007b23b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x178>
1007b2384:     	tst	w22, #0x1
1007b2388:     	mov	w8, #0x8                ; =8
1007b238c:     	mov	w9, #0xc                ; =12
1007b2390:     	csel	x8, x9, x8, ne
1007b2394:     	add	x9, sp, #0xf0
1007b2398:     	ldr	w8, [x9, x8]
1007b239c:     	and	w9, w24, #0x1
1007b23a0:     	eor	w24, w8, w9
1007b23a4:     	add	x19, x19, #0x1
1007b23a8:     	ldr	x8, [sp, #0x88]
1007b23ac:     	str	x19, [x8, #0x140]
1007b23b0:     	subs	x23, x23, #0x4
1007b23b4:     	b.eq	0x1007b24e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2a8>
1007b23b8:     	ldr	w25, [x28], #0x4
1007b23bc:     	ldur	x8, [x21, #0x40]
1007b23c0:     	lsr	w0, w24, #1
1007b23c4:     	cmn	x8, #0x1
1007b23c8:     	b.eq	0x1007b23f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1b8>
1007b23cc:     	ldr	x1, [x21, #0x50]
1007b23d0:     	cmp	x1, x0
1007b23d4:     	b.ls	0x1007b357c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x133c>
1007b23d8:     	ldr	x9, [x21, #0x48]
1007b23dc:     	add	x9, x9, x0, lsl #4
1007b23e0:     	ldr	x10, [x9]
1007b23e4:     	mov	w9, #0x1                ; =1
1007b23e8:     	lsl	x9, x9, x25
1007b23ec:     	tst	x10, x9
1007b23f0:     	b.ne	0x1007b2420 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1e0>
1007b23f4:     	b	0x1007b23b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
1007b23f8:     	ldr	x1, [x21, #0x58]
1007b23fc:     	cmp	x1, x0
1007b2400:     	b.ls	0x1007b3598 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1358>
1007b2404:     	ldr	x1, [x21, #0x50]
1007b2408:     	add	x9, x1, x0, lsl #5
1007b240c:     	ldr	x10, [x9, #0x18]!
1007b2410:     	mov	w9, #0x1                ; =1
1007b2414:     	lsl	x9, x9, x25
1007b2418:     	tst	x10, x9
1007b241c:     	b.eq	0x1007b23b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
1007b2420:     	ldr	w10, [x21, #0xf0]
1007b2424:     	cmp	w25, w10
1007b2428:     	b.hs	0x1007b2770 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x530>
1007b242c:     	cmn	x8, #0x1
1007b2430:     	b.eq	0x1007b2454 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x214>
1007b2434:     	cmp	x1, x0
1007b2438:     	b.ls	0x1007b3588 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1348>
1007b243c:     	ldr	x8, [x21, #0x48]
1007b2440:     	add	x8, x8, x0, lsl #4
1007b2444:     	ldr	x8, [x8]
1007b2448:     	tst	x8, x9
1007b244c:     	b.ne	0x1007b2474 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x234>
1007b2450:     	b	0x1007b23a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1007b2454:     	ldr	x8, [x21, #0x58]
1007b2458:     	cmp	x8, x0
1007b245c:     	b.ls	0x1007b35c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1384>
1007b2460:     	add	x8, x1, x0, lsl #5
1007b2464:     	add	x8, x8, #0x18
1007b2468:     	ldr	x8, [x8]
1007b246c:     	tst	x8, x9
1007b2470:     	b.eq	0x1007b23a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1007b2474:     	and	x8, x25, #0x3f
1007b2478:     	lsr	x22, x20, x8
1007b247c:     	ldrb	w8, [x21, #0xf5]
1007b2480:     	tbz	w8, #0x0, 0x1007b24cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x28c>
1007b2484:     	add	x0, sp, #0xf0
1007b2488:     	add	x1, x21, #0x40
1007b248c:     	mov	x2, x24
1007b2490:     	bl	0x100d9d3c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1007b2494:     	ldr	w8, [sp, #0xf0]
1007b2498:     	cmp	w8, #0x2
1007b249c:     	b.ne	0x1007b24ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x26c>
1007b24a0:     	ldr	w8, [sp, #0xf4]
1007b24a4:     	cmp	w8, w25
1007b24a8:     	b.eq	0x1007b2384 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x144>
1007b24ac:     	and	w1, w24, #0xfffffffe
1007b24b0:     	and	w3, w22, #0x1
1007b24b4:     	mov	x0, x21
1007b24b8:     	mov	x2, x25
1007b24bc:     	bl	0x100ca9840 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E14cofactor_innerB6_>
1007b24c0:     	and	w8, w24, #0x1
1007b24c4:     	eor	w24, w0, w8
1007b24c8:     	b	0x1007b23a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1007b24cc:     	and	w3, w22, #0x1
1007b24d0:     	mov	x0, x21
1007b24d4:     	mov	x1, x24
1007b24d8:     	mov	x2, x25
1007b24dc:     	bl	0x100ca9840 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E14cofactor_innerB6_>
1007b24e0:     	mov	x24, x0
1007b24e4:     	b	0x1007b23a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1007b24e8:     	ldr	x8, [sp, #0x80]
1007b24ec:     	cbz	x8, 0x1007b24f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2b8>
1007b24f0:     	ldr	x0, [sp, #0x68]
1007b24f4:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b24f8:     	ldur	x8, [x21, #0x40]
1007b24fc:     	lsr	w0, w24, #1
1007b2500:     	cmn	x8, #0x1
1007b2504:     	ldr	x28, [sp, #0x88]
1007b2508:     	ldr	x23, [sp, #0x60]
1007b250c:     	ldr	x10, [sp, #0x78]
1007b2510:     	b.eq	0x1007b253c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2fc>
1007b2514:     	ldr	x1, [x21, #0x50]
1007b2518:     	cmp	x1, x0
1007b251c:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b2520:     	ldr	x8, [x21, #0x48]
1007b2524:     	add	x8, x8, x0, lsl #4
1007b2528:     	ldr	x8, [x8]
1007b252c:     	bics	x9, x8, x10
1007b2530:     	str	x9, [sp, #0xf0]
1007b2534:     	b.eq	0x1007b2564 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x324>
1007b2538:     	b	0x1007b3554 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1314>
1007b253c:     	ldr	x1, [x21, #0x58]
1007b2540:     	cmp	x1, x0
1007b2544:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b2548:     	ldr	x8, [x21, #0x50]
1007b254c:     	add	x8, x8, x0, lsl #5
1007b2550:     	add	x8, x8, #0x18
1007b2554:     	ldr	x8, [x8]
1007b2558:     	bics	x9, x8, x10
1007b255c:     	str	x9, [sp, #0xf0]
1007b2560:     	b.ne	0x1007b3554 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1314>
1007b2564:     	mov	x20, #0x0               ; =0
1007b2568:     	bic	x8, x10, x8
1007b256c:     	fmov	d0, x8
1007b2570:     	cnt.8b	v0, v0
1007b2574:     	addv.8b	b0, v0
1007b2578:     	fmov	x8, d0
1007b257c:     	ldr	x9, [x28, #0x148]
1007b2580:     	add	x8, x9, x8
1007b2584:     	str	x8, [x28, #0x148]
1007b2588:     	ldr	w9, [sp, #0x70]
1007b258c:     	add	x27, x27, #0x18
1007b2590:     	stp	xzr, x20, [x26]
1007b2594:     	stp	w24, w9, [x26, #0x10]
1007b2598:     	cmp	x27, #0x30
1007b259c:     	b.ne	0x1007b229c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5c>
1007b25a0:     	ldr	w9, [x23, #0x10]
1007b25a4:     	cbz	w9, 0x1007b324c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x100c>
1007b25a8:     	ldr	w10, [x23, #0x28]
1007b25ac:     	cbz	w10, 0x1007b324c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x100c>
1007b25b0:     	cmp	w9, #0x1
1007b25b4:     	ccmp	w10, #0x1, #0x0, eq
1007b25b8:     	b.eq	0x1007b26dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x49c>
1007b25bc:     	ldr	x8, [x28, #0x88]
1007b25c0:     	cbz	x8, 0x1007b26e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4a4>
1007b25c4:     	mov	x8, #0x0                ; =0
1007b25c8:     	mov	x15, #0xa9c5            ; =43461
1007b25cc:     	movk	x15, #0x2e62, lsl #16
1007b25d0:     	movk	x15, #0x7aea, lsl #32
1007b25d4:     	movk	x15, #0xf135, lsl #48
1007b25d8:     	ldp	x11, x12, [x23]
1007b25dc:     	madd	x13, x9, x15, x11
1007b25e0:     	mov	x14, #0x6332            ; =25394
1007b25e4:     	movk	x14, #0x6ed3, lsl #16
1007b25e8:     	movk	x14, #0x765a, lsl #32
1007b25ec:     	movk	x14, #0x284f, lsl #48
1007b25f0:     	mul	x14, x14, x15
1007b25f4:     	madd	x13, x13, x15, x14
1007b25f8:     	add	x13, x13, x12
1007b25fc:     	madd	x16, x13, x15, x10
1007b2600:     	ldp	x13, x14, [x23, #0x18]
1007b2604:     	madd	x16, x16, x15, x13
1007b2608:     	madd	x16, x16, x15, x14
1007b260c:     	mul	x15, x16, x15
1007b2610:     	ror	x0, x15, #0x2c
1007b2614:     	lsr	x17, x0, #57
1007b2618:     	ldp	x16, x15, [x28, #0x70]
1007b261c:     	dup.8b	v0, w17
1007b2620:     	movi.2d	v1, #0xffffffffffffffff
1007b2624:     	mov	w17, #0x38              ; =56
1007b2628:     	and	x0, x0, x15
1007b262c:     	ldr	d2, [x16, x0]
1007b2630:     	cmeq.8b	v3, v2, v0
1007b2634:     	fmov	x1, d3
1007b2638:     	ands	x1, x1, #0x8080808080808080
1007b263c:     	b.eq	0x1007b26ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x46c>
1007b2640:     	rbit	x2, x1
1007b2644:     	clz	x2, x2
1007b2648:     	add	x2, x0, x2, lsr #3
1007b264c:     	and	x2, x2, x15
1007b2650:     	mneg	x2, x2, x17
1007b2654:     	add	x2, x16, x2
1007b2658:     	ldur	x3, [x2, #-0x38]
1007b265c:     	cmp	x11, x3
1007b2660:     	b.ne	0x1007b26a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1007b2664:     	ldur	x3, [x2, #-0x30]
1007b2668:     	cmp	x12, x3
1007b266c:     	b.ne	0x1007b26a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1007b2670:     	ldur	w3, [x2, #-0x28]
1007b2674:     	cmp	w9, w3
1007b2678:     	b.ne	0x1007b26a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1007b267c:     	ldur	x3, [x2, #-0x20]
1007b2680:     	cmp	x13, x3
1007b2684:     	b.ne	0x1007b26a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1007b2688:     	ldur	x3, [x2, #-0x18]
1007b268c:     	cmp	x14, x3
1007b2690:     	b.ne	0x1007b26a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1007b2694:     	ldur	w3, [x2, #-0x10]
1007b2698:     	cmp	w10, w3
1007b269c:     	b.eq	0x1007b275c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x51c>
1007b26a0:     	sub	x2, x1, #0x2
1007b26a4:     	ands	x1, x2, x1
1007b26a8:     	b.ne	0x1007b2640 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x400>
1007b26ac:     	cmeq.8b	v2, v2, v1
1007b26b0:     	fmov	x1, d2
1007b26b4:     	cbnz	x1, 0x1007b26e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4a4>
1007b26b8:     	add	x8, x8, #0x8
1007b26bc:     	add	x0, x0, x8
1007b26c0:     	and	x0, x0, x15
1007b26c4:     	ldr	d2, [x16, x0]
1007b26c8:     	cmeq.8b	v3, v2, v0
1007b26cc:     	fmov	x1, d3
1007b26d0:     	ands	x1, x1, #0x8080808080808080
1007b26d4:     	b.ne	0x1007b2640 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x400>
1007b26d8:     	b	0x1007b26ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x46c>
1007b26dc:     	mov	w0, #0x1                ; =1
1007b26e0:     	b	0x1007b3250 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1007b26e4:     	mov	x24, x28
1007b26e8:     	ldr	x8, [x24, #0xc8]!
1007b26ec:     	add	x8, x8, #0x1
1007b26f0:     	str	x8, [x24]
1007b26f4:     	mov	w11, #0x8481            ; =33921
1007b26f8:     	movk	w11, #0x1e, lsl #16
1007b26fc:     	cmp	x8, x11
1007b2700:     	b.hs	0x1007b35ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x136c>
1007b2704:     	ldr	x12, [x23]
1007b2708:     	ldr	x13, [x23, #0x18]
1007b270c:     	ldr	x8, [x21, #0x40]
1007b2710:     	cmn	x8, #0x1
1007b2714:     	b.eq	0x1007b278c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x54c>
1007b2718:     	ldr	x1, [x21, #0x50]
1007b271c:     	lsr	x0, x9, #1
1007b2720:     	cmp	x1, x0
1007b2724:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b2728:     	lsr	x8, x10, #1
1007b272c:     	cmp	x1, x8
1007b2730:     	b.ls	0x1007b3674 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1434>
1007b2734:     	ldr	x11, [x21, #0x48]
1007b2738:     	lsl	x14, x0, #4
1007b273c:     	ldr	x14, [x11, x14]
1007b2740:     	bic	x19, x14, x12
1007b2744:     	add	x8, x11, x8, lsl #4
1007b2748:     	ldr	x15, [x8]
1007b274c:     	ldp	x11, x1, [x28, #0x18]
1007b2750:     	mov	x22, #0x0               ; =0
1007b2754:     	cbnz	x19, 0x1007b27cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x58c>
1007b2758:     	b	0x1007b27fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5bc>
1007b275c:     	ldur	w0, [x2, #-0x8]
1007b2760:     	ldr	x8, [x28, #0xd0]
1007b2764:     	add	x8, x8, #0x1
1007b2768:     	str	x8, [x28, #0xd0]
1007b276c:     	b	0x1007b3250 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1007b2770:     	adrp	x0, 0x101481000 <dyld_stub_binder+0x101481000>
1007b2774:     	add	x0, x0, #0x46
1007b2778:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
1007b277c:     	add	x2, x2, #0xaa8
1007b2780:     	mov	w1, #0x2c               ; =44
1007b2784:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1007b2788:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b278c:     	ldr	x1, [x21, #0x58]
1007b2790:     	lsr	x0, x9, #1
1007b2794:     	cmp	x1, x0
1007b2798:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b279c:     	lsr	x8, x10, #1
1007b27a0:     	cmp	x1, x8
1007b27a4:     	b.ls	0x1007b3694 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1454>
1007b27a8:     	ldr	x11, [x21, #0x50]
1007b27ac:     	add	x14, x11, x0, lsl #5
1007b27b0:     	ldr	x14, [x14, #0x18]
1007b27b4:     	bic	x19, x14, x12
1007b27b8:     	add	x8, x11, x8, lsl #5
1007b27bc:     	ldr	x15, [x8, #0x18]!
1007b27c0:     	ldp	x11, x1, [x28, #0x18]
1007b27c4:     	mov	x22, #0x0               ; =0
1007b27c8:     	cbz	x19, 0x1007b27fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5bc>
1007b27cc:     	mov	w8, #0x1                ; =1
1007b27d0:     	mov	x14, x19
1007b27d4:     	rbit	x16, x14
1007b27d8:     	clz	x0, x16
1007b27dc:     	cmp	x0, x1
1007b27e0:     	b.hs	0x1007b35f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13b4>
1007b27e4:     	ldr	w16, [x11, x0, lsl #2]
1007b27e8:     	lsl	x16, x8, x16
1007b27ec:     	orr	x22, x16, x22
1007b27f0:     	sub	x16, x14, #0x1
1007b27f4:     	ands	x14, x16, x14
1007b27f8:     	b.ne	0x1007b27d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x594>
1007b27fc:     	ldp	x14, x8, [x28, #0x48]
1007b2800:     	bic	x15, x15, x13
1007b2804:     	cbz	x15, 0x1007b283c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5fc>
1007b2808:     	mov	x16, #0x0               ; =0
1007b280c:     	mov	w17, #0x1               ; =1
1007b2810:     	rbit	x0, x15
1007b2814:     	clz	x0, x0
1007b2818:     	cmp	x0, x8
1007b281c:     	b.hs	0x1007b3600 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13c0>
1007b2820:     	ldr	w0, [x14, x0, lsl #2]
1007b2824:     	lsl	x0, x17, x0
1007b2828:     	orr	x16, x0, x16
1007b282c:     	sub	x0, x15, #0x1
1007b2830:     	ands	x15, x0, x15
1007b2834:     	b.ne	0x1007b2810 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5d0>
1007b2838:     	orr	x22, x16, x22
1007b283c:     	eor	w9, w10, w9
1007b2840:     	cmp	x12, x13
1007b2844:     	ccmp	w9, #0x1, #0x0, eq
1007b2848:     	b.ne	0x1007b2924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1007b284c:     	ldr	x9, [x23, #0x8]
1007b2850:     	ldr	x10, [x23, #0x20]
1007b2854:     	cmp	x9, x10
1007b2858:     	b.ne	0x1007b2924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1007b285c:     	mov	w16, #0x4               ; =4
1007b2860:     	stp	xzr, x16, [sp, #0xf0]
1007b2864:     	str	xzr, [sp, #0x100]
1007b2868:     	mov	x20, #0x0               ; =0
1007b286c:     	cbz	x19, 0x1007b28cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x68c>
1007b2870:     	mov	w8, #0x4                ; =4
1007b2874:     	b	0x1007b289c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x65c>
1007b2878:     	ldr	x8, [sp, #0xf8]
1007b287c:     	rbit	x9, x19
1007b2880:     	clz	x9, x9
1007b2884:     	str	w9, [x8, x20, lsl #2]
1007b2888:     	add	x20, x20, #0x1
1007b288c:     	str	x20, [sp, #0x100]
1007b2890:     	sub	x9, x19, #0x1
1007b2894:     	ands	x19, x9, x19
1007b2898:     	b.eq	0x1007b28b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x674>
1007b289c:     	ldr	x9, [sp, #0xf0]
1007b28a0:     	cmp	x20, x9
1007b28a4:     	b.ne	0x1007b287c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x63c>
1007b28a8:     	add	x0, sp, #0xf0
1007b28ac:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b28b0:     	b	0x1007b2878 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x638>
1007b28b4:     	ldp	x9, x16, [sp, #0xf0]
1007b28b8:     	ldp	x14, x8, [x28, #0x48]
1007b28bc:     	ldp	x11, x1, [x28, #0x18]
1007b28c0:     	cmp	x9, #0x0
1007b28c4:     	cset	w19, eq
1007b28c8:     	b	0x1007b28d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x690>
1007b28cc:     	mov	w19, #0x1               ; =1
1007b28d0:     	mov	x9, #0x0                ; =0
1007b28d4:     	lsl	x10, x20, #2
1007b28d8:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b28dc:     	add	x2, x2, #0xa20
1007b28e0:     	adrp	x12, 0x101605000 <dyld_stub_binder+0x101605000>
1007b28e4:     	add	x12, x12, #0xa38
1007b28e8:     	cmp	x10, x9
1007b28ec:     	b.eq	0x1007b3248 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1008>
1007b28f0:     	ldr	w0, [x16, x9]
1007b28f4:     	cmp	x1, x0
1007b28f8:     	b.ls	0x1007b3610 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13d0>
1007b28fc:     	cmp	x8, x0
1007b2900:     	b.ls	0x1007b3618 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13d8>
1007b2904:     	ldr	w13, [x11, x0, lsl #2]
1007b2908:     	ldr	w15, [x14, x0, lsl #2]
1007b290c:     	add	x9, x9, #0x4
1007b2910:     	cmp	w13, w15
1007b2914:     	b.eq	0x1007b28e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6a8>
1007b2918:     	tbnz	w19, #0x0, 0x1007b2924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1007b291c:     	mov	x0, x16
1007b2920:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2924:     	fmov	d0, x22
1007b2928:     	cnt.8b	v0, v0
1007b292c:     	addv.8b	b0, v0
1007b2930:     	fmov	x19, d0
1007b2934:     	cmp	x19, #0x7
1007b2938:     	b.hs	0x1007b2ce8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xaa8>
1007b293c:     	add	x8, x28, #0x10
1007b2940:     	str	x8, [sp, #0x50]
1007b2944:     	ldr	x8, [x28, #0xd8]
1007b2948:     	add	x8, x8, #0x1
1007b294c:     	str	x8, [x28, #0xd8]
1007b2950:     	ldr	w8, [x28]
1007b2954:     	tbz	w8, #0x0, 0x1007b2d48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb08>
1007b2958:     	str	x19, [sp, #0x8]
1007b295c:     	mov	x26, #0x0               ; =0
1007b2960:     	ldr	x10, [x28, #0x8]
1007b2964:     	add	x8, x28, #0x90
1007b2968:     	str	x8, [sp, #0x48]
1007b296c:     	lsl	x9, x10, #6
1007b2970:     	tst	x10, #0xfc00000000000000
1007b2974:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1007b2978:     	ccmp	x9, x8, #0x2, eq
1007b297c:     	cset	w8, hi
1007b2980:     	str	w8, [sp, #0x14]
1007b2984:     	stp	x10, x24, [sp, #0x28]
1007b2988:     	sub	x8, x10, #0x1
1007b298c:     	stp	x9, x8, [sp, #0x18]
1007b2990:     	mov	w20, #0xff              ; =255
1007b2994:     	mov	w8, #0x1                ; =1
1007b2998:     	b	0x1007b2a00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7c0>
1007b299c:     	strb	w23, [x28]
1007b29a0:     	strb	w10, [x28, #0x1]
1007b29a4:     	str	w25, [x28, #0x4]
1007b29a8:     	stp	x9, x27, [x28, #0x8]
1007b29ac:     	ldp	x8, x9, [sp, #0x70]
1007b29b0:     	stp	x19, x9, [x28, #0x18]
1007b29b4:     	str	x8, [x28, #0x28]
1007b29b8:     	ldr	w8, [sp, #0x58]
1007b29bc:     	stp	w25, w8, [x28, #0x30]
1007b29c0:     	str	x22, [x28, #0x38]
1007b29c4:     	ldr	x28, [sp, #0x88]
1007b29c8:     	ldr	x23, [sp, #0x60]
1007b29cc:     	ldr	w13, [sp, #0x80]
1007b29d0:     	mov	w8, #0x0                ; =0
1007b29d4:     	ldr	x9, [x28, #0x130]
1007b29d8:     	ldp	x2, x10, [x28, #0x98]
1007b29dc:     	add	x10, x10, x2, lsl #6
1007b29e0:     	ldp	x11, x12, [x28, #0xb0]
1007b29e4:     	add	x10, x12, x10
1007b29e8:     	add	x10, x10, x11, lsl #6
1007b29ec:     	cmp	x10, x9
1007b29f0:     	csel	x9, x10, x9, hi
1007b29f4:     	str	x9, [x28, #0x130]
1007b29f8:     	mov	w26, #0x1               ; =1
1007b29fc:     	tbz	w13, #0x0, 0x1007b3470 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1230>
1007b2a00:     	mov	x13, x8
1007b2a04:     	add	x8, x26, x26, lsl #1
1007b2a08:     	lsl	x8, x8, #3
1007b2a0c:     	add	x9, x23, x8
1007b2a10:     	ldr	q0, [x9]
1007b2a14:     	str	q0, [sp, #0xc0]
1007b2a18:     	ldr	x25, [x9, #0x10]
1007b2a1c:     	str	x25, [sp, #0xd0]
1007b2a20:     	cmp	w25, #0x2
1007b2a24:     	b.lo	0x1007b29d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x790>
1007b2a28:     	str	w13, [sp, #0x80]
1007b2a2c:     	ldr	x9, [sp, #0x48]
1007b2a30:     	add	x24, x9, x8
1007b2a34:     	ldrb	w27, [x28, #0x150]
1007b2a38:     	ldr	x8, [x24, #0x8]
1007b2a3c:     	cbz	x8, 0x1007b2a48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x808>
1007b2a40:     	ldr	x0, [x24]
1007b2a44:     	b	0x1007b2acc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x88c>
1007b2a48:     	ldp	x9, x28, [sp, #0x20]
1007b2a4c:     	eor	x8, x28, x9
1007b2a50:     	cmp	x8, x9
1007b2a54:     	b.ls	0x1007b35dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x139c>
1007b2a58:     	ldr	x19, [sp, #0x18]
1007b2a5c:     	ldr	w8, [sp, #0x14]
1007b2a60:     	cbnz	w8, 0x1007b307c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe3c>
1007b2a64:     	cbz	x19, 0x1007b2a7c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x83c>
1007b2a68:     	mov	x0, x19
1007b2a6c:     	mov	w1, #0x8                ; =8
1007b2a70:     	bl	0x1012add90 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1007b2a74:     	cbnz	x0, 0x1007b2a80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x840>
1007b2a78:     	b	0x1007b36b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1474>
1007b2a7c:     	mov	w0, #0x8                ; =8
1007b2a80:     	mov	x8, x0
1007b2a84:     	mov	x9, x28
1007b2a88:     	cmp	x28, #0x4
1007b2a8c:     	b.hs	0x1007b2aa0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x860>
1007b2a90:     	strb	w20, [x8], #0x40
1007b2a94:     	subs	x9, x9, #0x1
1007b2a98:     	b.ne	0x1007b2a90 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x850>
1007b2a9c:     	b	0x1007b2ac4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x884>
1007b2aa0:     	add	x8, x0, #0x80
1007b2aa4:     	and	x9, x28, #0x3fffffffffffffc
1007b2aa8:     	sturb	w20, [x8, #-0x80]
1007b2aac:     	sturb	w20, [x8, #-0x40]
1007b2ab0:     	strb	w20, [x8]
1007b2ab4:     	strb	w20, [x8, #0x40]
1007b2ab8:     	add	x8, x8, #0x100
1007b2abc:     	subs	x9, x9, #0x4
1007b2ac0:     	b.ne	0x1007b2aa8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x868>
1007b2ac4:     	stp	x0, x28, [x24]
1007b2ac8:     	mov	x8, x28
1007b2acc:     	mov	w9, w25
1007b2ad0:     	ldp	x11, x12, [sp, #0xc0]
1007b2ad4:     	ldr	w19, [sp, #0xd4]
1007b2ad8:     	mov	x10, #0xa9c5            ; =43461
1007b2adc:     	movk	x10, #0x2e62, lsl #16
1007b2ae0:     	movk	x10, #0x7aea, lsl #32
1007b2ae4:     	movk	x10, #0xf135, lsl #48
1007b2ae8:     	stp	x12, x11, [sp, #0x70]
1007b2aec:     	madd	x9, x9, x10, x11
1007b2af0:     	madd	x9, x9, x10, x12
1007b2af4:     	madd	x9, x9, x10, x22
1007b2af8:     	mul	x9, x9, x10
1007b2afc:     	sub	x8, x8, #0x1
1007b2b00:     	and	x8, x8, x9, ror #44
1007b2b04:     	add	x28, x0, x8, lsl #6
1007b2b08:     	ldrb	w8, [x28]
1007b2b0c:     	cmp	w8, #0xff
1007b2b10:     	b.ne	0x1007b2b34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x8f4>
1007b2b14:     	ldr	x9, [sp, #0x88]
1007b2b18:     	ldr	x8, [x9, #0x120]
1007b2b1c:     	add	x8, x8, #0x1
1007b2b20:     	str	x8, [x9, #0x120]
1007b2b24:     	add	x8, x28, #0x10
1007b2b28:     	str	x8, [sp, #0x38]
1007b2b2c:     	add	x23, x28, #0x18
1007b2b30:     	b	0x1007b2bd8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x998>
1007b2b34:     	ldr	x9, [x28, #0x38]
1007b2b38:     	cmp	x9, x22
1007b2b3c:     	b.ne	0x1007b2b80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1007b2b40:     	ldr	x9, [x28, #0x20]
1007b2b44:     	ldr	x10, [sp, #0x78]
1007b2b48:     	cmp	x9, x10
1007b2b4c:     	b.ne	0x1007b2b80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1007b2b50:     	ldr	x9, [x28, #0x28]
1007b2b54:     	ldr	x10, [sp, #0x70]
1007b2b58:     	cmp	x9, x10
1007b2b5c:     	b.ne	0x1007b2b80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1007b2b60:     	ldr	w9, [x28, #0x30]
1007b2b64:     	cmp	w9, w25
1007b2b68:     	b.ne	0x1007b2b80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1007b2b6c:     	ldr	x28, [sp, #0x88]
1007b2b70:     	ldr	x8, [x28, #0x118]
1007b2b74:     	add	x8, x8, #0x1
1007b2b78:     	str	x8, [x28, #0x118]
1007b2b7c:     	b	0x1007b29cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x78c>
1007b2b80:     	mov	x23, x28
1007b2b84:     	ldr	x9, [x23, #0x18]!
1007b2b88:     	lsl	x10, x9, #3
1007b2b8c:     	cmp	w8, #0x2
1007b2b90:     	csel	x10, x10, xzr, eq
1007b2b94:     	ldr	x11, [x24, #0x10]
1007b2b98:     	sub	x10, x11, x10
1007b2b9c:     	cmp	w8, #0x2
1007b2ba0:     	mov	x8, x28
1007b2ba4:     	ldr	x0, [x8, #0x10]!
1007b2ba8:     	str	x8, [sp, #0x38]
1007b2bac:     	strb	w20, [x28]
1007b2bb0:     	ldr	x8, [sp, #0x88]
1007b2bb4:     	ldr	q0, [x8, #0x120]
1007b2bb8:     	mov	w11, #0x1               ; =1
1007b2bbc:     	dup.2d	v1, x11
1007b2bc0:     	add.2d	v0, v0, v1
1007b2bc4:     	str	q0, [x8, #0x120]
1007b2bc8:     	str	x10, [x24, #0x10]
1007b2bcc:     	ccmp	x9, #0x0, #0x4, hs
1007b2bd0:     	b.eq	0x1007b2bd8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x998>
1007b2bd4:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2bd8:     	ldr	x8, [sp, #0x50]
1007b2bdc:     	mov	w9, #0x30               ; =48
1007b2be0:     	madd	x3, x26, x9, x8
1007b2be4:     	add	x0, sp, #0xf0
1007b2be8:     	add	x2, sp, #0xc0
1007b2bec:     	mov	x1, x21
1007b2bf0:     	mov	x4, x22
1007b2bf4:     	mov	x5, x27
1007b2bf8:     	ldr	x6, [sp, #0x30]
1007b2bfc:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b2c00:     	ldr	x8, [sp, #0xf0]
1007b2c04:     	cmn	x8, #0x1
1007b2c08:     	str	w19, [sp, #0x58]
1007b2c0c:     	str	x23, [sp, #0x40]
1007b2c10:     	b.eq	0x1007b2c28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9e8>
1007b2c14:     	cmn	x8, #0x2
1007b2c18:     	b.ne	0x1007b2c4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa0c>
1007b2c1c:     	mov	w23, #0x0               ; =0
1007b2c20:     	ldrb	w10, [sp, #0xf8]
1007b2c24:     	b	0x1007b2c2c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9ec>
1007b2c28:     	mov	w23, #0x1               ; =1
1007b2c2c:     	mov	x26, #0x0               ; =0
1007b2c30:     	ldr	x8, [x24, #0x10]
1007b2c34:     	add	x8, x8, x26
1007b2c38:     	str	x8, [x24, #0x10]
1007b2c3c:     	ldrb	w8, [x28]
1007b2c40:     	cmp	w8, #0x2
1007b2c44:     	b.ne	0x1007b299c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1007b2c48:     	b	0x1007b2cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa7c>
1007b2c4c:     	ldp	x27, x19, [sp, #0xf8]
1007b2c50:     	ldr	x9, [sp, #0x108]
1007b2c54:     	lsl	x26, x19, #3
1007b2c58:     	cmp	x8, x19
1007b2c5c:     	b.ls	0x1007b2ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa60>
1007b2c60:     	mov	x23, x9
1007b2c64:     	str	x27, [sp, #0x68]
1007b2c68:     	cbz	x19, 0x1007b2c90 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa50>
1007b2c6c:     	lsl	x1, x8, #3
1007b2c70:     	ldr	x0, [sp, #0x68]
1007b2c74:     	mov	w2, #0x8                ; =8
1007b2c78:     	mov	x3, x26
1007b2c7c:     	bl	0x1012adde4 <__RNvCsiwXPDrQxTLA_7___rustc14___rust_realloc>
1007b2c80:     	mov	x27, x0
1007b2c84:     	mov	x9, x23
1007b2c88:     	cbnz	x0, 0x1007b2ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa60>
1007b2c8c:     	b	0x1007b36c0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1480>
1007b2c90:     	ldr	x0, [sp, #0x68]
1007b2c94:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2c98:     	mov	w27, #0x8               ; =8
1007b2c9c:     	mov	x9, x23
1007b2ca0:     	mov	w23, #0x2               ; =2
1007b2ca4:     	ldr	x8, [x24, #0x10]
1007b2ca8:     	add	x8, x8, x26
1007b2cac:     	str	x8, [x24, #0x10]
1007b2cb0:     	ldrb	w8, [x28]
1007b2cb4:     	cmp	w8, #0x2
1007b2cb8:     	b.ne	0x1007b299c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1007b2cbc:     	ldr	x8, [sp, #0x40]
1007b2cc0:     	ldr	x8, [x8]
1007b2cc4:     	cbz	x8, 0x1007b299c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1007b2cc8:     	ldr	x8, [sp, #0x38]
1007b2ccc:     	ldr	x0, [x8]
1007b2cd0:     	mov	x24, x9
1007b2cd4:     	mov	x26, x10
1007b2cd8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2cdc:     	mov	x10, x26
1007b2ce0:     	mov	x9, x24
1007b2ce4:     	b	0x1007b299c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1007b2ce8:     	mov	x0, x21
1007b2cec:     	mov	x1, x22
1007b2cf0:     	bl	0x100c9e180 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
1007b2cf4:     	ldr	q0, [x23]
1007b2cf8:     	str	q0, [sp, #0xc0]
1007b2cfc:     	ldr	x8, [x23, #0x10]
1007b2d00:     	str	x8, [sp, #0xd0]
1007b2d04:     	mov	w22, w0
1007b2d08:     	ldr	x1, [x28, #0x38]
1007b2d0c:     	cmp	x1, x22
1007b2d10:     	b.ls	0x1007b3650 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1410>
1007b2d14:     	ldr	x8, [x28, #0x30]
1007b2d18:     	ldr	w8, [x8, x22, lsl #2]
1007b2d1c:     	ldr	w9, [sp, #0xd0]
1007b2d20:     	ldr	x10, [x21, #0x40]
1007b2d24:     	lsr	x0, x9, #1
1007b2d28:     	cmn	x10, #0x1
1007b2d2c:     	b.eq	0x1007b3080 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe40>
1007b2d30:     	ldr	x1, [x21, #0x50]
1007b2d34:     	cmp	x1, x0
1007b2d38:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b2d3c:     	ldr	x9, [x21, #0x48]
1007b2d40:     	add	x9, x9, x0, lsl #4
1007b2d44:     	b	0x1007b3098 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe58>
1007b2d48:     	ldrb	w5, [x28, #0x150]
1007b2d4c:     	add	x0, sp, #0xc0
1007b2d50:     	add	x3, x28, #0x10
1007b2d54:     	mov	x1, x21
1007b2d58:     	mov	x2, x23
1007b2d5c:     	mov	x4, x22
1007b2d60:     	mov	x6, x24
1007b2d64:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b2d68:     	ldrb	w5, [x28, #0x150]
1007b2d6c:     	add	x0, sp, #0xf0
1007b2d70:     	add	x2, x23, #0x18
1007b2d74:     	add	x3, x28, #0x40
1007b2d78:     	mov	x1, x21
1007b2d7c:     	mov	x4, x22
1007b2d80:     	mov	x6, x24
1007b2d84:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b2d88:     	mov	w8, #0x1                ; =1
1007b2d8c:     	lsl	x9, x8, x19
1007b2d90:     	lsr	x9, x9, #6
1007b2d94:     	cmp	x19, #0x6
1007b2d98:     	csinc	x27, x8, x9, eq
1007b2d9c:     	lsl	x25, x27, #3
1007b2da0:     	mov	x0, x25
1007b2da4:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
1007b2da8:     	cbz	x0, 0x1007b36a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1464>
1007b2dac:     	mov	x24, x0
1007b2db0:     	mov	x0, #0x0                ; =0
1007b2db4:     	ldp	x19, x25, [sp, #0xc0]
1007b2db8:     	ldp	x1, x9, [sp, #0xd0]
1007b2dbc:     	sub	x10, x0, w25, uxtb
1007b2dc0:     	ldp	x20, x8, [sp, #0xf0]
1007b2dc4:     	ldp	x11, x12, [sp, #0x100]
1007b2dc8:     	mov	x26, x27
1007b2dcc:     	sub	x13, x27, #0x1
1007b2dd0:     	b	0x1007b2dec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbac>
1007b2dd4:     	tst	w8, #0x1
1007b2dd8:     	csel	x14, x14, xzr, ne
1007b2ddc:     	str	x14, [x24, x0, lsl #3]
1007b2de0:     	cmp	x13, x0
1007b2de4:     	b.eq	0x1007b2e40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1007b2de8:     	add	x0, x0, #0x1
1007b2dec:     	mov	x14, x10
1007b2df0:     	cmn	x19, #0x2
1007b2df4:     	b.eq	0x1007b2e08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbc8>
1007b2df8:     	cmp	x0, x1
1007b2dfc:     	b.hs	0x1007b3630 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13f0>
1007b2e00:     	ldr	x14, [x25, x0, lsl #3]
1007b2e04:     	eor	x14, x9, x14
1007b2e08:     	cmn	x20, #0x2
1007b2e0c:     	b.eq	0x1007b2dd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb94>
1007b2e10:     	cmp	x0, x11
1007b2e14:     	b.hs	0x1007b362c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13ec>
1007b2e18:     	ldr	x15, [x8, x0, lsl #3]
1007b2e1c:     	eor	x15, x12, x15
1007b2e20:     	and	x14, x15, x14
1007b2e24:     	str	x14, [x24, x0, lsl #3]
1007b2e28:     	cmp	x13, x0
1007b2e2c:     	b.ne	0x1007b2de8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xba8>
1007b2e30:     	cmp	x20, #0x1
1007b2e34:     	b.lt	0x1007b2e40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1007b2e38:     	mov	x0, x8
1007b2e3c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2e40:     	cmp	x19, #0x1
1007b2e44:     	b.lt	0x1007b2e50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc10>
1007b2e48:     	mov	x0, x25
1007b2e4c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2e50:     	ldr	x8, [x28, #0xc0]
1007b2e54:     	mov	w9, #0x4                ; =4
1007b2e58:     	stp	xzr, x9, [sp, #0xf0]
1007b2e5c:     	str	xzr, [sp, #0x100]
1007b2e60:     	ands	x20, x8, x22
1007b2e64:     	mov	x25, x26
1007b2e68:     	b.eq	0x1007b343c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11fc>
1007b2e6c:     	mov	x19, #0x0               ; =0
1007b2e70:     	mov	w8, #0x4                ; =4
1007b2e74:     	b	0x1007b2e9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc5c>
1007b2e78:     	ldr	x8, [sp, #0xf8]
1007b2e7c:     	rbit	x9, x20
1007b2e80:     	clz	x9, x9
1007b2e84:     	str	w9, [x8, x19, lsl #2]
1007b2e88:     	add	x19, x19, #0x1
1007b2e8c:     	str	x19, [sp, #0x100]
1007b2e90:     	sub	x9, x20, #0x1
1007b2e94:     	ands	x20, x9, x20
1007b2e98:     	b.eq	0x1007b2eb4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc74>
1007b2e9c:     	ldr	x9, [sp, #0xf0]
1007b2ea0:     	cmp	x19, x9
1007b2ea4:     	b.ne	0x1007b2e7c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc3c>
1007b2ea8:     	add	x0, sp, #0xf0
1007b2eac:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b2eb0:     	b	0x1007b2e78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc38>
1007b2eb4:     	mov	x1, x24
1007b2eb8:     	ldp	x8, x20, [sp, #0xf0]
1007b2ebc:     	str	x8, [sp, #0x70]
1007b2ec0:     	str	x20, [sp, #0x58]
1007b2ec4:     	cbz	x19, 0x1007b3418 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11d8>
1007b2ec8:     	add	x8, x20, x19, lsl #2
1007b2ecc:     	str	x8, [sp, #0x78]
1007b2ed0:     	b	0x1007b2eec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcac>
1007b2ed4:     	bic	x22, x22, x19
1007b2ed8:     	mov	x25, x24
1007b2edc:     	mov	x1, x26
1007b2ee0:     	ldr	x8, [sp, #0x78]
1007b2ee4:     	cmp	x20, x8
1007b2ee8:     	b.eq	0x1007b3420 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11e0>
1007b2eec:     	ldr	w8, [x20], #0x4
1007b2ef0:     	mov	w9, #0x1                ; =1
1007b2ef4:     	lsl	x19, x9, x8
1007b2ef8:     	sub	x8, x19, #0x1
1007b2efc:     	and	x8, x8, x22
1007b2f00:     	fmov	d0, x8
1007b2f04:     	cnt.8b	v0, v0
1007b2f08:     	addv.8b	b0, v0
1007b2f0c:     	fmov	w26, s0
1007b2f10:     	fmov	d0, x22
1007b2f14:     	cnt.8b	v0, v0
1007b2f18:     	addv.8b	b0, v0
1007b2f1c:     	fmov	w27, s0
1007b2f20:     	add	x0, sp, #0xc0
1007b2f24:     	mov	x2, x25
1007b2f28:     	mov	x3, x27
1007b2f2c:     	mov	x4, x26
1007b2f30:     	mov	w5, #0x0                ; =0
1007b2f34:     	mov	x24, x1
1007b2f38:     	str	x1, [sp, #0x68]
1007b2f3c:     	bl	0x100e44498 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1007b2f40:     	add	x0, sp, #0xf0
1007b2f44:     	mov	x1, x24
1007b2f48:     	mov	x2, x25
1007b2f4c:     	mov	x3, x27
1007b2f50:     	mov	x4, x26
1007b2f54:     	mov	w5, #0x1                ; =1
1007b2f58:     	bl	0x100e44498 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1007b2f5c:     	ldp	x27, x8, [sp, #0xc8]
1007b2f60:     	ldp	x23, x0, [sp, #0xf0]
1007b2f64:     	ldr	x9, [sp, #0x100]
1007b2f68:     	cmp	x9, x8
1007b2f6c:     	csel	x24, x9, x8, lo
1007b2f70:     	cbz	x24, 0x1007b2fdc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd9c>
1007b2f74:     	mov	x28, x19
1007b2f78:     	mov	x19, x0
1007b2f7c:     	str	x25, [sp, #0x80]
1007b2f80:     	lsl	x25, x24, #3
1007b2f84:     	mov	x0, x25
1007b2f88:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
1007b2f8c:     	cbz	x0, 0x1007b3640 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1400>
1007b2f90:     	mov	x26, x0
1007b2f94:     	cmp	x24, #0x8
1007b2f98:     	mov	x0, x19
1007b2f9c:     	mov	x8, #0x0                ; =0
1007b2fa0:     	b.hs	0x1007b300c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xdcc>
1007b2fa4:     	ldr	x25, [sp, #0x80]
1007b2fa8:     	mov	x19, x28
1007b2fac:     	lsl	x11, x8, #3
1007b2fb0:     	add	x9, x27, x11
1007b2fb4:     	add	x10, x0, x11
1007b2fb8:     	add	x11, x26, x11
1007b2fbc:     	sub	x8, x24, x8
1007b2fc0:     	ldr	x12, [x10], #0x8
1007b2fc4:     	ldr	x13, [x9], #0x8
1007b2fc8:     	orr	x12, x13, x12
1007b2fcc:     	str	x12, [x11], #0x8
1007b2fd0:     	subs	x8, x8, #0x1
1007b2fd4:     	b.ne	0x1007b2fc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd80>
1007b2fd8:     	b	0x1007b2fe0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda0>
1007b2fdc:     	mov	w26, #0x8               ; =8
1007b2fe0:     	cbz	x23, 0x1007b2fe8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda8>
1007b2fe4:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2fe8:     	cbz	x25, 0x1007b2ff4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xdb4>
1007b2fec:     	ldr	x0, [sp, #0x68]
1007b2ff0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b2ff4:     	ldr	x8, [sp, #0xc0]
1007b2ff8:     	ldr	x28, [sp, #0x88]
1007b2ffc:     	cbz	x8, 0x1007b2ed4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc94>
1007b3000:     	mov	x0, x27
1007b3004:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3008:     	b	0x1007b2ed4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc94>
1007b300c:     	sub	x9, x0, x26
1007b3010:     	cmn	x9, #0x40
1007b3014:     	ldr	x25, [sp, #0x80]
1007b3018:     	b.hi	0x1007b2fa8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd68>
1007b301c:     	sub	x9, x27, x26
1007b3020:     	cmn	x9, #0x40
1007b3024:     	mov	x19, x28
1007b3028:     	b.hi	0x1007b2fac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd6c>
1007b302c:     	and	x8, x24, #0xffffffffffffff8
1007b3030:     	add	x9, x27, #0x20
1007b3034:     	add	x10, x0, #0x20
1007b3038:     	add	x11, x26, #0x20
1007b303c:     	and	x12, x24, #0xffffffffffffff8
1007b3040:     	ldp	q0, q1, [x10, #-0x20]
1007b3044:     	ldp	q2, q3, [x10], #0x40
1007b3048:     	ldp	q4, q5, [x9, #-0x20]
1007b304c:     	ldp	q6, q7, [x9], #0x40
1007b3050:     	orr.16b	v0, v4, v0
1007b3054:     	orr.16b	v1, v5, v1
1007b3058:     	orr.16b	v2, v6, v2
1007b305c:     	orr.16b	v3, v7, v3
1007b3060:     	stp	q0, q1, [x11, #-0x20]
1007b3064:     	stp	q2, q3, [x11], #0x40
1007b3068:     	subs	x12, x12, #0x8
1007b306c:     	b.ne	0x1007b3040 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe00>
1007b3070:     	cmp	x24, x8
1007b3074:     	b.ne	0x1007b2fac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd6c>
1007b3078:     	b	0x1007b2fe0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda0>
1007b307c:     	bl	0x1013b9b90 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1007b3080:     	ldr	x1, [x21, #0x58]
1007b3084:     	cmp	x1, x0
1007b3088:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b308c:     	ldr	x9, [x21, #0x50]
1007b3090:     	add	x9, x9, x0, lsl #5
1007b3094:     	add	x9, x9, #0x18
1007b3098:     	ldr	x9, [x9]
1007b309c:     	mov	w10, #0x1               ; =1
1007b30a0:     	lsl	x8, x10, x8
1007b30a4:     	tst	x9, x8
1007b30a8:     	b.eq	0x1007b30bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe7c>
1007b30ac:     	ldp	x9, x10, [sp, #0xc0]
1007b30b0:     	orr	x9, x9, x8
1007b30b4:     	bic	x8, x10, x8
1007b30b8:     	stp	x9, x8, [sp, #0xc0]
1007b30bc:     	sub	x0, x29, #0x70
1007b30c0:     	add	x1, sp, #0xc0
1007b30c4:     	mov	x2, x21
1007b30c8:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b30cc:     	ldur	q0, [x29, #-0x70]
1007b30d0:     	stur	q0, [x29, #-0x90]
1007b30d4:     	ldur	x8, [x29, #-0x60]
1007b30d8:     	stur	q0, [x29, #-0xb0]
1007b30dc:     	str	q0, [sp, #0x90]
1007b30e0:     	str	x8, [sp, #0xa0]
1007b30e4:     	ldr	q0, [sp, #0x90]
1007b30e8:     	str	x8, [sp, #0x100]
1007b30ec:     	str	q0, [sp, #0xf0]
1007b30f0:     	ldur	q0, [x23, #0x18]
1007b30f4:     	str	q0, [sp, #0xc0]
1007b30f8:     	ldur	x8, [x23, #0x28]
1007b30fc:     	str	x8, [sp, #0xd0]
1007b3100:     	ldr	x1, [x28, #0x68]
1007b3104:     	cmp	x1, x22
1007b3108:     	b.ls	0x1007b3650 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1410>
1007b310c:     	ldr	x8, [x28, #0x60]
1007b3110:     	ldr	w8, [x8, x22, lsl #2]
1007b3114:     	ldr	w9, [sp, #0xd0]
1007b3118:     	ldr	x10, [x21, #0x40]
1007b311c:     	lsr	x0, x9, #1
1007b3120:     	cmn	x10, #0x1
1007b3124:     	b.eq	0x1007b3140 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf00>
1007b3128:     	ldr	x1, [x21, #0x50]
1007b312c:     	cmp	x1, x0
1007b3130:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b3134:     	ldr	x9, [x21, #0x48]
1007b3138:     	add	x9, x9, x0, lsl #4
1007b313c:     	b	0x1007b3158 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf18>
1007b3140:     	ldr	x1, [x21, #0x58]
1007b3144:     	cmp	x1, x0
1007b3148:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b314c:     	ldr	x9, [x21, #0x50]
1007b3150:     	add	x9, x9, x0, lsl #5
1007b3154:     	add	x9, x9, #0x18
1007b3158:     	ldr	x9, [x9]
1007b315c:     	mov	w10, #0x1               ; =1
1007b3160:     	lsl	x8, x10, x8
1007b3164:     	tst	x9, x8
1007b3168:     	b.eq	0x1007b317c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf3c>
1007b316c:     	ldp	x9, x10, [sp, #0xc0]
1007b3170:     	orr	x9, x9, x8
1007b3174:     	bic	x8, x10, x8
1007b3178:     	stp	x9, x8, [sp, #0xc0]
1007b317c:     	sub	x0, x29, #0x70
1007b3180:     	add	x1, sp, #0xc0
1007b3184:     	mov	x2, x21
1007b3188:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b318c:     	ldur	q0, [x29, #-0x70]
1007b3190:     	stur	q0, [x29, #-0x90]
1007b3194:     	ldur	x8, [x29, #-0x60]
1007b3198:     	stur	q0, [x29, #-0xb0]
1007b319c:     	str	q0, [sp, #0x90]
1007b31a0:     	str	x8, [sp, #0xa0]
1007b31a4:     	ldr	q0, [sp, #0x90]
1007b31a8:     	str	x8, [sp, #0x118]
1007b31ac:     	add	x8, sp, #0x9
1007b31b0:     	stur	q0, [x8, #0xff]
1007b31b4:     	ldp	q0, q1, [sp, #0xf0]
1007b31b8:     	ldr	q2, [sp, #0x110]
1007b31bc:     	stp	q1, q2, [sp, #0xa0]
1007b31c0:     	str	q0, [sp, #0x90]
1007b31c4:     	add	x2, sp, #0x90
1007b31c8:     	mov	x0, x28
1007b31cc:     	mov	x1, x21
1007b31d0:     	bl	0x1007b2240 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
1007b31d4:     	mov	x23, x0
1007b31d8:     	cmp	w0, #0x1
1007b31dc:     	b.ne	0x1007b31f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xfb4>
1007b31e0:     	ldr	x8, [x28, #0xc0]
1007b31e4:     	lsr	x8, x8, x22
1007b31e8:     	tbz	w8, #0x0, 0x1007b31f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xfb4>
1007b31ec:     	mov	w19, #0x1               ; =1
1007b31f0:     	b	0x1007b3400 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11c0>
1007b31f4:     	ldr	x8, [sp, #0x60]
1007b31f8:     	ldr	q0, [x8]
1007b31fc:     	stur	q0, [x29, #-0x70]
1007b3200:     	ldr	x8, [x8, #0x10]
1007b3204:     	stur	x8, [x29, #-0x60]
1007b3208:     	ldr	x1, [x28, #0x38]
1007b320c:     	cmp	x1, x22
1007b3210:     	b.ls	0x1007b3684 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1444>
1007b3214:     	ldr	x8, [x28, #0x30]
1007b3218:     	ldr	w8, [x8, x22, lsl #2]
1007b321c:     	ldur	w9, [x29, #-0x60]
1007b3220:     	ldr	x10, [x21, #0x40]
1007b3224:     	lsr	x0, x9, #1
1007b3228:     	cmn	x10, #0x1
1007b322c:     	b.eq	0x1007b3270 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1030>
1007b3230:     	ldr	x1, [x21, #0x50]
1007b3234:     	cmp	x1, x0
1007b3238:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b323c:     	ldr	x9, [x21, #0x48]
1007b3240:     	add	x9, x9, x0, lsl #4
1007b3244:     	b	0x1007b3288 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1048>
1007b3248:     	tbz	w19, #0x0, 0x1007b3408 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11c8>
1007b324c:     	mov	w0, #0x0                ; =0
1007b3250:     	add	sp, sp, #0x1c0
1007b3254:     	ldp	x29, x30, [sp, #0x50]
1007b3258:     	ldp	x20, x19, [sp, #0x40]
1007b325c:     	ldp	x22, x21, [sp, #0x30]
1007b3260:     	ldp	x24, x23, [sp, #0x20]
1007b3264:     	ldp	x26, x25, [sp, #0x10]
1007b3268:     	ldp	x28, x27, [sp], #0x60
1007b326c:     	ret
1007b3270:     	ldr	x1, [x21, #0x58]
1007b3274:     	cmp	x1, x0
1007b3278:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b327c:     	ldr	x9, [x21, #0x50]
1007b3280:     	add	x9, x9, x0, lsl #5
1007b3284:     	add	x9, x9, #0x18
1007b3288:     	ldr	x9, [x9]
1007b328c:     	mov	w10, #0x1               ; =1
1007b3290:     	lsl	x8, x10, x8
1007b3294:     	tst	x9, x8
1007b3298:     	b.eq	0x1007b32ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x106c>
1007b329c:     	ldur	q0, [x29, #-0x70]
1007b32a0:     	dup.2d	v1, x8
1007b32a4:     	orr.16b	v0, v0, v1
1007b32a8:     	stur	q0, [x29, #-0x70]
1007b32ac:     	sub	x0, x29, #0xb0
1007b32b0:     	sub	x1, x29, #0x70
1007b32b4:     	mov	x2, x21
1007b32b8:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b32bc:     	ldur	q0, [x29, #-0xb0]
1007b32c0:     	stur	q0, [x29, #-0xd0]
1007b32c4:     	ldur	x8, [x29, #-0xa0]
1007b32c8:     	stur	q0, [x29, #-0xf0]
1007b32cc:     	stur	q0, [x29, #-0x90]
1007b32d0:     	stur	x8, [x29, #-0x80]
1007b32d4:     	ldur	q0, [x29, #-0x90]
1007b32d8:     	str	x8, [sp, #0x100]
1007b32dc:     	str	q0, [sp, #0xf0]
1007b32e0:     	ldr	x8, [sp, #0x60]
1007b32e4:     	ldur	q0, [x8, #0x18]
1007b32e8:     	stur	q0, [x29, #-0x70]
1007b32ec:     	ldur	x8, [x8, #0x28]
1007b32f0:     	stur	x8, [x29, #-0x60]
1007b32f4:     	ldr	x1, [x28, #0x68]
1007b32f8:     	cmp	x1, x22
1007b32fc:     	b.ls	0x1007b3684 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1444>
1007b3300:     	ldr	x8, [x28, #0x60]
1007b3304:     	ldr	w8, [x8, x22, lsl #2]
1007b3308:     	ldur	w9, [x29, #-0x60]
1007b330c:     	ldr	x10, [x21, #0x40]
1007b3310:     	lsr	x0, x9, #1
1007b3314:     	cmn	x10, #0x1
1007b3318:     	b.eq	0x1007b3334 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x10f4>
1007b331c:     	ldr	x1, [x21, #0x50]
1007b3320:     	cmp	x1, x0
1007b3324:     	b.ls	0x1007b3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1007b3328:     	ldr	x9, [x21, #0x48]
1007b332c:     	add	x9, x9, x0, lsl #4
1007b3330:     	b	0x1007b334c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x110c>
1007b3334:     	ldr	x1, [x21, #0x58]
1007b3338:     	cmp	x1, x0
1007b333c:     	b.ls	0x1007b3698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1007b3340:     	ldr	x9, [x21, #0x50]
1007b3344:     	add	x9, x9, x0, lsl #5
1007b3348:     	add	x9, x9, #0x18
1007b334c:     	and	w19, w22, #0x3f
1007b3350:     	ldr	x9, [x9]
1007b3354:     	mov	w10, #0x1               ; =1
1007b3358:     	lsl	x8, x10, x8
1007b335c:     	tst	x9, x8
1007b3360:     	b.eq	0x1007b3374 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1134>
1007b3364:     	ldur	q0, [x29, #-0x70]
1007b3368:     	dup.2d	v1, x8
1007b336c:     	orr.16b	v0, v0, v1
1007b3370:     	stur	q0, [x29, #-0x70]
1007b3374:     	sub	x0, x29, #0xb0
1007b3378:     	sub	x1, x29, #0x70
1007b337c:     	mov	x2, x21
1007b3380:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b3384:     	ldur	q0, [x29, #-0xb0]
1007b3388:     	stur	q0, [x29, #-0xd0]
1007b338c:     	ldur	x8, [x29, #-0xa0]
1007b3390:     	stur	q0, [x29, #-0xf0]
1007b3394:     	stur	q0, [x29, #-0x90]
1007b3398:     	stur	x8, [x29, #-0x80]
1007b339c:     	ldur	q0, [x29, #-0x90]
1007b33a0:     	str	x8, [sp, #0x118]
1007b33a4:     	add	x8, sp, #0x9
1007b33a8:     	stur	q0, [x8, #0xff]
1007b33ac:     	ldp	q0, q1, [sp, #0xf0]
1007b33b0:     	ldr	q2, [sp, #0x110]
1007b33b4:     	stp	q1, q2, [sp, #0xd0]
1007b33b8:     	str	q0, [sp, #0xc0]
1007b33bc:     	add	x2, sp, #0xc0
1007b33c0:     	mov	x0, x28
1007b33c4:     	mov	x1, x21
1007b33c8:     	bl	0x1007b2240 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
1007b33cc:     	mov	x3, x0
1007b33d0:     	ldr	x8, [x28, #0xc0]
1007b33d4:     	mov	x0, x21
1007b33d8:     	lsr	x8, x8, x19
1007b33dc:     	tbz	w8, #0x0, 0x1007b33f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11b0>
1007b33e0:     	mov	w1, #0xe                ; =14
1007b33e4:     	mov	x2, x23
1007b33e8:     	bl	0x100caa240 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1007b33ec:     	b	0x1007b33fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11bc>
1007b33f0:     	mov	x1, x22
1007b33f4:     	mov	x2, x23
1007b33f8:     	bl	0x100caac8c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
1007b33fc:     	mov	x19, x0
1007b3400:     	ldr	x23, [sp, #0x60]
1007b3404:     	b	0x1007b3458 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1218>
1007b3408:     	mov	x0, x16
1007b340c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3410:     	mov	w0, #0x0                ; =0
1007b3414:     	b	0x1007b3250 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1007b3418:     	mov	x24, x25
1007b341c:     	mov	x26, x1
1007b3420:     	ldr	x8, [sp, #0x70]
1007b3424:     	cbz	x8, 0x1007b3430 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11f0>
1007b3428:     	ldr	x0, [sp, #0x58]
1007b342c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3430:     	mov	x25, x24
1007b3434:     	mov	x24, x26
1007b3438:     	ldr	x23, [sp, #0x60]
1007b343c:     	stp	x25, x24, [sp, #0xf0]
1007b3440:     	str	x25, [sp, #0x100]
1007b3444:     	add	x2, sp, #0xf0
1007b3448:     	mov	x0, x21
1007b344c:     	mov	x1, x22
1007b3450:     	bl	0x100caa6a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
1007b3454:     	mov	x19, x0
1007b3458:     	add	x0, x28, #0x70
1007b345c:     	mov	x1, x23
1007b3460:     	mov	x2, x19
1007b3464:     	bl	0x100d41f30 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
1007b3468:     	mov	x0, x19
1007b346c:     	b	0x1007b3250 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1007b3470:     	ldr	x1, [x28, #0x90]
1007b3474:     	add	x0, sp, #0xc0
1007b3478:     	mov	x3, x21
1007b347c:     	mov	x4, x23
1007b3480:     	mov	x5, x22
1007b3484:     	bl	0x1007ae438 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1007b3488:     	ldp	x1, x2, [x28, #0xa8]
1007b348c:     	add	x0, sp, #0xf0
1007b3490:     	add	x4, x23, #0x18
1007b3494:     	mov	x3, x21
1007b3498:     	mov	x5, x22
1007b349c:     	bl	0x1007ae438 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1007b34a0:     	mov	w8, #0x1                ; =1
1007b34a4:     	ldr	x10, [sp, #0x8]
1007b34a8:     	lsl	x9, x8, x10
1007b34ac:     	lsr	x9, x9, #6
1007b34b0:     	cmp	x10, #0x6
1007b34b4:     	csinc	x27, x8, x9, eq
1007b34b8:     	lsl	x25, x27, #3
1007b34bc:     	mov	x0, x25
1007b34c0:     	mov	w1, #0x8                ; =8
1007b34c4:     	bl	0x1012add90 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1007b34c8:     	cbz	x0, 0x1007b36d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1490>
1007b34cc:     	mov	x24, x0
1007b34d0:     	mov	x0, #0x0                ; =0
1007b34d4:     	ldp	x19, x25, [sp, #0xc0]
1007b34d8:     	ldp	x1, x9, [sp, #0xd0]
1007b34dc:     	sub	x10, x0, w25, uxtb
1007b34e0:     	ldp	x20, x8, [sp, #0xf0]
1007b34e4:     	ldp	x11, x12, [sp, #0x100]
1007b34e8:     	mov	x26, x27
1007b34ec:     	sub	x13, x27, #0x1
1007b34f0:     	b	0x1007b350c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12cc>
1007b34f4:     	tst	w8, #0x1
1007b34f8:     	csel	x14, x14, xzr, ne
1007b34fc:     	str	x14, [x24, x0, lsl #3]
1007b3500:     	cmp	x13, x0
1007b3504:     	b.eq	0x1007b2e40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1007b3508:     	add	x0, x0, #0x1
1007b350c:     	mov	x14, x10
1007b3510:     	cmn	x19, #0x2
1007b3514:     	b.eq	0x1007b3528 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12e8>
1007b3518:     	cmp	x0, x1
1007b351c:     	b.hs	0x1007b3664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1424>
1007b3520:     	ldr	x14, [x25, x0, lsl #3]
1007b3524:     	eor	x14, x9, x14
1007b3528:     	cmn	x20, #0x2
1007b352c:     	b.eq	0x1007b34f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12b4>
1007b3530:     	cmp	x0, x11
1007b3534:     	b.hs	0x1007b3660 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1420>
1007b3538:     	ldr	x15, [x8, x0, lsl #3]
1007b353c:     	eor	x15, x12, x15
1007b3540:     	and	x14, x15, x14
1007b3544:     	str	x14, [x24, x0, lsl #3]
1007b3548:     	cmp	x13, x0
1007b354c:     	b.ne	0x1007b3508 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c8>
1007b3550:     	b	0x1007b2e30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbf0>
1007b3554:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
1007b3558:     	add	x2, x2, #0x78
1007b355c:     	adrp	x3, 0x101461000 <dyld_stub_binder+0x101461000>
1007b3560:     	add	x3, x3, #0x185
1007b3564:     	adrp	x5, 0x1015fd000 <dyld_stub_binder+0x1015fd000>
1007b3568:     	add	x5, x5, #0xed8
1007b356c:     	add	x1, sp, #0xf0
1007b3570:     	mov	w0, #0x0                ; =0
1007b3574:     	mov	w4, #0x43               ; =67
1007b3578:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1007b357c:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b3580:     	add	x2, x2, #0x2e0
1007b3584:     	b	0x1007b35a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1360>
1007b3588:     	ldr	x20, [sp, #0x68]
1007b358c:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b3590:     	add	x2, x2, #0x2e0
1007b3594:     	b	0x1007b35d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1394>
1007b3598:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b359c:     	add	x2, x2, #0x2c8
1007b35a0:     	ldr	x20, [sp, #0x68]
1007b35a4:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b35a8:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b35ac:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007b35b0:     	add	x0, x0, #0x31b
1007b35b4:     	adrp	x2, 0x1015ff000 <dyld_stub_binder+0x1015ff000>
1007b35b8:     	add	x2, x2, #0x188
1007b35bc:     	mov	w1, #0x51               ; =81
1007b35c0:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1007b35c4:     	mov	x1, x8
1007b35c8:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b35cc:     	add	x2, x2, #0x2c8
1007b35d0:     	ldr	x20, [sp, #0x68]
1007b35d4:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b35d8:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b35dc:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007b35e0:     	add	x0, x0, #0x2ef
1007b35e4:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007b35e8:     	add	x2, x2, #0xd90
1007b35ec:     	mov	w1, #0x2c               ; =44
1007b35f0:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1007b35f4:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
1007b35f8:     	add	x2, x2, #0x678
1007b35fc:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3600:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
1007b3604:     	add	x2, x2, #0x678
1007b3608:     	mov	x1, x8
1007b360c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3610:     	mov	x20, x16
1007b3614:     	b	0x1007b3624 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13e4>
1007b3618:     	mov	x20, x16
1007b361c:     	mov	x1, x8
1007b3620:     	mov	x2, x12
1007b3624:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3628:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b362c:     	mov	x1, x11
1007b3630:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3634:     	add	x2, x2, #0xa50
1007b3638:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b363c:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b3640:     	mov	w0, #0x8                ; =8
1007b3644:     	mov	x1, x25
1007b3648:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b364c:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b3650:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3654:     	add	x2, x2, #0xa68
1007b3658:     	mov	x0, x22
1007b365c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3660:     	mov	x1, x11
1007b3664:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3668:     	add	x2, x2, #0xa50
1007b366c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3670:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b3674:     	mov	x0, x8
1007b3678:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b367c:     	add	x2, x2, #0x2e0
1007b3680:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3684:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3688:     	add	x2, x2, #0xa80
1007b368c:     	mov	x0, x22
1007b3690:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b3694:     	mov	x0, x8
1007b3698:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b369c:     	add	x2, x2, #0x2c8
1007b36a0:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b36a4:     	mov	w0, #0x8                ; =8
1007b36a8:     	mov	x1, x25
1007b36ac:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b36b0:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b36b4:     	mov	w0, #0x8                ; =8
1007b36b8:     	mov	x1, x19
1007b36bc:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b36c0:     	mov	w0, #0x8                ; =8
1007b36c4:     	mov	x1, x26
1007b36c8:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b36cc:     	b	0x1007b36dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1007b36d0:     	mov	w0, #0x8                ; =8
1007b36d4:     	mov	x1, x25
1007b36d8:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b36dc:     	brk	#0x1
1007b36e0:     	b	0x1007b36f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14b0>
1007b36e4:     	b	0x1007b36fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14bc>
1007b36e8:     	ldr	x20, [sp, #0x68]
1007b36ec:     	b	0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b36f0:     	mov	x19, x0
1007b36f4:     	ldr	x20, [sp, #0xf0]
1007b36f8:     	b	0x1007b3790 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1550>
1007b36fc:     	mov	x19, x0
1007b3700:     	b	0x1007b37a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1560>
1007b3704:     	b	0x1007b3784 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1544>
1007b3708:     	mov	x20, x0
1007b370c:     	cbz	x23, 0x1007b3750 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1510>
1007b3710:     	mov	x0, x19
1007b3714:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3718:     	b	0x1007b3750 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1510>
1007b371c:     	b	0x1007b37c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1588>
1007b3720:     	mov	x20, x24
1007b3724:     	mov	x19, x0
1007b3728:     	ldr	x8, [sp, #0xf0]
1007b372c:     	cbnz	x8, 0x1007b3738 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14f8>
1007b3730:     	mov	x0, x19
1007b3734:     	b	0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b3738:     	ldr	x0, [sp, #0xf8]
1007b373c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3740:     	mov	x0, x19
1007b3744:     	b	0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b3748:     	str	x25, [sp, #0x80]
1007b374c:     	mov	x20, x0
1007b3750:     	ldr	x8, [sp, #0xc0]
1007b3754:     	cbz	x8, 0x1007b376c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x152c>
1007b3758:     	ldr	x0, [sp, #0xc8]
1007b375c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3760:     	b	0x1007b376c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x152c>
1007b3764:     	str	x25, [sp, #0x80]
1007b3768:     	mov	x20, x0
1007b376c:     	ldr	x8, [sp, #0x70]
1007b3770:     	cbz	x8, 0x1007b377c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x153c>
1007b3774:     	ldr	x0, [sp, #0x58]
1007b3778:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b377c:     	mov	x0, x20
1007b3780:     	b	0x1007b37dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x159c>
1007b3784:     	mov	x19, x0
1007b3788:     	mov	x0, x24
1007b378c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3790:     	cmp	x20, #0x1
1007b3794:     	b.lt	0x1007b37a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1560>
1007b3798:     	ldr	x0, [sp, #0xf8]
1007b379c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b37a0:     	ldr	x8, [sp, #0xc0]
1007b37a4:     	cmp	x8, #0x1
1007b37a8:     	b.lt	0x1007b37f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b4>
1007b37ac:     	ldr	x20, [sp, #0xc8]
1007b37b0:     	mov	x0, x19
1007b37b4:     	b	0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b37b8:     	tbz	w19, #0x0, 0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b37bc:     	b	0x1007b37f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1007b37c0:     	b	0x1007b37dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x159c>
1007b37c4:     	b	0x1007b37e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a0>
1007b37c8:     	ldr	x8, [sp, #0xf0]
1007b37cc:     	cbz	x8, 0x1007b37f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1007b37d0:     	ldr	x20, [sp, #0xf8]
1007b37d4:     	b	0x1007b37e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1007b37d8:     	b	0x1007b37e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a0>
1007b37dc:     	ldr	x20, [sp, #0x68]
1007b37e0:     	ldr	x8, [sp, #0x80]
1007b37e4:     	cbz	x8, 0x1007b37f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1007b37e8:     	mov	x19, x0
1007b37ec:     	mov	x0, x20
1007b37f0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b37f4:     	mov	x0, x19
1007b37f8:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
