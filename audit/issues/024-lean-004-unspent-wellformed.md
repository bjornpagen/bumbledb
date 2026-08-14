# lean-004: `WellFormed` is bundled validation nobody spends; `interiorsDag` is dead

- **Severity:** high
- **Tree:** lean
- **Status:** OPEN (scoped — the telescope/`Vector` half is refused per CONTRACT §C5)
- **Source:** audit/lean.md H4
- **Depends on:** lean-001 (`plain_wellFormed` mentions `Query.plain`; land after the sum)
- **Conflicts with:** lean-003, lean-014 (shared deletions)

## The bug

`lean/Bumbledb/Query/Syntax.lean:492-493` — three independent Props glued with `∧`:

```lean
def Query.WellFormed (q : Query) : Prop :=
  q.sourcesInRange ∧ q.interiorsDag ∧ q.recLinear
```

`lean/Bumbledb/Query/Syntax.lean:473-475` — the DAG invariant, validated and never spent:

```lean
def Query.interiorsDag (q : Query) : Prop :=
  ∀ (i : Nat) (d : Interior), q.interiors[i]? = some d → ∀ r, r ∈ d.rules →
    ∀ (C : InteriorId), C ∈ r.interiorReads → C.id < i
```

`lean/Bumbledb/Exec/Reach.lean:974-976` — the soundness theorem's own doc admits the bundle is unspent:

```lean
/-- Interior DAG once, optional `reachDen`, then main `rulesAnswers` —
listed by `evalQueryList`. Premises: `Safe` / `WellTyped` / `recLinear`,
not full `WellFormed`. -/
```

Grep evidence: `interiorsDag` appears only at its definition and inside the `WellFormed` conjunction — no denotation or agreement theorem takes it as a hypothesis. `plain_wellFormed` (`Syntax.lean:498-510`) exists solely to inhabit the bundle.

## Why it's wrong

King's validate-then-discard, exactly (Insight 6): the bundle is proved (or assumed) and then every consumer picks out one conjunct (`recLinear`) and drops the rest. The DAG invariant — this cut's actual topology — is thrown away; `evalInteriorsAt` "works" on a cyclic list only because later interiors do not exist yet and a back-edge silently reads empty. A validated Prop nobody spends is worse than no Prop: it teaches readers the invariant is load-bearing when the code ignores it (Insight 15: the cost landed, the benefit never did).

## The fix

Per `audit/CONTRACT.md §C4`:

- DELETE `Query.WellFormed`, `Query.interiorsDag`, `Query.sourcesInRange`, `Query.plain_wellFormed`, `Rule.edbOnly` (`Syntax.lean:327-332`), `wellFormed_interior_reads_real` (`Reach.lean:185-190`), and the unspent corollary `derives_edb` (`Denotation.lean:806-834` — the only remaining *code* consumer of `edbOnly`; deleting the Prop without this theorem is a compile error). Nothing else spends them; the acceptance story stays `Safe`/`WellTyped` (+ the structural `LinearRec` after lean-002) as named premises of the agreement theorems — those are essential and already spent.
- **C8 — do not drop the Bridge row.** `@Query.wellFormed_interior_reads_real` (`Bridge.lean:553-556`) is a term-level `@` reference (`Obligation.row`'s first argument is the theorem itself); deleting the theorem without retargeting fails `lake build`. You cannot point a row at a comment. Retarget `@` to a named lemma of the recorded phantom semantics, e.g. `sourceDen I InteriorEnv.empty (.interior C)` is empty (definitional; name it if nothing is currently theorem-shaped). Keep the mechanism/instrument tokens (`UnknownInterior`, `rejects_a_negated_phantom_read`). The engine refusal stays the screen; Lean keeps phantom-empty (C5). Do not revive `sourcesInRange` under a new name.
- The ordering guarantee lives where it is real: lean-007's structural declaration-order fold (`evalInteriorTables.go` is already that fold at Level 1). Out-of-range/forward reads stay empty by construction — the recorded phantom semantics, unchanged.
- **Refused half (do NOT attempt):** interiors as a snoc-telescope/`Vector` with `Fin i`-bounded reads — refused per CONTRACT §C5 R-DENSE.
- Update the `Syntax.lean` module doc (the "unknown-interior gap" paragraph keeps its recording of the phantom semantics but stops citing `Query.WellFormed`/`sourcesInRange` as "the screen"; the engine's `UnknownInterior` refusal is the screen, at the boundary).

## Acceptance criteria

- [ ] Gone: `rg -nw 'WellFormed|interiorsDag|sourcesInRange|plain_wellFormed|edbOnly|wellFormed_interior_reads_real|derives_edb' lean --glob '!conformance/cases/**'` → no matches (Bridge row for the old theorem is retargeted, so the `@Query.wellFormed_interior_reads_real` token is gone from Bridge.lean too).
- [ ] Unchanged behavior: 268-case conformance green; `evalQuery_sound` (restated by lean-001) keeps exactly `Safe`/`WellTyped` premises per rule list — no new premise smuggled in, none dropped.
- [ ] New locks: none — the deletion is the fix; the module doc paragraph updated (grep `rg -n 'sourcesInRange' lean/Bumbledb/Query/Syntax.lean` → empty, `rg -n 'UnknownInterior' lean/Bumbledb/Query/Syntax.lean` → still present).
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Semantics identical; phantom-read semantics preserved and still recorded in the module doc; corpus frozen.
- Do not delete `Safe`/`WellTyped` or weaken any agreement theorem's remaining premises.
- C8: Bridge `@` arguments are theorems, never comments. The UnknownInterior row stays a row.
