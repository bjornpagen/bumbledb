
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010e8ff4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_>:
1010e8ff4:     	stp	d15, d14, [sp, #-0xa0]!
1010e8ff8:     	stp	d13, d12, [sp, #0x10]
1010e8ffc:     	stp	d11, d10, [sp, #0x20]
1010e9000:     	stp	d9, d8, [sp, #0x30]
1010e9004:     	stp	x28, x27, [sp, #0x40]
1010e9008:     	stp	x26, x25, [sp, #0x50]
1010e900c:     	stp	x24, x23, [sp, #0x60]
1010e9010:     	stp	x22, x21, [sp, #0x70]
1010e9014:     	stp	x20, x19, [sp, #0x80]
1010e9018:     	stp	x29, x30, [sp, #0x90]
1010e901c:     	add	x29, sp, #0x90
1010e9020:     	sub	sp, sp, #0x220
1010e9024:     	str	x0, [sp, #0xe8]
1010e9028:     	ldr	w23, [x3, #0x10]
1010e902c:     	cbz	w23, 0x1010e915c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x168>
1010e9030:     	mov	x19, x3
1010e9034:     	ldr	w26, [x3, #0x28]
1010e9038:     	cbz	w26, 0x1010e915c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x168>
1010e903c:     	ldr	x8, [x4, #0x18]
1010e9040:     	cbz	x8, 0x1010e9164 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x170>
1010e9044:     	mov	x8, #0x0                ; =0
1010e9048:     	mov	x13, #0xa9c5            ; =43461
1010e904c:     	movk	x13, #0x2e62, lsl #16
1010e9050:     	movk	x13, #0x7aea, lsl #32
1010e9054:     	movk	x13, #0xf135, lsl #48
1010e9058:     	ldp	x9, x10, [x19]
1010e905c:     	madd	x11, x23, x13, x9
1010e9060:     	mov	x12, #0x6332            ; =25394
1010e9064:     	movk	x12, #0x6ed3, lsl #16
1010e9068:     	movk	x12, #0x765a, lsl #32
1010e906c:     	movk	x12, #0x284f, lsl #48
1010e9070:     	mul	x12, x12, x13
1010e9074:     	madd	x11, x11, x13, x12
1010e9078:     	add	x11, x11, x10
1010e907c:     	madd	x14, x11, x13, x26
1010e9080:     	ldp	x11, x12, [x19, #0x18]
1010e9084:     	madd	x14, x14, x13, x11
1010e9088:     	madd	x14, x14, x13, x12
1010e908c:     	mul	x13, x14, x13
1010e9090:     	ror	x16, x13, #0x2c
1010e9094:     	lsr	x15, x16, #57
1010e9098:     	ldp	x14, x13, [x4]
1010e909c:     	dup.8b	v0, w15
1010e90a0:     	movi.2d	v1, #0xffffffffffffffff
1010e90a4:     	mov	w15, #0x38              ; =56
1010e90a8:     	and	x16, x16, x13
1010e90ac:     	ldr	d2, [x14, x16]
1010e90b0:     	cmeq.8b	v3, v2, v0
1010e90b4:     	fmov	x17, d3
1010e90b8:     	ands	x17, x17, #0x8080808080808080
1010e90bc:     	b.eq	0x1010e912c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x138>
1010e90c0:     	rbit	x0, x17
1010e90c4:     	clz	x0, x0
1010e90c8:     	add	x0, x16, x0, lsr #3
1010e90cc:     	and	x0, x0, x13
1010e90d0:     	mneg	x0, x0, x15
1010e90d4:     	add	x0, x14, x0
1010e90d8:     	ldur	x3, [x0, #-0x38]
1010e90dc:     	cmp	x9, x3
1010e90e0:     	b.ne	0x1010e9120 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x12c>
1010e90e4:     	ldur	x3, [x0, #-0x30]
1010e90e8:     	cmp	x10, x3
1010e90ec:     	b.ne	0x1010e9120 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x12c>
1010e90f0:     	ldur	w3, [x0, #-0x28]
1010e90f4:     	cmp	w23, w3
1010e90f8:     	b.ne	0x1010e9120 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x12c>
1010e90fc:     	ldur	x3, [x0, #-0x20]
1010e9100:     	cmp	x11, x3
1010e9104:     	b.ne	0x1010e9120 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x12c>
1010e9108:     	ldur	x3, [x0, #-0x18]
1010e910c:     	cmp	x12, x3
1010e9110:     	b.ne	0x1010e9120 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x12c>
1010e9114:     	ldur	w3, [x0, #-0x10]
1010e9118:     	cmp	w26, w3
1010e911c:     	b.eq	0x1010e9204 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x210>
1010e9120:     	sub	x0, x17, #0x2
1010e9124:     	ands	x17, x0, x17
1010e9128:     	b.ne	0x1010e90c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0xcc>
1010e912c:     	cmeq.8b	v2, v2, v1
1010e9130:     	fmov	x17, d2
1010e9134:     	cbnz	x17, 0x1010e9164 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x170>
1010e9138:     	add	x8, x8, #0x8
1010e913c:     	add	x16, x16, x8
1010e9140:     	and	x16, x16, x13
1010e9144:     	ldr	d2, [x14, x16]
1010e9148:     	cmeq.8b	v3, v2, v0
1010e914c:     	fmov	x17, d3
1010e9150:     	ands	x17, x17, #0x8080808080808080
1010e9154:     	b.ne	0x1010e90c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0xcc>
1010e9158:     	b	0x1010e912c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x138>
1010e915c:     	mov	x27, #0x0               ; =0
1010e9160:     	b	0x1010e97e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7f0>
1010e9164:     	mov	x22, x1
1010e9168:     	mov	x24, x2
1010e916c:     	str	x4, [sp, #0x68]
1010e9170:     	ldr	x0, [sp, #0xe8]
1010e9174:     	mov	x1, x19
1010e9178:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e917c:     	mov	x25, x0
1010e9180:     	fmov	d0, x25
1010e9184:     	cnt.8b	v0, v0
1010e9188:     	addv.8b	b0, v0
1010e918c:     	fmov	x20, d0
1010e9190:     	cmp	x20, #0xa
1010e9194:     	b.hs	0x1010e920c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x218>
1010e9198:     	ldr	x8, [x19, #0x8]
1010e919c:     	str	x8, [sp, #0xe0]
1010e91a0:     	ldr	x8, [x19, #0x20]
1010e91a4:     	str	x8, [sp, #0xb8]
1010e91a8:     	mov	w8, #0x4                ; =4
1010e91ac:     	stp	xzr, x8, [sp, #0x150]
1010e91b0:     	str	xzr, [sp, #0x160]
1010e91b4:     	cbz	x25, 0x1010e9794 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7a0>
1010e91b8:     	mov	x28, #0x0               ; =0
1010e91bc:     	mov	w8, #0x4                ; =4
1010e91c0:     	b	0x1010e91e8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x1f4>
1010e91c4:     	ldr	x8, [sp, #0x158]
1010e91c8:     	rbit	x9, x25
1010e91cc:     	clz	x9, x9
1010e91d0:     	str	w9, [x8, x21, lsl #2]
1010e91d4:     	add	x28, x21, #0x1
1010e91d8:     	str	x28, [sp, #0x160]
1010e91dc:     	sub	x9, x25, #0x1
1010e91e0:     	ands	x25, x9, x25
1010e91e4:     	b.eq	0x1010e9444 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x450>
1010e91e8:     	mov	x21, x28
1010e91ec:     	ldr	x9, [sp, #0x150]
1010e91f0:     	cmp	x28, x9
1010e91f4:     	b.ne	0x1010e91c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x1d4>
1010e91f8:     	add	x0, sp, #0x150
1010e91fc:     	bl	0x1018c259c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
1010e9200:     	b	0x1010e91c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x1d0>
1010e9204:     	ldur	x27, [x0, #-0x8]
1010e9208:     	b	0x1010e97e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7f0>
1010e920c:     	mov	x8, #0x0                ; =0
1010e9210:     	sub	x21, x29, #0xf8
1010e9214:     	lsl	x9, x24, #2
1010e9218:     	cmp	x9, x8
1010e921c:     	b.eq	0x1010e9818 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x824>
1010e9220:     	ldr	w23, [x22, x8]
1010e9224:     	lsr	x10, x25, x23
1010e9228:     	add	x8, x8, #0x4
1010e922c:     	tbz	w10, #0x0, 0x1010e9218 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x224>
1010e9230:     	ldr	q0, [x19]
1010e9234:     	stur	q0, [x29, #-0xc0]
1010e9238:     	ldr	x8, [x19, #0x10]
1010e923c:     	stur	x8, [x29, #-0xb0]
1010e9240:     	sub	x0, x29, #0xf8
1010e9244:     	sub	x1, x29, #0xc0
1010e9248:     	ldr	x27, [sp, #0xe8]
1010e924c:     	mov	x2, x27
1010e9250:     	mov	x3, x23
1010e9254:     	mov	w4, #0x0                ; =0
1010e9258:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e925c:     	ldr	q0, [x21]
1010e9260:     	str	q0, [sp, #0x1a0]
1010e9264:     	ldur	x8, [x29, #-0xe8]
1010e9268:     	str	q0, [sp, #0x180]
1010e926c:     	stur	q0, [x29, #-0xe0]
1010e9270:     	stur	x8, [x29, #-0xd0]
1010e9274:     	ldur	q0, [x29, #-0xe0]
1010e9278:     	str	x8, [sp, #0x160]
1010e927c:     	str	q0, [sp, #0x150]
1010e9280:     	ldur	q0, [x19, #0x18]
1010e9284:     	stur	q0, [x29, #-0xc0]
1010e9288:     	ldur	x8, [x19, #0x28]
1010e928c:     	stur	x8, [x29, #-0xb0]
1010e9290:     	sub	x0, x29, #0xf8
1010e9294:     	sub	x1, x29, #0xc0
1010e9298:     	mov	x2, x27
1010e929c:     	mov	x3, x23
1010e92a0:     	mov	w4, #0x0                ; =0
1010e92a4:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e92a8:     	ldr	q0, [x21]
1010e92ac:     	str	q0, [sp, #0x1a0]
1010e92b0:     	ldur	x8, [x29, #-0xe8]
1010e92b4:     	str	q0, [sp, #0x180]
1010e92b8:     	stur	q0, [x29, #-0xe0]
1010e92bc:     	stur	x8, [x29, #-0xd0]
1010e92c0:     	ldur	q0, [x29, #-0xe0]
1010e92c4:     	str	x8, [sp, #0x178]
1010e92c8:     	add	x8, sp, #0x69
1010e92cc:     	stur	q0, [x8, #0xff]
1010e92d0:     	ldp	q0, q1, [sp, #0x150]
1010e92d4:     	ldr	q2, [sp, #0x170]
1010e92d8:     	stp	q1, q2, [sp, #0x130]
1010e92dc:     	str	q0, [sp, #0x120]
1010e92e0:     	ldp	q0, q1, [sp, #0x120]
1010e92e4:     	ldr	q2, [sp, #0x140]
1010e92e8:     	stp	q1, q2, [sp, #0x100]
1010e92ec:     	str	q0, [sp, #0xf0]
1010e92f0:     	add	x1, sp, #0xf0
1010e92f4:     	mov	x0, x27
1010e92f8:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e92fc:     	mov	x26, x0
1010e9300:     	add	x3, sp, #0xf0
1010e9304:     	mov	x0, x27
1010e9308:     	mov	x1, x22
1010e930c:     	mov	x25, x24
1010e9310:     	mov	x2, x24
1010e9314:     	ldr	x24, [sp, #0x68]
1010e9318:     	mov	x4, x24
1010e931c:     	bl	0x1010e8ff4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_>
1010e9320:     	fmov	d0, x26
1010e9324:     	cnt.8b	v0, v0
1010e9328:     	addv.8b	b0, v0
1010e932c:     	fmov	w8, s0
1010e9330:     	sub	w8, w8, w20
1010e9334:     	mvn	w8, w8
1010e9338:     	lsl	x26, x0, x8
1010e933c:     	ldr	q0, [x19]
1010e9340:     	stur	q0, [x29, #-0xc0]
1010e9344:     	ldr	x8, [x19, #0x10]
1010e9348:     	stur	x8, [x29, #-0xb0]
1010e934c:     	sub	x0, x29, #0xf8
1010e9350:     	sub	x1, x29, #0xc0
1010e9354:     	mov	x2, x27
1010e9358:     	mov	x3, x23
1010e935c:     	mov	w4, #0x1                ; =1
1010e9360:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e9364:     	ldr	q0, [x21]
1010e9368:     	str	q0, [sp, #0x1a0]
1010e936c:     	ldur	x8, [x29, #-0xe8]
1010e9370:     	str	q0, [sp, #0x180]
1010e9374:     	stur	q0, [x29, #-0xe0]
1010e9378:     	stur	x8, [x29, #-0xd0]
1010e937c:     	ldur	q0, [x29, #-0xe0]
1010e9380:     	str	x8, [sp, #0x160]
1010e9384:     	str	q0, [sp, #0x150]
1010e9388:     	ldur	q0, [x19, #0x18]
1010e938c:     	stur	q0, [x29, #-0xc0]
1010e9390:     	ldur	x8, [x19, #0x28]
1010e9394:     	stur	x8, [x29, #-0xb0]
1010e9398:     	sub	x0, x29, #0xf8
1010e939c:     	sub	x1, x29, #0xc0
1010e93a0:     	mov	x2, x27
1010e93a4:     	mov	x3, x23
1010e93a8:     	mov	w4, #0x1                ; =1
1010e93ac:     	bl	0x100ab6980 <__RINvMs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancyNtB5_10Restricted8cofactorKm1_EB9_>
1010e93b0:     	ldr	q0, [x21]
1010e93b4:     	str	q0, [sp, #0x1a0]
1010e93b8:     	ldur	x8, [x29, #-0xe8]
1010e93bc:     	str	q0, [sp, #0x180]
1010e93c0:     	stur	q0, [x29, #-0xe0]
1010e93c4:     	stur	x8, [x29, #-0xd0]
1010e93c8:     	ldur	q0, [x29, #-0xe0]
1010e93cc:     	str	x8, [sp, #0x178]
1010e93d0:     	add	x8, sp, #0x69
1010e93d4:     	stur	q0, [x8, #0xff]
1010e93d8:     	ldp	q0, q1, [sp, #0x150]
1010e93dc:     	ldr	q2, [sp, #0x170]
1010e93e0:     	stp	q1, q2, [sp, #0x130]
1010e93e4:     	str	q0, [sp, #0x120]
1010e93e8:     	ldp	q0, q1, [sp, #0x120]
1010e93ec:     	ldr	q2, [sp, #0x140]
1010e93f0:     	stp	q1, q2, [sp, #0x100]
1010e93f4:     	str	q0, [sp, #0xf0]
1010e93f8:     	add	x1, sp, #0xf0
1010e93fc:     	mov	x0, x27
1010e9400:     	bl	0x100b5b8f0 <__RINvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction9variablesKm3_EB6_>
1010e9404:     	mov	x23, x0
1010e9408:     	add	x3, sp, #0xf0
1010e940c:     	mov	x0, x27
1010e9410:     	mov	x1, x22
1010e9414:     	mov	x2, x25
1010e9418:     	mov	x4, x24
1010e941c:     	bl	0x1010e8ff4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_>
1010e9420:     	fmov	d0, x23
1010e9424:     	cnt.8b	v0, v0
1010e9428:     	addv.8b	b0, v0
1010e942c:     	fmov	w8, s0
1010e9430:     	sub	w8, w8, w20
1010e9434:     	mvn	w8, w8
1010e9438:     	lsl	x8, x0, x8
1010e943c:     	add	x27, x8, x26
1010e9440:     	b	0x1010e97d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7e0>
1010e9444:     	ldp	x20, x25, [sp, #0x150]
1010e9448:     	cbz	x28, 0x1010e979c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7a8>
1010e944c:     	str	x20, [sp, #0x8]
1010e9450:     	str	x26, [sp, #0xb0]
1010e9454:     	mov	x27, #0x0               ; =0
1010e9458:     	mov	x24, #0x0               ; =0
1010e945c:     	str	xzr, [sp, #0xd8]
1010e9460:     	adrp	x8, 0x101973000 <GCC_except_table10364+0x18>
1010e9464:     	ldr	q0, [x8, #0x690]
1010e9468:     	str	q0, [sp, #0xc0]
1010e946c:     	mov	w22, #0x3f              ; =63
1010e9470:     	dup.2d	v1, x22
1010e9474:     	and	x9, x28, #0x1ffffffffffffffe
1010e9478:     	mov	w8, #0x2                ; =2
1010e947c:     	dup.2d	v0, x8
1010e9480:     	stp	q0, q1, [sp, #0x90]
1010e9484:     	neg	x8, x9
1010e9488:     	str	x8, [sp, #0x88]
1010e948c:     	mov	w8, #0x4                ; =4
1010e9490:     	dup.2d	v1, x8
1010e9494:     	mov	w8, #0x8                ; =8
1010e9498:     	dup.2d	v0, x8
1010e949c:     	stp	q0, q1, [sp, #0x40]
1010e94a0:     	mov	w8, #0xc                ; =12
1010e94a4:     	dup.2d	v1, x8
1010e94a8:     	mov	w8, #0x10               ; =16
1010e94ac:     	dup.2d	v0, x8
1010e94b0:     	stp	q0, q1, [sp, #0x20]
1010e94b4:     	adrp	x8, 0x101973000 <GCC_except_table10364+0x18>
1010e94b8:     	ldr	q0, [x8, #0x680]
1010e94bc:     	str	q0, [sp, #0x10]
1010e94c0:     	mov	w20, #0x1               ; =1
1010e94c4:     	dup.2d	v0, x20
1010e94c8:     	str	q0, [sp, #0x70]
1010e94cc:     	movi.2s	v8, #0x3f
1010e94d0:     	b	0x1010e94e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x4f0>
1010e94d4:     	add	x24, x24, #0x1
1010e94d8:     	and	x8, x28, #0x3f
1010e94dc:     	lsr	x8, x24, x8
1010e94e0:     	cbnz	x8, 0x1010e978c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x798>
1010e94e4:     	cbz	x21, 0x1010e9500 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x50c>
1010e94e8:     	dup.2d	v0, x24
1010e94ec:     	cmp	x28, #0x10
1010e94f0:     	b.hs	0x1010e950c <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x518>
1010e94f4:     	mov	x9, #0x0                ; =0
1010e94f8:     	mov	x26, #0x0               ; =0
1010e94fc:     	b	0x1010e96c0 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x6cc>
1010e9500:     	mov	x8, #0x0                ; =0
1010e9504:     	mov	x26, #0x0               ; =0
1010e9508:     	b	0x1010e9730 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x73c>
1010e950c:     	movi.2d	v1, #0000000000000000
1010e9510:     	add	x8, x25, #0x20
1010e9514:     	movi.2d	v2, #0000000000000000
1010e9518:     	and	x9, x28, #0x1ffffffffffffff0
1010e951c:     	ldr	q4, [sp, #0xc0]
1010e9520:     	ldp	q6, q15, [sp, #0x10]
1010e9524:     	movi.2d	v3, #0000000000000000
1010e9528:     	movi.2d	v7, #0000000000000000
1010e952c:     	movi.2d	v16, #0000000000000000
1010e9530:     	movi.2d	v5, #0000000000000000
1010e9534:     	movi.2d	v18, #0000000000000000
1010e9538:     	movi.2d	v17, #0000000000000000
1010e953c:     	ldp	q13, q12, [sp, #0x40]
1010e9540:     	ldr	q14, [sp, #0x30]
1010e9544:     	movi.4s	v8, #0x3f
1010e9548:     	add.2d	v19, v4, v12
1010e954c:     	add.2d	v20, v6, v12
1010e9550:     	add.2d	v21, v4, v13
1010e9554:     	add.2d	v22, v6, v13
1010e9558:     	add.2d	v23, v4, v14
1010e955c:     	add.2d	v24, v6, v14
1010e9560:     	ldp	q25, q26, [x8, #-0x20]
1010e9564:     	dup.2d	v27, x22
1010e9568:     	ldp	q28, q29, [x8], #0x40
1010e956c:     	and.16b	v30, v6, v27
1010e9570:     	and.16b	v31, v4, v27
1010e9574:     	and.16b	v20, v20, v27
1010e9578:     	and.16b	v19, v19, v27
1010e957c:     	and.16b	v22, v22, v27
1010e9580:     	and.16b	v21, v21, v27
1010e9584:     	and.16b	v24, v24, v27
1010e9588:     	and.16b	v23, v23, v27
1010e958c:     	neg.2d	v27, v31
1010e9590:     	ushl.2d	v27, v0, v27
1010e9594:     	neg.2d	v30, v30
1010e9598:     	ushl.2d	v30, v0, v30
1010e959c:     	neg.2d	v19, v19
1010e95a0:     	ushl.2d	v19, v0, v19
1010e95a4:     	neg.2d	v20, v20
1010e95a8:     	ushl.2d	v20, v0, v20
1010e95ac:     	neg.2d	v21, v21
1010e95b0:     	ushl.2d	v21, v0, v21
1010e95b4:     	neg.2d	v22, v22
1010e95b8:     	ushl.2d	v22, v0, v22
1010e95bc:     	neg.2d	v23, v23
1010e95c0:     	ushl.2d	v23, v0, v23
1010e95c4:     	neg.2d	v24, v24
1010e95c8:     	ushl.2d	v24, v0, v24
1010e95cc:     	dup.2d	v31, x20
1010e95d0:     	and.16b	v30, v30, v31
1010e95d4:     	and.16b	v27, v27, v31
1010e95d8:     	and.16b	v20, v20, v31
1010e95dc:     	and.16b	v19, v19, v31
1010e95e0:     	and.16b	v22, v22, v31
1010e95e4:     	and.16b	v21, v21, v31
1010e95e8:     	and.16b	v24, v24, v31
1010e95ec:     	and.16b	v23, v23, v31
1010e95f0:     	and.16b	v25, v25, v8
1010e95f4:     	and.16b	v26, v26, v8
1010e95f8:     	and.16b	v28, v28, v8
1010e95fc:     	and.16b	v29, v29, v8
1010e9600:     	ushll2.2d	v31, v25, #0x0
1010e9604:     	ushll.2d	v25, v25, #0x0
1010e9608:     	ushll2.2d	v9, v26, #0x0
1010e960c:     	ushll.2d	v26, v26, #0x0
1010e9610:     	ushll2.2d	v10, v28, #0x0
1010e9614:     	ushll.2d	v28, v28, #0x0
1010e9618:     	ushll2.2d	v11, v29, #0x0
1010e961c:     	ushll.2d	v29, v29, #0x0
1010e9620:     	ushl.2d	v25, v27, v25
1010e9624:     	ushl.2d	v27, v30, v31
1010e9628:     	ushl.2d	v19, v19, v26
1010e962c:     	ushl.2d	v20, v20, v9
1010e9630:     	ushl.2d	v21, v21, v28
1010e9634:     	ushl.2d	v22, v22, v10
1010e9638:     	ushl.2d	v23, v23, v29
1010e963c:     	ushl.2d	v24, v24, v11
1010e9640:     	orr.16b	v3, v27, v3
1010e9644:     	orr.16b	v2, v25, v2
1010e9648:     	orr.16b	v16, v20, v16
1010e964c:     	orr.16b	v7, v19, v7
1010e9650:     	orr.16b	v18, v22, v18
1010e9654:     	orr.16b	v5, v21, v5
1010e9658:     	orr.16b	v1, v24, v1
1010e965c:     	orr.16b	v17, v23, v17
1010e9660:     	add.2d	v6, v6, v15
1010e9664:     	add.2d	v4, v4, v15
1010e9668:     	subs	x9, x9, #0x10
1010e966c:     	b.ne	0x1010e9548 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x554>
1010e9670:     	orr.16b	v2, v7, v2
1010e9674:     	orr.16b	v3, v16, v3
1010e9678:     	orr.16b	v3, v18, v3
1010e967c:     	orr.16b	v2, v5, v2
1010e9680:     	orr.16b	v2, v17, v2
1010e9684:     	orr.16b	v1, v1, v3
1010e9688:     	orr.16b	v1, v2, v1
1010e968c:     	mov	d2, v1[1]
1010e9690:     	orr.8b	v1, v1, v2
1010e9694:     	fmov	x26, d1
1010e9698:     	and	x8, x28, #0x1ffffffffffffff0
1010e969c:     	cmp	x28, x8
1010e96a0:     	b.ne	0x1010e96ac <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x6b8>
1010e96a4:     	movi.2s	v8, #0x3f
1010e96a8:     	b	0x1010e9750 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x75c>
1010e96ac:     	and	x9, x28, #0x1ffffffffffffff0
1010e96b0:     	and	x8, x28, #0x1ffffffffffffff0
1010e96b4:     	and	x10, x28, #0xe
1010e96b8:     	movi.2s	v8, #0x3f
1010e96bc:     	cbz	x10, 0x1010e9730 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x73c>
1010e96c0:     	fmov	d1, x26
1010e96c4:     	dup.2d	v2, x9
1010e96c8:     	ldr	q3, [sp, #0xc0]
1010e96cc:     	orr.16b	v2, v2, v3
1010e96d0:     	ldr	x8, [sp, #0x88]
1010e96d4:     	add	x8, x8, x9
1010e96d8:     	add	x9, x25, x9, lsl #2
1010e96dc:     	ldp	q6, q5, [sp, #0x90]
1010e96e0:     	ldr	q7, [sp, #0x70]
1010e96e4:     	ldr	d3, [x9], #0x8
1010e96e8:     	and.16b	v4, v2, v5
1010e96ec:     	neg.2d	v4, v4
1010e96f0:     	ushl.2d	v4, v0, v4
1010e96f4:     	and.16b	v4, v4, v7
1010e96f8:     	and.8b	v3, v3, v8
1010e96fc:     	ushll.2d	v3, v3, #0x0
1010e9700:     	ushl.2d	v3, v4, v3
1010e9704:     	orr.16b	v1, v3, v1
1010e9708:     	add.2d	v2, v2, v6
1010e970c:     	adds	x8, x8, #0x2
1010e9710:     	b.ne	0x1010e96e4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x6f0>
1010e9714:     	mov	d0, v1[1]
1010e9718:     	orr.8b	v0, v1, v0
1010e971c:     	fmov	x26, d0
1010e9720:     	and	x8, x28, #0x1ffffffffffffffe
1010e9724:     	and	x9, x28, #0x1ffffffffffffffe
1010e9728:     	cmp	x28, x9
1010e972c:     	b.eq	0x1010e9750 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x75c>
1010e9730:     	ldr	w9, [x25, x8, lsl #2]
1010e9734:     	lsr	x10, x24, x8
1010e9738:     	and	x10, x10, #0x1
1010e973c:     	lsl	x9, x10, x9
1010e9740:     	orr	x26, x9, x26
1010e9744:     	add	x8, x8, #0x1
1010e9748:     	cmp	x28, x8
1010e974c:     	b.ne	0x1010e9730 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x73c>
1010e9750:     	ldp	x8, x0, [sp, #0xe0]
1010e9754:     	orr	x2, x26, x8
1010e9758:     	mov	x1, x23
1010e975c:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e9760:     	cbz	w0, 0x1010e94d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x4e0>
1010e9764:     	ldp	x1, x8, [sp, #0xb0]
1010e9768:     	orr	x2, x26, x8
1010e976c:     	ldr	x0, [sp, #0xe8]
1010e9770:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e9774:     	cmp	w0, #0x0
1010e9778:     	ldr	x8, [sp, #0xd8]
1010e977c:     	csinc	x27, x27, x8, eq
1010e9780:     	cinc	x8, x8, ne
1010e9784:     	str	x8, [sp, #0xd8]
1010e9788:     	b	0x1010e94d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x4e0>
1010e978c:     	ldr	x20, [sp, #0x8]
1010e9790:     	b	0x1010e97c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7d4>
1010e9794:     	mov	x20, #0x0               ; =0
1010e9798:     	mov	w25, #0x4               ; =4
1010e979c:     	ldp	x2, x0, [sp, #0xe0]
1010e97a0:     	mov	x1, x23
1010e97a4:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e97a8:     	cbz	w0, 0x1010e97c4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7d0>
1010e97ac:     	ldr	x0, [sp, #0xe8]
1010e97b0:     	mov	x1, x26
1010e97b4:     	ldr	x2, [sp, #0xb8]
1010e97b8:     	bl	0x1011651b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010e97bc:     	mov	w27, w0
1010e97c0:     	b	0x1010e97c8 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7d4>
1010e97c4:     	mov	x27, #0x0               ; =0
1010e97c8:     	cbz	x20, 0x1010e97d4 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x7e0>
1010e97cc:     	mov	x0, x25
1010e97d0:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e97d4:     	ldr	x0, [sp, #0x68]
1010e97d8:     	mov	x1, x19
1010e97dc:     	mov	x2, x27
1010e97e0:     	bl	0x10121d47c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapANtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9occupancy10Restrictedj2_yNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertBU_>
1010e97e4:     	mov	x0, x27
1010e97e8:     	add	sp, sp, #0x220
1010e97ec:     	ldp	x29, x30, [sp, #0x90]
1010e97f0:     	ldp	x20, x19, [sp, #0x80]
1010e97f4:     	ldp	x22, x21, [sp, #0x70]
1010e97f8:     	ldp	x24, x23, [sp, #0x60]
1010e97fc:     	ldp	x26, x25, [sp, #0x50]
1010e9800:     	ldp	x28, x27, [sp, #0x40]
1010e9804:     	ldp	d9, d8, [sp, #0x30]
1010e9808:     	ldp	d11, d10, [sp, #0x20]
1010e980c:     	ldp	d13, d12, [sp, #0x10]
1010e9810:     	ldp	d15, d14, [sp], #0xa0
1010e9814:     	ret
1010e9818:     	adrp	x0, 0x101b39000 <dyld_stub_binder+0x101b39000>
1010e981c:     	add	x0, x0, #0xf80
1010e9820:     	bl	0x1018c1834 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
1010e9824:     	mov	x19, x0
1010e9828:     	cbnz	x20, 0x1010e9850 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x85c>
1010e982c:     	b	0x1010e9858 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x864>
1010e9830:     	mov	x19, x0
1010e9834:     	ldr	x8, [sp, #0x150]
1010e9838:     	cbz	x8, 0x1010e9858 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x864>
1010e983c:     	ldr	x25, [sp, #0x158]
1010e9840:     	b	0x1010e9850 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x85c>
1010e9844:     	mov	x19, x0
1010e9848:     	ldr	x8, [sp, #0x8]
1010e984c:     	cbz	x8, 0x1010e9858 <__RINvNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab11contraction5count5visitKm9_Kb0_EB8_+0x864>
1010e9850:     	mov	x0, x25
1010e9854:     	bl	0x1018c9bb8 <dyld_stub_binder+0x1018c9bb8>
1010e9858:     	mov	x0, x19
1010e985c:     	bl	0x1018c9a08 <dyld_stub_binder+0x1018c9a08>
