
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b98c00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>:
100b98c00:     	sub	sp, sp, #0xf0
100b98c04:     	stp	x28, x27, [sp, #0x90]
100b98c08:     	stp	x26, x25, [sp, #0xa0]
100b98c0c:     	stp	x24, x23, [sp, #0xb0]
100b98c10:     	stp	x22, x21, [sp, #0xc0]
100b98c14:     	stp	x20, x19, [sp, #0xd0]
100b98c18:     	stp	x29, x30, [sp, #0xe0]
100b98c1c:     	add	x29, sp, #0xe0
100b98c20:     	and	w8, w1, #0xff
100b98c24:     	cmp	w8, #0x10
100b98c28:     	b.hs	0x100b99000 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x400>
100b98c2c:     	mov	x19, x3
100b98c30:     	ldrb	w8, [x0, #0xe5]
100b98c34:     	tbz	w8, #0x0, 0x100b98ce0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xe0>
100b98c38:     	and	w8, w1, #0xfc
100b98c3c:     	ubfiz	w9, w1, #2, #2
100b98c40:     	orr	w8, w9, w8, lsr #2
100b98c44:     	and	w9, w2, #0xfffffffe
100b98c48:     	tst	w2, #0x1
100b98c4c:     	csel	w9, w2, w9, eq
100b98c50:     	csel	w8, w1, w8, eq
100b98c54:     	mov	w10, #0xa               ; =10
100b98c58:     	and	w10, w10, w8, lsl #1
100b98c5c:     	mov	w11, #0x5               ; =5
100b98c60:     	and	w11, w11, w8, lsr #1
100b98c64:     	orr	w10, w10, w11
100b98c68:     	and	w11, w19, #0xfffffffe
100b98c6c:     	tst	w19, #0x1
100b98c70:     	csel	w11, w19, w11, eq
100b98c74:     	csel	w8, w8, w10, eq
100b98c78:     	ands	w27, w8, #0x1
100b98c7c:     	eor	w10, w8, #0xf
100b98c80:     	csel	w8, w8, w10, eq
100b98c84:     	mov	w10, #0x9               ; =9
100b98c88:     	and	w10, w8, w10
100b98c8c:     	lsr	w12, w8, #1
100b98c90:     	bfi	w10, w12, #2, #1
100b98c94:     	and	w12, w12, #0x2
100b98c98:     	orr	w10, w10, w12
100b98c9c:     	cmp	w9, w11
100b98ca0:     	csel	w19, w11, w9, ls
100b98ca4:     	csel	w2, w9, w11, ls
100b98ca8:     	csel	w1, w8, w10, ls
100b98cac:     	strb	w1, [sp, #0x7]
100b98cb0:     	stp	w2, w19, [sp, #0x8]
100b98cb4:     	and	w21, w1, #0xff
100b98cb8:     	cmp	w21, #0x9
100b98cbc:     	b.le	0x100b98cf8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xf8>
100b98cc0:     	cmp	w21, #0xa
100b98cc4:     	b.eq	0x100b98d9c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x19c>
100b98cc8:     	cmp	w21, #0xc
100b98ccc:     	b.eq	0x100b98dc4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c4>
100b98cd0:     	cmp	w21, #0xf
100b98cd4:     	b.ne	0x100b98d14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x114>
100b98cd8:     	mov	w21, #0x1               ; =1
100b98cdc:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98ce0:     	mov	w27, #0x0               ; =0
100b98ce4:     	strb	w1, [sp, #0x7]
100b98ce8:     	stp	w2, w19, [sp, #0x8]
100b98cec:     	and	w21, w1, #0xff
100b98cf0:     	cmp	w21, #0x9
100b98cf4:     	b.gt	0x100b98cc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xc0>
100b98cf8:     	cbz	w21, 0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98cfc:     	cmp	w21, #0x3
100b98d00:     	b.eq	0x100b98d70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x170>
100b98d04:     	cmp	w21, #0x5
100b98d08:     	b.ne	0x100b98d14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x114>
100b98d0c:     	eor	w21, w19, #0x1
100b98d10:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98d14:     	cmp	w2, #0x2
100b98d18:     	b.hs	0x100b98d3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x13c>
100b98d1c:     	ubfiz	x8, x2, #1, #7
100b98d20:     	and	w9, w1, #0xff
100b98d24:     	lsr	w8, w9, w8
100b98d28:     	and	w8, w8, #0x3
100b98d2c:     	cmp	w8, #0x1
100b98d30:     	b.gt	0x100b98d94 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x194>
100b98d34:     	cbnz	w8, 0x100b98d0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x10c>
100b98d38:     	b	0x100b98d68 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x168>
100b98d3c:     	cmp	w19, #0x2
100b98d40:     	b.hs	0x100b98d78 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x178>
100b98d44:     	and	w9, w1, #0xff
100b98d48:     	lsr	w8, w9, w19
100b98d4c:     	and	w8, w8, #0x1
100b98d50:     	orr	w10, w19, #0x2
100b98d54:     	lsr	w9, w9, w10
100b98d58:     	bfi	w8, w9, #1, #1
100b98d5c:     	cmp	w8, #0x1
100b98d60:     	b.gt	0x100b98dbc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1bc>
100b98d64:     	cbnz	w8, 0x100b98d70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x170>
100b98d68:     	mov	w21, #0x0               ; =0
100b98d6c:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98d70:     	eor	w21, w2, #0x1
100b98d74:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98d78:     	cmp	w2, w19
100b98d7c:     	b.ne	0x100b98da4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1a4>
100b98d80:     	and	w8, w1, #0x1
100b98d84:     	ubfx	w9, w1, #3, #1
100b98d88:     	orr	w8, w8, w9, lsl #1
100b98d8c:     	cmp	w8, #0x1
100b98d90:     	b.le	0x100b98d34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x134>
100b98d94:     	cmp	w8, #0x2
100b98d98:     	b.ne	0x100b98cd8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xd8>
100b98d9c:     	mov	x21, x19
100b98da0:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98da4:     	eor	w8, w2, w19
100b98da8:     	cmp	w8, #0x1
100b98dac:     	b.ne	0x100b98df0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1f0>
100b98db0:     	ubfx	w8, w1, #1, #2
100b98db4:     	cmp	w8, #0x1
100b98db8:     	b.le	0x100b98d64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x164>
100b98dbc:     	cmp	w8, #0x2
100b98dc0:     	b.ne	0x100b98cd8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xd8>
100b98dc4:     	mov	x21, x2
100b98dc8:     	and	w8, w27, #0xff
100b98dcc:     	eor	w0, w21, w8
100b98dd0:     	ldp	x29, x30, [sp, #0xe0]
100b98dd4:     	ldp	x20, x19, [sp, #0xd0]
100b98dd8:     	ldp	x22, x21, [sp, #0xc0]
100b98ddc:     	ldp	x24, x23, [sp, #0xb0]
100b98de0:     	ldp	x26, x25, [sp, #0xa0]
100b98de4:     	ldp	x28, x27, [sp, #0x90]
100b98de8:     	add	sp, sp, #0xf0
100b98dec:     	ret
100b98df0:     	mov	x22, x1
100b98df4:     	sturb	w1, [x29, #-0x58]
100b98df8:     	mov	x23, x2
100b98dfc:     	stur	w2, [x29, #-0x5c]
100b98e00:     	stur	w19, [x29, #-0x54]
100b98e04:     	mov	x20, x0
100b98e08:     	add	x0, x0, #0xa0
100b98e0c:     	sub	x1, x29, #0x5c
100b98e10:     	bl	0x10065e8f0 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100b98e14:     	cbz	x0, 0x100b98e20 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x220>
100b98e18:     	ldr	w21, [x0]
100b98e1c:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b98e20:     	mov	x0, x20
100b98e24:     	mov	x1, x23
100b98e28:     	bl	0x100b8f5c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E9variablesB6_>
100b98e2c:     	mov	x24, x0
100b98e30:     	mov	x0, x20
100b98e34:     	mov	x1, x19
100b98e38:     	bl	0x100b8f5c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E9variablesB6_>
100b98e3c:     	orr	x21, x0, x24
100b98e40:     	fmov	d0, x21
100b98e44:     	cnt.8b	v0, v0
100b98e48:     	addv.8b	b0, v0
100b98e4c:     	fmov	x8, d0
100b98e50:     	cmp	x8, #0xa
100b98e54:     	b.hs	0x100b98ee0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x2e0>
100b98e58:     	add	x24, sp, #0x10
100b98e5c:     	add	x0, sp, #0x10
100b98e60:     	mov	x1, x21
100b98e64:     	bl	0x100cc56e8 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100b98e68:     	mov	x8, x20
100b98e6c:     	ldrb	w9, [x20, #0xe4]
100b98e70:     	tbz	w9, #0x0, 0x100b98f98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x398>
100b98e74:     	mov	x1, x8
100b98e78:     	add	x0, sp, #0x40
100b98e7c:     	mov	x2, x23
100b98e80:     	mov	x3, x21
100b98e84:     	bl	0x100b8eda0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100b98e88:     	ldp	x23, x25, [sp, #0x48]
100b98e8c:     	add	x0, sp, #0x58
100b98e90:     	mov	x1, x20
100b98e94:     	mov	x2, x19
100b98e98:     	mov	x3, x21
100b98e9c:     	bl	0x100b8eda0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100b98ea0:     	ldp	x24, x5, [sp, #0x60]
100b98ea4:     	add	x0, sp, #0x28
100b98ea8:     	mov	x1, x22
100b98eac:     	mov	x2, x23
100b98eb0:     	mov	x3, x25
100b98eb4:     	mov	x4, x24
100b98eb8:     	bl	0x100d15334 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100b98ebc:     	ldr	x8, [sp, #0x58]
100b98ec0:     	cbz	x8, 0x100b98ecc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x2cc>
100b98ec4:     	mov	x0, x24
100b98ec8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b98ecc:     	ldr	x8, [sp, #0x40]
100b98ed0:     	cbz	x8, 0x100b98fc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3c8>
100b98ed4:     	mov	x0, x23
100b98ed8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b98edc:     	b	0x100b98fc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3c8>
100b98ee0:     	mov	x0, x20
100b98ee4:     	mov	x1, x21
100b98ee8:     	bl	0x100b8d6c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100b98eec:     	mov	x21, x0
100b98ef0:     	mov	x0, x20
100b98ef4:     	mov	x1, x23
100b98ef8:     	mov	x2, x21
100b98efc:     	mov	w3, #0x0                ; =0
100b98f00:     	bl	0x100b9a1fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100b98f04:     	mov	x25, x0
100b98f08:     	mov	x0, x20
100b98f0c:     	mov	x1, x23
100b98f10:     	mov	x2, x21
100b98f14:     	mov	w3, #0x1                ; =1
100b98f18:     	bl	0x100b9a1fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100b98f1c:     	mov	x23, x0
100b98f20:     	mov	x0, x20
100b98f24:     	mov	x1, x19
100b98f28:     	mov	x2, x21
100b98f2c:     	mov	w3, #0x0                ; =0
100b98f30:     	bl	0x100b9a1fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100b98f34:     	mov	x26, x0
100b98f38:     	mov	x0, x20
100b98f3c:     	mov	x1, x19
100b98f40:     	mov	x2, x21
100b98f44:     	mov	w3, #0x1                ; =1
100b98f48:     	bl	0x100b9a1fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100b98f4c:     	mov	x19, x0
100b98f50:     	mov	x0, x20
100b98f54:     	mov	x1, x22
100b98f58:     	mov	x2, x25
100b98f5c:     	mov	x3, x26
100b98f60:     	bl	0x100b98c00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100b98f64:     	mov	x25, x0
100b98f68:     	mov	x0, x20
100b98f6c:     	mov	x1, x22
100b98f70:     	mov	x2, x23
100b98f74:     	mov	x3, x19
100b98f78:     	bl	0x100b98c00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100b98f7c:     	mov	x3, x0
100b98f80:     	mov	x0, x20
100b98f84:     	mov	x1, x21
100b98f88:     	mov	x2, x25
100b98f8c:     	bl	0x100b9964c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100b98f90:     	mov	x21, x0
100b98f94:     	b	0x100b98fec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3ec>
100b98f98:     	ldr	x9, [sp, #0x20]
100b98f9c:     	mov	w10, #0x1               ; =1
100b98fa0:     	lsl	x1, x10, x9
100b98fa4:     	stp	x24, x8, [sp, #0x58]
100b98fa8:     	add	x8, sp, #0x8
100b98fac:     	add	x9, sp, #0xc
100b98fb0:     	stp	x8, x9, [sp, #0x68]
100b98fb4:     	add	x8, sp, #0x7
100b98fb8:     	str	x8, [sp, #0x78]
100b98fbc:     	add	x0, sp, #0x28
100b98fc0:     	add	x2, sp, #0x58
100b98fc4:     	bl	0x1007aebc0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E11apply_inners_0EB6_>
100b98fc8:     	add	x2, sp, #0x28
100b98fcc:     	mov	x0, x20
100b98fd0:     	mov	x1, x21
100b98fd4:     	bl	0x100b99064 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100b98fd8:     	mov	x21, x0
100b98fdc:     	ldr	x8, [sp, #0x10]
100b98fe0:     	cbz	x8, 0x100b98fec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3ec>
100b98fe4:     	ldr	x0, [sp, #0x18]
100b98fe8:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b98fec:     	add	x0, x20, #0xa0
100b98ff0:     	sub	x1, x29, #0x5c
100b98ff4:     	mov	x2, x21
100b98ff8:     	bl	0x100c267ac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100b98ffc:     	b	0x100b98dc8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100b99000:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100b99004:     	add	x0, x0, #0x70a
100b99008:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b9900c:     	add	x2, x2, #0x8d0
100b99010:     	mov	w1, #0x19               ; =25
100b99014:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b99018:     	mov	x19, x0
100b9901c:     	ldr	x8, [sp, #0x58]
100b99020:     	cbz	x8, 0x100b99034 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x434>
100b99024:     	mov	x0, x24
100b99028:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b9902c:     	b	0x100b99034 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x434>
100b99030:     	mov	x19, x0
100b99034:     	ldr	x8, [sp, #0x40]
100b99038:     	cbz	x8, 0x100b9904c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x44c>
100b9903c:     	mov	x0, x23
100b99040:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b99044:     	b	0x100b9904c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x44c>
100b99048:     	mov	x19, x0
100b9904c:     	ldr	x8, [sp, #0x10]
100b99050:     	cbz	x8, 0x100b9905c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x45c>
100b99054:     	ldr	x0, [sp, #0x18]
100b99058:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100b9905c:     	mov	x0, x19
100b99060:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
