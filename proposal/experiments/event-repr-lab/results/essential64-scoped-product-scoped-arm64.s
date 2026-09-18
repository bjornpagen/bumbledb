
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001010fc064 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_>:
1010fc064:     	sub	sp, sp, #0x130
1010fc068:     	stp	x28, x27, [sp, #0xd0]
1010fc06c:     	stp	x26, x25, [sp, #0xe0]
1010fc070:     	stp	x24, x23, [sp, #0xf0]
1010fc074:     	stp	x22, x21, [sp, #0x100]
1010fc078:     	stp	x20, x19, [sp, #0x110]
1010fc07c:     	stp	x29, x30, [sp, #0x120]
1010fc080:     	add	x29, sp, #0x120
1010fc084:     	str	x7, [sp, #0x28]
1010fc088:     	mov	x21, x6
1010fc08c:     	mov	x28, x5
1010fc090:     	mov	x26, x4
1010fc094:     	mov	x20, x3
1010fc098:     	mov	x23, x2
1010fc09c:     	mov	x19, x0
1010fc0a0:     	lsr	x24, x1, #1
1010fc0a4:     	tbnz	w1, #0x0, 0x1010fc0c8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x64>
1010fc0a8:     	ldr	x22, [x29, #0x18]
1010fc0ac:     	lsr	x25, x26, #1
1010fc0b0:     	tbnz	w26, #0x0, 0x1010fc0ec <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x88>
1010fc0b4:     	ldr	x26, [x29, #0x10]
1010fc0b8:     	ldr	w27, [x19, #0x128]
1010fc0bc:     	cmp	w27, #0x1
1010fc0c0:     	b.ne	0x1010fc114 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0xb0>
1010fc0c4:     	b	0x1010fc2ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1010fc0c8:     	ldr	w2, [x19, #0x128]
1010fc0cc:     	mov	x0, x19
1010fc0d0:     	mov	w1, #0x4                ; =4
1010fc0d4:     	mov	x3, x24
1010fc0d8:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc0dc:     	mov	x24, x0
1010fc0e0:     	ldr	x22, [x29, #0x18]
1010fc0e4:     	lsr	x25, x26, #1
1010fc0e8:     	tbz	w26, #0x0, 0x1010fc0b4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x50>
1010fc0ec:     	ldr	w2, [x19, #0x128]
1010fc0f0:     	mov	x0, x19
1010fc0f4:     	mov	w1, #0x4                ; =4
1010fc0f8:     	mov	x3, x25
1010fc0fc:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc100:     	mov	x25, x0
1010fc104:     	ldr	x26, [x29, #0x10]
1010fc108:     	ldr	w27, [x19, #0x128]
1010fc10c:     	cmp	w27, #0x1
1010fc110:     	b.eq	0x1010fc2ac <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x248>
1010fc114:     	str	x28, [sp, #0x20]
1010fc118:     	ldr	x8, [x19, #0x108]
1010fc11c:     	mov	w9, w8
1010fc120:     	stp	x9, x20, [sp, #0x30]
1010fc124:     	cmp	x20, x9
1010fc128:     	b.ne	0x1010fc37c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fc12c:     	cbz	x20, 0x1010fc230 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1cc>
1010fc130:     	mov	x10, #0x0               ; =0
1010fc134:     	lsl	x28, x20, #2
1010fc138:     	mov	w11, #0x1               ; =1
1010fc13c:     	mov	x12, x28
1010fc140:     	mov	x13, x23
1010fc144:     	ldr	w14, [x13], #0x4
1010fc148:     	cmp	w14, w8
1010fc14c:     	b.hs	0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc150:     	lsr	x15, x10, x14
1010fc154:     	tbnz	w15, #0x0, 0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc158:     	lsl	x14, x11, x14
1010fc15c:     	orr	x10, x14, x10
1010fc160:     	subs	x12, x12, #0x4
1010fc164:     	b.ne	0x1010fc144 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0xe0>
1010fc168:     	str	x21, [sp, #0x38]
1010fc16c:     	cmp	x21, x20
1010fc170:     	b.ne	0x1010fc37c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fc174:     	mov	x10, #0x0               ; =0
1010fc178:     	mov	w11, #0x1               ; =1
1010fc17c:     	mov	x12, x28
1010fc180:     	ldr	x13, [sp, #0x20]
1010fc184:     	ldr	w14, [x13], #0x4
1010fc188:     	cmp	w14, w8
1010fc18c:     	b.hs	0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc190:     	lsr	x15, x10, x14
1010fc194:     	tbnz	w15, #0x0, 0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc198:     	lsl	x14, x11, x14
1010fc19c:     	orr	x10, x14, x10
1010fc1a0:     	subs	x12, x12, #0x4
1010fc1a4:     	b.ne	0x1010fc184 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x120>
1010fc1a8:     	str	x22, [sp, #0x38]
1010fc1ac:     	cmp	x22, x20
1010fc1b0:     	b.ne	0x1010fc37c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fc1b4:     	mov	x10, #0x0               ; =0
1010fc1b8:     	mov	w11, #0x1               ; =1
1010fc1bc:     	mov	x12, x28
1010fc1c0:     	mov	x13, x26
1010fc1c4:     	ldr	w14, [x13], #0x4
1010fc1c8:     	cmp	w14, w8
1010fc1cc:     	b.hs	0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc1d0:     	lsr	x15, x10, x14
1010fc1d4:     	tbnz	w15, #0x0, 0x1010fc364 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x300>
1010fc1d8:     	lsl	x14, x11, x14
1010fc1dc:     	orr	x10, x14, x10
1010fc1e0:     	subs	x12, x12, #0x4
1010fc1e4:     	b.ne	0x1010fc1c4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x160>
1010fc1e8:     	ldr	x8, [sp, #0x28]
1010fc1ec:     	lsr	x8, x8, x9
1010fc1f0:     	str	x8, [sp, #0x38]
1010fc1f4:     	cbnz	x8, 0x1010fc398 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x334>
1010fc1f8:     	mov	x0, x28
1010fc1fc:     	mov	w1, #0x1                ; =1
1010fc200:     	bl	0x10150f0e4 <dyld_stub_binder+0x10150f0e4>
1010fc204:     	cbz	x0, 0x1010fc3cc <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x368>
1010fc208:     	mov	x21, x0
1010fc20c:     	mov	x8, #0x0                ; =0
1010fc210:     	ldr	w0, [x23, x8, lsl #2]
1010fc214:     	cmp	x20, x0
1010fc218:     	b.ls	0x1010fc3b8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x354>
1010fc21c:     	str	w8, [x21, x0, lsl #2]
1010fc220:     	add	x8, x8, #0x1
1010fc224:     	subs	x28, x28, #0x4
1010fc228:     	b.ne	0x1010fc210 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ac>
1010fc22c:     	b	0x1010fc250 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x1ec>
1010fc230:     	str	x21, [sp, #0x38]
1010fc234:     	cbnz	x21, 0x1010fc37c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fc238:     	str	x22, [sp, #0x38]
1010fc23c:     	cbnz	x22, 0x1010fc37c <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x318>
1010fc240:     	ldr	x8, [sp, #0x28]
1010fc244:     	str	x8, [sp, #0x38]
1010fc248:     	cbnz	x8, 0x1010fc398 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x334>
1010fc24c:     	mov	w21, #0x4               ; =4
1010fc250:     	mov	x0, x19
1010fc254:     	mov	x1, x27
1010fc258:     	mov	x2, x21
1010fc25c:     	mov	x3, x20
1010fc260:     	bl	0x100df77b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6renameB6_>
1010fc264:     	ldr	x28, [sp, #0x20]
1010fc268:     	mov	x3, x0
1010fc26c:     	mov	x0, x19
1010fc270:     	mov	w1, #0x8                ; =8
1010fc274:     	mov	x2, x24
1010fc278:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc27c:     	mov	x24, x0
1010fc280:     	ldr	w1, [x19, #0x128]
1010fc284:     	mov	x0, x19
1010fc288:     	mov	x2, x26
1010fc28c:     	mov	x3, x20
1010fc290:     	bl	0x100df77b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E6renameB6_>
1010fc294:     	mov	x27, x0
1010fc298:     	cbz	x20, 0x1010fc2a4 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x240>
1010fc29c:     	mov	x0, x21
1010fc2a0:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1010fc2a4:     	mov	x22, x20
1010fc2a8:     	mov	x21, x20
1010fc2ac:     	stp	x26, x22, [sp, #0x8]
1010fc2b0:     	add	x0, sp, #0x38
1010fc2b4:     	ldr	x8, [sp, #0x28]
1010fc2b8:     	str	x8, [sp]
1010fc2bc:     	mov	x1, x19
1010fc2c0:     	mov	x2, x24
1010fc2c4:     	mov	x3, x23
1010fc2c8:     	mov	x4, x20
1010fc2cc:     	mov	x5, x25
1010fc2d0:     	mov	x6, x28
1010fc2d4:     	mov	x7, x21
1010fc2d8:     	bl	0x100edf29c <__RNvMs7_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productINtB7_5ArenaKm6_E23mapped_product_observedBb_>
1010fc2dc:     	ldr	w3, [sp, #0x38]
1010fc2e0:     	mov	x0, x19
1010fc2e4:     	mov	w1, #0x8                ; =8
1010fc2e8:     	mov	x2, x27
1010fc2ec:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc2f0:     	mov	x3, x0
1010fc2f4:     	ldr	w2, [x19, #0x128]
1010fc2f8:     	mov	x0, x19
1010fc2fc:     	mov	w1, #0x8                ; =8
1010fc300:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc304:     	mov	x20, x0
1010fc308:     	ldr	x2, [x19, #0x118]
1010fc30c:     	mov	x0, x19
1010fc310:     	mov	x1, x20
1010fc314:     	bl	0x100dec3b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
1010fc318:     	mov	x21, x0
1010fc31c:     	cbz	w0, 0x1010fc338 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x2d4>
1010fc320:     	ldr	w2, [x19, #0x128]
1010fc324:     	mov	x0, x19
1010fc328:     	mov	w1, #0x4                ; =4
1010fc32c:     	mov	x3, x20
1010fc330:     	bl	0x100df61c0 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm6_E5applyB6_>
1010fc334:     	mov	x20, x0
1010fc338:     	mov	w8, w20
1010fc33c:     	mov	w9, w21
1010fc340:     	orr	x0, x9, x8, lsl #1
1010fc344:     	ldp	x29, x30, [sp, #0x120]
1010fc348:     	ldp	x20, x19, [sp, #0x110]
1010fc34c:     	ldp	x22, x21, [sp, #0x100]
1010fc350:     	ldp	x24, x23, [sp, #0xf0]
1010fc354:     	ldp	x26, x25, [sp, #0xe0]
1010fc358:     	ldp	x28, x27, [sp, #0xd0]
1010fc35c:     	add	sp, sp, #0x130
1010fc360:     	ret
1010fc364:     	adrp	x0, 0x10166a000 <__RNvNvXsi_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab8transferNtB7_5ErrorNtNtCs4sDCw1iE1MS_4core3fmt5Debug3fmt8___OFFSET+0x1ec8>
1010fc368:     	add	x0, x0, #0xc7c
1010fc36c:     	adrp	x2, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fc370:     	add	x2, x2, #0x168
1010fc374:     	mov	w1, #0x41               ; =65
1010fc378:     	bl	0x101506dc8 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
1010fc37c:     	adrp	x5, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fc380:     	add	x5, x5, #0x150
1010fc384:     	add	x1, sp, #0x38
1010fc388:     	add	x2, sp, #0x30
1010fc38c:     	mov	w0, #0x0                ; =0
1010fc390:     	mov	x3, #0x0                ; =0
1010fc394:     	bl	0x101506cb0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedjjEB4_>
1010fc398:     	adrp	x2, 0x101672000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x1fb7>
1010fc39c:     	add	x2, x2, #0x268
1010fc3a0:     	adrp	x5, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fc3a4:     	add	x5, x5, #0x138
1010fc3a8:     	add	x1, sp, #0x38
1010fc3ac:     	mov	w0, #0x0                ; =0
1010fc3b0:     	mov	x3, #0x0                ; =0
1010fc3b4:     	bl	0x101506ce0 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
1010fc3b8:     	adrp	x2, 0x1017cf000 <dyld_stub_binder+0x1017cf000>
1010fc3bc:     	add	x2, x2, #0x120
1010fc3c0:     	mov	x1, x20
1010fc3c4:     	bl	0x101506ddc <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1010fc3c8:     	brk	#0x1
1010fc3cc:     	mov	w0, #0x4                ; =4
1010fc3d0:     	mov	x1, x28
1010fc3d4:     	bl	0x1015065e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
1010fc3d8:     	mov	x19, x0
1010fc3dc:     	cbnz	x20, 0x1010fc3e8 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x384>
1010fc3e0:     	b	0x1010fc3f0 <__RNvXs_NtNtCs23EhFSy3h49_8bumbledb14event_repr_lab9essentialINtB4_9EssentialKm6_ENtNtB6_7carrier9RegionOps12view_productB8_+0x38c>
1010fc3e4:     	mov	x19, x0
1010fc3e8:     	mov	x0, x21
1010fc3ec:     	bl	0x10150f1f8 <dyld_stub_binder+0x10150f1f8>
1010fc3f0:     	mov	x0, x19
1010fc3f4:     	bl	0x10150f048 <dyld_stub_binder+0x10150f048>
