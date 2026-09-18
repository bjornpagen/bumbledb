
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000101650364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_>:
101650364:     	sub	sp, sp, #0x110
101650368:     	stp	x28, x27, [sp, #0xb0]
10165036c:     	stp	x26, x25, [sp, #0xc0]
101650370:     	stp	x24, x23, [sp, #0xd0]
101650374:     	stp	x22, x21, [sp, #0xe0]
101650378:     	stp	x20, x19, [sp, #0xf0]
10165037c:     	stp	x29, x30, [sp, #0x100]
101650380:     	add	x29, sp, #0x100
101650384:     	ldr	x8, [x0, #0x108]
101650388:     	cmp	w8, #0x3e
10165038c:     	and	x9, x8, #0x3f
101650390:     	ccmp	x3, x9, #0x0, ls
101650394:     	b.ne	0x101650774 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
101650398:     	mov	x21, x7
10165039c:     	mov	x22, x5
1016503a0:     	mov	x23, x4
1016503a4:     	mov	x19, x3
1016503a8:     	mov	x24, x2
1016503ac:     	mov	x25, x1
1016503b0:     	mov	x20, x0
1016503b4:     	lsl	x10, x3, #2
1016503b8:     	mov	x9, #0x0                ; =0
1016503bc:     	cbz	x3, 0x1016503f0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x8c>
1016503c0:     	mov	w11, #0x1               ; =1
1016503c4:     	mov	x12, x10
1016503c8:     	mov	x13, x24
1016503cc:     	ldr	w14, [x13], #0x4
1016503d0:     	cmp	w14, w8
1016503d4:     	b.hs	0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1016503d8:     	lsr	x15, x9, x14
1016503dc:     	tbnz	w15, #0x0, 0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1016503e0:     	lsl	x14, x11, x14
1016503e4:     	orr	x9, x14, x9
1016503e8:     	subs	x12, x12, #0x4
1016503ec:     	b.ne	0x1016503cc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x68>
1016503f0:     	stur	x9, [x29, #-0x58]
1016503f4:     	mov	x11, #-0x1              ; =-1
1016503f8:     	lsl	x11, x11, x19
1016503fc:     	mvn	x11, x11
101650400:     	str	x11, [sp, #0x18]
101650404:     	cmp	x9, x11
101650408:     	b.ne	0x10165078c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
10165040c:     	cmp	x6, x19
101650410:     	b.ne	0x101650774 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
101650414:     	mov	x11, #0x0               ; =0
101650418:     	cbz	x19, 0x10165044c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xe8>
10165041c:     	mov	w12, #0x1               ; =1
101650420:     	mov	x13, x10
101650424:     	mov	x14, x22
101650428:     	ldr	w15, [x14], #0x4
10165042c:     	cmp	w15, w8
101650430:     	b.hs	0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
101650434:     	lsr	x16, x11, x15
101650438:     	tbnz	w16, #0x0, 0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
10165043c:     	lsl	x15, x12, x15
101650440:     	orr	x11, x15, x11
101650444:     	subs	x13, x13, #0x4
101650448:     	b.ne	0x101650428 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xc4>
10165044c:     	stur	x11, [x29, #-0x58]
101650450:     	str	x9, [sp, #0x18]
101650454:     	cmp	x11, x9
101650458:     	b.ne	0x10165078c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
10165045c:     	ldr	x11, [x29, #0x18]
101650460:     	cmp	x11, x19
101650464:     	b.ne	0x101650774 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
101650468:     	ldr	x26, [x29, #0x10]
10165046c:     	mov	x11, #0x0               ; =0
101650470:     	cbz	x19, 0x1016504a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x13c>
101650474:     	mov	w12, #0x1               ; =1
101650478:     	mov	x13, x26
10165047c:     	ldr	w14, [x13], #0x4
101650480:     	cmp	w14, w8
101650484:     	b.hs	0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
101650488:     	lsr	x15, x11, x14
10165048c:     	tbnz	w15, #0x0, 0x10165075c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
101650490:     	lsl	x14, x12, x14
101650494:     	orr	x11, x14, x11
101650498:     	subs	x10, x10, #0x4
10165049c:     	b.ne	0x10165047c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x118>
1016504a0:     	stur	x11, [x29, #-0x58]
1016504a4:     	str	x9, [sp, #0x18]
1016504a8:     	cmp	x11, x9
1016504ac:     	b.ne	0x10165078c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
1016504b0:     	lsr	x8, x21, x8
1016504b4:     	str	x8, [sp, #0x18]
1016504b8:     	cbnz	x8, 0x1016507a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x444>
1016504bc:     	cbz	x21, 0x101650530 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
1016504c0:     	ldr	w8, [x20, #0x188]
1016504c4:     	cmp	w8, #0x1
1016504c8:     	b.eq	0x101650530 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
1016504cc:     	ldr	w8, [x20, #0x128]
1016504d0:     	cbz	w8, 0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1016504d4:     	ldr	w8, [x20, #0x12c]
1016504d8:     	add	w9, w8, w8, lsl #1
1016504dc:     	lsr	x9, x21, x9
1016504e0:     	cbnz	x9, 0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1016504e4:     	mov	x9, #-0x1               ; =-1
1016504e8:     	lsl	x10, x9, x8
1016504ec:     	mvn	x9, x10
1016504f0:     	bic	x11, x21, x10
1016504f4:     	cmp	x11, x9
1016504f8:     	ccmp	x11, #0x0, #0x4, ne
1016504fc:     	b.ne	0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
101650500:     	and	x11, x8, #0x3f
101650504:     	lsr	x11, x21, x11
101650508:     	bic	x11, x11, x10
10165050c:     	cmp	x11, x9
101650510:     	ccmp	x11, #0x0, #0x4, ne
101650514:     	b.ne	0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
101650518:     	ubfiz	w8, w8, #1, #5
10165051c:     	lsr	x8, x21, x8
101650520:     	bics	x8, x8, x10
101650524:     	b.eq	0x101650530 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
101650528:     	cmp	x8, x9
10165052c:     	b.ne	0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
101650530:     	mov	x0, x20
101650534:     	mov	x1, x24
101650538:     	mov	x2, x19
10165053c:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101650540:     	tbz	w0, #0x0, 0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
101650544:     	mov	x0, x20
101650548:     	mov	x1, x22
10165054c:     	mov	x2, x19
101650550:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101650554:     	cbz	w0, 0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
101650558:     	mov	x0, x20
10165055c:     	mov	x1, x26
101650560:     	mov	x2, x19
101650564:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101650568:     	cbz	w0, 0x1016505a0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
10165056c:     	stp	x26, x19, [sp, #0x8]
101650570:     	add	x0, sp, #0x18
101650574:     	str	x21, [sp]
101650578:     	mov	x1, x20
10165057c:     	mov	x2, x25
101650580:     	mov	x3, x24
101650584:     	mov	x4, x19
101650588:     	mov	x5, x23
10165058c:     	mov	x6, x22
101650590:     	mov	x7, x19
101650594:     	bl	0x101411320 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
101650598:     	ldr	w1, [sp, #0x18]
10165059c:     	b	0x101650738 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d4>
1016505a0:     	ldr	w27, [x20, #0x188]
1016505a4:     	mov	x0, x20
1016505a8:     	mov	x1, x27
1016505ac:     	mov	x2, x24
1016505b0:     	mov	x3, x19
1016505b4:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016505b8:     	cmp	w0, w27
1016505bc:     	b.ne	0x101650654 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f0>
1016505c0:     	mov	x0, x20
1016505c4:     	mov	x1, x27
1016505c8:     	mov	x2, x26
1016505cc:     	mov	x3, x19
1016505d0:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016505d4:     	cmp	w0, w27
1016505d8:     	b.ne	0x101650654 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f0>
1016505dc:     	mov	x0, x20
1016505e0:     	mov	x1, x24
1016505e4:     	mov	x2, x19
1016505e8:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016505ec:     	tbz	w0, #0x0, 0x10165065c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f8>
1016505f0:     	mov	x0, x20
1016505f4:     	mov	x1, x25
1016505f8:     	mov	x2, x24
1016505fc:     	mov	x3, x19
101650600:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101650604:     	mov	w24, w0
101650608:     	mov	x0, x20
10165060c:     	mov	x1, x22
101650610:     	mov	x2, x19
101650614:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101650618:     	tbnz	w0, #0x0, 0x1016506a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x344>
10165061c:     	ldr	w2, [x20, #0x188]
101650620:     	mov	x0, x20
101650624:     	mov	w1, #0x8                ; =8
101650628:     	mov	x3, x23
10165062c:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
101650630:     	mov	x1, x0
101650634:     	mov	x0, x20
101650638:     	mov	x2, x22
10165063c:     	mov	x3, x19
101650640:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101650644:     	mov	x1, x0
101650648:     	mov	x0, x20
10165064c:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
101650650:     	b	0x1016506bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x358>
101650654:     	mov	x0, #0x0                ; =0
101650658:     	b	0x10165073c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d8>
10165065c:     	ldr	w2, [x20, #0x188]
101650660:     	mov	x0, x20
101650664:     	mov	w1, #0x8                ; =8
101650668:     	mov	x3, x25
10165066c:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
101650670:     	mov	x1, x0
101650674:     	mov	x0, x20
101650678:     	mov	x2, x24
10165067c:     	mov	x3, x19
101650680:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101650684:     	mov	x1, x0
101650688:     	mov	x0, x20
10165068c:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
101650690:     	mov	w24, w0
101650694:     	mov	x0, x20
101650698:     	mov	x1, x22
10165069c:     	mov	x2, x19
1016506a0:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016506a4:     	tbz	w0, #0x0, 0x10165061c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2b8>
1016506a8:     	mov	x0, x20
1016506ac:     	mov	x1, x23
1016506b0:     	mov	x2, x22
1016506b4:     	mov	x3, x19
1016506b8:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016506bc:     	mov	w2, w0
1016506c0:     	mov	x0, x20
1016506c4:     	mov	x1, x24
1016506c8:     	mov	x3, x21
1016506cc:     	bl	0x101650b88 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps7relprodB8_>
1016506d0:     	mov	x21, x0
1016506d4:     	mov	x0, x20
1016506d8:     	mov	x1, x26
1016506dc:     	mov	x2, x19
1016506e0:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016506e4:     	tbz	w0, #0x0, 0x101650700 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x39c>
1016506e8:     	mov	x0, x20
1016506ec:     	mov	x1, x21
1016506f0:     	mov	x2, x26
1016506f4:     	mov	x3, x19
1016506f8:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016506fc:     	b	0x101650734 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d0>
101650700:     	ldr	w2, [x20, #0x188]
101650704:     	mov	x0, x20
101650708:     	mov	w1, #0x8                ; =8
10165070c:     	mov	x3, x21
101650710:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
101650714:     	mov	x1, x0
101650718:     	mov	x0, x20
10165071c:     	mov	x2, x26
101650720:     	mov	x3, x19
101650724:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101650728:     	mov	x1, x0
10165072c:     	mov	x0, x20
101650730:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
101650734:     	mov	w1, w0
101650738:     	mov	w0, #0x1                ; =1
10165073c:     	ldp	x29, x30, [sp, #0x100]
101650740:     	ldp	x20, x19, [sp, #0xf0]
101650744:     	ldp	x22, x21, [sp, #0xe0]
101650748:     	ldp	x24, x23, [sp, #0xd0]
10165074c:     	ldp	x26, x25, [sp, #0xc0]
101650750:     	ldp	x28, x27, [sp, #0xb0]
101650754:     	add	sp, sp, #0x110
101650758:     	ret
10165075c:     	adrp	x0, 0x101b4c000 <dyld_stub_binder+0x101b4c000>
101650760:     	add	x0, x0, #0xff0
101650764:     	adrp	x2, 0x101d30000 <dyld_stub_binder+0x101d30000>
101650768:     	add	x2, x2, #0x30
10165076c:     	mov	w1, #0x39               ; =57
101650770:     	bl	0x101a66248 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
101650774:     	adrp	x0, 0x101b4a000 <dyld_stub_binder+0x101b4a000>
101650778:     	add	x0, x0, #0xe72
10165077c:     	adrp	x2, 0x101d30000 <dyld_stub_binder+0x101d30000>
101650780:     	add	x2, x2, #0x18
101650784:     	mov	w1, #0x3d               ; =61
101650788:     	bl	0x101a66248 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
10165078c:     	adrp	x5, 0x101d30000 <dyld_stub_binder+0x101d30000>
101650790:     	add	x5, x5, #0x48
101650794:     	sub	x1, x29, #0x58
101650798:     	add	x2, sp, #0x18
10165079c:     	mov	w0, #0x0                ; =0
1016507a0:     	mov	x3, #0x0                ; =0
1016507a4:     	bl	0x101a66160 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1016507a8:     	adrp	x2, 0x101beb000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x12ef>
1016507ac:     	add	x2, x2, #0xf30
1016507b0:     	adrp	x5, 0x101d2a000 <dyld_stub_binder+0x101d2a000>
1016507b4:     	add	x5, x5, #0xdc0
1016507b8:     	add	x1, sp, #0x18
1016507bc:     	mov	w0, #0x0                ; =0
1016507c0:     	mov	x3, #0x0                ; =0
1016507c4:     	bl	0x101a66160 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
