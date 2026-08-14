# engine-017: `source.edb().is_none()` is the bind dispatch — the complement of EDB is not one thing

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN (scoped — the `AtomSource` re-encoding half is refused per CONTRACT §C1)
- **Source:** audit/engine.md F17
- **Depends on:** engine-010 (`DerivedBind` is the collapsing type), engine-018 (occurrence-level planning role)

## The bug

`crates/bumbledb/src/ir.rs:97-105` — `AtomSource::interior()` returns `Some` for the rec too (the rec IS addressed as an interior id). Downstream, "not EDB" is the entire dispatch:

```rust
// run_join.rs:96 — the derived-bind path
if occurrence.source.edb().is_none() {
    let image = idb_images[occ_idx].as_ref().expect(...)
// plan/selectivity.rs:139 — "THE GUARD"
let Some(relation) = occurrence.source.edb() else { floor }
// reach.rs:483, 527; build.rs:411 — same negative test
if occurrence.role.discharged() || occurrence.source.edb().is_some() { continue; }
occurrence.source.interior() == Some(rec_id)
```

The complement of EDB is three different binds — finished interior, rec delta, rec accumulated — recovered later by Option soup (engine-010) because the source type collapsed them.

## Why it's wrong

A binary test carries a ternary distinction, so the third coordinate re-enters through side channels (`rec_id == Some(q)` comparisons, `delta` markers passed beside the rule). Insight 4: when a type under-distinguishes, every consumer re-derives the missing case from context — each site slightly differently.

## The fix

Scoped per `audit/CONTRACT.md §C1/§C3`:

- **Refused half (do NOT attempt):** renaming/re-encoding `AtomSource` at the boundary (`Edb | Derived(DerivedId)`) — the boundary IR, its serde spelling, and the C ABI kinds stay shape-unchanged (§C1). `interior()` returning `Some` for the rec id IS the boundary numbering (§C2).
- **Accepted half:** past normalize, the bind kind is data on the occurrence, decided once: the normalized `Occurrence` (or the prepared plan's occurrence record) carries the derived-bind role — `Finished` vs `RecDelta` vs `RecAcc` for derived occurrences (the delta marking comes from `RecArm.delta`; every other self-read is `RecAcc`; non-self derived reads are `Finished`). `run_join` and `fill_plan_images` (engine-010's `DerivedBind`) dispatch on that role, never on `edb().is_none()` + id-comparison. Selectivity's floor choice reads the same role (engine-018).

## Acceptance criteria

- [ ] Gone: `rg -n 'edb\(\)\.is_none\(\)' crates/bumbledb/src` → no matches; `rg -n 'source\.edb\(\) else' crates/bumbledb/src/api crates/bumbledb/src/plan` → no matches used as the derived-bind / planning-floor dispatch (the `Some(relation)` arm that actually reads an EDB may stay); `rg -n '== Some\(rec_id\)|interior\(\) == Some' crates/bumbledb/src/api crates/bumbledb/src/plan` → no matches.
- [ ] Boundary untouched: `ir.rs::AtomSource` shape and serde unchanged.
- [ ] Unchanged tests: full engine + bench suites green unchanged.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Semantics identical (same images bound at the same occurrences every round). Lands after engine-010/engine-018 shape the role data.
