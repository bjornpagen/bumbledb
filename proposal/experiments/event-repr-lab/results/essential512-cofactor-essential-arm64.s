
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>:
100a20140:     	sub	sp, sp, #0x80
100a20144:     	stp	x28, x27, [sp, #0x20]
100a20148:     	stp	x26, x25, [sp, #0x30]
100a2014c:     	stp	x24, x23, [sp, #0x40]
100a20150:     	stp	x22, x21, [sp, #0x50]
100a20154:     	stp	x20, x19, [sp, #0x60]
100a20158:     	stp	x29, x30, [sp, #0x70]
100a2015c:     	add	x29, sp, #0x70
100a20160:     	ldr	w8, [x0, #0x90]
100a20164:     	cmp	w2, w8
100a20168:     	b.hs	0x100a204b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x378>
100a2016c:     	mov	x19, x1
100a20170:     	mov	x20, x0
100a20174:     	lsr	w0, w1, #1
100a20178:     	ldr	x1, [x20, #0x28]
100a2017c:     	cmp	x1, x0
100a20180:     	b.ls	0x100a204e8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3a8>
100a20184:     	mov	x22, x2
100a20188:     	ldr	x8, [x20, #0x20]
100a2018c:     	add	x8, x8, x0, lsl #5
100a20190:     	ldr	x27, [x8, #0x18]
100a20194:     	mov	w9, #0x1                ; =1
100a20198:     	lsl	x28, x9, x2
100a2019c:     	tst	x27, x28
100a201a0:     	b.eq	0x100a20474 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x334>
100a201a4:     	mov	x21, x3
100a201a8:     	ldr	x9, [x20, #0x88]
100a201ac:     	cbz	x9, 0x100a2027c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x13c>
100a201b0:     	mov	x9, #0x0                ; =0
100a201b4:     	mov	w10, w19
100a201b8:     	mov	x11, #0xa9c5            ; =43461
100a201bc:     	movk	x11, #0x2e62, lsl #16
100a201c0:     	movk	x11, #0x7aea, lsl #32
100a201c4:     	movk	x11, #0xf135, lsl #48
100a201c8:     	mul	x10, x10, x11
100a201cc:     	add	x10, x10, w22, uxtw
100a201d0:     	mul	x10, x10, x11
100a201d4:     	add	x10, x10, w21, uxtw
100a201d8:     	mul	x10, x10, x11
100a201dc:     	ror	x12, x10, #0x2c
100a201e0:     	lsr	x13, x12, #57
100a201e4:     	ldp	x11, x10, [x20, #0x70]
100a201e8:     	dup.8b	v0, w13
100a201ec:     	movi.2d	v1, #0xffffffffffffffff
100a201f0:     	and	x12, x12, x10
100a201f4:     	ldr	d2, [x11, x12]
100a201f8:     	cmeq.8b	v3, v2, v0
100a201fc:     	fmov	x13, d3
100a20200:     	ands	x13, x13, #0x8080808080808080
100a20204:     	b.eq	0x100a2024c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x10c>
100a20208:     	rbit	x14, x13
100a2020c:     	clz	x14, x14
100a20210:     	add	x14, x12, x14, lsr #3
100a20214:     	and	x14, x14, x10
100a20218:     	sub	x14, x11, x14, lsl #4
100a2021c:     	ldur	w15, [x14, #-0x10]
100a20220:     	cmp	w19, w15
100a20224:     	b.ne	0x100a20240 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x100>
100a20228:     	ldur	w15, [x14, #-0xc]
100a2022c:     	cmp	w22, w15
100a20230:     	b.ne	0x100a20240 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x100>
100a20234:     	ldurb	w15, [x14, #-0x8]
100a20238:     	cmp	w15, w21
100a2023c:     	b.eq	0x100a202b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x178>
100a20240:     	sub	x14, x13, #0x2
100a20244:     	ands	x13, x14, x13
100a20248:     	b.ne	0x100a20208 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xc8>
100a2024c:     	cmeq.8b	v2, v2, v1
100a20250:     	fmov	x13, d2
100a20254:     	cbnz	x13, 0x100a2027c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x13c>
100a20258:     	add	x9, x9, #0x8
100a2025c:     	add	x12, x12, x9
100a20260:     	and	x12, x12, x10
100a20264:     	ldr	d2, [x11, x12]
100a20268:     	cmeq.8b	v3, v2, v0
100a2026c:     	fmov	x13, d3
100a20270:     	ands	x13, x13, #0x8080808080808080
100a20274:     	b.ne	0x100a20208 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0xc8>
100a20278:     	b	0x100a2024c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x10c>
100a2027c:     	ldr	x9, [x8]
100a20280:     	eor	x10, x9, #0x8000000000000000
100a20284:     	cmp	x9, #0x0
100a20288:     	csinc	x9, x10, xzr, mi
100a2028c:     	cmp	x9, #0x1
100a20290:     	b.eq	0x100a202c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x180>
100a20294:     	cmp	x9, #0x2
100a20298:     	b.ne	0x100a204d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x390>
100a2029c:     	ldp	w23, w1, [x8, #0x8]
100a202a0:     	ldr	w24, [x8, #0x10]
100a202a4:     	cmp	w22, w23
100a202a8:     	b.ne	0x100a203d0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x290>
100a202ac:     	cmp	w21, #0x0
100a202b0:     	csel	w25, w24, w1, ne
100a202b4:     	b	0x100a20450 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x310>
100a202b8:     	ldur	w19, [x14, #-0x4]
100a202bc:     	b	0x100a20474 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x334>
100a202c0:     	ldr	x23, [x8, #0x10]
100a202c4:     	cbz	x23, 0x100a20498 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x358>
100a202c8:     	ldr	x26, [x8, #0x8]
100a202cc:     	lsl	x25, x23, #3
100a202d0:     	mov	x0, x25
100a202d4:     	bl	0x101109b84 <dyld_stub_binder+0x101109b84>
100a202d8:     	cbz	x0, 0x100a204f4 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3b4>
100a202dc:     	mov	x24, x0
100a202e0:     	mov	x1, x26
100a202e4:     	mov	x2, x25
100a202e8:     	bl	0x101109b9c <dyld_stub_binder+0x101109b9c>
100a202ec:     	stp	x24, xzr, [sp]
100a202f0:     	mov	w8, #0x4                ; =4
100a202f4:     	stp	x8, xzr, [sp, #0x10]
100a202f8:     	cbz	x27, 0x100a204ac <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x36c>
100a202fc:     	mov	x26, #0x0               ; =0
100a20300:     	mov	w8, #0x4                ; =4
100a20304:     	mov	w9, #0x1                ; =1
100a20308:     	mov	x24, x27
100a2030c:     	b	0x100a20338 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x1f8>
100a20310:     	ldr	x8, [sp, #0x10]
100a20314:     	rbit	x9, x24
100a20318:     	clz	x9, x9
100a2031c:     	str	w9, [x8, x26]
100a20320:     	str	x25, [sp, #0x18]
100a20324:     	sub	x10, x24, #0x1
100a20328:     	add	x26, x26, #0x4
100a2032c:     	add	x9, x25, #0x1
100a20330:     	ands	x24, x10, x24
100a20334:     	b.eq	0x100a20358 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x218>
100a20338:     	mov	x25, x9
100a2033c:     	sub	x9, x9, #0x1
100a20340:     	ldr	x10, [sp, #0x8]
100a20344:     	cmp	x9, x10
100a20348:     	b.ne	0x100a20314 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x1d4>
100a2034c:     	add	x0, sp, #0x8
100a20350:     	bl	0x1011024dc <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100a20354:     	b	0x100a20310 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x1d0>
100a20358:     	ldp	x24, x0, [sp, #0x8]
100a2035c:     	cbz	x25, 0x100a2037c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x23c>
100a20360:     	mov	x25, #0x0               ; =0
100a20364:     	ldr	w8, [x0, x25, lsl #2]
100a20368:     	cmp	w8, w22
100a2036c:     	b.eq	0x100a20390 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x250>
100a20370:     	add	x25, x25, #0x1
100a20374:     	subs	x26, x26, #0x4
100a20378:     	b.ne	0x100a20364 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x224>
100a2037c:     	mov	x20, x0
100a20380:     	adrp	x0, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a20384:     	add	x0, x0, #0xf18
100a20388:     	bl	0x101101774 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100a2038c:     	brk	#0x1
100a20390:     	cbz	x24, 0x100a20398 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x258>
100a20394:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a20398:     	fmov	d0, x27
100a2039c:     	cnt.8b	v0, v0
100a203a0:     	addv.8b	b0, v0
100a203a4:     	fmov	w3, s0
100a203a8:     	ldrb	w8, [x20, #0x94]
100a203ac:     	tbz	w8, #0x0, 0x100a20414 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x2d4>
100a203b0:     	add	x0, sp, #0x8
100a203b4:     	ldr	x24, [sp]
100a203b8:     	mov	x1, x24
100a203bc:     	mov	x2, x23
100a203c0:     	mov	x4, x25
100a203c4:     	mov	x5, x21
100a203c8:     	bl	0x100ba2798 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100a203cc:     	b	0x100a20430 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x2f0>
100a203d0:     	mov	x0, x20
100a203d4:     	mov	x2, x22
100a203d8:     	mov	x3, x21
100a203dc:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a203e0:     	mov	x25, x0
100a203e4:     	mov	x0, x20
100a203e8:     	mov	x1, x24
100a203ec:     	mov	x2, x22
100a203f0:     	mov	x3, x21
100a203f4:     	bl	0x100a20140 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_>
100a203f8:     	mov	x3, x0
100a203fc:     	mov	x0, x20
100a20400:     	mov	x1, x23
100a20404:     	mov	x2, x25
100a20408:     	bl	0x100a1f5ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E6branchB6_>
100a2040c:     	mov	x25, x0
100a20410:     	b	0x100a20450 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x310>
100a20414:     	add	x0, sp, #0x8
100a20418:     	ldr	x24, [sp]
100a2041c:     	mov	x1, x24
100a20420:     	mov	x2, x23
100a20424:     	mov	x4, x25
100a20428:     	mov	x5, x21
100a2042c:     	bl	0x100b51720 <__RNvNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw14cofactor_words>
100a20430:     	bic	x1, x27, x28
100a20434:     	add	x2, sp, #0x8
100a20438:     	mov	x0, x20
100a2043c:     	bl	0x100a1ef04 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E5tableB6_>
100a20440:     	mov	x25, x0
100a20444:     	cbz	x23, 0x100a20450 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x310>
100a20448:     	mov	x0, x24
100a2044c:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a20450:     	and	w8, w19, #0x1
100a20454:     	eor	w23, w25, w8
100a20458:     	stp	w19, w22, [sp, #0x8]
100a2045c:     	strb	w21, [sp, #0x10]
100a20460:     	add	x0, x20, #0x70
100a20464:     	add	x1, sp, #0x8
100a20468:     	mov	x2, x23
100a2046c:     	bl	0x100aae45c <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapTmmbEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCs23EhFSy3h49_8bumbledb>
100a20470:     	mov	x19, x23
100a20474:     	mov	x0, x19
100a20478:     	ldp	x29, x30, [sp, #0x70]
100a2047c:     	ldp	x20, x19, [sp, #0x60]
100a20480:     	ldp	x22, x21, [sp, #0x50]
100a20484:     	ldp	x24, x23, [sp, #0x40]
100a20488:     	ldp	x26, x25, [sp, #0x30]
100a2048c:     	ldp	x28, x27, [sp, #0x20]
100a20490:     	add	sp, sp, #0x80
100a20494:     	ret
100a20498:     	mov	w24, #0x8               ; =8
100a2049c:     	stp	x24, xzr, [sp]
100a204a0:     	mov	w8, #0x4                ; =4
100a204a4:     	stp	x8, xzr, [sp, #0x10]
100a204a8:     	cbnz	x27, 0x100a202fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x1bc>
100a204ac:     	mov	x24, #0x0               ; =0
100a204b0:     	mov	w20, #0x4               ; =4
100a204b4:     	b	0x100a20380 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x240>
100a204b8:     	adrp	x0, 0x1011bd000 <dyld_stub_binder+0x1011bd000>
100a204bc:     	add	x0, x0, #0xe7a
100a204c0:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a204c4:     	add	x2, x2, #0xee8
100a204c8:     	mov	w1, #0x2c               ; =44
100a204cc:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a204d0:     	adrp	x0, 0x10125b000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x1590>
100a204d4:     	add	x0, x0, #0x231
100a204d8:     	adrp	x2, 0x10136f000 <dyld_stub_binder+0x10136f000>
100a204dc:     	add	x2, x2, #0xf00
100a204e0:     	mov	w1, #0x28               ; =40
100a204e4:     	bl	0x1011016c8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100a204e8:     	adrp	x2, 0x101335000 <dyld_stub_binder+0x101335000>
100a204ec:     	add	x2, x2, #0xb78
100a204f0:     	bl	0x1011016dc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100a204f4:     	mov	w0, #0x8                ; =8
100a204f8:     	mov	x1, x25
100a204fc:     	bl	0x101100ee4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100a20500:     	mov	x19, x0
100a20504:     	b	0x100a2052c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3ec>
100a20508:     	mov	x19, x0
100a2050c:     	ldr	x8, [sp, #0x8]
100a20510:     	cbz	x8, 0x100a2052c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3ec>
100a20514:     	ldr	x0, [sp, #0x10]
100a20518:     	b	0x100a20528 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3e8>
100a2051c:     	mov	x19, x0
100a20520:     	cbz	x24, 0x100a2052c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3ec>
100a20524:     	mov	x0, x20
100a20528:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a2052c:     	cbz	x23, 0x100a20538 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm9_E8cofactorB6_+0x3f8>
100a20530:     	ldr	x0, [sp]
100a20534:     	bl	0x101109ab8 <dyld_stub_binder+0x101109ab8>
100a20538:     	mov	x0, x19
100a2053c:     	bl	0x101109908 <dyld_stub_binder+0x101109908>
