
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001012da8fc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_>:
1012da8fc:     	sub	sp, sp, #0x110
1012da900:     	stp	x28, x27, [sp, #0xb0]
1012da904:     	stp	x26, x25, [sp, #0xc0]
1012da908:     	stp	x24, x23, [sp, #0xd0]
1012da90c:     	stp	x22, x21, [sp, #0xe0]
1012da910:     	stp	x20, x19, [sp, #0xf0]
1012da914:     	stp	x29, x30, [sp, #0x100]
1012da918:     	add	x29, sp, #0x100
1012da91c:     	ldr	x8, [x0, #0x108]
1012da920:     	cmp	w8, #0x3e
1012da924:     	and	x9, x8, #0x3f
1012da928:     	ccmp	x3, x9, #0x0, ls
1012da92c:     	b.ne	0x1012dab7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012da930:     	mov	x22, x7
1012da934:     	mov	x20, x5
1012da938:     	mov	x24, x4
1012da93c:     	mov	x21, x3
1012da940:     	mov	x23, x2
1012da944:     	mov	x25, x1
1012da948:     	mov	x19, x0
1012da94c:     	lsl	x10, x3, #2
1012da950:     	mov	x9, #0x0                ; =0
1012da954:     	cbz	x3, 0x1012da988 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x8c>
1012da958:     	mov	w11, #0x1               ; =1
1012da95c:     	mov	x12, x10
1012da960:     	mov	x13, x23
1012da964:     	ldr	w14, [x13], #0x4
1012da968:     	cmp	w14, w8
1012da96c:     	b.hs	0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012da970:     	lsr	x15, x9, x14
1012da974:     	tbnz	w15, #0x0, 0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012da978:     	lsl	x14, x11, x14
1012da97c:     	orr	x9, x14, x9
1012da980:     	subs	x12, x12, #0x4
1012da984:     	b.ne	0x1012da964 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x68>
1012da988:     	stur	x9, [x29, #-0x58]
1012da98c:     	mov	x11, #-0x1              ; =-1
1012da990:     	lsl	x11, x11, x21
1012da994:     	mvn	x11, x11
1012da998:     	str	x11, [sp, #0x18]
1012da99c:     	cmp	x9, x11
1012da9a0:     	b.ne	0x1012dab94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012da9a4:     	cmp	x6, x21
1012da9a8:     	b.ne	0x1012dab7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012da9ac:     	mov	x11, #0x0               ; =0
1012da9b0:     	cbz	x21, 0x1012da9e4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xe8>
1012da9b4:     	mov	w12, #0x1               ; =1
1012da9b8:     	mov	x13, x10
1012da9bc:     	mov	x14, x20
1012da9c0:     	ldr	w15, [x14], #0x4
1012da9c4:     	cmp	w15, w8
1012da9c8:     	b.hs	0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012da9cc:     	lsr	x16, x11, x15
1012da9d0:     	tbnz	w16, #0x0, 0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012da9d4:     	lsl	x15, x12, x15
1012da9d8:     	orr	x11, x15, x11
1012da9dc:     	subs	x13, x13, #0x4
1012da9e0:     	b.ne	0x1012da9c0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xc4>
1012da9e4:     	stur	x11, [x29, #-0x58]
1012da9e8:     	str	x9, [sp, #0x18]
1012da9ec:     	cmp	x11, x9
1012da9f0:     	b.ne	0x1012dab94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012da9f4:     	ldr	x11, [x29, #0x18]
1012da9f8:     	cmp	x11, x21
1012da9fc:     	b.ne	0x1012dab7c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x280>
1012daa00:     	ldr	x26, [x29, #0x10]
1012daa04:     	mov	x11, #0x0               ; =0
1012daa08:     	cbz	x21, 0x1012daa38 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x13c>
1012daa0c:     	mov	w12, #0x1               ; =1
1012daa10:     	mov	x13, x26
1012daa14:     	ldr	w14, [x13], #0x4
1012daa18:     	cmp	w14, w8
1012daa1c:     	b.hs	0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012daa20:     	lsr	x15, x11, x14
1012daa24:     	tbnz	w15, #0x0, 0x1012dab64 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x268>
1012daa28:     	lsl	x14, x12, x14
1012daa2c:     	orr	x11, x14, x11
1012daa30:     	subs	x10, x10, #0x4
1012daa34:     	b.ne	0x1012daa14 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x118>
1012daa38:     	stur	x11, [x29, #-0x58]
1012daa3c:     	str	x9, [sp, #0x18]
1012daa40:     	cmp	x11, x9
1012daa44:     	b.ne	0x1012dab94 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x298>
1012daa48:     	lsr	x8, x22, x8
1012daa4c:     	str	x8, [sp, #0x18]
1012daa50:     	cbnz	x8, 0x1012dabb0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2b4>
1012daa54:     	mov	x0, x19
1012daa58:     	mov	x1, x23
1012daa5c:     	mov	x2, x21
1012daa60:     	bl	0x10100d044 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB2_9EssentialKm6_E11support_mapB6_>
1012daa64:     	tbz	w0, #0x0, 0x1012dab40 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x244>
1012daa68:     	mov	x0, x19
1012daa6c:     	mov	x1, x26
1012daa70:     	mov	x2, x21
1012daa74:     	bl	0x10100d044 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB2_9EssentialKm6_E11support_mapB6_>
1012daa78:     	cbz	w0, 0x1012dab40 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x244>
1012daa7c:     	lsr	x27, x25, #1
1012daa80:     	tbz	w25, #0x0, 0x1012daa9c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1a0>
1012daa84:     	ldr	w2, [x19, #0x148]
1012daa88:     	mov	x0, x19
1012daa8c:     	mov	w1, #0x4                ; =4
1012daa90:     	mov	x3, x27
1012daa94:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012daa98:     	mov	x27, x0
1012daa9c:     	lsr	x5, x24, #1
1012daaa0:     	tbz	w24, #0x0, 0x1012daabc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1c0>
1012daaa4:     	ldr	w2, [x19, #0x148]
1012daaa8:     	mov	x0, x19
1012daaac:     	mov	w1, #0x4                ; =4
1012daab0:     	mov	x3, x5
1012daab4:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012daab8:     	mov	x5, x0
1012daabc:     	stp	x26, x21, [sp, #0x8]
1012daac0:     	add	x0, sp, #0x18
1012daac4:     	str	x22, [sp]
1012daac8:     	mov	x1, x19
1012daacc:     	mov	x2, x27
1012daad0:     	mov	x3, x23
1012daad4:     	mov	x4, x21
1012daad8:     	mov	x6, x20
1012daadc:     	mov	x7, x21
1012daae0:     	bl	0x1010ae29c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>
1012daae4:     	ldr	w3, [sp, #0x18]
1012daae8:     	ldr	w2, [x19, #0x148]
1012daaec:     	mov	x0, x19
1012daaf0:     	mov	w1, #0x8                ; =8
1012daaf4:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012daaf8:     	mov	x20, x0
1012daafc:     	ldr	x2, [x19, #0x138]
1012dab00:     	mov	x0, x19
1012dab04:     	mov	x1, x20
1012dab08:     	bl	0x100fa5938 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1012dab0c:     	mov	x21, x0
1012dab10:     	cbz	w0, 0x1012dab2c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x230>
1012dab14:     	ldr	w2, [x19, #0x148]
1012dab18:     	mov	x0, x19
1012dab1c:     	mov	w1, #0x4                ; =4
1012dab20:     	mov	x3, x20
1012dab24:     	bl	0x100faf740 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1012dab28:     	mov	x20, x0
1012dab2c:     	mov	w8, w20
1012dab30:     	mov	w9, w21
1012dab34:     	orr	x1, x9, x8, lsl #1
1012dab38:     	mov	w0, #0x1                ; =1
1012dab3c:     	b	0x1012dab44 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x248>
1012dab40:     	mov	x0, #0x0                ; =0
1012dab44:     	ldp	x29, x30, [sp, #0x100]
1012dab48:     	ldp	x20, x19, [sp, #0xf0]
1012dab4c:     	ldp	x22, x21, [sp, #0xe0]
1012dab50:     	ldp	x24, x23, [sp, #0xd0]
1012dab54:     	ldp	x26, x25, [sp, #0xc0]
1012dab58:     	ldp	x28, x27, [sp, #0xb0]
1012dab5c:     	add	sp, sp, #0x110
1012dab60:     	ret
1012dab64:     	adrp	x0, 0x1017bc000 <dyld_stub_binder+0x1017bc000>
1012dab68:     	add	x0, x0, #0x48d
1012dab6c:     	adrp	x2, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dab70:     	add	x2, x2, #0xab8
1012dab74:     	mov	w1, #0x39               ; =57
1012dab78:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012dab7c:     	adrp	x0, 0x1017ba000 <dyld_stub_binder+0x1017ba000>
1012dab80:     	add	x0, x0, #0x3b9
1012dab84:     	adrp	x2, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dab88:     	add	x2, x2, #0xaa0
1012dab8c:     	mov	w1, #0x3d               ; =61
1012dab90:     	bl	0x1016e7508 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1012dab94:     	adrp	x5, 0x10198f000 <dyld_stub_binder+0x10198f000>
1012dab98:     	add	x5, x5, #0xad0
1012dab9c:     	sub	x1, x29, #0x58
1012daba0:     	add	x2, sp, #0x18
1012daba4:     	mov	w0, #0x0                ; =0
1012daba8:     	mov	x3, #0x0                ; =0
1012dabac:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012dabb0:     	adrp	x2, 0x10185b000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1ec7>
1012dabb4:     	add	x2, x2, #0x358
1012dabb8:     	adrp	x5, 0x1019c4000 <dyld_stub_binder+0x1019c4000>
1012dabbc:     	add	x5, x5, #0x418
1012dabc0:     	add	x1, sp, #0x18
1012dabc4:     	mov	w0, #0x0                ; =0
1012dabc8:     	mov	x3, #0x0                ; =0
1012dabcc:     	bl	0x1016e7420 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1012dabd0:     	nop
1012dabd4:     	nop
1012dabd8:     	nop
1012dabdc:     	nop
1012dabe0:     	nop
1012dabe4:     	nop
1012dabe8:     	nop
1012dabec:     	nop
1012dabf0:     	nop
1012dabf4:     	nop
1012dabf8:     	nop
1012dabfc:     	nop
