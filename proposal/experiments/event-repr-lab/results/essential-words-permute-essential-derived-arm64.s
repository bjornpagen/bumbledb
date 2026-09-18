
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba4580 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>:
100ba4580:     	sub	sp, sp, #0x80
100ba4584:     	stp	x28, x27, [sp, #0x20]
100ba4588:     	stp	x26, x25, [sp, #0x30]
100ba458c:     	stp	x24, x23, [sp, #0x40]
100ba4590:     	stp	x22, x21, [sp, #0x50]
100ba4594:     	stp	x20, x19, [sp, #0x60]
100ba4598:     	stp	x29, x30, [sp, #0x70]
100ba459c:     	add	x29, sp, #0x70
100ba45a0:     	cmp	x3, #0x15
100ba45a4:     	b.hs	0x100ba46a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x128>
100ba45a8:     	mov	x19, x3
100ba45ac:     	mov	x20, x1
100ba45b0:     	mov	w8, #0x1                ; =1
100ba45b4:     	lsl	x8, x8, x3
100ba45b8:     	lsr	x8, x8, #6
100ba45bc:     	cmp	x3, #0x6
100ba45c0:     	cinc	x8, x8, lo
100ba45c4:     	stp	x1, x8, [sp, #0x10]
100ba45c8:     	cmp	x1, x8
100ba45cc:     	b.ne	0x100ba46c0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x140>
100ba45d0:     	cbz	x19, 0x100ba4638 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xb8>
100ba45d4:     	mov	x21, x2
100ba45d8:     	mov	x23, x0
100ba45dc:     	mov	x8, #0x0                ; =0
100ba45e0:     	lsl	x24, x19, #2
100ba45e4:     	add	x25, x2, x24
100ba45e8:     	mov	w9, #0x1                ; =1
100ba45ec:     	mov	x10, x24
100ba45f0:     	mov	x11, x2
100ba45f4:     	ldr	w12, [x11], #0x4
100ba45f8:     	cmp	w12, w19
100ba45fc:     	b.hs	0x100ba4690 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100ba4600:     	lsr	x13, x8, x12
100ba4604:     	tbnz	w13, #0x0, 0x100ba4690 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x110>
100ba4608:     	lsl	x12, x9, x12
100ba460c:     	orr	x8, x12, x8
100ba4610:     	subs	x10, x10, #0x4
100ba4614:     	b.ne	0x100ba45f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x74>
100ba4618:     	mov	x0, x24
100ba461c:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba4620:     	cbz	x0, 0x100ba46dc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x15c>
100ba4624:     	mov	x22, x0
100ba4628:     	cmp	x19, #0x8
100ba462c:     	b.hs	0x100ba4658 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0xd8>
100ba4630:     	mov	x8, #0x0                ; =0
100ba4634:     	b	0x100ba46e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100ba4638:     	ldp	x29, x30, [sp, #0x70]
100ba463c:     	ldp	x20, x19, [sp, #0x60]
100ba4640:     	ldp	x22, x21, [sp, #0x50]
100ba4644:     	ldp	x24, x23, [sp, #0x40]
100ba4648:     	ldp	x26, x25, [sp, #0x30]
100ba464c:     	ldp	x28, x27, [sp, #0x20]
100ba4650:     	add	sp, sp, #0x80
100ba4654:     	ret
100ba4658:     	and	x8, x19, #0x18
100ba465c:     	adrp	x9, 0x101198000 <GCC_except_table8889+0x8>
100ba4660:     	ldr	q0, [x9, #0xcb0]
100ba4664:     	adrp	x9, 0x101198000 <GCC_except_table8889+0x8>
100ba4668:     	ldr	q1, [x9, #0xdb0]
100ba466c:     	stp	q0, q1, [x22]
100ba4670:     	cmp	x8, #0x8
100ba4674:     	b.eq	0x100ba46f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100ba4678:     	adrp	x9, 0x101199000 <dyld_stub_binder+0x101199000>
100ba467c:     	ldr	q0, [x9, #0x450]
100ba4680:     	adrp	x9, 0x101199000 <dyld_stub_binder+0x101199000>
100ba4684:     	ldr	q1, [x9, #0x460]
100ba4688:     	stp	q0, q1, [x22, #0x20]
100ba468c:     	b	0x100ba46f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x170>
100ba4690:     	adrp	x0, 0x1011c5000 <dyld_stub_binder+0x1011c5000>
100ba4694:     	add	x0, x0, #0x97c
100ba4698:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba469c:     	add	x2, x2, #0x8c8
100ba46a0:     	mov	w1, #0x49               ; =73
100ba46a4:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba46a8:     	adrp	x0, 0x1011c5000 <dyld_stub_binder+0x1011c5000>
100ba46ac:     	add	x0, x0, #0x952
100ba46b0:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba46b4:     	add	x2, x2, #0x898
100ba46b8:     	mov	w1, #0x2a               ; =42
100ba46bc:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba46c0:     	adrp	x5, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba46c4:     	add	x5, x5, #0x8b0
100ba46c8:     	add	x1, sp, #0x10
100ba46cc:     	add	x2, sp, #0x18
100ba46d0:     	mov	w0, #0x0                ; =0
100ba46d4:     	mov	x3, #0x0                ; =0
100ba46d8:     	bl	0x101104c30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100ba46dc:     	mov	w0, #0x4                ; =4
100ba46e0:     	mov	x1, x24
100ba46e4:     	bl	0x101104564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba46e8:     	str	w8, [x22, x8, lsl #2]
100ba46ec:     	add	x8, x8, #0x1
100ba46f0:     	cmp	x19, x8
100ba46f4:     	b.ne	0x100ba46e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x168>
100ba46f8:     	mov	w10, #0x0               ; =0
100ba46fc:     	lsl	x9, x20, #3
100ba4700:     	add	x8, x23, x9
100ba4704:     	sub	x9, x9, #0x8
100ba4708:     	lsr	x11, x9, #3
100ba470c:     	add	x11, x11, #0x1
100ba4710:     	and	x12, x11, #0x3ffffffffffffff8
100ba4714:     	add	x13, x23, x12, lsl #3
100ba4718:     	adrp	x12, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba471c:     	add	x12, x12, #0x8f8
100ba4720:     	stp	x12, x13, [sp]
100ba4724:     	adrp	x14, 0x1011a3000 <dyld_stub_binder+0x1011a3000>
100ba4728:     	add	x14, x14, #0x570
100ba472c:     	mov	w16, #0x1               ; =1
100ba4730:     	b	0x100ba473c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1bc>
100ba4734:     	cmp	x21, x25
100ba4738:     	b.eq	0x100ba4a58 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4d8>
100ba473c:     	mov	x17, #0x0               ; =0
100ba4740:     	mov	x12, x10
100ba4744:     	ldr	w0, [x21], #0x4
100ba4748:     	add	w10, w10, #0x1
100ba474c:     	mov	x13, x24
100ba4750:     	ldr	w1, [x22, x17, lsl #2]
100ba4754:     	cmp	w1, w12
100ba4758:     	b.eq	0x100ba476c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1ec>
100ba475c:     	add	x17, x17, #0x1
100ba4760:     	subs	x13, x13, #0x4
100ba4764:     	b.ne	0x100ba4750 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1d0>
100ba4768:     	b	0x100ba4ab4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x534>
100ba476c:     	cmp	w0, w17
100ba4770:     	b.eq	0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba4774:     	cmp	w0, w17
100ba4778:     	csel	w13, w0, w17, lo
100ba477c:     	csel	w1, w0, w17, hi
100ba4780:     	cmp	x19, x0
100ba4784:     	b.ls	0x100ba4a48 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4c8>
100ba4788:     	ldr	w12, [x22, x17, lsl #2]
100ba478c:     	ldr	w2, [x22, x0, lsl #2]
100ba4790:     	str	w2, [x22, x17, lsl #2]
100ba4794:     	str	w12, [x22, x0, lsl #2]
100ba4798:     	cmp	w1, #0x6
100ba479c:     	b.hs	0x100ba4888 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x308>
100ba47a0:     	cbz	x20, 0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba47a4:     	ldr	x12, [x14, w13, uxtw #3]
100ba47a8:     	ldr	x17, [x14, w1, uxtw #3]
100ba47ac:     	bic	x17, x17, x12
100ba47b0:     	mov	w12, #-0x1              ; =-1
100ba47b4:     	lsl	w12, w12, w13
100ba47b8:     	lsl	w13, w16, w1
100ba47bc:     	add	w12, w12, w13
100ba47c0:     	and	w0, w12, #0x3f
100ba47c4:     	mov	x12, x23
100ba47c8:     	cmp	x9, #0x38
100ba47cc:     	b.lo	0x100ba485c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100ba47d0:     	dup.2d	v0, x0
100ba47d4:     	dup.2d	v1, x17
100ba47d8:     	neg.2d	v2, v0
100ba47dc:     	add	x13, x23, #0x20
100ba47e0:     	and	x1, x11, #0x3ffffffffffffff8
100ba47e4:     	ldp	q3, q4, [x13, #-0x20]
100ba47e8:     	ldp	q5, q6, [x13]
100ba47ec:     	ushl.2d	v7, v3, v2
100ba47f0:     	ushl.2d	v16, v4, v2
100ba47f4:     	ushl.2d	v17, v5, v2
100ba47f8:     	ushl.2d	v18, v6, v2
100ba47fc:     	eor.16b	v7, v7, v3
100ba4800:     	eor.16b	v16, v16, v4
100ba4804:     	eor.16b	v17, v17, v5
100ba4808:     	eor.16b	v18, v18, v6
100ba480c:     	and.16b	v7, v1, v7
100ba4810:     	and.16b	v16, v1, v16
100ba4814:     	and.16b	v17, v1, v17
100ba4818:     	and.16b	v18, v1, v18
100ba481c:     	ushl.2d	v19, v7, v0
100ba4820:     	ushl.2d	v20, v16, v0
100ba4824:     	ushl.2d	v21, v17, v0
100ba4828:     	ushl.2d	v22, v18, v0
100ba482c:     	eor3.16b	v3, v3, v19, v7
100ba4830:     	eor3.16b	v4, v4, v20, v16
100ba4834:     	eor3.16b	v5, v5, v21, v17
100ba4838:     	stp	q3, q4, [x13, #-0x20]
100ba483c:     	eor3.16b	v3, v6, v22, v18
100ba4840:     	stp	q5, q3, [x13], #0x40
100ba4844:     	subs	x1, x1, #0x8
100ba4848:     	b.ne	0x100ba47e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x264>
100ba484c:     	ldr	x12, [sp, #0x8]
100ba4850:     	and	x13, x11, #0x3ffffffffffffff8
100ba4854:     	cmp	x11, x13
100ba4858:     	b.eq	0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba485c:     	ldr	x13, [x12]
100ba4860:     	lsr	x15, x13, x0
100ba4864:     	eor	x15, x15, x13
100ba4868:     	and	x15, x17, x15
100ba486c:     	lsl	x1, x15, x0
100ba4870:     	eor	x13, x13, x15
100ba4874:     	eor	x13, x13, x1
100ba4878:     	str	x13, [x12], #0x8
100ba487c:     	cmp	x12, x8
100ba4880:     	b.ne	0x100ba485c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x2dc>
100ba4884:     	b	0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba4888:     	cmp	w13, #0x6
100ba488c:     	b.hs	0x100ba49e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x460>
100ba4890:     	add	w17, w1, #0x3a
100ba4894:     	and	w12, w17, #0x3f
100ba4898:     	cmp	w12, #0x3f
100ba489c:     	b.eq	0x100ba4a98 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x518>
100ba48a0:     	cbz	x20, 0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba48a4:     	lsl	x0, x16, x17
100ba48a8:     	ldr	x4, [x14, w13, uxtw #3]
100ba48ac:     	lsl	w5, w16, w13
100ba48b0:     	mov	w13, #0x2               ; =2
100ba48b4:     	lsl	x6, x13, x12
100ba48b8:     	mov	w13, #0x8               ; =8
100ba48bc:     	lsl	x7, x13, x12
100ba48c0:     	lsr	x26, x7, #3
100ba48c4:     	dup.2d	v0, x5
100ba48c8:     	dup.2d	v1, x4
100ba48cc:     	lsl	x27, x0, #3
100ba48d0:     	neg.2d	v2, v0
100ba48d4:     	mov	x28, x20
100ba48d8:     	mov	x30, x23
100ba48dc:     	b	0x100ba48ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x36c>
100ba48e0:     	add	x30, x30, x17, lsl #3
100ba48e4:     	sub	x28, x28, x17
100ba48e8:     	cbz	x28, 0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba48ec:     	cmp	x6, x28
100ba48f0:     	csel	x17, x6, x28, lo
100ba48f4:     	subs	x12, x17, x0
100ba48f8:     	b.lo	0x100ba4a7c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x4fc>
100ba48fc:     	cmp	x12, x26
100ba4900:     	csel	x13, x12, x26, lo
100ba4904:     	cmp	x17, x0
100ba4908:     	ccmp	x30, #0x0, #0x4, ne
100ba490c:     	b.eq	0x100ba48e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba4910:     	cmp	x13, #0x4
100ba4914:     	b.lo	0x100ba499c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100ba4918:     	add	x12, x30, x0, lsl #3
100ba491c:     	add	x1, x30, x13, lsl #3
100ba4920:     	add	x2, x1, x7
100ba4924:     	cmp	x30, x2
100ba4928:     	ccmp	x12, x1, #0x2, lo
100ba492c:     	b.lo	0x100ba499c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x41c>
100ba4930:     	and	x2, x13, #0x1ffffffffffffffc
100ba4934:     	add	x1, x30, #0x10
100ba4938:     	add	x3, x1, x27
100ba493c:     	and	x12, x13, #0x1ffffffffffffffc
100ba4940:     	ldp	q3, q4, [x1, #-0x10]
100ba4944:     	ushl.2d	v5, v3, v2
100ba4948:     	ushl.2d	v6, v4, v2
100ba494c:     	ldp	q7, q16, [x3, #-0x10]
100ba4950:     	eor.16b	v5, v5, v7
100ba4954:     	eor.16b	v6, v6, v16
100ba4958:     	and.16b	v5, v5, v1
100ba495c:     	and.16b	v6, v6, v1
100ba4960:     	ushl.2d	v17, v5, v0
100ba4964:     	ushl.2d	v18, v6, v0
100ba4968:     	eor.16b	v3, v17, v3
100ba496c:     	eor.16b	v4, v18, v4
100ba4970:     	stp	q3, q4, [x1, #-0x10]
100ba4974:     	eor.16b	v3, v5, v7
100ba4978:     	eor.16b	v4, v6, v16
100ba497c:     	stp	q3, q4, [x3, #-0x10]
100ba4980:     	add	x3, x3, #0x20
100ba4984:     	add	x1, x1, #0x20
100ba4988:     	subs	x12, x12, #0x4
100ba498c:     	b.ne	0x100ba4940 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x3c0>
100ba4990:     	cmp	x13, x2
100ba4994:     	b.eq	0x100ba48e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba4998:     	b	0x100ba49a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x420>
100ba499c:     	mov	x2, #0x0                ; =0
100ba49a0:     	sub	x12, x13, x2
100ba49a4:     	add	x13, x30, x2, lsl #3
100ba49a8:     	ldr	x1, [x13]
100ba49ac:     	lsr	x2, x1, x5
100ba49b0:     	ldr	x3, [x13, x27]
100ba49b4:     	eor	x2, x2, x3
100ba49b8:     	and	x2, x2, x4
100ba49bc:     	lsl	x15, x2, x5
100ba49c0:     	eor	x15, x15, x1
100ba49c4:     	str	x15, [x13]
100ba49c8:     	eor	x15, x2, x3
100ba49cc:     	str	x15, [x13, x27]
100ba49d0:     	add	x13, x13, #0x8
100ba49d4:     	subs	x12, x12, #0x1
100ba49d8:     	b.ne	0x100ba49a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x428>
100ba49dc:     	b	0x100ba48e0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x360>
100ba49e0:     	cbz	x20, 0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba49e4:     	mov	x17, #0x0               ; =0
100ba49e8:     	add	w12, w13, #0x3a
100ba49ec:     	lsl	x12, x16, x12
100ba49f0:     	add	w13, w1, #0x3a
100ba49f4:     	lsl	x13, x16, x13
100ba49f8:     	eor	x1, x13, x12
100ba49fc:     	b	0x100ba4a1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x49c>
100ba4a00:     	ldr	x2, [x23, x17, lsl #3]
100ba4a04:     	ldr	x3, [x23, x0, lsl #3]
100ba4a08:     	str	x3, [x23, x17, lsl #3]
100ba4a0c:     	str	x2, [x23, x0, lsl #3]
100ba4a10:     	add	x17, x17, #0x1
100ba4a14:     	cmp	x20, x17
100ba4a18:     	b.eq	0x100ba4734 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x1b4>
100ba4a1c:     	tst	x17, x12
100ba4a20:     	b.eq	0x100ba4a10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100ba4a24:     	and	x0, x17, x13
100ba4a28:     	cbnz	x0, 0x100ba4a10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x490>
100ba4a2c:     	eor	x0, x1, x17
100ba4a30:     	cmp	x0, x20
100ba4a34:     	b.lo	0x100ba4a00 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x480>
100ba4a38:     	mov	x19, x20
100ba4a3c:     	adrp	x8, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba4a40:     	add	x8, x8, #0x910
100ba4a44:     	str	x8, [sp]
100ba4a48:     	mov	x1, x19
100ba4a4c:     	ldr	x2, [sp]
100ba4a50:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100ba4a54:     	b	0x100ba4ac0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba4a58:     	mov	x0, x22
100ba4a5c:     	ldp	x29, x30, [sp, #0x70]
100ba4a60:     	ldp	x20, x19, [sp, #0x60]
100ba4a64:     	ldp	x22, x21, [sp, #0x50]
100ba4a68:     	ldp	x24, x23, [sp, #0x40]
100ba4a6c:     	ldp	x26, x25, [sp, #0x30]
100ba4a70:     	ldp	x28, x27, [sp, #0x20]
100ba4a74:     	add	sp, sp, #0x80
100ba4a78:     	b	0x10110d138 <dyld_stub_binder+0x10110d138>
100ba4a7c:     	adrp	x0, 0x10125e000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xdc8>
100ba4a80:     	add	x0, x0, #0xb9b
100ba4a84:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba4a88:     	add	x2, x2, #0x940
100ba4a8c:     	mov	w1, #0x13               ; =19
100ba4a90:     	bl	0x101104bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba4a94:     	b	0x100ba4ac0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba4a98:     	adrp	x0, 0x1011a1000 <dyld_stub_binder+0x1011a1000>
100ba4a9c:     	add	x0, x0, #0xf21
100ba4aa0:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba4aa4:     	add	x2, x2, #0x928
100ba4aa8:     	mov	w1, #0x37               ; =55
100ba4aac:     	bl	0x101104bf4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100ba4ab0:     	b	0x100ba4ac0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute+0x540>
100ba4ab4:     	adrp	x0, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba4ab8:     	add	x0, x0, #0x8e0
100ba4abc:     	bl	0x101104df4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100ba4ac0:     	brk	#0x1
100ba4ac4:     	mov	x19, x0
100ba4ac8:     	mov	x0, x22
100ba4acc:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100ba4ad0:     	mov	x0, x19
100ba4ad4:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
