
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012da3bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_>:
1012da3bc:     	sub	sp, sp, #0x130
1012da3c0:     	stp	x28, x27, [sp, #0xd0]
1012da3c4:     	stp	x26, x25, [sp, #0xe0]
1012da3c8:     	stp	x24, x23, [sp, #0xf0]
1012da3cc:     	stp	x22, x21, [sp, #0x100]
1012da3d0:     	stp	x20, x19, [sp, #0x110]
1012da3d4:     	stp	x29, x30, [sp, #0x120]
1012da3d8:     	add	x29, sp, #0x120
1012da3dc:     	str	x7, [sp, #0x28]
1012da3e0:     	mov	x21, x6
1012da3e4:     	mov	x28, x5
1012da3e8:     	mov	x26, x4
1012da3ec:     	mov	x20, x3
1012da3f0:     	mov	x23, x2
1012da3f4:     	mov	x19, x0
1012da3f8:     	lsr	x24, x1, #1
1012da3fc:     	tbnz	w1, #0x0, 0x1012da420 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x64>
1012da400:     	ldr	x22, [x29, #0x18]
1012da404:     	lsr	x25, x26, #1
1012da408:     	tbnz	w26, #0x0, 0x1012da444 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x88>
1012da40c:     	ldr	x26, [x29, #0x10]
1012da410:     	ldr	w27, [x19, #0x148]
1012da414:     	cmp	w27, #0x1
1012da418:     	b.ne	0x1012da46c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0xb0>
1012da41c:     	b	0x1012da604 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1012da420:     	ldr	w2, [x19, #0x148]
1012da424:     	mov	x0, x19
1012da428:     	mov	w1, #0x4                ; =4
1012da42c:     	mov	x3, x24
1012da430:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da434:     	mov	x24, x0
1012da438:     	ldr	x22, [x29, #0x18]
1012da43c:     	lsr	x25, x26, #1
1012da440:     	tbz	w26, #0x0, 0x1012da40c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x50>
1012da444:     	ldr	w2, [x19, #0x148]
1012da448:     	mov	x0, x19
1012da44c:     	mov	w1, #0x4                ; =4
1012da450:     	mov	x3, x25
1012da454:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da458:     	mov	x25, x0
1012da45c:     	ldr	x26, [x29, #0x10]
1012da460:     	ldr	w27, [x19, #0x148]
1012da464:     	cmp	w27, #0x1
1012da468:     	b.eq	0x1012da604 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1012da46c:     	str	x28, [sp, #0x20]
1012da470:     	ldr	x8, [x19, #0x108]
1012da474:     	mov	w9, w8
1012da478:     	stp	x9, x20, [sp, #0x30]
1012da47c:     	cmp	x20, x9
1012da480:     	b.ne	0x1012da6d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012da484:     	cbz	x20, 0x1012da588 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1cc>
1012da488:     	mov	x10, #0x0               ; =0
1012da48c:     	lsl	x28, x20, #2
1012da490:     	mov	w11, #0x1               ; =1
1012da494:     	mov	x12, x28
1012da498:     	mov	x13, x23
1012da49c:     	ldr	w14, [x13], #0x4
1012da4a0:     	cmp	w14, w8
1012da4a4:     	b.hs	0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da4a8:     	lsr	x15, x10, x14
1012da4ac:     	tbnz	w15, #0x0, 0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da4b0:     	lsl	x14, x11, x14
1012da4b4:     	orr	x10, x14, x10
1012da4b8:     	subs	x12, x12, #0x4
1012da4bc:     	b.ne	0x1012da49c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0xe0>
1012da4c0:     	str	x21, [sp, #0x38]
1012da4c4:     	cmp	x21, x20
1012da4c8:     	b.ne	0x1012da6d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012da4cc:     	mov	x10, #0x0               ; =0
1012da4d0:     	mov	w11, #0x1               ; =1
1012da4d4:     	mov	x12, x28
1012da4d8:     	ldr	x13, [sp, #0x20]
1012da4dc:     	ldr	w14, [x13], #0x4
1012da4e0:     	cmp	w14, w8
1012da4e4:     	b.hs	0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da4e8:     	lsr	x15, x10, x14
1012da4ec:     	tbnz	w15, #0x0, 0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da4f0:     	lsl	x14, x11, x14
1012da4f4:     	orr	x10, x14, x10
1012da4f8:     	subs	x12, x12, #0x4
1012da4fc:     	b.ne	0x1012da4dc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x120>
1012da500:     	str	x22, [sp, #0x38]
1012da504:     	cmp	x22, x20
1012da508:     	b.ne	0x1012da6d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012da50c:     	mov	x10, #0x0               ; =0
1012da510:     	mov	w11, #0x1               ; =1
1012da514:     	mov	x12, x28
1012da518:     	mov	x13, x26
1012da51c:     	ldr	w14, [x13], #0x4
1012da520:     	cmp	w14, w8
1012da524:     	b.hs	0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da528:     	lsr	x15, x10, x14
1012da52c:     	tbnz	w15, #0x0, 0x1012da6c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012da530:     	lsl	x14, x11, x14
1012da534:     	orr	x10, x14, x10
1012da538:     	subs	x12, x12, #0x4
1012da53c:     	b.ne	0x1012da51c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x160>
1012da540:     	ldr	x8, [sp, #0x28]
1012da544:     	lsr	x8, x8, x9
1012da548:     	str	x8, [sp, #0x38]
1012da54c:     	cbnz	x8, 0x1012da6f4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x338>
1012da550:     	mov	x0, x28
1012da554:     	mov	w1, #0x1                ; =1
1012da558:     	bl	0x1016ef824 <dyld_stub_binder+0x1016ef824>
1012da55c:     	cbz	x0, 0x1012da728 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x36c>
1012da560:     	mov	x21, x0
1012da564:     	mov	x8, #0x0                ; =0
1012da568:     	ldr	w0, [x23, x8, lsl #2]
1012da56c:     	cmp	x20, x0
1012da570:     	b.ls	0x1012da714 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x358>
1012da574:     	str	w8, [x21, x0, lsl #2]
1012da578:     	add	x8, x8, #0x1
1012da57c:     	subs	x28, x28, #0x4
1012da580:     	b.ne	0x1012da568 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ac>
1012da584:     	b	0x1012da5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ec>
1012da588:     	str	x21, [sp, #0x38]
1012da58c:     	cbnz	x21, 0x1012da6d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012da590:     	str	x22, [sp, #0x38]
1012da594:     	cbnz	x22, 0x1012da6d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012da598:     	ldr	x8, [sp, #0x28]
1012da59c:     	str	x8, [sp, #0x38]
1012da5a0:     	cbnz	x8, 0x1012da6f4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x338>
1012da5a4:     	mov	w21, #0x4               ; =4
1012da5a8:     	mov	x0, x19
1012da5ac:     	mov	x1, x27
1012da5b0:     	mov	x2, x21
1012da5b4:     	mov	x3, x20
1012da5b8:     	bl	0x100fb0d38 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6renameB6_>
1012da5bc:     	ldr	x28, [sp, #0x20]
1012da5c0:     	mov	x3, x0
1012da5c4:     	mov	x0, x19
1012da5c8:     	mov	w1, #0x8                ; =8
1012da5cc:     	mov	x2, x24
1012da5d0:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da5d4:     	mov	x24, x0
1012da5d8:     	ldr	w1, [x19, #0x148]
1012da5dc:     	mov	x0, x19
1012da5e0:     	mov	x2, x26
1012da5e4:     	mov	x3, x20
1012da5e8:     	bl	0x100fb0d38 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6renameB6_>
1012da5ec:     	mov	x27, x0
1012da5f0:     	cbz	x20, 0x1012da5fc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x240>
1012da5f4:     	mov	x0, x21
1012da5f8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1012da5fc:     	mov	x22, x20
1012da600:     	mov	x21, x20
1012da604:     	stp	x26, x22, [sp, #0x8]
1012da608:     	add	x0, sp, #0x38
1012da60c:     	ldr	x8, [sp, #0x28]
1012da610:     	str	x8, [sp]
1012da614:     	mov	x1, x19
1012da618:     	mov	x2, x24
1012da61c:     	mov	x3, x23
1012da620:     	mov	x4, x20
1012da624:     	mov	x5, x25
1012da628:     	mov	x6, x28
1012da62c:     	mov	x7, x21
1012da630:     	bl	0x1010ae29c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>
1012da634:     	ldr	w3, [sp, #0x38]
1012da638:     	mov	x0, x19
1012da63c:     	mov	w1, #0x8                ; =8
1012da640:     	mov	x2, x27
1012da644:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da648:     	mov	x3, x0
1012da64c:     	ldr	w2, [x19, #0x148]
1012da650:     	mov	x0, x19
1012da654:     	mov	w1, #0x8                ; =8
1012da658:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da65c:     	mov	x20, x0
1012da660:     	ldr	x2, [x19, #0x138]
1012da664:     	mov	x0, x19
1012da668:     	mov	x1, x20
1012da66c:     	bl	0x100fa5938 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1012da670:     	mov	x21, x0
1012da674:     	cbz	w0, 0x1012da690 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x2d4>
1012da678:     	ldr	w2, [x19, #0x148]
1012da67c:     	mov	x0, x19
1012da680:     	mov	w1, #0x4                ; =4
1012da684:     	mov	x3, x20
1012da688:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012da68c:     	mov	x20, x0
1012da690:     	mov	w8, w20
1012da694:     	mov	w9, w21
1012da698:     	orr	x1, x9, x8, lsl #1
1012da69c:     	mov	w0, #0x1                ; =1
1012da6a0:     	ldp	x29, x30, [sp, #0x120]
1012da6a4:     	ldp	x20, x19, [sp, #0x110]
1012da6a8:     	ldp	x22, x21, [sp, #0x100]
1012da6ac:     	ldp	x24, x23, [sp, #0xf0]
1012da6b0:     	ldp	x26, x25, [sp, #0xe0]
1012da6b4:     	ldp	x28, x27, [sp, #0xd0]
1012da6b8:     	add	sp, sp, #0x130
1012da6bc:     	ret
1012da6c0:     	adrp	x0, 0x101853000 <__RNvNvXsi_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab8transferNtB7_5ErrorNtNtCs4sDCw1iE1MS_4core3fmt5Debug3fmt8___OFFSET+0x1e68>
1012da6c4:     	add	x0, x0, #0xd3a
1012da6c8:     	adrp	x2, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012da6cc:     	add	x2, x2, #0x3e8
1012da6d0:     	mov	w1, #0x41               ; =65
1012da6d4:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012da6d8:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012da6dc:     	add	x5, x5, #0x3d0
1012da6e0:     	add	x1, sp, #0x38
1012da6e4:     	add	x2, sp, #0x30
1012da6e8:     	mov	w0, #0x0                ; =0
1012da6ec:     	mov	x3, #0x0                ; =0
1012da6f0:     	bl	0x1016e73f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1012da6f4:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1012da6f8:     	add	x2, x2, #0x358
1012da6fc:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012da700:     	add	x5, x5, #0x3b8
1012da704:     	add	x1, sp, #0x38
1012da708:     	mov	w0, #0x0                ; =0
1012da70c:     	mov	x3, #0x0                ; =0
1012da710:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012da714:     	adrp	x2, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012da718:     	add	x2, x2, #0x3a0
1012da71c:     	mov	x1, x20
1012da720:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012da724:     	brk	#0x1
1012da728:     	mov	w0, #0x4                ; =4
1012da72c:     	mov	x1, x28
1012da730:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1012da734:     	mov	x19, x0
1012da738:     	cbnz	x20, 0x1012da744 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x388>
1012da73c:     	b	0x1012da74c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x390>
1012da740:     	mov	x19, x0
1012da744:     	mov	x0, x21
1012da748:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1012da74c:     	mov	x0, x19
1012da750:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
