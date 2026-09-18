
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100805c70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E20build_physical_wordsB6_>:
100805c70:     	sub	sp, sp, #0xc0
100805c74:     	stp	x24, x23, [sp, #0x80]
100805c78:     	stp	x22, x21, [sp, #0x90]
100805c7c:     	stp	x20, x19, [sp, #0xa0]
100805c80:     	stp	x29, x30, [sp, #0xb0]
100805c84:     	add	x29, sp, #0xb0
100805c88:     	ldr	x8, [x0, #0xc8]
100805c8c:     	cmp	x3, x8
100805c90:     	b.ne	0x100805cec <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E20build_physical_wordsB6_+0x7c>
100805c94:     	movi.2d	v0, #0000000000000000
100805c98:     	stp	q0, q0, [sp, #0x20]
100805c9c:     	stp	q0, q0, [sp]
100805ca0:     	cmp	x2, #0x9
100805ca4:     	b.hs	0x100805d4c <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E20build_physical_wordsB6_+0xdc>
100805ca8:     	lsl	x2, x2, #3
100805cac:     	mov	x19, x0
100805cb0:     	mov	x0, sp
100805cb4:     	bl	0x100e8a11c <dyld_stub_binder+0x100e8a11c>
100805cb8:     	ldp	q0, q1, [sp]
100805cbc:     	stp	q0, q1, [sp, #0x40]
100805cc0:     	ldp	q0, q1, [sp, #0x20]
100805cc4:     	stp	q0, q1, [sp, #0x60]
100805cc8:     	add	x1, sp, #0x40
100805ccc:     	mov	x0, x19
100805cd0:     	bl	0x1008062d8 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E4leafB6_>
100805cd4:     	ldp	x29, x30, [sp, #0xb0]
100805cd8:     	ldp	x20, x19, [sp, #0xa0]
100805cdc:     	ldp	x22, x21, [sp, #0x90]
100805ce0:     	ldp	x24, x23, [sp, #0x80]
100805ce4:     	add	sp, sp, #0xc0
100805ce8:     	ret
100805cec:     	lsr	x19, x2, #1
100805cf0:     	mov	x20, x3
100805cf4:     	add	x3, x3, #0x1
100805cf8:     	mov	x21, x0
100805cfc:     	mov	x22, x1
100805d00:     	mov	x24, x2
100805d04:     	mov	x2, x19
100805d08:     	bl	0x100805c70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E20build_physical_wordsB6_>
100805d0c:     	mov	x23, x0
100805d10:     	sub	x2, x24, x19
100805d14:     	add	x1, x22, x19, lsl #3
100805d18:     	add	x3, x20, #0x1
100805d1c:     	mov	x0, x21
100805d20:     	bl	0x100805c70 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E20build_physical_wordsB6_>
100805d24:     	mov	x3, x0
100805d28:     	mov	x0, x21
100805d2c:     	mov	x1, x20
100805d30:     	mov	x2, x23
100805d34:     	ldp	x29, x30, [sp, #0xb0]
100805d38:     	ldp	x20, x19, [sp, #0xa0]
100805d3c:     	ldp	x22, x21, [sp, #0x90]
100805d40:     	ldp	x24, x23, [sp, #0x80]
100805d44:     	add	sp, sp, #0xc0
100805d48:     	b	0x100805d64 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab6packedINtB2_6PackedKj8_E2mkB6_>
100805d4c:     	adrp	x3, 0x1010cf000 <dyld_stub_binder+0x1010cf000>
100805d50:     	add	x3, x3, #0x8e0
100805d54:     	mov	x0, #0x0                ; =0
100805d58:     	mov	x1, x2
100805d5c:     	mov	w2, #0x8                ; =8
100805d60:     	bl	0x100e81b94 <__RNvNtNtCs4sDCw1iE1MS_4core5slice5index16slice_index_fail>
