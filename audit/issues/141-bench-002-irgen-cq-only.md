# bench-002: irgen claims a structurally-free Query and never draws interiors or rec

- **Severity:** high
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F2
- **Depends on:** none (fuzz-local; parallel-safe)

## The bug

`crates/bumbledb-bench/src/corpus_gen/irgen.rs:37-71` — the hostile Query arm's contract:

```rust
/// A structurally-free query: every shape the IR type can spell is
/// reachable by some byte string — zero rules, misaligned heads, deep
/// condition spines. Valid and invalid programs both arise; the verdict
/// is the engine's.
pub fn random_query(rng: &mut Rng) -> Query {
    // …
    Query {
        interiors: vec![],
        rec: None,
        head,
        rules,
    }
}
```

`plausible` (line 126) is `Query::single`. Both paths pin `interiors` and `rec` empty. Empty interiors, empty rec, dangling `InteriorId`, empty `Rec.base`/`Rec.rec`, self-in-base, nonlinear self, negation-in-rec — the roster the engine exists to refuse on the new IR — are unemittable.

## Why it's wrong

Insight 1: a generator that cannot mint the illegal states of the type under test can only confirm the old CQ roster. "Every shape the IR type can spell" is a lie while two of four Query fields are constants. The engine's rec/interior refusals are then unfuzzed (Insight 6: the check never happens at this boundary because the input cannot carry the proof).

## The fix

Keep the engine as the sole judge (no validity logic in irgen). Extend the free draw so interiors and rec are reachable the same way empty rule lists and dangling relation ids are — as hostile data:

- Some draws emit `Interior` lists (including empty-rules interiors, dangling self-ids in atoms).
- Some draws emit `Rec` (including empty base/step, self-in-base, extra self-atoms, negated self).
- `Query::single` stays the coherent-core path (engine-037). The free arm is a Query, not a CQ with two frozen empty fields.
- Rename "programs" in the module comments (bench-009 owns the crate-wide sweep; this file's three sites move here if this lands first).

## Acceptance criteria

- [ ] A 512-seed sweep (the existing `the_arm_reaches_both_verdict_classes` pattern) produces at least one query with `!interiors.is_empty()` and at least one with `rec.is_some()`, and still both accept and reject.
- [ ] Gone: the two constructors that unconditionally write `interiors: vec![], rec: None` as the only free-draw shape (`Query::single` for the coherent core may remain).
- [ ] Unchanged: irgen still owns no roster; `db.prepare` is the judge. No corpus JSON regeneration.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench --lib corpus_gen::irgen`; `./scripts/check.sh`.

## Constraints

- Hostile by design — do not filter toward valid recs. Boundary `Query` shape unchanged (C1). Assertions never weakened.
