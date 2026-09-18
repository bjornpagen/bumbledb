
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d16098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>:
100d16098:     	stp	d15, d14, [sp, #-0xa0]!
100d1609c:     	stp	d13, d12, [sp, #0x10]
100d160a0:     	stp	d11, d10, [sp, #0x20]
100d160a4:     	stp	d9, d8, [sp, #0x30]
100d160a8:     	stp	x28, x27, [sp, #0x40]
100d160ac:     	stp	x26, x25, [sp, #0x50]
100d160b0:     	stp	x24, x23, [sp, #0x60]
100d160b4:     	stp	x22, x21, [sp, #0x70]
100d160b8:     	stp	x20, x19, [sp, #0x80]
100d160bc:     	stp	x29, x30, [sp, #0x90]
100d160c0:     	add	x29, sp, #0x90
100d160c4:     	sub	sp, sp, #0x280
100d160c8:     	cmp	w4, w3
100d160cc:     	b.hs	0x100d16ad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa40>
100d160d0:     	mov	x24, x5
100d160d4:     	mov	x23, x4
100d160d8:     	mov	x25, x2
100d160dc:     	mov	x22, x1
100d160e0:     	mov	x21, x0
100d160e4:     	sub	w8, w3, #0x1
100d160e8:     	and	w28, w8, #0x3f
100d160ec:     	mov	w9, #0x1                ; =1
100d160f0:     	lsl	x27, x9, x8
100d160f4:     	lsr	x8, x27, #6
100d160f8:     	cmp	w28, #0x6
100d160fc:     	cinc	x19, x8, lo
100d16100:     	cbz	x19, 0x100d161d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x140>
100d16104:     	lsl	x26, x19, #3
100d16108:     	mov	x0, x26
100d1610c:     	mov	w1, #0x1                ; =1
100d16110:     	bl	0x101284ba4 <dyld_stub_binder+0x101284ba4>
100d16114:     	cbz	x0, 0x100d16b30 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa98>
100d16118:     	mov	x20, x0
100d1611c:     	cmp	w23, #0x5
100d16120:     	str	x27, [sp, #0x208]
100d16124:     	b.ls	0x100d161e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x150>
100d16128:     	add	w8, w23, #0x3a
100d1612c:     	and	w26, w8, #0x3f
100d16130:     	cmp	w26, #0x3f
100d16134:     	b.eq	0x100d16b00 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa68>
100d16138:     	mov	w9, #0x1                ; =1
100d1613c:     	lsl	x23, x9, x8
100d16140:     	mov	w9, #0x2                ; =2
100d16144:     	lsl	x2, x9, x8
100d16148:     	neg	x8, x2
100d1614c:     	and	x8, x25, x8
100d16150:     	lsr	x9, x19, x26
100d16154:     	sub	x10, x23, #0x1
100d16158:     	tst	x19, x10
100d1615c:     	cinc	x9, x9, ne
100d16160:     	cmp	x19, #0x0
100d16164:     	csel	x9, xzr, x9, eq
100d16168:     	add	x10, x26, #0x1
100d1616c:     	lsr	x8, x8, x10
100d16170:     	cmp	x8, x9
100d16174:     	csel	x25, x8, x9, lo
100d16178:     	cbz	x25, 0x100d16a6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1617c:     	mov	w8, w24
100d16180:     	lsl	x0, x8, x26
100d16184:     	adds	x1, x0, x23
100d16188:     	b.hs	0x100d16af0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d1618c:     	cmp	x1, x2
100d16190:     	b.hi	0x100d16af0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d16194:     	mov	x24, #0x0               ; =0
100d16198:     	lsl	x27, x2, #3
100d1619c:     	add	x22, x22, x0, lsl #3
100d161a0:     	lsl	x8, x24, x26
100d161a4:     	sub	x9, x19, x8
100d161a8:     	cmp	x23, x9
100d161ac:     	csel	x0, x23, x9, lo
100d161b0:     	b.hi	0x100d16ac4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa2c>
100d161b4:     	add	x24, x24, #0x1
100d161b8:     	lsl	x2, x0, #3
100d161bc:     	add	x0, x20, x8, lsl #3
100d161c0:     	mov	x1, x22
100d161c4:     	bl	0x101284d9c <dyld_stub_binder+0x101284d9c>
100d161c8:     	add	x22, x22, x27
100d161cc:     	cmp	x25, x24
100d161d0:     	b.ne	0x100d161a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x108>
100d161d4:     	b	0x100d16a6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d161d8:     	mov	w20, #0x8               ; =8
100d161dc:     	cmp	w23, #0x5
100d161e0:     	str	x27, [sp, #0x208]
100d161e4:     	b.hi	0x100d16128 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x90>
100d161e8:     	cbz	x25, 0x100d16a6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d161ec:     	mov	x9, #0x0                ; =0
100d161f0:     	mov	w8, w23
100d161f4:     	dup.2d	v7, x8
100d161f8:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d161fc:     	ldr	q0, [x10, #0x730]
100d16200:     	ushl.2d	v0, v0, v7
100d16204:     	stur	q0, [x29, #-0xb0]
100d16208:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1620c:     	ldr	q0, [x10, #0x880]
100d16210:     	ushl.2d	v0, v0, v7
100d16214:     	stur	q0, [x29, #-0xc0]
100d16218:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1621c:     	ldr	q0, [x10, #0x890]
100d16220:     	ushl.2d	v0, v0, v7
100d16224:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16228:     	ldr	q1, [x10, #0x8a0]
100d1622c:     	ushl.2d	v1, v1, v7
100d16230:     	mov	w10, #0x3e              ; =62
100d16234:     	dup.2d	v2, x10
100d16238:     	and.16b	v3, v0, v2
100d1623c:     	and.16b	v0, v1, v2
100d16240:     	stp	q0, q3, [x29, #-0xe0]
100d16244:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16248:     	ldr	q0, [x10, #0x750]
100d1624c:     	ushl.2d	v0, v0, v7
100d16250:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16254:     	ldr	q1, [x10, #0x8b0]
100d16258:     	ushl.2d	v1, v1, v7
100d1625c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16260:     	ldr	q3, [x10, #0x8c0]
100d16264:     	ushl.2d	v3, v3, v7
100d16268:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1626c:     	ldr	q4, [x10, #0x8d0]
100d16270:     	ushl.2d	v4, v4, v7
100d16274:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16278:     	ldr	q16, [x10, #0x8e0]
100d1627c:     	ushl.2d	v16, v16, v7
100d16280:     	and.16b	v5, v1, v2
100d16284:     	and.16b	v1, v3, v2
100d16288:     	stp	q1, q5, [x29, #-0x100]
100d1628c:     	and.16b	v3, v4, v2
100d16290:     	and.16b	v1, v16, v2
100d16294:     	stp	q1, q3, [sp, #0x190]
100d16298:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1629c:     	ldr	q3, [x10, #0x8f0]
100d162a0:     	ushl.2d	v3, v3, v7
100d162a4:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162a8:     	ldr	q4, [x10, #0x900]
100d162ac:     	ushl.2d	v4, v4, v7
100d162b0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162b4:     	ldr	q16, [x10, #0x910]
100d162b8:     	ushl.2d	v16, v16, v7
100d162bc:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162c0:     	ldr	q17, [x10, #0x920]
100d162c4:     	ushl.2d	v17, v17, v7
100d162c8:     	and.16b	v5, v3, v2
100d162cc:     	and.16b	v1, v4, v2
100d162d0:     	stp	q1, q5, [sp, #0x170]
100d162d4:     	and.16b	v3, v16, v2
100d162d8:     	and.16b	v1, v17, v2
100d162dc:     	stp	q1, q3, [sp, #0x150]
100d162e0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162e4:     	ldr	q3, [x10, #0x930]
100d162e8:     	ushl.2d	v3, v3, v7
100d162ec:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162f0:     	ldr	q4, [x10, #0x940]
100d162f4:     	ushl.2d	v4, v4, v7
100d162f8:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d162fc:     	ldr	q16, [x10, #0x950]
100d16300:     	ushl.2d	v16, v16, v7
100d16304:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16308:     	ldr	q17, [x10, #0x960]
100d1630c:     	ushl.2d	v17, v17, v7
100d16310:     	and.16b	v5, v3, v2
100d16314:     	and.16b	v1, v4, v2
100d16318:     	stp	q5, q1, [sp, #0x110]
100d1631c:     	and.16b	v3, v16, v2
100d16320:     	and.16b	v1, v17, v2
100d16324:     	stp	q3, q1, [sp, #0x130]
100d16328:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1632c:     	ldr	q3, [x10, #0x970]
100d16330:     	ushl.2d	v3, v3, v7
100d16334:     	and.16b	v1, v3, v2
100d16338:     	str	q1, [sp, #0x100]
100d1633c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16340:     	ldr	q3, [x10, #0x990]
100d16344:     	ushl.2d	v3, v3, v7
100d16348:     	and.16b	v1, v3, v2
100d1634c:     	str	q1, [sp, #0xf0]
100d16350:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16354:     	ldr	q3, [x10, #0x9a0]
100d16358:     	ushl.2d	v3, v3, v7
100d1635c:     	and.16b	v1, v3, v2
100d16360:     	str	q1, [sp, #0xe0]
100d16364:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16368:     	ldr	q3, [x10, #0x9c0]
100d1636c:     	ushl.2d	v3, v3, v7
100d16370:     	and.16b	v1, v3, v2
100d16374:     	str	q1, [sp, #0xd0]
100d16378:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1637c:     	ldr	q3, [x10, #0x9d0]
100d16380:     	ushl.2d	v3, v3, v7
100d16384:     	and.16b	v1, v3, v2
100d16388:     	str	q1, [sp, #0xc0]
100d1638c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16390:     	ldr	q3, [x10, #0x9f0]
100d16394:     	ushl.2d	v3, v3, v7
100d16398:     	and.16b	v1, v3, v2
100d1639c:     	str	q1, [sp, #0xb0]
100d163a0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d163a4:     	ldr	q3, [x10, #0xa00]
100d163a8:     	ushl.2d	v3, v3, v7
100d163ac:     	and.16b	v1, v3, v2
100d163b0:     	str	q1, [sp, #0xa0]
100d163b4:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d163b8:     	ldr	q3, [x10, #0xa20]
100d163bc:     	ushl.2d	v3, v3, v7
100d163c0:     	mov	w10, #0x1e              ; =30
100d163c4:     	dup.2d	v4, x10
100d163c8:     	and.16b	v1, v3, v4
100d163cc:     	str	q1, [sp, #0x90]
100d163d0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d163d4:     	ldr	q3, [x10, #0x670]
100d163d8:     	ushl.2d	v3, v3, v7
100d163dc:     	mov	w10, #0x2f              ; =47
100d163e0:     	dup.2d	v4, x10
100d163e4:     	and.16b	v1, v3, v4
100d163e8:     	str	q1, [sp, #0x70]
100d163ec:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d163f0:     	ldr	q3, [x10, #0xa30]
100d163f4:     	ushl.2d	v3, v3, v7
100d163f8:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d163fc:     	ldr	q4, [x10, #0xa40]
100d16400:     	ushl.2d	v4, v4, v7
100d16404:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16408:     	ldr	q16, [x10, #0xa50]
100d1640c:     	ushl.2d	v16, v16, v7
100d16410:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16414:     	ldr	q17, [x10, #0xa60]
100d16418:     	ushl.2d	v17, v17, v7
100d1641c:     	and.16b	v1, v3, v2
100d16420:     	str	q1, [sp, #0x80]
100d16424:     	and.16b	v26, v4, v2
100d16428:     	and.16b	v27, v16, v2
100d1642c:     	and.16b	v28, v17, v2
100d16430:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16434:     	ldr	q2, [x10, #0x770]
100d16438:     	ushl.2d	v2, v2, v7
100d1643c:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16440:     	ldr	q3, [x10, #0x950]
100d16444:     	ushl.2d	v3, v3, v7
100d16448:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d1644c:     	ldr	q4, [x10, #0x940]
100d16450:     	ushl.2d	v4, v4, v7
100d16454:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16458:     	ldr	q16, [x10, #0x930]
100d1645c:     	ushl.2d	v23, v16, v7
100d16460:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16464:     	ldr	q17, [x10, #0x920]
100d16468:     	ushl.2d	v16, v17, v7
100d1646c:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d16470:     	ldr	q18, [x10, #0x910]
100d16474:     	ushl.2d	v17, v18, v7
100d16478:     	adrp	x10, 0x101316000 <GCC_except_table9261>
100d1647c:     	ldr	q19, [x10, #0x900]
100d16480:     	ushl.2d	v18, v19, v7
100d16484:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16488:     	ldr	q20, [x10, #0x600]
100d1648c:     	ushl.2d	v20, v20, v7
100d16490:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16494:     	ldr	q21, [x10, #0x610]
100d16498:     	ushl.2d	v21, v21, v7
100d1649c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164a0:     	ldr	q22, [x10, #0x500]
100d164a4:     	ushl.2d	v22, v22, v7
100d164a8:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164ac:     	ldr	q5, [x10, #0x620]
100d164b0:     	ushl.2d	v5, v5, v7
100d164b4:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164b8:     	ldr	q6, [x10, #0x630]
100d164bc:     	ushl.2d	v6, v6, v7
100d164c0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164c4:     	ldr	q24, [x10, #0x640]
100d164c8:     	ushl.2d	v24, v24, v7
100d164cc:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164d0:     	ldr	q25, [x10, #0x650]
100d164d4:     	ushl.2d	v25, v25, v7
100d164d8:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164dc:     	ldr	q1, [x10, #0x660]
100d164e0:     	ushl.2d	v1, v1, v7
100d164e4:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164e8:     	ldr	q29, [x10, #0x980]
100d164ec:     	ushl.2d	v29, v29, v7
100d164f0:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d164f4:     	ldr	q30, [x10, #0x6a0]
100d164f8:     	ushl.2d	v30, v30, v7
100d164fc:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16500:     	ldr	q31, [x10, #0x9b0]
100d16504:     	ushl.2d	v31, v31, v7
100d16508:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1650c:     	ldr	q8, [x10, #0x690]
100d16510:     	ushl.2d	v8, v8, v7
100d16514:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16518:     	ldr	q9, [x10, #0x9e0]
100d1651c:     	ushl.2d	v9, v9, v7
100d16520:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16524:     	ldr	q10, [x10, #0x680]
100d16528:     	ushl.2d	v10, v10, v7
100d1652c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16530:     	ldr	q11, [x10, #0xa10]
100d16534:     	ushl.2d	v11, v11, v7
100d16538:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d1653c:     	ldr	q15, [x10, #0xa70]
100d16540:     	ushl.2d	v15, v15, v7
100d16544:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16548:     	ldr	q14, [x10, #0xa80]
100d1654c:     	ushl.2d	v14, v14, v7
100d16550:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16554:     	ldr	q13, [x10, #0xa90]
100d16558:     	ushl.2d	v13, v13, v7
100d1655c:     	adrp	x10, 0x101317000 <dyld_stub_binder+0x101317000>
100d16560:     	ldr	q12, [x10, #0xaa0]
100d16564:     	ushl.2d	v12, v12, v7
100d16568:     	mov	w10, #0x3f              ; =63
100d1656c:     	dup.2d	v7, x10
100d16570:     	and.16b	v19, v23, v7
100d16574:     	and.16b	v16, v16, v7
100d16578:     	and.16b	v17, v17, v7
100d1657c:     	and.16b	v18, v18, v7
100d16580:     	and.16b	v20, v20, v7
100d16584:     	and.16b	v23, v21, v7
100d16588:     	mov.16b	v21, v20
100d1658c:     	and.16b	v20, v22, v7
100d16590:     	mov.16b	v22, v23
100d16594:     	and.16b	v5, v5, v7
100d16598:     	and.16b	v6, v6, v7
100d1659c:     	str	q6, [sp, #0x1f0]
100d165a0:     	and.16b	v6, v24, v7
100d165a4:     	str	q6, [sp, #0x1e0]
100d165a8:     	and.16b	v6, v25, v7
100d165ac:     	and.16b	v1, v1, v7
100d165b0:     	stp	q1, q6, [sp, #0x1c0]
100d165b4:     	and.16b	v6, v29, v7
100d165b8:     	and.16b	v1, v30, v7
100d165bc:     	stp	q1, q6, [sp, #0x50]
100d165c0:     	and.16b	v6, v31, v7
100d165c4:     	and.16b	v1, v8, v7
100d165c8:     	stp	q1, q6, [sp, #0x30]
100d165cc:     	mov.16b	v8, v20
100d165d0:     	and.16b	v6, v9, v7
100d165d4:     	mov.16b	v9, v5
100d165d8:     	and.16b	v10, v10, v7
100d165dc:     	and.16b	v29, v11, v7
100d165e0:     	and.16b	v1, v15, v7
100d165e4:     	stp	q1, q6, [sp, #0x10]
100d165e8:     	and.16b	v1, v14, v7
100d165ec:     	str	q1, [sp]
100d165f0:     	and.16b	v30, v13, v7
100d165f4:     	and.16b	v31, v12, v7
100d165f8:     	ldp	q1, q6, [x29, #-0xc0]
100d165fc:     	neg.2d	v5, v6
100d16600:     	neg.2d	v6, v1
100d16604:     	ldp	q1, q7, [x29, #-0xe0]
100d16608:     	neg.2d	v24, v7
100d1660c:     	neg.2d	v25, v1
100d16610:     	ldp	q1, q7, [x29, #-0x100]
100d16614:     	neg.2d	v11, v7
100d16618:     	neg.2d	v1, v1
100d1661c:     	ldr	q7, [sp, #0x1a0]
100d16620:     	neg.2d	v7, v7
100d16624:     	stur	q7, [x29, #-0xb0]
100d16628:     	ldr	q7, [sp, #0x190]
100d1662c:     	neg.2d	v7, v7
100d16630:     	stur	q7, [x29, #-0xc0]
100d16634:     	ldr	q7, [sp, #0x180]
100d16638:     	neg.2d	v7, v7
100d1663c:     	stur	q7, [x29, #-0xd0]
100d16640:     	ldr	q7, [sp, #0x170]
100d16644:     	neg.2d	v7, v7
100d16648:     	stur	q7, [x29, #-0xe0]
100d1664c:     	ldr	q7, [sp, #0x160]
100d16650:     	neg.2d	v7, v7
100d16654:     	stur	q7, [x29, #-0xf0]
100d16658:     	ldr	q7, [sp, #0x150]
100d1665c:     	neg.2d	v7, v7
100d16660:     	stur	q7, [x29, #-0x100]
100d16664:     	ldr	q7, [sp, #0x110]
100d16668:     	neg.2d	v7, v7
100d1666c:     	str	q7, [sp, #0x1a0]
100d16670:     	mov	w10, #0x1               ; =1
100d16674:     	ldr	q7, [sp, #0x120]
100d16678:     	neg.2d	v7, v7
100d1667c:     	str	q7, [sp, #0x190]
100d16680:     	lsl	x10, x10, x23
100d16684:     	ldr	q7, [sp, #0x130]
100d16688:     	neg.2d	v7, v7
100d1668c:     	str	q7, [sp, #0x180]
100d16690:     	mov	x11, #-0x1              ; =-1
100d16694:     	ldr	q7, [sp, #0x140]
100d16698:     	neg.2d	v7, v7
100d1669c:     	str	q7, [sp, #0x170]
100d166a0:     	lsl	x10, x11, x10
100d166a4:     	ldr	q7, [sp, #0x100]
100d166a8:     	neg.2d	v7, v7
100d166ac:     	str	q7, [sp, #0x160]
100d166b0:     	add	x11, x22, x25, lsl #3
100d166b4:     	ldr	q7, [sp, #0xf0]
100d166b8:     	neg.2d	v7, v7
100d166bc:     	str	q7, [sp, #0x150]
100d166c0:     	mov	w12, w24
100d166c4:     	ldr	q7, [sp, #0xe0]
100d166c8:     	neg.2d	v7, v7
100d166cc:     	str	q7, [sp, #0x140]
100d166d0:     	lsl	x12, x12, x8
100d166d4:     	ldr	q7, [sp, #0xd0]
100d166d8:     	neg.2d	v7, v7
100d166dc:     	str	q7, [sp, #0x130]
100d166e0:     	mov	w13, #0x20              ; =32
100d166e4:     	ldr	q7, [sp, #0xc0]
100d166e8:     	neg.2d	v7, v7
100d166ec:     	str	q7, [sp, #0x120]
100d166f0:     	lsr	x13, x13, x8
100d166f4:     	ldr	q7, [sp, #0xb0]
100d166f8:     	neg.2d	v7, v7
100d166fc:     	str	q7, [sp, #0x110]
100d16700:     	and	x14, x13, #0x38
100d16704:     	ldr	q7, [sp, #0xa0]
100d16708:     	neg.2d	v7, v7
100d1670c:     	str	q7, [sp, #0x100]
100d16710:     	ldr	q7, [sp, #0x90]
100d16714:     	neg.2d	v7, v7
100d16718:     	str	q7, [sp, #0xf0]
100d1671c:     	ldr	q7, [sp, #0x80]
100d16720:     	neg.2d	v7, v7
100d16724:     	str	q7, [sp, #0xe0]
100d16728:     	neg.2d	v7, v26
100d1672c:     	str	q7, [sp, #0xd0]
100d16730:     	neg.2d	v7, v27
100d16734:     	str	q7, [sp, #0xc0]
100d16738:     	neg.2d	v7, v28
100d1673c:     	str	q7, [sp, #0xb0]
100d16740:     	str	q1, [sp, #0x1b0]
100d16744:     	ldr	x15, [x22]
100d16748:     	lsr	x15, x15, x12
100d1674c:     	cmp	w23, #0x2
100d16750:     	b.ls	0x100d16760 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6c8>
100d16754:     	mov	x17, #0x0               ; =0
100d16758:     	mov	x16, #0x0               ; =0
100d1675c:     	b	0x100d16a0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x974>
100d16760:     	dup.2d	v12, x15
100d16764:     	ushl.2d	v7, v12, v5
100d16768:     	ushl.2d	v23, v12, v6
100d1676c:     	ushl.2d	v26, v12, v24
100d16770:     	ushl.2d	v27, v12, v25
100d16774:     	dup.2d	v13, x10
100d16778:     	bic.16b	v7, v7, v13
100d1677c:     	bic.16b	v28, v23, v13
100d16780:     	bic.16b	v14, v26, v13
100d16784:     	bic.16b	v15, v27, v13
100d16788:     	ushl.2d	v23, v7, v0
100d1678c:     	ushl.2d	v26, v28, v2
100d16790:     	ushl.2d	v27, v14, v3
100d16794:     	ushl.2d	v28, v15, v4
100d16798:     	cmp	x14, #0x8
100d1679c:     	b.eq	0x100d169e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d167a0:     	ushl.2d	v7, v12, v11
100d167a4:     	ushl.2d	v14, v12, v1
100d167a8:     	ldur	q20, [x29, #-0xb0]
100d167ac:     	ushl.2d	v15, v12, v20
100d167b0:     	ldur	q20, [x29, #-0xc0]
100d167b4:     	ushl.2d	v20, v12, v20
100d167b8:     	bic.16b	v7, v7, v13
100d167bc:     	bic.16b	v14, v14, v13
100d167c0:     	bic.16b	v15, v15, v13
100d167c4:     	bic.16b	v20, v20, v13
100d167c8:     	ushl.2d	v7, v7, v19
100d167cc:     	ushl.2d	v14, v14, v16
100d167d0:     	ushl.2d	v15, v15, v17
100d167d4:     	ushl.2d	v20, v20, v18
100d167d8:     	orr.16b	v23, v7, v23
100d167dc:     	orr.16b	v26, v14, v26
100d167e0:     	orr.16b	v27, v15, v27
100d167e4:     	orr.16b	v28, v20, v28
100d167e8:     	cmp	x14, #0x10
100d167ec:     	b.eq	0x100d169e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d167f0:     	ldp	q20, q7, [x29, #-0xe0]
100d167f4:     	ushl.2d	v7, v12, v7
100d167f8:     	ushl.2d	v20, v12, v20
100d167fc:     	ldp	q15, q14, [x29, #-0x100]
100d16800:     	ushl.2d	v14, v12, v14
100d16804:     	ushl.2d	v15, v12, v15
100d16808:     	bic.16b	v7, v7, v13
100d1680c:     	bic.16b	v20, v20, v13
100d16810:     	bic.16b	v14, v14, v13
100d16814:     	bic.16b	v15, v15, v13
100d16818:     	ushl.2d	v7, v7, v21
100d1681c:     	ushl.2d	v20, v20, v22
100d16820:     	ushl.2d	v14, v14, v8
100d16824:     	ushl.2d	v15, v15, v9
100d16828:     	orr.16b	v23, v7, v23
100d1682c:     	orr.16b	v26, v20, v26
100d16830:     	orr.16b	v27, v14, v27
100d16834:     	orr.16b	v28, v15, v28
100d16838:     	cmp	x14, #0x18
100d1683c:     	b.eq	0x100d169e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d16840:     	ldr	q1, [sp, #0x1a0]
100d16844:     	ushl.2d	v7, v12, v1
100d16848:     	ldr	q1, [sp, #0x190]
100d1684c:     	ushl.2d	v20, v12, v1
100d16850:     	ldr	q1, [sp, #0x180]
100d16854:     	ushl.2d	v14, v12, v1
100d16858:     	ldr	q1, [sp, #0x170]
100d1685c:     	ushl.2d	v15, v12, v1
100d16860:     	bic.16b	v7, v7, v13
100d16864:     	bic.16b	v20, v20, v13
100d16868:     	bic.16b	v14, v14, v13
100d1686c:     	bic.16b	v15, v15, v13
100d16870:     	ldr	q1, [sp, #0x1f0]
100d16874:     	ushl.2d	v7, v7, v1
100d16878:     	ldr	q1, [sp, #0x1e0]
100d1687c:     	ushl.2d	v20, v20, v1
100d16880:     	ldr	q1, [sp, #0x1d0]
100d16884:     	ushl.2d	v14, v14, v1
100d16888:     	ldr	q1, [sp, #0x1c0]
100d1688c:     	ushl.2d	v15, v15, v1
100d16890:     	orr.16b	v23, v7, v23
100d16894:     	orr.16b	v26, v20, v26
100d16898:     	orr.16b	v27, v14, v27
100d1689c:     	orr.16b	v28, v15, v28
100d168a0:     	cmp	x14, #0x20
100d168a4:     	b.eq	0x100d169e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x94c>
100d168a8:     	ldr	q1, [sp, #0x160]
100d168ac:     	ushl.2d	v7, v12, v1
100d168b0:     	bic.16b	v7, v7, v13
100d168b4:     	ldr	q1, [sp, #0x60]
100d168b8:     	ushl.2d	v7, v7, v1
100d168bc:     	ldr	q1, [sp, #0x150]
100d168c0:     	ushl.2d	v20, v12, v1
100d168c4:     	bic.16b	v20, v20, v13
100d168c8:     	ldr	q1, [sp, #0x50]
100d168cc:     	ushl.2d	v20, v20, v1
100d168d0:     	orr.16b	v1, v20, v7
100d168d4:     	str	q1, [sp, #0x90]
100d168d8:     	ldr	q1, [sp, #0x140]
100d168dc:     	ushl.2d	v20, v12, v1
100d168e0:     	bic.16b	v20, v20, v13
100d168e4:     	ldr	q1, [sp, #0x40]
100d168e8:     	ushl.2d	v20, v20, v1
100d168ec:     	ldr	q1, [sp, #0x130]
100d168f0:     	ushl.2d	v14, v12, v1
100d168f4:     	bic.16b	v14, v14, v13
100d168f8:     	ldr	q1, [sp, #0x30]
100d168fc:     	ushl.2d	v14, v14, v1
100d16900:     	orr.16b	v1, v14, v20
100d16904:     	str	q1, [sp, #0x80]
100d16908:     	ldr	q1, [sp, #0x120]
100d1690c:     	ushl.2d	v14, v12, v1
100d16910:     	bic.16b	v14, v14, v13
100d16914:     	ldr	q1, [sp, #0x20]
100d16918:     	ushl.2d	v14, v14, v1
100d1691c:     	ldp	q1, q7, [sp, #0x100]
100d16920:     	ushl.2d	v15, v12, v7
100d16924:     	bic.16b	v15, v15, v13
100d16928:     	ushl.2d	v15, v15, v10
100d1692c:     	orr.16b	v14, v15, v14
100d16930:     	ushl.2d	v15, v12, v1
100d16934:     	bic.16b	v15, v15, v13
100d16938:     	ushl.2d	v15, v15, v29
100d1693c:     	str	q0, [sp, #0xa0]
100d16940:     	mov.16b	v7, v18
100d16944:     	mov.16b	v18, v21
100d16948:     	ldr	q0, [sp, #0xf0]
100d1694c:     	ushl.2d	v21, v12, v0
100d16950:     	bic.16b	v21, v21, v13
100d16954:     	mov.16b	v1, v22
100d16958:     	ldr	q22, [sp, #0x70]
100d1695c:     	ushl.2d	v21, v21, v22
100d16960:     	orr.16b	v21, v21, v15
100d16964:     	ldr	q0, [sp, #0xe0]
100d16968:     	ushl.2d	v15, v12, v0
100d1696c:     	ldr	q0, [sp, #0xd0]
100d16970:     	ushl.2d	v22, v12, v0
100d16974:     	mov.16b	v0, v8
100d16978:     	ldp	q20, q8, [sp, #0xb0]
100d1697c:     	ushl.2d	v8, v12, v8
100d16980:     	ushl.2d	v12, v12, v20
100d16984:     	bic.16b	v15, v15, v13
100d16988:     	bic.16b	v22, v22, v13
100d1698c:     	bic.16b	v8, v8, v13
100d16990:     	bic.16b	v12, v12, v13
100d16994:     	ldr	q13, [sp, #0x10]
100d16998:     	ushl.2d	v13, v15, v13
100d1699c:     	orr.16b	v21, v21, v13
100d169a0:     	orr.16b	v23, v21, v23
100d169a4:     	ldr	q21, [sp]
100d169a8:     	ushl.2d	v21, v22, v21
100d169ac:     	mov.16b	v22, v1
100d169b0:     	orr.16b	v21, v14, v21
100d169b4:     	orr.16b	v26, v21, v26
100d169b8:     	ushl.2d	v21, v8, v30
100d169bc:     	mov.16b	v8, v0
100d169c0:     	ldp	q0, q1, [sp, #0x80]
100d169c4:     	orr.16b	v20, v0, v21
100d169c8:     	mov.16b	v21, v18
100d169cc:     	mov.16b	v18, v7
100d169d0:     	ldr	q0, [sp, #0xa0]
100d169d4:     	orr.16b	v27, v20, v27
100d169d8:     	ushl.2d	v20, v12, v31
100d169dc:     	orr.16b	v7, v1, v20
100d169e0:     	orr.16b	v28, v7, v28
100d169e4:     	ldr	q1, [sp, #0x1b0]
100d169e8:     	orr.16b	v7, v26, v23
100d169ec:     	orr.16b	v20, v28, v27
100d169f0:     	orr.16b	v7, v20, v7
100d169f4:     	mov	d20, v7[1]
100d169f8:     	orr.8b	v7, v7, v20
100d169fc:     	fmov	x16, d7
100d16a00:     	and	x17, x13, #0x38
100d16a04:     	cmp	x13, x14
100d16a08:     	b.eq	0x100d16a3c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9a4>
100d16a0c:     	lsl	x0, x17, #1
100d16a10:     	lsl	x1, x17, x8
100d16a14:     	add	x17, x17, #0x1
100d16a18:     	lsl	x2, x0, x8
100d16a1c:     	and	x2, x2, #0x3e
100d16a20:     	lsr	x2, x15, x2
100d16a24:     	bic	x2, x2, x10
100d16a28:     	lsl	x1, x2, x1
100d16a2c:     	orr	x16, x1, x16
100d16a30:     	add	x0, x0, #0x2
100d16a34:     	cmp	x13, x17
100d16a38:     	b.ne	0x100d16a10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x978>
100d16a3c:     	lsr	x0, x9, #1
100d16a40:     	cmp	x0, x19
100d16a44:     	b.hs	0x100d16b1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa84>
100d16a48:     	ubfiz	x15, x9, #5, #1
100d16a4c:     	add	x9, x9, #0x1
100d16a50:     	add	x22, x22, #0x8
100d16a54:     	ldr	x17, [x20, x0, lsl #3]
100d16a58:     	lsl	x15, x16, x15
100d16a5c:     	orr	x15, x17, x15
100d16a60:     	str	x15, [x20, x0, lsl #3]
100d16a64:     	cmp	x22, x11
100d16a68:     	b.ne	0x100d16744 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6ac>
100d16a6c:     	cmp	w28, #0x6
100d16a70:     	b.hs	0x100d16a8c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9f4>
100d16a74:     	mov	x8, #-0x1               ; =-1
100d16a78:     	ldr	x9, [sp, #0x208]
100d16a7c:     	lsl	x8, x8, x9
100d16a80:     	ldr	x9, [x20]
100d16a84:     	bic	x8, x9, x8
100d16a88:     	str	x8, [x20]
100d16a8c:     	stp	x19, x20, [x21]
100d16a90:     	str	x19, [x21, #0x10]
100d16a94:     	add	sp, sp, #0x280
100d16a98:     	ldp	x29, x30, [sp, #0x90]
100d16a9c:     	ldp	x20, x19, [sp, #0x80]
100d16aa0:     	ldp	x22, x21, [sp, #0x70]
100d16aa4:     	ldp	x24, x23, [sp, #0x60]
100d16aa8:     	ldp	x26, x25, [sp, #0x50]
100d16aac:     	ldp	x28, x27, [sp, #0x40]
100d16ab0:     	ldp	d9, d8, [sp, #0x30]
100d16ab4:     	ldp	d11, d10, [sp, #0x20]
100d16ab8:     	ldp	d13, d12, [sp, #0x10]
100d16abc:     	ldp	d15, d14, [sp], #0xa0
100d16ac0:     	ret
100d16ac4:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16ac8:     	add	x2, x2, #0x4c0
100d16acc:     	mov	x1, x23
100d16ad0:     	bl	0x10127cc38 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d16ad4:     	b	0x100d16b2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d16ad8:     	adrp	x0, 0x101343000 <dyld_stub_binder+0x101343000>
100d16adc:     	add	x0, x0, #0xae5
100d16ae0:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16ae4:     	add	x2, x2, #0x478
100d16ae8:     	mov	w1, #0x22               ; =34
100d16aec:     	bl	0x10127c888 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d16af0:     	adrp	x3, 0x101500000 <dyld_stub_binder+0x101500000>
100d16af4:     	add	x3, x3, #0x4d8
100d16af8:     	bl	0x10127c7d4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d16afc:     	b	0x100d16b2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d16b00:     	adrp	x0, 0x10131f000 <dyld_stub_binder+0x10131f000>
100d16b04:     	add	x0, x0, #0xaf5
100d16b08:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16b0c:     	add	x2, x2, #0x4a8
100d16b10:     	mov	w1, #0x37               ; =55
100d16b14:     	bl	0x10127c734 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d16b18:     	b	0x100d16b2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d16b1c:     	adrp	x2, 0x101500000 <dyld_stub_binder+0x101500000>
100d16b20:     	add	x2, x2, #0x490
100d16b24:     	mov	x1, x19
100d16b28:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d16b2c:     	brk	#0x1
100d16b30:     	mov	w0, #0x8                ; =8
100d16b34:     	mov	x1, x26
100d16b38:     	bl	0x10127c0a4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d16b3c:     	cbz	x19, 0x100d16b50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xab8>
100d16b40:     	mov	x19, x0
100d16b44:     	mov	x0, x20
100d16b48:     	bl	0x101284cb8 <dyld_stub_binder+0x101284cb8>
100d16b4c:     	mov	x0, x19
100d16b50:     	bl	0x101284b08 <dyld_stub_binder+0x101284b08>
