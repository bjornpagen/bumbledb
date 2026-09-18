
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001014abe80 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_>:
1014abe80:     	sub	sp, sp, #0x110
1014abe84:     	stp	x28, x27, [sp, #0xb0]
1014abe88:     	stp	x26, x25, [sp, #0xc0]
1014abe8c:     	stp	x24, x23, [sp, #0xd0]
1014abe90:     	stp	x22, x21, [sp, #0xe0]
1014abe94:     	stp	x20, x19, [sp, #0xf0]
1014abe98:     	stp	x29, x30, [sp, #0x100]
1014abe9c:     	add	x29, sp, #0x100
1014abea0:     	ldr	x8, [x0, #0x108]
1014abea4:     	cmp	w8, #0x3e
1014abea8:     	and	x9, x8, #0x3f
1014abeac:     	ccmp	x3, x9, #0x0, ls
1014abeb0:     	b.ne	0x1014ac290 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
1014abeb4:     	mov	x21, x7
1014abeb8:     	mov	x22, x5
1014abebc:     	mov	x23, x4
1014abec0:     	mov	x19, x3
1014abec4:     	mov	x24, x2
1014abec8:     	mov	x25, x1
1014abecc:     	mov	x20, x0
1014abed0:     	lsl	x10, x3, #2
1014abed4:     	mov	x9, #0x0                ; =0
1014abed8:     	cbz	x3, 0x1014abf0c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x8c>
1014abedc:     	mov	w11, #0x1               ; =1
1014abee0:     	mov	x12, x10
1014abee4:     	mov	x13, x24
1014abee8:     	ldr	w14, [x13], #0x4
1014abeec:     	cmp	w14, w8
1014abef0:     	b.hs	0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abef4:     	lsr	x15, x9, x14
1014abef8:     	tbnz	w15, #0x0, 0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abefc:     	lsl	x14, x11, x14
1014abf00:     	orr	x9, x14, x9
1014abf04:     	subs	x12, x12, #0x4
1014abf08:     	b.ne	0x1014abee8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x68>
1014abf0c:     	stur	x9, [x29, #-0x58]
1014abf10:     	mov	x11, #-0x1              ; =-1
1014abf14:     	lsl	x11, x11, x19
1014abf18:     	mvn	x11, x11
1014abf1c:     	str	x11, [sp, #0x18]
1014abf20:     	cmp	x9, x11
1014abf24:     	b.ne	0x1014ac2a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
1014abf28:     	cmp	x6, x19
1014abf2c:     	b.ne	0x1014ac290 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
1014abf30:     	mov	x11, #0x0               ; =0
1014abf34:     	cbz	x19, 0x1014abf68 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xe8>
1014abf38:     	mov	w12, #0x1               ; =1
1014abf3c:     	mov	x13, x10
1014abf40:     	mov	x14, x22
1014abf44:     	ldr	w15, [x14], #0x4
1014abf48:     	cmp	w15, w8
1014abf4c:     	b.hs	0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abf50:     	lsr	x16, x11, x15
1014abf54:     	tbnz	w16, #0x0, 0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abf58:     	lsl	x15, x12, x15
1014abf5c:     	orr	x11, x15, x11
1014abf60:     	subs	x13, x13, #0x4
1014abf64:     	b.ne	0x1014abf44 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xc4>
1014abf68:     	stur	x11, [x29, #-0x58]
1014abf6c:     	str	x9, [sp, #0x18]
1014abf70:     	cmp	x11, x9
1014abf74:     	b.ne	0x1014ac2a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
1014abf78:     	ldr	x11, [x29, #0x18]
1014abf7c:     	cmp	x11, x19
1014abf80:     	b.ne	0x1014ac290 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x410>
1014abf84:     	ldr	x26, [x29, #0x10]
1014abf88:     	mov	x11, #0x0               ; =0
1014abf8c:     	cbz	x19, 0x1014abfbc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x13c>
1014abf90:     	mov	w12, #0x1               ; =1
1014abf94:     	mov	x13, x26
1014abf98:     	ldr	w14, [x13], #0x4
1014abf9c:     	cmp	w14, w8
1014abfa0:     	b.hs	0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abfa4:     	lsr	x15, x11, x14
1014abfa8:     	tbnz	w15, #0x0, 0x1014ac278 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f8>
1014abfac:     	lsl	x14, x12, x14
1014abfb0:     	orr	x11, x14, x11
1014abfb4:     	subs	x10, x10, #0x4
1014abfb8:     	b.ne	0x1014abf98 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x118>
1014abfbc:     	stur	x11, [x29, #-0x58]
1014abfc0:     	str	x9, [sp, #0x18]
1014abfc4:     	cmp	x11, x9
1014abfc8:     	b.ne	0x1014ac2a8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x428>
1014abfcc:     	lsr	x8, x21, x8
1014abfd0:     	str	x8, [sp, #0x18]
1014abfd4:     	cbnz	x8, 0x1014ac2c4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x444>
1014abfd8:     	cbz	x21, 0x1014ac04c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
1014abfdc:     	ldr	w8, [x20, #0x188]
1014abfe0:     	cmp	w8, #0x1
1014abfe4:     	b.eq	0x1014ac04c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
1014abfe8:     	ldr	w8, [x20, #0x128]
1014abfec:     	cbz	w8, 0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014abff0:     	ldr	w8, [x20, #0x12c]
1014abff4:     	add	w9, w8, w8, lsl #1
1014abff8:     	lsr	x9, x21, x9
1014abffc:     	cbnz	x9, 0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac000:     	mov	x9, #-0x1               ; =-1
1014ac004:     	lsl	x10, x9, x8
1014ac008:     	mvn	x9, x10
1014ac00c:     	bic	x11, x21, x10
1014ac010:     	cmp	x11, x9
1014ac014:     	ccmp	x11, #0x0, #0x4, ne
1014ac018:     	b.ne	0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac01c:     	and	x11, x8, #0x3f
1014ac020:     	lsr	x11, x21, x11
1014ac024:     	bic	x11, x11, x10
1014ac028:     	cmp	x11, x9
1014ac02c:     	ccmp	x11, #0x0, #0x4, ne
1014ac030:     	b.ne	0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac034:     	ubfiz	w8, w8, #1, #5
1014ac038:     	lsr	x8, x21, x8
1014ac03c:     	bics	x8, x8, x10
1014ac040:     	b.eq	0x1014ac04c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1cc>
1014ac044:     	cmp	x8, x9
1014ac048:     	b.ne	0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac04c:     	mov	x0, x20
1014ac050:     	mov	x1, x24
1014ac054:     	mov	x2, x19
1014ac058:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac05c:     	tbz	w0, #0x0, 0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac060:     	mov	x0, x20
1014ac064:     	mov	x1, x22
1014ac068:     	mov	x2, x19
1014ac06c:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac070:     	cbz	w0, 0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac074:     	mov	x0, x20
1014ac078:     	mov	x1, x26
1014ac07c:     	mov	x2, x19
1014ac080:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac084:     	cbz	w0, 0x1014ac0bc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x23c>
1014ac088:     	stp	x26, x19, [sp, #0x8]
1014ac08c:     	add	x0, sp, #0x18
1014ac090:     	str	x21, [sp]
1014ac094:     	mov	x1, x20
1014ac098:     	mov	x2, x25
1014ac09c:     	mov	x3, x24
1014ac0a0:     	mov	x4, x19
1014ac0a4:     	mov	x5, x23
1014ac0a8:     	mov	x6, x22
1014ac0ac:     	mov	x7, x19
1014ac0b0:     	bl	0x10127a4e0 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
1014ac0b4:     	ldr	w1, [sp, #0x18]
1014ac0b8:     	b	0x1014ac254 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d4>
1014ac0bc:     	ldr	w27, [x20, #0x188]
1014ac0c0:     	mov	x0, x20
1014ac0c4:     	mov	x1, x27
1014ac0c8:     	mov	x2, x24
1014ac0cc:     	mov	x3, x19
1014ac0d0:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac0d4:     	cmp	w0, w27
1014ac0d8:     	b.ne	0x1014ac170 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f0>
1014ac0dc:     	mov	x0, x20
1014ac0e0:     	mov	x1, x27
1014ac0e4:     	mov	x2, x26
1014ac0e8:     	mov	x3, x19
1014ac0ec:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac0f0:     	cmp	w0, w27
1014ac0f4:     	b.ne	0x1014ac170 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f0>
1014ac0f8:     	mov	x0, x20
1014ac0fc:     	mov	x1, x24
1014ac100:     	mov	x2, x19
1014ac104:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac108:     	tbz	w0, #0x0, 0x1014ac178 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f8>
1014ac10c:     	mov	x0, x20
1014ac110:     	mov	x1, x25
1014ac114:     	mov	x2, x24
1014ac118:     	mov	x3, x19
1014ac11c:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac120:     	mov	w24, w0
1014ac124:     	mov	x0, x20
1014ac128:     	mov	x1, x22
1014ac12c:     	mov	x2, x19
1014ac130:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac134:     	tbnz	w0, #0x0, 0x1014ac1c4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x344>
1014ac138:     	ldr	w2, [x20, #0x188]
1014ac13c:     	mov	x0, x20
1014ac140:     	mov	w1, #0x8                ; =8
1014ac144:     	mov	x3, x23
1014ac148:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1014ac14c:     	mov	x1, x0
1014ac150:     	mov	x0, x20
1014ac154:     	mov	x2, x22
1014ac158:     	mov	x3, x19
1014ac15c:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac160:     	mov	x1, x0
1014ac164:     	mov	x0, x20
1014ac168:     	bl	0x1011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
1014ac16c:     	b	0x1014ac1d8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x358>
1014ac170:     	mov	x0, #0x0                ; =0
1014ac174:     	b	0x1014ac258 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d8>
1014ac178:     	ldr	w2, [x20, #0x188]
1014ac17c:     	mov	x0, x20
1014ac180:     	mov	w1, #0x8                ; =8
1014ac184:     	mov	x3, x25
1014ac188:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1014ac18c:     	mov	x1, x0
1014ac190:     	mov	x0, x20
1014ac194:     	mov	x2, x24
1014ac198:     	mov	x3, x19
1014ac19c:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac1a0:     	mov	x1, x0
1014ac1a4:     	mov	x0, x20
1014ac1a8:     	bl	0x1011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
1014ac1ac:     	mov	w24, w0
1014ac1b0:     	mov	x0, x20
1014ac1b4:     	mov	x1, x22
1014ac1b8:     	mov	x2, x19
1014ac1bc:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac1c0:     	tbz	w0, #0x0, 0x1014ac138 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2b8>
1014ac1c4:     	mov	x0, x20
1014ac1c8:     	mov	x1, x23
1014ac1cc:     	mov	x2, x22
1014ac1d0:     	mov	x3, x19
1014ac1d4:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac1d8:     	mov	w2, w0
1014ac1dc:     	mov	x0, x20
1014ac1e0:     	mov	x1, x24
1014ac1e4:     	mov	x3, x21
1014ac1e8:     	bl	0x1014ac7f4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps7relprodB8_>
1014ac1ec:     	mov	x21, x0
1014ac1f0:     	mov	x0, x20
1014ac1f4:     	mov	x1, x26
1014ac1f8:     	mov	x2, x19
1014ac1fc:     	bl	0x10116026c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1014ac200:     	tbz	w0, #0x0, 0x1014ac21c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x39c>
1014ac204:     	mov	x0, x20
1014ac208:     	mov	x1, x21
1014ac20c:     	mov	x2, x26
1014ac210:     	mov	x3, x19
1014ac214:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac218:     	b	0x1014ac250 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d0>
1014ac21c:     	ldr	w2, [x20, #0x188]
1014ac220:     	mov	x0, x20
1014ac224:     	mov	w1, #0x8                ; =8
1014ac228:     	mov	x3, x21
1014ac22c:     	bl	0x101171fc0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
1014ac230:     	mov	x1, x0
1014ac234:     	mov	x0, x20
1014ac238:     	mov	x2, x26
1014ac23c:     	mov	x3, x19
1014ac240:     	bl	0x1011734a0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1014ac244:     	mov	x1, x0
1014ac248:     	mov	x0, x20
1014ac24c:     	bl	0x1011607a8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
1014ac250:     	mov	w1, w0
1014ac254:     	mov	w0, #0x1                ; =1
1014ac258:     	ldp	x29, x30, [sp, #0x100]
1014ac25c:     	ldp	x20, x19, [sp, #0xf0]
1014ac260:     	ldp	x22, x21, [sp, #0xe0]
1014ac264:     	ldp	x24, x23, [sp, #0xd0]
1014ac268:     	ldp	x26, x25, [sp, #0xc0]
1014ac26c:     	ldp	x28, x27, [sp, #0xb0]
1014ac270:     	add	sp, sp, #0x110
1014ac274:     	ret
1014ac278:     	adrp	x0, 0x10199f000 <dyld_stub_binder+0x10199f000>
1014ac27c:     	add	x0, x0, #0xb2d
1014ac280:     	adrp	x2, 0x101b7b000 <dyld_stub_binder+0x101b7b000>
1014ac284:     	add	x2, x2, #0xea0
1014ac288:     	mov	w1, #0x39               ; =57
1014ac28c:     	bl	0x1018c1788 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1014ac290:     	adrp	x0, 0x10199d000 <dyld_stub_binder+0x10199d000>
1014ac294:     	add	x0, x0, #0xa02
1014ac298:     	adrp	x2, 0x101b7b000 <dyld_stub_binder+0x101b7b000>
1014ac29c:     	add	x2, x2, #0xe88
1014ac2a0:     	mov	w1, #0x3d               ; =61
1014ac2a4:     	bl	0x1018c1788 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1014ac2a8:     	adrp	x5, 0x101b7b000 <dyld_stub_binder+0x101b7b000>
1014ac2ac:     	add	x5, x5, #0xeb8
1014ac2b0:     	sub	x1, x29, #0x58
1014ac2b4:     	add	x2, sp, #0x18
1014ac2b8:     	mov	w0, #0x0                ; =0
1014ac2bc:     	mov	x3, #0x0                ; =0
1014ac2c0:     	bl	0x1018c16a0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1014ac2c4:     	adrp	x2, 0x101a3e000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x17d7>
1014ac2c8:     	add	x2, x2, #0xa48
1014ac2cc:     	adrp	x5, 0x101b76000 <dyld_stub_binder+0x101b76000>
1014ac2d0:     	add	x5, x5, #0xc30
1014ac2d4:     	add	x1, sp, #0x18
1014ac2d8:     	mov	w0, #0x0                ; =0
1014ac2dc:     	mov	x3, #0x0                ; =0
1014ac2e0:     	bl	0x1018c16a0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
