
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010fd2a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_>:
1010fd2a8:     	sub	sp, sp, #0x130
1010fd2ac:     	stp	x28, x27, [sp, #0xd0]
1010fd2b0:     	stp	x26, x25, [sp, #0xe0]
1010fd2b4:     	stp	x24, x23, [sp, #0xf0]
1010fd2b8:     	stp	x22, x21, [sp, #0x100]
1010fd2bc:     	stp	x20, x19, [sp, #0x110]
1010fd2c0:     	stp	x29, x30, [sp, #0x120]
1010fd2c4:     	add	x29, sp, #0x120
1010fd2c8:     	str	x7, [sp, #0x28]
1010fd2cc:     	mov	x21, x6
1010fd2d0:     	mov	x28, x5
1010fd2d4:     	mov	x26, x4
1010fd2d8:     	mov	x20, x3
1010fd2dc:     	mov	x23, x2
1010fd2e0:     	mov	x19, x0
1010fd2e4:     	lsr	x24, x1, #1
1010fd2e8:     	tbnz	w1, #0x0, 0x1010fd30c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x64>
1010fd2ec:     	ldr	x22, [x29, #0x18]
1010fd2f0:     	lsr	x25, x26, #1
1010fd2f4:     	tbnz	w26, #0x0, 0x1010fd330 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x88>
1010fd2f8:     	ldr	x26, [x29, #0x10]
1010fd2fc:     	ldr	w27, [x19, #0x128]
1010fd300:     	cmp	w27, #0x1
1010fd304:     	b.ne	0x1010fd358 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0xb0>
1010fd308:     	b	0x1010fd4f0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1010fd30c:     	ldr	w2, [x19, #0x128]
1010fd310:     	mov	x0, x19
1010fd314:     	mov	w1, #0x4                ; =4
1010fd318:     	mov	x3, x24
1010fd31c:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd320:     	mov	x24, x0
1010fd324:     	ldr	x22, [x29, #0x18]
1010fd328:     	lsr	x25, x26, #1
1010fd32c:     	tbz	w26, #0x0, 0x1010fd2f8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x50>
1010fd330:     	ldr	w2, [x19, #0x128]
1010fd334:     	mov	x0, x19
1010fd338:     	mov	w1, #0x4                ; =4
1010fd33c:     	mov	x3, x25
1010fd340:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd344:     	mov	x25, x0
1010fd348:     	ldr	x26, [x29, #0x10]
1010fd34c:     	ldr	w27, [x19, #0x128]
1010fd350:     	cmp	w27, #0x1
1010fd354:     	b.eq	0x1010fd4f0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1010fd358:     	str	x28, [sp, #0x20]
1010fd35c:     	ldr	x8, [x19, #0x108]
1010fd360:     	mov	w9, w8
1010fd364:     	stp	x9, x20, [sp, #0x30]
1010fd368:     	cmp	x20, x9
1010fd36c:     	b.ne	0x1010fd5c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fd370:     	cbz	x20, 0x1010fd474 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1cc>
1010fd374:     	mov	x10, #0x0               ; =0
1010fd378:     	lsl	x28, x20, #2
1010fd37c:     	mov	w11, #0x1               ; =1
1010fd380:     	mov	x12, x28
1010fd384:     	mov	x13, x23
1010fd388:     	ldr	w14, [x13], #0x4
1010fd38c:     	cmp	w14, w8
1010fd390:     	b.hs	0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd394:     	lsr	x15, x10, x14
1010fd398:     	tbnz	w15, #0x0, 0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd39c:     	lsl	x14, x11, x14
1010fd3a0:     	orr	x10, x14, x10
1010fd3a4:     	subs	x12, x12, #0x4
1010fd3a8:     	b.ne	0x1010fd388 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0xe0>
1010fd3ac:     	str	x21, [sp, #0x38]
1010fd3b0:     	cmp	x21, x20
1010fd3b4:     	b.ne	0x1010fd5c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fd3b8:     	mov	x10, #0x0               ; =0
1010fd3bc:     	mov	w11, #0x1               ; =1
1010fd3c0:     	mov	x12, x28
1010fd3c4:     	ldr	x13, [sp, #0x20]
1010fd3c8:     	ldr	w14, [x13], #0x4
1010fd3cc:     	cmp	w14, w8
1010fd3d0:     	b.hs	0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd3d4:     	lsr	x15, x10, x14
1010fd3d8:     	tbnz	w15, #0x0, 0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd3dc:     	lsl	x14, x11, x14
1010fd3e0:     	orr	x10, x14, x10
1010fd3e4:     	subs	x12, x12, #0x4
1010fd3e8:     	b.ne	0x1010fd3c8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x120>
1010fd3ec:     	str	x22, [sp, #0x38]
1010fd3f0:     	cmp	x22, x20
1010fd3f4:     	b.ne	0x1010fd5c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fd3f8:     	mov	x10, #0x0               ; =0
1010fd3fc:     	mov	w11, #0x1               ; =1
1010fd400:     	mov	x12, x28
1010fd404:     	mov	x13, x26
1010fd408:     	ldr	w14, [x13], #0x4
1010fd40c:     	cmp	w14, w8
1010fd410:     	b.hs	0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd414:     	lsr	x15, x10, x14
1010fd418:     	tbnz	w15, #0x0, 0x1010fd5a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fd41c:     	lsl	x14, x11, x14
1010fd420:     	orr	x10, x14, x10
1010fd424:     	subs	x12, x12, #0x4
1010fd428:     	b.ne	0x1010fd408 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x160>
1010fd42c:     	ldr	x8, [sp, #0x28]
1010fd430:     	lsr	x8, x8, x9
1010fd434:     	str	x8, [sp, #0x38]
1010fd438:     	cbnz	x8, 0x1010fd5dc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x334>
1010fd43c:     	mov	x0, x28
1010fd440:     	mov	w1, #0x1                ; =1
1010fd444:     	bl	0x10150f0e4 <dyld_stub_binder+0x10150f0e4>
1010fd448:     	cbz	x0, 0x1010fd610 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x368>
1010fd44c:     	mov	x21, x0
1010fd450:     	mov	x8, #0x0                ; =0
1010fd454:     	ldr	w0, [x23, x8, lsl #2]
1010fd458:     	cmp	x20, x0
1010fd45c:     	b.ls	0x1010fd5fc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x354>
1010fd460:     	str	w8, [x21, x0, lsl #2]
1010fd464:     	add	x8, x8, #0x1
1010fd468:     	subs	x28, x28, #0x4
1010fd46c:     	b.ne	0x1010fd454 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ac>
1010fd470:     	b	0x1010fd494 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ec>
1010fd474:     	str	x21, [sp, #0x38]
1010fd478:     	cbnz	x21, 0x1010fd5c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fd47c:     	str	x22, [sp, #0x38]
1010fd480:     	cbnz	x22, 0x1010fd5c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fd484:     	ldr	x8, [sp, #0x28]
1010fd488:     	str	x8, [sp, #0x38]
1010fd48c:     	cbnz	x8, 0x1010fd5dc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x334>
1010fd490:     	mov	w21, #0x4               ; =4
1010fd494:     	mov	x0, x19
1010fd498:     	mov	x1, x27
1010fd49c:     	mov	x2, x21
1010fd4a0:     	mov	x3, x20
1010fd4a4:     	bl	0x100dfabe0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1010fd4a8:     	ldr	x28, [sp, #0x20]
1010fd4ac:     	mov	x3, x0
1010fd4b0:     	mov	x0, x19
1010fd4b4:     	mov	w1, #0x8                ; =8
1010fd4b8:     	mov	x2, x24
1010fd4bc:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd4c0:     	mov	x24, x0
1010fd4c4:     	ldr	w1, [x19, #0x128]
1010fd4c8:     	mov	x0, x19
1010fd4cc:     	mov	x2, x26
1010fd4d0:     	mov	x3, x20
1010fd4d4:     	bl	0x100dfabe0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1010fd4d8:     	mov	x27, x0
1010fd4dc:     	cbz	x20, 0x1010fd4e8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x240>
1010fd4e0:     	mov	x0, x21
1010fd4e4:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1010fd4e8:     	mov	x22, x20
1010fd4ec:     	mov	x21, x20
1010fd4f0:     	stp	x26, x22, [sp, #0x8]
1010fd4f4:     	add	x0, sp, #0x38
1010fd4f8:     	ldr	x8, [sp, #0x28]
1010fd4fc:     	str	x8, [sp]
1010fd500:     	mov	x1, x19
1010fd504:     	mov	x2, x24
1010fd508:     	mov	x3, x23
1010fd50c:     	mov	x4, x20
1010fd510:     	mov	x5, x25
1010fd514:     	mov	x6, x28
1010fd518:     	mov	x7, x21
1010fd51c:     	bl	0x100edf720 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
1010fd520:     	ldr	w3, [sp, #0x38]
1010fd524:     	mov	x0, x19
1010fd528:     	mov	w1, #0x8                ; =8
1010fd52c:     	mov	x2, x27
1010fd530:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd534:     	mov	x3, x0
1010fd538:     	ldr	w2, [x19, #0x128]
1010fd53c:     	mov	x0, x19
1010fd540:     	mov	w1, #0x8                ; =8
1010fd544:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd548:     	mov	x20, x0
1010fd54c:     	ldr	x2, [x19, #0x118]
1010fd550:     	mov	x0, x19
1010fd554:     	mov	x1, x20
1010fd558:     	bl	0x100dec3b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010fd55c:     	mov	x21, x0
1010fd560:     	cbz	w0, 0x1010fd57c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x2d4>
1010fd564:     	ldr	w2, [x19, #0x128]
1010fd568:     	mov	x0, x19
1010fd56c:     	mov	w1, #0x4                ; =4
1010fd570:     	mov	x3, x20
1010fd574:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1010fd578:     	mov	x20, x0
1010fd57c:     	mov	w8, w20
1010fd580:     	mov	w9, w21
1010fd584:     	orr	x0, x9, x8, lsl #1
1010fd588:     	ldp	x29, x30, [sp, #0x120]
1010fd58c:     	ldp	x20, x19, [sp, #0x110]
1010fd590:     	ldp	x22, x21, [sp, #0x100]
1010fd594:     	ldp	x24, x23, [sp, #0xf0]
1010fd598:     	ldp	x26, x25, [sp, #0xe0]
1010fd59c:     	ldp	x28, x27, [sp, #0xd0]
1010fd5a0:     	add	sp, sp, #0x130
1010fd5a4:     	ret
1010fd5a8:     	adrp	x0, 0x10166a000 <__RNvNvXsi_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab8transferNtB7_5ErrorNtNtCs4sDCw1iE1MS_4core3fmt5Debug3fmt8___OFFSET+0x1ec8>
1010fd5ac:     	add	x0, x0, #0xc7c
1010fd5b0:     	adrp	x2, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fd5b4:     	add	x2, x2, #0x168
1010fd5b8:     	mov	w1, #0x41               ; =65
1010fd5bc:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1010fd5c0:     	adrp	x5, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fd5c4:     	add	x5, x5, #0x150
1010fd5c8:     	add	x1, sp, #0x38
1010fd5cc:     	add	x2, sp, #0x30
1010fd5d0:     	mov	w0, #0x0                ; =0
1010fd5d4:     	mov	x3, #0x0                ; =0
1010fd5d8:     	bl	0x101506cb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1010fd5dc:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
1010fd5e0:     	add	x2, x2, #0x268
1010fd5e4:     	adrp	x5, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fd5e8:     	add	x5, x5, #0x138
1010fd5ec:     	add	x1, sp, #0x38
1010fd5f0:     	mov	w0, #0x0                ; =0
1010fd5f4:     	mov	x3, #0x0                ; =0
1010fd5f8:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1010fd5fc:     	adrp	x2, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fd600:     	add	x2, x2, #0x120
1010fd604:     	mov	x1, x20
1010fd608:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010fd60c:     	brk	#0x1
1010fd610:     	mov	w0, #0x4                ; =4
1010fd614:     	mov	x1, x28
1010fd618:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1010fd61c:     	mov	x19, x0
1010fd620:     	cbnz	x20, 0x1010fd62c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x384>
1010fd624:     	b	0x1010fd634 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x38c>
1010fd628:     	mov	x19, x0
1010fd62c:     	mov	x0, x21
1010fd630:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1010fd634:     	mov	x0, x19
1010fd638:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
1010fd63c:     	nop
