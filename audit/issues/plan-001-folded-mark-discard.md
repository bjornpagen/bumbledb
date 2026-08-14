# plan-001: `FoldedMark` discards the parsed σ; polarity is a bool; introspection re-parses

- **Severity:** medium
- **Tree:** plan
- **Status:** OPEN
- **Source:** audit/plan-exec.md F3
- **Depends on:** none (mark shape; `into_stats` consumer rides this)

## The bug

`crates/bumbledb/src/plan/ground/evaluate.rs` parses resolvable filters, computes the surviving id-set `S`, then stores:

```rust
// ir/normalize.rs:84-92
pub struct FoldedMark {
    pub ids: u16,
    pub negated: bool,  // "the ! polarity the role no longer carries"
}
```

The module doc (`evaluate.rs:60-64`) admits the King move: "The fold mark remains `Copy`, so it cannot carry the parsed filter set. introspection reparses the retained original filters on its cold path; a failed reparse maps to an empty handle list after a debug assertion."

`exec/introspection/into_stats.rs:97-101`:

```rust
let parsed = crate::plan::ground::evaluate::parse_resolvable(&occurrence.filters);
debug_assert!(parsed.is_some(), "folded occurrences parsed at fold time");
let handles = parsed
    .map(|filters| surviving_ids(relation, &filters))
    .unwrap_or_default()
```

Release: silent empty picture. The same `S` also lives as `WordSet` filters attached to sibling occurrences. Three encodings of one σ. Polarity is a bool on Folded (Minsky product: Folded+negated / Folded+not).

## Why it's wrong

Validation (here: evaluation) discards proof; every diagnostic caller re-checks (Insight 6). `Copy` is not a reason to throw away a ≤256-row id list. A bool for polarity makes the two fold directions a product instead of a sum (Insight 4). The silent `unwrap_or_default` is the silent-omission class the COLT token tags were built to close.

## The fix

Per `audit/CONTRACT.md` §C1 (every trusted layer is a sum):

```rust
enum FoldedMark {
    Positive { ids: u16 },
    Negated  { ids: u16 },
}
```

Store the surviving id vec on the mark (n ≤ 256, diagnostic-sized) so `into_stats` does not re-run `parse_resolvable`/`surviving_ids`. Sibling `WordSet` attachment remains the execution rewrite. `negated: bool` dies. `into_stats` reads the mark; no `unwrap_or_default`.

## Acceptance criteria

- [ ] Gone: `rg -n 'negated: bool' crates/bumbledb/src/ir/normalize.rs` → no matches; `rg -n 'unwrap_or_default' crates/bumbledb/src/exec/introspection/into_stats.rs` → no matches; `rg -n 'parse_resolvable\(&occurrence.filters\)' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green; folded-occurrence introspection pictures byte-identical (handles still named, polarity still printed as `!` where the Negated arm is).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Semantics identical: fold still attaches `WordSet` to binders; rule-death channel unchanged; n ≤ 256 cap unchanged. `FoldedMark` lives in `ir/normalize.rs` (wave-1 IR) but this issue owns the shape because `plan/ground/evaluate.rs` is the one writer. Coordinate textually with engine-011 if comment vocabulary in `into_stats` moves.
