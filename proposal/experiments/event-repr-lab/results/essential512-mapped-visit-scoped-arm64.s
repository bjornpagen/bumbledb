
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001008a4678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>:
1008a4678:     	stp	x28, x27, [sp, #-0x60]!
1008a467c:     	stp	x26, x25, [sp, #0x10]
1008a4680:     	stp	x24, x23, [sp, #0x20]
1008a4684:     	stp	x22, x21, [sp, #0x30]
1008a4688:     	stp	x20, x19, [sp, #0x40]
1008a468c:     	stp	x29, x30, [sp, #0x50]
1008a4690:     	add	x29, sp, #0x50
1008a4694:     	sub	sp, sp, #0x1c0
1008a4698:     	mov	x23, x2
1008a469c:     	mov	x21, x1
1008a46a0:     	mov	x28, x0
1008a46a4:     	ldrb	w8, [x0, #0x151]
1008a46a8:     	str	x0, [sp, #0x88]
1008a46ac:     	str	x2, [sp, #0x60]
1008a46b0:     	cbz	w8, 0x1008a49d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
1008a46b4:     	mov	x27, #0x0               ; =0
1008a46b8:     	b	0x1008a46d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
1008a46bc:     	ldr	w9, [x26, #0x14]
1008a46c0:     	add	x27, x27, #0x18
1008a46c4:     	stp	xzr, x20, [x26]
1008a46c8:     	stp	w24, w9, [x26, #0x10]
1008a46cc:     	cmp	x27, #0x30
1008a46d0:     	b.eq	0x1008a49d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
1008a46d4:     	add	x26, x23, x27
1008a46d8:     	ldp	x19, x20, [x26]
1008a46dc:     	ldr	w24, [x26, #0x10]
1008a46e0:     	cbz	x19, 0x1008a46bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x44>
1008a46e4:     	ldr	x8, [x28, #0x138]
1008a46e8:     	add	x8, x8, #0x1
1008a46ec:     	str	x8, [x28, #0x138]
1008a46f0:     	ldur	x8, [x21, #0x40]
1008a46f4:     	lsr	x0, x24, #1
1008a46f8:     	cmn	x8, #0x1
1008a46fc:     	str	w9, [sp, #0x70]
1008a4700:     	b.eq	0x1008a471c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa4>
1008a4704:     	ldr	x1, [x21, #0x50]
1008a4708:     	cmp	x1, x0
1008a470c:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a4710:     	ldr	x8, [x21, #0x48]
1008a4714:     	add	x8, x8, x0, lsl #4
1008a4718:     	b	0x1008a4734 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbc>
1008a471c:     	ldr	x1, [x21, #0x58]
1008a4720:     	cmp	x1, x0
1008a4724:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a4728:     	ldr	x8, [x21, #0x50]
1008a472c:     	add	x8, x8, x0, lsl #5
1008a4730:     	add	x8, x8, #0x18
1008a4734:     	mov	x25, #0x0               ; =0
1008a4738:     	ldr	x8, [x8]
1008a473c:     	bic	x8, x8, x19
1008a4740:     	str	x8, [sp, #0x78]
1008a4744:     	mov	w8, #0x4                ; =4
1008a4748:     	stp	xzr, x8, [sp, #0xf0]
1008a474c:     	str	xzr, [sp, #0x100]
1008a4750:     	mov	w9, #0x4                ; =4
1008a4754:     	mov	w8, #0x4                ; =4
1008a4758:     	b	0x1008a4780 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x108>
1008a475c:     	rbit	x9, x19
1008a4760:     	clz	x9, x9
1008a4764:     	str	w9, [x8, x25, lsl #2]
1008a4768:     	add	x25, x25, #0x1
1008a476c:     	str	x25, [sp, #0x100]
1008a4770:     	sub	x10, x19, #0x1
1008a4774:     	add	x9, x23, #0x4
1008a4778:     	ands	x19, x10, x19
1008a477c:     	b.eq	0x1008a47a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x128>
1008a4780:     	mov	x23, x9
1008a4784:     	ldr	x9, [sp, #0xf0]
1008a4788:     	cmp	x25, x9
1008a478c:     	b.ne	0x1008a475c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
1008a4790:     	add	x0, sp, #0xf0
1008a4794:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a4798:     	ldr	x8, [sp, #0xf8]
1008a479c:     	b	0x1008a475c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
1008a47a0:     	ldp	x9, x8, [sp, #0xf0]
1008a47a4:     	str	x9, [sp, #0x80]
1008a47a8:     	str	x8, [sp, #0x68]
1008a47ac:     	cbz	x25, 0x1008a4920 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
1008a47b0:     	ldr	x19, [x28, #0x140]
1008a47b4:     	mov	x28, x8
1008a47b8:     	b	0x1008a47f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x178>
1008a47bc:     	tst	w22, #0x1
1008a47c0:     	mov	w8, #0x8                ; =8
1008a47c4:     	mov	w9, #0xc                ; =12
1008a47c8:     	csel	x8, x9, x8, ne
1008a47cc:     	add	x9, sp, #0xf0
1008a47d0:     	ldr	w8, [x9, x8]
1008a47d4:     	and	w9, w24, #0x1
1008a47d8:     	eor	w24, w8, w9
1008a47dc:     	add	x19, x19, #0x1
1008a47e0:     	ldr	x8, [sp, #0x88]
1008a47e4:     	str	x19, [x8, #0x140]
1008a47e8:     	subs	x23, x23, #0x4
1008a47ec:     	b.eq	0x1008a4920 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
1008a47f0:     	ldr	w25, [x28], #0x4
1008a47f4:     	ldur	x8, [x21, #0x40]
1008a47f8:     	lsr	w0, w24, #1
1008a47fc:     	cmn	x8, #0x1
1008a4800:     	b.eq	0x1008a4830 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1b8>
1008a4804:     	ldr	x1, [x21, #0x50]
1008a4808:     	cmp	x1, x0
1008a480c:     	b.ls	0x1008a59d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1360>
1008a4810:     	ldr	x9, [x21, #0x48]
1008a4814:     	add	x9, x9, x0, lsl #4
1008a4818:     	ldr	x10, [x9]
1008a481c:     	mov	w9, #0x1                ; =1
1008a4820:     	lsl	x9, x9, x25
1008a4824:     	tst	x10, x9
1008a4828:     	b.ne	0x1008a4858 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1e0>
1008a482c:     	b	0x1008a47e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1008a4830:     	ldr	x1, [x21, #0x58]
1008a4834:     	cmp	x1, x0
1008a4838:     	b.ls	0x1008a59f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x137c>
1008a483c:     	ldr	x1, [x21, #0x50]
1008a4840:     	add	x9, x1, x0, lsl #5
1008a4844:     	ldr	x10, [x9, #0x18]!
1008a4848:     	mov	w9, #0x1                ; =1
1008a484c:     	lsl	x9, x9, x25
1008a4850:     	tst	x10, x9
1008a4854:     	b.eq	0x1008a47e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1008a4858:     	ldr	w10, [x21, #0xf0]
1008a485c:     	cmp	w25, w10
1008a4860:     	b.hs	0x1008a4ba8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x530>
1008a4864:     	cmn	x8, #0x1
1008a4868:     	b.eq	0x1008a488c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x214>
1008a486c:     	cmp	x1, x0
1008a4870:     	b.ls	0x1008a59e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x136c>
1008a4874:     	ldr	x8, [x21, #0x48]
1008a4878:     	add	x8, x8, x0, lsl #4
1008a487c:     	ldr	x8, [x8]
1008a4880:     	tst	x8, x9
1008a4884:     	b.ne	0x1008a48ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x234>
1008a4888:     	b	0x1008a47dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1008a488c:     	ldr	x8, [x21, #0x58]
1008a4890:     	cmp	x8, x0
1008a4894:     	b.ls	0x1008a5a20 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13a8>
1008a4898:     	add	x8, x1, x0, lsl #5
1008a489c:     	add	x8, x8, #0x18
1008a48a0:     	ldr	x8, [x8]
1008a48a4:     	tst	x8, x9
1008a48a8:     	b.eq	0x1008a47dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1008a48ac:     	and	x8, x25, #0x3f
1008a48b0:     	lsr	x22, x20, x8
1008a48b4:     	ldrb	w8, [x21, #0xf5]
1008a48b8:     	tbz	w8, #0x0, 0x1008a4904 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x28c>
1008a48bc:     	add	x0, sp, #0xf0
1008a48c0:     	add	x1, x21, #0x40
1008a48c4:     	mov	x2, x24
1008a48c8:     	bl	0x100ee9280 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1008a48cc:     	ldr	w8, [sp, #0xf0]
1008a48d0:     	cmp	w8, #0x2
1008a48d4:     	b.ne	0x1008a48e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x26c>
1008a48d8:     	ldr	w8, [sp, #0xf4]
1008a48dc:     	cmp	w8, w25
1008a48e0:     	b.eq	0x1008a47bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x144>
1008a48e4:     	and	w1, w24, #0xfffffffe
1008a48e8:     	and	w3, w22, #0x1
1008a48ec:     	mov	x0, x21
1008a48f0:     	mov	x2, x25
1008a48f4:     	bl	0x100df9300 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
1008a48f8:     	and	w8, w24, #0x1
1008a48fc:     	eor	w24, w0, w8
1008a4900:     	b	0x1008a47dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1008a4904:     	and	w3, w22, #0x1
1008a4908:     	mov	x0, x21
1008a490c:     	mov	x1, x24
1008a4910:     	mov	x2, x25
1008a4914:     	bl	0x100df9300 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
1008a4918:     	mov	x24, x0
1008a491c:     	b	0x1008a47dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1008a4920:     	ldr	x8, [sp, #0x80]
1008a4924:     	cbz	x8, 0x1008a4930 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2b8>
1008a4928:     	ldr	x0, [sp, #0x68]
1008a492c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a4930:     	ldur	x8, [x21, #0x40]
1008a4934:     	lsr	w0, w24, #1
1008a4938:     	cmn	x8, #0x1
1008a493c:     	ldr	x28, [sp, #0x88]
1008a4940:     	ldr	x23, [sp, #0x60]
1008a4944:     	ldr	x10, [sp, #0x78]
1008a4948:     	b.eq	0x1008a4974 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2fc>
1008a494c:     	ldr	x1, [x21, #0x50]
1008a4950:     	cmp	x1, x0
1008a4954:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a4958:     	ldr	x8, [x21, #0x48]
1008a495c:     	add	x8, x8, x0, lsl #4
1008a4960:     	ldr	x8, [x8]
1008a4964:     	bics	x9, x8, x10
1008a4968:     	str	x9, [sp, #0xf0]
1008a496c:     	b.eq	0x1008a499c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x324>
1008a4970:     	b	0x1008a59b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
1008a4974:     	ldr	x1, [x21, #0x58]
1008a4978:     	cmp	x1, x0
1008a497c:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a4980:     	ldr	x8, [x21, #0x50]
1008a4984:     	add	x8, x8, x0, lsl #5
1008a4988:     	add	x8, x8, #0x18
1008a498c:     	ldr	x8, [x8]
1008a4990:     	bics	x9, x8, x10
1008a4994:     	str	x9, [sp, #0xf0]
1008a4998:     	b.ne	0x1008a59b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
1008a499c:     	mov	x20, #0x0               ; =0
1008a49a0:     	bic	x8, x10, x8
1008a49a4:     	fmov	d0, x8
1008a49a8:     	cnt.8b	v0, v0
1008a49ac:     	addv.8b	b0, v0
1008a49b0:     	fmov	x8, d0
1008a49b4:     	ldr	x9, [x28, #0x148]
1008a49b8:     	add	x8, x9, x8
1008a49bc:     	str	x8, [x28, #0x148]
1008a49c0:     	ldr	w9, [sp, #0x70]
1008a49c4:     	add	x27, x27, #0x18
1008a49c8:     	stp	xzr, x20, [x26]
1008a49cc:     	stp	w24, w9, [x26, #0x10]
1008a49d0:     	cmp	x27, #0x30
1008a49d4:     	b.ne	0x1008a46d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
1008a49d8:     	ldr	w9, [x23, #0x10]
1008a49dc:     	cbz	w9, 0x1008a5448 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
1008a49e0:     	ldr	w10, [x23, #0x28]
1008a49e4:     	cbz	w10, 0x1008a5448 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
1008a49e8:     	cmp	w9, #0x1
1008a49ec:     	ccmp	w10, #0x1, #0x0, eq
1008a49f0:     	b.eq	0x1008a4b14 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x49c>
1008a49f4:     	ldr	x8, [x28, #0x88]
1008a49f8:     	cbz	x8, 0x1008a4b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
1008a49fc:     	mov	x8, #0x0                ; =0
1008a4a00:     	mov	x15, #0xa9c5            ; =43461
1008a4a04:     	movk	x15, #0x2e62, lsl #16
1008a4a08:     	movk	x15, #0x7aea, lsl #32
1008a4a0c:     	movk	x15, #0xf135, lsl #48
1008a4a10:     	ldp	x11, x12, [x23]
1008a4a14:     	madd	x13, x9, x15, x11
1008a4a18:     	mov	x14, #0x6332            ; =25394
1008a4a1c:     	movk	x14, #0x6ed3, lsl #16
1008a4a20:     	movk	x14, #0x765a, lsl #32
1008a4a24:     	movk	x14, #0x284f, lsl #48
1008a4a28:     	mul	x14, x14, x15
1008a4a2c:     	madd	x13, x13, x15, x14
1008a4a30:     	add	x13, x13, x12
1008a4a34:     	madd	x16, x13, x15, x10
1008a4a38:     	ldp	x13, x14, [x23, #0x18]
1008a4a3c:     	madd	x16, x16, x15, x13
1008a4a40:     	madd	x16, x16, x15, x14
1008a4a44:     	mul	x15, x16, x15
1008a4a48:     	ror	x0, x15, #0x2c
1008a4a4c:     	lsr	x17, x0, #57
1008a4a50:     	ldp	x16, x15, [x28, #0x70]
1008a4a54:     	dup.8b	v0, w17
1008a4a58:     	movi.2d	v1, #0xffffffffffffffff
1008a4a5c:     	mov	w17, #0x38              ; =56
1008a4a60:     	and	x0, x0, x15
1008a4a64:     	ldr	d2, [x16, x0]
1008a4a68:     	cmeq.8b	v3, v2, v0
1008a4a6c:     	fmov	x1, d3
1008a4a70:     	ands	x1, x1, #0x8080808080808080
1008a4a74:     	b.eq	0x1008a4ae4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
1008a4a78:     	rbit	x2, x1
1008a4a7c:     	clz	x2, x2
1008a4a80:     	add	x2, x0, x2, lsr #3
1008a4a84:     	and	x2, x2, x15
1008a4a88:     	mneg	x2, x2, x17
1008a4a8c:     	add	x2, x16, x2
1008a4a90:     	ldur	x3, [x2, #-0x38]
1008a4a94:     	cmp	x11, x3
1008a4a98:     	b.ne	0x1008a4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1008a4a9c:     	ldur	x3, [x2, #-0x30]
1008a4aa0:     	cmp	x12, x3
1008a4aa4:     	b.ne	0x1008a4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1008a4aa8:     	ldur	w3, [x2, #-0x28]
1008a4aac:     	cmp	w9, w3
1008a4ab0:     	b.ne	0x1008a4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1008a4ab4:     	ldur	x3, [x2, #-0x20]
1008a4ab8:     	cmp	x13, x3
1008a4abc:     	b.ne	0x1008a4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1008a4ac0:     	ldur	x3, [x2, #-0x18]
1008a4ac4:     	cmp	x14, x3
1008a4ac8:     	b.ne	0x1008a4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1008a4acc:     	ldur	w3, [x2, #-0x10]
1008a4ad0:     	cmp	w10, w3
1008a4ad4:     	b.eq	0x1008a4b94 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x51c>
1008a4ad8:     	sub	x2, x1, #0x2
1008a4adc:     	ands	x1, x2, x1
1008a4ae0:     	b.ne	0x1008a4a78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
1008a4ae4:     	cmeq.8b	v2, v2, v1
1008a4ae8:     	fmov	x1, d2
1008a4aec:     	cbnz	x1, 0x1008a4b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
1008a4af0:     	add	x8, x8, #0x8
1008a4af4:     	add	x0, x0, x8
1008a4af8:     	and	x0, x0, x15
1008a4afc:     	ldr	d2, [x16, x0]
1008a4b00:     	cmeq.8b	v3, v2, v0
1008a4b04:     	fmov	x1, d3
1008a4b08:     	ands	x1, x1, #0x8080808080808080
1008a4b0c:     	b.ne	0x1008a4a78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
1008a4b10:     	b	0x1008a4ae4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
1008a4b14:     	mov	w0, #0x1                ; =1
1008a4b18:     	b	0x1008a544c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1008a4b1c:     	mov	x24, x28
1008a4b20:     	ldr	x8, [x24, #0xc8]!
1008a4b24:     	add	x8, x8, #0x1
1008a4b28:     	str	x8, [x24]
1008a4b2c:     	mov	w11, #0x8481            ; =33921
1008a4b30:     	movk	w11, #0x1e, lsl #16
1008a4b34:     	cmp	x8, x11
1008a4b38:     	b.hs	0x1008a5a08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1390>
1008a4b3c:     	ldr	x12, [x23]
1008a4b40:     	ldr	x13, [x23, #0x18]
1008a4b44:     	ldr	x8, [x21, #0x40]
1008a4b48:     	cmn	x8, #0x1
1008a4b4c:     	b.eq	0x1008a4bc4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x54c>
1008a4b50:     	ldr	x1, [x21, #0x50]
1008a4b54:     	lsr	x0, x9, #1
1008a4b58:     	cmp	x1, x0
1008a4b5c:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a4b60:     	lsr	x8, x10, #1
1008a4b64:     	cmp	x1, x8
1008a4b68:     	b.ls	0x1008a5abc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1444>
1008a4b6c:     	ldr	x11, [x21, #0x48]
1008a4b70:     	lsl	x14, x0, #4
1008a4b74:     	ldr	x14, [x11, x14]
1008a4b78:     	bic	x19, x14, x12
1008a4b7c:     	add	x8, x11, x8, lsl #4
1008a4b80:     	ldr	x15, [x8]
1008a4b84:     	ldp	x11, x1, [x28, #0x18]
1008a4b88:     	mov	x22, #0x0               ; =0
1008a4b8c:     	cbnz	x19, 0x1008a4c04 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x58c>
1008a4b90:     	b	0x1008a4c34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
1008a4b94:     	ldur	w0, [x2, #-0x8]
1008a4b98:     	ldr	x8, [x28, #0xd0]
1008a4b9c:     	add	x8, x8, #0x1
1008a4ba0:     	str	x8, [x28, #0xd0]
1008a4ba4:     	b	0x1008a544c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1008a4ba8:     	adrp	x0, 0x1015d1000 <dyld_stub_binder+0x1015d1000>
1008a4bac:     	add	x0, x0, #0x1d7
1008a4bb0:     	adrp	x2, 0x101795000 <dyld_stub_binder+0x101795000>
1008a4bb4:     	add	x2, x2, #0xda8
1008a4bb8:     	mov	w1, #0x2c               ; =44
1008a4bbc:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1008a4bc0:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a4bc4:     	ldr	x1, [x21, #0x58]
1008a4bc8:     	lsr	x0, x9, #1
1008a4bcc:     	cmp	x1, x0
1008a4bd0:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a4bd4:     	lsr	x8, x10, #1
1008a4bd8:     	cmp	x1, x8
1008a4bdc:     	b.ls	0x1008a5af0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1478>
1008a4be0:     	ldr	x11, [x21, #0x50]
1008a4be4:     	add	x14, x11, x0, lsl #5
1008a4be8:     	ldr	x14, [x14, #0x18]
1008a4bec:     	bic	x19, x14, x12
1008a4bf0:     	add	x8, x11, x8, lsl #5
1008a4bf4:     	ldr	x15, [x8, #0x18]!
1008a4bf8:     	ldp	x11, x1, [x28, #0x18]
1008a4bfc:     	mov	x22, #0x0               ; =0
1008a4c00:     	cbz	x19, 0x1008a4c34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
1008a4c04:     	mov	w8, #0x1                ; =1
1008a4c08:     	mov	x14, x19
1008a4c0c:     	rbit	x16, x14
1008a4c10:     	clz	x0, x16
1008a4c14:     	cmp	x0, x1
1008a4c18:     	b.hs	0x1008a5a50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13d8>
1008a4c1c:     	ldr	w16, [x11, x0, lsl #2]
1008a4c20:     	lsl	x16, x8, x16
1008a4c24:     	orr	x22, x16, x22
1008a4c28:     	sub	x16, x14, #0x1
1008a4c2c:     	ands	x14, x16, x14
1008a4c30:     	b.ne	0x1008a4c0c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x594>
1008a4c34:     	ldp	x14, x8, [x28, #0x48]
1008a4c38:     	bic	x15, x15, x13
1008a4c3c:     	cbz	x15, 0x1008a4c74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5fc>
1008a4c40:     	mov	x16, #0x0               ; =0
1008a4c44:     	mov	w17, #0x1               ; =1
1008a4c48:     	rbit	x0, x15
1008a4c4c:     	clz	x0, x0
1008a4c50:     	cmp	x0, x8
1008a4c54:     	b.hs	0x1008a5a5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13e4>
1008a4c58:     	ldr	w0, [x14, x0, lsl #2]
1008a4c5c:     	lsl	x0, x17, x0
1008a4c60:     	orr	x16, x0, x16
1008a4c64:     	sub	x0, x15, #0x1
1008a4c68:     	ands	x15, x0, x15
1008a4c6c:     	b.ne	0x1008a4c48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5d0>
1008a4c70:     	orr	x22, x16, x22
1008a4c74:     	eor	w9, w10, w9
1008a4c78:     	cmp	x12, x13
1008a4c7c:     	ccmp	w9, #0x1, #0x0, eq
1008a4c80:     	b.ne	0x1008a4d5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1008a4c84:     	ldr	x9, [x23, #0x8]
1008a4c88:     	ldr	x10, [x23, #0x20]
1008a4c8c:     	cmp	x9, x10
1008a4c90:     	b.ne	0x1008a4d5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1008a4c94:     	mov	w16, #0x4               ; =4
1008a4c98:     	stp	xzr, x16, [sp, #0xf0]
1008a4c9c:     	str	xzr, [sp, #0x100]
1008a4ca0:     	mov	x20, #0x0               ; =0
1008a4ca4:     	cbz	x19, 0x1008a4d04 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x68c>
1008a4ca8:     	mov	w8, #0x4                ; =4
1008a4cac:     	b	0x1008a4cd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x65c>
1008a4cb0:     	ldr	x8, [sp, #0xf8]
1008a4cb4:     	rbit	x9, x19
1008a4cb8:     	clz	x9, x9
1008a4cbc:     	str	w9, [x8, x20, lsl #2]
1008a4cc0:     	add	x20, x20, #0x1
1008a4cc4:     	str	x20, [sp, #0x100]
1008a4cc8:     	sub	x9, x19, #0x1
1008a4ccc:     	ands	x19, x9, x19
1008a4cd0:     	b.eq	0x1008a4cec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x674>
1008a4cd4:     	ldr	x9, [sp, #0xf0]
1008a4cd8:     	cmp	x20, x9
1008a4cdc:     	b.ne	0x1008a4cb4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x63c>
1008a4ce0:     	add	x0, sp, #0xf0
1008a4ce4:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a4ce8:     	b	0x1008a4cb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x638>
1008a4cec:     	ldp	x9, x16, [sp, #0xf0]
1008a4cf0:     	ldp	x14, x8, [x28, #0x48]
1008a4cf4:     	ldp	x11, x1, [x28, #0x18]
1008a4cf8:     	cmp	x9, #0x0
1008a4cfc:     	cset	w19, eq
1008a4d00:     	b	0x1008a4d08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x690>
1008a4d04:     	mov	w19, #0x1               ; =1
1008a4d08:     	mov	x9, #0x0                ; =0
1008a4d0c:     	lsl	x10, x20, #2
1008a4d10:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a4d14:     	add	x2, x2, #0xbe8
1008a4d18:     	adrp	x12, 0x101759000 <dyld_stub_binder+0x101759000>
1008a4d1c:     	add	x12, x12, #0xc00
1008a4d20:     	cmp	x10, x9
1008a4d24:     	b.eq	0x1008a5444 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdcc>
1008a4d28:     	ldr	w0, [x16, x9]
1008a4d2c:     	cmp	x1, x0
1008a4d30:     	b.ls	0x1008a5a6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13f4>
1008a4d34:     	cmp	x8, x0
1008a4d38:     	b.ls	0x1008a5a74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13fc>
1008a4d3c:     	ldr	w13, [x11, x0, lsl #2]
1008a4d40:     	ldr	w15, [x14, x0, lsl #2]
1008a4d44:     	add	x9, x9, #0x4
1008a4d48:     	cmp	w13, w15
1008a4d4c:     	b.eq	0x1008a4d20 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6a8>
1008a4d50:     	tbnz	w19, #0x0, 0x1008a4d5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1008a4d54:     	mov	x0, x16
1008a4d58:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a4d5c:     	fmov	d0, x22
1008a4d60:     	cnt.8b	v0, v0
1008a4d64:     	addv.8b	b0, v0
1008a4d68:     	fmov	x19, d0
1008a4d6c:     	cmp	x19, #0xa
1008a4d70:     	b.hs	0x1008a5120 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xaa8>
1008a4d74:     	add	x8, x28, #0x10
1008a4d78:     	str	x8, [sp, #0x50]
1008a4d7c:     	ldr	x8, [x28, #0xd8]
1008a4d80:     	add	x8, x8, #0x1
1008a4d84:     	str	x8, [x28, #0xd8]
1008a4d88:     	ldr	w8, [x28]
1008a4d8c:     	tbz	w8, #0x0, 0x1008a5180 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb08>
1008a4d90:     	str	x19, [sp, #0x8]
1008a4d94:     	mov	x26, #0x0               ; =0
1008a4d98:     	ldr	x10, [x28, #0x8]
1008a4d9c:     	add	x8, x28, #0x90
1008a4da0:     	str	x8, [sp, #0x48]
1008a4da4:     	lsl	x9, x10, #6
1008a4da8:     	tst	x10, #0xfc00000000000000
1008a4dac:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1008a4db0:     	ccmp	x9, x8, #0x2, eq
1008a4db4:     	cset	w8, hi
1008a4db8:     	str	w8, [sp, #0x14]
1008a4dbc:     	stp	x10, x24, [sp, #0x28]
1008a4dc0:     	sub	x8, x10, #0x1
1008a4dc4:     	stp	x9, x8, [sp, #0x18]
1008a4dc8:     	mov	w20, #0xff              ; =255
1008a4dcc:     	mov	w8, #0x1                ; =1
1008a4dd0:     	b	0x1008a4e38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7c0>
1008a4dd4:     	strb	w23, [x28]
1008a4dd8:     	strb	w10, [x28, #0x1]
1008a4ddc:     	str	w25, [x28, #0x4]
1008a4de0:     	stp	x9, x27, [x28, #0x8]
1008a4de4:     	ldp	x8, x9, [sp, #0x70]
1008a4de8:     	stp	x19, x9, [x28, #0x18]
1008a4dec:     	str	x8, [x28, #0x28]
1008a4df0:     	ldr	w8, [sp, #0x58]
1008a4df4:     	stp	w25, w8, [x28, #0x30]
1008a4df8:     	str	x22, [x28, #0x38]
1008a4dfc:     	ldr	x28, [sp, #0x88]
1008a4e00:     	ldr	x23, [sp, #0x60]
1008a4e04:     	ldr	w13, [sp, #0x80]
1008a4e08:     	mov	w8, #0x0                ; =0
1008a4e0c:     	ldr	x9, [x28, #0x130]
1008a4e10:     	ldp	x2, x10, [x28, #0x98]
1008a4e14:     	add	x10, x10, x2, lsl #6
1008a4e18:     	ldp	x11, x12, [x28, #0xb0]
1008a4e1c:     	add	x10, x12, x10
1008a4e20:     	add	x10, x10, x11, lsl #6
1008a4e24:     	cmp	x10, x9
1008a4e28:     	csel	x9, x10, x9, hi
1008a4e2c:     	str	x9, [x28, #0x130]
1008a4e30:     	mov	w26, #0x1               ; =1
1008a4e34:     	tbz	w13, #0x0, 0x1008a5614 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf9c>
1008a4e38:     	mov	x13, x8
1008a4e3c:     	add	x8, x26, x26, lsl #1
1008a4e40:     	lsl	x8, x8, #3
1008a4e44:     	add	x9, x23, x8
1008a4e48:     	ldr	q0, [x9]
1008a4e4c:     	str	q0, [sp, #0xc0]
1008a4e50:     	ldr	x25, [x9, #0x10]
1008a4e54:     	str	x25, [sp, #0xd0]
1008a4e58:     	cmp	w25, #0x2
1008a4e5c:     	b.lo	0x1008a4e08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x790>
1008a4e60:     	str	w13, [sp, #0x80]
1008a4e64:     	ldr	x9, [sp, #0x48]
1008a4e68:     	add	x24, x9, x8
1008a4e6c:     	ldrb	w27, [x28, #0x150]
1008a4e70:     	ldr	x8, [x24, #0x8]
1008a4e74:     	cbz	x8, 0x1008a4e80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x808>
1008a4e78:     	ldr	x0, [x24]
1008a4e7c:     	b	0x1008a4f04 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x88c>
1008a4e80:     	ldp	x9, x28, [sp, #0x20]
1008a4e84:     	eor	x8, x28, x9
1008a4e88:     	cmp	x8, x9
1008a4e8c:     	b.ls	0x1008a5a38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13c0>
1008a4e90:     	ldr	x19, [sp, #0x18]
1008a4e94:     	ldr	w8, [sp, #0x14]
1008a4e98:     	cbnz	w8, 0x1008a5278 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc00>
1008a4e9c:     	cbz	x19, 0x1008a4eb4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x83c>
1008a4ea0:     	mov	x0, x19
1008a4ea4:     	mov	w1, #0x8                ; =8
1008a4ea8:     	bl	0x1013fa450 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1008a4eac:     	cbnz	x0, 0x1008a4eb8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x840>
1008a4eb0:     	b	0x1008a5b10 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1498>
1008a4eb4:     	mov	w0, #0x8                ; =8
1008a4eb8:     	mov	x8, x0
1008a4ebc:     	mov	x9, x28
1008a4ec0:     	cmp	x28, #0x4
1008a4ec4:     	b.hs	0x1008a4ed8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x860>
1008a4ec8:     	strb	w20, [x8], #0x40
1008a4ecc:     	subs	x9, x9, #0x1
1008a4ed0:     	b.ne	0x1008a4ec8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x850>
1008a4ed4:     	b	0x1008a4efc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x884>
1008a4ed8:     	add	x8, x0, #0x80
1008a4edc:     	and	x9, x28, #0x3fffffffffffffc
1008a4ee0:     	sturb	w20, [x8, #-0x80]
1008a4ee4:     	sturb	w20, [x8, #-0x40]
1008a4ee8:     	strb	w20, [x8]
1008a4eec:     	strb	w20, [x8, #0x40]
1008a4ef0:     	add	x8, x8, #0x100
1008a4ef4:     	subs	x9, x9, #0x4
1008a4ef8:     	b.ne	0x1008a4ee0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x868>
1008a4efc:     	stp	x0, x28, [x24]
1008a4f00:     	mov	x8, x28
1008a4f04:     	mov	w9, w25
1008a4f08:     	ldp	x11, x12, [sp, #0xc0]
1008a4f0c:     	ldr	w19, [sp, #0xd4]
1008a4f10:     	mov	x10, #0xa9c5            ; =43461
1008a4f14:     	movk	x10, #0x2e62, lsl #16
1008a4f18:     	movk	x10, #0x7aea, lsl #32
1008a4f1c:     	movk	x10, #0xf135, lsl #48
1008a4f20:     	stp	x12, x11, [sp, #0x70]
1008a4f24:     	madd	x9, x9, x10, x11
1008a4f28:     	madd	x9, x9, x10, x12
1008a4f2c:     	madd	x9, x9, x10, x22
1008a4f30:     	mul	x9, x9, x10
1008a4f34:     	sub	x8, x8, #0x1
1008a4f38:     	and	x8, x8, x9, ror #44
1008a4f3c:     	add	x28, x0, x8, lsl #6
1008a4f40:     	ldrb	w8, [x28]
1008a4f44:     	cmp	w8, #0xff
1008a4f48:     	b.ne	0x1008a4f6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x8f4>
1008a4f4c:     	ldr	x9, [sp, #0x88]
1008a4f50:     	ldr	x8, [x9, #0x120]
1008a4f54:     	add	x8, x8, #0x1
1008a4f58:     	str	x8, [x9, #0x120]
1008a4f5c:     	add	x8, x28, #0x10
1008a4f60:     	str	x8, [sp, #0x38]
1008a4f64:     	add	x23, x28, #0x18
1008a4f68:     	b	0x1008a5010 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
1008a4f6c:     	ldr	x9, [x28, #0x38]
1008a4f70:     	cmp	x9, x22
1008a4f74:     	b.ne	0x1008a4fb8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1008a4f78:     	ldr	x9, [x28, #0x20]
1008a4f7c:     	ldr	x10, [sp, #0x78]
1008a4f80:     	cmp	x9, x10
1008a4f84:     	b.ne	0x1008a4fb8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1008a4f88:     	ldr	x9, [x28, #0x28]
1008a4f8c:     	ldr	x10, [sp, #0x70]
1008a4f90:     	cmp	x9, x10
1008a4f94:     	b.ne	0x1008a4fb8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1008a4f98:     	ldr	w9, [x28, #0x30]
1008a4f9c:     	cmp	w9, w25
1008a4fa0:     	b.ne	0x1008a4fb8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1008a4fa4:     	ldr	x28, [sp, #0x88]
1008a4fa8:     	ldr	x8, [x28, #0x118]
1008a4fac:     	add	x8, x8, #0x1
1008a4fb0:     	str	x8, [x28, #0x118]
1008a4fb4:     	b	0x1008a4e04 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x78c>
1008a4fb8:     	mov	x23, x28
1008a4fbc:     	ldr	x9, [x23, #0x18]!
1008a4fc0:     	lsl	x10, x9, #3
1008a4fc4:     	cmp	w8, #0x2
1008a4fc8:     	csel	x10, x10, xzr, eq
1008a4fcc:     	ldr	x11, [x24, #0x10]
1008a4fd0:     	sub	x10, x11, x10
1008a4fd4:     	cmp	w8, #0x2
1008a4fd8:     	mov	x8, x28
1008a4fdc:     	ldr	x0, [x8, #0x10]!
1008a4fe0:     	str	x8, [sp, #0x38]
1008a4fe4:     	strb	w20, [x28]
1008a4fe8:     	ldr	x8, [sp, #0x88]
1008a4fec:     	ldr	q0, [x8, #0x120]
1008a4ff0:     	mov	w11, #0x1               ; =1
1008a4ff4:     	dup.2d	v1, x11
1008a4ff8:     	add.2d	v0, v0, v1
1008a4ffc:     	str	q0, [x8, #0x120]
1008a5000:     	str	x10, [x24, #0x10]
1008a5004:     	ccmp	x9, #0x0, #0x4, hs
1008a5008:     	b.eq	0x1008a5010 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
1008a500c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5010:     	ldr	x8, [sp, #0x50]
1008a5014:     	mov	w9, #0x30               ; =48
1008a5018:     	madd	x3, x26, x9, x8
1008a501c:     	add	x0, sp, #0xf0
1008a5020:     	add	x2, sp, #0xc0
1008a5024:     	mov	x1, x21
1008a5028:     	mov	x4, x22
1008a502c:     	mov	x5, x27
1008a5030:     	ldr	x6, [sp, #0x30]
1008a5034:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a5038:     	ldr	x8, [sp, #0xf0]
1008a503c:     	cmn	x8, #0x1
1008a5040:     	str	w19, [sp, #0x58]
1008a5044:     	str	x23, [sp, #0x40]
1008a5048:     	b.eq	0x1008a5060 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9e8>
1008a504c:     	cmn	x8, #0x2
1008a5050:     	b.ne	0x1008a5084 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa0c>
1008a5054:     	mov	w23, #0x0               ; =0
1008a5058:     	ldrb	w10, [sp, #0xf8]
1008a505c:     	b	0x1008a5064 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9ec>
1008a5060:     	mov	w23, #0x1               ; =1
1008a5064:     	mov	x26, #0x0               ; =0
1008a5068:     	ldr	x8, [x24, #0x10]
1008a506c:     	add	x8, x8, x26
1008a5070:     	str	x8, [x24, #0x10]
1008a5074:     	ldrb	w8, [x28]
1008a5078:     	cmp	w8, #0x2
1008a507c:     	b.ne	0x1008a4dd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1008a5080:     	b	0x1008a50f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa7c>
1008a5084:     	ldp	x27, x19, [sp, #0xf8]
1008a5088:     	ldr	x9, [sp, #0x108]
1008a508c:     	lsl	x26, x19, #3
1008a5090:     	cmp	x8, x19
1008a5094:     	b.ls	0x1008a50d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
1008a5098:     	mov	x23, x9
1008a509c:     	str	x27, [sp, #0x68]
1008a50a0:     	cbz	x19, 0x1008a50c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa50>
1008a50a4:     	lsl	x1, x8, #3
1008a50a8:     	ldr	x0, [sp, #0x68]
1008a50ac:     	mov	w2, #0x8                ; =8
1008a50b0:     	mov	x3, x26
1008a50b4:     	bl	0x1013fa4a4 <__RNvCsiwXPDrQxTLA_7___rustc14___rust_realloc>
1008a50b8:     	mov	x27, x0
1008a50bc:     	mov	x9, x23
1008a50c0:     	cbnz	x0, 0x1008a50d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
1008a50c4:     	b	0x1008a5b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14a4>
1008a50c8:     	ldr	x0, [sp, #0x68]
1008a50cc:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a50d0:     	mov	w27, #0x8               ; =8
1008a50d4:     	mov	x9, x23
1008a50d8:     	mov	w23, #0x2               ; =2
1008a50dc:     	ldr	x8, [x24, #0x10]
1008a50e0:     	add	x8, x8, x26
1008a50e4:     	str	x8, [x24, #0x10]
1008a50e8:     	ldrb	w8, [x28]
1008a50ec:     	cmp	w8, #0x2
1008a50f0:     	b.ne	0x1008a4dd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1008a50f4:     	ldr	x8, [sp, #0x40]
1008a50f8:     	ldr	x8, [x8]
1008a50fc:     	cbz	x8, 0x1008a4dd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1008a5100:     	ldr	x8, [sp, #0x38]
1008a5104:     	ldr	x0, [x8]
1008a5108:     	mov	x24, x9
1008a510c:     	mov	x26, x10
1008a5110:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5114:     	mov	x10, x26
1008a5118:     	mov	x9, x24
1008a511c:     	b	0x1008a4dd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1008a5120:     	mov	x0, x21
1008a5124:     	mov	x1, x22
1008a5128:     	bl	0x100dea100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
1008a512c:     	ldr	q0, [x23]
1008a5130:     	str	q0, [sp, #0xc0]
1008a5134:     	ldr	x8, [x23, #0x10]
1008a5138:     	str	x8, [sp, #0xd0]
1008a513c:     	mov	w22, w0
1008a5140:     	ldr	x1, [x28, #0x38]
1008a5144:     	cmp	x1, x22
1008a5148:     	b.ls	0x1008a5aac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
1008a514c:     	ldr	x8, [x28, #0x30]
1008a5150:     	ldr	w8, [x8, x22, lsl #2]
1008a5154:     	ldr	w9, [sp, #0xd0]
1008a5158:     	ldr	x10, [x21, #0x40]
1008a515c:     	lsr	x0, x9, #1
1008a5160:     	cmn	x10, #0x1
1008a5164:     	b.eq	0x1008a527c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc04>
1008a5168:     	ldr	x1, [x21, #0x50]
1008a516c:     	cmp	x1, x0
1008a5170:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a5174:     	ldr	x9, [x21, #0x48]
1008a5178:     	add	x9, x9, x0, lsl #4
1008a517c:     	b	0x1008a5294 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc1c>
1008a5180:     	ldrb	w5, [x28, #0x150]
1008a5184:     	add	x0, sp, #0xc0
1008a5188:     	add	x3, x28, #0x10
1008a518c:     	mov	x1, x21
1008a5190:     	mov	x2, x23
1008a5194:     	mov	x4, x22
1008a5198:     	mov	x6, x24
1008a519c:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a51a0:     	ldrb	w5, [x28, #0x150]
1008a51a4:     	add	x0, sp, #0xf0
1008a51a8:     	add	x2, x23, #0x18
1008a51ac:     	add	x3, x28, #0x40
1008a51b0:     	mov	x1, x21
1008a51b4:     	mov	x4, x22
1008a51b8:     	mov	x6, x24
1008a51bc:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a51c0:     	mov	w8, #0x1                ; =1
1008a51c4:     	lsl	x8, x8, x19
1008a51c8:     	lsr	x8, x8, #6
1008a51cc:     	cmp	x19, #0x6
1008a51d0:     	cinc	x20, x8, lo
1008a51d4:     	cbz	x20, 0x1008a56fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
1008a51d8:     	lsl	x24, x20, #3
1008a51dc:     	mov	x0, x24
1008a51e0:     	mov	w1, #0x8                ; =8
1008a51e4:     	bl	0x1013fa450 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1008a51e8:     	cbz	x0, 0x1008a5b00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1488>
1008a51ec:     	mov	x25, x0
1008a51f0:     	mov	x0, #0x0                ; =0
1008a51f4:     	ldp	x8, x9, [sp, #0xc0]
1008a51f8:     	ldp	x1, x10, [sp, #0xd0]
1008a51fc:     	sub	x11, x0, w9, uxtb
1008a5200:     	ldp	x23, x13, [sp, #0xf0]
1008a5204:     	ldp	x12, x14, [sp, #0x100]
1008a5208:     	mov	x24, x25
1008a520c:     	b	0x1008a5228 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
1008a5210:     	tst	w13, #0x1
1008a5214:     	csel	x15, x15, xzr, ne
1008a5218:     	str	x15, [x24, x0, lsl #3]
1008a521c:     	add	x0, x0, #0x1
1008a5220:     	cmp	x20, x0
1008a5224:     	b.eq	0x1008a5270 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1008a5228:     	mov	x15, x11
1008a522c:     	cmn	x8, #0x2
1008a5230:     	b.eq	0x1008a5244 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbcc>
1008a5234:     	cmp	x0, x1
1008a5238:     	b.hs	0x1008a5a8c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1414>
1008a523c:     	ldr	x15, [x9, x0, lsl #3]
1008a5240:     	eor	x15, x10, x15
1008a5244:     	cmn	x23, #0x2
1008a5248:     	b.eq	0x1008a5210 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb98>
1008a524c:     	cmp	x0, x12
1008a5250:     	b.hs	0x1008a5a88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1410>
1008a5254:     	ldr	x16, [x13, x0, lsl #3]
1008a5258:     	eor	x16, x14, x16
1008a525c:     	and	x15, x16, x15
1008a5260:     	str	x15, [x24, x0, lsl #3]
1008a5264:     	add	x0, x0, #0x1
1008a5268:     	cmp	x20, x0
1008a526c:     	b.ne	0x1008a5228 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
1008a5270:     	mov	x27, x20
1008a5274:     	b	0x1008a5708 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1090>
1008a5278:     	bl	0x101506610 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1008a527c:     	ldr	x1, [x21, #0x58]
1008a5280:     	cmp	x1, x0
1008a5284:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a5288:     	ldr	x9, [x21, #0x50]
1008a528c:     	add	x9, x9, x0, lsl #5
1008a5290:     	add	x9, x9, #0x18
1008a5294:     	ldr	x9, [x9]
1008a5298:     	mov	w10, #0x1               ; =1
1008a529c:     	lsl	x8, x10, x8
1008a52a0:     	tst	x9, x8
1008a52a4:     	b.eq	0x1008a52b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc40>
1008a52a8:     	ldp	x9, x10, [sp, #0xc0]
1008a52ac:     	orr	x9, x9, x8
1008a52b0:     	bic	x8, x10, x8
1008a52b4:     	stp	x9, x8, [sp, #0xc0]
1008a52b8:     	sub	x0, x29, #0x70
1008a52bc:     	add	x1, sp, #0xc0
1008a52c0:     	mov	x2, x21
1008a52c4:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a52c8:     	ldur	q0, [x29, #-0x70]
1008a52cc:     	stur	q0, [x29, #-0x90]
1008a52d0:     	ldur	x8, [x29, #-0x60]
1008a52d4:     	stur	q0, [x29, #-0xb0]
1008a52d8:     	str	q0, [sp, #0x90]
1008a52dc:     	str	x8, [sp, #0xa0]
1008a52e0:     	ldr	q0, [sp, #0x90]
1008a52e4:     	str	x8, [sp, #0x100]
1008a52e8:     	str	q0, [sp, #0xf0]
1008a52ec:     	ldur	q0, [x23, #0x18]
1008a52f0:     	str	q0, [sp, #0xc0]
1008a52f4:     	ldur	x8, [x23, #0x28]
1008a52f8:     	str	x8, [sp, #0xd0]
1008a52fc:     	ldr	x1, [x28, #0x68]
1008a5300:     	cmp	x1, x22
1008a5304:     	b.ls	0x1008a5aac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
1008a5308:     	ldr	x8, [x28, #0x60]
1008a530c:     	ldr	w8, [x8, x22, lsl #2]
1008a5310:     	ldr	w9, [sp, #0xd0]
1008a5314:     	ldr	x10, [x21, #0x40]
1008a5318:     	lsr	x0, x9, #1
1008a531c:     	cmn	x10, #0x1
1008a5320:     	b.eq	0x1008a533c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcc4>
1008a5324:     	ldr	x1, [x21, #0x50]
1008a5328:     	cmp	x1, x0
1008a532c:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a5330:     	ldr	x9, [x21, #0x48]
1008a5334:     	add	x9, x9, x0, lsl #4
1008a5338:     	b	0x1008a5354 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcdc>
1008a533c:     	ldr	x1, [x21, #0x58]
1008a5340:     	cmp	x1, x0
1008a5344:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a5348:     	ldr	x9, [x21, #0x50]
1008a534c:     	add	x9, x9, x0, lsl #5
1008a5350:     	add	x9, x9, #0x18
1008a5354:     	ldr	x9, [x9]
1008a5358:     	mov	w10, #0x1               ; =1
1008a535c:     	lsl	x8, x10, x8
1008a5360:     	tst	x9, x8
1008a5364:     	b.eq	0x1008a5378 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd00>
1008a5368:     	ldp	x9, x10, [sp, #0xc0]
1008a536c:     	orr	x9, x9, x8
1008a5370:     	bic	x8, x10, x8
1008a5374:     	stp	x9, x8, [sp, #0xc0]
1008a5378:     	sub	x0, x29, #0x70
1008a537c:     	add	x1, sp, #0xc0
1008a5380:     	mov	x2, x21
1008a5384:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a5388:     	ldur	q0, [x29, #-0x70]
1008a538c:     	stur	q0, [x29, #-0x90]
1008a5390:     	ldur	x8, [x29, #-0x60]
1008a5394:     	stur	q0, [x29, #-0xb0]
1008a5398:     	str	q0, [sp, #0x90]
1008a539c:     	str	x8, [sp, #0xa0]
1008a53a0:     	ldr	q0, [sp, #0x90]
1008a53a4:     	str	x8, [sp, #0x118]
1008a53a8:     	add	x8, sp, #0x9
1008a53ac:     	stur	q0, [x8, #0xff]
1008a53b0:     	ldp	q0, q1, [sp, #0xf0]
1008a53b4:     	ldr	q2, [sp, #0x110]
1008a53b8:     	stp	q1, q2, [sp, #0xa0]
1008a53bc:     	str	q0, [sp, #0x90]
1008a53c0:     	add	x2, sp, #0x90
1008a53c4:     	mov	x0, x28
1008a53c8:     	mov	x1, x21
1008a53cc:     	bl	0x1008a4678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1008a53d0:     	mov	x23, x0
1008a53d4:     	cmp	w0, #0x1
1008a53d8:     	b.ne	0x1008a53f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
1008a53dc:     	ldr	x8, [x28, #0xc0]
1008a53e0:     	lsr	x8, x8, x22
1008a53e4:     	tbz	w8, #0x0, 0x1008a53f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
1008a53e8:     	mov	w8, #0x1                ; =1
1008a53ec:     	b	0x1008a55fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf84>
1008a53f0:     	ldr	x8, [sp, #0x60]
1008a53f4:     	ldr	q0, [x8]
1008a53f8:     	stur	q0, [x29, #-0x70]
1008a53fc:     	ldr	x8, [x8, #0x10]
1008a5400:     	stur	x8, [x29, #-0x60]
1008a5404:     	ldr	x1, [x28, #0x38]
1008a5408:     	cmp	x1, x22
1008a540c:     	b.ls	0x1008a5ae0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
1008a5410:     	ldr	x8, [x28, #0x30]
1008a5414:     	ldr	w8, [x8, x22, lsl #2]
1008a5418:     	ldur	w9, [x29, #-0x60]
1008a541c:     	ldr	x10, [x21, #0x40]
1008a5420:     	lsr	x0, x9, #1
1008a5424:     	cmn	x10, #0x1
1008a5428:     	b.eq	0x1008a546c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdf4>
1008a542c:     	ldr	x1, [x21, #0x50]
1008a5430:     	cmp	x1, x0
1008a5434:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a5438:     	ldr	x9, [x21, #0x48]
1008a543c:     	add	x9, x9, x0, lsl #4
1008a5440:     	b	0x1008a5484 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe0c>
1008a5444:     	tbz	w19, #0x0, 0x1008a5604 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf8c>
1008a5448:     	mov	w0, #0x0                ; =0
1008a544c:     	add	sp, sp, #0x1c0
1008a5450:     	ldp	x29, x30, [sp, #0x50]
1008a5454:     	ldp	x20, x19, [sp, #0x40]
1008a5458:     	ldp	x22, x21, [sp, #0x30]
1008a545c:     	ldp	x24, x23, [sp, #0x20]
1008a5460:     	ldp	x26, x25, [sp, #0x10]
1008a5464:     	ldp	x28, x27, [sp], #0x60
1008a5468:     	ret
1008a546c:     	ldr	x1, [x21, #0x58]
1008a5470:     	cmp	x1, x0
1008a5474:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a5478:     	ldr	x9, [x21, #0x50]
1008a547c:     	add	x9, x9, x0, lsl #5
1008a5480:     	add	x9, x9, #0x18
1008a5484:     	ldr	x9, [x9]
1008a5488:     	mov	w10, #0x1               ; =1
1008a548c:     	lsl	x8, x10, x8
1008a5490:     	tst	x9, x8
1008a5494:     	b.eq	0x1008a54a8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe30>
1008a5498:     	ldur	q0, [x29, #-0x70]
1008a549c:     	dup.2d	v1, x8
1008a54a0:     	orr.16b	v0, v0, v1
1008a54a4:     	stur	q0, [x29, #-0x70]
1008a54a8:     	sub	x0, x29, #0xb0
1008a54ac:     	sub	x1, x29, #0x70
1008a54b0:     	mov	x2, x21
1008a54b4:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a54b8:     	ldur	q0, [x29, #-0xb0]
1008a54bc:     	stur	q0, [x29, #-0xd0]
1008a54c0:     	ldur	x8, [x29, #-0xa0]
1008a54c4:     	stur	q0, [x29, #-0xf0]
1008a54c8:     	stur	q0, [x29, #-0x90]
1008a54cc:     	stur	x8, [x29, #-0x80]
1008a54d0:     	ldur	q0, [x29, #-0x90]
1008a54d4:     	str	x8, [sp, #0x100]
1008a54d8:     	str	q0, [sp, #0xf0]
1008a54dc:     	ldr	x8, [sp, #0x60]
1008a54e0:     	ldur	q0, [x8, #0x18]
1008a54e4:     	stur	q0, [x29, #-0x70]
1008a54e8:     	ldur	x8, [x8, #0x28]
1008a54ec:     	stur	x8, [x29, #-0x60]
1008a54f0:     	ldr	x1, [x28, #0x68]
1008a54f4:     	cmp	x1, x22
1008a54f8:     	b.ls	0x1008a5ae0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
1008a54fc:     	ldr	x8, [x28, #0x60]
1008a5500:     	ldr	w8, [x8, x22, lsl #2]
1008a5504:     	ldur	w9, [x29, #-0x60]
1008a5508:     	ldr	x10, [x21, #0x40]
1008a550c:     	lsr	x0, x9, #1
1008a5510:     	cmn	x10, #0x1
1008a5514:     	b.eq	0x1008a5530 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xeb8>
1008a5518:     	ldr	x1, [x21, #0x50]
1008a551c:     	cmp	x1, x0
1008a5520:     	b.ls	0x1008a5ac0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1008a5524:     	ldr	x9, [x21, #0x48]
1008a5528:     	add	x9, x9, x0, lsl #4
1008a552c:     	b	0x1008a5548 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xed0>
1008a5530:     	ldr	x1, [x21, #0x58]
1008a5534:     	cmp	x1, x0
1008a5538:     	b.ls	0x1008a5af4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1008a553c:     	ldr	x9, [x21, #0x50]
1008a5540:     	add	x9, x9, x0, lsl #5
1008a5544:     	add	x9, x9, #0x18
1008a5548:     	and	w19, w22, #0x3f
1008a554c:     	ldr	x9, [x9]
1008a5550:     	mov	w10, #0x1               ; =1
1008a5554:     	lsl	x8, x10, x8
1008a5558:     	tst	x9, x8
1008a555c:     	b.eq	0x1008a5570 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xef8>
1008a5560:     	ldur	q0, [x29, #-0x70]
1008a5564:     	dup.2d	v1, x8
1008a5568:     	orr.16b	v0, v0, v1
1008a556c:     	stur	q0, [x29, #-0x70]
1008a5570:     	sub	x0, x29, #0xb0
1008a5574:     	sub	x1, x29, #0x70
1008a5578:     	mov	x2, x21
1008a557c:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a5580:     	ldur	q0, [x29, #-0xb0]
1008a5584:     	stur	q0, [x29, #-0xd0]
1008a5588:     	ldur	x8, [x29, #-0xa0]
1008a558c:     	stur	q0, [x29, #-0xf0]
1008a5590:     	stur	q0, [x29, #-0x90]
1008a5594:     	stur	x8, [x29, #-0x80]
1008a5598:     	ldur	q0, [x29, #-0x90]
1008a559c:     	str	x8, [sp, #0x118]
1008a55a0:     	add	x8, sp, #0x9
1008a55a4:     	stur	q0, [x8, #0xff]
1008a55a8:     	ldp	q0, q1, [sp, #0xf0]
1008a55ac:     	ldr	q2, [sp, #0x110]
1008a55b0:     	stp	q1, q2, [sp, #0xd0]
1008a55b4:     	str	q0, [sp, #0xc0]
1008a55b8:     	add	x2, sp, #0xc0
1008a55bc:     	mov	x0, x28
1008a55c0:     	mov	x1, x21
1008a55c4:     	bl	0x1008a4678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1008a55c8:     	mov	x3, x0
1008a55cc:     	ldr	x8, [x28, #0xc0]
1008a55d0:     	mov	x0, x21
1008a55d4:     	lsr	x8, x8, x19
1008a55d8:     	tbz	w8, #0x0, 0x1008a55ec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf74>
1008a55dc:     	mov	w1, #0xe                ; =14
1008a55e0:     	mov	x2, x23
1008a55e4:     	bl	0x100df9700 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1008a55e8:     	b	0x1008a55f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf80>
1008a55ec:     	mov	x1, x22
1008a55f0:     	mov	x2, x23
1008a55f4:     	bl	0x100dfa14c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
1008a55f8:     	mov	x8, x0
1008a55fc:     	ldr	x19, [sp, #0x60]
1008a5600:     	b	0x1008a5994 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x131c>
1008a5604:     	mov	x0, x16
1008a5608:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a560c:     	mov	w0, #0x0                ; =0
1008a5610:     	b	0x1008a544c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1008a5614:     	ldr	x1, [x28, #0x90]
1008a5618:     	add	x0, sp, #0xc0
1008a561c:     	mov	x3, x21
1008a5620:     	mov	x4, x23
1008a5624:     	mov	x5, x22
1008a5628:     	bl	0x10089f2b8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1008a562c:     	ldp	x1, x2, [x28, #0xa8]
1008a5630:     	add	x0, sp, #0xf0
1008a5634:     	add	x4, x23, #0x18
1008a5638:     	mov	x3, x21
1008a563c:     	mov	x5, x22
1008a5640:     	bl	0x10089f2b8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1008a5644:     	mov	w8, #0x1                ; =1
1008a5648:     	ldr	x9, [sp, #0x8]
1008a564c:     	lsl	x8, x8, x9
1008a5650:     	lsr	x8, x8, #6
1008a5654:     	cmp	x9, #0x6
1008a5658:     	cinc	x20, x8, lo
1008a565c:     	cbz	x20, 0x1008a56fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
1008a5660:     	lsl	x24, x20, #3
1008a5664:     	mov	x0, x24
1008a5668:     	mov	w1, #0x8                ; =8
1008a566c:     	bl	0x1013fa450 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1008a5670:     	cbz	x0, 0x1008a5b2c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14b4>
1008a5674:     	mov	x25, x0
1008a5678:     	mov	x0, #0x0                ; =0
1008a567c:     	ldp	x8, x9, [sp, #0xc0]
1008a5680:     	ldp	x1, x10, [sp, #0xd0]
1008a5684:     	sub	x11, x0, w9, uxtb
1008a5688:     	ldp	x23, x13, [sp, #0xf0]
1008a568c:     	ldp	x12, x14, [sp, #0x100]
1008a5690:     	mov	x24, x25
1008a5694:     	b	0x1008a56b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
1008a5698:     	tst	w13, #0x1
1008a569c:     	csel	x15, x15, xzr, ne
1008a56a0:     	str	x15, [x24, x0, lsl #3]
1008a56a4:     	add	x0, x0, #0x1
1008a56a8:     	cmp	x20, x0
1008a56ac:     	b.eq	0x1008a5270 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1008a56b0:     	mov	x15, x11
1008a56b4:     	cmn	x8, #0x2
1008a56b8:     	b.eq	0x1008a56cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1054>
1008a56bc:     	cmp	x0, x1
1008a56c0:     	b.hs	0x1008a5ad0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1458>
1008a56c4:     	ldr	x15, [x9, x0, lsl #3]
1008a56c8:     	eor	x15, x10, x15
1008a56cc:     	cmn	x23, #0x2
1008a56d0:     	b.eq	0x1008a5698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1020>
1008a56d4:     	cmp	x0, x12
1008a56d8:     	b.hs	0x1008a5acc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1454>
1008a56dc:     	ldr	x16, [x13, x0, lsl #3]
1008a56e0:     	eor	x16, x14, x16
1008a56e4:     	and	x15, x16, x15
1008a56e8:     	str	x15, [x24, x0, lsl #3]
1008a56ec:     	add	x0, x0, #0x1
1008a56f0:     	cmp	x20, x0
1008a56f4:     	b.ne	0x1008a56b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
1008a56f8:     	b	0x1008a5270 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1008a56fc:     	mov	x27, #0x0               ; =0
1008a5700:     	ldr	x23, [sp, #0xf0]
1008a5704:     	mov	w24, #0x8               ; =8
1008a5708:     	cmp	x23, #0x1
1008a570c:     	b.lt	0x1008a5718 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10a0>
1008a5710:     	ldr	x0, [sp, #0xf8]
1008a5714:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5718:     	ldr	x8, [sp, #0xc0]
1008a571c:     	cmp	x8, #0x1
1008a5720:     	b.lt	0x1008a572c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10b4>
1008a5724:     	ldr	x0, [sp, #0xc8]
1008a5728:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a572c:     	ldr	x8, [x28, #0xc0]
1008a5730:     	mov	w9, #0x4                ; =4
1008a5734:     	stp	xzr, x9, [sp, #0xf0]
1008a5738:     	str	xzr, [sp, #0x100]
1008a573c:     	ands	x23, x8, x22
1008a5740:     	b.eq	0x1008a5974 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12fc>
1008a5744:     	mov	x19, #0x0               ; =0
1008a5748:     	mov	w8, #0x4                ; =4
1008a574c:     	b	0x1008a5774 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10fc>
1008a5750:     	ldr	x8, [sp, #0xf8]
1008a5754:     	rbit	x9, x23
1008a5758:     	clz	x9, x9
1008a575c:     	str	w9, [x8, x19, lsl #2]
1008a5760:     	add	x19, x19, #0x1
1008a5764:     	str	x19, [sp, #0x100]
1008a5768:     	sub	x9, x23, #0x1
1008a576c:     	ands	x23, x9, x23
1008a5770:     	b.eq	0x1008a578c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1114>
1008a5774:     	ldr	x9, [sp, #0xf0]
1008a5778:     	cmp	x19, x9
1008a577c:     	b.ne	0x1008a5754 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10dc>
1008a5780:     	add	x0, sp, #0xf0
1008a5784:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a5788:     	b	0x1008a5750 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10d8>
1008a578c:     	ldp	x9, x8, [sp, #0xf0]
1008a5790:     	str	x9, [sp, #0x70]
1008a5794:     	str	x8, [sp, #0x58]
1008a5798:     	cbz	x19, 0x1008a5954 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12dc>
1008a579c:     	mov	x23, x8
1008a57a0:     	mov	x25, x20
1008a57a4:     	add	x8, x8, x19, lsl #2
1008a57a8:     	str	x8, [sp, #0x78]
1008a57ac:     	b	0x1008a57cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1154>
1008a57b0:     	bic	x22, x22, x20
1008a57b4:     	mov	x24, x26
1008a57b8:     	mov	x27, x25
1008a57bc:     	mov	x20, x25
1008a57c0:     	ldr	x8, [sp, #0x78]
1008a57c4:     	cmp	x23, x8
1008a57c8:     	b.eq	0x1008a595c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12e4>
1008a57cc:     	str	x27, [sp, #0x80]
1008a57d0:     	ldr	w8, [x23], #0x4
1008a57d4:     	mov	w9, #0x1                ; =1
1008a57d8:     	lsl	x20, x9, x8
1008a57dc:     	sub	x8, x20, #0x1
1008a57e0:     	and	x8, x8, x22
1008a57e4:     	fmov	d0, x8
1008a57e8:     	cnt.8b	v0, v0
1008a57ec:     	addv.8b	b0, v0
1008a57f0:     	fmov	w26, s0
1008a57f4:     	fmov	d0, x22
1008a57f8:     	cnt.8b	v0, v0
1008a57fc:     	addv.8b	b0, v0
1008a5800:     	fmov	w27, s0
1008a5804:     	add	x0, sp, #0xc0
1008a5808:     	mov	x1, x24
1008a580c:     	mov	x2, x25
1008a5810:     	mov	x3, x27
1008a5814:     	mov	x4, x26
1008a5818:     	mov	w5, #0x0                ; =0
1008a581c:     	bl	0x100f906d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1008a5820:     	add	x0, sp, #0xf0
1008a5824:     	mov	x1, x24
1008a5828:     	mov	x2, x25
1008a582c:     	mov	x3, x27
1008a5830:     	mov	x4, x26
1008a5834:     	mov	w5, #0x1                ; =1
1008a5838:     	bl	0x100f906d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1008a583c:     	ldp	x19, x8, [sp, #0xc8]
1008a5840:     	ldp	x27, x28, [sp, #0xf0]
1008a5844:     	ldr	x9, [sp, #0x100]
1008a5848:     	cmp	x9, x8
1008a584c:     	csel	x25, x9, x8, lo
1008a5850:     	cbz	x25, 0x1008a58ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1234>
1008a5854:     	str	x24, [sp, #0x68]
1008a5858:     	lsl	x24, x25, #3
1008a585c:     	mov	x0, x24
1008a5860:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
1008a5864:     	cbz	x0, 0x1008a5a9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1424>
1008a5868:     	mov	x26, x0
1008a586c:     	cmp	x25, #0x8
1008a5870:     	b.hs	0x1008a58e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x126c>
1008a5874:     	mov	x8, #0x0                ; =0
1008a5878:     	ldr	x24, [sp, #0x68]
1008a587c:     	lsl	x11, x8, #3
1008a5880:     	add	x9, x19, x11
1008a5884:     	add	x10, x28, x11
1008a5888:     	add	x11, x26, x11
1008a588c:     	sub	x8, x25, x8
1008a5890:     	ldr	x12, [x10], #0x8
1008a5894:     	ldr	x13, [x9], #0x8
1008a5898:     	orr	x12, x13, x12
1008a589c:     	str	x12, [x11], #0x8
1008a58a0:     	subs	x8, x8, #0x1
1008a58a4:     	b.ne	0x1008a5890 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1218>
1008a58a8:     	b	0x1008a58b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
1008a58ac:     	mov	w26, #0x8               ; =8
1008a58b0:     	cbz	x27, 0x1008a58bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1244>
1008a58b4:     	mov	x0, x28
1008a58b8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a58bc:     	ldr	x8, [sp, #0x80]
1008a58c0:     	cbz	x8, 0x1008a58cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1254>
1008a58c4:     	mov	x0, x24
1008a58c8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a58cc:     	ldr	x8, [sp, #0xc0]
1008a58d0:     	ldr	x28, [sp, #0x88]
1008a58d4:     	cbz	x8, 0x1008a57b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
1008a58d8:     	mov	x0, x19
1008a58dc:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a58e0:     	b	0x1008a57b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
1008a58e4:     	mov	x8, #0x0                ; =0
1008a58e8:     	sub	x9, x28, x26
1008a58ec:     	cmn	x9, #0x40
1008a58f0:     	ldr	x24, [sp, #0x68]
1008a58f4:     	b.hi	0x1008a587c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1008a58f8:     	sub	x9, x19, x26
1008a58fc:     	cmn	x9, #0x40
1008a5900:     	b.hi	0x1008a587c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1008a5904:     	and	x8, x25, #0xffffffffffffff8
1008a5908:     	add	x9, x19, #0x20
1008a590c:     	add	x10, x28, #0x20
1008a5910:     	add	x11, x26, #0x20
1008a5914:     	and	x12, x25, #0xffffffffffffff8
1008a5918:     	ldp	q0, q1, [x10, #-0x20]
1008a591c:     	ldp	q2, q3, [x10], #0x40
1008a5920:     	ldp	q4, q5, [x9, #-0x20]
1008a5924:     	ldp	q6, q7, [x9], #0x40
1008a5928:     	orr.16b	v0, v4, v0
1008a592c:     	orr.16b	v1, v5, v1
1008a5930:     	orr.16b	v2, v6, v2
1008a5934:     	orr.16b	v3, v7, v3
1008a5938:     	stp	q0, q1, [x11, #-0x20]
1008a593c:     	stp	q2, q3, [x11], #0x40
1008a5940:     	subs	x12, x12, #0x8
1008a5944:     	b.ne	0x1008a5918 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12a0>
1008a5948:     	cmp	x25, x8
1008a594c:     	b.ne	0x1008a587c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1008a5950:     	b	0x1008a58b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
1008a5954:     	mov	x25, x27
1008a5958:     	mov	x26, x24
1008a595c:     	ldr	x8, [sp, #0x70]
1008a5960:     	cbz	x8, 0x1008a596c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12f4>
1008a5964:     	ldr	x0, [sp, #0x58]
1008a5968:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a596c:     	mov	x24, x26
1008a5970:     	mov	x27, x25
1008a5974:     	ldr	x19, [sp, #0x60]
1008a5978:     	stp	x27, x24, [sp, #0xf0]
1008a597c:     	str	x20, [sp, #0x100]
1008a5980:     	add	x2, sp, #0xf0
1008a5984:     	mov	x0, x21
1008a5988:     	mov	x1, x22
1008a598c:     	bl	0x100df9b64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
1008a5990:     	mov	x8, x0
1008a5994:     	add	x0, x28, #0x70
1008a5998:     	mov	x1, x19
1008a599c:     	mov	x19, x8
1008a59a0:     	mov	x2, x8
1008a59a4:     	bl	0x100e8ddf0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
1008a59a8:     	mov	x0, x19
1008a59ac:     	b	0x1008a544c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1008a59b0:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
1008a59b4:     	add	x2, x2, #0x268
1008a59b8:     	adrp	x3, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a59bc:     	add	x3, x3, #0x2d9
1008a59c0:     	adrp	x5, 0x101751000 <dyld_stub_binder+0x101751000>
1008a59c4:     	add	x5, x5, #0xed8
1008a59c8:     	add	x1, sp, #0xf0
1008a59cc:     	mov	w0, #0x0                ; =0
1008a59d0:     	mov	w4, #0x43               ; =67
1008a59d4:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1008a59d8:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a59dc:     	add	x2, x2, #0x5e0
1008a59e0:     	b	0x1008a59fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1384>
1008a59e4:     	ldr	x20, [sp, #0x68]
1008a59e8:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a59ec:     	add	x2, x2, #0x5e0
1008a59f0:     	b	0x1008a5a30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13b8>
1008a59f4:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a59f8:     	add	x2, x2, #0x5c8
1008a59fc:     	ldr	x20, [sp, #0x68]
1008a5a00:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a04:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5a08:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a5a0c:     	add	x0, x0, #0x46f
1008a5a10:     	adrp	x2, 0x101753000 <dyld_stub_binder+0x101753000>
1008a5a14:     	add	x2, x2, #0x188
1008a5a18:     	mov	w1, #0x51               ; =81
1008a5a1c:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1008a5a20:     	mov	x1, x8
1008a5a24:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a5a28:     	add	x2, x2, #0x5c8
1008a5a2c:     	ldr	x20, [sp, #0x68]
1008a5a30:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a34:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5a38:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a5a3c:     	add	x0, x0, #0x443
1008a5a40:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
1008a5a44:     	add	x2, x2, #0xd90
1008a5a48:     	mov	w1, #0x2c               ; =44
1008a5a4c:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1008a5a50:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
1008a5a54:     	add	x2, x2, #0x978
1008a5a58:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a5c:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
1008a5a60:     	add	x2, x2, #0x978
1008a5a64:     	mov	x1, x8
1008a5a68:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a6c:     	mov	x20, x16
1008a5a70:     	b	0x1008a5a80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1408>
1008a5a74:     	mov	x20, x16
1008a5a78:     	mov	x1, x8
1008a5a7c:     	mov	x2, x12
1008a5a80:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a84:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5a88:     	mov	x1, x12
1008a5a8c:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a5a90:     	add	x2, x2, #0xc18
1008a5a94:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5a98:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5a9c:     	mov	w0, #0x8                ; =8
1008a5aa0:     	mov	x1, x24
1008a5aa4:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a5aa8:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5aac:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a5ab0:     	add	x2, x2, #0xc30
1008a5ab4:     	mov	x0, x22
1008a5ab8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5abc:     	mov	x0, x8
1008a5ac0:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a5ac4:     	add	x2, x2, #0x5e0
1008a5ac8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5acc:     	mov	x1, x12
1008a5ad0:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a5ad4:     	add	x2, x2, #0xc18
1008a5ad8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5adc:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5ae0:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a5ae4:     	add	x2, x2, #0xc48
1008a5ae8:     	mov	x0, x22
1008a5aec:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5af0:     	mov	x0, x8
1008a5af4:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a5af8:     	add	x2, x2, #0x5c8
1008a5afc:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a5b00:     	mov	w0, #0x8                ; =8
1008a5b04:     	mov	x1, x24
1008a5b08:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a5b0c:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5b10:     	mov	w0, #0x8                ; =8
1008a5b14:     	mov	x1, x19
1008a5b18:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a5b1c:     	mov	w0, #0x8                ; =8
1008a5b20:     	mov	x1, x26
1008a5b24:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a5b28:     	b	0x1008a5b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1008a5b2c:     	mov	w0, #0x8                ; =8
1008a5b30:     	mov	x1, x24
1008a5b34:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a5b38:     	brk	#0x1
1008a5b3c:     	b	0x1008a5b4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14d4>
1008a5b40:     	b	0x1008a5b58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14e0>
1008a5b44:     	ldr	x20, [sp, #0x68]
1008a5b48:     	b	0x1008a5c3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1008a5b4c:     	mov	x19, x0
1008a5b50:     	ldr	x23, [sp, #0xf0]
1008a5b54:     	b	0x1008a5be4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x156c>
1008a5b58:     	mov	x19, x0
1008a5b5c:     	b	0x1008a5bf4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
1008a5b60:     	b	0x1008a5bd8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1560>
1008a5b64:     	mov	x20, x0
1008a5b68:     	cbz	x27, 0x1008a5ba4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
1008a5b6c:     	mov	x0, x28
1008a5b70:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5b74:     	b	0x1008a5ba4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
1008a5b78:     	b	0x1008a5c1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15a4>
1008a5b7c:     	str	x27, [sp, #0x80]
1008a5b80:     	str	x24, [sp, #0x68]
1008a5b84:     	mov	x20, x0
1008a5b88:     	ldr	x8, [sp, #0xf0]
1008a5b8c:     	cbz	x8, 0x1008a5bd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
1008a5b90:     	ldr	x8, [sp, #0xf8]
1008a5b94:     	str	x8, [sp, #0x58]
1008a5b98:     	b	0x1008a5bc8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1550>
1008a5b9c:     	str	x24, [sp, #0x68]
1008a5ba0:     	mov	x20, x0
1008a5ba4:     	ldr	x8, [sp, #0xc0]
1008a5ba8:     	cbz	x8, 0x1008a5bc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
1008a5bac:     	ldr	x0, [sp, #0xc8]
1008a5bb0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5bb4:     	b	0x1008a5bc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
1008a5bb8:     	str	x24, [sp, #0x68]
1008a5bbc:     	mov	x20, x0
1008a5bc0:     	ldr	x8, [sp, #0x70]
1008a5bc4:     	cbz	x8, 0x1008a5bd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
1008a5bc8:     	ldr	x0, [sp, #0x58]
1008a5bcc:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5bd0:     	mov	x0, x20
1008a5bd4:     	b	0x1008a5c30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
1008a5bd8:     	mov	x19, x0
1008a5bdc:     	mov	x0, x25
1008a5be0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5be4:     	cmp	x23, #0x1
1008a5be8:     	b.lt	0x1008a5bf4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
1008a5bec:     	ldr	x0, [sp, #0xf8]
1008a5bf0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5bf4:     	ldr	x8, [sp, #0xc0]
1008a5bf8:     	cmp	x8, #0x1
1008a5bfc:     	b.lt	0x1008a5c48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d0>
1008a5c00:     	ldr	x20, [sp, #0xc8]
1008a5c04:     	mov	x0, x19
1008a5c08:     	b	0x1008a5c3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1008a5c0c:     	tbz	w19, #0x0, 0x1008a5c3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1008a5c10:     	b	0x1008a5c4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1008a5c14:     	b	0x1008a5c30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
1008a5c18:     	b	0x1008a5c34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
1008a5c1c:     	ldr	x8, [sp, #0xf0]
1008a5c20:     	cbz	x8, 0x1008a5c4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1008a5c24:     	ldr	x20, [sp, #0xf8]
1008a5c28:     	b	0x1008a5c3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1008a5c2c:     	b	0x1008a5c34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
1008a5c30:     	ldr	x20, [sp, #0x68]
1008a5c34:     	ldr	x8, [sp, #0x80]
1008a5c38:     	cbz	x8, 0x1008a5c4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1008a5c3c:     	mov	x19, x0
1008a5c40:     	mov	x0, x20
1008a5c44:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a5c48:     	mov	x0, x19
1008a5c4c:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
