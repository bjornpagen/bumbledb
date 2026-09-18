
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010e813c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_>:
1010e813c:     	stp	d15, d14, [sp, #-0xa0]!
1010e8140:     	stp	d13, d12, [sp, #0x10]
1010e8144:     	stp	d11, d10, [sp, #0x20]
1010e8148:     	stp	d9, d8, [sp, #0x30]
1010e814c:     	stp	x28, x27, [sp, #0x40]
1010e8150:     	stp	x26, x25, [sp, #0x50]
1010e8154:     	stp	x24, x23, [sp, #0x60]
1010e8158:     	stp	x22, x21, [sp, #0x70]
1010e815c:     	stp	x20, x19, [sp, #0x80]
1010e8160:     	stp	x29, x30, [sp, #0x90]
1010e8164:     	add	x29, sp, #0x90
1010e8168:     	sub	sp, sp, #0x220
1010e816c:     	str	x0, [sp, #0xe8]
1010e8170:     	ldr	w23, [x3, #0x10]
1010e8174:     	cbz	w23, 0x1010e82a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x168>
1010e8178:     	mov	x19, x3
1010e817c:     	ldr	w26, [x3, #0x28]
1010e8180:     	cbz	w26, 0x1010e82a4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x168>
1010e8184:     	ldr	x8, [x4, #0x18]
1010e8188:     	cbz	x8, 0x1010e82ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x170>
1010e818c:     	mov	x8, #0x0                ; =0
1010e8190:     	mov	x13, #0xa9c5            ; =43461
1010e8194:     	movk	x13, #0x2e62, lsl #16
1010e8198:     	movk	x13, #0x7aea, lsl #32
1010e819c:     	movk	x13, #0xf135, lsl #48
1010e81a0:     	ldp	x9, x10, [x19]
1010e81a4:     	madd	x11, x23, x13, x9
1010e81a8:     	mov	x12, #0x6332            ; =25394
1010e81ac:     	movk	x12, #0x6ed3, lsl #16
1010e81b0:     	movk	x12, #0x765a, lsl #32
1010e81b4:     	movk	x12, #0x284f, lsl #48
1010e81b8:     	mul	x12, x12, x13
1010e81bc:     	madd	x11, x11, x13, x12
1010e81c0:     	add	x11, x11, x10
1010e81c4:     	madd	x14, x11, x13, x26
1010e81c8:     	ldp	x11, x12, [x19, #0x18]
1010e81cc:     	madd	x14, x14, x13, x11
1010e81d0:     	madd	x14, x14, x13, x12
1010e81d4:     	mul	x13, x14, x13
1010e81d8:     	ror	x16, x13, #0x2c
1010e81dc:     	lsr	x15, x16, #57
1010e81e0:     	ldp	x14, x13, [x4]
1010e81e4:     	dup.8b	v0, w15
1010e81e8:     	movi.2d	v1, #0xffffffffffffffff
1010e81ec:     	mov	w15, #0x38              ; =56
1010e81f0:     	and	x16, x16, x13
1010e81f4:     	ldr	d2, [x14, x16]
1010e81f8:     	cmeq.8b	v3, v2, v0
1010e81fc:     	fmov	x17, d3
1010e8200:     	ands	x17, x17, #0x8080808080808080
1010e8204:     	b.eq	0x1010e8274 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x138>
1010e8208:     	rbit	x0, x17
1010e820c:     	clz	x0, x0
1010e8210:     	add	x0, x16, x0, lsr #3
1010e8214:     	and	x0, x0, x13
1010e8218:     	mneg	x0, x0, x15
1010e821c:     	add	x0, x14, x0
1010e8220:     	ldur	x3, [x0, #-0x38]
1010e8224:     	cmp	x9, x3
1010e8228:     	b.ne	0x1010e8268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x12c>
1010e822c:     	ldur	x3, [x0, #-0x30]
1010e8230:     	cmp	x10, x3
1010e8234:     	b.ne	0x1010e8268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x12c>
1010e8238:     	ldur	w3, [x0, #-0x28]
1010e823c:     	cmp	w23, w3
1010e8240:     	b.ne	0x1010e8268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x12c>
1010e8244:     	ldur	x3, [x0, #-0x20]
1010e8248:     	cmp	x11, x3
1010e824c:     	b.ne	0x1010e8268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x12c>
1010e8250:     	ldur	x3, [x0, #-0x18]
1010e8254:     	cmp	x12, x3
1010e8258:     	b.ne	0x1010e8268 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x12c>
1010e825c:     	ldur	w3, [x0, #-0x10]
1010e8260:     	cmp	w26, w3
1010e8264:     	b.eq	0x1010e834c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x210>
1010e8268:     	sub	x0, x17, #0x2
1010e826c:     	ands	x17, x0, x17
1010e8270:     	b.ne	0x1010e8208 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0xcc>
1010e8274:     	cmeq.8b	v2, v2, v1
1010e8278:     	fmov	x17, d2
1010e827c:     	cbnz	x17, 0x1010e82ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x170>
1010e8280:     	add	x8, x8, #0x8
1010e8284:     	add	x16, x16, x8
1010e8288:     	and	x16, x16, x13
1010e828c:     	ldr	d2, [x14, x16]
1010e8290:     	cmeq.8b	v3, v2, v0
1010e8294:     	fmov	x17, d3
1010e8298:     	ands	x17, x17, #0x8080808080808080
1010e829c:     	b.ne	0x1010e8208 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0xcc>
1010e82a0:     	b	0x1010e8274 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x138>
1010e82a4:     	mov	x27, #0x0               ; =0
1010e82a8:     	b	0x1010e892c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7f0>
1010e82ac:     	mov	x22, x1
1010e82b0:     	mov	x24, x2
1010e82b4:     	str	x4, [sp, #0x68]
1010e82b8:     	ldr	x0, [sp, #0xe8]
1010e82bc:     	mov	x1, x19
1010e82c0:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e82c4:     	mov	x25, x0
1010e82c8:     	fmov	d0, x25
1010e82cc:     	cnt.8b	v0, v0
1010e82d0:     	addv.8b	b0, v0
1010e82d4:     	fmov	x20, d0
1010e82d8:     	cmp	x20, #0x7
1010e82dc:     	b.hs	0x1010e8354 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x218>
1010e82e0:     	ldr	x8, [x19, #0x8]
1010e82e4:     	str	x8, [sp, #0xe0]
1010e82e8:     	ldr	x8, [x19, #0x20]
1010e82ec:     	str	x8, [sp, #0xb8]
1010e82f0:     	mov	w8, #0x4                ; =4
1010e82f4:     	stp	xzr, x8, [sp, #0x150]
1010e82f8:     	str	xzr, [sp, #0x160]
1010e82fc:     	cbz	x25, 0x1010e88dc <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7a0>
1010e8300:     	mov	x28, #0x0               ; =0
1010e8304:     	mov	w8, #0x4                ; =4
1010e8308:     	b	0x1010e8330 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x1f4>
1010e830c:     	ldr	x8, [sp, #0x158]
1010e8310:     	rbit	x9, x25
1010e8314:     	clz	x9, x9
1010e8318:     	str	w9, [x8, x21, lsl #2]
1010e831c:     	add	x28, x21, #0x1
1010e8320:     	str	x28, [sp, #0x160]
1010e8324:     	sub	x9, x25, #0x1
1010e8328:     	ands	x25, x9, x25
1010e832c:     	b.eq	0x1010e858c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x450>
1010e8330:     	mov	x21, x28
1010e8334:     	ldr	x9, [sp, #0x150]
1010e8338:     	cmp	x28, x9
1010e833c:     	b.ne	0x1010e8310 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x1d4>
1010e8340:     	add	x0, sp, #0x150
1010e8344:     	bl	0x1018c259c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1010e8348:     	b	0x1010e830c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x1d0>
1010e834c:     	ldur	x27, [x0, #-0x8]
1010e8350:     	b	0x1010e892c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7f0>
1010e8354:     	mov	x8, #0x0                ; =0
1010e8358:     	sub	x21, x29, #0xf8
1010e835c:     	lsl	x9, x24, #2
1010e8360:     	cmp	x9, x8
1010e8364:     	b.eq	0x1010e8960 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x824>
1010e8368:     	ldr	w23, [x22, x8]
1010e836c:     	lsr	x10, x25, x23
1010e8370:     	add	x8, x8, #0x4
1010e8374:     	tbz	w10, #0x0, 0x1010e8360 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x224>
1010e8378:     	ldr	q0, [x19]
1010e837c:     	stur	q0, [x29, #-0xc0]
1010e8380:     	ldr	x8, [x19, #0x10]
1010e8384:     	stur	x8, [x29, #-0xb0]
1010e8388:     	sub	x0, x29, #0xf8
1010e838c:     	sub	x1, x29, #0xc0
1010e8390:     	ldr	x27, [sp, #0xe8]
1010e8394:     	mov	x2, x27
1010e8398:     	mov	x3, x23
1010e839c:     	mov	w4, #0x0                ; =0
1010e83a0:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e83a4:     	ldr	q0, [x21]
1010e83a8:     	str	q0, [sp, #0x1a0]
1010e83ac:     	ldur	x8, [x29, #-0xe8]
1010e83b0:     	str	q0, [sp, #0x180]
1010e83b4:     	stur	q0, [x29, #-0xe0]
1010e83b8:     	stur	x8, [x29, #-0xd0]
1010e83bc:     	ldur	q0, [x29, #-0xe0]
1010e83c0:     	str	x8, [sp, #0x160]
1010e83c4:     	str	q0, [sp, #0x150]
1010e83c8:     	ldur	q0, [x19, #0x18]
1010e83cc:     	stur	q0, [x29, #-0xc0]
1010e83d0:     	ldur	x8, [x19, #0x28]
1010e83d4:     	stur	x8, [x29, #-0xb0]
1010e83d8:     	sub	x0, x29, #0xf8
1010e83dc:     	sub	x1, x29, #0xc0
1010e83e0:     	mov	x2, x27
1010e83e4:     	mov	x3, x23
1010e83e8:     	mov	w4, #0x0                ; =0
1010e83ec:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e83f0:     	ldr	q0, [x21]
1010e83f4:     	str	q0, [sp, #0x1a0]
1010e83f8:     	ldur	x8, [x29, #-0xe8]
1010e83fc:     	str	q0, [sp, #0x180]
1010e8400:     	stur	q0, [x29, #-0xe0]
1010e8404:     	stur	x8, [x29, #-0xd0]
1010e8408:     	ldur	q0, [x29, #-0xe0]
1010e840c:     	str	x8, [sp, #0x178]
1010e8410:     	add	x8, sp, #0x69
1010e8414:     	stur	q0, [x8, #0xff]
1010e8418:     	ldp	q0, q1, [sp, #0x150]
1010e841c:     	ldr	q2, [sp, #0x170]
1010e8420:     	stp	q1, q2, [sp, #0x130]
1010e8424:     	str	q0, [sp, #0x120]
1010e8428:     	ldp	q0, q1, [sp, #0x120]
1010e842c:     	ldr	q2, [sp, #0x140]
1010e8430:     	stp	q1, q2, [sp, #0x100]
1010e8434:     	str	q0, [sp, #0xf0]
1010e8438:     	add	x1, sp, #0xf0
1010e843c:     	mov	x0, x27
1010e8440:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e8444:     	mov	x26, x0
1010e8448:     	add	x3, sp, #0xf0
1010e844c:     	mov	x0, x27
1010e8450:     	mov	x1, x22
1010e8454:     	mov	x25, x24
1010e8458:     	mov	x2, x24
1010e845c:     	ldr	x24, [sp, #0x68]
1010e8460:     	mov	x4, x24
1010e8464:     	bl	0x1010e813c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_>
1010e8468:     	fmov	d0, x26
1010e846c:     	cnt.8b	v0, v0
1010e8470:     	addv.8b	b0, v0
1010e8474:     	fmov	w8, s0
1010e8478:     	sub	w8, w8, w20
1010e847c:     	mvn	w8, w8
1010e8480:     	lsl	x26, x0, x8
1010e8484:     	ldr	q0, [x19]
1010e8488:     	stur	q0, [x29, #-0xc0]
1010e848c:     	ldr	x8, [x19, #0x10]
1010e8490:     	stur	x8, [x29, #-0xb0]
1010e8494:     	sub	x0, x29, #0xf8
1010e8498:     	sub	x1, x29, #0xc0
1010e849c:     	mov	x2, x27
1010e84a0:     	mov	x3, x23
1010e84a4:     	mov	w4, #0x1                ; =1
1010e84a8:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e84ac:     	ldr	q0, [x21]
1010e84b0:     	str	q0, [sp, #0x1a0]
1010e84b4:     	ldur	x8, [x29, #-0xe8]
1010e84b8:     	str	q0, [sp, #0x180]
1010e84bc:     	stur	q0, [x29, #-0xe0]
1010e84c0:     	stur	x8, [x29, #-0xd0]
1010e84c4:     	ldur	q0, [x29, #-0xe0]
1010e84c8:     	str	x8, [sp, #0x160]
1010e84cc:     	str	q0, [sp, #0x150]
1010e84d0:     	ldur	q0, [x19, #0x18]
1010e84d4:     	stur	q0, [x29, #-0xc0]
1010e84d8:     	ldur	x8, [x19, #0x28]
1010e84dc:     	stur	x8, [x29, #-0xb0]
1010e84e0:     	sub	x0, x29, #0xf8
1010e84e4:     	sub	x1, x29, #0xc0
1010e84e8:     	mov	x2, x27
1010e84ec:     	mov	x3, x23
1010e84f0:     	mov	w4, #0x1                ; =1
1010e84f4:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e84f8:     	ldr	q0, [x21]
1010e84fc:     	str	q0, [sp, #0x1a0]
1010e8500:     	ldur	x8, [x29, #-0xe8]
1010e8504:     	str	q0, [sp, #0x180]
1010e8508:     	stur	q0, [x29, #-0xe0]
1010e850c:     	stur	x8, [x29, #-0xd0]
1010e8510:     	ldur	q0, [x29, #-0xe0]
1010e8514:     	str	x8, [sp, #0x178]
1010e8518:     	add	x8, sp, #0x69
1010e851c:     	stur	q0, [x8, #0xff]
1010e8520:     	ldp	q0, q1, [sp, #0x150]
1010e8524:     	ldr	q2, [sp, #0x170]
1010e8528:     	stp	q1, q2, [sp, #0x130]
1010e852c:     	str	q0, [sp, #0x120]
1010e8530:     	ldp	q0, q1, [sp, #0x120]
1010e8534:     	ldr	q2, [sp, #0x140]
1010e8538:     	stp	q1, q2, [sp, #0x100]
1010e853c:     	str	q0, [sp, #0xf0]
1010e8540:     	add	x1, sp, #0xf0
1010e8544:     	mov	x0, x27
1010e8548:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e854c:     	mov	x23, x0
1010e8550:     	add	x3, sp, #0xf0
1010e8554:     	mov	x0, x27
1010e8558:     	mov	x1, x22
1010e855c:     	mov	x2, x25
1010e8560:     	mov	x4, x24
1010e8564:     	bl	0x1010e813c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_>
1010e8568:     	fmov	d0, x23
1010e856c:     	cnt.8b	v0, v0
1010e8570:     	addv.8b	b0, v0
1010e8574:     	fmov	w8, s0
1010e8578:     	sub	w8, w8, w20
1010e857c:     	mvn	w8, w8
1010e8580:     	lsl	x8, x0, x8
1010e8584:     	add	x27, x8, x26
1010e8588:     	b	0x1010e891c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7e0>
1010e858c:     	ldp	x20, x25, [sp, #0x150]
1010e8590:     	cbz	x28, 0x1010e88e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7a8>
1010e8594:     	str	x20, [sp, #0x8]
1010e8598:     	str	x26, [sp, #0xb0]
1010e859c:     	mov	x27, #0x0               ; =0
1010e85a0:     	mov	x24, #0x0               ; =0
1010e85a4:     	str	xzr, [sp, #0xd8]
1010e85a8:     	adrp	x8, 0x101973000 <GCC_except_table10364+0x18>
1010e85ac:     	ldr	q0, [x8, #0x690]
1010e85b0:     	str	q0, [sp, #0xc0]
1010e85b4:     	mov	w22, #0x3f              ; =63
1010e85b8:     	dup.2d	v1, x22
1010e85bc:     	and	x9, x28, #0x1ffffffffffffffe
1010e85c0:     	mov	w8, #0x2                ; =2
1010e85c4:     	dup.2d	v0, x8
1010e85c8:     	stp	q0, q1, [sp, #0x90]
1010e85cc:     	neg	x8, x9
1010e85d0:     	str	x8, [sp, #0x88]
1010e85d4:     	mov	w8, #0x4                ; =4
1010e85d8:     	dup.2d	v1, x8
1010e85dc:     	mov	w8, #0x8                ; =8
1010e85e0:     	dup.2d	v0, x8
1010e85e4:     	stp	q0, q1, [sp, #0x40]
1010e85e8:     	mov	w8, #0xc                ; =12
1010e85ec:     	dup.2d	v1, x8
1010e85f0:     	mov	w8, #0x10               ; =16
1010e85f4:     	dup.2d	v0, x8
1010e85f8:     	stp	q0, q1, [sp, #0x20]
1010e85fc:     	adrp	x8, 0x101973000 <GCC_except_table10364+0x18>
1010e8600:     	ldr	q0, [x8, #0x680]
1010e8604:     	str	q0, [sp, #0x10]
1010e8608:     	mov	w20, #0x1               ; =1
1010e860c:     	dup.2d	v0, x20
1010e8610:     	str	q0, [sp, #0x70]
1010e8614:     	movi.2s	v8, #0x3f
1010e8618:     	b	0x1010e862c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x4f0>
1010e861c:     	add	x24, x24, #0x1
1010e8620:     	and	x8, x28, #0x3f
1010e8624:     	lsr	x8, x24, x8
1010e8628:     	cbnz	x8, 0x1010e88d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x798>
1010e862c:     	cbz	x21, 0x1010e8648 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x50c>
1010e8630:     	dup.2d	v0, x24
1010e8634:     	cmp	x28, #0x10
1010e8638:     	b.hs	0x1010e8654 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x518>
1010e863c:     	mov	x9, #0x0                ; =0
1010e8640:     	mov	x26, #0x0               ; =0
1010e8644:     	b	0x1010e8808 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x6cc>
1010e8648:     	mov	x8, #0x0                ; =0
1010e864c:     	mov	x26, #0x0               ; =0
1010e8650:     	b	0x1010e8878 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x73c>
1010e8654:     	movi.2d	v1, #0000000000000000
1010e8658:     	add	x8, x25, #0x20
1010e865c:     	movi.2d	v2, #0000000000000000
1010e8660:     	and	x9, x28, #0x1ffffffffffffff0
1010e8664:     	ldr	q4, [sp, #0xc0]
1010e8668:     	ldp	q6, q15, [sp, #0x10]
1010e866c:     	movi.2d	v3, #0000000000000000
1010e8670:     	movi.2d	v7, #0000000000000000
1010e8674:     	movi.2d	v16, #0000000000000000
1010e8678:     	movi.2d	v5, #0000000000000000
1010e867c:     	movi.2d	v18, #0000000000000000
1010e8680:     	movi.2d	v17, #0000000000000000
1010e8684:     	ldp	q13, q12, [sp, #0x40]
1010e8688:     	ldr	q14, [sp, #0x30]
1010e868c:     	movi.4s	v8, #0x3f
1010e8690:     	add.2d	v19, v4, v12
1010e8694:     	add.2d	v20, v6, v12
1010e8698:     	add.2d	v21, v4, v13
1010e869c:     	add.2d	v22, v6, v13
1010e86a0:     	add.2d	v23, v4, v14
1010e86a4:     	add.2d	v24, v6, v14
1010e86a8:     	ldp	q25, q26, [x8, #-0x20]
1010e86ac:     	dup.2d	v27, x22
1010e86b0:     	ldp	q28, q29, [x8], #0x40
1010e86b4:     	and.16b	v30, v6, v27
1010e86b8:     	and.16b	v31, v4, v27
1010e86bc:     	and.16b	v20, v20, v27
1010e86c0:     	and.16b	v19, v19, v27
1010e86c4:     	and.16b	v22, v22, v27
1010e86c8:     	and.16b	v21, v21, v27
1010e86cc:     	and.16b	v24, v24, v27
1010e86d0:     	and.16b	v23, v23, v27
1010e86d4:     	neg.2d	v27, v31
1010e86d8:     	ushl.2d	v27, v0, v27
1010e86dc:     	neg.2d	v30, v30
1010e86e0:     	ushl.2d	v30, v0, v30
1010e86e4:     	neg.2d	v19, v19
1010e86e8:     	ushl.2d	v19, v0, v19
1010e86ec:     	neg.2d	v20, v20
1010e86f0:     	ushl.2d	v20, v0, v20
1010e86f4:     	neg.2d	v21, v21
1010e86f8:     	ushl.2d	v21, v0, v21
1010e86fc:     	neg.2d	v22, v22
1010e8700:     	ushl.2d	v22, v0, v22
1010e8704:     	neg.2d	v23, v23
1010e8708:     	ushl.2d	v23, v0, v23
1010e870c:     	neg.2d	v24, v24
1010e8710:     	ushl.2d	v24, v0, v24
1010e8714:     	dup.2d	v31, x20
1010e8718:     	and.16b	v30, v30, v31
1010e871c:     	and.16b	v27, v27, v31
1010e8720:     	and.16b	v20, v20, v31
1010e8724:     	and.16b	v19, v19, v31
1010e8728:     	and.16b	v22, v22, v31
1010e872c:     	and.16b	v21, v21, v31
1010e8730:     	and.16b	v24, v24, v31
1010e8734:     	and.16b	v23, v23, v31
1010e8738:     	and.16b	v25, v25, v8
1010e873c:     	and.16b	v26, v26, v8
1010e8740:     	and.16b	v28, v28, v8
1010e8744:     	and.16b	v29, v29, v8
1010e8748:     	ushll2.2d	v31, v25, #0x0
1010e874c:     	ushll.2d	v25, v25, #0x0
1010e8750:     	ushll2.2d	v9, v26, #0x0
1010e8754:     	ushll.2d	v26, v26, #0x0
1010e8758:     	ushll2.2d	v10, v28, #0x0
1010e875c:     	ushll.2d	v28, v28, #0x0
1010e8760:     	ushll2.2d	v11, v29, #0x0
1010e8764:     	ushll.2d	v29, v29, #0x0
1010e8768:     	ushl.2d	v25, v27, v25
1010e876c:     	ushl.2d	v27, v30, v31
1010e8770:     	ushl.2d	v19, v19, v26
1010e8774:     	ushl.2d	v20, v20, v9
1010e8778:     	ushl.2d	v21, v21, v28
1010e877c:     	ushl.2d	v22, v22, v10
1010e8780:     	ushl.2d	v23, v23, v29
1010e8784:     	ushl.2d	v24, v24, v11
1010e8788:     	orr.16b	v3, v27, v3
1010e878c:     	orr.16b	v2, v25, v2
1010e8790:     	orr.16b	v16, v20, v16
1010e8794:     	orr.16b	v7, v19, v7
1010e8798:     	orr.16b	v18, v22, v18
1010e879c:     	orr.16b	v5, v21, v5
1010e87a0:     	orr.16b	v1, v24, v1
1010e87a4:     	orr.16b	v17, v23, v17
1010e87a8:     	add.2d	v6, v6, v15
1010e87ac:     	add.2d	v4, v4, v15
1010e87b0:     	subs	x9, x9, #0x10
1010e87b4:     	b.ne	0x1010e8690 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x554>
1010e87b8:     	orr.16b	v2, v7, v2
1010e87bc:     	orr.16b	v3, v16, v3
1010e87c0:     	orr.16b	v3, v18, v3
1010e87c4:     	orr.16b	v2, v5, v2
1010e87c8:     	orr.16b	v2, v17, v2
1010e87cc:     	orr.16b	v1, v1, v3
1010e87d0:     	orr.16b	v1, v2, v1
1010e87d4:     	mov	d2, v1[1]
1010e87d8:     	orr.8b	v1, v1, v2
1010e87dc:     	fmov	x26, d1
1010e87e0:     	and	x8, x28, #0x1ffffffffffffff0
1010e87e4:     	cmp	x28, x8
1010e87e8:     	b.ne	0x1010e87f4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x6b8>
1010e87ec:     	movi.2s	v8, #0x3f
1010e87f0:     	b	0x1010e8898 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x75c>
1010e87f4:     	and	x9, x28, #0x1ffffffffffffff0
1010e87f8:     	and	x8, x28, #0x1ffffffffffffff0
1010e87fc:     	and	x10, x28, #0xe
1010e8800:     	movi.2s	v8, #0x3f
1010e8804:     	cbz	x10, 0x1010e8878 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x73c>
1010e8808:     	fmov	d1, x26
1010e880c:     	dup.2d	v2, x9
1010e8810:     	ldr	q3, [sp, #0xc0]
1010e8814:     	orr.16b	v2, v2, v3
1010e8818:     	ldr	x8, [sp, #0x88]
1010e881c:     	add	x8, x8, x9
1010e8820:     	add	x9, x25, x9, lsl #2
1010e8824:     	ldp	q6, q5, [sp, #0x90]
1010e8828:     	ldr	q7, [sp, #0x70]
1010e882c:     	ldr	d3, [x9], #0x8
1010e8830:     	and.16b	v4, v2, v5
1010e8834:     	neg.2d	v4, v4
1010e8838:     	ushl.2d	v4, v0, v4
1010e883c:     	and.16b	v4, v4, v7
1010e8840:     	and.8b	v3, v3, v8
1010e8844:     	ushll.2d	v3, v3, #0x0
1010e8848:     	ushl.2d	v3, v4, v3
1010e884c:     	orr.16b	v1, v3, v1
1010e8850:     	add.2d	v2, v2, v6
1010e8854:     	adds	x8, x8, #0x2
1010e8858:     	b.ne	0x1010e882c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x6f0>
1010e885c:     	mov	d0, v1[1]
1010e8860:     	orr.8b	v0, v1, v0
1010e8864:     	fmov	x26, d0
1010e8868:     	and	x8, x28, #0x1ffffffffffffffe
1010e886c:     	and	x9, x28, #0x1ffffffffffffffe
1010e8870:     	cmp	x28, x9
1010e8874:     	b.eq	0x1010e8898 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x75c>
1010e8878:     	ldr	w9, [x25, x8, lsl #2]
1010e887c:     	lsr	x10, x24, x8
1010e8880:     	and	x10, x10, #0x1
1010e8884:     	lsl	x9, x10, x9
1010e8888:     	orr	x26, x9, x26
1010e888c:     	add	x8, x8, #0x1
1010e8890:     	cmp	x28, x8
1010e8894:     	b.ne	0x1010e8878 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x73c>
1010e8898:     	ldp	x8, x0, [sp, #0xe0]
1010e889c:     	orr	x2, x26, x8
1010e88a0:     	mov	x1, x23
1010e88a4:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e88a8:     	cbz	w0, 0x1010e861c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x4e0>
1010e88ac:     	ldp	x1, x8, [sp, #0xb0]
1010e88b0:     	orr	x2, x26, x8
1010e88b4:     	ldr	x0, [sp, #0xe8]
1010e88b8:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e88bc:     	cmp	w0, #0x0
1010e88c0:     	ldr	x8, [sp, #0xd8]
1010e88c4:     	csinc	x27, x27, x8, eq
1010e88c8:     	cinc	x8, x8, ne
1010e88cc:     	str	x8, [sp, #0xd8]
1010e88d0:     	b	0x1010e861c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x4e0>
1010e88d4:     	ldr	x20, [sp, #0x8]
1010e88d8:     	b	0x1010e8910 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7d4>
1010e88dc:     	mov	x20, #0x0               ; =0
1010e88e0:     	mov	w25, #0x4               ; =4
1010e88e4:     	ldp	x2, x0, [sp, #0xe0]
1010e88e8:     	mov	x1, x23
1010e88ec:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e88f0:     	cbz	w0, 0x1010e890c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7d0>
1010e88f4:     	ldr	x0, [sp, #0xe8]
1010e88f8:     	mov	x1, x26
1010e88fc:     	ldr	x2, [sp, #0xb8]
1010e8900:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e8904:     	mov	w27, w0
1010e8908:     	b	0x1010e8910 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7d4>
1010e890c:     	mov	x27, #0x0               ; =0
1010e8910:     	cbz	x20, 0x1010e891c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x7e0>
1010e8914:     	mov	x0, x25
1010e8918:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e891c:     	ldr	x0, [sp, #0x68]
1010e8920:     	mov	x1, x19
1010e8924:     	mov	x2, x27
1010e8928:     	bl	0x10121d47c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
1010e892c:     	mov	x0, x27
1010e8930:     	add	sp, sp, #0x220
1010e8934:     	ldp	x29, x30, [sp, #0x90]
1010e8938:     	ldp	x20, x19, [sp, #0x80]
1010e893c:     	ldp	x22, x21, [sp, #0x70]
1010e8940:     	ldp	x24, x23, [sp, #0x60]
1010e8944:     	ldp	x26, x25, [sp, #0x50]
1010e8948:     	ldp	x28, x27, [sp, #0x40]
1010e894c:     	ldp	d9, d8, [sp, #0x30]
1010e8950:     	ldp	d11, d10, [sp, #0x20]
1010e8954:     	ldp	d13, d12, [sp, #0x10]
1010e8958:     	ldp	d15, d14, [sp], #0xa0
1010e895c:     	ret
1010e8960:     	adrp	x0, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010e8964:     	add	x0, x0, #0xf80
1010e8968:     	bl	0x1018c1834 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1010e896c:     	mov	x19, x0
1010e8970:     	cbnz	x20, 0x1010e8998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x85c>
1010e8974:     	b	0x1010e89a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x864>
1010e8978:     	mov	x19, x0
1010e897c:     	ldr	x8, [sp, #0x150]
1010e8980:     	cbz	x8, 0x1010e89a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x864>
1010e8984:     	ldr	x25, [sp, #0x158]
1010e8988:     	b	0x1010e8998 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x85c>
1010e898c:     	mov	x19, x0
1010e8990:     	ldr	x8, [sp, #0x8]
1010e8994:     	cbz	x8, 0x1010e89a0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm6_Kb0_EB8_+0x864>
1010e8998:     	mov	x0, x25
1010e899c:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e89a0:     	mov	x0, x19
1010e89a4:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
