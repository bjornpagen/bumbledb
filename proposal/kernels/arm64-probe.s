	.build_version macos, 27, 0	sdk_version 27, 0
	.section	__TEXT,__text,regular,pure_instructions
	.globl	_event_complement               ; -- Begin function event_complement
	.p2align	2
_event_complement:                      ; @event_complement
	.cfi_startproc
; %bb.0:
	eor	x0, x0, #0x1
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_steal_block                    ; -- Begin function steal_block
	.p2align	2
_steal_block:                           ; @steal_block
	.cfi_startproc
; %bb.0:
	ldr	q0, [x1]
	ldr	q1, [x2]
	ldr	q2, [x3]
	orr.16b	v1, v2, v1
	bic.16b	v0, v0, v1
	str	q0, [x0]
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_boolean4_block                 ; -- Begin function boolean4_block
	.p2align	2
_boolean4_block:                        ; @boolean4_block
	.cfi_startproc
; %bb.0:
	ldr	q0, [x1]
	ldr	q1, [x2]
	and	w8, w4, #0x1
	neg	x8, x8
	dup.2d	v2, x8
	ubfx	w8, w4, #1, #1
	neg	x8, x8
	dup.2d	v3, x8
	ubfx	w8, w4, #2, #1
	neg	x8, x8
	dup.2d	v4, x8
	ubfx	w8, w4, #3, #1
	neg	x8, x8
	dup.2d	v5, x8
	bit.16b	v4, v5, v1
	bsl.16b	v1, v3, v2
	bsl.16b	v0, v4, v1
	ldr	q1, [x3]
	and.16b	v0, v0, v1
	str	q0, [x0]
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_predicate_lookup16             ; -- Begin function predicate_lookup16
	.p2align	2
_predicate_lookup16:                    ; @predicate_lookup16
	.cfi_startproc
; %bb.0:
	ldr	q0, [x2]
	ldr	q1, [x1]
	tbl.16b	v0, { v0 }, v1
	str	q0, [x0]
	ret
	.cfi_endproc
                                        ; -- End function
	.no_dead_strip	_boolean4_block
	.no_dead_strip	_event_complement
	.no_dead_strip	_predicate_lookup16
	.no_dead_strip	_steal_block
.subsections_via_symbols
