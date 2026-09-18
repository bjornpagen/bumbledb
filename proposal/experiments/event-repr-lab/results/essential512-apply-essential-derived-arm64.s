
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a20d40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>:
100a20d40:     	sub	sp, sp, #0xf0
100a20d44:     	stp	x28, x27, [sp, #0x90]
100a20d48:     	stp	x26, x25, [sp, #0xa0]
100a20d4c:     	stp	x24, x23, [sp, #0xb0]
100a20d50:     	stp	x22, x21, [sp, #0xc0]
100a20d54:     	stp	x20, x19, [sp, #0xd0]
100a20d58:     	stp	x29, x30, [sp, #0xe0]
100a20d5c:     	add	x29, sp, #0xe0
100a20d60:     	and	w8, w1, #0xff
100a20d64:     	cmp	w8, #0x10
100a20d68:     	b.hs	0x100a2115c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x41c>
100a20d6c:     	mov	x20, x3
100a20d70:     	mov	x19, x0
100a20d74:     	ldrb	w8, [x0, #0xad]
100a20d78:     	tbz	w8, #0x0, 0x100a20e24 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xe4>
100a20d7c:     	and	w8, w1, #0xfc
100a20d80:     	ubfiz	w9, w1, #2, #2
100a20d84:     	orr	w8, w9, w8, lsr #2
100a20d88:     	and	w9, w2, #0xfffffffe
100a20d8c:     	tst	w2, #0x1
100a20d90:     	csel	w9, w2, w9, eq
100a20d94:     	csel	w8, w1, w8, eq
100a20d98:     	mov	w10, #0xa               ; =10
100a20d9c:     	and	w10, w10, w8, lsl #1
100a20da0:     	mov	w11, #0x5               ; =5
100a20da4:     	and	w11, w11, w8, lsr #1
100a20da8:     	orr	w10, w10, w11
100a20dac:     	and	w11, w20, #0xfffffffe
100a20db0:     	tst	w20, #0x1
100a20db4:     	csel	w11, w20, w11, eq
100a20db8:     	csel	w8, w8, w10, eq
100a20dbc:     	ands	w27, w8, #0x1
100a20dc0:     	eor	w10, w8, #0xf
100a20dc4:     	csel	w8, w8, w10, eq
100a20dc8:     	mov	w10, #0x9               ; =9
100a20dcc:     	and	w10, w8, w10
100a20dd0:     	lsr	w12, w8, #1
100a20dd4:     	bfi	w10, w12, #2, #1
100a20dd8:     	and	w12, w12, #0x2
100a20ddc:     	orr	w10, w10, w12
100a20de0:     	cmp	w9, w11
100a20de4:     	csel	w20, w11, w9, ls
100a20de8:     	csel	w2, w9, w11, ls
100a20dec:     	csel	w1, w8, w10, ls
100a20df0:     	strb	w1, [sp, #0x7]
100a20df4:     	stp	w2, w20, [sp, #0x8]
100a20df8:     	and	w21, w1, #0xff
100a20dfc:     	cmp	w21, #0x9
100a20e00:     	b.le	0x100a20e3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xfc>
100a20e04:     	cmp	w21, #0xa
100a20e08:     	b.eq	0x100a20ee0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1a0>
100a20e0c:     	cmp	w21, #0xc
100a20e10:     	b.eq	0x100a20f08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c8>
100a20e14:     	cmp	w21, #0xf
100a20e18:     	b.ne	0x100a20e58 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x118>
100a20e1c:     	mov	w21, #0x1               ; =1
100a20e20:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20e24:     	mov	w27, #0x0               ; =0
100a20e28:     	strb	w1, [sp, #0x7]
100a20e2c:     	stp	w2, w20, [sp, #0x8]
100a20e30:     	and	w21, w1, #0xff
100a20e34:     	cmp	w21, #0x9
100a20e38:     	b.gt	0x100a20e04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xc4>
100a20e3c:     	cbz	w21, 0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20e40:     	cmp	w21, #0x3
100a20e44:     	b.eq	0x100a20eb4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x174>
100a20e48:     	cmp	w21, #0x5
100a20e4c:     	b.ne	0x100a20e58 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x118>
100a20e50:     	eor	w21, w20, #0x1
100a20e54:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20e58:     	cmp	w2, #0x2
100a20e5c:     	b.hs	0x100a20e80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x140>
100a20e60:     	ubfiz	x8, x2, #1, #7
100a20e64:     	and	w9, w1, #0xff
100a20e68:     	lsr	w8, w9, w8
100a20e6c:     	and	w8, w8, #0x3
100a20e70:     	cmp	w8, #0x1
100a20e74:     	b.gt	0x100a20ed8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x198>
100a20e78:     	cbnz	w8, 0x100a20e50 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x110>
100a20e7c:     	b	0x100a20eac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x16c>
100a20e80:     	cmp	w20, #0x2
100a20e84:     	b.hs	0x100a20ebc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x17c>
100a20e88:     	and	w9, w1, #0xff
100a20e8c:     	lsr	w8, w9, w20
100a20e90:     	and	w8, w8, #0x1
100a20e94:     	orr	w10, w20, #0x2
100a20e98:     	lsr	w9, w9, w10
100a20e9c:     	bfi	w8, w9, #1, #1
100a20ea0:     	cmp	w8, #0x1
100a20ea4:     	b.gt	0x100a20f00 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1c0>
100a20ea8:     	cbnz	w8, 0x100a20eb4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x174>
100a20eac:     	mov	w21, #0x0               ; =0
100a20eb0:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20eb4:     	eor	w21, w2, #0x1
100a20eb8:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20ebc:     	cmp	w2, w20
100a20ec0:     	b.ne	0x100a20ee8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1a8>
100a20ec4:     	and	w8, w1, #0x1
100a20ec8:     	ubfx	w9, w1, #3, #1
100a20ecc:     	orr	w8, w8, w9, lsl #1
100a20ed0:     	cmp	w8, #0x1
100a20ed4:     	b.le	0x100a20e78 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x138>
100a20ed8:     	cmp	w8, #0x2
100a20edc:     	b.ne	0x100a20e1c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xdc>
100a20ee0:     	mov	x21, x20
100a20ee4:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20ee8:     	eor	w8, w2, w20
100a20eec:     	cmp	w8, #0x1
100a20ef0:     	b.ne	0x100a20f34 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1f4>
100a20ef4:     	ubfx	w8, w1, #1, #2
100a20ef8:     	cmp	w8, #0x1
100a20efc:     	b.le	0x100a20ea8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x168>
100a20f00:     	cmp	w8, #0x2
100a20f04:     	b.ne	0x100a20e1c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0xdc>
100a20f08:     	mov	x21, x2
100a20f0c:     	and	w8, w27, #0xff
100a20f10:     	eor	w0, w21, w8
100a20f14:     	ldp	x29, x30, [sp, #0xe0]
100a20f18:     	ldp	x20, x19, [sp, #0xd0]
100a20f1c:     	ldp	x22, x21, [sp, #0xc0]
100a20f20:     	ldp	x24, x23, [sp, #0xb0]
100a20f24:     	ldp	x26, x25, [sp, #0xa0]
100a20f28:     	ldp	x28, x27, [sp, #0x90]
100a20f2c:     	add	sp, sp, #0xf0
100a20f30:     	ret
100a20f34:     	mov	x21, x1
100a20f38:     	sturb	w1, [x29, #-0x58]
100a20f3c:     	mov	x23, x2
100a20f40:     	stur	w2, [x29, #-0x5c]
100a20f44:     	stur	w20, [x29, #-0x54]
100a20f48:     	add	x0, x19, #0x68
100a20f4c:     	sub	x1, x29, #0x5c
100a20f50:     	bl	0x10056d874 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_ECs23EhFSy3h49_8bumbledb>
100a20f54:     	cbz	x0, 0x100a20f60 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x220>
100a20f58:     	ldr	w21, [x0]
100a20f5c:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a20f60:     	ldr	x24, [x19, #0x28]
100a20f64:     	mov	x8, x23
100a20f68:     	lsr	w0, w8, #1
100a20f6c:     	cmp	x24, x0
100a20f70:     	b.ls	0x100a21174 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x434>
100a20f74:     	lsr	w8, w20, #1
100a20f78:     	cmp	x24, x8
100a20f7c:     	b.ls	0x100a21184 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x444>
100a20f80:     	ldr	x25, [x19, #0x20]
100a20f84:     	add	x9, x25, x0, lsl #5
100a20f88:     	ldr	x9, [x9, #0x18]
100a20f8c:     	add	x8, x25, x8, lsl #5
100a20f90:     	ldr	x8, [x8, #0x18]
100a20f94:     	orr	x22, x8, x9
100a20f98:     	fmov	d0, x22
100a20f9c:     	cnt.8b	v0, v0
100a20fa0:     	addv.8b	b0, v0
100a20fa4:     	fmov	x8, d0
100a20fa8:     	cmp	x8, #0xa
100a20fac:     	b.hs	0x100a2103c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x2fc>
100a20fb0:     	add	x26, sp, #0x10
100a20fb4:     	add	x0, sp, #0x10
100a20fb8:     	mov	x1, x22
100a20fbc:     	bl	0x100b53628 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw4axes>
100a20fc0:     	ldrb	w8, [x19, #0xac]
100a20fc4:     	tbz	w8, #0x0, 0x100a210f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3b4>
100a20fc8:     	add	x0, sp, #0x40
100a20fcc:     	mov	x1, x25
100a20fd0:     	mov	x2, x24
100a20fd4:     	mov	x3, x23
100a20fd8:     	mov	x4, x22
100a20fdc:     	bl	0x100a1aab8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a20fe0:     	ldp	x23, x26, [sp, #0x48]
100a20fe4:     	add	x0, sp, #0x58
100a20fe8:     	mov	x1, x25
100a20fec:     	mov	x2, x24
100a20ff0:     	mov	x3, x20
100a20ff4:     	mov	x4, x22
100a20ff8:     	bl	0x100a1aab8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E7alignedB6_>
100a20ffc:     	ldp	x20, x5, [sp, #0x60]
100a21000:     	add	x0, sp, #0x28
100a21004:     	mov	x1, x21
100a21008:     	mov	x2, x23
100a2100c:     	mov	x3, x26
100a21010:     	mov	x4, x20
100a21014:     	bl	0x100ba3d34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>
100a21018:     	ldr	x8, [sp, #0x58]
100a2101c:     	cbz	x8, 0x100a21028 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x2e8>
100a21020:     	mov	x0, x20
100a21024:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21028:     	ldr	x8, [sp, #0x40]
100a2102c:     	cbz	x8, 0x100a21124 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3e4>
100a21030:     	mov	x0, x23
100a21034:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21038:     	b	0x100a21124 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x3e4>
100a2103c:     	mov	x0, x19
100a21040:     	mov	x1, x22
100a21044:     	bl	0x100a18fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm2_E3topB6_>
100a21048:     	mov	x22, x0
100a2104c:     	mov	x0, x19
100a21050:     	mov	x1, x23
100a21054:     	mov	x2, x22
100a21058:     	mov	w3, #0x0                ; =0
100a2105c:     	bl	0x100a22314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a21060:     	mov	x24, x0
100a21064:     	mov	x0, x19
100a21068:     	mov	x1, x23
100a2106c:     	mov	x2, x22
100a21070:     	mov	w3, #0x1                ; =1
100a21074:     	bl	0x100a22314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a21078:     	mov	x23, x0
100a2107c:     	mov	x0, x19
100a21080:     	mov	x1, x20
100a21084:     	mov	x2, x22
100a21088:     	mov	w3, #0x0                ; =0
100a2108c:     	bl	0x100a22314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a21090:     	mov	x25, x0
100a21094:     	mov	x0, x19
100a21098:     	mov	x1, x20
100a2109c:     	mov	x2, x22
100a210a0:     	mov	w3, #0x1                ; =1
100a210a4:     	bl	0x100a22314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a210a8:     	mov	x20, x0
100a210ac:     	mov	x0, x19
100a210b0:     	mov	x1, x21
100a210b4:     	mov	x2, x24
100a210b8:     	mov	x3, x25
100a210bc:     	bl	0x100a20d40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100a210c0:     	mov	x24, x0
100a210c4:     	mov	x0, x19
100a210c8:     	mov	x1, x21
100a210cc:     	mov	x2, x23
100a210d0:     	mov	x3, x20
100a210d4:     	bl	0x100a20d40 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
100a210d8:     	mov	x3, x0
100a210dc:     	mov	x0, x19
100a210e0:     	mov	x1, x22
100a210e4:     	mov	x2, x24
100a210e8:     	bl	0x100a217cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100a210ec:     	mov	x21, x0
100a210f0:     	b	0x100a21148 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x408>
100a210f4:     	ldr	x8, [sp, #0x20]
100a210f8:     	mov	w9, #0x1                ; =1
100a210fc:     	lsl	x1, x9, x8
100a21100:     	stp	x26, x19, [sp, #0x58]
100a21104:     	add	x8, sp, #0x8
100a21108:     	add	x9, sp, #0xc
100a2110c:     	stp	x8, x9, [sp, #0x68]
100a21110:     	add	x8, sp, #0x7
100a21114:     	str	x8, [sp, #0x78]
100a21118:     	add	x0, sp, #0x28
100a2111c:     	add	x2, sp, #0x58
100a21120:     	bl	0x1006adc40 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw5wordsNCNvMB2_INtB2_5ArenaKm2_E11apply_inners_0EB6_>
100a21124:     	add	x2, sp, #0x28
100a21128:     	mov	x0, x19
100a2112c:     	mov	x1, x22
100a21130:     	bl	0x100a211e4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a21134:     	mov	x21, x0
100a21138:     	ldr	x8, [sp, #0x10]
100a2113c:     	cbz	x8, 0x100a21148 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x408>
100a21140:     	ldr	x0, [sp, #0x18]
100a21144:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a21148:     	add	x0, x19, #0x68
100a2114c:     	sub	x1, x29, #0x5c
100a21150:     	mov	x2, x21
100a21154:     	bl	0x100aaf670 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100a21158:     	b	0x100a20f0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x1cc>
100a2115c:     	adrp	x0, 0x1011c1000 <dyld_stub_binder+0x1011c1000>
100a21160:     	add	x0, x0, #0x5e2
100a21164:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a21168:     	add	x2, x2, #0xe28
100a2116c:     	mov	w1, #0x19               ; =25
100a21170:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a21174:     	adrp	x2, 0x101339000 <dyld_stub_binder+0x101339000>
100a21178:     	add	x2, x2, #0xba8
100a2117c:     	mov	x1, x24
100a21180:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a21184:     	adrp	x2, 0x101339000 <dyld_stub_binder+0x101339000>
100a21188:     	add	x2, x2, #0xba8
100a2118c:     	mov	x0, x8
100a21190:     	mov	x1, x24
100a21194:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a21198:     	mov	x19, x0
100a2119c:     	ldr	x8, [sp, #0x58]
100a211a0:     	cbz	x8, 0x100a211b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x474>
100a211a4:     	mov	x0, x20
100a211a8:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a211ac:     	b	0x100a211b4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x474>
100a211b0:     	mov	x19, x0
100a211b4:     	ldr	x8, [sp, #0x40]
100a211b8:     	cbz	x8, 0x100a211cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x48c>
100a211bc:     	mov	x0, x23
100a211c0:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a211c4:     	b	0x100a211cc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x48c>
100a211c8:     	mov	x19, x0
100a211cc:     	ldr	x8, [sp, #0x10]
100a211d0:     	cbz	x8, 0x100a211dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_+0x49c>
100a211d4:     	ldr	x0, [sp, #0x18]
100a211d8:     	bl	0x10110d138 <dyld_stub_binder+0x10110d138>
100a211dc:     	mov	x0, x19
100a211e0:     	bl	0x10110cf88 <dyld_stub_binder+0x10110cf88>
