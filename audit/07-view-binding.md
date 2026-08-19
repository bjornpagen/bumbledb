# 07 — `ViewMemo`: the active binding is still parallel vectors with a three-meaning `None`

- **Status:** **keep** — `Binding` arms are distinct proofs (unbound /
  derived / bound). Collapsing `None` would discard which proof held.
  Brooks: four vectors are accidental; the three meanings of absence
  are essential and must stay a sum, not one type.
- **Severity:** should-fix.
- **Supersedes:** PROP-006, REP-10, SPINE-17.
- **Adjudication (third pass): keep CONTESTED — owner ruling required.**
  The keep-ruling defends this file's fix, not the status quo. "The three
  meanings of absence are essential and must stay a sum" is exactly what
  `Binding::{Unbound, Derived, Bound}` provides — three arms, three distinct
  proofs. What the CURRENT code has is one `None` conflating all three plus
  an `expect` re-deriving which one held. And by the ruling's own words the
  four parallel vectors are accidental — so at minimum the vector
  unification onto one per-occurrence record proceeds under both readings.
  Nothing in this file collapses the proofs; it un-collapses them.

## Principle

Insight 4: `Vec<Option<ViewEpoch>>` beside `colts` beside `filters` beside
`parked` is a state machine spread over same-length vectors whose coupling is
convention, and whose `None` means three unrelated things — never executed,
just parked-and-vacated, derived-occurrence-never-uses-epochs. `ParkedView`
one field over is already the right record; the active binding deserves the
same shape.

## Evidence

- `crates/bumbledb/src/api/prepared.rs:796-805` — `ViewMemo { colts,
  epoch: Vec<Option<ViewEpoch>>, filters, parked, … }` with the doc comment
  itself spelling the overload ("`None` = unbound").
- Park/unpark hand-swaps the vectors in lockstep and proves coherence with
  `expect("a parked hit implies an executed active binding")`.

## The fix

Keep `colts: Vec<Colt>` separate — the kernel takes `&mut [Colt]` and that
boundary is recorded. Everything else collapses to one slot per occurrence:

```rust
enum Binding {
    Unbound,
    Derived,
    Bound { epoch: ViewEpoch, filters: Vec<FilterPredicate>, last_used: u64 },
}

struct OccMemo {
    active: Binding,
    parked: [Option<Bound>; PARKED_SLOTS],
    spare: Vec<u32>,
}
```

Three meanings become three arms; park/unpark is a move between identical
`Bound` shapes; the `expect` and the length-coupling convention delete. The
`ViewEpoch` rename already landed — this finishes threading the new name
through a new shape instead of the old one.

## Acceptance

- `ViewMemo` holds `colts` plus `Vec<OccMemo>` (or equivalent); no
  `Vec<Option<ViewEpoch>>` remains.
- The park/unpark `expect` is gone; a `Derived` occurrence cannot be parked
  by construction (no arm for it).
- Scenario lanes byte-identical; memo hit/miss counters unchanged.
