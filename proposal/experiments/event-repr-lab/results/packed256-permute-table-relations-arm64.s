
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010059f508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>:
10059f508:     	stp	x26, x25, [sp, #-0x50]!
10059f50c:     	stp	x24, x23, [sp, #0x10]
10059f510:     	stp	x22, x21, [sp, #0x20]
10059f514:     	stp	x20, x19, [sp, #0x30]
10059f518:     	stp	x29, x30, [sp, #0x40]
10059f51c:     	add	x29, sp, #0x40
10059f520:     	cmp	x3, #0x1
10059f524:     	b.ne	0x10059f544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x3c>
10059f528:     	lsr	x0, x2, #6
10059f52c:     	cmp	x2, #0x100
10059f530:     	b.hs	0x10059f630 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x128>
10059f534:     	ldr	x8, [x1, x0, lsl #3]
10059f538:     	lsr	x8, x8, x2
10059f53c:     	and	w22, w8, #0x1
10059f540:     	b	0x10059f5bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0xb4>
10059f544:     	mov	x21, x5
10059f548:     	mov	x20, x4
10059f54c:     	mov	x19, x0
10059f550:     	lsr	x23, x3, #1
10059f554:     	mov	x24, x1
10059f558:     	mov	x25, x2
10059f55c:     	mov	x26, x3
10059f560:     	mov	x3, x23
10059f564:     	bl	0x10059f508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>
10059f568:     	mov	x22, x0
10059f56c:     	add	x2, x23, x25
10059f570:     	mov	x0, x19
10059f574:     	mov	x1, x24
10059f578:     	mov	x3, x23
10059f57c:     	mov	x4, x20
10059f580:     	mov	x5, x21
10059f584:     	bl	0x10059f508 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>
10059f588:     	mov	x23, x0
10059f58c:     	rbit	x8, x26
10059f590:     	clz	x8, x8
10059f594:     	sub	x0, x8, #0x1
10059f598:     	ldr	x1, [x19, #0x28]
10059f59c:     	cmp	x0, x1
10059f5a0:     	b.hs	0x10059f640 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x138>
10059f5a4:     	ldr	x8, [x19, #0x20]
10059f5a8:     	ldr	w0, [x8, x0, lsl #2]
10059f5ac:     	cmp	x21, x0
10059f5b0:     	b.ls	0x10059f64c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x144>
10059f5b4:     	cmp	w22, w23
10059f5b8:     	b.ne	0x10059f5d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0xd0>
10059f5bc:     	mov	x0, x22
10059f5c0:     	ldp	x29, x30, [sp, #0x40]
10059f5c4:     	ldp	x20, x19, [sp, #0x30]
10059f5c8:     	ldp	x22, x21, [sp, #0x20]
10059f5cc:     	ldp	x24, x23, [sp, #0x10]
10059f5d0:     	ldp	x26, x25, [sp], #0x50
10059f5d4:     	ret
10059f5d8:     	ldr	w20, [x20, x0, lsl #2]
10059f5dc:     	mov	x0, x19
10059f5e0:     	mov	w1, #0x2                ; =2
10059f5e4:     	mov	x2, x20
10059f5e8:     	mov	x3, x22
10059f5ec:     	bl	0x1005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
10059f5f0:     	mov	x21, x0
10059f5f4:     	mov	x0, x19
10059f5f8:     	mov	w1, #0x8                ; =8
10059f5fc:     	mov	x2, x20
10059f600:     	mov	x3, x23
10059f604:     	bl	0x1005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
10059f608:     	mov	x3, x0
10059f60c:     	mov	x0, x19
10059f610:     	mov	w1, #0xe                ; =14
10059f614:     	mov	x2, x21
10059f618:     	ldp	x29, x30, [sp, #0x40]
10059f61c:     	ldp	x20, x19, [sp, #0x30]
10059f620:     	ldp	x22, x21, [sp, #0x20]
10059f624:     	ldp	x24, x23, [sp, #0x10]
10059f628:     	ldp	x26, x25, [sp], #0x50
10059f62c:     	b	0x1005a0400 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
10059f630:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059f634:     	add	x2, x2, #0x1a8
10059f638:     	mov	w1, #0x4                ; =4
10059f63c:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059f640:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059f644:     	add	x2, x2, #0x1c0
10059f648:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
10059f64c:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
10059f650:     	add	x2, x2, #0x1d8
10059f654:     	mov	x1, x21
10059f658:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
