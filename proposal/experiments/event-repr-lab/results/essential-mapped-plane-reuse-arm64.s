
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>:
100bbad68:     	sub	sp, sp, #0x1e0
100bbad6c:     	stp	d15, d14, [sp, #0x140]
100bbad70:     	stp	d13, d12, [sp, #0x150]
100bbad74:     	stp	d11, d10, [sp, #0x160]
100bbad78:     	stp	d9, d8, [sp, #0x170]
100bbad7c:     	stp	x28, x27, [sp, #0x180]
100bbad80:     	stp	x26, x25, [sp, #0x190]
100bbad84:     	stp	x24, x23, [sp, #0x1a0]
100bbad88:     	stp	x22, x21, [sp, #0x1b0]
100bbad8c:     	stp	x20, x19, [sp, #0x1c0]
100bbad90:     	stp	x29, x30, [sp, #0x1d0]
100bbad94:     	add	x29, sp, #0x1d0
100bbad98:     	str	x6, [sp, #0x98]
100bbad9c:     	mov	x24, x5
100bbada0:     	mov	x20, x4
100bbada4:     	mov	x27, x3
100bbada8:     	mov	x25, x2
100bbadac:     	mov	x23, x1
100bbadb0:     	mov	x28, x0
100bbadb4:     	str	x4, [sp, #0xb8]
100bbadb8:     	ldr	x21, [x2]
100bbadbc:     	ldp	x22, x26, [x3, #0x8]
100bbadc0:     	cbz	x21, 0x100bbae04 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9c>
100bbadc4:     	mov	x8, #0x0                ; =0
100bbadc8:     	mov	w9, #0x1                ; =1
100bbadcc:     	mov	x10, x21
100bbadd0:     	rbit	x11, x10
100bbadd4:     	clz	x0, x11
100bbadd8:     	cmp	x0, x26
100bbaddc:     	b.hs	0x100bbbb78 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe10>
100bbade0:     	ldr	w11, [x22, x0, lsl #2]
100bbade4:     	lsl	x11, x9, x11
100bbade8:     	orr	x8, x11, x8
100bbadec:     	sub	x11, x10, #0x1
100bbadf0:     	ands	x10, x11, x10
100bbadf4:     	b.ne	0x100bbadd0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x68>
100bbadf8:     	ands	x8, x8, x20
100bbadfc:     	stur	x8, [x29, #-0xb8]
100bbae00:     	b.ne	0x100bbbab0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd48>
100bbae04:     	ldr	w2, [x25, #0x10]
100bbae08:     	sub	x0, x29, #0xb8
100bbae0c:     	add	x1, x23, #0x40
100bbae10:     	str	x2, [sp, #0x78]
100bbae14:     	bl	0x100d9d3c0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100bbae18:     	ldur	w8, [x29, #-0xb8]
100bbae1c:     	cbz	w8, 0x100bbaea4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x13c>
100bbae20:     	cmp	w8, #0x1
100bbae24:     	str	x20, [sp, #0x60]
100bbae28:     	b.ne	0x100bbaebc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x154>
100bbae2c:     	ldp	x23, x8, [x29, #-0xb0]
100bbae30:     	str	x8, [sp, #0xa0]
100bbae34:     	ldur	x27, [x29, #-0xa0]
100bbae38:     	mov	w8, #0x4                ; =4
100bbae3c:     	stp	xzr, x8, [x29, #-0xb8]
100bbae40:     	stur	xzr, [x29, #-0xa8]
100bbae44:     	str	x28, [sp, #0x58]
100bbae48:     	cbz	x21, 0x100bbb1e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x478>
100bbae4c:     	mov	x28, #0x0               ; =0
100bbae50:     	mov	w8, #0x4                ; =4
100bbae54:     	mov	w9, #0x1                ; =1
100bbae58:     	b	0x100bbae84 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x11c>
100bbae5c:     	ldur	x8, [x29, #-0xb0]
100bbae60:     	rbit	x9, x21
100bbae64:     	clz	x9, x9
100bbae68:     	str	w9, [x8, x28]
100bbae6c:     	stur	x19, [x29, #-0xa8]
100bbae70:     	sub	x10, x21, #0x1
100bbae74:     	add	x28, x28, #0x4
100bbae78:     	add	x9, x19, #0x1
100bbae7c:     	ands	x21, x10, x21
100bbae80:     	b.eq	0x100bbb0c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x360>
100bbae84:     	mov	x19, x9
100bbae88:     	sub	x9, x9, #0x1
100bbae8c:     	ldur	x10, [x29, #-0xb8]
100bbae90:     	cmp	x9, x10
100bbae94:     	b.ne	0x100bbae60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf8>
100bbae98:     	sub	x0, x29, #0xb8
100bbae9c:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bbaea0:     	b	0x100bbae5c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf4>
100bbaea4:     	ldr	x8, [sp, #0x78]
100bbaea8:     	and	w8, w8, #0x1
100bbaeac:     	strb	w8, [x28, #0x8]
100bbaeb0:     	mov	x8, #-0x2               ; =-2
100bbaeb4:     	str	x8, [x28]
100bbaeb8:     	b	0x100bbba80 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bbaebc:     	ldp	w19, w8, [x29, #-0xb4]
100bbaec0:     	ldur	w9, [x29, #-0xac]
100bbaec4:     	ldr	x11, [sp, #0x98]
100bbaec8:     	ldr	x10, [x11, #0x38]
100bbaecc:     	add	x10, x10, #0x1
100bbaed0:     	str	x10, [x11, #0x38]
100bbaed4:     	tbz	w24, #0x0, 0x100bbb188 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x420>
100bbaed8:     	lsr	x10, x21, x19
100bbaedc:     	and	x11, x10, #0x1
100bbaee0:     	stur	x11, [x29, #-0xb8]
100bbaee4:     	tbnz	w10, #0x0, 0x100bbbb14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdac>
100bbaee8:     	ldr	x10, [sp, #0x78]
100bbaeec:     	and	w10, w10, #0x1
100bbaef0:     	eor	w8, w8, w10
100bbaef4:     	eor	w20, w9, w10
100bbaef8:     	stur	w8, [x29, #-0xa8]
100bbaefc:     	ldr	x24, [x25, #0x8]
100bbaf00:     	stp	x21, x24, [x29, #-0xb8]
100bbaf04:     	add	x0, sp, #0xc0
100bbaf08:     	sub	x1, x29, #0xb8
100bbaf0c:     	mov	x2, x23
100bbaf10:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100bbaf14:     	stur	w20, [x29, #-0xa8]
100bbaf18:     	stp	x21, x24, [x29, #-0xb8]
100bbaf1c:     	add	x0, sp, #0xd8
100bbaf20:     	sub	x1, x29, #0xb8
100bbaf24:     	mov	x2, x23
100bbaf28:     	bl	0x10074faac <__RINvMs1_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_productNtB6_10Restricted9normalizeKm1_EBc_>
100bbaf2c:     	sub	x0, x29, #0xe0
100bbaf30:     	add	x2, sp, #0xc0
100bbaf34:     	mov	x1, x23
100bbaf38:     	mov	x3, x27
100bbaf3c:     	ldr	x21, [sp, #0x60]
100bbaf40:     	mov	x4, x21
100bbaf44:     	mov	w5, #0x1                ; =1
100bbaf48:     	ldr	x20, [sp, #0x98]
100bbaf4c:     	mov	x6, x20
100bbaf50:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100bbaf54:     	sub	x0, x29, #0xb8
100bbaf58:     	add	x2, sp, #0xd8
100bbaf5c:     	mov	x1, x23
100bbaf60:     	mov	x3, x27
100bbaf64:     	mov	x4, x21
100bbaf68:     	mov	w5, #0x1                ; =1
100bbaf6c:     	mov	x6, x20
100bbaf70:     	bl	0x100bbad68 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_>
100bbaf74:     	cmp	x26, x19
100bbaf78:     	b.ls	0x100bbbc08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xea0>
100bbaf7c:     	ldr	w21, [x22, x19, lsl #2]
100bbaf80:     	ldr	x10, [sp, #0x60]
100bbaf84:     	lsr	x8, x10, x21
100bbaf88:     	and	x9, x8, #0x1
100bbaf8c:     	stur	x9, [x29, #-0xc0]
100bbaf90:     	tbz	w8, #0x0, 0x100bbbb34 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdcc>
100bbaf94:     	fmov	d0, x10
100bbaf98:     	cnt.8b	v0, v0
100bbaf9c:     	addv.8b	b0, v0
100bbafa0:     	fmov	x8, d0
100bbafa4:     	and	x9, x8, #0x3f
100bbafa8:     	mov	w10, #0x1               ; =1
100bbafac:     	lsl	x8, x10, x8
100bbafb0:     	lsr	x8, x8, #6
100bbafb4:     	cmp	x9, #0x6
100bbafb8:     	cinc	x20, x8, lo
100bbafbc:     	cbz	x20, 0x100bbb820 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xab8>
100bbafc0:     	lsl	x19, x20, #3
100bbafc4:     	mov	x0, x19
100bbafc8:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
100bbafcc:     	cbz	x0, 0x100bbbc30 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xec8>
100bbafd0:     	mov	x9, #0x0                ; =0
100bbafd4:     	and	x8, x21, #0x3f
100bbafd8:     	mov	x10, #-0x1              ; =-1
100bbafdc:     	lsl	x8, x10, x8
100bbafe0:     	ldr	x10, [sp, #0x60]
100bbafe4:     	bic	x8, x10, x8
100bbafe8:     	fmov	d0, x8
100bbafec:     	cnt.8b	v0, v0
100bbaff0:     	addv.8b	b0, v0
100bbaff4:     	fmov	x10, d0
100bbaff8:     	add	w8, w10, #0x3a
100bbaffc:     	mov	w11, #0x1               ; =1
100bbb000:     	lsl	x11, x11, x8
100bbb004:     	ldp	x12, x13, [x29, #-0xe0]
100bbb008:     	ldp	x1, x14, [x29, #-0xd0]
100bbb00c:     	sub	x15, x9, w13, uxtb
100bbb010:     	ldp	x21, x17, [x29, #-0xb8]
100bbb014:     	ldp	x16, x2, [x29, #-0xa8]
100bbb018:     	adrp	x3, 0x101462000 <dyld_stub_binder+0x101462000>
100bbb01c:     	add	x3, x3, #0x4e8
100bbb020:     	mov	x8, #0x0                ; =0
100bbb024:     	b	0x100bbb048 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100bbb028:     	tst	w17, #0x1
100bbb02c:     	csel	x6, x4, x9, ne
100bbb030:     	bic	x4, x5, x4
100bbb034:     	orr	x4, x6, x4
100bbb038:     	str	x4, [x0, x8, lsl #3]
100bbb03c:     	add	x8, x8, #0x1
100bbb040:     	cmp	x20, x8
100bbb044:     	b.eq	0x100bbb0c0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x358>
100bbb048:     	cmp	x10, #0x6
100bbb04c:     	b.hs	0x100bbb068 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x300>
100bbb050:     	ldr	x4, [x3, x10, lsl #3]
100bbb054:     	mvn	x4, x4
100bbb058:     	mov	x5, x15
100bbb05c:     	cmn	x12, #0x2
100bbb060:     	b.ne	0x100bbb07c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x314>
100bbb064:     	b	0x100bbb08c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100bbb068:     	tst	x8, x11
100bbb06c:     	csetm	x4, ne
100bbb070:     	mov	x5, x15
100bbb074:     	cmn	x12, #0x2
100bbb078:     	b.eq	0x100bbb08c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x324>
100bbb07c:     	cmp	x8, x1
100bbb080:     	b.hs	0x100bbbbe0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe78>
100bbb084:     	ldr	x5, [x13, x8, lsl #3]
100bbb088:     	eor	x5, x14, x5
100bbb08c:     	cmn	x21, #0x2
100bbb090:     	b.eq	0x100bbb028 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2c0>
100bbb094:     	cmp	x8, x16
100bbb098:     	b.hs	0x100bbbbd4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe6c>
100bbb09c:     	ldr	x6, [x17, x8, lsl #3]
100bbb0a0:     	eor	x6, x2, x6
100bbb0a4:     	and	x6, x6, x4
100bbb0a8:     	bic	x4, x5, x4
100bbb0ac:     	orr	x4, x6, x4
100bbb0b0:     	str	x4, [x0, x8, lsl #3]
100bbb0b4:     	add	x8, x8, #0x1
100bbb0b8:     	cmp	x20, x8
100bbb0bc:     	b.ne	0x100bbb048 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x2e0>
100bbb0c0:     	mov	x8, x20
100bbb0c4:     	b	0x100bbb82c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xac4>
100bbb0c8:     	ldp	x8, x21, [x29, #-0xb8]
100bbb0cc:     	str	x8, [sp, #0x40]
100bbb0d0:     	str	x21, [sp, #0x30]
100bbb0d4:     	cbz	x19, 0x100bbb248 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e0>
100bbb0d8:     	ldr	x8, [x25, #0x8]
100bbb0dc:     	str	x8, [sp, #0x80]
100bbb0e0:     	mov	x24, #-0x1              ; =-1
100bbb0e4:     	mov	x19, x23
100bbb0e8:     	b	0x100bbb114 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x3ac>
100bbb0ec:     	bic	x27, x27, x23
100bbb0f0:     	ldr	x9, [sp, #0x98]
100bbb0f4:     	ldr	x8, [x9, #0x20]
100bbb0f8:     	add	x8, x8, #0x1
100bbb0fc:     	str	x8, [x9, #0x20]
100bbb100:     	mov	x24, x20
100bbb104:     	mov	x23, x25
100bbb108:     	mov	x19, x25
100bbb10c:     	subs	x28, x28, #0x4
100bbb110:     	b.eq	0x100bbb24c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4e4>
100bbb114:     	ldr	w8, [x21], #0x4
100bbb118:     	mov	w9, #0x1                ; =1
100bbb11c:     	lsl	x23, x9, x8
100bbb120:     	sub	x9, x23, #0x1
100bbb124:     	and	x9, x9, x27
100bbb128:     	fmov	d0, x9
100bbb12c:     	cnt.8b	v0, v0
100bbb130:     	addv.8b	b0, v0
100bbb134:     	fmov	w4, s0
100bbb138:     	fmov	d0, x27
100bbb13c:     	cnt.8b	v0, v0
100bbb140:     	addv.8b	b0, v0
100bbb144:     	fmov	w3, s0
100bbb148:     	ldr	x9, [sp, #0x80]
100bbb14c:     	lsr	x8, x9, x8
100bbb150:     	sub	x0, x29, #0xb8
100bbb154:     	and	w5, w8, #0x1
100bbb158:     	mov	x1, x19
100bbb15c:     	ldr	x2, [sp, #0xa0]
100bbb160:     	bl	0x100e44498 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels8cofactor>
100bbb164:     	ldp	x20, x25, [x29, #-0xb8]
100bbb168:     	ldur	x8, [x29, #-0xa8]
100bbb16c:     	str	x8, [sp, #0xa0]
100bbb170:     	sub	x8, x24, #0x1
100bbb174:     	cmn	x8, #0x3
100bbb178:     	b.hi	0x100bbb0ec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100bbb17c:     	mov	x0, x19
100bbb180:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb184:     	b	0x100bbb0ec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x384>
100bbb188:     	mov	w8, #0x4                ; =4
100bbb18c:     	stp	xzr, x8, [x29, #-0xb8]
100bbb190:     	stur	xzr, [x29, #-0xa8]
100bbb194:     	mov	x26, #0x0               ; =0
100bbb198:     	cbz	x20, 0x100bbb48c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x724>
100bbb19c:     	mov	w8, #0x4                ; =4
100bbb1a0:     	b	0x100bbb1c8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x460>
100bbb1a4:     	ldur	x8, [x29, #-0xb0]
100bbb1a8:     	rbit	x9, x20
100bbb1ac:     	clz	x9, x9
100bbb1b0:     	str	w9, [x8, x26, lsl #2]
100bbb1b4:     	add	x26, x26, #0x1
100bbb1b8:     	stur	x26, [x29, #-0xa8]
100bbb1bc:     	sub	x9, x20, #0x1
100bbb1c0:     	ands	x20, x9, x20
100bbb1c4:     	b.eq	0x100bbb1f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x488>
100bbb1c8:     	ldur	x9, [x29, #-0xb8]
100bbb1cc:     	cmp	x26, x9
100bbb1d0:     	b.ne	0x100bbb1a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x440>
100bbb1d4:     	sub	x0, x29, #0xb8
100bbb1d8:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bbb1dc:     	b	0x100bbb1a4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x43c>
100bbb1e0:     	mov	x19, #-0x1              ; =-1
100bbb1e4:     	mov	x21, #0x0               ; =0
100bbb1e8:     	cbnz	x27, 0x100bbb26c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x504>
100bbb1ec:     	b	0x100bbb29c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100bbb1f0:     	ldp	x20, x19, [x29, #-0xb8]
100bbb1f4:     	cbz	x26, 0x100bbb8a8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb40>
100bbb1f8:     	lsl	x22, x26, #2
100bbb1fc:     	mov	x0, x22
100bbb200:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
100bbb204:     	cbz	x0, 0x100bbbc40 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xed8>
100bbb208:     	mov	x24, x0
100bbb20c:     	mov	x8, #0x0                ; =0
100bbb210:     	ldp	x9, x1, [x27, #0x20]
100bbb214:     	ldr	w0, [x19, x8, lsl #2]
100bbb218:     	cmp	x1, x0
100bbb21c:     	b.ls	0x100bbbbc4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe5c>
100bbb220:     	ldr	w10, [x9, x0, lsl #2]
100bbb224:     	str	w10, [x24, x8, lsl #2]
100bbb228:     	add	x8, x8, #0x1
100bbb22c:     	cmp	x26, x8
100bbb230:     	b.ne	0x100bbb214 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4ac>
100bbb234:     	cbz	x20, 0x100bbb240 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100bbb238:     	mov	x0, x19
100bbb23c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb240:     	ldr	x20, [sp, #0x60]
100bbb244:     	b	0x100bbb490 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x728>
100bbb248:     	mov	x20, #-0x1              ; =-1
100bbb24c:     	ldr	x8, [sp, #0x40]
100bbb250:     	cbz	x8, 0x100bbb25c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4f4>
100bbb254:     	ldr	x0, [sp, #0x30]
100bbb258:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb25c:     	mov	x19, x20
100bbb260:     	ldp	x28, x20, [sp, #0x58]
100bbb264:     	mov	x21, #0x0               ; =0
100bbb268:     	cbz	x27, 0x100bbb29c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x534>
100bbb26c:     	mov	w8, #0x1                ; =1
100bbb270:     	mov	x9, x27
100bbb274:     	rbit	x10, x9
100bbb278:     	clz	x0, x10
100bbb27c:     	cmp	x0, x26
100bbb280:     	b.hs	0x100bbbb88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe20>
100bbb284:     	ldr	w10, [x22, x0, lsl #2]
100bbb288:     	lsl	x10, x8, x10
100bbb28c:     	orr	x21, x10, x21
100bbb290:     	sub	x10, x9, #0x1
100bbb294:     	ands	x9, x10, x9
100bbb298:     	b.ne	0x100bbb274 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x50c>
100bbb29c:     	stur	x21, [x29, #-0xe0]
100bbb2a0:     	bics	x8, x21, x20
100bbb2a4:     	stur	x8, [x29, #-0xb8]
100bbb2a8:     	b.ne	0x100bbbad0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd68>
100bbb2ac:     	str	x19, [sp, #0x80]
100bbb2b0:     	mov	w25, #0x4               ; =4
100bbb2b4:     	stp	xzr, x25, [x29, #-0xb8]
100bbb2b8:     	stur	xzr, [x29, #-0xa8]
100bbb2bc:     	mov	x19, #0x0               ; =0
100bbb2c0:     	cbz	x21, 0x100bbb420 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6b8>
100bbb2c4:     	mov	w8, #0x4                ; =4
100bbb2c8:     	mov	x20, x21
100bbb2cc:     	b	0x100bbb2f0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x588>
100bbb2d0:     	rbit	x9, x20
100bbb2d4:     	clz	x9, x9
100bbb2d8:     	str	w9, [x8, x19, lsl #2]
100bbb2dc:     	add	x19, x19, #0x1
100bbb2e0:     	stur	x19, [x29, #-0xa8]
100bbb2e4:     	sub	x9, x20, #0x1
100bbb2e8:     	ands	x20, x9, x20
100bbb2ec:     	b.eq	0x100bbb30c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5a4>
100bbb2f0:     	ldur	x9, [x29, #-0xb8]
100bbb2f4:     	cmp	x19, x9
100bbb2f8:     	b.ne	0x100bbb2d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100bbb2fc:     	sub	x0, x29, #0xb8
100bbb300:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bbb304:     	ldur	x8, [x29, #-0xb0]
100bbb308:     	b	0x100bbb2d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x568>
100bbb30c:     	ldp	x8, x25, [x29, #-0xb8]
100bbb310:     	cmp	x8, #0x0
100bbb314:     	cset	w8, eq
100bbb318:     	str	w8, [sp, #0x40]
100bbb31c:     	mov	w8, #0x4                ; =4
100bbb320:     	stp	xzr, x8, [x29, #-0xb8]
100bbb324:     	stur	xzr, [x29, #-0xa8]
100bbb328:     	cbz	x27, 0x100bbb438 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6d0>
100bbb32c:     	str	x23, [sp, #0x30]
100bbb330:     	mov	x20, #0x0               ; =0
100bbb334:     	mov	w8, #0x4                ; =4
100bbb338:     	b	0x100bbb35c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5f4>
100bbb33c:     	rbit	x9, x27
100bbb340:     	clz	x9, x9
100bbb344:     	str	w9, [x8, x28, lsl #2]
100bbb348:     	add	x20, x28, #0x1
100bbb34c:     	stur	x20, [x29, #-0xa8]
100bbb350:     	sub	x9, x27, #0x1
100bbb354:     	ands	x27, x9, x27
100bbb358:     	b.eq	0x100bbb37c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x614>
100bbb35c:     	mov	x28, x20
100bbb360:     	ldur	x9, [x29, #-0xb8]
100bbb364:     	cmp	x20, x9
100bbb368:     	b.ne	0x100bbb33c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100bbb36c:     	sub	x0, x29, #0xb8
100bbb370:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bbb374:     	ldur	x8, [x29, #-0xb0]
100bbb378:     	b	0x100bbb33c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5d4>
100bbb37c:     	ldp	x8, x24, [x29, #-0xb8]
100bbb380:     	cbz	x20, 0x100bbb478 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x710>
100bbb384:     	str	x8, [sp, #0x20]
100bbb388:     	lsl	x0, x20, #2
100bbb38c:     	mov	x23, x0
100bbb390:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
100bbb394:     	cbz	x0, 0x100bbbc20 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xeb8>
100bbb398:     	mov	x27, x0
100bbb39c:     	cbz	x19, 0x100bbb3ec <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x684>
100bbb3a0:     	mov	x9, #0x0                ; =0
100bbb3a4:     	lsl	x8, x19, #2
100bbb3a8:     	b	0x100bbb3bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x654>
100bbb3ac:     	str	w10, [x27, x9, lsl #2]
100bbb3b0:     	cmp	x9, x28
100bbb3b4:     	add	x9, x9, #0x1
100bbb3b8:     	b.eq	0x100bbb3fc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x694>
100bbb3bc:     	ldr	w0, [x24, x9, lsl #2]
100bbb3c0:     	cmp	x26, x0
100bbb3c4:     	b.ls	0x100bbbb9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe34>
100bbb3c8:     	mov	x10, #0x0               ; =0
100bbb3cc:     	ldr	w11, [x22, x0, lsl #2]
100bbb3d0:     	mov	x12, x8
100bbb3d4:     	ldr	w13, [x25, x10, lsl #2]
100bbb3d8:     	cmp	w13, w11
100bbb3dc:     	b.eq	0x100bbb3ac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x644>
100bbb3e0:     	add	x10, x10, #0x1
100bbb3e4:     	subs	x12, x12, #0x4
100bbb3e8:     	b.ne	0x100bbb3d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x66c>
100bbb3ec:     	adrp	x0, 0x101605000 <dyld_stub_binder+0x101605000>
100bbb3f0:     	add	x0, x0, #0xea0
100bbb3f4:     	bl	0x1013ba3f4 <__RNvNtCs4sDCw1iE1MS_4core6option13unwrap_failed>
100bbb3f8:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbb3fc:     	ldr	x19, [sp, #0x80]
100bbb400:     	ldr	x8, [sp, #0x20]
100bbb404:     	ldr	x28, [sp, #0x58]
100bbb408:     	cbz	x8, 0x100bbb414 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100bbb40c:     	mov	x0, x24
100bbb410:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb414:     	mov	x24, x27
100bbb418:     	ldr	x23, [sp, #0x30]
100bbb41c:     	b	0x100bbb444 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6dc>
100bbb420:     	mov	w8, #0x1                ; =1
100bbb424:     	str	w8, [sp, #0x40]
100bbb428:     	mov	w8, #0x4                ; =4
100bbb42c:     	stp	xzr, x8, [x29, #-0xb8]
100bbb430:     	stur	xzr, [x29, #-0xa8]
100bbb434:     	cbnz	x27, 0x100bbb32c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x5c4>
100bbb438:     	mov	x20, #0x0               ; =0
100bbb43c:     	mov	w24, #0x4               ; =4
100bbb440:     	ldr	x19, [sp, #0x80]
100bbb444:     	mov	x8, #0x0                ; =0
100bbb448:     	lsl	x9, x20, #2
100bbb44c:     	str	x24, [sp, #0x30]
100bbb450:     	cbz	x9, 0x100bbb8e0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb78>
100bbb454:     	ldr	w10, [x24, x8, lsl #2]
100bbb458:     	sub	x9, x9, #0x4
100bbb45c:     	cmp	x8, x10
100bbb460:     	add	x8, x8, #0x1
100bbb464:     	b.eq	0x100bbb450 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6e8>
100bbb468:     	cmn	x19, #0x1
100bbb46c:     	b.eq	0x100bbb868 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb00>
100bbb470:     	ldr	x1, [sp, #0xa0]
100bbb474:     	b	0x100bbb8bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100bbb478:     	mov	w27, #0x4               ; =4
100bbb47c:     	ldr	x19, [sp, #0x80]
100bbb480:     	ldr	x28, [sp, #0x58]
100bbb484:     	cbnz	x8, 0x100bbb40c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6a4>
100bbb488:     	b	0x100bbb414 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x6ac>
100bbb48c:     	mov	w24, #0x4               ; =4
100bbb490:     	str	x28, [sp, #0x58]
100bbb494:     	fmov	d0, x20
100bbb498:     	cnt.8b	v0, v0
100bbb49c:     	addv.8b	b0, v0
100bbb4a0:     	fmov	x8, d0
100bbb4a4:     	mov	w9, #0x1                ; =1
100bbb4a8:     	lsl	x20, x9, x8
100bbb4ac:     	ldr	x9, [sp, #0x98]
100bbb4b0:     	ldr	x8, [x9, #0x40]
100bbb4b4:     	add	x8, x8, x20
100bbb4b8:     	str	x8, [x9, #0x40]
100bbb4bc:     	add	x8, x20, #0x3f
100bbb4c0:     	lsr	x22, x8, #6
100bbb4c4:     	lsl	x19, x22, #3
100bbb4c8:     	mov	x0, x19
100bbb4cc:     	mov	w1, #0x1                ; =1
100bbb4d0:     	bl	0x1013c2664 <dyld_stub_binder+0x1013c2664>
100bbb4d4:     	cbz	x0, 0x100bbbbf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe90>
100bbb4d8:     	mov	x21, x0
100bbb4dc:     	mov	x19, #0x0               ; =0
100bbb4e0:     	ldr	x25, [x25, #0x8]
100bbb4e4:     	and	x8, x26, #0xfffffffffffffffe
100bbb4e8:     	neg	x8, x8
100bbb4ec:     	str	x8, [sp, #0x98]
100bbb4f0:     	mov	w28, #0x1               ; =1
100bbb4f4:     	adrp	x8, 0x101457000 <GCC_except_table9514>
100bbb4f8:     	ldr	q0, [x8, #0x5e0]
100bbb4fc:     	str	q0, [sp, #0xa0]
100bbb500:     	mov	w8, #0x2                ; =2
100bbb504:     	dup.2d	v0, x8
100bbb508:     	str	q0, [sp, #0x80]
100bbb50c:     	mov	w8, #0x4                ; =4
100bbb510:     	dup.2d	v1, x8
100bbb514:     	mov	w8, #0x8                ; =8
100bbb518:     	dup.2d	v0, x8
100bbb51c:     	stp	q0, q1, [sp, #0x30]
100bbb520:     	mov	w8, #0xc                ; =12
100bbb524:     	dup.2d	v1, x8
100bbb528:     	mov	w8, #0x10               ; =16
100bbb52c:     	dup.2d	v0, x8
100bbb530:     	stp	q0, q1, [sp, #0x10]
100bbb534:     	adrp	x8, 0x101457000 <GCC_except_table9514>
100bbb538:     	ldr	q0, [x8, #0x600]
100bbb53c:     	str	q0, [sp]
100bbb540:     	mov	w27, #0x3f              ; =63
100bbb544:     	dup.2d	v0, x27
100bbb548:     	str	q0, [sp, #0x60]
100bbb54c:     	movi.2s	v8, #0x3f
100bbb550:     	b	0x100bbb560 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7f8>
100bbb554:     	add	x19, x19, #0x1
100bbb558:     	cmp	x19, x20
100bbb55c:     	b.eq	0x100bbb808 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaa0>
100bbb560:     	mov	x2, x25
100bbb564:     	cbz	x26, 0x100bbb7d8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa70>
100bbb568:     	cmp	x26, #0x1
100bbb56c:     	b.ne	0x100bbb57c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x814>
100bbb570:     	mov	x9, #0x0                ; =0
100bbb574:     	mov	x8, #0x0                ; =0
100bbb578:     	b	0x100bbb7b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bbb57c:     	dup.2d	v0, x19
100bbb580:     	cmp	x26, #0x10
100bbb584:     	b.hs	0x100bbb594 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x82c>
100bbb588:     	mov	x10, #0x0               ; =0
100bbb58c:     	mov	x8, #0x0                ; =0
100bbb590:     	b	0x100bbb740 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9d8>
100bbb594:     	movi.2d	v1, #0000000000000000
100bbb598:     	add	x8, x24, #0x20
100bbb59c:     	movi.2d	v2, #0000000000000000
100bbb5a0:     	and	x9, x26, #0xfffffffffffffff0
100bbb5a4:     	ldr	q4, [sp, #0xa0]
100bbb5a8:     	ldp	q6, q15, [sp]
100bbb5ac:     	movi.2d	v3, #0000000000000000
100bbb5b0:     	movi.2d	v7, #0000000000000000
100bbb5b4:     	movi.2d	v16, #0000000000000000
100bbb5b8:     	movi.2d	v5, #0000000000000000
100bbb5bc:     	movi.2d	v18, #0000000000000000
100bbb5c0:     	movi.2d	v17, #0000000000000000
100bbb5c4:     	ldp	q13, q12, [sp, #0x30]
100bbb5c8:     	ldr	q14, [sp, #0x20]
100bbb5cc:     	movi.4s	v8, #0x3f
100bbb5d0:     	add.2d	v19, v4, v12
100bbb5d4:     	add.2d	v20, v6, v12
100bbb5d8:     	add.2d	v21, v4, v13
100bbb5dc:     	add.2d	v22, v6, v13
100bbb5e0:     	add.2d	v23, v4, v14
100bbb5e4:     	add.2d	v24, v6, v14
100bbb5e8:     	ldp	q25, q26, [x8, #-0x20]
100bbb5ec:     	dup.2d	v27, x27
100bbb5f0:     	ldp	q28, q29, [x8], #0x40
100bbb5f4:     	and.16b	v30, v6, v27
100bbb5f8:     	and.16b	v31, v4, v27
100bbb5fc:     	and.16b	v20, v20, v27
100bbb600:     	and.16b	v19, v19, v27
100bbb604:     	and.16b	v22, v22, v27
100bbb608:     	and.16b	v21, v21, v27
100bbb60c:     	and.16b	v24, v24, v27
100bbb610:     	and.16b	v23, v23, v27
100bbb614:     	neg.2d	v27, v31
100bbb618:     	ushl.2d	v27, v0, v27
100bbb61c:     	neg.2d	v30, v30
100bbb620:     	ushl.2d	v30, v0, v30
100bbb624:     	neg.2d	v19, v19
100bbb628:     	ushl.2d	v19, v0, v19
100bbb62c:     	neg.2d	v20, v20
100bbb630:     	ushl.2d	v20, v0, v20
100bbb634:     	neg.2d	v21, v21
100bbb638:     	ushl.2d	v21, v0, v21
100bbb63c:     	neg.2d	v22, v22
100bbb640:     	ushl.2d	v22, v0, v22
100bbb644:     	neg.2d	v23, v23
100bbb648:     	ushl.2d	v23, v0, v23
100bbb64c:     	neg.2d	v24, v24
100bbb650:     	ushl.2d	v24, v0, v24
100bbb654:     	dup.2d	v31, x28
100bbb658:     	and.16b	v30, v30, v31
100bbb65c:     	and.16b	v27, v27, v31
100bbb660:     	and.16b	v20, v20, v31
100bbb664:     	and.16b	v19, v19, v31
100bbb668:     	and.16b	v22, v22, v31
100bbb66c:     	and.16b	v21, v21, v31
100bbb670:     	and.16b	v24, v24, v31
100bbb674:     	and.16b	v23, v23, v31
100bbb678:     	and.16b	v25, v25, v8
100bbb67c:     	and.16b	v26, v26, v8
100bbb680:     	and.16b	v28, v28, v8
100bbb684:     	and.16b	v29, v29, v8
100bbb688:     	ushll2.2d	v31, v25, #0x0
100bbb68c:     	ushll.2d	v25, v25, #0x0
100bbb690:     	ushll2.2d	v9, v26, #0x0
100bbb694:     	ushll.2d	v26, v26, #0x0
100bbb698:     	ushll2.2d	v10, v28, #0x0
100bbb69c:     	ushll.2d	v28, v28, #0x0
100bbb6a0:     	ushll2.2d	v11, v29, #0x0
100bbb6a4:     	ushll.2d	v29, v29, #0x0
100bbb6a8:     	ushl.2d	v25, v27, v25
100bbb6ac:     	ushl.2d	v27, v30, v31
100bbb6b0:     	ushl.2d	v19, v19, v26
100bbb6b4:     	ushl.2d	v20, v20, v9
100bbb6b8:     	ushl.2d	v21, v21, v28
100bbb6bc:     	ushl.2d	v22, v22, v10
100bbb6c0:     	ushl.2d	v23, v23, v29
100bbb6c4:     	ushl.2d	v24, v24, v11
100bbb6c8:     	orr.16b	v3, v27, v3
100bbb6cc:     	orr.16b	v2, v25, v2
100bbb6d0:     	orr.16b	v16, v20, v16
100bbb6d4:     	orr.16b	v7, v19, v7
100bbb6d8:     	orr.16b	v18, v22, v18
100bbb6dc:     	orr.16b	v5, v21, v5
100bbb6e0:     	orr.16b	v1, v24, v1
100bbb6e4:     	orr.16b	v17, v23, v17
100bbb6e8:     	add.2d	v6, v6, v15
100bbb6ec:     	add.2d	v4, v4, v15
100bbb6f0:     	subs	x9, x9, #0x10
100bbb6f4:     	b.ne	0x100bbb5d0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x868>
100bbb6f8:     	orr.16b	v2, v7, v2
100bbb6fc:     	orr.16b	v3, v16, v3
100bbb700:     	orr.16b	v3, v18, v3
100bbb704:     	orr.16b	v2, v5, v2
100bbb708:     	orr.16b	v2, v17, v2
100bbb70c:     	orr.16b	v1, v1, v3
100bbb710:     	orr.16b	v1, v2, v1
100bbb714:     	mov	d2, v1[1]
100bbb718:     	orr.8b	v1, v1, v2
100bbb71c:     	fmov	x8, d1
100bbb720:     	and	x9, x26, #0xfffffffffffffff0
100bbb724:     	cmp	x26, x9
100bbb728:     	movi.2s	v8, #0x3f
100bbb72c:     	b.eq	0x100bbb7d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100bbb730:     	and	x10, x26, #0xfffffffffffffff0
100bbb734:     	and	x9, x26, #0xfffffffffffffff0
100bbb738:     	and	x11, x26, #0xe
100bbb73c:     	cbz	x11, 0x100bbb7b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bbb740:     	fmov	d1, x8
100bbb744:     	dup.2d	v2, x10
100bbb748:     	ldr	q3, [sp, #0xa0]
100bbb74c:     	orr.16b	v2, v2, v3
100bbb750:     	ldr	x8, [sp, #0x98]
100bbb754:     	add	x8, x8, x10
100bbb758:     	add	x9, x24, x10, lsl #2
100bbb75c:     	ldr	q6, [sp, #0x80]
100bbb760:     	ldr	q7, [sp, #0x60]
100bbb764:     	ldr	d3, [x9], #0x8
100bbb768:     	and.16b	v4, v2, v7
100bbb76c:     	neg.2d	v4, v4
100bbb770:     	ushl.2d	v4, v0, v4
100bbb774:     	dup.2d	v5, x28
100bbb778:     	and.16b	v4, v4, v5
100bbb77c:     	and.8b	v3, v3, v8
100bbb780:     	ushll.2d	v3, v3, #0x0
100bbb784:     	ushl.2d	v3, v4, v3
100bbb788:     	orr.16b	v1, v3, v1
100bbb78c:     	add.2d	v2, v2, v6
100bbb790:     	adds	x8, x8, #0x2
100bbb794:     	b.ne	0x100bbb764 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x9fc>
100bbb798:     	mov	d0, v1[1]
100bbb79c:     	orr.8b	v0, v1, v0
100bbb7a0:     	fmov	x8, d0
100bbb7a4:     	and	x9, x26, #0xfffffffffffffffe
100bbb7a8:     	and	x10, x26, #0xfffffffffffffffe
100bbb7ac:     	cmp	x26, x10
100bbb7b0:     	b.eq	0x100bbb7d4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa6c>
100bbb7b4:     	ldr	w10, [x24, x9, lsl #2]
100bbb7b8:     	lsr	x11, x19, x9
100bbb7bc:     	and	x11, x11, #0x1
100bbb7c0:     	lsl	x10, x11, x10
100bbb7c4:     	orr	x8, x10, x8
100bbb7c8:     	add	x9, x9, #0x1
100bbb7cc:     	cmp	x26, x9
100bbb7d0:     	b.ne	0x100bbb7b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xa4c>
100bbb7d4:     	orr	x2, x8, x25
100bbb7d8:     	mov	x0, x23
100bbb7dc:     	ldr	x1, [sp, #0x78]
100bbb7e0:     	bl	0x100ca0438 <__RNvMNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_rawINtB2_5ArenaKm1_E8evaluateB6_>
100bbb7e4:     	cbz	w0, 0x100bbb554 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100bbb7e8:     	lsr	x0, x19, #6
100bbb7ec:     	cmp	x0, x22
100bbb7f0:     	b.hs	0x100bbbbb0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe48>
100bbb7f4:     	lsl	x8, x28, x19
100bbb7f8:     	ldr	x9, [x21, x0, lsl #3]
100bbb7fc:     	orr	x8, x9, x8
100bbb800:     	str	x8, [x21, x0, lsl #3]
100bbb804:     	b	0x100bbb554 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x7ec>
100bbb808:     	ldr	x8, [sp, #0x58]
100bbb80c:     	stp	x22, x21, [x8]
100bbb810:     	stp	x22, xzr, [x8, #0x10]
100bbb814:     	cbz	x26, 0x100bbba80 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bbb818:     	mov	x0, x24
100bbb81c:     	b	0x100bbba7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100bbb820:     	mov	x8, #0x0                ; =0
100bbb824:     	ldur	x21, [x29, #-0xb8]
100bbb828:     	mov	w0, #0x8                ; =8
100bbb82c:     	ldr	x10, [sp, #0x98]
100bbb830:     	ldr	x9, [x10, #0x48]
100bbb834:     	add	x9, x9, x20
100bbb838:     	str	x9, [x10, #0x48]
100bbb83c:     	stp	x8, x0, [x28]
100bbb840:     	stp	x20, xzr, [x28, #0x10]
100bbb844:     	cmp	x21, #0x1
100bbb848:     	b.lt	0x100bbb854 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xaec>
100bbb84c:     	ldur	x0, [x29, #-0xb0]
100bbb850:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb854:     	ldur	x8, [x29, #-0xe0]
100bbb858:     	cmp	x8, #0x1
100bbb85c:     	b.lt	0x100bbba80 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bbb860:     	ldur	x0, [x29, #-0xd8]
100bbb864:     	b	0x100bbba7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd14>
100bbb868:     	ldr	x1, [sp, #0xa0]
100bbb86c:     	cbz	x1, 0x100bbb8b4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb4c>
100bbb870:     	lsl	x27, x1, #3
100bbb874:     	mov	x0, x27
100bbb878:     	mov	x19, x1
100bbb87c:     	bl	0x1013c2844 <dyld_stub_binder+0x1013c2844>
100bbb880:     	cbz	x0, 0x100bbbc50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xee8>
100bbb884:     	mov	x26, x0
100bbb888:     	mov	x1, x23
100bbb88c:     	mov	x2, x27
100bbb890:     	bl	0x1013c285c <dyld_stub_binder+0x1013c285c>
100bbb894:     	cmn	x19, #0x1
100bbb898:     	b.eq	0x100bbbb58 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xdf0>
100bbb89c:     	mov	x1, x19
100bbb8a0:     	mov	x23, x26
100bbb8a4:     	b	0x100bbb8bc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xb54>
100bbb8a8:     	mov	w24, #0x4               ; =4
100bbb8ac:     	cbnz	x20, 0x100bbb238 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d0>
100bbb8b0:     	b	0x100bbb240 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x4d8>
100bbb8b4:     	mov	x19, #0x0               ; =0
100bbb8b8:     	mov	w23, #0x8               ; =8
100bbb8bc:     	mov	x0, x23
100bbb8c0:     	mov	x2, x24
100bbb8c4:     	mov	x3, x20
100bbb8c8:     	bl	0x100e43f40 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels7permute>
100bbb8cc:     	str	x19, [sp, #0x80]
100bbb8d0:     	ldr	x9, [sp, #0x98]
100bbb8d4:     	ldr	x8, [x9, #0x28]
100bbb8d8:     	add	x8, x8, #0x1
100bbb8dc:     	str	x8, [x9, #0x28]
100bbb8e0:     	mov	w8, #0x4                ; =4
100bbb8e4:     	stp	xzr, x8, [x29, #-0xb8]
100bbb8e8:     	stur	xzr, [x29, #-0xa8]
100bbb8ec:     	ldr	x8, [sp, #0x60]
100bbb8f0:     	bics	x22, x8, x21
100bbb8f4:     	b.eq	0x100bbba00 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc98>
100bbb8f8:     	mov	x24, x23
100bbb8fc:     	mov	x19, #0x0               ; =0
100bbb900:     	mov	w8, #0x4                ; =4
100bbb904:     	mov	w9, #0x1                ; =1
100bbb908:     	b	0x100bbb930 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbc8>
100bbb90c:     	rbit	x9, x22
100bbb910:     	clz	x9, x9
100bbb914:     	str	w9, [x8, x19]
100bbb918:     	stur	x23, [x29, #-0xa8]
100bbb91c:     	sub	x10, x22, #0x1
100bbb920:     	add	x19, x19, #0x4
100bbb924:     	add	x9, x23, #0x1
100bbb928:     	ands	x22, x10, x22
100bbb92c:     	b.eq	0x100bbb954 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xbec>
100bbb930:     	mov	x23, x9
100bbb934:     	sub	x9, x9, #0x1
100bbb938:     	ldur	x10, [x29, #-0xb8]
100bbb93c:     	cmp	x9, x10
100bbb940:     	b.ne	0x100bbb90c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100bbb944:     	sub	x0, x29, #0xb8
100bbb948:     	bl	0x1013bb15c <__RNvMs4_NtCsaexw8v31UlU_5alloc7raw_vecINtB5_6RawVecmE8grow_oneCsdnlOoPoZ040_7roaring>
100bbb94c:     	ldur	x8, [x29, #-0xb0]
100bbb950:     	b	0x100bbb90c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xba4>
100bbb954:     	ldp	x8, x28, [x29, #-0xb8]
100bbb958:     	str	x8, [sp, #0x20]
100bbb95c:     	str	x28, [sp, #0x10]
100bbb960:     	cbz	x23, 0x100bbba08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca0>
100bbb964:     	mov	w27, #0x1               ; =1
100bbb968:     	mov	x1, x24
100bbb96c:     	b	0x100bbb998 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc30>
100bbb970:     	orr	x21, x22, x21
100bbb974:     	stur	x21, [x29, #-0xe0]
100bbb978:     	ldr	x9, [sp, #0x98]
100bbb97c:     	ldr	x8, [x9, #0x30]
100bbb980:     	add	x8, x8, #0x1
100bbb984:     	str	x8, [x9, #0x30]
100bbb988:     	str	x26, [sp, #0x80]
100bbb98c:     	mov	x1, x24
100bbb990:     	subs	x19, x19, #0x4
100bbb994:     	b.eq	0x100bbba0c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xca4>
100bbb998:     	ldr	w8, [x28], #0x4
100bbb99c:     	lsl	x22, x27, x8
100bbb9a0:     	sub	x8, x22, #0x1
100bbb9a4:     	and	x8, x8, x21
100bbb9a8:     	fmov	d0, x8
100bbb9ac:     	cnt.8b	v0, v0
100bbb9b0:     	addv.8b	b0, v0
100bbb9b4:     	fmov	w4, s0
100bbb9b8:     	fmov	d0, x21
100bbb9bc:     	cnt.8b	v0, v0
100bbb9c0:     	addv.8b	b0, v0
100bbb9c4:     	fmov	w3, s0
100bbb9c8:     	sub	x0, x29, #0xb8
100bbb9cc:     	mov	x23, x1
100bbb9d0:     	ldr	x2, [sp, #0xa0]
100bbb9d4:     	bl	0x100e44f54 <__RNvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12word_kernels9broadcast>
100bbb9d8:     	ldp	x26, x24, [x29, #-0xb8]
100bbb9dc:     	ldur	x8, [x29, #-0xa8]
100bbb9e0:     	str	x8, [sp, #0xa0]
100bbb9e4:     	ldr	x8, [sp, #0x80]
100bbb9e8:     	sub	x8, x8, #0x1
100bbb9ec:     	cmn	x8, #0x3
100bbb9f0:     	b.hi	0x100bbb970 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100bbb9f4:     	mov	x0, x23
100bbb9f8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbb9fc:     	b	0x100bbb970 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xc08>
100bbba00:     	ldr	x19, [sp, #0x80]
100bbba04:     	b	0x100bbba28 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcc0>
100bbba08:     	ldr	x26, [sp, #0x80]
100bbba0c:     	ldr	x8, [sp, #0x20]
100bbba10:     	cbz	x8, 0x100bbba1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xcb4>
100bbba14:     	ldr	x0, [sp, #0x10]
100bbba18:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbba1c:     	mov	x19, x26
100bbba20:     	mov	x23, x24
100bbba24:     	ldr	x28, [sp, #0x58]
100bbba28:     	ldr	x24, [sp, #0x30]
100bbba2c:     	ldr	x8, [sp, #0x60]
100bbba30:     	cmp	x21, x8
100bbba34:     	b.ne	0x100bbbaf4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd8c>
100bbba38:     	cmn	x19, #0x1
100bbba3c:     	b.ne	0x100bbba50 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xce8>
100bbba40:     	ldr	x9, [sp, #0x98]
100bbba44:     	ldr	x8, [x9, #0x18]
100bbba48:     	add	x8, x8, #0x1
100bbba4c:     	str	x8, [x9, #0x18]
100bbba50:     	ldr	x8, [sp, #0x78]
100bbba54:     	sbfx	x8, x8, #0, #1
100bbba58:     	stp	x19, x23, [x28]
100bbba5c:     	ldr	x9, [sp, #0xa0]
100bbba60:     	stp	x9, x8, [x28, #0x10]
100bbba64:     	cbz	x20, 0x100bbba70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd08>
100bbba68:     	mov	x0, x24
100bbba6c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbba70:     	ldr	w8, [sp, #0x40]
100bbba74:     	tbnz	w8, #0x0, 0x100bbba80 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xd18>
100bbba78:     	mov	x0, x25
100bbba7c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbba80:     	ldp	x29, x30, [sp, #0x1d0]
100bbba84:     	ldp	x20, x19, [sp, #0x1c0]
100bbba88:     	ldp	x22, x21, [sp, #0x1b0]
100bbba8c:     	ldp	x24, x23, [sp, #0x1a0]
100bbba90:     	ldp	x26, x25, [sp, #0x190]
100bbba94:     	ldp	x28, x27, [sp, #0x180]
100bbba98:     	ldp	d9, d8, [sp, #0x170]
100bbba9c:     	ldp	d11, d10, [sp, #0x160]
100bbbaa0:     	ldp	d13, d12, [sp, #0x150]
100bbbaa4:     	ldp	d15, d14, [sp, #0x140]
100bbbaa8:     	add	sp, sp, #0x1e0
100bbbaac:     	ret
100bbbab0:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100bbbab4:     	add	x2, x2, #0x78
100bbbab8:     	adrp	x5, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbabc:     	add	x5, x5, #0x178
100bbbac0:     	sub	x1, x29, #0xb8
100bbbac4:     	mov	w0, #0x0                ; =0
100bbbac8:     	mov	x3, #0x0                ; =0
100bbbacc:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bbbad0:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100bbbad4:     	add	x2, x2, #0x78
100bbbad8:     	adrp	x5, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbadc:     	add	x5, x5, #0x118
100bbbae0:     	sub	x1, x29, #0xb8
100bbbae4:     	mov	w0, #0x0                ; =0
100bbbae8:     	mov	x3, #0x0                ; =0
100bbbaec:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bbbaf0:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbaf4:     	adrp	x5, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbaf8:     	add	x5, x5, #0x100
100bbbafc:     	sub	x1, x29, #0xe0
100bbbb00:     	add	x2, sp, #0xb8
100bbbb04:     	mov	w0, #0x0                ; =0
100bbbb08:     	mov	x3, #0x0                ; =0
100bbbb0c:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bbbb10:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbb14:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100bbbb18:     	add	x2, x2, #0x78
100bbbb1c:     	adrp	x5, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbb20:     	add	x5, x5, #0x160
100bbbb24:     	sub	x1, x29, #0xb8
100bbbb28:     	mov	w0, #0x0                ; =0
100bbbb2c:     	mov	x3, #0x0                ; =0
100bbbb30:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bbbb34:     	adrp	x2, 0x101522000 <__RNvNtNtNtCs4sDCw1iE1MS_4core7unicode12unicode_data11white_space14WHITESPACE_MAP+0x21a7>
100bbbb38:     	add	x2, x2, #0x78
100bbbb3c:     	adrp	x5, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbb40:     	add	x5, x5, #0x148
100bbbb44:     	sub	x1, x29, #0xc0
100bbbb48:     	mov	w0, #0x1                ; =1
100bbbb4c:     	mov	x3, #0x0                ; =0
100bbbb50:     	bl	0x1013ba260 <__RINvNtCs4sDCw1iE1MS_4core9panicking13assert_failedyyEB4_>
100bbbb54:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbb58:     	adrp	x0, 0x10151e000 <__RNvNvXsn_NtNtCs4sDCw1iE1MS_4core2io5errorNtB7_9ErrorKindNtNtBb_3fmt5Debug3fmt8___OFFSET+0xc88>
100bbbb5c:     	add	x0, x0, #0xb39
100bbbb60:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
100bbbb64:     	add	x2, x2, #0x198
100bbbb68:     	mov	x23, x26
100bbbb6c:     	mov	w1, #0x28               ; =40
100bbbb70:     	bl	0x1013ba348 <__RNvNtCs4sDCw1iE1MS_4core9panicking5panic>
100bbbb74:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbb78:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
100bbbb7c:     	add	x2, x2, #0x678
100bbbb80:     	mov	x1, x26
100bbbb84:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbb88:     	adrp	x2, 0x101644000 <dyld_stub_binder+0x101644000>
100bbbb8c:     	add	x2, x2, #0x678
100bbbb90:     	mov	x1, x26
100bbbb94:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbb98:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbb9c:     	adrp	x2, 0x101606000 <dyld_stub_binder+0x101606000>
100bbbba0:     	add	x2, x2, #0x1d0
100bbbba4:     	mov	x1, x26
100bbbba8:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbbac:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbbb0:     	adrp	x2, 0x101602000 <dyld_stub_binder+0x101602000>
100bbbbb4:     	add	x2, x2, #0x4d0
100bbbbb8:     	mov	x1, x22
100bbbbbc:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbbc0:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbbc4:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
100bbbbc8:     	add	x2, x2, #0xeb8
100bbbbcc:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbbd0:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbbd4:     	mov	x19, x0
100bbbbd8:     	mov	x1, x16
100bbbbdc:     	b	0x100bbbbe4 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xe7c>
100bbbbe0:     	mov	x19, x0
100bbbbe4:     	adrp	x2, 0x101605000 <dyld_stub_binder+0x101605000>
100bbbbe8:     	add	x2, x2, #0xa50
100bbbbec:     	mov	x0, x8
100bbbbf0:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbbf4:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbbf8:     	mov	w0, #0x8                ; =8
100bbbbfc:     	mov	x1, x19
100bbbc00:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bbbc04:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbc08:     	adrp	x2, 0x101604000 <dyld_stub_binder+0x101604000>
100bbbc0c:     	add	x2, x2, #0x130
100bbbc10:     	mov	x0, x19
100bbbc14:     	mov	x1, x26
100bbbc18:     	bl	0x1013ba35c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
100bbbc1c:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbc20:     	mov	w0, #0x4                ; =4
100bbbc24:     	mov	x1, x23
100bbbc28:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bbbc2c:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbc30:     	mov	w0, #0x8                ; =8
100bbbc34:     	mov	x1, x19
100bbbc38:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bbbc3c:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbc40:     	mov	w0, #0x4                ; =4
100bbbc44:     	mov	x1, x22
100bbbc48:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bbbc4c:     	b	0x100bbbc60 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xef8>
100bbbc50:     	mov	x19, #-0x1              ; =-1
100bbbc54:     	mov	w0, #0x8                ; =8
100bbbc58:     	mov	x1, x27
100bbbc5c:     	bl	0x1013b9b64 <__RNvNtCsaexw8v31UlU_5alloc7raw_vec12handle_error>
100bbbc60:     	brk	#0x1
100bbbc64:     	mov	x28, x0
100bbbc68:     	b	0x100bbbc94 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf2c>
100bbbc6c:     	mov	x28, x0
100bbbc70:     	b	0x100bbbdcc <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1064>
100bbbc74:     	mov	x28, x0
100bbbc78:     	b	0x100bbbdac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100bbbc7c:     	mov	x28, x0
100bbbc80:     	b	0x100bbbd88 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1020>
100bbbc84:     	b	0x100bbbd24 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfbc>
100bbbc88:     	mov	x28, x0
100bbbc8c:     	mov	x0, x24
100bbbc90:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbc94:     	cbz	x20, 0x100bbbe1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bbbc98:     	mov	x0, x19
100bbbc9c:     	b	0x100bbbe18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bbbca0:     	b	0x100bbbd7c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1014>
100bbbca4:     	mov	x28, x0
100bbbca8:     	ldur	x8, [x29, #-0xb8]
100bbbcac:     	cbnz	x8, 0x100bbbcb8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf50>
100bbbcb0:     	mov	x23, x24
100bbbcb4:     	b	0x100bbbd4c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bbbcb8:     	ldur	x0, [x29, #-0xb0]
100bbbcbc:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbcc0:     	mov	x23, x24
100bbbcc4:     	b	0x100bbbd4c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bbbcc8:     	mov	x28, x0
100bbbccc:     	mov	x0, x19
100bbbcd0:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbcd4:     	b	0x100bbbd9c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1034>
100bbbcd8:     	mov	x28, x0
100bbbcdc:     	ldur	x8, [x29, #-0xb8]
100bbbce0:     	cbnz	x8, 0x100bbbcf0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xf88>
100bbbce4:     	ldr	x23, [sp, #0x30]
100bbbce8:     	ldr	x19, [sp, #0x80]
100bbbcec:     	b	0x100bbbdf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bbbcf0:     	ldur	x24, [x29, #-0xb0]
100bbbcf4:     	ldr	x23, [sp, #0x30]
100bbbcf8:     	ldr	x19, [sp, #0x80]
100bbbcfc:     	b	0x100bbbdf0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bbbd00:     	mov	x28, x0
100bbbd04:     	ldur	x8, [x29, #-0xb8]
100bbbd08:     	cbnz	x8, 0x100bbbd14 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfac>
100bbbd0c:     	ldr	x19, [sp, #0x80]
100bbbd10:     	b	0x100bbbe08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bbbd14:     	ldur	x0, [x29, #-0xb0]
100bbbd18:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbd1c:     	ldr	x19, [sp, #0x80]
100bbbd20:     	b	0x100bbbe08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bbbd24:     	mov	x28, x0
100bbbd28:     	ldur	x8, [x29, #-0xb8]
100bbbd2c:     	cbz	x8, 0x100bbbe1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bbbd30:     	ldur	x0, [x29, #-0xb0]
100bbbd34:     	b	0x100bbbe18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bbbd38:     	mov	x28, x0
100bbbd3c:     	ldr	x8, [sp, #0x20]
100bbbd40:     	cbz	x8, 0x100bbbd4c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0xfe4>
100bbbd44:     	ldr	x0, [sp, #0x10]
100bbbd48:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbd4c:     	ldr	x19, [sp, #0x80]
100bbbd50:     	ldr	x24, [sp, #0x30]
100bbbd54:     	cbnz	x20, 0x100bbbdf0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bbbd58:     	b	0x100bbbdf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bbbd5c:     	mov	x28, x0
100bbbd60:     	ldr	x8, [sp, #0x40]
100bbbd64:     	cbz	x8, 0x100bbbd70 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1008>
100bbbd68:     	ldr	x0, [sp, #0x30]
100bbbd6c:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbd70:     	mov	x23, x19
100bbbd74:     	mov	x19, x24
100bbbd78:     	b	0x100bbbe08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bbbd7c:     	mov	x28, x0
100bbbd80:     	mov	x0, x21
100bbbd84:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbd88:     	cbz	x26, 0x100bbbe1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bbbd8c:     	mov	x0, x24
100bbbd90:     	b	0x100bbbe18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bbbd94:     	mov	x28, x0
100bbbd98:     	ldur	x21, [x29, #-0xb8]
100bbbd9c:     	cmp	x21, #0x1
100bbbda0:     	b.lt	0x100bbbdac <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1044>
100bbbda4:     	ldur	x0, [x29, #-0xb0]
100bbbda8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbdac:     	ldur	x8, [x29, #-0xe0]
100bbbdb0:     	cmp	x8, #0x1
100bbbdb4:     	b.lt	0x100bbbe1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bbbdb8:     	ldur	x0, [x29, #-0xd8]
100bbbdbc:     	b	0x100bbbe18 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b0>
100bbbdc0:     	mov	x28, x0
100bbbdc4:     	mov	x0, x27
100bbbdc8:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbdcc:     	ldr	x23, [sp, #0x30]
100bbbdd0:     	ldr	x19, [sp, #0x80]
100bbbdd4:     	ldr	x8, [sp, #0x20]
100bbbdd8:     	cbnz	x8, 0x100bbbdf0 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1088>
100bbbddc:     	b	0x100bbbdf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bbbde0:     	mov	x28, x0
100bbbde4:     	b	0x100bbbe08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bbbde8:     	mov	x28, x0
100bbbdec:     	cbz	x20, 0x100bbbdf8 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x1090>
100bbbdf0:     	mov	x0, x24
100bbbdf4:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbdf8:     	ldr	w8, [sp, #0x40]
100bbbdfc:     	tbnz	w8, #0x0, 0x100bbbe08 <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10a0>
100bbbe00:     	mov	x0, x25
100bbbe04:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbe08:     	sub	x8, x19, #0x1
100bbbe0c:     	cmn	x8, #0x3
100bbbe10:     	b.hi	0x100bbbe1c <__RINvNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw12view_product5planeKm1_EB8_+0x10b4>
100bbbe14:     	mov	x0, x23
100bbbe18:     	bl	0x1013c2778 <dyld_stub_binder+0x1013c2778>
100bbbe1c:     	mov	x0, x28
100bbbe20:     	bl	0x1013c25c8 <dyld_stub_binder+0x1013c25c8>
