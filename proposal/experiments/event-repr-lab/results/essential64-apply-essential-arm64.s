
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a1ba0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>:
100a1ba0c:     	sub	sp, sp, #0xe0
100a1ba10:     	stp	x26, x25, [sp, #0x90]
100a1ba14:     	stp	x24, x23, [sp, #0xa0]
100a1ba18:     	stp	x22, x21, [sp, #0xb0]
100a1ba1c:     	stp	x20, x19, [sp, #0xc0]
100a1ba20:     	stp	x29, x30, [sp, #0xd0]
100a1ba24:     	add	x29, sp, #0xd0
100a1ba28:     	mov	x8, x0
100a1ba2c:     	strb	w1, [sp, #0x7]
100a1ba30:     	and	w0, w1, #0xff
100a1ba34:     	stp	w2, w3, [sp, #0x8]
100a1ba38:     	cmp	w0, #0x10
100a1ba3c:     	b.hs	0x100a1bd8c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x380>
100a1ba40:     	cmp	w0, #0x9
100a1ba44:     	b.gt	0x100a1ba64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x58>
100a1ba48:     	cbz	w0, 0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1ba4c:     	cmp	w0, #0x3
100a1ba50:     	b.eq	0x100a1bb28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x11c>
100a1ba54:     	cmp	w0, #0x5
100a1ba58:     	b.ne	0x100a1ba84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x78>
100a1ba5c:     	eor	w0, w3, #0x1
100a1ba60:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1ba64:     	cmp	w0, #0xa
100a1ba68:     	b.eq	0x100a1bb04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xf8>
100a1ba6c:     	cmp	w0, #0xc
100a1ba70:     	b.eq	0x100a1bb38 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x12c>
100a1ba74:     	cmp	w0, #0xf
100a1ba78:     	b.ne	0x100a1ba84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x78>
100a1ba7c:     	mov	w0, #0x1                ; =1
100a1ba80:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1ba84:     	cmp	w2, #0x2
100a1ba88:     	b.hs	0x100a1baac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xa0>
100a1ba8c:     	ubfiz	x8, x2, #1, #7
100a1ba90:     	and	w9, w1, #0xff
100a1ba94:     	lsr	w8, w9, w8
100a1ba98:     	and	w8, w8, #0x3
100a1ba9c:     	cmp	w8, #0x1
100a1baa0:     	b.gt	0x100a1bafc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xf0>
100a1baa4:     	cbnz	w8, 0x100a1ba5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x50>
100a1baa8:     	b	0x100a1bad8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xcc>
100a1baac:     	cmp	w3, #0x2
100a1bab0:     	b.hs	0x100a1bae0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xd4>
100a1bab4:     	and	w9, w1, #0xff
100a1bab8:     	lsr	w8, w9, w3
100a1babc:     	and	w8, w8, #0x1
100a1bac0:     	orr	w10, w3, #0x2
100a1bac4:     	lsr	w9, w9, w10
100a1bac8:     	bfi	w8, w9, #1, #1
100a1bacc:     	cmp	w8, #0x1
100a1bad0:     	b.gt	0x100a1bb30 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x124>
100a1bad4:     	cbnz	w8, 0x100a1bb28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x11c>
100a1bad8:     	mov	w0, #0x0                ; =0
100a1badc:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1bae0:     	cmp	w2, w3
100a1bae4:     	b.ne	0x100a1bb0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x100>
100a1bae8:     	and	w8, w1, #0x1
100a1baec:     	ubfx	w9, w1, #3, #1
100a1baf0:     	orr	w8, w8, w9, lsl #1
100a1baf4:     	cmp	w8, #0x1
100a1baf8:     	b.le	0x100a1baa4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x98>
100a1bafc:     	cmp	w8, #0x2
100a1bb00:     	b.ne	0x100a1ba7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x70>
100a1bb04:     	mov	x0, x3
100a1bb08:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1bb0c:     	eor	w9, w3, w2
100a1bb10:     	cmp	w9, #0x1
100a1bb14:     	b.ne	0x100a1bb58 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x14c>
100a1bb18:     	ubfx	w8, w1, #1, #2
100a1bb1c:     	cmp	w8, #0x1
100a1bb20:     	b.gt	0x100a1bb30 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x124>
100a1bb24:     	cbz	w8, 0x100a1bad8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0xcc>
100a1bb28:     	eor	w0, w2, #0x1
100a1bb2c:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1bb30:     	cmp	w8, #0x2
100a1bb34:     	b.ne	0x100a1ba7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x70>
100a1bb38:     	mov	x0, x2
100a1bb3c:     	ldp	x29, x30, [sp, #0xd0]
100a1bb40:     	ldp	x20, x19, [sp, #0xc0]
100a1bb44:     	ldp	x22, x21, [sp, #0xb0]
100a1bb48:     	ldp	x24, x23, [sp, #0xa0]
100a1bb4c:     	ldp	x26, x25, [sp, #0x90]
100a1bb50:     	add	sp, sp, #0xe0
100a1bb54:     	ret
100a1bb58:     	mov	x20, x1
100a1bb5c:     	sturb	w1, [x29, #-0x48]
100a1bb60:     	mov	x23, x2
100a1bb64:     	stur	w2, [x29, #-0x4c]
100a1bb68:     	mov	x22, x3
100a1bb6c:     	stur	w3, [x29, #-0x44]
100a1bb70:     	mov	x19, x8
100a1bb74:     	add	x0, x8, #0x50
100a1bb78:     	sub	x1, x29, #0x4c
100a1bb7c:     	bl	0x10056d874 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100a1bb80:     	cbz	x0, 0x100a1bb8c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x180>
100a1bb84:     	ldr	w0, [x0]
100a1bb88:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1bb8c:     	ldr	x1, [x19, #0x28]
100a1bb90:     	mov	x8, x23
100a1bb94:     	lsr	w0, w8, #1
100a1bb98:     	cmp	x1, x0
100a1bb9c:     	b.ls	0x100a1bda4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x398>
100a1bba0:     	lsr	w8, w22, #1
100a1bba4:     	cmp	x1, x8
100a1bba8:     	b.ls	0x100a1bdb0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3a4>
100a1bbac:     	mov	x24, x19
100a1bbb0:     	ldr	x9, [x19, #0x20]
100a1bbb4:     	add	x10, x9, x0, lsl #5
100a1bbb8:     	ldr	x10, [x10, #0x18]
100a1bbbc:     	add	x8, x9, x8, lsl #5
100a1bbc0:     	ldr	x8, [x8, #0x18]
100a1bbc4:     	orr	x21, x8, x10
100a1bbc8:     	fmov	d0, x21
100a1bbcc:     	cnt.8b	v0, v0
100a1bbd0:     	addv.8b	b0, v0
100a1bbd4:     	fmov	x8, d0
100a1bbd8:     	cmp	x8, #0x7
100a1bbdc:     	b.hs	0x100a1bc68 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x25c>
100a1bbe0:     	add	x24, sp, #0x10
100a1bbe4:     	add	x0, sp, #0x10
100a1bbe8:     	mov	x1, x21
100a1bbec:     	bl	0x100b51868 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100a1bbf0:     	mov	x8, x19
100a1bbf4:     	ldrb	w9, [x19, #0x94]
100a1bbf8:     	tbz	w9, #0x0, 0x100a1bd20 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x314>
100a1bbfc:     	ldp	x1, x2, [x8, #0x20]
100a1bc00:     	add	x0, sp, #0x40
100a1bc04:     	mov	x3, x23
100a1bc08:     	mov	x4, x21
100a1bc0c:     	bl	0x100a1953c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1bc10:     	ldp	x23, x24, [sp, #0x48]
100a1bc14:     	ldp	x1, x2, [x19, #0x20]
100a1bc18:     	add	x0, sp, #0x58
100a1bc1c:     	mov	x3, x22
100a1bc20:     	mov	x4, x21
100a1bc24:     	bl	0x100a1953c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1bc28:     	ldp	x22, x5, [sp, #0x60]
100a1bc2c:     	add	x0, sp, #0x28
100a1bc30:     	mov	x1, x20
100a1bc34:     	mov	x2, x23
100a1bc38:     	mov	x3, x24
100a1bc3c:     	mov	x4, x22
100a1bc40:     	bl	0x100ba1a34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100a1bc44:     	ldr	x8, [sp, #0x58]
100a1bc48:     	cbz	x8, 0x100a1bc54 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x248>
100a1bc4c:     	mov	x0, x22
100a1bc50:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1bc54:     	ldr	x8, [sp, #0x40]
100a1bc58:     	cbz	x8, 0x100a1bd50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x344>
100a1bc5c:     	mov	x0, x23
100a1bc60:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1bc64:     	b	0x100a1bd50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x344>
100a1bc68:     	ldp	x0, x1, [x24, #0x8]
100a1bc6c:     	mov	x2, x21
100a1bc70:     	bl	0x100a17d00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a1bc74:     	mov	x21, x0
100a1bc78:     	mov	x0, x24
100a1bc7c:     	mov	x1, x23
100a1bc80:     	mov	x2, x21
100a1bc84:     	mov	w3, #0x0                ; =0
100a1bc88:     	bl	0x100a1d100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1bc8c:     	mov	x25, x0
100a1bc90:     	mov	x0, x24
100a1bc94:     	mov	x1, x23
100a1bc98:     	mov	x2, x21
100a1bc9c:     	mov	w3, #0x1                ; =1
100a1bca0:     	bl	0x100a1d100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1bca4:     	mov	x23, x0
100a1bca8:     	mov	x0, x24
100a1bcac:     	mov	x1, x22
100a1bcb0:     	mov	x2, x21
100a1bcb4:     	mov	w3, #0x0                ; =0
100a1bcb8:     	bl	0x100a1d100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1bcbc:     	mov	x26, x0
100a1bcc0:     	mov	x0, x24
100a1bcc4:     	mov	x1, x22
100a1bcc8:     	mov	x2, x21
100a1bccc:     	mov	w3, #0x1                ; =1
100a1bcd0:     	bl	0x100a1d100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E8cofactorB6_>
100a1bcd4:     	mov	x22, x0
100a1bcd8:     	mov	x0, x24
100a1bcdc:     	mov	x1, x20
100a1bce0:     	mov	x2, x25
100a1bce4:     	mov	x3, x26
100a1bce8:     	bl	0x100a1ba0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100a1bcec:     	mov	x25, x0
100a1bcf0:     	mov	x0, x24
100a1bcf4:     	mov	x1, x20
100a1bcf8:     	mov	x2, x23
100a1bcfc:     	mov	x3, x22
100a1bd00:     	bl	0x100a1ba0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
100a1bd04:     	mov	x3, x0
100a1bd08:     	mov	x0, x24
100a1bd0c:     	mov	x1, x21
100a1bd10:     	mov	x2, x25
100a1bd14:     	bl	0x100a1c5d4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
100a1bd18:     	mov	x20, x0
100a1bd1c:     	b	0x100a1bd74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x368>
100a1bd20:     	ldr	x9, [sp, #0x20]
100a1bd24:     	mov	w10, #0x1               ; =1
100a1bd28:     	lsl	x1, x10, x9
100a1bd2c:     	stp	x24, x8, [sp, #0x58]
100a1bd30:     	add	x8, sp, #0x8
100a1bd34:     	add	x9, sp, #0xc
100a1bd38:     	stp	x8, x9, [sp, #0x68]
100a1bd3c:     	add	x8, sp, #0x7
100a1bd40:     	str	x8, [sp, #0x78]
100a1bd44:     	add	x0, sp, #0x28
100a1bd48:     	add	x2, sp, #0x58
100a1bd4c:     	bl	0x1006ada80 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E5applys_0EB6_>
100a1bd50:     	add	x2, sp, #0x28
100a1bd54:     	mov	x0, x19
100a1bd58:     	mov	x1, x21
100a1bd5c:     	bl	0x100a1befc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
100a1bd60:     	mov	x20, x0
100a1bd64:     	ldr	x8, [sp, #0x10]
100a1bd68:     	cbz	x8, 0x100a1bd74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x368>
100a1bd6c:     	ldr	x0, [sp, #0x18]
100a1bd70:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1bd74:     	add	x0, x19, #0x50
100a1bd78:     	sub	x1, x29, #0x4c
100a1bd7c:     	mov	x2, x20
100a1bd80:     	bl	0x100aad7b0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100a1bd84:     	mov	x0, x20
100a1bd88:     	b	0x100a1bb3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x130>
100a1bd8c:     	adrp	x0, 0x1011bd000 <dyld_stub_binder+0x1011bd000>
100a1bd90:     	add	x0, x0, #0xe0c
100a1bd94:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1bd98:     	add	x2, x2, #0xd20
100a1bd9c:     	mov	w1, #0x19               ; =25
100a1bda0:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a1bda4:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a1bda8:     	add	x2, x2, #0xb78
100a1bdac:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1bdb0:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a1bdb4:     	add	x2, x2, #0xb78
100a1bdb8:     	mov	x0, x8
100a1bdbc:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1bdc0:     	mov	x19, x0
100a1bdc4:     	ldr	x8, [sp, #0x58]
100a1bdc8:     	cbz	x8, 0x100a1bddc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3d0>
100a1bdcc:     	mov	x0, x22
100a1bdd0:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1bdd4:     	b	0x100a1bddc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3d0>
100a1bdd8:     	mov	x19, x0
100a1bddc:     	ldr	x8, [sp, #0x40]
100a1bde0:     	cbz	x8, 0x100a1bdf4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3e8>
100a1bde4:     	mov	x0, x23
100a1bde8:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1bdec:     	b	0x100a1bdf4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3e8>
100a1bdf0:     	mov	x19, x0
100a1bdf4:     	ldr	x8, [sp, #0x10]
100a1bdf8:     	cbz	x8, 0x100a1be04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_+0x3f8>
100a1bdfc:     	ldr	x0, [sp, #0x18]
100a1be00:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1be04:     	mov	x0, x19
100a1be08:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
