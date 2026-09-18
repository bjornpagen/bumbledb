
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001009316ac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_>:
1009316ac:     	sub	sp, sp, #0x90
1009316b0:     	stp	x28, x27, [sp, #0x30]
1009316b4:     	stp	x26, x25, [sp, #0x40]
1009316b8:     	stp	x24, x23, [sp, #0x50]
1009316bc:     	stp	x22, x21, [sp, #0x60]
1009316c0:     	stp	x20, x19, [sp, #0x70]
1009316c4:     	stp	x29, x30, [sp, #0x80]
1009316c8:     	add	x29, sp, #0x80
1009316cc:     	mov	x24, x1
1009316d0:     	mov	x19, x0
1009316d4:     	ldr	x23, [x1]
1009316d8:     	cbz	x23, 0x10093171c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x70>
1009316dc:     	mov	x20, x3
1009316e0:     	mov	x22, x2
1009316e4:     	ldr	x8, [x3, #0x70]
1009316e8:     	add	x8, x8, #0x1
1009316ec:     	str	x8, [x3, #0x70]
1009316f0:     	ldr	w21, [x24, #0x10]
1009316f4:     	ldr	x8, [x2, #0x40]
1009316f8:     	lsr	x0, x21, #1
1009316fc:     	cmn	x8, #0x1
100931700:     	b.eq	0x100931730 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x84>
100931704:     	ldr	x1, [x22, #0x50]
100931708:     	cmp	x1, x0
10093170c:     	b.ls	0x100931958 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
100931710:     	ldr	x8, [x22, #0x48]
100931714:     	add	x8, x8, x0, lsl #4
100931718:     	b	0x100931748 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x9c>
10093171c:     	ldr	q0, [x24]
100931720:     	str	q0, [x19]
100931724:     	ldr	x8, [x24, #0x10]
100931728:     	str	x8, [x19, #0x10]
10093172c:     	b	0x1009318d0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x224>
100931730:     	ldr	x1, [x22, #0x58]
100931734:     	cmp	x1, x0
100931738:     	b.ls	0x100931964 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
10093173c:     	ldr	x8, [x22, #0x50]
100931740:     	add	x8, x8, x0, lsl #5
100931744:     	add	x8, x8, #0x18
100931748:     	mov	x26, #0x0               ; =0
10093174c:     	ldr	x8, [x8]
100931750:     	bic	x25, x8, x23
100931754:     	mov	w8, #0x4                ; =4
100931758:     	stp	xzr, x8, [sp, #0x18]
10093175c:     	str	xzr, [sp, #0x28]
100931760:     	mov	w9, #0x1                ; =1
100931764:     	b	0x10093178c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xe0>
100931768:     	rbit	x9, x23
10093176c:     	clz	x9, x9
100931770:     	str	w9, [x8, x26]
100931774:     	str	x28, [sp, #0x28]
100931778:     	sub	x10, x23, #0x1
10093177c:     	add	x26, x26, #0x4
100931780:     	add	x9, x28, #0x1
100931784:     	ands	x23, x10, x23
100931788:     	b.eq	0x1009317b0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x104>
10093178c:     	mov	x28, x9
100931790:     	sub	x9, x9, #0x1
100931794:     	ldr	x10, [sp, #0x18]
100931798:     	cmp	x9, x10
10093179c:     	b.ne	0x100931768 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
1009317a0:     	add	x0, sp, #0x18
1009317a4:     	bl	0x1016e831c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1009317a8:     	ldr	x8, [sp, #0x20]
1009317ac:     	b	0x100931768 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0xbc>
1009317b0:     	ldp	x8, x27, [sp, #0x18]
1009317b4:     	str	x8, [sp, #0x10]
1009317b8:     	cbz	x28, 0x100931864 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
1009317bc:     	ldr	x28, [x24, #0x8]
1009317c0:     	ldr	x24, [x20, #0x78]
1009317c4:     	mov	x23, x27
1009317c8:     	adrp	x8, 0x10198e000 <dyld_stub_binder+0x10198e000>
1009317cc:     	add	x8, x8, #0x4d0
1009317d0:     	str	x8, [sp, #0x8]
1009317d4:     	b	0x1009317e0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x134>
1009317d8:     	subs	x26, x26, #0x4
1009317dc:     	b.eq	0x100931864 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1b8>
1009317e0:     	ldr	w2, [x23], #0x4
1009317e4:     	ldr	x8, [x22, #0x40]
1009317e8:     	lsr	w0, w21, #1
1009317ec:     	cmn	x8, #0x1
1009317f0:     	b.eq	0x100931818 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x16c>
1009317f4:     	ldr	x1, [x22, #0x50]
1009317f8:     	cmp	x1, x0
1009317fc:     	b.ls	0x10093194c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2a0>
100931800:     	ldr	x8, [x22, #0x48]
100931804:     	add	x8, x8, x0, lsl #4
100931808:     	ldr	x8, [x8]
10093180c:     	lsr	x8, x8, x2
100931810:     	tbnz	w8, #0x0, 0x10093183c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x190>
100931814:     	b	0x1009317d8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
100931818:     	ldr	x1, [x22, #0x58]
10093181c:     	cmp	x1, x0
100931820:     	b.ls	0x100931940 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x294>
100931824:     	ldr	x8, [x22, #0x50]
100931828:     	add	x8, x8, x0, lsl #5
10093182c:     	add	x8, x8, #0x18
100931830:     	ldr	x8, [x8]
100931834:     	lsr	x8, x8, x2
100931838:     	tbz	w8, #0x0, 0x1009317d8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
10093183c:     	and	x8, x2, #0x3f
100931840:     	lsr	x8, x28, x8
100931844:     	and	w3, w8, #0x1
100931848:     	mov	x0, x22
10093184c:     	mov	x1, x21
100931850:     	bl	0x100fa57d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8cofactorB6_>
100931854:     	mov	x21, x0
100931858:     	add	x24, x24, #0x1
10093185c:     	str	x24, [x20, #0x78]
100931860:     	b	0x1009317d8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x12c>
100931864:     	ldr	x8, [sp, #0x10]
100931868:     	cbz	x8, 0x100931874 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1c8>
10093186c:     	mov	x0, x27
100931870:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
100931874:     	ldr	x8, [x22, #0x40]
100931878:     	lsr	w0, w21, #1
10093187c:     	cmn	x8, #0x1
100931880:     	b.eq	0x1009318f0 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x244>
100931884:     	ldr	x1, [x22, #0x50]
100931888:     	cmp	x1, x0
10093188c:     	b.ls	0x100931958 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2ac>
100931890:     	ldr	x8, [x22, #0x48]
100931894:     	add	x8, x8, x0, lsl #4
100931898:     	ldr	x8, [x8]
10093189c:     	bics	x9, x8, x25
1009318a0:     	str	x9, [sp, #0x18]
1009318a4:     	b.ne	0x100931918 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x26c>
1009318a8:     	bic	x8, x25, x8
1009318ac:     	fmov	d0, x8
1009318b0:     	cnt.8b	v0, v0
1009318b4:     	addv.8b	b0, v0
1009318b8:     	ldr	x8, [x20, #0x80]
1009318bc:     	fmov	x9, d0
1009318c0:     	add	x8, x8, x9
1009318c4:     	str	x8, [x20, #0x80]
1009318c8:     	str	w21, [x19, #0x10]
1009318cc:     	stp	xzr, xzr, [x19]
1009318d0:     	ldp	x29, x30, [sp, #0x80]
1009318d4:     	ldp	x20, x19, [sp, #0x70]
1009318d8:     	ldp	x22, x21, [sp, #0x60]
1009318dc:     	ldp	x24, x23, [sp, #0x50]
1009318e0:     	ldp	x26, x25, [sp, #0x40]
1009318e4:     	ldp	x28, x27, [sp, #0x30]
1009318e8:     	add	sp, sp, #0x90
1009318ec:     	ret
1009318f0:     	ldr	x1, [x22, #0x58]
1009318f4:     	cmp	x1, x0
1009318f8:     	b.ls	0x100931964 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2b8>
1009318fc:     	ldr	x8, [x22, #0x50]
100931900:     	add	x8, x8, x0, lsl #5
100931904:     	add	x8, x8, #0x18
100931908:     	ldr	x8, [x8]
10093190c:     	bics	x9, x8, x25
100931910:     	str	x9, [sp, #0x18]
100931914:     	b.eq	0x1009318a8 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x1fc>
100931918:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
10093191c:     	add	x2, x2, #0x358
100931920:     	adrp	x3, 0x101799000 <dyld_stub_binder+0x101799000>
100931924:     	add	x3, x3, #0xdbd
100931928:     	adrp	x5, 0x101945000 <dyld_stub_binder+0x101945000>
10093192c:     	add	x5, x5, #0xed8
100931930:     	add	x1, sp, #0x18
100931934:     	mov	w0, #0x0                ; =0
100931938:     	mov	w4, #0x43               ; =67
10093193c:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100931940:     	adrp	x8, 0x10198e000 <dyld_stub_binder+0x10198e000>
100931944:     	add	x8, x8, #0x4b8
100931948:     	str	x8, [sp, #0x8]
10093194c:     	ldr	x2, [sp, #0x8]
100931950:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100931954:     	brk	#0x1
100931958:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
10093195c:     	add	x2, x2, #0x4d0
100931960:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100931964:     	adrp	x2, 0x10198e000 <dyld_stub_binder+0x10198e000>
100931968:     	add	x2, x2, #0x4b8
10093196c:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100931970:     	b	0x100931988 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2dc>
100931974:     	mov	x19, x0
100931978:     	ldr	x8, [sp, #0x18]
10093197c:     	cbz	x8, 0x10093199c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
100931980:     	ldr	x27, [sp, #0x20]
100931984:     	b	0x100931994 <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2e8>
100931988:     	mov	x19, x0
10093198c:     	ldr	x8, [sp, #0x10]
100931990:     	cbz	x8, 0x10093199c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted12canonicalizeKm1_EBc_+0x2f0>
100931994:     	mov	x0, x27
100931998:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
10093199c:     	mov	x0, x19
1009319a0:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
