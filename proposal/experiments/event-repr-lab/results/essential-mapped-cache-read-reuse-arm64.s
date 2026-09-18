
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001007ae438 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>:
1007ae438:     	sub	sp, sp, #0x40
1007ae43c:     	stp	x20, x19, [sp, #0x20]
1007ae440:     	stp	x29, x30, [sp, #0x30]
1007ae444:     	add	x29, sp, #0x30
1007ae448:     	ldr	w9, [x4, #0x10]
1007ae44c:     	cmp	w9, #0x2
1007ae450:     	b.hs	0x1007ae45c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x24>
1007ae454:     	strb	w9, [x0, #0x8]
1007ae458:     	b	0x1007ae51c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xe4>
1007ae45c:     	ldp	x12, x11, [x4]
1007ae460:     	mov	x8, #0xa9c5             ; =43461
1007ae464:     	movk	x8, #0x2e62, lsl #16
1007ae468:     	movk	x8, #0x7aea, lsl #32
1007ae46c:     	movk	x8, #0xf135, lsl #48
1007ae470:     	madd	x10, x9, x8, x12
1007ae474:     	madd	x10, x10, x8, x11
1007ae478:     	madd	x10, x10, x8, x5
1007ae47c:     	mul	x8, x10, x8
1007ae480:     	sub	x10, x2, #0x1
1007ae484:     	and	x8, x10, x8, ror #44
1007ae488:     	cbz	x2, 0x1007ae580 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x148>
1007ae48c:     	add	x8, x1, x8, lsl #6
1007ae490:     	ldrb	w10, [x8]
1007ae494:     	cmp	w10, #0xff
1007ae498:     	b.eq	0x1007ae568 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x130>
1007ae49c:     	ldr	x13, [x8, #0x38]
1007ae4a0:     	cmp	x13, x5
1007ae4a4:     	b.ne	0x1007ae550 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1007ae4a8:     	ldr	x13, [x8, #0x20]
1007ae4ac:     	cmp	x13, x12
1007ae4b0:     	b.ne	0x1007ae550 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1007ae4b4:     	ldr	x12, [x8, #0x28]
1007ae4b8:     	cmp	x12, x11
1007ae4bc:     	b.ne	0x1007ae550 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1007ae4c0:     	ldr	w11, [x8, #0x30]
1007ae4c4:     	cmp	w11, w9
1007ae4c8:     	b.ne	0x1007ae550 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1007ae4cc:     	cbz	w10, 0x1007ae514 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xdc>
1007ae4d0:     	cmp	w10, #0x1
1007ae4d4:     	b.ne	0x1007ae534 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xfc>
1007ae4d8:     	mov	x20, x0
1007ae4dc:     	ldr	w19, [x8, #0x4]
1007ae4e0:     	mov	x0, sp
1007ae4e4:     	add	x1, x3, #0x40
1007ae4e8:     	mov	x2, x19
1007ae4ec:     	bl	0x100d9d3c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1007ae4f0:     	ldr	w8, [sp]
1007ae4f4:     	cmp	w8, #0x1
1007ae4f8:     	b.ne	0x1007ae594 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x15c>
1007ae4fc:     	ldp	x8, x9, [sp, #0x8]
1007ae500:     	sbfx	x10, x19, #0, #1
1007ae504:     	mov	x11, #-0x1              ; =-1
1007ae508:     	stp	x11, x8, [x20]
1007ae50c:     	stp	x9, x10, [x20, #0x10]
1007ae510:     	b	0x1007ae524 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
1007ae514:     	ldrb	w8, [x8, #0x1]
1007ae518:     	strb	w8, [x0, #0x8]
1007ae51c:     	mov	x8, #-0x2               ; =-2
1007ae520:     	str	x8, [x0]
1007ae524:     	ldp	x29, x30, [sp, #0x30]
1007ae528:     	ldp	x20, x19, [sp, #0x20]
1007ae52c:     	add	sp, sp, #0x40
1007ae530:     	ret
1007ae534:     	ldr	q0, [x8, #0x10]
1007ae538:     	ldr	x8, [x8, #0x8]
1007ae53c:     	mov	x9, #-0x1               ; =-1
1007ae540:     	str	x9, [x0]
1007ae544:     	stur	q0, [x0, #0x8]
1007ae548:     	str	x8, [x0, #0x18]
1007ae54c:     	b	0x1007ae524 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
1007ae550:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007ae554:     	add	x0, x0, #0x270
1007ae558:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007ae55c:     	add	x2, x2, #0xd60
1007ae560:     	mov	w1, #0x51               ; =81
1007ae564:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1007ae568:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007ae56c:     	add	x0, x0, #0x262
1007ae570:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007ae574:     	add	x2, x2, #0xd48
1007ae578:     	mov	w1, #0xe                ; =14
1007ae57c:     	bl	0x1013ba4a4 <__RNvNtCs4sDCw1iE1MS_4core6option13expect_failed>
1007ae580:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007ae584:     	add	x2, x2, #0xd30
1007ae588:     	mov	x0, x8
1007ae58c:     	mov	x1, #0x0                ; =0
1007ae590:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007ae594:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007ae598:     	add	x0, x0, #0x298
1007ae59c:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007ae5a0:     	add	x2, x2, #0xd78
1007ae5a4:     	mov	w1, #0xaf               ; =175
1007ae5a8:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
