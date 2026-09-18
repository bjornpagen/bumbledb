
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012dbd14 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_>:
1012dbd14:     	sub	sp, sp, #0x110
1012dbd18:     	stp	x28, x27, [sp, #0xb0]
1012dbd1c:     	stp	x26, x25, [sp, #0xc0]
1012dbd20:     	stp	x24, x23, [sp, #0xd0]
1012dbd24:     	stp	x22, x21, [sp, #0xe0]
1012dbd28:     	stp	x20, x19, [sp, #0xf0]
1012dbd2c:     	stp	x29, x30, [sp, #0x100]
1012dbd30:     	add	x29, sp, #0x100
1012dbd34:     	ldr	x8, [x0, #0x108]
1012dbd38:     	cmp	w8, #0x3e
1012dbd3c:     	and	x9, x8, #0x3f
1012dbd40:     	ccmp	x3, x9, #0x0, ls
1012dbd44:     	b.ne	0x1012dbf94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012dbd48:     	mov	x22, x7
1012dbd4c:     	mov	x20, x5
1012dbd50:     	mov	x24, x4
1012dbd54:     	mov	x21, x3
1012dbd58:     	mov	x23, x2
1012dbd5c:     	mov	x25, x1
1012dbd60:     	mov	x19, x0
1012dbd64:     	lsl	x10, x3, #2
1012dbd68:     	mov	x9, #0x0                ; =0
1012dbd6c:     	cbz	x3, 0x1012dbda0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x8c>
1012dbd70:     	mov	w11, #0x1               ; =1
1012dbd74:     	mov	x12, x10
1012dbd78:     	mov	x13, x23
1012dbd7c:     	ldr	w14, [x13], #0x4
1012dbd80:     	cmp	w14, w8
1012dbd84:     	b.hs	0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbd88:     	lsr	x15, x9, x14
1012dbd8c:     	tbnz	w15, #0x0, 0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbd90:     	lsl	x14, x11, x14
1012dbd94:     	orr	x9, x14, x9
1012dbd98:     	subs	x12, x12, #0x4
1012dbd9c:     	b.ne	0x1012dbd7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x68>
1012dbda0:     	stur	x9, [x29, #-0x58]
1012dbda4:     	mov	x11, #-0x1              ; =-1
1012dbda8:     	lsl	x11, x11, x21
1012dbdac:     	mvn	x11, x11
1012dbdb0:     	str	x11, [sp, #0x18]
1012dbdb4:     	cmp	x9, x11
1012dbdb8:     	b.ne	0x1012dbfac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012dbdbc:     	cmp	x6, x21
1012dbdc0:     	b.ne	0x1012dbf94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012dbdc4:     	mov	x11, #0x0               ; =0
1012dbdc8:     	cbz	x21, 0x1012dbdfc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xe8>
1012dbdcc:     	mov	w12, #0x1               ; =1
1012dbdd0:     	mov	x13, x10
1012dbdd4:     	mov	x14, x20
1012dbdd8:     	ldr	w15, [x14], #0x4
1012dbddc:     	cmp	w15, w8
1012dbde0:     	b.hs	0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbde4:     	lsr	x16, x11, x15
1012dbde8:     	tbnz	w16, #0x0, 0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbdec:     	lsl	x15, x12, x15
1012dbdf0:     	orr	x11, x15, x11
1012dbdf4:     	subs	x13, x13, #0x4
1012dbdf8:     	b.ne	0x1012dbdd8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xc4>
1012dbdfc:     	stur	x11, [x29, #-0x58]
1012dbe00:     	str	x9, [sp, #0x18]
1012dbe04:     	cmp	x11, x9
1012dbe08:     	b.ne	0x1012dbfac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012dbe0c:     	ldr	x11, [x29, #0x18]
1012dbe10:     	cmp	x11, x21
1012dbe14:     	b.ne	0x1012dbf94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012dbe18:     	ldr	x26, [x29, #0x10]
1012dbe1c:     	mov	x11, #0x0               ; =0
1012dbe20:     	cbz	x21, 0x1012dbe50 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x13c>
1012dbe24:     	mov	w12, #0x1               ; =1
1012dbe28:     	mov	x13, x26
1012dbe2c:     	ldr	w14, [x13], #0x4
1012dbe30:     	cmp	w14, w8
1012dbe34:     	b.hs	0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbe38:     	lsr	x15, x11, x14
1012dbe3c:     	tbnz	w15, #0x0, 0x1012dbf7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012dbe40:     	lsl	x14, x12, x14
1012dbe44:     	orr	x11, x14, x11
1012dbe48:     	subs	x10, x10, #0x4
1012dbe4c:     	b.ne	0x1012dbe2c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x118>
1012dbe50:     	stur	x11, [x29, #-0x58]
1012dbe54:     	str	x9, [sp, #0x18]
1012dbe58:     	cmp	x11, x9
1012dbe5c:     	b.ne	0x1012dbfac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012dbe60:     	lsr	x8, x22, x8
1012dbe64:     	str	x8, [sp, #0x18]
1012dbe68:     	cbnz	x8, 0x1012dbfc8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2b4>
1012dbe6c:     	mov	x0, x19
1012dbe70:     	mov	x1, x23
1012dbe74:     	mov	x2, x21
1012dbe78:     	bl	0x10100e6fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB2_9EssentialKm9_E11support_mapB6_>
1012dbe7c:     	tbz	w0, #0x0, 0x1012dbf58 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x244>
1012dbe80:     	mov	x0, x19
1012dbe84:     	mov	x1, x26
1012dbe88:     	mov	x2, x21
1012dbe8c:     	bl	0x10100e6fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB2_9EssentialKm9_E11support_mapB6_>
1012dbe90:     	cbz	w0, 0x1012dbf58 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x244>
1012dbe94:     	lsr	x27, x25, #1
1012dbe98:     	tbz	w25, #0x0, 0x1012dbeb4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1a0>
1012dbe9c:     	ldr	w2, [x19, #0x148]
1012dbea0:     	mov	x0, x19
1012dbea4:     	mov	w1, #0x4                ; =4
1012dbea8:     	mov	x3, x27
1012dbeac:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbeb0:     	mov	x27, x0
1012dbeb4:     	lsr	x5, x24, #1
1012dbeb8:     	tbz	w24, #0x0, 0x1012dbed4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1c0>
1012dbebc:     	ldr	w2, [x19, #0x148]
1012dbec0:     	mov	x0, x19
1012dbec4:     	mov	w1, #0x4                ; =4
1012dbec8:     	mov	x3, x5
1012dbecc:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbed0:     	mov	x5, x0
1012dbed4:     	stp	x26, x21, [sp, #0x8]
1012dbed8:     	add	x0, sp, #0x18
1012dbedc:     	str	x22, [sp]
1012dbee0:     	mov	x1, x19
1012dbee4:     	mov	x2, x27
1012dbee8:     	mov	x3, x23
1012dbeec:     	mov	x4, x21
1012dbef0:     	mov	x6, x20
1012dbef4:     	mov	x7, x21
1012dbef8:     	bl	0x1010ae720 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
1012dbefc:     	ldr	w3, [sp, #0x18]
1012dbf00:     	ldr	w2, [x19, #0x148]
1012dbf04:     	mov	x0, x19
1012dbf08:     	mov	w1, #0x8                ; =8
1012dbf0c:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbf10:     	mov	x20, x0
1012dbf14:     	ldr	x2, [x19, #0x138]
1012dbf18:     	mov	x0, x19
1012dbf1c:     	mov	x1, x20
1012dbf20:     	bl	0x100fa5938 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1012dbf24:     	mov	x21, x0
1012dbf28:     	cbz	w0, 0x1012dbf44 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x230>
1012dbf2c:     	ldr	w2, [x19, #0x148]
1012dbf30:     	mov	x0, x19
1012dbf34:     	mov	w1, #0x4                ; =4
1012dbf38:     	mov	x3, x20
1012dbf3c:     	bl	0x100fb2c80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1012dbf40:     	mov	x20, x0
1012dbf44:     	mov	w8, w20
1012dbf48:     	mov	w9, w21
1012dbf4c:     	orr	x1, x9, x8, lsl #1
1012dbf50:     	mov	w0, #0x1                ; =1
1012dbf54:     	b	0x1012dbf5c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x248>
1012dbf58:     	mov	x0, #0x0                ; =0
1012dbf5c:     	ldp	x29, x30, [sp, #0x100]
1012dbf60:     	ldp	x20, x19, [sp, #0xf0]
1012dbf64:     	ldp	x22, x21, [sp, #0xe0]
1012dbf68:     	ldp	x24, x23, [sp, #0xd0]
1012dbf6c:     	ldp	x26, x25, [sp, #0xc0]
1012dbf70:     	ldp	x28, x27, [sp, #0xb0]
1012dbf74:     	add	sp, sp, #0x110
1012dbf78:     	ret
1012dbf7c:     	adrp	x0, 0x1017bc000 <dyld_stub_binder+0x1017bc000>
1012dbf80:     	add	x0, x0, #0x48d
1012dbf84:     	adrp	x2, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dbf88:     	add	x2, x2, #0xab8
1012dbf8c:     	mov	w1, #0x39               ; =57
1012dbf90:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012dbf94:     	adrp	x0, 0x1017ba000 <dyld_stub_binder+0x1017ba000>
1012dbf98:     	add	x0, x0, #0x3b9
1012dbf9c:     	adrp	x2, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dbfa0:     	add	x2, x2, #0xaa0
1012dbfa4:     	mov	w1, #0x3d               ; =61
1012dbfa8:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012dbfac:     	adrp	x5, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dbfb0:     	add	x5, x5, #0xad0
1012dbfb4:     	sub	x1, x29, #0x58
1012dbfb8:     	add	x2, sp, #0x18
1012dbfbc:     	mov	w0, #0x0                ; =0
1012dbfc0:     	mov	x3, #0x0                ; =0
1012dbfc4:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012dbfc8:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1012dbfcc:     	add	x2, x2, #0x358
1012dbfd0:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dbfd4:     	add	x5, x5, #0x418
1012dbfd8:     	add	x1, sp, #0x18
1012dbfdc:     	mov	w0, #0x0                ; =0
1012dbfe0:     	mov	x3, #0x0                ; =0
1012dbfe4:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012dbfe8:     	nop
1012dbfec:     	nop
1012dbff0:     	nop
1012dbff4:     	nop
1012dbff8:     	nop
1012dbffc:     	nop
