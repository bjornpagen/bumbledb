
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006de424 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense>:
1006de424:     	sub	sp, sp, #0xe0
1006de428:     	stp	d15, d14, [sp, #0x40]
1006de42c:     	stp	d13, d12, [sp, #0x50]
1006de430:     	stp	d11, d10, [sp, #0x60]
1006de434:     	stp	d9, d8, [sp, #0x70]
1006de438:     	stp	x28, x27, [sp, #0x80]
1006de43c:     	stp	x26, x25, [sp, #0x90]
1006de440:     	stp	x24, x23, [sp, #0xa0]
1006de444:     	stp	x22, x21, [sp, #0xb0]
1006de448:     	stp	x20, x19, [sp, #0xc0]
1006de44c:     	stp	x29, x30, [sp, #0xd0]
1006de450:     	add	x29, sp, #0xd0
1006de454:     	ldr	x8, [x0, #0x10]
1006de458:     	and	x9, x8, #0x3f
1006de45c:     	mov	w10, #0x1               ; =1
1006de460:     	lsl	x8, x10, x8
1006de464:     	lsr	x8, x8, #6
1006de468:     	cmp	x9, #0x6
1006de46c:     	cinc	x8, x8, lo
1006de470:     	stp	x2, x8, [sp, #0x30]
1006de474:     	cmp	x2, x8
1006de478:     	b.ne	0x1006dedc8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9a4>
1006de47c:     	ldr	x8, [x0, #0x28]
1006de480:     	cbz	x8, 0x1006ded4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
1006de484:     	mov	x19, x1
1006de488:     	ldr	x9, [x0, #0x20]
1006de48c:     	add	x10, x9, x8, lsl #3
1006de490:     	lsl	x8, x2, #3
1006de494:     	add	x11, x1, x8
1006de498:     	sub	x12, x8, #0x8
1006de49c:     	lsr	x8, x12, #3
1006de4a0:     	add	x13, x8, #0x1
1006de4a4:     	and	x14, x13, #0x3ffffffffffffff8
1006de4a8:     	mov	w16, #0x1               ; =1
1006de4ac:     	mov	x8, #0x100000000        ; =4294967296
1006de4b0:     	str	x8, [sp, #0x20]
1006de4b4:     	mov	x8, #0x2                ; =2
1006de4b8:     	movk	x8, #0x3, lsl #32
1006de4bc:     	str	x8, [sp, #0x18]
1006de4c0:     	mov	x8, #0x4                ; =4
1006de4c4:     	movk	x8, #0x5, lsl #32
1006de4c8:     	fmov	d2, x8
1006de4cc:     	mov	x8, #0x6                ; =6
1006de4d0:     	movk	x8, #0x7, lsl #32
1006de4d4:     	fmov	d3, x8
1006de4d8:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de4dc:     	ldr	q0, [x8, #0x290]
1006de4e0:     	str	q0, [sp]
1006de4e4:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de4e8:     	ldr	q5, [x8, #0x960]
1006de4ec:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de4f0:     	ldr	q6, [x8, #0x970]
1006de4f4:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de4f8:     	ldr	q7, [x8, #0x3c0]
1006de4fc:     	mov	x8, #0x8                ; =8
1006de500:     	movk	x8, #0x9, lsl #32
1006de504:     	fmov	d16, x8
1006de508:     	mov	x8, #0xa                ; =10
1006de50c:     	movk	x8, #0xb, lsl #32
1006de510:     	fmov	d17, x8
1006de514:     	mov	x8, #0xc                ; =12
1006de518:     	movk	x8, #0xd, lsl #32
1006de51c:     	fmov	d18, x8
1006de520:     	mov	x8, #0xe                ; =14
1006de524:     	movk	x8, #0xf, lsl #32
1006de528:     	fmov	d19, x8
1006de52c:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de530:     	ldr	q20, [x8, #0x3d0]
1006de534:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de538:     	ldr	q21, [x8, #0x980]
1006de53c:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de540:     	ldr	q22, [x8, #0x990]
1006de544:     	adrp	x8, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de548:     	ldr	q23, [x8, #0x9a0]
1006de54c:     	mov	x8, #0x10               ; =16
1006de550:     	movk	x8, #0x11, lsl #32
1006de554:     	fmov	d24, x8
1006de558:     	mov	x8, #0x12               ; =18
1006de55c:     	movk	x8, #0x13, lsl #32
1006de560:     	fmov	d25, x8
1006de564:     	mov	x8, #0x14               ; =20
1006de568:     	movk	x8, #0x15, lsl #32
1006de56c:     	fmov	d26, x8
1006de570:     	mov	x8, #0x16               ; =22
1006de574:     	movk	x8, #0x17, lsl #32
1006de578:     	fmov	d27, x8
1006de57c:     	add	x8, x1, x14, lsl #3
1006de580:     	str	x8, [sp, #0x28]
1006de584:     	b	0x1006de594 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x170>
1006de588:     	add	x9, x9, #0x8
1006de58c:     	cmp	x9, x10
1006de590:     	b.eq	0x1006ded4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
1006de594:     	ldp	w1, w15, [x9]
1006de598:     	cmp	w15, #0x6
1006de59c:     	b.hs	0x1006de6bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x298>
1006de5a0:     	mov	x3, #0x0                ; =0
1006de5a4:     	mov	x8, #0x0                ; =0
1006de5a8:     	and	w14, w1, #0x1f
1006de5ac:     	lsl	w0, w16, w1
1006de5b0:     	mov	w1, #-0x1               ; =-1
1006de5b4:     	lsr	w4, w1, w15
1006de5b8:     	lsl	x6, x16, x3
1006de5bc:     	orr	x6, x6, x8
1006de5c0:     	tst	w4, #0x1
1006de5c4:     	csel	x4, x8, x6, eq
1006de5c8:     	tst	w0, w3
1006de5cc:     	add	x3, x3, #0x1
1006de5d0:     	csel	x8, x8, x4, eq
1006de5d4:     	sub	w1, w1, #0x1
1006de5d8:     	cmp	x3, #0x40
1006de5dc:     	b.ne	0x1006de5b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x190>
1006de5e0:     	cbz	x2, 0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006de5e4:     	mov	w17, #-0x1              ; =-1
1006de5e8:     	lsl	w14, w17, w14
1006de5ec:     	lsl	w15, w16, w15
1006de5f0:     	add	w14, w14, w15
1006de5f4:     	and	w15, w14, #0x3f
1006de5f8:     	mov	x14, x19
1006de5fc:     	cmp	x12, #0x38
1006de600:     	b.lo	0x1006de690 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
1006de604:     	dup.2d	v28, x15
1006de608:     	dup.2d	v29, x8
1006de60c:     	neg.2d	v30, v28
1006de610:     	add	x0, x19, #0x20
1006de614:     	and	x1, x13, #0x3ffffffffffffff8
1006de618:     	ldp	q31, q8, [x0, #-0x20]
1006de61c:     	ldp	q9, q10, [x0]
1006de620:     	ushl.2d	v11, v31, v30
1006de624:     	ushl.2d	v12, v8, v30
1006de628:     	ushl.2d	v13, v9, v30
1006de62c:     	ushl.2d	v14, v10, v30
1006de630:     	eor.16b	v11, v11, v31
1006de634:     	eor.16b	v12, v12, v8
1006de638:     	eor.16b	v13, v13, v9
1006de63c:     	eor.16b	v14, v14, v10
1006de640:     	and.16b	v11, v11, v29
1006de644:     	and.16b	v12, v12, v29
1006de648:     	and.16b	v13, v13, v29
1006de64c:     	and.16b	v14, v14, v29
1006de650:     	ushl.2d	v15, v11, v28
1006de654:     	ushl.2d	v0, v12, v28
1006de658:     	ushl.2d	v4, v13, v28
1006de65c:     	ushl.2d	v1, v14, v28
1006de660:     	eor3.16b	v31, v31, v15, v11
1006de664:     	eor3.16b	v0, v8, v0, v12
1006de668:     	eor3.16b	v4, v9, v4, v13
1006de66c:     	stp	q31, q0, [x0, #-0x20]
1006de670:     	eor3.16b	v0, v10, v1, v14
1006de674:     	stp	q4, q0, [x0], #0x40
1006de678:     	subs	x1, x1, #0x8
1006de67c:     	b.ne	0x1006de618 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x1f4>
1006de680:     	ldr	x14, [sp, #0x28]
1006de684:     	and	x17, x13, #0x3ffffffffffffff8
1006de688:     	cmp	x13, x17
1006de68c:     	b.eq	0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006de690:     	ldr	x17, [x14]
1006de694:     	lsr	x0, x17, x15
1006de698:     	eor	x0, x0, x17
1006de69c:     	and	x0, x0, x8
1006de6a0:     	lsl	x1, x0, x15
1006de6a4:     	eor	x17, x17, x0
1006de6a8:     	eor	x17, x17, x1
1006de6ac:     	str	x17, [x14], #0x8
1006de6b0:     	cmp	x14, x11
1006de6b4:     	b.ne	0x1006de690 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
1006de6b8:     	b	0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006de6bc:     	cmp	w1, #0x6
1006de6c0:     	b.hs	0x1006decf0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8cc>
1006de6c4:     	add	w14, w15, #0x3a
1006de6c8:     	and	w8, w14, #0x3f
1006de6cc:     	cmp	w8, #0x3f
1006de6d0:     	b.eq	0x1006ded90 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x96c>
1006de6d4:     	mov	w15, #0x2               ; =2
1006de6d8:     	lsl	x28, x15, x14
1006de6dc:     	add	x15, x8, #0x1
1006de6e0:     	lsr	x20, x2, x15
1006de6e4:     	mov	x15, #0xfffffffffffffff ; =1152921504606846975
1006de6e8:     	add	x15, x28, x15
1006de6ec:     	tst	x15, x2
1006de6f0:     	cset	w22, ne
1006de6f4:     	cinc	x15, x20, ne
1006de6f8:     	cbz	x15, 0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006de6fc:     	lsl	x6, x16, x14
1006de700:     	mov	w14, #0x8               ; =8
1006de704:     	lsl	x14, x14, x8
1006de708:     	lsr	x14, x14, #3
1006de70c:     	subs	x0, x28, x6
1006de710:     	cmp	x0, x14
1006de714:     	csel	x3, x0, x14, lo
1006de718:     	cmp	x28, x6
1006de71c:     	b.lo	0x1006deda8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x984>
1006de720:     	mov	x0, #0x0                ; =0
1006de724:     	b.eq	0x1006debdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
1006de728:     	lsl	x4, x16, x1
1006de72c:     	dup.2s	v28, w4
1006de730:     	ldp	d1, d0, [sp, #0x18]
1006de734:     	and.8b	v0, v28, v0
1006de738:     	and.8b	v1, v28, v1
1006de73c:     	and.8b	v4, v28, v2
1006de740:     	and.8b	v29, v28, v3
1006de744:     	cmeq.2s	v0, v0, #0
1006de748:     	ushll.2d	v0, v0, #0x0
1006de74c:     	cmeq.2s	v1, v1, #0
1006de750:     	ushll.2d	v1, v1, #0x0
1006de754:     	cmeq.2s	v4, v4, #0
1006de758:     	ushll.2d	v4, v4, #0x0
1006de75c:     	cmeq.2s	v29, v29, #0
1006de760:     	ushll.2d	v29, v29, #0x0
1006de764:     	ldr	q30, [sp]
1006de768:     	and.16b	v0, v0, v30
1006de76c:     	and.16b	v1, v1, v5
1006de770:     	and.16b	v4, v4, v6
1006de774:     	and.16b	v29, v29, v7
1006de778:     	and.8b	v30, v28, v16
1006de77c:     	and.8b	v31, v28, v17
1006de780:     	and.8b	v8, v28, v18
1006de784:     	and.8b	v9, v28, v19
1006de788:     	cmeq.2s	v30, v30, #0
1006de78c:     	ushll.2d	v30, v30, #0x0
1006de790:     	cmeq.2s	v31, v31, #0
1006de794:     	ushll.2d	v31, v31, #0x0
1006de798:     	cmeq.2s	v8, v8, #0
1006de79c:     	ushll.2d	v8, v8, #0x0
1006de7a0:     	cmeq.2s	v9, v9, #0
1006de7a4:     	ushll.2d	v9, v9, #0x0
1006de7a8:     	and.16b	v30, v30, v20
1006de7ac:     	and.16b	v31, v31, v21
1006de7b0:     	and.16b	v8, v8, v22
1006de7b4:     	and.16b	v9, v9, v23
1006de7b8:     	orr.16b	v0, v30, v0
1006de7bc:     	orr.16b	v1, v31, v1
1006de7c0:     	orr.16b	v4, v8, v4
1006de7c4:     	orr.16b	v29, v9, v29
1006de7c8:     	and.8b	v30, v28, v24
1006de7cc:     	and.8b	v31, v28, v25
1006de7d0:     	and.8b	v8, v28, v26
1006de7d4:     	and.8b	v9, v28, v27
1006de7d8:     	cmeq.2s	v30, v30, #0
1006de7dc:     	ushll.2d	v30, v30, #0x0
1006de7e0:     	cmeq.2s	v31, v31, #0
1006de7e4:     	ushll.2d	v31, v31, #0x0
1006de7e8:     	cmeq.2s	v8, v8, #0
1006de7ec:     	ushll.2d	v8, v8, #0x0
1006de7f0:     	cmeq.2s	v9, v9, #0
1006de7f4:     	ushll.2d	v9, v9, #0x0
1006de7f8:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de7fc:     	ldr	q10, [x14, #0x9b0]
1006de800:     	and.16b	v30, v30, v10
1006de804:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de808:     	ldr	q10, [x14, #0x9c0]
1006de80c:     	and.16b	v10, v31, v10
1006de810:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de814:     	ldr	q31, [x14, #0x9d0]
1006de818:     	and.16b	v11, v8, v31
1006de81c:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de820:     	ldr	q31, [x14, #0x9e0]
1006de824:     	and.16b	v9, v9, v31
1006de828:     	mov	x14, #0x18              ; =24
1006de82c:     	movk	x14, #0x19, lsl #32
1006de830:     	fmov	d31, x14
1006de834:     	and.8b	v31, v28, v31
1006de838:     	mov	x14, #0x1a              ; =26
1006de83c:     	movk	x14, #0x1b, lsl #32
1006de840:     	fmov	d8, x14
1006de844:     	and.8b	v8, v28, v8
1006de848:     	mov	x14, #0x1c              ; =28
1006de84c:     	movk	x14, #0x1d, lsl #32
1006de850:     	fmov	d12, x14
1006de854:     	and.8b	v12, v28, v12
1006de858:     	mov	x14, #0x1e              ; =30
1006de85c:     	movk	x14, #0x1f, lsl #32
1006de860:     	fmov	d13, x14
1006de864:     	and.8b	v13, v28, v13
1006de868:     	cmeq.2s	v31, v31, #0
1006de86c:     	ushll.2d	v31, v31, #0x0
1006de870:     	cmeq.2s	v8, v8, #0
1006de874:     	ushll.2d	v8, v8, #0x0
1006de878:     	cmeq.2s	v12, v12, #0
1006de87c:     	ushll.2d	v12, v12, #0x0
1006de880:     	cmeq.2s	v13, v13, #0
1006de884:     	ushll.2d	v13, v13, #0x0
1006de888:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de88c:     	ldr	q14, [x14, #0x9f0]
1006de890:     	and.16b	v31, v31, v14
1006de894:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de898:     	ldr	q14, [x14, #0xa00]
1006de89c:     	and.16b	v8, v8, v14
1006de8a0:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de8a4:     	ldr	q14, [x14, #0xa10]
1006de8a8:     	and.16b	v12, v12, v14
1006de8ac:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de8b0:     	ldr	q14, [x14, #0xa20]
1006de8b4:     	and.16b	v13, v13, v14
1006de8b8:     	orr.16b	v30, v31, v30
1006de8bc:     	orr.16b	v31, v30, v0
1006de8c0:     	orr.16b	v0, v8, v10
1006de8c4:     	orr.16b	v8, v0, v1
1006de8c8:     	orr.16b	v0, v12, v11
1006de8cc:     	orr.16b	v30, v0, v4
1006de8d0:     	orr.16b	v0, v13, v9
1006de8d4:     	orr.16b	v29, v0, v29
1006de8d8:     	mov	x14, #0x20              ; =32
1006de8dc:     	movk	x14, #0x21, lsl #32
1006de8e0:     	fmov	d0, x14
1006de8e4:     	and.8b	v0, v28, v0
1006de8e8:     	mov	x14, #0x22              ; =34
1006de8ec:     	movk	x14, #0x23, lsl #32
1006de8f0:     	fmov	d1, x14
1006de8f4:     	and.8b	v1, v28, v1
1006de8f8:     	mov	x14, #0x24              ; =36
1006de8fc:     	movk	x14, #0x25, lsl #32
1006de900:     	fmov	d4, x14
1006de904:     	and.8b	v4, v28, v4
1006de908:     	mov	x14, #0x26              ; =38
1006de90c:     	movk	x14, #0x27, lsl #32
1006de910:     	fmov	d9, x14
1006de914:     	and.8b	v9, v28, v9
1006de918:     	cmeq.2s	v0, v0, #0
1006de91c:     	sshll.2d	v0, v0, #0x0
1006de920:     	cmeq.2s	v1, v1, #0
1006de924:     	sshll.2d	v1, v1, #0x0
1006de928:     	cmeq.2s	v4, v4, #0
1006de92c:     	sshll.2d	v4, v4, #0x0
1006de930:     	cmeq.2s	v9, v9, #0
1006de934:     	sshll.2d	v9, v9, #0x0
1006de938:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de93c:     	ldr	q10, [x14, #0xa30]
1006de940:     	and.16b	v0, v0, v10
1006de944:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de948:     	ldr	q10, [x14, #0xa40]
1006de94c:     	and.16b	v1, v1, v10
1006de950:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de954:     	ldr	q10, [x14, #0xa50]
1006de958:     	and.16b	v4, v4, v10
1006de95c:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de960:     	ldr	q10, [x14, #0xa60]
1006de964:     	and.16b	v9, v9, v10
1006de968:     	mov	x14, #0x28              ; =40
1006de96c:     	movk	x14, #0x29, lsl #32
1006de970:     	fmov	d10, x14
1006de974:     	and.8b	v10, v28, v10
1006de978:     	mov	x14, #0x2a              ; =42
1006de97c:     	movk	x14, #0x2b, lsl #32
1006de980:     	fmov	d11, x14
1006de984:     	and.8b	v11, v28, v11
1006de988:     	mov	x14, #0x2c              ; =44
1006de98c:     	movk	x14, #0x2d, lsl #32
1006de990:     	fmov	d12, x14
1006de994:     	and.8b	v12, v28, v12
1006de998:     	mov	x14, #0x2e              ; =46
1006de99c:     	movk	x14, #0x2f, lsl #32
1006de9a0:     	fmov	d13, x14
1006de9a4:     	and.8b	v13, v28, v13
1006de9a8:     	cmeq.2s	v10, v10, #0
1006de9ac:     	sshll.2d	v10, v10, #0x0
1006de9b0:     	cmeq.2s	v11, v11, #0
1006de9b4:     	sshll.2d	v11, v11, #0x0
1006de9b8:     	cmeq.2s	v12, v12, #0
1006de9bc:     	sshll.2d	v12, v12, #0x0
1006de9c0:     	cmeq.2s	v13, v13, #0
1006de9c4:     	sshll.2d	v13, v13, #0x0
1006de9c8:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de9cc:     	ldr	q14, [x14, #0xa70]
1006de9d0:     	and.16b	v10, v10, v14
1006de9d4:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de9d8:     	ldr	q14, [x14, #0xa80]
1006de9dc:     	and.16b	v11, v11, v14
1006de9e0:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de9e4:     	ldr	q14, [x14, #0xa90]
1006de9e8:     	and.16b	v12, v12, v14
1006de9ec:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006de9f0:     	ldr	q14, [x14, #0xaa0]
1006de9f4:     	and.16b	v13, v13, v14
1006de9f8:     	orr.16b	v0, v10, v0
1006de9fc:     	orr.16b	v1, v11, v1
1006dea00:     	orr.16b	v4, v12, v4
1006dea04:     	orr.16b	v9, v13, v9
1006dea08:     	mov	x14, #0x30              ; =48
1006dea0c:     	movk	x14, #0x31, lsl #32
1006dea10:     	fmov	d10, x14
1006dea14:     	and.8b	v10, v28, v10
1006dea18:     	mov	x14, #0x32              ; =50
1006dea1c:     	movk	x14, #0x33, lsl #32
1006dea20:     	fmov	d11, x14
1006dea24:     	and.8b	v11, v28, v11
1006dea28:     	mov	x14, #0x34              ; =52
1006dea2c:     	movk	x14, #0x35, lsl #32
1006dea30:     	fmov	d12, x14
1006dea34:     	and.8b	v12, v28, v12
1006dea38:     	mov	x14, #0x36              ; =54
1006dea3c:     	movk	x14, #0x37, lsl #32
1006dea40:     	fmov	d13, x14
1006dea44:     	and.8b	v13, v28, v13
1006dea48:     	cmeq.2s	v10, v10, #0
1006dea4c:     	sshll.2d	v10, v10, #0x0
1006dea50:     	cmeq.2s	v11, v11, #0
1006dea54:     	sshll.2d	v11, v11, #0x0
1006dea58:     	cmeq.2s	v12, v12, #0
1006dea5c:     	sshll.2d	v12, v12, #0x0
1006dea60:     	cmeq.2s	v13, v13, #0
1006dea64:     	sshll.2d	v13, v13, #0x0
1006dea68:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006dea6c:     	ldr	q14, [x14, #0xab0]
1006dea70:     	and.16b	v10, v10, v14
1006dea74:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006dea78:     	ldr	q14, [x14, #0xac0]
1006dea7c:     	and.16b	v11, v11, v14
1006dea80:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006dea84:     	ldr	q14, [x14, #0xad0]
1006dea88:     	and.16b	v12, v12, v14
1006dea8c:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006dea90:     	ldr	q14, [x14, #0xae0]
1006dea94:     	and.16b	v13, v13, v14
1006dea98:     	orr.16b	v0, v10, v0
1006dea9c:     	orr.16b	v0, v0, v31
1006deaa0:     	orr.16b	v1, v11, v1
1006deaa4:     	orr.16b	v1, v1, v8
1006deaa8:     	orr.16b	v4, v12, v4
1006deaac:     	mov	x14, #0x38              ; =56
1006deab0:     	movk	x14, #0x39, lsl #32
1006deab4:     	fmov	d31, x14
1006deab8:     	orr.16b	v4, v4, v30
1006deabc:     	mov	x14, #0x3a              ; =58
1006deac0:     	movk	x14, #0x3b, lsl #32
1006deac4:     	fmov	d30, x14
1006deac8:     	orr.16b	v8, v13, v9
1006deacc:     	mov	x14, #0x3c              ; =60
1006dead0:     	movk	x14, #0x3d, lsl #32
1006dead4:     	fmov	d9, x14
1006dead8:     	orr.16b	v29, v8, v29
1006deadc:     	mov	x14, #0x3e              ; =62
1006deae0:     	movk	x14, #0x3f, lsl #32
1006deae4:     	fmov	d8, x14
1006deae8:     	and.8b	v31, v28, v31
1006deaec:     	and.8b	v30, v28, v30
1006deaf0:     	and.8b	v9, v28, v9
1006deaf4:     	and.8b	v28, v28, v8
1006deaf8:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006deafc:     	ldr	q8, [x14, #0xaf0]
1006deb00:     	cmeq.2s	v31, v31, #0
1006deb04:     	sshll.2d	v31, v31, #0x0
1006deb08:     	and.16b	v31, v31, v8
1006deb0c:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006deb10:     	ldr	q8, [x14, #0xb00]
1006deb14:     	cmeq.2s	v30, v30, #0
1006deb18:     	sshll.2d	v30, v30, #0x0
1006deb1c:     	and.16b	v30, v30, v8
1006deb20:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006deb24:     	ldr	q8, [x14, #0xb10]
1006deb28:     	cmeq.2s	v9, v9, #0
1006deb2c:     	sshll.2d	v9, v9, #0x0
1006deb30:     	and.16b	v8, v9, v8
1006deb34:     	adrp	x14, 0x100d0d000 <GCC_except_table7806+0xa4>
1006deb38:     	ldr	q9, [x14, #0xb20]
1006deb3c:     	cmeq.2s	v28, v28, #0
1006deb40:     	sshll.2d	v28, v28, #0x0
1006deb44:     	and.16b	v28, v28, v9
1006deb48:     	orr.16b	v0, v31, v0
1006deb4c:     	orr.16b	v1, v30, v1
1006deb50:     	orr.16b	v4, v8, v4
1006deb54:     	orr.16b	v28, v28, v29
1006deb58:     	orr.16b	v0, v1, v0
1006deb5c:     	orr.16b	v0, v4, v0
1006deb60:     	orr.16b	v0, v28, v0
1006deb64:     	mov	d1, v0[1]
1006deb68:     	orr.8b	v0, v0, v1
1006deb6c:     	fmov	x30, d0
1006deb70:     	cmp	x3, #0x1
1006deb74:     	csinc	x23, x3, xzr, hi
1006deb78:     	mov	w14, #0x10              ; =16
1006deb7c:     	lsl	x14, x14, x8
1006deb80:     	add	x1, x20, x22
1006deb84:     	sub	x1, x1, #0x1
1006deb88:     	madd	x1, x14, x1, x19
1006deb8c:     	add	x1, x1, x23, lsl #3
1006deb90:     	mov	w17, #0x8               ; =8
1006deb94:     	lsl	x8, x17, x8
1006deb98:     	add	x7, x19, x8
1006deb9c:     	add	x8, x1, x8
1006deba0:     	cmp	x19, x8
1006deba4:     	ccmp	x7, x1, #0x2, lo
1006deba8:     	ccmp	x14, #0x0, #0x8, hs
1006debac:     	cset	w20, mi
1006debb0:     	and	x22, x23, #0x1ffffffffffffffc
1006debb4:     	lsl	x8, x6, #3
1006debb8:     	add	x14, x19, #0x10
1006debbc:     	add	x21, x14, x8
1006debc0:     	lsl	x25, x28, #3
1006debc4:     	add	x7, x19, x8
1006debc8:     	mov	x1, x19
1006debcc:     	add	x27, x19, #0x10
1006debd0:     	dup.2d	v28, x4
1006debd4:     	dup.2d	v29, x30
1006debd8:     	b	0x1006dec18 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7f4>
1006debdc:     	adds	x8, x28, x0
1006debe0:     	b.hs	0x1006ded7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
1006debe4:     	cmp	x8, x2
1006debe8:     	b.hi	0x1006ded7c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
1006debec:     	mov	x0, x8
1006debf0:     	subs	x15, x15, #0x1
1006debf4:     	b.ne	0x1006debdc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
1006debf8:     	b	0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006debfc:     	add	x21, x21, x25
1006dec00:     	add	x27, x27, x25
1006dec04:     	add	x7, x7, x25
1006dec08:     	add	x1, x1, x25
1006dec0c:     	mov	x0, x8
1006dec10:     	sub	x15, x15, #0x1
1006dec14:     	cbz	x15, 0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006dec18:     	adds	x8, x0, x28
1006dec1c:     	b.hs	0x1006ded80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
1006dec20:     	cmp	x8, x2
1006dec24:     	b.hi	0x1006ded80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
1006dec28:     	cmp	x3, #0x4
1006dec2c:     	cset	w14, lo
1006dec30:     	orr	w14, w14, w20
1006dec34:     	tbz	w14, #0x0, 0x1006dec40 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x81c>
1006dec38:     	mov	x6, #0x0                ; =0
1006dec3c:     	b	0x1006decac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x888>
1006dec40:     	mov	x0, x27
1006dec44:     	mov	x6, x21
1006dec48:     	and	x14, x23, #0x1ffffffffffffffc
1006dec4c:     	ldp	q0, q1, [x0, #-0x10]
1006dec50:     	neg.2d	v4, v28
1006dec54:     	ushl.2d	v30, v0, v4
1006dec58:     	ushl.2d	v4, v1, v4
1006dec5c:     	ldp	q31, q8, [x6, #-0x10]
1006dec60:     	eor.16b	v30, v30, v31
1006dec64:     	eor.16b	v4, v4, v8
1006dec68:     	and.16b	v30, v30, v29
1006dec6c:     	and.16b	v4, v4, v29
1006dec70:     	ushl.2d	v9, v30, v28
1006dec74:     	ushl.2d	v10, v4, v28
1006dec78:     	eor.16b	v0, v9, v0
1006dec7c:     	eor.16b	v1, v10, v1
1006dec80:     	stp	q0, q1, [x0, #-0x10]
1006dec84:     	eor.16b	v0, v30, v31
1006dec88:     	eor.16b	v1, v4, v8
1006dec8c:     	stp	q0, q1, [x6, #-0x10]
1006dec90:     	add	x6, x6, #0x20
1006dec94:     	add	x0, x0, #0x20
1006dec98:     	subs	x14, x14, #0x4
1006dec9c:     	b.ne	0x1006dec4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x828>
1006deca0:     	and	x6, x23, #0x1ffffffffffffffc
1006deca4:     	cmp	x3, x22
1006deca8:     	b.eq	0x1006debfc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
1006decac:     	lsl	x0, x6, #3
1006decb0:     	add	x14, x7, x0
1006decb4:     	add	x0, x1, x0
1006decb8:     	sub	x6, x23, x6
1006decbc:     	ldr	x5, [x0]
1006decc0:     	lsr	x24, x5, x4
1006decc4:     	ldr	x26, [x14]
1006decc8:     	eor	x24, x24, x26
1006deccc:     	and	x24, x24, x30
1006decd0:     	lsl	x17, x24, x4
1006decd4:     	eor	x17, x17, x5
1006decd8:     	str	x17, [x0], #0x8
1006decdc:     	eor	x17, x24, x26
1006dece0:     	str	x17, [x14], #0x8
1006dece4:     	subs	x6, x6, #0x1
1006dece8:     	b.ne	0x1006decbc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x898>
1006decec:     	b	0x1006debfc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
1006decf0:     	cbz	x2, 0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006decf4:     	mov	x8, #0x0                ; =0
1006decf8:     	add	w14, w1, #0x3a
1006decfc:     	lsl	x14, x16, x14
1006ded00:     	add	w15, w15, #0x3a
1006ded04:     	lsl	x15, x16, x15
1006ded08:     	eor	x1, x15, x14
1006ded0c:     	b	0x1006ded1c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8f8>
1006ded10:     	add	x8, x8, #0x1
1006ded14:     	cmp	x2, x8
1006ded18:     	b.eq	0x1006de588 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1006ded1c:     	tst	x8, x14
1006ded20:     	b.eq	0x1006ded10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1006ded24:     	and	x17, x8, x15
1006ded28:     	cbnz	x17, 0x1006ded10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1006ded2c:     	eor	x0, x1, x8
1006ded30:     	cmp	x0, x2
1006ded34:     	b.hs	0x1006dede4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9c0>
1006ded38:     	ldr	x17, [x19, x8, lsl #3]
1006ded3c:     	ldr	x3, [x19, x0, lsl #3]
1006ded40:     	str	x3, [x19, x8, lsl #3]
1006ded44:     	str	x17, [x19, x0, lsl #3]
1006ded48:     	b	0x1006ded10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1006ded4c:     	ldp	x29, x30, [sp, #0xd0]
1006ded50:     	ldp	x20, x19, [sp, #0xc0]
1006ded54:     	ldp	x22, x21, [sp, #0xb0]
1006ded58:     	ldp	x24, x23, [sp, #0xa0]
1006ded5c:     	ldp	x26, x25, [sp, #0x90]
1006ded60:     	ldp	x28, x27, [sp, #0x80]
1006ded64:     	ldp	d9, d8, [sp, #0x70]
1006ded68:     	ldp	d11, d10, [sp, #0x60]
1006ded6c:     	ldp	d13, d12, [sp, #0x50]
1006ded70:     	ldp	d15, d14, [sp, #0x40]
1006ded74:     	add	sp, sp, #0xe0
1006ded78:     	ret
1006ded7c:     	add	x8, x28, x0
1006ded80:     	adrp	x3, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ded84:     	add	x3, x3, #0xa18
1006ded88:     	mov	x1, x8
1006ded8c:     	bl	0x100c9afd4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
1006ded90:     	adrp	x0, 0x100d34000 <dyld_stub_binder+0x100d34000>
1006ded94:     	add	x0, x0, #0xeec
1006ded98:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006ded9c:     	add	x2, x2, #0xc28
1006deda0:     	mov	w1, #0x1b               ; =27
1006deda4:     	bl	0x100c9b088 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1006deda8:     	cmp	x28, x2
1006dedac:     	b.hi	0x1006dedf8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9d4>
1006dedb0:     	adrp	x0, 0x100dd1000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xf98>
1006dedb4:     	add	x0, x0, #0x85b
1006dedb8:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006dedbc:     	add	x2, x2, #0xa00
1006dedc0:     	mov	w1, #0x13               ; =19
1006dedc4:     	bl	0x100c9af34 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1006dedc8:     	adrp	x5, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006dedcc:     	add	x5, x5, #0x9d0
1006dedd0:     	add	x1, sp, #0x30
1006dedd4:     	add	x2, sp, #0x38
1006dedd8:     	mov	w0, #0x0                ; =0
1006deddc:     	mov	x3, #0x0                ; =0
1006dede0:     	bl	0x100c9af70 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1006dede4:     	adrp	x8, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006dede8:     	add	x8, x8, #0x9e8
1006dedec:     	mov	x1, x2
1006dedf0:     	mov	x2, x8
1006dedf4:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006dedf8:     	mov	x0, #0x0                ; =0
1006dedfc:     	mov	x8, x28
1006dee00:     	adrp	x3, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006dee04:     	add	x3, x3, #0xa18
1006dee08:     	mov	x1, x8
1006dee0c:     	bl	0x100c9afd4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
