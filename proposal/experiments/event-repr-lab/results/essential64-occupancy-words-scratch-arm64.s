
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b24294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_>:
100b24294:     	stp	x28, x27, [sp, #-0x60]!
100b24298:     	stp	x26, x25, [sp, #0x10]
100b2429c:     	stp	x24, x23, [sp, #0x20]
100b242a0:     	stp	x22, x21, [sp, #0x30]
100b242a4:     	stp	x20, x19, [sp, #0x40]
100b242a8:     	stp	x29, x30, [sp, #0x50]
100b242ac:     	add	x29, sp, #0x50
100b242b0:     	sub	sp, sp, #0x210
100b242b4:     	ldr	w19, [x3, #0x10]
100b242b8:     	cbz	w19, 0x100b242ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x58>
100b242bc:     	mov	x21, x5
100b242c0:     	mov	x27, x4
100b242c4:     	mov	x20, x3
100b242c8:     	mov	x23, x2
100b242cc:     	mov	x24, x1
100b242d0:     	mov	x25, x0
100b242d4:     	mov	x0, x4
100b242d8:     	mov	x1, x3
100b242dc:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b242e0:     	cbz	x0, 0x100b242f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x60>
100b242e4:     	ldrb	w22, [x0]
100b242e8:     	b	0x100b24a28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x794>
100b242ec:     	mov	w22, #0x0               ; =0
100b242f0:     	b	0x100b24a28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x794>
100b242f4:     	ldr	x8, [x21]
100b242f8:     	add	x8, x8, #0x1
100b242fc:     	str	x8, [x21]
100b24300:     	mov	x22, x25
100b24304:     	ldr	x8, [x22, #0x30]!
100b24308:     	ldr	x1, [x22, #0x10]
100b2430c:     	ldr	x9, [x20]
100b24310:     	lsr	x0, x19, #1
100b24314:     	cmn	x8, #0x1
100b24318:     	b.eq	0x100b24370 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0xdc>
100b2431c:     	cmp	x1, x0
100b24320:     	b.ls	0x100b24b18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x884>
100b24324:     	ldr	w8, [x20, #0x28]
100b24328:     	lsr	x8, x8, #1
100b2432c:     	cmp	x1, x8
100b24330:     	b.ls	0x100b24b04 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x870>
100b24334:     	ldr	w10, [x20, #0x40]
100b24338:     	lsr	x10, x10, #1
100b2433c:     	cmp	x1, x10
100b24340:     	b.ls	0x100b24b14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x880>
100b24344:     	ldr	x11, [x22, #0x8]
100b24348:     	lsl	x8, x8, #4
100b2434c:     	ldr	x8, [x11, x8]
100b24350:     	ldr	x12, [x20, #0x18]
100b24354:     	bic	x8, x8, x12
100b24358:     	lsl	x12, x0, #4
100b2435c:     	ldr	x12, [x11, x12]
100b24360:     	bic	x9, x12, x9
100b24364:     	orr	x8, x8, x9
100b24368:     	add	x9, x11, x10, lsl #4
100b2436c:     	b	0x100b243c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x130>
100b24370:     	ldr	x8, [x22, #0x18]
100b24374:     	cmp	x8, x0
100b24378:     	b.ls	0x100b24b3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x8a8>
100b2437c:     	ldr	w10, [x20, #0x28]
100b24380:     	lsr	x11, x10, #1
100b24384:     	cmp	x8, x11
100b24388:     	b.ls	0x100b24b24 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x890>
100b2438c:     	ldr	w10, [x20, #0x40]
100b24390:     	lsr	x10, x10, #1
100b24394:     	cmp	x8, x10
100b24398:     	b.ls	0x100b24b38 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x8a4>
100b2439c:     	add	x8, x1, x11, lsl #5
100b243a0:     	ldr	x8, [x8, #0x18]
100b243a4:     	ldr	x11, [x20, #0x18]
100b243a8:     	bic	x8, x8, x11
100b243ac:     	add	x11, x1, x0, lsl #5
100b243b0:     	ldr	x11, [x11, #0x18]
100b243b4:     	bic	x9, x11, x9
100b243b8:     	orr	x8, x8, x9
100b243bc:     	add	x9, x1, x10, lsl #5
100b243c0:     	add	x9, x9, #0x18
100b243c4:     	ldr	x9, [x9]
100b243c8:     	mov	x19, x20
100b243cc:     	ldr	x10, [x19, #0x30]!
100b243d0:     	bic	x9, x9, x10
100b243d4:     	orr	x11, x9, x8
100b243d8:     	fmov	d0, x11
100b243dc:     	cnt.8b	v0, v0
100b243e0:     	addv.8b	b0, v0
100b243e4:     	fmov	x8, d0
100b243e8:     	cmp	x8, #0x7
100b243ec:     	mov	x9, x20
100b243f0:     	str	x21, [sp, #0x48]
100b243f4:     	b.hs	0x100b24670 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x3dc>
100b243f8:     	stp	x9, x8, [sp, #0x8]
100b243fc:     	str	x27, [sp]
100b24400:     	mov	x27, #0x0               ; =0
100b24404:     	ldr	x8, [x21, #0x10]
100b24408:     	add	x8, x8, #0x1
100b2440c:     	str	x8, [x21, #0x10]
100b24410:     	ldp	x23, x20, [x21, #0x30]
100b24414:     	add	x26, sp, #0xb0
100b24418:     	mov	x10, x9
100b2441c:     	str	x22, [sp, #0x18]
100b24420:     	str	x11, [sp, #0x30]
100b24424:     	b	0x100b2445c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x1c8>
100b24428:     	and	w24, w19, #0x1
100b2442c:     	add	x23, x23, #0x1
100b24430:     	str	x23, [x21, #0x30]
100b24434:     	mov	x19, #-0x2              ; =-2
100b24438:     	ldr	x11, [sp, #0x30]
100b2443c:     	ldr	x10, [sp, #0x38]
100b24440:     	add	x10, x10, #0x18
100b24444:     	add	x9, x26, x27, lsl #5
100b24448:     	stp	x19, x24, [x9]
100b2444c:     	stp	x25, x8, [x9, #0x10]
100b24450:     	add	x27, x27, #0x1
100b24454:     	cmp	x27, #0x3
100b24458:     	b.eq	0x100b24798 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x504>
100b2445c:     	ldp	x28, x8, [x10]
100b24460:     	stp	x10, x8, [sp, #0x38]
100b24464:     	ldr	w19, [x10, #0x10]
100b24468:     	stur	x11, [x29, #-0x90]
100b2446c:     	add	x0, sp, #0x50
100b24470:     	mov	x1, x22
100b24474:     	mov	x2, x19
100b24478:     	bl	0x100c86bac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b2447c:     	ldr	w8, [sp, #0x50]
100b24480:     	cbz	w8, 0x100b24428 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x194>
100b24484:     	cmp	w8, #0x1
100b24488:     	ldr	x9, [sp, #0x30]
100b2448c:     	b.ne	0x100b24acc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x838>
100b24490:     	ldp	x26, x25, [sp, #0x58]
100b24494:     	ldr	x22, [sp, #0x68]
100b24498:     	stur	x22, [x29, #-0x78]
100b2449c:     	bics	x8, x28, x22
100b244a0:     	str	x8, [sp, #0x50]
100b244a4:     	b.ne	0x100b24a4c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7b8>
100b244a8:     	ldr	x8, [sp, #0x40]
100b244ac:     	bics	x8, x8, x28
100b244b0:     	str	x8, [sp, #0x50]
100b244b4:     	b.ne	0x100b24a5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7c8>
100b244b8:     	orr	x8, x28, x9
100b244bc:     	bics	x8, x22, x8
100b244c0:     	str	x8, [sp, #0x50]
100b244c4:     	b.ne	0x100b24a6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7d8>
100b244c8:     	ands	x8, x28, x9
100b244cc:     	str	x8, [sp, #0x50]
100b244d0:     	b.ne	0x100b24a7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7e8>
100b244d4:     	stp	x19, x23, [sp, #0x20]
100b244d8:     	cbz	x28, 0x100b24590 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x2fc>
100b244dc:     	mov	x19, #-0x1              ; =-1
100b244e0:     	b	0x100b2450c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x278>
100b244e4:     	add	x20, x20, #0x1
100b244e8:     	ldr	x8, [sp, #0x48]
100b244ec:     	str	x20, [x8, #0x38]
100b244f0:     	bic	x22, x22, x21
100b244f4:     	stur	x22, [x29, #-0x78]
100b244f8:     	mov	x26, x24
100b244fc:     	mov	x19, x23
100b24500:     	cmp	x21, x28
100b24504:     	eor	x28, x21, x28
100b24508:     	b.eq	0x100b24578 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x2e4>
100b2450c:     	neg	x8, x28
100b24510:     	and	x21, x28, x8
100b24514:     	sub	x8, x21, #0x1
100b24518:     	and	x8, x8, x22
100b2451c:     	fmov	d0, x8
100b24520:     	cnt.8b	v0, v0
100b24524:     	addv.8b	b0, v0
100b24528:     	fmov	w4, s0
100b2452c:     	fmov	d0, x22
100b24530:     	cnt.8b	v0, v0
100b24534:     	addv.8b	b0, v0
100b24538:     	fmov	w3, s0
100b2453c:     	ldr	x8, [sp, #0x40]
100b24540:     	tst	x21, x8
100b24544:     	cset	w5, ne
100b24548:     	add	x0, sp, #0x50
100b2454c:     	mov	x1, x26
100b24550:     	mov	x2, x25
100b24554:     	bl	0x100d1dbd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b24558:     	ldp	x23, x24, [sp, #0x50]
100b2455c:     	ldr	x25, [sp, #0x60]
100b24560:     	sub	x8, x19, #0x1
100b24564:     	cmn	x8, #0x3
100b24568:     	b.hi	0x100b244e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x250>
100b2456c:     	mov	x0, x26
100b24570:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b24574:     	b	0x100b244e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x250>
100b24578:     	mvn	x8, x22
100b2457c:     	mov	x26, x24
100b24580:     	ldr	x9, [sp, #0x30]
100b24584:     	ands	x28, x8, x9
100b24588:     	b.ne	0x100b24610 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x37c>
100b2458c:     	b	0x100b245a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x30c>
100b24590:     	mvn	x8, x22
100b24594:     	mov	x23, #-0x1              ; =-1
100b24598:     	ands	x28, x8, x9
100b2459c:     	b.ne	0x100b24610 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x37c>
100b245a0:     	mov	x19, x23
100b245a4:     	mov	x24, x26
100b245a8:     	ldr	x11, [sp, #0x30]
100b245ac:     	cmp	x22, x11
100b245b0:     	b.ne	0x100b24aa0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x80c>
100b245b4:     	cmn	x19, #0x1
100b245b8:     	mov	w8, #0x28               ; =40
100b245bc:     	mov	w9, #0x20               ; =32
100b245c0:     	csel	x8, x9, x8, eq
100b245c4:     	ldr	x21, [sp, #0x48]
100b245c8:     	ldr	x9, [x21, x8]
100b245cc:     	add	x9, x9, #0x1
100b245d0:     	str	x9, [x21, x8]
100b245d4:     	ldp	x22, x8, [sp, #0x18]
100b245d8:     	sbfx	x8, x8, #0, #1
100b245dc:     	ldr	x23, [sp, #0x28]
100b245e0:     	add	x26, sp, #0xb0
100b245e4:     	b	0x100b2443c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x1a8>
100b245e8:     	add	x20, x20, #0x1
100b245ec:     	ldr	x8, [sp, #0x48]
100b245f0:     	str	x20, [x8, #0x38]
100b245f4:     	orr	x22, x21, x22
100b245f8:     	stur	x22, [x29, #-0x78]
100b245fc:     	mov	x26, x24
100b24600:     	mov	x23, x19
100b24604:     	cmp	x21, x28
100b24608:     	eor	x28, x21, x28
100b2460c:     	b.eq	0x100b245a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x314>
100b24610:     	neg	x8, x28
100b24614:     	and	x21, x28, x8
100b24618:     	sub	x8, x21, #0x1
100b2461c:     	and	x8, x8, x22
100b24620:     	fmov	d0, x8
100b24624:     	cnt.8b	v0, v0
100b24628:     	addv.8b	b0, v0
100b2462c:     	fmov	w4, s0
100b24630:     	fmov	d0, x22
100b24634:     	cnt.8b	v0, v0
100b24638:     	addv.8b	b0, v0
100b2463c:     	fmov	w3, s0
100b24640:     	add	x0, sp, #0x50
100b24644:     	mov	x1, x26
100b24648:     	mov	x2, x25
100b2464c:     	bl	0x100d1e694 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100b24650:     	ldp	x19, x24, [sp, #0x50]
100b24654:     	ldr	x25, [sp, #0x60]
100b24658:     	sub	x8, x23, #0x1
100b2465c:     	cmn	x8, #0x3
100b24660:     	b.hi	0x100b245e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x354>
100b24664:     	mov	x0, x26
100b24668:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b2466c:     	b	0x100b245e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x354>
100b24670:     	mov	x21, x9
100b24674:     	mov	x8, #0x0                ; =0
100b24678:     	add	x20, sp, #0x110
100b2467c:     	lsl	x9, x23, #2
100b24680:     	cmp	x9, x8
100b24684:     	b.eq	0x100b24ac0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x82c>
100b24688:     	ldr	w22, [x24, x8]
100b2468c:     	lsr	x10, x11, x22
100b24690:     	add	x8, x8, #0x4
100b24694:     	tbz	w10, #0x0, 0x100b24680 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x3ec>
100b24698:     	ldr	q0, [x21]
100b2469c:     	str	q0, [sp, #0xb0]
100b246a0:     	ldr	x8, [x21, #0x10]
100b246a4:     	str	x8, [sp, #0xc0]
100b246a8:     	sub	x0, x29, #0x78
100b246ac:     	add	x1, sp, #0xb0
100b246b0:     	mov	x2, x25
100b246b4:     	mov	x3, x22
100b246b8:     	mov	w4, #0x0                ; =0
100b246bc:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b246c0:     	ldur	q0, [x20, #0xd8]
100b246c4:     	ldur	x8, [x29, #-0x68]
100b246c8:     	stur	x8, [x29, #-0xa0]
100b246cc:     	str	q0, [sp, #0x50]
100b246d0:     	str	x8, [sp, #0x60]
100b246d4:     	str	q0, [sp, #0x110]
100b246d8:     	str	x8, [sp, #0x120]
100b246dc:     	ldur	q0, [x21, #0x18]
100b246e0:     	str	q0, [sp, #0xb0]
100b246e4:     	ldr	x8, [x21, #0x28]
100b246e8:     	str	x8, [sp, #0xc0]
100b246ec:     	sub	x0, x29, #0x78
100b246f0:     	add	x1, sp, #0xb0
100b246f4:     	mov	x2, x25
100b246f8:     	mov	x3, x22
100b246fc:     	mov	w4, #0x0                ; =0
100b24700:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b24704:     	ldur	q0, [x20, #0xd8]
100b24708:     	ldur	x8, [x29, #-0x68]
100b2470c:     	stur	x8, [x29, #-0xa0]
100b24710:     	str	q0, [sp, #0x50]
100b24714:     	str	x8, [sp, #0x60]
100b24718:     	stur	q0, [x20, #0x18]
100b2471c:     	str	x8, [sp, #0x138]
100b24720:     	ldr	q0, [x19]
100b24724:     	str	q0, [sp, #0xb0]
100b24728:     	ldr	x8, [x19, #0x10]
100b2472c:     	str	x8, [sp, #0xc0]
100b24730:     	sub	x0, x29, #0x78
100b24734:     	add	x1, sp, #0xb0
100b24738:     	mov	x2, x25
100b2473c:     	mov	x26, x22
100b24740:     	mov	x3, x22
100b24744:     	mov	w4, #0x0                ; =0
100b24748:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2474c:     	ldur	q0, [x20, #0xd8]
100b24750:     	ldur	x8, [x29, #-0x68]
100b24754:     	stur	x8, [x29, #-0xa0]
100b24758:     	str	q0, [sp, #0x50]
100b2475c:     	str	q0, [sp, #0x140]
100b24760:     	str	x8, [sp, #0x150]
100b24764:     	add	x3, sp, #0x110
100b24768:     	mov	x0, x25
100b2476c:     	mov	x1, x24
100b24770:     	mov	x2, x23
100b24774:     	mov	x4, x27
100b24778:     	ldr	x28, [sp, #0x48]
100b2477c:     	mov	x5, x28
100b24780:     	bl	0x100b24294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_>
100b24784:     	and	w8, w0, #0xff
100b24788:     	cmp	w8, #0xf
100b2478c:     	b.ne	0x100b248d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x63c>
100b24790:     	mov	w22, #0xf               ; =15
100b24794:     	b	0x100b249d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x744>
100b24798:     	mov	x0, #0x0                ; =0
100b2479c:     	mov	w22, #0x0               ; =0
100b247a0:     	ldp	q1, q0, [sp, #0xf0]
100b247a4:     	stp	q1, q0, [sp, #0x90]
100b247a8:     	ldp	q1, q0, [sp, #0xd0]
100b247ac:     	stp	q1, q0, [sp, #0x70]
100b247b0:     	ldp	q1, q0, [sp, #0xb0]
100b247b4:     	stp	q1, q0, [sp, #0x50]
100b247b8:     	mov	w8, #0x1                ; =1
100b247bc:     	ldr	x12, [sp, #0x10]
100b247c0:     	lsl	x9, x8, x12
100b247c4:     	mov	x10, #-0x1              ; =-1
100b247c8:     	lsl	x11, x10, x9
100b247cc:     	cmp	x12, #0x6
100b247d0:     	csinv	x10, x10, x11, eq
100b247d4:     	lsr	x9, x9, #6
100b247d8:     	csinc	x11, x8, x9, eq
100b247dc:     	ldp	x9, x8, [sp, #0x50]
100b247e0:     	tst	w8, #0x1
100b247e4:     	csel	x12, x10, xzr, ne
100b247e8:     	ldp	x1, x13, [sp, #0x60]
100b247ec:     	ldp	x19, x23, [sp, #0x70]
100b247f0:     	sub	x15, x0, w23, uxtb
100b247f4:     	ldp	x14, x16, [sp, #0x80]
100b247f8:     	ldp	x20, x24, [sp, #0x90]
100b247fc:     	sub	x17, x0, w24, uxtb
100b24800:     	ldr	x2, [x21, #0x18]
100b24804:     	add	x2, x2, #0x1
100b24808:     	mov	w3, #0x2                ; =2
100b2480c:     	mov	w4, #0x4                ; =4
100b24810:     	mov	w6, #0x8                ; =8
100b24814:     	ldp	x5, x7, [sp, #0xa0]
100b24818:     	b	0x100b24868 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x5d4>
100b2481c:     	bic	x27, x21, x25
100b24820:     	tst	x26, x27
100b24824:     	csel	w28, wzr, w3, eq
100b24828:     	and	x21, x25, x21
100b2482c:     	bics	xzr, x21, x26
100b24830:     	csel	w25, wzr, w4, eq
100b24834:     	tst	x26, x21
100b24838:     	csel	w21, wzr, w6, eq
100b2483c:     	bics	xzr, x27, x26
100b24840:     	cinc	w26, w28, ne
100b24844:     	orr	w21, w25, w21
100b24848:     	orr	w21, w26, w21
100b2484c:     	orr	w22, w21, w22
100b24850:     	and	w21, w22, #0xff
100b24854:     	add	x2, x2, #0x1
100b24858:     	add	x0, x0, #0x1
100b2485c:     	cmp	w21, #0xf
100b24860:     	ldr	x21, [sp, #0x48]
100b24864:     	b.eq	0x100b249e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x74c>
100b24868:     	cmp	x11, x0
100b2486c:     	b.eq	0x100b249e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x750>
100b24870:     	str	x2, [x21, #0x18]
100b24874:     	mov	x21, x12
100b24878:     	cmn	x9, #0x2
100b2487c:     	b.eq	0x100b24894 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x600>
100b24880:     	cmp	x0, x1
100b24884:     	b.hs	0x100b24af4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x860>
100b24888:     	ldr	x21, [x8, x0, lsl #3]
100b2488c:     	eor	x21, x21, x13
100b24890:     	and	x21, x21, x10
100b24894:     	mov	x25, x15
100b24898:     	cmn	x19, #0x2
100b2489c:     	b.eq	0x100b248b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x61c>
100b248a0:     	cmp	x0, x14
100b248a4:     	b.hs	0x100b24ae8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x854>
100b248a8:     	ldr	x25, [x23, x0, lsl #3]
100b248ac:     	eor	x25, x25, x16
100b248b0:     	mov	x26, x17
100b248b4:     	cmn	x20, #0x2
100b248b8:     	b.eq	0x100b2481c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x588>
100b248bc:     	cmp	x0, x5
100b248c0:     	b.hs	0x100b24af0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x85c>
100b248c4:     	ldr	x26, [x24, x0, lsl #3]
100b248c8:     	eor	x26, x26, x7
100b248cc:     	b	0x100b2481c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x588>
100b248d0:     	mov	x22, x0
100b248d4:     	ldr	q0, [x21]
100b248d8:     	str	q0, [sp, #0xb0]
100b248dc:     	ldr	x8, [x21, #0x10]
100b248e0:     	str	x8, [sp, #0xc0]
100b248e4:     	sub	x0, x29, #0x78
100b248e8:     	add	x1, sp, #0xb0
100b248ec:     	mov	x2, x25
100b248f0:     	mov	x3, x26
100b248f4:     	mov	w4, #0x1                ; =1
100b248f8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b248fc:     	ldur	q0, [x20, #0xd8]
100b24900:     	stur	q0, [x29, #-0x90]
100b24904:     	ldur	x8, [x29, #-0x68]
100b24908:     	stur	q0, [x29, #-0xb0]
100b2490c:     	str	q0, [sp, #0x50]
100b24910:     	str	x8, [sp, #0x60]
100b24914:     	ldr	q0, [sp, #0x50]
100b24918:     	stur	x8, [x29, #-0xf0]
100b2491c:     	stur	q0, [x29, #-0x100]
100b24920:     	ldur	q0, [x21, #0x18]
100b24924:     	str	q0, [sp, #0xb0]
100b24928:     	ldur	x8, [x21, #0x28]
100b2492c:     	str	x8, [sp, #0xc0]
100b24930:     	sub	x0, x29, #0x78
100b24934:     	add	x1, sp, #0xb0
100b24938:     	mov	x2, x25
100b2493c:     	mov	x3, x26
100b24940:     	mov	w4, #0x1                ; =1
100b24944:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b24948:     	ldur	q0, [x20, #0xd8]
100b2494c:     	stur	q0, [x29, #-0x90]
100b24950:     	ldur	x8, [x29, #-0x68]
100b24954:     	stur	q0, [x29, #-0xb0]
100b24958:     	str	q0, [sp, #0x50]
100b2495c:     	str	x8, [sp, #0x60]
100b24960:     	ldr	q0, [sp, #0x50]
100b24964:     	stur	x8, [x29, #-0xd8]
100b24968:     	stur	q0, [x20, #0x68]
100b2496c:     	ldr	q0, [x19]
100b24970:     	str	q0, [sp, #0xb0]
100b24974:     	ldr	x8, [x19, #0x10]
100b24978:     	str	x8, [sp, #0xc0]
100b2497c:     	sub	x0, x29, #0x78
100b24980:     	add	x1, sp, #0xb0
100b24984:     	mov	x2, x25
100b24988:     	mov	x3, x26
100b2498c:     	mov	w4, #0x1                ; =1
100b24990:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b24994:     	ldur	q0, [x20, #0xd8]
100b24998:     	stur	q0, [x29, #-0x90]
100b2499c:     	ldur	x8, [x29, #-0x68]
100b249a0:     	stur	q0, [x29, #-0xb0]
100b249a4:     	str	q0, [sp, #0x50]
100b249a8:     	str	x8, [sp, #0x60]
100b249ac:     	ldr	q0, [sp, #0x50]
100b249b0:     	stur	x8, [x29, #-0xc0]
100b249b4:     	stur	q0, [x29, #-0xd0]
100b249b8:     	sub	x3, x29, #0x100
100b249bc:     	mov	x0, x25
100b249c0:     	mov	x1, x24
100b249c4:     	mov	x2, x23
100b249c8:     	mov	x4, x27
100b249cc:     	mov	x5, x28
100b249d0:     	bl	0x100b24294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_>
100b249d4:     	orr	w22, w0, w22
100b249d8:     	mov	x1, x21
100b249dc:     	b	0x100b24a1c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x788>
100b249e0:     	mov	w22, #0xf               ; =15
100b249e4:     	cmp	x9, #0x1
100b249e8:     	ldr	x27, [sp]
100b249ec:     	b.lt	0x100b249f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x764>
100b249f0:     	mov	x0, x8
100b249f4:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b249f8:     	cmp	x19, #0x1
100b249fc:     	b.lt	0x100b24a08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x774>
100b24a00:     	mov	x0, x23
100b24a04:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b24a08:     	cmp	x20, #0x1
100b24a0c:     	b.lt	0x100b24a18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x784>
100b24a10:     	mov	x0, x24
100b24a14:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b24a18:     	ldr	x1, [sp, #0x8]
100b24a1c:     	mov	x0, x27
100b24a20:     	mov	x2, x22
100b24a24:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b24a28:     	mov	x0, x22
100b24a2c:     	add	sp, sp, #0x210
100b24a30:     	ldp	x29, x30, [sp, #0x50]
100b24a34:     	ldp	x20, x19, [sp, #0x40]
100b24a38:     	ldp	x22, x21, [sp, #0x30]
100b24a3c:     	ldp	x24, x23, [sp, #0x20]
100b24a40:     	ldp	x26, x25, [sp, #0x10]
100b24a44:     	ldp	x28, x27, [sp], #0x60
100b24a48:     	ret
100b24a4c:     	add	x1, sp, #0x50
100b24a50:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24a54:     	add	x5, x5, #0xf0
100b24a58:     	b	0x100b24a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7f4>
100b24a5c:     	add	x1, sp, #0x50
100b24a60:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24a64:     	add	x5, x5, #0xd8
100b24a68:     	b	0x100b24a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7f4>
100b24a6c:     	add	x1, sp, #0x50
100b24a70:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24a74:     	add	x5, x5, #0xc0
100b24a78:     	b	0x100b24a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x7f4>
100b24a7c:     	add	x1, sp, #0x50
100b24a80:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24a84:     	add	x5, x5, #0xa8
100b24a88:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b24a8c:     	add	x2, x2, #0x788
100b24a90:     	mov	w0, #0x0                ; =0
100b24a94:     	mov	x3, #0x0                ; =0
100b24a98:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b24a9c:     	b	0x100b24b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x86c>
100b24aa0:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24aa4:     	add	x5, x5, #0x90
100b24aa8:     	sub	x1, x29, #0x78
100b24aac:     	sub	x2, x29, #0x90
100b24ab0:     	mov	w0, #0x0                ; =0
100b24ab4:     	mov	x3, #0x0                ; =0
100b24ab8:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b24abc:     	b	0x100b24b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x86c>
100b24ac0:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b24ac4:     	add	x0, x0, #0xd90
100b24ac8:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b24acc:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100b24ad0:     	add	x0, x0, #0xd49
100b24ad4:     	adrp	x2, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b24ad8:     	add	x2, x2, #0x108
100b24adc:     	mov	w1, #0xc9               ; =201
100b24ae0:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b24ae4:     	b	0x100b24b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x86c>
100b24ae8:     	mov	x1, x14
100b24aec:     	b	0x100b24af4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x860>
100b24af0:     	mov	x1, x5
100b24af4:     	adrp	x2, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b24af8:     	add	x2, x2, #0x220
100b24afc:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24b00:     	brk	#0x1
100b24b04:     	mov	x0, x8
100b24b08:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24b0c:     	add	x2, x2, #0x6c0
100b24b10:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24b14:     	mov	x0, x10
100b24b18:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24b1c:     	add	x2, x2, #0x6c0
100b24b20:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24b24:     	mov	x0, x11
100b24b28:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24b2c:     	add	x2, x2, #0x6a8
100b24b30:     	mov	x1, x8
100b24b34:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24b38:     	mov	x0, x10
100b24b3c:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b24b40:     	add	x2, x2, #0x6a8
100b24b44:     	mov	x1, x8
100b24b48:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b24b4c:     	mov	x20, x0
100b24b50:     	add	x0, sp, #0x50
100b24b54:     	bl	0x10071517c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej3_EBK_>
100b24b58:     	mov	x0, x20
100b24b5c:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
100b24b60:     	b	0x100b24b98 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x904>
100b24b64:     	mov	x20, x0
100b24b68:     	mov	x19, x23
100b24b6c:     	b	0x100b24b80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x8ec>
100b24b70:     	mov	x20, x0
100b24b74:     	b	0x100b24b80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x8ec>
100b24b78:     	mov	x20, x0
100b24b7c:     	mov	x26, x24
100b24b80:     	sub	x8, x19, #0x1
100b24b84:     	cmn	x8, #0x3
100b24b88:     	b.hi	0x100b24b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x908>
100b24b8c:     	mov	x0, x26
100b24b90:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b24b94:     	b	0x100b24b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x908>
100b24b98:     	mov	x20, x0
100b24b9c:     	cbnz	x27, 0x100b24ba8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x914>
100b24ba0:     	mov	x0, x20
100b24ba4:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
100b24ba8:     	add	x8, sp, #0xb0
100b24bac:     	add	x19, x8, #0x8
100b24bb0:     	b	0x100b24bc0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x92c>
100b24bb4:     	add	x19, x19, #0x20
100b24bb8:     	subs	x27, x27, #0x1
100b24bbc:     	b.eq	0x100b24ba0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x90c>
100b24bc0:     	ldur	x8, [x19, #-0x8]
100b24bc4:     	cmp	x8, #0x1
100b24bc8:     	b.lt	0x100b24bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x920>
100b24bcc:     	ldr	x0, [x19]
100b24bd0:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b24bd4:     	b	0x100b24bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_+0x920>
