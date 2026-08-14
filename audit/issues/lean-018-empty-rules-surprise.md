# lean-018: `evalQuery_empty_rules` is a product-field surprise ("the rec is never the answer")

- **Severity:** low
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md L4
- **Depends on:** lean-001

## The bug

`lean/Bumbledb/Exec/Reach.lean:1065-1069`:

```lean
theorem evalQuery_empty_rules {C : Classify} {q : Query} {I : Instance}
    {ρ : ParamEnv} (hr : q.rules = []) :
    ∀ t, t ∉ evalQuery C q I ρ := by
```

with the Bridge row (`Bridge.lean:583-586`) captioned "Empty main denotes the empty set; the rec is never the answer."

## Why it's wrong

The theorem is true and its language content is essential (the query result is main's projection; a derived table is never the result). What is accidental is that it reads as a *surprise* needing a Bridge caveat: in the product, `rules` and `rec` are peer fields, so "empty main with a live rec still denotes ∅" looks like a behavior choice rather than the only possible reading (Insight 3 — the representation should make the intended reading the obvious one).

## The fix

Per `audit/CONTRACT.md §C4`: restate over lean-001's sum — the hypothesis `q.rules = []` becomes per-constructor (`Query.rules q = []` via the total accessor, or two lemmas by case), and the doc comment/Bridge caption change from warning-tone to structural-tone: main's `rulesAnswers` over an empty list is the empty union; the rec arm's finished table is an *environment entry*, never the conclusion. Keep the theorem name `evalQuery_empty_rules` (Bridge row and engine `EmptyRuleSet` mapping key on it).

This is a restatement-and-prose issue only; no deletion.

## Acceptance criteria

- [ ] `evalQuery_empty_rules` survives under that name, restated over the sum, same conclusion.
- [ ] Bridge row resolves; mechanism column still cites `EmptyRuleSet (crates/bumbledb/src/error.rs)`.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/spec-census.sh`.

## Constraints

- Semantics identical; the engine's `EmptyRuleSet` boundary refusal is untouched (this theorem covers the model side where empty main is representable).
- Lands after lean-001 (trivially rides along).
