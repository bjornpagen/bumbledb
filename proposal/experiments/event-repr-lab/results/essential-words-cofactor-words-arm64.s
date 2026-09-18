
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100d1a098 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>:
100d1a098:     	stp	d15, d14, [sp, #-0xa0]!
100d1a09c:     	stp	d13, d12, [sp, #0x10]
100d1a0a0:     	stp	d11, d10, [sp, #0x20]
100d1a0a4:     	stp	d9, d8, [sp, #0x30]
100d1a0a8:     	stp	x28, x27, [sp, #0x40]
100d1a0ac:     	stp	x26, x25, [sp, #0x50]
100d1a0b0:     	stp	x24, x23, [sp, #0x60]
100d1a0b4:     	stp	x22, x21, [sp, #0x70]
100d1a0b8:     	stp	x20, x19, [sp, #0x80]
100d1a0bc:     	stp	x29, x30, [sp, #0x90]
100d1a0c0:     	add	x29, sp, #0x90
100d1a0c4:     	sub	sp, sp, #0x280
100d1a0c8:     	cmp	w4, w3
100d1a0cc:     	b.hs	0x100d1aad8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa40>
100d1a0d0:     	mov	x24, x5
100d1a0d4:     	mov	x23, x4
100d1a0d8:     	mov	x25, x2
100d1a0dc:     	mov	x22, x1
100d1a0e0:     	mov	x21, x0
100d1a0e4:     	sub	w8, w3, #0x1
100d1a0e8:     	and	w28, w8, #0x3f
100d1a0ec:     	mov	w9, #0x1                ; =1
100d1a0f0:     	lsl	x27, x9, x8
100d1a0f4:     	lsr	x8, x27, #6
100d1a0f8:     	cmp	w28, #0x6
100d1a0fc:     	cinc	x19, x8, lo
100d1a100:     	cbz	x19, 0x100d1a1d8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x140>
100d1a104:     	lsl	x26, x19, #3
100d1a108:     	mov	x0, x26
100d1a10c:     	mov	w1, #0x1                ; =1
100d1a110:     	bl	0x10128a2e4 <dyld_stub_binder+0x10128a2e4>
100d1a114:     	cbz	x0, 0x100d1ab30 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa98>
100d1a118:     	mov	x20, x0
100d1a11c:     	cmp	w23, #0x5
100d1a120:     	str	x27, [sp, #0x208]
100d1a124:     	b.ls	0x100d1a1e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x150>
100d1a128:     	add	w8, w23, #0x3a
100d1a12c:     	and	w26, w8, #0x3f
100d1a130:     	cmp	w26, #0x3f
100d1a134:     	b.eq	0x100d1ab00 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa68>
100d1a138:     	mov	w9, #0x1                ; =1
100d1a13c:     	lsl	x23, x9, x8
100d1a140:     	mov	w9, #0x2                ; =2
100d1a144:     	lsl	x2, x9, x8
100d1a148:     	neg	x8, x2
100d1a14c:     	and	x8, x25, x8
100d1a150:     	lsr	x9, x19, x26
100d1a154:     	sub	x10, x23, #0x1
100d1a158:     	tst	x19, x10
100d1a15c:     	cinc	x9, x9, ne
100d1a160:     	cmp	x19, #0x0
100d1a164:     	csel	x9, xzr, x9, eq
100d1a168:     	add	x10, x26, #0x1
100d1a16c:     	lsr	x8, x8, x10
100d1a170:     	cmp	x8, x9
100d1a174:     	csel	x25, x8, x9, lo
100d1a178:     	cbz	x25, 0x100d1aa6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1a17c:     	mov	w8, w24
100d1a180:     	lsl	x0, x8, x26
100d1a184:     	adds	x1, x0, x23
100d1a188:     	b.hs	0x100d1aaf0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d1a18c:     	cmp	x1, x2
100d1a190:     	b.hi	0x100d1aaf0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa58>
100d1a194:     	mov	x24, #0x0               ; =0
100d1a198:     	lsl	x27, x2, #3
100d1a19c:     	add	x22, x22, x0, lsl #3
100d1a1a0:     	lsl	x8, x24, x26
100d1a1a4:     	sub	x9, x19, x8
100d1a1a8:     	cmp	x23, x9
100d1a1ac:     	csel	x0, x23, x9, lo
100d1a1b0:     	b.hi	0x100d1aac4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa2c>
100d1a1b4:     	add	x24, x24, #0x1
100d1a1b8:     	lsl	x2, x0, #3
100d1a1bc:     	add	x0, x20, x8, lsl #3
100d1a1c0:     	mov	x1, x22
100d1a1c4:     	bl	0x10128a4dc <dyld_stub_binder+0x10128a4dc>
100d1a1c8:     	add	x22, x22, x27
100d1a1cc:     	cmp	x25, x24
100d1a1d0:     	b.ne	0x100d1a1a0 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x108>
100d1a1d4:     	b	0x100d1aa6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1a1d8:     	mov	w20, #0x8               ; =8
100d1a1dc:     	cmp	w23, #0x5
100d1a1e0:     	str	x27, [sp, #0x208]
100d1a1e4:     	b.hi	0x100d1a128 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x90>
100d1a1e8:     	cbz	x25, 0x100d1aa6c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9d4>
100d1a1ec:     	mov	x9, #0x0                ; =0
100d1a1f0:     	mov	w8, w23
100d1a1f4:     	dup.2d	v7, x8
100d1a1f8:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a1fc:     	ldr	q0, [x10, #0x220]
100d1a200:     	ushl.2d	v0, v0, v7
100d1a204:     	stur	q0, [x29, #-0xb0]
100d1a208:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a20c:     	ldr	q0, [x10, #0x370]
100d1a210:     	ushl.2d	v0, v0, v7
100d1a214:     	stur	q0, [x29, #-0xc0]
100d1a218:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a21c:     	ldr	q0, [x10, #0x380]
100d1a220:     	ushl.2d	v0, v0, v7
100d1a224:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a228:     	ldr	q1, [x10, #0x390]
100d1a22c:     	ushl.2d	v1, v1, v7
100d1a230:     	mov	w10, #0x3e              ; =62
100d1a234:     	dup.2d	v2, x10
100d1a238:     	and.16b	v3, v0, v2
100d1a23c:     	and.16b	v0, v1, v2
100d1a240:     	stp	q0, q3, [x29, #-0xe0]
100d1a244:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a248:     	ldr	q0, [x10, #0x240]
100d1a24c:     	ushl.2d	v0, v0, v7
100d1a250:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a254:     	ldr	q1, [x10, #0x3a0]
100d1a258:     	ushl.2d	v1, v1, v7
100d1a25c:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a260:     	ldr	q3, [x10, #0x3b0]
100d1a264:     	ushl.2d	v3, v3, v7
100d1a268:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a26c:     	ldr	q4, [x10, #0x3c0]
100d1a270:     	ushl.2d	v4, v4, v7
100d1a274:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a278:     	ldr	q16, [x10, #0x3d0]
100d1a27c:     	ushl.2d	v16, v16, v7
100d1a280:     	and.16b	v5, v1, v2
100d1a284:     	and.16b	v1, v3, v2
100d1a288:     	stp	q1, q5, [x29, #-0x100]
100d1a28c:     	and.16b	v3, v4, v2
100d1a290:     	and.16b	v1, v16, v2
100d1a294:     	stp	q1, q3, [sp, #0x190]
100d1a298:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a29c:     	ldr	q3, [x10, #0x3e0]
100d1a2a0:     	ushl.2d	v3, v3, v7
100d1a2a4:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2a8:     	ldr	q4, [x10, #0x3f0]
100d1a2ac:     	ushl.2d	v4, v4, v7
100d1a2b0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2b4:     	ldr	q16, [x10, #0x400]
100d1a2b8:     	ushl.2d	v16, v16, v7
100d1a2bc:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2c0:     	ldr	q17, [x10, #0x410]
100d1a2c4:     	ushl.2d	v17, v17, v7
100d1a2c8:     	and.16b	v5, v3, v2
100d1a2cc:     	and.16b	v1, v4, v2
100d1a2d0:     	stp	q1, q5, [sp, #0x170]
100d1a2d4:     	and.16b	v3, v16, v2
100d1a2d8:     	and.16b	v1, v17, v2
100d1a2dc:     	stp	q1, q3, [sp, #0x150]
100d1a2e0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2e4:     	ldr	q3, [x10, #0x420]
100d1a2e8:     	ushl.2d	v3, v3, v7
100d1a2ec:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2f0:     	ldr	q4, [x10, #0x430]
100d1a2f4:     	ushl.2d	v4, v4, v7
100d1a2f8:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a2fc:     	ldr	q16, [x10, #0x440]
100d1a300:     	ushl.2d	v16, v16, v7
100d1a304:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a308:     	ldr	q17, [x10, #0x450]
100d1a30c:     	ushl.2d	v17, v17, v7
100d1a310:     	and.16b	v5, v3, v2
100d1a314:     	and.16b	v1, v4, v2
100d1a318:     	stp	q5, q1, [sp, #0x110]
100d1a31c:     	and.16b	v3, v16, v2
100d1a320:     	and.16b	v1, v17, v2
100d1a324:     	stp	q3, q1, [sp, #0x130]
100d1a328:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a32c:     	ldr	q3, [x10, #0x460]
100d1a330:     	ushl.2d	v3, v3, v7
100d1a334:     	and.16b	v1, v3, v2
100d1a338:     	str	q1, [sp, #0x100]
100d1a33c:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a340:     	ldr	q3, [x10, #0x480]
100d1a344:     	ushl.2d	v3, v3, v7
100d1a348:     	and.16b	v1, v3, v2
100d1a34c:     	str	q1, [sp, #0xf0]
100d1a350:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a354:     	ldr	q3, [x10, #0x490]
100d1a358:     	ushl.2d	v3, v3, v7
100d1a35c:     	and.16b	v1, v3, v2
100d1a360:     	str	q1, [sp, #0xe0]
100d1a364:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a368:     	ldr	q3, [x10, #0x4b0]
100d1a36c:     	ushl.2d	v3, v3, v7
100d1a370:     	and.16b	v1, v3, v2
100d1a374:     	str	q1, [sp, #0xd0]
100d1a378:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a37c:     	ldr	q3, [x10, #0x4c0]
100d1a380:     	ushl.2d	v3, v3, v7
100d1a384:     	and.16b	v1, v3, v2
100d1a388:     	str	q1, [sp, #0xc0]
100d1a38c:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a390:     	ldr	q3, [x10, #0x4e0]
100d1a394:     	ushl.2d	v3, v3, v7
100d1a398:     	and.16b	v1, v3, v2
100d1a39c:     	str	q1, [sp, #0xb0]
100d1a3a0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a3a4:     	ldr	q3, [x10, #0x4f0]
100d1a3a8:     	ushl.2d	v3, v3, v7
100d1a3ac:     	and.16b	v1, v3, v2
100d1a3b0:     	str	q1, [sp, #0xa0]
100d1a3b4:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a3b8:     	ldr	q3, [x10, #0x510]
100d1a3bc:     	ushl.2d	v3, v3, v7
100d1a3c0:     	mov	w10, #0x1e              ; =30
100d1a3c4:     	dup.2d	v4, x10
100d1a3c8:     	and.16b	v1, v3, v4
100d1a3cc:     	str	q1, [sp, #0x90]
100d1a3d0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a3d4:     	ldr	q3, [x10, #0x160]
100d1a3d8:     	ushl.2d	v3, v3, v7
100d1a3dc:     	mov	w10, #0x2f              ; =47
100d1a3e0:     	dup.2d	v4, x10
100d1a3e4:     	and.16b	v1, v3, v4
100d1a3e8:     	str	q1, [sp, #0x70]
100d1a3ec:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a3f0:     	ldr	q3, [x10, #0x520]
100d1a3f4:     	ushl.2d	v3, v3, v7
100d1a3f8:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a3fc:     	ldr	q4, [x10, #0x530]
100d1a400:     	ushl.2d	v4, v4, v7
100d1a404:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a408:     	ldr	q16, [x10, #0x540]
100d1a40c:     	ushl.2d	v16, v16, v7
100d1a410:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a414:     	ldr	q17, [x10, #0x550]
100d1a418:     	ushl.2d	v17, v17, v7
100d1a41c:     	and.16b	v1, v3, v2
100d1a420:     	str	q1, [sp, #0x80]
100d1a424:     	and.16b	v26, v4, v2
100d1a428:     	and.16b	v27, v16, v2
100d1a42c:     	and.16b	v28, v17, v2
100d1a430:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a434:     	ldr	q2, [x10, #0x260]
100d1a438:     	ushl.2d	v2, v2, v7
100d1a43c:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a440:     	ldr	q3, [x10, #0x440]
100d1a444:     	ushl.2d	v3, v3, v7
100d1a448:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a44c:     	ldr	q4, [x10, #0x430]
100d1a450:     	ushl.2d	v4, v4, v7
100d1a454:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a458:     	ldr	q16, [x10, #0x420]
100d1a45c:     	ushl.2d	v23, v16, v7
100d1a460:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a464:     	ldr	q17, [x10, #0x410]
100d1a468:     	ushl.2d	v16, v17, v7
100d1a46c:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a470:     	ldr	q18, [x10, #0x400]
100d1a474:     	ushl.2d	v17, v18, v7
100d1a478:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a47c:     	ldr	q19, [x10, #0x3f0]
100d1a480:     	ushl.2d	v18, v19, v7
100d1a484:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a488:     	ldr	q20, [x10, #0xf0]
100d1a48c:     	ushl.2d	v20, v20, v7
100d1a490:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a494:     	ldr	q21, [x10, #0x100]
100d1a498:     	ushl.2d	v21, v21, v7
100d1a49c:     	adrp	x10, 0x10131c000 <GCC_except_table9305+0x108>
100d1a4a0:     	ldr	q22, [x10, #0xff0]
100d1a4a4:     	ushl.2d	v22, v22, v7
100d1a4a8:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4ac:     	ldr	q5, [x10, #0x110]
100d1a4b0:     	ushl.2d	v5, v5, v7
100d1a4b4:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4b8:     	ldr	q6, [x10, #0x120]
100d1a4bc:     	ushl.2d	v6, v6, v7
100d1a4c0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4c4:     	ldr	q24, [x10, #0x130]
100d1a4c8:     	ushl.2d	v24, v24, v7
100d1a4cc:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4d0:     	ldr	q25, [x10, #0x140]
100d1a4d4:     	ushl.2d	v25, v25, v7
100d1a4d8:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4dc:     	ldr	q1, [x10, #0x150]
100d1a4e0:     	ushl.2d	v1, v1, v7
100d1a4e4:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4e8:     	ldr	q29, [x10, #0x470]
100d1a4ec:     	ushl.2d	v29, v29, v7
100d1a4f0:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a4f4:     	ldr	q30, [x10, #0x190]
100d1a4f8:     	ushl.2d	v30, v30, v7
100d1a4fc:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a500:     	ldr	q31, [x10, #0x4a0]
100d1a504:     	ushl.2d	v31, v31, v7
100d1a508:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a50c:     	ldr	q8, [x10, #0x180]
100d1a510:     	ushl.2d	v8, v8, v7
100d1a514:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a518:     	ldr	q9, [x10, #0x4d0]
100d1a51c:     	ushl.2d	v9, v9, v7
100d1a520:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a524:     	ldr	q10, [x10, #0x170]
100d1a528:     	ushl.2d	v10, v10, v7
100d1a52c:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a530:     	ldr	q11, [x10, #0x500]
100d1a534:     	ushl.2d	v11, v11, v7
100d1a538:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a53c:     	ldr	q15, [x10, #0x560]
100d1a540:     	ushl.2d	v15, v15, v7
100d1a544:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a548:     	ldr	q14, [x10, #0x570]
100d1a54c:     	ushl.2d	v14, v14, v7
100d1a550:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a554:     	ldr	q13, [x10, #0x580]
100d1a558:     	ushl.2d	v13, v13, v7
100d1a55c:     	adrp	x10, 0x10131d000 <dyld_stub_binder+0x10131d000>
100d1a560:     	ldr	q12, [x10, #0x590]
100d1a564:     	ushl.2d	v12, v12, v7
100d1a568:     	mov	w10, #0x3f              ; =63
100d1a56c:     	dup.2d	v7, x10
100d1a570:     	and.16b	v19, v23, v7
100d1a574:     	and.16b	v16, v16, v7
100d1a578:     	and.16b	v17, v17, v7
100d1a57c:     	and.16b	v18, v18, v7
100d1a580:     	and.16b	v20, v20, v7
100d1a584:     	and.16b	v23, v21, v7
100d1a588:     	mov.16b	v21, v20
100d1a58c:     	and.16b	v20, v22, v7
100d1a590:     	mov.16b	v22, v23
100d1a594:     	and.16b	v5, v5, v7
100d1a598:     	and.16b	v6, v6, v7
100d1a59c:     	str	q6, [sp, #0x1f0]
100d1a5a0:     	and.16b	v6, v24, v7
100d1a5a4:     	str	q6, [sp, #0x1e0]
100d1a5a8:     	and.16b	v6, v25, v7
100d1a5ac:     	and.16b	v1, v1, v7
100d1a5b0:     	stp	q1, q6, [sp, #0x1c0]
100d1a5b4:     	and.16b	v6, v29, v7
100d1a5b8:     	and.16b	v1, v30, v7
100d1a5bc:     	stp	q1, q6, [sp, #0x50]
100d1a5c0:     	and.16b	v6, v31, v7
100d1a5c4:     	and.16b	v1, v8, v7
100d1a5c8:     	stp	q1, q6, [sp, #0x30]
100d1a5cc:     	mov.16b	v8, v20
100d1a5d0:     	and.16b	v6, v9, v7
100d1a5d4:     	mov.16b	v9, v5
100d1a5d8:     	and.16b	v10, v10, v7
100d1a5dc:     	and.16b	v29, v11, v7
100d1a5e0:     	and.16b	v1, v15, v7
100d1a5e4:     	stp	q1, q6, [sp, #0x10]
100d1a5e8:     	and.16b	v1, v14, v7
100d1a5ec:     	str	q1, [sp]
100d1a5f0:     	and.16b	v30, v13, v7
100d1a5f4:     	and.16b	v31, v12, v7
100d1a5f8:     	ldp	q1, q6, [x29, #-0xc0]
100d1a5fc:     	neg.2d	v5, v6
100d1a600:     	neg.2d	v6, v1
100d1a604:     	ldp	q1, q7, [x29, #-0xe0]
100d1a608:     	neg.2d	v24, v7
100d1a60c:     	neg.2d	v25, v1
100d1a610:     	ldp	q1, q7, [x29, #-0x100]
100d1a614:     	neg.2d	v11, v7
100d1a618:     	neg.2d	v1, v1
100d1a61c:     	ldr	q7, [sp, #0x1a0]
100d1a620:     	neg.2d	v7, v7
100d1a624:     	stur	q7, [x29, #-0xb0]
100d1a628:     	ldr	q7, [sp, #0x190]
100d1a62c:     	neg.2d	v7, v7
100d1a630:     	stur	q7, [x29, #-0xc0]
100d1a634:     	ldr	q7, [sp, #0x180]
100d1a638:     	neg.2d	v7, v7
100d1a63c:     	stur	q7, [x29, #-0xd0]
100d1a640:     	ldr	q7, [sp, #0x170]
100d1a644:     	neg.2d	v7, v7
100d1a648:     	stur	q7, [x29, #-0xe0]
100d1a64c:     	ldr	q7, [sp, #0x160]
100d1a650:     	neg.2d	v7, v7
100d1a654:     	stur	q7, [x29, #-0xf0]
100d1a658:     	ldr	q7, [sp, #0x150]
100d1a65c:     	neg.2d	v7, v7
100d1a660:     	stur	q7, [x29, #-0x100]
100d1a664:     	ldr	q7, [sp, #0x110]
100d1a668:     	neg.2d	v7, v7
100d1a66c:     	str	q7, [sp, #0x1a0]
100d1a670:     	mov	w10, #0x1               ; =1
100d1a674:     	ldr	q7, [sp, #0x120]
100d1a678:     	neg.2d	v7, v7
100d1a67c:     	str	q7, [sp, #0x190]
100d1a680:     	lsl	x10, x10, x23
100d1a684:     	ldr	q7, [sp, #0x130]
100d1a688:     	neg.2d	v7, v7
100d1a68c:     	str	q7, [sp, #0x180]
100d1a690:     	mov	x11, #-0x1              ; =-1
100d1a694:     	ldr	q7, [sp, #0x140]
100d1a698:     	neg.2d	v7, v7
100d1a69c:     	str	q7, [sp, #0x170]
100d1a6a0:     	lsl	x10, x11, x10
100d1a6a4:     	ldr	q7, [sp, #0x100]
100d1a6a8:     	neg.2d	v7, v7
100d1a6ac:     	str	q7, [sp, #0x160]
100d1a6b0:     	add	x11, x22, x25, lsl #3
100d1a6b4:     	ldr	q7, [sp, #0xf0]
100d1a6b8:     	neg.2d	v7, v7
100d1a6bc:     	str	q7, [sp, #0x150]
100d1a6c0:     	mov	w12, w24
100d1a6c4:     	ldr	q7, [sp, #0xe0]
100d1a6c8:     	neg.2d	v7, v7
100d1a6cc:     	str	q7, [sp, #0x140]
100d1a6d0:     	lsl	x12, x12, x8
100d1a6d4:     	ldr	q7, [sp, #0xd0]
100d1a6d8:     	neg.2d	v7, v7
100d1a6dc:     	str	q7, [sp, #0x130]
100d1a6e0:     	mov	w13, #0x20              ; =32
100d1a6e4:     	ldr	q7, [sp, #0xc0]
100d1a6e8:     	neg.2d	v7, v7
100d1a6ec:     	str	q7, [sp, #0x120]
100d1a6f0:     	lsr	x13, x13, x8
100d1a6f4:     	ldr	q7, [sp, #0xb0]
100d1a6f8:     	neg.2d	v7, v7
100d1a6fc:     	str	q7, [sp, #0x110]
100d1a700:     	and	x14, x13, #0x38
100d1a704:     	ldr	q7, [sp, #0xa0]
100d1a708:     	neg.2d	v7, v7
100d1a70c:     	str	q7, [sp, #0x100]
100d1a710:     	ldr	q7, [sp, #0x90]
100d1a714:     	neg.2d	v7, v7
100d1a718:     	str	q7, [sp, #0xf0]
100d1a71c:     	ldr	q7, [sp, #0x80]
100d1a720:     	neg.2d	v7, v7
100d1a724:     	str	q7, [sp, #0xe0]
100d1a728:     	neg.2d	v7, v26
100d1a72c:     	str	q7, [sp, #0xd0]
100d1a730:     	neg.2d	v7, v27
100d1a734:     	str	q7, [sp, #0xc0]
100d1a738:     	neg.2d	v7, v28
100d1a73c:     	str	q7, [sp, #0xb0]
100d1a740:     	str	q1, [sp, #0x1b0]
100d1a744:     	ldr	x15, [x22]
100d1a748:     	lsr	x15, x15, x12
100d1a74c:     	cmp	w23, #0x2
100d1a750:     	b.ls	0x100d1a760 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6c8>
100d1a754:     	mov	x17, #0x0               ; =0
100d1a758:     	mov	x16, #0x0               ; =0
100d1a75c:     	b	0x100d1aa0c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x974>
100d1a760:     	dup.2d	v12, x15
100d1a764:     	ushl.2d	v7, v12, v5
100d1a768:     	ushl.2d	v23, v12, v6
100d1a76c:     	ushl.2d	v26, v12, v24
100d1a770:     	ushl.2d	v27, v12, v25
100d1a774:     	dup.2d	v13, x10
100d1a778:     	bic.16b	v7, v7, v13
100d1a77c:     	bic.16b	v28, v23, v13
100d1a780:     	bic.16b	v14, v26, v13
100d1a784:     	bic.16b	v15, v27, v13
100d1a788:     	ushl.2d	v23, v7, v0
100d1a78c:     	ushl.2d	v26, v28, v2
100d1a790:     	ushl.2d	v27, v14, v3
100d1a794:     	ushl.2d	v28, v15, v4
100d1a798:     	cmp	x14, #0x8
100d1a79c:     	b.eq	0x100d1a9e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1a7a0:     	ushl.2d	v7, v12, v11
100d1a7a4:     	ushl.2d	v14, v12, v1
100d1a7a8:     	ldur	q20, [x29, #-0xb0]
100d1a7ac:     	ushl.2d	v15, v12, v20
100d1a7b0:     	ldur	q20, [x29, #-0xc0]
100d1a7b4:     	ushl.2d	v20, v12, v20
100d1a7b8:     	bic.16b	v7, v7, v13
100d1a7bc:     	bic.16b	v14, v14, v13
100d1a7c0:     	bic.16b	v15, v15, v13
100d1a7c4:     	bic.16b	v20, v20, v13
100d1a7c8:     	ushl.2d	v7, v7, v19
100d1a7cc:     	ushl.2d	v14, v14, v16
100d1a7d0:     	ushl.2d	v15, v15, v17
100d1a7d4:     	ushl.2d	v20, v20, v18
100d1a7d8:     	orr.16b	v23, v7, v23
100d1a7dc:     	orr.16b	v26, v14, v26
100d1a7e0:     	orr.16b	v27, v15, v27
100d1a7e4:     	orr.16b	v28, v20, v28
100d1a7e8:     	cmp	x14, #0x10
100d1a7ec:     	b.eq	0x100d1a9e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1a7f0:     	ldp	q20, q7, [x29, #-0xe0]
100d1a7f4:     	ushl.2d	v7, v12, v7
100d1a7f8:     	ushl.2d	v20, v12, v20
100d1a7fc:     	ldp	q15, q14, [x29, #-0x100]
100d1a800:     	ushl.2d	v14, v12, v14
100d1a804:     	ushl.2d	v15, v12, v15
100d1a808:     	bic.16b	v7, v7, v13
100d1a80c:     	bic.16b	v20, v20, v13
100d1a810:     	bic.16b	v14, v14, v13
100d1a814:     	bic.16b	v15, v15, v13
100d1a818:     	ushl.2d	v7, v7, v21
100d1a81c:     	ushl.2d	v20, v20, v22
100d1a820:     	ushl.2d	v14, v14, v8
100d1a824:     	ushl.2d	v15, v15, v9
100d1a828:     	orr.16b	v23, v7, v23
100d1a82c:     	orr.16b	v26, v20, v26
100d1a830:     	orr.16b	v27, v14, v27
100d1a834:     	orr.16b	v28, v15, v28
100d1a838:     	cmp	x14, #0x18
100d1a83c:     	b.eq	0x100d1a9e8 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x950>
100d1a840:     	ldr	q1, [sp, #0x1a0]
100d1a844:     	ushl.2d	v7, v12, v1
100d1a848:     	ldr	q1, [sp, #0x190]
100d1a84c:     	ushl.2d	v20, v12, v1
100d1a850:     	ldr	q1, [sp, #0x180]
100d1a854:     	ushl.2d	v14, v12, v1
100d1a858:     	ldr	q1, [sp, #0x170]
100d1a85c:     	ushl.2d	v15, v12, v1
100d1a860:     	bic.16b	v7, v7, v13
100d1a864:     	bic.16b	v20, v20, v13
100d1a868:     	bic.16b	v14, v14, v13
100d1a86c:     	bic.16b	v15, v15, v13
100d1a870:     	ldr	q1, [sp, #0x1f0]
100d1a874:     	ushl.2d	v7, v7, v1
100d1a878:     	ldr	q1, [sp, #0x1e0]
100d1a87c:     	ushl.2d	v20, v20, v1
100d1a880:     	ldr	q1, [sp, #0x1d0]
100d1a884:     	ushl.2d	v14, v14, v1
100d1a888:     	ldr	q1, [sp, #0x1c0]
100d1a88c:     	ushl.2d	v15, v15, v1
100d1a890:     	orr.16b	v23, v7, v23
100d1a894:     	orr.16b	v26, v20, v26
100d1a898:     	orr.16b	v27, v14, v27
100d1a89c:     	orr.16b	v28, v15, v28
100d1a8a0:     	cmp	x14, #0x20
100d1a8a4:     	b.eq	0x100d1a9e4 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x94c>
100d1a8a8:     	ldr	q1, [sp, #0x160]
100d1a8ac:     	ushl.2d	v7, v12, v1
100d1a8b0:     	bic.16b	v7, v7, v13
100d1a8b4:     	ldr	q1, [sp, #0x60]
100d1a8b8:     	ushl.2d	v7, v7, v1
100d1a8bc:     	ldr	q1, [sp, #0x150]
100d1a8c0:     	ushl.2d	v20, v12, v1
100d1a8c4:     	bic.16b	v20, v20, v13
100d1a8c8:     	ldr	q1, [sp, #0x50]
100d1a8cc:     	ushl.2d	v20, v20, v1
100d1a8d0:     	orr.16b	v1, v20, v7
100d1a8d4:     	str	q1, [sp, #0x90]
100d1a8d8:     	ldr	q1, [sp, #0x140]
100d1a8dc:     	ushl.2d	v20, v12, v1
100d1a8e0:     	bic.16b	v20, v20, v13
100d1a8e4:     	ldr	q1, [sp, #0x40]
100d1a8e8:     	ushl.2d	v20, v20, v1
100d1a8ec:     	ldr	q1, [sp, #0x130]
100d1a8f0:     	ushl.2d	v14, v12, v1
100d1a8f4:     	bic.16b	v14, v14, v13
100d1a8f8:     	ldr	q1, [sp, #0x30]
100d1a8fc:     	ushl.2d	v14, v14, v1
100d1a900:     	orr.16b	v1, v14, v20
100d1a904:     	str	q1, [sp, #0x80]
100d1a908:     	ldr	q1, [sp, #0x120]
100d1a90c:     	ushl.2d	v14, v12, v1
100d1a910:     	bic.16b	v14, v14, v13
100d1a914:     	ldr	q1, [sp, #0x20]
100d1a918:     	ushl.2d	v14, v14, v1
100d1a91c:     	ldp	q1, q7, [sp, #0x100]
100d1a920:     	ushl.2d	v15, v12, v7
100d1a924:     	bic.16b	v15, v15, v13
100d1a928:     	ushl.2d	v15, v15, v10
100d1a92c:     	orr.16b	v14, v15, v14
100d1a930:     	ushl.2d	v15, v12, v1
100d1a934:     	bic.16b	v15, v15, v13
100d1a938:     	ushl.2d	v15, v15, v29
100d1a93c:     	str	q0, [sp, #0xa0]
100d1a940:     	mov.16b	v7, v18
100d1a944:     	mov.16b	v18, v21
100d1a948:     	ldr	q0, [sp, #0xf0]
100d1a94c:     	ushl.2d	v21, v12, v0
100d1a950:     	bic.16b	v21, v21, v13
100d1a954:     	mov.16b	v1, v22
100d1a958:     	ldr	q22, [sp, #0x70]
100d1a95c:     	ushl.2d	v21, v21, v22
100d1a960:     	orr.16b	v21, v21, v15
100d1a964:     	ldr	q0, [sp, #0xe0]
100d1a968:     	ushl.2d	v15, v12, v0
100d1a96c:     	ldr	q0, [sp, #0xd0]
100d1a970:     	ushl.2d	v22, v12, v0
100d1a974:     	mov.16b	v0, v8
100d1a978:     	ldp	q20, q8, [sp, #0xb0]
100d1a97c:     	ushl.2d	v8, v12, v8
100d1a980:     	ushl.2d	v12, v12, v20
100d1a984:     	bic.16b	v15, v15, v13
100d1a988:     	bic.16b	v22, v22, v13
100d1a98c:     	bic.16b	v8, v8, v13
100d1a990:     	bic.16b	v12, v12, v13
100d1a994:     	ldr	q13, [sp, #0x10]
100d1a998:     	ushl.2d	v13, v15, v13
100d1a99c:     	orr.16b	v21, v21, v13
100d1a9a0:     	orr.16b	v23, v21, v23
100d1a9a4:     	ldr	q21, [sp]
100d1a9a8:     	ushl.2d	v21, v22, v21
100d1a9ac:     	mov.16b	v22, v1
100d1a9b0:     	orr.16b	v21, v14, v21
100d1a9b4:     	orr.16b	v26, v21, v26
100d1a9b8:     	ushl.2d	v21, v8, v30
100d1a9bc:     	mov.16b	v8, v0
100d1a9c0:     	ldp	q0, q1, [sp, #0x80]
100d1a9c4:     	orr.16b	v20, v0, v21
100d1a9c8:     	mov.16b	v21, v18
100d1a9cc:     	mov.16b	v18, v7
100d1a9d0:     	ldr	q0, [sp, #0xa0]
100d1a9d4:     	orr.16b	v27, v20, v27
100d1a9d8:     	ushl.2d	v20, v12, v31
100d1a9dc:     	orr.16b	v7, v1, v20
100d1a9e0:     	orr.16b	v28, v7, v28
100d1a9e4:     	ldr	q1, [sp, #0x1b0]
100d1a9e8:     	orr.16b	v7, v26, v23
100d1a9ec:     	orr.16b	v20, v28, v27
100d1a9f0:     	orr.16b	v7, v20, v7
100d1a9f4:     	mov	d20, v7[1]
100d1a9f8:     	orr.8b	v7, v7, v20
100d1a9fc:     	fmov	x16, d7
100d1aa00:     	and	x17, x13, #0x38
100d1aa04:     	cmp	x13, x14
100d1aa08:     	b.eq	0x100d1aa3c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9a4>
100d1aa0c:     	lsl	x0, x17, #1
100d1aa10:     	lsl	x1, x17, x8
100d1aa14:     	add	x17, x17, #0x1
100d1aa18:     	lsl	x2, x0, x8
100d1aa1c:     	and	x2, x2, #0x3e
100d1aa20:     	lsr	x2, x15, x2
100d1aa24:     	bic	x2, x2, x10
100d1aa28:     	lsl	x1, x2, x1
100d1aa2c:     	orr	x16, x1, x16
100d1aa30:     	add	x0, x0, #0x2
100d1aa34:     	cmp	x13, x17
100d1aa38:     	b.ne	0x100d1aa10 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x978>
100d1aa3c:     	lsr	x0, x9, #1
100d1aa40:     	cmp	x0, x19
100d1aa44:     	b.hs	0x100d1ab1c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa84>
100d1aa48:     	ubfiz	x15, x9, #5, #1
100d1aa4c:     	add	x9, x9, #0x1
100d1aa50:     	add	x22, x22, #0x8
100d1aa54:     	ldr	x17, [x20, x0, lsl #3]
100d1aa58:     	lsl	x15, x16, x15
100d1aa5c:     	orr	x15, x17, x15
100d1aa60:     	str	x15, [x20, x0, lsl #3]
100d1aa64:     	cmp	x22, x11
100d1aa68:     	b.ne	0x100d1a744 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x6ac>
100d1aa6c:     	cmp	w28, #0x6
100d1aa70:     	b.hs	0x100d1aa8c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0x9f4>
100d1aa74:     	mov	x8, #-0x1               ; =-1
100d1aa78:     	ldr	x9, [sp, #0x208]
100d1aa7c:     	lsl	x8, x8, x9
100d1aa80:     	ldr	x9, [x20]
100d1aa84:     	bic	x8, x9, x8
100d1aa88:     	str	x8, [x20]
100d1aa8c:     	stp	x19, x20, [x21]
100d1aa90:     	str	x19, [x21, #0x10]
100d1aa94:     	add	sp, sp, #0x280
100d1aa98:     	ldp	x29, x30, [sp, #0x90]
100d1aa9c:     	ldp	x20, x19, [sp, #0x80]
100d1aaa0:     	ldp	x22, x21, [sp, #0x70]
100d1aaa4:     	ldp	x24, x23, [sp, #0x60]
100d1aaa8:     	ldp	x26, x25, [sp, #0x50]
100d1aaac:     	ldp	x28, x27, [sp, #0x40]
100d1aab0:     	ldp	d9, d8, [sp, #0x30]
100d1aab4:     	ldp	d11, d10, [sp, #0x20]
100d1aab8:     	ldp	d13, d12, [sp, #0x10]
100d1aabc:     	ldp	d15, d14, [sp], #0xa0
100d1aac0:     	ret
100d1aac4:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aac8:     	add	x2, x2, #0x798
100d1aacc:     	mov	x1, x23
100d1aad0:     	bl	0x101282378 <__RNvNvNtCs4sDCw1iE1MS_4core5slice20copy_from_slice_impl17len_mismatch_fail>
100d1aad4:     	b	0x100d1ab2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1aad8:     	adrp	x0, 0x101349000 <dyld_stub_binder+0x101349000>
100d1aadc:     	add	x0, x0, #0x92d
100d1aae0:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aae4:     	add	x2, x2, #0x750
100d1aae8:     	mov	w1, #0x22               ; =34
100d1aaec:     	bl	0x101281fc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100d1aaf0:     	adrp	x3, 0x101508000 <dyld_stub_binder+0x101508000>
100d1aaf4:     	add	x3, x3, #0x7b0
100d1aaf8:     	bl	0x101281f14 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100d1aafc:     	b	0x100d1ab2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1ab00:     	adrp	x0, 0x101325000 <dyld_stub_binder+0x101325000>
100d1ab04:     	add	x0, x0, #0x745
100d1ab08:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1ab0c:     	add	x2, x2, #0x780
100d1ab10:     	mov	w1, #0x37               ; =55
100d1ab14:     	bl	0x101281e74 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
100d1ab18:     	b	0x100d1ab2c <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xa94>
100d1ab1c:     	adrp	x2, 0x101508000 <dyld_stub_binder+0x101508000>
100d1ab20:     	add	x2, x2, #0x768
100d1ab24:     	mov	x1, x19
100d1ab28:     	bl	0x101281fdc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100d1ab2c:     	brk	#0x1
100d1ab30:     	mov	w0, #0x8                ; =8
100d1ab34:     	mov	x1, x26
100d1ab38:     	bl	0x1012817e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100d1ab3c:     	cbz	x19, 0x100d1ab50 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor+0xab8>
100d1ab40:     	mov	x19, x0
100d1ab44:     	mov	x0, x20
100d1ab48:     	bl	0x10128a3f8 <dyld_stub_binder+0x10128a3f8>
100d1ab4c:     	mov	x0, x19
100d1ab50:     	bl	0x10128a248 <dyld_stub_binder+0x10128a248>
