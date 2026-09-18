
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b9a1fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>:
100b9a1fc:     	sub	sp, sp, #0x50
100b9a200:     	stp	x22, x21, [sp, #0x20]
100b9a204:     	stp	x20, x19, [sp, #0x30]
100b9a208:     	stp	x29, x30, [sp, #0x40]
100b9a20c:     	add	x29, sp, #0x40
100b9a210:     	ldr	w8, [x0, #0xe0]
100b9a214:     	cmp	w2, w8
100b9a218:     	b.hs	0x100b9a324 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x128>
100b9a21c:     	mov	x19, x1
100b9a220:     	mov	x1, x0
100b9a224:     	ldr	x9, [x1, #0x30]!
100b9a228:     	lsr	w8, w19, #1
100b9a22c:     	cmn	x9, #0x1
100b9a230:     	b.eq	0x100b9a258 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x5c>
100b9a234:     	ldr	x9, [x0, #0x40]
100b9a238:     	cmp	x9, x8
100b9a23c:     	b.ls	0x100b9a33c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x140>
100b9a240:     	ldr	x9, [x0, #0x38]
100b9a244:     	add	x8, x9, x8, lsl #4
100b9a248:     	ldr	x8, [x8]
100b9a24c:     	lsr	x8, x8, x2
100b9a250:     	tbnz	w8, #0x0, 0x100b9a27c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x80>
100b9a254:     	b	0x100b9a30c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x110>
100b9a258:     	ldr	x9, [x0, #0x48]
100b9a25c:     	cmp	x9, x8
100b9a260:     	b.ls	0x100b9a350 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x154>
100b9a264:     	ldr	x9, [x0, #0x40]
100b9a268:     	add	x8, x9, x8, lsl #5
100b9a26c:     	add	x8, x8, #0x18
100b9a270:     	ldr	x8, [x8]
100b9a274:     	lsr	x8, x8, x2
100b9a278:     	tbz	w8, #0x0, 0x100b9a30c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x110>
100b9a27c:     	ldrb	w8, [x0, #0xe5]
100b9a280:     	tbz	w8, #0x0, 0x100b9a2dc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xe0>
100b9a284:     	mov	x21, x0
100b9a288:     	mov	x20, x3
100b9a28c:     	mov	x0, sp
100b9a290:     	mov	x22, x2
100b9a294:     	mov	x2, x19
100b9a298:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b9a29c:     	mov	x2, x22
100b9a2a0:     	ldr	w8, [sp]
100b9a2a4:     	cmp	w8, #0x2
100b9a2a8:     	b.ne	0x100b9a2f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xf8>
100b9a2ac:     	ldr	w8, [sp, #0x4]
100b9a2b0:     	cmp	w8, w2
100b9a2b4:     	b.ne	0x100b9a2f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xf8>
100b9a2b8:     	tst	w20, #0x1
100b9a2bc:     	mov	w8, #0x8                ; =8
100b9a2c0:     	mov	w9, #0xc                ; =12
100b9a2c4:     	csel	x8, x9, x8, ne
100b9a2c8:     	mov	x9, sp
100b9a2cc:     	ldr	w8, [x9, x8]
100b9a2d0:     	and	w9, w19, #0x1
100b9a2d4:     	eor	w19, w8, w9
100b9a2d8:     	b	0x100b9a30c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x110>
100b9a2dc:     	mov	x1, x19
100b9a2e0:     	ldp	x29, x30, [sp, #0x40]
100b9a2e4:     	ldp	x20, x19, [sp, #0x30]
100b9a2e8:     	ldp	x22, x21, [sp, #0x20]
100b9a2ec:     	add	sp, sp, #0x50
100b9a2f0:     	b	0x100b98800 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
100b9a2f4:     	and	w1, w19, #0xfffffffe
100b9a2f8:     	mov	x0, x21
100b9a2fc:     	mov	x3, x20
100b9a300:     	bl	0x100b98800 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E14cofactor_innerB6_>
100b9a304:     	and	w8, w19, #0x1
100b9a308:     	eor	w19, w0, w8
100b9a30c:     	mov	x0, x19
100b9a310:     	ldp	x29, x30, [sp, #0x40]
100b9a314:     	ldp	x20, x19, [sp, #0x30]
100b9a318:     	ldp	x22, x21, [sp, #0x20]
100b9a31c:     	add	sp, sp, #0x50
100b9a320:     	ret
100b9a324:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100b9a328:     	add	x0, x0, #0x60b
100b9a32c:     	adrp	x2, 0x1014f8000 <dyld_stub_binder+0x1014f8000>
100b9a330:     	add	x2, x2, #0x918
100b9a334:     	mov	w1, #0x2c               ; =44
100b9a338:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100b9a33c:     	adrp	x2, 0x1014fc000 <dyld_stub_binder+0x1014fc000>
100b9a340:     	add	x2, x2, #0x90
100b9a344:     	mov	x0, x8
100b9a348:     	mov	x1, x9
100b9a34c:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b9a350:     	adrp	x2, 0x1014fc000 <dyld_stub_binder+0x1014fc000>
100b9a354:     	add	x2, x2, #0x78
100b9a358:     	mov	x0, x8
100b9a35c:     	mov	x1, x9
100b9a360:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
