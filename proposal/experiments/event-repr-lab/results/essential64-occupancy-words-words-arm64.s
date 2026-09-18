
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b22294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_>:
100b22294:     	stp	x28, x27, [sp, #-0x60]!
100b22298:     	stp	x26, x25, [sp, #0x10]
100b2229c:     	stp	x24, x23, [sp, #0x20]
100b222a0:     	stp	x22, x21, [sp, #0x30]
100b222a4:     	stp	x20, x19, [sp, #0x40]
100b222a8:     	stp	x29, x30, [sp, #0x50]
100b222ac:     	add	x29, sp, #0x50
100b222b0:     	sub	sp, sp, #0x210
100b222b4:     	ldr	w19, [x3, #0x10]
100b222b8:     	cbz	w19, 0x100b222ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x58>
100b222bc:     	mov	x21, x5
100b222c0:     	mov	x27, x4
100b222c4:     	mov	x20, x3
100b222c8:     	mov	x23, x2
100b222cc:     	mov	x24, x1
100b222d0:     	mov	x25, x0
100b222d4:     	mov	x0, x4
100b222d8:     	mov	x1, x3
100b222dc:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b222e0:     	cbz	x0, 0x100b222f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x60>
100b222e4:     	ldrb	w22, [x0]
100b222e8:     	b	0x100b22a28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x794>
100b222ec:     	mov	w22, #0x0               ; =0
100b222f0:     	b	0x100b22a28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x794>
100b222f4:     	ldr	x8, [x21]
100b222f8:     	add	x8, x8, #0x1
100b222fc:     	str	x8, [x21]
100b22300:     	mov	x22, x25
100b22304:     	ldr	x8, [x22, #0x30]!
100b22308:     	ldr	x1, [x22, #0x10]
100b2230c:     	ldr	x9, [x20]
100b22310:     	lsr	x0, x19, #1
100b22314:     	cmn	x8, #0x1
100b22318:     	b.eq	0x100b22370 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0xdc>
100b2231c:     	cmp	x1, x0
100b22320:     	b.ls	0x100b22b18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x884>
100b22324:     	ldr	w8, [x20, #0x28]
100b22328:     	lsr	x8, x8, #1
100b2232c:     	cmp	x1, x8
100b22330:     	b.ls	0x100b22b04 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x870>
100b22334:     	ldr	w10, [x20, #0x40]
100b22338:     	lsr	x10, x10, #1
100b2233c:     	cmp	x1, x10
100b22340:     	b.ls	0x100b22b14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x880>
100b22344:     	ldr	x11, [x22, #0x8]
100b22348:     	lsl	x8, x8, #4
100b2234c:     	ldr	x8, [x11, x8]
100b22350:     	ldr	x12, [x20, #0x18]
100b22354:     	bic	x8, x8, x12
100b22358:     	lsl	x12, x0, #4
100b2235c:     	ldr	x12, [x11, x12]
100b22360:     	bic	x9, x12, x9
100b22364:     	orr	x8, x8, x9
100b22368:     	add	x9, x11, x10, lsl #4
100b2236c:     	b	0x100b223c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x130>
100b22370:     	ldr	x8, [x22, #0x18]
100b22374:     	cmp	x8, x0
100b22378:     	b.ls	0x100b22b3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x8a8>
100b2237c:     	ldr	w10, [x20, #0x28]
100b22380:     	lsr	x11, x10, #1
100b22384:     	cmp	x8, x11
100b22388:     	b.ls	0x100b22b24 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x890>
100b2238c:     	ldr	w10, [x20, #0x40]
100b22390:     	lsr	x10, x10, #1
100b22394:     	cmp	x8, x10
100b22398:     	b.ls	0x100b22b38 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x8a4>
100b2239c:     	add	x8, x1, x11, lsl #5
100b223a0:     	ldr	x8, [x8, #0x18]
100b223a4:     	ldr	x11, [x20, #0x18]
100b223a8:     	bic	x8, x8, x11
100b223ac:     	add	x11, x1, x0, lsl #5
100b223b0:     	ldr	x11, [x11, #0x18]
100b223b4:     	bic	x9, x11, x9
100b223b8:     	orr	x8, x8, x9
100b223bc:     	add	x9, x1, x10, lsl #5
100b223c0:     	add	x9, x9, #0x18
100b223c4:     	ldr	x9, [x9]
100b223c8:     	mov	x19, x20
100b223cc:     	ldr	x10, [x19, #0x30]!
100b223d0:     	bic	x9, x9, x10
100b223d4:     	orr	x11, x9, x8
100b223d8:     	fmov	d0, x11
100b223dc:     	cnt.8b	v0, v0
100b223e0:     	addv.8b	b0, v0
100b223e4:     	fmov	x8, d0
100b223e8:     	cmp	x8, #0x7
100b223ec:     	mov	x9, x20
100b223f0:     	str	x21, [sp, #0x48]
100b223f4:     	b.hs	0x100b22670 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x3dc>
100b223f8:     	stp	x9, x8, [sp, #0x8]
100b223fc:     	str	x27, [sp]
100b22400:     	mov	x27, #0x0               ; =0
100b22404:     	ldr	x8, [x21, #0x10]
100b22408:     	add	x8, x8, #0x1
100b2240c:     	str	x8, [x21, #0x10]
100b22410:     	ldp	x23, x20, [x21, #0x30]
100b22414:     	add	x26, sp, #0xb0
100b22418:     	mov	x10, x9
100b2241c:     	str	x22, [sp, #0x18]
100b22420:     	str	x11, [sp, #0x30]
100b22424:     	b	0x100b2245c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x1c8>
100b22428:     	and	w24, w19, #0x1
100b2242c:     	add	x23, x23, #0x1
100b22430:     	str	x23, [x21, #0x30]
100b22434:     	mov	x19, #-0x2              ; =-2
100b22438:     	ldr	x11, [sp, #0x30]
100b2243c:     	ldr	x10, [sp, #0x38]
100b22440:     	add	x10, x10, #0x18
100b22444:     	add	x9, x26, x27, lsl #5
100b22448:     	stp	x19, x24, [x9]
100b2244c:     	stp	x25, x8, [x9, #0x10]
100b22450:     	add	x27, x27, #0x1
100b22454:     	cmp	x27, #0x3
100b22458:     	b.eq	0x100b22798 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x504>
100b2245c:     	ldp	x28, x8, [x10]
100b22460:     	stp	x10, x8, [sp, #0x38]
100b22464:     	ldr	w19, [x10, #0x10]
100b22468:     	stur	x11, [x29, #-0x90]
100b2246c:     	add	x0, sp, #0x50
100b22470:     	mov	x1, x22
100b22474:     	mov	x2, x19
100b22478:     	bl	0x100c82fec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b2247c:     	ldr	w8, [sp, #0x50]
100b22480:     	cbz	w8, 0x100b22428 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x194>
100b22484:     	cmp	w8, #0x1
100b22488:     	ldr	x9, [sp, #0x30]
100b2248c:     	b.ne	0x100b22acc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x838>
100b22490:     	ldp	x26, x25, [sp, #0x58]
100b22494:     	ldr	x22, [sp, #0x68]
100b22498:     	stur	x22, [x29, #-0x78]
100b2249c:     	bics	x8, x28, x22
100b224a0:     	str	x8, [sp, #0x50]
100b224a4:     	b.ne	0x100b22a4c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7b8>
100b224a8:     	ldr	x8, [sp, #0x40]
100b224ac:     	bics	x8, x8, x28
100b224b0:     	str	x8, [sp, #0x50]
100b224b4:     	b.ne	0x100b22a5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7c8>
100b224b8:     	orr	x8, x28, x9
100b224bc:     	bics	x8, x22, x8
100b224c0:     	str	x8, [sp, #0x50]
100b224c4:     	b.ne	0x100b22a6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7d8>
100b224c8:     	ands	x8, x28, x9
100b224cc:     	str	x8, [sp, #0x50]
100b224d0:     	b.ne	0x100b22a7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7e8>
100b224d4:     	stp	x19, x23, [sp, #0x20]
100b224d8:     	cbz	x28, 0x100b22590 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x2fc>
100b224dc:     	mov	x19, #-0x1              ; =-1
100b224e0:     	b	0x100b2250c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x278>
100b224e4:     	add	x20, x20, #0x1
100b224e8:     	ldr	x8, [sp, #0x48]
100b224ec:     	str	x20, [x8, #0x38]
100b224f0:     	bic	x22, x22, x21
100b224f4:     	stur	x22, [x29, #-0x78]
100b224f8:     	mov	x26, x24
100b224fc:     	mov	x19, x23
100b22500:     	cmp	x21, x28
100b22504:     	eor	x28, x21, x28
100b22508:     	b.eq	0x100b22578 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x2e4>
100b2250c:     	neg	x8, x28
100b22510:     	and	x21, x28, x8
100b22514:     	sub	x8, x21, #0x1
100b22518:     	and	x8, x8, x22
100b2251c:     	fmov	d0, x8
100b22520:     	cnt.8b	v0, v0
100b22524:     	addv.8b	b0, v0
100b22528:     	fmov	w4, s0
100b2252c:     	fmov	d0, x22
100b22530:     	cnt.8b	v0, v0
100b22534:     	addv.8b	b0, v0
100b22538:     	fmov	w3, s0
100b2253c:     	ldr	x8, [sp, #0x40]
100b22540:     	tst	x21, x8
100b22544:     	cset	w5, ne
100b22548:     	add	x0, sp, #0x50
100b2254c:     	mov	x1, x26
100b22550:     	mov	x2, x25
100b22554:     	bl	0x100d1a098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b22558:     	ldp	x23, x24, [sp, #0x50]
100b2255c:     	ldr	x25, [sp, #0x60]
100b22560:     	sub	x8, x19, #0x1
100b22564:     	cmn	x8, #0x3
100b22568:     	b.hi	0x100b224e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x250>
100b2256c:     	mov	x0, x26
100b22570:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b22574:     	b	0x100b224e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x250>
100b22578:     	mvn	x8, x22
100b2257c:     	mov	x26, x24
100b22580:     	ldr	x9, [sp, #0x30]
100b22584:     	ands	x28, x8, x9
100b22588:     	b.ne	0x100b22610 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x37c>
100b2258c:     	b	0x100b225a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x30c>
100b22590:     	mvn	x8, x22
100b22594:     	mov	x23, #-0x1              ; =-1
100b22598:     	ands	x28, x8, x9
100b2259c:     	b.ne	0x100b22610 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x37c>
100b225a0:     	mov	x19, x23
100b225a4:     	mov	x24, x26
100b225a8:     	ldr	x11, [sp, #0x30]
100b225ac:     	cmp	x22, x11
100b225b0:     	b.ne	0x100b22aa0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x80c>
100b225b4:     	cmn	x19, #0x1
100b225b8:     	mov	w8, #0x28               ; =40
100b225bc:     	mov	w9, #0x20               ; =32
100b225c0:     	csel	x8, x9, x8, eq
100b225c4:     	ldr	x21, [sp, #0x48]
100b225c8:     	ldr	x9, [x21, x8]
100b225cc:     	add	x9, x9, #0x1
100b225d0:     	str	x9, [x21, x8]
100b225d4:     	ldp	x22, x8, [sp, #0x18]
100b225d8:     	sbfx	x8, x8, #0, #1
100b225dc:     	ldr	x23, [sp, #0x28]
100b225e0:     	add	x26, sp, #0xb0
100b225e4:     	b	0x100b2243c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x1a8>
100b225e8:     	add	x20, x20, #0x1
100b225ec:     	ldr	x8, [sp, #0x48]
100b225f0:     	str	x20, [x8, #0x38]
100b225f4:     	orr	x22, x21, x22
100b225f8:     	stur	x22, [x29, #-0x78]
100b225fc:     	mov	x26, x24
100b22600:     	mov	x23, x19
100b22604:     	cmp	x21, x28
100b22608:     	eor	x28, x21, x28
100b2260c:     	b.eq	0x100b225a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x314>
100b22610:     	neg	x8, x28
100b22614:     	and	x21, x28, x8
100b22618:     	sub	x8, x21, #0x1
100b2261c:     	and	x8, x8, x22
100b22620:     	fmov	d0, x8
100b22624:     	cnt.8b	v0, v0
100b22628:     	addv.8b	b0, v0
100b2262c:     	fmov	w4, s0
100b22630:     	fmov	d0, x22
100b22634:     	cnt.8b	v0, v0
100b22638:     	addv.8b	b0, v0
100b2263c:     	fmov	w3, s0
100b22640:     	add	x0, sp, #0x50
100b22644:     	mov	x1, x26
100b22648:     	mov	x2, x25
100b2264c:     	bl	0x100d1ab54 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100b22650:     	ldp	x19, x24, [sp, #0x50]
100b22654:     	ldr	x25, [sp, #0x60]
100b22658:     	sub	x8, x23, #0x1
100b2265c:     	cmn	x8, #0x3
100b22660:     	b.hi	0x100b225e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x354>
100b22664:     	mov	x0, x26
100b22668:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b2266c:     	b	0x100b225e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x354>
100b22670:     	mov	x21, x9
100b22674:     	mov	x8, #0x0                ; =0
100b22678:     	add	x20, sp, #0x110
100b2267c:     	lsl	x9, x23, #2
100b22680:     	cmp	x9, x8
100b22684:     	b.eq	0x100b22ac0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x82c>
100b22688:     	ldr	w22, [x24, x8]
100b2268c:     	lsr	x10, x11, x22
100b22690:     	add	x8, x8, #0x4
100b22694:     	tbz	w10, #0x0, 0x100b22680 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x3ec>
100b22698:     	ldr	q0, [x21]
100b2269c:     	str	q0, [sp, #0xb0]
100b226a0:     	ldr	x8, [x21, #0x10]
100b226a4:     	str	x8, [sp, #0xc0]
100b226a8:     	sub	x0, x29, #0x78
100b226ac:     	add	x1, sp, #0xb0
100b226b0:     	mov	x2, x25
100b226b4:     	mov	x3, x22
100b226b8:     	mov	w4, #0x0                ; =0
100b226bc:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b226c0:     	ldur	q0, [x20, #0xd8]
100b226c4:     	ldur	x8, [x29, #-0x68]
100b226c8:     	stur	x8, [x29, #-0xa0]
100b226cc:     	str	q0, [sp, #0x50]
100b226d0:     	str	x8, [sp, #0x60]
100b226d4:     	str	q0, [sp, #0x110]
100b226d8:     	str	x8, [sp, #0x120]
100b226dc:     	ldur	q0, [x21, #0x18]
100b226e0:     	str	q0, [sp, #0xb0]
100b226e4:     	ldr	x8, [x21, #0x28]
100b226e8:     	str	x8, [sp, #0xc0]
100b226ec:     	sub	x0, x29, #0x78
100b226f0:     	add	x1, sp, #0xb0
100b226f4:     	mov	x2, x25
100b226f8:     	mov	x3, x22
100b226fc:     	mov	w4, #0x0                ; =0
100b22700:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22704:     	ldur	q0, [x20, #0xd8]
100b22708:     	ldur	x8, [x29, #-0x68]
100b2270c:     	stur	x8, [x29, #-0xa0]
100b22710:     	str	q0, [sp, #0x50]
100b22714:     	str	x8, [sp, #0x60]
100b22718:     	stur	q0, [x20, #0x18]
100b2271c:     	str	x8, [sp, #0x138]
100b22720:     	ldr	q0, [x19]
100b22724:     	str	q0, [sp, #0xb0]
100b22728:     	ldr	x8, [x19, #0x10]
100b2272c:     	str	x8, [sp, #0xc0]
100b22730:     	sub	x0, x29, #0x78
100b22734:     	add	x1, sp, #0xb0
100b22738:     	mov	x2, x25
100b2273c:     	mov	x26, x22
100b22740:     	mov	x3, x22
100b22744:     	mov	w4, #0x0                ; =0
100b22748:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2274c:     	ldur	q0, [x20, #0xd8]
100b22750:     	ldur	x8, [x29, #-0x68]
100b22754:     	stur	x8, [x29, #-0xa0]
100b22758:     	str	q0, [sp, #0x50]
100b2275c:     	str	q0, [sp, #0x140]
100b22760:     	str	x8, [sp, #0x150]
100b22764:     	add	x3, sp, #0x110
100b22768:     	mov	x0, x25
100b2276c:     	mov	x1, x24
100b22770:     	mov	x2, x23
100b22774:     	mov	x4, x27
100b22778:     	ldr	x28, [sp, #0x48]
100b2277c:     	mov	x5, x28
100b22780:     	bl	0x100b22294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_>
100b22784:     	and	w8, w0, #0xff
100b22788:     	cmp	w8, #0xf
100b2278c:     	b.ne	0x100b228d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x63c>
100b22790:     	mov	w22, #0xf               ; =15
100b22794:     	b	0x100b229d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x744>
100b22798:     	mov	x0, #0x0                ; =0
100b2279c:     	mov	w22, #0x0               ; =0
100b227a0:     	ldp	q1, q0, [sp, #0xf0]
100b227a4:     	stp	q1, q0, [sp, #0x90]
100b227a8:     	ldp	q1, q0, [sp, #0xd0]
100b227ac:     	stp	q1, q0, [sp, #0x70]
100b227b0:     	ldp	q1, q0, [sp, #0xb0]
100b227b4:     	stp	q1, q0, [sp, #0x50]
100b227b8:     	mov	w8, #0x1                ; =1
100b227bc:     	ldr	x12, [sp, #0x10]
100b227c0:     	lsl	x9, x8, x12
100b227c4:     	mov	x10, #-0x1              ; =-1
100b227c8:     	lsl	x11, x10, x9
100b227cc:     	cmp	x12, #0x6
100b227d0:     	csinv	x10, x10, x11, eq
100b227d4:     	lsr	x9, x9, #6
100b227d8:     	csinc	x11, x8, x9, eq
100b227dc:     	ldp	x9, x8, [sp, #0x50]
100b227e0:     	tst	w8, #0x1
100b227e4:     	csel	x12, x10, xzr, ne
100b227e8:     	ldp	x1, x13, [sp, #0x60]
100b227ec:     	ldp	x19, x23, [sp, #0x70]
100b227f0:     	sub	x15, x0, w23, uxtb
100b227f4:     	ldp	x14, x16, [sp, #0x80]
100b227f8:     	ldp	x20, x24, [sp, #0x90]
100b227fc:     	sub	x17, x0, w24, uxtb
100b22800:     	ldr	x2, [x21, #0x18]
100b22804:     	add	x2, x2, #0x1
100b22808:     	mov	w3, #0x2                ; =2
100b2280c:     	mov	w4, #0x4                ; =4
100b22810:     	mov	w6, #0x8                ; =8
100b22814:     	ldp	x5, x7, [sp, #0xa0]
100b22818:     	b	0x100b22868 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x5d4>
100b2281c:     	bic	x27, x21, x25
100b22820:     	tst	x26, x27
100b22824:     	csel	w28, wzr, w3, eq
100b22828:     	and	x21, x25, x21
100b2282c:     	bics	xzr, x21, x26
100b22830:     	csel	w25, wzr, w4, eq
100b22834:     	tst	x26, x21
100b22838:     	csel	w21, wzr, w6, eq
100b2283c:     	bics	xzr, x27, x26
100b22840:     	cinc	w26, w28, ne
100b22844:     	orr	w21, w25, w21
100b22848:     	orr	w21, w26, w21
100b2284c:     	orr	w22, w21, w22
100b22850:     	and	w21, w22, #0xff
100b22854:     	add	x2, x2, #0x1
100b22858:     	add	x0, x0, #0x1
100b2285c:     	cmp	w21, #0xf
100b22860:     	ldr	x21, [sp, #0x48]
100b22864:     	b.eq	0x100b229e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x74c>
100b22868:     	cmp	x11, x0
100b2286c:     	b.eq	0x100b229e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x750>
100b22870:     	str	x2, [x21, #0x18]
100b22874:     	mov	x21, x12
100b22878:     	cmn	x9, #0x2
100b2287c:     	b.eq	0x100b22894 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x600>
100b22880:     	cmp	x0, x1
100b22884:     	b.hs	0x100b22af4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x860>
100b22888:     	ldr	x21, [x8, x0, lsl #3]
100b2288c:     	eor	x21, x21, x13
100b22890:     	and	x21, x21, x10
100b22894:     	mov	x25, x15
100b22898:     	cmn	x19, #0x2
100b2289c:     	b.eq	0x100b228b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x61c>
100b228a0:     	cmp	x0, x14
100b228a4:     	b.hs	0x100b22ae8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x854>
100b228a8:     	ldr	x25, [x23, x0, lsl #3]
100b228ac:     	eor	x25, x25, x16
100b228b0:     	mov	x26, x17
100b228b4:     	cmn	x20, #0x2
100b228b8:     	b.eq	0x100b2281c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x588>
100b228bc:     	cmp	x0, x5
100b228c0:     	b.hs	0x100b22af0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x85c>
100b228c4:     	ldr	x26, [x24, x0, lsl #3]
100b228c8:     	eor	x26, x26, x7
100b228cc:     	b	0x100b2281c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x588>
100b228d0:     	mov	x22, x0
100b228d4:     	ldr	q0, [x21]
100b228d8:     	str	q0, [sp, #0xb0]
100b228dc:     	ldr	x8, [x21, #0x10]
100b228e0:     	str	x8, [sp, #0xc0]
100b228e4:     	sub	x0, x29, #0x78
100b228e8:     	add	x1, sp, #0xb0
100b228ec:     	mov	x2, x25
100b228f0:     	mov	x3, x26
100b228f4:     	mov	w4, #0x1                ; =1
100b228f8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b228fc:     	ldur	q0, [x20, #0xd8]
100b22900:     	stur	q0, [x29, #-0x90]
100b22904:     	ldur	x8, [x29, #-0x68]
100b22908:     	stur	q0, [x29, #-0xb0]
100b2290c:     	str	q0, [sp, #0x50]
100b22910:     	str	x8, [sp, #0x60]
100b22914:     	ldr	q0, [sp, #0x50]
100b22918:     	stur	x8, [x29, #-0xf0]
100b2291c:     	stur	q0, [x29, #-0x100]
100b22920:     	ldur	q0, [x21, #0x18]
100b22924:     	str	q0, [sp, #0xb0]
100b22928:     	ldur	x8, [x21, #0x28]
100b2292c:     	str	x8, [sp, #0xc0]
100b22930:     	sub	x0, x29, #0x78
100b22934:     	add	x1, sp, #0xb0
100b22938:     	mov	x2, x25
100b2293c:     	mov	x3, x26
100b22940:     	mov	w4, #0x1                ; =1
100b22944:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22948:     	ldur	q0, [x20, #0xd8]
100b2294c:     	stur	q0, [x29, #-0x90]
100b22950:     	ldur	x8, [x29, #-0x68]
100b22954:     	stur	q0, [x29, #-0xb0]
100b22958:     	str	q0, [sp, #0x50]
100b2295c:     	str	x8, [sp, #0x60]
100b22960:     	ldr	q0, [sp, #0x50]
100b22964:     	stur	x8, [x29, #-0xd8]
100b22968:     	stur	q0, [x20, #0x68]
100b2296c:     	ldr	q0, [x19]
100b22970:     	str	q0, [sp, #0xb0]
100b22974:     	ldr	x8, [x19, #0x10]
100b22978:     	str	x8, [sp, #0xc0]
100b2297c:     	sub	x0, x29, #0x78
100b22980:     	add	x1, sp, #0xb0
100b22984:     	mov	x2, x25
100b22988:     	mov	x3, x26
100b2298c:     	mov	w4, #0x1                ; =1
100b22990:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b22994:     	ldur	q0, [x20, #0xd8]
100b22998:     	stur	q0, [x29, #-0x90]
100b2299c:     	ldur	x8, [x29, #-0x68]
100b229a0:     	stur	q0, [x29, #-0xb0]
100b229a4:     	str	q0, [sp, #0x50]
100b229a8:     	str	x8, [sp, #0x60]
100b229ac:     	ldr	q0, [sp, #0x50]
100b229b0:     	stur	x8, [x29, #-0xc0]
100b229b4:     	stur	q0, [x29, #-0xd0]
100b229b8:     	sub	x3, x29, #0x100
100b229bc:     	mov	x0, x25
100b229c0:     	mov	x1, x24
100b229c4:     	mov	x2, x23
100b229c8:     	mov	x4, x27
100b229cc:     	mov	x5, x28
100b229d0:     	bl	0x100b22294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_>
100b229d4:     	orr	w22, w0, w22
100b229d8:     	mov	x1, x21
100b229dc:     	b	0x100b22a1c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x788>
100b229e0:     	mov	w22, #0xf               ; =15
100b229e4:     	cmp	x9, #0x1
100b229e8:     	ldr	x27, [sp]
100b229ec:     	b.lt	0x100b229f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x764>
100b229f0:     	mov	x0, x8
100b229f4:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b229f8:     	cmp	x19, #0x1
100b229fc:     	b.lt	0x100b22a08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x774>
100b22a00:     	mov	x0, x23
100b22a04:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b22a08:     	cmp	x20, #0x1
100b22a0c:     	b.lt	0x100b22a18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x784>
100b22a10:     	mov	x0, x24
100b22a14:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b22a18:     	ldr	x1, [sp, #0x8]
100b22a1c:     	mov	x0, x27
100b22a20:     	mov	x2, x22
100b22a24:     	bl	0x100c293a0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b22a28:     	mov	x0, x22
100b22a2c:     	add	sp, sp, #0x210
100b22a30:     	ldp	x29, x30, [sp, #0x50]
100b22a34:     	ldp	x20, x19, [sp, #0x40]
100b22a38:     	ldp	x22, x21, [sp, #0x30]
100b22a3c:     	ldp	x24, x23, [sp, #0x20]
100b22a40:     	ldp	x26, x25, [sp, #0x10]
100b22a44:     	ldp	x28, x27, [sp], #0x60
100b22a48:     	ret
100b22a4c:     	add	x1, sp, #0x50
100b22a50:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22a54:     	add	x5, x5, #0xf0
100b22a58:     	b	0x100b22a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7f4>
100b22a5c:     	add	x1, sp, #0x50
100b22a60:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22a64:     	add	x5, x5, #0xd8
100b22a68:     	b	0x100b22a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7f4>
100b22a6c:     	add	x1, sp, #0x50
100b22a70:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22a74:     	add	x5, x5, #0xc0
100b22a78:     	b	0x100b22a88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x7f4>
100b22a7c:     	add	x1, sp, #0x50
100b22a80:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22a84:     	add	x5, x5, #0xa8
100b22a88:     	adrp	x2, 0x1013e6000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x2177>
100b22a8c:     	add	x2, x2, #0xa8
100b22a90:     	mov	w0, #0x0                ; =0
100b22a94:     	mov	x3, #0x0                ; =0
100b22a98:     	bl	0x101281ee0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b22a9c:     	b	0x100b22b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x86c>
100b22aa0:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22aa4:     	add	x5, x5, #0x90
100b22aa8:     	sub	x1, x29, #0x78
100b22aac:     	sub	x2, x29, #0x90
100b22ab0:     	mov	w0, #0x0                ; =0
100b22ab4:     	mov	x3, #0x0                ; =0
100b22ab8:     	bl	0x101281ee0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b22abc:     	b	0x100b22b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x86c>
100b22ac0:     	adrp	x0, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b22ac4:     	add	x0, x0, #0xb78
100b22ac8:     	bl	0x101282074 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b22acc:     	adrp	x0, 0x101325000 <dyld_stub_binder+0x101325000>
100b22ad0:     	add	x0, x0, #0x849
100b22ad4:     	adrp	x2, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b22ad8:     	add	x2, x2, #0x108
100b22adc:     	mov	w1, #0xc9               ; =201
100b22ae0:     	bl	0x101281e74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b22ae4:     	b	0x100b22b00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x86c>
100b22ae8:     	mov	x1, x14
100b22aec:     	b	0x100b22af4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x860>
100b22af0:     	mov	x1, x5
100b22af4:     	adrp	x2, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b22af8:     	add	x2, x2, #0x20
100b22afc:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22b00:     	brk	#0x1
100b22b04:     	mov	x0, x8
100b22b08:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22b0c:     	add	x2, x2, #0x368
100b22b10:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22b14:     	mov	x0, x10
100b22b18:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22b1c:     	add	x2, x2, #0x368
100b22b20:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22b24:     	mov	x0, x11
100b22b28:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22b2c:     	add	x2, x2, #0x350
100b22b30:     	mov	x1, x8
100b22b34:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22b38:     	mov	x0, x10
100b22b3c:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b22b40:     	add	x2, x2, #0x350
100b22b44:     	mov	x1, x8
100b22b48:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b22b4c:     	mov	x20, x0
100b22b50:     	add	x0, sp, #0x50
100b22b54:     	bl	0x10071517c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej3_EBK_>
100b22b58:     	mov	x0, x20
100b22b5c:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
100b22b60:     	b	0x100b22b98 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x904>
100b22b64:     	mov	x20, x0
100b22b68:     	mov	x19, x23
100b22b6c:     	b	0x100b22b80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x8ec>
100b22b70:     	mov	x20, x0
100b22b74:     	b	0x100b22b80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x8ec>
100b22b78:     	mov	x20, x0
100b22b7c:     	mov	x26, x24
100b22b80:     	sub	x8, x19, #0x1
100b22b84:     	cmn	x8, #0x3
100b22b88:     	b.hi	0x100b22b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x908>
100b22b8c:     	mov	x0, x26
100b22b90:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b22b94:     	b	0x100b22b9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x908>
100b22b98:     	mov	x20, x0
100b22b9c:     	cbnz	x27, 0x100b22ba8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x914>
100b22ba0:     	mov	x0, x20
100b22ba4:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
100b22ba8:     	add	x8, sp, #0xb0
100b22bac:     	add	x19, x8, #0x8
100b22bb0:     	b	0x100b22bc0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x92c>
100b22bb4:     	add	x19, x19, #0x20
100b22bb8:     	subs	x27, x27, #0x1
100b22bbc:     	b.eq	0x100b22ba0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x90c>
100b22bc0:     	ldur	x8, [x19, #-0x8]
100b22bc4:     	cmp	x8, #0x1
100b22bc8:     	b.lt	0x100b22bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x920>
100b22bcc:     	ldr	x0, [x19]
100b22bd0:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b22bd4:     	b	0x100b22bb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kb1_EB8_+0x920>
100b22bd8:     	nop
100b22bdc:     	nop
100b22be0:     	nop
100b22be4:     	nop
100b22be8:     	nop
100b22bec:     	nop
100b22bf0:     	nop
100b22bf4:     	nop
100b22bf8:     	nop
100b22bfc:     	nop
