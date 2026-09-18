
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006cfe80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>:
1006cfe80:     	sub	sp, sp, #0x50
1006cfe84:     	stp	x24, x23, [sp, #0x10]
1006cfe88:     	stp	x22, x21, [sp, #0x20]
1006cfe8c:     	stp	x20, x19, [sp, #0x30]
1006cfe90:     	stp	x29, x30, [sp, #0x40]
1006cfe94:     	add	x29, sp, #0x40
1006cfe98:     	mov	x20, x3
1006cfe9c:     	mov	x23, x2
1006cfea0:     	mov	x22, x1
1006cfea4:     	mov	x19, x0
1006cfea8:     	mov	w1, w2
1006cfeac:     	mov	w2, w3
1006cfeb0:     	mov	x0, x22
1006cfeb4:     	bl	0x1007989d8 <__RNvNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrier7trivial>
1006cfeb8:     	cmp	x0, #0x1
1006cfebc:     	b.ne	0x1006cfec8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x48>
1006cfec0:     	mov	x21, x1
1006cfec4:     	b	0x1006d057c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6fc>
1006cfec8:     	mov	w8, #0x9                ; =9
1006cfecc:     	and	w8, w22, w8
1006cfed0:     	lsr	w9, w22, #1
1006cfed4:     	bfi	w8, w9, #2, #1
1006cfed8:     	and	w9, w9, #0x2
1006cfedc:     	orr	w8, w8, w9
1006cfee0:     	cmp	w23, w20
1006cfee4:     	csel	w24, w23, w20, hi
1006cfee8:     	csel	w23, w20, w23, hi
1006cfeec:     	csel	w20, w8, w22, hi
1006cfef0:     	ldr	x8, [x19, #0xb8]
1006cfef4:     	cbz	x8, 0x1006cffc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x144>
1006cfef8:     	mov	x8, #0x0                ; =0
1006cfefc:     	and	x9, x20, #0xff
1006cff00:     	mov	x10, #0xa9c5            ; =43461
1006cff04:     	movk	x10, #0x2e62, lsl #16
1006cff08:     	movk	x10, #0x7aea, lsl #32
1006cff0c:     	movk	x10, #0xf135, lsl #48
1006cff10:     	mul	x9, x9, x10
1006cff14:     	add	x9, x9, w23, uxtw
1006cff18:     	mul	x9, x9, x10
1006cff1c:     	add	x9, x9, w24, uxtw
1006cff20:     	mul	x9, x9, x10
1006cff24:     	ror	x11, x9, #0x2c
1006cff28:     	lsr	x12, x11, #57
1006cff2c:     	ldp	x10, x9, [x19, #0xa0]
1006cff30:     	dup.8b	v0, w12
1006cff34:     	movi.2d	v1, #0xffffffffffffffff
1006cff38:     	and	x11, x11, x9
1006cff3c:     	ldr	d2, [x10, x11]
1006cff40:     	cmeq.8b	v3, v2, v0
1006cff44:     	fmov	x12, d3
1006cff48:     	ands	x12, x12, #0x8080808080808080
1006cff4c:     	b.eq	0x1006cff94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x114>
1006cff50:     	rbit	x13, x12
1006cff54:     	clz	x13, x13
1006cff58:     	add	x13, x11, x13, lsr #3
1006cff5c:     	and	x13, x13, x9
1006cff60:     	sub	x13, x10, x13, lsl #4
1006cff64:     	ldurb	w14, [x13, #-0xc]
1006cff68:     	cmp	w14, w20, uxtb
1006cff6c:     	b.ne	0x1006cff88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x108>
1006cff70:     	ldur	w14, [x13, #-0x10]
1006cff74:     	cmp	w23, w14
1006cff78:     	b.ne	0x1006cff88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x108>
1006cff7c:     	ldur	w14, [x13, #-0x8]
1006cff80:     	cmp	w24, w14
1006cff84:     	b.eq	0x1006d0064 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1e4>
1006cff88:     	sub	x13, x12, #0x2
1006cff8c:     	ands	x12, x13, x12
1006cff90:     	b.ne	0x1006cff50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0xd0>
1006cff94:     	cmeq.8b	v2, v2, v1
1006cff98:     	fmov	x12, d2
1006cff9c:     	cbnz	x12, 0x1006cffc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x144>
1006cffa0:     	add	x8, x8, #0x8
1006cffa4:     	add	x11, x11, x8
1006cffa8:     	and	x11, x11, x9
1006cffac:     	ldr	d2, [x10, x11]
1006cffb0:     	cmeq.8b	v3, v2, v0
1006cffb4:     	fmov	x12, d3
1006cffb8:     	ands	x12, x12, #0x8080808080808080
1006cffbc:     	b.ne	0x1006cff50 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0xd0>
1006cffc0:     	b	0x1006cff94 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x114>
1006cffc4:     	tbnz	w23, #0x1, 0x1006d006c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1ec>
1006cffc8:     	ldr	x8, [x19, #0xc8]
1006cffcc:     	tbnz	w24, #0x1, 0x1006d008c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x20c>
1006cffd0:     	ldr	x9, [x19, #0xc8]
1006cffd4:     	cmp	x9, x8
1006cffd8:     	csel	x21, x9, x8, lo
1006cffdc:     	cmp	x21, x9
1006cffe0:     	b.ne	0x1006d00bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x23c>
1006cffe4:     	lsr	w0, w23, #2
1006cffe8:     	ldr	x1, [x19, #0x58]
1006cffec:     	cmp	x1, x0
1006cfff0:     	b.ls	0x1006d05a4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x724>
1006cfff4:     	lsr	w8, w24, #2
1006cfff8:     	cmp	x1, x8
1006cfffc:     	b.ls	0x1006d05b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x730>
1006d0000:     	ldr	x10, [x19, #0xd8]
1006d0004:     	tst	w23, #0x1
1006d0008:     	csel	x9, xzr, x10, eq
1006d000c:     	ldr	x11, [x19, #0x50]
1006d0010:     	ldr	x12, [x11, x0, lsl #3]
1006d0014:     	eor	x9, x12, x9
1006d0018:     	ldr	x8, [x11, x8, lsl #3]
1006d001c:     	tst	w24, #0x1
1006d0020:     	csel	x10, xzr, x10, eq
1006d0024:     	eor	x8, x8, x10
1006d0028:     	and	x1, x20, #0xff
1006d002c:     	adrp	x10, 0x100d14000 <dyld_stub_binder+0x100d14000>
1006d0030:     	add	x10, x10, #0x786
1006d0034:     	adr	x11, 0x1006d0044 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x1c4>
1006d0038:     	ldrh	w12, [x10, x1, lsl #1]
1006d003c:     	add	x11, x11, x12, lsl #2
1006d0040:     	br	x11
1006d0044:     	mov	x10, #0x0               ; =0
1006d0048:     	and	x11, x8, x9
1006d004c:     	eor	x12, x8, x9
1006d0050:     	bic	x13, x9, x8
1006d0054:     	bic	x14, x8, x9
1006d0058:     	orr	x15, x8, x9
1006d005c:     	mov	w16, #0x1               ; =1
1006d0060:     	b	0x1006d0238 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b8>
1006d0064:     	ldur	w21, [x13, #-0x4]
1006d0068:     	b	0x1006d057c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6fc>
1006d006c:     	lsr	w0, w23, #2
1006d0070:     	ldr	x1, [x19, #0x40]
1006d0074:     	cmp	x1, x0
1006d0078:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d007c:     	ldr	x8, [x19, #0x38]
1006d0080:     	lsl	x9, x0, #4
1006d0084:     	ldr	w8, [x8, x9]
1006d0088:     	tbz	w24, #0x1, 0x1006cffd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x150>
1006d008c:     	lsr	w0, w24, #2
1006d0090:     	ldr	x1, [x19, #0x40]
1006d0094:     	cmp	x1, x0
1006d0098:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d009c:     	ldr	x9, [x19, #0x38]
1006d00a0:     	lsl	x10, x0, #4
1006d00a4:     	ldr	w10, [x9, x10]
1006d00a8:     	ldr	x9, [x19, #0xc8]
1006d00ac:     	cmp	x10, x8
1006d00b0:     	csel	x21, x10, x8, lo
1006d00b4:     	cmp	x21, x9
1006d00b8:     	b.eq	0x1006cffe4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x164>
1006d00bc:     	mov	x2, x23
1006d00c0:     	tbz	w23, #0x1, 0x1006d00f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x278>
1006d00c4:     	lsr	w0, w23, #2
1006d00c8:     	ldr	x1, [x19, #0x40]
1006d00cc:     	cmp	x1, x0
1006d00d0:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d00d4:     	ldr	x8, [x19, #0x38]
1006d00d8:     	add	x8, x8, x0, lsl #4
1006d00dc:     	ldr	w9, [x8]
1006d00e0:     	mov	x2, x23
1006d00e4:     	cmp	x21, x9
1006d00e8:     	b.ne	0x1006d00f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x278>
1006d00ec:     	ldr	w8, [x8, #0x4]
1006d00f0:     	and	w9, w23, #0x1
1006d00f4:     	eor	w2, w8, w9
1006d00f8:     	mov	x3, x24
1006d00fc:     	tbz	w24, #0x1, 0x1006d0134 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2b4>
1006d0100:     	lsr	w0, w24, #2
1006d0104:     	ldr	x1, [x19, #0x40]
1006d0108:     	cmp	x1, x0
1006d010c:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d0110:     	ldr	x8, [x19, #0x38]
1006d0114:     	add	x8, x8, x0, lsl #4
1006d0118:     	ldr	w9, [x8]
1006d011c:     	mov	x3, x24
1006d0120:     	cmp	x21, x9
1006d0124:     	b.ne	0x1006d0134 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2b4>
1006d0128:     	ldr	w8, [x8, #0x4]
1006d012c:     	and	w9, w24, #0x1
1006d0130:     	eor	w3, w8, w9
1006d0134:     	mov	x0, x19
1006d0138:     	mov	x1, x20
1006d013c:     	bl	0x1006cfe80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>
1006d0140:     	mov	x22, x0
1006d0144:     	tbnz	w23, #0x1, 0x1006d01ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x32c>
1006d0148:     	ldr	x8, [x19, #0xc8]
1006d014c:     	mov	x2, x23
1006d0150:     	cmp	x8, x21
1006d0154:     	b.ne	0x1006d01d4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x354>
1006d0158:     	lsr	w0, w23, #2
1006d015c:     	ldr	x1, [x19, #0x40]
1006d0160:     	cmp	x1, x0
1006d0164:     	b.ls	0x1006d05c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x740>
1006d0168:     	ldr	x8, [x19, #0x38]
1006d016c:     	add	x8, x8, x0, lsl #4
1006d0170:     	ldr	w8, [x8, #0x8]
1006d0174:     	and	w9, w23, #0x1
1006d0178:     	eor	w2, w8, w9
1006d017c:     	tbz	w24, #0x1, 0x1006d01d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x358>
1006d0180:     	lsr	w0, w24, #2
1006d0184:     	ldr	x1, [x19, #0x40]
1006d0188:     	cmp	x1, x0
1006d018c:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d0190:     	ldr	x8, [x19, #0x38]
1006d0194:     	lsl	x9, x0, #4
1006d0198:     	ldr	w8, [x8, x9]
1006d019c:     	mov	x3, x24
1006d01a0:     	cmp	x8, x21
1006d01a4:     	b.ne	0x1006d020c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x38c>
1006d01a8:     	b	0x1006d01e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x368>
1006d01ac:     	lsr	w0, w23, #2
1006d01b0:     	ldr	x1, [x19, #0x40]
1006d01b4:     	cmp	x1, x0
1006d01b8:     	b.ls	0x1006d0598 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x718>
1006d01bc:     	ldr	x8, [x19, #0x38]
1006d01c0:     	lsl	x9, x0, #4
1006d01c4:     	ldr	w8, [x8, x9]
1006d01c8:     	mov	x2, x23
1006d01cc:     	cmp	x8, x21
1006d01d0:     	b.eq	0x1006d0158 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x2d8>
1006d01d4:     	tbnz	w24, #0x1, 0x1006d0180 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x300>
1006d01d8:     	ldr	x8, [x19, #0xc8]
1006d01dc:     	mov	x3, x24
1006d01e0:     	cmp	x8, x21
1006d01e4:     	b.ne	0x1006d020c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x38c>
1006d01e8:     	lsr	w0, w24, #2
1006d01ec:     	ldr	x1, [x19, #0x40]
1006d01f0:     	cmp	x1, x0
1006d01f4:     	b.ls	0x1006d05c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x740>
1006d01f8:     	ldr	x8, [x19, #0x38]
1006d01fc:     	add	x8, x8, x0, lsl #4
1006d0200:     	ldr	w8, [x8, #0x8]
1006d0204:     	and	w9, w24, #0x1
1006d0208:     	eor	w3, w8, w9
1006d020c:     	mov	x0, x19
1006d0210:     	mov	x1, x20
1006d0214:     	bl	0x1006cfe80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_>
1006d0218:     	mov	x3, x0
1006d021c:     	mov	x0, x19
1006d0220:     	mov	x1, x21
1006d0224:     	mov	x2, x22
1006d0228:     	bl	0x1006cf234 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E2mkB6_>
1006d022c:     	b	0x1006d055c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6dc>
1006d0230:     	eor	w16, w16, #0xf
1006d0234:     	mvn	x10, x10
1006d0238:     	and	w17, w16, #0xff
1006d023c:     	cmp	w17, #0x7
1006d0240:     	b.le	0x1006d0260 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3e0>
1006d0244:     	cmp	w17, #0xb
1006d0248:     	b.gt	0x1006d027c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3fc>
1006d024c:     	cmp	w17, #0x8
1006d0250:     	b.eq	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0254:     	cmp	w17, #0xa
1006d0258:     	b.ne	0x1006d0230 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
1006d025c:     	b	0x1006d0510 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
1006d0260:     	cmp	w17, #0x2
1006d0264:     	b.eq	0x1006d0528 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a8>
1006d0268:     	cmp	w17, #0x4
1006d026c:     	b.eq	0x1006d0430 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5b0>
1006d0270:     	cmp	w17, #0x6
1006d0274:     	b.ne	0x1006d0230 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
1006d0278:     	b	0x1006d03d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x550>
1006d027c:     	cmp	w17, #0xc
1006d0280:     	b.eq	0x1006d0518 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
1006d0284:     	cmp	w17, #0xe
1006d0288:     	b.ne	0x1006d0230 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x3b0>
1006d028c:     	mov	x11, x15
1006d0290:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0294:     	orr	x1, x8, x9
1006d0298:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d029c:     	mov	x10, #0x0               ; =0
1006d02a0:     	eor	x11, x8, x9
1006d02a4:     	bic	x12, x9, x8
1006d02a8:     	and	x9, x8, x9
1006d02ac:     	mov	w13, #0xb               ; =11
1006d02b0:     	b	0x1006d02bc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x43c>
1006d02b4:     	eor	w13, w13, #0xf
1006d02b8:     	mvn	x10, x10
1006d02bc:     	and	w14, w13, #0xff
1006d02c0:     	cmp	w14, #0x7
1006d02c4:     	b.gt	0x1006d02dc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x45c>
1006d02c8:     	cmp	w14, #0x4
1006d02cc:     	b.eq	0x1006d0520 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a0>
1006d02d0:     	cmp	w14, #0x6
1006d02d4:     	b.ne	0x1006d02b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x434>
1006d02d8:     	b	0x1006d0484 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x604>
1006d02dc:     	cmp	w14, #0x8
1006d02e0:     	b.eq	0x1006d04f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
1006d02e4:     	cmp	w14, #0xa
1006d02e8:     	b.ne	0x1006d02b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x434>
1006d02ec:     	b	0x1006d0534 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
1006d02f0:     	bic	x1, x9, x8
1006d02f4:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d02f8:     	mov	x1, x9
1006d02fc:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0300:     	mov	x10, #0x0               ; =0
1006d0304:     	eor	x11, x8, x9
1006d0308:     	and	x8, x8, x9
1006d030c:     	mov	w9, #0x9                ; =9
1006d0310:     	and	w12, w9, #0xff
1006d0314:     	cmp	w12, #0x6
1006d0318:     	b.eq	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d031c:     	cmp	w12, #0x8
1006d0320:     	b.eq	0x1006d0510 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
1006d0324:     	eor	w9, w9, #0xf
1006d0328:     	mvn	x10, x10
1006d032c:     	and	w12, w9, #0xff
1006d0330:     	cmp	w12, #0x6
1006d0334:     	b.ne	0x1006d031c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x49c>
1006d0338:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d033c:     	bic	x1, x8, x9
1006d0340:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0344:     	mov	x10, #0x0               ; =0
1006d0348:     	orr	x12, x8, x9
1006d034c:     	and	x13, x8, x9
1006d0350:     	eor	x11, x8, x9
1006d0354:     	bic	x14, x9, x8
1006d0358:     	bic	x15, x8, x9
1006d035c:     	mov	w16, #0xf               ; =15
1006d0360:     	b	0x1006d036c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4ec>
1006d0364:     	eor	w16, w16, #0xf
1006d0368:     	mvn	x10, x10
1006d036c:     	and	w17, w16, #0xff
1006d0370:     	cmp	w17, #0x7
1006d0374:     	b.gt	0x1006d0390 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x510>
1006d0378:     	cmp	w17, #0x3
1006d037c:     	b.gt	0x1006d03ac <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x52c>
1006d0380:     	cbz	w17, 0x1006d054c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6cc>
1006d0384:     	cmp	w17, #0x2
1006d0388:     	b.ne	0x1006d0364 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
1006d038c:     	b	0x1006d028c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x40c>
1006d0390:     	cmp	w17, #0xb
1006d0394:     	b.gt	0x1006d03c0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x540>
1006d0398:     	cmp	w17, #0x8
1006d039c:     	b.eq	0x1006d0430 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5b0>
1006d03a0:     	cmp	w17, #0xa
1006d03a4:     	b.ne	0x1006d0364 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
1006d03a8:     	b	0x1006d0510 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
1006d03ac:     	cmp	w17, #0x4
1006d03b0:     	b.eq	0x1006d0528 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a8>
1006d03b4:     	cmp	w17, #0x6
1006d03b8:     	b.ne	0x1006d0364 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
1006d03bc:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d03c0:     	cmp	w17, #0xc
1006d03c4:     	b.eq	0x1006d0518 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
1006d03c8:     	cmp	w17, #0xe
1006d03cc:     	b.ne	0x1006d0364 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x4e4>
1006d03d0:     	mov	x11, x12
1006d03d4:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d03d8:     	mov	x10, #0x0               ; =0
1006d03dc:     	and	x12, x8, x9
1006d03e0:     	eor	x13, x8, x9
1006d03e4:     	bic	x11, x9, x8
1006d03e8:     	mov	w14, #0x3               ; =3
1006d03ec:     	b	0x1006d03f8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x578>
1006d03f0:     	eor	w14, w14, #0xf
1006d03f4:     	mvn	x10, x10
1006d03f8:     	and	w15, w14, #0xff
1006d03fc:     	cmp	w15, #0x7
1006d0400:     	b.le	0x1006d0420 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5a0>
1006d0404:     	cmp	w15, #0x8
1006d0408:     	b.eq	0x1006d03d0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x550>
1006d040c:     	cmp	w15, #0xa
1006d0410:     	b.eq	0x1006d0510 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x690>
1006d0414:     	cmp	w15, #0xc
1006d0418:     	b.ne	0x1006d03f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x570>
1006d041c:     	b	0x1006d0518 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x698>
1006d0420:     	cmp	w15, #0x4
1006d0424:     	b.eq	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0428:     	cmp	w15, #0x6
1006d042c:     	b.ne	0x1006d03f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x570>
1006d0430:     	mov	x11, x13
1006d0434:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0438:     	and	x8, x8, x9
1006d043c:     	mvn	x1, x8
1006d0440:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0444:     	mov	x10, #0x0               ; =0
1006d0448:     	eor	x11, x8, x9
1006d044c:     	and	x9, x8, x9
1006d0450:     	mov	w12, #0x5               ; =5
1006d0454:     	and	w13, w12, #0xff
1006d0458:     	cmp	w13, #0x6
1006d045c:     	b.eq	0x1006d0484 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x604>
1006d0460:     	cmp	w13, #0x8
1006d0464:     	b.eq	0x1006d04f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
1006d0468:     	cmp	w13, #0xa
1006d046c:     	b.eq	0x1006d0534 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
1006d0470:     	eor	w12, w12, #0xf
1006d0474:     	mvn	x10, x10
1006d0478:     	and	w13, w12, #0xff
1006d047c:     	cmp	w13, #0x6
1006d0480:     	b.ne	0x1006d0460 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x5e0>
1006d0484:     	mov	x9, x11
1006d0488:     	b	0x1006d04f0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x670>
1006d048c:     	mov	x10, #0x0               ; =0
1006d0490:     	and	x11, x8, x9
1006d0494:     	eor	x12, x8, x9
1006d0498:     	bic	x13, x9, x8
1006d049c:     	bic	x14, x8, x9
1006d04a0:     	mov	w15, #0xd               ; =13
1006d04a4:     	b	0x1006d04b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x630>
1006d04a8:     	eor	w15, w15, #0xf
1006d04ac:     	mvn	x10, x10
1006d04b0:     	and	w16, w15, #0xff
1006d04b4:     	cmp	w16, #0x7
1006d04b8:     	b.gt	0x1006d04d8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x658>
1006d04bc:     	cmp	w16, #0x2
1006d04c0:     	b.eq	0x1006d053c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6bc>
1006d04c4:     	cmp	w16, #0x4
1006d04c8:     	b.eq	0x1006d0544 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6c4>
1006d04cc:     	cmp	w16, #0x6
1006d04d0:     	b.ne	0x1006d04a8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x628>
1006d04d4:     	b	0x1006d0520 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6a0>
1006d04d8:     	cmp	w16, #0x8
1006d04dc:     	b.eq	0x1006d0530 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b0>
1006d04e0:     	cmp	w16, #0xa
1006d04e4:     	b.eq	0x1006d0534 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6b4>
1006d04e8:     	cmp	w16, #0xc
1006d04ec:     	b.ne	0x1006d04a8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x628>
1006d04f0:     	eor	x1, x9, x10
1006d04f4:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d04f8:     	eor	x1, x8, x9
1006d04fc:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0500:     	mov	x1, x8
1006d0504:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0508:     	and	x1, x8, x9
1006d050c:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0510:     	mov	x11, x8
1006d0514:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0518:     	mov	x11, x9
1006d051c:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0520:     	eor	x1, x12, x10
1006d0524:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0528:     	mov	x11, x14
1006d052c:     	b	0x1006d0550 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d0>
1006d0530:     	mov	x8, x11
1006d0534:     	eor	x1, x8, x10
1006d0538:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d053c:     	eor	x1, x14, x10
1006d0540:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d0544:     	eor	x1, x13, x10
1006d0548:     	b	0x1006d0554 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E6raw_opB6_+0x6d4>
1006d054c:     	mov	x11, #0x0               ; =0
1006d0550:     	eor	x1, x11, x10
1006d0554:     	mov	x0, x19
1006d0558:     	bl	0x1006cf78c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj1_E4leafB6_>
1006d055c:     	mov	x21, x0
1006d0560:     	strb	w20, [sp, #0x8]
1006d0564:     	str	w23, [sp, #0x4]
1006d0568:     	str	w24, [sp, #0xc]
1006d056c:     	add	x0, x19, #0xa0
1006d0570:     	add	x1, sp, #0x4
1006d0574:     	mov	x2, x21
1006d0578:     	bl	0x1007284d8 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapThmmEmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006d057c:     	mov	x0, x21
1006d0580:     	ldp	x29, x30, [sp, #0x40]
1006d0584:     	ldp	x20, x19, [sp, #0x30]
1006d0588:     	ldp	x22, x21, [sp, #0x20]
1006d058c:     	ldp	x24, x23, [sp, #0x10]
1006d0590:     	add	sp, sp, #0x50
1006d0594:     	ret
1006d0598:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d059c:     	add	x2, x2, #0x760
1006d05a0:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d05a4:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d05a8:     	add	x2, x2, #0x928
1006d05ac:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d05b0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d05b4:     	add	x2, x2, #0x928
1006d05b8:     	mov	x0, x8
1006d05bc:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d05c0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d05c4:     	add	x2, x2, #0x910
1006d05c8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d05cc:     	nop
1006d05d0:     	nop
1006d05d4:     	nop
1006d05d8:     	nop
1006d05dc:     	nop
1006d05e0:     	nop
1006d05e4:     	nop
1006d05e8:     	nop
1006d05ec:     	nop
1006d05f0:     	nop
1006d05f4:     	nop
1006d05f8:     	nop
1006d05fc:     	nop
