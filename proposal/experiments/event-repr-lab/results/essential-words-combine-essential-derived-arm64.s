
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100ba3d34 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine>:
100ba3d34:     	sub	sp, sp, #0x50
100ba3d38:     	stp	x24, x23, [sp, #0x10]
100ba3d3c:     	stp	x22, x21, [sp, #0x20]
100ba3d40:     	stp	x20, x19, [sp, #0x30]
100ba3d44:     	stp	x29, x30, [sp, #0x40]
100ba3d48:     	add	x29, sp, #0x40
100ba3d4c:     	stp	x3, x5, [sp]
100ba3d50:     	cmp	x3, x5
100ba3d54:     	b.ne	0x100ba4374 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x640>
100ba3d58:     	and	w8, w1, #0xff
100ba3d5c:     	cmp	w8, #0xf
100ba3d60:     	b.hi	0x100ba4390 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x65c>
100ba3d64:     	mov	x21, x4
100ba3d68:     	mov	x19, x3
100ba3d6c:     	mov	x22, x2
100ba3d70:     	mov	x20, x0
100ba3d74:     	and	x8, x1, #0xff
100ba3d78:     	adrp	x9, 0x1011a0000 <dyld_stub_binder+0x1011a0000>
100ba3d7c:     	add	x9, x9, #0xafa
100ba3d80:     	adr	x10, 0x100ba3d90 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5c>
100ba3d84:     	ldrb	w11, [x9, x8]
100ba3d88:     	add	x10, x10, x11, lsl #2
100ba3d8c:     	br	x10
100ba3d90:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3d94:     	lsl	x21, x19, #3
100ba3d98:     	mov	x0, x21
100ba3d9c:     	mov	w1, #0x1                ; =1
100ba3da0:     	bl	0x10110d024 <dyld_stub_binder+0x10110d024>
100ba3da4:     	cbnz	x0, 0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba3da8:     	b	0x100ba43b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x680>
100ba3dac:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3db0:     	lsl	x23, x19, #3
100ba3db4:     	mov	x0, x23
100ba3db8:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3dbc:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3dc0:     	cmp	x19, #0x8
100ba3dc4:     	b.hs	0x100ba3ffc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2c8>
100ba3dc8:     	mov	x8, #0x0                ; =0
100ba3dcc:     	b	0x100ba43d4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6a0>
100ba3dd0:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3dd4:     	lsl	x23, x19, #3
100ba3dd8:     	mov	x0, x23
100ba3ddc:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3de0:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3de4:     	cmp	x19, #0x8
100ba3de8:     	b.hs	0x100ba4044 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x310>
100ba3dec:     	mov	x8, #0x0                ; =0
100ba3df0:     	b	0x100ba43f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6c0>
100ba3df4:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3df8:     	lsl	x23, x19, #3
100ba3dfc:     	mov	x0, x23
100ba3e00:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3e04:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3e08:     	cmp	x19, #0x8
100ba3e0c:     	b.hs	0x100ba408c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x358>
100ba3e10:     	mov	x8, #0x0                ; =0
100ba3e14:     	b	0x100ba4414 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6e0>
100ba3e18:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3e1c:     	lsl	x21, x19, #3
100ba3e20:     	mov	x0, x21
100ba3e24:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3e28:     	cbz	x0, 0x100ba43b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x680>
100ba3e2c:     	mov	x23, x0
100ba3e30:     	mov	x1, x22
100ba3e34:     	mov	x2, x21
100ba3e38:     	b	0x100ba3fa8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x274>
100ba3e3c:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3e40:     	lsl	x23, x19, #3
100ba3e44:     	mov	x0, x23
100ba3e48:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3e4c:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3e50:     	cmp	x19, #0x8
100ba3e54:     	b.hs	0x100ba40d4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3a0>
100ba3e58:     	mov	x8, #0x0                ; =0
100ba3e5c:     	b	0x100ba4434 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x700>
100ba3e60:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3e64:     	lsl	x23, x19, #3
100ba3e68:     	mov	x0, x23
100ba3e6c:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3e70:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3e74:     	cmp	x19, #0x8
100ba3e78:     	b.hs	0x100ba412c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3f8>
100ba3e7c:     	mov	x8, #0x0                ; =0
100ba3e80:     	b	0x100ba4454 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x720>
100ba3e84:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3e88:     	lsl	x21, x19, #3
100ba3e8c:     	mov	x0, x21
100ba3e90:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3e94:     	cbz	x0, 0x100ba43b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x680>
100ba3e98:     	mov	x22, x0
100ba3e9c:     	mov	w1, #0xff               ; =255
100ba3ea0:     	mov	x2, x21
100ba3ea4:     	bl	0x10110d234 <dyld_stub_binder+0x10110d234>
100ba3ea8:     	mov	x0, x22
100ba3eac:     	b	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba3eb0:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3eb4:     	lsl	x21, x19, #3
100ba3eb8:     	mov	x0, x21
100ba3ebc:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3ec0:     	cbz	x0, 0x100ba43b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x680>
100ba3ec4:     	cmp	x19, #0x8
100ba3ec8:     	b.hs	0x100ba4174 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x440>
100ba3ecc:     	mov	x8, #0x0                ; =0
100ba3ed0:     	b	0x100ba4474 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x740>
100ba3ed4:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3ed8:     	lsl	x23, x19, #3
100ba3edc:     	mov	x0, x23
100ba3ee0:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3ee4:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3ee8:     	cmp	x19, #0x8
100ba3eec:     	b.hs	0x100ba41b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x47c>
100ba3ef0:     	mov	x8, #0x0                ; =0
100ba3ef4:     	b	0x100ba4490 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x75c>
100ba3ef8:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3efc:     	lsl	x23, x19, #3
100ba3f00:     	mov	x0, x23
100ba3f04:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3f08:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3f0c:     	cmp	x19, #0x8
100ba3f10:     	b.hs	0x100ba4208 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4d4>
100ba3f14:     	mov	x8, #0x0                ; =0
100ba3f18:     	b	0x100ba44b4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x780>
100ba3f1c:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3f20:     	lsl	x22, x19, #3
100ba3f24:     	mov	x0, x22
100ba3f28:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3f2c:     	cbz	x0, 0x100ba43c0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x68c>
100ba3f30:     	cmp	x19, #0x8
100ba3f34:     	b.hs	0x100ba4260 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x52c>
100ba3f38:     	mov	x8, #0x0                ; =0
100ba3f3c:     	b	0x100ba44d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7a4>
100ba3f40:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3f44:     	lsl	x23, x19, #3
100ba3f48:     	mov	x0, x23
100ba3f4c:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3f50:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3f54:     	cmp	x19, #0x8
100ba3f58:     	b.hs	0x100ba429c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x568>
100ba3f5c:     	mov	x8, #0x0                ; =0
100ba3f60:     	b	0x100ba44f4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7c0>
100ba3f64:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3f68:     	lsl	x23, x19, #3
100ba3f6c:     	mov	x0, x23
100ba3f70:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3f74:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3f78:     	cmp	x19, #0x8
100ba3f7c:     	b.hs	0x100ba42e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5b0>
100ba3f80:     	mov	x8, #0x0                ; =0
100ba3f84:     	b	0x100ba4514 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7e0>
100ba3f88:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3f8c:     	lsl	x22, x19, #3
100ba3f90:     	mov	x0, x22
100ba3f94:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3f98:     	cbz	x0, 0x100ba43c0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x68c>
100ba3f9c:     	mov	x23, x0
100ba3fa0:     	mov	x1, x21
100ba3fa4:     	mov	x2, x22
100ba3fa8:     	bl	0x10110d21c <dyld_stub_binder+0x10110d21c>
100ba3fac:     	mov	x0, x23
100ba3fb0:     	b	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba3fb4:     	cbz	x19, 0x100ba3fd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a4>
100ba3fb8:     	lsl	x23, x19, #3
100ba3fbc:     	mov	x0, x23
100ba3fc0:     	bl	0x10110d204 <dyld_stub_binder+0x10110d204>
100ba3fc4:     	cbz	x0, 0x100ba43a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x674>
100ba3fc8:     	cmp	x19, #0x8
100ba3fcc:     	b.hs	0x100ba432c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5f8>
100ba3fd0:     	mov	x8, #0x0                ; =0
100ba3fd4:     	b	0x100ba4534 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x800>
100ba3fd8:     	mov	w0, #0x8                ; =8
100ba3fdc:     	stp	x19, x0, [x20]
100ba3fe0:     	str	x19, [x20, #0x10]
100ba3fe4:     	ldp	x29, x30, [sp, #0x40]
100ba3fe8:     	ldp	x20, x19, [sp, #0x30]
100ba3fec:     	ldp	x22, x21, [sp, #0x20]
100ba3ff0:     	ldp	x24, x23, [sp, #0x10]
100ba3ff4:     	add	sp, sp, #0x50
100ba3ff8:     	ret
100ba3ffc:     	and	x8, x19, #0xffffffffffffff8
100ba4000:     	add	x9, x22, #0x20
100ba4004:     	add	x10, x0, #0x20
100ba4008:     	add	x11, x21, #0x20
100ba400c:     	and	x12, x19, #0xffffffffffffff8
100ba4010:     	ldp	q0, q1, [x9, #-0x20]
100ba4014:     	ldp	q2, q3, [x9], #0x40
100ba4018:     	ldp	q4, q5, [x11, #-0x20]
100ba401c:     	ldp	q6, q7, [x11], #0x40
100ba4020:     	orr.16b	v0, v4, v0
100ba4024:     	orr.16b	v1, v5, v1
100ba4028:     	orr.16b	v2, v6, v2
100ba402c:     	orr.16b	v3, v7, v3
100ba4030:     	stp	q0, q1, [x10, #-0x20]
100ba4034:     	stp	q2, q3, [x10], #0x40
100ba4038:     	subs	x12, x12, #0x8
100ba403c:     	b.ne	0x100ba4010 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2dc>
100ba4040:     	b	0x100ba43cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x698>
100ba4044:     	and	x8, x19, #0xffffffffffffff8
100ba4048:     	add	x9, x22, #0x20
100ba404c:     	add	x10, x0, #0x20
100ba4050:     	add	x11, x21, #0x20
100ba4054:     	and	x12, x19, #0xffffffffffffff8
100ba4058:     	ldp	q0, q1, [x9, #-0x20]
100ba405c:     	ldp	q2, q3, [x9], #0x40
100ba4060:     	ldp	q4, q5, [x11, #-0x20]
100ba4064:     	ldp	q6, q7, [x11], #0x40
100ba4068:     	orn.16b	v0, v4, v0
100ba406c:     	orn.16b	v1, v5, v1
100ba4070:     	orn.16b	v2, v6, v2
100ba4074:     	orn.16b	v3, v7, v3
100ba4078:     	stp	q0, q1, [x10, #-0x20]
100ba407c:     	stp	q2, q3, [x10], #0x40
100ba4080:     	subs	x12, x12, #0x8
100ba4084:     	b.ne	0x100ba4058 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x324>
100ba4088:     	b	0x100ba43ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b8>
100ba408c:     	and	x8, x19, #0xffffffffffffff8
100ba4090:     	add	x9, x22, #0x20
100ba4094:     	add	x10, x0, #0x20
100ba4098:     	add	x11, x21, #0x20
100ba409c:     	and	x12, x19, #0xffffffffffffff8
100ba40a0:     	ldp	q0, q1, [x9, #-0x20]
100ba40a4:     	ldp	q2, q3, [x9], #0x40
100ba40a8:     	ldp	q4, q5, [x11, #-0x20]
100ba40ac:     	ldp	q6, q7, [x11], #0x40
100ba40b0:     	bic.16b	v0, v0, v4
100ba40b4:     	bic.16b	v1, v1, v5
100ba40b8:     	bic.16b	v2, v2, v6
100ba40bc:     	bic.16b	v3, v3, v7
100ba40c0:     	stp	q0, q1, [x10, #-0x20]
100ba40c4:     	stp	q2, q3, [x10], #0x40
100ba40c8:     	subs	x12, x12, #0x8
100ba40cc:     	b.ne	0x100ba40a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x36c>
100ba40d0:     	b	0x100ba440c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d8>
100ba40d4:     	and	x8, x19, #0xffffffffffffff8
100ba40d8:     	add	x9, x22, #0x20
100ba40dc:     	add	x10, x0, #0x20
100ba40e0:     	add	x11, x21, #0x20
100ba40e4:     	and	x12, x19, #0xffffffffffffff8
100ba40e8:     	ldp	q0, q1, [x9, #-0x20]
100ba40ec:     	ldp	q2, q3, [x9], #0x40
100ba40f0:     	ldp	q4, q5, [x11, #-0x20]
100ba40f4:     	ldp	q6, q7, [x11], #0x40
100ba40f8:     	eor.16b	v0, v0, v4
100ba40fc:     	eor.16b	v1, v1, v5
100ba4100:     	eor.16b	v2, v2, v6
100ba4104:     	eor.16b	v3, v3, v7
100ba4108:     	mvn.16b	v0, v0
100ba410c:     	mvn.16b	v1, v1
100ba4110:     	mvn.16b	v2, v2
100ba4114:     	mvn.16b	v3, v3
100ba4118:     	stp	q0, q1, [x10, #-0x20]
100ba411c:     	stp	q2, q3, [x10], #0x40
100ba4120:     	subs	x12, x12, #0x8
100ba4124:     	b.ne	0x100ba40e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x3b4>
100ba4128:     	b	0x100ba442c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f8>
100ba412c:     	and	x8, x19, #0xffffffffffffff8
100ba4130:     	add	x9, x22, #0x20
100ba4134:     	add	x10, x0, #0x20
100ba4138:     	add	x11, x21, #0x20
100ba413c:     	and	x12, x19, #0xffffffffffffff8
100ba4140:     	ldp	q0, q1, [x9, #-0x20]
100ba4144:     	ldp	q2, q3, [x9], #0x40
100ba4148:     	ldp	q4, q5, [x11, #-0x20]
100ba414c:     	ldp	q6, q7, [x11], #0x40
100ba4150:     	bic.16b	v0, v4, v0
100ba4154:     	bic.16b	v1, v5, v1
100ba4158:     	bic.16b	v2, v6, v2
100ba415c:     	bic.16b	v3, v7, v3
100ba4160:     	stp	q0, q1, [x10, #-0x20]
100ba4164:     	stp	q2, q3, [x10], #0x40
100ba4168:     	subs	x12, x12, #0x8
100ba416c:     	b.ne	0x100ba4140 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x40c>
100ba4170:     	b	0x100ba444c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x718>
100ba4174:     	and	x8, x19, #0xffffffffffffff8
100ba4178:     	add	x9, x22, #0x20
100ba417c:     	add	x10, x0, #0x20
100ba4180:     	and	x11, x19, #0xffffffffffffff8
100ba4184:     	ldp	q0, q1, [x9, #-0x20]
100ba4188:     	ldp	q2, q3, [x9], #0x40
100ba418c:     	mvn.16b	v0, v0
100ba4190:     	mvn.16b	v1, v1
100ba4194:     	mvn.16b	v2, v2
100ba4198:     	mvn.16b	v3, v3
100ba419c:     	stp	q0, q1, [x10, #-0x20]
100ba41a0:     	stp	q2, q3, [x10], #0x40
100ba41a4:     	subs	x11, x11, #0x8
100ba41a8:     	b.ne	0x100ba4184 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x450>
100ba41ac:     	b	0x100ba446c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x738>
100ba41b0:     	and	x8, x19, #0xffffffffffffff8
100ba41b4:     	add	x9, x22, #0x20
100ba41b8:     	add	x10, x0, #0x20
100ba41bc:     	add	x11, x21, #0x20
100ba41c0:     	and	x12, x19, #0xffffffffffffff8
100ba41c4:     	ldp	q0, q1, [x9, #-0x20]
100ba41c8:     	ldp	q2, q3, [x9], #0x40
100ba41cc:     	ldp	q4, q5, [x11, #-0x20]
100ba41d0:     	ldp	q6, q7, [x11], #0x40
100ba41d4:     	and.16b	v0, v4, v0
100ba41d8:     	and.16b	v1, v5, v1
100ba41dc:     	and.16b	v2, v6, v2
100ba41e0:     	and.16b	v3, v7, v3
100ba41e4:     	mvn.16b	v0, v0
100ba41e8:     	mvn.16b	v1, v1
100ba41ec:     	mvn.16b	v2, v2
100ba41f0:     	mvn.16b	v3, v3
100ba41f4:     	stp	q0, q1, [x10, #-0x20]
100ba41f8:     	stp	q2, q3, [x10], #0x40
100ba41fc:     	subs	x12, x12, #0x8
100ba4200:     	b.ne	0x100ba41c4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x490>
100ba4204:     	b	0x100ba4488 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100ba4208:     	and	x8, x19, #0xffffffffffffff8
100ba420c:     	add	x9, x22, #0x20
100ba4210:     	add	x10, x0, #0x20
100ba4214:     	add	x11, x21, #0x20
100ba4218:     	and	x12, x19, #0xffffffffffffff8
100ba421c:     	ldp	q0, q1, [x9, #-0x20]
100ba4220:     	ldp	q2, q3, [x9], #0x40
100ba4224:     	ldp	q4, q5, [x11, #-0x20]
100ba4228:     	ldp	q6, q7, [x11], #0x40
100ba422c:     	orr.16b	v0, v4, v0
100ba4230:     	orr.16b	v1, v5, v1
100ba4234:     	orr.16b	v2, v6, v2
100ba4238:     	orr.16b	v3, v7, v3
100ba423c:     	mvn.16b	v0, v0
100ba4240:     	mvn.16b	v1, v1
100ba4244:     	mvn.16b	v2, v2
100ba4248:     	mvn.16b	v3, v3
100ba424c:     	stp	q0, q1, [x10, #-0x20]
100ba4250:     	stp	q2, q3, [x10], #0x40
100ba4254:     	subs	x12, x12, #0x8
100ba4258:     	b.ne	0x100ba421c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x4e8>
100ba425c:     	b	0x100ba44ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100ba4260:     	and	x8, x19, #0xffffffffffffff8
100ba4264:     	add	x9, x21, #0x20
100ba4268:     	add	x10, x0, #0x20
100ba426c:     	and	x11, x19, #0xffffffffffffff8
100ba4270:     	ldp	q0, q1, [x9, #-0x20]
100ba4274:     	ldp	q2, q3, [x9], #0x40
100ba4278:     	mvn.16b	v0, v0
100ba427c:     	mvn.16b	v1, v1
100ba4280:     	mvn.16b	v2, v2
100ba4284:     	mvn.16b	v3, v3
100ba4288:     	stp	q0, q1, [x10, #-0x20]
100ba428c:     	stp	q2, q3, [x10], #0x40
100ba4290:     	subs	x11, x11, #0x8
100ba4294:     	b.ne	0x100ba4270 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x53c>
100ba4298:     	b	0x100ba44d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x79c>
100ba429c:     	and	x8, x19, #0xffffffffffffff8
100ba42a0:     	add	x9, x22, #0x20
100ba42a4:     	add	x10, x0, #0x20
100ba42a8:     	add	x11, x21, #0x20
100ba42ac:     	and	x12, x19, #0xffffffffffffff8
100ba42b0:     	ldp	q0, q1, [x9, #-0x20]
100ba42b4:     	ldp	q2, q3, [x9], #0x40
100ba42b8:     	ldp	q4, q5, [x11, #-0x20]
100ba42bc:     	ldp	q6, q7, [x11], #0x40
100ba42c0:     	orn.16b	v0, v0, v4
100ba42c4:     	orn.16b	v1, v1, v5
100ba42c8:     	orn.16b	v2, v2, v6
100ba42cc:     	orn.16b	v3, v3, v7
100ba42d0:     	stp	q0, q1, [x10, #-0x20]
100ba42d4:     	stp	q2, q3, [x10], #0x40
100ba42d8:     	subs	x12, x12, #0x8
100ba42dc:     	b.ne	0x100ba42b0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x57c>
100ba42e0:     	b	0x100ba44ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b8>
100ba42e4:     	and	x8, x19, #0xffffffffffffff8
100ba42e8:     	add	x9, x22, #0x20
100ba42ec:     	add	x10, x0, #0x20
100ba42f0:     	add	x11, x21, #0x20
100ba42f4:     	and	x12, x19, #0xffffffffffffff8
100ba42f8:     	ldp	q0, q1, [x9, #-0x20]
100ba42fc:     	ldp	q2, q3, [x9], #0x40
100ba4300:     	ldp	q4, q5, [x11, #-0x20]
100ba4304:     	ldp	q6, q7, [x11], #0x40
100ba4308:     	eor.16b	v0, v4, v0
100ba430c:     	eor.16b	v1, v5, v1
100ba4310:     	eor.16b	v2, v6, v2
100ba4314:     	eor.16b	v3, v7, v3
100ba4318:     	stp	q0, q1, [x10, #-0x20]
100ba431c:     	stp	q2, q3, [x10], #0x40
100ba4320:     	subs	x12, x12, #0x8
100ba4324:     	b.ne	0x100ba42f8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x5c4>
100ba4328:     	b	0x100ba450c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d8>
100ba432c:     	and	x8, x19, #0xffffffffffffff8
100ba4330:     	add	x9, x22, #0x20
100ba4334:     	add	x10, x0, #0x20
100ba4338:     	add	x11, x21, #0x20
100ba433c:     	and	x12, x19, #0xffffffffffffff8
100ba4340:     	ldp	q0, q1, [x9, #-0x20]
100ba4344:     	ldp	q2, q3, [x9], #0x40
100ba4348:     	ldp	q4, q5, [x11, #-0x20]
100ba434c:     	ldp	q6, q7, [x11], #0x40
100ba4350:     	and.16b	v0, v4, v0
100ba4354:     	and.16b	v1, v5, v1
100ba4358:     	and.16b	v2, v6, v2
100ba435c:     	and.16b	v3, v7, v3
100ba4360:     	stp	q0, q1, [x10, #-0x20]
100ba4364:     	stp	q2, q3, [x10], #0x40
100ba4368:     	subs	x12, x12, #0x8
100ba436c:     	b.ne	0x100ba4340 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x60c>
100ba4370:     	b	0x100ba452c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7f8>
100ba4374:     	adrp	x5, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba4378:     	add	x5, x5, #0x868
100ba437c:     	mov	x1, sp
100ba4380:     	add	x2, sp, #0x8
100ba4384:     	mov	w0, #0x0                ; =0
100ba4388:     	mov	x3, #0x0                ; =0
100ba438c:     	bl	0x101104c30 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100ba4390:     	adrp	x0, 0x10125e000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xdc8>
100ba4394:     	add	x0, x0, #0x9f9
100ba4398:     	adrp	x2, 0x10137b000 <dyld_stub_binder+0x10137b000>
100ba439c:     	add	x2, x2, #0x880
100ba43a0:     	mov	w1, #0x28               ; =40
100ba43a4:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100ba43a8:     	mov	w0, #0x8                ; =8
100ba43ac:     	mov	x1, x23
100ba43b0:     	bl	0x101104564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba43b4:     	mov	w0, #0x8                ; =8
100ba43b8:     	mov	x1, x21
100ba43bc:     	bl	0x101104564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba43c0:     	mov	w0, #0x8                ; =8
100ba43c4:     	mov	x1, x22
100ba43c8:     	bl	0x101104564 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100ba43cc:     	cmp	x19, x8
100ba43d0:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba43d4:     	ldr	x9, [x22, x8, lsl #3]
100ba43d8:     	ldr	x10, [x21, x8, lsl #3]
100ba43dc:     	orr	x9, x10, x9
100ba43e0:     	str	x9, [x0, x8, lsl #3]
100ba43e4:     	add	x8, x8, #0x1
100ba43e8:     	b	0x100ba43cc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x698>
100ba43ec:     	cmp	x19, x8
100ba43f0:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba43f4:     	ldr	x9, [x22, x8, lsl #3]
100ba43f8:     	ldr	x10, [x21, x8, lsl #3]
100ba43fc:     	orn	x9, x10, x9
100ba4400:     	str	x9, [x0, x8, lsl #3]
100ba4404:     	add	x8, x8, #0x1
100ba4408:     	b	0x100ba43ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6b8>
100ba440c:     	cmp	x19, x8
100ba4410:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4414:     	ldr	x9, [x22, x8, lsl #3]
100ba4418:     	ldr	x10, [x21, x8, lsl #3]
100ba441c:     	bic	x9, x9, x10
100ba4420:     	str	x9, [x0, x8, lsl #3]
100ba4424:     	add	x8, x8, #0x1
100ba4428:     	b	0x100ba440c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6d8>
100ba442c:     	cmp	x19, x8
100ba4430:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4434:     	ldr	x9, [x22, x8, lsl #3]
100ba4438:     	ldr	x10, [x21, x8, lsl #3]
100ba443c:     	eon	x9, x9, x10
100ba4440:     	str	x9, [x0, x8, lsl #3]
100ba4444:     	add	x8, x8, #0x1
100ba4448:     	b	0x100ba442c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x6f8>
100ba444c:     	cmp	x19, x8
100ba4450:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4454:     	ldr	x9, [x22, x8, lsl #3]
100ba4458:     	ldr	x10, [x21, x8, lsl #3]
100ba445c:     	bic	x9, x10, x9
100ba4460:     	str	x9, [x0, x8, lsl #3]
100ba4464:     	add	x8, x8, #0x1
100ba4468:     	b	0x100ba444c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x718>
100ba446c:     	cmp	x19, x8
100ba4470:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4474:     	ldr	x9, [x22, x8, lsl #3]
100ba4478:     	mvn	x9, x9
100ba447c:     	str	x9, [x0, x8, lsl #3]
100ba4480:     	add	x8, x8, #0x1
100ba4484:     	b	0x100ba446c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x738>
100ba4488:     	cmp	x19, x8
100ba448c:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4490:     	ldr	x9, [x22, x8, lsl #3]
100ba4494:     	ldr	x10, [x21, x8, lsl #3]
100ba4498:     	and	x9, x10, x9
100ba449c:     	mvn	x9, x9
100ba44a0:     	str	x9, [x0, x8, lsl #3]
100ba44a4:     	add	x8, x8, #0x1
100ba44a8:     	b	0x100ba4488 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x754>
100ba44ac:     	cmp	x19, x8
100ba44b0:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba44b4:     	ldr	x9, [x22, x8, lsl #3]
100ba44b8:     	ldr	x10, [x21, x8, lsl #3]
100ba44bc:     	orr	x9, x10, x9
100ba44c0:     	mvn	x9, x9
100ba44c4:     	str	x9, [x0, x8, lsl #3]
100ba44c8:     	add	x8, x8, #0x1
100ba44cc:     	b	0x100ba44ac <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x778>
100ba44d0:     	cmp	x19, x8
100ba44d4:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba44d8:     	ldr	x9, [x21, x8, lsl #3]
100ba44dc:     	mvn	x9, x9
100ba44e0:     	str	x9, [x0, x8, lsl #3]
100ba44e4:     	add	x8, x8, #0x1
100ba44e8:     	b	0x100ba44d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x79c>
100ba44ec:     	cmp	x19, x8
100ba44f0:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba44f4:     	ldr	x9, [x22, x8, lsl #3]
100ba44f8:     	ldr	x10, [x21, x8, lsl #3]
100ba44fc:     	orn	x9, x9, x10
100ba4500:     	str	x9, [x0, x8, lsl #3]
100ba4504:     	add	x8, x8, #0x1
100ba4508:     	b	0x100ba44ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7b8>
100ba450c:     	cmp	x19, x8
100ba4510:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4514:     	ldr	x9, [x22, x8, lsl #3]
100ba4518:     	ldr	x10, [x21, x8, lsl #3]
100ba451c:     	eor	x9, x10, x9
100ba4520:     	str	x9, [x0, x8, lsl #3]
100ba4524:     	add	x8, x8, #0x1
100ba4528:     	b	0x100ba450c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7d8>
100ba452c:     	cmp	x19, x8
100ba4530:     	b.eq	0x100ba3fdc <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x2a8>
100ba4534:     	ldr	x9, [x22, x8, lsl #3]
100ba4538:     	ldr	x10, [x21, x8, lsl #3]
100ba453c:     	and	x9, x10, x9
100ba4540:     	str	x9, [x0, x8, lsl #3]
100ba4544:     	add	x8, x8, #0x1
100ba4548:     	b	0x100ba452c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7combine+0x7f8>
100ba454c:     	nop
100ba4550:     	nop
100ba4554:     	nop
100ba4558:     	nop
100ba455c:     	nop
100ba4560:     	nop
100ba4564:     	nop
100ba4568:     	nop
100ba456c:     	nop
100ba4570:     	nop
100ba4574:     	nop
100ba4578:     	nop
100ba457c:     	nop
