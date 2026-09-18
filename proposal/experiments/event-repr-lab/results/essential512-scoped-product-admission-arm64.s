
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012db97c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_>:
1012db97c:     	sub	sp, sp, #0x130
1012db980:     	stp	x28, x27, [sp, #0xd0]
1012db984:     	stp	x26, x25, [sp, #0xe0]
1012db988:     	stp	x24, x23, [sp, #0xf0]
1012db98c:     	stp	x22, x21, [sp, #0x100]
1012db990:     	stp	x20, x19, [sp, #0x110]
1012db994:     	stp	x29, x30, [sp, #0x120]
1012db998:     	add	x29, sp, #0x120
1012db99c:     	str	x7, [sp, #0x28]
1012db9a0:     	mov	x21, x6
1012db9a4:     	mov	x28, x5
1012db9a8:     	mov	x26, x4
1012db9ac:     	mov	x20, x3
1012db9b0:     	mov	x23, x2
1012db9b4:     	mov	x19, x0
1012db9b8:     	lsr	x24, x1, #1
1012db9bc:     	tbnz	w1, #0x0, 0x1012db9e0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x64>
1012db9c0:     	ldr	x22, [x29, #0x18]
1012db9c4:     	lsr	x25, x26, #1
1012db9c8:     	tbnz	w26, #0x0, 0x1012dba04 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x88>
1012db9cc:     	ldr	x26, [x29, #0x10]
1012db9d0:     	ldr	w27, [x19, #0x148]
1012db9d4:     	cmp	w27, #0x1
1012db9d8:     	b.ne	0x1012dba2c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0xb0>
1012db9dc:     	b	0x1012dbbc4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1012db9e0:     	ldr	w2, [x19, #0x148]
1012db9e4:     	mov	x0, x19
1012db9e8:     	mov	w1, #0x4                ; =4
1012db9ec:     	mov	x3, x24
1012db9f0:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012db9f4:     	mov	x24, x0
1012db9f8:     	ldr	x22, [x29, #0x18]
1012db9fc:     	lsr	x25, x26, #1
1012dba00:     	tbz	w26, #0x0, 0x1012db9cc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x50>
1012dba04:     	ldr	w2, [x19, #0x148]
1012dba08:     	mov	x0, x19
1012dba0c:     	mov	w1, #0x4                ; =4
1012dba10:     	mov	x3, x25
1012dba14:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dba18:     	mov	x25, x0
1012dba1c:     	ldr	x26, [x29, #0x10]
1012dba20:     	ldr	w27, [x19, #0x148]
1012dba24:     	cmp	w27, #0x1
1012dba28:     	b.eq	0x1012dbbc4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1012dba2c:     	str	x28, [sp, #0x20]
1012dba30:     	ldr	x8, [x19, #0x108]
1012dba34:     	mov	w9, w8
1012dba38:     	stp	x9, x20, [sp, #0x30]
1012dba3c:     	cmp	x20, x9
1012dba40:     	b.ne	0x1012dbc98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012dba44:     	cbz	x20, 0x1012dbb48 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1cc>
1012dba48:     	mov	x10, #0x0               ; =0
1012dba4c:     	lsl	x28, x20, #2
1012dba50:     	mov	w11, #0x1               ; =1
1012dba54:     	mov	x12, x28
1012dba58:     	mov	x13, x23
1012dba5c:     	ldr	w14, [x13], #0x4
1012dba60:     	cmp	w14, w8
1012dba64:     	b.hs	0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dba68:     	lsr	x15, x10, x14
1012dba6c:     	tbnz	w15, #0x0, 0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dba70:     	lsl	x14, x11, x14
1012dba74:     	orr	x10, x14, x10
1012dba78:     	subs	x12, x12, #0x4
1012dba7c:     	b.ne	0x1012dba5c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0xe0>
1012dba80:     	str	x21, [sp, #0x38]
1012dba84:     	cmp	x21, x20
1012dba88:     	b.ne	0x1012dbc98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012dba8c:     	mov	x10, #0x0               ; =0
1012dba90:     	mov	w11, #0x1               ; =1
1012dba94:     	mov	x12, x28
1012dba98:     	ldr	x13, [sp, #0x20]
1012dba9c:     	ldr	w14, [x13], #0x4
1012dbaa0:     	cmp	w14, w8
1012dbaa4:     	b.hs	0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dbaa8:     	lsr	x15, x10, x14
1012dbaac:     	tbnz	w15, #0x0, 0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dbab0:     	lsl	x14, x11, x14
1012dbab4:     	orr	x10, x14, x10
1012dbab8:     	subs	x12, x12, #0x4
1012dbabc:     	b.ne	0x1012dba9c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x120>
1012dbac0:     	str	x22, [sp, #0x38]
1012dbac4:     	cmp	x22, x20
1012dbac8:     	b.ne	0x1012dbc98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012dbacc:     	mov	x10, #0x0               ; =0
1012dbad0:     	mov	w11, #0x1               ; =1
1012dbad4:     	mov	x12, x28
1012dbad8:     	mov	x13, x26
1012dbadc:     	ldr	w14, [x13], #0x4
1012dbae0:     	cmp	w14, w8
1012dbae4:     	b.hs	0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dbae8:     	lsr	x15, x10, x14
1012dbaec:     	tbnz	w15, #0x0, 0x1012dbc80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x304>
1012dbaf0:     	lsl	x14, x11, x14
1012dbaf4:     	orr	x10, x14, x10
1012dbaf8:     	subs	x12, x12, #0x4
1012dbafc:     	b.ne	0x1012dbadc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x160>
1012dbb00:     	ldr	x8, [sp, #0x28]
1012dbb04:     	lsr	x8, x8, x9
1012dbb08:     	str	x8, [sp, #0x38]
1012dbb0c:     	cbnz	x8, 0x1012dbcb4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x338>
1012dbb10:     	mov	x0, x28
1012dbb14:     	mov	w1, #0x1                ; =1
1012dbb18:     	bl	0x1016ef824 <dyld_stub_binder+0x1016ef824>
1012dbb1c:     	cbz	x0, 0x1012dbce8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x36c>
1012dbb20:     	mov	x21, x0
1012dbb24:     	mov	x8, #0x0                ; =0
1012dbb28:     	ldr	w0, [x23, x8, lsl #2]
1012dbb2c:     	cmp	x20, x0
1012dbb30:     	b.ls	0x1012dbcd4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x358>
1012dbb34:     	str	w8, [x21, x0, lsl #2]
1012dbb38:     	add	x8, x8, #0x1
1012dbb3c:     	subs	x28, x28, #0x4
1012dbb40:     	b.ne	0x1012dbb28 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ac>
1012dbb44:     	b	0x1012dbb68 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ec>
1012dbb48:     	str	x21, [sp, #0x38]
1012dbb4c:     	cbnz	x21, 0x1012dbc98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012dbb50:     	str	x22, [sp, #0x38]
1012dbb54:     	cbnz	x22, 0x1012dbc98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x31c>
1012dbb58:     	ldr	x8, [sp, #0x28]
1012dbb5c:     	str	x8, [sp, #0x38]
1012dbb60:     	cbnz	x8, 0x1012dbcb4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x338>
1012dbb64:     	mov	w21, #0x4               ; =4
1012dbb68:     	mov	x0, x19
1012dbb6c:     	mov	x1, x27
1012dbb70:     	mov	x2, x21
1012dbb74:     	mov	x3, x20
1012dbb78:     	bl	0x100fb4160 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1012dbb7c:     	ldr	x28, [sp, #0x20]
1012dbb80:     	mov	x3, x0
1012dbb84:     	mov	x0, x19
1012dbb88:     	mov	w1, #0x8                ; =8
1012dbb8c:     	mov	x2, x24
1012dbb90:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbb94:     	mov	x24, x0
1012dbb98:     	ldr	w1, [x19, #0x148]
1012dbb9c:     	mov	x0, x19
1012dbba0:     	mov	x2, x26
1012dbba4:     	mov	x3, x20
1012dbba8:     	bl	0x100fb4160 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1012dbbac:     	mov	x27, x0
1012dbbb0:     	cbz	x20, 0x1012dbbbc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x240>
1012dbbb4:     	mov	x0, x21
1012dbbb8:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1012dbbbc:     	mov	x22, x20
1012dbbc0:     	mov	x21, x20
1012dbbc4:     	stp	x26, x22, [sp, #0x8]
1012dbbc8:     	add	x0, sp, #0x38
1012dbbcc:     	ldr	x8, [sp, #0x28]
1012dbbd0:     	str	x8, [sp]
1012dbbd4:     	mov	x1, x19
1012dbbd8:     	mov	x2, x24
1012dbbdc:     	mov	x3, x23
1012dbbe0:     	mov	x4, x20
1012dbbe4:     	mov	x5, x25
1012dbbe8:     	mov	x6, x28
1012dbbec:     	mov	x7, x21
1012dbbf0:     	bl	0x1010ae720 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
1012dbbf4:     	ldr	w3, [sp, #0x38]
1012dbbf8:     	mov	x0, x19
1012dbbfc:     	mov	w1, #0x8                ; =8
1012dbc00:     	mov	x2, x27
1012dbc04:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbc08:     	mov	x3, x0
1012dbc0c:     	ldr	w2, [x19, #0x148]
1012dbc10:     	mov	x0, x19
1012dbc14:     	mov	w1, #0x8                ; =8
1012dbc18:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbc1c:     	mov	x20, x0
1012dbc20:     	ldr	x2, [x19, #0x138]
1012dbc24:     	mov	x0, x19
1012dbc28:     	mov	x1, x20
1012dbc2c:     	bl	0x100fa5938 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1012dbc30:     	mov	x21, x0
1012dbc34:     	cbz	w0, 0x1012dbc50 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x2d4>
1012dbc38:     	ldr	w2, [x19, #0x148]
1012dbc3c:     	mov	x0, x19
1012dbc40:     	mov	w1, #0x4                ; =4
1012dbc44:     	mov	x3, x20
1012dbc48:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbc4c:     	mov	x20, x0
1012dbc50:     	mov	w8, w20
1012dbc54:     	mov	w9, w21
1012dbc58:     	orr	x1, x9, x8, lsl #1
1012dbc5c:     	mov	w0, #0x1                ; =1
1012dbc60:     	ldp	x29, x30, [sp, #0x120]
1012dbc64:     	ldp	x20, x19, [sp, #0x110]
1012dbc68:     	ldp	x22, x21, [sp, #0x100]
1012dbc6c:     	ldp	x24, x23, [sp, #0xf0]
1012dbc70:     	ldp	x26, x25, [sp, #0xe0]
1012dbc74:     	ldp	x28, x27, [sp, #0xd0]
1012dbc78:     	add	sp, sp, #0x130
1012dbc7c:     	ret
1012dbc80:     	adrp	x0, 0x101853000 <__RNvNvXsi_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab8transferNtB7_5ErrorNtNtCs4sDCw1iE1MS_4core3fmt5Debug3fmt8___OFFSET+0x1e68>
1012dbc84:     	add	x0, x0, #0xd3a
1012dbc88:     	adrp	x2, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dbc8c:     	add	x2, x2, #0x3e8
1012dbc90:     	mov	w1, #0x41               ; =65
1012dbc94:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012dbc98:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dbc9c:     	add	x5, x5, #0x3d0
1012dbca0:     	add	x1, sp, #0x38
1012dbca4:     	add	x2, sp, #0x30
1012dbca8:     	mov	w0, #0x0                ; =0
1012dbcac:     	mov	x3, #0x0                ; =0
1012dbcb0:     	bl	0x1016e73f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1012dbcb4:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1012dbcb8:     	add	x2, x2, #0x358
1012dbcbc:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dbcc0:     	add	x5, x5, #0x3b8
1012dbcc4:     	add	x1, sp, #0x38
1012dbcc8:     	mov	w0, #0x0                ; =0
1012dbccc:     	mov	x3, #0x0                ; =0
1012dbcd0:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012dbcd4:     	adrp	x2, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dbcd8:     	add	x2, x2, #0x3a0
1012dbcdc:     	mov	x1, x20
1012dbce0:     	bl	0x1016e751c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1012dbce4:     	brk	#0x1
1012dbce8:     	mov	w0, #0x4                ; =4
1012dbcec:     	mov	x1, x28
1012dbcf0:     	bl	0x1016e6d2c <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1012dbcf4:     	mov	x19, x0
1012dbcf8:     	cbnz	x20, 0x1012dbd04 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x388>
1012dbcfc:     	b	0x1012dbd0c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps12view_productB8_+0x390>
1012dbd00:     	mov	x19, x0
1012dbd04:     	mov	x0, x21
1012dbd08:     	bl	0x1016ef938 <dyld_stub_binder+0x1016ef938>
1012dbd0c:     	mov	x0, x19
1012dbd10:     	bl	0x1016ef788 <dyld_stub_binder+0x1016ef788>
