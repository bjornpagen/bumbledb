# engine-020: querygen's reach/interiors arm is a side entry, and interiors-only shapes live in a `Recursive` enum

- **Severity:** medium
- **Tree:** engine (bench generator)
- **Status:** OPEN
- **Source:** audit/engine.md F20
- **Depends on:** bench-004, bench-005 / engine-021 (randomized entry only; corpus reconstructers are C1-frozen and independent)

## The bug

`crates/bumbledb-bench/src/querygen.rs:19-22` — the module doc states the sidecar proudly:

```rust
//! The **reach/interiors arm** ([`random_reach_query`],
//! `shapes_recursive.rs`) is its own entry beside [`random_query`], not
//! a [`Shape`] row: ...
```

`SHAPE_WEIGHTS` (58-75) enumerates sixteen CQ shapes; reach coverage is a second generator bolted on. And `shapes_recursive.rs:14-31` files interiors-only shapes under a *Recursive* tag:

```rust
pub enum RecursiveVariant {
    Linear, Negation, Fold, EmptyDelta, PrimerReachXx,
    InteriorsDag,        // interiors-only
    InteriorsAntiJoin,   // interiors-only
    ManyInteriors,       // interiors-only
}
```

## Why it's wrong

The generator's grammar is the distribution the differential harness actually explores; a side entry means reach queries only appear where a caller remembered the second function, and "interiors-only" classified as `RecursiveVariant` misdescribes the coverage ledger (a coverage report saying "recursive: ManyInteriors" is wrong on its face — Insight 1). It is the engine's sidecar layout (engine-001) reproduced in the test grammar (Insight 2).

## The fix

Two consumers, two laws. Mixing them would regenerate the 268 checked-in cases (C1).

- **Randomized lanes** (verify stamp, opgen fuzz, contradict, `differential/tests/{closed,fold}.rs`, `corpus_gen/rng.rs` digest): one public entry that can draw interiors/rec. `enum QueryClass { Cq(Shape), Derived(DerivedShape) }` — `random_query` used by those callers draws the class, then the shape. Co-land with bench-004 (walks / `EdbAtom`) and bench-005 / engine-021 (one expressibility gate). Otherwise a rec draw panics `atom.relation()` or hits `expect("expressible queries translate")` on interval-derived columns.
- **Corpus reconstructers stay frozen** (C1): `conformance.rs` seeded replay/build (`:1527`, `:1663`) must keep **today's CQ `SHAPE_WEIGHTS` stream** — a new class coin-flip at the start of that function changes every seed→query map and the 246 files fail byte-identity. `conformance/reach.rs` (`:485`, `:555`) must keep **today's `rng.range(8)` arm mapping** and the same constructors. `random_reach_query` remains the reach-corpus reconstructer (internal is fine; deleting the call sites is not).
- **Honest tags without breaking provenance:** coverage reports may classify `InteriorsDag` / `InteriorsAntiJoin` / `ManyInteriors` as interiors, not recursive. Do **not** change `RecursiveVariant`'s `Debug` names that `reach-*.json` provenance embeds (`"variant":"{variant:?}"`). Mapping after the draw is enough; renaming the enum variants changes replay documents.

## Acceptance criteria

- [ ] Stamp/fuzz/contradict draw interiors-or-rec through the one randomized entry (bench-003's pin), without a second public "remember to call reach" requirement on those paths.
- [ ] Frozen reconstructers: `conformance.rs` seeded path still calls a CQ-only draw whose RNG stream matches today's `random_query`; `conformance/reach.rs` still calls `random_reach_query` with the same `range(8)` mapping. `git diff --stat lean/conformance/cases` empty.
- [ ] Honest coverage labels: interiors-only rows are not reported as "recursive" in coverage output. Provenance `Debug` strings in reach cases unchanged.
- [ ] Unchanged: all differential/coverage tests green (do NOT lower any coverage assertion; if a threshold fails, adjust the *randomized* weights, never the corpus reconstructers).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh`.

## Constraints

- C1: 268 cases frozen. Coverage may not regress; assertions may not weaken. Bench-local — no engine types change. Blocked on bench-004 + bench-005 for the randomized entry; corpus reconstructers are not blocked on those.
