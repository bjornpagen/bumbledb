# engine-020: querygen's reach/interiors arm is a side entry, and interiors-only shapes live in a `Recursive` enum

- **Severity:** medium
- **Tree:** engine (bench generator)
- **Status:** OPEN
- **Source:** audit/engine.md F20
- **Depends on:** none (bench-local; parallel-safe)

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

- Top-level generator sum: `enum QueryClass { Cq(Shape), Derived(DerivedShape) }` with ONE entry (`random_query` draws the class, then the shape) — or `Shape` gains the derived rows directly; either way `random_reach_query` stops being a separate public entry callers must know about (it may remain as the internal constructor).
- Split the tag: `enum DerivedShape { Interiors(InteriorsShape), Rec(RecShape) }` — `InteriorsDag`/`InteriorsAntiJoin`/`ManyInteriors` under `Interiors`, the five rec rows under `Rec`. `RecursiveCoverage`/`recursive_coverage` follow the split so coverage reports name what ran.
- Callers (differential harness, coverage tests) updated mechanically; drawn distributions may change ONLY by making reach/interiors draws part of the one entry — record chosen weights in `SHAPE_WEIGHTS`-style data.

## Acceptance criteria

- [ ] One entry: `rg -n 'random_reach_query' crates/bumbledb-bench/src --glob '!querygen/*'` → no external callers (harness draws through the one entry); the module doc paragraph quoted above is gone.
- [ ] Honest tags: `rg -n 'InteriorsDag|InteriorsAntiJoin|ManyInteriors' crates/bumbledb-bench/src` shows them under an interiors/derived tag, not `RecursiveVariant`.
- [ ] Unchanged: all differential/coverage tests green (coverage thresholds may need re-derivation if the distribution changed — do NOT lower any coverage assertion; if a threshold fails, adjust weights until the old coverage holds).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Coverage may not regress; assertions may not weaken. Bench-local — no engine types change.
