
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010099570c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>:
10099570c:     	stp	x28, x27, [sp, #-0x60]!
100995710:     	stp	x26, x25, [sp, #0x10]
100995714:     	stp	x24, x23, [sp, #0x20]
100995718:     	stp	x22, x21, [sp, #0x30]
10099571c:     	stp	x20, x19, [sp, #0x40]
100995720:     	stp	x29, x30, [sp, #0x50]
100995724:     	add	x29, sp, #0x50
100995728:     	sub	sp, sp, #0x1c0
10099572c:     	mov	x23, x2
100995730:     	mov	x21, x1
100995734:     	mov	x28, x0
100995738:     	ldrb	w8, [x0, #0x151]
10099573c:     	str	x0, [sp, #0x88]
100995740:     	str	x2, [sp, #0x60]
100995744:     	cbz	w8, 0x100995a6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
100995748:     	mov	x27, #0x0               ; =0
10099574c:     	b	0x100995768 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
100995750:     	ldr	w9, [x26, #0x14]
100995754:     	add	x27, x27, #0x18
100995758:     	stp	xzr, x20, [x26]
10099575c:     	stp	w24, w9, [x26, #0x10]
100995760:     	cmp	x27, #0x30
100995764:     	b.eq	0x100995a6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
100995768:     	add	x26, x23, x27
10099576c:     	ldp	x19, x20, [x26]
100995770:     	ldr	w24, [x26, #0x10]
100995774:     	cbz	x19, 0x100995750 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x44>
100995778:     	ldr	x8, [x28, #0x138]
10099577c:     	add	x8, x8, #0x1
100995780:     	str	x8, [x28, #0x138]
100995784:     	ldur	x8, [x21, #0x40]
100995788:     	lsr	x0, x24, #1
10099578c:     	cmn	x8, #0x1
100995790:     	str	w9, [sp, #0x70]
100995794:     	b.eq	0x1009957b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa4>
100995798:     	ldr	x1, [x21, #0x50]
10099579c:     	cmp	x1, x0
1009957a0:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1009957a4:     	ldr	x8, [x21, #0x48]
1009957a8:     	add	x8, x8, x0, lsl #4
1009957ac:     	b	0x1009957c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbc>
1009957b0:     	ldr	x1, [x21, #0x58]
1009957b4:     	cmp	x1, x0
1009957b8:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1009957bc:     	ldr	x8, [x21, #0x50]
1009957c0:     	add	x8, x8, x0, lsl #5
1009957c4:     	add	x8, x8, #0x18
1009957c8:     	mov	x25, #0x0               ; =0
1009957cc:     	ldr	x8, [x8]
1009957d0:     	bic	x8, x8, x19
1009957d4:     	str	x8, [sp, #0x78]
1009957d8:     	mov	w8, #0x4                ; =4
1009957dc:     	stp	xzr, x8, [sp, #0xf0]
1009957e0:     	str	xzr, [sp, #0x100]
1009957e4:     	mov	w9, #0x4                ; =4
1009957e8:     	mov	w8, #0x4                ; =4
1009957ec:     	b	0x100995814 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x108>
1009957f0:     	rbit	x9, x19
1009957f4:     	clz	x9, x9
1009957f8:     	str	w9, [x8, x25, lsl #2]
1009957fc:     	add	x25, x25, #0x1
100995800:     	str	x25, [sp, #0x100]
100995804:     	sub	x10, x19, #0x1
100995808:     	add	x9, x23, #0x4
10099580c:     	ands	x19, x10, x19
100995810:     	b.eq	0x100995834 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x128>
100995814:     	mov	x23, x9
100995818:     	ldr	x9, [sp, #0xf0]
10099581c:     	cmp	x25, x9
100995820:     	b.ne	0x1009957f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
100995824:     	add	x0, sp, #0xf0
100995828:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10099582c:     	ldr	x8, [sp, #0xf8]
100995830:     	b	0x1009957f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
100995834:     	ldp	x9, x8, [sp, #0xf0]
100995838:     	str	x9, [sp, #0x80]
10099583c:     	str	x8, [sp, #0x68]
100995840:     	cbz	x25, 0x1009959b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
100995844:     	ldr	x19, [x28, #0x140]
100995848:     	mov	x28, x8
10099584c:     	b	0x100995884 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x178>
100995850:     	tst	w22, #0x1
100995854:     	mov	w8, #0x8                ; =8
100995858:     	mov	w9, #0xc                ; =12
10099585c:     	csel	x8, x9, x8, ne
100995860:     	add	x9, sp, #0xf0
100995864:     	ldr	w8, [x9, x8]
100995868:     	and	w9, w24, #0x1
10099586c:     	eor	w24, w8, w9
100995870:     	add	x19, x19, #0x1
100995874:     	ldr	x8, [sp, #0x88]
100995878:     	str	x19, [x8, #0x140]
10099587c:     	subs	x23, x23, #0x4
100995880:     	b.eq	0x1009959b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
100995884:     	ldr	w25, [x28], #0x4
100995888:     	ldur	x8, [x21, #0x40]
10099588c:     	lsr	w0, w24, #1
100995890:     	cmn	x8, #0x1
100995894:     	b.eq	0x1009958c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1b8>
100995898:     	ldr	x1, [x21, #0x50]
10099589c:     	cmp	x1, x0
1009958a0:     	b.ls	0x100996a6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1360>
1009958a4:     	ldr	x9, [x21, #0x48]
1009958a8:     	add	x9, x9, x0, lsl #4
1009958ac:     	ldr	x10, [x9]
1009958b0:     	mov	w9, #0x1                ; =1
1009958b4:     	lsl	x9, x9, x25
1009958b8:     	tst	x10, x9
1009958bc:     	b.ne	0x1009958ec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1e0>
1009958c0:     	b	0x10099587c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1009958c4:     	ldr	x1, [x21, #0x58]
1009958c8:     	cmp	x1, x0
1009958cc:     	b.ls	0x100996a88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x137c>
1009958d0:     	ldr	x1, [x21, #0x50]
1009958d4:     	add	x9, x1, x0, lsl #5
1009958d8:     	ldr	x10, [x9, #0x18]!
1009958dc:     	mov	w9, #0x1                ; =1
1009958e0:     	lsl	x9, x9, x25
1009958e4:     	tst	x10, x9
1009958e8:     	b.eq	0x10099587c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1009958ec:     	ldr	w10, [x21, #0xf0]
1009958f0:     	cmp	w25, w10
1009958f4:     	b.hs	0x100995c3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x530>
1009958f8:     	cmn	x8, #0x1
1009958fc:     	b.eq	0x100995920 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x214>
100995900:     	cmp	x1, x0
100995904:     	b.ls	0x100996a78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x136c>
100995908:     	ldr	x8, [x21, #0x48]
10099590c:     	add	x8, x8, x0, lsl #4
100995910:     	ldr	x8, [x8]
100995914:     	tst	x8, x9
100995918:     	b.ne	0x100995940 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x234>
10099591c:     	b	0x100995870 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
100995920:     	ldr	x8, [x21, #0x58]
100995924:     	cmp	x8, x0
100995928:     	b.ls	0x100996ab4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13a8>
10099592c:     	add	x8, x1, x0, lsl #5
100995930:     	add	x8, x8, #0x18
100995934:     	ldr	x8, [x8]
100995938:     	tst	x8, x9
10099593c:     	b.eq	0x100995870 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
100995940:     	and	x8, x25, #0x3f
100995944:     	lsr	x22, x20, x8
100995948:     	ldrb	w8, [x21, #0xf5]
10099594c:     	tbz	w8, #0x0, 0x100995998 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x28c>
100995950:     	add	x0, sp, #0xf0
100995954:     	add	x1, x21, #0x40
100995958:     	mov	x2, x24
10099595c:     	bl	0x1010b8900 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100995960:     	ldr	w8, [sp, #0xf0]
100995964:     	cmp	w8, #0x2
100995968:     	b.ne	0x100995978 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x26c>
10099596c:     	ldr	w8, [sp, #0xf4]
100995970:     	cmp	w8, w25
100995974:     	b.eq	0x100995850 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x144>
100995978:     	and	w1, w24, #0xfffffffe
10099597c:     	and	w3, w22, #0x1
100995980:     	mov	x0, x21
100995984:     	mov	x2, x25
100995988:     	bl	0x100fb2880 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
10099598c:     	and	w8, w24, #0x1
100995990:     	eor	w24, w0, w8
100995994:     	b	0x100995870 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
100995998:     	and	w3, w22, #0x1
10099599c:     	mov	x0, x21
1009959a0:     	mov	x1, x24
1009959a4:     	mov	x2, x25
1009959a8:     	bl	0x100fb2880 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
1009959ac:     	mov	x24, x0
1009959b0:     	b	0x100995870 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1009959b4:     	ldr	x8, [sp, #0x80]
1009959b8:     	cbz	x8, 0x1009959c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2b8>
1009959bc:     	ldr	x0, [sp, #0x68]
1009959c0:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009959c4:     	ldur	x8, [x21, #0x40]
1009959c8:     	lsr	w0, w24, #1
1009959cc:     	cmn	x8, #0x1
1009959d0:     	ldr	x28, [sp, #0x88]
1009959d4:     	ldr	x23, [sp, #0x60]
1009959d8:     	ldr	x10, [sp, #0x78]
1009959dc:     	b.eq	0x100995a08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2fc>
1009959e0:     	ldr	x1, [x21, #0x50]
1009959e4:     	cmp	x1, x0
1009959e8:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1009959ec:     	ldr	x8, [x21, #0x48]
1009959f0:     	add	x8, x8, x0, lsl #4
1009959f4:     	ldr	x8, [x8]
1009959f8:     	bics	x9, x8, x10
1009959fc:     	str	x9, [sp, #0xf0]
100995a00:     	b.eq	0x100995a30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x324>
100995a04:     	b	0x100996a44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
100995a08:     	ldr	x1, [x21, #0x58]
100995a0c:     	cmp	x1, x0
100995a10:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
100995a14:     	ldr	x8, [x21, #0x50]
100995a18:     	add	x8, x8, x0, lsl #5
100995a1c:     	add	x8, x8, #0x18
100995a20:     	ldr	x8, [x8]
100995a24:     	bics	x9, x8, x10
100995a28:     	str	x9, [sp, #0xf0]
100995a2c:     	b.ne	0x100996a44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
100995a30:     	mov	x20, #0x0               ; =0
100995a34:     	bic	x8, x10, x8
100995a38:     	fmov	d0, x8
100995a3c:     	cnt.8b	v0, v0
100995a40:     	addv.8b	b0, v0
100995a44:     	fmov	x8, d0
100995a48:     	ldr	x9, [x28, #0x148]
100995a4c:     	add	x8, x9, x8
100995a50:     	str	x8, [x28, #0x148]
100995a54:     	ldr	w9, [sp, #0x70]
100995a58:     	add	x27, x27, #0x18
100995a5c:     	stp	xzr, x20, [x26]
100995a60:     	stp	w24, w9, [x26, #0x10]
100995a64:     	cmp	x27, #0x30
100995a68:     	b.ne	0x100995768 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
100995a6c:     	ldr	w9, [x23, #0x10]
100995a70:     	cbz	w9, 0x1009964dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
100995a74:     	ldr	w10, [x23, #0x28]
100995a78:     	cbz	w10, 0x1009964dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
100995a7c:     	cmp	w9, #0x1
100995a80:     	ccmp	w10, #0x1, #0x0, eq
100995a84:     	b.eq	0x100995ba8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x49c>
100995a88:     	ldr	x8, [x28, #0x88]
100995a8c:     	cbz	x8, 0x100995bb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
100995a90:     	mov	x8, #0x0                ; =0
100995a94:     	mov	x15, #0xa9c5            ; =43461
100995a98:     	movk	x15, #0x2e62, lsl #16
100995a9c:     	movk	x15, #0x7aea, lsl #32
100995aa0:     	movk	x15, #0xf135, lsl #48
100995aa4:     	ldp	x11, x12, [x23]
100995aa8:     	madd	x13, x9, x15, x11
100995aac:     	mov	x14, #0x6332            ; =25394
100995ab0:     	movk	x14, #0x6ed3, lsl #16
100995ab4:     	movk	x14, #0x765a, lsl #32
100995ab8:     	movk	x14, #0x284f, lsl #48
100995abc:     	mul	x14, x14, x15
100995ac0:     	madd	x13, x13, x15, x14
100995ac4:     	add	x13, x13, x12
100995ac8:     	madd	x16, x13, x15, x10
100995acc:     	ldp	x13, x14, [x23, #0x18]
100995ad0:     	madd	x16, x16, x15, x13
100995ad4:     	madd	x16, x16, x15, x14
100995ad8:     	mul	x15, x16, x15
100995adc:     	ror	x0, x15, #0x2c
100995ae0:     	lsr	x17, x0, #57
100995ae4:     	ldp	x16, x15, [x28, #0x70]
100995ae8:     	dup.8b	v0, w17
100995aec:     	movi.2d	v1, #0xffffffffffffffff
100995af0:     	mov	w17, #0x38              ; =56
100995af4:     	and	x0, x0, x15
100995af8:     	ldr	d2, [x16, x0]
100995afc:     	cmeq.8b	v3, v2, v0
100995b00:     	fmov	x1, d3
100995b04:     	ands	x1, x1, #0x8080808080808080
100995b08:     	b.eq	0x100995b78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
100995b0c:     	rbit	x2, x1
100995b10:     	clz	x2, x2
100995b14:     	add	x2, x0, x2, lsr #3
100995b18:     	and	x2, x2, x15
100995b1c:     	mneg	x2, x2, x17
100995b20:     	add	x2, x16, x2
100995b24:     	ldur	x3, [x2, #-0x38]
100995b28:     	cmp	x11, x3
100995b2c:     	b.ne	0x100995b6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
100995b30:     	ldur	x3, [x2, #-0x30]
100995b34:     	cmp	x12, x3
100995b38:     	b.ne	0x100995b6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
100995b3c:     	ldur	w3, [x2, #-0x28]
100995b40:     	cmp	w9, w3
100995b44:     	b.ne	0x100995b6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
100995b48:     	ldur	x3, [x2, #-0x20]
100995b4c:     	cmp	x13, x3
100995b50:     	b.ne	0x100995b6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
100995b54:     	ldur	x3, [x2, #-0x18]
100995b58:     	cmp	x14, x3
100995b5c:     	b.ne	0x100995b6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
100995b60:     	ldur	w3, [x2, #-0x10]
100995b64:     	cmp	w10, w3
100995b68:     	b.eq	0x100995c28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x51c>
100995b6c:     	sub	x2, x1, #0x2
100995b70:     	ands	x1, x2, x1
100995b74:     	b.ne	0x100995b0c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
100995b78:     	cmeq.8b	v2, v2, v1
100995b7c:     	fmov	x1, d2
100995b80:     	cbnz	x1, 0x100995bb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
100995b84:     	add	x8, x8, #0x8
100995b88:     	add	x0, x0, x8
100995b8c:     	and	x0, x0, x15
100995b90:     	ldr	d2, [x16, x0]
100995b94:     	cmeq.8b	v3, v2, v0
100995b98:     	fmov	x1, d3
100995b9c:     	ands	x1, x1, #0x8080808080808080
100995ba0:     	b.ne	0x100995b0c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
100995ba4:     	b	0x100995b78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
100995ba8:     	mov	w0, #0x1                ; =1
100995bac:     	b	0x1009964e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
100995bb0:     	mov	x24, x28
100995bb4:     	ldr	x8, [x24, #0xc8]!
100995bb8:     	add	x8, x8, #0x1
100995bbc:     	str	x8, [x24]
100995bc0:     	mov	w11, #0x8481            ; =33921
100995bc4:     	movk	w11, #0x1e, lsl #16
100995bc8:     	cmp	x8, x11
100995bcc:     	b.hs	0x100996a9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1390>
100995bd0:     	ldr	x12, [x23]
100995bd4:     	ldr	x13, [x23, #0x18]
100995bd8:     	ldr	x8, [x21, #0x40]
100995bdc:     	cmn	x8, #0x1
100995be0:     	b.eq	0x100995c58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x54c>
100995be4:     	ldr	x1, [x21, #0x50]
100995be8:     	lsr	x0, x9, #1
100995bec:     	cmp	x1, x0
100995bf0:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
100995bf4:     	lsr	x8, x10, #1
100995bf8:     	cmp	x1, x8
100995bfc:     	b.ls	0x100996b50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1444>
100995c00:     	ldr	x11, [x21, #0x48]
100995c04:     	lsl	x14, x0, #4
100995c08:     	ldr	x14, [x11, x14]
100995c0c:     	bic	x19, x14, x12
100995c10:     	add	x8, x11, x8, lsl #4
100995c14:     	ldr	x15, [x8]
100995c18:     	ldp	x11, x1, [x28, #0x18]
100995c1c:     	mov	x22, #0x0               ; =0
100995c20:     	cbnz	x19, 0x100995c98 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x58c>
100995c24:     	b	0x100995cc8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
100995c28:     	ldur	w0, [x2, #-0x8]
100995c2c:     	ldr	x8, [x28, #0xd0]
100995c30:     	add	x8, x8, #0x1
100995c34:     	str	x8, [x28, #0xd0]
100995c38:     	b	0x1009964e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
100995c3c:     	adrp	x0, 0x1017ba000 <dyld_stub_binder+0x1017ba000>
100995c40:     	add	x0, x0, #0xe7
100995c44:     	adrp	x2, 0x10198a000 <dyld_stub_binder+0x10198a000>
100995c48:     	add	x2, x2, #0xc80
100995c4c:     	mov	w1, #0x2c               ; =44
100995c50:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100995c54:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100995c58:     	ldr	x1, [x21, #0x58]
100995c5c:     	lsr	x0, x9, #1
100995c60:     	cmp	x1, x0
100995c64:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
100995c68:     	lsr	x8, x10, #1
100995c6c:     	cmp	x1, x8
100995c70:     	b.ls	0x100996b84 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1478>
100995c74:     	ldr	x11, [x21, #0x50]
100995c78:     	add	x14, x11, x0, lsl #5
100995c7c:     	ldr	x14, [x14, #0x18]
100995c80:     	bic	x19, x14, x12
100995c84:     	add	x8, x11, x8, lsl #5
100995c88:     	ldr	x15, [x8, #0x18]!
100995c8c:     	ldp	x11, x1, [x28, #0x18]
100995c90:     	mov	x22, #0x0               ; =0
100995c94:     	cbz	x19, 0x100995cc8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
100995c98:     	mov	w8, #0x1                ; =1
100995c9c:     	mov	x14, x19
100995ca0:     	rbit	x16, x14
100995ca4:     	clz	x0, x16
100995ca8:     	cmp	x0, x1
100995cac:     	b.hs	0x100996ae4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13d8>
100995cb0:     	ldr	w16, [x11, x0, lsl #2]
100995cb4:     	lsl	x16, x8, x16
100995cb8:     	orr	x22, x16, x22
100995cbc:     	sub	x16, x14, #0x1
100995cc0:     	ands	x14, x16, x14
100995cc4:     	b.ne	0x100995ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x594>
100995cc8:     	ldp	x14, x8, [x28, #0x48]
100995ccc:     	bic	x15, x15, x13
100995cd0:     	cbz	x15, 0x100995d08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5fc>
100995cd4:     	mov	x16, #0x0               ; =0
100995cd8:     	mov	w17, #0x1               ; =1
100995cdc:     	rbit	x0, x15
100995ce0:     	clz	x0, x0
100995ce4:     	cmp	x0, x8
100995ce8:     	b.hs	0x100996af0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13e4>
100995cec:     	ldr	w0, [x14, x0, lsl #2]
100995cf0:     	lsl	x0, x17, x0
100995cf4:     	orr	x16, x0, x16
100995cf8:     	sub	x0, x15, #0x1
100995cfc:     	ands	x15, x0, x15
100995d00:     	b.ne	0x100995cdc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5d0>
100995d04:     	orr	x22, x16, x22
100995d08:     	eor	w9, w10, w9
100995d0c:     	cmp	x12, x13
100995d10:     	ccmp	w9, #0x1, #0x0, eq
100995d14:     	b.ne	0x100995df0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
100995d18:     	ldr	x9, [x23, #0x8]
100995d1c:     	ldr	x10, [x23, #0x20]
100995d20:     	cmp	x9, x10
100995d24:     	b.ne	0x100995df0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
100995d28:     	mov	w16, #0x4               ; =4
100995d2c:     	stp	xzr, x16, [sp, #0xf0]
100995d30:     	str	xzr, [sp, #0x100]
100995d34:     	mov	x20, #0x0               ; =0
100995d38:     	cbz	x19, 0x100995d98 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x68c>
100995d3c:     	mov	w8, #0x4                ; =4
100995d40:     	b	0x100995d68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x65c>
100995d44:     	ldr	x8, [sp, #0xf8]
100995d48:     	rbit	x9, x19
100995d4c:     	clz	x9, x9
100995d50:     	str	w9, [x8, x20, lsl #2]
100995d54:     	add	x20, x20, #0x1
100995d58:     	str	x20, [sp, #0x100]
100995d5c:     	sub	x9, x19, #0x1
100995d60:     	ands	x19, x9, x19
100995d64:     	b.eq	0x100995d80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x674>
100995d68:     	ldr	x9, [sp, #0xf0]
100995d6c:     	cmp	x20, x9
100995d70:     	b.ne	0x100995d48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x63c>
100995d74:     	add	x0, sp, #0xf0
100995d78:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100995d7c:     	b	0x100995d44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x638>
100995d80:     	ldp	x9, x16, [sp, #0xf0]
100995d84:     	ldp	x14, x8, [x28, #0x48]
100995d88:     	ldp	x11, x1, [x28, #0x18]
100995d8c:     	cmp	x9, #0x0
100995d90:     	cset	w19, eq
100995d94:     	b	0x100995d9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x690>
100995d98:     	mov	w19, #0x1               ; =1
100995d9c:     	mov	x9, #0x0                ; =0
100995da0:     	lsl	x10, x20, #2
100995da4:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100995da8:     	add	x2, x2, #0x568
100995dac:     	adrp	x12, 0x10194e000 <dyld_stub_binder+0x10194e000>
100995db0:     	add	x12, x12, #0x580
100995db4:     	cmp	x10, x9
100995db8:     	b.eq	0x1009964d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdcc>
100995dbc:     	ldr	w0, [x16, x9]
100995dc0:     	cmp	x1, x0
100995dc4:     	b.ls	0x100996b00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13f4>
100995dc8:     	cmp	x8, x0
100995dcc:     	b.ls	0x100996b08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13fc>
100995dd0:     	ldr	w13, [x11, x0, lsl #2]
100995dd4:     	ldr	w15, [x14, x0, lsl #2]
100995dd8:     	add	x9, x9, #0x4
100995ddc:     	cmp	w13, w15
100995de0:     	b.eq	0x100995db4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6a8>
100995de4:     	tbnz	w19, #0x0, 0x100995df0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
100995de8:     	mov	x0, x16
100995dec:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100995df0:     	fmov	d0, x22
100995df4:     	cnt.8b	v0, v0
100995df8:     	addv.8b	b0, v0
100995dfc:     	fmov	x19, d0
100995e00:     	cmp	x19, #0xa
100995e04:     	b.hs	0x1009961b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xaa8>
100995e08:     	add	x8, x28, #0x10
100995e0c:     	str	x8, [sp, #0x50]
100995e10:     	ldr	x8, [x28, #0xd8]
100995e14:     	add	x8, x8, #0x1
100995e18:     	str	x8, [x28, #0xd8]
100995e1c:     	ldr	w8, [x28]
100995e20:     	tbz	w8, #0x0, 0x100996214 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb08>
100995e24:     	str	x19, [sp, #0x8]
100995e28:     	mov	x26, #0x0               ; =0
100995e2c:     	ldr	x10, [x28, #0x8]
100995e30:     	add	x8, x28, #0x90
100995e34:     	str	x8, [sp, #0x48]
100995e38:     	lsl	x9, x10, #6
100995e3c:     	tst	x10, #0xfc00000000000000
100995e40:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
100995e44:     	ccmp	x9, x8, #0x2, eq
100995e48:     	cset	w8, hi
100995e4c:     	str	w8, [sp, #0x14]
100995e50:     	stp	x10, x24, [sp, #0x28]
100995e54:     	sub	x8, x10, #0x1
100995e58:     	stp	x9, x8, [sp, #0x18]
100995e5c:     	mov	w20, #0xff              ; =255
100995e60:     	mov	w8, #0x1                ; =1
100995e64:     	b	0x100995ecc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7c0>
100995e68:     	strb	w23, [x28]
100995e6c:     	strb	w10, [x28, #0x1]
100995e70:     	str	w25, [x28, #0x4]
100995e74:     	stp	x9, x27, [x28, #0x8]
100995e78:     	ldp	x8, x9, [sp, #0x70]
100995e7c:     	stp	x19, x9, [x28, #0x18]
100995e80:     	str	x8, [x28, #0x28]
100995e84:     	ldr	w8, [sp, #0x58]
100995e88:     	stp	w25, w8, [x28, #0x30]
100995e8c:     	str	x22, [x28, #0x38]
100995e90:     	ldr	x28, [sp, #0x88]
100995e94:     	ldr	x23, [sp, #0x60]
100995e98:     	ldr	w13, [sp, #0x80]
100995e9c:     	mov	w8, #0x0                ; =0
100995ea0:     	ldr	x9, [x28, #0x130]
100995ea4:     	ldp	x2, x10, [x28, #0x98]
100995ea8:     	add	x10, x10, x2, lsl #6
100995eac:     	ldp	x11, x12, [x28, #0xb0]
100995eb0:     	add	x10, x12, x10
100995eb4:     	add	x10, x10, x11, lsl #6
100995eb8:     	cmp	x10, x9
100995ebc:     	csel	x9, x10, x9, hi
100995ec0:     	str	x9, [x28, #0x130]
100995ec4:     	mov	w26, #0x1               ; =1
100995ec8:     	tbz	w13, #0x0, 0x1009966a8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf9c>
100995ecc:     	mov	x13, x8
100995ed0:     	add	x8, x26, x26, lsl #1
100995ed4:     	lsl	x8, x8, #3
100995ed8:     	add	x9, x23, x8
100995edc:     	ldr	q0, [x9]
100995ee0:     	str	q0, [sp, #0xc0]
100995ee4:     	ldr	x25, [x9, #0x10]
100995ee8:     	str	x25, [sp, #0xd0]
100995eec:     	cmp	w25, #0x2
100995ef0:     	b.lo	0x100995e9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x790>
100995ef4:     	str	w13, [sp, #0x80]
100995ef8:     	ldr	x9, [sp, #0x48]
100995efc:     	add	x24, x9, x8
100995f00:     	ldrb	w27, [x28, #0x150]
100995f04:     	ldr	x8, [x24, #0x8]
100995f08:     	cbz	x8, 0x100995f14 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x808>
100995f0c:     	ldr	x0, [x24]
100995f10:     	b	0x100995f98 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x88c>
100995f14:     	ldp	x9, x28, [sp, #0x20]
100995f18:     	eor	x8, x28, x9
100995f1c:     	cmp	x8, x9
100995f20:     	b.ls	0x100996acc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13c0>
100995f24:     	ldr	x19, [sp, #0x18]
100995f28:     	ldr	w8, [sp, #0x14]
100995f2c:     	cbnz	w8, 0x10099630c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc00>
100995f30:     	cbz	x19, 0x100995f48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x83c>
100995f34:     	mov	x0, x19
100995f38:     	mov	w1, #0x8                ; =8
100995f3c:     	bl	0x1015d9f98 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
100995f40:     	cbnz	x0, 0x100995f4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x840>
100995f44:     	b	0x100996ba4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1498>
100995f48:     	mov	w0, #0x8                ; =8
100995f4c:     	mov	x8, x0
100995f50:     	mov	x9, x28
100995f54:     	cmp	x28, #0x4
100995f58:     	b.hs	0x100995f6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x860>
100995f5c:     	strb	w20, [x8], #0x40
100995f60:     	subs	x9, x9, #0x1
100995f64:     	b.ne	0x100995f5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x850>
100995f68:     	b	0x100995f90 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x884>
100995f6c:     	add	x8, x0, #0x80
100995f70:     	and	x9, x28, #0x3fffffffffffffc
100995f74:     	sturb	w20, [x8, #-0x80]
100995f78:     	sturb	w20, [x8, #-0x40]
100995f7c:     	strb	w20, [x8]
100995f80:     	strb	w20, [x8, #0x40]
100995f84:     	add	x8, x8, #0x100
100995f88:     	subs	x9, x9, #0x4
100995f8c:     	b.ne	0x100995f74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x868>
100995f90:     	stp	x0, x28, [x24]
100995f94:     	mov	x8, x28
100995f98:     	mov	w9, w25
100995f9c:     	ldp	x11, x12, [sp, #0xc0]
100995fa0:     	ldr	w19, [sp, #0xd4]
100995fa4:     	mov	x10, #0xa9c5            ; =43461
100995fa8:     	movk	x10, #0x2e62, lsl #16
100995fac:     	movk	x10, #0x7aea, lsl #32
100995fb0:     	movk	x10, #0xf135, lsl #48
100995fb4:     	stp	x12, x11, [sp, #0x70]
100995fb8:     	madd	x9, x9, x10, x11
100995fbc:     	madd	x9, x9, x10, x12
100995fc0:     	madd	x9, x9, x10, x22
100995fc4:     	mul	x9, x9, x10
100995fc8:     	sub	x8, x8, #0x1
100995fcc:     	and	x8, x8, x9, ror #44
100995fd0:     	add	x28, x0, x8, lsl #6
100995fd4:     	ldrb	w8, [x28]
100995fd8:     	cmp	w8, #0xff
100995fdc:     	b.ne	0x100996000 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x8f4>
100995fe0:     	ldr	x9, [sp, #0x88]
100995fe4:     	ldr	x8, [x9, #0x120]
100995fe8:     	add	x8, x8, #0x1
100995fec:     	str	x8, [x9, #0x120]
100995ff0:     	add	x8, x28, #0x10
100995ff4:     	str	x8, [sp, #0x38]
100995ff8:     	add	x23, x28, #0x18
100995ffc:     	b	0x1009960a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
100996000:     	ldr	x9, [x28, #0x38]
100996004:     	cmp	x9, x22
100996008:     	b.ne	0x10099604c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
10099600c:     	ldr	x9, [x28, #0x20]
100996010:     	ldr	x10, [sp, #0x78]
100996014:     	cmp	x9, x10
100996018:     	b.ne	0x10099604c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
10099601c:     	ldr	x9, [x28, #0x28]
100996020:     	ldr	x10, [sp, #0x70]
100996024:     	cmp	x9, x10
100996028:     	b.ne	0x10099604c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
10099602c:     	ldr	w9, [x28, #0x30]
100996030:     	cmp	w9, w25
100996034:     	b.ne	0x10099604c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
100996038:     	ldr	x28, [sp, #0x88]
10099603c:     	ldr	x8, [x28, #0x118]
100996040:     	add	x8, x8, #0x1
100996044:     	str	x8, [x28, #0x118]
100996048:     	b	0x100995e98 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x78c>
10099604c:     	mov	x23, x28
100996050:     	ldr	x9, [x23, #0x18]!
100996054:     	lsl	x10, x9, #3
100996058:     	cmp	w8, #0x2
10099605c:     	csel	x10, x10, xzr, eq
100996060:     	ldr	x11, [x24, #0x10]
100996064:     	sub	x10, x11, x10
100996068:     	cmp	w8, #0x2
10099606c:     	mov	x8, x28
100996070:     	ldr	x0, [x8, #0x10]!
100996074:     	str	x8, [sp, #0x38]
100996078:     	strb	w20, [x28]
10099607c:     	ldr	x8, [sp, #0x88]
100996080:     	ldr	q0, [x8, #0x120]
100996084:     	mov	w11, #0x1               ; =1
100996088:     	dup.2d	v1, x11
10099608c:     	add.2d	v0, v0, v1
100996090:     	str	q0, [x8, #0x120]
100996094:     	str	x10, [x24, #0x10]
100996098:     	ccmp	x9, #0x0, #0x4, hs
10099609c:     	b.eq	0x1009960a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
1009960a0:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009960a4:     	ldr	x8, [sp, #0x50]
1009960a8:     	mov	w9, #0x30               ; =48
1009960ac:     	madd	x3, x26, x9, x8
1009960b0:     	add	x0, sp, #0xf0
1009960b4:     	add	x2, sp, #0xc0
1009960b8:     	mov	x1, x21
1009960bc:     	mov	x4, x22
1009960c0:     	mov	x5, x27
1009960c4:     	ldr	x6, [sp, #0x30]
1009960c8:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1009960cc:     	ldr	x8, [sp, #0xf0]
1009960d0:     	cmn	x8, #0x1
1009960d4:     	str	w19, [sp, #0x58]
1009960d8:     	str	x23, [sp, #0x40]
1009960dc:     	b.eq	0x1009960f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9e8>
1009960e0:     	cmn	x8, #0x2
1009960e4:     	b.ne	0x100996118 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa0c>
1009960e8:     	mov	w23, #0x0               ; =0
1009960ec:     	ldrb	w10, [sp, #0xf8]
1009960f0:     	b	0x1009960f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9ec>
1009960f4:     	mov	w23, #0x1               ; =1
1009960f8:     	mov	x26, #0x0               ; =0
1009960fc:     	ldr	x8, [x24, #0x10]
100996100:     	add	x8, x8, x26
100996104:     	str	x8, [x24, #0x10]
100996108:     	ldrb	w8, [x28]
10099610c:     	cmp	w8, #0x2
100996110:     	b.ne	0x100995e68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
100996114:     	b	0x100996188 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa7c>
100996118:     	ldp	x27, x19, [sp, #0xf8]
10099611c:     	ldr	x9, [sp, #0x108]
100996120:     	lsl	x26, x19, #3
100996124:     	cmp	x8, x19
100996128:     	b.ls	0x10099616c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
10099612c:     	mov	x23, x9
100996130:     	str	x27, [sp, #0x68]
100996134:     	cbz	x19, 0x10099615c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa50>
100996138:     	lsl	x1, x8, #3
10099613c:     	ldr	x0, [sp, #0x68]
100996140:     	mov	w2, #0x8                ; =8
100996144:     	mov	x3, x26
100996148:     	bl	0x1015d9fec <__RNvCsiwXPDrQxTLA_7___rustc14___rust_realloc>
10099614c:     	mov	x27, x0
100996150:     	mov	x9, x23
100996154:     	cbnz	x0, 0x10099616c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
100996158:     	b	0x100996bb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14a4>
10099615c:     	ldr	x0, [sp, #0x68]
100996160:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996164:     	mov	w27, #0x8               ; =8
100996168:     	mov	x9, x23
10099616c:     	mov	w23, #0x2               ; =2
100996170:     	ldr	x8, [x24, #0x10]
100996174:     	add	x8, x8, x26
100996178:     	str	x8, [x24, #0x10]
10099617c:     	ldrb	w8, [x28]
100996180:     	cmp	w8, #0x2
100996184:     	b.ne	0x100995e68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
100996188:     	ldr	x8, [sp, #0x40]
10099618c:     	ldr	x8, [x8]
100996190:     	cbz	x8, 0x100995e68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
100996194:     	ldr	x8, [sp, #0x38]
100996198:     	ldr	x0, [x8]
10099619c:     	mov	x24, x9
1009961a0:     	mov	x26, x10
1009961a4:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009961a8:     	mov	x10, x26
1009961ac:     	mov	x9, x24
1009961b0:     	b	0x100995e68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1009961b4:     	mov	x0, x21
1009961b8:     	mov	x1, x22
1009961bc:     	bl	0x100fa3680 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
1009961c0:     	ldr	q0, [x23]
1009961c4:     	str	q0, [sp, #0xc0]
1009961c8:     	ldr	x8, [x23, #0x10]
1009961cc:     	str	x8, [sp, #0xd0]
1009961d0:     	mov	w22, w0
1009961d4:     	ldr	x1, [x28, #0x38]
1009961d8:     	cmp	x1, x22
1009961dc:     	b.ls	0x100996b40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
1009961e0:     	ldr	x8, [x28, #0x30]
1009961e4:     	ldr	w8, [x8, x22, lsl #2]
1009961e8:     	ldr	w9, [sp, #0xd0]
1009961ec:     	ldr	x10, [x21, #0x40]
1009961f0:     	lsr	x0, x9, #1
1009961f4:     	cmn	x10, #0x1
1009961f8:     	b.eq	0x100996310 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc04>
1009961fc:     	ldr	x1, [x21, #0x50]
100996200:     	cmp	x1, x0
100996204:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
100996208:     	ldr	x9, [x21, #0x48]
10099620c:     	add	x9, x9, x0, lsl #4
100996210:     	b	0x100996328 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc1c>
100996214:     	ldrb	w5, [x28, #0x150]
100996218:     	add	x0, sp, #0xc0
10099621c:     	add	x3, x28, #0x10
100996220:     	mov	x1, x21
100996224:     	mov	x2, x23
100996228:     	mov	x4, x22
10099622c:     	mov	x6, x24
100996230:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100996234:     	ldrb	w5, [x28, #0x150]
100996238:     	add	x0, sp, #0xf0
10099623c:     	add	x2, x23, #0x18
100996240:     	add	x3, x28, #0x40
100996244:     	mov	x1, x21
100996248:     	mov	x4, x22
10099624c:     	mov	x6, x24
100996250:     	bl	0x100ebf6e8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100996254:     	mov	w8, #0x1                ; =1
100996258:     	lsl	x8, x8, x19
10099625c:     	lsr	x8, x8, #6
100996260:     	cmp	x19, #0x6
100996264:     	cinc	x20, x8, lo
100996268:     	cbz	x20, 0x100996790 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
10099626c:     	lsl	x24, x20, #3
100996270:     	mov	x0, x24
100996274:     	mov	w1, #0x8                ; =8
100996278:     	bl	0x1015d9f98 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
10099627c:     	cbz	x0, 0x100996b94 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1488>
100996280:     	mov	x25, x0
100996284:     	mov	x0, #0x0                ; =0
100996288:     	ldp	x8, x9, [sp, #0xc0]
10099628c:     	ldp	x1, x10, [sp, #0xd0]
100996290:     	sub	x11, x0, w9, uxtb
100996294:     	ldp	x23, x13, [sp, #0xf0]
100996298:     	ldp	x12, x14, [sp, #0x100]
10099629c:     	mov	x24, x25
1009962a0:     	b	0x1009962bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
1009962a4:     	tst	w13, #0x1
1009962a8:     	csel	x15, x15, xzr, ne
1009962ac:     	str	x15, [x24, x0, lsl #3]
1009962b0:     	add	x0, x0, #0x1
1009962b4:     	cmp	x20, x0
1009962b8:     	b.eq	0x100996304 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1009962bc:     	mov	x15, x11
1009962c0:     	cmn	x8, #0x2
1009962c4:     	b.eq	0x1009962d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbcc>
1009962c8:     	cmp	x0, x1
1009962cc:     	b.hs	0x100996b20 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1414>
1009962d0:     	ldr	x15, [x9, x0, lsl #3]
1009962d4:     	eor	x15, x10, x15
1009962d8:     	cmn	x23, #0x2
1009962dc:     	b.eq	0x1009962a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb98>
1009962e0:     	cmp	x0, x12
1009962e4:     	b.hs	0x100996b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1410>
1009962e8:     	ldr	x16, [x13, x0, lsl #3]
1009962ec:     	eor	x16, x14, x16
1009962f0:     	and	x15, x16, x15
1009962f4:     	str	x15, [x24, x0, lsl #3]
1009962f8:     	add	x0, x0, #0x1
1009962fc:     	cmp	x20, x0
100996300:     	b.ne	0x1009962bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
100996304:     	mov	x27, x20
100996308:     	b	0x10099679c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1090>
10099630c:     	bl	0x1016e6d58 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
100996310:     	ldr	x1, [x21, #0x58]
100996314:     	cmp	x1, x0
100996318:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
10099631c:     	ldr	x9, [x21, #0x50]
100996320:     	add	x9, x9, x0, lsl #5
100996324:     	add	x9, x9, #0x18
100996328:     	ldr	x9, [x9]
10099632c:     	mov	w10, #0x1               ; =1
100996330:     	lsl	x8, x10, x8
100996334:     	tst	x9, x8
100996338:     	b.eq	0x10099634c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc40>
10099633c:     	ldp	x9, x10, [sp, #0xc0]
100996340:     	orr	x9, x9, x8
100996344:     	bic	x8, x10, x8
100996348:     	stp	x9, x8, [sp, #0xc0]
10099634c:     	sub	x0, x29, #0x70
100996350:     	add	x1, sp, #0xc0
100996354:     	mov	x2, x21
100996358:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
10099635c:     	ldur	q0, [x29, #-0x70]
100996360:     	stur	q0, [x29, #-0x90]
100996364:     	ldur	x8, [x29, #-0x60]
100996368:     	stur	q0, [x29, #-0xb0]
10099636c:     	str	q0, [sp, #0x90]
100996370:     	str	x8, [sp, #0xa0]
100996374:     	ldr	q0, [sp, #0x90]
100996378:     	str	x8, [sp, #0x100]
10099637c:     	str	q0, [sp, #0xf0]
100996380:     	ldur	q0, [x23, #0x18]
100996384:     	str	q0, [sp, #0xc0]
100996388:     	ldur	x8, [x23, #0x28]
10099638c:     	str	x8, [sp, #0xd0]
100996390:     	ldr	x1, [x28, #0x68]
100996394:     	cmp	x1, x22
100996398:     	b.ls	0x100996b40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
10099639c:     	ldr	x8, [x28, #0x60]
1009963a0:     	ldr	w8, [x8, x22, lsl #2]
1009963a4:     	ldr	w9, [sp, #0xd0]
1009963a8:     	ldr	x10, [x21, #0x40]
1009963ac:     	lsr	x0, x9, #1
1009963b0:     	cmn	x10, #0x1
1009963b4:     	b.eq	0x1009963d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcc4>
1009963b8:     	ldr	x1, [x21, #0x50]
1009963bc:     	cmp	x1, x0
1009963c0:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1009963c4:     	ldr	x9, [x21, #0x48]
1009963c8:     	add	x9, x9, x0, lsl #4
1009963cc:     	b	0x1009963e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcdc>
1009963d0:     	ldr	x1, [x21, #0x58]
1009963d4:     	cmp	x1, x0
1009963d8:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1009963dc:     	ldr	x9, [x21, #0x50]
1009963e0:     	add	x9, x9, x0, lsl #5
1009963e4:     	add	x9, x9, #0x18
1009963e8:     	ldr	x9, [x9]
1009963ec:     	mov	w10, #0x1               ; =1
1009963f0:     	lsl	x8, x10, x8
1009963f4:     	tst	x9, x8
1009963f8:     	b.eq	0x10099640c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd00>
1009963fc:     	ldp	x9, x10, [sp, #0xc0]
100996400:     	orr	x9, x9, x8
100996404:     	bic	x8, x10, x8
100996408:     	stp	x9, x8, [sp, #0xc0]
10099640c:     	sub	x0, x29, #0x70
100996410:     	add	x1, sp, #0xc0
100996414:     	mov	x2, x21
100996418:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
10099641c:     	ldur	q0, [x29, #-0x70]
100996420:     	stur	q0, [x29, #-0x90]
100996424:     	ldur	x8, [x29, #-0x60]
100996428:     	stur	q0, [x29, #-0xb0]
10099642c:     	str	q0, [sp, #0x90]
100996430:     	str	x8, [sp, #0xa0]
100996434:     	ldr	q0, [sp, #0x90]
100996438:     	str	x8, [sp, #0x118]
10099643c:     	add	x8, sp, #0x9
100996440:     	stur	q0, [x8, #0xff]
100996444:     	ldp	q0, q1, [sp, #0xf0]
100996448:     	ldr	q2, [sp, #0x110]
10099644c:     	stp	q1, q2, [sp, #0xa0]
100996450:     	str	q0, [sp, #0x90]
100996454:     	add	x2, sp, #0x90
100996458:     	mov	x0, x28
10099645c:     	mov	x1, x21
100996460:     	bl	0x10099570c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
100996464:     	mov	x23, x0
100996468:     	cmp	w0, #0x1
10099646c:     	b.ne	0x100996484 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
100996470:     	ldr	x8, [x28, #0xc0]
100996474:     	lsr	x8, x8, x22
100996478:     	tbz	w8, #0x0, 0x100996484 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
10099647c:     	mov	w8, #0x1                ; =1
100996480:     	b	0x100996690 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf84>
100996484:     	ldr	x8, [sp, #0x60]
100996488:     	ldr	q0, [x8]
10099648c:     	stur	q0, [x29, #-0x70]
100996490:     	ldr	x8, [x8, #0x10]
100996494:     	stur	x8, [x29, #-0x60]
100996498:     	ldr	x1, [x28, #0x38]
10099649c:     	cmp	x1, x22
1009964a0:     	b.ls	0x100996b74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
1009964a4:     	ldr	x8, [x28, #0x30]
1009964a8:     	ldr	w8, [x8, x22, lsl #2]
1009964ac:     	ldur	w9, [x29, #-0x60]
1009964b0:     	ldr	x10, [x21, #0x40]
1009964b4:     	lsr	x0, x9, #1
1009964b8:     	cmn	x10, #0x1
1009964bc:     	b.eq	0x100996500 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdf4>
1009964c0:     	ldr	x1, [x21, #0x50]
1009964c4:     	cmp	x1, x0
1009964c8:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1009964cc:     	ldr	x9, [x21, #0x48]
1009964d0:     	add	x9, x9, x0, lsl #4
1009964d4:     	b	0x100996518 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe0c>
1009964d8:     	tbz	w19, #0x0, 0x100996698 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf8c>
1009964dc:     	mov	w0, #0x0                ; =0
1009964e0:     	add	sp, sp, #0x1c0
1009964e4:     	ldp	x29, x30, [sp, #0x50]
1009964e8:     	ldp	x20, x19, [sp, #0x40]
1009964ec:     	ldp	x22, x21, [sp, #0x30]
1009964f0:     	ldp	x24, x23, [sp, #0x20]
1009964f4:     	ldp	x26, x25, [sp, #0x10]
1009964f8:     	ldp	x28, x27, [sp], #0x60
1009964fc:     	ret
100996500:     	ldr	x1, [x21, #0x58]
100996504:     	cmp	x1, x0
100996508:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
10099650c:     	ldr	x9, [x21, #0x50]
100996510:     	add	x9, x9, x0, lsl #5
100996514:     	add	x9, x9, #0x18
100996518:     	ldr	x9, [x9]
10099651c:     	mov	w10, #0x1               ; =1
100996520:     	lsl	x8, x10, x8
100996524:     	tst	x9, x8
100996528:     	b.eq	0x10099653c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe30>
10099652c:     	ldur	q0, [x29, #-0x70]
100996530:     	dup.2d	v1, x8
100996534:     	orr.16b	v0, v0, v1
100996538:     	stur	q0, [x29, #-0x70]
10099653c:     	sub	x0, x29, #0xb0
100996540:     	sub	x1, x29, #0x70
100996544:     	mov	x2, x21
100996548:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
10099654c:     	ldur	q0, [x29, #-0xb0]
100996550:     	stur	q0, [x29, #-0xd0]
100996554:     	ldur	x8, [x29, #-0xa0]
100996558:     	stur	q0, [x29, #-0xf0]
10099655c:     	stur	q0, [x29, #-0x90]
100996560:     	stur	x8, [x29, #-0x80]
100996564:     	ldur	q0, [x29, #-0x90]
100996568:     	str	x8, [sp, #0x100]
10099656c:     	str	q0, [sp, #0xf0]
100996570:     	ldr	x8, [sp, #0x60]
100996574:     	ldur	q0, [x8, #0x18]
100996578:     	stur	q0, [x29, #-0x70]
10099657c:     	ldur	x8, [x8, #0x28]
100996580:     	stur	x8, [x29, #-0x60]
100996584:     	ldr	x1, [x28, #0x68]
100996588:     	cmp	x1, x22
10099658c:     	b.ls	0x100996b74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
100996590:     	ldr	x8, [x28, #0x60]
100996594:     	ldr	w8, [x8, x22, lsl #2]
100996598:     	ldur	w9, [x29, #-0x60]
10099659c:     	ldr	x10, [x21, #0x40]
1009965a0:     	lsr	x0, x9, #1
1009965a4:     	cmn	x10, #0x1
1009965a8:     	b.eq	0x1009965c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xeb8>
1009965ac:     	ldr	x1, [x21, #0x50]
1009965b0:     	cmp	x1, x0
1009965b4:     	b.ls	0x100996b54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1009965b8:     	ldr	x9, [x21, #0x48]
1009965bc:     	add	x9, x9, x0, lsl #4
1009965c0:     	b	0x1009965dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xed0>
1009965c4:     	ldr	x1, [x21, #0x58]
1009965c8:     	cmp	x1, x0
1009965cc:     	b.ls	0x100996b88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1009965d0:     	ldr	x9, [x21, #0x50]
1009965d4:     	add	x9, x9, x0, lsl #5
1009965d8:     	add	x9, x9, #0x18
1009965dc:     	and	w19, w22, #0x3f
1009965e0:     	ldr	x9, [x9]
1009965e4:     	mov	w10, #0x1               ; =1
1009965e8:     	lsl	x8, x10, x8
1009965ec:     	tst	x9, x8
1009965f0:     	b.eq	0x100996604 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xef8>
1009965f4:     	ldur	q0, [x29, #-0x70]
1009965f8:     	dup.2d	v1, x8
1009965fc:     	orr.16b	v0, v0, v1
100996600:     	stur	q0, [x29, #-0x70]
100996604:     	sub	x0, x29, #0xb0
100996608:     	sub	x1, x29, #0x70
10099660c:     	mov	x2, x21
100996610:     	bl	0x1009319a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100996614:     	ldur	q0, [x29, #-0xb0]
100996618:     	stur	q0, [x29, #-0xd0]
10099661c:     	ldur	x8, [x29, #-0xa0]
100996620:     	stur	q0, [x29, #-0xf0]
100996624:     	stur	q0, [x29, #-0x90]
100996628:     	stur	x8, [x29, #-0x80]
10099662c:     	ldur	q0, [x29, #-0x90]
100996630:     	str	x8, [sp, #0x118]
100996634:     	add	x8, sp, #0x9
100996638:     	stur	q0, [x8, #0xff]
10099663c:     	ldp	q0, q1, [sp, #0xf0]
100996640:     	ldr	q2, [sp, #0x110]
100996644:     	stp	q1, q2, [sp, #0xd0]
100996648:     	str	q0, [sp, #0xc0]
10099664c:     	add	x2, sp, #0xc0
100996650:     	mov	x0, x28
100996654:     	mov	x1, x21
100996658:     	bl	0x10099570c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
10099665c:     	mov	x3, x0
100996660:     	ldr	x8, [x28, #0xc0]
100996664:     	mov	x0, x21
100996668:     	lsr	x8, x8, x19
10099666c:     	tbz	w8, #0x0, 0x100996680 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf74>
100996670:     	mov	w1, #0xe                ; =14
100996674:     	mov	x2, x23
100996678:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
10099667c:     	b	0x10099668c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf80>
100996680:     	mov	x1, x22
100996684:     	mov	x2, x23
100996688:     	bl	0x100fb36cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
10099668c:     	mov	x8, x0
100996690:     	ldr	x19, [sp, #0x60]
100996694:     	b	0x100996a28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x131c>
100996698:     	mov	x0, x16
10099669c:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009966a0:     	mov	w0, #0x0                ; =0
1009966a4:     	b	0x1009964e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1009966a8:     	ldr	x1, [x28, #0x90]
1009966ac:     	add	x0, sp, #0xc0
1009966b0:     	mov	x3, x21
1009966b4:     	mov	x4, x23
1009966b8:     	mov	x5, x22
1009966bc:     	bl	0x100990338 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1009966c0:     	ldp	x1, x2, [x28, #0xa8]
1009966c4:     	add	x0, sp, #0xf0
1009966c8:     	add	x4, x23, #0x18
1009966cc:     	mov	x3, x21
1009966d0:     	mov	x5, x22
1009966d4:     	bl	0x100990338 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1009966d8:     	mov	w8, #0x1                ; =1
1009966dc:     	ldr	x9, [sp, #0x8]
1009966e0:     	lsl	x8, x8, x9
1009966e4:     	lsr	x8, x8, #6
1009966e8:     	cmp	x9, #0x6
1009966ec:     	cinc	x20, x8, lo
1009966f0:     	cbz	x20, 0x100996790 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
1009966f4:     	lsl	x24, x20, #3
1009966f8:     	mov	x0, x24
1009966fc:     	mov	w1, #0x8                ; =8
100996700:     	bl	0x1015d9f98 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
100996704:     	cbz	x0, 0x100996bc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14b4>
100996708:     	mov	x25, x0
10099670c:     	mov	x0, #0x0                ; =0
100996710:     	ldp	x8, x9, [sp, #0xc0]
100996714:     	ldp	x1, x10, [sp, #0xd0]
100996718:     	sub	x11, x0, w9, uxtb
10099671c:     	ldp	x23, x13, [sp, #0xf0]
100996720:     	ldp	x12, x14, [sp, #0x100]
100996724:     	mov	x24, x25
100996728:     	b	0x100996744 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
10099672c:     	tst	w13, #0x1
100996730:     	csel	x15, x15, xzr, ne
100996734:     	str	x15, [x24, x0, lsl #3]
100996738:     	add	x0, x0, #0x1
10099673c:     	cmp	x20, x0
100996740:     	b.eq	0x100996304 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
100996744:     	mov	x15, x11
100996748:     	cmn	x8, #0x2
10099674c:     	b.eq	0x100996760 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1054>
100996750:     	cmp	x0, x1
100996754:     	b.hs	0x100996b64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1458>
100996758:     	ldr	x15, [x9, x0, lsl #3]
10099675c:     	eor	x15, x10, x15
100996760:     	cmn	x23, #0x2
100996764:     	b.eq	0x10099672c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1020>
100996768:     	cmp	x0, x12
10099676c:     	b.hs	0x100996b60 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1454>
100996770:     	ldr	x16, [x13, x0, lsl #3]
100996774:     	eor	x16, x14, x16
100996778:     	and	x15, x16, x15
10099677c:     	str	x15, [x24, x0, lsl #3]
100996780:     	add	x0, x0, #0x1
100996784:     	cmp	x20, x0
100996788:     	b.ne	0x100996744 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
10099678c:     	b	0x100996304 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
100996790:     	mov	x27, #0x0               ; =0
100996794:     	ldr	x23, [sp, #0xf0]
100996798:     	mov	w24, #0x8               ; =8
10099679c:     	cmp	x23, #0x1
1009967a0:     	b.lt	0x1009967ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10a0>
1009967a4:     	ldr	x0, [sp, #0xf8]
1009967a8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009967ac:     	ldr	x8, [sp, #0xc0]
1009967b0:     	cmp	x8, #0x1
1009967b4:     	b.lt	0x1009967c0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10b4>
1009967b8:     	ldr	x0, [sp, #0xc8]
1009967bc:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1009967c0:     	ldr	x8, [x28, #0xc0]
1009967c4:     	mov	w9, #0x4                ; =4
1009967c8:     	stp	xzr, x9, [sp, #0xf0]
1009967cc:     	str	xzr, [sp, #0x100]
1009967d0:     	ands	x23, x8, x22
1009967d4:     	b.eq	0x100996a08 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12fc>
1009967d8:     	mov	x19, #0x0               ; =0
1009967dc:     	mov	w8, #0x4                ; =4
1009967e0:     	b	0x100996808 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10fc>
1009967e4:     	ldr	x8, [sp, #0xf8]
1009967e8:     	rbit	x9, x23
1009967ec:     	clz	x9, x9
1009967f0:     	str	w9, [x8, x19, lsl #2]
1009967f4:     	add	x19, x19, #0x1
1009967f8:     	str	x19, [sp, #0x100]
1009967fc:     	sub	x9, x23, #0x1
100996800:     	ands	x23, x9, x23
100996804:     	b.eq	0x100996820 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1114>
100996808:     	ldr	x9, [sp, #0xf0]
10099680c:     	cmp	x19, x9
100996810:     	b.ne	0x1009967e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10dc>
100996814:     	add	x0, sp, #0xf0
100996818:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
10099681c:     	b	0x1009967e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10d8>
100996820:     	ldp	x9, x8, [sp, #0xf0]
100996824:     	str	x9, [sp, #0x70]
100996828:     	str	x8, [sp, #0x58]
10099682c:     	cbz	x19, 0x1009969e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12dc>
100996830:     	mov	x23, x8
100996834:     	mov	x25, x20
100996838:     	add	x8, x8, x19, lsl #2
10099683c:     	str	x8, [sp, #0x78]
100996840:     	b	0x100996860 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1154>
100996844:     	bic	x22, x22, x20
100996848:     	mov	x24, x26
10099684c:     	mov	x27, x25
100996850:     	mov	x20, x25
100996854:     	ldr	x8, [sp, #0x78]
100996858:     	cmp	x23, x8
10099685c:     	b.eq	0x1009969f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12e4>
100996860:     	str	x27, [sp, #0x80]
100996864:     	ldr	w8, [x23], #0x4
100996868:     	mov	w9, #0x1                ; =1
10099686c:     	lsl	x20, x9, x8
100996870:     	sub	x8, x20, #0x1
100996874:     	and	x8, x8, x22
100996878:     	fmov	d0, x8
10099687c:     	cnt.8b	v0, v0
100996880:     	addv.8b	b0, v0
100996884:     	fmov	w26, s0
100996888:     	fmov	d0, x22
10099688c:     	cnt.8b	v0, v0
100996890:     	addv.8b	b0, v0
100996894:     	fmov	w27, s0
100996898:     	add	x0, sp, #0xc0
10099689c:     	mov	x1, x24
1009968a0:     	mov	x2, x25
1009968a4:     	mov	x3, x27
1009968a8:     	mov	x4, x26
1009968ac:     	mov	w5, #0x0                ; =0
1009968b0:     	bl	0x101169958 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1009968b4:     	add	x0, sp, #0xf0
1009968b8:     	mov	x1, x24
1009968bc:     	mov	x2, x25
1009968c0:     	mov	x3, x27
1009968c4:     	mov	x4, x26
1009968c8:     	mov	w5, #0x1                ; =1
1009968cc:     	bl	0x101169958 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1009968d0:     	ldp	x19, x8, [sp, #0xc8]
1009968d4:     	ldp	x27, x28, [sp, #0xf0]
1009968d8:     	ldr	x9, [sp, #0x100]
1009968dc:     	cmp	x9, x8
1009968e0:     	csel	x25, x9, x8, lo
1009968e4:     	cbz	x25, 0x100996940 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1234>
1009968e8:     	str	x24, [sp, #0x68]
1009968ec:     	lsl	x24, x25, #3
1009968f0:     	mov	x0, x24
1009968f4:     	bl	0x1016efa04 <dyld_stub_binder+0x1016efa04>
1009968f8:     	cbz	x0, 0x100996b30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1424>
1009968fc:     	mov	x26, x0
100996900:     	cmp	x25, #0x8
100996904:     	b.hs	0x100996978 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x126c>
100996908:     	mov	x8, #0x0                ; =0
10099690c:     	ldr	x24, [sp, #0x68]
100996910:     	lsl	x11, x8, #3
100996914:     	add	x9, x19, x11
100996918:     	add	x10, x28, x11
10099691c:     	add	x11, x26, x11
100996920:     	sub	x8, x25, x8
100996924:     	ldr	x12, [x10], #0x8
100996928:     	ldr	x13, [x9], #0x8
10099692c:     	orr	x12, x13, x12
100996930:     	str	x12, [x11], #0x8
100996934:     	subs	x8, x8, #0x1
100996938:     	b.ne	0x100996924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1218>
10099693c:     	b	0x100996944 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
100996940:     	mov	w26, #0x8               ; =8
100996944:     	cbz	x27, 0x100996950 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1244>
100996948:     	mov	x0, x28
10099694c:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996950:     	ldr	x8, [sp, #0x80]
100996954:     	cbz	x8, 0x100996960 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1254>
100996958:     	mov	x0, x24
10099695c:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996960:     	ldr	x8, [sp, #0xc0]
100996964:     	ldr	x28, [sp, #0x88]
100996968:     	cbz	x8, 0x100996844 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
10099696c:     	mov	x0, x19
100996970:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996974:     	b	0x100996844 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
100996978:     	mov	x8, #0x0                ; =0
10099697c:     	sub	x9, x28, x26
100996980:     	cmn	x9, #0x40
100996984:     	ldr	x24, [sp, #0x68]
100996988:     	b.hi	0x100996910 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
10099698c:     	sub	x9, x19, x26
100996990:     	cmn	x9, #0x40
100996994:     	b.hi	0x100996910 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
100996998:     	and	x8, x25, #0xffffffffffffff8
10099699c:     	add	x9, x19, #0x20
1009969a0:     	add	x10, x28, #0x20
1009969a4:     	add	x11, x26, #0x20
1009969a8:     	and	x12, x25, #0xffffffffffffff8
1009969ac:     	ldp	q0, q1, [x10, #-0x20]
1009969b0:     	ldp	q2, q3, [x10], #0x40
1009969b4:     	ldp	q4, q5, [x9, #-0x20]
1009969b8:     	ldp	q6, q7, [x9], #0x40
1009969bc:     	orr.16b	v0, v4, v0
1009969c0:     	orr.16b	v1, v5, v1
1009969c4:     	orr.16b	v2, v6, v2
1009969c8:     	orr.16b	v3, v7, v3
1009969cc:     	stp	q0, q1, [x11, #-0x20]
1009969d0:     	stp	q2, q3, [x11], #0x40
1009969d4:     	subs	x12, x12, #0x8
1009969d8:     	b.ne	0x1009969ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12a0>
1009969dc:     	cmp	x25, x8
1009969e0:     	b.ne	0x100996910 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1009969e4:     	b	0x100996944 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
1009969e8:     	mov	x25, x27
1009969ec:     	mov	x26, x24
1009969f0:     	ldr	x8, [sp, #0x70]
1009969f4:     	cbz	x8, 0x100996a00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12f4>
1009969f8:     	ldr	x0, [sp, #0x58]
1009969fc:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996a00:     	mov	x24, x26
100996a04:     	mov	x27, x25
100996a08:     	ldr	x19, [sp, #0x60]
100996a0c:     	stp	x27, x24, [sp, #0xf0]
100996a10:     	str	x20, [sp, #0x100]
100996a14:     	add	x2, sp, #0xf0
100996a18:     	mov	x0, x21
100996a1c:     	mov	x1, x22
100996a20:     	bl	0x100fb30e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100996a24:     	mov	x8, x0
100996a28:     	add	x0, x28, #0x70
100996a2c:     	mov	x1, x19
100996a30:     	mov	x19, x8
100996a34:     	mov	x2, x8
100996a38:     	bl	0x101056eb0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
100996a3c:     	mov	x0, x19
100996a40:     	b	0x1009964e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
100996a44:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
100996a48:     	add	x2, x2, #0x358
100996a4c:     	adrp	x3, 0x101799000 <dyld_stub_binder+0x101799000>
100996a50:     	add	x3, x3, #0xdbd
100996a54:     	adrp	x5, 0x101945000 <dyld_stub_binder+0x101945000>
100996a58:     	add	x5, x5, #0xed8
100996a5c:     	add	x1, sp, #0xf0
100996a60:     	mov	w0, #0x0                ; =0
100996a64:     	mov	w4, #0x43               ; =67
100996a68:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100996a6c:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996a70:     	add	x2, x2, #0x4d0
100996a74:     	b	0x100996a90 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1384>
100996a78:     	ldr	x20, [sp, #0x68]
100996a7c:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996a80:     	add	x2, x2, #0x4d0
100996a84:     	b	0x100996ac4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13b8>
100996a88:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996a8c:     	add	x2, x2, #0x4b8
100996a90:     	ldr	x20, [sp, #0x68]
100996a94:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996a98:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996a9c:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
100996aa0:     	add	x0, x0, #0xf53
100996aa4:     	adrp	x2, 0x101947000 <dyld_stub_binder+0x101947000>
100996aa8:     	add	x2, x2, #0x188
100996aac:     	mov	w1, #0x51               ; =81
100996ab0:     	bl	0x1016e73bc <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100996ab4:     	mov	x1, x8
100996ab8:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996abc:     	add	x2, x2, #0x4b8
100996ac0:     	ldr	x20, [sp, #0x68]
100996ac4:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996ac8:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996acc:     	adrp	x0, 0x101799000 <dyld_stub_binder+0x101799000>
100996ad0:     	add	x0, x0, #0xf27
100996ad4:     	adrp	x2, 0x101946000 <dyld_stub_binder+0x101946000>
100996ad8:     	add	x2, x2, #0xd90
100996adc:     	mov	w1, #0x2c               ; =44
100996ae0:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100996ae4:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
100996ae8:     	add	x2, x2, #0x838
100996aec:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996af0:     	adrp	x2, 0x10198d000 <dyld_stub_binder+0x10198d000>
100996af4:     	add	x2, x2, #0x838
100996af8:     	mov	x1, x8
100996afc:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b00:     	mov	x20, x16
100996b04:     	b	0x100996b14 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1408>
100996b08:     	mov	x20, x16
100996b0c:     	mov	x1, x8
100996b10:     	mov	x2, x12
100996b14:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b18:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996b1c:     	mov	x1, x12
100996b20:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100996b24:     	add	x2, x2, #0x598
100996b28:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b2c:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996b30:     	mov	w0, #0x8                ; =8
100996b34:     	mov	x1, x24
100996b38:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100996b3c:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996b40:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100996b44:     	add	x2, x2, #0x5b0
100996b48:     	mov	x0, x22
100996b4c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b50:     	mov	x0, x8
100996b54:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996b58:     	add	x2, x2, #0x4d0
100996b5c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b60:     	mov	x1, x12
100996b64:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100996b68:     	add	x2, x2, #0x598
100996b6c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b70:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996b74:     	adrp	x2, 0x10194e000 <dyld_stub_binder+0x10194e000>
100996b78:     	add	x2, x2, #0x5c8
100996b7c:     	mov	x0, x22
100996b80:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b84:     	mov	x0, x8
100996b88:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100996b8c:     	add	x2, x2, #0x4b8
100996b90:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100996b94:     	mov	w0, #0x8                ; =8
100996b98:     	mov	x1, x24
100996b9c:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100996ba0:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996ba4:     	mov	w0, #0x8                ; =8
100996ba8:     	mov	x1, x19
100996bac:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100996bb0:     	mov	w0, #0x8                ; =8
100996bb4:     	mov	x1, x26
100996bb8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100996bbc:     	b	0x100996bcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
100996bc0:     	mov	w0, #0x8                ; =8
100996bc4:     	mov	x1, x24
100996bc8:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100996bcc:     	brk	#0x1
100996bd0:     	b	0x100996be0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14d4>
100996bd4:     	b	0x100996bec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14e0>
100996bd8:     	ldr	x20, [sp, #0x68]
100996bdc:     	b	0x100996cd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
100996be0:     	mov	x19, x0
100996be4:     	ldr	x23, [sp, #0xf0]
100996be8:     	b	0x100996c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x156c>
100996bec:     	mov	x19, x0
100996bf0:     	b	0x100996c88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
100996bf4:     	b	0x100996c6c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1560>
100996bf8:     	mov	x20, x0
100996bfc:     	cbz	x27, 0x100996c38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
100996c00:     	mov	x0, x28
100996c04:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996c08:     	b	0x100996c38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
100996c0c:     	b	0x100996cb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15a4>
100996c10:     	str	x27, [sp, #0x80]
100996c14:     	str	x24, [sp, #0x68]
100996c18:     	mov	x20, x0
100996c1c:     	ldr	x8, [sp, #0xf0]
100996c20:     	cbz	x8, 0x100996c64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
100996c24:     	ldr	x8, [sp, #0xf8]
100996c28:     	str	x8, [sp, #0x58]
100996c2c:     	b	0x100996c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1550>
100996c30:     	str	x24, [sp, #0x68]
100996c34:     	mov	x20, x0
100996c38:     	ldr	x8, [sp, #0xc0]
100996c3c:     	cbz	x8, 0x100996c54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
100996c40:     	ldr	x0, [sp, #0xc8]
100996c44:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996c48:     	b	0x100996c54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
100996c4c:     	str	x24, [sp, #0x68]
100996c50:     	mov	x20, x0
100996c54:     	ldr	x8, [sp, #0x70]
100996c58:     	cbz	x8, 0x100996c64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
100996c5c:     	ldr	x0, [sp, #0x58]
100996c60:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996c64:     	mov	x0, x20
100996c68:     	b	0x100996cc4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
100996c6c:     	mov	x19, x0
100996c70:     	mov	x0, x25
100996c74:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996c78:     	cmp	x23, #0x1
100996c7c:     	b.lt	0x100996c88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
100996c80:     	ldr	x0, [sp, #0xf8]
100996c84:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996c88:     	ldr	x8, [sp, #0xc0]
100996c8c:     	cmp	x8, #0x1
100996c90:     	b.lt	0x100996cdc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d0>
100996c94:     	ldr	x20, [sp, #0xc8]
100996c98:     	mov	x0, x19
100996c9c:     	b	0x100996cd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
100996ca0:     	tbz	w19, #0x0, 0x100996cd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
100996ca4:     	b	0x100996ce0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
100996ca8:     	b	0x100996cc4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
100996cac:     	b	0x100996cc8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
100996cb0:     	ldr	x8, [sp, #0xf0]
100996cb4:     	cbz	x8, 0x100996ce0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
100996cb8:     	ldr	x20, [sp, #0xf8]
100996cbc:     	b	0x100996cd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
100996cc0:     	b	0x100996cc8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
100996cc4:     	ldr	x20, [sp, #0x68]
100996cc8:     	ldr	x8, [sp, #0x80]
100996ccc:     	cbz	x8, 0x100996ce0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
100996cd0:     	mov	x19, x0
100996cd4:     	mov	x0, x20
100996cd8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100996cdc:     	mov	x0, x19
100996ce0:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
