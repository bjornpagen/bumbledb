
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d15210 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant>:
100d15210:     	stp	x24, x23, [sp, #-0x40]!
100d15214:     	stp	x22, x21, [sp, #0x10]
100d15218:     	stp	x20, x19, [sp, #0x20]
100d1521c:     	stp	x29, x30, [sp, #0x30]
100d15220:     	add	x29, sp, #0x30
100d15224:     	cmp	w2, #0x6
100d15228:     	b.hs	0x100d15264 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x54>
100d1522c:     	mov	w8, #0x1                ; =1
100d15230:     	lsl	w8, w8, w2
100d15234:     	lsl	x9, x1, #3
100d15238:     	adrp	x10, 0x101321000 <dyld_stub_binder+0x101321000>
100d1523c:     	add	x10, x10, #0x3b8
100d15240:     	cbz	x9, 0x100d152ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100d15244:     	ldr	x11, [x0], #0x8
100d15248:     	lsr	x12, x11, x8
100d1524c:     	eor	x11, x12, x11
100d15250:     	ldr	x12, [x10, w2, uxtw #3]
100d15254:     	sub	x9, x9, #0x8
100d15258:     	and	x11, x11, x12
100d1525c:     	cbz	x11, 0x100d15240 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x30>
100d15260:     	b	0x100d152d0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc0>
100d15264:     	add	w9, w2, #0x3a
100d15268:     	and	w8, w9, #0x3f
100d1526c:     	cmp	w8, #0x3f
100d15270:     	b.eq	0x100d15304 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xf4>
100d15274:     	mov	w10, #0x2               ; =2
100d15278:     	lsl	x19, x10, x9
100d1527c:     	mov	w9, #0x1                ; =1
100d15280:     	lsl	x8, x9, x8
100d15284:     	neg	x9, x19
100d15288:     	and	x9, x1, x9
100d1528c:     	cmp	x19, x8
100d15290:     	b.lo	0x100d152e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xd4>
100d15294:     	cmp	x19, x8, lsl #1
100d15298:     	b.ne	0x100d152d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xc8>
100d1529c:     	lsl	x20, x8, #3
100d152a0:     	lsl	x21, x19, #3
100d152a4:     	add	x22, x9, x19
100d152a8:     	sub	x22, x22, x19
100d152ac:     	cmp	x19, x22
100d152b0:     	b.hi	0x100d152ec <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xdc>
100d152b4:     	add	x23, x0, x21
100d152b8:     	add	x1, x0, x20
100d152bc:     	mov	x2, x20
100d152c0:     	bl	0x101284d90 <dyld_stub_binder+0x101284d90>
100d152c4:     	mov	x8, x0
100d152c8:     	mov	x0, x23
100d152cc:     	cbz	w8, 0x100d152a8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x98>
100d152d0:     	mov	w0, #0x0                ; =0
100d152d4:     	b	0x100d152f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100d152d8:     	cmp	x19, x9
100d152dc:     	cset	w0, hi
100d152e0:     	b	0x100d152f0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0xe0>
100d152e4:     	cmp	x19, x9
100d152e8:     	b.ls	0x100d1531c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels10irrelevant+0x10c>
100d152ec:     	mov	w0, #0x1                ; =1
100d152f0:     	ldp	x29, x30, [sp, #0x30]
100d152f4:     	ldp	x20, x19, [sp, #0x20]
100d152f8:     	ldp	x22, x21, [sp, #0x10]
100d152fc:     	ldp	x24, x23, [sp], #0x40
100d15300:     	ret
100d15304:     	adrp	x0, 0x10131f000 <dyld_stub_binder+0x10131f000>
100d15308:     	add	x0, x0, #0xaf5
100d1530c:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d15310:     	add	x2, x2, #0x388
100d15314:     	mov	w1, #0x37               ; =55
100d15318:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d1531c:     	adrp	x3, 0x1014bf000 <dyld_stub_binder+0x1014bf000>
100d15320:     	add	x3, x3, #0x2f8
100d15324:     	mov	x0, #0x0                ; =0
100d15328:     	mov	x1, x8
100d1532c:     	mov	x2, x19
100d15330:     	bl	0x10127c7d4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
