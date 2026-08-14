# engine-021: the SQL translator gates on two flags and names interiors-only "reach"

- **Severity:** medium
- **Tree:** engine (bench translator)
- **Status:** OPEN
- **Source:** audit/engine.md F21
- **Depends on:** none (bench-local; parallel-safe)

## The bug

`crates/bumbledb-bench/src/translate/query.rs:37-39`:

```rust
if !query.interiors.is_empty() || query.rec.is_some() {
    return super::reach::translate_query(query, schema, sets);
}
```

Two flags, one WITH path — interiors-only goes through a module named `reach`. Inside `translate/reach.rs`, the Option is consulted a third time to pick the keyword (line 53: `let recursive = query.rec.is_some();`), and `sqlite_reach_expressible` (line 14) is named for rec while screening interiors too.

## Why it's wrong

One translator input (a Query) is dispatched by a two-flag product to modules whose names describe only one of the four states (Insight 1: `reach.rs` translating an interiors-only query teaches the reader a falsehood). The Option is re-consulted at each stage instead of parsed once into "WITH" vs "WITH RECURSIVE" — the same rec-as-flag coordinate as engine-003, in the oracle that exists to check the engine (Insight 2).

## The fix

- ONE translator over Query: `translate` handles all shapes; CTEs are derived tables in declaration order; the rec, when present, is the last CTE and flips the keyword. The gate (`interiors.is_empty() || rec.is_some()`) becomes the degenerate case of the one path: zero CTEs → no `WITH` clause (the code at `reach.rs:47-52` already does this — make it THE path, delete the front gate).
- Module rename: `translate/reach.rs` → `translate/derived.rs` (or fold into `query.rs`); `sqlite_reach_expressible` → a name that says what it screens (`sqlite_derived_expressible` or `refuse_interval_derived_columns` as the public face). Callers updated.
- `query.rec.is_some()` consulted ONCE, at the entry, into a local shape choice; the keyword choice reads that.

## Acceptance criteria

- [ ] One path: `rg -n 'interiors\.is_empty\(\) \|\| query\.rec\.is_some\(\)|!query\.interiors\.is_empty\(\) \|\| query\.rec\.is_some\(\)' crates/bumbledb-bench/src` → no matches; `rg -c 'rec\.is_some\(\)' crates/bumbledb-bench/src/translate` → ≤ 1.
- [ ] Honest names: `rg -n 'sqlite_reach_expressible' crates/bumbledb-bench/src` → no matches (renamed); no module named `reach` translating interiors-only queries.
- [ ] Unchanged: emitted SQL byte-identical on the whole differential corpus (the three-way comparator is the lock) — `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench` green with zero SQL-snapshot edits.
- [ ] Green: `./scripts/check.sh`; `./scripts/lean.sh` (three-way comparator).

## Constraints

- `WITH` vs `WITH RECURSIVE` semantics locked (SQLite requirement). SQL output must not change.
