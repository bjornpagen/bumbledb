# lean-003: rec identity is a numbering coincidence recomputed at every site

- **Severity:** high
- **Tree:** lean
- **Status:** OPEN (scoped — the Fin-telescope half is refused per CONTRACT §C5)
- **Source:** audit/lean.md H3
- **Depends on:** lean-001, lean-002 (the sum and structural self ARE most of this fix)
- **Conflicts with:** lean-004 (shares the `WellFormed` deletions)

## The bug

`lean/Bumbledb/Query/Syntax.lean:318-321` — a dead dual coordinate, defined and then fenced off by comments:

```lean
def Query.recId (q : Query) : Option InteriorId :=
  match q.rec with
  | none => none
  | some _ => some ⟨q.interiors.length⟩
```

with the guard comments at `Syntax.lean:477-478` ("`self` is `⟨q.interiors.length⟩` — do not match `recId` beside it") and `Reach.lean:747` ("Match `q.rec` only. Do not match `recId`.").

`lean/Bumbledb/Exec/Reach.lean:196-198` — the operator takes rec and its identity as *independent* arguments:

```lean
def reachOp (C : Classify) (rec : Rec) (self : InteriorId)
    (I : Instance) (W : InteriorEnv) (ρ : ParamEnv)
```

`lean/Bumbledb/Exec/Reach.lean:755-756` and `777-778` — every evaluator rebuilds the identity from the length convention:

```lean
        let self : InteriorId := ⟨q.interiors.length⟩
```

`lean/Bumbledb/Exec/Reach.lean:185-190` — the WellFormed screen does not refine the id; it is just `hwf.1`:

```lean
theorem wellFormed_interior_reads_real {q : Query} (hwf : q.WellFormed)
    ... : C.id < q.derivedCount :=
  hwf.1 r hr a ha C hsrc
```

## Why it's wrong

Rec is not a constructor; it is "whatever id equals `interiors.length` today". `reachOp C rec ⟨999⟩` is well-typed and denotes a different operator than the intended one — the identity and the thing identified can disagree (Insight 11: the off-by-one lives in the numbering convention). `recId` is a second coordinate for the same fact, dead on arrival, with comments posted as guards where the type should have made the dual path unwritable (Insight 4).

## The fix

Scoped per `audit/CONTRACT.md §C2/§C5`:

- With lean-001's sum, the `reach` constructor is the ONE site that knows the rec's id: `evalQuery`'s reach case computes `⟨interiors.length⟩` once and passes it where the environment update needs it. No other site recomputes it.
- With lean-002's structural self, `reachOp`/`reachDen` no longer *select* the self atom by id-comparison — the floating `self : InteriorId` parameter survives only as "the env slot the finished rec publishes into" (used by `InteriorEnv.update self …` and main's reads), passed from the one computation site.
- DELETE: `Query.recId`, both "do not match recId" guard comments, `Query.derivedCount` as a flag (per-constructor arithmetic where needed), `wellFormed_interior_reads_real` (dies with lean-004's bundle).
- **Refused half (do NOT attempt):** `AtomSource` as `edb | interior (Fin n) | recSelf` with telescope-indexed interiors — refused per CONTRACT §C5 R-DENSE: `InteriorId` stays a dense `Nat`, the spec models the open boundary IR the frozen corpus feeds, and denotations stay total (Insights 15/16 — the dependent-index bookkeeping across the proof tree costs more than the branches it deletes).

## Acceptance criteria

- [ ] Gone: `rg -nw 'recId' lean --glob '!conformance/cases/**'` → no matches; `rg -n 'do not match recId|Do not match .?recId' lean` → no matches; `rg -c 'InteriorId := ⟨.*interiors.length⟩|⟨q.interiors.length⟩' lean/Bumbledb` → at most 1 site (the `evalQuery`/`evalQueryList` reach case pair; 2 sites if Level 0/Level 1 each compute it once).
- [ ] Not attempted: `rg -nw 'recSelf|Fin n' lean/Bumbledb/Query/Syntax.lean` → no matches (the refused half stays refused).
- [ ] Unchanged behavior: 268-case conformance green; `reach_den_finite` survives.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Semantics identical; corpus frozen; phantom-read semantics preserved (dense ids stay).
- Lands after (or with) lean-001+lean-002; most of its edits are theirs — this issue owns verifying the recomputation sites are gone and the dead dual path deleted.
