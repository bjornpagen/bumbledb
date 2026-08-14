# lean-009: `Query.allRules` flattens provenance, then three inversion lemmas rebuild it

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md M3
- **Depends on:** lean-001 (lands as part of the sum's `evalQuery_sound` restatement)

## The bug

`lean/Bumbledb/Query/Syntax.lean:309-314` — one list erasing which lane each rule came from:

```lean
def Query.allRules (q : Query) : List Rule :=
  (q.interiors.flatMap Interior.rules) ++
    (match q.rec with
     | none => []
     | some rec => rec.base ++ rec.rec) ++
    q.rules
```

Then `lean/Bumbledb/Exec/Reach.lean:946-963` re-derives the provenance the flatten erased:

```lean
theorem mem_allRules_interior {q : Query} {d : Interior}
    (hd : d ∈ q.interiors) {r : Rule} (hr : r ∈ d.rules) :
    r ∈ q.allRules := ...
theorem mem_allRules_rec ...
theorem mem_allRules_main ...
```

and `evalQuery_sound` (`Reach.lean:984-993, 1013-1016`) spends its first ten lines splitting the bundled premise back apart:

```lean
  have hinterS : ∀ d, d ∈ q.interiors → ∀ r, r ∈ d.rules → Safe r :=
    fun d hd r hr => hsafe r (mem_allRules_interior hd hr)
```

## Why it's wrong

Bundle-then-unbundle: the premise `∀ r ∈ q.allRules, Safe r` is *stated* over the flattened list, so the proof immediately reconstructs the three per-lane facts through injection lemmas that exist only to invert the flatten (Insight 6: information established (which lane) is erased and re-derived). The three `mem_allRules_*` lemmas are pure coordinate-change tax.

## The fix

Per `audit/CONTRACT.md §C4`: state premises per lane. `evalQuery_sound` (restated by constructor cases under lean-001) takes:

- `cq` case: `hInter : ∀ d ∈ interiors, ∀ r ∈ d.rules, Safe r ∧ r.WellTyped` and `hMain : ∀ r ∈ rules, Safe r ∧ r.WellTyped`.
- `reach` case: those two plus per-arm premises over `LinearRec`'s `base`/`step` (which after lean-002 carry no linearity obligations).

DELETE `Query.allRules` and `mem_allRules_interior`/`mem_allRules_rec`/`mem_allRules_main`. If a caller (conformance harness, battery) genuinely wants "every rule in the query is Safe" as one hypothesis, it states the conjunction of the per-lane facts — the lanes are the structure.

## Acceptance criteria

- [ ] Gone: `rg -nw 'allRules|mem_allRules_interior|mem_allRules_rec|mem_allRules_main' lean --glob '!conformance/cases/**'` → no matches.
- [ ] Unchanged: `evalQuery_sound` survives (restated) with per-lane `Safe`/`WellTyped` premises and the same conclusion; 268-case conformance green.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Semantics identical; no premise strengthened or weakened in aggregate — the per-lane split must be exactly the old bundle's content.
- Lands with lean-001 (same theorem body).
