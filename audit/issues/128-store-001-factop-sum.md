# store-001: `FactOp` is one product for insert and delete; memberships are dead weight on delete

- **Severity:** medium
- **Tree:** store
- **Status:** OPEN
- **Source:** audit/storage-schema.md F7
- **Depends on:** none
- **Conflicts with:** none

## The bug

`storage/commit/plan.rs:84-118` — one `FactOp` for both apply lists. The comment on `memberships` says the quiet part:

> Dead weight on a delete op (removing a reference cannot violate an inclusion); only the insert-side judgment consumes it.

`MarkEdgeOp.weight: Option<u64>` is `None` on delete "by construction — never derived." Insert-only fields sit on delete ops and are ignored.

## Why it's wrong

Insight 7 — tag-plus-all-payloads. Two roles, one struct, illegal fields present. Insight 3 — the delete special case belongs to the representation (a Delete op that cannot carry memberships), not a comment.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum FactOp<'d> {
    Delete { relation, fact, fact_hash, determinants, edges, capacity_keys },
    Insert { relation, fact, fact_hash, fresh_row: Option<FreshRowOp>,
             determinants, edges, memberships, capacity_edges },
}
```

Applier matches. Weight lives only on insert capacity edges (or `CapacityEdge::{Unit, Weighted(u64)}`).

Delete does **not** need `fresh_row`: `delete_fact` (`applier.rs:14-95`) takes the row id from the `M` entry, then deletes `F`/`U`/`R`. Memberships are insert-only (`judgment.rs:520-536` walks `plan.inserts`). Omitting both from Delete is correct. Do not drop `determinants`/`edges`/`capacity` key-bytes from Delete — those removals are live.

## Acceptance criteria

- [ ] Gone: the `memberships` "dead weight on a delete op" comment and the field on delete ops — `rg -n 'Dead weight on a delete' crates/bumbledb/src/storage/commit/plan.rs`.
- [ ] `plan.commits` deletes vs inserts are different types (or enum arms); delete construction cannot set memberships.
- [ ] Unchanged tests: `storage/commit/tests/{plan,apply,judgment}.rs` green, assertions untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Delete-then-insert apply order unchanged. Closed-target membership judgment still insert-only. Key-symmetric `R` bytes for containment/capacity edges unchanged.
