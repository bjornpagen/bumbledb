# engine-004: empty `Rec.base`/`Rec.rec` stay representable on the witness after being refused

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F4
- **Depends on:** engine-005 (the typed `ValidatedRec` is where nonemptiness lives)

## The bug

The boundary refusals exist and even explain themselves — `crates/bumbledb/src/error.rs:859-862`:

```rust
/// `Rec.base` is empty: a constantly-empty rec (math: `T(∅) = ∅`).
EmptyRecursiveBase,
/// `Rec.rec` is empty: that is an interior — write an interior.
EmptyRecursiveStep,
```

but the check happens twice on the same fact at different stages — `ir/validate/validate.rs` rejects empty lists in `rec_roster` (~245-251) and then `lower_rec_pool` (~340-345) rejects emptiness *again* after DNF distribution — and the witness type (`ValidatedRec { base: Vec<LoweredRule>, rec: Vec<LoweredRule>, … }`) still admits the refused state, which is why downstream indexes `[0]` on faith.

## Why it's wrong

Parse, don't validate (Insight 6): the roster refusal establishes "both arms nonempty" and then returns a type that cannot say so, so the fact is re-checked once (post-DNF) and assumed everywhere else. The two emptiness checks also conflate two different facts: a *written* empty arm (roster error, the representation talking — "write an interior") and a DNF-collapsed pool (a nonempty written arm whose lowering died) — reusing the same variant for both muddies which boundary refused (Insight 3).

## The fix

Per `audit/CONTRACT.md §C3`:

- `ValidatedRec`'s arms are nonempty by construction on the witness: either a `NonEmpty<T>` (first + rest) layout or a constructor private to the validate module that refuses empties once. Downstream `[0]` reads become total (`first()` on the nonempty carrier).
- The written-arm emptiness check happens exactly once, in `rec_roster`, with the existing names `EmptyRecursiveBase`/`EmptyRecursiveStep` (locked).
- A post-DNF-emptied pool is judged where it happens (`lower_rec_pool`) but is a DIFFERENT parsed outcome — keep the same public error names if tests pin them (check `crates/bumbledb/src/ir/validate/tests` and `tests/adversarial_ir.rs` first; if a test distinguishes the stage, it stays; do not weaken), but the code path must be one check per fact, not one fact checked at two stages with the same variant. If both stages genuinely observe distinct facts (written-empty vs. lowered-empty), record that in a comment naming both; do not delete either observable refusal.

## Acceptance criteria

- [x] Witness cannot spell it: `rg -n 'base: Vec<LoweredRule>|rec: Vec<LoweredRule>' crates/bumbledb/src/ir/validate.rs` → no matches (nonempty carrier instead).
- [x] Unchanged tests: every adversarial test asserting `EmptyRecursiveBase`/`EmptyRecursiveStep` passes UNCHANGED (names and trigger inputs identical).
- [x] New locks: `a_rec_step_whose_dnf_is_empty_is_empty_recursive_step` pins today's `EmptyRecursiveStep` for a nonempty written arm whose DNF is `Or([])`.
- [x] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb --lib ir::validate` (127 passed).

## Constraints

- The untrusted `ir.rs::Rec` stays open (hostile input must be representable to be refused by name) — CONTRACT §C1.
- Error NAMES locked; `T(∅)=∅` semantics untouched. Lands with/after engine-005.
