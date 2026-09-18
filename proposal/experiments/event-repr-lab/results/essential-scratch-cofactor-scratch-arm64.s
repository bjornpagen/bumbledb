
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d20d38 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into>:
100d20d38:     	stp	d15, d14, [sp, #-0xa0]!
100d20d3c:     	stp	d13, d12, [sp, #0x10]
100d20d40:     	stp	d11, d10, [sp, #0x20]
100d20d44:     	stp	d9, d8, [sp, #0x30]
100d20d48:     	stp	x28, x27, [sp, #0x40]
100d20d4c:     	stp	x26, x25, [sp, #0x50]
100d20d50:     	stp	x24, x23, [sp, #0x60]
100d20d54:     	stp	x22, x21, [sp, #0x70]
100d20d58:     	stp	x20, x19, [sp, #0x80]
100d20d5c:     	stp	x29, x30, [sp, #0x90]
100d20d60:     	add	x29, sp, #0x90
100d20d64:     	sub	sp, sp, #0x280
100d20d68:     	cmp	w3, w2
100d20d6c:     	b.hs	0x100d2178c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xa54>
100d20d70:     	mov	x24, x1
100d20d74:     	stur	x1, [x29, #-0xb0]
100d20d78:     	and	w8, w2, #0x3f
100d20d7c:     	mov	w9, #0x1                ; =1
100d20d80:     	lsl	x9, x9, x2
100d20d84:     	lsr	x9, x9, #6
100d20d88:     	cmp	w8, #0x6
100d20d8c:     	cinc	x8, x9, lo
100d20d90:     	stur	x8, [x29, #-0xa0]
100d20d94:     	cmp	x1, x8
100d20d98:     	b.ne	0x100d217a4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xa6c>
100d20d9c:     	mov	x20, x6
100d20da0:     	sub	w8, w2, #0x1
100d20da4:     	and	w26, w8, #0x3f
100d20da8:     	mov	w9, #0x1                ; =1
100d20dac:     	lsl	x25, x9, x8
100d20db0:     	lsr	x8, x25, #6
100d20db4:     	cmp	w26, #0x6
100d20db8:     	cinc	x8, x8, lo
100d20dbc:     	stp	x6, x8, [x29, #-0xa8]
100d20dc0:     	cmp	x6, x8
100d20dc4:     	b.ne	0x100d217c0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xa88>
100d20dc8:     	mov	x19, x5
100d20dcc:     	mov	x23, x4
100d20dd0:     	mov	x22, x3
100d20dd4:     	mov	x21, x0
100d20dd8:     	lsl	x1, x20, #3
100d20ddc:     	mov	x0, x5
100d20de0:     	bl	0x101290718 <dyld_stub_binder+0x101290718>
100d20de4:     	cmp	w22, #0x5
100d20de8:     	b.ls	0x100d20e9c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x164>
100d20dec:     	add	w8, w22, #0x3a
100d20df0:     	and	w27, w8, #0x3f
100d20df4:     	cmp	w27, #0x3f
100d20df8:     	b.eq	0x100d217e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xab0>
100d20dfc:     	mov	w9, #0x1                ; =1
100d20e00:     	lsl	x22, x9, x8
100d20e04:     	mov	w9, #0x2                ; =2
100d20e08:     	lsl	x2, x9, x8
100d20e0c:     	neg	x8, x2
100d20e10:     	and	x8, x24, x8
100d20e14:     	lsr	x9, x20, x27
100d20e18:     	add	x10, x22, #0x7f
100d20e1c:     	tst	x10, x20
100d20e20:     	cinc	x9, x9, ne
100d20e24:     	cmp	x20, #0x0
100d20e28:     	csel	x9, xzr, x9, eq
100d20e2c:     	add	x10, x27, #0x1
100d20e30:     	lsr	x8, x8, x10
100d20e34:     	cmp	x8, x9
100d20e38:     	csel	x24, x8, x9, lo
100d20e3c:     	cbz	x24, 0x100d2172c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x9f4>
100d20e40:     	mov	w8, w23
100d20e44:     	lsl	x0, x8, x27
100d20e48:     	adds	x1, x0, x22
100d20e4c:     	b.hs	0x100d217dc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xaa4>
100d20e50:     	cmp	x1, x2
100d20e54:     	b.hi	0x100d217dc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xaa4>
100d20e58:     	mov	x23, #0x0               ; =0
100d20e5c:     	lsl	x28, x2, #3
100d20e60:     	add	x21, x21, x0, lsl #3
100d20e64:     	lsl	x8, x23, x27
100d20e68:     	sub	x9, x20, x8
100d20e6c:     	cmp	x22, x9
100d20e70:     	csel	x0, x22, x9, lo
100d20e74:     	b.hi	0x100d2177c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xa44>
100d20e78:     	add	x23, x23, #0x1
100d20e7c:     	lsl	x2, x0, #3
100d20e80:     	add	x0, x19, x8, lsl #3
100d20e84:     	mov	x1, x21
100d20e88:     	bl	0x10129091c <dyld_stub_binder+0x10129091c>
100d20e8c:     	add	x21, x21, x28
100d20e90:     	cmp	x24, x23
100d20e94:     	b.ne	0x100d20e64 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x12c>
100d20e98:     	b	0x100d2172c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x9f4>
100d20e9c:     	cbz	x24, 0x100d2172c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x9f4>
100d20ea0:     	mov	x9, #0x0                ; =0
100d20ea4:     	mov	w8, w22
100d20ea8:     	dup.2d	v7, x8
100d20eac:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d20eb0:     	ldr	q0, [x10, #0x720]
100d20eb4:     	ushl.2d	v0, v0, v7
100d20eb8:     	stur	q0, [x29, #-0xc0]
100d20ebc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20ec0:     	ldr	q0, [x10, #0x870]
100d20ec4:     	ushl.2d	v0, v0, v7
100d20ec8:     	stur	q0, [x29, #-0xd0]
100d20ecc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20ed0:     	ldr	q0, [x10, #0x880]
100d20ed4:     	ushl.2d	v0, v0, v7
100d20ed8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20edc:     	ldr	q1, [x10, #0x890]
100d20ee0:     	ushl.2d	v1, v1, v7
100d20ee4:     	mov	w10, #0x3e              ; =62
100d20ee8:     	dup.2d	v2, x10
100d20eec:     	and.16b	v3, v0, v2
100d20ef0:     	and.16b	v0, v1, v2
100d20ef4:     	stp	q0, q3, [x29, #-0xf0]
100d20ef8:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d20efc:     	ldr	q0, [x10, #0x740]
100d20f00:     	ushl.2d	v0, v0, v7
100d20f04:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f08:     	ldr	q1, [x10, #0x8a0]
100d20f0c:     	ushl.2d	v1, v1, v7
100d20f10:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f14:     	ldr	q3, [x10, #0x8b0]
100d20f18:     	ushl.2d	v3, v3, v7
100d20f1c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f20:     	ldr	q4, [x10, #0x8c0]
100d20f24:     	ushl.2d	v4, v4, v7
100d20f28:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f2c:     	ldr	q16, [x10, #0x8d0]
100d20f30:     	ushl.2d	v16, v16, v7
100d20f34:     	and.16b	v1, v1, v2
100d20f38:     	stur	q1, [x29, #-0x100]
100d20f3c:     	and.16b	v1, v3, v2
100d20f40:     	str	q1, [sp, #0x200]
100d20f44:     	and.16b	v3, v4, v2
100d20f48:     	and.16b	v1, v16, v2
100d20f4c:     	stp	q1, q3, [sp, #0x1a0]
100d20f50:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f54:     	ldr	q3, [x10, #0x8e0]
100d20f58:     	ushl.2d	v3, v3, v7
100d20f5c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f60:     	ldr	q4, [x10, #0x8f0]
100d20f64:     	ushl.2d	v4, v4, v7
100d20f68:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f6c:     	ldr	q16, [x10, #0x900]
100d20f70:     	ushl.2d	v16, v16, v7
100d20f74:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f78:     	ldr	q17, [x10, #0x910]
100d20f7c:     	ushl.2d	v17, v17, v7
100d20f80:     	and.16b	v5, v3, v2
100d20f84:     	and.16b	v1, v4, v2
100d20f88:     	stp	q1, q5, [sp, #0x180]
100d20f8c:     	and.16b	v3, v16, v2
100d20f90:     	and.16b	v1, v17, v2
100d20f94:     	stp	q1, q3, [sp, #0x150]
100d20f98:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20f9c:     	ldr	q3, [x10, #0x920]
100d20fa0:     	ushl.2d	v3, v3, v7
100d20fa4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20fa8:     	ldr	q4, [x10, #0x930]
100d20fac:     	ushl.2d	v4, v4, v7
100d20fb0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20fb4:     	ldr	q16, [x10, #0x940]
100d20fb8:     	ushl.2d	v16, v16, v7
100d20fbc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20fc0:     	ldr	q17, [x10, #0x950]
100d20fc4:     	ushl.2d	v17, v17, v7
100d20fc8:     	and.16b	v5, v3, v2
100d20fcc:     	and.16b	v1, v4, v2
100d20fd0:     	stp	q5, q1, [sp, #0x110]
100d20fd4:     	and.16b	v3, v16, v2
100d20fd8:     	and.16b	v1, v17, v2
100d20fdc:     	stp	q3, q1, [sp, #0x130]
100d20fe0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20fe4:     	ldr	q3, [x10, #0x960]
100d20fe8:     	ushl.2d	v3, v3, v7
100d20fec:     	and.16b	v1, v3, v2
100d20ff0:     	str	q1, [sp, #0x100]
100d20ff4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d20ff8:     	ldr	q3, [x10, #0x980]
100d20ffc:     	ushl.2d	v3, v3, v7
100d21000:     	and.16b	v1, v3, v2
100d21004:     	str	q1, [sp, #0xf0]
100d21008:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2100c:     	ldr	q3, [x10, #0x990]
100d21010:     	ushl.2d	v3, v3, v7
100d21014:     	and.16b	v1, v3, v2
100d21018:     	str	q1, [sp, #0xe0]
100d2101c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21020:     	ldr	q3, [x10, #0x9b0]
100d21024:     	ushl.2d	v3, v3, v7
100d21028:     	and.16b	v1, v3, v2
100d2102c:     	str	q1, [sp, #0xd0]
100d21030:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21034:     	ldr	q3, [x10, #0x9c0]
100d21038:     	ushl.2d	v3, v3, v7
100d2103c:     	and.16b	v1, v3, v2
100d21040:     	str	q1, [sp, #0xc0]
100d21044:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21048:     	ldr	q3, [x10, #0x9e0]
100d2104c:     	ushl.2d	v3, v3, v7
100d21050:     	and.16b	v1, v3, v2
100d21054:     	str	q1, [sp, #0xb0]
100d21058:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2105c:     	ldr	q3, [x10, #0x9f0]
100d21060:     	ushl.2d	v3, v3, v7
100d21064:     	and.16b	v1, v3, v2
100d21068:     	str	q1, [sp, #0xa0]
100d2106c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21070:     	ldr	q3, [x10, #0xa10]
100d21074:     	ushl.2d	v3, v3, v7
100d21078:     	mov	w10, #0x1e              ; =30
100d2107c:     	dup.2d	v4, x10
100d21080:     	and.16b	v1, v3, v4
100d21084:     	str	q1, [sp, #0x90]
100d21088:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2108c:     	ldr	q3, [x10, #0x660]
100d21090:     	ushl.2d	v3, v3, v7
100d21094:     	mov	w10, #0x2f              ; =47
100d21098:     	dup.2d	v4, x10
100d2109c:     	and.16b	v1, v3, v4
100d210a0:     	str	q1, [sp, #0x70]
100d210a4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d210a8:     	ldr	q3, [x10, #0xa20]
100d210ac:     	ushl.2d	v3, v3, v7
100d210b0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d210b4:     	ldr	q4, [x10, #0xa30]
100d210b8:     	ushl.2d	v4, v4, v7
100d210bc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d210c0:     	ldr	q16, [x10, #0xa40]
100d210c4:     	ushl.2d	v16, v16, v7
100d210c8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d210cc:     	ldr	q17, [x10, #0xa50]
100d210d0:     	ushl.2d	v17, v17, v7
100d210d4:     	and.16b	v1, v3, v2
100d210d8:     	str	q1, [sp, #0x80]
100d210dc:     	and.16b	v26, v4, v2
100d210e0:     	and.16b	v27, v16, v2
100d210e4:     	and.16b	v28, v17, v2
100d210e8:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d210ec:     	ldr	q2, [x10, #0x760]
100d210f0:     	ushl.2d	v2, v2, v7
100d210f4:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d210f8:     	ldr	q3, [x10, #0x940]
100d210fc:     	ushl.2d	v3, v3, v7
100d21100:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d21104:     	ldr	q4, [x10, #0x930]
100d21108:     	ushl.2d	v4, v4, v7
100d2110c:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d21110:     	ldr	q16, [x10, #0x920]
100d21114:     	ushl.2d	v23, v16, v7
100d21118:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d2111c:     	ldr	q17, [x10, #0x910]
100d21120:     	ushl.2d	v16, v17, v7
100d21124:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d21128:     	ldr	q18, [x10, #0x900]
100d2112c:     	ushl.2d	v17, v18, v7
100d21130:     	adrp	x10, 0x101322000 <GCC_except_table9287>
100d21134:     	ldr	q19, [x10, #0x8f0]
100d21138:     	ushl.2d	v18, v19, v7
100d2113c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21140:     	ldr	q20, [x10, #0x5f0]
100d21144:     	ushl.2d	v20, v20, v7
100d21148:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2114c:     	ldr	q21, [x10, #0x600]
100d21150:     	ushl.2d	v21, v21, v7
100d21154:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21158:     	ldr	q22, [x10, #0x4f0]
100d2115c:     	ushl.2d	v22, v22, v7
100d21160:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21164:     	ldr	q5, [x10, #0x610]
100d21168:     	ushl.2d	v5, v5, v7
100d2116c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21170:     	ldr	q6, [x10, #0x620]
100d21174:     	ushl.2d	v6, v6, v7
100d21178:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2117c:     	ldr	q24, [x10, #0x630]
100d21180:     	ushl.2d	v24, v24, v7
100d21184:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21188:     	ldr	q25, [x10, #0x640]
100d2118c:     	ushl.2d	v25, v25, v7
100d21190:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21194:     	ldr	q1, [x10, #0x650]
100d21198:     	ushl.2d	v1, v1, v7
100d2119c:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211a0:     	ldr	q29, [x10, #0x970]
100d211a4:     	ushl.2d	v29, v29, v7
100d211a8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211ac:     	ldr	q30, [x10, #0x690]
100d211b0:     	ushl.2d	v30, v30, v7
100d211b4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211b8:     	ldr	q31, [x10, #0x9a0]
100d211bc:     	ushl.2d	v31, v31, v7
100d211c0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211c4:     	ldr	q8, [x10, #0x680]
100d211c8:     	ushl.2d	v8, v8, v7
100d211cc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211d0:     	ldr	q9, [x10, #0x9d0]
100d211d4:     	ushl.2d	v9, v9, v7
100d211d8:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211dc:     	ldr	q10, [x10, #0x670]
100d211e0:     	ushl.2d	v10, v10, v7
100d211e4:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211e8:     	ldr	q11, [x10, #0xa00]
100d211ec:     	ushl.2d	v11, v11, v7
100d211f0:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d211f4:     	ldr	q15, [x10, #0xa60]
100d211f8:     	ushl.2d	v15, v15, v7
100d211fc:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21200:     	ldr	q14, [x10, #0xa70]
100d21204:     	ushl.2d	v14, v14, v7
100d21208:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d2120c:     	ldr	q13, [x10, #0xa80]
100d21210:     	ushl.2d	v13, v13, v7
100d21214:     	adrp	x10, 0x101323000 <dyld_stub_binder+0x101323000>
100d21218:     	ldr	q12, [x10, #0xa90]
100d2121c:     	ushl.2d	v12, v12, v7
100d21220:     	mov	w10, #0x3f              ; =63
100d21224:     	dup.2d	v7, x10
100d21228:     	and.16b	v19, v23, v7
100d2122c:     	and.16b	v16, v16, v7
100d21230:     	and.16b	v17, v17, v7
100d21234:     	and.16b	v18, v18, v7
100d21238:     	and.16b	v20, v20, v7
100d2123c:     	and.16b	v23, v21, v7
100d21240:     	mov.16b	v21, v20
100d21244:     	and.16b	v20, v22, v7
100d21248:     	mov.16b	v22, v23
100d2124c:     	and.16b	v5, v5, v7
100d21250:     	and.16b	v6, v6, v7
100d21254:     	str	q6, [sp, #0x1f0]
100d21258:     	and.16b	v6, v24, v7
100d2125c:     	str	q6, [sp, #0x1e0]
100d21260:     	and.16b	v6, v25, v7
100d21264:     	and.16b	v1, v1, v7
100d21268:     	stp	q1, q6, [sp, #0x1c0]
100d2126c:     	and.16b	v6, v29, v7
100d21270:     	and.16b	v1, v30, v7
100d21274:     	stp	q1, q6, [sp, #0x50]
100d21278:     	and.16b	v6, v31, v7
100d2127c:     	and.16b	v1, v8, v7
100d21280:     	stp	q1, q6, [sp, #0x30]
100d21284:     	mov.16b	v8, v20
100d21288:     	and.16b	v6, v9, v7
100d2128c:     	mov.16b	v9, v5
100d21290:     	and.16b	v10, v10, v7
100d21294:     	and.16b	v29, v11, v7
100d21298:     	and.16b	v1, v15, v7
100d2129c:     	stp	q1, q6, [sp, #0x10]
100d212a0:     	and.16b	v1, v14, v7
100d212a4:     	str	q1, [sp]
100d212a8:     	and.16b	v30, v13, v7
100d212ac:     	and.16b	v31, v12, v7
100d212b0:     	ldp	q1, q6, [x29, #-0xd0]
100d212b4:     	neg.2d	v5, v6
100d212b8:     	neg.2d	v6, v1
100d212bc:     	ldp	q1, q7, [x29, #-0xf0]
100d212c0:     	neg.2d	v24, v7
100d212c4:     	neg.2d	v25, v1
100d212c8:     	ldur	q1, [x29, #-0x100]
100d212cc:     	neg.2d	v11, v1
100d212d0:     	ldr	q1, [sp, #0x200]
100d212d4:     	neg.2d	v1, v1
100d212d8:     	ldr	q7, [sp, #0x1b0]
100d212dc:     	neg.2d	v7, v7
100d212e0:     	stur	q7, [x29, #-0xc0]
100d212e4:     	ldr	q7, [sp, #0x1a0]
100d212e8:     	neg.2d	v7, v7
100d212ec:     	stur	q7, [x29, #-0xd0]
100d212f0:     	ldr	q7, [sp, #0x190]
100d212f4:     	neg.2d	v7, v7
100d212f8:     	stur	q7, [x29, #-0xe0]
100d212fc:     	ldr	q7, [sp, #0x180]
100d21300:     	neg.2d	v7, v7
100d21304:     	stur	q7, [x29, #-0xf0]
100d21308:     	ldr	q7, [sp, #0x160]
100d2130c:     	neg.2d	v7, v7
100d21310:     	stur	q7, [x29, #-0x100]
100d21314:     	ldr	q7, [sp, #0x150]
100d21318:     	neg.2d	v7, v7
100d2131c:     	str	q7, [sp, #0x200]
100d21320:     	ldr	q7, [sp, #0x110]
100d21324:     	neg.2d	v7, v7
100d21328:     	str	q7, [sp, #0x1b0]
100d2132c:     	mov	w10, #0x1               ; =1
100d21330:     	ldr	q7, [sp, #0x120]
100d21334:     	neg.2d	v7, v7
100d21338:     	str	q7, [sp, #0x1a0]
100d2133c:     	lsl	x10, x10, x22
100d21340:     	ldr	q7, [sp, #0x130]
100d21344:     	neg.2d	v7, v7
100d21348:     	str	q7, [sp, #0x190]
100d2134c:     	mov	x11, #-0x1              ; =-1
100d21350:     	ldr	q7, [sp, #0x140]
100d21354:     	neg.2d	v7, v7
100d21358:     	str	q7, [sp, #0x180]
100d2135c:     	lsl	x10, x11, x10
100d21360:     	ldr	q7, [sp, #0x100]
100d21364:     	neg.2d	v7, v7
100d21368:     	str	q7, [sp, #0x160]
100d2136c:     	add	x11, x21, x24, lsl #3
100d21370:     	ldr	q7, [sp, #0xf0]
100d21374:     	neg.2d	v7, v7
100d21378:     	str	q7, [sp, #0x150]
100d2137c:     	mov	w12, w23
100d21380:     	ldr	q7, [sp, #0xe0]
100d21384:     	neg.2d	v7, v7
100d21388:     	str	q7, [sp, #0x140]
100d2138c:     	lsl	x12, x12, x8
100d21390:     	ldr	q7, [sp, #0xd0]
100d21394:     	neg.2d	v7, v7
100d21398:     	str	q7, [sp, #0x130]
100d2139c:     	mov	w13, #0x20              ; =32
100d213a0:     	ldr	q7, [sp, #0xc0]
100d213a4:     	neg.2d	v7, v7
100d213a8:     	str	q7, [sp, #0x120]
100d213ac:     	lsr	x13, x13, x8
100d213b0:     	ldr	q7, [sp, #0xb0]
100d213b4:     	neg.2d	v7, v7
100d213b8:     	str	q7, [sp, #0x110]
100d213bc:     	and	x14, x13, #0x38
100d213c0:     	ldr	q7, [sp, #0xa0]
100d213c4:     	neg.2d	v7, v7
100d213c8:     	str	q7, [sp, #0x100]
100d213cc:     	ldr	q7, [sp, #0x90]
100d213d0:     	neg.2d	v7, v7
100d213d4:     	str	q7, [sp, #0xf0]
100d213d8:     	ldr	q7, [sp, #0x80]
100d213dc:     	neg.2d	v7, v7
100d213e0:     	str	q7, [sp, #0xe0]
100d213e4:     	neg.2d	v7, v26
100d213e8:     	str	q7, [sp, #0xd0]
100d213ec:     	neg.2d	v7, v27
100d213f0:     	str	q7, [sp, #0xc0]
100d213f4:     	neg.2d	v7, v28
100d213f8:     	str	q7, [sp, #0xb0]
100d213fc:     	str	q1, [sp, #0x170]
100d21400:     	ldr	x15, [x21]
100d21404:     	lsr	x15, x15, x12
100d21408:     	cmp	w22, #0x2
100d2140c:     	b.ls	0x100d2141c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x6e4>
100d21410:     	mov	x17, #0x0               ; =0
100d21414:     	mov	x16, #0x0               ; =0
100d21418:     	b	0x100d216cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x994>
100d2141c:     	dup.2d	v12, x15
100d21420:     	ushl.2d	v7, v12, v5
100d21424:     	ushl.2d	v23, v12, v6
100d21428:     	ushl.2d	v26, v12, v24
100d2142c:     	ushl.2d	v27, v12, v25
100d21430:     	dup.2d	v13, x10
100d21434:     	bic.16b	v7, v7, v13
100d21438:     	bic.16b	v28, v23, v13
100d2143c:     	bic.16b	v14, v26, v13
100d21440:     	bic.16b	v15, v27, v13
100d21444:     	ushl.2d	v23, v7, v0
100d21448:     	ushl.2d	v26, v28, v2
100d2144c:     	ushl.2d	v27, v14, v3
100d21450:     	ushl.2d	v28, v15, v4
100d21454:     	cmp	x14, #0x8
100d21458:     	b.eq	0x100d216a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x970>
100d2145c:     	ushl.2d	v7, v12, v11
100d21460:     	ushl.2d	v14, v12, v1
100d21464:     	ldur	q20, [x29, #-0xc0]
100d21468:     	ushl.2d	v15, v12, v20
100d2146c:     	ldur	q20, [x29, #-0xd0]
100d21470:     	ushl.2d	v20, v12, v20
100d21474:     	bic.16b	v7, v7, v13
100d21478:     	bic.16b	v14, v14, v13
100d2147c:     	bic.16b	v15, v15, v13
100d21480:     	bic.16b	v20, v20, v13
100d21484:     	ushl.2d	v7, v7, v19
100d21488:     	ushl.2d	v14, v14, v16
100d2148c:     	ushl.2d	v15, v15, v17
100d21490:     	ushl.2d	v20, v20, v18
100d21494:     	orr.16b	v23, v7, v23
100d21498:     	orr.16b	v26, v14, v26
100d2149c:     	orr.16b	v27, v15, v27
100d214a0:     	orr.16b	v28, v20, v28
100d214a4:     	cmp	x14, #0x10
100d214a8:     	b.eq	0x100d216a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x970>
100d214ac:     	ldp	q20, q7, [x29, #-0xf0]
100d214b0:     	ushl.2d	v7, v12, v7
100d214b4:     	ushl.2d	v20, v12, v20
100d214b8:     	ldur	q14, [x29, #-0x100]
100d214bc:     	ushl.2d	v14, v12, v14
100d214c0:     	ldr	q15, [sp, #0x200]
100d214c4:     	ushl.2d	v15, v12, v15
100d214c8:     	bic.16b	v7, v7, v13
100d214cc:     	bic.16b	v20, v20, v13
100d214d0:     	bic.16b	v14, v14, v13
100d214d4:     	bic.16b	v15, v15, v13
100d214d8:     	ushl.2d	v7, v7, v21
100d214dc:     	ushl.2d	v20, v20, v22
100d214e0:     	ushl.2d	v14, v14, v8
100d214e4:     	ushl.2d	v15, v15, v9
100d214e8:     	orr.16b	v23, v7, v23
100d214ec:     	orr.16b	v26, v20, v26
100d214f0:     	orr.16b	v27, v14, v27
100d214f4:     	orr.16b	v28, v15, v28
100d214f8:     	cmp	x14, #0x18
100d214fc:     	b.eq	0x100d216a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x970>
100d21500:     	ldr	q1, [sp, #0x1b0]
100d21504:     	ushl.2d	v7, v12, v1
100d21508:     	ldr	q1, [sp, #0x1a0]
100d2150c:     	ushl.2d	v20, v12, v1
100d21510:     	ldr	q1, [sp, #0x190]
100d21514:     	ushl.2d	v14, v12, v1
100d21518:     	ldr	q1, [sp, #0x180]
100d2151c:     	ushl.2d	v15, v12, v1
100d21520:     	bic.16b	v7, v7, v13
100d21524:     	bic.16b	v20, v20, v13
100d21528:     	bic.16b	v14, v14, v13
100d2152c:     	bic.16b	v15, v15, v13
100d21530:     	ldr	q1, [sp, #0x1f0]
100d21534:     	ushl.2d	v7, v7, v1
100d21538:     	ldr	q1, [sp, #0x1e0]
100d2153c:     	ushl.2d	v20, v20, v1
100d21540:     	ldr	q1, [sp, #0x1d0]
100d21544:     	ushl.2d	v14, v14, v1
100d21548:     	ldr	q1, [sp, #0x1c0]
100d2154c:     	ushl.2d	v15, v15, v1
100d21550:     	orr.16b	v23, v7, v23
100d21554:     	orr.16b	v26, v20, v26
100d21558:     	orr.16b	v27, v14, v27
100d2155c:     	orr.16b	v28, v15, v28
100d21560:     	cmp	x14, #0x20
100d21564:     	b.eq	0x100d216a4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x96c>
100d21568:     	ldr	q1, [sp, #0x160]
100d2156c:     	ushl.2d	v7, v12, v1
100d21570:     	bic.16b	v7, v7, v13
100d21574:     	ldr	q1, [sp, #0x60]
100d21578:     	ushl.2d	v7, v7, v1
100d2157c:     	ldr	q1, [sp, #0x150]
100d21580:     	ushl.2d	v20, v12, v1
100d21584:     	bic.16b	v20, v20, v13
100d21588:     	ldr	q1, [sp, #0x50]
100d2158c:     	ushl.2d	v20, v20, v1
100d21590:     	orr.16b	v1, v20, v7
100d21594:     	str	q1, [sp, #0x90]
100d21598:     	ldr	q1, [sp, #0x140]
100d2159c:     	ushl.2d	v20, v12, v1
100d215a0:     	bic.16b	v20, v20, v13
100d215a4:     	ldr	q1, [sp, #0x40]
100d215a8:     	ushl.2d	v20, v20, v1
100d215ac:     	ldr	q1, [sp, #0x130]
100d215b0:     	ushl.2d	v14, v12, v1
100d215b4:     	bic.16b	v14, v14, v13
100d215b8:     	ldr	q1, [sp, #0x30]
100d215bc:     	ushl.2d	v14, v14, v1
100d215c0:     	orr.16b	v1, v14, v20
100d215c4:     	str	q1, [sp, #0x80]
100d215c8:     	ldr	q1, [sp, #0x120]
100d215cc:     	ushl.2d	v14, v12, v1
100d215d0:     	bic.16b	v14, v14, v13
100d215d4:     	ldr	q1, [sp, #0x20]
100d215d8:     	ushl.2d	v14, v14, v1
100d215dc:     	ldp	q1, q7, [sp, #0x100]
100d215e0:     	ushl.2d	v15, v12, v7
100d215e4:     	bic.16b	v15, v15, v13
100d215e8:     	ushl.2d	v15, v15, v10
100d215ec:     	orr.16b	v14, v15, v14
100d215f0:     	ushl.2d	v15, v12, v1
100d215f4:     	bic.16b	v15, v15, v13
100d215f8:     	ushl.2d	v15, v15, v29
100d215fc:     	str	q0, [sp, #0xa0]
100d21600:     	mov.16b	v7, v18
100d21604:     	mov.16b	v18, v21
100d21608:     	ldr	q0, [sp, #0xf0]
100d2160c:     	ushl.2d	v21, v12, v0
100d21610:     	bic.16b	v21, v21, v13
100d21614:     	mov.16b	v1, v22
100d21618:     	ldr	q22, [sp, #0x70]
100d2161c:     	ushl.2d	v21, v21, v22
100d21620:     	orr.16b	v21, v21, v15
100d21624:     	ldr	q0, [sp, #0xe0]
100d21628:     	ushl.2d	v15, v12, v0
100d2162c:     	ldr	q0, [sp, #0xd0]
100d21630:     	ushl.2d	v22, v12, v0
100d21634:     	mov.16b	v0, v8
100d21638:     	ldp	q20, q8, [sp, #0xb0]
100d2163c:     	ushl.2d	v8, v12, v8
100d21640:     	ushl.2d	v12, v12, v20
100d21644:     	bic.16b	v15, v15, v13
100d21648:     	bic.16b	v22, v22, v13
100d2164c:     	bic.16b	v8, v8, v13
100d21650:     	bic.16b	v12, v12, v13
100d21654:     	ldr	q13, [sp, #0x10]
100d21658:     	ushl.2d	v13, v15, v13
100d2165c:     	orr.16b	v21, v21, v13
100d21660:     	orr.16b	v23, v21, v23
100d21664:     	ldr	q21, [sp]
100d21668:     	ushl.2d	v21, v22, v21
100d2166c:     	mov.16b	v22, v1
100d21670:     	orr.16b	v21, v14, v21
100d21674:     	orr.16b	v26, v21, v26
100d21678:     	ushl.2d	v21, v8, v30
100d2167c:     	mov.16b	v8, v0
100d21680:     	ldp	q0, q1, [sp, #0x80]
100d21684:     	orr.16b	v20, v0, v21
100d21688:     	mov.16b	v21, v18
100d2168c:     	mov.16b	v18, v7
100d21690:     	ldr	q0, [sp, #0xa0]
100d21694:     	orr.16b	v27, v20, v27
100d21698:     	ushl.2d	v20, v12, v31
100d2169c:     	orr.16b	v7, v1, v20
100d216a0:     	orr.16b	v28, v7, v28
100d216a4:     	ldr	q1, [sp, #0x170]
100d216a8:     	orr.16b	v7, v26, v23
100d216ac:     	orr.16b	v20, v28, v27
100d216b0:     	orr.16b	v7, v20, v7
100d216b4:     	mov	d20, v7[1]
100d216b8:     	orr.8b	v7, v7, v20
100d216bc:     	fmov	x16, d7
100d216c0:     	and	x17, x13, #0x38
100d216c4:     	cmp	x13, x14
100d216c8:     	b.eq	0x100d216fc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x9c4>
100d216cc:     	lsl	x0, x17, #1
100d216d0:     	lsl	x1, x17, x8
100d216d4:     	add	x17, x17, #0x1
100d216d8:     	lsl	x2, x0, x8
100d216dc:     	and	x2, x2, #0x3e
100d216e0:     	lsr	x2, x15, x2
100d216e4:     	bic	x2, x2, x10
100d216e8:     	lsl	x1, x2, x1
100d216ec:     	orr	x16, x1, x16
100d216f0:     	add	x0, x0, #0x2
100d216f4:     	cmp	x13, x17
100d216f8:     	b.ne	0x100d216d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x998>
100d216fc:     	lsr	x0, x9, #1
100d21700:     	cmp	x0, x20
100d21704:     	b.hs	0x100d21800 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xac8>
100d21708:     	ubfiz	x15, x9, #5, #1
100d2170c:     	add	x9, x9, #0x1
100d21710:     	add	x21, x21, #0x8
100d21714:     	ldr	x17, [x19, x0, lsl #3]
100d21718:     	lsl	x15, x16, x15
100d2171c:     	orr	x15, x17, x15
100d21720:     	str	x15, [x19, x0, lsl #3]
100d21724:     	cmp	x21, x11
100d21728:     	b.ne	0x100d21400 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0x6c8>
100d2172c:     	cmp	w26, #0x6
100d21730:     	b.hs	0x100d2174c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xa14>
100d21734:     	cbz	x20, 0x100d21810 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy7scratch13cofactor_into+0xad8>
100d21738:     	mov	x8, #-0x1               ; =-1
100d2173c:     	lsl	x8, x8, x25
100d21740:     	ldr	x9, [x19]
100d21744:     	bic	x8, x9, x8
100d21748:     	str	x8, [x19]
100d2174c:     	add	sp, sp, #0x280
100d21750:     	ldp	x29, x30, [sp, #0x90]
100d21754:     	ldp	x20, x19, [sp, #0x80]
100d21758:     	ldp	x22, x21, [sp, #0x70]
100d2175c:     	ldp	x24, x23, [sp, #0x60]
100d21760:     	ldp	x26, x25, [sp, #0x50]
100d21764:     	ldp	x28, x27, [sp, #0x40]
100d21768:     	ldp	d9, d8, [sp, #0x30]
100d2176c:     	ldp	d11, d10, [sp, #0x20]
100d21770:     	ldp	d13, d12, [sp, #0x10]
100d21774:     	ldp	d15, d14, [sp], #0xa0
100d21778:     	ret
100d2177c:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21780:     	add	x2, x2, #0xe98
100d21784:     	mov	x1, x22
100d21788:     	bl	0x1012887b8 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d2178c:     	adrp	x0, 0x10134f000 <dyld_stub_binder+0x10134f000>
100d21790:     	add	x0, x0, #0xfcd
100d21794:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21798:     	add	x2, x2, #0xe08
100d2179c:     	mov	w1, #0x22               ; =34
100d217a0:     	bl	0x101288408 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d217a4:     	adrp	x5, 0x101510000 <dyld_stub_binder+0x101510000>
100d217a8:     	add	x5, x5, #0xe20
100d217ac:     	sub	x1, x29, #0xb0
100d217b0:     	sub	x2, x29, #0xa0
100d217b4:     	mov	w0, #0x0                ; =0
100d217b8:     	mov	x3, #0x0                ; =0
100d217bc:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d217c0:     	adrp	x5, 0x101510000 <dyld_stub_binder+0x101510000>
100d217c4:     	add	x5, x5, #0xe38
100d217c8:     	sub	x1, x29, #0xa8
100d217cc:     	sub	x2, x29, #0xa0
100d217d0:     	mov	w0, #0x0                ; =0
100d217d4:     	mov	x3, #0x0                ; =0
100d217d8:     	bl	0x1012882f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100d217dc:     	adrp	x3, 0x101510000 <dyld_stub_binder+0x101510000>
100d217e0:     	add	x3, x3, #0xeb0
100d217e4:     	bl	0x101288354 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d217e8:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100d217ec:     	add	x0, x0, #0xc45
100d217f0:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d217f4:     	add	x2, x2, #0xe68
100d217f8:     	mov	w1, #0x37               ; =55
100d217fc:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d21800:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21804:     	add	x2, x2, #0xe50
100d21808:     	mov	x1, x20
100d2180c:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d21810:     	adrp	x2, 0x101510000 <dyld_stub_binder+0x101510000>
100d21814:     	add	x2, x2, #0xe80
100d21818:     	mov	x0, #0x0                ; =0
100d2181c:     	mov	x1, #0x0                ; =0
100d21820:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
