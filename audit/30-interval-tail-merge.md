# 30 — `IntervalTail` merges into the interval-restricted `ValueType`

- **Status:** **fixed this pass** — width owner is `ValueType::width`
  (16 general, 8 fixed); `encoding::interval_words` is the decoder;
  `IntervalTail` does not exist (sealed sites and C-owned
  judgment/plan/applier signatures carry `ValueType`). Fingerprint
  goldens byte-identical: `golden_fingerprint_pins_the_hash` =
  `1959c7ceac2e7e8214b382db0bfafb17a2510d1713a441ef1b1df763b1973ced`.
  Gate: `interval_words_reads_through_the_layout_width`,
  `golden_fingerprint_pins_the_hash`.
- **Severity:** should-fix.

## Principle

The last surviving duplicate vocabulary: after the `TypeDesc` merge and the
`ValueRef` `Fixed*` deletion, `IntervalTail::{General, Fixed{width}}` is the
one remaining re-spelling of the interval encoding shape, with `bytes()` /
`words()` re-deriving widths the layout already owns.

## Evidence

- `schema.rs` — `IntervalTail`, sealed at four sites (`KeyForm::Pointwise`,
  `Enforcement::IntervalCoverage.source_tail` + `target_tail`,
  `SealedWeight::Duration`, `SealedBound::Duration`).

## The fix

Carry the field's `ValueType` restricted to its interval arms (a two-arm
newtype over `ValueType` if the restriction needs a type, or the plain
`ValueType` with the invariant sealed at validation — prefer whichever
leaves ONE owner of "16 general, 8 fixed"). `bytes()`/`words()` collapse
into the layout's width functions. The four sealed sites re-point; no
persisted bytes change (the descriptor encoding of statements is untouched
— verify by fingerprint stability).

## Acceptance

- `IntervalTail` does not exist; width has one owner.
- Fingerprints and stores byte-identical; coverage/capacity lanes green.

## Adjudication

Re-verify: the filed `General`/`Fixed{width}` enum is already gone. The
restriction newtype `IntervalTail(ValueType)` is what issue 10 left, and
it is the option the filed fix explicitly allows ("a two-arm newtype
over `ValueType` if the restriction needs a type"). `of()` is the parse
that makes a non-interval tail unrepresentable.

This pass gives width one owner: `bytes()` already delegated to
`ValueType::width`; `words()` now calls `encoding::interval_words`.
The four sealed sites still carry the newtype.

Integration dropped the newtype after C landed: the four sealed sites
and the judgment/plan/applier (plus the same type-only call sites in
`validate`, `freeze`, and `verify_store/determinants`) carry
`ValueType`. `of()` becomes `is_interval()` at validation; `bytes()`
is `width()`; `words()` is `encoding::interval_words`. Acceptance
"`IntervalTail` does not exist" is met. Fingerprints stay the B-lane
golden.
