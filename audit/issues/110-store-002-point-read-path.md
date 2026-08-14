# store-002: point-read access path is `is_closed × fresh_row × U-tree`

- **Severity:** medium
- **Tree:** store
- **Status:** FIXED(c51881ab)
- **Source:** audit/storage-schema.md F8
- **Depends on:** schema-001, schema-002
- **Conflicts with:** none (call sites; lands after the sums)

## The bug

Three independent tests reconstruct one fact — which probe this key is — at every point-read and at the capacity parent probe:

```rust
// api/db/snapshot.rs:261-278
let bytes = if rel.is_closed() { closed_fact_by_determinant(...) }
    else if statement.fresh_row { read::fact_at(..., fresh_row_id(...))? }
    else { read::fact_for_key(...)? };

// api/db/get.rs:366-376 — same forest plus delta overlay
// storage/commit/judgment.rs:1306-1318 — same forest for the parent holder
```

Closed + fresh_row is representable (closed branch wins). Fresh-row + U-tree is representable (`fresh_row` skips U).

## Why it's wrong

Insight 4 — a bool product of kinds schema-001 and schema-002 already parsed. Every site re-derives the access path with a different nesting.

## The fix

Match schema-001's relation kind, then schema-002's `KeyForm`. One probe function used by snapshot get, write-tx get, and `check_capacity`. The bool product deletes.

## Acceptance criteria

- [ ] Gone: `rg -n 'if rel.is_closed\(\)' crates/bumbledb/src/api/db/snapshot.rs crates/bumbledb/src/api/db/get.rs` — kind is a match on the sealed relation, not a bool.
- [ ] Gone: `rg -n 'else if statement.fresh_row' crates/bumbledb/src/api/db`.
- [ ] One probe helper (or three matches on the same sum); capacity parent probe uses it.
- [ ] Unchanged tests: point-read matrix (typed/dyn × snapshot/write × closed/fresh/ordinary) green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands after schema-001 and schema-002. Overlay-then-committed discipline on the write path unchanged. Closed relations still have no delta arm.
