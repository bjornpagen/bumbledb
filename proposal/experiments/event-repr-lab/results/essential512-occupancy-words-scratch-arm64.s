
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b26054 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_>:
100b26054:     	stp	x28, x27, [sp, #-0x60]!
100b26058:     	stp	x26, x25, [sp, #0x10]
100b2605c:     	stp	x24, x23, [sp, #0x20]
100b26060:     	stp	x22, x21, [sp, #0x30]
100b26064:     	stp	x20, x19, [sp, #0x40]
100b26068:     	stp	x29, x30, [sp, #0x50]
100b2606c:     	add	x29, sp, #0x50
100b26070:     	sub	sp, sp, #0x210
100b26074:     	ldr	w19, [x3, #0x10]
100b26078:     	cbz	w19, 0x100b260ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x58>
100b2607c:     	mov	x21, x5
100b26080:     	mov	x27, x4
100b26084:     	mov	x20, x3
100b26088:     	mov	x23, x2
100b2608c:     	mov	x24, x1
100b26090:     	mov	x25, x0
100b26094:     	mov	x0, x4
100b26098:     	mov	x1, x3
100b2609c:     	bl	0x10065e334 <__RINvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB6_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE3getBO_EBV_>
100b260a0:     	cbz	x0, 0x100b260b4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x60>
100b260a4:     	ldrb	w22, [x0]
100b260a8:     	b	0x100b267e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x794>
100b260ac:     	mov	w22, #0x0               ; =0
100b260b0:     	b	0x100b267e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x794>
100b260b4:     	ldr	x8, [x21]
100b260b8:     	add	x8, x8, #0x1
100b260bc:     	str	x8, [x21]
100b260c0:     	mov	x22, x25
100b260c4:     	ldr	x8, [x22, #0x30]!
100b260c8:     	ldr	x1, [x22, #0x10]
100b260cc:     	ldr	x9, [x20]
100b260d0:     	lsr	x0, x19, #1
100b260d4:     	cmn	x8, #0x1
100b260d8:     	b.eq	0x100b26130 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0xdc>
100b260dc:     	cmp	x1, x0
100b260e0:     	b.ls	0x100b268d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x884>
100b260e4:     	ldr	w8, [x20, #0x28]
100b260e8:     	lsr	x8, x8, #1
100b260ec:     	cmp	x1, x8
100b260f0:     	b.ls	0x100b268c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x870>
100b260f4:     	ldr	w10, [x20, #0x40]
100b260f8:     	lsr	x10, x10, #1
100b260fc:     	cmp	x1, x10
100b26100:     	b.ls	0x100b268d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x880>
100b26104:     	ldr	x11, [x22, #0x8]
100b26108:     	lsl	x8, x8, #4
100b2610c:     	ldr	x8, [x11, x8]
100b26110:     	ldr	x12, [x20, #0x18]
100b26114:     	bic	x8, x8, x12
100b26118:     	lsl	x12, x0, #4
100b2611c:     	ldr	x12, [x11, x12]
100b26120:     	bic	x9, x12, x9
100b26124:     	orr	x8, x8, x9
100b26128:     	add	x9, x11, x10, lsl #4
100b2612c:     	b	0x100b26184 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x130>
100b26130:     	ldr	x8, [x22, #0x18]
100b26134:     	cmp	x8, x0
100b26138:     	b.ls	0x100b268fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x8a8>
100b2613c:     	ldr	w10, [x20, #0x28]
100b26140:     	lsr	x11, x10, #1
100b26144:     	cmp	x8, x11
100b26148:     	b.ls	0x100b268e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x890>
100b2614c:     	ldr	w10, [x20, #0x40]
100b26150:     	lsr	x10, x10, #1
100b26154:     	cmp	x8, x10
100b26158:     	b.ls	0x100b268f8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x8a4>
100b2615c:     	add	x8, x1, x11, lsl #5
100b26160:     	ldr	x8, [x8, #0x18]
100b26164:     	ldr	x11, [x20, #0x18]
100b26168:     	bic	x8, x8, x11
100b2616c:     	add	x11, x1, x0, lsl #5
100b26170:     	ldr	x11, [x11, #0x18]
100b26174:     	bic	x9, x11, x9
100b26178:     	orr	x8, x8, x9
100b2617c:     	add	x9, x1, x10, lsl #5
100b26180:     	add	x9, x9, #0x18
100b26184:     	ldr	x9, [x9]
100b26188:     	mov	x19, x20
100b2618c:     	ldr	x10, [x19, #0x30]!
100b26190:     	bic	x9, x9, x10
100b26194:     	orr	x11, x9, x8
100b26198:     	fmov	d0, x11
100b2619c:     	cnt.8b	v0, v0
100b261a0:     	addv.8b	b0, v0
100b261a4:     	fmov	x8, d0
100b261a8:     	cmp	x8, #0xa
100b261ac:     	mov	x9, x20
100b261b0:     	str	x21, [sp, #0x48]
100b261b4:     	b.hs	0x100b26430 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x3dc>
100b261b8:     	stp	x9, x8, [sp, #0x8]
100b261bc:     	str	x27, [sp]
100b261c0:     	mov	x27, #0x0               ; =0
100b261c4:     	ldr	x8, [x21, #0x10]
100b261c8:     	add	x8, x8, #0x1
100b261cc:     	str	x8, [x21, #0x10]
100b261d0:     	ldp	x23, x20, [x21, #0x30]
100b261d4:     	add	x26, sp, #0xb0
100b261d8:     	mov	x10, x9
100b261dc:     	str	x22, [sp, #0x18]
100b261e0:     	str	x11, [sp, #0x30]
100b261e4:     	b	0x100b2621c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x1c8>
100b261e8:     	and	w24, w19, #0x1
100b261ec:     	add	x23, x23, #0x1
100b261f0:     	str	x23, [x21, #0x30]
100b261f4:     	mov	x19, #-0x2              ; =-2
100b261f8:     	ldr	x11, [sp, #0x30]
100b261fc:     	ldr	x10, [sp, #0x38]
100b26200:     	add	x10, x10, #0x18
100b26204:     	add	x9, x26, x27, lsl #5
100b26208:     	stp	x19, x24, [x9]
100b2620c:     	stp	x25, x8, [x9, #0x10]
100b26210:     	add	x27, x27, #0x1
100b26214:     	cmp	x27, #0x3
100b26218:     	b.eq	0x100b26558 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x504>
100b2621c:     	ldp	x28, x8, [x10]
100b26220:     	stp	x10, x8, [sp, #0x38]
100b26224:     	ldr	w19, [x10, #0x10]
100b26228:     	stur	x11, [x29, #-0x90]
100b2622c:     	add	x0, sp, #0x50
100b26230:     	mov	x1, x22
100b26234:     	mov	x2, x19
100b26238:     	bl	0x100c86bac <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b2623c:     	ldr	w8, [sp, #0x50]
100b26240:     	cbz	w8, 0x100b261e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x194>
100b26244:     	cmp	w8, #0x1
100b26248:     	ldr	x9, [sp, #0x30]
100b2624c:     	b.ne	0x100b2688c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x838>
100b26250:     	ldp	x26, x25, [sp, #0x58]
100b26254:     	ldr	x22, [sp, #0x68]
100b26258:     	stur	x22, [x29, #-0x78]
100b2625c:     	bics	x8, x28, x22
100b26260:     	str	x8, [sp, #0x50]
100b26264:     	b.ne	0x100b2680c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7b8>
100b26268:     	ldr	x8, [sp, #0x40]
100b2626c:     	bics	x8, x8, x28
100b26270:     	str	x8, [sp, #0x50]
100b26274:     	b.ne	0x100b2681c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7c8>
100b26278:     	orr	x8, x28, x9
100b2627c:     	bics	x8, x22, x8
100b26280:     	str	x8, [sp, #0x50]
100b26284:     	b.ne	0x100b2682c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7d8>
100b26288:     	ands	x8, x28, x9
100b2628c:     	str	x8, [sp, #0x50]
100b26290:     	b.ne	0x100b2683c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7e8>
100b26294:     	stp	x19, x23, [sp, #0x20]
100b26298:     	cbz	x28, 0x100b26350 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x2fc>
100b2629c:     	mov	x19, #-0x1              ; =-1
100b262a0:     	b	0x100b262cc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x278>
100b262a4:     	add	x20, x20, #0x1
100b262a8:     	ldr	x8, [sp, #0x48]
100b262ac:     	str	x20, [x8, #0x38]
100b262b0:     	bic	x22, x22, x21
100b262b4:     	stur	x22, [x29, #-0x78]
100b262b8:     	mov	x26, x24
100b262bc:     	mov	x19, x23
100b262c0:     	cmp	x21, x28
100b262c4:     	eor	x28, x21, x28
100b262c8:     	b.eq	0x100b26338 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x2e4>
100b262cc:     	neg	x8, x28
100b262d0:     	and	x21, x28, x8
100b262d4:     	sub	x8, x21, #0x1
100b262d8:     	and	x8, x8, x22
100b262dc:     	fmov	d0, x8
100b262e0:     	cnt.8b	v0, v0
100b262e4:     	addv.8b	b0, v0
100b262e8:     	fmov	w4, s0
100b262ec:     	fmov	d0, x22
100b262f0:     	cnt.8b	v0, v0
100b262f4:     	addv.8b	b0, v0
100b262f8:     	fmov	w3, s0
100b262fc:     	ldr	x8, [sp, #0x40]
100b26300:     	tst	x21, x8
100b26304:     	cset	w5, ne
100b26308:     	add	x0, sp, #0x50
100b2630c:     	mov	x1, x26
100b26310:     	mov	x2, x25
100b26314:     	bl	0x100d1dbd8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100b26318:     	ldp	x23, x24, [sp, #0x50]
100b2631c:     	ldr	x25, [sp, #0x60]
100b26320:     	sub	x8, x19, #0x1
100b26324:     	cmn	x8, #0x3
100b26328:     	b.hi	0x100b262a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x250>
100b2632c:     	mov	x0, x26
100b26330:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b26334:     	b	0x100b262a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x250>
100b26338:     	mvn	x8, x22
100b2633c:     	mov	x26, x24
100b26340:     	ldr	x9, [sp, #0x30]
100b26344:     	ands	x28, x8, x9
100b26348:     	b.ne	0x100b263d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x37c>
100b2634c:     	b	0x100b26360 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x30c>
100b26350:     	mvn	x8, x22
100b26354:     	mov	x23, #-0x1              ; =-1
100b26358:     	ands	x28, x8, x9
100b2635c:     	b.ne	0x100b263d0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x37c>
100b26360:     	mov	x19, x23
100b26364:     	mov	x24, x26
100b26368:     	ldr	x11, [sp, #0x30]
100b2636c:     	cmp	x22, x11
100b26370:     	b.ne	0x100b26860 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x80c>
100b26374:     	cmn	x19, #0x1
100b26378:     	mov	w8, #0x28               ; =40
100b2637c:     	mov	w9, #0x20               ; =32
100b26380:     	csel	x8, x9, x8, eq
100b26384:     	ldr	x21, [sp, #0x48]
100b26388:     	ldr	x9, [x21, x8]
100b2638c:     	add	x9, x9, #0x1
100b26390:     	str	x9, [x21, x8]
100b26394:     	ldp	x22, x8, [sp, #0x18]
100b26398:     	sbfx	x8, x8, #0, #1
100b2639c:     	ldr	x23, [sp, #0x28]
100b263a0:     	add	x26, sp, #0xb0
100b263a4:     	b	0x100b261fc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x1a8>
100b263a8:     	add	x20, x20, #0x1
100b263ac:     	ldr	x8, [sp, #0x48]
100b263b0:     	str	x20, [x8, #0x38]
100b263b4:     	orr	x22, x21, x22
100b263b8:     	stur	x22, [x29, #-0x78]
100b263bc:     	mov	x26, x24
100b263c0:     	mov	x23, x19
100b263c4:     	cmp	x21, x28
100b263c8:     	eor	x28, x21, x28
100b263cc:     	b.eq	0x100b26368 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x314>
100b263d0:     	neg	x8, x28
100b263d4:     	and	x21, x28, x8
100b263d8:     	sub	x8, x21, #0x1
100b263dc:     	and	x8, x8, x22
100b263e0:     	fmov	d0, x8
100b263e4:     	cnt.8b	v0, v0
100b263e8:     	addv.8b	b0, v0
100b263ec:     	fmov	w4, s0
100b263f0:     	fmov	d0, x22
100b263f4:     	cnt.8b	v0, v0
100b263f8:     	addv.8b	b0, v0
100b263fc:     	fmov	w3, s0
100b26400:     	add	x0, sp, #0x50
100b26404:     	mov	x1, x26
100b26408:     	mov	x2, x25
100b2640c:     	bl	0x100d1e694 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100b26410:     	ldp	x19, x24, [sp, #0x50]
100b26414:     	ldr	x25, [sp, #0x60]
100b26418:     	sub	x8, x23, #0x1
100b2641c:     	cmn	x8, #0x3
100b26420:     	b.hi	0x100b263a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x354>
100b26424:     	mov	x0, x26
100b26428:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b2642c:     	b	0x100b263a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x354>
100b26430:     	mov	x21, x9
100b26434:     	mov	x8, #0x0                ; =0
100b26438:     	add	x20, sp, #0x110
100b2643c:     	lsl	x9, x23, #2
100b26440:     	cmp	x9, x8
100b26444:     	b.eq	0x100b26880 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x82c>
100b26448:     	ldr	w22, [x24, x8]
100b2644c:     	lsr	x10, x11, x22
100b26450:     	add	x8, x8, #0x4
100b26454:     	tbz	w10, #0x0, 0x100b26440 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x3ec>
100b26458:     	ldr	q0, [x21]
100b2645c:     	str	q0, [sp, #0xb0]
100b26460:     	ldr	x8, [x21, #0x10]
100b26464:     	str	x8, [sp, #0xc0]
100b26468:     	sub	x0, x29, #0x78
100b2646c:     	add	x1, sp, #0xb0
100b26470:     	mov	x2, x25
100b26474:     	mov	x3, x22
100b26478:     	mov	w4, #0x0                ; =0
100b2647c:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26480:     	ldur	q0, [x20, #0xd8]
100b26484:     	ldur	x8, [x29, #-0x68]
100b26488:     	stur	x8, [x29, #-0xa0]
100b2648c:     	str	q0, [sp, #0x50]
100b26490:     	str	x8, [sp, #0x60]
100b26494:     	str	q0, [sp, #0x110]
100b26498:     	str	x8, [sp, #0x120]
100b2649c:     	ldur	q0, [x21, #0x18]
100b264a0:     	str	q0, [sp, #0xb0]
100b264a4:     	ldr	x8, [x21, #0x28]
100b264a8:     	str	x8, [sp, #0xc0]
100b264ac:     	sub	x0, x29, #0x78
100b264b0:     	add	x1, sp, #0xb0
100b264b4:     	mov	x2, x25
100b264b8:     	mov	x3, x22
100b264bc:     	mov	w4, #0x0                ; =0
100b264c0:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b264c4:     	ldur	q0, [x20, #0xd8]
100b264c8:     	ldur	x8, [x29, #-0x68]
100b264cc:     	stur	x8, [x29, #-0xa0]
100b264d0:     	str	q0, [sp, #0x50]
100b264d4:     	str	x8, [sp, #0x60]
100b264d8:     	stur	q0, [x20, #0x18]
100b264dc:     	str	x8, [sp, #0x138]
100b264e0:     	ldr	q0, [x19]
100b264e4:     	str	q0, [sp, #0xb0]
100b264e8:     	ldr	x8, [x19, #0x10]
100b264ec:     	str	x8, [sp, #0xc0]
100b264f0:     	sub	x0, x29, #0x78
100b264f4:     	add	x1, sp, #0xb0
100b264f8:     	mov	x2, x25
100b264fc:     	mov	x26, x22
100b26500:     	mov	x3, x22
100b26504:     	mov	w4, #0x0                ; =0
100b26508:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b2650c:     	ldur	q0, [x20, #0xd8]
100b26510:     	ldur	x8, [x29, #-0x68]
100b26514:     	stur	x8, [x29, #-0xa0]
100b26518:     	str	q0, [sp, #0x50]
100b2651c:     	str	q0, [sp, #0x140]
100b26520:     	str	x8, [sp, #0x150]
100b26524:     	add	x3, sp, #0x110
100b26528:     	mov	x0, x25
100b2652c:     	mov	x1, x24
100b26530:     	mov	x2, x23
100b26534:     	mov	x4, x27
100b26538:     	ldr	x28, [sp, #0x48]
100b2653c:     	mov	x5, x28
100b26540:     	bl	0x100b26054 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_>
100b26544:     	and	w8, w0, #0xff
100b26548:     	cmp	w8, #0xf
100b2654c:     	b.ne	0x100b26690 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x63c>
100b26550:     	mov	w22, #0xf               ; =15
100b26554:     	b	0x100b26798 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x744>
100b26558:     	mov	x0, #0x0                ; =0
100b2655c:     	mov	w22, #0x0               ; =0
100b26560:     	ldp	q1, q0, [sp, #0xf0]
100b26564:     	stp	q1, q0, [sp, #0x90]
100b26568:     	ldp	q1, q0, [sp, #0xd0]
100b2656c:     	stp	q1, q0, [sp, #0x70]
100b26570:     	ldp	q1, q0, [sp, #0xb0]
100b26574:     	stp	q1, q0, [sp, #0x50]
100b26578:     	mov	w8, #0x1                ; =1
100b2657c:     	ldr	x11, [sp, #0x10]
100b26580:     	lsl	x8, x8, x11
100b26584:     	mov	x9, #-0x1               ; =-1
100b26588:     	lsl	x10, x9, x8
100b2658c:     	cmp	x11, #0x6
100b26590:     	csinv	x10, x9, x10, hs
100b26594:     	lsr	x8, x8, #6
100b26598:     	cinc	x11, x8, lo
100b2659c:     	ldp	x9, x8, [sp, #0x50]
100b265a0:     	tst	w8, #0x1
100b265a4:     	csel	x12, x10, xzr, ne
100b265a8:     	ldp	x1, x13, [sp, #0x60]
100b265ac:     	ldp	x19, x23, [sp, #0x70]
100b265b0:     	sub	x15, x0, w23, uxtb
100b265b4:     	ldp	x14, x16, [sp, #0x80]
100b265b8:     	ldp	x20, x24, [sp, #0x90]
100b265bc:     	sub	x17, x0, w24, uxtb
100b265c0:     	ldr	x2, [x21, #0x18]
100b265c4:     	add	x2, x2, #0x1
100b265c8:     	mov	w3, #0x2                ; =2
100b265cc:     	mov	w4, #0x4                ; =4
100b265d0:     	mov	w6, #0x8                ; =8
100b265d4:     	ldp	x5, x7, [sp, #0xa0]
100b265d8:     	b	0x100b26628 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x5d4>
100b265dc:     	bic	x27, x21, x25
100b265e0:     	tst	x26, x27
100b265e4:     	csel	w28, wzr, w3, eq
100b265e8:     	and	x21, x25, x21
100b265ec:     	bics	xzr, x21, x26
100b265f0:     	csel	w25, wzr, w4, eq
100b265f4:     	tst	x26, x21
100b265f8:     	csel	w21, wzr, w6, eq
100b265fc:     	bics	xzr, x27, x26
100b26600:     	cinc	w26, w28, ne
100b26604:     	orr	w21, w25, w21
100b26608:     	orr	w21, w26, w21
100b2660c:     	orr	w22, w21, w22
100b26610:     	and	w21, w22, #0xff
100b26614:     	add	x2, x2, #0x1
100b26618:     	add	x0, x0, #0x1
100b2661c:     	cmp	w21, #0xf
100b26620:     	ldr	x21, [sp, #0x48]
100b26624:     	b.eq	0x100b267a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x74c>
100b26628:     	cmp	x11, x0
100b2662c:     	b.eq	0x100b267a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x750>
100b26630:     	str	x2, [x21, #0x18]
100b26634:     	mov	x21, x12
100b26638:     	cmn	x9, #0x2
100b2663c:     	b.eq	0x100b26654 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x600>
100b26640:     	cmp	x0, x1
100b26644:     	b.hs	0x100b268b4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x860>
100b26648:     	ldr	x21, [x8, x0, lsl #3]
100b2664c:     	eor	x21, x21, x13
100b26650:     	and	x21, x21, x10
100b26654:     	mov	x25, x15
100b26658:     	cmn	x19, #0x2
100b2665c:     	b.eq	0x100b26670 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x61c>
100b26660:     	cmp	x0, x14
100b26664:     	b.hs	0x100b268a8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x854>
100b26668:     	ldr	x25, [x23, x0, lsl #3]
100b2666c:     	eor	x25, x25, x16
100b26670:     	mov	x26, x17
100b26674:     	cmn	x20, #0x2
100b26678:     	b.eq	0x100b265dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x588>
100b2667c:     	cmp	x0, x5
100b26680:     	b.hs	0x100b268b0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x85c>
100b26684:     	ldr	x26, [x24, x0, lsl #3]
100b26688:     	eor	x26, x26, x7
100b2668c:     	b	0x100b265dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x588>
100b26690:     	mov	x22, x0
100b26694:     	ldr	q0, [x21]
100b26698:     	str	q0, [sp, #0xb0]
100b2669c:     	ldr	x8, [x21, #0x10]
100b266a0:     	str	x8, [sp, #0xc0]
100b266a4:     	sub	x0, x29, #0x78
100b266a8:     	add	x1, sp, #0xb0
100b266ac:     	mov	x2, x25
100b266b0:     	mov	x3, x26
100b266b4:     	mov	w4, #0x1                ; =1
100b266b8:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b266bc:     	ldur	q0, [x20, #0xd8]
100b266c0:     	stur	q0, [x29, #-0x90]
100b266c4:     	ldur	x8, [x29, #-0x68]
100b266c8:     	stur	q0, [x29, #-0xb0]
100b266cc:     	str	q0, [sp, #0x50]
100b266d0:     	str	x8, [sp, #0x60]
100b266d4:     	ldr	q0, [sp, #0x50]
100b266d8:     	stur	x8, [x29, #-0xf0]
100b266dc:     	stur	q0, [x29, #-0x100]
100b266e0:     	ldur	q0, [x21, #0x18]
100b266e4:     	str	q0, [sp, #0xb0]
100b266e8:     	ldur	x8, [x21, #0x28]
100b266ec:     	str	x8, [sp, #0xc0]
100b266f0:     	sub	x0, x29, #0x78
100b266f4:     	add	x1, sp, #0xb0
100b266f8:     	mov	x2, x25
100b266fc:     	mov	x3, x26
100b26700:     	mov	w4, #0x1                ; =1
100b26704:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26708:     	ldur	q0, [x20, #0xd8]
100b2670c:     	stur	q0, [x29, #-0x90]
100b26710:     	ldur	x8, [x29, #-0x68]
100b26714:     	stur	q0, [x29, #-0xb0]
100b26718:     	str	q0, [sp, #0x50]
100b2671c:     	str	x8, [sp, #0x60]
100b26720:     	ldr	q0, [sp, #0x50]
100b26724:     	stur	x8, [x29, #-0xd8]
100b26728:     	stur	q0, [x20, #0x68]
100b2672c:     	ldr	q0, [x19]
100b26730:     	str	q0, [sp, #0xb0]
100b26734:     	ldr	x8, [x19, #0x10]
100b26738:     	str	x8, [sp, #0xc0]
100b2673c:     	sub	x0, x29, #0x78
100b26740:     	add	x1, sp, #0xb0
100b26744:     	mov	x2, x25
100b26748:     	mov	x3, x26
100b2674c:     	mov	w4, #0x1                ; =1
100b26750:     	bl	0x1006c0e80 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
100b26754:     	ldur	q0, [x20, #0xd8]
100b26758:     	stur	q0, [x29, #-0x90]
100b2675c:     	ldur	x8, [x29, #-0x68]
100b26760:     	stur	q0, [x29, #-0xb0]
100b26764:     	str	q0, [sp, #0x50]
100b26768:     	str	x8, [sp, #0x60]
100b2676c:     	ldr	q0, [sp, #0x50]
100b26770:     	stur	x8, [x29, #-0xc0]
100b26774:     	stur	q0, [x29, #-0xd0]
100b26778:     	sub	x3, x29, #0x100
100b2677c:     	mov	x0, x25
100b26780:     	mov	x1, x24
100b26784:     	mov	x2, x23
100b26788:     	mov	x4, x27
100b2678c:     	mov	x5, x28
100b26790:     	bl	0x100b26054 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_>
100b26794:     	orr	w22, w0, w22
100b26798:     	mov	x1, x21
100b2679c:     	b	0x100b267dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x788>
100b267a0:     	mov	w22, #0xf               ; =15
100b267a4:     	cmp	x9, #0x1
100b267a8:     	ldr	x27, [sp]
100b267ac:     	b.lt	0x100b267b8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x764>
100b267b0:     	mov	x0, x8
100b267b4:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b267b8:     	cmp	x19, #0x1
100b267bc:     	b.lt	0x100b267c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x774>
100b267c0:     	mov	x0, x23
100b267c4:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b267c8:     	cmp	x20, #0x1
100b267cc:     	b.lt	0x100b267d8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x784>
100b267d0:     	mov	x0, x24
100b267d4:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b267d8:     	ldr	x1, [sp, #0x8]
100b267dc:     	mov	x0, x27
100b267e0:     	mov	x2, x22
100b267e4:     	bl	0x100c2cf60 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj3_hNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
100b267e8:     	mov	x0, x22
100b267ec:     	add	sp, sp, #0x210
100b267f0:     	ldp	x29, x30, [sp, #0x50]
100b267f4:     	ldp	x20, x19, [sp, #0x40]
100b267f8:     	ldp	x22, x21, [sp, #0x30]
100b267fc:     	ldp	x24, x23, [sp, #0x20]
100b26800:     	ldp	x26, x25, [sp, #0x10]
100b26804:     	ldp	x28, x27, [sp], #0x60
100b26808:     	ret
100b2680c:     	add	x1, sp, #0x50
100b26810:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26814:     	add	x5, x5, #0xf0
100b26818:     	b	0x100b26848 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7f4>
100b2681c:     	add	x1, sp, #0x50
100b26820:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26824:     	add	x5, x5, #0xd8
100b26828:     	b	0x100b26848 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7f4>
100b2682c:     	add	x1, sp, #0x50
100b26830:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26834:     	add	x5, x5, #0xc0
100b26838:     	b	0x100b26848 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x7f4>
100b2683c:     	add	x1, sp, #0x50
100b26840:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26844:     	add	x5, x5, #0xa8
100b26848:     	adrp	x2, 0x1013ec000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1a97>
100b2684c:     	add	x2, x2, #0x788
100b26850:     	mov	w0, #0x0                ; =0
100b26854:     	mov	x3, #0x0                ; =0
100b26858:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b2685c:     	b	0x100b268c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x86c>
100b26860:     	adrp	x5, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26864:     	add	x5, x5, #0x90
100b26868:     	sub	x1, x29, #0x78
100b2686c:     	sub	x2, x29, #0x90
100b26870:     	mov	w0, #0x0                ; =0
100b26874:     	mov	x3, #0x0                ; =0
100b26878:     	bl	0x101288320 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100b2687c:     	b	0x100b268c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x86c>
100b26880:     	adrp	x0, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b26884:     	add	x0, x0, #0xd90
100b26888:     	bl	0x1012884b4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100b2688c:     	adrp	x0, 0x10132b000 <dyld_stub_binder+0x10132b000>
100b26890:     	add	x0, x0, #0xd49
100b26894:     	adrp	x2, 0x1014c4000 <dyld_stub_binder+0x1014c4000>
100b26898:     	add	x2, x2, #0x108
100b2689c:     	mov	w1, #0xc9               ; =201
100b268a0:     	bl	0x1012882b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100b268a4:     	b	0x100b268c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x86c>
100b268a8:     	mov	x1, x14
100b268ac:     	b	0x100b268b4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x860>
100b268b0:     	mov	x1, x5
100b268b4:     	adrp	x2, 0x1014cc000 <dyld_stub_binder+0x1014cc000>
100b268b8:     	add	x2, x2, #0x220
100b268bc:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b268c0:     	brk	#0x1
100b268c4:     	mov	x0, x8
100b268c8:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b268cc:     	add	x2, x2, #0x6c0
100b268d0:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b268d4:     	mov	x0, x10
100b268d8:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b268dc:     	add	x2, x2, #0x6c0
100b268e0:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b268e4:     	mov	x0, x11
100b268e8:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b268ec:     	add	x2, x2, #0x6a8
100b268f0:     	mov	x1, x8
100b268f4:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b268f8:     	mov	x0, x10
100b268fc:     	adrp	x2, 0x10150c000 <dyld_stub_binder+0x10150c000>
100b26900:     	add	x2, x2, #0x6a8
100b26904:     	mov	x1, x8
100b26908:     	bl	0x10128841c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100b2690c:     	mov	x20, x0
100b26910:     	add	x0, sp, #0x50
100b26914:     	bl	0x10071517c <__RINvNtCs4sDCw1iE1MS_4core3ptr9drop_glueANtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy5words5Planej3_EBK_>
100b26918:     	mov	x0, x20
100b2691c:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
100b26920:     	b	0x100b26958 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x904>
100b26924:     	mov	x20, x0
100b26928:     	mov	x19, x23
100b2692c:     	b	0x100b26940 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x8ec>
100b26930:     	mov	x20, x0
100b26934:     	b	0x100b26940 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x8ec>
100b26938:     	mov	x20, x0
100b2693c:     	mov	x26, x24
100b26940:     	sub	x8, x19, #0x1
100b26944:     	cmn	x8, #0x3
100b26948:     	b.hi	0x100b2695c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x908>
100b2694c:     	mov	x0, x26
100b26950:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b26954:     	b	0x100b2695c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x908>
100b26958:     	mov	x20, x0
100b2695c:     	cbnz	x27, 0x100b26968 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x914>
100b26960:     	mov	x0, x20
100b26964:     	bl	0x101290688 <dyld_stub_binder+0x101290688>
100b26968:     	add	x8, sp, #0xb0
100b2696c:     	add	x19, x8, #0x8
100b26970:     	b	0x100b26980 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x92c>
100b26974:     	add	x19, x19, #0x20
100b26978:     	subs	x27, x27, #0x1
100b2697c:     	b.eq	0x100b26960 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x90c>
100b26980:     	ldur	x8, [x19, #-0x8]
100b26984:     	cmp	x8, #0x1
100b26988:     	b.lt	0x100b26974 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x920>
100b2698c:     	ldr	x0, [x19]
100b26990:     	bl	0x101290838 <dyld_stub_binder+0x101290838>
100b26994:     	b	0x100b26974 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy13classify_with5visitKm9_Kh1_Kj0_EB8_+0x920>
