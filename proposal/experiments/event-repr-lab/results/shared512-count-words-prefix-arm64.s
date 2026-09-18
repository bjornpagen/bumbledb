
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010127b420 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>:
10127b420:     	sub	sp, sp, #0x1e0
10127b424:     	stp	x28, x27, [sp, #0x180]
10127b428:     	stp	x26, x25, [sp, #0x190]
10127b42c:     	stp	x24, x23, [sp, #0x1a0]
10127b430:     	stp	x22, x21, [sp, #0x1b0]
10127b434:     	stp	x20, x19, [sp, #0x1c0]
10127b438:     	stp	x29, x30, [sp, #0x1d0]
10127b43c:     	add	x29, sp, #0x1d0
10127b440:     	ldr	w8, [x3, #0x10]
10127b444:     	cbz	w8, 0x10127b578 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
10127b448:     	mov	x19, x3
10127b44c:     	ldr	w9, [x3, #0x28]
10127b450:     	cbz	w9, 0x10127b578 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x158>
10127b454:     	mov	x20, x4
10127b458:     	ldr	x10, [x4, #0x18]
10127b45c:     	cbz	x10, 0x10127b580 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
10127b460:     	mov	x10, #0x0               ; =0
10127b464:     	mov	x15, #0xa9c5            ; =43461
10127b468:     	movk	x15, #0x2e62, lsl #16
10127b46c:     	movk	x15, #0x7aea, lsl #32
10127b470:     	movk	x15, #0xf135, lsl #48
10127b474:     	ldp	x11, x12, [x19]
10127b478:     	madd	x13, x8, x15, x11
10127b47c:     	mov	x14, #0x6332            ; =25394
10127b480:     	movk	x14, #0x6ed3, lsl #16
10127b484:     	movk	x14, #0x765a, lsl #32
10127b488:     	movk	x14, #0x284f, lsl #48
10127b48c:     	mul	x14, x14, x15
10127b490:     	madd	x13, x13, x15, x14
10127b494:     	add	x13, x13, x12
10127b498:     	madd	x16, x13, x15, x9
10127b49c:     	ldp	x13, x14, [x19, #0x18]
10127b4a0:     	madd	x16, x16, x15, x13
10127b4a4:     	madd	x16, x16, x15, x14
10127b4a8:     	mul	x15, x16, x15
10127b4ac:     	ror	x3, x15, #0x2c
10127b4b0:     	lsr	x17, x3, #57
10127b4b4:     	ldp	x16, x15, [x20]
10127b4b8:     	dup.8b	v0, w17
10127b4bc:     	movi.2d	v1, #0xffffffffffffffff
10127b4c0:     	mov	w17, #0x38              ; =56
10127b4c4:     	and	x3, x3, x15
10127b4c8:     	ldr	d2, [x16, x3]
10127b4cc:     	cmeq.8b	v3, v2, v0
10127b4d0:     	fmov	x4, d3
10127b4d4:     	ands	x4, x4, #0x8080808080808080
10127b4d8:     	b.eq	0x10127b548 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
10127b4dc:     	rbit	x5, x4
10127b4e0:     	clz	x5, x5
10127b4e4:     	add	x5, x3, x5, lsr #3
10127b4e8:     	and	x5, x5, x15
10127b4ec:     	mneg	x5, x5, x17
10127b4f0:     	add	x5, x16, x5
10127b4f4:     	ldur	x6, [x5, #-0x38]
10127b4f8:     	cmp	x11, x6
10127b4fc:     	b.ne	0x10127b53c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
10127b500:     	ldur	x6, [x5, #-0x30]
10127b504:     	cmp	x12, x6
10127b508:     	b.ne	0x10127b53c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
10127b50c:     	ldur	w6, [x5, #-0x28]
10127b510:     	cmp	w8, w6
10127b514:     	b.ne	0x10127b53c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
10127b518:     	ldur	x6, [x5, #-0x20]
10127b51c:     	cmp	x13, x6
10127b520:     	b.ne	0x10127b53c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
10127b524:     	ldur	x6, [x5, #-0x18]
10127b528:     	cmp	x14, x6
10127b52c:     	b.ne	0x10127b53c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x11c>
10127b530:     	ldur	w6, [x5, #-0x10]
10127b534:     	cmp	w9, w6
10127b538:     	b.eq	0x10127b690 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x270>
10127b53c:     	sub	x5, x4, #0x2
10127b540:     	ands	x4, x5, x4
10127b544:     	b.ne	0x10127b4dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
10127b548:     	cmeq.8b	v2, v2, v1
10127b54c:     	fmov	x4, d2
10127b550:     	cbnz	x4, 0x10127b580 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x160>
10127b554:     	add	x10, x10, #0x8
10127b558:     	add	x3, x3, x10
10127b55c:     	and	x3, x3, x15
10127b560:     	ldr	d2, [x16, x3]
10127b564:     	cmeq.8b	v3, v2, v0
10127b568:     	fmov	x4, d3
10127b56c:     	ands	x4, x4, #0x8080808080808080
10127b570:     	b.ne	0x10127b4dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0xbc>
10127b574:     	b	0x10127b548 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x128>
10127b578:     	mov	x23, #0x0               ; =0
10127b57c:     	b	0x10127bc0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
10127b580:     	mov	x22, x1
10127b584:     	mov	x24, x2
10127b588:     	mov	x23, x0
10127b58c:     	mov	x1, x19
10127b590:     	bl	0x100c53b2c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
10127b594:     	mov	x21, x0
10127b598:     	fmov	d0, x21
10127b59c:     	cnt.8b	v0, v0
10127b5a0:     	addv.8b	b0, v0
10127b5a4:     	fmov	x26, d0
10127b5a8:     	cmp	x26, #0xa
10127b5ac:     	b.hs	0x10127b698 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x278>
10127b5b0:     	strb	wzr, [sp, #0x120]
10127b5b4:     	movi.2d	v0, #0000000000000000
10127b5b8:     	stp	q0, q0, [sp, #0x100]
10127b5bc:     	stp	q0, q0, [sp, #0xe0]
10127b5c0:     	str	q0, [sp, #0xd0]
10127b5c4:     	sub	x0, x29, #0xa0
10127b5c8:     	add	x4, sp, #0xd0
10127b5cc:     	mov	x1, x23
10127b5d0:     	mov	x2, x19
10127b5d4:     	mov	x3, x21
10127b5d8:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
10127b5dc:     	ldp	x25, x22, [x29, #-0xa0]
10127b5e0:     	ldur	q0, [x29, #-0x90]
10127b5e4:     	str	q0, [sp, #0xb0]
10127b5e8:     	str	q0, [sp, #0x90]
10127b5ec:     	str	q0, [sp]
10127b5f0:     	str	q0, [sp, #0x70]
10127b5f4:     	sub	x0, x29, #0xa0
10127b5f8:     	add	x2, x19, #0x18
10127b5fc:     	add	x4, sp, #0xd0
10127b600:     	mov	x1, x23
10127b604:     	mov	x3, x21
10127b608:     	bl	0x100005d78 <__RINvMNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5wordsNtB3_5Plane3newKm9_EB9_>
10127b60c:     	ldp	x24, x21, [x29, #-0xa0]
10127b610:     	ldur	q0, [x29, #-0x90]
10127b614:     	stp	x25, x22, [x29, #-0xa0]
10127b618:     	ldr	q1, [sp, #0x70]
10127b61c:     	stur	q1, [x29, #-0x90]
10127b620:     	stp	x24, x21, [x29, #-0x80]
10127b624:     	stur	q0, [x29, #-0x70]
10127b628:     	mov	w8, #0x1                ; =1
10127b62c:     	lsl	x10, x8, x26
10127b630:     	cmp	x26, #0x6
10127b634:     	cset	w8, lo
10127b638:     	mov	x9, #-0x1               ; =-1
10127b63c:     	lsl	x11, x9, x10
10127b640:     	csinv	x9, x9, x11, hs
10127b644:     	lsr	x10, x10, #6
10127b648:     	cinc	x13, x10, lo
10127b64c:     	cbz	x13, 0x10127b958 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x538>
10127b650:     	mov	x12, #0x0               ; =0
10127b654:     	ldp	x0, x11, [x29, #-0x90]
10127b658:     	sub	x14, x12, w22, uxtb
10127b65c:     	cmn	x24, #0x2
10127b660:     	b.ne	0x10127b8cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4ac>
10127b664:     	cmn	x25, #0x2
10127b668:     	b.ne	0x10127b900 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x4e0>
10127b66c:     	tst	w21, #0x1
10127b670:     	csel	x8, x14, xzr, ne
10127b674:     	and	x8, x8, x9
10127b678:     	fmov	d0, x8
10127b67c:     	cnt.8b	v0, v0
10127b680:     	addv.8b	b0, v0
10127b684:     	fmov	x8, d0
10127b688:     	mul	x23, x8, x13
10127b68c:     	b	0x10127bbfc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
10127b690:     	ldur	x23, [x5, #-0x8]
10127b694:     	b	0x10127bc0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7ec>
10127b698:     	mov	x10, #0x0               ; =0
10127b69c:     	add	x27, sp, #0xd0
10127b6a0:     	lsl	x11, x24, #2
10127b6a4:     	mov	x8, x22
10127b6a8:     	cmp	x11, x10
10127b6ac:     	b.eq	0x10127bc30 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x810>
10127b6b0:     	ldr	w9, [x8, x10]
10127b6b4:     	lsr	x12, x21, x9
10127b6b8:     	add	x10, x10, #0x4
10127b6bc:     	tbz	w12, #0x0, 0x10127b6a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x288>
10127b6c0:     	ldr	q0, [x19]
10127b6c4:     	str	q0, [sp, #0xb0]
10127b6c8:     	ldr	x10, [x19, #0x10]
10127b6cc:     	str	x10, [sp, #0xc0]
10127b6d0:     	add	x0, sp, #0x70
10127b6d4:     	mov	x21, x8
10127b6d8:     	add	x1, sp, #0xb0
10127b6dc:     	mov	x22, x23
10127b6e0:     	mov	x2, x23
10127b6e4:     	mov	x23, x9
10127b6e8:     	mov	x3, x23
10127b6ec:     	mov	w4, #0x0                ; =0
10127b6f0:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
10127b6f4:     	ldr	q0, [sp, #0x70]
10127b6f8:     	str	q0, [sp, #0x50]
10127b6fc:     	ldr	x8, [sp, #0x80]
10127b700:     	str	q0, [sp, #0x30]
10127b704:     	str	q0, [sp, #0x90]
10127b708:     	str	x8, [sp, #0xa0]
10127b70c:     	ldr	q0, [sp, #0x90]
10127b710:     	str	x8, [sp, #0xe0]
10127b714:     	str	q0, [sp, #0xd0]
10127b718:     	ldur	q0, [x19, #0x18]
10127b71c:     	str	q0, [sp, #0xb0]
10127b720:     	ldur	x8, [x19, #0x28]
10127b724:     	str	x8, [sp, #0xc0]
10127b728:     	add	x0, sp, #0x70
10127b72c:     	add	x1, sp, #0xb0
10127b730:     	mov	x2, x22
10127b734:     	mov	x3, x23
10127b738:     	mov	w4, #0x0                ; =0
10127b73c:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
10127b740:     	ldr	q0, [sp, #0x70]
10127b744:     	str	q0, [sp, #0x50]
10127b748:     	ldr	x8, [sp, #0x80]
10127b74c:     	str	q0, [sp, #0x30]
10127b750:     	str	q0, [sp, #0x90]
10127b754:     	str	x8, [sp, #0xa0]
10127b758:     	ldr	q0, [sp, #0x90]
10127b75c:     	str	x8, [sp, #0xf8]
10127b760:     	stur	q0, [x27, #0x18]
10127b764:     	ldp	q0, q1, [sp, #0xd0]
10127b768:     	ldr	q2, [sp, #0xf0]
10127b76c:     	stp	q1, q2, [x29, #-0x90]
10127b770:     	stur	q0, [x29, #-0xa0]
10127b774:     	ldp	q0, q1, [x29, #-0xa0]
10127b778:     	ldur	q2, [x29, #-0x80]
10127b77c:     	stp	q1, q2, [sp, #0x10]
10127b780:     	str	q0, [sp]
10127b784:     	mov	x1, sp
10127b788:     	mov	x0, x22
10127b78c:     	bl	0x100c53b2c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
10127b790:     	mov	x25, x0
10127b794:     	mov	x3, sp
10127b798:     	mov	x0, x22
10127b79c:     	mov	x1, x21
10127b7a0:     	mov	x2, x24
10127b7a4:     	mov	x4, x20
10127b7a8:     	bl	0x10127b420 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
10127b7ac:     	fmov	d0, x25
10127b7b0:     	cnt.8b	v0, v0
10127b7b4:     	addv.8b	b0, v0
10127b7b8:     	fmov	w8, s0
10127b7bc:     	sub	w8, w8, w26
10127b7c0:     	mvn	w8, w8
10127b7c4:     	lsl	x25, x0, x8
10127b7c8:     	ldr	q0, [x19]
10127b7cc:     	str	q0, [sp, #0xb0]
10127b7d0:     	ldr	x8, [x19, #0x10]
10127b7d4:     	str	x8, [sp, #0xc0]
10127b7d8:     	add	x0, sp, #0x70
10127b7dc:     	add	x1, sp, #0xb0
10127b7e0:     	mov	x2, x22
10127b7e4:     	mov	x3, x23
10127b7e8:     	mov	w4, #0x1                ; =1
10127b7ec:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
10127b7f0:     	ldr	q0, [sp, #0x70]
10127b7f4:     	str	q0, [sp, #0x50]
10127b7f8:     	ldr	x8, [sp, #0x80]
10127b7fc:     	str	q0, [sp, #0x30]
10127b800:     	str	q0, [sp, #0x90]
10127b804:     	str	x8, [sp, #0xa0]
10127b808:     	ldr	q0, [sp, #0x90]
10127b80c:     	str	x8, [sp, #0xe0]
10127b810:     	str	q0, [sp, #0xd0]
10127b814:     	ldur	q0, [x19, #0x18]
10127b818:     	str	q0, [sp, #0xb0]
10127b81c:     	ldur	x8, [x19, #0x28]
10127b820:     	str	x8, [sp, #0xc0]
10127b824:     	add	x0, sp, #0x70
10127b828:     	add	x1, sp, #0xb0
10127b82c:     	mov	x2, x22
10127b830:     	mov	x3, x23
10127b834:     	mov	w4, #0x1                ; =1
10127b838:     	bl	0x100b9e2c0 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
10127b83c:     	ldr	q0, [sp, #0x70]
10127b840:     	str	q0, [sp, #0x50]
10127b844:     	ldr	x8, [sp, #0x80]
10127b848:     	str	q0, [sp, #0x30]
10127b84c:     	str	q0, [sp, #0x90]
10127b850:     	str	x8, [sp, #0xa0]
10127b854:     	ldr	q0, [sp, #0x90]
10127b858:     	str	x8, [sp, #0xf8]
10127b85c:     	stur	q0, [x27, #0x18]
10127b860:     	ldp	q0, q1, [sp, #0xd0]
10127b864:     	ldr	q2, [sp, #0xf0]
10127b868:     	stp	q1, q2, [x29, #-0x90]
10127b86c:     	stur	q0, [x29, #-0xa0]
10127b870:     	ldp	q0, q1, [x29, #-0xa0]
10127b874:     	ldur	q2, [x29, #-0x80]
10127b878:     	stp	q1, q2, [sp, #0x10]
10127b87c:     	str	q0, [sp]
10127b880:     	mov	x1, sp
10127b884:     	mov	x0, x22
10127b888:     	bl	0x100c53b2c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
10127b88c:     	mov	x23, x0
10127b890:     	mov	x3, sp
10127b894:     	mov	x0, x22
10127b898:     	mov	x1, x21
10127b89c:     	mov	x2, x24
10127b8a0:     	mov	x4, x20
10127b8a4:     	bl	0x10127b420 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_>
10127b8a8:     	fmov	d0, x23
10127b8ac:     	cnt.8b	v0, v0
10127b8b0:     	addv.8b	b0, v0
10127b8b4:     	fmov	w8, s0
10127b8b8:     	sub	w8, w8, w26
10127b8bc:     	mvn	w8, w8
10127b8c0:     	lsl	x8, x0, x8
10127b8c4:     	add	x23, x8, x25
10127b8c8:     	b	0x10127bbfc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
10127b8cc:     	ldp	x15, x12, [x29, #-0x70]
10127b8d0:     	sub	x16, x13, #0x1
10127b8d4:     	cmn	x25, #0x2
10127b8d8:     	b.ne	0x10127b924 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x504>
10127b8dc:     	mov	x0, x15
10127b8e0:     	cmp	x15, x16
10127b8e4:     	b.ls	0x10127bc3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
10127b8e8:     	and	x9, x9, x14
10127b8ec:     	cmp	x13, #0x8
10127b8f0:     	b.hs	0x10127b960 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x540>
10127b8f4:     	mov	x23, #0x0               ; =0
10127b8f8:     	mov	x13, #0x0               ; =0
10127b8fc:     	b	0x10127b9e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5c8>
10127b900:     	sub	x12, x13, #0x1
10127b904:     	cmp	x0, x12
10127b908:     	tbz	w21, #0x0, 0x10127b954 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x534>
10127b90c:     	b.ls	0x10127bc3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
10127b910:     	cmp	x13, #0x8
10127b914:     	b.hs	0x10127bb20 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x700>
10127b918:     	mov	x23, #0x0               ; =0
10127b91c:     	mov	x13, #0x0               ; =0
10127b920:     	b	0x10127bba8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x788>
10127b924:     	cmp	x15, x16
10127b928:     	csel	x14, x15, x16, lo
10127b92c:     	cmp	x0, x14
10127b930:     	b.ls	0x10127bc3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
10127b934:     	mov	x0, x15
10127b938:     	cmp	x15, x14
10127b93c:     	b.eq	0x10127bc3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
10127b940:     	cmp	x13, #0x8
10127b944:     	b.hs	0x10127ba20 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x600>
10127b948:     	mov	x23, #0x0               ; =0
10127b94c:     	mov	x15, #0x0               ; =0
10127b950:     	b	0x10127bad4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6b4>
10127b954:     	b.ls	0x10127bc3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x81c>
10127b958:     	mov	x23, #0x0               ; =0
10127b95c:     	b	0x10127bbdc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
10127b960:     	dup.2d	v0, x12
10127b964:     	dup.2d	v1, x9
10127b968:     	ldp	q3, q2, [x21, #0x20]
10127b96c:     	eor.16b	v2, v2, v0
10127b970:     	and.16b	v2, v2, v1
10127b974:     	cnt.16b	v2, v2
10127b978:     	movi.16b	v4, #0x1
10127b97c:     	movi.2d	v5, #0000000000000000
10127b980:     	udot.4s	v5, v4, v2
10127b984:     	eor.16b	v2, v3, v0
10127b988:     	and.16b	v2, v2, v1
10127b98c:     	cnt.16b	v2, v2
10127b990:     	movi.2d	v3, #0000000000000000
10127b994:     	udot.4s	v3, v4, v2
10127b998:     	ldp	q6, q2, [x21]
10127b99c:     	eor.16b	v2, v2, v0
10127b9a0:     	and.16b	v2, v2, v1
10127b9a4:     	cnt.16b	v2, v2
10127b9a8:     	movi.2d	v7, #0000000000000000
10127b9ac:     	udot.4s	v7, v4, v2
10127b9b0:     	movi.2d	v2, #0000000000000000
10127b9b4:     	uaddlp.2d	v7, v7
10127b9b8:     	eor.16b	v0, v6, v0
10127b9bc:     	and.16b	v0, v0, v1
10127b9c0:     	cnt.16b	v0, v0
10127b9c4:     	udot.4s	v2, v4, v0
10127b9c8:     	uadalp.2d	v7, v2
10127b9cc:     	uadalp.2d	v7, v3
10127b9d0:     	uadalp.2d	v7, v5
10127b9d4:     	addp.2d	d0, v7
10127b9d8:     	fmov	x23, d0
10127b9dc:     	cmp	x13, #0x8
10127b9e0:     	b.eq	0x10127bbec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
10127b9e4:     	mov	w13, #0x8               ; =8
10127b9e8:     	add	x11, x21, x13, lsl #3
10127b9ec:     	add	x8, x10, x8
10127b9f0:     	sub	x8, x8, x13
10127b9f4:     	ldr	x10, [x11], #0x8
10127b9f8:     	eor	x10, x10, x12
10127b9fc:     	and	x10, x10, x9
10127ba00:     	fmov	d0, x10
10127ba04:     	cnt.8b	v0, v0
10127ba08:     	addv.8b	b0, v0
10127ba0c:     	fmov	x10, d0
10127ba10:     	add	x23, x10, x23
10127ba14:     	subs	x8, x8, #0x1
10127ba18:     	b.ne	0x10127b9f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x5d4>
10127ba1c:     	b	0x10127bbec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
10127ba20:     	dup.2d	v0, x11
10127ba24:     	ldp	q2, q1, [x22, #0x20]
10127ba28:     	eor.16b	v1, v1, v0
10127ba2c:     	dup.2d	v3, x12
10127ba30:     	ldp	q5, q4, [x21, #0x20]
10127ba34:     	eor.16b	v4, v4, v3
10127ba38:     	and.16b	v1, v1, v4
10127ba3c:     	dup.2d	v4, x9
10127ba40:     	and.16b	v1, v1, v4
10127ba44:     	cnt.16b	v1, v1
10127ba48:     	movi.16b	v6, #0x1
10127ba4c:     	movi.2d	v7, #0000000000000000
10127ba50:     	udot.4s	v7, v6, v1
10127ba54:     	eor.16b	v1, v2, v0
10127ba58:     	eor.16b	v2, v5, v3
10127ba5c:     	and.16b	v1, v1, v2
10127ba60:     	and.16b	v1, v1, v4
10127ba64:     	cnt.16b	v1, v1
10127ba68:     	movi.2d	v2, #0000000000000000
10127ba6c:     	udot.4s	v2, v6, v1
10127ba70:     	ldp	q5, q1, [x22]
10127ba74:     	eor.16b	v1, v1, v0
10127ba78:     	ldp	q17, q16, [x21]
10127ba7c:     	eor.16b	v16, v16, v3
10127ba80:     	and.16b	v1, v1, v16
10127ba84:     	and.16b	v1, v1, v4
10127ba88:     	cnt.16b	v1, v1
10127ba8c:     	movi.2d	v16, #0000000000000000
10127ba90:     	udot.4s	v16, v6, v1
10127ba94:     	eor.16b	v0, v5, v0
10127ba98:     	movi.2d	v1, #0000000000000000
10127ba9c:     	uaddlp.2d	v5, v16
10127baa0:     	eor.16b	v3, v17, v3
10127baa4:     	and.16b	v0, v0, v3
10127baa8:     	and.16b	v0, v0, v4
10127baac:     	cnt.16b	v0, v0
10127bab0:     	udot.4s	v1, v6, v0
10127bab4:     	uadalp.2d	v5, v1
10127bab8:     	uadalp.2d	v5, v2
10127babc:     	uadalp.2d	v5, v7
10127bac0:     	addp.2d	d0, v5
10127bac4:     	fmov	x23, d0
10127bac8:     	cmp	x13, #0x8
10127bacc:     	b.eq	0x10127bbdc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
10127bad0:     	mov	w15, #0x8               ; =8
10127bad4:     	lsl	x14, x15, #3
10127bad8:     	add	x13, x21, x14
10127badc:     	add	x14, x22, x14
10127bae0:     	add	x8, x10, x8
10127bae4:     	sub	x8, x8, x15
10127bae8:     	ldr	x10, [x14], #0x8
10127baec:     	eor	x10, x10, x11
10127baf0:     	ldr	x15, [x13], #0x8
10127baf4:     	eor	x15, x15, x12
10127baf8:     	and	x10, x10, x15
10127bafc:     	and	x10, x10, x9
10127bb00:     	fmov	d0, x10
10127bb04:     	cnt.8b	v0, v0
10127bb08:     	addv.8b	b0, v0
10127bb0c:     	fmov	x10, d0
10127bb10:     	add	x23, x10, x23
10127bb14:     	subs	x8, x8, #0x1
10127bb18:     	b.ne	0x10127bae8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x6c8>
10127bb1c:     	b	0x10127bbdc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
10127bb20:     	dup.2d	v0, x11
10127bb24:     	dup.2d	v1, x9
10127bb28:     	ldp	q3, q2, [x22, #0x20]
10127bb2c:     	eor.16b	v2, v2, v0
10127bb30:     	and.16b	v2, v2, v1
10127bb34:     	cnt.16b	v2, v2
10127bb38:     	movi.16b	v4, #0x1
10127bb3c:     	movi.2d	v5, #0000000000000000
10127bb40:     	udot.4s	v5, v4, v2
10127bb44:     	eor.16b	v2, v3, v0
10127bb48:     	and.16b	v2, v2, v1
10127bb4c:     	cnt.16b	v2, v2
10127bb50:     	movi.2d	v3, #0000000000000000
10127bb54:     	udot.4s	v3, v4, v2
10127bb58:     	ldp	q6, q2, [x22]
10127bb5c:     	eor.16b	v2, v2, v0
10127bb60:     	and.16b	v2, v2, v1
10127bb64:     	cnt.16b	v2, v2
10127bb68:     	movi.2d	v7, #0000000000000000
10127bb6c:     	udot.4s	v7, v4, v2
10127bb70:     	movi.2d	v2, #0000000000000000
10127bb74:     	uaddlp.2d	v7, v7
10127bb78:     	eor.16b	v0, v6, v0
10127bb7c:     	and.16b	v0, v0, v1
10127bb80:     	cnt.16b	v0, v0
10127bb84:     	udot.4s	v2, v4, v0
10127bb88:     	uadalp.2d	v7, v2
10127bb8c:     	uadalp.2d	v7, v3
10127bb90:     	uadalp.2d	v7, v5
10127bb94:     	addp.2d	d0, v7
10127bb98:     	fmov	x23, d0
10127bb9c:     	cmp	x13, #0x8
10127bba0:     	b.eq	0x10127bbdc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7bc>
10127bba4:     	mov	w13, #0x8               ; =8
10127bba8:     	add	x12, x22, x13, lsl #3
10127bbac:     	add	x8, x10, x8
10127bbb0:     	sub	x8, x8, x13
10127bbb4:     	ldr	x10, [x12], #0x8
10127bbb8:     	eor	x10, x10, x11
10127bbbc:     	and	x10, x10, x9
10127bbc0:     	fmov	d0, x10
10127bbc4:     	cnt.8b	v0, v0
10127bbc8:     	addv.8b	b0, v0
10127bbcc:     	fmov	x10, d0
10127bbd0:     	add	x23, x10, x23
10127bbd4:     	subs	x8, x8, #0x1
10127bbd8:     	b.ne	0x10127bbb4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x794>
10127bbdc:     	cmp	x25, #0x1
10127bbe0:     	b.lt	0x10127bbec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7cc>
10127bbe4:     	mov	x0, x22
10127bbe8:     	bl	0x101a6e678 <dyld_stub_binder+0x101a6e678>
10127bbec:     	cmp	x24, #0x1
10127bbf0:     	b.lt	0x10127bbfc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x7dc>
10127bbf4:     	mov	x0, x21
10127bbf8:     	bl	0x101a6e678 <dyld_stub_binder+0x101a6e678>
10127bbfc:     	mov	x0, x20
10127bc00:     	mov	x1, x19
10127bc04:     	mov	x2, x23
10127bc08:     	bl	0x1013b0d7c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
10127bc0c:     	mov	x0, x23
10127bc10:     	ldp	x29, x30, [sp, #0x1d0]
10127bc14:     	ldp	x20, x19, [sp, #0x1c0]
10127bc18:     	ldp	x22, x21, [sp, #0x1b0]
10127bc1c:     	ldp	x24, x23, [sp, #0x1a0]
10127bc20:     	ldp	x26, x25, [sp, #0x190]
10127bc24:     	ldp	x28, x27, [sp, #0x180]
10127bc28:     	add	sp, sp, #0x1e0
10127bc2c:     	ret
10127bc30:     	adrp	x0, 0x101cee000 <dyld_stub_binder+0x101cee000>
10127bc34:     	add	x0, x0, #0xd0
10127bc38:     	bl	0x101a662f4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
10127bc3c:     	adrp	x2, 0x101ced000 <dyld_stub_binder+0x101ced000>
10127bc40:     	add	x2, x2, #0x440
10127bc44:     	mov	x1, x0
10127bc48:     	bl	0x101a6625c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10127bc4c:     	brk	#0x1
10127bc50:     	mov	x19, x0
10127bc54:     	sub	x0, x29, #0xa0
10127bc58:     	bl	0x100c04a50 <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej2_EBK_>
10127bc5c:     	mov	x0, x19
10127bc60:     	bl	0x101a6e4c8 <dyld_stub_binder+0x101a6e4c8>
10127bc64:     	mov	x19, x0
10127bc68:     	cmp	x25, #0x1
10127bc6c:     	b.lt	0x10127bc78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb1_EB8_+0x858>
10127bc70:     	mov	x0, x22
10127bc74:     	bl	0x101a6e678 <dyld_stub_binder+0x101a6e678>
10127bc78:     	mov	x0, x19
10127bc7c:     	bl	0x101a6e4c8 <dyld_stub_binder+0x101a6e4c8>
