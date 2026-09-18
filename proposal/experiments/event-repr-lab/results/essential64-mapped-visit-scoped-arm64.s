
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001008a30bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>:
1008a30bc:     	stp	x28, x27, [sp, #-0x60]!
1008a30c0:     	stp	x26, x25, [sp, #0x10]
1008a30c4:     	stp	x24, x23, [sp, #0x20]
1008a30c8:     	stp	x22, x21, [sp, #0x30]
1008a30cc:     	stp	x20, x19, [sp, #0x40]
1008a30d0:     	stp	x29, x30, [sp, #0x50]
1008a30d4:     	add	x29, sp, #0x50
1008a30d8:     	sub	sp, sp, #0x1c0
1008a30dc:     	mov	x23, x2
1008a30e0:     	mov	x21, x1
1008a30e4:     	mov	x28, x0
1008a30e8:     	ldrb	w8, [x0, #0x151]
1008a30ec:     	str	x0, [sp, #0x88]
1008a30f0:     	str	x2, [sp, #0x60]
1008a30f4:     	cbz	w8, 0x1008a341c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x360>
1008a30f8:     	mov	x27, #0x0               ; =0
1008a30fc:     	b	0x1008a3118 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5c>
1008a3100:     	ldr	w9, [x26, #0x14]
1008a3104:     	add	x27, x27, #0x18
1008a3108:     	stp	xzr, x20, [x26]
1008a310c:     	stp	w24, w9, [x26, #0x10]
1008a3110:     	cmp	x27, #0x30
1008a3114:     	b.eq	0x1008a341c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x360>
1008a3118:     	add	x26, x23, x27
1008a311c:     	ldp	x19, x20, [x26]
1008a3120:     	ldr	w24, [x26, #0x10]
1008a3124:     	cbz	x19, 0x1008a3100 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x44>
1008a3128:     	ldr	x8, [x28, #0x138]
1008a312c:     	add	x8, x8, #0x1
1008a3130:     	str	x8, [x28, #0x138]
1008a3134:     	ldur	x8, [x21, #0x40]
1008a3138:     	lsr	x0, x24, #1
1008a313c:     	cmn	x8, #0x1
1008a3140:     	str	w9, [sp, #0x70]
1008a3144:     	b.eq	0x1008a3160 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa4>
1008a3148:     	ldr	x1, [x21, #0x50]
1008a314c:     	cmp	x1, x0
1008a3150:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a3154:     	ldr	x8, [x21, #0x48]
1008a3158:     	add	x8, x8, x0, lsl #4
1008a315c:     	b	0x1008a3178 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbc>
1008a3160:     	ldr	x1, [x21, #0x58]
1008a3164:     	cmp	x1, x0
1008a3168:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a316c:     	ldr	x8, [x21, #0x50]
1008a3170:     	add	x8, x8, x0, lsl #5
1008a3174:     	add	x8, x8, #0x18
1008a3178:     	mov	x25, #0x0               ; =0
1008a317c:     	ldr	x8, [x8]
1008a3180:     	bic	x8, x8, x19
1008a3184:     	str	x8, [sp, #0x78]
1008a3188:     	mov	w8, #0x4                ; =4
1008a318c:     	stp	xzr, x8, [sp, #0xf0]
1008a3190:     	str	xzr, [sp, #0x100]
1008a3194:     	mov	w9, #0x4                ; =4
1008a3198:     	mov	w8, #0x4                ; =4
1008a319c:     	b	0x1008a31c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x108>
1008a31a0:     	rbit	x9, x19
1008a31a4:     	clz	x9, x9
1008a31a8:     	str	w9, [x8, x25, lsl #2]
1008a31ac:     	add	x25, x25, #0x1
1008a31b0:     	str	x25, [sp, #0x100]
1008a31b4:     	sub	x10, x19, #0x1
1008a31b8:     	add	x9, x23, #0x4
1008a31bc:     	ands	x19, x10, x19
1008a31c0:     	b.eq	0x1008a31e4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x128>
1008a31c4:     	mov	x23, x9
1008a31c8:     	ldr	x9, [sp, #0xf0]
1008a31cc:     	cmp	x25, x9
1008a31d0:     	b.ne	0x1008a31a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe4>
1008a31d4:     	add	x0, sp, #0xf0
1008a31d8:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a31dc:     	ldr	x8, [sp, #0xf8]
1008a31e0:     	b	0x1008a31a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe4>
1008a31e4:     	ldp	x9, x8, [sp, #0xf0]
1008a31e8:     	str	x9, [sp, #0x80]
1008a31ec:     	str	x8, [sp, #0x68]
1008a31f0:     	cbz	x25, 0x1008a3364 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2a8>
1008a31f4:     	ldr	x19, [x28, #0x140]
1008a31f8:     	mov	x28, x8
1008a31fc:     	b	0x1008a3234 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x178>
1008a3200:     	tst	w22, #0x1
1008a3204:     	mov	w8, #0x8                ; =8
1008a3208:     	mov	w9, #0xc                ; =12
1008a320c:     	csel	x8, x9, x8, ne
1008a3210:     	add	x9, sp, #0xf0
1008a3214:     	ldr	w8, [x9, x8]
1008a3218:     	and	w9, w24, #0x1
1008a321c:     	eor	w24, w8, w9
1008a3220:     	add	x19, x19, #0x1
1008a3224:     	ldr	x8, [sp, #0x88]
1008a3228:     	str	x19, [x8, #0x140]
1008a322c:     	subs	x23, x23, #0x4
1008a3230:     	b.eq	0x1008a3364 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2a8>
1008a3234:     	ldr	w25, [x28], #0x4
1008a3238:     	ldur	x8, [x21, #0x40]
1008a323c:     	lsr	w0, w24, #1
1008a3240:     	cmn	x8, #0x1
1008a3244:     	b.eq	0x1008a3274 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1b8>
1008a3248:     	ldr	x1, [x21, #0x50]
1008a324c:     	cmp	x1, x0
1008a3250:     	b.ls	0x1008a43f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x133c>
1008a3254:     	ldr	x9, [x21, #0x48]
1008a3258:     	add	x9, x9, x0, lsl #4
1008a325c:     	ldr	x10, [x9]
1008a3260:     	mov	w9, #0x1                ; =1
1008a3264:     	lsl	x9, x9, x25
1008a3268:     	tst	x10, x9
1008a326c:     	b.ne	0x1008a329c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1e0>
1008a3270:     	b	0x1008a322c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
1008a3274:     	ldr	x1, [x21, #0x58]
1008a3278:     	cmp	x1, x0
1008a327c:     	b.ls	0x1008a4414 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1358>
1008a3280:     	ldr	x1, [x21, #0x50]
1008a3284:     	add	x9, x1, x0, lsl #5
1008a3288:     	ldr	x10, [x9, #0x18]!
1008a328c:     	mov	w9, #0x1                ; =1
1008a3290:     	lsl	x9, x9, x25
1008a3294:     	tst	x10, x9
1008a3298:     	b.eq	0x1008a322c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x170>
1008a329c:     	ldr	w10, [x21, #0xf0]
1008a32a0:     	cmp	w25, w10
1008a32a4:     	b.hs	0x1008a35ec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x530>
1008a32a8:     	cmn	x8, #0x1
1008a32ac:     	b.eq	0x1008a32d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x214>
1008a32b0:     	cmp	x1, x0
1008a32b4:     	b.ls	0x1008a4404 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1348>
1008a32b8:     	ldr	x8, [x21, #0x48]
1008a32bc:     	add	x8, x8, x0, lsl #4
1008a32c0:     	ldr	x8, [x8]
1008a32c4:     	tst	x8, x9
1008a32c8:     	b.ne	0x1008a32f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x234>
1008a32cc:     	b	0x1008a3220 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1008a32d0:     	ldr	x8, [x21, #0x58]
1008a32d4:     	cmp	x8, x0
1008a32d8:     	b.ls	0x1008a4440 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1384>
1008a32dc:     	add	x8, x1, x0, lsl #5
1008a32e0:     	add	x8, x8, #0x18
1008a32e4:     	ldr	x8, [x8]
1008a32e8:     	tst	x8, x9
1008a32ec:     	b.eq	0x1008a3220 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1008a32f0:     	and	x8, x25, #0x3f
1008a32f4:     	lsr	x22, x20, x8
1008a32f8:     	ldrb	w8, [x21, #0xf5]
1008a32fc:     	tbz	w8, #0x0, 0x1008a3348 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x28c>
1008a3300:     	add	x0, sp, #0xf0
1008a3304:     	add	x1, x21, #0x40
1008a3308:     	mov	x2, x24
1008a330c:     	bl	0x100ee9280 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
1008a3310:     	ldr	w8, [sp, #0xf0]
1008a3314:     	cmp	w8, #0x2
1008a3318:     	b.ne	0x1008a3328 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x26c>
1008a331c:     	ldr	w8, [sp, #0xf4]
1008a3320:     	cmp	w8, w25
1008a3324:     	b.eq	0x1008a3200 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x144>
1008a3328:     	and	w1, w24, #0xfffffffe
1008a332c:     	and	w3, w22, #0x1
1008a3330:     	mov	x0, x21
1008a3334:     	mov	x2, x25
1008a3338:     	bl	0x100df57c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E14cofactor_innerB6_>
1008a333c:     	and	w8, w24, #0x1
1008a3340:     	eor	w24, w0, w8
1008a3344:     	b	0x1008a3220 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1008a3348:     	and	w3, w22, #0x1
1008a334c:     	mov	x0, x21
1008a3350:     	mov	x1, x24
1008a3354:     	mov	x2, x25
1008a3358:     	bl	0x100df57c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E14cofactor_innerB6_>
1008a335c:     	mov	x24, x0
1008a3360:     	b	0x1008a3220 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x164>
1008a3364:     	ldr	x8, [sp, #0x80]
1008a3368:     	cbz	x8, 0x1008a3374 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2b8>
1008a336c:     	ldr	x0, [sp, #0x68]
1008a3370:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3374:     	ldur	x8, [x21, #0x40]
1008a3378:     	lsr	w0, w24, #1
1008a337c:     	cmn	x8, #0x1
1008a3380:     	ldr	x28, [sp, #0x88]
1008a3384:     	ldr	x23, [sp, #0x60]
1008a3388:     	ldr	x10, [sp, #0x78]
1008a338c:     	b.eq	0x1008a33b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x2fc>
1008a3390:     	ldr	x1, [x21, #0x50]
1008a3394:     	cmp	x1, x0
1008a3398:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a339c:     	ldr	x8, [x21, #0x48]
1008a33a0:     	add	x8, x8, x0, lsl #4
1008a33a4:     	ldr	x8, [x8]
1008a33a8:     	bics	x9, x8, x10
1008a33ac:     	str	x9, [sp, #0xf0]
1008a33b0:     	b.eq	0x1008a33e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x324>
1008a33b4:     	b	0x1008a43d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1314>
1008a33b8:     	ldr	x1, [x21, #0x58]
1008a33bc:     	cmp	x1, x0
1008a33c0:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a33c4:     	ldr	x8, [x21, #0x50]
1008a33c8:     	add	x8, x8, x0, lsl #5
1008a33cc:     	add	x8, x8, #0x18
1008a33d0:     	ldr	x8, [x8]
1008a33d4:     	bics	x9, x8, x10
1008a33d8:     	str	x9, [sp, #0xf0]
1008a33dc:     	b.ne	0x1008a43d0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1314>
1008a33e0:     	mov	x20, #0x0               ; =0
1008a33e4:     	bic	x8, x10, x8
1008a33e8:     	fmov	d0, x8
1008a33ec:     	cnt.8b	v0, v0
1008a33f0:     	addv.8b	b0, v0
1008a33f4:     	fmov	x8, d0
1008a33f8:     	ldr	x9, [x28, #0x148]
1008a33fc:     	add	x8, x9, x8
1008a3400:     	str	x8, [x28, #0x148]
1008a3404:     	ldr	w9, [sp, #0x70]
1008a3408:     	add	x27, x27, #0x18
1008a340c:     	stp	xzr, x20, [x26]
1008a3410:     	stp	w24, w9, [x26, #0x10]
1008a3414:     	cmp	x27, #0x30
1008a3418:     	b.ne	0x1008a3118 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5c>
1008a341c:     	ldr	w9, [x23, #0x10]
1008a3420:     	cbz	w9, 0x1008a40c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x100c>
1008a3424:     	ldr	w10, [x23, #0x28]
1008a3428:     	cbz	w10, 0x1008a40c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x100c>
1008a342c:     	cmp	w9, #0x1
1008a3430:     	ccmp	w10, #0x1, #0x0, eq
1008a3434:     	b.eq	0x1008a3558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x49c>
1008a3438:     	ldr	x8, [x28, #0x88]
1008a343c:     	cbz	x8, 0x1008a3560 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4a4>
1008a3440:     	mov	x8, #0x0                ; =0
1008a3444:     	mov	x15, #0xa9c5            ; =43461
1008a3448:     	movk	x15, #0x2e62, lsl #16
1008a344c:     	movk	x15, #0x7aea, lsl #32
1008a3450:     	movk	x15, #0xf135, lsl #48
1008a3454:     	ldp	x11, x12, [x23]
1008a3458:     	madd	x13, x9, x15, x11
1008a345c:     	mov	x14, #0x6332            ; =25394
1008a3460:     	movk	x14, #0x6ed3, lsl #16
1008a3464:     	movk	x14, #0x765a, lsl #32
1008a3468:     	movk	x14, #0x284f, lsl #48
1008a346c:     	mul	x14, x14, x15
1008a3470:     	madd	x13, x13, x15, x14
1008a3474:     	add	x13, x13, x12
1008a3478:     	madd	x16, x13, x15, x10
1008a347c:     	ldp	x13, x14, [x23, #0x18]
1008a3480:     	madd	x16, x16, x15, x13
1008a3484:     	madd	x16, x16, x15, x14
1008a3488:     	mul	x15, x16, x15
1008a348c:     	ror	x0, x15, #0x2c
1008a3490:     	lsr	x17, x0, #57
1008a3494:     	ldp	x16, x15, [x28, #0x70]
1008a3498:     	dup.8b	v0, w17
1008a349c:     	movi.2d	v1, #0xffffffffffffffff
1008a34a0:     	mov	w17, #0x38              ; =56
1008a34a4:     	and	x0, x0, x15
1008a34a8:     	ldr	d2, [x16, x0]
1008a34ac:     	cmeq.8b	v3, v2, v0
1008a34b0:     	fmov	x1, d3
1008a34b4:     	ands	x1, x1, #0x8080808080808080
1008a34b8:     	b.eq	0x1008a3528 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x46c>
1008a34bc:     	rbit	x2, x1
1008a34c0:     	clz	x2, x2
1008a34c4:     	add	x2, x0, x2, lsr #3
1008a34c8:     	and	x2, x2, x15
1008a34cc:     	mneg	x2, x2, x17
1008a34d0:     	add	x2, x16, x2
1008a34d4:     	ldur	x3, [x2, #-0x38]
1008a34d8:     	cmp	x11, x3
1008a34dc:     	b.ne	0x1008a351c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1008a34e0:     	ldur	x3, [x2, #-0x30]
1008a34e4:     	cmp	x12, x3
1008a34e8:     	b.ne	0x1008a351c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1008a34ec:     	ldur	w3, [x2, #-0x28]
1008a34f0:     	cmp	w9, w3
1008a34f4:     	b.ne	0x1008a351c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1008a34f8:     	ldur	x3, [x2, #-0x20]
1008a34fc:     	cmp	x13, x3
1008a3500:     	b.ne	0x1008a351c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1008a3504:     	ldur	x3, [x2, #-0x18]
1008a3508:     	cmp	x14, x3
1008a350c:     	b.ne	0x1008a351c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x460>
1008a3510:     	ldur	w3, [x2, #-0x10]
1008a3514:     	cmp	w10, w3
1008a3518:     	b.eq	0x1008a35d8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x51c>
1008a351c:     	sub	x2, x1, #0x2
1008a3520:     	ands	x1, x2, x1
1008a3524:     	b.ne	0x1008a34bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x400>
1008a3528:     	cmeq.8b	v2, v2, v1
1008a352c:     	fmov	x1, d2
1008a3530:     	cbnz	x1, 0x1008a3560 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x4a4>
1008a3534:     	add	x8, x8, #0x8
1008a3538:     	add	x0, x0, x8
1008a353c:     	and	x0, x0, x15
1008a3540:     	ldr	d2, [x16, x0]
1008a3544:     	cmeq.8b	v3, v2, v0
1008a3548:     	fmov	x1, d3
1008a354c:     	ands	x1, x1, #0x8080808080808080
1008a3550:     	b.ne	0x1008a34bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x400>
1008a3554:     	b	0x1008a3528 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x46c>
1008a3558:     	mov	w0, #0x1                ; =1
1008a355c:     	b	0x1008a40cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1008a3560:     	mov	x24, x28
1008a3564:     	ldr	x8, [x24, #0xc8]!
1008a3568:     	add	x8, x8, #0x1
1008a356c:     	str	x8, [x24]
1008a3570:     	mov	w11, #0x8481            ; =33921
1008a3574:     	movk	w11, #0x1e, lsl #16
1008a3578:     	cmp	x8, x11
1008a357c:     	b.hs	0x1008a4428 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x136c>
1008a3580:     	ldr	x12, [x23]
1008a3584:     	ldr	x13, [x23, #0x18]
1008a3588:     	ldr	x8, [x21, #0x40]
1008a358c:     	cmn	x8, #0x1
1008a3590:     	b.eq	0x1008a3608 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x54c>
1008a3594:     	ldr	x1, [x21, #0x50]
1008a3598:     	lsr	x0, x9, #1
1008a359c:     	cmp	x1, x0
1008a35a0:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a35a4:     	lsr	x8, x10, #1
1008a35a8:     	cmp	x1, x8
1008a35ac:     	b.ls	0x1008a44f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1434>
1008a35b0:     	ldr	x11, [x21, #0x48]
1008a35b4:     	lsl	x14, x0, #4
1008a35b8:     	ldr	x14, [x11, x14]
1008a35bc:     	bic	x19, x14, x12
1008a35c0:     	add	x8, x11, x8, lsl #4
1008a35c4:     	ldr	x15, [x8]
1008a35c8:     	ldp	x11, x1, [x28, #0x18]
1008a35cc:     	mov	x22, #0x0               ; =0
1008a35d0:     	cbnz	x19, 0x1008a3648 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x58c>
1008a35d4:     	b	0x1008a3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5bc>
1008a35d8:     	ldur	w0, [x2, #-0x8]
1008a35dc:     	ldr	x8, [x28, #0xd0]
1008a35e0:     	add	x8, x8, #0x1
1008a35e4:     	str	x8, [x28, #0xd0]
1008a35e8:     	b	0x1008a40cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1008a35ec:     	adrp	x0, 0x1015d1000 <dyld_stub_binder+0x1015d1000>
1008a35f0:     	add	x0, x0, #0x1d7
1008a35f4:     	adrp	x2, 0x101795000 <dyld_stub_binder+0x101795000>
1008a35f8:     	add	x2, x2, #0xda8
1008a35fc:     	mov	w1, #0x2c               ; =44
1008a3600:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1008a3604:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a3608:     	ldr	x1, [x21, #0x58]
1008a360c:     	lsr	x0, x9, #1
1008a3610:     	cmp	x1, x0
1008a3614:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a3618:     	lsr	x8, x10, #1
1008a361c:     	cmp	x1, x8
1008a3620:     	b.ls	0x1008a4510 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1454>
1008a3624:     	ldr	x11, [x21, #0x50]
1008a3628:     	add	x14, x11, x0, lsl #5
1008a362c:     	ldr	x14, [x14, #0x18]
1008a3630:     	bic	x19, x14, x12
1008a3634:     	add	x8, x11, x8, lsl #5
1008a3638:     	ldr	x15, [x8, #0x18]!
1008a363c:     	ldp	x11, x1, [x28, #0x18]
1008a3640:     	mov	x22, #0x0               ; =0
1008a3644:     	cbz	x19, 0x1008a3678 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5bc>
1008a3648:     	mov	w8, #0x1                ; =1
1008a364c:     	mov	x14, x19
1008a3650:     	rbit	x16, x14
1008a3654:     	clz	x0, x16
1008a3658:     	cmp	x0, x1
1008a365c:     	b.hs	0x1008a4470 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13b4>
1008a3660:     	ldr	w16, [x11, x0, lsl #2]
1008a3664:     	lsl	x16, x8, x16
1008a3668:     	orr	x22, x16, x22
1008a366c:     	sub	x16, x14, #0x1
1008a3670:     	ands	x14, x16, x14
1008a3674:     	b.ne	0x1008a3650 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x594>
1008a3678:     	ldp	x14, x8, [x28, #0x48]
1008a367c:     	bic	x15, x15, x13
1008a3680:     	cbz	x15, 0x1008a36b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5fc>
1008a3684:     	mov	x16, #0x0               ; =0
1008a3688:     	mov	w17, #0x1               ; =1
1008a368c:     	rbit	x0, x15
1008a3690:     	clz	x0, x0
1008a3694:     	cmp	x0, x8
1008a3698:     	b.hs	0x1008a447c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13c0>
1008a369c:     	ldr	w0, [x14, x0, lsl #2]
1008a36a0:     	lsl	x0, x17, x0
1008a36a4:     	orr	x16, x0, x16
1008a36a8:     	sub	x0, x15, #0x1
1008a36ac:     	ands	x15, x0, x15
1008a36b0:     	b.ne	0x1008a368c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x5d0>
1008a36b4:     	orr	x22, x16, x22
1008a36b8:     	eor	w9, w10, w9
1008a36bc:     	cmp	x12, x13
1008a36c0:     	ccmp	w9, #0x1, #0x0, eq
1008a36c4:     	b.ne	0x1008a37a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1008a36c8:     	ldr	x9, [x23, #0x8]
1008a36cc:     	ldr	x10, [x23, #0x20]
1008a36d0:     	cmp	x9, x10
1008a36d4:     	b.ne	0x1008a37a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1008a36d8:     	mov	w16, #0x4               ; =4
1008a36dc:     	stp	xzr, x16, [sp, #0xf0]
1008a36e0:     	str	xzr, [sp, #0x100]
1008a36e4:     	mov	x20, #0x0               ; =0
1008a36e8:     	cbz	x19, 0x1008a3748 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x68c>
1008a36ec:     	mov	w8, #0x4                ; =4
1008a36f0:     	b	0x1008a3718 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x65c>
1008a36f4:     	ldr	x8, [sp, #0xf8]
1008a36f8:     	rbit	x9, x19
1008a36fc:     	clz	x9, x9
1008a3700:     	str	w9, [x8, x20, lsl #2]
1008a3704:     	add	x20, x20, #0x1
1008a3708:     	str	x20, [sp, #0x100]
1008a370c:     	sub	x9, x19, #0x1
1008a3710:     	ands	x19, x9, x19
1008a3714:     	b.eq	0x1008a3730 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x674>
1008a3718:     	ldr	x9, [sp, #0xf0]
1008a371c:     	cmp	x20, x9
1008a3720:     	b.ne	0x1008a36f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x63c>
1008a3724:     	add	x0, sp, #0xf0
1008a3728:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a372c:     	b	0x1008a36f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x638>
1008a3730:     	ldp	x9, x16, [sp, #0xf0]
1008a3734:     	ldp	x14, x8, [x28, #0x48]
1008a3738:     	ldp	x11, x1, [x28, #0x18]
1008a373c:     	cmp	x9, #0x0
1008a3740:     	cset	w19, eq
1008a3744:     	b	0x1008a374c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x690>
1008a3748:     	mov	w19, #0x1               ; =1
1008a374c:     	mov	x9, #0x0                ; =0
1008a3750:     	lsl	x10, x20, #2
1008a3754:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a3758:     	add	x2, x2, #0xbe8
1008a375c:     	adrp	x12, 0x101759000 <dyld_stub_binder+0x101759000>
1008a3760:     	add	x12, x12, #0xc00
1008a3764:     	cmp	x10, x9
1008a3768:     	b.eq	0x1008a40c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1008>
1008a376c:     	ldr	w0, [x16, x9]
1008a3770:     	cmp	x1, x0
1008a3774:     	b.ls	0x1008a448c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13d0>
1008a3778:     	cmp	x8, x0
1008a377c:     	b.ls	0x1008a4494 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13d8>
1008a3780:     	ldr	w13, [x11, x0, lsl #2]
1008a3784:     	ldr	w15, [x14, x0, lsl #2]
1008a3788:     	add	x9, x9, #0x4
1008a378c:     	cmp	w13, w15
1008a3790:     	b.eq	0x1008a3764 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6a8>
1008a3794:     	tbnz	w19, #0x0, 0x1008a37a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x6e4>
1008a3798:     	mov	x0, x16
1008a379c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a37a0:     	fmov	d0, x22
1008a37a4:     	cnt.8b	v0, v0
1008a37a8:     	addv.8b	b0, v0
1008a37ac:     	fmov	x19, d0
1008a37b0:     	cmp	x19, #0x7
1008a37b4:     	b.hs	0x1008a3b64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xaa8>
1008a37b8:     	add	x8, x28, #0x10
1008a37bc:     	str	x8, [sp, #0x50]
1008a37c0:     	ldr	x8, [x28, #0xd8]
1008a37c4:     	add	x8, x8, #0x1
1008a37c8:     	str	x8, [x28, #0xd8]
1008a37cc:     	ldr	w8, [x28]
1008a37d0:     	tbz	w8, #0x0, 0x1008a3bc4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb08>
1008a37d4:     	str	x19, [sp, #0x8]
1008a37d8:     	mov	x26, #0x0               ; =0
1008a37dc:     	ldr	x10, [x28, #0x8]
1008a37e0:     	add	x8, x28, #0x90
1008a37e4:     	str	x8, [sp, #0x48]
1008a37e8:     	lsl	x9, x10, #6
1008a37ec:     	tst	x10, #0xfc00000000000000
1008a37f0:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1008a37f4:     	ccmp	x9, x8, #0x2, eq
1008a37f8:     	cset	w8, hi
1008a37fc:     	str	w8, [sp, #0x14]
1008a3800:     	stp	x10, x24, [sp, #0x28]
1008a3804:     	sub	x8, x10, #0x1
1008a3808:     	stp	x9, x8, [sp, #0x18]
1008a380c:     	mov	w20, #0xff              ; =255
1008a3810:     	mov	w8, #0x1                ; =1
1008a3814:     	b	0x1008a387c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x7c0>
1008a3818:     	strb	w23, [x28]
1008a381c:     	strb	w10, [x28, #0x1]
1008a3820:     	str	w25, [x28, #0x4]
1008a3824:     	stp	x9, x27, [x28, #0x8]
1008a3828:     	ldp	x8, x9, [sp, #0x70]
1008a382c:     	stp	x19, x9, [x28, #0x18]
1008a3830:     	str	x8, [x28, #0x28]
1008a3834:     	ldr	w8, [sp, #0x58]
1008a3838:     	stp	w25, w8, [x28, #0x30]
1008a383c:     	str	x22, [x28, #0x38]
1008a3840:     	ldr	x28, [sp, #0x88]
1008a3844:     	ldr	x23, [sp, #0x60]
1008a3848:     	ldr	w13, [sp, #0x80]
1008a384c:     	mov	w8, #0x0                ; =0
1008a3850:     	ldr	x9, [x28, #0x130]
1008a3854:     	ldp	x2, x10, [x28, #0x98]
1008a3858:     	add	x10, x10, x2, lsl #6
1008a385c:     	ldp	x11, x12, [x28, #0xb0]
1008a3860:     	add	x10, x12, x10
1008a3864:     	add	x10, x10, x11, lsl #6
1008a3868:     	cmp	x10, x9
1008a386c:     	csel	x9, x10, x9, hi
1008a3870:     	str	x9, [x28, #0x130]
1008a3874:     	mov	w26, #0x1               ; =1
1008a3878:     	tbz	w13, #0x0, 0x1008a42ec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1230>
1008a387c:     	mov	x13, x8
1008a3880:     	add	x8, x26, x26, lsl #1
1008a3884:     	lsl	x8, x8, #3
1008a3888:     	add	x9, x23, x8
1008a388c:     	ldr	q0, [x9]
1008a3890:     	str	q0, [sp, #0xc0]
1008a3894:     	ldr	x25, [x9, #0x10]
1008a3898:     	str	x25, [sp, #0xd0]
1008a389c:     	cmp	w25, #0x2
1008a38a0:     	b.lo	0x1008a384c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x790>
1008a38a4:     	str	w13, [sp, #0x80]
1008a38a8:     	ldr	x9, [sp, #0x48]
1008a38ac:     	add	x24, x9, x8
1008a38b0:     	ldrb	w27, [x28, #0x150]
1008a38b4:     	ldr	x8, [x24, #0x8]
1008a38b8:     	cbz	x8, 0x1008a38c4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x808>
1008a38bc:     	ldr	x0, [x24]
1008a38c0:     	b	0x1008a3948 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x88c>
1008a38c4:     	ldp	x9, x28, [sp, #0x20]
1008a38c8:     	eor	x8, x28, x9
1008a38cc:     	cmp	x8, x9
1008a38d0:     	b.ls	0x1008a4458 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x139c>
1008a38d4:     	ldr	x19, [sp, #0x18]
1008a38d8:     	ldr	w8, [sp, #0x14]
1008a38dc:     	cbnz	w8, 0x1008a3ef8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe3c>
1008a38e0:     	cbz	x19, 0x1008a38f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x83c>
1008a38e4:     	mov	x0, x19
1008a38e8:     	mov	w1, #0x8                ; =8
1008a38ec:     	bl	0x1013fa450 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1008a38f0:     	cbnz	x0, 0x1008a38fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x840>
1008a38f4:     	b	0x1008a4530 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1474>
1008a38f8:     	mov	w0, #0x8                ; =8
1008a38fc:     	mov	x8, x0
1008a3900:     	mov	x9, x28
1008a3904:     	cmp	x28, #0x4
1008a3908:     	b.hs	0x1008a391c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x860>
1008a390c:     	strb	w20, [x8], #0x40
1008a3910:     	subs	x9, x9, #0x1
1008a3914:     	b.ne	0x1008a390c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x850>
1008a3918:     	b	0x1008a3940 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x884>
1008a391c:     	add	x8, x0, #0x80
1008a3920:     	and	x9, x28, #0x3fffffffffffffc
1008a3924:     	sturb	w20, [x8, #-0x80]
1008a3928:     	sturb	w20, [x8, #-0x40]
1008a392c:     	strb	w20, [x8]
1008a3930:     	strb	w20, [x8, #0x40]
1008a3934:     	add	x8, x8, #0x100
1008a3938:     	subs	x9, x9, #0x4
1008a393c:     	b.ne	0x1008a3924 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x868>
1008a3940:     	stp	x0, x28, [x24]
1008a3944:     	mov	x8, x28
1008a3948:     	mov	w9, w25
1008a394c:     	ldp	x11, x12, [sp, #0xc0]
1008a3950:     	ldr	w19, [sp, #0xd4]
1008a3954:     	mov	x10, #0xa9c5            ; =43461
1008a3958:     	movk	x10, #0x2e62, lsl #16
1008a395c:     	movk	x10, #0x7aea, lsl #32
1008a3960:     	movk	x10, #0xf135, lsl #48
1008a3964:     	stp	x12, x11, [sp, #0x70]
1008a3968:     	madd	x9, x9, x10, x11
1008a396c:     	madd	x9, x9, x10, x12
1008a3970:     	madd	x9, x9, x10, x22
1008a3974:     	mul	x9, x9, x10
1008a3978:     	sub	x8, x8, #0x1
1008a397c:     	and	x8, x8, x9, ror #44
1008a3980:     	add	x28, x0, x8, lsl #6
1008a3984:     	ldrb	w8, [x28]
1008a3988:     	cmp	w8, #0xff
1008a398c:     	b.ne	0x1008a39b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x8f4>
1008a3990:     	ldr	x9, [sp, #0x88]
1008a3994:     	ldr	x8, [x9, #0x120]
1008a3998:     	add	x8, x8, #0x1
1008a399c:     	str	x8, [x9, #0x120]
1008a39a0:     	add	x8, x28, #0x10
1008a39a4:     	str	x8, [sp, #0x38]
1008a39a8:     	add	x23, x28, #0x18
1008a39ac:     	b	0x1008a3a54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x998>
1008a39b0:     	ldr	x9, [x28, #0x38]
1008a39b4:     	cmp	x9, x22
1008a39b8:     	b.ne	0x1008a39fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1008a39bc:     	ldr	x9, [x28, #0x20]
1008a39c0:     	ldr	x10, [sp, #0x78]
1008a39c4:     	cmp	x9, x10
1008a39c8:     	b.ne	0x1008a39fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1008a39cc:     	ldr	x9, [x28, #0x28]
1008a39d0:     	ldr	x10, [sp, #0x70]
1008a39d4:     	cmp	x9, x10
1008a39d8:     	b.ne	0x1008a39fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1008a39dc:     	ldr	w9, [x28, #0x30]
1008a39e0:     	cmp	w9, w25
1008a39e4:     	b.ne	0x1008a39fc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x940>
1008a39e8:     	ldr	x28, [sp, #0x88]
1008a39ec:     	ldr	x8, [x28, #0x118]
1008a39f0:     	add	x8, x8, #0x1
1008a39f4:     	str	x8, [x28, #0x118]
1008a39f8:     	b	0x1008a3848 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x78c>
1008a39fc:     	mov	x23, x28
1008a3a00:     	ldr	x9, [x23, #0x18]!
1008a3a04:     	lsl	x10, x9, #3
1008a3a08:     	cmp	w8, #0x2
1008a3a0c:     	csel	x10, x10, xzr, eq
1008a3a10:     	ldr	x11, [x24, #0x10]
1008a3a14:     	sub	x10, x11, x10
1008a3a18:     	cmp	w8, #0x2
1008a3a1c:     	mov	x8, x28
1008a3a20:     	ldr	x0, [x8, #0x10]!
1008a3a24:     	str	x8, [sp, #0x38]
1008a3a28:     	strb	w20, [x28]
1008a3a2c:     	ldr	x8, [sp, #0x88]
1008a3a30:     	ldr	q0, [x8, #0x120]
1008a3a34:     	mov	w11, #0x1               ; =1
1008a3a38:     	dup.2d	v1, x11
1008a3a3c:     	add.2d	v0, v0, v1
1008a3a40:     	str	q0, [x8, #0x120]
1008a3a44:     	str	x10, [x24, #0x10]
1008a3a48:     	ccmp	x9, #0x0, #0x4, hs
1008a3a4c:     	b.eq	0x1008a3a54 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x998>
1008a3a50:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3a54:     	ldr	x8, [sp, #0x50]
1008a3a58:     	mov	w9, #0x30               ; =48
1008a3a5c:     	madd	x3, x26, x9, x8
1008a3a60:     	add	x0, sp, #0xf0
1008a3a64:     	add	x2, sp, #0xc0
1008a3a68:     	mov	x1, x21
1008a3a6c:     	mov	x4, x22
1008a3a70:     	mov	x5, x27
1008a3a74:     	ldr	x6, [sp, #0x30]
1008a3a78:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a3a7c:     	ldr	x8, [sp, #0xf0]
1008a3a80:     	cmn	x8, #0x1
1008a3a84:     	str	w19, [sp, #0x58]
1008a3a88:     	str	x23, [sp, #0x40]
1008a3a8c:     	b.eq	0x1008a3aa4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9e8>
1008a3a90:     	cmn	x8, #0x2
1008a3a94:     	b.ne	0x1008a3ac8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa0c>
1008a3a98:     	mov	w23, #0x0               ; =0
1008a3a9c:     	ldrb	w10, [sp, #0xf8]
1008a3aa0:     	b	0x1008a3aa8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x9ec>
1008a3aa4:     	mov	w23, #0x1               ; =1
1008a3aa8:     	mov	x26, #0x0               ; =0
1008a3aac:     	ldr	x8, [x24, #0x10]
1008a3ab0:     	add	x8, x8, x26
1008a3ab4:     	str	x8, [x24, #0x10]
1008a3ab8:     	ldrb	w8, [x28]
1008a3abc:     	cmp	w8, #0x2
1008a3ac0:     	b.ne	0x1008a3818 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1008a3ac4:     	b	0x1008a3b38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa7c>
1008a3ac8:     	ldp	x27, x19, [sp, #0xf8]
1008a3acc:     	ldr	x9, [sp, #0x108]
1008a3ad0:     	lsl	x26, x19, #3
1008a3ad4:     	cmp	x8, x19
1008a3ad8:     	b.ls	0x1008a3b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa60>
1008a3adc:     	mov	x23, x9
1008a3ae0:     	str	x27, [sp, #0x68]
1008a3ae4:     	cbz	x19, 0x1008a3b0c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa50>
1008a3ae8:     	lsl	x1, x8, #3
1008a3aec:     	ldr	x0, [sp, #0x68]
1008a3af0:     	mov	w2, #0x8                ; =8
1008a3af4:     	mov	x3, x26
1008a3af8:     	bl	0x1013fa4a4 <__RNvCsiwXPDrQxTLA_7___rustc14___rust_realloc>
1008a3afc:     	mov	x27, x0
1008a3b00:     	mov	x9, x23
1008a3b04:     	cbnz	x0, 0x1008a3b1c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xa60>
1008a3b08:     	b	0x1008a453c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1480>
1008a3b0c:     	ldr	x0, [sp, #0x68]
1008a3b10:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3b14:     	mov	w27, #0x8               ; =8
1008a3b18:     	mov	x9, x23
1008a3b1c:     	mov	w23, #0x2               ; =2
1008a3b20:     	ldr	x8, [x24, #0x10]
1008a3b24:     	add	x8, x8, x26
1008a3b28:     	str	x8, [x24, #0x10]
1008a3b2c:     	ldrb	w8, [x28]
1008a3b30:     	cmp	w8, #0x2
1008a3b34:     	b.ne	0x1008a3818 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1008a3b38:     	ldr	x8, [sp, #0x40]
1008a3b3c:     	ldr	x8, [x8]
1008a3b40:     	cbz	x8, 0x1008a3818 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1008a3b44:     	ldr	x8, [sp, #0x38]
1008a3b48:     	ldr	x0, [x8]
1008a3b4c:     	mov	x24, x9
1008a3b50:     	mov	x26, x10
1008a3b54:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3b58:     	mov	x10, x26
1008a3b5c:     	mov	x9, x24
1008a3b60:     	b	0x1008a3818 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x75c>
1008a3b64:     	mov	x0, x21
1008a3b68:     	mov	x1, x22
1008a3b6c:     	bl	0x100dea100 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E3topB6_>
1008a3b70:     	ldr	q0, [x23]
1008a3b74:     	str	q0, [sp, #0xc0]
1008a3b78:     	ldr	x8, [x23, #0x10]
1008a3b7c:     	str	x8, [sp, #0xd0]
1008a3b80:     	mov	w22, w0
1008a3b84:     	ldr	x1, [x28, #0x38]
1008a3b88:     	cmp	x1, x22
1008a3b8c:     	b.ls	0x1008a44cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1410>
1008a3b90:     	ldr	x8, [x28, #0x30]
1008a3b94:     	ldr	w8, [x8, x22, lsl #2]
1008a3b98:     	ldr	w9, [sp, #0xd0]
1008a3b9c:     	ldr	x10, [x21, #0x40]
1008a3ba0:     	lsr	x0, x9, #1
1008a3ba4:     	cmn	x10, #0x1
1008a3ba8:     	b.eq	0x1008a3efc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe40>
1008a3bac:     	ldr	x1, [x21, #0x50]
1008a3bb0:     	cmp	x1, x0
1008a3bb4:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a3bb8:     	ldr	x9, [x21, #0x48]
1008a3bbc:     	add	x9, x9, x0, lsl #4
1008a3bc0:     	b	0x1008a3f14 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe58>
1008a3bc4:     	ldrb	w5, [x28, #0x150]
1008a3bc8:     	add	x0, sp, #0xc0
1008a3bcc:     	add	x3, x28, #0x10
1008a3bd0:     	mov	x1, x21
1008a3bd4:     	mov	x2, x23
1008a3bd8:     	mov	x4, x22
1008a3bdc:     	mov	x6, x24
1008a3be0:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a3be4:     	ldrb	w5, [x28, #0x150]
1008a3be8:     	add	x0, sp, #0xf0
1008a3bec:     	add	x2, x23, #0x18
1008a3bf0:     	add	x3, x28, #0x40
1008a3bf4:     	mov	x1, x21
1008a3bf8:     	mov	x4, x22
1008a3bfc:     	mov	x6, x24
1008a3c00:     	bl	0x100d06968 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
1008a3c04:     	mov	w8, #0x1                ; =1
1008a3c08:     	lsl	x9, x8, x19
1008a3c0c:     	lsr	x9, x9, #6
1008a3c10:     	cmp	x19, #0x6
1008a3c14:     	csinc	x27, x8, x9, eq
1008a3c18:     	lsl	x25, x27, #3
1008a3c1c:     	mov	x0, x25
1008a3c20:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
1008a3c24:     	cbz	x0, 0x1008a4520 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1464>
1008a3c28:     	mov	x24, x0
1008a3c2c:     	mov	x0, #0x0                ; =0
1008a3c30:     	ldp	x19, x25, [sp, #0xc0]
1008a3c34:     	ldp	x1, x9, [sp, #0xd0]
1008a3c38:     	sub	x10, x0, w25, uxtb
1008a3c3c:     	ldp	x20, x8, [sp, #0xf0]
1008a3c40:     	ldp	x11, x12, [sp, #0x100]
1008a3c44:     	mov	x26, x27
1008a3c48:     	sub	x13, x27, #0x1
1008a3c4c:     	b	0x1008a3c68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbac>
1008a3c50:     	tst	w8, #0x1
1008a3c54:     	csel	x14, x14, xzr, ne
1008a3c58:     	str	x14, [x24, x0, lsl #3]
1008a3c5c:     	cmp	x13, x0
1008a3c60:     	b.eq	0x1008a3cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1008a3c64:     	add	x0, x0, #0x1
1008a3c68:     	mov	x14, x10
1008a3c6c:     	cmn	x19, #0x2
1008a3c70:     	b.eq	0x1008a3c84 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbc8>
1008a3c74:     	cmp	x0, x1
1008a3c78:     	b.hs	0x1008a44ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13f0>
1008a3c7c:     	ldr	x14, [x25, x0, lsl #3]
1008a3c80:     	eor	x14, x9, x14
1008a3c84:     	cmn	x20, #0x2
1008a3c88:     	b.eq	0x1008a3c50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xb94>
1008a3c8c:     	cmp	x0, x11
1008a3c90:     	b.hs	0x1008a44a8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13ec>
1008a3c94:     	ldr	x15, [x8, x0, lsl #3]
1008a3c98:     	eor	x15, x12, x15
1008a3c9c:     	and	x14, x15, x14
1008a3ca0:     	str	x14, [x24, x0, lsl #3]
1008a3ca4:     	cmp	x13, x0
1008a3ca8:     	b.ne	0x1008a3c64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xba8>
1008a3cac:     	cmp	x20, #0x1
1008a3cb0:     	b.lt	0x1008a3cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1008a3cb4:     	mov	x0, x8
1008a3cb8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3cbc:     	cmp	x19, #0x1
1008a3cc0:     	b.lt	0x1008a3ccc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc10>
1008a3cc4:     	mov	x0, x25
1008a3cc8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3ccc:     	ldr	x8, [x28, #0xc0]
1008a3cd0:     	mov	w9, #0x4                ; =4
1008a3cd4:     	stp	xzr, x9, [sp, #0xf0]
1008a3cd8:     	str	xzr, [sp, #0x100]
1008a3cdc:     	ands	x20, x8, x22
1008a3ce0:     	mov	x25, x26
1008a3ce4:     	b.eq	0x1008a42b8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11fc>
1008a3ce8:     	mov	x19, #0x0               ; =0
1008a3cec:     	mov	w8, #0x4                ; =4
1008a3cf0:     	b	0x1008a3d18 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc5c>
1008a3cf4:     	ldr	x8, [sp, #0xf8]
1008a3cf8:     	rbit	x9, x20
1008a3cfc:     	clz	x9, x9
1008a3d00:     	str	w9, [x8, x19, lsl #2]
1008a3d04:     	add	x19, x19, #0x1
1008a3d08:     	str	x19, [sp, #0x100]
1008a3d0c:     	sub	x9, x20, #0x1
1008a3d10:     	ands	x20, x9, x20
1008a3d14:     	b.eq	0x1008a3d30 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc74>
1008a3d18:     	ldr	x9, [sp, #0xf0]
1008a3d1c:     	cmp	x19, x9
1008a3d20:     	b.ne	0x1008a3cf8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc3c>
1008a3d24:     	add	x0, sp, #0xf0
1008a3d28:     	bl	0x101507bdc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1008a3d2c:     	b	0x1008a3cf4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc38>
1008a3d30:     	mov	x1, x24
1008a3d34:     	ldp	x8, x20, [sp, #0xf0]
1008a3d38:     	str	x8, [sp, #0x70]
1008a3d3c:     	str	x20, [sp, #0x58]
1008a3d40:     	cbz	x19, 0x1008a4294 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11d8>
1008a3d44:     	add	x8, x20, x19, lsl #2
1008a3d48:     	str	x8, [sp, #0x78]
1008a3d4c:     	b	0x1008a3d68 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xcac>
1008a3d50:     	bic	x22, x22, x19
1008a3d54:     	mov	x25, x24
1008a3d58:     	mov	x1, x26
1008a3d5c:     	ldr	x8, [sp, #0x78]
1008a3d60:     	cmp	x20, x8
1008a3d64:     	b.eq	0x1008a429c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11e0>
1008a3d68:     	ldr	w8, [x20], #0x4
1008a3d6c:     	mov	w9, #0x1                ; =1
1008a3d70:     	lsl	x19, x9, x8
1008a3d74:     	sub	x8, x19, #0x1
1008a3d78:     	and	x8, x8, x22
1008a3d7c:     	fmov	d0, x8
1008a3d80:     	cnt.8b	v0, v0
1008a3d84:     	addv.8b	b0, v0
1008a3d88:     	fmov	w26, s0
1008a3d8c:     	fmov	d0, x22
1008a3d90:     	cnt.8b	v0, v0
1008a3d94:     	addv.8b	b0, v0
1008a3d98:     	fmov	w27, s0
1008a3d9c:     	add	x0, sp, #0xc0
1008a3da0:     	mov	x2, x25
1008a3da4:     	mov	x3, x27
1008a3da8:     	mov	x4, x26
1008a3dac:     	mov	w5, #0x0                ; =0
1008a3db0:     	mov	x24, x1
1008a3db4:     	str	x1, [sp, #0x68]
1008a3db8:     	bl	0x100f906d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1008a3dbc:     	add	x0, sp, #0xf0
1008a3dc0:     	mov	x1, x24
1008a3dc4:     	mov	x2, x25
1008a3dc8:     	mov	x3, x27
1008a3dcc:     	mov	x4, x26
1008a3dd0:     	mov	w5, #0x1                ; =1
1008a3dd4:     	bl	0x100f906d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
1008a3dd8:     	ldp	x27, x8, [sp, #0xc8]
1008a3ddc:     	ldp	x23, x0, [sp, #0xf0]
1008a3de0:     	ldr	x9, [sp, #0x100]
1008a3de4:     	cmp	x9, x8
1008a3de8:     	csel	x24, x9, x8, lo
1008a3dec:     	cbz	x24, 0x1008a3e58 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd9c>
1008a3df0:     	mov	x28, x19
1008a3df4:     	mov	x19, x0
1008a3df8:     	str	x25, [sp, #0x80]
1008a3dfc:     	lsl	x25, x24, #3
1008a3e00:     	mov	x0, x25
1008a3e04:     	bl	0x10150f2c4 <dyld_stub_binder+0x10150f2c4>
1008a3e08:     	cbz	x0, 0x1008a44bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1400>
1008a3e0c:     	mov	x26, x0
1008a3e10:     	cmp	x24, #0x8
1008a3e14:     	mov	x0, x19
1008a3e18:     	mov	x8, #0x0                ; =0
1008a3e1c:     	b.hs	0x1008a3e88 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xdcc>
1008a3e20:     	ldr	x25, [sp, #0x80]
1008a3e24:     	mov	x19, x28
1008a3e28:     	lsl	x11, x8, #3
1008a3e2c:     	add	x9, x27, x11
1008a3e30:     	add	x10, x0, x11
1008a3e34:     	add	x11, x26, x11
1008a3e38:     	sub	x8, x24, x8
1008a3e3c:     	ldr	x12, [x10], #0x8
1008a3e40:     	ldr	x13, [x9], #0x8
1008a3e44:     	orr	x12, x13, x12
1008a3e48:     	str	x12, [x11], #0x8
1008a3e4c:     	subs	x8, x8, #0x1
1008a3e50:     	b.ne	0x1008a3e3c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd80>
1008a3e54:     	b	0x1008a3e5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda0>
1008a3e58:     	mov	w26, #0x8               ; =8
1008a3e5c:     	cbz	x23, 0x1008a3e64 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda8>
1008a3e60:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3e64:     	cbz	x25, 0x1008a3e70 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xdb4>
1008a3e68:     	ldr	x0, [sp, #0x68]
1008a3e6c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3e70:     	ldr	x8, [sp, #0xc0]
1008a3e74:     	ldr	x28, [sp, #0x88]
1008a3e78:     	cbz	x8, 0x1008a3d50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc94>
1008a3e7c:     	mov	x0, x27
1008a3e80:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a3e84:     	b	0x1008a3d50 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc94>
1008a3e88:     	sub	x9, x0, x26
1008a3e8c:     	cmn	x9, #0x40
1008a3e90:     	ldr	x25, [sp, #0x80]
1008a3e94:     	b.hi	0x1008a3e24 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd68>
1008a3e98:     	sub	x9, x27, x26
1008a3e9c:     	cmn	x9, #0x40
1008a3ea0:     	mov	x19, x28
1008a3ea4:     	b.hi	0x1008a3e28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd6c>
1008a3ea8:     	and	x8, x24, #0xffffffffffffff8
1008a3eac:     	add	x9, x27, #0x20
1008a3eb0:     	add	x10, x0, #0x20
1008a3eb4:     	add	x11, x26, #0x20
1008a3eb8:     	and	x12, x24, #0xffffffffffffff8
1008a3ebc:     	ldp	q0, q1, [x10, #-0x20]
1008a3ec0:     	ldp	q2, q3, [x10], #0x40
1008a3ec4:     	ldp	q4, q5, [x9, #-0x20]
1008a3ec8:     	ldp	q6, q7, [x9], #0x40
1008a3ecc:     	orr.16b	v0, v4, v0
1008a3ed0:     	orr.16b	v1, v5, v1
1008a3ed4:     	orr.16b	v2, v6, v2
1008a3ed8:     	orr.16b	v3, v7, v3
1008a3edc:     	stp	q0, q1, [x11, #-0x20]
1008a3ee0:     	stp	q2, q3, [x11], #0x40
1008a3ee4:     	subs	x12, x12, #0x8
1008a3ee8:     	b.ne	0x1008a3ebc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe00>
1008a3eec:     	cmp	x24, x8
1008a3ef0:     	b.ne	0x1008a3e28 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xd6c>
1008a3ef4:     	b	0x1008a3e5c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xda0>
1008a3ef8:     	bl	0x101506610 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1008a3efc:     	ldr	x1, [x21, #0x58]
1008a3f00:     	cmp	x1, x0
1008a3f04:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a3f08:     	ldr	x9, [x21, #0x50]
1008a3f0c:     	add	x9, x9, x0, lsl #5
1008a3f10:     	add	x9, x9, #0x18
1008a3f14:     	ldr	x9, [x9]
1008a3f18:     	mov	w10, #0x1               ; =1
1008a3f1c:     	lsl	x8, x10, x8
1008a3f20:     	tst	x9, x8
1008a3f24:     	b.eq	0x1008a3f38 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xe7c>
1008a3f28:     	ldp	x9, x10, [sp, #0xc0]
1008a3f2c:     	orr	x9, x9, x8
1008a3f30:     	bic	x8, x10, x8
1008a3f34:     	stp	x9, x8, [sp, #0xc0]
1008a3f38:     	sub	x0, x29, #0x70
1008a3f3c:     	add	x1, sp, #0xc0
1008a3f40:     	mov	x2, x21
1008a3f44:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a3f48:     	ldur	q0, [x29, #-0x70]
1008a3f4c:     	stur	q0, [x29, #-0x90]
1008a3f50:     	ldur	x8, [x29, #-0x60]
1008a3f54:     	stur	q0, [x29, #-0xb0]
1008a3f58:     	str	q0, [sp, #0x90]
1008a3f5c:     	str	x8, [sp, #0xa0]
1008a3f60:     	ldr	q0, [sp, #0x90]
1008a3f64:     	str	x8, [sp, #0x100]
1008a3f68:     	str	q0, [sp, #0xf0]
1008a3f6c:     	ldur	q0, [x23, #0x18]
1008a3f70:     	str	q0, [sp, #0xc0]
1008a3f74:     	ldur	x8, [x23, #0x28]
1008a3f78:     	str	x8, [sp, #0xd0]
1008a3f7c:     	ldr	x1, [x28, #0x68]
1008a3f80:     	cmp	x1, x22
1008a3f84:     	b.ls	0x1008a44cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1410>
1008a3f88:     	ldr	x8, [x28, #0x60]
1008a3f8c:     	ldr	w8, [x8, x22, lsl #2]
1008a3f90:     	ldr	w9, [sp, #0xd0]
1008a3f94:     	ldr	x10, [x21, #0x40]
1008a3f98:     	lsr	x0, x9, #1
1008a3f9c:     	cmn	x10, #0x1
1008a3fa0:     	b.eq	0x1008a3fbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf00>
1008a3fa4:     	ldr	x1, [x21, #0x50]
1008a3fa8:     	cmp	x1, x0
1008a3fac:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a3fb0:     	ldr	x9, [x21, #0x48]
1008a3fb4:     	add	x9, x9, x0, lsl #4
1008a3fb8:     	b	0x1008a3fd4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf18>
1008a3fbc:     	ldr	x1, [x21, #0x58]
1008a3fc0:     	cmp	x1, x0
1008a3fc4:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a3fc8:     	ldr	x9, [x21, #0x50]
1008a3fcc:     	add	x9, x9, x0, lsl #5
1008a3fd0:     	add	x9, x9, #0x18
1008a3fd4:     	ldr	x9, [x9]
1008a3fd8:     	mov	w10, #0x1               ; =1
1008a3fdc:     	lsl	x8, x10, x8
1008a3fe0:     	tst	x9, x8
1008a3fe4:     	b.eq	0x1008a3ff8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xf3c>
1008a3fe8:     	ldp	x9, x10, [sp, #0xc0]
1008a3fec:     	orr	x9, x9, x8
1008a3ff0:     	bic	x8, x10, x8
1008a3ff4:     	stp	x9, x8, [sp, #0xc0]
1008a3ff8:     	sub	x0, x29, #0x70
1008a3ffc:     	add	x1, sp, #0xc0
1008a4000:     	mov	x2, x21
1008a4004:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a4008:     	ldur	q0, [x29, #-0x70]
1008a400c:     	stur	q0, [x29, #-0x90]
1008a4010:     	ldur	x8, [x29, #-0x60]
1008a4014:     	stur	q0, [x29, #-0xb0]
1008a4018:     	str	q0, [sp, #0x90]
1008a401c:     	str	x8, [sp, #0xa0]
1008a4020:     	ldr	q0, [sp, #0x90]
1008a4024:     	str	x8, [sp, #0x118]
1008a4028:     	add	x8, sp, #0x9
1008a402c:     	stur	q0, [x8, #0xff]
1008a4030:     	ldp	q0, q1, [sp, #0xf0]
1008a4034:     	ldr	q2, [sp, #0x110]
1008a4038:     	stp	q1, q2, [sp, #0xa0]
1008a403c:     	str	q0, [sp, #0x90]
1008a4040:     	add	x2, sp, #0x90
1008a4044:     	mov	x0, x28
1008a4048:     	mov	x1, x21
1008a404c:     	bl	0x1008a30bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
1008a4050:     	mov	x23, x0
1008a4054:     	cmp	w0, #0x1
1008a4058:     	b.ne	0x1008a4070 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xfb4>
1008a405c:     	ldr	x8, [x28, #0xc0]
1008a4060:     	lsr	x8, x8, x22
1008a4064:     	tbz	w8, #0x0, 0x1008a4070 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xfb4>
1008a4068:     	mov	w19, #0x1               ; =1
1008a406c:     	b	0x1008a427c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11c0>
1008a4070:     	ldr	x8, [sp, #0x60]
1008a4074:     	ldr	q0, [x8]
1008a4078:     	stur	q0, [x29, #-0x70]
1008a407c:     	ldr	x8, [x8, #0x10]
1008a4080:     	stur	x8, [x29, #-0x60]
1008a4084:     	ldr	x1, [x28, #0x38]
1008a4088:     	cmp	x1, x22
1008a408c:     	b.ls	0x1008a4500 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1444>
1008a4090:     	ldr	x8, [x28, #0x30]
1008a4094:     	ldr	w8, [x8, x22, lsl #2]
1008a4098:     	ldur	w9, [x29, #-0x60]
1008a409c:     	ldr	x10, [x21, #0x40]
1008a40a0:     	lsr	x0, x9, #1
1008a40a4:     	cmn	x10, #0x1
1008a40a8:     	b.eq	0x1008a40ec <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1030>
1008a40ac:     	ldr	x1, [x21, #0x50]
1008a40b0:     	cmp	x1, x0
1008a40b4:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a40b8:     	ldr	x9, [x21, #0x48]
1008a40bc:     	add	x9, x9, x0, lsl #4
1008a40c0:     	b	0x1008a4104 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1048>
1008a40c4:     	tbz	w19, #0x0, 0x1008a4284 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11c8>
1008a40c8:     	mov	w0, #0x0                ; =0
1008a40cc:     	add	sp, sp, #0x1c0
1008a40d0:     	ldp	x29, x30, [sp, #0x50]
1008a40d4:     	ldp	x20, x19, [sp, #0x40]
1008a40d8:     	ldp	x22, x21, [sp, #0x30]
1008a40dc:     	ldp	x24, x23, [sp, #0x20]
1008a40e0:     	ldp	x26, x25, [sp, #0x10]
1008a40e4:     	ldp	x28, x27, [sp], #0x60
1008a40e8:     	ret
1008a40ec:     	ldr	x1, [x21, #0x58]
1008a40f0:     	cmp	x1, x0
1008a40f4:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a40f8:     	ldr	x9, [x21, #0x50]
1008a40fc:     	add	x9, x9, x0, lsl #5
1008a4100:     	add	x9, x9, #0x18
1008a4104:     	ldr	x9, [x9]
1008a4108:     	mov	w10, #0x1               ; =1
1008a410c:     	lsl	x8, x10, x8
1008a4110:     	tst	x9, x8
1008a4114:     	b.eq	0x1008a4128 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x106c>
1008a4118:     	ldur	q0, [x29, #-0x70]
1008a411c:     	dup.2d	v1, x8
1008a4120:     	orr.16b	v0, v0, v1
1008a4124:     	stur	q0, [x29, #-0x70]
1008a4128:     	sub	x0, x29, #0xb0
1008a412c:     	sub	x1, x29, #0x70
1008a4130:     	mov	x2, x21
1008a4134:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a4138:     	ldur	q0, [x29, #-0xb0]
1008a413c:     	stur	q0, [x29, #-0xd0]
1008a4140:     	ldur	x8, [x29, #-0xa0]
1008a4144:     	stur	q0, [x29, #-0xf0]
1008a4148:     	stur	q0, [x29, #-0x90]
1008a414c:     	stur	x8, [x29, #-0x80]
1008a4150:     	ldur	q0, [x29, #-0x90]
1008a4154:     	str	x8, [sp, #0x100]
1008a4158:     	str	q0, [sp, #0xf0]
1008a415c:     	ldr	x8, [sp, #0x60]
1008a4160:     	ldur	q0, [x8, #0x18]
1008a4164:     	stur	q0, [x29, #-0x70]
1008a4168:     	ldur	x8, [x8, #0x28]
1008a416c:     	stur	x8, [x29, #-0x60]
1008a4170:     	ldr	x1, [x28, #0x68]
1008a4174:     	cmp	x1, x22
1008a4178:     	b.ls	0x1008a4500 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1444>
1008a417c:     	ldr	x8, [x28, #0x60]
1008a4180:     	ldr	w8, [x8, x22, lsl #2]
1008a4184:     	ldur	w9, [x29, #-0x60]
1008a4188:     	ldr	x10, [x21, #0x40]
1008a418c:     	lsr	x0, x9, #1
1008a4190:     	cmn	x10, #0x1
1008a4194:     	b.eq	0x1008a41b0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x10f4>
1008a4198:     	ldr	x1, [x21, #0x50]
1008a419c:     	cmp	x1, x0
1008a41a0:     	b.ls	0x1008a44f4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1438>
1008a41a4:     	ldr	x9, [x21, #0x48]
1008a41a8:     	add	x9, x9, x0, lsl #4
1008a41ac:     	b	0x1008a41c8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x110c>
1008a41b0:     	ldr	x1, [x21, #0x58]
1008a41b4:     	cmp	x1, x0
1008a41b8:     	b.ls	0x1008a4514 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1458>
1008a41bc:     	ldr	x9, [x21, #0x50]
1008a41c0:     	add	x9, x9, x0, lsl #5
1008a41c4:     	add	x9, x9, #0x18
1008a41c8:     	and	w19, w22, #0x3f
1008a41cc:     	ldr	x9, [x9]
1008a41d0:     	mov	w10, #0x1               ; =1
1008a41d4:     	lsl	x8, x10, x8
1008a41d8:     	tst	x9, x8
1008a41dc:     	b.eq	0x1008a41f0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1134>
1008a41e0:     	ldur	q0, [x29, #-0x70]
1008a41e4:     	dup.2d	v1, x8
1008a41e8:     	orr.16b	v0, v0, v1
1008a41ec:     	stur	q0, [x29, #-0x70]
1008a41f0:     	sub	x0, x29, #0xb0
1008a41f4:     	sub	x1, x29, #0x70
1008a41f8:     	mov	x2, x21
1008a41fc:     	bl	0x10084092c <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
1008a4200:     	ldur	q0, [x29, #-0xb0]
1008a4204:     	stur	q0, [x29, #-0xd0]
1008a4208:     	ldur	x8, [x29, #-0xa0]
1008a420c:     	stur	q0, [x29, #-0xf0]
1008a4210:     	stur	q0, [x29, #-0x90]
1008a4214:     	stur	x8, [x29, #-0x80]
1008a4218:     	ldur	q0, [x29, #-0x90]
1008a421c:     	str	x8, [sp, #0x118]
1008a4220:     	add	x8, sp, #0x9
1008a4224:     	stur	q0, [x8, #0xff]
1008a4228:     	ldp	q0, q1, [sp, #0xf0]
1008a422c:     	ldr	q2, [sp, #0x110]
1008a4230:     	stp	q1, q2, [sp, #0xd0]
1008a4234:     	str	q0, [sp, #0xc0]
1008a4238:     	add	x2, sp, #0xc0
1008a423c:     	mov	x0, x28
1008a4240:     	mov	x1, x21
1008a4244:     	bl	0x1008a30bc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_>
1008a4248:     	mov	x3, x0
1008a424c:     	ldr	x8, [x28, #0xc0]
1008a4250:     	mov	x0, x21
1008a4254:     	lsr	x8, x8, x19
1008a4258:     	tbz	w8, #0x0, 0x1008a426c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11b0>
1008a425c:     	mov	w1, #0xe                ; =14
1008a4260:     	mov	x2, x23
1008a4264:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1008a4268:     	b	0x1008a4278 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11bc>
1008a426c:     	mov	x1, x22
1008a4270:     	mov	x2, x23
1008a4274:     	bl	0x100df6c0c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6branchB6_>
1008a4278:     	mov	x19, x0
1008a427c:     	ldr	x23, [sp, #0x60]
1008a4280:     	b	0x1008a42d4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1218>
1008a4284:     	mov	x0, x16
1008a4288:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a428c:     	mov	w0, #0x0                ; =0
1008a4290:     	b	0x1008a40cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1008a4294:     	mov	x24, x25
1008a4298:     	mov	x26, x1
1008a429c:     	ldr	x8, [sp, #0x70]
1008a42a0:     	cbz	x8, 0x1008a42ac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x11f0>
1008a42a4:     	ldr	x0, [sp, #0x58]
1008a42a8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a42ac:     	mov	x25, x24
1008a42b0:     	mov	x24, x26
1008a42b4:     	ldr	x23, [sp, #0x60]
1008a42b8:     	stp	x25, x24, [sp, #0xf0]
1008a42bc:     	str	x25, [sp, #0x100]
1008a42c0:     	add	x2, sp, #0xf0
1008a42c4:     	mov	x0, x21
1008a42c8:     	mov	x1, x22
1008a42cc:     	bl	0x100df6624 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5tableB6_>
1008a42d0:     	mov	x19, x0
1008a42d4:     	add	x0, x28, #0x70
1008a42d8:     	mov	x1, x23
1008a42dc:     	mov	x2, x19
1008a42e0:     	bl	0x100e8ddf0 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product10Restrictedj2_mNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBW_>
1008a42e4:     	mov	x0, x19
1008a42e8:     	b	0x1008a40cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1010>
1008a42ec:     	ldr	x1, [x28, #0x90]
1008a42f0:     	add	x0, sp, #0xc0
1008a42f4:     	mov	x3, x21
1008a42f8:     	mov	x4, x23
1008a42fc:     	mov	x5, x22
1008a4300:     	bl	0x10089f2b8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1008a4304:     	ldp	x1, x2, [x28, #0xa8]
1008a4308:     	add	x0, sp, #0xf0
1008a430c:     	add	x4, x23, #0x18
1008a4310:     	mov	x3, x21
1008a4314:     	mov	x5, x22
1008a4318:     	bl	0x10089f2b8 <__RINvMs5_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10PlaneCache4readKm1_EBc_>
1008a431c:     	mov	w8, #0x1                ; =1
1008a4320:     	ldr	x10, [sp, #0x8]
1008a4324:     	lsl	x9, x8, x10
1008a4328:     	lsr	x9, x9, #6
1008a432c:     	cmp	x10, #0x6
1008a4330:     	csinc	x27, x8, x9, eq
1008a4334:     	lsl	x25, x27, #3
1008a4338:     	mov	x0, x25
1008a433c:     	mov	w1, #0x8                ; =8
1008a4340:     	bl	0x1013fa450 <__RNvCsiwXPDrQxTLA_7___rustc12___rust_alloc>
1008a4344:     	cbz	x0, 0x1008a454c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1490>
1008a4348:     	mov	x24, x0
1008a434c:     	mov	x0, #0x0                ; =0
1008a4350:     	ldp	x19, x25, [sp, #0xc0]
1008a4354:     	ldp	x1, x9, [sp, #0xd0]
1008a4358:     	sub	x10, x0, w25, uxtb
1008a435c:     	ldp	x20, x8, [sp, #0xf0]
1008a4360:     	ldp	x11, x12, [sp, #0x100]
1008a4364:     	mov	x26, x27
1008a4368:     	sub	x13, x27, #0x1
1008a436c:     	b	0x1008a4388 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12cc>
1008a4370:     	tst	w8, #0x1
1008a4374:     	csel	x14, x14, xzr, ne
1008a4378:     	str	x14, [x24, x0, lsl #3]
1008a437c:     	cmp	x13, x0
1008a4380:     	b.eq	0x1008a3cbc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xc00>
1008a4384:     	add	x0, x0, #0x1
1008a4388:     	mov	x14, x10
1008a438c:     	cmn	x19, #0x2
1008a4390:     	b.eq	0x1008a43a4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12e8>
1008a4394:     	cmp	x0, x1
1008a4398:     	b.hs	0x1008a44e0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1424>
1008a439c:     	ldr	x14, [x25, x0, lsl #3]
1008a43a0:     	eor	x14, x9, x14
1008a43a4:     	cmn	x20, #0x2
1008a43a8:     	b.eq	0x1008a4370 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12b4>
1008a43ac:     	cmp	x0, x11
1008a43b0:     	b.hs	0x1008a44dc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1420>
1008a43b4:     	ldr	x15, [x8, x0, lsl #3]
1008a43b8:     	eor	x15, x12, x15
1008a43bc:     	and	x14, x15, x14
1008a43c0:     	str	x14, [x24, x0, lsl #3]
1008a43c4:     	cmp	x13, x0
1008a43c8:     	b.ne	0x1008a4384 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x12c8>
1008a43cc:     	b	0x1008a3cac <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0xbf0>
1008a43d0:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
1008a43d4:     	add	x2, x2, #0x268
1008a43d8:     	adrp	x3, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a43dc:     	add	x3, x3, #0x2d9
1008a43e0:     	adrp	x5, 0x101751000 <dyld_stub_binder+0x101751000>
1008a43e4:     	add	x5, x5, #0xed8
1008a43e8:     	add	x1, sp, #0xf0
1008a43ec:     	mov	w0, #0x0                ; =0
1008a43f0:     	mov	w4, #0x43               ; =67
1008a43f4:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1008a43f8:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a43fc:     	add	x2, x2, #0x5e0
1008a4400:     	b	0x1008a441c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1360>
1008a4404:     	ldr	x20, [sp, #0x68]
1008a4408:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a440c:     	add	x2, x2, #0x5e0
1008a4410:     	b	0x1008a4450 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1394>
1008a4414:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a4418:     	add	x2, x2, #0x5c8
1008a441c:     	ldr	x20, [sp, #0x68]
1008a4420:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a4424:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a4428:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a442c:     	add	x0, x0, #0x46f
1008a4430:     	adrp	x2, 0x101753000 <dyld_stub_binder+0x101753000>
1008a4434:     	add	x2, x2, #0x188
1008a4438:     	mov	w1, #0x51               ; =81
1008a443c:     	bl	0x101506c74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1008a4440:     	mov	x1, x8
1008a4444:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a4448:     	add	x2, x2, #0x5c8
1008a444c:     	ldr	x20, [sp, #0x68]
1008a4450:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a4454:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a4458:     	adrp	x0, 0x1015b1000 <dyld_stub_binder+0x1015b1000>
1008a445c:     	add	x0, x0, #0x443
1008a4460:     	adrp	x2, 0x101752000 <dyld_stub_binder+0x101752000>
1008a4464:     	add	x2, x2, #0xd90
1008a4468:     	mov	w1, #0x2c               ; =44
1008a446c:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1008a4470:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
1008a4474:     	add	x2, x2, #0x978
1008a4478:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a447c:     	adrp	x2, 0x101798000 <dyld_stub_binder+0x101798000>
1008a4480:     	add	x2, x2, #0x978
1008a4484:     	mov	x1, x8
1008a4488:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a448c:     	mov	x20, x16
1008a4490:     	b	0x1008a44a0 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x13e4>
1008a4494:     	mov	x20, x16
1008a4498:     	mov	x1, x8
1008a449c:     	mov	x2, x12
1008a44a0:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a44a4:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a44a8:     	mov	x1, x11
1008a44ac:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a44b0:     	add	x2, x2, #0xc18
1008a44b4:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a44b8:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a44bc:     	mov	w0, #0x8                ; =8
1008a44c0:     	mov	x1, x25
1008a44c4:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a44c8:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a44cc:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a44d0:     	add	x2, x2, #0xc30
1008a44d4:     	mov	x0, x22
1008a44d8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a44dc:     	mov	x1, x11
1008a44e0:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a44e4:     	add	x2, x2, #0xc18
1008a44e8:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a44ec:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a44f0:     	mov	x0, x8
1008a44f4:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a44f8:     	add	x2, x2, #0x5e0
1008a44fc:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a4500:     	adrp	x2, 0x101759000 <dyld_stub_binder+0x101759000>
1008a4504:     	add	x2, x2, #0xc48
1008a4508:     	mov	x0, x22
1008a450c:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a4510:     	mov	x0, x8
1008a4514:     	adrp	x2, 0x101799000 <dyld_stub_binder+0x101799000>
1008a4518:     	add	x2, x2, #0x5c8
1008a451c:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1008a4520:     	mov	w0, #0x8                ; =8
1008a4524:     	mov	x1, x25
1008a4528:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a452c:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a4530:     	mov	w0, #0x8                ; =8
1008a4534:     	mov	x1, x19
1008a4538:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a453c:     	mov	w0, #0x8                ; =8
1008a4540:     	mov	x1, x26
1008a4544:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a4548:     	b	0x1008a4558 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x149c>
1008a454c:     	mov	w0, #0x8                ; =8
1008a4550:     	mov	x1, x25
1008a4554:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1008a4558:     	brk	#0x1
1008a455c:     	b	0x1008a456c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14b0>
1008a4560:     	b	0x1008a4578 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14bc>
1008a4564:     	ldr	x20, [sp, #0x68]
1008a4568:     	b	0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a456c:     	mov	x19, x0
1008a4570:     	ldr	x20, [sp, #0xf0]
1008a4574:     	b	0x1008a460c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1550>
1008a4578:     	mov	x19, x0
1008a457c:     	b	0x1008a461c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1560>
1008a4580:     	b	0x1008a4600 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1544>
1008a4584:     	mov	x20, x0
1008a4588:     	cbz	x23, 0x1008a45cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1510>
1008a458c:     	mov	x0, x19
1008a4590:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a4594:     	b	0x1008a45cc <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1510>
1008a4598:     	b	0x1008a4644 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1588>
1008a459c:     	mov	x20, x24
1008a45a0:     	mov	x19, x0
1008a45a4:     	ldr	x8, [sp, #0xf0]
1008a45a8:     	cbnz	x8, 0x1008a45b4 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x14f8>
1008a45ac:     	mov	x0, x19
1008a45b0:     	b	0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a45b4:     	ldr	x0, [sp, #0xf8]
1008a45b8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a45bc:     	mov	x0, x19
1008a45c0:     	b	0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a45c4:     	str	x25, [sp, #0x80]
1008a45c8:     	mov	x20, x0
1008a45cc:     	ldr	x8, [sp, #0xc0]
1008a45d0:     	cbz	x8, 0x1008a45e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x152c>
1008a45d4:     	ldr	x0, [sp, #0xc8]
1008a45d8:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a45dc:     	b	0x1008a45e8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x152c>
1008a45e0:     	str	x25, [sp, #0x80]
1008a45e4:     	mov	x20, x0
1008a45e8:     	ldr	x8, [sp, #0x70]
1008a45ec:     	cbz	x8, 0x1008a45f8 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x153c>
1008a45f0:     	ldr	x0, [sp, #0x58]
1008a45f4:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a45f8:     	mov	x0, x20
1008a45fc:     	b	0x1008a4658 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x159c>
1008a4600:     	mov	x19, x0
1008a4604:     	mov	x0, x24
1008a4608:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a460c:     	cmp	x20, #0x1
1008a4610:     	b.lt	0x1008a461c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x1560>
1008a4614:     	ldr	x0, [sp, #0xf8]
1008a4618:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a461c:     	ldr	x8, [sp, #0xc0]
1008a4620:     	cmp	x8, #0x1
1008a4624:     	b.lt	0x1008a4670 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b4>
1008a4628:     	ldr	x20, [sp, #0xc8]
1008a462c:     	mov	x0, x19
1008a4630:     	b	0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a4634:     	tbz	w19, #0x0, 0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a4638:     	b	0x1008a4674 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1008a463c:     	b	0x1008a4658 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x159c>
1008a4640:     	b	0x1008a465c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a0>
1008a4644:     	ldr	x8, [sp, #0xf0]
1008a4648:     	cbz	x8, 0x1008a4674 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1008a464c:     	ldr	x20, [sp, #0xf8]
1008a4650:     	b	0x1008a4664 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a8>
1008a4654:     	b	0x1008a465c <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15a0>
1008a4658:     	ldr	x20, [sp, #0x68]
1008a465c:     	ldr	x8, [sp, #0x80]
1008a4660:     	cbz	x8, 0x1008a4674 <__RINvMs6_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_7Product5visitKm6_EBc_+0x15b8>
1008a4664:     	mov	x19, x0
1008a4668:     	mov	x0, x20
1008a466c:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1008a4670:     	mov	x0, x19
1008a4674:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
