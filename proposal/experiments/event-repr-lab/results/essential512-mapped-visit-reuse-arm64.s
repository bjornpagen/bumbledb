
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001007b37fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>:
1007b37fc:     	stp	x28, x27, [sp, #-0x60]!
1007b3800:     	stp	x26, x25, [sp, #0x10]
1007b3804:     	stp	x24, x23, [sp, #0x20]
1007b3808:     	stp	x22, x21, [sp, #0x30]
1007b380c:     	stp	x20, x19, [sp, #0x40]
1007b3810:     	stp	x29, x30, [sp, #0x50]
1007b3814:     	add	x29, sp, #0x50
1007b3818:     	sub	sp, sp, #0x1c0
1007b381c:     	mov	x23, x2
1007b3820:     	mov	x21, x1
1007b3824:     	mov	x28, x0
1007b3828:     	ldrb	w8, [x0, #0x151]
1007b382c:     	str	x0, [sp, #0x88]
1007b3830:     	str	x2, [sp, #0x60]
1007b3834:     	cbz	w8, 0x1007b3b5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
1007b3838:     	mov	x27, #0x0               ; =0
1007b383c:     	b	0x1007b3858 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
1007b3840:     	ldr	w9, [x26, #0x14]
1007b3844:     	add	x27, x27, #0x18
1007b3848:     	stp	xzr, x20, [x26]
1007b384c:     	stp	w24, w9, [x26, #0x10]
1007b3850:     	cmp	x27, #0x30
1007b3854:     	b.eq	0x1007b3b5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x360>
1007b3858:     	add	x26, x23, x27
1007b385c:     	ldp	x19, x20, [x26]
1007b3860:     	ldr	w24, [x26, #0x10]
1007b3864:     	cbz	x19, 0x1007b3840 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x44>
1007b3868:     	ldr	x8, [x28, #0x138]
1007b386c:     	add	x8, x8, #0x1
1007b3870:     	str	x8, [x28, #0x138]
1007b3874:     	ldur	x8, [x21, #0x40]
1007b3878:     	lsr	x0, x24, #1
1007b387c:     	cmn	x8, #0x1
1007b3880:     	str	w9, [sp, #0x70]
1007b3884:     	b.eq	0x1007b38a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa4>
1007b3888:     	ldr	x1, [x21, #0x50]
1007b388c:     	cmp	x1, x0
1007b3890:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b3894:     	ldr	x8, [x21, #0x48]
1007b3898:     	add	x8, x8, x0, lsl #4
1007b389c:     	b	0x1007b38b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbc>
1007b38a0:     	ldr	x1, [x21, #0x58]
1007b38a4:     	cmp	x1, x0
1007b38a8:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b38ac:     	ldr	x8, [x21, #0x50]
1007b38b0:     	add	x8, x8, x0, lsl #5
1007b38b4:     	add	x8, x8, #0x18
1007b38b8:     	mov	x25, #0x0               ; =0
1007b38bc:     	ldr	x8, [x8]
1007b38c0:     	bic	x8, x8, x19
1007b38c4:     	str	x8, [sp, #0x78]
1007b38c8:     	mov	w8, #0x4                ; =4
1007b38cc:     	stp	xzr, x8, [sp, #0xf0]
1007b38d0:     	str	xzr, [sp, #0x100]
1007b38d4:     	mov	w9, #0x4                ; =4
1007b38d8:     	mov	w8, #0x4                ; =4
1007b38dc:     	b	0x1007b3904 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x108>
1007b38e0:     	rbit	x9, x19
1007b38e4:     	clz	x9, x9
1007b38e8:     	str	w9, [x8, x25, lsl #2]
1007b38ec:     	add	x25, x25, #0x1
1007b38f0:     	str	x25, [sp, #0x100]
1007b38f4:     	sub	x10, x19, #0x1
1007b38f8:     	add	x9, x23, #0x4
1007b38fc:     	ands	x19, x10, x19
1007b3900:     	b.eq	0x1007b3924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x128>
1007b3904:     	mov	x23, x9
1007b3908:     	ldr	x9, [sp, #0xf0]
1007b390c:     	cmp	x25, x9
1007b3910:     	b.ne	0x1007b38e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
1007b3914:     	add	x0, sp, #0xf0
1007b3918:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b391c:     	ldr	x8, [sp, #0xf8]
1007b3920:     	b	0x1007b38e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe4>
1007b3924:     	ldp	x9, x8, [sp, #0xf0]
1007b3928:     	str	x9, [sp, #0x80]
1007b392c:     	str	x8, [sp, #0x68]
1007b3930:     	cbz	x25, 0x1007b3aa4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
1007b3934:     	ldr	x19, [x28, #0x140]
1007b3938:     	mov	x28, x8
1007b393c:     	b	0x1007b3974 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x178>
1007b3940:     	tst	w22, #0x1
1007b3944:     	mov	w8, #0x8                ; =8
1007b3948:     	mov	w9, #0xc                ; =12
1007b394c:     	csel	x8, x9, x8, ne
1007b3950:     	add	x9, sp, #0xf0
1007b3954:     	ldr	w8, [x9, x8]
1007b3958:     	and	w9, w24, #0x1
1007b395c:     	eor	w24, w8, w9
1007b3960:     	add	x19, x19, #0x1
1007b3964:     	ldr	x8, [sp, #0x88]
1007b3968:     	str	x19, [x8, #0x140]
1007b396c:     	subs	x23, x23, #0x4
1007b3970:     	b.eq	0x1007b3aa4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2a8>
1007b3974:     	ldr	w25, [x28], #0x4
1007b3978:     	ldur	x8, [x21, #0x40]
1007b397c:     	lsr	w0, w24, #1
1007b3980:     	cmn	x8, #0x1
1007b3984:     	b.eq	0x1007b39b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1b8>
1007b3988:     	ldr	x1, [x21, #0x50]
1007b398c:     	cmp	x1, x0
1007b3990:     	b.ls	0x1007b4b5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1360>
1007b3994:     	ldr	x9, [x21, #0x48]
1007b3998:     	add	x9, x9, x0, lsl #4
1007b399c:     	ldr	x10, [x9]
1007b39a0:     	mov	w9, #0x1                ; =1
1007b39a4:     	lsl	x9, x9, x25
1007b39a8:     	tst	x10, x9
1007b39ac:     	b.ne	0x1007b39dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1e0>
1007b39b0:     	b	0x1007b396c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1007b39b4:     	ldr	x1, [x21, #0x58]
1007b39b8:     	cmp	x1, x0
1007b39bc:     	b.ls	0x1007b4b78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x137c>
1007b39c0:     	ldr	x1, [x21, #0x50]
1007b39c4:     	add	x9, x1, x0, lsl #5
1007b39c8:     	ldr	x10, [x9, #0x18]!
1007b39cc:     	mov	w9, #0x1                ; =1
1007b39d0:     	lsl	x9, x9, x25
1007b39d4:     	tst	x10, x9
1007b39d8:     	b.eq	0x1007b396c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x170>
1007b39dc:     	ldr	w10, [x21, #0xf0]
1007b39e0:     	cmp	w25, w10
1007b39e4:     	b.hs	0x1007b3d2c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x530>
1007b39e8:     	cmn	x8, #0x1
1007b39ec:     	b.eq	0x1007b3a10 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x214>
1007b39f0:     	cmp	x1, x0
1007b39f4:     	b.ls	0x1007b4b68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x136c>
1007b39f8:     	ldr	x8, [x21, #0x48]
1007b39fc:     	add	x8, x8, x0, lsl #4
1007b3a00:     	ldr	x8, [x8]
1007b3a04:     	tst	x8, x9
1007b3a08:     	b.ne	0x1007b3a30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x234>
1007b3a0c:     	b	0x1007b3960 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1007b3a10:     	ldr	x8, [x21, #0x58]
1007b3a14:     	cmp	x8, x0
1007b3a18:     	b.ls	0x1007b4ba4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13a8>
1007b3a1c:     	add	x8, x1, x0, lsl #5
1007b3a20:     	add	x8, x8, #0x18
1007b3a24:     	ldr	x8, [x8]
1007b3a28:     	tst	x8, x9
1007b3a2c:     	b.eq	0x1007b3960 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1007b3a30:     	and	x8, x25, #0x3f
1007b3a34:     	lsr	x22, x20, x8
1007b3a38:     	ldrb	w8, [x21, #0xf5]
1007b3a3c:     	tbz	w8, #0x0, 0x1007b3a88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x28c>
1007b3a40:     	add	x0, sp, #0xf0
1007b3a44:     	add	x1, x21, #0x40
1007b3a48:     	mov	x2, x24
1007b3a4c:     	bl	0x100d9d3c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1007b3a50:     	ldr	w8, [sp, #0xf0]
1007b3a54:     	cmp	w8, #0x2
1007b3a58:     	b.ne	0x1007b3a68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x26c>
1007b3a5c:     	ldr	w8, [sp, #0xf4]
1007b3a60:     	cmp	w8, w25
1007b3a64:     	b.eq	0x1007b3940 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x144>
1007b3a68:     	and	w1, w24, #0xfffffffe
1007b3a6c:     	and	w3, w22, #0x1
1007b3a70:     	mov	x0, x21
1007b3a74:     	mov	x2, x25
1007b3a78:     	bl	0x100cad380 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
1007b3a7c:     	and	w8, w24, #0x1
1007b3a80:     	eor	w24, w0, w8
1007b3a84:     	b	0x1007b3960 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1007b3a88:     	and	w3, w22, #0x1
1007b3a8c:     	mov	x0, x21
1007b3a90:     	mov	x1, x24
1007b3a94:     	mov	x2, x25
1007b3a98:     	bl	0x100cad380 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
1007b3a9c:     	mov	x24, x0
1007b3aa0:     	b	0x1007b3960 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x164>
1007b3aa4:     	ldr	x8, [sp, #0x80]
1007b3aa8:     	cbz	x8, 0x1007b3ab4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2b8>
1007b3aac:     	ldr	x0, [sp, #0x68]
1007b3ab0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3ab4:     	ldur	x8, [x21, #0x40]
1007b3ab8:     	lsr	w0, w24, #1
1007b3abc:     	cmn	x8, #0x1
1007b3ac0:     	ldr	x28, [sp, #0x88]
1007b3ac4:     	ldr	x23, [sp, #0x60]
1007b3ac8:     	ldr	x10, [sp, #0x78]
1007b3acc:     	b.eq	0x1007b3af8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x2fc>
1007b3ad0:     	ldr	x1, [x21, #0x50]
1007b3ad4:     	cmp	x1, x0
1007b3ad8:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b3adc:     	ldr	x8, [x21, #0x48]
1007b3ae0:     	add	x8, x8, x0, lsl #4
1007b3ae4:     	ldr	x8, [x8]
1007b3ae8:     	bics	x9, x8, x10
1007b3aec:     	str	x9, [sp, #0xf0]
1007b3af0:     	b.eq	0x1007b3b20 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x324>
1007b3af4:     	b	0x1007b4b34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
1007b3af8:     	ldr	x1, [x21, #0x58]
1007b3afc:     	cmp	x1, x0
1007b3b00:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b3b04:     	ldr	x8, [x21, #0x50]
1007b3b08:     	add	x8, x8, x0, lsl #5
1007b3b0c:     	add	x8, x8, #0x18
1007b3b10:     	ldr	x8, [x8]
1007b3b14:     	bics	x9, x8, x10
1007b3b18:     	str	x9, [sp, #0xf0]
1007b3b1c:     	b.ne	0x1007b4b34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1338>
1007b3b20:     	mov	x20, #0x0               ; =0
1007b3b24:     	bic	x8, x10, x8
1007b3b28:     	fmov	d0, x8
1007b3b2c:     	cnt.8b	v0, v0
1007b3b30:     	addv.8b	b0, v0
1007b3b34:     	fmov	x8, d0
1007b3b38:     	ldr	x9, [x28, #0x148]
1007b3b3c:     	add	x8, x9, x8
1007b3b40:     	str	x8, [x28, #0x148]
1007b3b44:     	ldr	w9, [sp, #0x70]
1007b3b48:     	add	x27, x27, #0x18
1007b3b4c:     	stp	xzr, x20, [x26]
1007b3b50:     	stp	w24, w9, [x26, #0x10]
1007b3b54:     	cmp	x27, #0x30
1007b3b58:     	b.ne	0x1007b3858 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5c>
1007b3b5c:     	ldr	w9, [x23, #0x10]
1007b3b60:     	cbz	w9, 0x1007b45cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
1007b3b64:     	ldr	w10, [x23, #0x28]
1007b3b68:     	cbz	w10, 0x1007b45cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd0>
1007b3b6c:     	cmp	w9, #0x1
1007b3b70:     	ccmp	w10, #0x1, #0x0, eq
1007b3b74:     	b.eq	0x1007b3c98 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x49c>
1007b3b78:     	ldr	x8, [x28, #0x88]
1007b3b7c:     	cbz	x8, 0x1007b3ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
1007b3b80:     	mov	x8, #0x0                ; =0
1007b3b84:     	mov	x15, #0xa9c5            ; =43461
1007b3b88:     	movk	x15, #0x2e62, lsl #16
1007b3b8c:     	movk	x15, #0x7aea, lsl #32
1007b3b90:     	movk	x15, #0xf135, lsl #48
1007b3b94:     	ldp	x11, x12, [x23]
1007b3b98:     	madd	x13, x9, x15, x11
1007b3b9c:     	mov	x14, #0x6332            ; =25394
1007b3ba0:     	movk	x14, #0x6ed3, lsl #16
1007b3ba4:     	movk	x14, #0x765a, lsl #32
1007b3ba8:     	movk	x14, #0x284f, lsl #48
1007b3bac:     	mul	x14, x14, x15
1007b3bb0:     	madd	x13, x13, x15, x14
1007b3bb4:     	add	x13, x13, x12
1007b3bb8:     	madd	x16, x13, x15, x10
1007b3bbc:     	ldp	x13, x14, [x23, #0x18]
1007b3bc0:     	madd	x16, x16, x15, x13
1007b3bc4:     	madd	x16, x16, x15, x14
1007b3bc8:     	mul	x15, x16, x15
1007b3bcc:     	ror	x0, x15, #0x2c
1007b3bd0:     	lsr	x17, x0, #57
1007b3bd4:     	ldp	x16, x15, [x28, #0x70]
1007b3bd8:     	dup.8b	v0, w17
1007b3bdc:     	movi.2d	v1, #0xffffffffffffffff
1007b3be0:     	mov	w17, #0x38              ; =56
1007b3be4:     	and	x0, x0, x15
1007b3be8:     	ldr	d2, [x16, x0]
1007b3bec:     	cmeq.8b	v3, v2, v0
1007b3bf0:     	fmov	x1, d3
1007b3bf4:     	ands	x1, x1, #0x8080808080808080
1007b3bf8:     	b.eq	0x1007b3c68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
1007b3bfc:     	rbit	x2, x1
1007b3c00:     	clz	x2, x2
1007b3c04:     	add	x2, x0, x2, lsr #3
1007b3c08:     	and	x2, x2, x15
1007b3c0c:     	mneg	x2, x2, x17
1007b3c10:     	add	x2, x16, x2
1007b3c14:     	ldur	x3, [x2, #-0x38]
1007b3c18:     	cmp	x11, x3
1007b3c1c:     	b.ne	0x1007b3c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1007b3c20:     	ldur	x3, [x2, #-0x30]
1007b3c24:     	cmp	x12, x3
1007b3c28:     	b.ne	0x1007b3c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1007b3c2c:     	ldur	w3, [x2, #-0x28]
1007b3c30:     	cmp	w9, w3
1007b3c34:     	b.ne	0x1007b3c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1007b3c38:     	ldur	x3, [x2, #-0x20]
1007b3c3c:     	cmp	x13, x3
1007b3c40:     	b.ne	0x1007b3c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1007b3c44:     	ldur	x3, [x2, #-0x18]
1007b3c48:     	cmp	x14, x3
1007b3c4c:     	b.ne	0x1007b3c5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x460>
1007b3c50:     	ldur	w3, [x2, #-0x10]
1007b3c54:     	cmp	w10, w3
1007b3c58:     	b.eq	0x1007b3d18 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x51c>
1007b3c5c:     	sub	x2, x1, #0x2
1007b3c60:     	ands	x1, x2, x1
1007b3c64:     	b.ne	0x1007b3bfc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
1007b3c68:     	cmeq.8b	v2, v2, v1
1007b3c6c:     	fmov	x1, d2
1007b3c70:     	cbnz	x1, 0x1007b3ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x4a4>
1007b3c74:     	add	x8, x8, #0x8
1007b3c78:     	add	x0, x0, x8
1007b3c7c:     	and	x0, x0, x15
1007b3c80:     	ldr	d2, [x16, x0]
1007b3c84:     	cmeq.8b	v3, v2, v0
1007b3c88:     	fmov	x1, d3
1007b3c8c:     	ands	x1, x1, #0x8080808080808080
1007b3c90:     	b.ne	0x1007b3bfc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x400>
1007b3c94:     	b	0x1007b3c68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x46c>
1007b3c98:     	mov	w0, #0x1                ; =1
1007b3c9c:     	b	0x1007b45d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1007b3ca0:     	mov	x24, x28
1007b3ca4:     	ldr	x8, [x24, #0xc8]!
1007b3ca8:     	add	x8, x8, #0x1
1007b3cac:     	str	x8, [x24]
1007b3cb0:     	mov	w11, #0x8481            ; =33921
1007b3cb4:     	movk	w11, #0x1e, lsl #16
1007b3cb8:     	cmp	x8, x11
1007b3cbc:     	b.hs	0x1007b4b8c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1390>
1007b3cc0:     	ldr	x12, [x23]
1007b3cc4:     	ldr	x13, [x23, #0x18]
1007b3cc8:     	ldr	x8, [x21, #0x40]
1007b3ccc:     	cmn	x8, #0x1
1007b3cd0:     	b.eq	0x1007b3d48 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x54c>
1007b3cd4:     	ldr	x1, [x21, #0x50]
1007b3cd8:     	lsr	x0, x9, #1
1007b3cdc:     	cmp	x1, x0
1007b3ce0:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b3ce4:     	lsr	x8, x10, #1
1007b3ce8:     	cmp	x1, x8
1007b3cec:     	b.ls	0x1007b4c40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1444>
1007b3cf0:     	ldr	x11, [x21, #0x48]
1007b3cf4:     	lsl	x14, x0, #4
1007b3cf8:     	ldr	x14, [x11, x14]
1007b3cfc:     	bic	x19, x14, x12
1007b3d00:     	add	x8, x11, x8, lsl #4
1007b3d04:     	ldr	x15, [x8]
1007b3d08:     	ldp	x11, x1, [x28, #0x18]
1007b3d0c:     	mov	x22, #0x0               ; =0
1007b3d10:     	cbnz	x19, 0x1007b3d88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x58c>
1007b3d14:     	b	0x1007b3db8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
1007b3d18:     	ldur	w0, [x2, #-0x8]
1007b3d1c:     	ldr	x8, [x28, #0xd0]
1007b3d20:     	add	x8, x8, #0x1
1007b3d24:     	str	x8, [x28, #0xd0]
1007b3d28:     	b	0x1007b45d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1007b3d2c:     	adrp	x0, 0x101481000 <dyld_stub_binder+0x101481000>
1007b3d30:     	add	x0, x0, #0x46
1007b3d34:     	adrp	x2, 0x101641000 <dyld_stub_binder+0x101641000>
1007b3d38:     	add	x2, x2, #0xaa8
1007b3d3c:     	mov	w1, #0x2c               ; =44
1007b3d40:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1007b3d44:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b3d48:     	ldr	x1, [x21, #0x58]
1007b3d4c:     	lsr	x0, x9, #1
1007b3d50:     	cmp	x1, x0
1007b3d54:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b3d58:     	lsr	x8, x10, #1
1007b3d5c:     	cmp	x1, x8
1007b3d60:     	b.ls	0x1007b4c74 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1478>
1007b3d64:     	ldr	x11, [x21, #0x50]
1007b3d68:     	add	x14, x11, x0, lsl #5
1007b3d6c:     	ldr	x14, [x14, #0x18]
1007b3d70:     	bic	x19, x14, x12
1007b3d74:     	add	x8, x11, x8, lsl #5
1007b3d78:     	ldr	x15, [x8, #0x18]!
1007b3d7c:     	ldp	x11, x1, [x28, #0x18]
1007b3d80:     	mov	x22, #0x0               ; =0
1007b3d84:     	cbz	x19, 0x1007b3db8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5bc>
1007b3d88:     	mov	w8, #0x1                ; =1
1007b3d8c:     	mov	x14, x19
1007b3d90:     	rbit	x16, x14
1007b3d94:     	clz	x0, x16
1007b3d98:     	cmp	x0, x1
1007b3d9c:     	b.hs	0x1007b4bd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13d8>
1007b3da0:     	ldr	w16, [x11, x0, lsl #2]
1007b3da4:     	lsl	x16, x8, x16
1007b3da8:     	orr	x22, x16, x22
1007b3dac:     	sub	x16, x14, #0x1
1007b3db0:     	ands	x14, x16, x14
1007b3db4:     	b.ne	0x1007b3d90 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x594>
1007b3db8:     	ldp	x14, x8, [x28, #0x48]
1007b3dbc:     	bic	x15, x15, x13
1007b3dc0:     	cbz	x15, 0x1007b3df8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5fc>
1007b3dc4:     	mov	x16, #0x0               ; =0
1007b3dc8:     	mov	w17, #0x1               ; =1
1007b3dcc:     	rbit	x0, x15
1007b3dd0:     	clz	x0, x0
1007b3dd4:     	cmp	x0, x8
1007b3dd8:     	b.hs	0x1007b4be0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13e4>
1007b3ddc:     	ldr	w0, [x14, x0, lsl #2]
1007b3de0:     	lsl	x0, x17, x0
1007b3de4:     	orr	x16, x0, x16
1007b3de8:     	sub	x0, x15, #0x1
1007b3dec:     	ands	x15, x0, x15
1007b3df0:     	b.ne	0x1007b3dcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x5d0>
1007b3df4:     	orr	x22, x16, x22
1007b3df8:     	eor	w9, w10, w9
1007b3dfc:     	cmp	x12, x13
1007b3e00:     	ccmp	w9, #0x1, #0x0, eq
1007b3e04:     	b.ne	0x1007b3ee0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1007b3e08:     	ldr	x9, [x23, #0x8]
1007b3e0c:     	ldr	x10, [x23, #0x20]
1007b3e10:     	cmp	x9, x10
1007b3e14:     	b.ne	0x1007b3ee0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1007b3e18:     	mov	w16, #0x4               ; =4
1007b3e1c:     	stp	xzr, x16, [sp, #0xf0]
1007b3e20:     	str	xzr, [sp, #0x100]
1007b3e24:     	mov	x20, #0x0               ; =0
1007b3e28:     	cbz	x19, 0x1007b3e88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x68c>
1007b3e2c:     	mov	w8, #0x4                ; =4
1007b3e30:     	b	0x1007b3e58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x65c>
1007b3e34:     	ldr	x8, [sp, #0xf8]
1007b3e38:     	rbit	x9, x19
1007b3e3c:     	clz	x9, x9
1007b3e40:     	str	w9, [x8, x20, lsl #2]
1007b3e44:     	add	x20, x20, #0x1
1007b3e48:     	str	x20, [sp, #0x100]
1007b3e4c:     	sub	x9, x19, #0x1
1007b3e50:     	ands	x19, x9, x19
1007b3e54:     	b.eq	0x1007b3e70 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x674>
1007b3e58:     	ldr	x9, [sp, #0xf0]
1007b3e5c:     	cmp	x20, x9
1007b3e60:     	b.ne	0x1007b3e38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x63c>
1007b3e64:     	add	x0, sp, #0xf0
1007b3e68:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b3e6c:     	b	0x1007b3e34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x638>
1007b3e70:     	ldp	x9, x16, [sp, #0xf0]
1007b3e74:     	ldp	x14, x8, [x28, #0x48]
1007b3e78:     	ldp	x11, x1, [x28, #0x18]
1007b3e7c:     	cmp	x9, #0x0
1007b3e80:     	cset	w19, eq
1007b3e84:     	b	0x1007b3e8c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x690>
1007b3e88:     	mov	w19, #0x1               ; =1
1007b3e8c:     	mov	x9, #0x0                ; =0
1007b3e90:     	lsl	x10, x20, #2
1007b3e94:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3e98:     	add	x2, x2, #0xa20
1007b3e9c:     	adrp	x12, 0x101605000 <dyld_stub_binder+0x101605000>
1007b3ea0:     	add	x12, x12, #0xa38
1007b3ea4:     	cmp	x10, x9
1007b3ea8:     	b.eq	0x1007b45c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdcc>
1007b3eac:     	ldr	w0, [x16, x9]
1007b3eb0:     	cmp	x1, x0
1007b3eb4:     	b.ls	0x1007b4bf0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13f4>
1007b3eb8:     	cmp	x8, x0
1007b3ebc:     	b.ls	0x1007b4bf8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13fc>
1007b3ec0:     	ldr	w13, [x11, x0, lsl #2]
1007b3ec4:     	ldr	w15, [x14, x0, lsl #2]
1007b3ec8:     	add	x9, x9, #0x4
1007b3ecc:     	cmp	w13, w15
1007b3ed0:     	b.eq	0x1007b3ea4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6a8>
1007b3ed4:     	tbnz	w19, #0x0, 0x1007b3ee0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x6e4>
1007b3ed8:     	mov	x0, x16
1007b3edc:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b3ee0:     	fmov	d0, x22
1007b3ee4:     	cnt.8b	v0, v0
1007b3ee8:     	addv.8b	b0, v0
1007b3eec:     	fmov	x19, d0
1007b3ef0:     	cmp	x19, #0xa
1007b3ef4:     	b.hs	0x1007b42a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xaa8>
1007b3ef8:     	add	x8, x28, #0x10
1007b3efc:     	str	x8, [sp, #0x50]
1007b3f00:     	ldr	x8, [x28, #0xd8]
1007b3f04:     	add	x8, x8, #0x1
1007b3f08:     	str	x8, [x28, #0xd8]
1007b3f0c:     	ldr	w8, [x28]
1007b3f10:     	tbz	w8, #0x0, 0x1007b4304 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb08>
1007b3f14:     	str	x19, [sp, #0x8]
1007b3f18:     	mov	x26, #0x0               ; =0
1007b3f1c:     	ldr	x10, [x28, #0x8]
1007b3f20:     	add	x8, x28, #0x90
1007b3f24:     	str	x8, [sp, #0x48]
1007b3f28:     	lsl	x9, x10, #6
1007b3f2c:     	tst	x10, #0xfc00000000000000
1007b3f30:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1007b3f34:     	ccmp	x9, x8, #0x2, eq
1007b3f38:     	cset	w8, hi
1007b3f3c:     	str	w8, [sp, #0x14]
1007b3f40:     	stp	x10, x24, [sp, #0x28]
1007b3f44:     	sub	x8, x10, #0x1
1007b3f48:     	stp	x9, x8, [sp, #0x18]
1007b3f4c:     	mov	w20, #0xff              ; =255
1007b3f50:     	mov	w8, #0x1                ; =1
1007b3f54:     	b	0x1007b3fbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x7c0>
1007b3f58:     	strb	w23, [x28]
1007b3f5c:     	strb	w10, [x28, #0x1]
1007b3f60:     	str	w25, [x28, #0x4]
1007b3f64:     	stp	x9, x27, [x28, #0x8]
1007b3f68:     	ldp	x8, x9, [sp, #0x70]
1007b3f6c:     	stp	x19, x9, [x28, #0x18]
1007b3f70:     	str	x8, [x28, #0x28]
1007b3f74:     	ldr	w8, [sp, #0x58]
1007b3f78:     	stp	w25, w8, [x28, #0x30]
1007b3f7c:     	str	x22, [x28, #0x38]
1007b3f80:     	ldr	x28, [sp, #0x88]
1007b3f84:     	ldr	x23, [sp, #0x60]
1007b3f88:     	ldr	w13, [sp, #0x80]
1007b3f8c:     	mov	w8, #0x0                ; =0
1007b3f90:     	ldr	x9, [x28, #0x130]
1007b3f94:     	ldp	x2, x10, [x28, #0x98]
1007b3f98:     	add	x10, x10, x2, lsl #6
1007b3f9c:     	ldp	x11, x12, [x28, #0xb0]
1007b3fa0:     	add	x10, x12, x10
1007b3fa4:     	add	x10, x10, x11, lsl #6
1007b3fa8:     	cmp	x10, x9
1007b3fac:     	csel	x9, x10, x9, hi
1007b3fb0:     	str	x9, [x28, #0x130]
1007b3fb4:     	mov	w26, #0x1               ; =1
1007b3fb8:     	tbz	w13, #0x0, 0x1007b4798 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf9c>
1007b3fbc:     	mov	x13, x8
1007b3fc0:     	add	x8, x26, x26, lsl #1
1007b3fc4:     	lsl	x8, x8, #3
1007b3fc8:     	add	x9, x23, x8
1007b3fcc:     	ldr	q0, [x9]
1007b3fd0:     	str	q0, [sp, #0xc0]
1007b3fd4:     	ldr	x25, [x9, #0x10]
1007b3fd8:     	str	x25, [sp, #0xd0]
1007b3fdc:     	cmp	w25, #0x2
1007b3fe0:     	b.lo	0x1007b3f8c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x790>
1007b3fe4:     	str	w13, [sp, #0x80]
1007b3fe8:     	ldr	x9, [sp, #0x48]
1007b3fec:     	add	x24, x9, x8
1007b3ff0:     	ldrb	w27, [x28, #0x150]
1007b3ff4:     	ldr	x8, [x24, #0x8]
1007b3ff8:     	cbz	x8, 0x1007b4004 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x808>
1007b3ffc:     	ldr	x0, [x24]
1007b4000:     	b	0x1007b4088 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x88c>
1007b4004:     	ldp	x9, x28, [sp, #0x20]
1007b4008:     	eor	x8, x28, x9
1007b400c:     	cmp	x8, x9
1007b4010:     	b.ls	0x1007b4bbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13c0>
1007b4014:     	ldr	x19, [sp, #0x18]
1007b4018:     	ldr	w8, [sp, #0x14]
1007b401c:     	cbnz	w8, 0x1007b43fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc00>
1007b4020:     	cbz	x19, 0x1007b4038 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x83c>
1007b4024:     	mov	x0, x19
1007b4028:     	mov	w1, #0x8                ; =8
1007b402c:     	bl	0x1012add90 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1007b4030:     	cbnz	x0, 0x1007b403c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x840>
1007b4034:     	b	0x1007b4c94 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1498>
1007b4038:     	mov	w0, #0x8                ; =8
1007b403c:     	mov	x8, x0
1007b4040:     	mov	x9, x28
1007b4044:     	cmp	x28, #0x4
1007b4048:     	b.hs	0x1007b405c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x860>
1007b404c:     	strb	w20, [x8], #0x40
1007b4050:     	subs	x9, x9, #0x1
1007b4054:     	b.ne	0x1007b404c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x850>
1007b4058:     	b	0x1007b4080 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x884>
1007b405c:     	add	x8, x0, #0x80
1007b4060:     	and	x9, x28, #0x3fffffffffffffc
1007b4064:     	sturb	w20, [x8, #-0x80]
1007b4068:     	sturb	w20, [x8, #-0x40]
1007b406c:     	strb	w20, [x8]
1007b4070:     	strb	w20, [x8, #0x40]
1007b4074:     	add	x8, x8, #0x100
1007b4078:     	subs	x9, x9, #0x4
1007b407c:     	b.ne	0x1007b4064 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x868>
1007b4080:     	stp	x0, x28, [x24]
1007b4084:     	mov	x8, x28
1007b4088:     	mov	w9, w25
1007b408c:     	ldp	x11, x12, [sp, #0xc0]
1007b4090:     	ldr	w19, [sp, #0xd4]
1007b4094:     	mov	x10, #0xa9c5            ; =43461
1007b4098:     	movk	x10, #0x2e62, lsl #16
1007b409c:     	movk	x10, #0x7aea, lsl #32
1007b40a0:     	movk	x10, #0xf135, lsl #48
1007b40a4:     	stp	x12, x11, [sp, #0x70]
1007b40a8:     	madd	x9, x9, x10, x11
1007b40ac:     	madd	x9, x9, x10, x12
1007b40b0:     	madd	x9, x9, x10, x22
1007b40b4:     	mul	x9, x9, x10
1007b40b8:     	sub	x8, x8, #0x1
1007b40bc:     	and	x8, x8, x9, ror #44
1007b40c0:     	add	x28, x0, x8, lsl #6
1007b40c4:     	ldrb	w8, [x28]
1007b40c8:     	cmp	w8, #0xff
1007b40cc:     	b.ne	0x1007b40f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x8f4>
1007b40d0:     	ldr	x9, [sp, #0x88]
1007b40d4:     	ldr	x8, [x9, #0x120]
1007b40d8:     	add	x8, x8, #0x1
1007b40dc:     	str	x8, [x9, #0x120]
1007b40e0:     	add	x8, x28, #0x10
1007b40e4:     	str	x8, [sp, #0x38]
1007b40e8:     	add	x23, x28, #0x18
1007b40ec:     	b	0x1007b4194 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
1007b40f0:     	ldr	x9, [x28, #0x38]
1007b40f4:     	cmp	x9, x22
1007b40f8:     	b.ne	0x1007b413c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1007b40fc:     	ldr	x9, [x28, #0x20]
1007b4100:     	ldr	x10, [sp, #0x78]
1007b4104:     	cmp	x9, x10
1007b4108:     	b.ne	0x1007b413c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1007b410c:     	ldr	x9, [x28, #0x28]
1007b4110:     	ldr	x10, [sp, #0x70]
1007b4114:     	cmp	x9, x10
1007b4118:     	b.ne	0x1007b413c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1007b411c:     	ldr	w9, [x28, #0x30]
1007b4120:     	cmp	w9, w25
1007b4124:     	b.ne	0x1007b413c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x940>
1007b4128:     	ldr	x28, [sp, #0x88]
1007b412c:     	ldr	x8, [x28, #0x118]
1007b4130:     	add	x8, x8, #0x1
1007b4134:     	str	x8, [x28, #0x118]
1007b4138:     	b	0x1007b3f88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x78c>
1007b413c:     	mov	x23, x28
1007b4140:     	ldr	x9, [x23, #0x18]!
1007b4144:     	lsl	x10, x9, #3
1007b4148:     	cmp	w8, #0x2
1007b414c:     	csel	x10, x10, xzr, eq
1007b4150:     	ldr	x11, [x24, #0x10]
1007b4154:     	sub	x10, x11, x10
1007b4158:     	cmp	w8, #0x2
1007b415c:     	mov	x8, x28
1007b4160:     	ldr	x0, [x8, #0x10]!
1007b4164:     	str	x8, [sp, #0x38]
1007b4168:     	strb	w20, [x28]
1007b416c:     	ldr	x8, [sp, #0x88]
1007b4170:     	ldr	q0, [x8, #0x120]
1007b4174:     	mov	w11, #0x1               ; =1
1007b4178:     	dup.2d	v1, x11
1007b417c:     	add.2d	v0, v0, v1
1007b4180:     	str	q0, [x8, #0x120]
1007b4184:     	str	x10, [x24, #0x10]
1007b4188:     	ccmp	x9, #0x0, #0x4, hs
1007b418c:     	b.eq	0x1007b4194 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x998>
1007b4190:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4194:     	ldr	x8, [sp, #0x50]
1007b4198:     	mov	w9, #0x30               ; =48
1007b419c:     	madd	x3, x26, x9, x8
1007b41a0:     	add	x0, sp, #0xf0
1007b41a4:     	add	x2, sp, #0xc0
1007b41a8:     	mov	x1, x21
1007b41ac:     	mov	x4, x22
1007b41b0:     	mov	x5, x27
1007b41b4:     	ldr	x6, [sp, #0x30]
1007b41b8:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b41bc:     	ldr	x8, [sp, #0xf0]
1007b41c0:     	cmn	x8, #0x1
1007b41c4:     	str	w19, [sp, #0x58]
1007b41c8:     	str	x23, [sp, #0x40]
1007b41cc:     	b.eq	0x1007b41e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9e8>
1007b41d0:     	cmn	x8, #0x2
1007b41d4:     	b.ne	0x1007b4208 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa0c>
1007b41d8:     	mov	w23, #0x0               ; =0
1007b41dc:     	ldrb	w10, [sp, #0xf8]
1007b41e0:     	b	0x1007b41e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x9ec>
1007b41e4:     	mov	w23, #0x1               ; =1
1007b41e8:     	mov	x26, #0x0               ; =0
1007b41ec:     	ldr	x8, [x24, #0x10]
1007b41f0:     	add	x8, x8, x26
1007b41f4:     	str	x8, [x24, #0x10]
1007b41f8:     	ldrb	w8, [x28]
1007b41fc:     	cmp	w8, #0x2
1007b4200:     	b.ne	0x1007b3f58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1007b4204:     	b	0x1007b4278 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa7c>
1007b4208:     	ldp	x27, x19, [sp, #0xf8]
1007b420c:     	ldr	x9, [sp, #0x108]
1007b4210:     	lsl	x26, x19, #3
1007b4214:     	cmp	x8, x19
1007b4218:     	b.ls	0x1007b425c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
1007b421c:     	mov	x23, x9
1007b4220:     	str	x27, [sp, #0x68]
1007b4224:     	cbz	x19, 0x1007b424c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa50>
1007b4228:     	lsl	x1, x8, #3
1007b422c:     	ldr	x0, [sp, #0x68]
1007b4230:     	mov	w2, #0x8                ; =8
1007b4234:     	mov	x3, x26
1007b4238:     	bl	0x1012adde4 <__RNvCsiwXPDrQxTLA_7___rustc14___rust_realloc>
1007b423c:     	mov	x27, x0
1007b4240:     	mov	x9, x23
1007b4244:     	cbnz	x0, 0x1007b425c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xa60>
1007b4248:     	b	0x1007b4ca0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14a4>
1007b424c:     	ldr	x0, [sp, #0x68]
1007b4250:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4254:     	mov	w27, #0x8               ; =8
1007b4258:     	mov	x9, x23
1007b425c:     	mov	w23, #0x2               ; =2
1007b4260:     	ldr	x8, [x24, #0x10]
1007b4264:     	add	x8, x8, x26
1007b4268:     	str	x8, [x24, #0x10]
1007b426c:     	ldrb	w8, [x28]
1007b4270:     	cmp	w8, #0x2
1007b4274:     	b.ne	0x1007b3f58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1007b4278:     	ldr	x8, [sp, #0x40]
1007b427c:     	ldr	x8, [x8]
1007b4280:     	cbz	x8, 0x1007b3f58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1007b4284:     	ldr	x8, [sp, #0x38]
1007b4288:     	ldr	x0, [x8]
1007b428c:     	mov	x24, x9
1007b4290:     	mov	x26, x10
1007b4294:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4298:     	mov	x10, x26
1007b429c:     	mov	x9, x24
1007b42a0:     	b	0x1007b3f58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x75c>
1007b42a4:     	mov	x0, x21
1007b42a8:     	mov	x1, x22
1007b42ac:     	bl	0x100c9e180 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
1007b42b0:     	ldr	q0, [x23]
1007b42b4:     	str	q0, [sp, #0xc0]
1007b42b8:     	ldr	x8, [x23, #0x10]
1007b42bc:     	str	x8, [sp, #0xd0]
1007b42c0:     	mov	w22, w0
1007b42c4:     	ldr	x1, [x28, #0x38]
1007b42c8:     	cmp	x1, x22
1007b42cc:     	b.ls	0x1007b4c30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
1007b42d0:     	ldr	x8, [x28, #0x30]
1007b42d4:     	ldr	w8, [x8, x22, lsl #2]
1007b42d8:     	ldr	w9, [sp, #0xd0]
1007b42dc:     	ldr	x10, [x21, #0x40]
1007b42e0:     	lsr	x0, x9, #1
1007b42e4:     	cmn	x10, #0x1
1007b42e8:     	b.eq	0x1007b4400 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc04>
1007b42ec:     	ldr	x1, [x21, #0x50]
1007b42f0:     	cmp	x1, x0
1007b42f4:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b42f8:     	ldr	x9, [x21, #0x48]
1007b42fc:     	add	x9, x9, x0, lsl #4
1007b4300:     	b	0x1007b4418 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc1c>
1007b4304:     	ldrb	w5, [x28, #0x150]
1007b4308:     	add	x0, sp, #0xc0
1007b430c:     	add	x3, x28, #0x10
1007b4310:     	mov	x1, x21
1007b4314:     	mov	x2, x23
1007b4318:     	mov	x4, x22
1007b431c:     	mov	x6, x24
1007b4320:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b4324:     	ldrb	w5, [x28, #0x150]
1007b4328:     	add	x0, sp, #0xf0
1007b432c:     	add	x2, x23, #0x18
1007b4330:     	add	x3, x28, #0x40
1007b4334:     	mov	x1, x21
1007b4338:     	mov	x4, x22
1007b433c:     	mov	x6, x24
1007b4340:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1007b4344:     	mov	w8, #0x1                ; =1
1007b4348:     	lsl	x8, x8, x19
1007b434c:     	lsr	x8, x8, #6
1007b4350:     	cmp	x19, #0x6
1007b4354:     	cinc	x20, x8, lo
1007b4358:     	cbz	x20, 0x1007b4880 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
1007b435c:     	lsl	x24, x20, #3
1007b4360:     	mov	x0, x24
1007b4364:     	mov	w1, #0x8                ; =8
1007b4368:     	bl	0x1012add90 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1007b436c:     	cbz	x0, 0x1007b4c84 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1488>
1007b4370:     	mov	x25, x0
1007b4374:     	mov	x0, #0x0                ; =0
1007b4378:     	ldp	x8, x9, [sp, #0xc0]
1007b437c:     	ldp	x1, x10, [sp, #0xd0]
1007b4380:     	sub	x11, x0, w9, uxtb
1007b4384:     	ldp	x23, x13, [sp, #0xf0]
1007b4388:     	ldp	x12, x14, [sp, #0x100]
1007b438c:     	mov	x24, x25
1007b4390:     	b	0x1007b43ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
1007b4394:     	tst	w13, #0x1
1007b4398:     	csel	x15, x15, xzr, ne
1007b439c:     	str	x15, [x24, x0, lsl #3]
1007b43a0:     	add	x0, x0, #0x1
1007b43a4:     	cmp	x20, x0
1007b43a8:     	b.eq	0x1007b43f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1007b43ac:     	mov	x15, x11
1007b43b0:     	cmn	x8, #0x2
1007b43b4:     	b.eq	0x1007b43c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbcc>
1007b43b8:     	cmp	x0, x1
1007b43bc:     	b.hs	0x1007b4c10 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1414>
1007b43c0:     	ldr	x15, [x9, x0, lsl #3]
1007b43c4:     	eor	x15, x10, x15
1007b43c8:     	cmn	x23, #0x2
1007b43cc:     	b.eq	0x1007b4394 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xb98>
1007b43d0:     	cmp	x0, x12
1007b43d4:     	b.hs	0x1007b4c0c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1410>
1007b43d8:     	ldr	x16, [x13, x0, lsl #3]
1007b43dc:     	eor	x16, x14, x16
1007b43e0:     	and	x15, x16, x15
1007b43e4:     	str	x15, [x24, x0, lsl #3]
1007b43e8:     	add	x0, x0, #0x1
1007b43ec:     	cmp	x20, x0
1007b43f0:     	b.ne	0x1007b43ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbb0>
1007b43f4:     	mov	x27, x20
1007b43f8:     	b	0x1007b488c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1090>
1007b43fc:     	bl	0x1013b9b90 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1007b4400:     	ldr	x1, [x21, #0x58]
1007b4404:     	cmp	x1, x0
1007b4408:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b440c:     	ldr	x9, [x21, #0x50]
1007b4410:     	add	x9, x9, x0, lsl #5
1007b4414:     	add	x9, x9, #0x18
1007b4418:     	ldr	x9, [x9]
1007b441c:     	mov	w10, #0x1               ; =1
1007b4420:     	lsl	x8, x10, x8
1007b4424:     	tst	x9, x8
1007b4428:     	b.eq	0x1007b443c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xc40>
1007b442c:     	ldp	x9, x10, [sp, #0xc0]
1007b4430:     	orr	x9, x9, x8
1007b4434:     	bic	x8, x10, x8
1007b4438:     	stp	x9, x8, [sp, #0xc0]
1007b443c:     	sub	x0, x29, #0x70
1007b4440:     	add	x1, sp, #0xc0
1007b4444:     	mov	x2, x21
1007b4448:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b444c:     	ldur	q0, [x29, #-0x70]
1007b4450:     	stur	q0, [x29, #-0x90]
1007b4454:     	ldur	x8, [x29, #-0x60]
1007b4458:     	stur	q0, [x29, #-0xb0]
1007b445c:     	str	q0, [sp, #0x90]
1007b4460:     	str	x8, [sp, #0xa0]
1007b4464:     	ldr	q0, [sp, #0x90]
1007b4468:     	str	x8, [sp, #0x100]
1007b446c:     	str	q0, [sp, #0xf0]
1007b4470:     	ldur	q0, [x23, #0x18]
1007b4474:     	str	q0, [sp, #0xc0]
1007b4478:     	ldur	x8, [x23, #0x28]
1007b447c:     	str	x8, [sp, #0xd0]
1007b4480:     	ldr	x1, [x28, #0x68]
1007b4484:     	cmp	x1, x22
1007b4488:     	b.ls	0x1007b4c30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1434>
1007b448c:     	ldr	x8, [x28, #0x60]
1007b4490:     	ldr	w8, [x8, x22, lsl #2]
1007b4494:     	ldr	w9, [sp, #0xd0]
1007b4498:     	ldr	x10, [x21, #0x40]
1007b449c:     	lsr	x0, x9, #1
1007b44a0:     	cmn	x10, #0x1
1007b44a4:     	b.eq	0x1007b44c0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcc4>
1007b44a8:     	ldr	x1, [x21, #0x50]
1007b44ac:     	cmp	x1, x0
1007b44b0:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b44b4:     	ldr	x9, [x21, #0x48]
1007b44b8:     	add	x9, x9, x0, lsl #4
1007b44bc:     	b	0x1007b44d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xcdc>
1007b44c0:     	ldr	x1, [x21, #0x58]
1007b44c4:     	cmp	x1, x0
1007b44c8:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b44cc:     	ldr	x9, [x21, #0x50]
1007b44d0:     	add	x9, x9, x0, lsl #5
1007b44d4:     	add	x9, x9, #0x18
1007b44d8:     	ldr	x9, [x9]
1007b44dc:     	mov	w10, #0x1               ; =1
1007b44e0:     	lsl	x8, x10, x8
1007b44e4:     	tst	x9, x8
1007b44e8:     	b.eq	0x1007b44fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd00>
1007b44ec:     	ldp	x9, x10, [sp, #0xc0]
1007b44f0:     	orr	x9, x9, x8
1007b44f4:     	bic	x8, x10, x8
1007b44f8:     	stp	x9, x8, [sp, #0xc0]
1007b44fc:     	sub	x0, x29, #0x70
1007b4500:     	add	x1, sp, #0xc0
1007b4504:     	mov	x2, x21
1007b4508:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b450c:     	ldur	q0, [x29, #-0x70]
1007b4510:     	stur	q0, [x29, #-0x90]
1007b4514:     	ldur	x8, [x29, #-0x60]
1007b4518:     	stur	q0, [x29, #-0xb0]
1007b451c:     	str	q0, [sp, #0x90]
1007b4520:     	str	x8, [sp, #0xa0]
1007b4524:     	ldr	q0, [sp, #0x90]
1007b4528:     	str	x8, [sp, #0x118]
1007b452c:     	add	x8, sp, #0x9
1007b4530:     	stur	q0, [x8, #0xff]
1007b4534:     	ldp	q0, q1, [sp, #0xf0]
1007b4538:     	ldr	q2, [sp, #0x110]
1007b453c:     	stp	q1, q2, [sp, #0xa0]
1007b4540:     	str	q0, [sp, #0x90]
1007b4544:     	add	x2, sp, #0x90
1007b4548:     	mov	x0, x28
1007b454c:     	mov	x1, x21
1007b4550:     	bl	0x1007b37fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1007b4554:     	mov	x23, x0
1007b4558:     	cmp	w0, #0x1
1007b455c:     	b.ne	0x1007b4574 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
1007b4560:     	ldr	x8, [x28, #0xc0]
1007b4564:     	lsr	x8, x8, x22
1007b4568:     	tbz	w8, #0x0, 0x1007b4574 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xd78>
1007b456c:     	mov	w8, #0x1                ; =1
1007b4570:     	b	0x1007b4780 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf84>
1007b4574:     	ldr	x8, [sp, #0x60]
1007b4578:     	ldr	q0, [x8]
1007b457c:     	stur	q0, [x29, #-0x70]
1007b4580:     	ldr	x8, [x8, #0x10]
1007b4584:     	stur	x8, [x29, #-0x60]
1007b4588:     	ldr	x1, [x28, #0x38]
1007b458c:     	cmp	x1, x22
1007b4590:     	b.ls	0x1007b4c64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
1007b4594:     	ldr	x8, [x28, #0x30]
1007b4598:     	ldr	w8, [x8, x22, lsl #2]
1007b459c:     	ldur	w9, [x29, #-0x60]
1007b45a0:     	ldr	x10, [x21, #0x40]
1007b45a4:     	lsr	x0, x9, #1
1007b45a8:     	cmn	x10, #0x1
1007b45ac:     	b.eq	0x1007b45f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdf4>
1007b45b0:     	ldr	x1, [x21, #0x50]
1007b45b4:     	cmp	x1, x0
1007b45b8:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b45bc:     	ldr	x9, [x21, #0x48]
1007b45c0:     	add	x9, x9, x0, lsl #4
1007b45c4:     	b	0x1007b4608 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe0c>
1007b45c8:     	tbz	w19, #0x0, 0x1007b4788 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf8c>
1007b45cc:     	mov	w0, #0x0                ; =0
1007b45d0:     	add	sp, sp, #0x1c0
1007b45d4:     	ldp	x29, x30, [sp, #0x50]
1007b45d8:     	ldp	x20, x19, [sp, #0x40]
1007b45dc:     	ldp	x22, x21, [sp, #0x30]
1007b45e0:     	ldp	x24, x23, [sp, #0x20]
1007b45e4:     	ldp	x26, x25, [sp, #0x10]
1007b45e8:     	ldp	x28, x27, [sp], #0x60
1007b45ec:     	ret
1007b45f0:     	ldr	x1, [x21, #0x58]
1007b45f4:     	cmp	x1, x0
1007b45f8:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b45fc:     	ldr	x9, [x21, #0x50]
1007b4600:     	add	x9, x9, x0, lsl #5
1007b4604:     	add	x9, x9, #0x18
1007b4608:     	ldr	x9, [x9]
1007b460c:     	mov	w10, #0x1               ; =1
1007b4610:     	lsl	x8, x10, x8
1007b4614:     	tst	x9, x8
1007b4618:     	b.eq	0x1007b462c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xe30>
1007b461c:     	ldur	q0, [x29, #-0x70]
1007b4620:     	dup.2d	v1, x8
1007b4624:     	orr.16b	v0, v0, v1
1007b4628:     	stur	q0, [x29, #-0x70]
1007b462c:     	sub	x0, x29, #0xb0
1007b4630:     	sub	x1, x29, #0x70
1007b4634:     	mov	x2, x21
1007b4638:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b463c:     	ldur	q0, [x29, #-0xb0]
1007b4640:     	stur	q0, [x29, #-0xd0]
1007b4644:     	ldur	x8, [x29, #-0xa0]
1007b4648:     	stur	q0, [x29, #-0xf0]
1007b464c:     	stur	q0, [x29, #-0x90]
1007b4650:     	stur	x8, [x29, #-0x80]
1007b4654:     	ldur	q0, [x29, #-0x90]
1007b4658:     	str	x8, [sp, #0x100]
1007b465c:     	str	q0, [sp, #0xf0]
1007b4660:     	ldr	x8, [sp, #0x60]
1007b4664:     	ldur	q0, [x8, #0x18]
1007b4668:     	stur	q0, [x29, #-0x70]
1007b466c:     	ldur	x8, [x8, #0x28]
1007b4670:     	stur	x8, [x29, #-0x60]
1007b4674:     	ldr	x1, [x28, #0x68]
1007b4678:     	cmp	x1, x22
1007b467c:     	b.ls	0x1007b4c64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1468>
1007b4680:     	ldr	x8, [x28, #0x60]
1007b4684:     	ldr	w8, [x8, x22, lsl #2]
1007b4688:     	ldur	w9, [x29, #-0x60]
1007b468c:     	ldr	x10, [x21, #0x40]
1007b4690:     	lsr	x0, x9, #1
1007b4694:     	cmn	x10, #0x1
1007b4698:     	b.eq	0x1007b46b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xeb8>
1007b469c:     	ldr	x1, [x21, #0x50]
1007b46a0:     	cmp	x1, x0
1007b46a4:     	b.ls	0x1007b4c44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1448>
1007b46a8:     	ldr	x9, [x21, #0x48]
1007b46ac:     	add	x9, x9, x0, lsl #4
1007b46b0:     	b	0x1007b46cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xed0>
1007b46b4:     	ldr	x1, [x21, #0x58]
1007b46b8:     	cmp	x1, x0
1007b46bc:     	b.ls	0x1007b4c78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x147c>
1007b46c0:     	ldr	x9, [x21, #0x50]
1007b46c4:     	add	x9, x9, x0, lsl #5
1007b46c8:     	add	x9, x9, #0x18
1007b46cc:     	and	w19, w22, #0x3f
1007b46d0:     	ldr	x9, [x9]
1007b46d4:     	mov	w10, #0x1               ; =1
1007b46d8:     	lsl	x8, x10, x8
1007b46dc:     	tst	x9, x8
1007b46e0:     	b.eq	0x1007b46f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xef8>
1007b46e4:     	ldur	q0, [x29, #-0x70]
1007b46e8:     	dup.2d	v1, x8
1007b46ec:     	orr.16b	v0, v0, v1
1007b46f0:     	stur	q0, [x29, #-0x70]
1007b46f4:     	sub	x0, x29, #0xb0
1007b46f8:     	sub	x1, x29, #0x70
1007b46fc:     	mov	x2, x21
1007b4700:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1007b4704:     	ldur	q0, [x29, #-0xb0]
1007b4708:     	stur	q0, [x29, #-0xd0]
1007b470c:     	ldur	x8, [x29, #-0xa0]
1007b4710:     	stur	q0, [x29, #-0xf0]
1007b4714:     	stur	q0, [x29, #-0x90]
1007b4718:     	stur	x8, [x29, #-0x80]
1007b471c:     	ldur	q0, [x29, #-0x90]
1007b4720:     	str	x8, [sp, #0x118]
1007b4724:     	add	x8, sp, #0x9
1007b4728:     	stur	q0, [x8, #0xff]
1007b472c:     	ldp	q0, q1, [sp, #0xf0]
1007b4730:     	ldr	q2, [sp, #0x110]
1007b4734:     	stp	q1, q2, [sp, #0xd0]
1007b4738:     	str	q0, [sp, #0xc0]
1007b473c:     	add	x2, sp, #0xc0
1007b4740:     	mov	x0, x28
1007b4744:     	mov	x1, x21
1007b4748:     	bl	0x1007b37fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_>
1007b474c:     	mov	x3, x0
1007b4750:     	ldr	x8, [x28, #0xc0]
1007b4754:     	mov	x0, x21
1007b4758:     	lsr	x8, x8, x19
1007b475c:     	tbz	w8, #0x0, 0x1007b4770 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf74>
1007b4760:     	mov	w1, #0xe                ; =14
1007b4764:     	mov	x2, x23
1007b4768:     	bl	0x100cad780 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1007b476c:     	b	0x1007b477c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xf80>
1007b4770:     	mov	x1, x22
1007b4774:     	mov	x2, x23
1007b4778:     	bl	0x100cae1cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
1007b477c:     	mov	x8, x0
1007b4780:     	ldr	x19, [sp, #0x60]
1007b4784:     	b	0x1007b4b18 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x131c>
1007b4788:     	mov	x0, x16
1007b478c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4790:     	mov	w0, #0x0                ; =0
1007b4794:     	b	0x1007b45d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1007b4798:     	ldr	x1, [x28, #0x90]
1007b479c:     	add	x0, sp, #0xc0
1007b47a0:     	mov	x3, x21
1007b47a4:     	mov	x4, x23
1007b47a8:     	mov	x5, x22
1007b47ac:     	bl	0x1007ae438 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1007b47b0:     	ldp	x1, x2, [x28, #0xa8]
1007b47b4:     	add	x0, sp, #0xf0
1007b47b8:     	add	x4, x23, #0x18
1007b47bc:     	mov	x3, x21
1007b47c0:     	mov	x5, x22
1007b47c4:     	bl	0x1007ae438 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1007b47c8:     	mov	w8, #0x1                ; =1
1007b47cc:     	ldr	x9, [sp, #0x8]
1007b47d0:     	lsl	x8, x8, x9
1007b47d4:     	lsr	x8, x8, #6
1007b47d8:     	cmp	x9, #0x6
1007b47dc:     	cinc	x20, x8, lo
1007b47e0:     	cbz	x20, 0x1007b4880 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1084>
1007b47e4:     	lsl	x24, x20, #3
1007b47e8:     	mov	x0, x24
1007b47ec:     	mov	w1, #0x8                ; =8
1007b47f0:     	bl	0x1012add90 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1007b47f4:     	cbz	x0, 0x1007b4cb0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14b4>
1007b47f8:     	mov	x25, x0
1007b47fc:     	mov	x0, #0x0                ; =0
1007b4800:     	ldp	x8, x9, [sp, #0xc0]
1007b4804:     	ldp	x1, x10, [sp, #0xd0]
1007b4808:     	sub	x11, x0, w9, uxtb
1007b480c:     	ldp	x23, x13, [sp, #0xf0]
1007b4810:     	ldp	x12, x14, [sp, #0x100]
1007b4814:     	mov	x24, x25
1007b4818:     	b	0x1007b4834 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
1007b481c:     	tst	w13, #0x1
1007b4820:     	csel	x15, x15, xzr, ne
1007b4824:     	str	x15, [x24, x0, lsl #3]
1007b4828:     	add	x0, x0, #0x1
1007b482c:     	cmp	x20, x0
1007b4830:     	b.eq	0x1007b43f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1007b4834:     	mov	x15, x11
1007b4838:     	cmn	x8, #0x2
1007b483c:     	b.eq	0x1007b4850 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1054>
1007b4840:     	cmp	x0, x1
1007b4844:     	b.hs	0x1007b4c54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1458>
1007b4848:     	ldr	x15, [x9, x0, lsl #3]
1007b484c:     	eor	x15, x10, x15
1007b4850:     	cmn	x23, #0x2
1007b4854:     	b.eq	0x1007b481c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1020>
1007b4858:     	cmp	x0, x12
1007b485c:     	b.hs	0x1007b4c50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1454>
1007b4860:     	ldr	x16, [x13, x0, lsl #3]
1007b4864:     	eor	x16, x14, x16
1007b4868:     	and	x15, x16, x15
1007b486c:     	str	x15, [x24, x0, lsl #3]
1007b4870:     	add	x0, x0, #0x1
1007b4874:     	cmp	x20, x0
1007b4878:     	b.ne	0x1007b4834 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1038>
1007b487c:     	b	0x1007b43f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xbf8>
1007b4880:     	mov	x27, #0x0               ; =0
1007b4884:     	ldr	x23, [sp, #0xf0]
1007b4888:     	mov	w24, #0x8               ; =8
1007b488c:     	cmp	x23, #0x1
1007b4890:     	b.lt	0x1007b489c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10a0>
1007b4894:     	ldr	x0, [sp, #0xf8]
1007b4898:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b489c:     	ldr	x8, [sp, #0xc0]
1007b48a0:     	cmp	x8, #0x1
1007b48a4:     	b.lt	0x1007b48b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10b4>
1007b48a8:     	ldr	x0, [sp, #0xc8]
1007b48ac:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b48b0:     	ldr	x8, [x28, #0xc0]
1007b48b4:     	mov	w9, #0x4                ; =4
1007b48b8:     	stp	xzr, x9, [sp, #0xf0]
1007b48bc:     	str	xzr, [sp, #0x100]
1007b48c0:     	ands	x23, x8, x22
1007b48c4:     	b.eq	0x1007b4af8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12fc>
1007b48c8:     	mov	x19, #0x0               ; =0
1007b48cc:     	mov	w8, #0x4                ; =4
1007b48d0:     	b	0x1007b48f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10fc>
1007b48d4:     	ldr	x8, [sp, #0xf8]
1007b48d8:     	rbit	x9, x23
1007b48dc:     	clz	x9, x9
1007b48e0:     	str	w9, [x8, x19, lsl #2]
1007b48e4:     	add	x19, x19, #0x1
1007b48e8:     	str	x19, [sp, #0x100]
1007b48ec:     	sub	x9, x23, #0x1
1007b48f0:     	ands	x23, x9, x23
1007b48f4:     	b.eq	0x1007b4910 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1114>
1007b48f8:     	ldr	x9, [sp, #0xf0]
1007b48fc:     	cmp	x19, x9
1007b4900:     	b.ne	0x1007b48d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10dc>
1007b4904:     	add	x0, sp, #0xf0
1007b4908:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1007b490c:     	b	0x1007b48d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x10d8>
1007b4910:     	ldp	x9, x8, [sp, #0xf0]
1007b4914:     	str	x9, [sp, #0x70]
1007b4918:     	str	x8, [sp, #0x58]
1007b491c:     	cbz	x19, 0x1007b4ad8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12dc>
1007b4920:     	mov	x23, x8
1007b4924:     	mov	x25, x20
1007b4928:     	add	x8, x8, x19, lsl #2
1007b492c:     	str	x8, [sp, #0x78]
1007b4930:     	b	0x1007b4950 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1154>
1007b4934:     	bic	x22, x22, x20
1007b4938:     	mov	x24, x26
1007b493c:     	mov	x27, x25
1007b4940:     	mov	x20, x25
1007b4944:     	ldr	x8, [sp, #0x78]
1007b4948:     	cmp	x23, x8
1007b494c:     	b.eq	0x1007b4ae0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12e4>
1007b4950:     	str	x27, [sp, #0x80]
1007b4954:     	ldr	w8, [x23], #0x4
1007b4958:     	mov	w9, #0x1                ; =1
1007b495c:     	lsl	x20, x9, x8
1007b4960:     	sub	x8, x20, #0x1
1007b4964:     	and	x8, x8, x22
1007b4968:     	fmov	d0, x8
1007b496c:     	cnt.8b	v0, v0
1007b4970:     	addv.8b	b0, v0
1007b4974:     	fmov	w26, s0
1007b4978:     	fmov	d0, x22
1007b497c:     	cnt.8b	v0, v0
1007b4980:     	addv.8b	b0, v0
1007b4984:     	fmov	w27, s0
1007b4988:     	add	x0, sp, #0xc0
1007b498c:     	mov	x1, x24
1007b4990:     	mov	x2, x25
1007b4994:     	mov	x3, x27
1007b4998:     	mov	x4, x26
1007b499c:     	mov	w5, #0x0                ; =0
1007b49a0:     	bl	0x100e44498 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1007b49a4:     	add	x0, sp, #0xf0
1007b49a8:     	mov	x1, x24
1007b49ac:     	mov	x2, x25
1007b49b0:     	mov	x3, x27
1007b49b4:     	mov	x4, x26
1007b49b8:     	mov	w5, #0x1                ; =1
1007b49bc:     	bl	0x100e44498 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1007b49c0:     	ldp	x19, x8, [sp, #0xc8]
1007b49c4:     	ldp	x27, x28, [sp, #0xf0]
1007b49c8:     	ldr	x9, [sp, #0x100]
1007b49cc:     	cmp	x9, x8
1007b49d0:     	csel	x25, x9, x8, lo
1007b49d4:     	cbz	x25, 0x1007b4a30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1234>
1007b49d8:     	str	x24, [sp, #0x68]
1007b49dc:     	lsl	x24, x25, #3
1007b49e0:     	mov	x0, x24
1007b49e4:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
1007b49e8:     	cbz	x0, 0x1007b4c20 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1424>
1007b49ec:     	mov	x26, x0
1007b49f0:     	cmp	x25, #0x8
1007b49f4:     	b.hs	0x1007b4a68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x126c>
1007b49f8:     	mov	x8, #0x0                ; =0
1007b49fc:     	ldr	x24, [sp, #0x68]
1007b4a00:     	lsl	x11, x8, #3
1007b4a04:     	add	x9, x19, x11
1007b4a08:     	add	x10, x28, x11
1007b4a0c:     	add	x11, x26, x11
1007b4a10:     	sub	x8, x25, x8
1007b4a14:     	ldr	x12, [x10], #0x8
1007b4a18:     	ldr	x13, [x9], #0x8
1007b4a1c:     	orr	x12, x13, x12
1007b4a20:     	str	x12, [x11], #0x8
1007b4a24:     	subs	x8, x8, #0x1
1007b4a28:     	b.ne	0x1007b4a14 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1218>
1007b4a2c:     	b	0x1007b4a34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
1007b4a30:     	mov	w26, #0x8               ; =8
1007b4a34:     	cbz	x27, 0x1007b4a40 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1244>
1007b4a38:     	mov	x0, x28
1007b4a3c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4a40:     	ldr	x8, [sp, #0x80]
1007b4a44:     	cbz	x8, 0x1007b4a50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1254>
1007b4a48:     	mov	x0, x24
1007b4a4c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4a50:     	ldr	x8, [sp, #0xc0]
1007b4a54:     	ldr	x28, [sp, #0x88]
1007b4a58:     	cbz	x8, 0x1007b4934 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
1007b4a5c:     	mov	x0, x19
1007b4a60:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4a64:     	b	0x1007b4934 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1138>
1007b4a68:     	mov	x8, #0x0                ; =0
1007b4a6c:     	sub	x9, x28, x26
1007b4a70:     	cmn	x9, #0x40
1007b4a74:     	ldr	x24, [sp, #0x68]
1007b4a78:     	b.hi	0x1007b4a00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1007b4a7c:     	sub	x9, x19, x26
1007b4a80:     	cmn	x9, #0x40
1007b4a84:     	b.hi	0x1007b4a00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1007b4a88:     	and	x8, x25, #0xffffffffffffff8
1007b4a8c:     	add	x9, x19, #0x20
1007b4a90:     	add	x10, x28, #0x20
1007b4a94:     	add	x11, x26, #0x20
1007b4a98:     	and	x12, x25, #0xffffffffffffff8
1007b4a9c:     	ldp	q0, q1, [x10, #-0x20]
1007b4aa0:     	ldp	q2, q3, [x10], #0x40
1007b4aa4:     	ldp	q4, q5, [x9, #-0x20]
1007b4aa8:     	ldp	q6, q7, [x9], #0x40
1007b4aac:     	orr.16b	v0, v4, v0
1007b4ab0:     	orr.16b	v1, v5, v1
1007b4ab4:     	orr.16b	v2, v6, v2
1007b4ab8:     	orr.16b	v3, v7, v3
1007b4abc:     	stp	q0, q1, [x11, #-0x20]
1007b4ac0:     	stp	q2, q3, [x11], #0x40
1007b4ac4:     	subs	x12, x12, #0x8
1007b4ac8:     	b.ne	0x1007b4a9c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12a0>
1007b4acc:     	cmp	x25, x8
1007b4ad0:     	b.ne	0x1007b4a00 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1204>
1007b4ad4:     	b	0x1007b4a34 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1238>
1007b4ad8:     	mov	x25, x27
1007b4adc:     	mov	x26, x24
1007b4ae0:     	ldr	x8, [sp, #0x70]
1007b4ae4:     	cbz	x8, 0x1007b4af0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x12f4>
1007b4ae8:     	ldr	x0, [sp, #0x58]
1007b4aec:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4af0:     	mov	x24, x26
1007b4af4:     	mov	x27, x25
1007b4af8:     	ldr	x19, [sp, #0x60]
1007b4afc:     	stp	x27, x24, [sp, #0xf0]
1007b4b00:     	str	x20, [sp, #0x100]
1007b4b04:     	add	x2, sp, #0xf0
1007b4b08:     	mov	x0, x21
1007b4b0c:     	mov	x1, x22
1007b4b10:     	bl	0x100cadbe4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
1007b4b14:     	mov	x8, x0
1007b4b18:     	add	x0, x28, #0x70
1007b4b1c:     	mov	x1, x19
1007b4b20:     	mov	x19, x8
1007b4b24:     	mov	x2, x8
1007b4b28:     	bl	0x100d41f30 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
1007b4b2c:     	mov	x0, x19
1007b4b30:     	b	0x1007b45d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0xdd4>
1007b4b34:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
1007b4b38:     	add	x2, x2, #0x78
1007b4b3c:     	adrp	x3, 0x101461000 <dyld_stub_binder+0x101461000>
1007b4b40:     	add	x3, x3, #0x185
1007b4b44:     	adrp	x5, 0x1015fd000 <dyld_stub_binder+0x1015fd000>
1007b4b48:     	add	x5, x5, #0xed8
1007b4b4c:     	add	x1, sp, #0xf0
1007b4b50:     	mov	w0, #0x0                ; =0
1007b4b54:     	mov	w4, #0x43               ; =67
1007b4b58:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1007b4b5c:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4b60:     	add	x2, x2, #0x2e0
1007b4b64:     	b	0x1007b4b80 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1384>
1007b4b68:     	ldr	x20, [sp, #0x68]
1007b4b6c:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4b70:     	add	x2, x2, #0x2e0
1007b4b74:     	b	0x1007b4bb4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x13b8>
1007b4b78:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4b7c:     	add	x2, x2, #0x2c8
1007b4b80:     	ldr	x20, [sp, #0x68]
1007b4b84:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4b88:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4b8c:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007b4b90:     	add	x0, x0, #0x31b
1007b4b94:     	adrp	x2, 0x1015ff000 <dyld_stub_binder+0x1015ff000>
1007b4b98:     	add	x2, x2, #0x188
1007b4b9c:     	mov	w1, #0x51               ; =81
1007b4ba0:     	bl	0x1013ba1f4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1007b4ba4:     	mov	x1, x8
1007b4ba8:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4bac:     	add	x2, x2, #0x2c8
1007b4bb0:     	ldr	x20, [sp, #0x68]
1007b4bb4:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4bb8:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4bbc:     	adrp	x0, 0x101461000 <dyld_stub_binder+0x101461000>
1007b4bc0:     	add	x0, x0, #0x2ef
1007b4bc4:     	adrp	x2, 0x1015fe000 <dyld_stub_binder+0x1015fe000>
1007b4bc8:     	add	x2, x2, #0xd90
1007b4bcc:     	mov	w1, #0x2c               ; =44
1007b4bd0:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1007b4bd4:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
1007b4bd8:     	add	x2, x2, #0x678
1007b4bdc:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4be0:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
1007b4be4:     	add	x2, x2, #0x678
1007b4be8:     	mov	x1, x8
1007b4bec:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4bf0:     	mov	x20, x16
1007b4bf4:     	b	0x1007b4c04 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1408>
1007b4bf8:     	mov	x20, x16
1007b4bfc:     	mov	x1, x8
1007b4c00:     	mov	x2, x12
1007b4c04:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c08:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4c0c:     	mov	x1, x12
1007b4c10:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b4c14:     	add	x2, x2, #0xa50
1007b4c18:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c1c:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4c20:     	mov	w0, #0x8                ; =8
1007b4c24:     	mov	x1, x24
1007b4c28:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b4c2c:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4c30:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b4c34:     	add	x2, x2, #0xa68
1007b4c38:     	mov	x0, x22
1007b4c3c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c40:     	mov	x0, x8
1007b4c44:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4c48:     	add	x2, x2, #0x2e0
1007b4c4c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c50:     	mov	x1, x12
1007b4c54:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b4c58:     	add	x2, x2, #0xa50
1007b4c5c:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c60:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4c64:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
1007b4c68:     	add	x2, x2, #0xa80
1007b4c6c:     	mov	x0, x22
1007b4c70:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c74:     	mov	x0, x8
1007b4c78:     	adrp	x2, 0x101645000 <dyld_stub_binder+0x101645000>
1007b4c7c:     	add	x2, x2, #0x2c8
1007b4c80:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007b4c84:     	mov	w0, #0x8                ; =8
1007b4c88:     	mov	x1, x24
1007b4c8c:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b4c90:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4c94:     	mov	w0, #0x8                ; =8
1007b4c98:     	mov	x1, x19
1007b4c9c:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b4ca0:     	mov	w0, #0x8                ; =8
1007b4ca4:     	mov	x1, x26
1007b4ca8:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b4cac:     	b	0x1007b4cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14c0>
1007b4cb0:     	mov	w0, #0x8                ; =8
1007b4cb4:     	mov	x1, x24
1007b4cb8:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007b4cbc:     	brk	#0x1
1007b4cc0:     	b	0x1007b4cd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14d4>
1007b4cc4:     	b	0x1007b4cdc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x14e0>
1007b4cc8:     	ldr	x20, [sp, #0x68]
1007b4ccc:     	b	0x1007b4dc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1007b4cd0:     	mov	x19, x0
1007b4cd4:     	ldr	x23, [sp, #0xf0]
1007b4cd8:     	b	0x1007b4d68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x156c>
1007b4cdc:     	mov	x19, x0
1007b4ce0:     	b	0x1007b4d78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
1007b4ce4:     	b	0x1007b4d5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1560>
1007b4ce8:     	mov	x20, x0
1007b4cec:     	cbz	x27, 0x1007b4d28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
1007b4cf0:     	mov	x0, x28
1007b4cf4:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4cf8:     	b	0x1007b4d28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x152c>
1007b4cfc:     	b	0x1007b4da0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15a4>
1007b4d00:     	str	x27, [sp, #0x80]
1007b4d04:     	str	x24, [sp, #0x68]
1007b4d08:     	mov	x20, x0
1007b4d0c:     	ldr	x8, [sp, #0xf0]
1007b4d10:     	cbz	x8, 0x1007b4d54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
1007b4d14:     	ldr	x8, [sp, #0xf8]
1007b4d18:     	str	x8, [sp, #0x58]
1007b4d1c:     	b	0x1007b4d4c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1550>
1007b4d20:     	str	x24, [sp, #0x68]
1007b4d24:     	mov	x20, x0
1007b4d28:     	ldr	x8, [sp, #0xc0]
1007b4d2c:     	cbz	x8, 0x1007b4d44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
1007b4d30:     	ldr	x0, [sp, #0xc8]
1007b4d34:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4d38:     	b	0x1007b4d44 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1548>
1007b4d3c:     	str	x24, [sp, #0x68]
1007b4d40:     	mov	x20, x0
1007b4d44:     	ldr	x8, [sp, #0x70]
1007b4d48:     	cbz	x8, 0x1007b4d54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x1558>
1007b4d4c:     	ldr	x0, [sp, #0x58]
1007b4d50:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4d54:     	mov	x0, x20
1007b4d58:     	b	0x1007b4db4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
1007b4d5c:     	mov	x19, x0
1007b4d60:     	mov	x0, x25
1007b4d64:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4d68:     	cmp	x23, #0x1
1007b4d6c:     	b.lt	0x1007b4d78 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x157c>
1007b4d70:     	ldr	x0, [sp, #0xf8]
1007b4d74:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4d78:     	ldr	x8, [sp, #0xc0]
1007b4d7c:     	cmp	x8, #0x1
1007b4d80:     	b.lt	0x1007b4dcc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d0>
1007b4d84:     	ldr	x20, [sp, #0xc8]
1007b4d88:     	mov	x0, x19
1007b4d8c:     	b	0x1007b4dc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1007b4d90:     	tbz	w19, #0x0, 0x1007b4dc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1007b4d94:     	b	0x1007b4dd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1007b4d98:     	b	0x1007b4db4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15b8>
1007b4d9c:     	b	0x1007b4db8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
1007b4da0:     	ldr	x8, [sp, #0xf0]
1007b4da4:     	cbz	x8, 0x1007b4dd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1007b4da8:     	ldr	x20, [sp, #0xf8]
1007b4dac:     	b	0x1007b4dc0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15c4>
1007b4db0:     	b	0x1007b4db8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15bc>
1007b4db4:     	ldr	x20, [sp, #0x68]
1007b4db8:     	ldr	x8, [sp, #0x80]
1007b4dbc:     	cbz	x8, 0x1007b4dd0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm9_EBc_+0x15d4>
1007b4dc0:     	mov	x19, x0
1007b4dc4:     	mov	x0, x20
1007b4dc8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
1007b4dcc:     	mov	x0, x19
1007b4dd0:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
