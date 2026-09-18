
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006c90c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec>:
1006c90c0:     	stp	x28, x27, [sp, #-0x60]!
1006c90c4:     	stp	x26, x25, [sp, #0x10]
1006c90c8:     	stp	x24, x23, [sp, #0x20]
1006c90cc:     	stp	x22, x21, [sp, #0x30]
1006c90d0:     	stp	x20, x19, [sp, #0x40]
1006c90d4:     	stp	x29, x30, [sp, #0x50]
1006c90d8:     	add	x29, sp, #0x50
1006c90dc:     	sub	sp, sp, #0xc40
1006c90e0:     	ldr	xzr, [sp]
1006c90e4:     	mov	w24, #0x0               ; =0
1006c90e8:     	cbz	w1, 0x1006c99d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x914>
1006c90ec:     	cbz	w2, 0x1006c99d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x914>
1006c90f0:     	mov	x19, x4
1006c90f4:     	mov	x22, x3
1006c90f8:     	mov	x23, x0
1006c90fc:     	nop
1006c9100:     	cmp	w2, w1
1006c9104:     	csel	w27, w2, w1, lo
1006c9108:     	csel	w21, w2, w1, hi
1006c910c:     	ldr	x8, [x4, #0x18]
1006c9110:     	cbz	x8, 0x1006c91c8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x108>
1006c9114:     	mov	x8, #0x0                ; =0
1006c9118:     	mov	x9, #0xa9c5             ; =43461
1006c911c:     	movk	x9, #0x2e62, lsl #16
1006c9120:     	movk	x9, #0x7aea, lsl #32
1006c9124:     	movk	x9, #0xf135, lsl #48
1006c9128:     	mul	x10, x27, x9
1006c912c:     	add	x10, x10, w21, uxtw
1006c9130:     	mul	x9, x10, x9
1006c9134:     	ror	x12, x9, #0x2c
1006c9138:     	lsr	x11, x12, #57
1006c913c:     	ldp	x10, x9, [x19]
1006c9140:     	dup.8b	v0, w11
1006c9144:     	movi.2d	v1, #0xffffffffffffffff
1006c9148:     	mov	w11, #0xc               ; =12
1006c914c:     	and	x12, x12, x9
1006c9150:     	ldr	d2, [x10, x12]
1006c9154:     	cmeq.8b	v3, v2, v0
1006c9158:     	fmov	x13, d3
1006c915c:     	ands	x13, x13, #0x8080808080808080
1006c9160:     	b.eq	0x1006c9198 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0xd8>
1006c9164:     	rbit	x14, x13
1006c9168:     	clz	x14, x14
1006c916c:     	add	x14, x12, x14, lsr #3
1006c9170:     	and	x14, x14, x9
1006c9174:     	mneg	x14, x14, x11
1006c9178:     	add	x14, x10, x14
1006c917c:     	ldp	w15, w16, [x14, #-0xc]
1006c9180:     	cmp	w27, w15
1006c9184:     	ccmp	w21, w16, #0x0, eq
1006c9188:     	b.eq	0x1006c9260 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x1a0>
1006c918c:     	sub	x14, x13, #0x2
1006c9190:     	ands	x13, x14, x13
1006c9194:     	b.ne	0x1006c9164 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0xa4>
1006c9198:     	cmeq.8b	v2, v2, v1
1006c919c:     	fmov	x13, d2
1006c91a0:     	cbnz	x13, 0x1006c91c8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x108>
1006c91a4:     	add	x8, x8, #0x8
1006c91a8:     	add	x12, x12, x8
1006c91ac:     	and	x12, x12, x9
1006c91b0:     	ldr	d2, [x10, x12]
1006c91b4:     	cmeq.8b	v3, v2, v0
1006c91b8:     	fmov	x13, d3
1006c91bc:     	ands	x13, x13, #0x8080808080808080
1006c91c0:     	b.ne	0x1006c9164 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0xa4>
1006c91c4:     	b	0x1006c9198 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0xd8>
1006c91c8:     	tbnz	w1, #0x1, 0x1006c9268 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x1a8>
1006c91cc:     	ldr	x9, [x23, #0x110]
1006c91d0:     	tbnz	w2, #0x1, 0x1006c9288 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x1c8>
1006c91d4:     	mov	x25, x2
1006c91d8:     	ldr	x0, [x23, #0x110]
1006c91dc:     	cmp	x0, x9
1006c91e0:     	csel	x24, x0, x9, lo
1006c91e4:     	cmp	x24, x0
1006c91e8:     	b.ne	0x1006c92bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x1fc>
1006c91ec:     	lsr	w9, w1, #2
1006c91f0:     	ldr	x8, [x23, #0xa0]
1006c91f4:     	cmp	x8, x9
1006c91f8:     	b.ls	0x1006c9a34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x974>
1006c91fc:     	ldr	x11, [x23, #0x98]
1006c9200:     	ldr	x10, [x11, x9, lsl #3]
1006c9204:     	tbz	w1, #0x0, 0x1006c9220 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x160>
1006c9208:     	ldr	x1, [x23, #0x58]
1006c920c:     	cmp	x0, x1
1006c9210:     	b.hs	0x1006c9a48 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x988>
1006c9214:     	ldr	x9, [x23, #0x50]
1006c9218:     	ldr	x9, [x9, x0, lsl #3]
1006c921c:     	eor	x10, x9, x10
1006c9220:     	lsr	w9, w25, #2
1006c9224:     	cmp	x8, x9
1006c9228:     	b.ls	0x1006c9a34 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x974>
1006c922c:     	ldr	x8, [x11, x9, lsl #3]
1006c9230:     	tbz	w25, #0x0, 0x1006c924c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x18c>
1006c9234:     	ldr	x1, [x23, #0x58]
1006c9238:     	cmp	x0, x1
1006c923c:     	b.hs	0x1006c9a48 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x988>
1006c9240:     	ldr	x9, [x23, #0x50]
1006c9244:     	ldr	x9, [x9, x0, lsl #3]
1006c9248:     	eor	x8, x9, x8
1006c924c:     	and	x1, x8, x10
1006c9250:     	mov	x0, x23
1006c9254:     	mov	x2, x22
1006c9258:     	bl	0x1006c9d50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411reduce_leaf>
1006c925c:     	b	0x1006c99bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x8fc>
1006c9260:     	ldur	w24, [x14, #-0x4]
1006c9264:     	b	0x1006c99d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x914>
1006c9268:     	lsr	w0, w1, #2
1006c926c:     	ldr	x8, [x23, #0x70]
1006c9270:     	cmp	x8, x0
1006c9274:     	b.ls	0x1006c9a24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x964>
1006c9278:     	ldr	x8, [x23, #0x68]
1006c927c:     	lsl	x9, x0, #4
1006c9280:     	ldr	w9, [x8, x9]
1006c9284:     	tbz	w2, #0x1, 0x1006c91d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x114>
1006c9288:     	lsr	w0, w2, #2
1006c928c:     	ldr	x8, [x23, #0x70]
1006c9290:     	cmp	x8, x0
1006c9294:     	b.ls	0x1006c9a24 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x964>
1006c9298:     	mov	x25, x2
1006c929c:     	ldr	x8, [x23, #0x68]
1006c92a0:     	lsl	x10, x0, #4
1006c92a4:     	ldr	w8, [x8, x10]
1006c92a8:     	ldr	x0, [x23, #0x110]
1006c92ac:     	cmp	x8, x9
1006c92b0:     	csel	x24, x8, x9, lo
1006c92b4:     	cmp	x24, x0
1006c92b8:     	b.eq	0x1006c91ec <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x12c>
1006c92bc:     	str	xzr, [sp, #0x38]
1006c92c0:     	str	wzr, [sp, #0x40]
1006c92c4:     	str	xzr, [sp, #0x48]
1006c92c8:     	str	wzr, [sp, #0x50]
1006c92cc:     	str	xzr, [sp, #0x58]
1006c92d0:     	str	wzr, [sp, #0x60]
1006c92d4:     	str	xzr, [sp, #0x68]
1006c92d8:     	str	wzr, [sp, #0x70]
1006c92dc:     	str	xzr, [sp, #0x78]
1006c92e0:     	str	wzr, [sp, #0x80]
1006c92e4:     	str	xzr, [sp, #0x88]
1006c92e8:     	str	wzr, [sp, #0x90]
1006c92ec:     	str	xzr, [sp, #0x98]
1006c92f0:     	str	wzr, [sp, #0xa0]
1006c92f4:     	str	xzr, [sp, #0xa8]
1006c92f8:     	str	wzr, [sp, #0xb0]
1006c92fc:     	str	xzr, [sp, #0xb8]
1006c9300:     	str	wzr, [sp, #0xc0]
1006c9304:     	str	xzr, [sp, #0xc8]
1006c9308:     	str	wzr, [sp, #0xd0]
1006c930c:     	str	xzr, [sp, #0xd8]
1006c9310:     	str	wzr, [sp, #0xe0]
1006c9314:     	str	xzr, [sp, #0xe8]
1006c9318:     	str	wzr, [sp, #0xf0]
1006c931c:     	str	xzr, [sp, #0xf8]
1006c9320:     	str	wzr, [sp, #0x100]
1006c9324:     	str	xzr, [sp, #0x108]
1006c9328:     	str	wzr, [sp, #0x110]
1006c932c:     	str	xzr, [sp, #0x118]
1006c9330:     	str	wzr, [sp, #0x120]
1006c9334:     	str	xzr, [sp, #0x128]
1006c9338:     	str	wzr, [sp, #0x130]
1006c933c:     	str	xzr, [sp, #0x138]
1006c9340:     	str	wzr, [sp, #0x140]
1006c9344:     	str	xzr, [sp, #0x148]
1006c9348:     	str	wzr, [sp, #0x150]
1006c934c:     	str	xzr, [sp, #0x158]
1006c9350:     	str	wzr, [sp, #0x160]
1006c9354:     	str	xzr, [sp, #0x168]
1006c9358:     	str	wzr, [sp, #0x170]
1006c935c:     	str	xzr, [sp, #0x178]
1006c9360:     	str	wzr, [sp, #0x180]
1006c9364:     	str	xzr, [sp, #0x188]
1006c9368:     	str	wzr, [sp, #0x190]
1006c936c:     	str	xzr, [sp, #0x198]
1006c9370:     	str	wzr, [sp, #0x1a0]
1006c9374:     	str	xzr, [sp, #0x1a8]
1006c9378:     	str	wzr, [sp, #0x1b0]
1006c937c:     	str	xzr, [sp, #0x1b8]
1006c9380:     	str	wzr, [sp, #0x1c0]
1006c9384:     	str	xzr, [sp, #0x1c8]
1006c9388:     	str	wzr, [sp, #0x1d0]
1006c938c:     	str	xzr, [sp, #0x1d8]
1006c9390:     	str	wzr, [sp, #0x1e0]
1006c9394:     	str	xzr, [sp, #0x1e8]
1006c9398:     	str	wzr, [sp, #0x1f0]
1006c939c:     	str	xzr, [sp, #0x1f8]
1006c93a0:     	str	wzr, [sp, #0x200]
1006c93a4:     	str	xzr, [sp, #0x208]
1006c93a8:     	str	wzr, [sp, #0x210]
1006c93ac:     	str	xzr, [sp, #0x218]
1006c93b0:     	str	wzr, [sp, #0x220]
1006c93b4:     	str	xzr, [sp, #0x228]
1006c93b8:     	str	wzr, [sp, #0x230]
1006c93bc:     	str	xzr, [sp, #0x238]
1006c93c0:     	str	wzr, [sp, #0x240]
1006c93c4:     	str	xzr, [sp, #0x248]
1006c93c8:     	str	wzr, [sp, #0x250]
1006c93cc:     	str	xzr, [sp, #0x258]
1006c93d0:     	str	wzr, [sp, #0x260]
1006c93d4:     	str	xzr, [sp, #0x268]
1006c93d8:     	str	wzr, [sp, #0x270]
1006c93dc:     	str	xzr, [sp, #0x278]
1006c93e0:     	str	wzr, [sp, #0x280]
1006c93e4:     	str	xzr, [sp, #0x288]
1006c93e8:     	str	wzr, [sp, #0x290]
1006c93ec:     	str	xzr, [sp, #0x298]
1006c93f0:     	str	wzr, [sp, #0x2a0]
1006c93f4:     	str	xzr, [sp, #0x2a8]
1006c93f8:     	str	wzr, [sp, #0x2b0]
1006c93fc:     	str	xzr, [sp, #0x2b8]
1006c9400:     	str	wzr, [sp, #0x2c0]
1006c9404:     	str	xzr, [sp, #0x2c8]
1006c9408:     	str	wzr, [sp, #0x2d0]
1006c940c:     	str	xzr, [sp, #0x2d8]
1006c9410:     	str	wzr, [sp, #0x2e0]
1006c9414:     	str	xzr, [sp, #0x2e8]
1006c9418:     	str	wzr, [sp, #0x2f0]
1006c941c:     	str	xzr, [sp, #0x2f8]
1006c9420:     	str	wzr, [sp, #0x300]
1006c9424:     	str	xzr, [sp, #0x308]
1006c9428:     	str	wzr, [sp, #0x310]
1006c942c:     	str	xzr, [sp, #0x318]
1006c9430:     	str	wzr, [sp, #0x320]
1006c9434:     	str	xzr, [sp, #0x328]
1006c9438:     	str	wzr, [sp, #0x330]
1006c943c:     	str	xzr, [sp, #0x338]
1006c9440:     	str	wzr, [sp, #0x340]
1006c9444:     	str	xzr, [sp, #0x348]
1006c9448:     	str	wzr, [sp, #0x350]
1006c944c:     	str	xzr, [sp, #0x358]
1006c9450:     	str	wzr, [sp, #0x360]
1006c9454:     	str	xzr, [sp, #0x368]
1006c9458:     	str	wzr, [sp, #0x370]
1006c945c:     	str	xzr, [sp, #0x378]
1006c9460:     	str	wzr, [sp, #0x380]
1006c9464:     	str	xzr, [sp, #0x388]
1006c9468:     	str	wzr, [sp, #0x390]
1006c946c:     	str	xzr, [sp, #0x398]
1006c9470:     	str	wzr, [sp, #0x3a0]
1006c9474:     	str	xzr, [sp, #0x3a8]
1006c9478:     	str	wzr, [sp, #0x3b0]
1006c947c:     	str	xzr, [sp, #0x3b8]
1006c9480:     	str	wzr, [sp, #0x3c0]
1006c9484:     	str	xzr, [sp, #0x3c8]
1006c9488:     	str	wzr, [sp, #0x3d0]
1006c948c:     	str	xzr, [sp, #0x3d8]
1006c9490:     	str	wzr, [sp, #0x3e0]
1006c9494:     	str	xzr, [sp, #0x3e8]
1006c9498:     	str	wzr, [sp, #0x3f0]
1006c949c:     	str	xzr, [sp, #0x3f8]
1006c94a0:     	str	wzr, [sp, #0x400]
1006c94a4:     	str	xzr, [sp, #0x408]
1006c94a8:     	str	wzr, [sp, #0x410]
1006c94ac:     	str	xzr, [sp, #0x418]
1006c94b0:     	str	wzr, [sp, #0x420]
1006c94b4:     	str	xzr, [sp, #0x428]
1006c94b8:     	str	wzr, [sp, #0x430]
1006c94bc:     	str	xzr, [sp, #0x438]
1006c94c0:     	str	wzr, [sp, #0x440]
1006c94c4:     	str	xzr, [sp, #0x448]
1006c94c8:     	str	wzr, [sp, #0x450]
1006c94cc:     	str	xzr, [sp, #0x458]
1006c94d0:     	str	wzr, [sp, #0x460]
1006c94d4:     	str	xzr, [sp, #0x468]
1006c94d8:     	str	wzr, [sp, #0x470]
1006c94dc:     	str	xzr, [sp, #0x478]
1006c94e0:     	str	wzr, [sp, #0x480]
1006c94e4:     	str	xzr, [sp, #0x488]
1006c94e8:     	str	wzr, [sp, #0x490]
1006c94ec:     	str	xzr, [sp, #0x498]
1006c94f0:     	str	wzr, [sp, #0x4a0]
1006c94f4:     	str	xzr, [sp, #0x4a8]
1006c94f8:     	str	wzr, [sp, #0x4b0]
1006c94fc:     	str	xzr, [sp, #0x4b8]
1006c9500:     	str	wzr, [sp, #0x4c0]
1006c9504:     	str	xzr, [sp, #0x4c8]
1006c9508:     	str	wzr, [sp, #0x4d0]
1006c950c:     	str	xzr, [sp, #0x4d8]
1006c9510:     	str	wzr, [sp, #0x4e0]
1006c9514:     	str	xzr, [sp, #0x4e8]
1006c9518:     	str	wzr, [sp, #0x4f0]
1006c951c:     	str	xzr, [sp, #0x4f8]
1006c9520:     	str	wzr, [sp, #0x500]
1006c9524:     	str	xzr, [sp, #0x508]
1006c9528:     	str	wzr, [sp, #0x510]
1006c952c:     	str	xzr, [sp, #0x518]
1006c9530:     	str	wzr, [sp, #0x520]
1006c9534:     	str	xzr, [sp, #0x528]
1006c9538:     	str	wzr, [sp, #0x530]
1006c953c:     	str	xzr, [sp, #0x538]
1006c9540:     	str	wzr, [sp, #0x540]
1006c9544:     	str	xzr, [sp, #0x548]
1006c9548:     	str	wzr, [sp, #0x550]
1006c954c:     	str	xzr, [sp, #0x558]
1006c9550:     	str	wzr, [sp, #0x560]
1006c9554:     	str	xzr, [sp, #0x568]
1006c9558:     	str	wzr, [sp, #0x570]
1006c955c:     	str	xzr, [sp, #0x578]
1006c9560:     	str	wzr, [sp, #0x580]
1006c9564:     	str	xzr, [sp, #0x588]
1006c9568:     	str	wzr, [sp, #0x590]
1006c956c:     	str	xzr, [sp, #0x598]
1006c9570:     	str	wzr, [sp, #0x5a0]
1006c9574:     	str	xzr, [sp, #0x5a8]
1006c9578:     	str	wzr, [sp, #0x5b0]
1006c957c:     	str	xzr, [sp, #0x5b8]
1006c9580:     	str	wzr, [sp, #0x5c0]
1006c9584:     	str	xzr, [sp, #0x5c8]
1006c9588:     	str	wzr, [sp, #0x5d0]
1006c958c:     	str	xzr, [sp, #0x5d8]
1006c9590:     	str	wzr, [sp, #0x5e0]
1006c9594:     	str	xzr, [sp, #0x5e8]
1006c9598:     	str	wzr, [sp, #0x5f0]
1006c959c:     	str	xzr, [sp, #0x5f8]
1006c95a0:     	str	wzr, [sp, #0x600]
1006c95a4:     	str	xzr, [sp, #0x608]
1006c95a8:     	str	wzr, [sp, #0x610]
1006c95ac:     	str	xzr, [sp, #0x618]
1006c95b0:     	str	wzr, [sp, #0x620]
1006c95b4:     	str	xzr, [sp, #0x628]
1006c95b8:     	str	wzr, [sp, #0x630]
1006c95bc:     	str	xzr, [sp, #0x638]
1006c95c0:     	str	wzr, [sp, #0x640]
1006c95c4:     	str	xzr, [sp, #0x648]
1006c95c8:     	str	wzr, [sp, #0x650]
1006c95cc:     	str	xzr, [sp, #0x658]
1006c95d0:     	str	wzr, [sp, #0x660]
1006c95d4:     	str	xzr, [sp, #0x668]
1006c95d8:     	str	wzr, [sp, #0x670]
1006c95dc:     	str	xzr, [sp, #0x678]
1006c95e0:     	str	wzr, [sp, #0x680]
1006c95e4:     	str	xzr, [sp, #0x688]
1006c95e8:     	str	wzr, [sp, #0x690]
1006c95ec:     	str	xzr, [sp, #0x698]
1006c95f0:     	str	wzr, [sp, #0x6a0]
1006c95f4:     	str	xzr, [sp, #0x6a8]
1006c95f8:     	str	wzr, [sp, #0x6b0]
1006c95fc:     	str	xzr, [sp, #0x6b8]
1006c9600:     	str	wzr, [sp, #0x6c0]
1006c9604:     	str	xzr, [sp, #0x6c8]
1006c9608:     	str	wzr, [sp, #0x6d0]
1006c960c:     	str	xzr, [sp, #0x6d8]
1006c9610:     	str	wzr, [sp, #0x6e0]
1006c9614:     	str	xzr, [sp, #0x6e8]
1006c9618:     	str	wzr, [sp, #0x6f0]
1006c961c:     	str	xzr, [sp, #0x6f8]
1006c9620:     	str	wzr, [sp, #0x700]
1006c9624:     	str	xzr, [sp, #0x708]
1006c9628:     	str	wzr, [sp, #0x710]
1006c962c:     	str	xzr, [sp, #0x718]
1006c9630:     	str	wzr, [sp, #0x720]
1006c9634:     	str	xzr, [sp, #0x728]
1006c9638:     	str	wzr, [sp, #0x730]
1006c963c:     	str	xzr, [sp, #0x738]
1006c9640:     	str	wzr, [sp, #0x740]
1006c9644:     	str	xzr, [sp, #0x748]
1006c9648:     	str	wzr, [sp, #0x750]
1006c964c:     	str	xzr, [sp, #0x758]
1006c9650:     	str	wzr, [sp, #0x760]
1006c9654:     	str	xzr, [sp, #0x768]
1006c9658:     	str	wzr, [sp, #0x770]
1006c965c:     	str	xzr, [sp, #0x778]
1006c9660:     	str	wzr, [sp, #0x780]
1006c9664:     	str	xzr, [sp, #0x788]
1006c9668:     	str	wzr, [sp, #0x790]
1006c966c:     	str	xzr, [sp, #0x798]
1006c9670:     	str	wzr, [sp, #0x7a0]
1006c9674:     	str	xzr, [sp, #0x7a8]
1006c9678:     	str	wzr, [sp, #0x7b0]
1006c967c:     	str	xzr, [sp, #0x7b8]
1006c9680:     	str	wzr, [sp, #0x7c0]
1006c9684:     	str	xzr, [sp, #0x7c8]
1006c9688:     	str	wzr, [sp, #0x7d0]
1006c968c:     	str	xzr, [sp, #0x7d8]
1006c9690:     	str	wzr, [sp, #0x7e0]
1006c9694:     	str	xzr, [sp, #0x7e8]
1006c9698:     	str	wzr, [sp, #0x7f0]
1006c969c:     	str	xzr, [sp, #0x7f8]
1006c96a0:     	str	wzr, [sp, #0x800]
1006c96a4:     	str	xzr, [sp, #0x808]
1006c96a8:     	str	wzr, [sp, #0x810]
1006c96ac:     	str	xzr, [sp, #0x818]
1006c96b0:     	str	wzr, [sp, #0x820]
1006c96b4:     	str	xzr, [sp, #0x828]
1006c96b8:     	str	wzr, [sp, #0x830]
1006c96bc:     	str	xzr, [sp, #0x838]
1006c96c0:     	str	wzr, [sp, #0x840]
1006c96c4:     	str	xzr, [sp, #0x848]
1006c96c8:     	str	wzr, [sp, #0x850]
1006c96cc:     	str	xzr, [sp, #0x858]
1006c96d0:     	str	wzr, [sp, #0x860]
1006c96d4:     	str	xzr, [sp, #0x868]
1006c96d8:     	str	wzr, [sp, #0x870]
1006c96dc:     	str	xzr, [sp, #0x878]
1006c96e0:     	str	wzr, [sp, #0x880]
1006c96e4:     	str	xzr, [sp, #0x888]
1006c96e8:     	str	wzr, [sp, #0x890]
1006c96ec:     	str	xzr, [sp, #0x898]
1006c96f0:     	str	wzr, [sp, #0x8a0]
1006c96f4:     	str	xzr, [sp, #0x8a8]
1006c96f8:     	str	wzr, [sp, #0x8b0]
1006c96fc:     	str	xzr, [sp, #0x8b8]
1006c9700:     	str	wzr, [sp, #0x8c0]
1006c9704:     	str	xzr, [sp, #0x8c8]
1006c9708:     	str	wzr, [sp, #0x8d0]
1006c970c:     	str	xzr, [sp, #0x8d8]
1006c9710:     	str	wzr, [sp, #0x8e0]
1006c9714:     	str	xzr, [sp, #0x8e8]
1006c9718:     	str	wzr, [sp, #0x8f0]
1006c971c:     	str	xzr, [sp, #0x8f8]
1006c9720:     	str	wzr, [sp, #0x900]
1006c9724:     	str	xzr, [sp, #0x908]
1006c9728:     	str	wzr, [sp, #0x910]
1006c972c:     	str	xzr, [sp, #0x918]
1006c9730:     	str	wzr, [sp, #0x920]
1006c9734:     	str	xzr, [sp, #0x928]
1006c9738:     	str	wzr, [sp, #0x930]
1006c973c:     	str	xzr, [sp, #0x938]
1006c9740:     	str	wzr, [sp, #0x940]
1006c9744:     	str	xzr, [sp, #0x948]
1006c9748:     	str	wzr, [sp, #0x950]
1006c974c:     	str	xzr, [sp, #0x958]
1006c9750:     	str	wzr, [sp, #0x960]
1006c9754:     	str	xzr, [sp, #0x968]
1006c9758:     	str	wzr, [sp, #0x970]
1006c975c:     	str	xzr, [sp, #0x978]
1006c9760:     	str	wzr, [sp, #0x980]
1006c9764:     	str	xzr, [sp, #0x988]
1006c9768:     	str	wzr, [sp, #0x990]
1006c976c:     	str	xzr, [sp, #0x998]
1006c9770:     	str	wzr, [sp, #0x9a0]
1006c9774:     	str	xzr, [sp, #0x9a8]
1006c9778:     	str	wzr, [sp, #0x9b0]
1006c977c:     	str	xzr, [sp, #0x9b8]
1006c9780:     	str	wzr, [sp, #0x9c0]
1006c9784:     	str	xzr, [sp, #0x9c8]
1006c9788:     	str	wzr, [sp, #0x9d0]
1006c978c:     	str	xzr, [sp, #0x9d8]
1006c9790:     	str	wzr, [sp, #0x9e0]
1006c9794:     	str	xzr, [sp, #0x9e8]
1006c9798:     	str	wzr, [sp, #0x9f0]
1006c979c:     	str	xzr, [sp, #0x9f8]
1006c97a0:     	str	wzr, [sp, #0xa00]
1006c97a4:     	str	xzr, [sp, #0xa08]
1006c97a8:     	str	wzr, [sp, #0xa10]
1006c97ac:     	str	xzr, [sp, #0xa18]
1006c97b0:     	str	wzr, [sp, #0xa20]
1006c97b4:     	str	xzr, [sp, #0xa28]
1006c97b8:     	str	wzr, [sp, #0xa30]
1006c97bc:     	str	xzr, [sp, #0xa38]
1006c97c0:     	str	wzr, [sp, #0xa40]
1006c97c4:     	str	xzr, [sp, #0xa48]
1006c97c8:     	str	wzr, [sp, #0xa50]
1006c97cc:     	str	xzr, [sp, #0xa58]
1006c97d0:     	str	wzr, [sp, #0xa60]
1006c97d4:     	str	xzr, [sp, #0xa68]
1006c97d8:     	str	wzr, [sp, #0xa70]
1006c97dc:     	str	xzr, [sp, #0xa78]
1006c97e0:     	str	wzr, [sp, #0xa80]
1006c97e4:     	str	xzr, [sp, #0xa88]
1006c97e8:     	str	wzr, [sp, #0xa90]
1006c97ec:     	str	xzr, [sp, #0xa98]
1006c97f0:     	str	wzr, [sp, #0xaa0]
1006c97f4:     	str	xzr, [sp, #0xaa8]
1006c97f8:     	str	wzr, [sp, #0xab0]
1006c97fc:     	str	xzr, [sp, #0xab8]
1006c9800:     	str	wzr, [sp, #0xac0]
1006c9804:     	str	xzr, [sp, #0xac8]
1006c9808:     	str	wzr, [sp, #0xad0]
1006c980c:     	str	xzr, [sp, #0xad8]
1006c9810:     	str	wzr, [sp, #0xae0]
1006c9814:     	str	xzr, [sp, #0xae8]
1006c9818:     	str	wzr, [sp, #0xaf0]
1006c981c:     	str	xzr, [sp, #0xaf8]
1006c9820:     	str	wzr, [sp, #0xb00]
1006c9824:     	str	xzr, [sp, #0xb08]
1006c9828:     	str	wzr, [sp, #0xb10]
1006c982c:     	str	xzr, [sp, #0xb18]
1006c9830:     	str	wzr, [sp, #0xb20]
1006c9834:     	str	xzr, [sp, #0xb28]
1006c9838:     	str	wzr, [sp, #0xb30]
1006c983c:     	str	xzr, [sp, #0xb38]
1006c9840:     	str	wzr, [sp, #0xb40]
1006c9844:     	str	xzr, [sp, #0xb48]
1006c9848:     	str	wzr, [sp, #0xb50]
1006c984c:     	str	xzr, [sp, #0xb58]
1006c9850:     	str	wzr, [sp, #0xb60]
1006c9854:     	str	xzr, [sp, #0xb68]
1006c9858:     	str	wzr, [sp, #0xb70]
1006c985c:     	str	xzr, [sp, #0xb78]
1006c9860:     	str	wzr, [sp, #0xb80]
1006c9864:     	str	xzr, [sp, #0xb88]
1006c9868:     	str	wzr, [sp, #0xb90]
1006c986c:     	str	xzr, [sp, #0xb98]
1006c9870:     	str	wzr, [sp, #0xba0]
1006c9874:     	str	xzr, [sp, #0xba8]
1006c9878:     	str	wzr, [sp, #0xbb0]
1006c987c:     	str	xzr, [sp, #0xbb8]
1006c9880:     	str	wzr, [sp, #0xbc0]
1006c9884:     	str	xzr, [sp, #0xbc8]
1006c9888:     	str	wzr, [sp, #0xbd0]
1006c988c:     	str	xzr, [sp, #0xbd8]
1006c9890:     	str	wzr, [sp, #0xbe0]
1006c9894:     	str	xzr, [sp, #0xbe8]
1006c9898:     	str	wzr, [sp, #0xbf0]
1006c989c:     	str	xzr, [sp, #0xbf8]
1006c98a0:     	str	wzr, [sp, #0xc00]
1006c98a4:     	str	xzr, [sp, #0xc08]
1006c98a8:     	str	wzr, [sp, #0xc10]
1006c98ac:     	str	xzr, [sp, #0xc18]
1006c98b0:     	str	wzr, [sp, #0xc20]
1006c98b4:     	str	xzr, [sp, #0xc28]
1006c98b8:     	str	wzr, [sp, #0xc30]
1006c98bc:     	add	x20, sp, #0x38
1006c98c0:     	add	x3, sp, #0x38
1006c98c4:     	mov	x0, x23
1006c98c8:     	mov	x2, x24
1006c98cc:     	bl	0x1006c8780 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411branches_at>
1006c98d0:     	mov	x26, x0
1006c98d4:     	add	x3, sp, #0x438
1006c98d8:     	mov	x0, x23
1006c98dc:     	mov	x1, x25
1006c98e0:     	mov	x2, x24
1006c98e4:     	bl	0x1006c8780 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411branches_at>
1006c98e8:     	cbz	x26, 0x1006c998c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x8cc>
1006c98ec:     	cbz	x0, 0x1006c998c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x8cc>
1006c98f0:     	stp	x24, x27, [sp, #0x8]
1006c98f4:     	str	w21, [sp, #0x1c]
1006c98f8:     	mov	x25, #0x0               ; =0
1006c98fc:     	add	x9, x20, x26, lsl #4
1006c9900:     	lsl	x8, x0, #4
1006c9904:     	str	x8, [sp, #0x30]
1006c9908:     	add	x8, sp, #0x438
1006c990c:     	add	x8, x8, #0x8
1006c9910:     	stp	x9, x8, [sp, #0x20]
1006c9914:     	add	x24, sp, #0x38
1006c9918:     	add	x21, sp, #0x838
1006c991c:     	b	0x1006c992c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x86c>
1006c9920:     	ldr	x8, [sp, #0x20]
1006c9924:     	cmp	x24, x8
1006c9928:     	b.eq	0x1006c9994 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x8d4>
1006c992c:     	mov	x20, x24
1006c9930:     	add	x24, x24, #0x10
1006c9934:     	ldp	x28, x27, [sp, #0x28]
1006c9938:     	b	0x1006c9948 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x888>
1006c993c:     	add	x28, x28, #0x10
1006c9940:     	subs	x27, x27, #0x10
1006c9944:     	b.eq	0x1006c9920 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x860>
1006c9948:     	ldr	x8, [x20]
1006c994c:     	ldur	x9, [x28, #-0x8]
1006c9950:     	ands	x26, x9, x8
1006c9954:     	b.eq	0x1006c993c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x87c>
1006c9958:     	ldr	w1, [x20, #0x8]
1006c995c:     	ldr	w2, [x28]
1006c9960:     	mov	x0, x23
1006c9964:     	mov	x3, x22
1006c9968:     	mov	x4, x19
1006c996c:     	bl	0x1006c90c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec>
1006c9970:     	cmp	x25, #0x3f
1006c9974:     	b.hi	0x1006c9a10 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x950>
1006c9978:     	add	x8, x21, x25, lsl #4
1006c997c:     	str	x26, [x8]
1006c9980:     	str	w0, [x8, #0x8]
1006c9984:     	add	x25, x25, #0x1
1006c9988:     	b	0x1006c993c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x87c>
1006c998c:     	mov	x25, #0x0               ; =0
1006c9990:     	b	0x1006c99a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x8e4>
1006c9994:     	cmp	x25, #0x41
1006c9998:     	b.hs	0x1006c99f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block6411product_rec+0x938>
1006c999c:     	ldr	w21, [sp, #0x1c]
1006c99a0:     	ldp	x24, x27, [sp, #0x8]
1006c99a4:     	add	x2, sp, #0x838
1006c99a8:     	mov	x0, x23
1006c99ac:     	mov	x1, x24
1006c99b0:     	mov	x3, x25
1006c99b4:     	mov	x4, x22
1006c99b8:     	bl	0x1006ccfc8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6blocksNtB2_7Block648quantify>
1006c99bc:     	mov	x24, x0
1006c99c0:     	mov	x0, x19
1006c99c4:     	mov	x1, x27
1006c99c8:     	mov	x2, x21
1006c99cc:     	mov	x3, x24
1006c99d0:     	bl	0x100728c0c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006c99d4:     	mov	x0, x24
1006c99d8:     	add	sp, sp, #0xc40
1006c99dc:     	ldp	x29, x30, [sp, #0x50]
1006c99e0:     	ldp	x20, x19, [sp, #0x40]
1006c99e4:     	ldp	x22, x21, [sp, #0x30]
1006c99e8:     	ldp	x24, x23, [sp, #0x20]
1006c99ec:     	ldp	x26, x25, [sp, #0x10]
1006c99f0:     	ldp	x28, x27, [sp], #0x60
1006c99f4:     	ret
1006c99f8:     	adrp	x3, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006c99fc:     	add	x3, x3, #0x310
1006c9a00:     	mov	x0, #0x0                ; =0
1006c9a04:     	mov	x1, x25
1006c9a08:     	mov	w2, #0x40               ; =64
1006c9a0c:     	bl	0x100c9afd4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
1006c9a10:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006c9a14:     	add	x2, x2, #0x2f8
1006c9a18:     	mov	x0, x25
1006c9a1c:     	mov	w1, #0x40               ; =64
1006c9a20:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006c9a24:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006c9a28:     	add	x2, x2, #0x238
1006c9a2c:     	mov	x1, x8
1006c9a30:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006c9a34:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006c9a38:     	add	x2, x2, #0x568
1006c9a3c:     	mov	x0, x9
1006c9a40:     	mov	x1, x8
1006c9a44:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006c9a48:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006c9a4c:     	add	x2, x2, #0x580
1006c9a50:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
