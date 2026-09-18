
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100c7f270 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert>:
100c7f270:     	sub	sp, sp, #0xc0
100c7f274:     	stp	d9, d8, [sp, #0x50]
100c7f278:     	stp	x28, x27, [sp, #0x60]
100c7f27c:     	stp	x26, x25, [sp, #0x70]
100c7f280:     	stp	x24, x23, [sp, #0x80]
100c7f284:     	stp	x22, x21, [sp, #0x90]
100c7f288:     	stp	x20, x19, [sp, #0xa0]
100c7f28c:     	stp	x29, x30, [sp, #0xb0]
100c7f290:     	add	x29, sp, #0xb0
100c7f294:     	mov	x20, x1
100c7f298:     	mov	x19, x0
100c7f29c:     	ldr	x8, [x0]
100c7f2a0:     	cmn	x8, #0x1
100c7f2a4:     	b.eq	0x100c7f358 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0xe8>
100c7f2a8:     	ldr	x26, [x19, #0x10]
100c7f2ac:     	mov	w8, #0x8480             ; =33920
100c7f2b0:     	movk	w8, #0x1e, lsl #16
100c7f2b4:     	cmp	x26, x8
100c7f2b8:     	b.hs	0x100c7f8a0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x630>
100c7f2bc:     	ldr	x1, [x19, #0x68]
100c7f2c0:     	mov	x0, x20
100c7f2c4:     	bl	0x100c7ea1c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store11fingerprint>
100c7f2c8:     	mov	x21, x0
100c7f2cc:     	ldr	x25, [x20]
100c7f2d0:     	eor	x8, x25, #0x8000000000000000
100c7f2d4:     	cmp	x25, #0x0
100c7f2d8:     	csinc	x8, x8, xzr, mi
100c7f2dc:     	cmp	x8, #0x1
100c7f2e0:     	b.eq	0x100c7f3bc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x14c>
100c7f2e4:     	cmp	x8, #0x2
100c7f2e8:     	b.ne	0x100c7f8c8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x658>
100c7f2ec:     	ldr	w8, [x20, #0x8]
100c7f2f0:     	adrp	x2, 0x1014fc000 <dyld_stub_binder+0x1014fc000>
100c7f2f4:     	add	x2, x2, #0x18
100c7f2f8:     	adrp	x0, 0x101340000 <dyld_stub_binder+0x101340000>
100c7f2fc:     	add	x0, x0, #0xf06
100c7f300:     	mov	w1, #0x5f               ; =95
100c7f304:     	cmp	w8, #0x3e
100c7f308:     	b.hi	0x100c7f8f4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x684>
100c7f30c:     	ldr	w9, [x20, #0xc]
100c7f310:     	lsr	w10, w9, #28
100c7f314:     	cbnz	w10, 0x100c7f8f4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x684>
100c7f318:     	ldr	w10, [x20, #0x10]
100c7f31c:     	lsr	w11, w10, #28
100c7f320:     	cbnz	w11, 0x100c7f8f4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x684>
100c7f324:     	adrp	x2, 0x1014fc000 <dyld_stub_binder+0x1014fc000>
100c7f328:     	add	x2, x2, #0x30
100c7f32c:     	mov	w1, #0x37               ; =55
100c7f330:     	adrp	x0, 0x101340000 <dyld_stub_binder+0x101340000>
100c7f334:     	add	x0, x0, #0xf65
100c7f338:     	cmp	w26, w9, lsr #1
100c7f33c:     	b.ls	0x100c7f8f4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x684>
100c7f340:     	lsr	w11, w10, #1
100c7f344:     	cmp	w11, w26
100c7f348:     	b.hs	0x100c7f8f4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x684>
100c7f34c:     	orr	x8, x9, x8, lsl #56
100c7f350:     	orr	x23, x8, x10, lsl #28
100c7f354:     	b	0x100c7f448 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x1d8>
100c7f358:     	ldr	x27, [x19, #0x18]
100c7f35c:     	mov	w8, #0x8480             ; =33920
100c7f360:     	movk	w8, #0x1e, lsl #16
100c7f364:     	cmp	x27, x8
100c7f368:     	b.hs	0x100c7f8a0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x630>
100c7f36c:     	ldr	x28, [x20, #0x18]
100c7f370:     	ldr	x25, [x20]
100c7f374:     	eor	x8, x25, #0x8000000000000000
100c7f378:     	cmp	x25, #0x0
100c7f37c:     	csinc	x26, x8, xzr, mi
100c7f380:     	cbz	x26, 0x100c7f4cc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x25c>
100c7f384:     	cmp	x26, #0x1
100c7f388:     	b.ne	0x100c7f4e8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x278>
100c7f38c:     	ldr	x24, [x20, #0x10]
100c7f390:     	cbz	x24, 0x100c7f510 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2a0>
100c7f394:     	ldr	x23, [x20, #0x8]
100c7f398:     	lsl	x22, x24, #3
100c7f39c:     	mov	x0, x22
100c7f3a0:     	bl	0x101284d84 <dyld_stub_binder+0x101284d84>
100c7f3a4:     	cbz	x0, 0x100c7f97c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x70c>
100c7f3a8:     	mov	x21, x0
100c7f3ac:     	mov	x1, x23
100c7f3b0:     	mov	x2, x22
100c7f3b4:     	bl	0x101284d9c <dyld_stub_binder+0x101284d9c>
100c7f3b8:     	b	0x100c7f514 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2a4>
100c7f3bc:     	ldr	d0, [x20, #0x18]
100c7f3c0:     	cnt.8b	v0, v0
100c7f3c4:     	addv.8b	b0, v0
100c7f3c8:     	fmov	x8, d0
100c7f3cc:     	sub	w9, w8, #0x1
100c7f3d0:     	cmp	w9, #0xb
100c7f3d4:     	b.hi	0x100c7f8e0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x670>
100c7f3d8:     	ldr	x22, [x20, #0x10]
100c7f3dc:     	and	x9, x8, #0x3f
100c7f3e0:     	mov	w10, #0x1               ; =1
100c7f3e4:     	lsl	x8, x10, x8
100c7f3e8:     	lsr	x8, x8, #6
100c7f3ec:     	cmp	x9, #0x6
100c7f3f0:     	cinc	x8, x8, lo
100c7f3f4:     	stp	x22, x8, [sp, #0x40]
100c7f3f8:     	cmp	x22, x8
100c7f3fc:     	b.ne	0x100c7f8fc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x68c>
100c7f400:     	ldr	x23, [x19, #0x28]
100c7f404:     	add	x27, x23, x22
100c7f408:     	lsr	x8, x27, #32
100c7f40c:     	cbnz	x8, 0x100c7f91c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x6ac>
100c7f410:     	ldr	x24, [x20, #0x8]
100c7f414:     	ldur	x8, [x19, #0x18]
100c7f418:     	sub	x8, x8, x23
100c7f41c:     	cmp	x22, x8
100c7f420:     	b.hi	0x100c7f938 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x6c8>
100c7f424:     	mov	x8, x23
100c7f428:     	cbz	x22, 0x100c7f440 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x1d0>
100c7f42c:     	ldr	x9, [x19, #0x20]
100c7f430:     	add	x0, x9, x8, lsl #3
100c7f434:     	lsl	x2, x22, #3
100c7f438:     	mov	x1, x24
100c7f43c:     	bl	0x101284d9c <dyld_stub_binder+0x101284d9c>
100c7f440:     	str	x27, [x19, #0x28]
100c7f444:     	orr	x23, x23, #0x8000000000000000
100c7f448:     	lsl	w22, w26, #1
100c7f44c:     	add	x0, x19, #0x48
100c7f450:     	mov	x1, x21
100c7f454:     	mov	x2, x22
100c7f458:     	bl	0x100c28780 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapymNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100c7f45c:     	tst	w0, #0x1
100c7f460:     	csinv	w21, w1, wzr, ne
100c7f464:     	ldr	x26, [x20, #0x18]
100c7f468:     	ldr	x24, [x19, #0x10]
100c7f46c:     	ldr	x8, [x19]
100c7f470:     	cmp	x24, x8
100c7f474:     	b.ne	0x100c7f480 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x210>
100c7f478:     	mov	x0, x19
100c7f47c:     	bl	0x1012162b0 <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecAyj2_E8grow_oneCs23EhFSy3h49_8bumbledb>
100c7f480:     	mov	x0, x19
100c7f484:     	ldr	x8, [x0, #0x30]!
100c7f488:     	ldur	x9, [x0, #-0x28]
100c7f48c:     	add	x9, x9, x24, lsl #4
100c7f490:     	stp	x26, x23, [x9]
100c7f494:     	add	x9, x24, #0x1
100c7f498:     	stur	x9, [x0, #-0x20]
100c7f49c:     	ldr	x23, [x0, #0x10]
100c7f4a0:     	cmp	x23, x8
100c7f4a4:     	b.ne	0x100c7f4ac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x23c>
100c7f4a8:     	bl	0x10127d69c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100c7f4ac:     	ldr	x8, [x19, #0x38]
100c7f4b0:     	str	w21, [x8, x23, lsl #2]
100c7f4b4:     	add	x8, x23, #0x1
100c7f4b8:     	str	x8, [x19, #0x40]
100c7f4bc:     	cmp	x25, #0x1
100c7f4c0:     	b.lt	0x100c7f820 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5b0>
100c7f4c4:     	ldr	x0, [x20, #0x8]
100c7f4c8:     	b	0x100c7f81c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5ac>
100c7f4cc:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
100c7f4d0:     	mov	w9, w21
100c7f4d4:     	orr	x21, x9, x26, lsl #32
100c7f4d8:     	ldur	x8, [x19, #0x8]
100c7f4dc:     	cmp	x27, x8
100c7f4e0:     	b.eq	0x100c7f534 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2c4>
100c7f4e4:     	b	0x100c7f53c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2cc>
100c7f4e8:     	ldp	w21, w8, [x20, #0x8]
100c7f4ec:     	mov	x24, #0x2               ; =2
100c7f4f0:     	movk	x24, #0x8000, lsl #48
100c7f4f4:     	ldr	w23, [x20, #0x10]
100c7f4f8:     	mov	w9, w21
100c7f4fc:     	orr	x21, x9, x8, lsl #32
100c7f500:     	ldur	x8, [x19, #0x8]
100c7f504:     	cmp	x27, x8
100c7f508:     	b.eq	0x100c7f534 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2c4>
100c7f50c:     	b	0x100c7f53c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2cc>
100c7f510:     	mov	w21, #0x8               ; =8
100c7f514:     	lsr	x8, x21, #32
100c7f518:     	lsr	x22, x24, #32
100c7f51c:     	mov	x23, x24
100c7f520:     	mov	w9, w21
100c7f524:     	orr	x21, x9, x8, lsl #32
100c7f528:     	ldur	x8, [x19, #0x8]
100c7f52c:     	cmp	x27, x8
100c7f530:     	b.ne	0x100c7f53c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x2cc>
100c7f534:     	add	x0, x19, #0x8
100c7f538:     	bl	0x1012161e8 <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecAjj4_E8grow_oneCs23EhFSy3h49_8bumbledb>
100c7f53c:     	mov	x8, #0xa9c5             ; =43461
100c7f540:     	movk	x8, #0x2e62, lsl #16
100c7f544:     	movk	x8, #0x7aea, lsl #32
100c7f548:     	movk	x8, #0xf135, lsl #48
100c7f54c:     	ldr	x9, [x19, #0x10]
100c7f550:     	add	x9, x9, x27, lsl #5
100c7f554:     	stp	x24, x21, [x9]
100c7f558:     	stp	w23, w22, [x9, #0x10]
100c7f55c:     	str	x28, [x9, #0x18]
100c7f560:     	add	x9, x27, #0x1
100c7f564:     	str	x9, [x19, #0x18]
100c7f568:     	ldp	x21, x23, [x20, #0x8]
100c7f56c:     	madd	x9, x28, x8, x26
100c7f570:     	mul	x20, x9, x8
100c7f574:     	lsl	x22, x23, #3
100c7f578:     	cbz	x26, 0x100c7f65c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x3ec>
100c7f57c:     	cmp	x26, #0x1
100c7f580:     	b.ne	0x100c7f5d0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x360>
100c7f584:     	mov	x11, #0x8d3             ; =2259
100c7f588:     	movk	x11, #0x85a3, lsl #16
100c7f58c:     	movk	x11, #0x6a88, lsl #32
100c7f590:     	movk	x11, #0x243f, lsl #48
100c7f594:     	mov	x10, #0x7344            ; =29508
100c7f598:     	movk	x10, #0x370, lsl #16
100c7f59c:     	movk	x10, #0x8a2e, lsl #32
100c7f5a0:     	movk	x10, #0x1319, lsl #48
100c7f5a4:     	add	x9, x23, x20
100c7f5a8:     	mul	x9, x9, x8
100c7f5ac:     	cmp	x23, #0x3
100c7f5b0:     	b.hs	0x100c7f5e8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x378>
100c7f5b4:     	cbz	x23, 0x100c7f644 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x3d4>
100c7f5b8:     	ldr	x12, [x21]
100c7f5bc:     	eor	x11, x12, x11
100c7f5c0:     	add	x12, x21, x22
100c7f5c4:     	ldur	x12, [x12, #-0x8]
100c7f5c8:     	eor	x10, x12, x10
100c7f5cc:     	b	0x100c7f644 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x3d4>
100c7f5d0:     	add	x9, x20, w21, uxtw
100c7f5d4:     	mul	x9, x9, x8
100c7f5d8:     	add	x9, x9, x21, lsr #32
100c7f5dc:     	mul	x9, x9, x8
100c7f5e0:     	add	x9, x9, w23, uxtw
100c7f5e4:     	b	0x100c7f658 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x3e8>
100c7f5e8:     	mov	x13, #0x0               ; =0
100c7f5ec:     	sub	x12, x22, #0x10
100c7f5f0:     	add	x14, x21, #0x8
100c7f5f4:     	mov	x15, #0x31d0            ; =12752
100c7f5f8:     	movk	x15, #0x299f, lsl #16
100c7f5fc:     	movk	x15, #0x3822, lsl #32
100c7f600:     	movk	x15, #0xa409, lsl #48
100c7f604:     	mov	x16, x10
100c7f608:     	add	x13, x13, #0x10
100c7f60c:     	ldp	x10, x17, [x14, #-0x8]
100c7f610:     	eor	x10, x10, x11
100c7f614:     	eor	x11, x17, x15
100c7f618:     	mul	x17, x11, x10
100c7f61c:     	umulh	x10, x11, x10
100c7f620:     	eor	x10, x10, x17
100c7f624:     	add	x14, x14, #0x10
100c7f628:     	mov	x11, x16
100c7f62c:     	cmp	x13, x12
100c7f630:     	b.lo	0x100c7f604 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x394>
100c7f634:     	add	x11, x21, x12
100c7f638:     	ldp	x12, x13, [x11]
100c7f63c:     	eor	x11, x12, x16
100c7f640:     	eor	x10, x13, x10
100c7f644:     	mul	x12, x11, x10
100c7f648:     	umulh	x10, x11, x10
100c7f64c:     	eor	x10, x10, x12
100c7f650:     	eor	x10, x22, x10
100c7f654:     	add	x9, x10, x9
100c7f658:     	mul	x20, x9, x8
100c7f65c:     	stp	x25, x28, [sp, #0x30]
100c7f660:     	ldr	x8, [x19, #0x30]
100c7f664:     	cbz	x8, 0x100c7f8b8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x648>
100c7f668:     	str	x22, [sp, #0x18]
100c7f66c:     	mov	x25, #0x0               ; =0
100c7f670:     	mov	x11, #0x0               ; =0
100c7f674:     	lsl	w22, w27, #1
100c7f678:     	ror	x8, x20, #0x2c
100c7f67c:     	ldp	x24, x12, [x19, #0x20]
100c7f680:     	lsr	x10, x8, #57
100c7f684:     	dup.8b	v8, w10
100c7f688:     	lsr	x28, x21, #32
100c7f68c:     	movi.2d	v1, #0xffffffffffffffff
100c7f690:     	mov	w15, #0x28              ; =40
100c7f694:     	and	x17, x8, x12
100c7f698:     	ldr	d9, [x24, x17]
100c7f69c:     	cmeq.8b	v0, v9, v8
100c7f6a0:     	fmov	x8, d0
100c7f6a4:     	ands	x1, x8, #0x8080808080808080
100c7f6a8:     	b.eq	0x100c7f7a4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x534>
100c7f6ac:     	stp	x23, x21, [sp, #0x20]
100c7f6b0:     	rbit	x8, x1
100c7f6b4:     	clz	x8, x8
100c7f6b8:     	add	x8, x17, x8, lsr #3
100c7f6bc:     	and	x8, x8, x12
100c7f6c0:     	mneg	x8, x8, x15
100c7f6c4:     	add	x23, x24, x8
100c7f6c8:     	ldur	x8, [x23, #-0x10]
100c7f6cc:     	ldr	x9, [sp, #0x38]
100c7f6d0:     	cmp	x9, x8
100c7f6d4:     	b.ne	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f6d8:     	ldur	x8, [x23, #-0x28]
100c7f6dc:     	eor	x9, x8, #0x8000000000000000
100c7f6e0:     	cmp	x8, #0x0
100c7f6e4:     	csinc	x8, x9, xzr, mi
100c7f6e8:     	cmp	x26, x8
100c7f6ec:     	b.ne	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f6f0:     	cmp	x26, #0x1
100c7f6f4:     	b.eq	0x100c7f730 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x4c0>
100c7f6f8:     	cmp	x26, #0x2
100c7f6fc:     	b.ne	0x100c7f808 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x598>
100c7f700:     	ldur	w8, [x23, #-0x20]
100c7f704:     	ldr	x9, [sp, #0x28]
100c7f708:     	cmp	w8, w9
100c7f70c:     	b.ne	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f710:     	ldur	w8, [x23, #-0x1c]
100c7f714:     	cmp	w8, w28
100c7f718:     	b.ne	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f71c:     	ldur	w8, [x23, #-0x18]
100c7f720:     	ldr	x9, [sp, #0x20]
100c7f724:     	cmp	w8, w9
100c7f728:     	b.eq	0x100c7f808 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x598>
100c7f72c:     	b	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f730:     	ldur	x8, [x23, #-0x18]
100c7f734:     	ldr	x9, [sp, #0x20]
100c7f738:     	cmp	x9, x8
100c7f73c:     	b.ne	0x100c7f794 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x524>
100c7f740:     	str	x1, [sp, #0x8]
100c7f744:     	ldur	x1, [x23, #-0x20]
100c7f748:     	ldr	x0, [sp, #0x28]
100c7f74c:     	ldr	x2, [sp, #0x18]
100c7f750:     	mov	x21, x10
100c7f754:     	mov	x20, x11
100c7f758:     	mov	x27, x26
100c7f75c:     	mov	x26, x12
100c7f760:     	str	w22, [sp, #0x14]
100c7f764:     	mov	x22, x17
100c7f768:     	bl	0x101284d90 <dyld_stub_binder+0x101284d90>
100c7f76c:     	ldr	x1, [sp, #0x8]
100c7f770:     	mov	x17, x22
100c7f774:     	ldr	w22, [sp, #0x14]
100c7f778:     	mov	w15, #0x28              ; =40
100c7f77c:     	movi.2d	v1, #0xffffffffffffffff
100c7f780:     	mov	x12, x26
100c7f784:     	mov	x26, x27
100c7f788:     	mov	x11, x20
100c7f78c:     	mov	x10, x21
100c7f790:     	cbz	w0, 0x100c7f808 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x598>
100c7f794:     	sub	x8, x1, #0x2
100c7f798:     	ands	x1, x8, x1
100c7f79c:     	ldp	x23, x21, [sp, #0x20]
100c7f7a0:     	b.ne	0x100c7f6b0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x440>
100c7f7a4:     	cmp	x25, #0x1
100c7f7a8:     	b.eq	0x100c7f7cc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x55c>
100c7f7ac:     	cmlt.8b	v0, v9, #0
100c7f7b0:     	fmov	x8, d0
100c7f7b4:     	cbz	x8, 0x100c7f7e0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x570>
100c7f7b8:     	rbit	x8, x8
100c7f7bc:     	clz	x8, x8
100c7f7c0:     	add	x8, x17, x8, lsr #3
100c7f7c4:     	and	x8, x8, x12
100c7f7c8:     	str	x8, [sp]
100c7f7cc:     	cmeq.8b	v0, v9, v1
100c7f7d0:     	fmov	x8, d0
100c7f7d4:     	cbnz	x8, 0x100c7f848 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5d8>
100c7f7d8:     	mov	w25, #0x1               ; =1
100c7f7dc:     	b	0x100c7f7e4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x574>
100c7f7e0:     	mov	x25, #0x0               ; =0
100c7f7e4:     	add	x11, x11, #0x8
100c7f7e8:     	add	x8, x11, x17
100c7f7ec:     	and	x17, x8, x12
100c7f7f0:     	ldr	d9, [x24, x17]
100c7f7f4:     	cmeq.8b	v0, v9, v8
100c7f7f8:     	fmov	x8, d0
100c7f7fc:     	ands	x1, x8, #0x8080808080808080
100c7f800:     	b.ne	0x100c7f6ac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x43c>
100c7f804:     	b	0x100c7f7a4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x534>
100c7f808:     	stur	w22, [x23, #-0x8]
100c7f80c:     	ldr	x8, [sp, #0x30]
100c7f810:     	cmp	x8, #0x1
100c7f814:     	b.lt	0x100c7f820 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5b0>
100c7f818:     	ldr	x0, [sp, #0x28]
100c7f81c:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100c7f820:     	mov	x0, x22
100c7f824:     	ldp	x29, x30, [sp, #0xb0]
100c7f828:     	ldp	x20, x19, [sp, #0xa0]
100c7f82c:     	ldp	x22, x21, [sp, #0x90]
100c7f830:     	ldp	x24, x23, [sp, #0x80]
100c7f834:     	ldp	x26, x25, [sp, #0x70]
100c7f838:     	ldp	x28, x27, [sp, #0x60]
100c7f83c:     	ldp	d9, d8, [sp, #0x50]
100c7f840:     	add	sp, sp, #0xc0
100c7f844:     	ret
100c7f848:     	ldr	x11, [sp]
100c7f84c:     	ldrsb	w8, [x24, x11]
100c7f850:     	tbz	w8, #0x1f, 0x100c7f95c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x6ec>
100c7f854:     	and	x8, x8, #0x1
100c7f858:     	sub	x9, x11, #0x8
100c7f85c:     	and	x9, x9, x12
100c7f860:     	strb	w10, [x24, x11]
100c7f864:     	add	x9, x24, x9
100c7f868:     	strb	w10, [x9, #0x8]
100c7f86c:     	ldr	q0, [x19, #0x30]
100c7f870:     	movi.2d	v1, #0xffffffffffffffff
100c7f874:     	mov.d	v1[0], x8
100c7f878:     	sub.2d	v0, v0, v1
100c7f87c:     	str	q0, [x19, #0x30]
100c7f880:     	mov	w8, #0x28               ; =40
100c7f884:     	mneg	x8, x11, x8
100c7f888:     	add	x8, x24, x8
100c7f88c:     	ldp	x11, x9, [sp, #0x30]
100c7f890:     	stp	x11, x21, [x8, #-0x28]
100c7f894:     	stp	x23, x9, [x8, #-0x18]
100c7f898:     	stur	w22, [x8, #-0x8]
100c7f89c:     	b	0x100c7f820 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5b0>
100c7f8a0:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f8a4:     	add	x2, x2, #0xfa0
100c7f8a8:     	mov	w1, #0x43               ; =67
100c7f8ac:     	adrp	x0, 0x10133f000 <dyld_stub_binder+0x10133f000>
100c7f8b0:     	add	x0, x0, #0x6d3
100c7f8b4:     	b	0x100c7f930 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x6c0>
100c7f8b8:     	add	x0, x19, #0x20
100c7f8bc:     	add	x1, x19, #0x40
100c7f8c0:     	bl	0x1012091a4 <__RINvMs6_NtCsbXQ38keiWF6_9hashbrown3rawINtB6_8RawTableTNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storage4NodemEE14reserve_rehashNCINvNtB8_3map11make_hasherBQ_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE0EBY_>
100c7f8c4:     	b	0x100c7f668 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x3f8>
100c7f8c8:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f8cc:     	add	x2, x2, #0xfb8
100c7f8d0:     	mov	w1, #0x49               ; =73
100c7f8d4:     	adrp	x0, 0x101340000 <dyld_stub_binder+0x101340000>
100c7f8d8:     	add	x0, x0, #0xe7f
100c7f8dc:     	b	0x100c7f930 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x6c0>
100c7f8e0:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f8e4:     	add	x2, x2, #0xfd0
100c7f8e8:     	mov	w1, #0x41               ; =65
100c7f8ec:     	adrp	x0, 0x101340000 <dyld_stub_binder+0x101340000>
100c7f8f0:     	add	x0, x0, #0xea3
100c7f8f4:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100c7f8f8:     	b	0x100c7f988 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x718>
100c7f8fc:     	adrp	x5, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f900:     	add	x5, x5, #0xfe8
100c7f904:     	add	x1, sp, #0x40
100c7f908:     	add	x2, sp, #0x48
100c7f90c:     	mov	w0, #0x0                ; =0
100c7f910:     	mov	x3, #0x0                ; =0
100c7f914:     	bl	0x10127c770 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
100c7f918:     	b	0x100c7f988 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x718>
100c7f91c:     	adrp	x2, 0x1014fc000 <dyld_stub_binder+0x1014fc000>
100c7f920:     	add	x2, x2, #0x0
100c7f924:     	mov	w1, #0x45               ; =69
100c7f928:     	adrp	x0, 0x101340000 <dyld_stub_binder+0x101340000>
100c7f92c:     	add	x0, x0, #0xee4
100c7f930:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100c7f934:     	b	0x100c7f988 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x718>
100c7f938:     	add	x0, x19, #0x18
100c7f93c:     	mov	x1, x23
100c7f940:     	mov	x2, x22
100c7f944:     	mov	w3, #0x8                ; =8
100c7f948:     	mov	w4, #0x8                ; =8
100c7f94c:     	bl	0x1012127c8 <__RINvNvMs2_NtCsaexw8v31UlU_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs23EhFSy3h49_8bumbledb>
100c7f950:     	ldr	x8, [x19, #0x28]
100c7f954:     	add	x27, x8, x22
100c7f958:     	b	0x100c7f42c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x1bc>
100c7f95c:     	ldr	d0, [x24]
100c7f960:     	cmlt.8b	v0, v0, #0
100c7f964:     	fmov	x8, d0
100c7f968:     	rbit	x8, x8
100c7f96c:     	clz	x8, x8
100c7f970:     	lsr	x11, x8, #3
100c7f974:     	ldrb	w8, [x24, x11]
100c7f978:     	b	0x100c7f854 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x5e4>
100c7f97c:     	mov	w0, #0x8                ; =8
100c7f980:     	mov	x1, x22
100c7f984:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100c7f988:     	brk	#0x1
100c7f98c:     	mov	x19, x0
100c7f990:     	ldr	x8, [sp, #0x30]
100c7f994:     	cmp	x8, #0x1
100c7f998:     	b.lt	0x100c7f9e4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x774>
100c7f99c:     	mov	x0, x21
100c7f9a0:     	b	0x100c7f9e0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x770>
100c7f9a4:     	mov	x19, x0
100c7f9a8:     	cmp	x24, #0x1
100c7f9ac:     	b.ge	0x100c7f9b8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x748>
100c7f9b0:     	mov	x0, x19
100c7f9b4:     	b	0x100c7f9cc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x75c>
100c7f9b8:     	mov	x0, x21
100c7f9bc:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100c7f9c0:     	mov	x0, x19
100c7f9c4:     	b	0x100c7f9cc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x75c>
100c7f9c8:     	ldr	x25, [x20]
100c7f9cc:     	cmp	x25, #0x1
100c7f9d0:     	b.lt	0x100c7f9e8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store6insert+0x778>
100c7f9d4:     	ldr	x8, [x20, #0x8]
100c7f9d8:     	mov	x19, x0
100c7f9dc:     	mov	x0, x8
100c7f9e0:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100c7f9e4:     	mov	x0, x19
100c7f9e8:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
100c7f9ec:     	nop
100c7f9f0:     	nop
100c7f9f4:     	nop
100c7f9f8:     	nop
100c7f9fc:     	nop
