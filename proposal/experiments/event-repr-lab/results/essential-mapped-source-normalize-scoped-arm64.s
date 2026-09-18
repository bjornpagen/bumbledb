
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100840634 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_>:
100840634:     	sub	sp, sp, #0x90
100840638:     	stp	x28, x27, [sp, #0x30]
10084063c:     	stp	x26, x25, [sp, #0x40]
100840640:     	stp	x24, x23, [sp, #0x50]
100840644:     	stp	x22, x21, [sp, #0x60]
100840648:     	stp	x20, x19, [sp, #0x70]
10084064c:     	stp	x29, x30, [sp, #0x80]
100840650:     	add	x29, sp, #0x80
100840654:     	mov	x24, x1
100840658:     	mov	x19, x0
10084065c:     	ldr	x23, [x1]
100840660:     	cbz	x23, 0x1008406a4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x70>
100840664:     	mov	x20, x3
100840668:     	mov	x22, x2
10084066c:     	ldr	x8, [x3, #0x70]
100840670:     	add	x8, x8, #0x1
100840674:     	str	x8, [x3, #0x70]
100840678:     	ldr	w21, [x24, #0x10]
10084067c:     	ldr	x8, [x2, #0x40]
100840680:     	lsr	x0, x21, #1
100840684:     	cmn	x8, #0x1
100840688:     	b.eq	0x1008406b8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x84>
10084068c:     	ldr	x1, [x22, #0x50]
100840690:     	cmp	x1, x0
100840694:     	b.ls	0x1008408e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
100840698:     	ldr	x8, [x22, #0x48]
10084069c:     	add	x8, x8, x0, lsl #4
1008406a0:     	b	0x1008406d0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x9c>
1008406a4:     	ldr	q0, [x24]
1008406a8:     	str	q0, [x19]
1008406ac:     	ldr	x8, [x24, #0x10]
1008406b0:     	str	x8, [x19, #0x10]
1008406b4:     	b	0x100840858 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x224>
1008406b8:     	ldr	x1, [x22, #0x58]
1008406bc:     	cmp	x1, x0
1008406c0:     	b.ls	0x1008408ec <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
1008406c4:     	ldr	x8, [x22, #0x50]
1008406c8:     	add	x8, x8, x0, lsl #5
1008406cc:     	add	x8, x8, #0x18
1008406d0:     	mov	x26, #0x0               ; =0
1008406d4:     	ldr	x8, [x8]
1008406d8:     	bic	x25, x8, x23
1008406dc:     	mov	w8, #0x4                ; =4
1008406e0:     	stp	xzr, x8, [sp, #0x18]
1008406e4:     	str	xzr, [sp, #0x28]
1008406e8:     	mov	w9, #0x1                ; =1
1008406ec:     	b	0x100840714 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xe0>
1008406f0:     	rbit	x9, x23
1008406f4:     	clz	x9, x9
1008406f8:     	str	w9, [x8, x26]
1008406fc:     	str	x28, [sp, #0x28]
100840700:     	sub	x10, x23, #0x1
100840704:     	add	x26, x26, #0x4
100840708:     	add	x9, x28, #0x1
10084070c:     	ands	x23, x10, x23
100840710:     	b.eq	0x100840738 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x104>
100840714:     	mov	x28, x9
100840718:     	sub	x9, x9, #0x1
10084071c:     	ldr	x10, [sp, #0x18]
100840720:     	cmp	x9, x10
100840724:     	b.ne	0x1008406f0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
100840728:     	add	x0, sp, #0x18
10084072c:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100840730:     	ldr	x8, [sp, #0x20]
100840734:     	b	0x1008406f0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
100840738:     	ldp	x8, x27, [sp, #0x18]
10084073c:     	str	x8, [sp, #0x10]
100840740:     	cbz	x28, 0x1008407ec <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
100840744:     	ldr	x28, [x24, #0x8]
100840748:     	ldr	x24, [x20, #0x78]
10084074c:     	mov	x23, x27
100840750:     	adrp	x8, 0x101799000 <dyld_stub_binder+0x101799000>
100840754:     	add	x8, x8, #0x5e0
100840758:     	str	x8, [sp, #0x8]
10084075c:     	b	0x100840768 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x134>
100840760:     	subs	x26, x26, #0x4
100840764:     	b.eq	0x1008407ec <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
100840768:     	ldr	w2, [x23], #0x4
10084076c:     	ldr	x8, [x22, #0x40]
100840770:     	lsr	w0, w21, #1
100840774:     	cmn	x8, #0x1
100840778:     	b.eq	0x1008407a0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x16c>
10084077c:     	ldr	x1, [x22, #0x50]
100840780:     	cmp	x1, x0
100840784:     	b.ls	0x1008408d4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2a0>
100840788:     	ldr	x8, [x22, #0x48]
10084078c:     	add	x8, x8, x0, lsl #4
100840790:     	ldr	x8, [x8]
100840794:     	lsr	x8, x8, x2
100840798:     	tbnz	w8, #0x0, 0x1008407c4 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x190>
10084079c:     	b	0x100840760 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
1008407a0:     	ldr	x1, [x22, #0x58]
1008407a4:     	cmp	x1, x0
1008407a8:     	b.ls	0x1008408c8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x294>
1008407ac:     	ldr	x8, [x22, #0x50]
1008407b0:     	add	x8, x8, x0, lsl #5
1008407b4:     	add	x8, x8, #0x18
1008407b8:     	ldr	x8, [x8]
1008407bc:     	lsr	x8, x8, x2
1008407c0:     	tbz	w8, #0x0, 0x100840760 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
1008407c4:     	and	x8, x2, #0x3f
1008407c8:     	lsr	x8, x28, x8
1008407cc:     	and	w3, w8, #0x1
1008407d0:     	mov	x0, x22
1008407d4:     	mov	x1, x21
1008407d8:     	bl	0x100dec250 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8cofactorB6_>
1008407dc:     	mov	x21, x0
1008407e0:     	add	x24, x24, #0x1
1008407e4:     	str	x24, [x20, #0x78]
1008407e8:     	b	0x100840760 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
1008407ec:     	ldr	x8, [sp, #0x10]
1008407f0:     	cbz	x8, 0x1008407fc <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1c8>
1008407f4:     	mov	x0, x27
1008407f8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008407fc:     	ldr	x8, [x22, #0x40]
100840800:     	lsr	w0, w21, #1
100840804:     	cmn	x8, #0x1
100840808:     	b.eq	0x100840878 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x244>
10084080c:     	ldr	x1, [x22, #0x50]
100840810:     	cmp	x1, x0
100840814:     	b.ls	0x1008408e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
100840818:     	ldr	x8, [x22, #0x48]
10084081c:     	add	x8, x8, x0, lsl #4
100840820:     	ldr	x8, [x8]
100840824:     	bics	x9, x8, x25
100840828:     	str	x9, [sp, #0x18]
10084082c:     	b.ne	0x1008408a0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x26c>
100840830:     	bic	x8, x25, x8
100840834:     	fmov	d0, x8
100840838:     	cnt.8b	v0, v0
10084083c:     	addv.8b	b0, v0
100840840:     	ldr	x8, [x20, #0x80]
100840844:     	fmov	x9, d0
100840848:     	add	x8, x8, x9
10084084c:     	str	x8, [x20, #0x80]
100840850:     	str	w21, [x19, #0x10]
100840854:     	stp	xzr, xzr, [x19]
100840858:     	ldp	x29, x30, [sp, #0x80]
10084085c:     	ldp	x20, x19, [sp, #0x70]
100840860:     	ldp	x22, x21, [sp, #0x60]
100840864:     	ldp	x24, x23, [sp, #0x50]
100840868:     	ldp	x26, x25, [sp, #0x40]
10084086c:     	ldp	x28, x27, [sp, #0x30]
100840870:     	add	sp, sp, #0x90
100840874:     	ret
100840878:     	ldr	x1, [x22, #0x58]
10084087c:     	cmp	x1, x0
100840880:     	b.ls	0x1008408ec <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
100840884:     	ldr	x8, [x22, #0x50]
100840888:     	add	x8, x8, x0, lsl #5
10084088c:     	add	x8, x8, #0x18
100840890:     	ldr	x8, [x8]
100840894:     	bics	x9, x8, x25
100840898:     	str	x9, [sp, #0x18]
10084089c:     	b.eq	0x100840830 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1fc>
1008408a0:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
1008408a4:     	add	x2, x2, #0x268
1008408a8:     	adrp	x3, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008408ac:     	add	x3, x3, #0x2d9
1008408b0:     	adrp	x5, 0x101751000 <dyld_stub_binder+0x101751000>
1008408b4:     	add	x5, x5, #0xed8
1008408b8:     	add	x1, sp, #0x18
1008408bc:     	mov	w0, #0x0                ; =0
1008408c0:     	mov	w4, #0x43               ; =67
1008408c4:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1008408c8:     	adrp	x8, 0x101799000 <dyld_stub_binder+0x101799000>
1008408cc:     	add	x8, x8, #0x5c8
1008408d0:     	str	x8, [sp, #0x8]
1008408d4:     	ldr	x2, [sp, #0x8]
1008408d8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008408dc:     	brk	#0x1
1008408e0:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008408e4:     	add	x2, x2, #0x5e0
1008408e8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008408ec:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008408f0:     	add	x2, x2, #0x5c8
1008408f4:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008408f8:     	b	0x100840910 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2dc>
1008408fc:     	mov	x19, x0
100840900:     	ldr	x8, [sp, #0x18]
100840904:     	cbz	x8, 0x100840924 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
100840908:     	ldr	x27, [sp, #0x20]
10084090c:     	b	0x10084091c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2e8>
100840910:     	mov	x19, x0
100840914:     	ldr	x8, [sp, #0x10]
100840918:     	cbz	x8, 0x100840924 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
10084091c:     	mov	x0, x27
100840920:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
100840924:     	mov	x0, x19
100840928:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
