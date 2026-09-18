
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a1eb04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>:
100a1eb04:     	sub	sp, sp, #0xe0
100a1eb08:     	stp	x26, x25, [sp, #0x90]
100a1eb0c:     	stp	x24, x23, [sp, #0xa0]
100a1eb10:     	stp	x22, x21, [sp, #0xb0]
100a1eb14:     	stp	x20, x19, [sp, #0xc0]
100a1eb18:     	stp	x29, x30, [sp, #0xd0]
100a1eb1c:     	add	x29, sp, #0xd0
100a1eb20:     	mov	x8, x0
100a1eb24:     	strb	w1, [sp, #0x7]
100a1eb28:     	and	w0, w1, #0xff
100a1eb2c:     	stp	w2, w3, [sp, #0x8]
100a1eb30:     	cmp	w0, #0x10
100a1eb34:     	b.hs	0x100a1ee84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x380>
100a1eb38:     	cmp	w0, #0x9
100a1eb3c:     	b.gt	0x100a1eb5c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x58>
100a1eb40:     	cbz	w0, 0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1eb44:     	cmp	w0, #0x3
100a1eb48:     	b.eq	0x100a1ec20 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x11c>
100a1eb4c:     	cmp	w0, #0x5
100a1eb50:     	b.ne	0x100a1eb7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x78>
100a1eb54:     	eor	w0, w3, #0x1
100a1eb58:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1eb5c:     	cmp	w0, #0xa
100a1eb60:     	b.eq	0x100a1ebfc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xf8>
100a1eb64:     	cmp	w0, #0xc
100a1eb68:     	b.eq	0x100a1ec30 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x12c>
100a1eb6c:     	cmp	w0, #0xf
100a1eb70:     	b.ne	0x100a1eb7c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x78>
100a1eb74:     	mov	w0, #0x1                ; =1
100a1eb78:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1eb7c:     	cmp	w2, #0x2
100a1eb80:     	b.hs	0x100a1eba4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xa0>
100a1eb84:     	ubfiz	x8, x2, #1, #7
100a1eb88:     	and	w9, w1, #0xff
100a1eb8c:     	lsr	w8, w9, w8
100a1eb90:     	and	w8, w8, #0x3
100a1eb94:     	cmp	w8, #0x1
100a1eb98:     	b.gt	0x100a1ebf4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xf0>
100a1eb9c:     	cbnz	w8, 0x100a1eb54 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x50>
100a1eba0:     	b	0x100a1ebd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xcc>
100a1eba4:     	cmp	w3, #0x2
100a1eba8:     	b.hs	0x100a1ebd8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xd4>
100a1ebac:     	and	w9, w1, #0xff
100a1ebb0:     	lsr	w8, w9, w3
100a1ebb4:     	and	w8, w8, #0x1
100a1ebb8:     	orr	w10, w3, #0x2
100a1ebbc:     	lsr	w9, w9, w10
100a1ebc0:     	bfi	w8, w9, #1, #1
100a1ebc4:     	cmp	w8, #0x1
100a1ebc8:     	b.gt	0x100a1ec28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x124>
100a1ebcc:     	cbnz	w8, 0x100a1ec20 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x11c>
100a1ebd0:     	mov	w0, #0x0                ; =0
100a1ebd4:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1ebd8:     	cmp	w2, w3
100a1ebdc:     	b.ne	0x100a1ec04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x100>
100a1ebe0:     	and	w8, w1, #0x1
100a1ebe4:     	ubfx	w9, w1, #3, #1
100a1ebe8:     	orr	w8, w8, w9, lsl #1
100a1ebec:     	cmp	w8, #0x1
100a1ebf0:     	b.le	0x100a1eb9c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x98>
100a1ebf4:     	cmp	w8, #0x2
100a1ebf8:     	b.ne	0x100a1eb74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x70>
100a1ebfc:     	mov	x0, x3
100a1ec00:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1ec04:     	eor	w9, w3, w2
100a1ec08:     	cmp	w9, #0x1
100a1ec0c:     	b.ne	0x100a1ec50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x14c>
100a1ec10:     	ubfx	w8, w1, #1, #2
100a1ec14:     	cmp	w8, #0x1
100a1ec18:     	b.gt	0x100a1ec28 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x124>
100a1ec1c:     	cbz	w8, 0x100a1ebd0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xcc>
100a1ec20:     	eor	w0, w2, #0x1
100a1ec24:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1ec28:     	cmp	w8, #0x2
100a1ec2c:     	b.ne	0x100a1eb74 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x70>
100a1ec30:     	mov	x0, x2
100a1ec34:     	ldp	x29, x30, [sp, #0xd0]
100a1ec38:     	ldp	x20, x19, [sp, #0xc0]
100a1ec3c:     	ldp	x22, x21, [sp, #0xb0]
100a1ec40:     	ldp	x24, x23, [sp, #0xa0]
100a1ec44:     	ldp	x26, x25, [sp, #0x90]
100a1ec48:     	add	sp, sp, #0xe0
100a1ec4c:     	ret
100a1ec50:     	mov	x20, x1
100a1ec54:     	sturb	w1, [x29, #-0x48]
100a1ec58:     	mov	x23, x2
100a1ec5c:     	stur	w2, [x29, #-0x4c]
100a1ec60:     	mov	x22, x3
100a1ec64:     	stur	w3, [x29, #-0x44]
100a1ec68:     	mov	x19, x8
100a1ec6c:     	add	x0, x8, #0x50
100a1ec70:     	sub	x1, x29, #0x4c
100a1ec74:     	bl	0x10056d874 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100a1ec78:     	cbz	x0, 0x100a1ec84 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x180>
100a1ec7c:     	ldr	w0, [x0]
100a1ec80:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1ec84:     	ldr	x1, [x19, #0x28]
100a1ec88:     	mov	x8, x23
100a1ec8c:     	lsr	w0, w8, #1
100a1ec90:     	cmp	x1, x0
100a1ec94:     	b.ls	0x100a1ee9c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x398>
100a1ec98:     	lsr	w8, w22, #1
100a1ec9c:     	cmp	x1, x8
100a1eca0:     	b.ls	0x100a1eea8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3a4>
100a1eca4:     	mov	x24, x19
100a1eca8:     	ldr	x9, [x19, #0x20]
100a1ecac:     	add	x10, x9, x0, lsl #5
100a1ecb0:     	ldr	x10, [x10, #0x18]
100a1ecb4:     	add	x8, x9, x8, lsl #5
100a1ecb8:     	ldr	x8, [x8, #0x18]
100a1ecbc:     	orr	x21, x8, x10
100a1ecc0:     	fmov	d0, x21
100a1ecc4:     	cnt.8b	v0, v0
100a1ecc8:     	addv.8b	b0, v0
100a1eccc:     	fmov	x8, d0
100a1ecd0:     	cmp	x8, #0xa
100a1ecd4:     	b.hs	0x100a1ed60 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x25c>
100a1ecd8:     	add	x24, sp, #0x10
100a1ecdc:     	add	x0, sp, #0x10
100a1ece0:     	mov	x1, x21
100a1ece4:     	bl	0x100b51868 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100a1ece8:     	mov	x8, x19
100a1ecec:     	ldrb	w9, [x19, #0x94]
100a1ecf0:     	tbz	w9, #0x0, 0x100a1ee18 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x314>
100a1ecf4:     	ldp	x1, x2, [x8, #0x20]
100a1ecf8:     	add	x0, sp, #0x40
100a1ecfc:     	mov	x3, x23
100a1ed00:     	mov	x4, x21
100a1ed04:     	bl	0x100a1953c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1ed08:     	ldp	x23, x24, [sp, #0x48]
100a1ed0c:     	ldp	x1, x2, [x19, #0x20]
100a1ed10:     	add	x0, sp, #0x58
100a1ed14:     	mov	x3, x22
100a1ed18:     	mov	x4, x21
100a1ed1c:     	bl	0x100a1953c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a1ed20:     	ldp	x22, x5, [sp, #0x60]
100a1ed24:     	add	x0, sp, #0x28
100a1ed28:     	mov	x1, x20
100a1ed2c:     	mov	x2, x23
100a1ed30:     	mov	x3, x24
100a1ed34:     	mov	x4, x22
100a1ed38:     	bl	0x100ba1a34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100a1ed3c:     	ldr	x8, [sp, #0x58]
100a1ed40:     	cbz	x8, 0x100a1ed4c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x248>
100a1ed44:     	mov	x0, x22
100a1ed48:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1ed4c:     	ldr	x8, [sp, #0x40]
100a1ed50:     	cbz	x8, 0x100a1ee48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x344>
100a1ed54:     	mov	x0, x23
100a1ed58:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1ed5c:     	b	0x100a1ee48 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x344>
100a1ed60:     	ldp	x0, x1, [x24, #0x8]
100a1ed64:     	mov	x2, x21
100a1ed68:     	bl	0x100a17d00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a1ed6c:     	mov	x21, x0
100a1ed70:     	mov	x0, x24
100a1ed74:     	mov	x1, x23
100a1ed78:     	mov	x2, x21
100a1ed7c:     	mov	w3, #0x0                ; =0
100a1ed80:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a1ed84:     	mov	x25, x0
100a1ed88:     	mov	x0, x24
100a1ed8c:     	mov	x1, x23
100a1ed90:     	mov	x2, x21
100a1ed94:     	mov	w3, #0x1                ; =1
100a1ed98:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a1ed9c:     	mov	x23, x0
100a1eda0:     	mov	x0, x24
100a1eda4:     	mov	x1, x22
100a1eda8:     	mov	x2, x21
100a1edac:     	mov	w3, #0x0                ; =0
100a1edb0:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a1edb4:     	mov	x26, x0
100a1edb8:     	mov	x0, x24
100a1edbc:     	mov	x1, x22
100a1edc0:     	mov	x2, x21
100a1edc4:     	mov	w3, #0x1                ; =1
100a1edc8:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a1edcc:     	mov	x22, x0
100a1edd0:     	mov	x0, x24
100a1edd4:     	mov	x1, x20
100a1edd8:     	mov	x2, x25
100a1eddc:     	mov	x3, x26
100a1ede0:     	bl	0x100a1eb04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100a1ede4:     	mov	x25, x0
100a1ede8:     	mov	x0, x24
100a1edec:     	mov	x1, x20
100a1edf0:     	mov	x2, x23
100a1edf4:     	mov	x3, x22
100a1edf8:     	bl	0x100a1eb04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100a1edfc:     	mov	x3, x0
100a1ee00:     	mov	x0, x24
100a1ee04:     	mov	x1, x21
100a1ee08:     	mov	x2, x25
100a1ee0c:     	bl	0x100a1f5ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100a1ee10:     	mov	x20, x0
100a1ee14:     	b	0x100a1ee6c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x368>
100a1ee18:     	ldr	x9, [sp, #0x20]
100a1ee1c:     	mov	w10, #0x1               ; =1
100a1ee20:     	lsl	x1, x10, x9
100a1ee24:     	stp	x24, x8, [sp, #0x58]
100a1ee28:     	add	x8, sp, #0x8
100a1ee2c:     	add	x9, sp, #0xc
100a1ee30:     	stp	x8, x9, [sp, #0x68]
100a1ee34:     	add	x8, sp, #0x7
100a1ee38:     	str	x8, [sp, #0x78]
100a1ee3c:     	add	x0, sp, #0x28
100a1ee40:     	add	x2, sp, #0x58
100a1ee44:     	bl	0x1006ada80 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E5applys_0EB6_>
100a1ee48:     	add	x2, sp, #0x28
100a1ee4c:     	mov	x0, x19
100a1ee50:     	mov	x1, x21
100a1ee54:     	bl	0x100a1ef04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a1ee58:     	mov	x20, x0
100a1ee5c:     	ldr	x8, [sp, #0x10]
100a1ee60:     	cbz	x8, 0x100a1ee6c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x368>
100a1ee64:     	ldr	x0, [sp, #0x18]
100a1ee68:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1ee6c:     	add	x0, x19, #0x50
100a1ee70:     	sub	x1, x29, #0x4c
100a1ee74:     	mov	x2, x20
100a1ee78:     	bl	0x100aad7b0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100a1ee7c:     	mov	x0, x20
100a1ee80:     	b	0x100a1ec34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x130>
100a1ee84:     	adrp	x0, 0x1011bd000 <dyld_stub_binder+0x1011bd000>
100a1ee88:     	add	x0, x0, #0xe0c
100a1ee8c:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a1ee90:     	add	x2, x2, #0xd20
100a1ee94:     	mov	w1, #0x19               ; =25
100a1ee98:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a1ee9c:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a1eea0:     	add	x2, x2, #0xb78
100a1eea4:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1eea8:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a1eeac:     	add	x2, x2, #0xb78
100a1eeb0:     	mov	x0, x8
100a1eeb4:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a1eeb8:     	mov	x19, x0
100a1eebc:     	ldr	x8, [sp, #0x58]
100a1eec0:     	cbz	x8, 0x100a1eed4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3d0>
100a1eec4:     	mov	x0, x22
100a1eec8:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1eecc:     	b	0x100a1eed4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3d0>
100a1eed0:     	mov	x19, x0
100a1eed4:     	ldr	x8, [sp, #0x40]
100a1eed8:     	cbz	x8, 0x100a1eeec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3e8>
100a1eedc:     	mov	x0, x23
100a1eee0:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1eee4:     	b	0x100a1eeec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3e8>
100a1eee8:     	mov	x19, x0
100a1eeec:     	ldr	x8, [sp, #0x10]
100a1eef0:     	cbz	x8, 0x100a1eefc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3f8>
100a1eef4:     	ldr	x0, [sp, #0x18]
100a1eef8:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a1eefc:     	mov	x0, x19
100a1ef00:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
