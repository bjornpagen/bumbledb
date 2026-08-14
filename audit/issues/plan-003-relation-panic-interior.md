# plan-003: `PlanOccurrence::relation()` panics on Interior

- **Severity:** medium
- **Tree:** plan
- **Status:** OPEN
- **Source:** audit/plan-exec.md F5
- **Depends on:** plan-001 (folded/eliminated marks carrying `RelationId` lets `into_stats` stop calling this); engine-017 (the `edb() else` forest this accessor serves)

## The bug

`crates/bumbledb/src/plan/fj.rs:218-237`:

```rust
pub fn relation(&self) -> RelationId {
    match self.source {
        AtomSource::Edb(relation) => relation,
        AtomSource::Interior(_) => {
            unreachable!("caller asserted a stored-relation (Edb) occurrence")
        }
    }
}
```

`exec/introspection/into_stats.rs:76,96-114` calls `occurrence.relation()` for eliminated and folded occurrences. Those happen to be stored — grounding refuses Interior (`evaluate.rs:136-141,208-211`) — so the panic is "safe" only because a different module's guard exists. The sealed plan type still treats Interior as a programmer error.

## Why it's wrong

A panicking helper is the type-code not yet replaced (Insight 7): Interior is still the crash case of "everything is a stored relation." The proof that folded/eliminated occs are EDB never made it into the mark (plan-001); every caller re-asserts. Complementary to engine-017 (`edb().is_none()` as bind dispatch) — that issue owns the Option test; this issue owns the panicking accessor on the plan witness.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layers match the source sum; `AtomSource` *shape* stays C1):

- Delete `PlanOccurrence::relation()`. Callers match `occurrence.source`.
- Folded/eliminated marks carry the `RelationId` they folded/eliminated against (parse at ground time, plan-001) so `into_stats` does not re-assert.
- Do **not** rename/re-encode boundary `AtomSource` (engine-017's refused half).

## Acceptance criteria

- [ ] Gone: `rg -n 'fn relation\(' crates/bumbledb/src/plan/fj.rs` → no matches; `rg -n 'caller asserted a stored-relation' crates/bumbledb/src/plan` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green; introspection still names the stored relation on eliminated/folded lines.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Boundary `AtomSource` untouched (C1). Lands with or after plan-001 if marks gain `RelationId`; otherwise `into_stats` matches `Edb(id)` and `debug_assert`s the Interior arm is unreachable for those roles — still better than a panicking accessor on every occurrence.
