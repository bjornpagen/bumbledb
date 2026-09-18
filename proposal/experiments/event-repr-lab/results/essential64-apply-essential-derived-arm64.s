
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a1d600 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>:
100a1d600:     	sub	sp, sp, #0xf0
100a1d604:     	stp	x28, x27, [sp, #0x90]
100a1d608:     	stp	x26, x25, [sp, #0xa0]
100a1d60c:     	stp	x24, x23, [sp, #0xb0]
100a1d610:     	stp	x22, x21, [sp, #0xc0]
100a1d614:     	stp	x20, x19, [sp, #0xd0]
100a1d618:     	stp	x29, x30, [sp, #0xe0]
100a1d61c:     	add	x29, sp, #0xe0
100a1d620:     	and	w8, w1, #0xff
100a1d624:     	cmp	w8, #0x10
100a1d628:     	b.hs	0x100a1da1c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x41c>
100a1d62c:     	mov	x20, x3
100a1d630:     	mov	x19, x0
100a1d634:     	ldrb	w8, [x0, #0xad]
100a1d638:     	tbz	w8, #0x0, 0x100a1d6e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xe4>
100a1d63c:     	and	w8, w1, #0xfc
100a1d640:     	ubfiz	w9, w1, #2, #2
100a1d644:     	orr	w8, w9, w8, lsr #2
100a1d648:     	and	w9, w2, #0xfffffffe
100a1d64c:     	tst	w2, #0x1
100a1d650:     	csel	w9, w2, w9, eq
100a1d654:     	csel	w8, w1, w8, eq
100a1d658:     	mov	w10, #0xa               ; =10
100a1d65c:     	and	w10, w10, w8, lsl #1
100a1d660:     	mov	w11, #0x5               ; =5
100a1d664:     	and	w11, w11, w8, lsr #1
100a1d668:     	orr	w10, w10, w11
100a1d66c:     	and	w11, w20, #0xfffffffe
100a1d670:     	tst	w20, #0x1
100a1d674:     	csel	w11, w20, w11, eq
100a1d678:     	csel	w8, w8, w10, eq
100a1d67c:     	ands	w27, w8, #0x1
100a1d680:     	eor	w10, w8, #0xf
100a1d684:     	csel	w8, w8, w10, eq
100a1d688:     	mov	w10, #0x9               ; =9
100a1d68c:     	and	w10, w8, w10
100a1d690:     	lsr	w12, w8, #1
100a1d694:     	bfi	w10, w12, #2, #1
100a1d698:     	and	w12, w12, #0x2
100a1d69c:     	orr	w10, w10, w12
100a1d6a0:     	cmp	w9, w11
100a1d6a4:     	csel	w20, w11, w9, ls
100a1d6a8:     	csel	w2, w9, w11, ls
100a1d6ac:     	csel	w1, w8, w10, ls
100a1d6b0:     	strb	w1, [sp, #0x7]
100a1d6b4:     	stp	w2, w20, [sp, #0x8]
100a1d6b8:     	and	w21, w1, #0xff
100a1d6bc:     	cmp	w21, #0x9
100a1d6c0:     	b.le	0x100a1d6fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xfc>
100a1d6c4:     	cmp	w21, #0xa
100a1d6c8:     	b.eq	0x100a1d7a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1a0>
100a1d6cc:     	cmp	w21, #0xc
100a1d6d0:     	b.eq	0x100a1d7c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c8>
100a1d6d4:     	cmp	w21, #0xf
100a1d6d8:     	b.ne	0x100a1d718 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x118>
100a1d6dc:     	mov	w21, #0x1               ; =1
100a1d6e0:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d6e4:     	mov	w27, #0x0               ; =0
100a1d6e8:     	strb	w1, [sp, #0x7]
100a1d6ec:     	stp	w2, w20, [sp, #0x8]
100a1d6f0:     	and	w21, w1, #0xff
100a1d6f4:     	cmp	w21, #0x9
100a1d6f8:     	b.gt	0x100a1d6c4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xc4>
100a1d6fc:     	cbz	w21, 0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d700:     	cmp	w21, #0x3
100a1d704:     	b.eq	0x100a1d774 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x174>
100a1d708:     	cmp	w21, #0x5
100a1d70c:     	b.ne	0x100a1d718 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x118>
100a1d710:     	eor	w21, w20, #0x1
100a1d714:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d718:     	cmp	w2, #0x2
100a1d71c:     	b.hs	0x100a1d740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x140>
100a1d720:     	ubfiz	x8, x2, #1, #7
100a1d724:     	and	w9, w1, #0xff
100a1d728:     	lsr	w8, w9, w8
100a1d72c:     	and	w8, w8, #0x3
100a1d730:     	cmp	w8, #0x1
100a1d734:     	b.gt	0x100a1d798 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x198>
100a1d738:     	cbnz	w8, 0x100a1d710 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x110>
100a1d73c:     	b	0x100a1d76c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x16c>
100a1d740:     	cmp	w20, #0x2
100a1d744:     	b.hs	0x100a1d77c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x17c>
100a1d748:     	and	w9, w1, #0xff
100a1d74c:     	lsr	w8, w9, w20
100a1d750:     	and	w8, w8, #0x1
100a1d754:     	orr	w10, w20, #0x2
100a1d758:     	lsr	w9, w9, w10
100a1d75c:     	bfi	w8, w9, #1, #1
100a1d760:     	cmp	w8, #0x1
100a1d764:     	b.gt	0x100a1d7c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1c0>
100a1d768:     	cbnz	w8, 0x100a1d774 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x174>
100a1d76c:     	mov	w21, #0x0               ; =0
100a1d770:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d774:     	eor	w21, w2, #0x1
100a1d778:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d77c:     	cmp	w2, w20
100a1d780:     	b.ne	0x100a1d7a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1a8>
100a1d784:     	and	w8, w1, #0x1
100a1d788:     	ubfx	w9, w1, #3, #1
100a1d78c:     	orr	w8, w8, w9, lsl #1
100a1d790:     	cmp	w8, #0x1
100a1d794:     	b.le	0x100a1d738 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x138>
100a1d798:     	cmp	w8, #0x2
100a1d79c:     	b.ne	0x100a1d6dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xdc>
100a1d7a0:     	mov	x21, x20
100a1d7a4:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d7a8:     	eor	w8, w2, w20
100a1d7ac:     	cmp	w8, #0x1
100a1d7b0:     	b.ne	0x100a1d7f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1f4>
100a1d7b4:     	ubfx	w8, w1, #1, #2
100a1d7b8:     	cmp	w8, #0x1
100a1d7bc:     	b.le	0x100a1d768 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x168>
100a1d7c0:     	cmp	w8, #0x2
100a1d7c4:     	b.ne	0x100a1d6dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xdc>
100a1d7c8:     	mov	x21, x2
100a1d7cc:     	and	w8, w27, #0xff
100a1d7d0:     	eor	w0, w21, w8
100a1d7d4:     	ldp	x29, x30, [sp, #0xe0]
100a1d7d8:     	ldp	x20, x19, [sp, #0xd0]
100a1d7dc:     	ldp	x22, x21, [sp, #0xc0]
100a1d7e0:     	ldp	x24, x23, [sp, #0xb0]
100a1d7e4:     	ldp	x26, x25, [sp, #0xa0]
100a1d7e8:     	ldp	x28, x27, [sp, #0x90]
100a1d7ec:     	add	sp, sp, #0xf0
100a1d7f0:     	ret
100a1d7f4:     	mov	x21, x1
100a1d7f8:     	sturb	w1, [x29, #-0x58]
100a1d7fc:     	mov	x23, x2
100a1d800:     	stur	w2, [x29, #-0x5c]
100a1d804:     	stur	w20, [x29, #-0x54]
100a1d808:     	add	x0, x19, #0x68
100a1d80c:     	sub	x1, x29, #0x5c
100a1d810:     	bl	0x10056d874 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100a1d814:     	cbz	x0, 0x100a1d820 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x220>
100a1d818:     	ldr	w21, [x0]
100a1d81c:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1d820:     	ldr	x24, [x19, #0x28]
100a1d824:     	mov	x8, x23
100a1d828:     	lsr	w0, w8, #1
100a1d82c:     	cmp	x24, x0
100a1d830:     	b.ls	0x100a1da34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x434>
100a1d834:     	lsr	w8, w20, #1
100a1d838:     	cmp	x24, x8
100a1d83c:     	b.ls	0x100a1da44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x444>
100a1d840:     	ldr	x25, [x19, #0x20]
100a1d844:     	add	x9, x25, x0, lsl #5
100a1d848:     	ldr	x9, [x9, #0x18]
100a1d84c:     	add	x8, x25, x8, lsl #5
100a1d850:     	ldr	x8, [x8, #0x18]
100a1d854:     	orr	x22, x8, x9
100a1d858:     	fmov	d0, x22
100a1d85c:     	cnt.8b	v0, v0
100a1d860:     	addv.8b	b0, v0
100a1d864:     	fmov	x8, d0
100a1d868:     	cmp	x8, #0x7
100a1d86c:     	b.hs	0x100a1d8fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x2fc>
100a1d870:     	add	x26, sp, #0x10
100a1d874:     	add	x0, sp, #0x10
100a1d878:     	mov	x1, x22
100a1d87c:     	bl	0x100b53628 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100a1d880:     	ldrb	w8, [x19, #0xac]
100a1d884:     	tbz	w8, #0x0, 0x100a1d9b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3b4>
100a1d888:     	add	x0, sp, #0x40
100a1d88c:     	mov	x1, x25
100a1d890:     	mov	x2, x24
100a1d894:     	mov	x3, x23
100a1d898:     	mov	x4, x22
100a1d89c:     	bl	0x100a1aab8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1d8a0:     	ldp	x23, x26, [sp, #0x48]
100a1d8a4:     	add	x0, sp, #0x58
100a1d8a8:     	mov	x1, x25
100a1d8ac:     	mov	x2, x24
100a1d8b0:     	mov	x3, x20
100a1d8b4:     	mov	x4, x22
100a1d8b8:     	bl	0x100a1aab8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1d8bc:     	ldp	x20, x5, [sp, #0x60]
100a1d8c0:     	add	x0, sp, #0x28
100a1d8c4:     	mov	x1, x21
100a1d8c8:     	mov	x2, x23
100a1d8cc:     	mov	x3, x26
100a1d8d0:     	mov	x4, x20
100a1d8d4:     	bl	0x100ba3d34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100a1d8d8:     	ldr	x8, [sp, #0x58]
100a1d8dc:     	cbz	x8, 0x100a1d8e8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x2e8>
100a1d8e0:     	mov	x0, x20
100a1d8e4:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1d8e8:     	ldr	x8, [sp, #0x40]
100a1d8ec:     	cbz	x8, 0x100a1d9e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3e4>
100a1d8f0:     	mov	x0, x23
100a1d8f4:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1d8f8:     	b	0x100a1d9e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3e4>
100a1d8fc:     	mov	x0, x19
100a1d900:     	mov	x1, x22
100a1d904:     	bl	0x100a18fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a1d908:     	mov	x22, x0
100a1d90c:     	mov	x0, x19
100a1d910:     	mov	x1, x23
100a1d914:     	mov	x2, x22
100a1d918:     	mov	w3, #0x0                ; =0
100a1d91c:     	bl	0x100a1ef98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1d920:     	mov	x24, x0
100a1d924:     	mov	x0, x19
100a1d928:     	mov	x1, x23
100a1d92c:     	mov	x2, x22
100a1d930:     	mov	w3, #0x1                ; =1
100a1d934:     	bl	0x100a1ef98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1d938:     	mov	x23, x0
100a1d93c:     	mov	x0, x19
100a1d940:     	mov	x1, x20
100a1d944:     	mov	x2, x22
100a1d948:     	mov	w3, #0x0                ; =0
100a1d94c:     	bl	0x100a1ef98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1d950:     	mov	x25, x0
100a1d954:     	mov	x0, x19
100a1d958:     	mov	x1, x20
100a1d95c:     	mov	x2, x22
100a1d960:     	mov	w3, #0x1                ; =1
100a1d964:     	bl	0x100a1ef98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1d968:     	mov	x20, x0
100a1d96c:     	mov	x0, x19
100a1d970:     	mov	x1, x21
100a1d974:     	mov	x2, x24
100a1d978:     	mov	x3, x25
100a1d97c:     	bl	0x100a1d600 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100a1d980:     	mov	x24, x0
100a1d984:     	mov	x0, x19
100a1d988:     	mov	x1, x21
100a1d98c:     	mov	x2, x23
100a1d990:     	mov	x3, x20
100a1d994:     	bl	0x100a1d600 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100a1d998:     	mov	x3, x0
100a1d99c:     	mov	x0, x19
100a1d9a0:     	mov	x1, x22
100a1d9a4:     	mov	x2, x24
100a1d9a8:     	bl	0x100a1e220 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
100a1d9ac:     	mov	x21, x0
100a1d9b0:     	b	0x100a1da08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x408>
100a1d9b4:     	ldr	x8, [sp, #0x20]
100a1d9b8:     	mov	w9, #0x1                ; =1
100a1d9bc:     	lsl	x1, x9, x8
100a1d9c0:     	stp	x26, x19, [sp, #0x58]
100a1d9c4:     	add	x8, sp, #0x8
100a1d9c8:     	add	x9, sp, #0xc
100a1d9cc:     	stp	x8, x9, [sp, #0x68]
100a1d9d0:     	add	x8, sp, #0x7
100a1d9d4:     	str	x8, [sp, #0x78]
100a1d9d8:     	add	x0, sp, #0x28
100a1d9dc:     	add	x2, sp, #0x58
100a1d9e0:     	bl	0x1006adc40 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E11apply_inners_0EB6_>
100a1d9e4:     	add	x2, sp, #0x28
100a1d9e8:     	mov	x0, x19
100a1d9ec:     	mov	x1, x22
100a1d9f0:     	bl	0x100a1dc38 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
100a1d9f4:     	mov	x21, x0
100a1d9f8:     	ldr	x8, [sp, #0x10]
100a1d9fc:     	cbz	x8, 0x100a1da08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x408>
100a1da00:     	ldr	x0, [sp, #0x18]
100a1da04:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1da08:     	add	x0, x19, #0x68
100a1da0c:     	sub	x1, x29, #0x5c
100a1da10:     	mov	x2, x21
100a1da14:     	bl	0x100aaf670 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100a1da18:     	b	0x100a1d7cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x1cc>
100a1da1c:     	adrp	x0, 0x1011c1000 <dyld_stub_binder+0x1011c1000>
100a1da20:     	add	x0, x0, #0x5e2
100a1da24:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a1da28:     	add	x2, x2, #0xe28
100a1da2c:     	mov	w1, #0x19               ; =25
100a1da30:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a1da34:     	adrp	x2, 0x101339000 <dyld_stub_binder+0x101339000>
100a1da38:     	add	x2, x2, #0xba8
100a1da3c:     	mov	x1, x24
100a1da40:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1da44:     	adrp	x2, 0x101339000 <dyld_stub_binder+0x101339000>
100a1da48:     	add	x2, x2, #0xba8
100a1da4c:     	mov	x0, x8
100a1da50:     	mov	x1, x24
100a1da54:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1da58:     	mov	x19, x0
100a1da5c:     	ldr	x8, [sp, #0x58]
100a1da60:     	cbz	x8, 0x100a1da74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x474>
100a1da64:     	mov	x0, x20
100a1da68:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1da6c:     	b	0x100a1da74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x474>
100a1da70:     	mov	x19, x0
100a1da74:     	ldr	x8, [sp, #0x40]
100a1da78:     	cbz	x8, 0x100a1da8c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x48c>
100a1da7c:     	mov	x0, x23
100a1da80:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1da84:     	b	0x100a1da8c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x48c>
100a1da88:     	mov	x19, x0
100a1da8c:     	ldr	x8, [sp, #0x10]
100a1da90:     	cbz	x8, 0x100a1da9c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x49c>
100a1da94:     	ldr	x0, [sp, #0x18]
100a1da98:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a1da9c:     	mov	x0, x19
100a1daa0:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
