
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010089f42c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_>:
10089f42c:     	sub	sp, sp, #0xa0
10089f430:     	stp	x28, x27, [sp, #0x40]
10089f434:     	stp	x26, x25, [sp, #0x50]
10089f438:     	stp	x24, x23, [sp, #0x60]
10089f43c:     	stp	x22, x21, [sp, #0x70]
10089f440:     	stp	x20, x19, [sp, #0x80]
10089f444:     	stp	x29, x30, [sp, #0x90]
10089f448:     	add	x29, sp, #0x90
10089f44c:     	ldr	w24, [x2, #0x10]
10089f450:     	cmp	w24, #0x2
10089f454:     	b.lo	0x10089f6fc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
10089f458:     	mov	x19, x4
10089f45c:     	mov	x20, x0
10089f460:     	ldr	x8, [x0, #0x8]
10089f464:     	cbz	x8, 0x10089f474 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x48>
10089f468:     	ldr	x0, [x20]
10089f46c:     	mov	x6, x8
10089f470:     	b	0x10089f540 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x114>
10089f474:     	sub	x8, x6, #0x1
10089f478:     	eor	x9, x6, x8
10089f47c:     	cmp	x9, x8
10089f480:     	b.ls	0x10089f784 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x358>
10089f484:     	lsl	x21, x6, #6
10089f488:     	lsr	x8, x6, #58
10089f48c:     	mov	x9, #-0x7               ; =-7
10089f490:     	movk	x9, #0x7fff, lsl #48
10089f494:     	cmp	x21, x9
10089f498:     	ccmp	x8, #0x0, #0x0, lo
10089f49c:     	b.eq	0x10089f4a4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x78>
10089f4a0:     	bl	0x101506610 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
10089f4a4:     	cbz	x21, 0x10089f508 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xdc>
10089f4a8:     	mov	x0, x21
10089f4ac:     	mov	x22, x7
10089f4b0:     	mov	x26, x5
10089f4b4:     	mov	x23, x3
10089f4b8:     	mov	x27, x2
10089f4bc:     	mov	x25, x1
10089f4c0:     	mov	x28, x6
10089f4c4:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
10089f4c8:     	mov	x6, x28
10089f4cc:     	mov	x1, x25
10089f4d0:     	mov	x2, x27
10089f4d4:     	mov	x3, x23
10089f4d8:     	mov	x5, x26
10089f4dc:     	mov	x7, x22
10089f4e0:     	cbz	x0, 0x10089f79c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x370>
10089f4e4:     	cmp	x6, #0x3
10089f4e8:     	b.hi	0x10089f514 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xe8>
10089f4ec:     	mov	w8, #0xff               ; =255
10089f4f0:     	mov	x9, x0
10089f4f4:     	mov	x10, x6
10089f4f8:     	strb	w8, [x9], #0x40
10089f4fc:     	subs	x10, x10, #0x1
10089f500:     	b.ne	0x10089f4f8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xcc>
10089f504:     	b	0x10089f53c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x110>
10089f508:     	mov	w0, #0x8                ; =8
10089f50c:     	cmp	x6, #0x3
10089f510:     	b.ls	0x10089f4ec <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xc0>
10089f514:     	and	x8, x6, #0x3fffffffffffffc
10089f518:     	add	x9, x0, #0x80
10089f51c:     	mov	w10, #0xff              ; =255
10089f520:     	sturb	w10, [x9, #-0x80]
10089f524:     	sturb	w10, [x9, #-0x40]
10089f528:     	strb	w10, [x9]
10089f52c:     	strb	w10, [x9, #0x40]
10089f530:     	add	x9, x9, #0x100
10089f534:     	subs	x8, x8, #0x4
10089f538:     	b.ne	0x10089f520 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xf4>
10089f53c:     	stp	x0, x6, [x20]
10089f540:     	ldp	x11, x12, [x2]
10089f544:     	ldr	w10, [x2, #0x14]
10089f548:     	mov	x8, #0xa9c5             ; =43461
10089f54c:     	movk	x8, #0x2e62, lsl #16
10089f550:     	movk	x8, #0x7aea, lsl #32
10089f554:     	movk	x8, #0xf135, lsl #48
10089f558:     	madd	x9, x24, x8, x11
10089f55c:     	madd	x9, x9, x8, x12
10089f560:     	madd	x9, x9, x8, x19
10089f564:     	mul	x8, x9, x8
10089f568:     	sub	x9, x6, #0x1
10089f56c:     	and	x8, x9, x8, ror #44
10089f570:     	add	x28, x0, x8, lsl #6
10089f574:     	ldrb	w8, [x28]
10089f578:     	cmp	w8, #0xff
10089f57c:     	str	w10, [sp, #0x1c]
10089f580:     	stp	x12, x11, [sp, #0x8]
10089f584:     	b.ne	0x10089f5a4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x178>
10089f588:     	ldr	x8, [x7, #0x58]
10089f58c:     	add	x8, x8, #0x1
10089f590:     	str	x8, [x7, #0x58]
10089f594:     	add	x8, x28, #0x10
10089f598:     	str	x8, [sp]
10089f59c:     	add	x25, x28, #0x18
10089f5a0:     	b	0x10089f668 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
10089f5a4:     	ldr	x9, [x28, #0x38]
10089f5a8:     	cmp	x9, x19
10089f5ac:     	b.ne	0x10089f5e4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
10089f5b0:     	ldr	x9, [x28, #0x20]
10089f5b4:     	cmp	x9, x11
10089f5b8:     	b.ne	0x10089f5e4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
10089f5bc:     	ldr	x9, [x28, #0x28]
10089f5c0:     	cmp	x9, x12
10089f5c4:     	b.ne	0x10089f5e4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
10089f5c8:     	ldr	w9, [x28, #0x30]
10089f5cc:     	cmp	w9, w24
10089f5d0:     	b.ne	0x10089f5e4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
10089f5d4:     	ldr	x8, [x7, #0x50]
10089f5d8:     	add	x8, x8, #0x1
10089f5dc:     	str	x8, [x7, #0x50]
10089f5e0:     	b	0x10089f6fc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
10089f5e4:     	ldr	x9, [x7, #0x58]
10089f5e8:     	add	x9, x9, #0x1
10089f5ec:     	str	x9, [x7, #0x58]
10089f5f0:     	mov	x25, x28
10089f5f4:     	ldr	x9, [x25, #0x18]!
10089f5f8:     	mov	w10, #0xff              ; =255
10089f5fc:     	strb	w10, [x28]
10089f600:     	lsl	x10, x9, #3
10089f604:     	cmp	w8, #0x2
10089f608:     	csel	x10, x10, xzr, eq
10089f60c:     	ldr	x11, [x20, #0x10]
10089f610:     	sub	x10, x11, x10
10089f614:     	mov	x11, x28
10089f618:     	ldr	x0, [x11, #0x10]!
10089f61c:     	str	x11, [sp]
10089f620:     	cmp	w8, #0x2
10089f624:     	ldr	x8, [x7, #0x60]
10089f628:     	add	x8, x8, #0x1
10089f62c:     	str	x8, [x7, #0x60]
10089f630:     	str	x10, [x20, #0x10]
10089f634:     	ccmp	x9, #0x0, #0x4, hs
10089f638:     	b.eq	0x10089f668 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
10089f63c:     	mov	x21, x7
10089f640:     	mov	x26, x5
10089f644:     	mov	x22, x3
10089f648:     	mov	x27, x2
10089f64c:     	mov	x23, x1
10089f650:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
10089f654:     	mov	x1, x23
10089f658:     	mov	x2, x27
10089f65c:     	mov	x3, x22
10089f660:     	mov	x5, x26
10089f664:     	mov	x7, x21
10089f668:     	add	x0, sp, #0x20
10089f66c:     	mov	x4, x19
10089f670:     	mov	x6, x7
10089f674:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
10089f678:     	ldr	x8, [sp, #0x20]
10089f67c:     	cmn	x8, #0x1
10089f680:     	b.eq	0x10089f698 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x26c>
10089f684:     	cmn	x8, #0x2
10089f688:     	b.ne	0x10089f71c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2f0>
10089f68c:     	mov	w27, #0x0               ; =0
10089f690:     	ldrb	w26, [sp, #0x28]
10089f694:     	b	0x10089f69c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x270>
10089f698:     	mov	w27, #0x1               ; =1
10089f69c:     	mov	x21, #0x0               ; =0
10089f6a0:     	ldr	x8, [x20, #0x10]
10089f6a4:     	add	x8, x8, x21
10089f6a8:     	str	x8, [x20, #0x10]
10089f6ac:     	ldrb	w8, [x28]
10089f6b0:     	cmp	w8, #0x2
10089f6b4:     	b.ne	0x10089f6d4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
10089f6b8:     	ldr	x8, [x25]
10089f6bc:     	cbz	x8, 0x10089f6d4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
10089f6c0:     	ldr	x8, [sp]
10089f6c4:     	ldr	x0, [x8]
10089f6c8:     	mov	x20, x9
10089f6cc:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
10089f6d0:     	mov	x9, x20
10089f6d4:     	strb	w27, [x28]
10089f6d8:     	strb	w26, [x28, #0x1]
10089f6dc:     	str	w24, [x28, #0x4]
10089f6e0:     	stp	x9, x22, [x28, #0x8]
10089f6e4:     	ldp	x8, x9, [sp, #0x8]
10089f6e8:     	stp	x23, x9, [x28, #0x18]
10089f6ec:     	str	x8, [x28, #0x28]
10089f6f0:     	ldr	w8, [sp, #0x1c]
10089f6f4:     	stp	w24, w8, [x28, #0x30]
10089f6f8:     	str	x19, [x28, #0x38]
10089f6fc:     	ldp	x29, x30, [sp, #0x90]
10089f700:     	ldp	x20, x19, [sp, #0x80]
10089f704:     	ldp	x22, x21, [sp, #0x70]
10089f708:     	ldp	x24, x23, [sp, #0x60]
10089f70c:     	ldp	x26, x25, [sp, #0x50]
10089f710:     	ldp	x28, x27, [sp, #0x40]
10089f714:     	add	sp, sp, #0xa0
10089f718:     	ret
10089f71c:     	ldp	x27, x23, [sp, #0x28]
10089f720:     	ldr	x9, [sp, #0x38]
10089f724:     	lsl	x21, x23, #3
10089f728:     	cmp	x8, x23
10089f72c:     	b.ls	0x10089f760 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x334>
10089f730:     	mov	x26, x9
10089f734:     	cbz	x23, 0x10089f76c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x340>
10089f738:     	mov	x0, x27
10089f73c:     	mov	x1, x21
10089f740:     	bl	0x10150f594 <dyld_stub_binder+0x10150f594>
10089f744:     	mov	x22, x0
10089f748:     	mov	x9, x26
10089f74c:     	cbnz	x0, 0x10089f77c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x350>
10089f750:     	mov	w0, #0x8                ; =8
10089f754:     	mov	x1, x21
10089f758:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
10089f75c:     	brk	#0x1
10089f760:     	mov	x22, x27
10089f764:     	mov	w27, #0x2               ; =2
10089f768:     	b	0x10089f6a0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
10089f76c:     	mov	x0, x27
10089f770:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
10089f774:     	mov	w22, #0x8               ; =8
10089f778:     	mov	x9, x26
10089f77c:     	mov	w27, #0x2               ; =2
10089f780:     	b	0x10089f6a0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
10089f784:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
10089f788:     	add	x0, x0, #0x443
10089f78c:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
10089f790:     	add	x2, x2, #0xd90
10089f794:     	mov	w1, #0x2c               ; =44
10089f798:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
10089f79c:     	mov	w0, #0x8                ; =8
10089f7a0:     	mov	x1, x21
10089f7a4:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
10089f7a8:     	mov	x19, x0
10089f7ac:     	mov	x0, x27
10089f7b0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
10089f7b4:     	mov	x0, x19
10089f7b8:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
		...
