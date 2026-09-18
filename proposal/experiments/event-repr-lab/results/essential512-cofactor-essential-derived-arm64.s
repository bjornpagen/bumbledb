
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a22314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>:
100a22314:     	stp	x20, x19, [sp, #-0x20]!
100a22318:     	stp	x29, x30, [sp, #0x10]
100a2231c:     	add	x29, sp, #0x10
100a22320:     	ldr	w8, [x0, #0xa8]
100a22324:     	cmp	w2, w8
100a22328:     	b.hs	0x100a223c8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xb4>
100a2232c:     	lsr	w8, w1, #1
100a22330:     	ldr	x9, [x0, #0x28]
100a22334:     	cmp	x9, x8
100a22338:     	b.ls	0x100a223e0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xcc>
100a2233c:     	ldr	x9, [x0, #0x20]
100a22340:     	add	x8, x9, x8, lsl #5
100a22344:     	ldr	x9, [x8, #0x18]
100a22348:     	lsr	x9, x9, x2
100a2234c:     	tbz	w9, #0x0, 0x100a223b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xa4>
100a22350:     	ldrb	w9, [x0, #0xad]
100a22354:     	tbz	w9, #0x0, 0x100a22398 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x84>
100a22358:     	ldr	x9, [x8]
100a2235c:     	mov	x10, #0x2               ; =2
100a22360:     	movk	x10, #0x8000, lsl #48
100a22364:     	cmp	x9, x10
100a22368:     	b.ne	0x100a223a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x90>
100a2236c:     	ldr	w9, [x8, #0x8]
100a22370:     	cmp	w9, w2
100a22374:     	b.ne	0x100a223a4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x90>
100a22378:     	cmp	w3, #0x0
100a2237c:     	mov	w9, #0xc                ; =12
100a22380:     	mov	w10, #0x10              ; =16
100a22384:     	csel	x9, x10, x9, ne
100a22388:     	ldr	w8, [x8, x9]
100a2238c:     	and	w9, w1, #0x1
100a22390:     	eor	w1, w8, w9
100a22394:     	b	0x100a223b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xa4>
100a22398:     	ldp	x29, x30, [sp, #0x10]
100a2239c:     	ldp	x20, x19, [sp], #0x20
100a223a0:     	b	0x100a20940 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
100a223a4:     	mov	x19, x1
100a223a8:     	and	w1, w1, #0xfffffffe
100a223ac:     	bl	0x100a20940 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
100a223b0:     	and	w8, w19, #0x1
100a223b4:     	eor	w1, w0, w8
100a223b8:     	mov	x0, x1
100a223bc:     	ldp	x29, x30, [sp, #0x10]
100a223c0:     	ldp	x20, x19, [sp], #0x20
100a223c4:     	ret
100a223c8:     	adrp	x0, 0x1011c1000 <dyld_stub_binder+0x1011c1000>
100a223cc:     	add	x0, x0, #0x650
100a223d0:     	adrp	x2, 0x101373000 <dyld_stub_binder+0x101373000>
100a223d4:     	add	x2, x2, #0xfd8
100a223d8:     	mov	w1, #0x2c               ; =44
100a223dc:     	bl	0x101104d48 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a223e0:     	adrp	x2, 0x101339000 <dyld_stub_binder+0x101339000>
100a223e4:     	add	x2, x2, #0xba8
100a223e8:     	mov	x0, x8
100a223ec:     	mov	x1, x9
100a223f0:     	bl	0x101104d5c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
