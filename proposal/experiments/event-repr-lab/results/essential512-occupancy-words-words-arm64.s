
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b234d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_>:
100b234d4:     	stp	x28, x27, [sp, #-0x60]!
100b234d8:     	stp	x26, x25, [sp, #0x10]
100b234dc:     	stp	x24, x23, [sp, #0x20]
100b234e0:     	stp	x22, x21, [sp, #0x30]
100b234e4:     	stp	x20, x19, [sp, #0x40]
100b234e8:     	stp	x29, x30, [sp, #0x50]
100b234ec:     	add	x29, sp, #0x50
100b234f0:     	sub	sp, sp, #0x210
100b234f4:     	ldr	w19, [x3, #0x10]
100b234f8:     	cbz	w19, 0x100b2352c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x58>
100b234fc:     	mov	x21, x5
100b23500:     	mov	x27, x4
100b23504:     	mov	x20, x3
100b23508:     	mov	x23, x2
100b2350c:     	mov	x24, x1
100b23510:     	mov	x25, x0
100b23514:     	mov	x0, x4
100b23518:     	mov	x1, x3
100b2351c:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b23520:     	cbz	x0, 0x100b23534 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x60>
100b23524:     	ldrb	w22, [x0]
100b23528:     	b	0x100b23c68 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x794>
100b2352c:     	mov	w22, #0x0               ; =0
100b23530:     	b	0x100b23c68 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x794>
100b23534:     	ldr	x8, [x21]
100b23538:     	add	x8, x8, #0x1
100b2353c:     	str	x8, [x21]
100b23540:     	mov	x22, x25
100b23544:     	ldr	x8, [x22, #0x30]!
100b23548:     	ldr	x1, [x22, #0x10]
100b2354c:     	ldr	x9, [x20]
100b23550:     	lsr	x0, x19, #1
100b23554:     	cmn	x8, #0x1
100b23558:     	b.eq	0x100b235b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0xdc>
100b2355c:     	cmp	x1, x0
100b23560:     	b.ls	0x100b23d58 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x884>
100b23564:     	ldr	w8, [x20, #0x28]
100b23568:     	lsr	x8, x8, #1
100b2356c:     	cmp	x1, x8
100b23570:     	b.ls	0x100b23d44 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x870>
100b23574:     	ldr	w10, [x20, #0x40]
100b23578:     	lsr	x10, x10, #1
100b2357c:     	cmp	x1, x10
100b23580:     	b.ls	0x100b23d54 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x880>
100b23584:     	ldr	x11, [x22, #0x8]
100b23588:     	lsl	x8, x8, #4
100b2358c:     	ldr	x8, [x11, x8]
100b23590:     	ldr	x12, [x20, #0x18]
100b23594:     	bic	x8, x8, x12
100b23598:     	lsl	x12, x0, #4
100b2359c:     	ldr	x12, [x11, x12]
100b235a0:     	bic	x9, x12, x9
100b235a4:     	orr	x8, x8, x9
100b235a8:     	add	x9, x11, x10, lsl #4
100b235ac:     	b	0x100b23604 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x130>
100b235b0:     	ldr	x8, [x22, #0x18]
100b235b4:     	cmp	x8, x0
100b235b8:     	b.ls	0x100b23d7c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x8a8>
100b235bc:     	ldr	w10, [x20, #0x28]
100b235c0:     	lsr	x11, x10, #1
100b235c4:     	cmp	x8, x11
100b235c8:     	b.ls	0x100b23d64 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x890>
100b235cc:     	ldr	w10, [x20, #0x40]
100b235d0:     	lsr	x10, x10, #1
100b235d4:     	cmp	x8, x10
100b235d8:     	b.ls	0x100b23d78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x8a4>
100b235dc:     	add	x8, x1, x11, lsl #5
100b235e0:     	ldr	x8, [x8, #0x18]
100b235e4:     	ldr	x11, [x20, #0x18]
100b235e8:     	bic	x8, x8, x11
100b235ec:     	add	x11, x1, x0, lsl #5
100b235f0:     	ldr	x11, [x11, #0x18]
100b235f4:     	bic	x9, x11, x9
100b235f8:     	orr	x8, x8, x9
100b235fc:     	add	x9, x1, x10, lsl #5
100b23600:     	add	x9, x9, #0x18
100b23604:     	ldr	x9, [x9]
100b23608:     	mov	x19, x20
100b2360c:     	ldr	x10, [x19, #0x30]!
100b23610:     	bic	x9, x9, x10
100b23614:     	orr	x11, x9, x8
100b23618:     	fmov	d0, x11
100b2361c:     	cnt.8b	v0, v0
100b23620:     	addv.8b	b0, v0
100b23624:     	fmov	x8, d0
100b23628:     	cmp	x8, #0xa
100b2362c:     	mov	x9, x20
100b23630:     	str	x21, [sp, #0x48]
100b23634:     	b.hs	0x100b238b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x3dc>
100b23638:     	stp	x9, x8, [sp, #0x8]
100b2363c:     	str	x27, [sp]
100b23640:     	mov	x27, #0x0               ; =0
100b23644:     	ldr	x8, [x21, #0x10]
100b23648:     	add	x8, x8, #0x1
100b2364c:     	str	x8, [x21, #0x10]
100b23650:     	ldp	x23, x20, [x21, #0x30]
100b23654:     	add	x26, sp, #0xb0
100b23658:     	mov	x10, x9
100b2365c:     	str	x22, [sp, #0x18]
100b23660:     	str	x11, [sp, #0x30]
100b23664:     	b	0x100b2369c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x1c8>
100b23668:     	and	w24, w19, #0x1
100b2366c:     	add	x23, x23, #0x1
100b23670:     	str	x23, [x21, #0x30]
100b23674:     	mov	x19, #-0x2              ; =-2
100b23678:     	ldr	x11, [sp, #0x30]
100b2367c:     	ldr	x10, [sp, #0x38]
100b23680:     	add	x10, x10, #0x18
100b23684:     	add	x9, x26, x27, lsl #5
100b23688:     	stp	x19, x24, [x9]
100b2368c:     	stp	x25, x8, [x9, #0x10]
100b23690:     	add	x27, x27, #0x1
100b23694:     	cmp	x27, #0x3
100b23698:     	b.eq	0x100b239d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x504>
100b2369c:     	ldp	x28, x8, [x10]
100b236a0:     	stp	x10, x8, [sp, #0x38]
100b236a4:     	ldr	w19, [x10, #0x10]
100b236a8:     	stur	x11, [x29, #-0x90]
100b236ac:     	add	x0, sp, #0x50
100b236b0:     	mov	x1, x22
100b236b4:     	mov	x2, x19
100b236b8:     	bl	0x100c82fec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b236bc:     	ldr	w8, [sp, #0x50]
100b236c0:     	cbz	w8, 0x100b23668 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x194>
100b236c4:     	cmp	w8, #0x1
100b236c8:     	ldr	x9, [sp, #0x30]
100b236cc:     	b.ne	0x100b23d0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x838>
100b236d0:     	ldp	x26, x25, [sp, #0x58]
100b236d4:     	ldr	x22, [sp, #0x68]
100b236d8:     	stur	x22, [x29, #-0x78]
100b236dc:     	bics	x8, x28, x22
100b236e0:     	str	x8, [sp, #0x50]
100b236e4:     	b.ne	0x100b23c8c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7b8>
100b236e8:     	ldr	x8, [sp, #0x40]
100b236ec:     	bics	x8, x8, x28
100b236f0:     	str	x8, [sp, #0x50]
100b236f4:     	b.ne	0x100b23c9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7c8>
100b236f8:     	orr	x8, x28, x9
100b236fc:     	bics	x8, x22, x8
100b23700:     	str	x8, [sp, #0x50]
100b23704:     	b.ne	0x100b23cac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7d8>
100b23708:     	ands	x8, x28, x9
100b2370c:     	str	x8, [sp, #0x50]
100b23710:     	b.ne	0x100b23cbc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7e8>
100b23714:     	stp	x19, x23, [sp, #0x20]
100b23718:     	cbz	x28, 0x100b237d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x2fc>
100b2371c:     	mov	x19, #-0x1              ; =-1
100b23720:     	b	0x100b2374c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x278>
100b23724:     	add	x20, x20, #0x1
100b23728:     	ldr	x8, [sp, #0x48]
100b2372c:     	str	x20, [x8, #0x38]
100b23730:     	bic	x22, x22, x21
100b23734:     	stur	x22, [x29, #-0x78]
100b23738:     	mov	x26, x24
100b2373c:     	mov	x19, x23
100b23740:     	cmp	x21, x28
100b23744:     	eor	x28, x21, x28
100b23748:     	b.eq	0x100b237b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x2e4>
100b2374c:     	neg	x8, x28
100b23750:     	and	x21, x28, x8
100b23754:     	sub	x8, x21, #0x1
100b23758:     	and	x8, x8, x22
100b2375c:     	fmov	d0, x8
100b23760:     	cnt.8b	v0, v0
100b23764:     	addv.8b	b0, v0
100b23768:     	fmov	w4, s0
100b2376c:     	fmov	d0, x22
100b23770:     	cnt.8b	v0, v0
100b23774:     	addv.8b	b0, v0
100b23778:     	fmov	w3, s0
100b2377c:     	ldr	x8, [sp, #0x40]
100b23780:     	tst	x21, x8
100b23784:     	cset	w5, ne
100b23788:     	add	x0, sp, #0x50
100b2378c:     	mov	x1, x26
100b23790:     	mov	x2, x25
100b23794:     	bl	0x100d1a098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b23798:     	ldp	x23, x24, [sp, #0x50]
100b2379c:     	ldr	x25, [sp, #0x60]
100b237a0:     	sub	x8, x19, #0x1
100b237a4:     	cmn	x8, #0x3
100b237a8:     	b.hi	0x100b23724 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x250>
100b237ac:     	mov	x0, x26
100b237b0:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b237b4:     	b	0x100b23724 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x250>
100b237b8:     	mvn	x8, x22
100b237bc:     	mov	x26, x24
100b237c0:     	ldr	x9, [sp, #0x30]
100b237c4:     	ands	x28, x8, x9
100b237c8:     	b.ne	0x100b23850 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x37c>
100b237cc:     	b	0x100b237e0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x30c>
100b237d0:     	mvn	x8, x22
100b237d4:     	mov	x23, #-0x1              ; =-1
100b237d8:     	ands	x28, x8, x9
100b237dc:     	b.ne	0x100b23850 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x37c>
100b237e0:     	mov	x19, x23
100b237e4:     	mov	x24, x26
100b237e8:     	ldr	x11, [sp, #0x30]
100b237ec:     	cmp	x22, x11
100b237f0:     	b.ne	0x100b23ce0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x80c>
100b237f4:     	cmn	x19, #0x1
100b237f8:     	mov	w8, #0x28               ; =40
100b237fc:     	mov	w9, #0x20               ; =32
100b23800:     	csel	x8, x9, x8, eq
100b23804:     	ldr	x21, [sp, #0x48]
100b23808:     	ldr	x9, [x21, x8]
100b2380c:     	add	x9, x9, #0x1
100b23810:     	str	x9, [x21, x8]
100b23814:     	ldp	x22, x8, [sp, #0x18]
100b23818:     	sbfx	x8, x8, #0, #1
100b2381c:     	ldr	x23, [sp, #0x28]
100b23820:     	add	x26, sp, #0xb0
100b23824:     	b	0x100b2367c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x1a8>
100b23828:     	add	x20, x20, #0x1
100b2382c:     	ldr	x8, [sp, #0x48]
100b23830:     	str	x20, [x8, #0x38]
100b23834:     	orr	x22, x21, x22
100b23838:     	stur	x22, [x29, #-0x78]
100b2383c:     	mov	x26, x24
100b23840:     	mov	x23, x19
100b23844:     	cmp	x21, x28
100b23848:     	eor	x28, x21, x28
100b2384c:     	b.eq	0x100b237e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x314>
100b23850:     	neg	x8, x28
100b23854:     	and	x21, x28, x8
100b23858:     	sub	x8, x21, #0x1
100b2385c:     	and	x8, x8, x22
100b23860:     	fmov	d0, x8
100b23864:     	cnt.8b	v0, v0
100b23868:     	addv.8b	b0, v0
100b2386c:     	fmov	w4, s0
100b23870:     	fmov	d0, x22
100b23874:     	cnt.8b	v0, v0
100b23878:     	addv.8b	b0, v0
100b2387c:     	fmov	w3, s0
100b23880:     	add	x0, sp, #0x50
100b23884:     	mov	x1, x26
100b23888:     	mov	x2, x25
100b2388c:     	bl	0x100d1ab54 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100b23890:     	ldp	x19, x24, [sp, #0x50]
100b23894:     	ldr	x25, [sp, #0x60]
100b23898:     	sub	x8, x23, #0x1
100b2389c:     	cmn	x8, #0x3
100b238a0:     	b.hi	0x100b23828 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x354>
100b238a4:     	mov	x0, x26
100b238a8:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b238ac:     	b	0x100b23828 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x354>
100b238b0:     	mov	x21, x9
100b238b4:     	mov	x8, #0x0                ; =0
100b238b8:     	add	x20, sp, #0x110
100b238bc:     	lsl	x9, x23, #2
100b238c0:     	cmp	x9, x8
100b238c4:     	b.eq	0x100b23d00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x82c>
100b238c8:     	ldr	w22, [x24, x8]
100b238cc:     	lsr	x10, x11, x22
100b238d0:     	add	x8, x8, #0x4
100b238d4:     	tbz	w10, #0x0, 0x100b238c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x3ec>
100b238d8:     	ldr	q0, [x21]
100b238dc:     	str	q0, [sp, #0xb0]
100b238e0:     	ldr	x8, [x21, #0x10]
100b238e4:     	str	x8, [sp, #0xc0]
100b238e8:     	sub	x0, x29, #0x78
100b238ec:     	add	x1, sp, #0xb0
100b238f0:     	mov	x2, x25
100b238f4:     	mov	x3, x22
100b238f8:     	mov	w4, #0x0                ; =0
100b238fc:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23900:     	ldur	q0, [x20, #0xd8]
100b23904:     	ldur	x8, [x29, #-0x68]
100b23908:     	stur	x8, [x29, #-0xa0]
100b2390c:     	str	q0, [sp, #0x50]
100b23910:     	str	x8, [sp, #0x60]
100b23914:     	str	q0, [sp, #0x110]
100b23918:     	str	x8, [sp, #0x120]
100b2391c:     	ldur	q0, [x21, #0x18]
100b23920:     	str	q0, [sp, #0xb0]
100b23924:     	ldr	x8, [x21, #0x28]
100b23928:     	str	x8, [sp, #0xc0]
100b2392c:     	sub	x0, x29, #0x78
100b23930:     	add	x1, sp, #0xb0
100b23934:     	mov	x2, x25
100b23938:     	mov	x3, x22
100b2393c:     	mov	w4, #0x0                ; =0
100b23940:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23944:     	ldur	q0, [x20, #0xd8]
100b23948:     	ldur	x8, [x29, #-0x68]
100b2394c:     	stur	x8, [x29, #-0xa0]
100b23950:     	str	q0, [sp, #0x50]
100b23954:     	str	x8, [sp, #0x60]
100b23958:     	stur	q0, [x20, #0x18]
100b2395c:     	str	x8, [sp, #0x138]
100b23960:     	ldr	q0, [x19]
100b23964:     	str	q0, [sp, #0xb0]
100b23968:     	ldr	x8, [x19, #0x10]
100b2396c:     	str	x8, [sp, #0xc0]
100b23970:     	sub	x0, x29, #0x78
100b23974:     	add	x1, sp, #0xb0
100b23978:     	mov	x2, x25
100b2397c:     	mov	x26, x22
100b23980:     	mov	x3, x22
100b23984:     	mov	w4, #0x0                ; =0
100b23988:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2398c:     	ldur	q0, [x20, #0xd8]
100b23990:     	ldur	x8, [x29, #-0x68]
100b23994:     	stur	x8, [x29, #-0xa0]
100b23998:     	str	q0, [sp, #0x50]
100b2399c:     	str	q0, [sp, #0x140]
100b239a0:     	str	x8, [sp, #0x150]
100b239a4:     	add	x3, sp, #0x110
100b239a8:     	mov	x0, x25
100b239ac:     	mov	x1, x24
100b239b0:     	mov	x2, x23
100b239b4:     	mov	x4, x27
100b239b8:     	ldr	x28, [sp, #0x48]
100b239bc:     	mov	x5, x28
100b239c0:     	bl	0x100b234d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_>
100b239c4:     	and	w8, w0, #0xff
100b239c8:     	cmp	w8, #0xf
100b239cc:     	b.ne	0x100b23b10 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x63c>
100b239d0:     	mov	w22, #0xf               ; =15
100b239d4:     	b	0x100b23c18 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x744>
100b239d8:     	mov	x0, #0x0                ; =0
100b239dc:     	mov	w22, #0x0               ; =0
100b239e0:     	ldp	q1, q0, [sp, #0xf0]
100b239e4:     	stp	q1, q0, [sp, #0x90]
100b239e8:     	ldp	q1, q0, [sp, #0xd0]
100b239ec:     	stp	q1, q0, [sp, #0x70]
100b239f0:     	ldp	q1, q0, [sp, #0xb0]
100b239f4:     	stp	q1, q0, [sp, #0x50]
100b239f8:     	mov	w8, #0x1                ; =1
100b239fc:     	ldr	x11, [sp, #0x10]
100b23a00:     	lsl	x8, x8, x11
100b23a04:     	mov	x9, #-0x1               ; =-1
100b23a08:     	lsl	x10, x9, x8
100b23a0c:     	cmp	x11, #0x6
100b23a10:     	csinv	x10, x9, x10, hs
100b23a14:     	lsr	x8, x8, #6
100b23a18:     	cinc	x11, x8, lo
100b23a1c:     	ldp	x9, x8, [sp, #0x50]
100b23a20:     	tst	w8, #0x1
100b23a24:     	csel	x12, x10, xzr, ne
100b23a28:     	ldp	x1, x13, [sp, #0x60]
100b23a2c:     	ldp	x19, x23, [sp, #0x70]
100b23a30:     	sub	x15, x0, w23, uxtb
100b23a34:     	ldp	x14, x16, [sp, #0x80]
100b23a38:     	ldp	x20, x24, [sp, #0x90]
100b23a3c:     	sub	x17, x0, w24, uxtb
100b23a40:     	ldr	x2, [x21, #0x18]
100b23a44:     	add	x2, x2, #0x1
100b23a48:     	mov	w3, #0x2                ; =2
100b23a4c:     	mov	w4, #0x4                ; =4
100b23a50:     	mov	w6, #0x8                ; =8
100b23a54:     	ldp	x5, x7, [sp, #0xa0]
100b23a58:     	b	0x100b23aa8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x5d4>
100b23a5c:     	bic	x27, x21, x25
100b23a60:     	tst	x26, x27
100b23a64:     	csel	w28, wzr, w3, eq
100b23a68:     	and	x21, x25, x21
100b23a6c:     	bics	xzr, x21, x26
100b23a70:     	csel	w25, wzr, w4, eq
100b23a74:     	tst	x26, x21
100b23a78:     	csel	w21, wzr, w6, eq
100b23a7c:     	bics	xzr, x27, x26
100b23a80:     	cinc	w26, w28, ne
100b23a84:     	orr	w21, w25, w21
100b23a88:     	orr	w21, w26, w21
100b23a8c:     	orr	w22, w21, w22
100b23a90:     	and	w21, w22, #0xff
100b23a94:     	add	x2, x2, #0x1
100b23a98:     	add	x0, x0, #0x1
100b23a9c:     	cmp	w21, #0xf
100b23aa0:     	ldr	x21, [sp, #0x48]
100b23aa4:     	b.eq	0x100b23c20 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x74c>
100b23aa8:     	cmp	x11, x0
100b23aac:     	b.eq	0x100b23c24 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x750>
100b23ab0:     	str	x2, [x21, #0x18]
100b23ab4:     	mov	x21, x12
100b23ab8:     	cmn	x9, #0x2
100b23abc:     	b.eq	0x100b23ad4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x600>
100b23ac0:     	cmp	x0, x1
100b23ac4:     	b.hs	0x100b23d34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x860>
100b23ac8:     	ldr	x21, [x8, x0, lsl #3]
100b23acc:     	eor	x21, x21, x13
100b23ad0:     	and	x21, x21, x10
100b23ad4:     	mov	x25, x15
100b23ad8:     	cmn	x19, #0x2
100b23adc:     	b.eq	0x100b23af0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x61c>
100b23ae0:     	cmp	x0, x14
100b23ae4:     	b.hs	0x100b23d28 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x854>
100b23ae8:     	ldr	x25, [x23, x0, lsl #3]
100b23aec:     	eor	x25, x25, x16
100b23af0:     	mov	x26, x17
100b23af4:     	cmn	x20, #0x2
100b23af8:     	b.eq	0x100b23a5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x588>
100b23afc:     	cmp	x0, x5
100b23b00:     	b.hs	0x100b23d30 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x85c>
100b23b04:     	ldr	x26, [x24, x0, lsl #3]
100b23b08:     	eor	x26, x26, x7
100b23b0c:     	b	0x100b23a5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x588>
100b23b10:     	mov	x22, x0
100b23b14:     	ldr	q0, [x21]
100b23b18:     	str	q0, [sp, #0xb0]
100b23b1c:     	ldr	x8, [x21, #0x10]
100b23b20:     	str	x8, [sp, #0xc0]
100b23b24:     	sub	x0, x29, #0x78
100b23b28:     	add	x1, sp, #0xb0
100b23b2c:     	mov	x2, x25
100b23b30:     	mov	x3, x26
100b23b34:     	mov	w4, #0x1                ; =1
100b23b38:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23b3c:     	ldur	q0, [x20, #0xd8]
100b23b40:     	stur	q0, [x29, #-0x90]
100b23b44:     	ldur	x8, [x29, #-0x68]
100b23b48:     	stur	q0, [x29, #-0xb0]
100b23b4c:     	str	q0, [sp, #0x50]
100b23b50:     	str	x8, [sp, #0x60]
100b23b54:     	ldr	q0, [sp, #0x50]
100b23b58:     	stur	x8, [x29, #-0xf0]
100b23b5c:     	stur	q0, [x29, #-0x100]
100b23b60:     	ldur	q0, [x21, #0x18]
100b23b64:     	str	q0, [sp, #0xb0]
100b23b68:     	ldur	x8, [x21, #0x28]
100b23b6c:     	str	x8, [sp, #0xc0]
100b23b70:     	sub	x0, x29, #0x78
100b23b74:     	add	x1, sp, #0xb0
100b23b78:     	mov	x2, x25
100b23b7c:     	mov	x3, x26
100b23b80:     	mov	w4, #0x1                ; =1
100b23b84:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23b88:     	ldur	q0, [x20, #0xd8]
100b23b8c:     	stur	q0, [x29, #-0x90]
100b23b90:     	ldur	x8, [x29, #-0x68]
100b23b94:     	stur	q0, [x29, #-0xb0]
100b23b98:     	str	q0, [sp, #0x50]
100b23b9c:     	str	x8, [sp, #0x60]
100b23ba0:     	ldr	q0, [sp, #0x50]
100b23ba4:     	stur	x8, [x29, #-0xd8]
100b23ba8:     	stur	q0, [x20, #0x68]
100b23bac:     	ldr	q0, [x19]
100b23bb0:     	str	q0, [sp, #0xb0]
100b23bb4:     	ldr	x8, [x19, #0x10]
100b23bb8:     	str	x8, [sp, #0xc0]
100b23bbc:     	sub	x0, x29, #0x78
100b23bc0:     	add	x1, sp, #0xb0
100b23bc4:     	mov	x2, x25
100b23bc8:     	mov	x3, x26
100b23bcc:     	mov	w4, #0x1                ; =1
100b23bd0:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b23bd4:     	ldur	q0, [x20, #0xd8]
100b23bd8:     	stur	q0, [x29, #-0x90]
100b23bdc:     	ldur	x8, [x29, #-0x68]
100b23be0:     	stur	q0, [x29, #-0xb0]
100b23be4:     	str	q0, [sp, #0x50]
100b23be8:     	str	x8, [sp, #0x60]
100b23bec:     	ldr	q0, [sp, #0x50]
100b23bf0:     	stur	x8, [x29, #-0xc0]
100b23bf4:     	stur	q0, [x29, #-0xd0]
100b23bf8:     	sub	x3, x29, #0x100
100b23bfc:     	mov	x0, x25
100b23c00:     	mov	x1, x24
100b23c04:     	mov	x2, x23
100b23c08:     	mov	x4, x27
100b23c0c:     	mov	x5, x28
100b23c10:     	bl	0x100b234d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_>
100b23c14:     	orr	w22, w0, w22
100b23c18:     	mov	x1, x21
100b23c1c:     	b	0x100b23c5c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x788>
100b23c20:     	mov	w22, #0xf               ; =15
100b23c24:     	cmp	x9, #0x1
100b23c28:     	ldr	x27, [sp]
100b23c2c:     	b.lt	0x100b23c38 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x764>
100b23c30:     	mov	x0, x8
100b23c34:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23c38:     	cmp	x19, #0x1
100b23c3c:     	b.lt	0x100b23c48 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x774>
100b23c40:     	mov	x0, x23
100b23c44:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23c48:     	cmp	x20, #0x1
100b23c4c:     	b.lt	0x100b23c58 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x784>
100b23c50:     	mov	x0, x24
100b23c54:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23c58:     	ldr	x1, [sp, #0x8]
100b23c5c:     	mov	x0, x27
100b23c60:     	mov	x2, x22
100b23c64:     	bl	0x100c293a0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b23c68:     	mov	x0, x22
100b23c6c:     	add	sp, sp, #0x210
100b23c70:     	ldp	x29, x30, [sp, #0x50]
100b23c74:     	ldp	x20, x19, [sp, #0x40]
100b23c78:     	ldp	x22, x21, [sp, #0x30]
100b23c7c:     	ldp	x24, x23, [sp, #0x20]
100b23c80:     	ldp	x26, x25, [sp, #0x10]
100b23c84:     	ldp	x28, x27, [sp], #0x60
100b23c88:     	ret
100b23c8c:     	add	x1, sp, #0x50
100b23c90:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23c94:     	add	x5, x5, #0xf0
100b23c98:     	b	0x100b23cc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7f4>
100b23c9c:     	add	x1, sp, #0x50
100b23ca0:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23ca4:     	add	x5, x5, #0xd8
100b23ca8:     	b	0x100b23cc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7f4>
100b23cac:     	add	x1, sp, #0x50
100b23cb0:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23cb4:     	add	x5, x5, #0xc0
100b23cb8:     	b	0x100b23cc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x7f4>
100b23cbc:     	add	x1, sp, #0x50
100b23cc0:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23cc4:     	add	x5, x5, #0xa8
100b23cc8:     	adrp	x2, 0x1013e6000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x2177>
100b23ccc:     	add	x2, x2, #0xa8
100b23cd0:     	mov	w0, #0x0                ; =0
100b23cd4:     	mov	x3, #0x0                ; =0
100b23cd8:     	bl	0x101281ee0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b23cdc:     	b	0x100b23d40 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x86c>
100b23ce0:     	adrp	x5, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23ce4:     	add	x5, x5, #0x90
100b23ce8:     	sub	x1, x29, #0x78
100b23cec:     	sub	x2, x29, #0x90
100b23cf0:     	mov	w0, #0x0                ; =0
100b23cf4:     	mov	x3, #0x0                ; =0
100b23cf8:     	bl	0x101281ee0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b23cfc:     	b	0x100b23d40 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x86c>
100b23d00:     	adrp	x0, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b23d04:     	add	x0, x0, #0xb78
100b23d08:     	bl	0x101282074 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b23d0c:     	adrp	x0, 0x101325000 <dyld_stub_binder+0x101325000>
100b23d10:     	add	x0, x0, #0x849
100b23d14:     	adrp	x2, 0x1014bc000 <dyld_stub_binder+0x1014bc000>
100b23d18:     	add	x2, x2, #0x108
100b23d1c:     	mov	w1, #0xc9               ; =201
100b23d20:     	bl	0x101281e74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b23d24:     	b	0x100b23d40 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x86c>
100b23d28:     	mov	x1, x14
100b23d2c:     	b	0x100b23d34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x860>
100b23d30:     	mov	x1, x5
100b23d34:     	adrp	x2, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b23d38:     	add	x2, x2, #0x20
100b23d3c:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23d40:     	brk	#0x1
100b23d44:     	mov	x0, x8
100b23d48:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23d4c:     	add	x2, x2, #0x368
100b23d50:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23d54:     	mov	x0, x10
100b23d58:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23d5c:     	add	x2, x2, #0x368
100b23d60:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23d64:     	mov	x0, x11
100b23d68:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23d6c:     	add	x2, x2, #0x350
100b23d70:     	mov	x1, x8
100b23d74:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23d78:     	mov	x0, x10
100b23d7c:     	adrp	x2, 0x101504000 <dyld_stub_binder+0x101504000>
100b23d80:     	add	x2, x2, #0x350
100b23d84:     	mov	x1, x8
100b23d88:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b23d8c:     	mov	x20, x0
100b23d90:     	add	x0, sp, #0x50
100b23d94:     	bl	0x10071517c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej3_EBK_>
100b23d98:     	mov	x0, x20
100b23d9c:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
100b23da0:     	b	0x100b23dd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x904>
100b23da4:     	mov	x20, x0
100b23da8:     	mov	x19, x23
100b23dac:     	b	0x100b23dc0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x8ec>
100b23db0:     	mov	x20, x0
100b23db4:     	b	0x100b23dc0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x8ec>
100b23db8:     	mov	x20, x0
100b23dbc:     	mov	x26, x24
100b23dc0:     	sub	x8, x19, #0x1
100b23dc4:     	cmn	x8, #0x3
100b23dc8:     	b.hi	0x100b23ddc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x908>
100b23dcc:     	mov	x0, x26
100b23dd0:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23dd4:     	b	0x100b23ddc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x908>
100b23dd8:     	mov	x20, x0
100b23ddc:     	cbnz	x27, 0x100b23de8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x914>
100b23de0:     	mov	x0, x20
100b23de4:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
100b23de8:     	add	x8, sp, #0xb0
100b23dec:     	add	x19, x8, #0x8
100b23df0:     	b	0x100b23e00 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x92c>
100b23df4:     	add	x19, x19, #0x20
100b23df8:     	subs	x27, x27, #0x1
100b23dfc:     	b.eq	0x100b23de0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x90c>
100b23e00:     	ldur	x8, [x19, #-0x8]
100b23e04:     	cmp	x8, #0x1
100b23e08:     	b.lt	0x100b23df4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x920>
100b23e0c:     	ldr	x0, [x19]
100b23e10:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100b23e14:     	b	0x100b23df4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kb1_EB8_+0x920>
