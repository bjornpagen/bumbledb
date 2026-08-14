# bench-005: two expressibility gates for one translator

- **Severity:** medium
- **Tree:** bench
- **Status:** FIXED(deff26c0)
- **Source:** audit/bench.md F5
- **Depends on:** engine-021 (the reach-named gate merges into this one function; land together or this after)

## The bug

`crates/bumbledb-bench/src/translate.rs:229-261` — `sqlite_expressible` on a `Query` checks Pack and returns `Ok` for everything else, including interiors/rec. `sqlite_reach_expressible` (`translate/reach.rs:14`, engine-021) screens `IntervalDerivedColumn` and is named for rec while screening interiors.

Verify's randomized lane (`verify/run.rs:152`) uses the first. Reach tests (`differential/tests/recursive.rs:256,529`, `querygen/tests.rs:582`) use the second. `translate.rs:39-44` still documents "Interiors + rec = `WITH [RECURSIVE]` ([`reach`])."

## Why it's wrong

One translator input (a Query) is dispatched to two gates selected by which generator entry produced it — the same two-flag product as engine-021's front door, one layer up (Insight 1). `sqlite_expressible` claiming every non-Pack Query is expressible is false once interiors carry interval columns.

## The fix

Per engine-021: ONE `sqlite_expressible(&Query)` (or `LaneCase::Query`): Pack, then interval-typed derived columns. `sqlite_reach_expressible` dies into it. Callers do not choose a gate by shape. Module docs speak `WITH [RECURSIVE]` as the translator spelling of derived tables, not a module named `reach`.

## Acceptance criteria

- [ ] Gone: `rg -n 'sqlite_reach_expressible' crates/bumbledb-bench/src` → no matches (renamed/merged; engine-021's criterion, confirmed here).
- [ ] One Query gate: `sqlite_expressible(&LaneCase::Query(q))` refuses `IntervalDerivedColumn` on interiors-or-rec the way `sqlite_reach_expressible` does today.
- [ ] Unchanged: Pack routing in `verify/run.rs` still enumerates, never silently skips; three-way SQL answers byte-identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Coordinate with engine-021 (one rename). `WITH` vs `WITH RECURSIVE` semantics locked. No Program vocabulary.
