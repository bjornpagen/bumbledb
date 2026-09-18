
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010089f2b8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>:
10089f2b8:     	sub	sp, sp, #0x40
10089f2bc:     	stp	x20, x19, [sp, #0x20]
10089f2c0:     	stp	x29, x30, [sp, #0x30]
10089f2c4:     	add	x29, sp, #0x30
10089f2c8:     	ldr	w9, [x4, #0x10]
10089f2cc:     	cmp	w9, #0x2
10089f2d0:     	b.hs	0x10089f2dc <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x24>
10089f2d4:     	strb	w9, [x0, #0x8]
10089f2d8:     	b	0x10089f39c <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xe4>
10089f2dc:     	ldp	x12, x11, [x4]
10089f2e0:     	mov	x8, #0xa9c5             ; =43461
10089f2e4:     	movk	x8, #0x2e62, lsl #16
10089f2e8:     	movk	x8, #0x7aea, lsl #32
10089f2ec:     	movk	x8, #0xf135, lsl #48
10089f2f0:     	madd	x10, x9, x8, x12
10089f2f4:     	madd	x10, x10, x8, x11
10089f2f8:     	madd	x10, x10, x8, x5
10089f2fc:     	mul	x8, x10, x8
10089f300:     	sub	x10, x2, #0x1
10089f304:     	and	x8, x10, x8, ror #44
10089f308:     	cbz	x2, 0x10089f400 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x148>
10089f30c:     	add	x8, x1, x8, lsl #6
10089f310:     	ldrb	w10, [x8]
10089f314:     	cmp	w10, #0xff
10089f318:     	b.eq	0x10089f3e8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x130>
10089f31c:     	ldr	x13, [x8, #0x38]
10089f320:     	cmp	x13, x5
10089f324:     	b.ne	0x10089f3d0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
10089f328:     	ldr	x13, [x8, #0x20]
10089f32c:     	cmp	x13, x12
10089f330:     	b.ne	0x10089f3d0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
10089f334:     	ldr	x12, [x8, #0x28]
10089f338:     	cmp	x12, x11
10089f33c:     	b.ne	0x10089f3d0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
10089f340:     	ldr	w11, [x8, #0x30]
10089f344:     	cmp	w11, w9
10089f348:     	b.ne	0x10089f3d0 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x118>
10089f34c:     	cbz	w10, 0x10089f394 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xdc>
10089f350:     	cmp	w10, #0x1
10089f354:     	b.ne	0x10089f3b4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xfc>
10089f358:     	mov	x20, x0
10089f35c:     	ldr	w19, [x8, #0x4]
10089f360:     	mov	x0, sp
10089f364:     	add	x1, x3, #0x40
10089f368:     	mov	x2, x19
10089f36c:     	bl	0x100ee9280 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
10089f370:     	ldr	w8, [sp]
10089f374:     	cmp	w8, #0x1
10089f378:     	b.ne	0x10089f414 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0x15c>
10089f37c:     	ldp	x8, x9, [sp, #0x8]
10089f380:     	sbfx	x10, x19, #0, #1
10089f384:     	mov	x11, #-0x1              ; =-1
10089f388:     	stp	x11, x8, [x20]
10089f38c:     	stp	x9, x10, [x20, #0x10]
10089f390:     	b	0x10089f3a4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
10089f394:     	ldrb	w8, [x8, #0x1]
10089f398:     	strb	w8, [x0, #0x8]
10089f39c:     	mov	x8, #-0x2               ; =-2
10089f3a0:     	str	x8, [x0]
10089f3a4:     	ldp	x29, x30, [sp, #0x30]
10089f3a8:     	ldp	x20, x19, [sp, #0x20]
10089f3ac:     	add	sp, sp, #0x40
10089f3b0:     	ret
10089f3b4:     	ldr	q0, [x8, #0x10]
10089f3b8:     	ldr	x8, [x8, #0x8]
10089f3bc:     	mov	x9, #-0x1               ; =-1
10089f3c0:     	str	x9, [x0]
10089f3c4:     	stur	q0, [x0, #0x8]
10089f3c8:     	str	x8, [x0, #0x18]
10089f3cc:     	b	0x10089f3a4 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_+0xec>
10089f3d0:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
10089f3d4:     	add	x0, x0, #0x3c4
10089f3d8:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
10089f3dc:     	add	x2, x2, #0xd60
10089f3e0:     	mov	w1, #0x51               ; =81
10089f3e4:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
10089f3e8:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
10089f3ec:     	add	x0, x0, #0x3b6
10089f3f0:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
10089f3f4:     	add	x2, x2, #0xd48
10089f3f8:     	mov	w1, #0xe                ; =14
10089f3fc:     	bl	0x101506f24 <__RNvNtCs4sDCw1iE1MS_4core6option13expect_failed>
10089f400:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
10089f404:     	add	x2, x2, #0xd30
10089f408:     	mov	x0, x8
10089f40c:     	mov	x1, #0x0                ; =0
10089f410:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10089f414:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
10089f418:     	add	x0, x0, #0x3ec
10089f41c:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
10089f420:     	add	x2, x2, #0xd78
10089f424:     	mov	w1, #0xaf               ; =175
10089f428:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
