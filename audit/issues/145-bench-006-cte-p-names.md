# bench-006: derived CTEs are still Datalog `p{id}` — goldens lock the name

- **Severity:** medium
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F6
- **Depends on:** engine-021 (the one WITH path is renamed once; goldens move with it)

## The bug

`crates/bumbledb-bench/src/translate/builder.rs:121-126`:

```rust
bumbledb::AtomSource::Interior(id) => format!("p{}", id.0),
```

The three closure goldens pin the name (`translate/goldens.rs:147-160`): `WITH RECURSIVE p0(c0, c1)`, `FROM "p0"`, `¬p0(c0 = x)`. engine-033 is the engine's diagnostic `predicate p{id}`; this is the same coordinate on the SQL oracle the engine is checked against.

## Why it's wrong

Names are representation (Insight 1). The 3-way arbitration anchor is what a human reads when engine and SQLite disagree; teaching rec as predicate p0 keeps the deleted Datalog/IDB vocabulary alive in the one place the bench claims is infrastructure, not product (C7).

## The fix

Per `audit/CONTRACT.md` §C3 vocabulary: CTE name `interior{id}` for interiors, `rec` for the rec (one rec, no number). Positional columns `c{i}` stay (the translator's derived-table spelling). Goldens rewrite mechanically — SQL *answers* stay byte-identical; SQL *strings* do not.

## Acceptance criteria

- [ ] Gone: `rg -n 'format!\("p\{\}' crates/bumbledb-bench/src/translate` → no matches; `rg -n '"p0"' crates/bumbledb-bench/src/translate/goldens.rs` → no matches (replaced by `rec` / `interior0` as applicable).
- [ ] Unchanged: differential/conformance answer sets byte-identical; golden-vs-translator equality tests still pass (goldens updated in the same commit).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh` (three-way comparator).

## Constraints

- Answers locked; SQL identifiers are the change. Coordinate with engine-021 so the one WITH path is renamed once. Do not name CTEs `predicate`. Closure's hand-written `CLOSURE_SQL` (`reach(n)`) is SQLite idiom for that family, not this translator identifier — leave it unless a later pass unifies hand SQL with the translator.
