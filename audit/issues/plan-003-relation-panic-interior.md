# plan-003: `relation()` panics on Interior — two accessors, type-level hole

- **Severity:** medium
- **Tree:** plan
- **Status:** OPEN
- **Source:** audit/plan-exec.md F5
- **Depends on:** plan-001 (folded marks carrying `RelationId` lets `into_stats` stop calling this); engine-017 (the `edb() else` forest this accessor serves)

## The bug

Two panicking EDB-only accessors, same sentence:

```rust
// plan/fj.rs:218-237 — PlanOccurrence
pub fn relation(&self) -> RelationId {
    match self.source {
        AtomSource::Edb(relation) => relation,
        AtomSource::Interior(_) => {
            unreachable!("caller asserted a stored-relation (Edb) occurrence")
        }
    }
}

// ir/normalize.rs:148-166 — Occurrence (same panic, consumed by plan/exec)
```

Callers that still go through the panic:

- `exec/introspection/into_stats.rs:76,96-114` — `PlanOccurrence::relation()` for eliminated and folded occurrences.
- `plan/selectivity.rs:190,227` — `Occurrence::relation()` inside `occurrence_estimate`, after `occurrence_stats`' `edb() else { floor }` (`selectivity.rs:139-149`).
- `exec/dispatch/classify.rs:129` — `Occurrence::relation()` after `occurrence.source.edb()?` (`classify.rs:94`).
- `api/prepared/run_join.rs:125` — `PlanOccurrence::relation()` after `source.edb().is_none() { continue }` (`run_join.rs:96-124`). (Prepared-layer caller; this issue still owns the accessor it calls.)

Interior **cannot** reach these sites post-validate on the current guards (grounding refuses Interior folds at `evaluate.rs:136-141,208-211`; classify/selectivity/run_join return or continue first). The panic is "safe" only because a different module's guard exists. The sealed types still treat Interior as a programmer error.

## Why it's wrong

A panicking helper is the type-code not yet replaced (Insight 7): Interior is still the crash case of "everything is a stored relation." Panic-as-proof is a defect; making the state unrepresentable is the fix. The proof that folded/eliminated occs are EDB never made it into the mark (plan-001); every caller re-asserts. Complementary to engine-017 (`edb().is_none()` as bind dispatch) — that issue owns the Option test; this issue owns the panicking accessors.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layers match the source sum; `AtomSource` *shape* stays C1):

- Delete **both** `PlanOccurrence::relation()` and `Occurrence::relation()`. Callers match `occurrence.source`.
- Discharged marks carry the `RelationId` they folded/eliminated against (parse at ground time, plan-001) so `into_stats` does not re-assert. Live (Positive/Negated) occurrences keep `AtomSource` and match it.
- Do **not** rename/re-encode boundary `AtomSource` (engine-017's refused half).
- Do **not** make `relation()` return a dummy / `Option<RelationId>` / accept Interior as a relation id. Do **not** replace `unreachable!` with a silent fallback.

## Acceptance criteria

- [ ] Gone: `rg -n 'fn relation\(' crates/bumbledb/src/plan/fj.rs crates/bumbledb/src/ir/normalize.rs` → no matches; `rg -n 'caller asserted a stored-relation' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green; introspection still names the stored relation on eliminated/folded lines.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Boundary `AtomSource` untouched (C1). Lands with or after plan-001 if marks gain `RelationId`; otherwise `into_stats` matches `Edb(id)` and `debug_assert`s the Interior arm is unreachable for those roles — still better than a panicking accessor on every occurrence. `run_join.rs` match is mechanical (it already branched on `edb().is_none()`); do not restyle that forest — engine-017 owns the bind-role parse.
