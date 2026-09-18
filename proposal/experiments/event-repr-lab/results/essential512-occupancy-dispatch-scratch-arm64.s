
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a41280 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_>:
100a41280:     	sub	sp, sp, #0x1e0
100a41284:     	stp	x20, x19, [sp, #0x1c0]
100a41288:     	stp	x29, x30, [sp, #0x1d0]
100a4128c:     	add	x29, sp, #0x1d0
100a41290:     	mov	x8, x4
100a41294:     	mov	x9, x3
100a41298:     	mov	x19, x0
100a4129c:     	adrp	x10, 0x101544000 <dyld_stub_binder+0x101544000>
100a412a0:     	add	x10, x10, #0xc98
100a412a4:     	ldp	q0, q1, [x10]
100a412a8:     	stp	q0, q1, [sp]
100a412ac:     	strb	wzr, [sp, #0x70]
100a412b0:     	movi.2d	v0, #0000000000000000
100a412b4:     	stp	q0, q0, [sp, #0x50]
100a412b8:     	stp	q0, q0, [sp, #0x30]
100a412bc:     	str	q0, [sp, #0x20]
100a412c0:     	stp	xzr, xzr, [sp, #0x78]
100a412c4:     	str	w5, [sp, #0x88]
100a412c8:     	stp	xzr, xzr, [sp, #0x90]
100a412cc:     	str	w6, [sp, #0xa0]
100a412d0:     	stp	xzr, xzr, [sp, #0xa8]
100a412d4:     	str	w7, [sp, #0xb8]
100a412d8:     	ands	w10, w1, #0xff
100a412dc:     	b.eq	0x100a41308 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0x88>
100a412e0:     	cmp	w10, #0x1
100a412e4:     	b.ne	0x100a41328 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0xa8>
100a412e8:     	add	x3, sp, #0x78
100a412ec:     	mov	x4, sp
100a412f0:     	add	x5, sp, #0x20
100a412f4:     	mov	x0, x2
100a412f8:     	mov	x1, x9
100a412fc:     	mov	x2, x8
100a41300:     	bl	0x100b26054 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_>
100a41304:     	b	0x100a41368 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0xe8>
100a41308:     	add	x3, sp, #0x78
100a4130c:     	mov	x4, sp
100a41310:     	add	x5, sp, #0x20
100a41314:     	mov	x0, x2
100a41318:     	mov	x1, x9
100a4131c:     	mov	x2, x8
100a41320:     	bl	0x100b25780 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh0_Kj0_EB8_>
100a41324:     	b	0x100a41368 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0xe8>
100a41328:     	stp	q0, q0, [sp, #0x1a0]
100a4132c:     	stp	q0, q0, [sp, #0x180]
100a41330:     	stp	q0, q0, [sp, #0x160]
100a41334:     	stp	q0, q0, [sp, #0x140]
100a41338:     	stp	q0, q0, [sp, #0x120]
100a4133c:     	stp	q0, q0, [sp, #0x100]
100a41340:     	stp	q0, q0, [sp, #0xe0]
100a41344:     	stp	q0, q0, [sp, #0xc0]
100a41348:     	add	x3, sp, #0x78
100a4134c:     	mov	x4, sp
100a41350:     	add	x5, sp, #0x20
100a41354:     	add	x6, sp, #0xc0
100a41358:     	mov	x0, x2
100a4135c:     	mov	x1, x9
100a41360:     	mov	x2, x8
100a41364:     	bl	0x100b26998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh2_Kj8_EB8_>
100a41368:     	strb	w0, [sp, #0x70]
100a4136c:     	ldr	x9, [sp, #0x18]
100a41370:     	ldr	x8, [sp, #0x8]
100a41374:     	str	x9, [sp, #0x68]
100a41378:     	ldr	x9, [sp, #0x70]
100a4137c:     	str	x9, [x19, #0x50]
100a41380:     	ldp	q0, q1, [sp, #0x20]
100a41384:     	stp	q0, q1, [x19]
100a41388:     	ldp	q0, q1, [sp, #0x40]
100a4138c:     	stp	q0, q1, [x19, #0x20]
100a41390:     	ldr	q0, [sp, #0x60]
100a41394:     	str	q0, [x19, #0x40]
100a41398:     	cbz	x8, 0x100a413c0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0x140>
100a4139c:     	add	x9, x8, x8, lsl #2
100a413a0:     	lsl	x9, x9, #4
100a413a4:     	add	x8, x9, x8
100a413a8:     	cmn	x8, #0x59
100a413ac:     	b.eq	0x100a413c0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0x140>
100a413b0:     	ldr	x8, [sp]
100a413b4:     	sub	x8, x8, x9
100a413b8:     	sub	x0, x8, #0x50
100a413bc:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100a413c0:     	ldp	x29, x30, [sp, #0x1d0]
100a413c4:     	ldp	x20, x19, [sp, #0x1c0]
100a413c8:     	add	sp, sp, #0x1e0
100a413cc:     	ret
100a413d0:     	mov	x19, x0
100a413d4:     	ldr	x9, [sp, #0x8]
100a413d8:     	cbz	x9, 0x100a41400 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0x180>
100a413dc:     	add	x8, x9, x9, lsl #2
100a413e0:     	lsl	x8, x8, #4
100a413e4:     	add	x9, x8, x9
100a413e8:     	cmn	x9, #0x59
100a413ec:     	b.eq	0x100a41400 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_withKm9_EB6_+0x180>
100a413f0:     	ldr	x9, [sp]
100a413f4:     	sub	x8, x9, x8
100a413f8:     	sub	x0, x8, #0x50
100a413fc:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100a41400:     	mov	x0, x19
100a41404:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
