
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>:
1011607a8:     	stp	x24, x23, [sp, #-0x40]!
1011607ac:     	stp	x22, x21, [sp, #0x10]
1011607b0:     	stp	x20, x19, [sp, #0x20]
1011607b4:     	stp	x29, x30, [sp, #0x30]
1011607b8:     	add	x29, sp, #0x30
1011607bc:     	mov	x19, x1
1011607c0:     	mov	x20, x0
1011607c4:     	ldr	x8, [x0, #0x40]
1011607c8:     	lsr	w0, w1, #1
1011607cc:     	cmn	x8, #0x1
1011607d0:     	b.eq	0x1011607fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x54>
1011607d4:     	ldr	x1, [x20, #0x50]
1011607d8:     	cmp	x1, x0
1011607dc:     	b.ls	0x101160a14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x26c>
1011607e0:     	ldr	x9, [x20, #0x48]
1011607e4:     	add	x9, x9, x0, lsl #4
1011607e8:     	ldr	x9, [x9]
1011607ec:     	ldr	x10, [x20, #0x180]
1011607f0:     	bics	xzr, x9, x10
1011607f4:     	b.ne	0x101160824 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x7c>
1011607f8:     	b	0x1011609f0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
1011607fc:     	ldr	x1, [x20, #0x58]
101160800:     	cmp	x1, x0
101160804:     	b.ls	0x101160a30 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x288>
101160808:     	ldr	x1, [x20, #0x50]
10116080c:     	add	x9, x1, x0, lsl #5
101160810:     	add	x9, x9, #0x18
101160814:     	ldr	x9, [x9]
101160818:     	ldr	x10, [x20, #0x180]
10116081c:     	bics	xzr, x9, x10
101160820:     	b.eq	0x1011609f0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
101160824:     	and	w9, w19, #0xfffffffe
101160828:     	ldr	x10, [x20, #0x148]
10116082c:     	cbz	x10, 0x1011608d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x128>
101160830:     	mov	x10, #0x0               ; =0
101160834:     	mov	x11, #0xa9c5            ; =43461
101160838:     	movk	x11, #0x2e62, lsl #16
10116083c:     	movk	x11, #0x7aea, lsl #32
101160840:     	movk	x11, #0xf135, lsl #48
101160844:     	mul	x11, x9, x11
101160848:     	ror	x13, x11, #0x2c
10116084c:     	lsr	x14, x13, #57
101160850:     	ldp	x12, x11, [x20, #0x130]
101160854:     	dup.8b	v0, w14
101160858:     	movi.2d	v1, #0xffffffffffffffff
10116085c:     	and	x13, x13, x11
101160860:     	ldr	d2, [x12, x13]
101160864:     	cmeq.8b	v3, v2, v0
101160868:     	fmov	x14, d3
10116086c:     	ands	x14, x14, #0x8080808080808080
101160870:     	b.eq	0x1011608a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xf8>
101160874:     	rbit	x15, x14
101160878:     	clz	x15, x15
10116087c:     	add	x15, x13, x15, lsr #3
101160880:     	and	x15, x15, x11
101160884:     	sub	x15, x12, x15, lsl #3
101160888:     	ldur	w16, [x15, #-0x8]
10116088c:     	cmp	w9, w16
101160890:     	b.eq	0x1011608ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x144>
101160894:     	sub	x15, x14, #0x2
101160898:     	ands	x14, x15, x14
10116089c:     	b.ne	0x101160874 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xcc>
1011608a0:     	cmeq.8b	v2, v2, v1
1011608a4:     	fmov	x14, d2
1011608a8:     	cbnz	x14, 0x1011608d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x128>
1011608ac:     	add	x10, x10, #0x8
1011608b0:     	add	x13, x13, x10
1011608b4:     	and	x13, x13, x11
1011608b8:     	ldr	d2, [x12, x13]
1011608bc:     	cmeq.8b	v3, v2, v0
1011608c0:     	fmov	x14, d3
1011608c4:     	ands	x14, x14, #0x8080808080808080
1011608c8:     	b.ne	0x101160874 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xcc>
1011608cc:     	b	0x1011608a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0xf8>
1011608d0:     	cmn	x8, #0x1
1011608d4:     	b.eq	0x1011608fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x154>
1011608d8:     	cmp	x1, x0
1011608dc:     	b.ls	0x101160a14 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x26c>
1011608e0:     	ldr	x8, [x20, #0x48]
1011608e4:     	add	x8, x8, x0, lsl #4
1011608e8:     	b	0x101160910 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x168>
1011608ec:     	ldur	w8, [x15, #-0x4]
1011608f0:     	and	w9, w19, #0x1
1011608f4:     	eor	w19, w8, w9
1011608f8:     	b	0x1011609f0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x248>
1011608fc:     	ldr	x8, [x20, #0x58]
101160900:     	cmp	x8, x0
101160904:     	b.ls	0x101160a3c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x294>
101160908:     	add	x8, x1, x0, lsl #5
10116090c:     	add	x8, x8, #0x18
101160910:     	ldr	x8, [x8]
101160914:     	ldp	x9, x10, [x20, #0x100]
101160918:     	lsl	x10, x10, #2
10116091c:     	cbz	x10, 0x101160a08 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x260>
101160920:     	ldr	w21, [x9], #0x4
101160924:     	lsr	x11, x8, x21
101160928:     	sub	x10, x10, #0x4
10116092c:     	tbz	w11, #0x0, 0x10116091c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x174>
101160930:     	and	w1, w19, #0xfffffffe
101160934:     	mov	x0, x20
101160938:     	mov	x2, x21
10116093c:     	mov	w3, #0x0                ; =0
101160940:     	bl	0x1011735bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
101160944:     	mov	x22, x0
101160948:     	and	w1, w19, #0xfffffffe
10116094c:     	mov	x0, x20
101160950:     	mov	x2, x21
101160954:     	mov	w3, #0x1                ; =1
101160958:     	bl	0x1011735bc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
10116095c:     	mov	x24, x0
101160960:     	mov	x0, x20
101160964:     	mov	x1, x22
101160968:     	bl	0x1011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
10116096c:     	mov	x23, x0
101160970:     	mov	x0, x20
101160974:     	mov	x1, x24
101160978:     	bl	0x1011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
10116097c:     	ldr	x1, [x20, #0x120]
101160980:     	cmp	x1, x21
101160984:     	b.ls	0x101160a20 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_+0x278>
101160988:     	mov	x22, x0
10116098c:     	ldr	x8, [x20, #0x118]
101160990:     	ldr	w21, [x8, x21, lsl #2]
101160994:     	mov	x0, x20
101160998:     	mov	w1, #0x4                ; =4
10116099c:     	mov	x2, x23
1011609a0:     	mov	x3, x21
1011609a4:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1011609a8:     	mov	x23, x0
1011609ac:     	mov	x0, x20
1011609b0:     	mov	w1, #0x8                ; =8
1011609b4:     	mov	x2, x22
1011609b8:     	mov	x3, x21
1011609bc:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1011609c0:     	mov	x3, x0
1011609c4:     	mov	x0, x20
1011609c8:     	mov	w1, #0xe                ; =14
1011609cc:     	mov	x2, x23
1011609d0:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1011609d4:     	mov	x21, x0
1011609d8:     	add	x0, x20, #0x130
1011609dc:     	and	w1, w19, #0xfffffffe
1011609e0:     	mov	x2, x21
1011609e4:     	bl	0x101220dac <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapmmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
1011609e8:     	and	w8, w19, #0x1
1011609ec:     	eor	w19, w21, w8
1011609f0:     	mov	x0, x19
1011609f4:     	ldp	x29, x30, [sp, #0x30]
1011609f8:     	ldp	x20, x19, [sp, #0x20]
1011609fc:     	ldp	x22, x21, [sp, #0x10]
101160a00:     	ldp	x24, x23, [sp], #0x40
101160a04:     	ret
101160a08:     	adrp	x0, 0x101b76000 <dyld_stub_binder+0x101b76000>
101160a0c:     	add	x0, x0, #0xc78
101160a10:     	bl	0x1018c1834 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
101160a14:     	adrp	x2, 0x101b7a000 <dyld_stub_binder+0x101b7a000>
101160a18:     	add	x2, x2, #0x8b8
101160a1c:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
101160a20:     	adrp	x2, 0x101b76000 <dyld_stub_binder+0x101b76000>
101160a24:     	add	x2, x2, #0xc90
101160a28:     	mov	x0, x21
101160a2c:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
101160a30:     	adrp	x2, 0x101b7a000 <dyld_stub_binder+0x101b7a000>
101160a34:     	add	x2, x2, #0x8a0
101160a38:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
101160a3c:     	adrp	x2, 0x101b7a000 <dyld_stub_binder+0x101b7a000>
101160a40:     	add	x2, x2, #0x8a0
101160a44:     	mov	x1, x8
101160a48:     	bl	0x1018c179c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
