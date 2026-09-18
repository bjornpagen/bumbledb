
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001005bfff4 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op>:
1005bfff4:     	stp	x24, x23, [sp, #-0x40]!
1005bfff8:     	stp	x22, x21, [sp, #0x10]
1005bfffc:     	stp	x20, x19, [sp, #0x20]
1005c0000:     	stp	x29, x30, [sp, #0x30]
1005c0004:     	add	x29, sp, #0x30
1005c0008:     	cmp	x5, x3
1005c000c:     	csel	x20, x5, x3, lo
1005c0010:     	lsr	x8, x20, #60
1005c0014:     	cbz	x8, 0x1005c001c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x28>
1005c0018:     	bl	0x1009b6410 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec17capacity_overflow>
1005c001c:     	cbz	x20, 0x1005c0130 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x13c>
1005c0020:     	mov	x21, x1
1005c0024:     	mov	x22, x2
1005c0028:     	mov	x23, x4
1005c002c:     	mov	x24, x0
1005c0030:     	lsl	x19, x20, #3
1005c0034:     	mov	x0, x19
1005c0038:     	bl	0x1009bf100 <dyld_stub_binder+0x1009bf100>
1005c003c:     	cbz	x0, 0x1005c0154 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x160>
1005c0040:     	mov	x8, x0
1005c0044:     	mov	x10, #0x0               ; =0
1005c0048:     	mov	x0, x24
1005c004c:     	mov	x9, x23
1005c0050:     	mov	x11, x22
1005c0054:     	mov	x12, x21
1005c0058:     	b	0x1005c0070 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x7c>
1005c005c:     	eor	x15, x16, x15
1005c0060:     	eor	x14, x15, x14
1005c0064:     	str	x14, [x8, x13, lsl #3]
1005c0068:     	cmp	x10, x20
1005c006c:     	b.eq	0x1005c0128 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x134>
1005c0070:     	mov	x14, #0x0               ; =0
1005c0074:     	mov	x13, x10
1005c0078:     	add	x10, x10, #0x1
1005c007c:     	ldr	x15, [x11, x13, lsl #3]
1005c0080:     	ldr	x16, [x9, x13, lsl #3]
1005c0084:     	mov	x17, x12
1005c0088:     	b	0x1005c0094 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0xa0>
1005c008c:     	eor	w17, w17, #0xf
1005c0090:     	mvn	x14, x14
1005c0094:     	and	w1, w17, #0xff
1005c0098:     	cmp	w1, #0x7
1005c009c:     	b.gt	0x1005c00b8 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0xc4>
1005c00a0:     	cmp	w1, #0x3
1005c00a4:     	b.gt	0x1005c00d4 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0xe0>
1005c00a8:     	cbz	w1, 0x1005c0118 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x124>
1005c00ac:     	cmp	w1, #0x2
1005c00b0:     	b.ne	0x1005c008c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x98>
1005c00b4:     	b	0x1005c0100 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x10c>
1005c00b8:     	cmp	w1, #0xb
1005c00bc:     	b.gt	0x1005c00e8 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0xf4>
1005c00c0:     	cmp	w1, #0x8
1005c00c4:     	b.eq	0x1005c0110 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x11c>
1005c00c8:     	cmp	w1, #0xa
1005c00cc:     	b.ne	0x1005c008c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x98>
1005c00d0:     	b	0x1005c0108 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x114>
1005c00d4:     	cmp	w1, #0x4
1005c00d8:     	b.eq	0x1005c0120 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x12c>
1005c00dc:     	cmp	w1, #0x6
1005c00e0:     	b.ne	0x1005c008c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x98>
1005c00e4:     	b	0x1005c005c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x68>
1005c00e8:     	cmp	w1, #0xc
1005c00ec:     	b.eq	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c00f0:     	cmp	w1, #0xe
1005c00f4:     	b.ne	0x1005c008c <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x98>
1005c00f8:     	orr	x15, x16, x15
1005c00fc:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0100:     	bic	x15, x16, x15
1005c0104:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0108:     	mov	x15, x16
1005c010c:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0110:     	and	x15, x16, x15
1005c0114:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0118:     	mov	x15, #0x0               ; =0
1005c011c:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0120:     	bic	x15, x15, x16
1005c0124:     	b	0x1005c0060 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x6c>
1005c0128:     	mov	x9, x20
1005c012c:     	b	0x1005c0138 <__RNvXNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6finiteNtB2_5DenseNtB2_3Set6raw_op+0x144>
1005c0130:     	mov	x9, #0x0                ; =0
1005c0134:     	mov	w8, #0x8                ; =8
1005c0138:     	stp	x9, x8, [x0]
1005c013c:     	str	x20, [x0, #0x10]
1005c0140:     	ldp	x29, x30, [sp, #0x30]
1005c0144:     	ldp	x20, x19, [sp, #0x20]
1005c0148:     	ldp	x22, x21, [sp, #0x10]
1005c014c:     	ldp	x24, x23, [sp], #0x40
1005c0150:     	ret
1005c0154:     	mov	w0, #0x8                ; =8
1005c0158:     	mov	x1, x19
1005c015c:     	bl	0x1009b63e4 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
