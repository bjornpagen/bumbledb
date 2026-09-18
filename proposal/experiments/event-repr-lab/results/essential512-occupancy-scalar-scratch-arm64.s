
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b25780 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_>:
100b25780:     	stp	d15, d14, [sp, #-0xa0]!
100b25784:     	stp	d13, d12, [sp, #0x10]
100b25788:     	stp	d11, d10, [sp, #0x20]
100b2578c:     	stp	d9, d8, [sp, #0x30]
100b25790:     	stp	x28, x27, [sp, #0x40]
100b25794:     	stp	x26, x25, [sp, #0x50]
100b25798:     	stp	x24, x23, [sp, #0x60]
100b2579c:     	stp	x22, x21, [sp, #0x70]
100b257a0:     	stp	x20, x19, [sp, #0x80]
100b257a4:     	stp	x29, x30, [sp, #0x90]
100b257a8:     	add	x29, sp, #0x90
100b257ac:     	sub	sp, sp, #0x230
100b257b0:     	ldr	w8, [x3, #0x10]
100b257b4:     	str	x8, [sp, #0xe0]
100b257b8:     	cbz	w8, 0x100b257ec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x6c>
100b257bc:     	mov	x21, x5
100b257c0:     	mov	x23, x4
100b257c4:     	mov	x24, x3
100b257c8:     	mov	x26, x2
100b257cc:     	mov	x27, x1
100b257d0:     	mov	x28, x0
100b257d4:     	mov	x0, x4
100b257d8:     	mov	x1, x3
100b257dc:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b257e0:     	cbz	x0, 0x100b257f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x74>
100b257e4:     	ldrb	w27, [x0]
100b257e8:     	b	0x100b25f9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x81c>
100b257ec:     	mov	w27, #0x0               ; =0
100b257f0:     	b	0x100b25f9c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x81c>
100b257f4:     	ldr	x8, [x21]
100b257f8:     	add	x8, x8, #0x1
100b257fc:     	str	x8, [x21]
100b25800:     	ldr	x8, [x28, #0x30]
100b25804:     	ldr	x1, [x28, #0x40]
100b25808:     	ldr	x9, [x24]
100b2580c:     	ldr	x10, [sp, #0xe0]
100b25810:     	lsr	x0, x10, #1
100b25814:     	cmn	x8, #0x1
100b25818:     	b.eq	0x100b25874 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0xf4>
100b2581c:     	cmp	x1, x0
100b25820:     	b.ls	0x100b25ff0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x870>
100b25824:     	ldr	w8, [x24, #0x28]
100b25828:     	lsr	x12, x8, #1
100b2582c:     	cmp	x1, x12
100b25830:     	b.ls	0x100b25fdc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x85c>
100b25834:     	ldr	w10, [x24, #0x40]
100b25838:     	lsr	x11, x10, #1
100b2583c:     	cmp	x1, x11
100b25840:     	b.ls	0x100b25fec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x86c>
100b25844:     	ldr	x13, [x28, #0x38]
100b25848:     	lsl	x12, x12, #4
100b2584c:     	ldr	x12, [x13, x12]
100b25850:     	ldr	x14, [x24, #0x18]
100b25854:     	lsl	x15, x0, #4
100b25858:     	ldr	x15, [x13, x15]
100b2585c:     	bic	x12, x12, x14
100b25860:     	bic	x9, x15, x9
100b25864:     	orr	x9, x12, x9
100b25868:     	add	x11, x13, x11, lsl #4
100b2586c:     	stp	x10, x8, [sp, #0xc0]
100b25870:     	b	0x100b258d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x150>
100b25874:     	ldr	x8, [x28, #0x48]
100b25878:     	cmp	x8, x0
100b2587c:     	b.ls	0x100b26014 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x894>
100b25880:     	ldr	w10, [x24, #0x28]
100b25884:     	str	x10, [sp, #0xc8]
100b25888:     	lsr	x11, x10, #1
100b2588c:     	cmp	x8, x11
100b25890:     	b.ls	0x100b25ffc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x87c>
100b25894:     	ldr	w10, [x24, #0x40]
100b25898:     	str	x10, [sp, #0xc0]
100b2589c:     	lsr	x10, x10, #1
100b258a0:     	cmp	x8, x10
100b258a4:     	b.ls	0x100b26010 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x890>
100b258a8:     	add	x8, x1, x11, lsl #5
100b258ac:     	ldr	x8, [x8, #0x18]
100b258b0:     	ldr	x11, [x24, #0x18]
100b258b4:     	bic	x8, x8, x11
100b258b8:     	add	x11, x1, x0, lsl #5
100b258bc:     	ldr	x11, [x11, #0x18]
100b258c0:     	bic	x9, x11, x9
100b258c4:     	orr	x9, x8, x9
100b258c8:     	add	x8, x1, x10, lsl #5
100b258cc:     	add	x11, x8, #0x18
100b258d0:     	ldr	x8, [x11]
100b258d4:     	mov	x19, x24
100b258d8:     	ldr	x10, [x19, #0x30]!
100b258dc:     	bic	x8, x8, x10
100b258e0:     	orr	x20, x8, x9
100b258e4:     	fmov	d0, x20
100b258e8:     	cnt.8b	v0, v0
100b258ec:     	addv.8b	b0, v0
100b258f0:     	fmov	x8, d0
100b258f4:     	cmp	x8, #0xa
100b258f8:     	str	x28, [sp, #0xd8]
100b258fc:     	b.hs	0x100b25974 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x1f4>
100b25900:     	str	x24, [sp, #0x18]
100b25904:     	ldr	x8, [x21, #0x10]
100b25908:     	add	x8, x8, #0x1
100b2590c:     	str	x8, [x21, #0x10]
100b25910:     	mov	w26, #0x4               ; =4
100b25914:     	stp	xzr, x26, [x29, #-0xc0]
100b25918:     	stur	xzr, [x29, #-0xb0]
100b2591c:     	mov	w24, #0x1               ; =1
100b25920:     	str	x23, [sp, #0x8]
100b25924:     	str	x21, [sp, #0xd0]
100b25928:     	mov	x22, #0x0               ; =0
100b2592c:     	cbz	x20, 0x100b25bd4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x454>
100b25930:     	mov	w8, #0x4                ; =4
100b25934:     	b	0x100b2595c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x1dc>
100b25938:     	ldur	x8, [x29, #-0xb8]
100b2593c:     	rbit	x9, x20
100b25940:     	clz	x9, x9
100b25944:     	str	w9, [x8, x22, lsl #2]
100b25948:     	add	x22, x22, #0x1
100b2594c:     	stur	x22, [x29, #-0xb0]
100b25950:     	sub	x9, x20, #0x1
100b25954:     	ands	x20, x9, x20
100b25958:     	b.eq	0x100b25aa8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x328>
100b2595c:     	ldur	x9, [x29, #-0xc0]
100b25960:     	cmp	x22, x9
100b25964:     	b.ne	0x100b2593c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x1bc>
100b25968:     	sub	x0, x29, #0xc0
100b2596c:     	bl	0x10128921c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100b25970:     	b	0x100b25938 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x1b8>
100b25974:     	mov	x28, x21
100b25978:     	mov	x9, #0x0                ; =0
100b2597c:     	sub	x25, x29, #0xf8
100b25980:     	lsl	x10, x26, #2
100b25984:     	cmp	x10, x9
100b25988:     	b.eq	0x100b25fd0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x850>
100b2598c:     	ldr	w8, [x27, x9]
100b25990:     	lsr	x11, x20, x8
100b25994:     	add	x9, x9, #0x4
100b25998:     	tbz	w11, #0x0, 0x100b25984 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x204>
100b2599c:     	ldr	q0, [x24]
100b259a0:     	stur	q0, [x29, #-0xc0]
100b259a4:     	ldr	x9, [x24, #0x10]
100b259a8:     	stur	x9, [x29, #-0xb0]
100b259ac:     	sub	x0, x29, #0xf8
100b259b0:     	sub	x1, x29, #0xc0
100b259b4:     	ldr	x21, [sp, #0xd8]
100b259b8:     	mov	x2, x21
100b259bc:     	mov	x20, x8
100b259c0:     	mov	x3, x20
100b259c4:     	mov	w4, #0x0                ; =0
100b259c8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b259cc:     	ldr	q0, [x25]
100b259d0:     	ldur	x8, [x29, #-0xe8]
100b259d4:     	str	x8, [sp, #0x1a0]
100b259d8:     	stur	q0, [x29, #-0xe0]
100b259dc:     	stur	x8, [x29, #-0xd0]
100b259e0:     	str	q0, [sp, #0xf0]
100b259e4:     	str	x8, [sp, #0x100]
100b259e8:     	ldur	q0, [x24, #0x18]
100b259ec:     	stur	q0, [x29, #-0xc0]
100b259f0:     	ldr	x8, [x24, #0x28]
100b259f4:     	stur	x8, [x29, #-0xb0]
100b259f8:     	sub	x0, x29, #0xf8
100b259fc:     	sub	x1, x29, #0xc0
100b25a00:     	mov	x2, x21
100b25a04:     	mov	x3, x20
100b25a08:     	mov	w4, #0x0                ; =0
100b25a0c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25a10:     	ldr	q0, [x25]
100b25a14:     	ldur	x8, [x29, #-0xe8]
100b25a18:     	str	x8, [sp, #0x1a0]
100b25a1c:     	stur	q0, [x29, #-0xe0]
100b25a20:     	stur	x8, [x29, #-0xd0]
100b25a24:     	add	x9, sp, #0x9
100b25a28:     	stur	q0, [x9, #0xff]
100b25a2c:     	str	x8, [sp, #0x118]
100b25a30:     	ldr	q0, [x19]
100b25a34:     	stur	q0, [x29, #-0xc0]
100b25a38:     	ldr	x8, [x19, #0x10]
100b25a3c:     	stur	x8, [x29, #-0xb0]
100b25a40:     	sub	x0, x29, #0xf8
100b25a44:     	sub	x1, x29, #0xc0
100b25a48:     	mov	x2, x21
100b25a4c:     	mov	x22, x20
100b25a50:     	mov	x3, x20
100b25a54:     	mov	w4, #0x0                ; =0
100b25a58:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25a5c:     	ldr	q0, [x25]
100b25a60:     	ldur	x8, [x29, #-0xe8]
100b25a64:     	str	x8, [sp, #0x1a0]
100b25a68:     	stur	q0, [x29, #-0xe0]
100b25a6c:     	str	q0, [sp, #0x120]
100b25a70:     	str	x8, [sp, #0x130]
100b25a74:     	add	x3, sp, #0xf0
100b25a78:     	mov	x0, x21
100b25a7c:     	mov	x1, x27
100b25a80:     	mov	x2, x26
100b25a84:     	mov	x4, x23
100b25a88:     	mov	x5, x28
100b25a8c:     	bl	0x100b25780 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_>
100b25a90:     	mov	x25, x23
100b25a94:     	and	w8, w0, #0xff
100b25a98:     	cmp	w8, #0xf
100b25a9c:     	b.ne	0x100b25ab8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x338>
100b25aa0:     	mov	w27, #0xf               ; =15
100b25aa4:     	b	0x100b25bcc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x44c>
100b25aa8:     	ldp	x8, x26, [x29, #-0xc0]
100b25aac:     	cmp	x8, #0x0
100b25ab0:     	cset	w8, eq
100b25ab4:     	b	0x100b25bd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x458>
100b25ab8:     	mov	x23, x0
100b25abc:     	ldr	q0, [x24]
100b25ac0:     	stur	q0, [x29, #-0xc0]
100b25ac4:     	ldr	x8, [x24, #0x10]
100b25ac8:     	stur	x8, [x29, #-0xb0]
100b25acc:     	sub	x0, x29, #0xf8
100b25ad0:     	sub	x1, x29, #0xc0
100b25ad4:     	mov	x2, x21
100b25ad8:     	mov	x20, x22
100b25adc:     	mov	x3, x20
100b25ae0:     	mov	w4, #0x1                ; =1
100b25ae4:     	sub	x22, x29, #0xf8
100b25ae8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25aec:     	ldr	q0, [x22]
100b25af0:     	str	q0, [sp, #0x1b0]
100b25af4:     	ldur	x8, [x29, #-0xe8]
100b25af8:     	str	q0, [sp, #0x190]
100b25afc:     	stur	q0, [x29, #-0xe0]
100b25b00:     	stur	x8, [x29, #-0xd0]
100b25b04:     	ldur	q0, [x29, #-0xe0]
100b25b08:     	str	x8, [sp, #0x150]
100b25b0c:     	str	q0, [sp, #0x140]
100b25b10:     	ldur	q0, [x24, #0x18]
100b25b14:     	stur	q0, [x29, #-0xc0]
100b25b18:     	ldur	x8, [x24, #0x28]
100b25b1c:     	stur	x8, [x29, #-0xb0]
100b25b20:     	sub	x0, x29, #0xf8
100b25b24:     	sub	x1, x29, #0xc0
100b25b28:     	mov	x2, x21
100b25b2c:     	mov	x3, x20
100b25b30:     	mov	w4, #0x1                ; =1
100b25b34:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25b38:     	ldr	q0, [x22]
100b25b3c:     	str	q0, [sp, #0x1b0]
100b25b40:     	ldur	x8, [x29, #-0xe8]
100b25b44:     	str	q0, [sp, #0x190]
100b25b48:     	stur	q0, [x29, #-0xe0]
100b25b4c:     	stur	x8, [x29, #-0xd0]
100b25b50:     	ldur	q0, [x29, #-0xe0]
100b25b54:     	str	x8, [sp, #0x168]
100b25b58:     	add	x8, sp, #0x59
100b25b5c:     	stur	q0, [x8, #0xff]
100b25b60:     	ldr	q0, [x19]
100b25b64:     	stur	q0, [x29, #-0xc0]
100b25b68:     	ldr	x8, [x19, #0x10]
100b25b6c:     	stur	x8, [x29, #-0xb0]
100b25b70:     	sub	x0, x29, #0xf8
100b25b74:     	sub	x1, x29, #0xc0
100b25b78:     	mov	x2, x21
100b25b7c:     	mov	x3, x20
100b25b80:     	mov	w4, #0x1                ; =1
100b25b84:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b25b88:     	ldr	q0, [x22]
100b25b8c:     	str	q0, [sp, #0x1b0]
100b25b90:     	ldur	x8, [x29, #-0xe8]
100b25b94:     	str	q0, [sp, #0x190]
100b25b98:     	stur	q0, [x29, #-0xe0]
100b25b9c:     	stur	x8, [x29, #-0xd0]
100b25ba0:     	ldur	q0, [x29, #-0xe0]
100b25ba4:     	str	x8, [sp, #0x180]
100b25ba8:     	str	q0, [sp, #0x170]
100b25bac:     	add	x3, sp, #0x140
100b25bb0:     	mov	x0, x21
100b25bb4:     	mov	x1, x27
100b25bb8:     	mov	x2, x26
100b25bbc:     	mov	x4, x25
100b25bc0:     	mov	x5, x28
100b25bc4:     	bl	0x100b25780 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_>
100b25bc8:     	orr	w27, w0, w23
100b25bcc:     	mov	x0, x25
100b25bd0:     	b	0x100b25f90 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x810>
100b25bd4:     	mov	w8, #0x1                ; =1
100b25bd8:     	str	w8, [sp, #0x14]
100b25bdc:     	mov	w27, #0x0               ; =0
100b25be0:     	mov	x21, #0x0               ; =0
100b25be4:     	ldr	x8, [sp, #0x18]
100b25be8:     	ldr	x10, [x8, #0x8]
100b25bec:     	ldr	x9, [x8, #0x20]
100b25bf0:     	stp	x9, x10, [sp, #0xb0]
100b25bf4:     	ldr	x8, [x8, #0x38]
100b25bf8:     	str	x8, [sp, #0xa8]
100b25bfc:     	ldr	x8, [sp, #0xd0]
100b25c00:     	ldr	x8, [x8, #0x8]
100b25c04:     	str	x8, [sp, #0xe8]
100b25c08:     	and	x8, x22, #0xfffffffffffffffe
100b25c0c:     	neg	x8, x8
100b25c10:     	str	x8, [sp, #0x88]
100b25c14:     	mov	w23, #0x2               ; =2
100b25c18:     	adrp	x8, 0x101322000 <GCC_except_table9287>
100b25c1c:     	ldr	q0, [x8, #0x740]
100b25c20:     	str	q0, [sp, #0x90]
100b25c24:     	mov	w8, #0x4                ; =4
100b25c28:     	dup.2d	v1, x8
100b25c2c:     	mov	w8, #0x8                ; =8
100b25c30:     	dup.2d	v0, x8
100b25c34:     	stp	q0, q1, [sp, #0x50]
100b25c38:     	mov	w8, #0xc                ; =12
100b25c3c:     	dup.2d	v1, x8
100b25c40:     	mov	w8, #0x10               ; =16
100b25c44:     	dup.2d	v0, x8
100b25c48:     	stp	q0, q1, [sp, #0x30]
100b25c4c:     	adrp	x8, 0x101322000 <GCC_except_table9287>
100b25c50:     	ldr	q0, [x8, #0x760]
100b25c54:     	str	q0, [sp, #0x20]
100b25c58:     	mov	w8, #0x3f               ; =63
100b25c5c:     	dup.2d	v0, x8
100b25c60:     	str	q0, [sp, #0x70]
100b25c64:     	movi.2s	v8, #0x3f
100b25c68:     	b	0x100b25c80 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x500>
100b25c6c:     	add	x21, x21, #0x1
100b25c70:     	and	x8, x22, #0x3f
100b25c74:     	lsr	x8, x21, x8
100b25c78:     	ldr	x28, [sp, #0xd8]
100b25c7c:     	cbnz	x8, 0x100b25f78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x7f8>
100b25c80:     	ldr	x9, [sp, #0xe8]
100b25c84:     	add	x9, x9, #0x1
100b25c88:     	ldr	x8, [sp, #0xd0]
100b25c8c:     	str	x9, [sp, #0xe8]
100b25c90:     	str	x9, [x8, #0x8]
100b25c94:     	mov	x25, x22
100b25c98:     	cbz	x22, 0x100b25f0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x78c>
100b25c9c:     	cmp	x22, #0x1
100b25ca0:     	b.ne	0x100b25cb0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x530>
100b25ca4:     	mov	x8, #0x0                ; =0
100b25ca8:     	mov	x25, #0x0               ; =0
100b25cac:     	b	0x100b25eec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x76c>
100b25cb0:     	dup.2d	v0, x21
100b25cb4:     	cmp	x22, #0x10
100b25cb8:     	b.hs	0x100b25cc8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x548>
100b25cbc:     	mov	x9, #0x0                ; =0
100b25cc0:     	mov	x25, #0x0               ; =0
100b25cc4:     	b	0x100b25e78 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x6f8>
100b25cc8:     	movi.2d	v1, #0000000000000000
100b25ccc:     	add	x8, x26, #0x20
100b25cd0:     	movi.2d	v2, #0000000000000000
100b25cd4:     	and	x9, x22, #0x1ffffffffffffff0
100b25cd8:     	ldr	q4, [sp, #0x90]
100b25cdc:     	ldp	q6, q15, [sp, #0x20]
100b25ce0:     	movi.2d	v3, #0000000000000000
100b25ce4:     	movi.2d	v7, #0000000000000000
100b25ce8:     	movi.2d	v16, #0000000000000000
100b25cec:     	movi.2d	v5, #0000000000000000
100b25cf0:     	movi.2d	v18, #0000000000000000
100b25cf4:     	movi.2d	v17, #0000000000000000
100b25cf8:     	ldp	q13, q12, [sp, #0x50]
100b25cfc:     	ldr	q14, [sp, #0x40]
100b25d00:     	mov	w10, #0x3f              ; =63
100b25d04:     	movi.4s	v8, #0x3f
100b25d08:     	add.2d	v19, v4, v12
100b25d0c:     	add.2d	v20, v6, v12
100b25d10:     	add.2d	v21, v4, v13
100b25d14:     	add.2d	v22, v6, v13
100b25d18:     	add.2d	v23, v4, v14
100b25d1c:     	add.2d	v24, v6, v14
100b25d20:     	ldp	q25, q26, [x8, #-0x20]
100b25d24:     	dup.2d	v27, x10
100b25d28:     	ldp	q28, q29, [x8], #0x40
100b25d2c:     	and.16b	v30, v6, v27
100b25d30:     	and.16b	v31, v4, v27
100b25d34:     	and.16b	v20, v20, v27
100b25d38:     	and.16b	v19, v19, v27
100b25d3c:     	and.16b	v22, v22, v27
100b25d40:     	and.16b	v21, v21, v27
100b25d44:     	and.16b	v24, v24, v27
100b25d48:     	and.16b	v23, v23, v27
100b25d4c:     	neg.2d	v27, v31
100b25d50:     	ushl.2d	v27, v0, v27
100b25d54:     	neg.2d	v30, v30
100b25d58:     	ushl.2d	v30, v0, v30
100b25d5c:     	neg.2d	v19, v19
100b25d60:     	ushl.2d	v19, v0, v19
100b25d64:     	neg.2d	v20, v20
100b25d68:     	ushl.2d	v20, v0, v20
100b25d6c:     	neg.2d	v21, v21
100b25d70:     	ushl.2d	v21, v0, v21
100b25d74:     	neg.2d	v22, v22
100b25d78:     	ushl.2d	v22, v0, v22
100b25d7c:     	neg.2d	v23, v23
100b25d80:     	ushl.2d	v23, v0, v23
100b25d84:     	neg.2d	v24, v24
100b25d88:     	ushl.2d	v24, v0, v24
100b25d8c:     	dup.2d	v31, x24
100b25d90:     	and.16b	v30, v30, v31
100b25d94:     	and.16b	v27, v27, v31
100b25d98:     	and.16b	v20, v20, v31
100b25d9c:     	and.16b	v19, v19, v31
100b25da0:     	and.16b	v22, v22, v31
100b25da4:     	and.16b	v21, v21, v31
100b25da8:     	and.16b	v24, v24, v31
100b25dac:     	and.16b	v23, v23, v31
100b25db0:     	and.16b	v25, v25, v8
100b25db4:     	and.16b	v26, v26, v8
100b25db8:     	and.16b	v28, v28, v8
100b25dbc:     	and.16b	v29, v29, v8
100b25dc0:     	ushll2.2d	v31, v25, #0x0
100b25dc4:     	ushll.2d	v25, v25, #0x0
100b25dc8:     	ushll2.2d	v9, v26, #0x0
100b25dcc:     	ushll.2d	v26, v26, #0x0
100b25dd0:     	ushll2.2d	v10, v28, #0x0
100b25dd4:     	ushll.2d	v28, v28, #0x0
100b25dd8:     	ushll2.2d	v11, v29, #0x0
100b25ddc:     	ushll.2d	v29, v29, #0x0
100b25de0:     	ushl.2d	v25, v27, v25
100b25de4:     	ushl.2d	v27, v30, v31
100b25de8:     	ushl.2d	v19, v19, v26
100b25dec:     	ushl.2d	v20, v20, v9
100b25df0:     	ushl.2d	v21, v21, v28
100b25df4:     	ushl.2d	v22, v22, v10
100b25df8:     	ushl.2d	v23, v23, v29
100b25dfc:     	ushl.2d	v24, v24, v11
100b25e00:     	orr.16b	v3, v27, v3
100b25e04:     	orr.16b	v2, v25, v2
100b25e08:     	orr.16b	v16, v20, v16
100b25e0c:     	orr.16b	v7, v19, v7
100b25e10:     	orr.16b	v18, v22, v18
100b25e14:     	orr.16b	v5, v21, v5
100b25e18:     	orr.16b	v1, v24, v1
100b25e1c:     	orr.16b	v17, v23, v17
100b25e20:     	add.2d	v6, v6, v15
100b25e24:     	add.2d	v4, v4, v15
100b25e28:     	subs	x9, x9, #0x10
100b25e2c:     	b.ne	0x100b25d08 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x588>
100b25e30:     	orr.16b	v2, v7, v2
100b25e34:     	orr.16b	v3, v16, v3
100b25e38:     	orr.16b	v3, v18, v3
100b25e3c:     	orr.16b	v2, v5, v2
100b25e40:     	orr.16b	v2, v17, v2
100b25e44:     	orr.16b	v1, v1, v3
100b25e48:     	orr.16b	v1, v2, v1
100b25e4c:     	mov	d2, v1[1]
100b25e50:     	orr.8b	v1, v1, v2
100b25e54:     	fmov	x25, d1
100b25e58:     	and	x8, x22, #0x1ffffffffffffff0
100b25e5c:     	cmp	x22, x8
100b25e60:     	movi.2s	v8, #0x3f
100b25e64:     	b.eq	0x100b25f0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x78c>
100b25e68:     	and	x9, x22, #0x1ffffffffffffff0
100b25e6c:     	and	x8, x22, #0x1ffffffffffffff0
100b25e70:     	and	x10, x22, #0xe
100b25e74:     	cbz	x10, 0x100b25eec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x76c>
100b25e78:     	fmov	d1, x25
100b25e7c:     	dup.2d	v2, x9
100b25e80:     	ldr	q3, [sp, #0x90]
100b25e84:     	orr.16b	v2, v2, v3
100b25e88:     	ldr	x8, [sp, #0x88]
100b25e8c:     	add	x8, x8, x9
100b25e90:     	add	x9, x26, x9, lsl #2
100b25e94:     	ldr	q6, [sp, #0x70]
100b25e98:     	ldr	d3, [x9], #0x8
100b25e9c:     	and.16b	v4, v2, v6
100b25ea0:     	neg.2d	v4, v4
100b25ea4:     	ushl.2d	v4, v0, v4
100b25ea8:     	dup.2d	v5, x24
100b25eac:     	and.16b	v4, v4, v5
100b25eb0:     	and.8b	v3, v3, v8
100b25eb4:     	ushll.2d	v3, v3, #0x0
100b25eb8:     	ushl.2d	v3, v4, v3
100b25ebc:     	orr.16b	v1, v3, v1
100b25ec0:     	dup.2d	v3, x23
100b25ec4:     	add.2d	v2, v2, v3
100b25ec8:     	adds	x8, x8, #0x2
100b25ecc:     	b.ne	0x100b25e98 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x718>
100b25ed0:     	mov	d0, v1[1]
100b25ed4:     	orr.8b	v0, v1, v0
100b25ed8:     	fmov	x25, d0
100b25edc:     	and	x8, x22, #0x1ffffffffffffffe
100b25ee0:     	and	x9, x22, #0x1ffffffffffffffe
100b25ee4:     	cmp	x22, x9
100b25ee8:     	b.eq	0x100b25f0c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x78c>
100b25eec:     	ldr	w9, [x26, x8, lsl #2]
100b25ef0:     	lsr	x10, x21, x8
100b25ef4:     	and	x10, x10, #0x1
100b25ef8:     	lsl	x9, x10, x9
100b25efc:     	orr	x25, x9, x25
100b25f00:     	add	x8, x8, #0x1
100b25f04:     	cmp	x22, x8
100b25f08:     	b.ne	0x100b25eec <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x76c>
100b25f0c:     	ldr	x8, [sp, #0xb8]
100b25f10:     	orr	x2, x8, x25
100b25f14:     	mov	x0, x28
100b25f18:     	ldr	x1, [sp, #0xe0]
100b25f1c:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b25f20:     	mov	x19, x0
100b25f24:     	ldr	x8, [sp, #0xb0]
100b25f28:     	orr	x2, x8, x25
100b25f2c:     	mov	x0, x28
100b25f30:     	ldr	x1, [sp, #0xc8]
100b25f34:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b25f38:     	mov	x20, x0
100b25f3c:     	ldr	x8, [sp, #0xa8]
100b25f40:     	orr	x2, x8, x25
100b25f44:     	mov	x0, x28
100b25f48:     	ldr	x1, [sp, #0xc0]
100b25f4c:     	bl	0x100b92268 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b25f50:     	tbz	w19, #0x0, 0x100b25c6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x4ec>
100b25f54:     	cmp	w20, #0x0
100b25f58:     	csel	w8, w23, wzr, ne
100b25f5c:     	orr	w8, w8, w0
100b25f60:     	lsl	w8, w24, w8
100b25f64:     	orr	w27, w8, w27
100b25f68:     	and	w8, w27, #0xff
100b25f6c:     	cmp	w8, #0xf
100b25f70:     	b.ne	0x100b25c6c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x4ec>
100b25f74:     	mov	w27, #0xf               ; =15
100b25f78:     	ldr	w8, [sp, #0x14]
100b25f7c:     	tbnz	w8, #0x0, 0x100b25f88 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x808>
100b25f80:     	mov	x0, x26
100b25f84:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b25f88:     	ldr	x24, [sp, #0x18]
100b25f8c:     	ldr	x0, [sp, #0x8]
100b25f90:     	mov	x1, x24
100b25f94:     	mov	x2, x27
100b25f98:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b25f9c:     	mov	x0, x27
100b25fa0:     	add	sp, sp, #0x230
100b25fa4:     	ldp	x29, x30, [sp, #0x90]
100b25fa8:     	ldp	x20, x19, [sp, #0x80]
100b25fac:     	ldp	x22, x21, [sp, #0x70]
100b25fb0:     	ldp	x24, x23, [sp, #0x60]
100b25fb4:     	ldp	x26, x25, [sp, #0x50]
100b25fb8:     	ldp	x28, x27, [sp, #0x40]
100b25fbc:     	ldp	d9, d8, [sp, #0x30]
100b25fc0:     	ldp	d11, d10, [sp, #0x20]
100b25fc4:     	ldp	d13, d12, [sp, #0x10]
100b25fc8:     	ldp	d15, d14, [sp], #0xa0
100b25fcc:     	ret
100b25fd0:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b25fd4:     	add	x0, x0, #0xd90
100b25fd8:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b25fdc:     	mov	x0, x12
100b25fe0:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25fe4:     	add	x2, x2, #0x6c0
100b25fe8:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25fec:     	mov	x0, x11
100b25ff0:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b25ff4:     	add	x2, x2, #0x6c0
100b25ff8:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b25ffc:     	mov	x0, x11
100b26000:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b26004:     	add	x2, x2, #0x6a8
100b26008:     	mov	x1, x8
100b2600c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b26010:     	mov	x0, x10
100b26014:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b26018:     	add	x2, x2, #0x6a8
100b2601c:     	mov	x1, x8
100b26020:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b26024:     	mov	x19, x0
100b26028:     	ldur	x8, [x29, #-0xc0]
100b2602c:     	cbz	x8, 0x100b2604c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x8cc>
100b26030:     	ldur	x26, [x29, #-0xb8]
100b26034:     	b	0x100b26044 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x8c4>
100b26038:     	mov	x19, x0
100b2603c:     	ldr	w8, [sp, #0x14]
100b26040:     	tbnz	w8, #0x0, 0x100b2604c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_+0x8cc>
100b26044:     	mov	x0, x26
100b26048:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b2604c:     	mov	x0, x19
100b26050:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
