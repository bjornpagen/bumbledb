
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001005f50f4 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_>:
1005f50f4:     	stp	x24, x23, [sp, #-0x40]!
1005f50f8:     	stp	x22, x21, [sp, #0x10]
1005f50fc:     	stp	x20, x19, [sp, #0x20]
1005f5100:     	stp	x29, x30, [sp, #0x30]
1005f5104:     	add	x29, sp, #0x30
1005f5108:     	cmp	x5, x3
1005f510c:     	csel	x20, x5, x3, lo
1005f5110:     	lsr	x8, x20, #60
1005f5114:     	cbz	x8, 0x1005f511c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x28>
1005f5118:     	bl	0x1009ec010 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1005f511c:     	cbz	x20, 0x1005f5230 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x13c>
1005f5120:     	mov	x21, x1
1005f5124:     	mov	x22, x2
1005f5128:     	mov	x23, x4
1005f512c:     	mov	x24, x0
1005f5130:     	lsl	x19, x20, #3
1005f5134:     	mov	x0, x19
1005f5138:     	bl	0x1009f4d00 <dyld_stub_binder+0x1009f4d00>
1005f513c:     	cbz	x0, 0x1005f5254 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x160>
1005f5140:     	mov	x8, x0
1005f5144:     	mov	x10, #0x0               ; =0
1005f5148:     	mov	x0, x24
1005f514c:     	mov	x9, x23
1005f5150:     	mov	x11, x22
1005f5154:     	mov	x12, x21
1005f5158:     	b	0x1005f5170 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x7c>
1005f515c:     	eor	x15, x16, x15
1005f5160:     	eor	x14, x15, x14
1005f5164:     	str	x14, [x8, x13, lsl #3]
1005f5168:     	cmp	x10, x20
1005f516c:     	b.eq	0x1005f5228 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x134>
1005f5170:     	mov	x14, #0x0               ; =0
1005f5174:     	mov	x13, x10
1005f5178:     	add	x10, x10, #0x1
1005f517c:     	ldr	x15, [x11, x13, lsl #3]
1005f5180:     	ldr	x16, [x9, x13, lsl #3]
1005f5184:     	mov	x17, x12
1005f5188:     	b	0x1005f5194 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0xa0>
1005f518c:     	eor	w17, w17, #0xf
1005f5190:     	mvn	x14, x14
1005f5194:     	and	w1, w17, #0xff
1005f5198:     	cmp	w1, #0x7
1005f519c:     	b.gt	0x1005f51b8 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0xc4>
1005f51a0:     	cmp	w1, #0x3
1005f51a4:     	b.gt	0x1005f51d4 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0xe0>
1005f51a8:     	cbz	w1, 0x1005f5218 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x124>
1005f51ac:     	cmp	w1, #0x2
1005f51b0:     	b.ne	0x1005f518c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x98>
1005f51b4:     	b	0x1005f5200 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x10c>
1005f51b8:     	cmp	w1, #0xb
1005f51bc:     	b.gt	0x1005f51e8 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0xf4>
1005f51c0:     	cmp	w1, #0x8
1005f51c4:     	b.eq	0x1005f5210 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x11c>
1005f51c8:     	cmp	w1, #0xa
1005f51cc:     	b.ne	0x1005f518c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x98>
1005f51d0:     	b	0x1005f5208 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x114>
1005f51d4:     	cmp	w1, #0x4
1005f51d8:     	b.eq	0x1005f5220 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x12c>
1005f51dc:     	cmp	w1, #0x6
1005f51e0:     	b.ne	0x1005f518c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x98>
1005f51e4:     	b	0x1005f515c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x68>
1005f51e8:     	cmp	w1, #0xc
1005f51ec:     	b.eq	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f51f0:     	cmp	w1, #0xe
1005f51f4:     	b.ne	0x1005f518c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x98>
1005f51f8:     	orr	x15, x16, x15
1005f51fc:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5200:     	bic	x15, x16, x15
1005f5204:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5208:     	mov	x15, x16
1005f520c:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5210:     	and	x15, x16, x15
1005f5214:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5218:     	mov	x15, #0x0               ; =0
1005f521c:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5220:     	bic	x15, x15, x16
1005f5224:     	b	0x1005f5160 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x6c>
1005f5228:     	mov	x9, x20
1005f522c:     	b	0x1005f5238 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_opB6_+0x144>
1005f5230:     	mov	x9, #0x0                ; =0
1005f5234:     	mov	w8, #0x8                ; =8
1005f5238:     	stp	x9, x8, [x0]
1005f523c:     	str	x20, [x0, #0x10]
1005f5240:     	ldp	x29, x30, [sp, #0x30]
1005f5244:     	ldp	x20, x19, [sp, #0x20]
1005f5248:     	ldp	x22, x21, [sp, #0x10]
1005f524c:     	ldp	x24, x23, [sp], #0x40
1005f5250:     	ret
1005f5254:     	mov	w0, #0x8                ; =8
1005f5258:     	mov	x1, x19
1005f525c:     	bl	0x1009ebfe4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
