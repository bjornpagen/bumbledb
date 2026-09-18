
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100790dfc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract>:
100790dfc:     	sub	sp, sp, #0x70
100790e00:     	stp	x28, x27, [sp, #0x10]
100790e04:     	stp	x26, x25, [sp, #0x20]
100790e08:     	stp	x24, x23, [sp, #0x30]
100790e0c:     	stp	x22, x21, [sp, #0x40]
100790e10:     	stp	x20, x19, [sp, #0x50]
100790e14:     	stp	x29, x30, [sp, #0x60]
100790e18:     	add	x29, sp, #0x60
100790e1c:     	lsl	x20, x3, #3
100790e20:     	lsr	x8, x3, #61
100790e24:     	mov	x9, #-0x7               ; =-7
100790e28:     	movk	x9, #0x7fff, lsl #48
100790e2c:     	cmp	x20, x9
100790e30:     	ccmp	x8, #0x0, #0x0, lo
100790e34:     	b.eq	0x100790e3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x40>
100790e38:     	bl	0x100dabd90 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
100790e3c:     	cbz	x20, 0x100790ea0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xa4>
100790e40:     	mov	x21, x5
100790e44:     	mov	x23, x6
100790e48:     	mov	x22, x7
100790e4c:     	mov	x25, x1
100790e50:     	mov	x24, x0
100790e54:     	mov	x27, x4
100790e58:     	mov	x26, x2
100790e5c:     	mov	x28, x3
100790e60:     	mov	x0, x20
100790e64:     	mov	w1, #0x1                ; =1
100790e68:     	bl	0x100db4824 <dyld_stub_binder+0x100db4824>
100790e6c:     	cbz	x0, 0x1007910bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x2c0>
100790e70:     	mov	x19, x0
100790e74:     	mov	x3, x28
100790e78:     	mov	x9, x28
100790e7c:     	mov	x2, x26
100790e80:     	mov	x4, x27
100790e84:     	mov	x0, x24
100790e88:     	mov	x1, x25
100790e8c:     	mov	x7, x22
100790e90:     	mov	x6, x23
100790e94:     	mov	x5, x21
100790e98:     	cbnz	x21, 0x100790eac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xb0>
100790e9c:     	b	0x10079104c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x250>
100790ea0:     	mov	x9, #0x0                ; =0
100790ea4:     	mov	w19, #0x8               ; =8
100790ea8:     	cbz	x5, 0x10079104c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x250>
100790eac:     	add	x10, x4, x5, lsl #3
100790eb0:     	mov	x11, #0x0               ; =0
100790eb4:     	adrp	x8, 0x100fe6000 <dyld_stub_binder+0x100fe6000>
100790eb8:     	add	x8, x8, #0x7e8
100790ebc:     	mov	w13, #0x18              ; =24
100790ec0:     	adrp	x12, 0x100fe6000 <dyld_stub_binder+0x100fe6000>
100790ec4:     	add	x12, x12, #0x800
100790ec8:     	cbnz	x6, 0x100790ee0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xe4>
100790ecc:     	b	0x100790fa4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x1a8>
100790ed0:     	add	x4, x4, #0x8
100790ed4:     	add	x11, x11, #0x1
100790ed8:     	cmp	x4, x10
100790edc:     	b.eq	0x10079104c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x250>
100790ee0:     	cmp	x11, x7
100790ee4:     	b.eq	0x1007910a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x2a4>
100790ee8:     	cmp	x11, x2
100790eec:     	b.eq	0x100791088 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x28c>
100790ef0:     	madd	x16, x11, x13, x1
100790ef4:     	ldr	x15, [x16, #0x10]
100790ef8:     	cbz	x15, 0x100790ed0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xd4>
100790efc:     	ldr	x14, [x4]
100790f00:     	ldr	x17, [x6, x11, lsl #3]
100790f04:     	eor	x14, x17, x14
100790f08:     	ldr	x16, [x16, #0x8]
100790f0c:     	add	x15, x16, x15, lsl #4
100790f10:     	add	x20, x16, #0x10
100790f14:     	ldr	x17, [x16, #0x8]
100790f18:     	ands	x17, x17, x14
100790f1c:     	b.eq	0x100790f38 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x13c>
100790f20:     	mov	x5, x16
100790f24:     	mov	x16, x20
100790f28:     	ldr	x5, [x5]
100790f2c:     	cmp	x5, x3
100790f30:     	b.lo	0x100790f70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x174>
100790f34:     	b	0x100791074 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x278>
100790f38:     	cmp	x20, x15
100790f3c:     	b.eq	0x100790ed0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xd4>
100790f40:     	add	x16, x16, #0x20
100790f44:     	ldur	x17, [x16, #-0x8]
100790f48:     	ands	x17, x17, x14
100790f4c:     	b.ne	0x100790f60 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x164>
100790f50:     	cmp	x16, x15
100790f54:     	add	x16, x16, #0x10
100790f58:     	b.ne	0x100790f44 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x148>
100790f5c:     	b	0x100790ed0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xd4>
100790f60:     	sub	x5, x16, #0x10
100790f64:     	ldr	x5, [x5]
100790f68:     	cmp	x5, x3
100790f6c:     	b.hs	0x100791074 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x278>
100790f70:     	fmov	d0, x17
100790f74:     	cnt.8b	v0, v0
100790f78:     	addv.8b	b0, v0
100790f7c:     	fmov	x17, d0
100790f80:     	ldr	x20, [x19, x5, lsl #3]
100790f84:     	add	x17, x20, x17
100790f88:     	str	x17, [x19, x5, lsl #3]
100790f8c:     	cmp	x16, x15
100790f90:     	b.ne	0x100790f10 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x114>
100790f94:     	b	0x100790ed0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0xd4>
100790f98:     	add	x11, x11, #0x1
100790f9c:     	cmp	x4, x10
100790fa0:     	b.eq	0x10079104c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x250>
100790fa4:     	cmp	x11, x2
100790fa8:     	b.eq	0x100791088 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x28c>
100790fac:     	ldr	x14, [x4], #0x8
100790fb0:     	madd	x16, x11, x13, x1
100790fb4:     	ldr	x15, [x16, #0x10]
100790fb8:     	cbz	x15, 0x100790f98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x19c>
100790fbc:     	ldr	x16, [x16, #0x8]
100790fc0:     	add	x15, x16, x15, lsl #4
100790fc4:     	add	x6, x16, #0x10
100790fc8:     	ldr	x17, [x16, #0x8]
100790fcc:     	ands	x17, x17, x14
100790fd0:     	b.eq	0x100790fec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x1f0>
100790fd4:     	mov	x5, x16
100790fd8:     	mov	x16, x6
100790fdc:     	ldr	x5, [x5]
100790fe0:     	cmp	x5, x3
100790fe4:     	b.lo	0x100791024 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x228>
100790fe8:     	b	0x100791074 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x278>
100790fec:     	cmp	x6, x15
100790ff0:     	b.eq	0x100790f98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x19c>
100790ff4:     	add	x16, x16, #0x20
100790ff8:     	ldur	x17, [x16, #-0x8]
100790ffc:     	ands	x17, x17, x14
100791000:     	b.ne	0x100791014 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x218>
100791004:     	cmp	x16, x15
100791008:     	add	x16, x16, #0x10
10079100c:     	b.ne	0x100790ff8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x1fc>
100791010:     	b	0x100790f98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x19c>
100791014:     	sub	x5, x16, #0x10
100791018:     	ldr	x5, [x5]
10079101c:     	cmp	x5, x3
100791020:     	b.hs	0x100791074 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x278>
100791024:     	fmov	d0, x17
100791028:     	cnt.8b	v0, v0
10079102c:     	addv.8b	b0, v0
100791030:     	fmov	x17, d0
100791034:     	ldr	x6, [x19, x5, lsl #3]
100791038:     	add	x17, x6, x17
10079103c:     	str	x17, [x19, x5, lsl #3]
100791040:     	cmp	x16, x15
100791044:     	b.ne	0x100790fc4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x1c8>
100791048:     	b	0x100790f98 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x19c>
10079104c:     	stp	x9, x19, [x0]
100791050:     	str	x3, [x0, #0x10]
100791054:     	ldp	x29, x30, [sp, #0x60]
100791058:     	ldp	x20, x19, [sp, #0x50]
10079105c:     	ldp	x22, x21, [sp, #0x40]
100791060:     	ldp	x24, x23, [sp, #0x30]
100791064:     	ldp	x26, x25, [sp, #0x20]
100791068:     	ldp	x28, x27, [sp, #0x10]
10079106c:     	add	sp, sp, #0x70
100791070:     	ret
100791074:     	str	x9, [sp, #0x8]
100791078:     	mov	x1, x3
10079107c:     	mov	x0, x5
100791080:     	mov	x8, x12
100791084:     	b	0x100791094 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x298>
100791088:     	str	x9, [sp, #0x8]
10079108c:     	mov	x0, x2
100791090:     	mov	x1, x2
100791094:     	mov	x2, x8
100791098:     	bl	0x100dac55c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10079109c:     	b	0x1007910b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x2bc>
1007910a0:     	str	x9, [sp, #0x8]
1007910a4:     	adrp	x2, 0x100fac000 <dyld_stub_binder+0x100fac000>
1007910a8:     	add	x2, x2, #0xbc8
1007910ac:     	mov	x0, x7
1007910b0:     	mov	x1, x7
1007910b4:     	bl	0x100dac55c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1007910b8:     	brk	#0x1
1007910bc:     	mov	w0, #0x8                ; =8
1007910c0:     	mov	x1, x20
1007910c4:     	bl	0x100dabd64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1007910c8:     	mov	x20, x0
1007910cc:     	ldr	x8, [sp, #0x8]
1007910d0:     	cbz	x8, 0x1007910dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11observationNtB2_9TablePlan8contract+0x2e0>
1007910d4:     	mov	x0, x19
1007910d8:     	bl	0x100db4938 <dyld_stub_binder+0x100db4938>
1007910dc:     	mov	x0, x20
1007910e0:     	bl	0x100db4788 <dyld_stub_binder+0x100db4788>
