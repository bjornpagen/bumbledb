
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>:
100c7f12c:     	stp	x29, x30, [sp, #-0x10]!
100c7f130:     	mov	x29, sp
100c7f134:     	ldr	x9, [x1]
100c7f138:     	lsr	w8, w2, #1
100c7f13c:     	cmn	x9, #0x1
100c7f140:     	b.eq	0x100c7f180 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x54>
100c7f144:     	ldr	x9, [x1, #0x10]
100c7f148:     	cmp	x9, x8
100c7f14c:     	b.ls	0x100c7f248 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x11c>
100c7f150:     	ldr	x9, [x1, #0x8]
100c7f154:     	add	x8, x9, x8, lsl #4
100c7f158:     	ldr	x10, [x8]
100c7f15c:     	cmp	w2, #0x2
100c7f160:     	b.lo	0x100c7f1c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x94>
100c7f164:     	ldr	x8, [x8, #0x8]
100c7f168:     	tbnz	x8, #0x3f, 0x100c7f1e8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0xbc>
100c7f16c:     	lsr	x9, x8, #28
100c7f170:     	ubfx	x11, x8, #56, #6
100c7f174:     	and	x8, x8, #0xfffffff
100c7f178:     	bfi	x8, x9, #32, #28
100c7f17c:     	b	0x100c7f1d0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0xa4>
100c7f180:     	ldr	x9, [x1, #0x18]
100c7f184:     	cmp	x9, x8
100c7f188:     	b.ls	0x100c7f25c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x130>
100c7f18c:     	ldr	x9, [x1, #0x10]
100c7f190:     	add	x9, x9, x8, lsl #5
100c7f194:     	ldr	x10, [x9, #0x18]
100c7f198:     	ldr	x8, [x9]
100c7f19c:     	eor	x11, x8, #0x8000000000000000
100c7f1a0:     	cmp	x8, #0x0
100c7f1a4:     	csinc	x8, x11, xzr, mi
100c7f1a8:     	cbz	x8, 0x100c7f1c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x94>
100c7f1ac:     	cmp	x8, #0x1
100c7f1b0:     	b.ne	0x100c7f1c8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x9c>
100c7f1b4:     	ldp	x8, x12, [x9, #0x8]
100c7f1b8:     	mov	w9, #0x1                ; =1
100c7f1bc:     	b	0x100c7f1d4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0xa8>
100c7f1c0:     	mov	w9, #0x0                ; =0
100c7f1c4:     	b	0x100c7f1d4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0xa8>
100c7f1c8:     	ldr	w11, [x9, #0x8]
100c7f1cc:     	ldur	x8, [x9, #0xc]
100c7f1d0:     	mov	w9, #0x2                ; =2
100c7f1d4:     	stp	x12, x10, [x0, #0x10]
100c7f1d8:     	stp	w9, w11, [x0]
100c7f1dc:     	str	x8, [x0, #0x8]
100c7f1e0:     	ldp	x29, x30, [sp], #0x10
100c7f1e4:     	ret
100c7f1e8:     	and	x8, x8, #0x7fffffffffffffff
100c7f1ec:     	fmov	d0, x10
100c7f1f0:     	cnt.8b	v0, v0
100c7f1f4:     	addv.8b	b0, v0
100c7f1f8:     	fmov	x9, d0
100c7f1fc:     	and	x11, x9, #0x3f
100c7f200:     	mov	w12, #0x1               ; =1
100c7f204:     	lsl	x9, x12, x9
100c7f208:     	lsr	x9, x9, #6
100c7f20c:     	cmp	x11, #0x6
100c7f210:     	cinc	x12, x9, lo
100c7f214:     	ldr	x2, [x1, #0x28]
100c7f218:     	add	x9, x12, x8
100c7f21c:     	cmp	x9, x2
100c7f220:     	b.hi	0x100c7f234 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0x108>
100c7f224:     	ldr	x9, [x1, #0x20]
100c7f228:     	add	x8, x9, x8, lsl #3
100c7f22c:     	mov	w9, #0x1                ; =1
100c7f230:     	b	0x100c7f1d4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node+0xa8>
100c7f234:     	adrp	x3, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f238:     	add	x3, x3, #0xf88
100c7f23c:     	mov	x0, x8
100c7f240:     	mov	x1, x9
100c7f244:     	bl	0x10127c7d4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
100c7f248:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f24c:     	add	x2, x2, #0xf70
100c7f250:     	mov	x0, x8
100c7f254:     	mov	x1, x9
100c7f258:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100c7f25c:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f260:     	add	x2, x2, #0xf58
100c7f264:     	mov	x0, x8
100c7f268:     	mov	x1, x9
100c7f26c:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
