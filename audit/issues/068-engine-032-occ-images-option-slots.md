# engine-032: `occ_images`/`finished` are `Vec<Option<Arc<_>>>` — absence representable for live occurrences, then `expect`ed

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(6a9f47d2)
- **Source:** audit/engine.md F32
- **Depends on:** engine-013 (the unified layout owns these fields), engine-010 (the consumer)

## The bug

`reach.rs:53-58`: `occ_images: Vec<Option<Arc<RelationImage>>>` sized to ALL occurrences (`fill_plan_images` line 522: `resize(plan.occurrences().len(), None)`), `None` for EDB and discharged occurrences; `finished: Vec<Option<Arc<RelationImage>>>` indexed by derived id, `None` until `stash_finished`. The consumer then asserts what the type failed to say — `run_join.rs:97-99`:

```rust
let image = idb_images[occ_idx]
    .as_ref()
    .expect("the reach driver supplies every Interior occurrence's image");
```

## Why it's wrong

Hoare's null in every slot (Insight 4): "not every occurrence is derived" is essential, but spelling it as `Option` in a dense array makes "derived occurrence without an image" representable — and it is then panicked at the single hottest read of the join loop. The `finished` holes are the sealing phase leaking into data (same as engine-006's validator-side holes).

## The fix

Per `audit/CONTRACT.md §C3` ("Binds/scratch") — pick the layout `run_join`'s access pattern favors (it indexes by `occ_idx` inside the occurrence walk):

- EDB/discharged occurrences don't get an `Option` hole to skip: either a dense map keyed by the derived occurrences only (e.g. pairs `(occ_idx, Arc)` or a per-plan precomputed `derived_occ_ids: Vec<OccId>` aligned with a `Vec<Arc>`), or — if dense-by-occ-idx indexing measurably wins — keep the dense array PRIVATE behind an accessor that types the invariant (`fn derived_image(&self, occ: OccId) -> &Arc<…>`) with the fill and the read in one module so no `expect` string narrates a cross-module trust. Prefer the typed layout; benchmark before keeping the hole.
- `finished` becomes the seal-ordered `Vec<Arc<RelationImage>>` from engine-013 (interior 0..n, then rec) — filled in order, never `None`.

## Acceptance criteria

- [ ] Gone: `rg -n 'expect\("the reach driver supplies' crates/bumbledb/src` → no matches; `rg -n 'Vec<Option<Arc<RelationImage>>>' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: all suites green unchanged; allocation gates in `./scripts/check.sh` pass (no new per-round allocation).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`.

## Constraints

- Hot-loop performance is semantics here: if the typed layout regresses the differential benches, document the measurement in the issue thread and take the accessor-encapsulation arm instead. Lands with engine-013.
