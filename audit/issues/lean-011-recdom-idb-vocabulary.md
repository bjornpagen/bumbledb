# lean-011: `recDom` speaks `idb` and re-litigates self vs. finished interiors by id comparison

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md M5
- **Depends on:** lean-002 (`recDom` is restated over `LinearRec`)

## The bug

`lean/Bumbledb/Exec/Reach.lean:359-361` — the doc comment reaches for dead Program vocabulary:

```lean
/-- Active domain of a rec: filler plus stored columns and finished
interior columns. Ignores the accumulating self (same as ignoring
`idb` on the old program domain). -/
```

and because self is just another `InteriorId`, the candidate-space proof must case on the id to find out whether an interior read is "the accumulating self" or "a finished table" — `Reach.lean:559-568` inside `evalRule_in_cands`:

```lean
    unfold InteriorTables.toEnv InteriorTables.update at hrow
    by_cases hQ : Q = self
    · subst hQ
      rw [if_pos rfl] at hrow
      ...
    · rw [if_neg hQ] at hrow
```

The domain definition itself (`recDom`, 362-369) walks `(rec.base ++ rec.rec)` atoms and treats an `.interior C` source uniformly, relying on the ignores-self reading being a *consequence* of update-shadowing rather than of the type.

## Why it's wrong

"Which table is the accumulator" is a structural fact of the rec (there is exactly one self), but the representation makes it an id-equality question re-asked inside proofs (Insight 11: identity by numbering convention forces comparisons where a constructor case would do). The `idb` comment is Program vocabulary explaining the current code by reference to the deleted system — a sign the representation does not explain itself (Insight 1).

## The fix

Per `audit/CONTRACT.md §C2/§C4`: after lean-002, a step arm's self-occurrence is `RecStep.selfBindings` — not an `.interior self` atom in `atoms`. Restate `recDom`/`recCands` over `LinearRec`:

- `recDom` walks base arms' atoms + step arms' *non-self* atoms (plus, for column reads through `selfBindings`, the candidate tuples' own values — which is exactly the "ignores the accumulating self" fact, now by construction: self contributes no NEW domain values because its rows are already candidates).
- `evalRule_in_cands`'s `by_cases hQ : Q = self` disappears for step arms: the self read is its own structural case (via `RecStep`), and finished-interior reads are the only `.interior` sources left in `atoms`.
- Doc comment rewritten in present-tense vocabulary: "Ignores the accumulating self: its rows are candidates already." No `idb`, no "old program domain".

## Acceptance criteria

- [ ] Gone: `rg -in 'idb|program domain' lean/Bumbledb/Exec/Reach.lean` → no matches; `rg -n 'by_cases hQ : Q = self' lean/Bumbledb/Exec/Reach.lean` → no matches.
- [ ] Unchanged: `reach_den_finite` and `evalLinearReach_eq_lfp` survive (restated over `LinearRec`) with the same content; 268-case conformance green (22 reach cases exercise the candidate space).
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Semantics identical — the candidate space may only grow-or-equal on accepted queries (finiteness must still hold; the theorem is the check).
- Lands after lean-002.
