
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100809724 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense>:
100809724:     	sub	sp, sp, #0xe0
100809728:     	stp	d15, d14, [sp, #0x40]
10080972c:     	stp	d13, d12, [sp, #0x50]
100809730:     	stp	d11, d10, [sp, #0x60]
100809734:     	stp	d9, d8, [sp, #0x70]
100809738:     	stp	x28, x27, [sp, #0x80]
10080973c:     	stp	x26, x25, [sp, #0x90]
100809740:     	stp	x24, x23, [sp, #0xa0]
100809744:     	stp	x22, x21, [sp, #0xb0]
100809748:     	stp	x20, x19, [sp, #0xc0]
10080974c:     	stp	x29, x30, [sp, #0xd0]
100809750:     	add	x29, sp, #0xd0
100809754:     	ldr	x8, [x0, #0x10]
100809758:     	and	x9, x8, #0x3f
10080975c:     	mov	w10, #0x1               ; =1
100809760:     	lsl	x8, x10, x8
100809764:     	lsr	x8, x8, #6
100809768:     	cmp	x9, #0x6
10080976c:     	cinc	x8, x8, lo
100809770:     	stp	x2, x8, [sp, #0x30]
100809774:     	cmp	x2, x8
100809778:     	b.ne	0x10080a0c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9a4>
10080977c:     	ldr	x8, [x0, #0x28]
100809780:     	cbz	x8, 0x10080a04c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
100809784:     	mov	x19, x1
100809788:     	ldr	x9, [x0, #0x20]
10080978c:     	add	x10, x9, x8, lsl #3
100809790:     	lsl	x8, x2, #3
100809794:     	add	x11, x1, x8
100809798:     	sub	x12, x8, #0x8
10080979c:     	lsr	x8, x12, #3
1008097a0:     	add	x13, x8, #0x1
1008097a4:     	and	x14, x13, #0x3ffffffffffffff8
1008097a8:     	mov	w16, #0x1               ; =1
1008097ac:     	mov	x8, #0x100000000        ; =4294967296
1008097b0:     	str	x8, [sp, #0x20]
1008097b4:     	mov	x8, #0x2                ; =2
1008097b8:     	movk	x8, #0x3, lsl #32
1008097bc:     	str	x8, [sp, #0x18]
1008097c0:     	mov	x8, #0x4                ; =4
1008097c4:     	movk	x8, #0x5, lsl #32
1008097c8:     	fmov	d2, x8
1008097cc:     	mov	x8, #0x6                ; =6
1008097d0:     	movk	x8, #0x7, lsl #32
1008097d4:     	fmov	d3, x8
1008097d8:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
1008097dc:     	ldr	q0, [x8, #0x520]
1008097e0:     	str	q0, [sp]
1008097e4:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
1008097e8:     	ldr	q5, [x8, #0xc10]
1008097ec:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
1008097f0:     	ldr	q6, [x8, #0xc20]
1008097f4:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
1008097f8:     	ldr	q7, [x8, #0x650]
1008097fc:     	mov	x8, #0x8                ; =8
100809800:     	movk	x8, #0x9, lsl #32
100809804:     	fmov	d16, x8
100809808:     	mov	x8, #0xa                ; =10
10080980c:     	movk	x8, #0xb, lsl #32
100809810:     	fmov	d17, x8
100809814:     	mov	x8, #0xc                ; =12
100809818:     	movk	x8, #0xd, lsl #32
10080981c:     	fmov	d18, x8
100809820:     	mov	x8, #0xe                ; =14
100809824:     	movk	x8, #0xf, lsl #32
100809828:     	fmov	d19, x8
10080982c:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
100809830:     	ldr	q20, [x8, #0x660]
100809834:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
100809838:     	ldr	q21, [x8, #0xc30]
10080983c:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
100809840:     	ldr	q22, [x8, #0xc40]
100809844:     	adrp	x8, 0x100f05000 <GCC_except_table8391+0x14>
100809848:     	ldr	q23, [x8, #0xc50]
10080984c:     	mov	x8, #0x10               ; =16
100809850:     	movk	x8, #0x11, lsl #32
100809854:     	fmov	d24, x8
100809858:     	mov	x8, #0x12               ; =18
10080985c:     	movk	x8, #0x13, lsl #32
100809860:     	fmov	d25, x8
100809864:     	mov	x8, #0x14               ; =20
100809868:     	movk	x8, #0x15, lsl #32
10080986c:     	fmov	d26, x8
100809870:     	mov	x8, #0x16               ; =22
100809874:     	movk	x8, #0x17, lsl #32
100809878:     	fmov	d27, x8
10080987c:     	add	x8, x1, x14, lsl #3
100809880:     	str	x8, [sp, #0x28]
100809884:     	b	0x100809894 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x170>
100809888:     	add	x9, x9, #0x8
10080988c:     	cmp	x9, x10
100809890:     	b.eq	0x10080a04c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
100809894:     	ldp	w1, w15, [x9]
100809898:     	cmp	w15, #0x6
10080989c:     	b.hs	0x1008099bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x298>
1008098a0:     	mov	x3, #0x0                ; =0
1008098a4:     	mov	x8, #0x0                ; =0
1008098a8:     	and	w14, w1, #0x1f
1008098ac:     	lsl	w0, w16, w1
1008098b0:     	mov	w1, #-0x1               ; =-1
1008098b4:     	lsr	w4, w1, w15
1008098b8:     	lsl	x6, x16, x3
1008098bc:     	orr	x6, x6, x8
1008098c0:     	tst	w4, #0x1
1008098c4:     	csel	x4, x8, x6, eq
1008098c8:     	tst	w0, w3
1008098cc:     	add	x3, x3, #0x1
1008098d0:     	csel	x8, x8, x4, eq
1008098d4:     	sub	w1, w1, #0x1
1008098d8:     	cmp	x3, #0x40
1008098dc:     	b.ne	0x1008098b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x190>
1008098e0:     	cbz	x2, 0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1008098e4:     	mov	w17, #-0x1              ; =-1
1008098e8:     	lsl	w14, w17, w14
1008098ec:     	lsl	w15, w16, w15
1008098f0:     	add	w14, w14, w15
1008098f4:     	and	w15, w14, #0x3f
1008098f8:     	mov	x14, x19
1008098fc:     	cmp	x12, #0x38
100809900:     	b.lo	0x100809990 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
100809904:     	dup.2d	v28, x15
100809908:     	dup.2d	v29, x8
10080990c:     	neg.2d	v30, v28
100809910:     	add	x0, x19, #0x20
100809914:     	and	x1, x13, #0x3ffffffffffffff8
100809918:     	ldp	q31, q8, [x0, #-0x20]
10080991c:     	ldp	q9, q10, [x0]
100809920:     	ushl.2d	v11, v31, v30
100809924:     	ushl.2d	v12, v8, v30
100809928:     	ushl.2d	v13, v9, v30
10080992c:     	ushl.2d	v14, v10, v30
100809930:     	eor.16b	v11, v11, v31
100809934:     	eor.16b	v12, v12, v8
100809938:     	eor.16b	v13, v13, v9
10080993c:     	eor.16b	v14, v14, v10
100809940:     	and.16b	v11, v11, v29
100809944:     	and.16b	v12, v12, v29
100809948:     	and.16b	v13, v13, v29
10080994c:     	and.16b	v14, v14, v29
100809950:     	ushl.2d	v15, v11, v28
100809954:     	ushl.2d	v0, v12, v28
100809958:     	ushl.2d	v4, v13, v28
10080995c:     	ushl.2d	v1, v14, v28
100809960:     	eor3.16b	v31, v31, v15, v11
100809964:     	eor3.16b	v0, v8, v0, v12
100809968:     	eor3.16b	v4, v9, v4, v13
10080996c:     	stp	q31, q0, [x0, #-0x20]
100809970:     	eor3.16b	v0, v10, v1, v14
100809974:     	stp	q4, q0, [x0], #0x40
100809978:     	subs	x1, x1, #0x8
10080997c:     	b.ne	0x100809918 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x1f4>
100809980:     	ldr	x14, [sp, #0x28]
100809984:     	and	x17, x13, #0x3ffffffffffffff8
100809988:     	cmp	x13, x17
10080998c:     	b.eq	0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
100809990:     	ldr	x17, [x14]
100809994:     	lsr	x0, x17, x15
100809998:     	eor	x0, x0, x17
10080999c:     	and	x0, x0, x8
1008099a0:     	lsl	x1, x0, x15
1008099a4:     	eor	x17, x17, x0
1008099a8:     	eor	x17, x17, x1
1008099ac:     	str	x17, [x14], #0x8
1008099b0:     	cmp	x14, x11
1008099b4:     	b.ne	0x100809990 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
1008099b8:     	b	0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1008099bc:     	cmp	w1, #0x6
1008099c0:     	b.hs	0x100809ff0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8cc>
1008099c4:     	add	w14, w15, #0x3a
1008099c8:     	and	w8, w14, #0x3f
1008099cc:     	cmp	w8, #0x3f
1008099d0:     	b.eq	0x10080a090 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x96c>
1008099d4:     	mov	w15, #0x2               ; =2
1008099d8:     	lsl	x28, x15, x14
1008099dc:     	add	x15, x8, #0x1
1008099e0:     	lsr	x20, x2, x15
1008099e4:     	mov	x15, #0xfffffffffffffff ; =1152921504606846975
1008099e8:     	add	x15, x28, x15
1008099ec:     	tst	x15, x2
1008099f0:     	cset	w22, ne
1008099f4:     	cinc	x15, x20, ne
1008099f8:     	cbz	x15, 0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1008099fc:     	lsl	x6, x16, x14
100809a00:     	mov	w14, #0x8               ; =8
100809a04:     	lsl	x14, x14, x8
100809a08:     	lsr	x14, x14, #3
100809a0c:     	subs	x0, x28, x6
100809a10:     	cmp	x0, x14
100809a14:     	csel	x3, x0, x14, lo
100809a18:     	cmp	x28, x6
100809a1c:     	b.lo	0x10080a0a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x984>
100809a20:     	mov	x0, #0x0                ; =0
100809a24:     	b.eq	0x100809edc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
100809a28:     	lsl	x4, x16, x1
100809a2c:     	dup.2s	v28, w4
100809a30:     	ldp	d1, d0, [sp, #0x18]
100809a34:     	and.8b	v0, v28, v0
100809a38:     	and.8b	v1, v28, v1
100809a3c:     	and.8b	v4, v28, v2
100809a40:     	and.8b	v29, v28, v3
100809a44:     	cmeq.2s	v0, v0, #0
100809a48:     	ushll.2d	v0, v0, #0x0
100809a4c:     	cmeq.2s	v1, v1, #0
100809a50:     	ushll.2d	v1, v1, #0x0
100809a54:     	cmeq.2s	v4, v4, #0
100809a58:     	ushll.2d	v4, v4, #0x0
100809a5c:     	cmeq.2s	v29, v29, #0
100809a60:     	ushll.2d	v29, v29, #0x0
100809a64:     	ldr	q30, [sp]
100809a68:     	and.16b	v0, v0, v30
100809a6c:     	and.16b	v1, v1, v5
100809a70:     	and.16b	v4, v4, v6
100809a74:     	and.16b	v29, v29, v7
100809a78:     	and.8b	v30, v28, v16
100809a7c:     	and.8b	v31, v28, v17
100809a80:     	and.8b	v8, v28, v18
100809a84:     	and.8b	v9, v28, v19
100809a88:     	cmeq.2s	v30, v30, #0
100809a8c:     	ushll.2d	v30, v30, #0x0
100809a90:     	cmeq.2s	v31, v31, #0
100809a94:     	ushll.2d	v31, v31, #0x0
100809a98:     	cmeq.2s	v8, v8, #0
100809a9c:     	ushll.2d	v8, v8, #0x0
100809aa0:     	cmeq.2s	v9, v9, #0
100809aa4:     	ushll.2d	v9, v9, #0x0
100809aa8:     	and.16b	v30, v30, v20
100809aac:     	and.16b	v31, v31, v21
100809ab0:     	and.16b	v8, v8, v22
100809ab4:     	and.16b	v9, v9, v23
100809ab8:     	orr.16b	v0, v30, v0
100809abc:     	orr.16b	v1, v31, v1
100809ac0:     	orr.16b	v4, v8, v4
100809ac4:     	orr.16b	v29, v9, v29
100809ac8:     	and.8b	v30, v28, v24
100809acc:     	and.8b	v31, v28, v25
100809ad0:     	and.8b	v8, v28, v26
100809ad4:     	and.8b	v9, v28, v27
100809ad8:     	cmeq.2s	v30, v30, #0
100809adc:     	ushll.2d	v30, v30, #0x0
100809ae0:     	cmeq.2s	v31, v31, #0
100809ae4:     	ushll.2d	v31, v31, #0x0
100809ae8:     	cmeq.2s	v8, v8, #0
100809aec:     	ushll.2d	v8, v8, #0x0
100809af0:     	cmeq.2s	v9, v9, #0
100809af4:     	ushll.2d	v9, v9, #0x0
100809af8:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809afc:     	ldr	q10, [x14, #0xc60]
100809b00:     	and.16b	v30, v30, v10
100809b04:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809b08:     	ldr	q10, [x14, #0xc70]
100809b0c:     	and.16b	v10, v31, v10
100809b10:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809b14:     	ldr	q31, [x14, #0xc80]
100809b18:     	and.16b	v11, v8, v31
100809b1c:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809b20:     	ldr	q31, [x14, #0xc90]
100809b24:     	and.16b	v9, v9, v31
100809b28:     	mov	x14, #0x18              ; =24
100809b2c:     	movk	x14, #0x19, lsl #32
100809b30:     	fmov	d31, x14
100809b34:     	and.8b	v31, v28, v31
100809b38:     	mov	x14, #0x1a              ; =26
100809b3c:     	movk	x14, #0x1b, lsl #32
100809b40:     	fmov	d8, x14
100809b44:     	and.8b	v8, v28, v8
100809b48:     	mov	x14, #0x1c              ; =28
100809b4c:     	movk	x14, #0x1d, lsl #32
100809b50:     	fmov	d12, x14
100809b54:     	and.8b	v12, v28, v12
100809b58:     	mov	x14, #0x1e              ; =30
100809b5c:     	movk	x14, #0x1f, lsl #32
100809b60:     	fmov	d13, x14
100809b64:     	and.8b	v13, v28, v13
100809b68:     	cmeq.2s	v31, v31, #0
100809b6c:     	ushll.2d	v31, v31, #0x0
100809b70:     	cmeq.2s	v8, v8, #0
100809b74:     	ushll.2d	v8, v8, #0x0
100809b78:     	cmeq.2s	v12, v12, #0
100809b7c:     	ushll.2d	v12, v12, #0x0
100809b80:     	cmeq.2s	v13, v13, #0
100809b84:     	ushll.2d	v13, v13, #0x0
100809b88:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809b8c:     	ldr	q14, [x14, #0xca0]
100809b90:     	and.16b	v31, v31, v14
100809b94:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809b98:     	ldr	q14, [x14, #0xcb0]
100809b9c:     	and.16b	v8, v8, v14
100809ba0:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809ba4:     	ldr	q14, [x14, #0xcc0]
100809ba8:     	and.16b	v12, v12, v14
100809bac:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809bb0:     	ldr	q14, [x14, #0xcd0]
100809bb4:     	and.16b	v13, v13, v14
100809bb8:     	orr.16b	v30, v31, v30
100809bbc:     	orr.16b	v31, v30, v0
100809bc0:     	orr.16b	v0, v8, v10
100809bc4:     	orr.16b	v8, v0, v1
100809bc8:     	orr.16b	v0, v12, v11
100809bcc:     	orr.16b	v30, v0, v4
100809bd0:     	orr.16b	v0, v13, v9
100809bd4:     	orr.16b	v29, v0, v29
100809bd8:     	mov	x14, #0x20              ; =32
100809bdc:     	movk	x14, #0x21, lsl #32
100809be0:     	fmov	d0, x14
100809be4:     	and.8b	v0, v28, v0
100809be8:     	mov	x14, #0x22              ; =34
100809bec:     	movk	x14, #0x23, lsl #32
100809bf0:     	fmov	d1, x14
100809bf4:     	and.8b	v1, v28, v1
100809bf8:     	mov	x14, #0x24              ; =36
100809bfc:     	movk	x14, #0x25, lsl #32
100809c00:     	fmov	d4, x14
100809c04:     	and.8b	v4, v28, v4
100809c08:     	mov	x14, #0x26              ; =38
100809c0c:     	movk	x14, #0x27, lsl #32
100809c10:     	fmov	d9, x14
100809c14:     	and.8b	v9, v28, v9
100809c18:     	cmeq.2s	v0, v0, #0
100809c1c:     	sshll.2d	v0, v0, #0x0
100809c20:     	cmeq.2s	v1, v1, #0
100809c24:     	sshll.2d	v1, v1, #0x0
100809c28:     	cmeq.2s	v4, v4, #0
100809c2c:     	sshll.2d	v4, v4, #0x0
100809c30:     	cmeq.2s	v9, v9, #0
100809c34:     	sshll.2d	v9, v9, #0x0
100809c38:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809c3c:     	ldr	q10, [x14, #0xce0]
100809c40:     	and.16b	v0, v0, v10
100809c44:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809c48:     	ldr	q10, [x14, #0xcf0]
100809c4c:     	and.16b	v1, v1, v10
100809c50:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809c54:     	ldr	q10, [x14, #0xd00]
100809c58:     	and.16b	v4, v4, v10
100809c5c:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809c60:     	ldr	q10, [x14, #0xd10]
100809c64:     	and.16b	v9, v9, v10
100809c68:     	mov	x14, #0x28              ; =40
100809c6c:     	movk	x14, #0x29, lsl #32
100809c70:     	fmov	d10, x14
100809c74:     	and.8b	v10, v28, v10
100809c78:     	mov	x14, #0x2a              ; =42
100809c7c:     	movk	x14, #0x2b, lsl #32
100809c80:     	fmov	d11, x14
100809c84:     	and.8b	v11, v28, v11
100809c88:     	mov	x14, #0x2c              ; =44
100809c8c:     	movk	x14, #0x2d, lsl #32
100809c90:     	fmov	d12, x14
100809c94:     	and.8b	v12, v28, v12
100809c98:     	mov	x14, #0x2e              ; =46
100809c9c:     	movk	x14, #0x2f, lsl #32
100809ca0:     	fmov	d13, x14
100809ca4:     	and.8b	v13, v28, v13
100809ca8:     	cmeq.2s	v10, v10, #0
100809cac:     	sshll.2d	v10, v10, #0x0
100809cb0:     	cmeq.2s	v11, v11, #0
100809cb4:     	sshll.2d	v11, v11, #0x0
100809cb8:     	cmeq.2s	v12, v12, #0
100809cbc:     	sshll.2d	v12, v12, #0x0
100809cc0:     	cmeq.2s	v13, v13, #0
100809cc4:     	sshll.2d	v13, v13, #0x0
100809cc8:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809ccc:     	ldr	q14, [x14, #0xd20]
100809cd0:     	and.16b	v10, v10, v14
100809cd4:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809cd8:     	ldr	q14, [x14, #0xd30]
100809cdc:     	and.16b	v11, v11, v14
100809ce0:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809ce4:     	ldr	q14, [x14, #0xd40]
100809ce8:     	and.16b	v12, v12, v14
100809cec:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809cf0:     	ldr	q14, [x14, #0xd50]
100809cf4:     	and.16b	v13, v13, v14
100809cf8:     	orr.16b	v0, v10, v0
100809cfc:     	orr.16b	v1, v11, v1
100809d00:     	orr.16b	v4, v12, v4
100809d04:     	orr.16b	v9, v13, v9
100809d08:     	mov	x14, #0x30              ; =48
100809d0c:     	movk	x14, #0x31, lsl #32
100809d10:     	fmov	d10, x14
100809d14:     	and.8b	v10, v28, v10
100809d18:     	mov	x14, #0x32              ; =50
100809d1c:     	movk	x14, #0x33, lsl #32
100809d20:     	fmov	d11, x14
100809d24:     	and.8b	v11, v28, v11
100809d28:     	mov	x14, #0x34              ; =52
100809d2c:     	movk	x14, #0x35, lsl #32
100809d30:     	fmov	d12, x14
100809d34:     	and.8b	v12, v28, v12
100809d38:     	mov	x14, #0x36              ; =54
100809d3c:     	movk	x14, #0x37, lsl #32
100809d40:     	fmov	d13, x14
100809d44:     	and.8b	v13, v28, v13
100809d48:     	cmeq.2s	v10, v10, #0
100809d4c:     	sshll.2d	v10, v10, #0x0
100809d50:     	cmeq.2s	v11, v11, #0
100809d54:     	sshll.2d	v11, v11, #0x0
100809d58:     	cmeq.2s	v12, v12, #0
100809d5c:     	sshll.2d	v12, v12, #0x0
100809d60:     	cmeq.2s	v13, v13, #0
100809d64:     	sshll.2d	v13, v13, #0x0
100809d68:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809d6c:     	ldr	q14, [x14, #0xd60]
100809d70:     	and.16b	v10, v10, v14
100809d74:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809d78:     	ldr	q14, [x14, #0xd70]
100809d7c:     	and.16b	v11, v11, v14
100809d80:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809d84:     	ldr	q14, [x14, #0xd80]
100809d88:     	and.16b	v12, v12, v14
100809d8c:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809d90:     	ldr	q14, [x14, #0xd90]
100809d94:     	and.16b	v13, v13, v14
100809d98:     	orr.16b	v0, v10, v0
100809d9c:     	orr.16b	v0, v0, v31
100809da0:     	orr.16b	v1, v11, v1
100809da4:     	orr.16b	v1, v1, v8
100809da8:     	orr.16b	v4, v12, v4
100809dac:     	mov	x14, #0x38              ; =56
100809db0:     	movk	x14, #0x39, lsl #32
100809db4:     	fmov	d31, x14
100809db8:     	orr.16b	v4, v4, v30
100809dbc:     	mov	x14, #0x3a              ; =58
100809dc0:     	movk	x14, #0x3b, lsl #32
100809dc4:     	fmov	d30, x14
100809dc8:     	orr.16b	v8, v13, v9
100809dcc:     	mov	x14, #0x3c              ; =60
100809dd0:     	movk	x14, #0x3d, lsl #32
100809dd4:     	fmov	d9, x14
100809dd8:     	orr.16b	v29, v8, v29
100809ddc:     	mov	x14, #0x3e              ; =62
100809de0:     	movk	x14, #0x3f, lsl #32
100809de4:     	fmov	d8, x14
100809de8:     	and.8b	v31, v28, v31
100809dec:     	and.8b	v30, v28, v30
100809df0:     	and.8b	v9, v28, v9
100809df4:     	and.8b	v28, v28, v8
100809df8:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809dfc:     	ldr	q8, [x14, #0xda0]
100809e00:     	cmeq.2s	v31, v31, #0
100809e04:     	sshll.2d	v31, v31, #0x0
100809e08:     	and.16b	v31, v31, v8
100809e0c:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809e10:     	ldr	q8, [x14, #0xdb0]
100809e14:     	cmeq.2s	v30, v30, #0
100809e18:     	sshll.2d	v30, v30, #0x0
100809e1c:     	and.16b	v30, v30, v8
100809e20:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809e24:     	ldr	q8, [x14, #0xdc0]
100809e28:     	cmeq.2s	v9, v9, #0
100809e2c:     	sshll.2d	v9, v9, #0x0
100809e30:     	and.16b	v8, v9, v8
100809e34:     	adrp	x14, 0x100f05000 <GCC_except_table8391+0x14>
100809e38:     	ldr	q9, [x14, #0xdd0]
100809e3c:     	cmeq.2s	v28, v28, #0
100809e40:     	sshll.2d	v28, v28, #0x0
100809e44:     	and.16b	v28, v28, v9
100809e48:     	orr.16b	v0, v31, v0
100809e4c:     	orr.16b	v1, v30, v1
100809e50:     	orr.16b	v4, v8, v4
100809e54:     	orr.16b	v28, v28, v29
100809e58:     	orr.16b	v0, v1, v0
100809e5c:     	orr.16b	v0, v4, v0
100809e60:     	orr.16b	v0, v28, v0
100809e64:     	mov	d1, v0[1]
100809e68:     	orr.8b	v0, v0, v1
100809e6c:     	fmov	x30, d0
100809e70:     	cmp	x3, #0x1
100809e74:     	csinc	x23, x3, xzr, hi
100809e78:     	mov	w14, #0x10              ; =16
100809e7c:     	lsl	x14, x14, x8
100809e80:     	add	x1, x20, x22
100809e84:     	sub	x1, x1, #0x1
100809e88:     	madd	x1, x14, x1, x19
100809e8c:     	add	x1, x1, x23, lsl #3
100809e90:     	mov	w17, #0x8               ; =8
100809e94:     	lsl	x8, x17, x8
100809e98:     	add	x7, x19, x8
100809e9c:     	add	x8, x1, x8
100809ea0:     	cmp	x19, x8
100809ea4:     	ccmp	x7, x1, #0x2, lo
100809ea8:     	ccmp	x14, #0x0, #0x8, hs
100809eac:     	cset	w20, mi
100809eb0:     	and	x22, x23, #0x1ffffffffffffffc
100809eb4:     	lsl	x8, x6, #3
100809eb8:     	add	x14, x19, #0x10
100809ebc:     	add	x21, x14, x8
100809ec0:     	lsl	x25, x28, #3
100809ec4:     	add	x7, x19, x8
100809ec8:     	mov	x1, x19
100809ecc:     	add	x27, x19, #0x10
100809ed0:     	dup.2d	v28, x4
100809ed4:     	dup.2d	v29, x30
100809ed8:     	b	0x100809f18 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7f4>
100809edc:     	adds	x8, x28, x0
100809ee0:     	b.hs	0x10080a07c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
100809ee4:     	cmp	x8, x2
100809ee8:     	b.hi	0x10080a07c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
100809eec:     	mov	x0, x8
100809ef0:     	subs	x15, x15, #0x1
100809ef4:     	b.ne	0x100809edc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
100809ef8:     	b	0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
100809efc:     	add	x21, x21, x25
100809f00:     	add	x27, x27, x25
100809f04:     	add	x7, x7, x25
100809f08:     	add	x1, x1, x25
100809f0c:     	mov	x0, x8
100809f10:     	sub	x15, x15, #0x1
100809f14:     	cbz	x15, 0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
100809f18:     	adds	x8, x0, x28
100809f1c:     	b.hs	0x10080a080 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
100809f20:     	cmp	x8, x2
100809f24:     	b.hi	0x10080a080 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
100809f28:     	cmp	x3, #0x4
100809f2c:     	cset	w14, lo
100809f30:     	orr	w14, w14, w20
100809f34:     	tbz	w14, #0x0, 0x100809f40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x81c>
100809f38:     	mov	x6, #0x0                ; =0
100809f3c:     	b	0x100809fac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x888>
100809f40:     	mov	x0, x27
100809f44:     	mov	x6, x21
100809f48:     	and	x14, x23, #0x1ffffffffffffffc
100809f4c:     	ldp	q0, q1, [x0, #-0x10]
100809f50:     	neg.2d	v4, v28
100809f54:     	ushl.2d	v30, v0, v4
100809f58:     	ushl.2d	v4, v1, v4
100809f5c:     	ldp	q31, q8, [x6, #-0x10]
100809f60:     	eor.16b	v30, v30, v31
100809f64:     	eor.16b	v4, v4, v8
100809f68:     	and.16b	v30, v30, v29
100809f6c:     	and.16b	v4, v4, v29
100809f70:     	ushl.2d	v9, v30, v28
100809f74:     	ushl.2d	v10, v4, v28
100809f78:     	eor.16b	v0, v9, v0
100809f7c:     	eor.16b	v1, v10, v1
100809f80:     	stp	q0, q1, [x0, #-0x10]
100809f84:     	eor.16b	v0, v30, v31
100809f88:     	eor.16b	v1, v4, v8
100809f8c:     	stp	q0, q1, [x6, #-0x10]
100809f90:     	add	x6, x6, #0x20
100809f94:     	add	x0, x0, #0x20
100809f98:     	subs	x14, x14, #0x4
100809f9c:     	b.ne	0x100809f4c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x828>
100809fa0:     	and	x6, x23, #0x1ffffffffffffffc
100809fa4:     	cmp	x3, x22
100809fa8:     	b.eq	0x100809efc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
100809fac:     	lsl	x0, x6, #3
100809fb0:     	add	x14, x7, x0
100809fb4:     	add	x0, x1, x0
100809fb8:     	sub	x6, x23, x6
100809fbc:     	ldr	x5, [x0]
100809fc0:     	lsr	x24, x5, x4
100809fc4:     	ldr	x26, [x14]
100809fc8:     	eor	x24, x24, x26
100809fcc:     	and	x24, x24, x30
100809fd0:     	lsl	x17, x24, x4
100809fd4:     	eor	x17, x17, x5
100809fd8:     	str	x17, [x0], #0x8
100809fdc:     	eor	x17, x24, x26
100809fe0:     	str	x17, [x14], #0x8
100809fe4:     	subs	x6, x6, #0x1
100809fe8:     	b.ne	0x100809fbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x898>
100809fec:     	b	0x100809efc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
100809ff0:     	cbz	x2, 0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
100809ff4:     	mov	x8, #0x0                ; =0
100809ff8:     	add	w14, w1, #0x3a
100809ffc:     	lsl	x14, x16, x14
10080a000:     	add	w15, w15, #0x3a
10080a004:     	lsl	x15, x16, x15
10080a008:     	eor	x1, x15, x14
10080a00c:     	b	0x10080a01c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8f8>
10080a010:     	add	x8, x8, #0x1
10080a014:     	cmp	x2, x8
10080a018:     	b.eq	0x100809888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
10080a01c:     	tst	x8, x14
10080a020:     	b.eq	0x10080a010 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
10080a024:     	and	x17, x8, x15
10080a028:     	cbnz	x17, 0x10080a010 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
10080a02c:     	eor	x0, x1, x8
10080a030:     	cmp	x0, x2
10080a034:     	b.hs	0x10080a0e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9c0>
10080a038:     	ldr	x17, [x19, x8, lsl #3]
10080a03c:     	ldr	x3, [x19, x0, lsl #3]
10080a040:     	str	x3, [x19, x8, lsl #3]
10080a044:     	str	x17, [x19, x0, lsl #3]
10080a048:     	b	0x10080a010 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
10080a04c:     	ldp	x29, x30, [sp, #0xd0]
10080a050:     	ldp	x20, x19, [sp, #0xc0]
10080a054:     	ldp	x22, x21, [sp, #0xb0]
10080a058:     	ldp	x24, x23, [sp, #0xa0]
10080a05c:     	ldp	x26, x25, [sp, #0x90]
10080a060:     	ldp	x28, x27, [sp, #0x80]
10080a064:     	ldp	d9, d8, [sp, #0x70]
10080a068:     	ldp	d11, d10, [sp, #0x60]
10080a06c:     	ldp	d13, d12, [sp, #0x50]
10080a070:     	ldp	d15, d14, [sp, #0x40]
10080a074:     	add	sp, sp, #0xe0
10080a078:     	ret
10080a07c:     	add	x8, x28, x0
10080a080:     	adrp	x3, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
10080a084:     	add	x3, x3, #0xaa8
10080a088:     	mov	x1, x8
10080a08c:     	bl	0x100e81b94 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
10080a090:     	adrp	x0, 0x100f2d000 <dyld_stub_binder+0x100f2d000>
10080a094:     	add	x0, x0, #0xe84
10080a098:     	adrp	x2, 0x1010d0000 <dyld_stub_binder+0x1010d0000>
10080a09c:     	add	x2, x2, #0xe68
10080a0a0:     	mov	w1, #0x1b               ; =27
10080a0a4:     	bl	0x100e81c48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
10080a0a8:     	cmp	x28, x2
10080a0ac:     	b.hi	0x10080a0f8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9d4>
10080a0b0:     	adrp	x0, 0x100fca000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x12b0>
10080a0b4:     	add	x0, x0, #0x6b3
10080a0b8:     	adrp	x2, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
10080a0bc:     	add	x2, x2, #0xa90
10080a0c0:     	mov	w1, #0x13               ; =19
10080a0c4:     	bl	0x100e81af4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
10080a0c8:     	adrp	x5, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
10080a0cc:     	add	x5, x5, #0xa60
10080a0d0:     	add	x1, sp, #0x30
10080a0d4:     	add	x2, sp, #0x38
10080a0d8:     	mov	w0, #0x0                ; =0
10080a0dc:     	mov	x3, #0x0                ; =0
10080a0e0:     	bl	0x100e81b30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
10080a0e4:     	adrp	x8, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
10080a0e8:     	add	x8, x8, #0xa78
10080a0ec:     	mov	x1, x2
10080a0f0:     	mov	x2, x8
10080a0f4:     	bl	0x100e81c5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10080a0f8:     	mov	x0, #0x0                ; =0
10080a0fc:     	mov	x8, x28
10080a100:     	adrp	x3, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
10080a104:     	add	x3, x3, #0xaa8
10080a108:     	mov	x1, x8
10080a10c:     	bl	0x100e81b94 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
