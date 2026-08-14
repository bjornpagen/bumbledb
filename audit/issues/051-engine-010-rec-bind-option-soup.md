# engine-010: `fill_plan_images` takes three independent `Option`s for one rec bind; dead params linger

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED(6a9f47d2)
- **Source:** audit/engine.md F10
- **Depends on:** engine-013 (unified derived-images layout), engine-001

## The bug

`crates/bumbledb/src/api/prepared/reach.rs:514-520`:

```rust
fn fill_plan_images(
    plan: &crate::plan::fj::ValidatedPlan,
    derived: &mut DerivedScratch,
    rec_id: Option<usize>,
    rec_delta: Option<&Arc<RelationImage>>,
    rec_acc: Option<&Arc<RelationImage>>,
)
```

admits `rec_id` without images, images without `rec_id`, delta without acc; the body is nested `is_some()` reconciliation (531-543) plus a second full walk that overwrites the self occurrence when both halves are Some (546-560), ending in a literal `let _ = rec_acc;`. Two parameters are pure corpses:

```rust
// reach.rs:499,510 — fill_finished_images
variant_delta: Option<OccId>, ... let _ = variant_delta;
// reach.rs:572,582 — run_into_projection
rec_delta: Option<OccId>, ... let _ = rec_delta;
```

And `run_join.rs:31-32,96-99` consumes the result as `idb_images: &[Option<Arc<RelationImage>>]` with `.expect("the reach driver supplies every Interior occurrence's image")` — None on a live Interior occurrence is representable, then panicked.

## Why it's wrong

One fact — "how does a derived occurrence bind this round" — is smeared across three independent `Option`s whose 8 combinations include 5 nonsense states the body must reconcile (Insight 4). The `let _ =` parameters are the k-variant bind's ghost limbs (Insight 2), and `idb_*` is deleted-IDB vocabulary steering live parameters (Insight 1).

## The fix

Per `audit/CONTRACT.md §C3` ("Binds/scratch"):

```rust
enum DerivedBind<'a> {
    Finished(&'a DerivedImages),
    Rec { id: usize, delta: &'a Arc<RelationImage>, acc: &'a Arc<RelationImage> },
}
fn fill_plan_images(plan: &ValidatedPlan, derived: ..., bind: DerivedBind<'_>)
```

- Interiors/base/main call with `Finished`; rec arms with `Rec { .. }` — delta-without-acc is unrepresentable. The one walk fills each derived occurrence: self-and-delta-occ → delta, self-otherwise → acc, other → finished. (The current double-walk with a comment explaining the overwrite deletes.)
- DELETE the `variant_delta` and `rec_delta` parameters nobody reads.
- `run_join`'s slice: rename `idb_images`/`idb_retired` → `derived_images`/`derived_retired`; layout per engine-032 (no `Option` slot for a live derived occurrence + no `expect`).

## Acceptance criteria

- [ ] Gone: `rg -n 'let _ = variant_delta|let _ = rec_delta|let _ = rec_acc' crates/bumbledb/src` → no matches; `rg -n 'rec_id: Option|rec_delta: Option|rec_acc: Option' crates/bumbledb/src/api/prepared/reach.rs` → no matches; `rg -nw 'idb_images|idb_retired' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: all reach differential/conformance tests green unchanged (bind semantics identical: delta at the marked occ, acc at other self reads, finished elsewhere).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh` (Bridge tokens citing `run_join`/reach mechanisms unchanged or moved together).

## Constraints

- Semantics identical (semi-naive bind discipline byte-identical). Lands after engine-013's layout (or as one change with it).
