
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b135c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_>:
100b135c0:     	stp	d15, d14, [sp, #-0xa0]!
100b135c4:     	stp	d13, d12, [sp, #0x10]
100b135c8:     	stp	d11, d10, [sp, #0x20]
100b135cc:     	stp	d9, d8, [sp, #0x30]
100b135d0:     	stp	x28, x27, [sp, #0x40]
100b135d4:     	stp	x26, x25, [sp, #0x50]
100b135d8:     	stp	x24, x23, [sp, #0x60]
100b135dc:     	stp	x22, x21, [sp, #0x70]
100b135e0:     	stp	x20, x19, [sp, #0x80]
100b135e4:     	stp	x29, x30, [sp, #0x90]
100b135e8:     	add	x29, sp, #0x90
100b135ec:     	sub	sp, sp, #0x220
100b135f0:     	ldr	w8, [x3, #0x10]
100b135f4:     	str	x8, [sp, #0xd0]
100b135f8:     	cbz	w8, 0x100b1362c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6c>
100b135fc:     	mov	x25, x5
100b13600:     	mov	x24, x4
100b13604:     	mov	x22, x3
100b13608:     	mov	x26, x2
100b1360c:     	mov	x27, x1
100b13610:     	mov	x28, x0
100b13614:     	mov	x0, x4
100b13618:     	mov	x1, x3
100b1361c:     	bl	0x10065e4b4 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b13620:     	cbz	x0, 0x100b13634 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x74>
100b13624:     	ldrb	w27, [x0]
100b13628:     	b	0x100b13d44 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x784>
100b1362c:     	mov	w27, #0x0               ; =0
100b13630:     	b	0x100b13d44 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x784>
100b13634:     	ldr	x8, [x25]
100b13638:     	add	x8, x8, #0x1
100b1363c:     	str	x8, [x25]
100b13640:     	ldr	x1, [x28, #0x28]
100b13644:     	ldr	x8, [sp, #0xd0]
100b13648:     	lsr	x0, x8, #1
100b1364c:     	cmp	x1, x0
100b13650:     	b.ls	0x100b13d88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x7c8>
100b13654:     	ldr	w8, [x22, #0x28]
100b13658:     	str	x8, [sp, #0xc8]
100b1365c:     	lsr	x9, x8, #1
100b13660:     	cmp	x1, x9
100b13664:     	b.ls	0x100b13d84 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x7c4>
100b13668:     	ldr	w8, [x22, #0x40]
100b1366c:     	str	x8, [sp, #0xc0]
100b13670:     	lsr	x8, x8, #1
100b13674:     	cmp	x1, x8
100b13678:     	b.ls	0x100b13d94 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x7d4>
100b1367c:     	ldr	x10, [x28, #0x20]
100b13680:     	add	x9, x10, x9, lsl #5
100b13684:     	ldr	x9, [x9, #0x18]
100b13688:     	ldr	x11, [x22, #0x18]
100b1368c:     	bic	x9, x9, x11
100b13690:     	add	x11, x10, x0, lsl #5
100b13694:     	ldr	x11, [x11, #0x18]
100b13698:     	ldr	x12, [x22]
100b1369c:     	bic	x11, x11, x12
100b136a0:     	orr	x9, x9, x11
100b136a4:     	ldr	x11, [x22, #0x30]
100b136a8:     	add	x8, x10, x8, lsl #5
100b136ac:     	ldr	x8, [x8, #0x18]
100b136b0:     	bic	x8, x8, x11
100b136b4:     	orr	x20, x8, x9
100b136b8:     	fmov	d0, x20
100b136bc:     	cnt.8b	v0, v0
100b136c0:     	addv.8b	b0, v0
100b136c4:     	fmov	x8, d0
100b136c8:     	cmp	x8, #0xa
100b136cc:     	b.hs	0x100b13734 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x174>
100b136d0:     	str	x24, [sp, #0x8]
100b136d4:     	str	x22, [sp, #0x18]
100b136d8:     	mov	w26, #0x4               ; =4
100b136dc:     	stp	xzr, x26, [x29, #-0xc0]
100b136e0:     	stur	xzr, [x29, #-0xb0]
100b136e4:     	mov	w24, #0x1               ; =1
100b136e8:     	mov	x22, #0x0               ; =0
100b136ec:     	cbz	x20, 0x100b1397c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x3bc>
100b136f0:     	mov	w8, #0x4                ; =4
100b136f4:     	b	0x100b1371c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x15c>
100b136f8:     	ldur	x8, [x29, #-0xb8]
100b136fc:     	rbit	x9, x20
100b13700:     	clz	x9, x9
100b13704:     	str	w9, [x8, x22, lsl #2]
100b13708:     	add	x22, x22, #0x1
100b1370c:     	stur	x22, [x29, #-0xb0]
100b13710:     	sub	x9, x20, #0x1
100b13714:     	ands	x20, x9, x20
100b13718:     	b.eq	0x100b13858 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x298>
100b1371c:     	ldur	x9, [x29, #-0xc0]
100b13720:     	cmp	x22, x9
100b13724:     	b.ne	0x100b136fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x13c>
100b13728:     	sub	x0, x29, #0xc0
100b1372c:     	bl	0x1012692dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b13730:     	b	0x100b136f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x138>
100b13734:     	mov	x9, #0x0                ; =0
100b13738:     	sub	x19, x29, #0xf8
100b1373c:     	lsl	x10, x26, #2
100b13740:     	cmp	x10, x9
100b13744:     	b.eq	0x100b13d78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x7b8>
100b13748:     	ldr	w8, [x27, x9]
100b1374c:     	lsr	x11, x20, x8
100b13750:     	add	x9, x9, #0x4
100b13754:     	tbz	w11, #0x0, 0x100b13740 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x180>
100b13758:     	ldr	q0, [x22]
100b1375c:     	stur	q0, [x29, #-0xc0]
100b13760:     	ldr	x9, [x22, #0x10]
100b13764:     	stur	x9, [x29, #-0xb0]
100b13768:     	sub	x0, x29, #0xf8
100b1376c:     	sub	x1, x29, #0xc0
100b13770:     	mov	x2, x28
100b13774:     	mov	x20, x8
100b13778:     	mov	x3, x20
100b1377c:     	mov	w4, #0x0                ; =0
100b13780:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b13784:     	ldr	q0, [x19]
100b13788:     	ldur	x8, [x29, #-0xe8]
100b1378c:     	str	x8, [sp, #0x190]
100b13790:     	stur	q0, [x29, #-0xe0]
100b13794:     	stur	x8, [x29, #-0xd0]
100b13798:     	str	q0, [sp, #0xe0]
100b1379c:     	str	x8, [sp, #0xf0]
100b137a0:     	ldur	q0, [x22, #0x18]
100b137a4:     	stur	q0, [x29, #-0xc0]
100b137a8:     	ldur	x8, [x22, #0x28]
100b137ac:     	stur	x8, [x29, #-0xb0]
100b137b0:     	sub	x0, x29, #0xf8
100b137b4:     	sub	x1, x29, #0xc0
100b137b8:     	mov	x2, x28
100b137bc:     	mov	x3, x20
100b137c0:     	mov	w4, #0x0                ; =0
100b137c4:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b137c8:     	ldr	q0, [x19]
100b137cc:     	ldur	x8, [x29, #-0xe8]
100b137d0:     	str	x8, [sp, #0x190]
100b137d4:     	stur	q0, [x29, #-0xe0]
100b137d8:     	stur	x8, [x29, #-0xd0]
100b137dc:     	stur	q0, [sp, #0xf8]
100b137e0:     	str	x8, [sp, #0x108]
100b137e4:     	ldur	q0, [x22, #0x30]
100b137e8:     	stur	q0, [x29, #-0xc0]
100b137ec:     	ldur	x8, [x22, #0x40]
100b137f0:     	stur	x8, [x29, #-0xb0]
100b137f4:     	sub	x0, x29, #0xf8
100b137f8:     	sub	x1, x29, #0xc0
100b137fc:     	mov	x2, x28
100b13800:     	mov	x21, x20
100b13804:     	mov	x3, x20
100b13808:     	mov	w4, #0x0                ; =0
100b1380c:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b13810:     	ldr	q0, [x19]
100b13814:     	ldur	x8, [x29, #-0xe8]
100b13818:     	str	x8, [sp, #0x190]
100b1381c:     	stur	q0, [x29, #-0xe0]
100b13820:     	str	q0, [sp, #0x110]
100b13824:     	str	x8, [sp, #0x120]
100b13828:     	add	x3, sp, #0xe0
100b1382c:     	mov	x0, x28
100b13830:     	mov	x1, x27
100b13834:     	mov	x2, x26
100b13838:     	mov	x4, x24
100b1383c:     	mov	x5, x25
100b13840:     	bl	0x100b135c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_>
100b13844:     	and	w8, w0, #0xff
100b13848:     	cmp	w8, #0xf
100b1384c:     	b.ne	0x100b13868 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x2a8>
100b13850:     	mov	w27, #0xf               ; =15
100b13854:     	b	0x100b13d34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x774>
100b13858:     	ldp	x8, x26, [x29, #-0xc0]
100b1385c:     	cmp	x8, #0x0
100b13860:     	cset	w8, eq
100b13864:     	b	0x100b13980 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x3c0>
100b13868:     	mov	x23, x0
100b1386c:     	ldr	q0, [x22]
100b13870:     	stur	q0, [x29, #-0xc0]
100b13874:     	ldr	x8, [x22, #0x10]
100b13878:     	stur	x8, [x29, #-0xb0]
100b1387c:     	sub	x0, x29, #0xf8
100b13880:     	sub	x1, x29, #0xc0
100b13884:     	mov	x2, x28
100b13888:     	mov	x20, x21
100b1388c:     	mov	x3, x20
100b13890:     	mov	w4, #0x1                ; =1
100b13894:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b13898:     	ldr	q0, [x19]
100b1389c:     	str	q0, [sp, #0x1a0]
100b138a0:     	ldur	x8, [x29, #-0xe8]
100b138a4:     	str	q0, [sp, #0x180]
100b138a8:     	stur	q0, [x29, #-0xe0]
100b138ac:     	stur	x8, [x29, #-0xd0]
100b138b0:     	ldur	q0, [x29, #-0xe0]
100b138b4:     	str	x8, [sp, #0x140]
100b138b8:     	str	q0, [sp, #0x130]
100b138bc:     	ldur	q0, [x22, #0x18]
100b138c0:     	stur	q0, [x29, #-0xc0]
100b138c4:     	ldur	x8, [x22, #0x28]
100b138c8:     	stur	x8, [x29, #-0xb0]
100b138cc:     	sub	x0, x29, #0xf8
100b138d0:     	sub	x1, x29, #0xc0
100b138d4:     	mov	x2, x28
100b138d8:     	mov	x3, x20
100b138dc:     	mov	w4, #0x1                ; =1
100b138e0:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b138e4:     	ldr	q0, [x19]
100b138e8:     	str	q0, [sp, #0x1a0]
100b138ec:     	ldur	x8, [x29, #-0xe8]
100b138f0:     	str	q0, [sp, #0x180]
100b138f4:     	stur	q0, [x29, #-0xe0]
100b138f8:     	stur	x8, [x29, #-0xd0]
100b138fc:     	ldur	q0, [x29, #-0xe0]
100b13900:     	str	x8, [sp, #0x158]
100b13904:     	add	x8, sp, #0x49
100b13908:     	stur	q0, [x8, #0xff]
100b1390c:     	ldur	q0, [x22, #0x30]
100b13910:     	stur	q0, [x29, #-0xc0]
100b13914:     	ldur	x8, [x22, #0x40]
100b13918:     	stur	x8, [x29, #-0xb0]
100b1391c:     	sub	x0, x29, #0xf8
100b13920:     	sub	x1, x29, #0xc0
100b13924:     	mov	x2, x28
100b13928:     	mov	x3, x20
100b1392c:     	mov	w4, #0x1                ; =1
100b13930:     	bl	0x100002f00 <__RINvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB3_10Restricted8cofactorKm1_EB7_>
100b13934:     	ldr	q0, [x19]
100b13938:     	str	q0, [sp, #0x1a0]
100b1393c:     	ldur	x8, [x29, #-0xe8]
100b13940:     	str	q0, [sp, #0x180]
100b13944:     	stur	q0, [x29, #-0xe0]
100b13948:     	stur	x8, [x29, #-0xd0]
100b1394c:     	ldur	q0, [x29, #-0xe0]
100b13950:     	str	x8, [sp, #0x170]
100b13954:     	str	q0, [sp, #0x160]
100b13958:     	add	x3, sp, #0x130
100b1395c:     	mov	x0, x28
100b13960:     	mov	x1, x27
100b13964:     	mov	x2, x26
100b13968:     	mov	x4, x24
100b1396c:     	mov	x5, x25
100b13970:     	bl	0x100b135c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_>
100b13974:     	orr	w27, w0, w23
100b13978:     	b	0x100b13d34 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x774>
100b1397c:     	mov	w8, #0x1                ; =1
100b13980:     	str	w8, [sp, #0x14]
100b13984:     	mov	w27, #0x0               ; =0
100b13988:     	mov	x21, #0x0               ; =0
100b1398c:     	ldr	x8, [sp, #0x18]
100b13990:     	ldr	x10, [x8, #0x8]
100b13994:     	ldr	x9, [x8, #0x20]
100b13998:     	stp	x9, x10, [sp, #0xa8]
100b1399c:     	ldr	x8, [x8, #0x38]
100b139a0:     	str	x8, [sp, #0xa0]
100b139a4:     	str	x25, [sp, #0xb8]
100b139a8:     	ldr	x8, [x25, #0x8]
100b139ac:     	str	x8, [sp, #0xd8]
100b139b0:     	and	x8, x22, #0xfffffffffffffffe
100b139b4:     	neg	x8, x8
100b139b8:     	str	x8, [sp, #0x88]
100b139bc:     	mov	w23, #0x2               ; =2
100b139c0:     	adrp	x8, 0x101301000 <dyld_stub_binder+0x101301000>
100b139c4:     	ldr	q0, [x8]
100b139c8:     	str	q0, [sp, #0x90]
100b139cc:     	mov	w8, #0x4                ; =4
100b139d0:     	dup.2d	v1, x8
100b139d4:     	mov	w8, #0x8                ; =8
100b139d8:     	dup.2d	v0, x8
100b139dc:     	stp	q0, q1, [sp, #0x50]
100b139e0:     	mov	w8, #0xc                ; =12
100b139e4:     	dup.2d	v1, x8
100b139e8:     	mov	w8, #0x10               ; =16
100b139ec:     	dup.2d	v0, x8
100b139f0:     	stp	q0, q1, [sp, #0x30]
100b139f4:     	adrp	x8, 0x101301000 <dyld_stub_binder+0x101301000>
100b139f8:     	ldr	q0, [x8, #0x20]
100b139fc:     	str	q0, [sp, #0x20]
100b13a00:     	mov	w8, #0x3f               ; =63
100b13a04:     	dup.2d	v0, x8
100b13a08:     	str	q0, [sp, #0x70]
100b13a0c:     	movi.2s	v8, #0x3f
100b13a10:     	b	0x100b13a24 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x464>
100b13a14:     	add	x21, x21, #0x1
100b13a18:     	and	x8, x22, #0x3f
100b13a1c:     	lsr	x8, x21, x8
100b13a20:     	cbnz	x8, 0x100b13d1c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x75c>
100b13a24:     	ldr	x9, [sp, #0xd8]
100b13a28:     	add	x9, x9, #0x1
100b13a2c:     	ldr	x8, [sp, #0xb8]
100b13a30:     	str	x9, [sp, #0xd8]
100b13a34:     	str	x9, [x8, #0x8]
100b13a38:     	mov	x25, x22
100b13a3c:     	cbz	x22, 0x100b13cb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6f0>
100b13a40:     	cmp	x22, #0x1
100b13a44:     	b.ne	0x100b13a54 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x494>
100b13a48:     	mov	x8, #0x0                ; =0
100b13a4c:     	mov	x25, #0x0               ; =0
100b13a50:     	b	0x100b13c90 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6d0>
100b13a54:     	dup.2d	v0, x21
100b13a58:     	cmp	x22, #0x10
100b13a5c:     	b.hs	0x100b13a6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x4ac>
100b13a60:     	mov	x9, #0x0                ; =0
100b13a64:     	mov	x25, #0x0               ; =0
100b13a68:     	b	0x100b13c1c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x65c>
100b13a6c:     	movi.2d	v1, #0000000000000000
100b13a70:     	add	x8, x26, #0x20
100b13a74:     	movi.2d	v2, #0000000000000000
100b13a78:     	and	x9, x22, #0x1ffffffffffffff0
100b13a7c:     	ldr	q4, [sp, #0x90]
100b13a80:     	ldp	q6, q15, [sp, #0x20]
100b13a84:     	movi.2d	v3, #0000000000000000
100b13a88:     	movi.2d	v7, #0000000000000000
100b13a8c:     	movi.2d	v16, #0000000000000000
100b13a90:     	movi.2d	v5, #0000000000000000
100b13a94:     	movi.2d	v18, #0000000000000000
100b13a98:     	movi.2d	v17, #0000000000000000
100b13a9c:     	ldp	q13, q12, [sp, #0x50]
100b13aa0:     	ldr	q14, [sp, #0x40]
100b13aa4:     	mov	w10, #0x3f              ; =63
100b13aa8:     	movi.4s	v8, #0x3f
100b13aac:     	add.2d	v19, v4, v12
100b13ab0:     	add.2d	v20, v6, v12
100b13ab4:     	add.2d	v21, v4, v13
100b13ab8:     	add.2d	v22, v6, v13
100b13abc:     	add.2d	v23, v4, v14
100b13ac0:     	add.2d	v24, v6, v14
100b13ac4:     	ldp	q25, q26, [x8, #-0x20]
100b13ac8:     	dup.2d	v27, x10
100b13acc:     	ldp	q28, q29, [x8], #0x40
100b13ad0:     	and.16b	v30, v6, v27
100b13ad4:     	and.16b	v31, v4, v27
100b13ad8:     	and.16b	v20, v20, v27
100b13adc:     	and.16b	v19, v19, v27
100b13ae0:     	and.16b	v22, v22, v27
100b13ae4:     	and.16b	v21, v21, v27
100b13ae8:     	and.16b	v24, v24, v27
100b13aec:     	and.16b	v23, v23, v27
100b13af0:     	neg.2d	v27, v31
100b13af4:     	ushl.2d	v27, v0, v27
100b13af8:     	neg.2d	v30, v30
100b13afc:     	ushl.2d	v30, v0, v30
100b13b00:     	neg.2d	v19, v19
100b13b04:     	ushl.2d	v19, v0, v19
100b13b08:     	neg.2d	v20, v20
100b13b0c:     	ushl.2d	v20, v0, v20
100b13b10:     	neg.2d	v21, v21
100b13b14:     	ushl.2d	v21, v0, v21
100b13b18:     	neg.2d	v22, v22
100b13b1c:     	ushl.2d	v22, v0, v22
100b13b20:     	neg.2d	v23, v23
100b13b24:     	ushl.2d	v23, v0, v23
100b13b28:     	neg.2d	v24, v24
100b13b2c:     	ushl.2d	v24, v0, v24
100b13b30:     	dup.2d	v31, x24
100b13b34:     	and.16b	v30, v30, v31
100b13b38:     	and.16b	v27, v27, v31
100b13b3c:     	and.16b	v20, v20, v31
100b13b40:     	and.16b	v19, v19, v31
100b13b44:     	and.16b	v22, v22, v31
100b13b48:     	and.16b	v21, v21, v31
100b13b4c:     	and.16b	v24, v24, v31
100b13b50:     	and.16b	v23, v23, v31
100b13b54:     	and.16b	v25, v25, v8
100b13b58:     	and.16b	v26, v26, v8
100b13b5c:     	and.16b	v28, v28, v8
100b13b60:     	and.16b	v29, v29, v8
100b13b64:     	ushll2.2d	v31, v25, #0x0
100b13b68:     	ushll.2d	v25, v25, #0x0
100b13b6c:     	ushll2.2d	v9, v26, #0x0
100b13b70:     	ushll.2d	v26, v26, #0x0
100b13b74:     	ushll2.2d	v10, v28, #0x0
100b13b78:     	ushll.2d	v28, v28, #0x0
100b13b7c:     	ushll2.2d	v11, v29, #0x0
100b13b80:     	ushll.2d	v29, v29, #0x0
100b13b84:     	ushl.2d	v25, v27, v25
100b13b88:     	ushl.2d	v27, v30, v31
100b13b8c:     	ushl.2d	v19, v19, v26
100b13b90:     	ushl.2d	v20, v20, v9
100b13b94:     	ushl.2d	v21, v21, v28
100b13b98:     	ushl.2d	v22, v22, v10
100b13b9c:     	ushl.2d	v23, v23, v29
100b13ba0:     	ushl.2d	v24, v24, v11
100b13ba4:     	orr.16b	v3, v27, v3
100b13ba8:     	orr.16b	v2, v25, v2
100b13bac:     	orr.16b	v16, v20, v16
100b13bb0:     	orr.16b	v7, v19, v7
100b13bb4:     	orr.16b	v18, v22, v18
100b13bb8:     	orr.16b	v5, v21, v5
100b13bbc:     	orr.16b	v1, v24, v1
100b13bc0:     	orr.16b	v17, v23, v17
100b13bc4:     	add.2d	v6, v6, v15
100b13bc8:     	add.2d	v4, v4, v15
100b13bcc:     	subs	x9, x9, #0x10
100b13bd0:     	b.ne	0x100b13aac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x4ec>
100b13bd4:     	orr.16b	v2, v7, v2
100b13bd8:     	orr.16b	v3, v16, v3
100b13bdc:     	orr.16b	v3, v18, v3
100b13be0:     	orr.16b	v2, v5, v2
100b13be4:     	orr.16b	v2, v17, v2
100b13be8:     	orr.16b	v1, v1, v3
100b13bec:     	orr.16b	v1, v2, v1
100b13bf0:     	mov	d2, v1[1]
100b13bf4:     	orr.8b	v1, v1, v2
100b13bf8:     	fmov	x25, d1
100b13bfc:     	and	x8, x22, #0x1ffffffffffffff0
100b13c00:     	cmp	x22, x8
100b13c04:     	movi.2s	v8, #0x3f
100b13c08:     	b.eq	0x100b13cb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6f0>
100b13c0c:     	and	x9, x22, #0x1ffffffffffffff0
100b13c10:     	and	x8, x22, #0x1ffffffffffffff0
100b13c14:     	and	x10, x22, #0xe
100b13c18:     	cbz	x10, 0x100b13c90 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6d0>
100b13c1c:     	fmov	d1, x25
100b13c20:     	dup.2d	v2, x9
100b13c24:     	ldr	q3, [sp, #0x90]
100b13c28:     	orr.16b	v2, v2, v3
100b13c2c:     	ldr	x8, [sp, #0x88]
100b13c30:     	add	x8, x8, x9
100b13c34:     	add	x9, x26, x9, lsl #2
100b13c38:     	ldr	q6, [sp, #0x70]
100b13c3c:     	ldr	d3, [x9], #0x8
100b13c40:     	and.16b	v4, v2, v6
100b13c44:     	neg.2d	v4, v4
100b13c48:     	ushl.2d	v4, v0, v4
100b13c4c:     	dup.2d	v5, x24
100b13c50:     	and.16b	v4, v4, v5
100b13c54:     	and.8b	v3, v3, v8
100b13c58:     	ushll.2d	v3, v3, #0x0
100b13c5c:     	ushl.2d	v3, v4, v3
100b13c60:     	orr.16b	v1, v3, v1
100b13c64:     	dup.2d	v3, x23
100b13c68:     	add.2d	v2, v2, v3
100b13c6c:     	adds	x8, x8, #0x2
100b13c70:     	b.ne	0x100b13c3c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x67c>
100b13c74:     	mov	d0, v1[1]
100b13c78:     	orr.8b	v0, v1, v0
100b13c7c:     	fmov	x25, d0
100b13c80:     	and	x8, x22, #0x1ffffffffffffffe
100b13c84:     	and	x9, x22, #0x1ffffffffffffffe
100b13c88:     	cmp	x22, x9
100b13c8c:     	b.eq	0x100b13cb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6f0>
100b13c90:     	ldr	w9, [x26, x8, lsl #2]
100b13c94:     	lsr	x10, x21, x8
100b13c98:     	and	x10, x10, #0x1
100b13c9c:     	lsl	x9, x10, x9
100b13ca0:     	orr	x25, x9, x25
100b13ca4:     	add	x8, x8, #0x1
100b13ca8:     	cmp	x22, x8
100b13cac:     	b.ne	0x100b13c90 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x6d0>
100b13cb0:     	ldr	x8, [sp, #0xb0]
100b13cb4:     	orr	x2, x8, x25
100b13cb8:     	mov	x0, x28
100b13cbc:     	ldr	x1, [sp, #0xd0]
100b13cc0:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b13cc4:     	mov	x19, x0
100b13cc8:     	ldr	x8, [sp, #0xa8]
100b13ccc:     	orr	x2, x8, x25
100b13cd0:     	mov	x0, x28
100b13cd4:     	ldr	x1, [sp, #0xc8]
100b13cd8:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b13cdc:     	mov	x20, x0
100b13ce0:     	ldr	x8, [sp, #0xa0]
100b13ce4:     	orr	x2, x8, x25
100b13ce8:     	mov	x0, x28
100b13cec:     	ldr	x1, [sp, #0xc0]
100b13cf0:     	bl	0x100b7e918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b13cf4:     	tbz	w19, #0x0, 0x100b13a14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x454>
100b13cf8:     	cmp	w20, #0x0
100b13cfc:     	csel	w8, w23, wzr, ne
100b13d00:     	orr	w8, w8, w0
100b13d04:     	lsl	w8, w24, w8
100b13d08:     	orr	w27, w8, w27
100b13d0c:     	and	w8, w27, #0xff
100b13d10:     	cmp	w8, #0xf
100b13d14:     	b.ne	0x100b13a14 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x454>
100b13d18:     	mov	w27, #0xf               ; =15
100b13d1c:     	ldr	w8, [sp, #0x14]
100b13d20:     	tbnz	w8, #0x0, 0x100b13d2c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x76c>
100b13d24:     	mov	x0, x26
100b13d28:     	bl	0x1012708f8 <dyld_stub_binder+0x1012708f8>
100b13d2c:     	ldr	x22, [sp, #0x18]
100b13d30:     	ldr	x24, [sp, #0x8]
100b13d34:     	mov	x0, x24
100b13d38:     	mov	x1, x22
100b13d3c:     	mov	x2, x27
100b13d40:     	bl	0x100c169ec <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b13d44:     	mov	x0, x27
100b13d48:     	add	sp, sp, #0x220
100b13d4c:     	ldp	x29, x30, [sp, #0x90]
100b13d50:     	ldp	x20, x19, [sp, #0x80]
100b13d54:     	ldp	x22, x21, [sp, #0x70]
100b13d58:     	ldp	x24, x23, [sp, #0x60]
100b13d5c:     	ldp	x26, x25, [sp, #0x50]
100b13d60:     	ldp	x28, x27, [sp, #0x40]
100b13d64:     	ldp	d9, d8, [sp, #0x30]
100b13d68:     	ldp	d11, d10, [sp, #0x20]
100b13d6c:     	ldp	d13, d12, [sp, #0x10]
100b13d70:     	ldp	d15, d14, [sp], #0xa0
100b13d74:     	ret
100b13d78:     	adrp	x0, 0x1014a4000 <dyld_stub_binder+0x1014a4000>
100b13d7c:     	add	x0, x0, #0x858
100b13d80:     	bl	0x101268574 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b13d84:     	mov	x0, x9
100b13d88:     	adrp	x2, 0x10149c000 <dyld_stub_binder+0x10149c000>
100b13d8c:     	add	x2, x2, #0x18
100b13d90:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b13d94:     	mov	x0, x8
100b13d98:     	adrp	x2, 0x10149c000 <dyld_stub_binder+0x10149c000>
100b13d9c:     	add	x2, x2, #0x18
100b13da0:     	bl	0x1012684dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b13da4:     	mov	x19, x0
100b13da8:     	ldur	x8, [x29, #-0xc0]
100b13dac:     	cbz	x8, 0x100b13dcc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x80c>
100b13db0:     	ldur	x26, [x29, #-0xb8]
100b13db4:     	b	0x100b13dc4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x804>
100b13db8:     	mov	x19, x0
100b13dbc:     	ldr	w8, [sp, #0x14]
100b13dc0:     	tbnz	w8, #0x0, 0x100b13dcc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy8classify5visitKm9_EB8_+0x80c>
100b13dc4:     	mov	x0, x26
100b13dc8:     	bl	0x1012708f8 <dyld_stub_binder+0x1012708f8>
100b13dcc:     	mov	x0, x19
100b13dd0:     	bl	0x101270748 <dyld_stub_binder+0x101270748>
