# engine-019: the naive oracle's rec is an `if let Some` afterthought on a CQ evaluator

- **Severity:** medium
- **Tree:** engine (bench oracle)
- **Status:** FIXED(5bdc1f10)
- **Source:** audit/engine.md F19
- **Depends on:** none (bench-local; parallel-safe)

## The bug

`crates/bumbledb-bench/src/naive/query.rs` — `InteriorWorld` (177-183) is documented as "finished interior and rec tables" (one name for two kinds), and `query` (254-288) evaluates the rec as a flag branch with the same id pun as the engine:

```rust
if let Some(rec) = &query.rec {
    interval.push(self.seal_intervals(&rec.head, &rec.base, &interval));
    sets.push(BTreeSet::new());
    let rec_id = sets.len() - 1;      // the pun (engine-003), oracle edition
    loop {
        ...
        let mut next = self.rows_for(&rec.head, &rec.base, params, &preds)?;
        next.extend(self.rows_for(&rec.head, &rec.rec, params, &preds)?);
        if next == sets[rec_id] { break; }
        sets[rec_id] = next;
    }
}
```

## Why it's wrong

The oracle is allowed to be dumb (full naive T(I) re-evaluation is *essential* — it is the definitional contrast to the engine's semi-naive); it is not allowed to lie about the tables (Insight 1). `InteriorWorld` holding rec rows under the interior name, and rec-as-Option-branch, reproduce in the oracle exactly the coordinate the engine findings condemn — so the differential harness cannot catch a class of engine bugs it shares representation with.

## The fix

Keep the boundary consumption (`query.rec`, `interiors.len()` numbering — the oracle reads the untrusted `Query`, §C1) but structure the evaluation as the two-arm story:

- Rename `InteriorWorld` → `DerivedWorld` (doc: "finished derived tables — interiors in declaration order, then the rec's accumulating table"); `Src::Interior` → `Src::Derived` if the rename is cheap (bench-internal).
- `query` becomes: evaluate interiors fold; then match on shape ONCE (`match &query.rec { None => .., Some(rec) => lfp }`) with the lfp as its own function `fn rec_lfp(...)` — one place computes the rec's set index, passed in, not re-derived. The naive full-T(I) loop stays byte-identical in behavior.
- No engine types change; this is oracle-local hygiene.

## Acceptance criteria

- [ ] Gone: `rg -n 'InteriorWorld' crates/bumbledb-bench/src` → no matches; `rg -n 'sets\.len\(\) - 1' crates/bumbledb-bench/src/naive` → no matches (index passed, not re-derived).
- [ ] Unchanged: ALL differential tests (`tree_closure_matches_the_hand_answer_on_every_oracle`, `cyclic_closure_...`, the seeded corpus runs) green with zero edits — the oracle's ANSWERS are pinned by the whole differential suite.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`. The `Bridge.lean` row citing `NaiveDb::query (crates/bumbledb-bench/src/naive/query.rs)` still resolves.

## Constraints

- Naive full-lfp evaluation strategy locked (it is the point of the oracle — audit "Not counted" list). Boundary `Query` consumption unchanged.
