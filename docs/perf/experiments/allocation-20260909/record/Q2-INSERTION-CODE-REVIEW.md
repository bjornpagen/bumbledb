# Q2 insertion code: bounded static review

Evidence: q2-codegen-1 exports the ACTUAL q2-time-build-q1-1 and q2-time-build-q2-1
ordinary executables. No new build, timing, CPU sample or trace was required.
Hashes are recorded in its STATE.json and Q2-ORDINARY-TIMING-REVIEW.md.

## Two-word scalar path, not extrapolation from arity four

q2-function-3.log dispatches key width two through 0x10013499c to 0x100134c08.
Its normal non-growth probe begins 0x100135fe8; live ctrl/stamp validation
precedes slot-to-row load 0x1001360a0 and the two-word compare at 0x1001360b8.
Duplicate success jumps to 0x100136a00, which loads/checks values.len even for
WordMap<()> before returning false. This is an unused value-reference bounds
check, not a key-validation check; key/ctrl/stamp bounds must remain safe.

Vacant insertion at 0x100136108 checks the u32 row ordinal and ctrl slot.
0x100136138–0x10013614c checks key capacity and ZST values capacity overflow.
0x100136158 sets width TWO; 0x10013615c calls Vec<u64>::append_elements.
After the call it reloads values.len, checks overflow again, and increments
that ZST length before publishing the index and incrementing a separate map
length. The shared helper also performs its own key reservation. Out-of-line
copy and duplicate row counts are real static work, not measured cycle shares.

## Two-word bulk path differs

q2-function-5.log dispatches width two to 0x100138b80. Normal probing is at
0x100138c8c, slot-to-row access at 0x100138d44. Duplicate success checks the
ZST value length at 0x100138d7c; the load-boundary duplicate path repeats this
at 0x100138e3c. These checks exist even though insert_rows ignores the reference.

Vacant insertion at 0x100138e78 has key-capacity checks BOTH at 0x100138ea8
and 0x100138ec8. Unlike scalar insertion, the copy is already inlined:
ldr/str q0 at 0x100138ed8/0x100138edc, key-length +2 at 0x100138ee0.
ZST value length is checked again, incremented/stored at 0x100138ef4/ef8.
Then separate map length increments at 0x100138b5c and the returned reference
adds another unused bounds check at 0x100138b68.

Therefore a copy-call explanation alone cannot explain all warm losses. Both
paths retain extra bookkeeping; only scalar has the observed copy call. The
sinks mix scalar and bulk consumers. The saved numeric panel establishes
two-word map ownership but does not count their call proportions; do not
pretend static dispatch establishes dynamic attribution.

Scalar hashed insertion grows 9,776 -> 11,220 bytes, bulk 9,020 -> 11,256,
group probe 8,744 -> 11,496 and text resolve 10,044 -> 10,804. Proved-unique
insertion (512) and scratch insertion (5,056) are unchanged. Function size is
not instruction counts executed or cycle cost. RawVec<()>::grow_one is a cold
overflow branch; it is NOT evidence of warm allocation.

## Selected follow-up: make dense publication single-source

Keep Q2 frozen. On a separate Q3 candidate, retain one generic WordMap and:

1. Use values.len as the one row cardinality, including zero-width keys and
   zero-sized values. Delete the redundant map len field.
2. Make shared internal entries return (dense row ordinal, inserted), and
   borrow a mutable value only in get_or_insert_with. insert and insert_rows
   must not manufacture discarded value references. All real key/stamp/slot
   validation stays safe. Do not special-case () with unsafe references.
3. Reserve value capacity first; let extend_from_slice on Copy u64 keys do
   its own one reservation/copy. Delete the earlier redundant key reserve.
   Constructor and value reserve precede key-length mutation; key reserve
   failure leaves keys unchanged. After the copy, the already-reserved Copy
   value push cannot call user code or allocate. Publish ctrl/stamps last.

This is a coherent removal of redundant publication state, not a new hash
implementation, growth policy or load factor. Do NOT also tune the copy helper,
group-value representation, probe loop, planner, SDK or disk layout here.
Keep generic mutable values for ResolveMemo and group semantics unchanged.

Obligations: preserve scalar/bulk agreement at widths 1–8/dynamic; zero-width
entries; duplicate closure not called, including load boundary; constructor
panic before and after growth; value mutation; dense order and iter_since;
stale ordinals and 600 clears; grow payload-pointer stability; u32 spill
boundary. Replace the synthetic len-field test with vec![(); count] (the
standard library's unit specialization, no allocation or loop), not resize's
generic Clone loop or unsafe set_len. Exact owner totals should remain Q2's;
transient allocation ORDER can change, so recheck peak/counter assumptions.

Inspect actual fresh ordinary generated code before any speed claim. If the
redundant checks/stores remain, report that; do not assume source cleanup
improves code generation. Then predeclare targeted ordinary affected cases,
negative controls, and saved-corpus text consumer coverage. Full tracing stays
disabled, and no candidate is accepted by this static diagnosis.

The unit-vector distinction was checked in the installed compiler's
alloc/src/vec/{mod.rs,spec_from_elem.rs}: resize calls extend_with's per-element
clone loop; SpecFromElem for () constructs a length directly. No giant debug
loop was run to discover this.
