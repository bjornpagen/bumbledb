
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001009904ac <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_>:
1009904ac:     	sub	sp, sp, #0xa0
1009904b0:     	stp	x28, x27, [sp, #0x40]
1009904b4:     	stp	x26, x25, [sp, #0x50]
1009904b8:     	stp	x24, x23, [sp, #0x60]
1009904bc:     	stp	x22, x21, [sp, #0x70]
1009904c0:     	stp	x20, x19, [sp, #0x80]
1009904c4:     	stp	x29, x30, [sp, #0x90]
1009904c8:     	add	x29, sp, #0x90
1009904cc:     	ldr	w24, [x2, #0x10]
1009904d0:     	cmp	w24, #0x2
1009904d4:     	b.lo	0x10099077c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
1009904d8:     	mov	x19, x4
1009904dc:     	mov	x20, x0
1009904e0:     	ldr	x8, [x0, #0x8]
1009904e4:     	cbz	x8, 0x1009904f4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x48>
1009904e8:     	ldr	x0, [x20]
1009904ec:     	mov	x6, x8
1009904f0:     	b	0x1009905c0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x114>
1009904f4:     	sub	x8, x6, #0x1
1009904f8:     	eor	x9, x6, x8
1009904fc:     	cmp	x9, x8
100990500:     	b.ls	0x100990804 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x358>
100990504:     	lsl	x21, x6, #6
100990508:     	lsr	x8, x6, #58
10099050c:     	mov	x9, #-0x7               ; =-7
100990510:     	movk	x9, #0x7fff, lsl #48
100990514:     	cmp	x21, x9
100990518:     	ccmp	x8, #0x0, #0x0, lo
10099051c:     	b.eq	0x100990524 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x78>
100990520:     	bl	0x1016e6d58 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
100990524:     	cbz	x21, 0x100990588 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xdc>
100990528:     	mov	x0, x21
10099052c:     	mov	x22, x7
100990530:     	mov	x26, x5
100990534:     	mov	x23, x3
100990538:     	mov	x27, x2
10099053c:     	mov	x25, x1
100990540:     	mov	x28, x6
100990544:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
100990548:     	mov	x6, x28
10099054c:     	mov	x1, x25
100990550:     	mov	x2, x27
100990554:     	mov	x3, x23
100990558:     	mov	x5, x26
10099055c:     	mov	x7, x22
100990560:     	cbz	x0, 0x10099081c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x370>
100990564:     	cmp	x6, #0x3
100990568:     	b.hi	0x100990594 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xe8>
10099056c:     	mov	w8, #0xff               ; =255
100990570:     	mov	x9, x0
100990574:     	mov	x10, x6
100990578:     	strb	w8, [x9], #0x40
10099057c:     	subs	x10, x10, #0x1
100990580:     	b.ne	0x100990578 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xcc>
100990584:     	b	0x1009905bc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x110>
100990588:     	mov	w0, #0x8                ; =8
10099058c:     	cmp	x6, #0x3
100990590:     	b.ls	0x10099056c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xc0>
100990594:     	and	x8, x6, #0x3fffffffffffffc
100990598:     	add	x9, x0, #0x80
10099059c:     	mov	w10, #0xff              ; =255
1009905a0:     	sturb	w10, [x9, #-0x80]
1009905a4:     	sturb	w10, [x9, #-0x40]
1009905a8:     	strb	w10, [x9]
1009905ac:     	strb	w10, [x9, #0x40]
1009905b0:     	add	x9, x9, #0x100
1009905b4:     	subs	x8, x8, #0x4
1009905b8:     	b.ne	0x1009905a0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0xf4>
1009905bc:     	stp	x0, x6, [x20]
1009905c0:     	ldp	x11, x12, [x2]
1009905c4:     	ldr	w10, [x2, #0x14]
1009905c8:     	mov	x8, #0xa9c5             ; =43461
1009905cc:     	movk	x8, #0x2e62, lsl #16
1009905d0:     	movk	x8, #0x7aea, lsl #32
1009905d4:     	movk	x8, #0xf135, lsl #48
1009905d8:     	madd	x9, x24, x8, x11
1009905dc:     	madd	x9, x9, x8, x12
1009905e0:     	madd	x9, x9, x8, x19
1009905e4:     	mul	x8, x9, x8
1009905e8:     	sub	x9, x6, #0x1
1009905ec:     	and	x8, x9, x8, ror #44
1009905f0:     	add	x28, x0, x8, lsl #6
1009905f4:     	ldrb	w8, [x28]
1009905f8:     	cmp	w8, #0xff
1009905fc:     	str	w10, [sp, #0x1c]
100990600:     	stp	x12, x11, [sp, #0x8]
100990604:     	b.ne	0x100990624 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x178>
100990608:     	ldr	x8, [x7, #0x58]
10099060c:     	add	x8, x8, #0x1
100990610:     	str	x8, [x7, #0x58]
100990614:     	add	x8, x28, #0x10
100990618:     	str	x8, [sp]
10099061c:     	add	x25, x28, #0x18
100990620:     	b	0x1009906e8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
100990624:     	ldr	x9, [x28, #0x38]
100990628:     	cmp	x9, x19
10099062c:     	b.ne	0x100990664 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
100990630:     	ldr	x9, [x28, #0x20]
100990634:     	cmp	x9, x11
100990638:     	b.ne	0x100990664 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
10099063c:     	ldr	x9, [x28, #0x28]
100990640:     	cmp	x9, x12
100990644:     	b.ne	0x100990664 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
100990648:     	ldr	w9, [x28, #0x30]
10099064c:     	cmp	w9, w24
100990650:     	b.ne	0x100990664 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x1b8>
100990654:     	ldr	x8, [x7, #0x50]
100990658:     	add	x8, x8, #0x1
10099065c:     	str	x8, [x7, #0x50]
100990660:     	b	0x10099077c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2d0>
100990664:     	ldr	x9, [x7, #0x58]
100990668:     	add	x9, x9, #0x1
10099066c:     	str	x9, [x7, #0x58]
100990670:     	mov	x25, x28
100990674:     	ldr	x9, [x25, #0x18]!
100990678:     	mov	w10, #0xff              ; =255
10099067c:     	strb	w10, [x28]
100990680:     	lsl	x10, x9, #3
100990684:     	cmp	w8, #0x2
100990688:     	csel	x10, x10, xzr, eq
10099068c:     	ldr	x11, [x20, #0x10]
100990690:     	sub	x10, x11, x10
100990694:     	mov	x11, x28
100990698:     	ldr	x0, [x11, #0x10]!
10099069c:     	str	x11, [sp]
1009906a0:     	cmp	w8, #0x2
1009906a4:     	ldr	x8, [x7, #0x60]
1009906a8:     	add	x8, x8, #0x1
1009906ac:     	str	x8, [x7, #0x60]
1009906b0:     	str	x10, [x20, #0x10]
1009906b4:     	ccmp	x9, #0x0, #0x4, hs
1009906b8:     	b.eq	0x1009906e8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x23c>
1009906bc:     	mov	x21, x7
1009906c0:     	mov	x26, x5
1009906c4:     	mov	x22, x3
1009906c8:     	mov	x27, x2
1009906cc:     	mov	x23, x1
1009906d0:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009906d4:     	mov	x1, x23
1009906d8:     	mov	x2, x27
1009906dc:     	mov	x3, x22
1009906e0:     	mov	x5, x26
1009906e4:     	mov	x7, x21
1009906e8:     	add	x0, sp, #0x20
1009906ec:     	mov	x4, x19
1009906f0:     	mov	x6, x7
1009906f4:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1009906f8:     	ldr	x8, [sp, #0x20]
1009906fc:     	cmn	x8, #0x1
100990700:     	b.eq	0x100990718 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x26c>
100990704:     	cmn	x8, #0x2
100990708:     	b.ne	0x10099079c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2f0>
10099070c:     	mov	w27, #0x0               ; =0
100990710:     	ldrb	w26, [sp, #0x28]
100990714:     	b	0x10099071c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x270>
100990718:     	mov	w27, #0x1               ; =1
10099071c:     	mov	x21, #0x0               ; =0
100990720:     	ldr	x8, [x20, #0x10]
100990724:     	add	x8, x8, x21
100990728:     	str	x8, [x20, #0x10]
10099072c:     	ldrb	w8, [x28]
100990730:     	cmp	w8, #0x2
100990734:     	b.ne	0x100990754 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
100990738:     	ldr	x8, [x25]
10099073c:     	cbz	x8, 0x100990754 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x2a8>
100990740:     	ldr	x8, [sp]
100990744:     	ldr	x0, [x8]
100990748:     	mov	x20, x9
10099074c:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100990750:     	mov	x9, x20
100990754:     	strb	w27, [x28]
100990758:     	strb	w26, [x28, #0x1]
10099075c:     	str	w24, [x28, #0x4]
100990760:     	stp	x9, x22, [x28, #0x8]
100990764:     	ldp	x8, x9, [sp, #0x8]
100990768:     	stp	x23, x9, [x28, #0x18]
10099076c:     	str	x8, [x28, #0x28]
100990770:     	ldr	w8, [sp, #0x1c]
100990774:     	stp	w24, w8, [x28, #0x30]
100990778:     	str	x19, [x28, #0x38]
10099077c:     	ldp	x29, x30, [sp, #0x90]
100990780:     	ldp	x20, x19, [sp, #0x80]
100990784:     	ldp	x22, x21, [sp, #0x70]
100990788:     	ldp	x24, x23, [sp, #0x60]
10099078c:     	ldp	x26, x25, [sp, #0x50]
100990790:     	ldp	x28, x27, [sp, #0x40]
100990794:     	add	sp, sp, #0xa0
100990798:     	ret
10099079c:     	ldp	x27, x23, [sp, #0x28]
1009907a0:     	ldr	x9, [sp, #0x38]
1009907a4:     	lsl	x21, x23, #3
1009907a8:     	cmp	x8, x23
1009907ac:     	b.ls	0x1009907e0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x334>
1009907b0:     	mov	x26, x9
1009907b4:     	cbz	x23, 0x1009907ec <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x340>
1009907b8:     	mov	x0, x27
1009907bc:     	mov	x1, x21
1009907c0:     	bl	0x1016efcd4 <dyld_stub_binder+0x1016efcd4>
1009907c4:     	mov	x22, x0
1009907c8:     	mov	x9, x26
1009907cc:     	cbnz	x0, 0x1009907fc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x350>
1009907d0:     	mov	w0, #0x8                ; =8
1009907d4:     	mov	x1, x21
1009907d8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1009907dc:     	brk	#0x1
1009907e0:     	mov	x22, x27
1009907e4:     	mov	w27, #0x2               ; =2
1009907e8:     	b	0x100990720 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
1009907ec:     	mov	x0, x27
1009907f0:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009907f4:     	mov	w22, #0x8               ; =8
1009907f8:     	mov	x9, x26
1009907fc:     	mov	w27, #0x2               ; =2
100990800:     	b	0x100990720 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache7prepareKm3_EBc_+0x274>
100990804:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
100990808:     	add	x0, x0, #0xf27
10099080c:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
100990810:     	add	x2, x2, #0xd90
100990814:     	mov	w1, #0x2c               ; =44
100990818:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
10099081c:     	mov	w0, #0x8                ; =8
100990820:     	mov	x1, x21
100990824:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100990828:     	mov	x19, x0
10099082c:     	mov	x0, x27
100990830:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100990834:     	mov	x0, x19
100990838:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
		...
