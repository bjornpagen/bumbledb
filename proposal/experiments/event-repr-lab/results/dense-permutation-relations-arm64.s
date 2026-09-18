
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001005a1fe4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense>:
1005a1fe4:     	sub	sp, sp, #0xe0
1005a1fe8:     	stp	d15, d14, [sp, #0x40]
1005a1fec:     	stp	d13, d12, [sp, #0x50]
1005a1ff0:     	stp	d11, d10, [sp, #0x60]
1005a1ff4:     	stp	d9, d8, [sp, #0x70]
1005a1ff8:     	stp	x28, x27, [sp, #0x80]
1005a1ffc:     	stp	x26, x25, [sp, #0x90]
1005a2000:     	stp	x24, x23, [sp, #0xa0]
1005a2004:     	stp	x22, x21, [sp, #0xb0]
1005a2008:     	stp	x20, x19, [sp, #0xc0]
1005a200c:     	stp	x29, x30, [sp, #0xd0]
1005a2010:     	add	x29, sp, #0xd0
1005a2014:     	ldr	x8, [x0, #0x10]
1005a2018:     	and	x9, x8, #0x3f
1005a201c:     	mov	w10, #0x1               ; =1
1005a2020:     	lsl	x8, x10, x8
1005a2024:     	lsr	x8, x8, #6
1005a2028:     	cmp	x9, #0x6
1005a202c:     	cinc	x8, x8, lo
1005a2030:     	stp	x2, x8, [sp, #0x30]
1005a2034:     	cmp	x2, x8
1005a2038:     	b.ne	0x1005a2988 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9a4>
1005a203c:     	ldr	x8, [x0, #0x28]
1005a2040:     	cbz	x8, 0x1005a290c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
1005a2044:     	mov	x19, x1
1005a2048:     	ldr	x9, [x0, #0x20]
1005a204c:     	add	x10, x9, x8, lsl #3
1005a2050:     	lsl	x8, x2, #3
1005a2054:     	add	x11, x1, x8
1005a2058:     	sub	x12, x8, #0x8
1005a205c:     	lsr	x8, x12, #3
1005a2060:     	add	x13, x8, #0x1
1005a2064:     	and	x14, x13, #0x3ffffffffffffff8
1005a2068:     	mov	w16, #0x1               ; =1
1005a206c:     	mov	x8, #0x100000000        ; =4294967296
1005a2070:     	str	x8, [sp, #0x20]
1005a2074:     	mov	x8, #0x2                ; =2
1005a2078:     	movk	x8, #0x3, lsl #32
1005a207c:     	str	x8, [sp, #0x18]
1005a2080:     	mov	x8, #0x4                ; =4
1005a2084:     	movk	x8, #0x5, lsl #32
1005a2088:     	fmov	d2, x8
1005a208c:     	mov	x8, #0x6                ; =6
1005a2090:     	movk	x8, #0x7, lsl #32
1005a2094:     	fmov	d3, x8
1005a2098:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a209c:     	ldr	q0, [x8, #0x2c0]
1005a20a0:     	str	q0, [sp]
1005a20a4:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a20a8:     	ldr	q5, [x8, #0x680]
1005a20ac:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a20b0:     	ldr	q6, [x8, #0x690]
1005a20b4:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a20b8:     	ldr	q7, [x8, #0x3c0]
1005a20bc:     	mov	x8, #0x8                ; =8
1005a20c0:     	movk	x8, #0x9, lsl #32
1005a20c4:     	fmov	d16, x8
1005a20c8:     	mov	x8, #0xa                ; =10
1005a20cc:     	movk	x8, #0xb, lsl #32
1005a20d0:     	fmov	d17, x8
1005a20d4:     	mov	x8, #0xc                ; =12
1005a20d8:     	movk	x8, #0xd, lsl #32
1005a20dc:     	fmov	d18, x8
1005a20e0:     	mov	x8, #0xe                ; =14
1005a20e4:     	movk	x8, #0xf, lsl #32
1005a20e8:     	fmov	d19, x8
1005a20ec:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a20f0:     	ldr	q20, [x8, #0x3d0]
1005a20f4:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a20f8:     	ldr	q21, [x8, #0x6a0]
1005a20fc:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2100:     	ldr	q22, [x8, #0x6b0]
1005a2104:     	adrp	x8, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2108:     	ldr	q23, [x8, #0x6c0]
1005a210c:     	mov	x8, #0x10               ; =16
1005a2110:     	movk	x8, #0x11, lsl #32
1005a2114:     	fmov	d24, x8
1005a2118:     	mov	x8, #0x12               ; =18
1005a211c:     	movk	x8, #0x13, lsl #32
1005a2120:     	fmov	d25, x8
1005a2124:     	mov	x8, #0x14               ; =20
1005a2128:     	movk	x8, #0x15, lsl #32
1005a212c:     	fmov	d26, x8
1005a2130:     	mov	x8, #0x16               ; =22
1005a2134:     	movk	x8, #0x17, lsl #32
1005a2138:     	fmov	d27, x8
1005a213c:     	add	x8, x1, x14, lsl #3
1005a2140:     	str	x8, [sp, #0x28]
1005a2144:     	b	0x1005a2154 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x170>
1005a2148:     	add	x9, x9, #0x8
1005a214c:     	cmp	x9, x10
1005a2150:     	b.eq	0x1005a290c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x928>
1005a2154:     	ldp	w1, w15, [x9]
1005a2158:     	cmp	w15, #0x6
1005a215c:     	b.hs	0x1005a227c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x298>
1005a2160:     	mov	x3, #0x0                ; =0
1005a2164:     	mov	x8, #0x0                ; =0
1005a2168:     	and	w14, w1, #0x1f
1005a216c:     	lsl	w0, w16, w1
1005a2170:     	mov	w1, #-0x1               ; =-1
1005a2174:     	lsr	w4, w1, w15
1005a2178:     	lsl	x6, x16, x3
1005a217c:     	orr	x6, x6, x8
1005a2180:     	tst	w4, #0x1
1005a2184:     	csel	x4, x8, x6, eq
1005a2188:     	tst	w0, w3
1005a218c:     	add	x3, x3, #0x1
1005a2190:     	csel	x8, x8, x4, eq
1005a2194:     	sub	w1, w1, #0x1
1005a2198:     	cmp	x3, #0x40
1005a219c:     	b.ne	0x1005a2174 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x190>
1005a21a0:     	cbz	x2, 0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a21a4:     	mov	w17, #-0x1              ; =-1
1005a21a8:     	lsl	w14, w17, w14
1005a21ac:     	lsl	w15, w16, w15
1005a21b0:     	add	w14, w14, w15
1005a21b4:     	and	w15, w14, #0x3f
1005a21b8:     	mov	x14, x19
1005a21bc:     	cmp	x12, #0x38
1005a21c0:     	b.lo	0x1005a2250 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
1005a21c4:     	dup.2d	v28, x15
1005a21c8:     	dup.2d	v29, x8
1005a21cc:     	neg.2d	v30, v28
1005a21d0:     	add	x0, x19, #0x20
1005a21d4:     	and	x1, x13, #0x3ffffffffffffff8
1005a21d8:     	ldp	q31, q8, [x0, #-0x20]
1005a21dc:     	ldp	q9, q10, [x0]
1005a21e0:     	ushl.2d	v11, v31, v30
1005a21e4:     	ushl.2d	v12, v8, v30
1005a21e8:     	ushl.2d	v13, v9, v30
1005a21ec:     	ushl.2d	v14, v10, v30
1005a21f0:     	eor.16b	v11, v11, v31
1005a21f4:     	eor.16b	v12, v12, v8
1005a21f8:     	eor.16b	v13, v13, v9
1005a21fc:     	eor.16b	v14, v14, v10
1005a2200:     	and.16b	v11, v11, v29
1005a2204:     	and.16b	v12, v12, v29
1005a2208:     	and.16b	v13, v13, v29
1005a220c:     	and.16b	v14, v14, v29
1005a2210:     	ushl.2d	v15, v11, v28
1005a2214:     	ushl.2d	v0, v12, v28
1005a2218:     	ushl.2d	v4, v13, v28
1005a221c:     	ushl.2d	v1, v14, v28
1005a2220:     	eor3.16b	v31, v31, v15, v11
1005a2224:     	eor3.16b	v0, v8, v0, v12
1005a2228:     	eor3.16b	v4, v9, v4, v13
1005a222c:     	stp	q31, q0, [x0, #-0x20]
1005a2230:     	eor3.16b	v0, v10, v1, v14
1005a2234:     	stp	q4, q0, [x0], #0x40
1005a2238:     	subs	x1, x1, #0x8
1005a223c:     	b.ne	0x1005a21d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x1f4>
1005a2240:     	ldr	x14, [sp, #0x28]
1005a2244:     	and	x17, x13, #0x3ffffffffffffff8
1005a2248:     	cmp	x13, x17
1005a224c:     	b.eq	0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a2250:     	ldr	x17, [x14]
1005a2254:     	lsr	x0, x17, x15
1005a2258:     	eor	x0, x0, x17
1005a225c:     	and	x0, x0, x8
1005a2260:     	lsl	x1, x0, x15
1005a2264:     	eor	x17, x17, x0
1005a2268:     	eor	x17, x17, x1
1005a226c:     	str	x17, [x14], #0x8
1005a2270:     	cmp	x14, x11
1005a2274:     	b.ne	0x1005a2250 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x26c>
1005a2278:     	b	0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a227c:     	cmp	w1, #0x6
1005a2280:     	b.hs	0x1005a28b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8cc>
1005a2284:     	add	w14, w15, #0x3a
1005a2288:     	and	w8, w14, #0x3f
1005a228c:     	cmp	w8, #0x3f
1005a2290:     	b.eq	0x1005a2950 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x96c>
1005a2294:     	mov	w15, #0x2               ; =2
1005a2298:     	lsl	x28, x15, x14
1005a229c:     	add	x15, x8, #0x1
1005a22a0:     	lsr	x20, x2, x15
1005a22a4:     	mov	x15, #0xfffffffffffffff ; =1152921504606846975
1005a22a8:     	add	x15, x28, x15
1005a22ac:     	tst	x15, x2
1005a22b0:     	cset	w22, ne
1005a22b4:     	cinc	x15, x20, ne
1005a22b8:     	cbz	x15, 0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a22bc:     	lsl	x6, x16, x14
1005a22c0:     	mov	w14, #0x8               ; =8
1005a22c4:     	lsl	x14, x14, x8
1005a22c8:     	lsr	x14, x14, #3
1005a22cc:     	subs	x0, x28, x6
1005a22d0:     	cmp	x0, x14
1005a22d4:     	csel	x3, x0, x14, lo
1005a22d8:     	cmp	x28, x6
1005a22dc:     	b.lo	0x1005a2968 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x984>
1005a22e0:     	mov	x0, #0x0                ; =0
1005a22e4:     	b.eq	0x1005a279c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
1005a22e8:     	lsl	x4, x16, x1
1005a22ec:     	dup.2s	v28, w4
1005a22f0:     	ldp	d1, d0, [sp, #0x18]
1005a22f4:     	and.8b	v0, v28, v0
1005a22f8:     	and.8b	v1, v28, v1
1005a22fc:     	and.8b	v4, v28, v2
1005a2300:     	and.8b	v29, v28, v3
1005a2304:     	cmeq.2s	v0, v0, #0
1005a2308:     	ushll.2d	v0, v0, #0x0
1005a230c:     	cmeq.2s	v1, v1, #0
1005a2310:     	ushll.2d	v1, v1, #0x0
1005a2314:     	cmeq.2s	v4, v4, #0
1005a2318:     	ushll.2d	v4, v4, #0x0
1005a231c:     	cmeq.2s	v29, v29, #0
1005a2320:     	ushll.2d	v29, v29, #0x0
1005a2324:     	ldr	q30, [sp]
1005a2328:     	and.16b	v0, v0, v30
1005a232c:     	and.16b	v1, v1, v5
1005a2330:     	and.16b	v4, v4, v6
1005a2334:     	and.16b	v29, v29, v7
1005a2338:     	and.8b	v30, v28, v16
1005a233c:     	and.8b	v31, v28, v17
1005a2340:     	and.8b	v8, v28, v18
1005a2344:     	and.8b	v9, v28, v19
1005a2348:     	cmeq.2s	v30, v30, #0
1005a234c:     	ushll.2d	v30, v30, #0x0
1005a2350:     	cmeq.2s	v31, v31, #0
1005a2354:     	ushll.2d	v31, v31, #0x0
1005a2358:     	cmeq.2s	v8, v8, #0
1005a235c:     	ushll.2d	v8, v8, #0x0
1005a2360:     	cmeq.2s	v9, v9, #0
1005a2364:     	ushll.2d	v9, v9, #0x0
1005a2368:     	and.16b	v30, v30, v20
1005a236c:     	and.16b	v31, v31, v21
1005a2370:     	and.16b	v8, v8, v22
1005a2374:     	and.16b	v9, v9, v23
1005a2378:     	orr.16b	v0, v30, v0
1005a237c:     	orr.16b	v1, v31, v1
1005a2380:     	orr.16b	v4, v8, v4
1005a2384:     	orr.16b	v29, v9, v29
1005a2388:     	and.8b	v30, v28, v24
1005a238c:     	and.8b	v31, v28, v25
1005a2390:     	and.8b	v8, v28, v26
1005a2394:     	and.8b	v9, v28, v27
1005a2398:     	cmeq.2s	v30, v30, #0
1005a239c:     	ushll.2d	v30, v30, #0x0
1005a23a0:     	cmeq.2s	v31, v31, #0
1005a23a4:     	ushll.2d	v31, v31, #0x0
1005a23a8:     	cmeq.2s	v8, v8, #0
1005a23ac:     	ushll.2d	v8, v8, #0x0
1005a23b0:     	cmeq.2s	v9, v9, #0
1005a23b4:     	ushll.2d	v9, v9, #0x0
1005a23b8:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a23bc:     	ldr	q10, [x14, #0x6d0]
1005a23c0:     	and.16b	v30, v30, v10
1005a23c4:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a23c8:     	ldr	q10, [x14, #0x6e0]
1005a23cc:     	and.16b	v10, v31, v10
1005a23d0:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a23d4:     	ldr	q31, [x14, #0x6f0]
1005a23d8:     	and.16b	v11, v8, v31
1005a23dc:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a23e0:     	ldr	q31, [x14, #0x700]
1005a23e4:     	and.16b	v9, v9, v31
1005a23e8:     	mov	x14, #0x18              ; =24
1005a23ec:     	movk	x14, #0x19, lsl #32
1005a23f0:     	fmov	d31, x14
1005a23f4:     	and.8b	v31, v28, v31
1005a23f8:     	mov	x14, #0x1a              ; =26
1005a23fc:     	movk	x14, #0x1b, lsl #32
1005a2400:     	fmov	d8, x14
1005a2404:     	and.8b	v8, v28, v8
1005a2408:     	mov	x14, #0x1c              ; =28
1005a240c:     	movk	x14, #0x1d, lsl #32
1005a2410:     	fmov	d12, x14
1005a2414:     	and.8b	v12, v28, v12
1005a2418:     	mov	x14, #0x1e              ; =30
1005a241c:     	movk	x14, #0x1f, lsl #32
1005a2420:     	fmov	d13, x14
1005a2424:     	and.8b	v13, v28, v13
1005a2428:     	cmeq.2s	v31, v31, #0
1005a242c:     	ushll.2d	v31, v31, #0x0
1005a2430:     	cmeq.2s	v8, v8, #0
1005a2434:     	ushll.2d	v8, v8, #0x0
1005a2438:     	cmeq.2s	v12, v12, #0
1005a243c:     	ushll.2d	v12, v12, #0x0
1005a2440:     	cmeq.2s	v13, v13, #0
1005a2444:     	ushll.2d	v13, v13, #0x0
1005a2448:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a244c:     	ldr	q14, [x14, #0x710]
1005a2450:     	and.16b	v31, v31, v14
1005a2454:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2458:     	ldr	q14, [x14, #0x720]
1005a245c:     	and.16b	v8, v8, v14
1005a2460:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2464:     	ldr	q14, [x14, #0x730]
1005a2468:     	and.16b	v12, v12, v14
1005a246c:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2470:     	ldr	q14, [x14, #0x740]
1005a2474:     	and.16b	v13, v13, v14
1005a2478:     	orr.16b	v30, v31, v30
1005a247c:     	orr.16b	v31, v30, v0
1005a2480:     	orr.16b	v0, v8, v10
1005a2484:     	orr.16b	v8, v0, v1
1005a2488:     	orr.16b	v0, v12, v11
1005a248c:     	orr.16b	v30, v0, v4
1005a2490:     	orr.16b	v0, v13, v9
1005a2494:     	orr.16b	v29, v0, v29
1005a2498:     	mov	x14, #0x20              ; =32
1005a249c:     	movk	x14, #0x21, lsl #32
1005a24a0:     	fmov	d0, x14
1005a24a4:     	and.8b	v0, v28, v0
1005a24a8:     	mov	x14, #0x22              ; =34
1005a24ac:     	movk	x14, #0x23, lsl #32
1005a24b0:     	fmov	d1, x14
1005a24b4:     	and.8b	v1, v28, v1
1005a24b8:     	mov	x14, #0x24              ; =36
1005a24bc:     	movk	x14, #0x25, lsl #32
1005a24c0:     	fmov	d4, x14
1005a24c4:     	and.8b	v4, v28, v4
1005a24c8:     	mov	x14, #0x26              ; =38
1005a24cc:     	movk	x14, #0x27, lsl #32
1005a24d0:     	fmov	d9, x14
1005a24d4:     	and.8b	v9, v28, v9
1005a24d8:     	cmeq.2s	v0, v0, #0
1005a24dc:     	sshll.2d	v0, v0, #0x0
1005a24e0:     	cmeq.2s	v1, v1, #0
1005a24e4:     	sshll.2d	v1, v1, #0x0
1005a24e8:     	cmeq.2s	v4, v4, #0
1005a24ec:     	sshll.2d	v4, v4, #0x0
1005a24f0:     	cmeq.2s	v9, v9, #0
1005a24f4:     	sshll.2d	v9, v9, #0x0
1005a24f8:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a24fc:     	ldr	q10, [x14, #0x750]
1005a2500:     	and.16b	v0, v0, v10
1005a2504:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2508:     	ldr	q10, [x14, #0x760]
1005a250c:     	and.16b	v1, v1, v10
1005a2510:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2514:     	ldr	q10, [x14, #0x770]
1005a2518:     	and.16b	v4, v4, v10
1005a251c:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2520:     	ldr	q10, [x14, #0x780]
1005a2524:     	and.16b	v9, v9, v10
1005a2528:     	mov	x14, #0x28              ; =40
1005a252c:     	movk	x14, #0x29, lsl #32
1005a2530:     	fmov	d10, x14
1005a2534:     	and.8b	v10, v28, v10
1005a2538:     	mov	x14, #0x2a              ; =42
1005a253c:     	movk	x14, #0x2b, lsl #32
1005a2540:     	fmov	d11, x14
1005a2544:     	and.8b	v11, v28, v11
1005a2548:     	mov	x14, #0x2c              ; =44
1005a254c:     	movk	x14, #0x2d, lsl #32
1005a2550:     	fmov	d12, x14
1005a2554:     	and.8b	v12, v28, v12
1005a2558:     	mov	x14, #0x2e              ; =46
1005a255c:     	movk	x14, #0x2f, lsl #32
1005a2560:     	fmov	d13, x14
1005a2564:     	and.8b	v13, v28, v13
1005a2568:     	cmeq.2s	v10, v10, #0
1005a256c:     	sshll.2d	v10, v10, #0x0
1005a2570:     	cmeq.2s	v11, v11, #0
1005a2574:     	sshll.2d	v11, v11, #0x0
1005a2578:     	cmeq.2s	v12, v12, #0
1005a257c:     	sshll.2d	v12, v12, #0x0
1005a2580:     	cmeq.2s	v13, v13, #0
1005a2584:     	sshll.2d	v13, v13, #0x0
1005a2588:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a258c:     	ldr	q14, [x14, #0x790]
1005a2590:     	and.16b	v10, v10, v14
1005a2594:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2598:     	ldr	q14, [x14, #0x7a0]
1005a259c:     	and.16b	v11, v11, v14
1005a25a0:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a25a4:     	ldr	q14, [x14, #0x7b0]
1005a25a8:     	and.16b	v12, v12, v14
1005a25ac:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a25b0:     	ldr	q14, [x14, #0x7c0]
1005a25b4:     	and.16b	v13, v13, v14
1005a25b8:     	orr.16b	v0, v10, v0
1005a25bc:     	orr.16b	v1, v11, v1
1005a25c0:     	orr.16b	v4, v12, v4
1005a25c4:     	orr.16b	v9, v13, v9
1005a25c8:     	mov	x14, #0x30              ; =48
1005a25cc:     	movk	x14, #0x31, lsl #32
1005a25d0:     	fmov	d10, x14
1005a25d4:     	and.8b	v10, v28, v10
1005a25d8:     	mov	x14, #0x32              ; =50
1005a25dc:     	movk	x14, #0x33, lsl #32
1005a25e0:     	fmov	d11, x14
1005a25e4:     	and.8b	v11, v28, v11
1005a25e8:     	mov	x14, #0x34              ; =52
1005a25ec:     	movk	x14, #0x35, lsl #32
1005a25f0:     	fmov	d12, x14
1005a25f4:     	and.8b	v12, v28, v12
1005a25f8:     	mov	x14, #0x36              ; =54
1005a25fc:     	movk	x14, #0x37, lsl #32
1005a2600:     	fmov	d13, x14
1005a2604:     	and.8b	v13, v28, v13
1005a2608:     	cmeq.2s	v10, v10, #0
1005a260c:     	sshll.2d	v10, v10, #0x0
1005a2610:     	cmeq.2s	v11, v11, #0
1005a2614:     	sshll.2d	v11, v11, #0x0
1005a2618:     	cmeq.2s	v12, v12, #0
1005a261c:     	sshll.2d	v12, v12, #0x0
1005a2620:     	cmeq.2s	v13, v13, #0
1005a2624:     	sshll.2d	v13, v13, #0x0
1005a2628:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a262c:     	ldr	q14, [x14, #0x7d0]
1005a2630:     	and.16b	v10, v10, v14
1005a2634:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2638:     	ldr	q14, [x14, #0x7e0]
1005a263c:     	and.16b	v11, v11, v14
1005a2640:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2644:     	ldr	q14, [x14, #0x7f0]
1005a2648:     	and.16b	v12, v12, v14
1005a264c:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a2650:     	ldr	q14, [x14, #0x800]
1005a2654:     	and.16b	v13, v13, v14
1005a2658:     	orr.16b	v0, v10, v0
1005a265c:     	orr.16b	v0, v0, v31
1005a2660:     	orr.16b	v1, v11, v1
1005a2664:     	orr.16b	v1, v1, v8
1005a2668:     	orr.16b	v4, v12, v4
1005a266c:     	mov	x14, #0x38              ; =56
1005a2670:     	movk	x14, #0x39, lsl #32
1005a2674:     	fmov	d31, x14
1005a2678:     	orr.16b	v4, v4, v30
1005a267c:     	mov	x14, #0x3a              ; =58
1005a2680:     	movk	x14, #0x3b, lsl #32
1005a2684:     	fmov	d30, x14
1005a2688:     	orr.16b	v8, v13, v9
1005a268c:     	mov	x14, #0x3c              ; =60
1005a2690:     	movk	x14, #0x3d, lsl #32
1005a2694:     	fmov	d9, x14
1005a2698:     	orr.16b	v29, v8, v29
1005a269c:     	mov	x14, #0x3e              ; =62
1005a26a0:     	movk	x14, #0x3f, lsl #32
1005a26a4:     	fmov	d8, x14
1005a26a8:     	and.8b	v31, v28, v31
1005a26ac:     	and.8b	v30, v28, v30
1005a26b0:     	and.8b	v9, v28, v9
1005a26b4:     	and.8b	v28, v28, v8
1005a26b8:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a26bc:     	ldr	q8, [x14, #0x810]
1005a26c0:     	cmeq.2s	v31, v31, #0
1005a26c4:     	sshll.2d	v31, v31, #0x0
1005a26c8:     	and.16b	v31, v31, v8
1005a26cc:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a26d0:     	ldr	q8, [x14, #0x820]
1005a26d4:     	cmeq.2s	v30, v30, #0
1005a26d8:     	sshll.2d	v30, v30, #0x0
1005a26dc:     	and.16b	v30, v30, v8
1005a26e0:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a26e4:     	ldr	q8, [x14, #0x830]
1005a26e8:     	cmeq.2s	v9, v9, #0
1005a26ec:     	sshll.2d	v9, v9, #0x0
1005a26f0:     	and.16b	v8, v9, v8
1005a26f4:     	adrp	x14, 0x100bdb000 <GCC_except_table7561+0x70>
1005a26f8:     	ldr	q9, [x14, #0x840]
1005a26fc:     	cmeq.2s	v28, v28, #0
1005a2700:     	sshll.2d	v28, v28, #0x0
1005a2704:     	and.16b	v28, v28, v9
1005a2708:     	orr.16b	v0, v31, v0
1005a270c:     	orr.16b	v1, v30, v1
1005a2710:     	orr.16b	v4, v8, v4
1005a2714:     	orr.16b	v28, v28, v29
1005a2718:     	orr.16b	v0, v1, v0
1005a271c:     	orr.16b	v0, v4, v0
1005a2720:     	orr.16b	v0, v28, v0
1005a2724:     	mov	d1, v0[1]
1005a2728:     	orr.8b	v0, v0, v1
1005a272c:     	fmov	x30, d0
1005a2730:     	cmp	x3, #0x1
1005a2734:     	csinc	x23, x3, xzr, hi
1005a2738:     	mov	w14, #0x10              ; =16
1005a273c:     	lsl	x14, x14, x8
1005a2740:     	add	x1, x20, x22
1005a2744:     	sub	x1, x1, #0x1
1005a2748:     	madd	x1, x14, x1, x19
1005a274c:     	add	x1, x1, x23, lsl #3
1005a2750:     	mov	w17, #0x8               ; =8
1005a2754:     	lsl	x8, x17, x8
1005a2758:     	add	x7, x19, x8
1005a275c:     	add	x8, x1, x8
1005a2760:     	cmp	x19, x8
1005a2764:     	ccmp	x7, x1, #0x2, lo
1005a2768:     	ccmp	x14, #0x0, #0x8, hs
1005a276c:     	cset	w20, mi
1005a2770:     	and	x22, x23, #0x1ffffffffffffffc
1005a2774:     	lsl	x8, x6, #3
1005a2778:     	add	x14, x19, #0x10
1005a277c:     	add	x21, x14, x8
1005a2780:     	lsl	x25, x28, #3
1005a2784:     	add	x7, x19, x8
1005a2788:     	mov	x1, x19
1005a278c:     	add	x27, x19, #0x10
1005a2790:     	dup.2d	v28, x4
1005a2794:     	dup.2d	v29, x30
1005a2798:     	b	0x1005a27d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7f4>
1005a279c:     	adds	x8, x28, x0
1005a27a0:     	b.hs	0x1005a293c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
1005a27a4:     	cmp	x8, x2
1005a27a8:     	b.hi	0x1005a293c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x958>
1005a27ac:     	mov	x0, x8
1005a27b0:     	subs	x15, x15, #0x1
1005a27b4:     	b.ne	0x1005a279c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7b8>
1005a27b8:     	b	0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a27bc:     	add	x21, x21, x25
1005a27c0:     	add	x27, x27, x25
1005a27c4:     	add	x7, x7, x25
1005a27c8:     	add	x1, x1, x25
1005a27cc:     	mov	x0, x8
1005a27d0:     	sub	x15, x15, #0x1
1005a27d4:     	cbz	x15, 0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a27d8:     	adds	x8, x0, x28
1005a27dc:     	b.hs	0x1005a2940 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
1005a27e0:     	cmp	x8, x2
1005a27e4:     	b.hi	0x1005a2940 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x95c>
1005a27e8:     	cmp	x3, #0x4
1005a27ec:     	cset	w14, lo
1005a27f0:     	orr	w14, w14, w20
1005a27f4:     	tbz	w14, #0x0, 0x1005a2800 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x81c>
1005a27f8:     	mov	x6, #0x0                ; =0
1005a27fc:     	b	0x1005a286c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x888>
1005a2800:     	mov	x0, x27
1005a2804:     	mov	x6, x21
1005a2808:     	and	x14, x23, #0x1ffffffffffffffc
1005a280c:     	ldp	q0, q1, [x0, #-0x10]
1005a2810:     	neg.2d	v4, v28
1005a2814:     	ushl.2d	v30, v0, v4
1005a2818:     	ushl.2d	v4, v1, v4
1005a281c:     	ldp	q31, q8, [x6, #-0x10]
1005a2820:     	eor.16b	v30, v30, v31
1005a2824:     	eor.16b	v4, v4, v8
1005a2828:     	and.16b	v30, v30, v29
1005a282c:     	and.16b	v4, v4, v29
1005a2830:     	ushl.2d	v9, v30, v28
1005a2834:     	ushl.2d	v10, v4, v28
1005a2838:     	eor.16b	v0, v9, v0
1005a283c:     	eor.16b	v1, v10, v1
1005a2840:     	stp	q0, q1, [x0, #-0x10]
1005a2844:     	eor.16b	v0, v30, v31
1005a2848:     	eor.16b	v1, v4, v8
1005a284c:     	stp	q0, q1, [x6, #-0x10]
1005a2850:     	add	x6, x6, #0x20
1005a2854:     	add	x0, x0, #0x20
1005a2858:     	subs	x14, x14, #0x4
1005a285c:     	b.ne	0x1005a280c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x828>
1005a2860:     	and	x6, x23, #0x1ffffffffffffffc
1005a2864:     	cmp	x3, x22
1005a2868:     	b.eq	0x1005a27bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
1005a286c:     	lsl	x0, x6, #3
1005a2870:     	add	x14, x7, x0
1005a2874:     	add	x0, x1, x0
1005a2878:     	sub	x6, x23, x6
1005a287c:     	ldr	x5, [x0]
1005a2880:     	lsr	x24, x5, x4
1005a2884:     	ldr	x26, [x14]
1005a2888:     	eor	x24, x24, x26
1005a288c:     	and	x24, x24, x30
1005a2890:     	lsl	x17, x24, x4
1005a2894:     	eor	x17, x17, x5
1005a2898:     	str	x17, [x0], #0x8
1005a289c:     	eor	x17, x24, x26
1005a28a0:     	str	x17, [x14], #0x8
1005a28a4:     	subs	x6, x6, #0x1
1005a28a8:     	b.ne	0x1005a287c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x898>
1005a28ac:     	b	0x1005a27bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x7d8>
1005a28b0:     	cbz	x2, 0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a28b4:     	mov	x8, #0x0                ; =0
1005a28b8:     	add	w14, w1, #0x3a
1005a28bc:     	lsl	x14, x16, x14
1005a28c0:     	add	w15, w15, #0x3a
1005a28c4:     	lsl	x15, x16, x15
1005a28c8:     	eor	x1, x15, x14
1005a28cc:     	b	0x1005a28dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8f8>
1005a28d0:     	add	x8, x8, #0x1
1005a28d4:     	cmp	x2, x8
1005a28d8:     	b.eq	0x1005a2148 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x164>
1005a28dc:     	tst	x8, x14
1005a28e0:     	b.eq	0x1005a28d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1005a28e4:     	and	x17, x8, x15
1005a28e8:     	cbnz	x17, 0x1005a28d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1005a28ec:     	eor	x0, x1, x8
1005a28f0:     	cmp	x0, x2
1005a28f4:     	b.hs	0x1005a29a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9c0>
1005a28f8:     	ldr	x17, [x19, x8, lsl #3]
1005a28fc:     	ldr	x3, [x19, x0, lsl #3]
1005a2900:     	str	x3, [x19, x8, lsl #3]
1005a2904:     	str	x17, [x19, x0, lsl #3]
1005a2908:     	b	0x1005a28d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x8ec>
1005a290c:     	ldp	x29, x30, [sp, #0xd0]
1005a2910:     	ldp	x20, x19, [sp, #0xc0]
1005a2914:     	ldp	x22, x21, [sp, #0xb0]
1005a2918:     	ldp	x24, x23, [sp, #0xa0]
1005a291c:     	ldp	x26, x25, [sp, #0x90]
1005a2920:     	ldp	x28, x27, [sp, #0x80]
1005a2924:     	ldp	d9, d8, [sp, #0x70]
1005a2928:     	ldp	d11, d10, [sp, #0x60]
1005a292c:     	ldp	d13, d12, [sp, #0x50]
1005a2930:     	ldp	d15, d14, [sp, #0x40]
1005a2934:     	add	sp, sp, #0xe0
1005a2938:     	ret
1005a293c:     	add	x8, x28, x0
1005a2940:     	adrp	x3, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a2944:     	add	x3, x3, #0x3a0
1005a2948:     	mov	x1, x8
1005a294c:     	bl	0x100b6d654 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
1005a2950:     	adrp	x0, 0x100c02000 <dyld_stub_binder+0x100c02000>
1005a2954:     	add	x0, x0, #0xbf2
1005a2958:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a295c:     	add	x2, x2, #0x5f8
1005a2960:     	mov	w1, #0x1b               ; =27
1005a2964:     	bl	0x100b6d708 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1005a2968:     	cmp	x28, x2
1005a296c:     	b.hi	0x1005a29b8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense+0x9d4>
1005a2970:     	adrp	x0, 0x100c9f000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0x1318>
1005a2974:     	add	x0, x0, #0x4db
1005a2978:     	adrp	x2, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a297c:     	add	x2, x2, #0x388
1005a2980:     	mov	w1, #0x13               ; =19
1005a2984:     	bl	0x100b6d5b4 <__RNvNtCs4sDCw1iE1MS_4core9panicking9panic_fmt>
1005a2988:     	adrp	x5, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a298c:     	add	x5, x5, #0x358
1005a2990:     	add	x1, sp, #0x30
1005a2994:     	add	x2, sp, #0x38
1005a2998:     	mov	w0, #0x0                ; =0
1005a299c:     	mov	x3, #0x0                ; =0
1005a29a0:     	bl	0x100b6d5f0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1005a29a4:     	adrp	x8, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a29a8:     	add	x8, x8, #0x370
1005a29ac:     	mov	x1, x2
1005a29b0:     	mov	x2, x8
1005a29b4:     	bl	0x100b6d71c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1005a29b8:     	mov	x0, #0x0                ; =0
1005a29bc:     	mov	x8, x28
1005a29c0:     	adrp	x3, 0x100d8e000 <dyld_stub_binder+0x100d8e000>
1005a29c4:     	add	x3, x3, #0x3a0
1005a29c8:     	mov	x1, x8
1005a29cc:     	bl	0x100b6d654 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
