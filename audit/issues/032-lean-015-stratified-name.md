# lean-015: `odd_not_stratified` carries the dead stratum vocabulary

- **Severity:** low
- **Tree:** lean
- **Status:** FIXED(c8af2915)
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

- **After lean-002 (expected):** `oddRec` / `oddQuery` / `odd_not_stratified` are unwritable and DELETED. `oddOp` CANNOT stay `Query.reachOp C oddRec oddSelf` — `Rec` is gone. Restate `oddOp` as a raw set operator with the same math: a rule `{ finds := [], atoms := [], negated := [oddAtom], conditions := [] }` evaluated against `sourceDen I (empty.update oddSelf X)` (empty base, negated self, no bindings). Then restated `odd_step_of_empty` / `odd_step_of_nonempty` / `odd_not_monotone` / `odd_rounds_oscillate` / `odd_no_fixpoint` keep the same *conclusions* about that operator (empty derives; nonempty underives; not monotone; oscillates; no fixpoint). Section prose (`Countermodels.lean:1369-1393`) says the odd loop is unrepresentable in rec *syntax* — the countermodel lives at the operator level. Update the `reachOp_mono` doc (`Exec/Reach.lean:224-227`).
- **If somehow landing before lean-002:** rename to `odd_not_recLinear`; no other change.

`odd_not_stratified` is not a Bridge `@` token today (do not invent a row). Successor-chain theorems (`succOp_monotone`, `succ_chain_ascends`, `succ_prefixed_infinite`) are untouched here (they use `naiveIter` — lean-010).

## Acceptance criteria

- [x] Gone: `rg -nw 'odd_not_stratified|stratified' lean --glob '!conformance/cases/**'` → no matches (prose included).
- [x] Walls intact: `rg -nw 'odd_not_monotone|odd_rounds_oscillate|odd_no_fixpoint|succOp_monotone|succ_chain_ascends|succ_prefixed_infinite' lean/Bumbledb/Countermodels.lean` → all present; odd-* conclusions unchanged (not monotone / oscillates / no fixpoint); `oddOp` is not defined via `Rec`.
- [x] Commands green: `cd lean && lake build`; `./scripts/spec-census.sh`; no `sorry`/`admit`.

## Constraints

- The walls' mathematical content must not weaken — deletion is only legal for the *syntax* witness whose type died. `oddOp` is restated, not copied as `reachOp C oddRec`. Do not keep a `LinearRec` with a negated field to preserve `oddRec`.
