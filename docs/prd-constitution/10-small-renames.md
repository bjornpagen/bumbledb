# PRD 10 — Small renames: stride, cardinality, origin capacity, typed order refusals

**Depends on:** 08 (image/ and error.rs quiet).
**Modules:** `crates/bumbledb/src/image/{pitch.rs,tests/pitch.rs}` +
`image.rs`/`image/build.rs` (pitch, 40 hits), `image/distinct.rs` (12
hits), `error.rs` (`OverflowKind::Origins` :857), `ir/validate/context.rs`
(`screen_order_operand` :298-308 and the generic order-refusal arms
:1075/:1104/:1131), `error/display.rs`, docs (50-storage pitch ×4,
00-product ×2).
**Authority:** the audit's misleading-names table, verified current;
four independent small cuts batched to one PRD because each is
mechanical and none interacts.
**Representation move:** one — the str/bool order refusal gains typed
diagnostics (a small unrepresentable-state upgrade: the generic
`IllegalComparison` stops covering four distinguishable causes).

## Context (decided shape) — four cuts

1. **pitch → stride.** `image/pitch.rs` → `image/stride.rs`;
   `PitchPadder` → `StridePadder`; `PAD_MIN_PITCH` → `PAD_MIN_STRIDE`;
   tests file follows; docs mentions follow ("cache-line stride
   padding"). The measured numbers in comments move verbatim.
2. **image/distinct → image/cardinality.** The module computes lazy
   per-column exact distinct-value counts for the estimator —
   cardinality statistics. `image/distinct.rs` →
   `image/cardinality.rs`; `DistinctCounter` → `CardinalityCounter`;
   `fn distinct(column)` → `fn cardinality(column)`. NOT touched:
   `CountDistinct` (the aggregate — correct name), `provably_distinct`
   (PRD 17 retypes it, name stays), "distinct bindings" prose (the
   semantic term).
3. **OverflowKind::Origins → OverflowKind::OriginCapacity.** It is a
   counter-capacity exhaustion, not arithmetic overflow; the display
   string follows ("origin capacity exceeded"). `OverflowKind::
   Aggregate` untouched (that one IS arithmetic).
4. **Typed order refusals.** `screen_order_operand` grows two arms:
   `ValidationError::OrderComparisonOnString { index }` and
   `OrderComparisonOnBool { index }` (siblings of the existing
   `OrderComparisonOnInterval`/`OnFixedBytes`), replacing the
   fall-through to the generic gate for these two types. The generic
   `IllegalComparison` remains for genuinely-mixed-type operands only.
   Display arms mirror the interval/bytes wording. The existing
   rejection tests re-anchor to the new variants; NEW lock tests pin
   str-order and bool-order rejection explicitly (both written orders).

## Technical direction

Four independent compiler-driven passes; land as one commit. For cut 4,
read the classify arms (context.rs:1075, 1104, 1131) first — the
screen happens per operand BEFORE classification, so the new arms live
in `screen_order_operand` and the generic arms become unreachable for
single-type str/bool operands; verify by making the generic arm's
str/bool path a compile-visible dead end (the screen returns Err
first), not by leaving both.

## Passing criteria

- `[shape]` `grep -rni "pitch" crates docs/architecture` → zero;
  `grep -rn "image/distinct\|DistinctCounter" crates` → zero;
  `grep -rn "OverflowKind::Origins\b" crates` → zero.
- `[shape]` `grep -n "OrderComparisonOnString\|OrderComparisonOnBool" crates/bumbledb/src/error.rs` → both present.
- `[test]` str-order and bool-order rejection locks green (dedicated
  variants asserted); pitch/stride tests green with unchanged measured
  values; full suite green.
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`50-storage.md` + `00-product.md` stride wording; `20-query-ir.md`'s
comparison table lists the four typed order refusals.
