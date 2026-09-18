
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000101650f74 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_>:
101650f74:     	sub	sp, sp, #0x110
101650f78:     	stp	x28, x27, [sp, #0xb0]
101650f7c:     	stp	x26, x25, [sp, #0xc0]
101650f80:     	stp	x24, x23, [sp, #0xd0]
101650f84:     	stp	x22, x21, [sp, #0xe0]
101650f88:     	stp	x20, x19, [sp, #0xf0]
101650f8c:     	stp	x29, x30, [sp, #0x100]
101650f90:     	add	x29, sp, #0x100
101650f94:     	ldr	x8, [x0, #0x108]
101650f98:     	cmp	w8, #0x3e
101650f9c:     	and	x9, x8, #0x3f
101650fa0:     	ccmp	x3, x9, #0x0, ls
101650fa4:     	b.ne	0x101651380 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x40c>
101650fa8:     	mov	x21, x7
101650fac:     	mov	x22, x5
101650fb0:     	mov	x23, x4
101650fb4:     	mov	x19, x3
101650fb8:     	mov	x24, x2
101650fbc:     	mov	x25, x1
101650fc0:     	mov	x20, x0
101650fc4:     	lsl	x10, x3, #2
101650fc8:     	mov	x9, #0x0                ; =0
101650fcc:     	cbz	x3, 0x101651000 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x8c>
101650fd0:     	mov	w11, #0x1               ; =1
101650fd4:     	mov	x12, x10
101650fd8:     	mov	x13, x24
101650fdc:     	ldr	w14, [x13], #0x4
101650fe0:     	cmp	w14, w8
101650fe4:     	b.hs	0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
101650fe8:     	lsr	x15, x9, x14
101650fec:     	tbnz	w15, #0x0, 0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
101650ff0:     	lsl	x14, x11, x14
101650ff4:     	orr	x9, x14, x9
101650ff8:     	subs	x12, x12, #0x4
101650ffc:     	b.ne	0x101650fdc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x68>
101651000:     	stur	x9, [x29, #-0x58]
101651004:     	mov	x11, #-0x1              ; =-1
101651008:     	lsl	x11, x11, x19
10165100c:     	mvn	x11, x11
101651010:     	str	x11, [sp, #0x18]
101651014:     	cmp	x9, x11
101651018:     	b.ne	0x101651398 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x424>
10165101c:     	cmp	x6, x19
101651020:     	b.ne	0x101651380 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x40c>
101651024:     	mov	x11, #0x0               ; =0
101651028:     	cbz	x19, 0x10165105c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xe8>
10165102c:     	mov	w12, #0x1               ; =1
101651030:     	mov	x13, x10
101651034:     	mov	x14, x22
101651038:     	ldr	w15, [x14], #0x4
10165103c:     	cmp	w15, w8
101651040:     	b.hs	0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
101651044:     	lsr	x16, x11, x15
101651048:     	tbnz	w16, #0x0, 0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
10165104c:     	lsl	x15, x12, x15
101651050:     	orr	x11, x15, x11
101651054:     	subs	x13, x13, #0x4
101651058:     	b.ne	0x101651038 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0xc4>
10165105c:     	stur	x11, [x29, #-0x58]
101651060:     	str	x9, [sp, #0x18]
101651064:     	cmp	x11, x9
101651068:     	b.ne	0x101651398 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x424>
10165106c:     	ldr	x11, [x29, #0x18]
101651070:     	cmp	x11, x19
101651074:     	b.ne	0x101651380 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x40c>
101651078:     	ldr	x26, [x29, #0x10]
10165107c:     	mov	x11, #0x0               ; =0
101651080:     	cbz	x19, 0x1016510b0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x13c>
101651084:     	mov	w12, #0x1               ; =1
101651088:     	mov	x13, x26
10165108c:     	ldr	w14, [x13], #0x4
101651090:     	cmp	w14, w8
101651094:     	b.hs	0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
101651098:     	lsr	x15, x11, x14
10165109c:     	tbnz	w15, #0x0, 0x101651368 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3f4>
1016510a0:     	lsl	x14, x12, x14
1016510a4:     	orr	x11, x14, x11
1016510a8:     	subs	x10, x10, #0x4
1016510ac:     	b.ne	0x10165108c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x118>
1016510b0:     	stur	x11, [x29, #-0x58]
1016510b4:     	str	x9, [sp, #0x18]
1016510b8:     	cmp	x11, x9
1016510bc:     	b.ne	0x101651398 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x424>
1016510c0:     	lsr	x8, x21, x8
1016510c4:     	str	x8, [sp, #0x18]
1016510c8:     	cbnz	x8, 0x1016513b4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x440>
1016510cc:     	cbz	x21, 0x10165113c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1c8>
1016510d0:     	ldr	w8, [x20, #0x188]
1016510d4:     	cmp	w8, #0x1
1016510d8:     	b.eq	0x10165113c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x1c8>
1016510dc:     	ldr	w8, [x20, #0x128]
1016510e0:     	cbz	w8, 0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
1016510e4:     	ldr	w8, [x20, #0x12c]
1016510e8:     	add	w9, w8, w8, lsl #1
1016510ec:     	lsr	x9, x21, x9
1016510f0:     	cbnz	x9, 0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
1016510f4:     	mov	x9, #-0x1               ; =-1
1016510f8:     	lsl	x9, x9, x8
1016510fc:     	bic	x10, x21, x9
101651100:     	add	x11, x10, #0x1
101651104:     	tst	x11, x10
101651108:     	b.ne	0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
10165110c:     	and	x10, x8, #0x3f
101651110:     	lsr	x10, x21, x10
101651114:     	bic	x10, x10, x9
101651118:     	add	x11, x10, #0x1
10165111c:     	tst	x11, x10
101651120:     	b.ne	0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
101651124:     	ubfiz	w8, w8, #1, #5
101651128:     	lsr	x8, x21, x8
10165112c:     	bic	x8, x8, x9
101651130:     	add	x9, x8, #0x1
101651134:     	tst	x9, x8
101651138:     	b.ne	0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
10165113c:     	mov	x0, x20
101651140:     	mov	x1, x24
101651144:     	mov	x2, x19
101651148:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
10165114c:     	tbz	w0, #0x0, 0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
101651150:     	mov	x0, x20
101651154:     	mov	x1, x22
101651158:     	mov	x2, x19
10165115c:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101651160:     	cbz	w0, 0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
101651164:     	mov	x0, x20
101651168:     	mov	x1, x26
10165116c:     	mov	x2, x19
101651170:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101651174:     	cbz	w0, 0x1016511ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x238>
101651178:     	stp	x26, x19, [sp, #0x8]
10165117c:     	add	x0, sp, #0x18
101651180:     	str	x21, [sp]
101651184:     	mov	x1, x20
101651188:     	mov	x2, x25
10165118c:     	mov	x3, x24
101651190:     	mov	x4, x19
101651194:     	mov	x5, x23
101651198:     	mov	x6, x22
10165119c:     	mov	x7, x19
1016511a0:     	bl	0x101411320 <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm9_E23mapped_product_observedBb_>
1016511a4:     	ldr	w1, [sp, #0x18]
1016511a8:     	b	0x101651344 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d0>
1016511ac:     	ldr	w27, [x20, #0x188]
1016511b0:     	mov	x0, x20
1016511b4:     	mov	x1, x27
1016511b8:     	mov	x2, x24
1016511bc:     	mov	x3, x19
1016511c0:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016511c4:     	cmp	w0, w27
1016511c8:     	b.ne	0x101651260 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2ec>
1016511cc:     	mov	x0, x20
1016511d0:     	mov	x1, x27
1016511d4:     	mov	x2, x26
1016511d8:     	mov	x3, x19
1016511dc:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016511e0:     	cmp	w0, w27
1016511e4:     	b.ne	0x101651260 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2ec>
1016511e8:     	mov	x0, x20
1016511ec:     	mov	x1, x24
1016511f0:     	mov	x2, x19
1016511f4:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016511f8:     	tbz	w0, #0x0, 0x101651268 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2f4>
1016511fc:     	mov	x0, x20
101651200:     	mov	x1, x25
101651204:     	mov	x2, x24
101651208:     	mov	x3, x19
10165120c:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101651210:     	mov	w24, w0
101651214:     	mov	x0, x20
101651218:     	mov	x1, x22
10165121c:     	mov	x2, x19
101651220:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
101651224:     	tbnz	w0, #0x0, 0x1016512b4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x340>
101651228:     	ldr	w2, [x20, #0x188]
10165122c:     	mov	x0, x20
101651230:     	mov	w1, #0x8                ; =8
101651234:     	mov	x3, x23
101651238:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
10165123c:     	mov	x1, x0
101651240:     	mov	x0, x20
101651244:     	mov	x2, x22
101651248:     	mov	x3, x19
10165124c:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101651250:     	mov	x1, x0
101651254:     	mov	x0, x20
101651258:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
10165125c:     	b	0x1016512c8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x354>
101651260:     	mov	x0, #0x0                ; =0
101651264:     	b	0x101651348 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3d4>
101651268:     	ldr	w2, [x20, #0x188]
10165126c:     	mov	x0, x20
101651270:     	mov	w1, #0x8                ; =8
101651274:     	mov	x3, x25
101651278:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
10165127c:     	mov	x1, x0
101651280:     	mov	x0, x20
101651284:     	mov	x2, x24
101651288:     	mov	x3, x19
10165128c:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101651290:     	mov	x1, x0
101651294:     	mov	x0, x20
101651298:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
10165129c:     	mov	w24, w0
1016512a0:     	mov	x0, x20
1016512a4:     	mov	x1, x22
1016512a8:     	mov	x2, x19
1016512ac:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016512b0:     	tbz	w0, #0x0, 0x101651228 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x2b4>
1016512b4:     	mov	x0, x20
1016512b8:     	mov	x1, x23
1016512bc:     	mov	x2, x22
1016512c0:     	mov	x3, x19
1016512c4:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
1016512c8:     	mov	w2, w0
1016512cc:     	mov	x0, x20
1016512d0:     	mov	x1, x24
1016512d4:     	mov	x3, x21
1016512d8:     	bl	0x1016515e8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps7relprodB8_>
1016512dc:     	mov	x21, x0
1016512e0:     	mov	x0, x20
1016512e4:     	mov	x1, x26
1016512e8:     	mov	x2, x19
1016512ec:     	bl	0x1012f1e2c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E8commutesB6_>
1016512f0:     	tbz	w0, #0x0, 0x10165130c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x398>
1016512f4:     	mov	x0, x20
1016512f8:     	mov	x1, x21
1016512fc:     	mov	x2, x26
101651300:     	mov	x3, x19
101651304:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101651308:     	b	0x101651340 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB4_10RetractionKm9_Kb1_ENtNtB6_7carrier9RegionOps18preserving_productB8_+0x3cc>
10165130c:     	ldr	w2, [x20, #0x188]
101651310:     	mov	x0, x20
101651314:     	mov	w1, #0x8                ; =8
101651318:     	mov	x3, x21
10165131c:     	bl	0x101303b80 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5applyB6_>
101651320:     	mov	x1, x0
101651324:     	mov	x0, x20
101651328:     	mov	x2, x26
10165132c:     	mov	x3, x19
101651330:     	bl	0x101305060 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6renameB6_>
101651334:     	mov	x1, x0
101651338:     	mov	x0, x20
10165133c:     	bl	0x1012f2368 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab10retractionINtB2_10RetractionKm9_E9normalizeB6_>
101651340:     	mov	w1, w0
101651344:     	mov	w0, #0x1                ; =1
101651348:     	ldp	x29, x30, [sp, #0x100]
10165134c:     	ldp	x20, x19, [sp, #0xf0]
101651350:     	ldp	x22, x21, [sp, #0xe0]
101651354:     	ldp	x24, x23, [sp, #0xd0]
101651358:     	ldp	x26, x25, [sp, #0xc0]
10165135c:     	ldp	x28, x27, [sp, #0xb0]
101651360:     	add	sp, sp, #0x110
101651364:     	ret
101651368:     	adrp	x0, 0x101b4c000 <dyld_stub_binder+0x101b4c000>
10165136c:     	add	x0, x0, #0xff0
101651370:     	adrp	x2, 0x101d30000 <dyld_stub_binder+0x101d30000>
101651374:     	add	x2, x2, #0x30
101651378:     	mov	w1, #0x39               ; =57
10165137c:     	bl	0x101a66248 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
101651380:     	adrp	x0, 0x101b4a000 <dyld_stub_binder+0x101b4a000>
101651384:     	add	x0, x0, #0xe72
101651388:     	adrp	x2, 0x101d30000 <dyld_stub_binder+0x101d30000>
10165138c:     	add	x2, x2, #0x18
101651390:     	mov	w1, #0x3d               ; =61
101651394:     	bl	0x101a66248 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
101651398:     	adrp	x5, 0x101d30000 <dyld_stub_binder+0x101d30000>
10165139c:     	add	x5, x5, #0x48
1016513a0:     	sub	x1, x29, #0x58
1016513a4:     	add	x2, sp, #0x18
1016513a8:     	mov	w0, #0x0                ; =0
1016513ac:     	mov	x3, #0x0                ; =0
1016513b0:     	bl	0x101a66160 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1016513b4:     	adrp	x2, 0x101beb000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x12ef>
1016513b8:     	add	x2, x2, #0xf30
1016513bc:     	adrp	x5, 0x101d2a000 <dyld_stub_binder+0x101d2a000>
1016513c0:     	add	x5, x5, #0xdc0
1016513c4:     	add	x1, sp, #0x18
1016513c8:     	mov	w0, #0x0                ; =0
1016513cc:     	mov	x3, #0x0                ; =0
1016513d0:     	bl	0x101a66160 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
