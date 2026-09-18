
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bc34d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>:
100bc34d0:     	sub	sp, sp, #0x90
100bc34d4:     	stp	x28, x27, [sp, #0x30]
100bc34d8:     	stp	x26, x25, [sp, #0x40]
100bc34dc:     	stp	x24, x23, [sp, #0x50]
100bc34e0:     	stp	x22, x21, [sp, #0x60]
100bc34e4:     	stp	x20, x19, [sp, #0x70]
100bc34e8:     	stp	x29, x30, [sp, #0x80]
100bc34ec:     	add	x29, sp, #0x80
100bc34f0:     	cbz	w1, 0x100bc38e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x414>
100bc34f4:     	ldr	x8, [x4, #0x18]
100bc34f8:     	cbz	x8, 0x100bc35c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bc34fc:     	mov	x8, #0x0                ; =0
100bc3500:     	mov	w9, w1
100bc3504:     	mov	x10, #0xa9c5            ; =43461
100bc3508:     	movk	x10, #0x2e62, lsl #16
100bc350c:     	movk	x10, #0x7aea, lsl #32
100bc3510:     	movk	x10, #0xf135, lsl #48
100bc3514:     	mul	x9, x9, x10
100bc3518:     	add	x9, x9, w2, uxtw
100bc351c:     	mul	x9, x9, x10
100bc3520:     	add	x9, x9, w3, uxtw
100bc3524:     	mul	x9, x9, x10
100bc3528:     	ror	x11, x9, #0x2c
100bc352c:     	lsr	x12, x11, #57
100bc3530:     	ldp	x10, x9, [x4]
100bc3534:     	dup.8b	v0, w12
100bc3538:     	movi.2d	v1, #0xffffffffffffffff
100bc353c:     	and	x11, x11, x9
100bc3540:     	ldr	d2, [x10, x11]
100bc3544:     	cmeq.8b	v3, v2, v0
100bc3548:     	fmov	x12, d3
100bc354c:     	ands	x12, x12, #0x8080808080808080
100bc3550:     	b.eq	0x100bc3598 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bc3554:     	rbit	x13, x12
100bc3558:     	clz	x13, x13
100bc355c:     	add	x13, x11, x13, lsr #3
100bc3560:     	and	x13, x13, x9
100bc3564:     	sub	x13, x10, x13, lsl #4
100bc3568:     	ldur	w14, [x13, #-0x10]
100bc356c:     	cmp	w1, w14
100bc3570:     	b.ne	0x100bc358c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bc3574:     	ldur	w14, [x13, #-0xc]
100bc3578:     	cmp	w2, w14
100bc357c:     	b.ne	0x100bc358c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xbc>
100bc3580:     	ldur	w14, [x13, #-0x8]
100bc3584:     	cmp	w3, w14
100bc3588:     	b.eq	0x100bc38ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x41c>
100bc358c:     	sub	x13, x12, #0x2
100bc3590:     	ands	x12, x13, x12
100bc3594:     	b.ne	0x100bc3554 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bc3598:     	cmeq.8b	v2, v2, v1
100bc359c:     	fmov	x12, d2
100bc35a0:     	cbnz	x12, 0x100bc35c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xf8>
100bc35a4:     	add	x8, x8, #0x8
100bc35a8:     	add	x11, x11, x8
100bc35ac:     	and	x11, x11, x9
100bc35b0:     	ldr	d2, [x10, x11]
100bc35b4:     	cmeq.8b	v3, v2, v0
100bc35b8:     	fmov	x12, d3
100bc35bc:     	ands	x12, x12, #0x8080808080808080
100bc35c0:     	b.ne	0x100bc3554 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x84>
100bc35c4:     	b	0x100bc3598 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0xc8>
100bc35c8:     	tbnz	w1, #0x1, 0x100bc38f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x424>
100bc35cc:     	ldr	x10, [x0, #0xc8]
100bc35d0:     	tbnz	w2, #0x1, 0x100bc3914 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x444>
100bc35d4:     	ldr	x8, [x0, #0xc8]
100bc35d8:     	cmp	x8, x10
100bc35dc:     	csel	x10, x8, x10, lo
100bc35e0:     	tbnz	w3, #0x1, 0x100bc393c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x46c>
100bc35e4:     	ldr	x8, [x0, #0xc8]
100bc35e8:     	cmp	x8, x10
100bc35ec:     	csel	x21, x8, x10, lo
100bc35f0:     	cmp	x21, x8
100bc35f4:     	b.ne	0x100bc396c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x49c>
100bc35f8:     	lsr	w9, w1, #2
100bc35fc:     	ldr	x8, [x0, #0x58]
100bc3600:     	cmp	x8, x9
100bc3604:     	b.ls	0x100bc3bd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bc3608:     	ldr	x23, [x0, #0x50]
100bc360c:     	add	x9, x23, x9, lsl #6
100bc3610:     	ldp	x7, x5, [x9]
100bc3614:     	ldp	x16, x14, [x9, #0x10]
100bc3618:     	ldp	x13, x12, [x9, #0x20]
100bc361c:     	ldp	x11, x10, [x9, #0x30]
100bc3620:     	tbz	w1, #0x0, 0x100bc3654 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x184>
100bc3624:     	ldp	x9, x15, [x0, #0xd8]
100bc3628:     	eor	x7, x9, x7
100bc362c:     	eor	x5, x15, x5
100bc3630:     	ldp	x9, x15, [x0, #0xe8]
100bc3634:     	eor	x16, x9, x16
100bc3638:     	eor	x14, x15, x14
100bc363c:     	ldp	x9, x15, [x0, #0xf8]
100bc3640:     	eor	x13, x9, x13
100bc3644:     	eor	x12, x15, x12
100bc3648:     	ldp	x9, x15, [x0, #0x108]
100bc364c:     	eor	x11, x9, x11
100bc3650:     	eor	x10, x15, x10
100bc3654:     	lsr	w9, w2, #2
100bc3658:     	cmp	x8, x9
100bc365c:     	b.ls	0x100bc3bd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bc3660:     	add	x9, x23, x9, lsl #6
100bc3664:     	ldp	x24, x22, [x9]
100bc3668:     	ldp	x21, x20, [x9, #0x10]
100bc366c:     	ldp	x19, x6, [x9, #0x20]
100bc3670:     	ldp	x17, x15, [x9, #0x30]
100bc3674:     	tbz	w2, #0x0, 0x100bc36a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x1d8>
100bc3678:     	ldp	x9, x25, [x0, #0xd8]
100bc367c:     	eor	x24, x9, x24
100bc3680:     	eor	x22, x25, x22
100bc3684:     	ldp	x9, x25, [x0, #0xe8]
100bc3688:     	eor	x21, x9, x21
100bc368c:     	eor	x20, x25, x20
100bc3690:     	ldp	x9, x25, [x0, #0xf8]
100bc3694:     	eor	x19, x9, x19
100bc3698:     	eor	x6, x25, x6
100bc369c:     	ldp	x9, x25, [x0, #0x108]
100bc36a0:     	eor	x17, x9, x17
100bc36a4:     	eor	x15, x25, x15
100bc36a8:     	lsr	w9, w3, #2
100bc36ac:     	cmp	x8, x9
100bc36b0:     	b.ls	0x100bc3bd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x700>
100bc36b4:     	stp	x15, x10, [sp]
100bc36b8:     	add	x8, x23, x9, lsl #6
100bc36bc:     	ldp	x30, x28, [x8]
100bc36c0:     	ldp	x27, x26, [x8, #0x10]
100bc36c4:     	ldp	x25, x23, [x8, #0x20]
100bc36c8:     	ldp	x9, x8, [x8, #0x30]
100bc36cc:     	stp	x11, x12, [sp, #0x10]
100bc36d0:     	mov	x15, x13
100bc36d4:     	tbz	w3, #0x0, 0x100bc3708 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x238>
100bc36d8:     	ldp	x10, x11, [x0, #0xd8]
100bc36dc:     	eor	x30, x10, x30
100bc36e0:     	eor	x28, x11, x28
100bc36e4:     	ldp	x10, x11, [x0, #0xe8]
100bc36e8:     	eor	x27, x10, x27
100bc36ec:     	eor	x26, x11, x26
100bc36f0:     	ldp	x10, x11, [x0, #0xf8]
100bc36f4:     	eor	x25, x10, x25
100bc36f8:     	eor	x23, x11, x23
100bc36fc:     	ldp	x10, x11, [x0, #0x108]
100bc3700:     	eor	x9, x10, x9
100bc3704:     	eor	x8, x11, x8
100bc3708:     	bic	x10, x7, x24
100bc370c:     	tst	x10, x30
100bc3710:     	mov	w0, #0x2                ; =2
100bc3714:     	csel	w11, wzr, w0, eq
100bc3718:     	and	x24, x24, x7
100bc371c:     	bics	xzr, x24, x30
100bc3720:     	mov	w7, #0x4                ; =4
100bc3724:     	csel	w12, wzr, w7, eq
100bc3728:     	tst	x24, x30
100bc372c:     	mov	w24, #0x8               ; =8
100bc3730:     	csel	w13, wzr, w24, eq
100bc3734:     	bics	xzr, x10, x30
100bc3738:     	cinc	w10, w11, ne
100bc373c:     	orr	w11, w12, w13
100bc3740:     	orr	w30, w10, w11
100bc3744:     	cmp	w30, #0xf
100bc3748:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc374c:     	bic	x10, x5, x22
100bc3750:     	tst	x10, x28
100bc3754:     	csel	w11, wzr, w0, eq
100bc3758:     	and	x12, x22, x5
100bc375c:     	bics	xzr, x12, x28
100bc3760:     	csel	w13, wzr, w7, eq
100bc3764:     	tst	x12, x28
100bc3768:     	csel	w12, wzr, w24, eq
100bc376c:     	bics	xzr, x10, x28
100bc3770:     	cinc	w10, w11, ne
100bc3774:     	orr	w11, w13, w12
100bc3778:     	orr	w10, w10, w11
100bc377c:     	orr	w5, w10, w30
100bc3780:     	cmp	w5, #0xf
100bc3784:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc3788:     	bic	x10, x16, x21
100bc378c:     	tst	x10, x27
100bc3790:     	csel	w11, wzr, w0, eq
100bc3794:     	and	x12, x21, x16
100bc3798:     	bics	xzr, x12, x27
100bc379c:     	mov	w16, #0x4               ; =4
100bc37a0:     	csel	w13, wzr, w16, eq
100bc37a4:     	tst	x12, x27
100bc37a8:     	mov	w7, #0x8                ; =8
100bc37ac:     	csel	w12, wzr, w7, eq
100bc37b0:     	bics	xzr, x10, x27
100bc37b4:     	cinc	w10, w11, ne
100bc37b8:     	orr	w11, w13, w12
100bc37bc:     	orr	w10, w10, w11
100bc37c0:     	orr	w5, w10, w5
100bc37c4:     	cmp	w5, #0xf
100bc37c8:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc37cc:     	bic	x10, x14, x20
100bc37d0:     	tst	x10, x26
100bc37d4:     	csel	w11, wzr, w0, eq
100bc37d8:     	and	x12, x20, x14
100bc37dc:     	bics	xzr, x12, x26
100bc37e0:     	csel	w13, wzr, w16, eq
100bc37e4:     	tst	x12, x26
100bc37e8:     	csel	w12, wzr, w7, eq
100bc37ec:     	bics	xzr, x10, x26
100bc37f0:     	cinc	w10, w11, ne
100bc37f4:     	orr	w11, w13, w12
100bc37f8:     	orr	w10, w10, w11
100bc37fc:     	orr	w16, w10, w5
100bc3800:     	cmp	w16, #0xf
100bc3804:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc3808:     	bic	x10, x15, x19
100bc380c:     	tst	x10, x25
100bc3810:     	mov	w14, #0x2               ; =2
100bc3814:     	csel	w11, wzr, w14, eq
100bc3818:     	and	x12, x19, x15
100bc381c:     	bics	xzr, x12, x25
100bc3820:     	mov	w13, #0x4               ; =4
100bc3824:     	csel	w5, wzr, w13, eq
100bc3828:     	tst	x12, x25
100bc382c:     	mov	w0, #0x8                ; =8
100bc3830:     	csel	w12, wzr, w0, eq
100bc3834:     	bics	xzr, x10, x25
100bc3838:     	cinc	w10, w11, ne
100bc383c:     	orr	w11, w5, w12
100bc3840:     	orr	w10, w10, w11
100bc3844:     	orr	w16, w10, w16
100bc3848:     	cmp	w16, #0xf
100bc384c:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc3850:     	ldr	x12, [sp, #0x18]
100bc3854:     	bic	x10, x12, x6
100bc3858:     	tst	x10, x23
100bc385c:     	csel	w11, wzr, w14, eq
100bc3860:     	and	x12, x6, x12
100bc3864:     	bics	xzr, x12, x23
100bc3868:     	csel	w13, wzr, w13, eq
100bc386c:     	tst	x12, x23
100bc3870:     	csel	w12, wzr, w0, eq
100bc3874:     	bics	xzr, x10, x23
100bc3878:     	cinc	w10, w11, ne
100bc387c:     	orr	w11, w13, w12
100bc3880:     	orr	w10, w10, w11
100bc3884:     	orr	w13, w10, w16
100bc3888:     	cmp	w13, #0xf
100bc388c:     	b.eq	0x100bc38dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x40c>
100bc3890:     	ldr	x11, [sp, #0x10]
100bc3894:     	bic	x10, x11, x17
100bc3898:     	tst	x10, x9
100bc389c:     	mov	w12, #0x2               ; =2
100bc38a0:     	csel	w16, wzr, w12, eq
100bc38a4:     	and	x14, x17, x11
100bc38a8:     	bics	xzr, x14, x9
100bc38ac:     	mov	w11, #0x4               ; =4
100bc38b0:     	csel	w17, wzr, w11, eq
100bc38b4:     	tst	x14, x9
100bc38b8:     	mov	w14, #0x8               ; =8
100bc38bc:     	csel	w0, wzr, w14, eq
100bc38c0:     	bics	xzr, x10, x9
100bc38c4:     	cinc	w9, w16, ne
100bc38c8:     	orr	w10, w17, w0
100bc38cc:     	orr	w9, w9, w10
100bc38d0:     	orr	w9, w9, w13
100bc38d4:     	cmp	w9, #0xf
100bc38d8:     	b.ne	0x100bc3b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6b0>
100bc38dc:     	mov	w20, #0xf               ; =15
100bc38e0:     	b	0x100bc3b44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bc38e4:     	mov	w20, #0x0               ; =0
100bc38e8:     	b	0x100bc3b5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bc38ec:     	ldurb	w20, [x13, #-0x4]
100bc38f0:     	b	0x100bc3b5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x68c>
100bc38f4:     	lsr	w8, w1, #2
100bc38f8:     	ldr	x9, [x0, #0x40]
100bc38fc:     	cmp	x9, x8
100bc3900:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc3904:     	ldr	x9, [x0, #0x38]
100bc3908:     	lsl	x8, x8, #4
100bc390c:     	ldr	w10, [x9, x8]
100bc3910:     	tbz	w2, #0x1, 0x100bc35d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x104>
100bc3914:     	lsr	w8, w2, #2
100bc3918:     	ldr	x9, [x0, #0x40]
100bc391c:     	cmp	x9, x8
100bc3920:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc3924:     	ldr	x9, [x0, #0x38]
100bc3928:     	lsl	x8, x8, #4
100bc392c:     	ldr	w8, [x9, x8]
100bc3930:     	cmp	x8, x10
100bc3934:     	csel	x10, x8, x10, lo
100bc3938:     	tbz	w3, #0x1, 0x100bc35e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x114>
100bc393c:     	lsr	w8, w3, #2
100bc3940:     	ldr	x9, [x0, #0x40]
100bc3944:     	cmp	x9, x8
100bc3948:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc394c:     	ldr	x9, [x0, #0x38]
100bc3950:     	lsl	x8, x8, #4
100bc3954:     	ldr	w9, [x9, x8]
100bc3958:     	ldr	x8, [x0, #0xc8]
100bc395c:     	cmp	x9, x10
100bc3960:     	csel	x21, x9, x10, lo
100bc3964:     	cmp	x21, x8
100bc3968:     	b.eq	0x100bc35f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x128>
100bc396c:     	mov	x8, x1
100bc3970:     	tbz	w1, #0x1, 0x100bc39a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bc3974:     	lsr	w8, w1, #2
100bc3978:     	ldr	x9, [x0, #0x40]
100bc397c:     	cmp	x9, x8
100bc3980:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc3984:     	ldr	x9, [x0, #0x38]
100bc3988:     	add	x9, x9, x8, lsl #4
100bc398c:     	ldr	w10, [x9]
100bc3990:     	mov	x8, x1
100bc3994:     	cmp	x21, x10
100bc3998:     	b.ne	0x100bc39a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x4d8>
100bc399c:     	ldr	w8, [x9, #0x4]
100bc39a0:     	and	w9, w1, #0x1
100bc39a4:     	eor	w8, w8, w9
100bc39a8:     	mov	x9, x2
100bc39ac:     	tbz	w2, #0x1, 0x100bc39e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bc39b0:     	lsr	w9, w2, #2
100bc39b4:     	ldr	x10, [x0, #0x40]
100bc39b8:     	cmp	x10, x9
100bc39bc:     	b.ls	0x100bc3be4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x714>
100bc39c0:     	ldr	x10, [x0, #0x38]
100bc39c4:     	add	x10, x10, x9, lsl #4
100bc39c8:     	ldr	w11, [x10]
100bc39cc:     	mov	x9, x2
100bc39d0:     	cmp	x21, x11
100bc39d4:     	b.ne	0x100bc39e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x514>
100bc39d8:     	ldr	w9, [x10, #0x4]
100bc39dc:     	and	w10, w2, #0x1
100bc39e0:     	eor	w9, w9, w10
100bc39e4:     	mov	x22, x1
100bc39e8:     	mov	x10, x3
100bc39ec:     	tbz	w3, #0x1, 0x100bc3a24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bc39f0:     	lsr	w10, w3, #2
100bc39f4:     	ldr	x1, [x0, #0x40]
100bc39f8:     	cmp	x1, x10
100bc39fc:     	b.ls	0x100bc3bf8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x728>
100bc3a00:     	ldr	x11, [x0, #0x38]
100bc3a04:     	add	x11, x11, x10, lsl #4
100bc3a08:     	ldr	w12, [x11]
100bc3a0c:     	mov	x10, x3
100bc3a10:     	cmp	x21, x12
100bc3a14:     	b.ne	0x100bc3a24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x554>
100bc3a18:     	ldr	w10, [x11, #0x4]
100bc3a1c:     	and	w11, w3, #0x1
100bc3a20:     	eor	w10, w10, w11
100bc3a24:     	mov	x23, x2
100bc3a28:     	mov	x24, x3
100bc3a2c:     	mov	x25, x0
100bc3a30:     	mov	x1, x8
100bc3a34:     	mov	x2, x9
100bc3a38:     	mov	x3, x10
100bc3a3c:     	mov	x19, x4
100bc3a40:     	bl	0x100bc34d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bc3a44:     	and	w8, w0, #0xff
100bc3a48:     	cmp	w8, #0xf
100bc3a4c:     	b.ne	0x100bc3a60 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x590>
100bc3a50:     	mov	w20, #0xf               ; =15
100bc3a54:     	mov	x4, x19
100bc3a58:     	mov	x3, x24
100bc3a5c:     	b	0x100bc3b3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x66c>
100bc3a60:     	mov	x20, x0
100bc3a64:     	mov	x1, x22
100bc3a68:     	mov	x10, x24
100bc3a6c:     	mov	x11, x23
100bc3a70:     	mov	x0, x25
100bc3a74:     	tbz	w22, #0x1, 0x100bc3ab0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bc3a78:     	mov	x9, x22
100bc3a7c:     	lsr	w8, w22, #2
100bc3a80:     	ldr	x1, [x0, #0x40]
100bc3a84:     	cmp	x1, x8
100bc3a88:     	b.ls	0x100bc3c08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x738>
100bc3a8c:     	ldr	x12, [x0, #0x38]
100bc3a90:     	add	x8, x12, x8, lsl #4
100bc3a94:     	ldr	w12, [x8]
100bc3a98:     	mov	x1, x9
100bc3a9c:     	cmp	x21, x12
100bc3aa0:     	b.ne	0x100bc3ab0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x5e0>
100bc3aa4:     	ldr	w8, [x8, #0x8]
100bc3aa8:     	and	w9, w9, #0x1
100bc3aac:     	eor	w1, w8, w9
100bc3ab0:     	mov	x2, x11
100bc3ab4:     	tbz	w11, #0x1, 0x100bc3aec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bc3ab8:     	lsr	w8, w11, #2
100bc3abc:     	ldr	x9, [x0, #0x40]
100bc3ac0:     	cmp	x9, x8
100bc3ac4:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc3ac8:     	ldr	x9, [x0, #0x38]
100bc3acc:     	add	x8, x9, x8, lsl #4
100bc3ad0:     	ldr	w9, [x8]
100bc3ad4:     	mov	x2, x11
100bc3ad8:     	cmp	x21, x9
100bc3adc:     	b.ne	0x100bc3aec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x61c>
100bc3ae0:     	ldr	w8, [x8, #0x8]
100bc3ae4:     	and	w9, w11, #0x1
100bc3ae8:     	eor	w2, w8, w9
100bc3aec:     	mov	x3, x10
100bc3af0:     	tbz	w10, #0x1, 0x100bc3b28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bc3af4:     	lsr	w8, w10, #2
100bc3af8:     	ldr	x9, [x0, #0x40]
100bc3afc:     	cmp	x9, x8
100bc3b00:     	b.ls	0x100bc3bbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x6ec>
100bc3b04:     	ldr	x9, [x0, #0x38]
100bc3b08:     	add	x8, x9, x8, lsl #4
100bc3b0c:     	ldr	w9, [x8]
100bc3b10:     	mov	x3, x10
100bc3b14:     	cmp	x21, x9
100bc3b18:     	b.ne	0x100bc3b28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x658>
100bc3b1c:     	ldr	w8, [x8, #0x8]
100bc3b20:     	and	w9, w10, #0x1
100bc3b24:     	eor	w3, w8, w9
100bc3b28:     	mov	x4, x19
100bc3b2c:     	bl	0x100bc34d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_>
100bc3b30:     	mov	x3, x24
100bc3b34:     	mov	x4, x19
100bc3b38:     	orr	w20, w0, w20
100bc3b3c:     	mov	x2, x23
100bc3b40:     	mov	x1, x22
100bc3b44:     	stp	w1, w2, [sp, #0x24]
100bc3b48:     	str	w3, [sp, #0x2c]
100bc3b4c:     	add	x1, sp, #0x24
100bc3b50:     	mov	x0, x4
100bc3b54:     	mov	x2, x20
100bc3b58:     	bl	0x100c18e30 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmmEhNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100bc3b5c:     	mov	x0, x20
100bc3b60:     	ldp	x29, x30, [sp, #0x80]
100bc3b64:     	ldp	x20, x19, [sp, #0x70]
100bc3b68:     	ldp	x22, x21, [sp, #0x60]
100bc3b6c:     	ldp	x24, x23, [sp, #0x50]
100bc3b70:     	ldp	x26, x25, [sp, #0x40]
100bc3b74:     	ldp	x28, x27, [sp, #0x30]
100bc3b78:     	add	sp, sp, #0x90
100bc3b7c:     	ret
100bc3b80:     	ldp	x15, x13, [sp]
100bc3b84:     	bic	x10, x13, x15
100bc3b88:     	tst	x10, x8
100bc3b8c:     	csel	w12, wzr, w12, eq
100bc3b90:     	and	x13, x15, x13
100bc3b94:     	bics	xzr, x13, x8
100bc3b98:     	csel	w11, wzr, w11, eq
100bc3b9c:     	tst	x13, x8
100bc3ba0:     	csel	w13, wzr, w14, eq
100bc3ba4:     	bics	xzr, x10, x8
100bc3ba8:     	cinc	w8, w12, ne
100bc3bac:     	orr	w10, w11, w13
100bc3bb0:     	orr	w8, w8, w10
100bc3bb4:     	orr	w20, w8, w9
100bc3bb8:     	b	0x100bc3b44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E9occupancyB6_+0x674>
100bc3bbc:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bc3bc0:     	add	x2, x2, #0xe88
100bc3bc4:     	mov	x0, x8
100bc3bc8:     	mov	x1, x9
100bc3bcc:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc3bd0:     	adrp	x2, 0x1014e1000 <dyld_stub_binder+0x1014e1000>
100bc3bd4:     	add	x2, x2, #0x110
100bc3bd8:     	mov	x0, x9
100bc3bdc:     	mov	x1, x8
100bc3be0:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc3be4:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bc3be8:     	add	x2, x2, #0xe88
100bc3bec:     	mov	x0, x9
100bc3bf0:     	mov	x1, x10
100bc3bf4:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc3bf8:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bc3bfc:     	add	x2, x2, #0xe88
100bc3c00:     	mov	x0, x10
100bc3c04:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bc3c08:     	adrp	x2, 0x1014e0000 <dyld_stub_binder+0x1014e0000>
100bc3c0c:     	add	x2, x2, #0xe88
100bc3c10:     	mov	x0, x8
100bc3c14:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
