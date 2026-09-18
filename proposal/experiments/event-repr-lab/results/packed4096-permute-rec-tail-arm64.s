
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006d0f14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_>:
1006d0f14:     	stp	x28, x27, [sp, #-0x60]!
1006d0f18:     	stp	x26, x25, [sp, #0x10]
1006d0f1c:     	stp	x24, x23, [sp, #0x20]
1006d0f20:     	stp	x22, x21, [sp, #0x30]
1006d0f24:     	stp	x20, x19, [sp, #0x40]
1006d0f28:     	stp	x29, x30, [sp, #0x50]
1006d0f2c:     	add	x29, sp, #0x50
1006d0f30:     	sub	sp, sp, #0x410
1006d0f34:     	ldr	xzr, [sp]
1006d0f38:     	mov	x21, x1
1006d0f3c:     	cmp	w1, #0x2
1006d0f40:     	b.hs	0x1006d0f4c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x38>
1006d0f44:     	mov	w28, #0x0               ; =0
1006d0f48:     	b	0x1006d11c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2b0>
1006d0f4c:     	mov	x19, x5
1006d0f50:     	and	w24, w21, #0xfffffffe
1006d0f54:     	and	w28, w21, #0x1
1006d0f58:     	ldr	x8, [x5, #0x18]
1006d0f5c:     	cbz	x8, 0x1006d1000 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0xec>
1006d0f60:     	mov	x8, #0x0                ; =0
1006d0f64:     	mov	x9, #0xa9c5             ; =43461
1006d0f68:     	movk	x9, #0x2e62, lsl #16
1006d0f6c:     	movk	x9, #0x7aea, lsl #32
1006d0f70:     	movk	x9, #0xf135, lsl #48
1006d0f74:     	mul	x9, x24, x9
1006d0f78:     	ror	x11, x9, #0x2c
1006d0f7c:     	lsr	x12, x11, #57
1006d0f80:     	ldp	x10, x9, [x19]
1006d0f84:     	dup.8b	v0, w12
1006d0f88:     	movi.2d	v1, #0xffffffffffffffff
1006d0f8c:     	and	x11, x11, x9
1006d0f90:     	ldr	d2, [x10, x11]
1006d0f94:     	cmeq.8b	v3, v2, v0
1006d0f98:     	fmov	x12, d3
1006d0f9c:     	ands	x12, x12, #0x8080808080808080
1006d0fa0:     	b.eq	0x1006d0fd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0xbc>
1006d0fa4:     	rbit	x13, x12
1006d0fa8:     	clz	x13, x13
1006d0fac:     	add	x13, x11, x13, lsr #3
1006d0fb0:     	and	x13, x13, x9
1006d0fb4:     	sub	x13, x10, x13, lsl #3
1006d0fb8:     	ldur	w14, [x13, #-0x8]
1006d0fbc:     	cmp	w24, w14
1006d0fc0:     	b.eq	0x1006d1078 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x164>
1006d0fc4:     	sub	x13, x12, #0x2
1006d0fc8:     	ands	x12, x13, x12
1006d0fcc:     	b.ne	0x1006d0fa4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x90>
1006d0fd0:     	cmeq.8b	v2, v2, v1
1006d0fd4:     	fmov	x12, d2
1006d0fd8:     	cbnz	x12, 0x1006d1000 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0xec>
1006d0fdc:     	add	x8, x8, #0x8
1006d0fe0:     	add	x11, x11, x8
1006d0fe4:     	and	x11, x11, x9
1006d0fe8:     	ldr	d2, [x10, x11]
1006d0fec:     	cmeq.8b	v3, v2, v0
1006d0ff0:     	fmov	x12, d3
1006d0ff4:     	ands	x12, x12, #0x8080808080808080
1006d0ff8:     	b.ne	0x1006d0fa4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x90>
1006d0ffc:     	b	0x1006d0fd0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0xbc>
1006d1000:     	tbnz	w21, #0x1, 0x1006d1080 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x16c>
1006d1004:     	lsr	w8, w21, #2
1006d1008:     	cbz	x4, 0x1006d1168 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x254>
1006d100c:     	ldr	x1, [x0, #0x58]
1006d1010:     	cmp	x1, x8
1006d1014:     	b.ls	0x1006d1210 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2fc>
1006d1018:     	mov	x22, x4
1006d101c:     	ldr	x9, [x0, #0x50]
1006d1020:     	add	x1, x9, x8, lsl #9
1006d1024:     	mov	x20, x0
1006d1028:     	add	x0, sp, #0x8
1006d102c:     	mov	w2, #0x200              ; =512
1006d1030:     	bl	0x100ca35d8 <dyld_stub_binder+0x100ca35d8>
1006d1034:     	ldr	x8, [x20, #0xd0]
1006d1038:     	lsr	x9, x8, #6
1006d103c:     	tst	x8, #0x3f
1006d1040:     	cinc	x2, x9, ne
1006d1044:     	cmp	x2, #0x41
1006d1048:     	b.hs	0x1006d11e8 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2d4>
1006d104c:     	add	x1, sp, #0x8
1006d1050:     	mov	x0, x22
1006d1054:     	bl	0x1006de424 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab7carrierNtB2_11Permutation5dense>
1006d1058:     	add	x0, sp, #0x208
1006d105c:     	add	x1, sp, #0x8
1006d1060:     	mov	w2, #0x200              ; =512
1006d1064:     	bl	0x100ca35d8 <dyld_stub_binder+0x100ca35d8>
1006d1068:     	add	x1, sp, #0x208
1006d106c:     	mov	x0, x20
1006d1070:     	bl	0x1006d2b64 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E4leafB6_>
1006d1074:     	b	0x1006d11b0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x29c>
1006d1078:     	ldur	w21, [x13, #-0x4]
1006d107c:     	b	0x1006d11c4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2b0>
1006d1080:     	and	w8, w21, #0xfffffffe
1006d1084:     	str	x8, [sp]
1006d1088:     	lsr	w8, w21, #2
1006d108c:     	ldr	x1, [x0, #0x40]
1006d1090:     	cmp	x1, x8
1006d1094:     	b.ls	0x1006d1200 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2ec>
1006d1098:     	ldr	x9, [x0, #0x38]
1006d109c:     	add	x8, x9, x8, lsl #4
1006d10a0:     	ldp	w22, w1, [x8]
1006d10a4:     	ldr	w23, [x8, #0x8]
1006d10a8:     	mov	x24, x0
1006d10ac:     	mov	x25, x2
1006d10b0:     	mov	x26, x3
1006d10b4:     	mov	x27, x4
1006d10b8:     	mov	x5, x19
1006d10bc:     	bl	0x1006d0f14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_>
1006d10c0:     	mov	x21, x0
1006d10c4:     	mov	x0, x24
1006d10c8:     	mov	x1, x23
1006d10cc:     	mov	x20, x25
1006d10d0:     	mov	x2, x25
1006d10d4:     	mov	x25, x26
1006d10d8:     	mov	x3, x26
1006d10dc:     	mov	x4, x27
1006d10e0:     	mov	x5, x19
1006d10e4:     	bl	0x1006d0f14 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_>
1006d10e8:     	ldr	x1, [x24, #0x10]
1006d10ec:     	cmp	x1, x22
1006d10f0:     	b.ls	0x1006d1220 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x30c>
1006d10f4:     	mov	x23, x0
1006d10f8:     	ldr	x8, [x24, #0x8]
1006d10fc:     	ldr	w0, [x8, x22, lsl #2]
1006d1100:     	cmp	x25, x0
1006d1104:     	b.ls	0x1006d1230 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x31c>
1006d1108:     	cmp	w21, w23
1006d110c:     	b.ne	0x1006d1118 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x204>
1006d1110:     	ldr	x24, [sp]
1006d1114:     	b	0x1006d11b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2a0>
1006d1118:     	ldr	w22, [x20, x0, lsl #2]
1006d111c:     	mov	x0, x24
1006d1120:     	mov	w1, #0x2                ; =2
1006d1124:     	mov	x2, x22
1006d1128:     	mov	x3, x21
1006d112c:     	bl	0x1006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>
1006d1130:     	mov	x21, x0
1006d1134:     	mov	x0, x24
1006d1138:     	mov	w1, #0x8                ; =8
1006d113c:     	mov	x2, x22
1006d1140:     	mov	x3, x23
1006d1144:     	bl	0x1006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>
1006d1148:     	mov	x3, x0
1006d114c:     	mov	x0, x24
1006d1150:     	mov	w1, #0xe                ; =14
1006d1154:     	mov	x2, x21
1006d1158:     	bl	0x1006d3600 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E6raw_opB6_>
1006d115c:     	mov	x21, x0
1006d1160:     	ldr	x24, [sp]
1006d1164:     	b	0x1006d11b4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2a0>
1006d1168:     	ldr	x1, [x0, #0x58]
1006d116c:     	cmp	x1, x8
1006d1170:     	b.ls	0x1006d1210 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E11permute_recB6_+0x2fc>
1006d1174:     	mov	x21, x3
1006d1178:     	mov	x22, x2
1006d117c:     	ldr	x9, [x0, #0x50]
1006d1180:     	add	x1, x9, x8, lsl #9
1006d1184:     	mov	x23, x0
1006d1188:     	add	x0, sp, #0x208
1006d118c:     	mov	w2, #0x200              ; =512
1006d1190:     	bl	0x100ca35d8 <dyld_stub_binder+0x100ca35d8>
1006d1194:     	ldr	x3, [x23, #0xd0]
1006d1198:     	add	x1, sp, #0x208
1006d119c:     	mov	x0, x23
1006d11a0:     	mov	x2, #0x0                ; =0
1006d11a4:     	mov	x4, x22
1006d11a8:     	mov	x5, x21
1006d11ac:     	bl	0x1006d249c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj40_E13permute_tableB6_>
1006d11b0:     	mov	x21, x0
1006d11b4:     	mov	x0, x19
1006d11b8:     	mov	x1, x24
1006d11bc:     	mov	x2, x21
1006d11c0:     	bl	0x100729484 <__RNvMs1_NtCsbXQ38keiWF6_9hashbrown3mapINtB5_7HashMapmmNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherE6insertCscwNfrOFzE52_8bumbledb>
1006d11c4:     	eor	w0, w21, w28
1006d11c8:     	add	sp, sp, #0x410
1006d11cc:     	ldp	x29, x30, [sp, #0x50]
1006d11d0:     	ldp	x20, x19, [sp, #0x40]
1006d11d4:     	ldp	x22, x21, [sp, #0x30]
1006d11d8:     	ldp	x24, x23, [sp, #0x20]
1006d11dc:     	ldp	x26, x25, [sp, #0x10]
1006d11e0:     	ldp	x28, x27, [sp], #0x60
1006d11e4:     	ret
1006d11e8:     	adrp	x3, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d11ec:     	add	x3, x3, #0x700
1006d11f0:     	mov	x0, #0x0                ; =0
1006d11f4:     	mov	x1, x2
1006d11f8:     	mov	w2, #0x40               ; =64
1006d11fc:     	bl	0x100c9afd4 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
1006d1200:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d1204:     	add	x2, x2, #0x718
1006d1208:     	mov	x0, x8
1006d120c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d1210:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d1214:     	add	x2, x2, #0x928
1006d1218:     	mov	x0, x8
1006d121c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d1220:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d1224:     	add	x2, x2, #0x730
1006d1228:     	mov	x0, x22
1006d122c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d1230:     	mov	x1, x25
1006d1234:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d1238:     	add	x2, x2, #0x748
1006d123c:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
