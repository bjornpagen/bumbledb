
/Users/bjorn/Documents/bumbledb/proposal/experiments/event-repr-lab/.scratch/engine/target/release/build/bumbledb/e9559f2bb6afbcd7/out/bumbledb-e9559f2bb6afbcd7:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000100c7ec8c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find>:
100c7ec8c:     	sub	sp, sp, #0xb0
100c7ec90:     	stp	d9, d8, [sp, #0x40]
100c7ec94:     	stp	x28, x27, [sp, #0x50]
100c7ec98:     	stp	x26, x25, [sp, #0x60]
100c7ec9c:     	stp	x24, x23, [sp, #0x70]
100c7eca0:     	stp	x22, x21, [sp, #0x80]
100c7eca4:     	stp	x20, x19, [sp, #0x90]
100c7eca8:     	stp	x29, x30, [sp, #0xa0]
100c7ecac:     	add	x29, sp, #0xa0
100c7ecb0:     	mov	x21, x1
100c7ecb4:     	mov	x19, x0
100c7ecb8:     	ldr	x8, [x0]
100c7ecbc:     	cmn	x8, #0x1
100c7ecc0:     	b.eq	0x100c7ed78 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0xec>
100c7ecc4:     	ldr	x1, [x19, #0x68]
100c7ecc8:     	mov	x0, x21
100c7eccc:     	bl	0x100c7ea1c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store11fingerprint>
100c7ecd0:     	ldr	x8, [x19, #0x60]
100c7ecd4:     	cbz	x8, 0x100c7efbc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x330>
100c7ecd8:     	mov	x8, #0x0                ; =0
100c7ecdc:     	mov	x9, #0xa9c5             ; =43461
100c7ece0:     	movk	x9, #0x2e62, lsl #16
100c7ece4:     	movk	x9, #0x7aea, lsl #32
100c7ece8:     	movk	x9, #0xf135, lsl #48
100c7ecec:     	mul	x9, x0, x9
100c7ecf0:     	ror	x11, x9, #0x2c
100c7ecf4:     	lsr	x12, x11, #57
100c7ecf8:     	ldp	x10, x9, [x19, #0x48]
100c7ecfc:     	dup.8b	v0, w12
100c7ed00:     	movi.2d	v1, #0xffffffffffffffff
100c7ed04:     	and	x11, x11, x9
100c7ed08:     	ldr	d2, [x10, x11]
100c7ed0c:     	cmeq.8b	v3, v2, v0
100c7ed10:     	fmov	x12, d3
100c7ed14:     	ands	x12, x12, #0x8080808080808080
100c7ed18:     	b.eq	0x100c7ed48 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0xbc>
100c7ed1c:     	rbit	x13, x12
100c7ed20:     	clz	x13, x13
100c7ed24:     	add	x13, x11, x13, lsr #3
100c7ed28:     	and	x13, x13, x9
100c7ed2c:     	sub	x13, x10, x13, lsl #4
100c7ed30:     	ldur	x14, [x13, #-0x10]
100c7ed34:     	cmp	x0, x14
100c7ed38:     	b.eq	0x100c7eec4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x238>
100c7ed3c:     	sub	x13, x12, #0x2
100c7ed40:     	ands	x12, x13, x12
100c7ed44:     	b.ne	0x100c7ed1c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x90>
100c7ed48:     	cmeq.8b	v2, v2, v1
100c7ed4c:     	fmov	x12, d2
100c7ed50:     	cbnz	x12, 0x100c7efbc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x330>
100c7ed54:     	add	x8, x8, #0x8
100c7ed58:     	add	x11, x11, x8
100c7ed5c:     	and	x11, x11, x9
100c7ed60:     	ldr	d2, [x10, x11]
100c7ed64:     	cmeq.8b	v3, v2, v0
100c7ed68:     	fmov	x12, d3
100c7ed6c:     	ands	x12, x12, #0x8080808080808080
100c7ed70:     	b.ne	0x100c7ed1c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x90>
100c7ed74:     	b	0x100c7ed48 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0xbc>
100c7ed78:     	ldr	x8, [x19, #0x38]
100c7ed7c:     	cbz	x8, 0x100c7efbc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x330>
100c7ed80:     	mov	x0, x21
100c7ed84:     	bl	0x100b27448 <__RINvYNtCs2Rl17A9a9f3_10rustc_hash13FxBuildHasherNtNtCs4sDCw1iE1MS_4core4hash11BuildHasher8hash_oneRNtNtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storage4NodeEB1H_>
100c7ed88:     	mov	x10, #0x0               ; =0
100c7ed8c:     	lsr	x8, x0, #57
100c7ed90:     	ldp	x25, x24, [x19, #0x20]
100c7ed94:     	dup.8b	v8, w8
100c7ed98:     	ldp	x8, x3, [x21]
100c7ed9c:     	eor	x9, x8, #0x8000000000000000
100c7eda0:     	cmp	x8, #0x0
100c7eda4:     	csinc	x1, x9, xzr, mi
100c7eda8:     	ldp	w20, w22, [x21, #0x8]
100c7edac:     	ldr	w27, [x21, #0x10]
100c7edb0:     	ldp	x23, x26, [x21, #0x10]
100c7edb4:     	lsl	x2, x23, #3
100c7edb8:     	movi.2d	v1, #0xffffffffffffffff
100c7edbc:     	mov	w4, #0x28               ; =40
100c7edc0:     	and	x21, x0, x24
100c7edc4:     	ldr	d9, [x25, x21]
100c7edc8:     	cmeq.8b	v0, v9, v8
100c7edcc:     	fmov	x8, d0
100c7edd0:     	ands	x5, x8, #0x8080808080808080
100c7edd4:     	b.eq	0x100c7ee94 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x208>
100c7edd8:     	rbit	x8, x5
100c7eddc:     	clz	x8, x8
100c7ede0:     	add	x8, x21, x8, lsr #3
100c7ede4:     	and	x8, x8, x24
100c7ede8:     	mneg	x8, x8, x4
100c7edec:     	add	x28, x25, x8
100c7edf0:     	ldur	x8, [x28, #-0x10]
100c7edf4:     	cmp	x26, x8
100c7edf8:     	b.ne	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7edfc:     	ldur	x8, [x28, #-0x28]
100c7ee00:     	eor	x9, x8, #0x8000000000000000
100c7ee04:     	cmp	x8, #0x0
100c7ee08:     	csinc	x8, x9, xzr, mi
100c7ee0c:     	cmp	x1, x8
100c7ee10:     	b.ne	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7ee14:     	cmp	x1, #0x1
100c7ee18:     	b.eq	0x100c7ee4c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1c0>
100c7ee1c:     	cmp	x1, #0x2
100c7ee20:     	b.ne	0x100c7efe8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x35c>
100c7ee24:     	ldur	w8, [x28, #-0x20]
100c7ee28:     	cmp	w20, w8
100c7ee2c:     	b.ne	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7ee30:     	ldur	w8, [x28, #-0x1c]
100c7ee34:     	cmp	w22, w8
100c7ee38:     	b.ne	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7ee3c:     	ldur	w8, [x28, #-0x18]
100c7ee40:     	cmp	w27, w8
100c7ee44:     	b.eq	0x100c7efe8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x35c>
100c7ee48:     	b	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7ee4c:     	ldur	x8, [x28, #-0x18]
100c7ee50:     	cmp	x23, x8
100c7ee54:     	b.ne	0x100c7ee88 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x1fc>
100c7ee58:     	stp	x3, x1, [sp, #0x10]
100c7ee5c:     	ldur	x1, [x28, #-0x20]
100c7ee60:     	mov	x0, x3
100c7ee64:     	mov	x19, x10
100c7ee68:     	stp	x5, x2, [sp]
100c7ee6c:     	bl	0x101284d90 <dyld_stub_binder+0x101284d90>
100c7ee70:     	ldp	x5, x2, [sp]
100c7ee74:     	mov	w4, #0x28               ; =40
100c7ee78:     	movi.2d	v1, #0xffffffffffffffff
100c7ee7c:     	ldp	x3, x1, [sp, #0x10]
100c7ee80:     	mov	x10, x19
100c7ee84:     	cbz	w0, 0x100c7efe8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x35c>
100c7ee88:     	sub	x8, x5, #0x2
100c7ee8c:     	ands	x5, x8, x5
100c7ee90:     	b.ne	0x100c7edd8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x14c>
100c7ee94:     	cmeq.8b	v0, v9, v1
100c7ee98:     	fmov	x8, d0
100c7ee9c:     	cbnz	x8, 0x100c7efbc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x330>
100c7eea0:     	add	x10, x10, #0x8
100c7eea4:     	add	x0, x21, x10
100c7eea8:     	and	x21, x0, x24
100c7eeac:     	ldr	d9, [x25, x21]
100c7eeb0:     	cmeq.8b	v0, v9, v8
100c7eeb4:     	fmov	x8, d0
100c7eeb8:     	ands	x5, x8, #0x8080808080808080
100c7eebc:     	b.ne	0x100c7edd8 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x14c>
100c7eec0:     	b	0x100c7ee94 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x208>
100c7eec4:     	ldur	w22, [x13, #-0x8]
100c7eec8:     	cmn	w22, #0x1
100c7eecc:     	b.eq	0x100c7efbc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x330>
100c7eed0:     	ldr	x26, [x21, #0x18]
100c7eed4:     	ldr	x27, [x21]
100c7eed8:     	ldp	w8, w24, [x21, #0x8]
100c7eedc:     	str	w8, [sp, #0x18]
100c7eee0:     	ldr	w28, [x21, #0x10]
100c7eee4:     	ldp	x25, x20, [x19, #0x38]
100c7eee8:     	tbnz	x27, #0x3f, 0x100c7eff4 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x368>
100c7eeec:     	str	w28, [sp, #0x8]
100c7eef0:     	str	w24, [sp, #0x10]
100c7eef4:     	ldp	x23, x28, [x21, #0x8]
100c7eef8:     	lsl	x24, x28, #3
100c7eefc:     	mov	x21, x22
100c7ef00:     	add	x0, sp, #0x20
100c7ef04:     	mov	x1, x19
100c7ef08:     	mov	x2, x22
100c7ef0c:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100c7ef10:     	ldr	x8, [sp, #0x38]
100c7ef14:     	cmp	x8, x26
100c7ef18:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef1c:     	ldr	w8, [sp, #0x20]
100c7ef20:     	cbz	w8, 0x100c7ef50 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x2c4>
100c7ef24:     	ldr	x0, [sp, #0x28]
100c7ef28:     	cmp	w8, #0x1
100c7ef2c:     	b.ne	0x100c7ef60 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x2d4>
100c7ef30:     	ldr	x8, [sp, #0x30]
100c7ef34:     	cmp	x8, x28
100c7ef38:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef3c:     	mov	x1, x23
100c7ef40:     	mov	x2, x24
100c7ef44:     	bl	0x101284d90 <dyld_stub_binder+0x101284d90>
100c7ef48:     	cbnz	w0, 0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef4c:     	b	0x100c7efec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x360>
100c7ef50:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
100c7ef54:     	cmp	x27, x8
100c7ef58:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef5c:     	b	0x100c7efec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x360>
100c7ef60:     	ldr	w8, [sp, #0x24]
100c7ef64:     	mov	x9, #0x2                ; =2
100c7ef68:     	movk	x9, #0x8000, lsl #48
100c7ef6c:     	cmp	x27, x9
100c7ef70:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef74:     	ldr	w9, [sp, #0x18]
100c7ef78:     	cmp	w8, w9
100c7ef7c:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef80:     	ldr	w8, [sp, #0x10]
100c7ef84:     	cmp	w8, w0
100c7ef88:     	b.ne	0x100c7ef9c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x310>
100c7ef8c:     	lsr	x8, x0, #32
100c7ef90:     	ldr	w9, [sp, #0x8]
100c7ef94:     	cmp	w9, w8
100c7ef98:     	b.eq	0x100c7efec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x360>
100c7ef9c:     	lsr	w8, w21, #1
100c7efa0:     	cmp	x20, x8
100c7efa4:     	b.ls	0x100c7f118 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x48c>
100c7efa8:     	ldr	w22, [x25, x8, lsl #2]
100c7efac:     	cmn	w22, #0x1
100c7efb0:     	b.ne	0x100c7eefc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x270>
100c7efb4:     	mov	w0, #0x0                ; =0
100c7efb8:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7efbc:     	mov	w0, #0x0                ; =0
100c7efc0:     	mov	x1, x21
100c7efc4:     	ldp	x29, x30, [sp, #0xa0]
100c7efc8:     	ldp	x20, x19, [sp, #0x90]
100c7efcc:     	ldp	x22, x21, [sp, #0x80]
100c7efd0:     	ldp	x24, x23, [sp, #0x70]
100c7efd4:     	ldp	x26, x25, [sp, #0x60]
100c7efd8:     	ldp	x28, x27, [sp, #0x50]
100c7efdc:     	ldp	d9, d8, [sp, #0x40]
100c7efe0:     	add	sp, sp, #0xb0
100c7efe4:     	ret
100c7efe8:     	ldur	w21, [x28, #-0x8]
100c7efec:     	mov	w0, #0x1                ; =1
100c7eff0:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7eff4:     	mov	x8, #0x2                ; =2
100c7eff8:     	movk	x8, #0x8000, lsl #48
100c7effc:     	cmp	x27, x8
100c7f000:     	b.ne	0x100c7f08c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x400>
100c7f004:     	mov	x23, #-0x8000000000000000 ; =-9223372036854775808
100c7f008:     	mov	x21, x22
100c7f00c:     	add	x0, sp, #0x20
100c7f010:     	mov	x1, x19
100c7f014:     	mov	x2, x22
100c7f018:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100c7f01c:     	ldr	x8, [sp, #0x38]
100c7f020:     	cmp	x8, x26
100c7f024:     	b.ne	0x100c7f06c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3e0>
100c7f028:     	ldr	w8, [sp, #0x20]
100c7f02c:     	cbz	w8, 0x100c7f064 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3d8>
100c7f030:     	cmp	w8, #0x1
100c7f034:     	b.eq	0x100c7f06c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3e0>
100c7f038:     	ldr	w8, [sp, #0x24]
100c7f03c:     	ldr	w9, [sp, #0x18]
100c7f040:     	cmp	w8, w9
100c7f044:     	b.ne	0x100c7f06c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3e0>
100c7f048:     	ldr	x8, [sp, #0x28]
100c7f04c:     	cmp	w24, w8
100c7f050:     	b.ne	0x100c7f06c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3e0>
100c7f054:     	lsr	x8, x8, #32
100c7f058:     	cmp	w28, w8
100c7f05c:     	b.ne	0x100c7f06c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x3e0>
100c7f060:     	b	0x100c7efec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x360>
100c7f064:     	cmp	x27, x23
100c7f068:     	b.eq	0x100c7efec <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x360>
100c7f06c:     	lsr	w8, w21, #1
100c7f070:     	cmp	x20, x8
100c7f074:     	b.ls	0x100c7f118 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x48c>
100c7f078:     	mov	w0, #0x0                ; =0
100c7f07c:     	ldr	w22, [x25, x8, lsl #2]
100c7f080:     	cmn	w22, #0x1
100c7f084:     	b.ne	0x100c7f008 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x37c>
100c7f088:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7f08c:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
100c7f090:     	cmp	x27, x8
100c7f094:     	b.ne	0x100c7f0dc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x450>
100c7f098:     	add	x0, sp, #0x20
100c7f09c:     	mov	x1, x19
100c7f0a0:     	mov	x2, x22
100c7f0a4:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100c7f0a8:     	ldr	x8, [sp, #0x38]
100c7f0ac:     	cmp	x8, x26
100c7f0b0:     	b.ne	0x100c7f0bc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x430>
100c7f0b4:     	ldr	w8, [sp, #0x20]
100c7f0b8:     	cbz	w8, 0x100c7f10c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x480>
100c7f0bc:     	lsr	w8, w22, #1
100c7f0c0:     	cmp	x20, x8
100c7f0c4:     	b.ls	0x100c7f118 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x48c>
100c7f0c8:     	mov	w0, #0x0                ; =0
100c7f0cc:     	ldr	w22, [x25, x8, lsl #2]
100c7f0d0:     	cmn	w22, #0x1
100c7f0d4:     	b.ne	0x100c7f098 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x40c>
100c7f0d8:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7f0dc:     	add	x0, sp, #0x20
100c7f0e0:     	mov	x1, x19
100c7f0e4:     	mov	x2, x22
100c7f0e8:     	bl	0x100c7f12c <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4node>
100c7f0ec:     	lsr	w8, w22, #1
100c7f0f0:     	cmp	x20, x8
100c7f0f4:     	b.ls	0x100c7f118 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x48c>
100c7f0f8:     	mov	w0, #0x0                ; =0
100c7f0fc:     	ldr	w22, [x25, x8, lsl #2]
100c7f100:     	cmn	w22, #0x1
100c7f104:     	b.ne	0x100c7f0dc <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x450>
100c7f108:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7f10c:     	mov	w0, #0x1                ; =1
100c7f110:     	mov	x21, x22
100c7f114:     	b	0x100c7efc0 <__RNvMs_NtNtNtCs23EhFSy3h49_8bumbledb14event_repr_lab13essential_raw7storageNtB4_5Store4find+0x334>
100c7f118:     	adrp	x2, 0x1014fb000 <dyld_stub_binder+0x1014fb000>
100c7f11c:     	add	x2, x2, #0xf40
100c7f120:     	mov	x0, x8
100c7f124:     	mov	x1, x20
100c7f128:     	bl	0x10127c89c <__RNvNtCs4sDCw1iE1MS_4core9panicking18panic_bounds_check>
