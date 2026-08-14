# lean-015: `odd_not_stratified` carries the dead stratum vocabulary

- **Severity:** low
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md L1
- **Depends on:** lean-002 (decides whether the theorem is renamed or deleted)

## The bug

`lean/Bumbledb/Countermodels.lean:1411-1417`:

```lean
/-- **No recLinear witness**: empty base, and the rec arm negates
self. The name keeps the wall; the statement is `¬ recLinear`. -/
theorem odd_not_stratified : ¬ oddQuery.recLinear := by
```

The statement is about `recLinear`; the name says "stratified" — the coordinate the cut deleted. The section prose (`Countermodels.lean:1373-1381`) names the theorem as one of the two wall fences.

## Why it's wrong

A name is a representation of intent (Insight 1); this one teaches readers that stratification is still the acceptance concept. The wall itself (`odd_not_monotone`, `odd_rounds_oscillate`, `odd_no_fixpoint`) is essential and operator-level; only the syntax-level witness carries the dead vocabulary.

## The fix

Two cases, decided by lean-002's landing:

- **After lean-002 (expected):** `oddRec`/`oddQuery` are unwritable in `LinearRec` syntax, so `odd_not_stratified` and its syntax inhabitants are DELETED. The operator-level walls stay verbatim (`oddOp`, `odd_not_monotone`, `odd_rounds_oscillate`, `odd_no_fixpoint`), and the section prose (`Countermodels.lean:1369-1393`) is rewritten to say the odd loop is now *unrepresentable* in rec syntax — the countermodel lives at the operator level, which is where its content always was. Update the `reachOp_mono` doc comment (`Exec/Reach.lean:224-227`) which cites `odd_not_stratified`'s neighborhood.
- **If somehow landing before lean-002:** rename to `odd_not_recLinear`; no other change.

Check `lean/Bumbledb/Bridge.lean` and `scripts/spec-census.sh` for tokens naming `odd_not_stratified` and move them with the change.

## Acceptance criteria

- [ ] Gone: `rg -nw 'odd_not_stratified|stratified' lean --glob '!conformance/cases/**'` → no matches (prose included).
- [ ] Walls intact: `rg -nw 'odd_not_monotone|odd_rounds_oscillate|odd_no_fixpoint|succOp_monotone|succ_chain_ascends|succ_prefixed_infinite' lean/Bumbledb/Countermodels.lean` → all present, statements unchanged.
- [ ] Commands green: `cd lean && lake build`; `./scripts/spec-census.sh`; no `sorry`/`admit`.

## Constraints

- The walls' mathematical content must not weaken — deletion is only legal for the *syntax* witness whose type died; every operator-level theorem survives verbatim.
