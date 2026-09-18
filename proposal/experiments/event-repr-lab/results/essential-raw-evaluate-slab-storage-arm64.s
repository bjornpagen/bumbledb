
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100b8c068 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>:
100b8c068:     	sub	sp, sp, #0x50
100b8c06c:     	stp	x22, x21, [sp, #0x20]
100b8c070:     	stp	x20, x19, [sp, #0x30]
100b8c074:     	stp	x29, x30, [sp, #0x40]
100b8c078:     	add	x29, sp, #0x40
100b8c07c:     	mov	x20, x2
100b8c080:     	mov	x19, x1
100b8c084:     	mov	x21, x0
100b8c088:     	mov	x0, sp
100b8c08c:     	add	x1, x21, #0x30
100b8c090:     	mov	x2, x19
100b8c094:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100b8c098:     	ldr	w8, [sp]
100b8c09c:     	cbz	w8, 0x100b8c0fc <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0x94>
100b8c0a0:     	cmp	w8, #0x1
100b8c0a4:     	b.ne	0x100b8c104 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0x9c>
100b8c0a8:     	ldp	x1, x9, [sp, #0x10]
100b8c0ac:     	cbz	x9, 0x100b8c150 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0xe8>
100b8c0b0:     	mov	x8, #0x0                ; =0
100b8c0b4:     	mov	w10, #0x0               ; =0
100b8c0b8:     	rbit	x11, x9
100b8c0bc:     	clz	x11, x11
100b8c0c0:     	lsr	x11, x20, x11
100b8c0c4:     	and	x11, x11, #0x1
100b8c0c8:     	lsl	x11, x11, x10
100b8c0cc:     	orr	x8, x11, x8
100b8c0d0:     	sub	x11, x9, #0x1
100b8c0d4:     	add	w10, w10, #0x1
100b8c0d8:     	ands	x9, x11, x9
100b8c0dc:     	b.ne	0x100b8c0b8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0x50>
100b8c0e0:     	lsr	x0, x8, #6
100b8c0e4:     	cmp	x0, x1
100b8c0e8:     	b.hs	0x100b8c160 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0xf8>
100b8c0ec:     	ldr	x9, [sp, #0x8]
100b8c0f0:     	ldr	x9, [x9, x0, lsl #3]
100b8c0f4:     	lsr	x0, x9, x8
100b8c0f8:     	b	0x100b8c134 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0xcc>
100b8c0fc:     	mov	w0, #0x0                ; =0
100b8c100:     	b	0x100b8c134 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0xcc>
100b8c104:     	ldr	w8, [sp, #0x4]
100b8c108:     	mov	w9, #0x1                ; =1
100b8c10c:     	lsl	x8, x9, x8
100b8c110:     	tst	x8, x20
100b8c114:     	mov	w8, #0xc                ; =12
100b8c118:     	mov	w9, #0x8                ; =8
100b8c11c:     	csel	x8, x9, x8, eq
100b8c120:     	mov	x9, sp
100b8c124:     	ldr	w1, [x9, x8]
100b8c128:     	mov	x0, x21
100b8c12c:     	mov	x2, x20
100b8c130:     	bl	0x100b8c068 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100b8c134:     	eor	w8, w0, w19
100b8c138:     	and	w0, w8, #0x1
100b8c13c:     	ldp	x29, x30, [sp, #0x40]
100b8c140:     	ldp	x20, x19, [sp, #0x30]
100b8c144:     	ldp	x22, x21, [sp, #0x20]
100b8c148:     	add	sp, sp, #0x50
100b8c14c:     	ret
100b8c150:     	mov	x8, #0x0                ; =0
100b8c154:     	lsr	x0, x8, #6
100b8c158:     	cmp	x0, x1
100b8c15c:     	b.lo	0x100b8c0ec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_+0x84>
100b8c160:     	adrp	x2, 0x1014be000 <dyld_stub_binder+0x1014be000>
100b8c164:     	add	x2, x2, #0x290
100b8c168:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
