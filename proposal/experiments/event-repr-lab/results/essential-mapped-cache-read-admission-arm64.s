
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100990338 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>:
100990338:     	sub	sp, sp, #0x40
10099033c:     	stp	x20, x19, [sp, #0x20]
100990340:     	stp	x29, x30, [sp, #0x30]
100990344:     	add	x29, sp, #0x30
100990348:     	ldr	w9, [x4, #0x10]
10099034c:     	cmp	w9, #0x2
100990350:     	b.hs	0x10099035c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x24>
100990354:     	strb	w9, [x0, #0x8]
100990358:     	b	0x10099041c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xe4>
10099035c:     	ldp	x12, x11, [x4]
100990360:     	mov	x8, #0xa9c5             ; =43461
100990364:     	movk	x8, #0x2e62, lsl #16
100990368:     	movk	x8, #0x7aea, lsl #32
10099036c:     	movk	x8, #0xf135, lsl #48
100990370:     	madd	x10, x9, x8, x12
100990374:     	madd	x10, x10, x8, x11
100990378:     	madd	x10, x10, x8, x5
10099037c:     	mul	x8, x10, x8
100990380:     	sub	x10, x2, #0x1
100990384:     	and	x8, x10, x8, ror #44
100990388:     	cbz	x2, 0x100990480 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x148>
10099038c:     	add	x8, x1, x8, lsl #6
100990390:     	ldrb	w10, [x8]
100990394:     	cmp	w10, #0xff
100990398:     	b.eq	0x100990468 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x130>
10099039c:     	ldr	x13, [x8, #0x38]
1009903a0:     	cmp	x13, x5
1009903a4:     	b.ne	0x100990450 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1009903a8:     	ldr	x13, [x8, #0x20]
1009903ac:     	cmp	x13, x12
1009903b0:     	b.ne	0x100990450 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1009903b4:     	ldr	x12, [x8, #0x28]
1009903b8:     	cmp	x12, x11
1009903bc:     	b.ne	0x100990450 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1009903c0:     	ldr	w11, [x8, #0x30]
1009903c4:     	cmp	w11, w9
1009903c8:     	b.ne	0x100990450 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
1009903cc:     	cbz	w10, 0x100990414 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xdc>
1009903d0:     	cmp	w10, #0x1
1009903d4:     	b.ne	0x100990434 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xfc>
1009903d8:     	mov	x20, x0
1009903dc:     	ldr	w19, [x8, #0x4]
1009903e0:     	mov	x0, sp
1009903e4:     	add	x1, x3, #0x40
1009903e8:     	mov	x2, x19
1009903ec:     	bl	0x1010b8900 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1009903f0:     	ldr	w8, [sp]
1009903f4:     	cmp	w8, #0x1
1009903f8:     	b.ne	0x100990494 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x15c>
1009903fc:     	ldp	x8, x9, [sp, #0x8]
100990400:     	sbfx	x10, x19, #0, #1
100990404:     	mov	x11, #-0x1              ; =-1
100990408:     	stp	x11, x8, [x20]
10099040c:     	stp	x9, x10, [x20, #0x10]
100990410:     	b	0x100990424 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
100990414:     	ldrb	w8, [x8, #0x1]
100990418:     	strb	w8, [x0, #0x8]
10099041c:     	mov	x8, #-0x2               ; =-2
100990420:     	str	x8, [x0]
100990424:     	ldp	x29, x30, [sp, #0x30]
100990428:     	ldp	x20, x19, [sp, #0x20]
10099042c:     	add	sp, sp, #0x40
100990430:     	ret
100990434:     	ldr	q0, [x8, #0x10]
100990438:     	ldr	x8, [x8, #0x8]
10099043c:     	mov	x9, #-0x1               ; =-1
100990440:     	str	x9, [x0]
100990444:     	stur	q0, [x0, #0x8]
100990448:     	str	x8, [x0, #0x18]
10099044c:     	b	0x100990424 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
100990450:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
100990454:     	add	x0, x0, #0xea8
100990458:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
10099045c:     	add	x2, x2, #0xd60
100990460:     	mov	w1, #0x51               ; =81
100990464:     	bl	0x1016e73bc <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100990468:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
10099046c:     	add	x0, x0, #0xe9a
100990470:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
100990474:     	add	x2, x2, #0xd48
100990478:     	mov	w1, #0xe                ; =14
10099047c:     	bl	0x1016e7664 <__RNvNtCs4sDCw1iE1MS_4core6option13expect_failed>
100990480:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
100990484:     	add	x2, x2, #0xd30
100990488:     	mov	x0, x8
10099048c:     	mov	x1, #0x0                ; =0
100990490:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100990494:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
100990498:     	add	x0, x0, #0xed0
10099049c:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
1009904a0:     	add	x2, x2, #0xd78
1009904a4:     	mov	w1, #0xaf               ; =175
1009904a8:     	bl	0x1016e73bc <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
