
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a41114 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_>:
100a41114:     	sub	sp, sp, #0x100
100a41118:     	stp	x20, x19, [sp, #0xe0]
100a4111c:     	stp	x29, x30, [sp, #0xf0]
100a41120:     	add	x29, sp, #0xf0
100a41124:     	mov	x8, x4
100a41128:     	mov	x9, x3
100a4112c:     	mov	x19, x0
100a41130:     	adrp	x10, 0x101544000 <dyld_stub_binder+0x101544000>
100a41134:     	add	x10, x10, #0xc98
100a41138:     	ldp	q0, q1, [x10]
100a4113c:     	stp	q0, q1, [sp]
100a41140:     	strb	wzr, [sp, #0x70]
100a41144:     	movi.2d	v0, #0000000000000000
100a41148:     	stp	q0, q0, [sp, #0x50]
100a4114c:     	stp	q0, q0, [sp, #0x30]
100a41150:     	str	q0, [sp, #0x20]
100a41154:     	stp	xzr, xzr, [sp, #0x78]
100a41158:     	str	w5, [sp, #0x88]
100a4115c:     	stp	xzr, xzr, [sp, #0x90]
100a41160:     	str	w6, [sp, #0xa0]
100a41164:     	stp	xzr, xzr, [sp, #0xa8]
100a41168:     	str	w7, [sp, #0xb8]
100a4116c:     	ands	w10, w1, #0xff
100a41170:     	b.eq	0x100a4119c <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0x88>
100a41174:     	cmp	w10, #0x1
100a41178:     	b.ne	0x100a411bc <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0xa8>
100a4117c:     	add	x3, sp, #0x78
100a41180:     	mov	x4, sp
100a41184:     	add	x5, sp, #0x20
100a41188:     	mov	x0, x2
100a4118c:     	mov	x1, x9
100a41190:     	mov	x2, x8
100a41194:     	bl	0x100b24294 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh1_Kj0_EB8_>
100a41198:     	b	0x100a411e0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0xcc>
100a4119c:     	add	x3, sp, #0x78
100a411a0:     	mov	x4, sp
100a411a4:     	add	x5, sp, #0x20
100a411a8:     	mov	x0, x2
100a411ac:     	mov	x1, x9
100a411b0:     	mov	x2, x8
100a411b4:     	bl	0x100b239c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh0_Kj0_EB8_>
100a411b8:     	b	0x100a411e0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0xcc>
100a411bc:     	stp	q0, q0, [x29, #-0x30]
100a411c0:     	add	x3, sp, #0x78
100a411c4:     	mov	x4, sp
100a411c8:     	add	x5, sp, #0x20
100a411cc:     	sub	x6, x29, #0x30
100a411d0:     	mov	x0, x2
100a411d4:     	mov	x1, x9
100a411d8:     	mov	x2, x8
100a411dc:     	bl	0x100b24bd8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm6_Kh2_Kj1_EB8_>
100a411e0:     	strb	w0, [sp, #0x70]
100a411e4:     	ldr	x9, [sp, #0x18]
100a411e8:     	ldr	x8, [sp, #0x8]
100a411ec:     	str	x9, [sp, #0x68]
100a411f0:     	ldr	x9, [sp, #0x70]
100a411f4:     	str	x9, [x19, #0x50]
100a411f8:     	ldp	q0, q1, [sp, #0x20]
100a411fc:     	stp	q0, q1, [x19]
100a41200:     	ldp	q0, q1, [sp, #0x40]
100a41204:     	stp	q0, q1, [x19, #0x20]
100a41208:     	ldr	q0, [sp, #0x60]
100a4120c:     	str	q0, [x19, #0x40]
100a41210:     	cbz	x8, 0x100a41238 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0x124>
100a41214:     	add	x9, x8, x8, lsl #2
100a41218:     	lsl	x9, x9, #4
100a4121c:     	add	x8, x9, x8
100a41220:     	cmn	x8, #0x59
100a41224:     	b.eq	0x100a41238 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0x124>
100a41228:     	ldr	x8, [sp]
100a4122c:     	sub	x8, x8, x9
100a41230:     	sub	x0, x8, #0x50
100a41234:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100a41238:     	ldp	x29, x30, [sp, #0xf0]
100a4123c:     	ldp	x20, x19, [sp, #0xe0]
100a41240:     	add	sp, sp, #0x100
100a41244:     	ret
100a41248:     	mov	x19, x0
100a4124c:     	ldr	x9, [sp, #0x8]
100a41250:     	cbz	x9, 0x100a41278 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0x164>
100a41254:     	add	x8, x9, x9, lsl #2
100a41258:     	lsl	x8, x8, #4
100a4125c:     	add	x9, x8, x9
100a41260:     	cmn	x9, #0x59
100a41264:     	b.eq	0x100a41278 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm6_EB6_+0x164>
100a41268:     	ldr	x9, [sp]
100a4126c:     	sub	x8, x9, x8
100a41270:     	sub	x0, x8, #0x50
100a41274:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100a41278:     	mov	x0, x19
100a4127c:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
