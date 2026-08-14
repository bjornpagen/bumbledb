# engine-018: two names for one planning floor; the delta/finished choice rides a side channel

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F18
- **Depends on:** engine-007/engine-039 (the side channel dies with the rec-arm prepare split)

## The bug

`crates/bumbledb/src/plan/selectivity.rs:95-110`:

```rust
pub(crate) const DELTA_PLANNING_ROWS: u64 = 1;
pub(crate) const ACCUMULATED_PLANNING_ROWS: u64 = 16;
/// Finished-interior (and main's read of finished rec) planning row
/// count: equal to the accumulated floor — ...
pub(crate) const INTERIOR_PLANNING_ROWS: u64 = ACCUMULATED_PLANNING_ROWS;
```

The third constant exists to preserve a distinction the numbers deny. The comment at 89-94 still says "a delta-variant plan's marked occurrence" (dead vocabulary, engine-007), and the delta/finished choice reaches the planner as `delta == Some(occ_id)` — an argument beside the rule (`build.rs`, `prepare_rule_variant`'s `delta: Option<OccId>`, engine-039) instead of a property of the occurrence.

## Why it's wrong

Two floors are essential (frontier vs. table); an alias constant creates a *third name* implying a third class that isn't there, and the choice between the two real floors travels as a side-channel Option rather than as data on the thing it describes (Insight 5). If the numbers ever diverge, which sites meant "interior" and which meant "accumulated"? The alias makes that future bug easy.

## The fix

Per `audit/CONTRACT.md §C3`:

- Two constants only: `DELTA_PLANNING_ROWS = 1` and one finished/accumulated floor (keep the name `ACCUMULATED_PLANNING_ROWS` or rename to `FINISHED_DERIVED_PLANNING_ROWS` — one name, one number, doc explaining it covers rec-accumulated reads, finished interiors, and main's finished-rec reads alike). Delete `INTERIOR_PLANNING_ROWS`.
- The floor an occurrence gets is decided by the occurrence's derived-bind role (engine-017): `RecDelta → DELTA_PLANNING_ROWS`, other derived → the finished floor; `occurrence_stats`' caller stops threading the choice as `Option<OccId>` (engine-039's `prepare_rec_arm(delta: OccId)` marks the occurrence before stats run).
- Comment vocabulary: "a rec arm's marked delta occurrence", no "variant".

## Acceptance criteria

- [ ] Gone: `rg -nw 'INTERIOR_PLANNING_ROWS' crates/bumbledb/src` → no matches; `rg -n 'delta-variant' crates/bumbledb/src` → no matches.
- [ ] Unchanged: planned join orders identical on the whole test corpus (the floors' VALUES don't change) — full `cargo test -p bumbledb` and bench differential suites green unchanged.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Floor values 1 and 16 locked (plan changes would be semantic drift in cost estimates). Lands after engine-017/039 shape the occurrence role, or with them.
