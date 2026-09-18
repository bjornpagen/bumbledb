
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b956c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>:
100b956c0:     	sub	sp, sp, #0xf0
100b956c4:     	stp	x28, x27, [sp, #0x90]
100b956c8:     	stp	x26, x25, [sp, #0xa0]
100b956cc:     	stp	x24, x23, [sp, #0xb0]
100b956d0:     	stp	x22, x21, [sp, #0xc0]
100b956d4:     	stp	x20, x19, [sp, #0xd0]
100b956d8:     	stp	x29, x30, [sp, #0xe0]
100b956dc:     	add	x29, sp, #0xe0
100b956e0:     	and	w8, w1, #0xff
100b956e4:     	cmp	w8, #0x10
100b956e8:     	b.hs	0x100b95ac0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x400>
100b956ec:     	mov	x19, x3
100b956f0:     	ldrb	w8, [x0, #0xe5]
100b956f4:     	tbz	w8, #0x0, 0x100b957a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xe0>
100b956f8:     	and	w8, w1, #0xfc
100b956fc:     	ubfiz	w9, w1, #2, #2
100b95700:     	orr	w8, w9, w8, lsr #2
100b95704:     	and	w9, w2, #0xfffffffe
100b95708:     	tst	w2, #0x1
100b9570c:     	csel	w9, w2, w9, eq
100b95710:     	csel	w8, w1, w8, eq
100b95714:     	mov	w10, #0xa               ; =10
100b95718:     	and	w10, w10, w8, lsl #1
100b9571c:     	mov	w11, #0x5               ; =5
100b95720:     	and	w11, w11, w8, lsr #1
100b95724:     	orr	w10, w10, w11
100b95728:     	and	w11, w19, #0xfffffffe
100b9572c:     	tst	w19, #0x1
100b95730:     	csel	w11, w19, w11, eq
100b95734:     	csel	w8, w8, w10, eq
100b95738:     	ands	w27, w8, #0x1
100b9573c:     	eor	w10, w8, #0xf
100b95740:     	csel	w8, w8, w10, eq
100b95744:     	mov	w10, #0x9               ; =9
100b95748:     	and	w10, w8, w10
100b9574c:     	lsr	w12, w8, #1
100b95750:     	bfi	w10, w12, #2, #1
100b95754:     	and	w12, w12, #0x2
100b95758:     	orr	w10, w10, w12
100b9575c:     	cmp	w9, w11
100b95760:     	csel	w19, w11, w9, ls
100b95764:     	csel	w2, w9, w11, ls
100b95768:     	csel	w1, w8, w10, ls
100b9576c:     	strb	w1, [sp, #0x7]
100b95770:     	stp	w2, w19, [sp, #0x8]
100b95774:     	and	w21, w1, #0xff
100b95778:     	cmp	w21, #0x9
100b9577c:     	b.le	0x100b957b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xf8>
100b95780:     	cmp	w21, #0xa
100b95784:     	b.eq	0x100b9585c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x19c>
100b95788:     	cmp	w21, #0xc
100b9578c:     	b.eq	0x100b95884 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c4>
100b95790:     	cmp	w21, #0xf
100b95794:     	b.ne	0x100b957d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x114>
100b95798:     	mov	w21, #0x1               ; =1
100b9579c:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b957a0:     	mov	w27, #0x0               ; =0
100b957a4:     	strb	w1, [sp, #0x7]
100b957a8:     	stp	w2, w19, [sp, #0x8]
100b957ac:     	and	w21, w1, #0xff
100b957b0:     	cmp	w21, #0x9
100b957b4:     	b.gt	0x100b95780 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xc0>
100b957b8:     	cbz	w21, 0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b957bc:     	cmp	w21, #0x3
100b957c0:     	b.eq	0x100b95830 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x170>
100b957c4:     	cmp	w21, #0x5
100b957c8:     	b.ne	0x100b957d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x114>
100b957cc:     	eor	w21, w19, #0x1
100b957d0:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b957d4:     	cmp	w2, #0x2
100b957d8:     	b.hs	0x100b957fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x13c>
100b957dc:     	ubfiz	x8, x2, #1, #7
100b957e0:     	and	w9, w1, #0xff
100b957e4:     	lsr	w8, w9, w8
100b957e8:     	and	w8, w8, #0x3
100b957ec:     	cmp	w8, #0x1
100b957f0:     	b.gt	0x100b95854 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x194>
100b957f4:     	cbnz	w8, 0x100b957cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x10c>
100b957f8:     	b	0x100b95828 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x168>
100b957fc:     	cmp	w19, #0x2
100b95800:     	b.hs	0x100b95838 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x178>
100b95804:     	and	w9, w1, #0xff
100b95808:     	lsr	w8, w9, w19
100b9580c:     	and	w8, w8, #0x1
100b95810:     	orr	w10, w19, #0x2
100b95814:     	lsr	w9, w9, w10
100b95818:     	bfi	w8, w9, #1, #1
100b9581c:     	cmp	w8, #0x1
100b95820:     	b.gt	0x100b9587c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1bc>
100b95824:     	cbnz	w8, 0x100b95830 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x170>
100b95828:     	mov	w21, #0x0               ; =0
100b9582c:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b95830:     	eor	w21, w2, #0x1
100b95834:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b95838:     	cmp	w2, w19
100b9583c:     	b.ne	0x100b95864 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1a4>
100b95840:     	and	w8, w1, #0x1
100b95844:     	ubfx	w9, w1, #3, #1
100b95848:     	orr	w8, w8, w9, lsl #1
100b9584c:     	cmp	w8, #0x1
100b95850:     	b.le	0x100b957f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x134>
100b95854:     	cmp	w8, #0x2
100b95858:     	b.ne	0x100b95798 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xd8>
100b9585c:     	mov	x21, x19
100b95860:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b95864:     	eor	w8, w2, w19
100b95868:     	cmp	w8, #0x1
100b9586c:     	b.ne	0x100b958b0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1f0>
100b95870:     	ubfx	w8, w1, #1, #2
100b95874:     	cmp	w8, #0x1
100b95878:     	b.le	0x100b95824 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x164>
100b9587c:     	cmp	w8, #0x2
100b95880:     	b.ne	0x100b95798 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xd8>
100b95884:     	mov	x21, x2
100b95888:     	and	w8, w27, #0xff
100b9588c:     	eor	w0, w21, w8
100b95890:     	ldp	x29, x30, [sp, #0xe0]
100b95894:     	ldp	x20, x19, [sp, #0xd0]
100b95898:     	ldp	x22, x21, [sp, #0xc0]
100b9589c:     	ldp	x24, x23, [sp, #0xb0]
100b958a0:     	ldp	x26, x25, [sp, #0xa0]
100b958a4:     	ldp	x28, x27, [sp, #0x90]
100b958a8:     	add	sp, sp, #0xf0
100b958ac:     	ret
100b958b0:     	mov	x22, x1
100b958b4:     	sturb	w1, [x29, #-0x58]
100b958b8:     	mov	x23, x2
100b958bc:     	stur	w2, [x29, #-0x5c]
100b958c0:     	stur	w19, [x29, #-0x54]
100b958c4:     	mov	x20, x0
100b958c8:     	add	x0, x0, #0xa0
100b958cc:     	sub	x1, x29, #0x5c
100b958d0:     	bl	0x10065e8f0 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100b958d4:     	cbz	x0, 0x100b958e0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x220>
100b958d8:     	ldr	w21, [x0]
100b958dc:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b958e0:     	mov	x0, x20
100b958e4:     	mov	x1, x23
100b958e8:     	bl	0x100b8f5c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E9variablesB6_>
100b958ec:     	mov	x24, x0
100b958f0:     	mov	x0, x20
100b958f4:     	mov	x1, x19
100b958f8:     	bl	0x100b8f5c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E9variablesB6_>
100b958fc:     	orr	x21, x0, x24
100b95900:     	fmov	d0, x21
100b95904:     	cnt.8b	v0, v0
100b95908:     	addv.8b	b0, v0
100b9590c:     	fmov	x8, d0
100b95910:     	cmp	x8, #0x7
100b95914:     	b.hs	0x100b959a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x2e0>
100b95918:     	add	x24, sp, #0x10
100b9591c:     	add	x0, sp, #0x10
100b95920:     	mov	x1, x21
100b95924:     	bl	0x100cc56e8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100b95928:     	mov	x8, x20
100b9592c:     	ldrb	w9, [x20, #0xe4]
100b95930:     	tbz	w9, #0x0, 0x100b95a58 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x398>
100b95934:     	mov	x1, x8
100b95938:     	add	x0, sp, #0x40
100b9593c:     	mov	x2, x23
100b95940:     	mov	x3, x21
100b95944:     	bl	0x100b8eda0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100b95948:     	ldp	x23, x25, [sp, #0x48]
100b9594c:     	add	x0, sp, #0x58
100b95950:     	mov	x1, x20
100b95954:     	mov	x2, x19
100b95958:     	mov	x3, x21
100b9595c:     	bl	0x100b8eda0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100b95960:     	ldp	x24, x5, [sp, #0x60]
100b95964:     	add	x0, sp, #0x28
100b95968:     	mov	x1, x22
100b9596c:     	mov	x2, x23
100b95970:     	mov	x3, x25
100b95974:     	mov	x4, x24
100b95978:     	bl	0x100d15334 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100b9597c:     	ldr	x8, [sp, #0x58]
100b95980:     	cbz	x8, 0x100b9598c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x2cc>
100b95984:     	mov	x0, x24
100b95988:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b9598c:     	ldr	x8, [sp, #0x40]
100b95990:     	cbz	x8, 0x100b95a88 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3c8>
100b95994:     	mov	x0, x23
100b95998:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b9599c:     	b	0x100b95a88 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3c8>
100b959a0:     	mov	x0, x20
100b959a4:     	mov	x1, x21
100b959a8:     	bl	0x100b8d6c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100b959ac:     	mov	x21, x0
100b959b0:     	mov	x0, x20
100b959b4:     	mov	x1, x23
100b959b8:     	mov	x2, x21
100b959bc:     	mov	w3, #0x0                ; =0
100b959c0:     	bl	0x100b96dd4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100b959c4:     	mov	x25, x0
100b959c8:     	mov	x0, x20
100b959cc:     	mov	x1, x23
100b959d0:     	mov	x2, x21
100b959d4:     	mov	w3, #0x1                ; =1
100b959d8:     	bl	0x100b96dd4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100b959dc:     	mov	x23, x0
100b959e0:     	mov	x0, x20
100b959e4:     	mov	x1, x19
100b959e8:     	mov	x2, x21
100b959ec:     	mov	w3, #0x0                ; =0
100b959f0:     	bl	0x100b96dd4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100b959f4:     	mov	x26, x0
100b959f8:     	mov	x0, x20
100b959fc:     	mov	x1, x19
100b95a00:     	mov	x2, x21
100b95a04:     	mov	w3, #0x1                ; =1
100b95a08:     	bl	0x100b96dd4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100b95a0c:     	mov	x19, x0
100b95a10:     	mov	x0, x20
100b95a14:     	mov	x1, x22
100b95a18:     	mov	x2, x25
100b95a1c:     	mov	x3, x26
100b95a20:     	bl	0x100b956c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100b95a24:     	mov	x25, x0
100b95a28:     	mov	x0, x20
100b95a2c:     	mov	x1, x22
100b95a30:     	mov	x2, x23
100b95a34:     	mov	x3, x19
100b95a38:     	bl	0x100b956c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100b95a3c:     	mov	x3, x0
100b95a40:     	mov	x0, x20
100b95a44:     	mov	x1, x21
100b95a48:     	mov	x2, x25
100b95a4c:     	bl	0x100b9610c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
100b95a50:     	mov	x21, x0
100b95a54:     	b	0x100b95aac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3ec>
100b95a58:     	ldr	x9, [sp, #0x20]
100b95a5c:     	mov	w10, #0x1               ; =1
100b95a60:     	lsl	x1, x10, x9
100b95a64:     	stp	x24, x8, [sp, #0x58]
100b95a68:     	add	x8, sp, #0x8
100b95a6c:     	add	x9, sp, #0xc
100b95a70:     	stp	x8, x9, [sp, #0x68]
100b95a74:     	add	x8, sp, #0x7
100b95a78:     	str	x8, [sp, #0x78]
100b95a7c:     	add	x0, sp, #0x28
100b95a80:     	add	x2, sp, #0x58
100b95a84:     	bl	0x1007aebc0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E11apply_inners_0EB6_>
100b95a88:     	add	x2, sp, #0x28
100b95a8c:     	mov	x0, x20
100b95a90:     	mov	x1, x21
100b95a94:     	bl	0x100b95b24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
100b95a98:     	mov	x21, x0
100b95a9c:     	ldr	x8, [sp, #0x10]
100b95aa0:     	cbz	x8, 0x100b95aac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3ec>
100b95aa4:     	ldr	x0, [sp, #0x18]
100b95aa8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b95aac:     	add	x0, x20, #0xa0
100b95ab0:     	sub	x1, x29, #0x5c
100b95ab4:     	mov	x2, x21
100b95ab8:     	bl	0x100c267ac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100b95abc:     	b	0x100b95888 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100b95ac0:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100b95ac4:     	add	x0, x0, #0x70a
100b95ac8:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b95acc:     	add	x2, x2, #0x8d0
100b95ad0:     	mov	w1, #0x19               ; =25
100b95ad4:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b95ad8:     	mov	x19, x0
100b95adc:     	ldr	x8, [sp, #0x58]
100b95ae0:     	cbz	x8, 0x100b95af4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x434>
100b95ae4:     	mov	x0, x24
100b95ae8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b95aec:     	b	0x100b95af4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x434>
100b95af0:     	mov	x19, x0
100b95af4:     	ldr	x8, [sp, #0x40]
100b95af8:     	cbz	x8, 0x100b95b0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x44c>
100b95afc:     	mov	x0, x23
100b95b00:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b95b04:     	b	0x100b95b0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x44c>
100b95b08:     	mov	x19, x0
100b95b0c:     	ldr	x8, [sp, #0x10]
100b95b10:     	cbz	x8, 0x100b95b1c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x45c>
100b95b14:     	ldr	x0, [sp, #0x18]
100b95b18:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b95b1c:     	mov	x0, x19
100b95b20:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
