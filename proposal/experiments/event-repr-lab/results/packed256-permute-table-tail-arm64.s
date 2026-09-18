
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/4eb96717b825fda0/out/bumbledb-4eb96717b825fda0:	file format mach-o arm64

Disassembly of section __TEXT,__text:

00000001006d6d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>:
1006d6d88:     	stp	x26, x25, [sp, #-0x50]!
1006d6d8c:     	stp	x24, x23, [sp, #0x10]
1006d6d90:     	stp	x22, x21, [sp, #0x20]
1006d6d94:     	stp	x20, x19, [sp, #0x30]
1006d6d98:     	stp	x29, x30, [sp, #0x40]
1006d6d9c:     	add	x29, sp, #0x40
1006d6da0:     	cmp	x3, #0x1
1006d6da4:     	b.ne	0x1006d6dc4 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x3c>
1006d6da8:     	lsr	x0, x2, #6
1006d6dac:     	cmp	x2, #0x100
1006d6db0:     	b.hs	0x1006d6eb0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x128>
1006d6db4:     	ldr	x8, [x1, x0, lsl #3]
1006d6db8:     	lsr	x8, x8, x2
1006d6dbc:     	and	w22, w8, #0x1
1006d6dc0:     	b	0x1006d6e3c <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0xb4>
1006d6dc4:     	mov	x21, x5
1006d6dc8:     	mov	x20, x4
1006d6dcc:     	mov	x19, x0
1006d6dd0:     	lsr	x23, x3, #1
1006d6dd4:     	mov	x24, x1
1006d6dd8:     	mov	x25, x2
1006d6ddc:     	mov	x26, x3
1006d6de0:     	mov	x3, x23
1006d6de4:     	bl	0x1006d6d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>
1006d6de8:     	mov	x22, x0
1006d6dec:     	add	x2, x23, x25
1006d6df0:     	mov	x0, x19
1006d6df4:     	mov	x1, x24
1006d6df8:     	mov	x3, x23
1006d6dfc:     	mov	x4, x20
1006d6e00:     	mov	x5, x21
1006d6e04:     	bl	0x1006d6d88 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_>
1006d6e08:     	mov	x23, x0
1006d6e0c:     	rbit	x8, x26
1006d6e10:     	clz	x8, x8
1006d6e14:     	sub	x0, x8, #0x1
1006d6e18:     	ldr	x1, [x19, #0x28]
1006d6e1c:     	cmp	x0, x1
1006d6e20:     	b.hs	0x1006d6ec0 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x138>
1006d6e24:     	ldr	x8, [x19, #0x20]
1006d6e28:     	ldr	w0, [x8, x0, lsl #2]
1006d6e2c:     	cmp	x21, x0
1006d6e30:     	b.ls	0x1006d6ecc <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0x144>
1006d6e34:     	cmp	w22, w23
1006d6e38:     	b.ne	0x1006d6e58 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E13permute_tableB6_+0xd0>
1006d6e3c:     	mov	x0, x22
1006d6e40:     	ldp	x29, x30, [sp, #0x40]
1006d6e44:     	ldp	x20, x19, [sp, #0x30]
1006d6e48:     	ldp	x22, x21, [sp, #0x20]
1006d6e4c:     	ldp	x24, x23, [sp, #0x10]
1006d6e50:     	ldp	x26, x25, [sp], #0x50
1006d6e54:     	ret
1006d6e58:     	ldr	w20, [x20, x0, lsl #2]
1006d6e5c:     	mov	x0, x19
1006d6e60:     	mov	w1, #0x2                ; =2
1006d6e64:     	mov	x2, x20
1006d6e68:     	mov	x3, x22
1006d6e6c:     	bl	0x1006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1006d6e70:     	mov	x21, x0
1006d6e74:     	mov	x0, x19
1006d6e78:     	mov	w1, #0x8                ; =8
1006d6e7c:     	mov	x2, x20
1006d6e80:     	mov	x3, x23
1006d6e84:     	bl	0x1006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1006d6e88:     	mov	x3, x0
1006d6e8c:     	mov	x0, x19
1006d6e90:     	mov	w1, #0xe                ; =14
1006d6e94:     	mov	x2, x21
1006d6e98:     	ldp	x29, x30, [sp, #0x40]
1006d6e9c:     	ldp	x20, x19, [sp, #0x30]
1006d6ea0:     	ldp	x22, x21, [sp, #0x20]
1006d6ea4:     	ldp	x24, x23, [sp, #0x10]
1006d6ea8:     	ldp	x26, x25, [sp], #0x50
1006d6eac:     	b	0x1006d7c80 <__RNvMNtNtCscwNfrOFzE52_8bumbledb14event_repr_lab6packedINtB2_6PackedKj4_E6raw_opB6_>
1006d6eb0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d6eb4:     	add	x2, x2, #0x808
1006d6eb8:     	mov	w1, #0x4                ; =4
1006d6ebc:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d6ec0:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d6ec4:     	add	x2, x2, #0x820
1006d6ec8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
1006d6ecc:     	adrp	x2, 0x100ec6000 <dyld_stub_binder+0x100ec6000>
1006d6ed0:     	add	x2, x2, #0x838
1006d6ed4:     	mov	x1, x21
1006d6ed8:     	bl	0x100c9b09c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
