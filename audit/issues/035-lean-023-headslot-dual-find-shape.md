# lean-023: `HeadSlot` is a fourth encoding of the head-shape row

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean-rest.md M2
- **Depends on:** none (unify onto Aggregates; coordinate with lean-008 so Conformance maps into that type rather than the reverse)
- **Conflicts with:** none. Not DUPLICATE(lean-008): `CFind`/`CQuery` live in Conformance; `HeadSlot` is Dedup-local. Aggregates' `AggOp` is the recorded op inventory and stays.

## The bug

One head-shape row, four inductives.

`lean/Bumbledb/Query/Aggregates.lean:2049-2056` — the recorded op inventory (good: Count carries no `over`):

```lean
inductive AggOp where
  | count
  | sum (v : VarId)
  | min (v : VarId)
  | max (v : VarId)
  | pack (v : VarId)
  | measureFold (op : ScalarFold) (v : VarId)
```

`lean/Bumbledb/Query/Aggregates.lean:1427-1430` — grouping keys:

```lean
inductive KeyTerm where
  | var (v : VarId)
  | measure (v : VarId)
```

`lean/Bumbledb/Conformance.lean:166-169` — lean-008's wrapper (`CFind = var | measure | agg AggOp`).

`lean/Bumbledb/Exec/Dedup.lean:1428-1440` — a fourth, independent encoding of the union-key *quotient*:

```lean
inductive HeadSlot where
  | key (k : KeyTerm)
  | fold (v : VarId)
  | foldMeasure (v : VarId)
  | count
```

`HeadSlot.fold v` is writable with no corresponding `Sum`/`Min`/`Max`/`Pack`. `keysOf` / `headRow` / `union_regime_agg_heads` inhabit this type, not `AggOp`/`CFind`. Aggregates itself does **not** host `CQuery` — that leftover is lean-008.

## Why it's wrong

Insight 4: two coordinates for one head admit disagreement (union-key mask vs op inventory). Insight 1: the union key omitting Count's constant column is a *reading* of the head; storing it as a parallel datatype patches the trace instead of changing the representation. lean-008 already requires one head wrapper; leaving `HeadSlot` independent re-splits what that fix unifies.

## The fix

Per `audit/CONTRACT.md §C4` / §C6 (Count is its own kind):

- Keep `AggOp` as the op inventory in `Query/Aggregates.lean`. Keep `KeyTerm` as the grouping-key view there (`var` / `measure`).
- The union-key *quotient* is a **function** of that inventory, not a fourth inductive. Delete `inductive HeadSlot`. Example:

  ```lean
  def headSlot : (KeyTerm ⊕ AggOp) → …  -- or a small Find sum that lives in Aggregates
  ```

  Count → no key words; `sum`/`min`/`max`/`pack` → fold the `VarId`; `measureFold` → foldMeasure; `KeyTerm` → key. `keysOf` / `headRow` / `union_regime_agg_heads` take `List` of the Aggregates head type (or `List AggOp` plus key positions), applying the quotient as a function.
- **Do not make `Exec/Dedup.lean` import `Conformance.lean`.** `CFind` is the decoder spelling of the same row (lean-008's wrapper). Conformance's `headRow` (`Conformance.lean:831`) should call the Aggregates/Dedup function after this lands — one executable reading, not a parallel definition. lean-008 may keep `CFind` as a thin decode layer that maps into the Aggregates sum.

`HeadSlot.fold v` without a corresponding `AggOp` is exactly the illegal state this deletes; the union-key *reading* (Count contributes no words) stays as the function's Count case.

## Acceptance criteria

- [ ] Gone: `rg -n 'inductive HeadSlot' lean` → no matches; union-key theorems (`headRow`, `keysOf`, `union_regime_agg_heads`) take the Aggregates head type (or `List AggOp` plus keys), not a parallel inductive. `Exec/Dedup.lean` does not import `Conformance.lean`.
- [ ] Unchanged: `union_regime_agg_heads`, `agg_over_distinct_bindings`, `empty_global_no_answer` survive with the same content (Count contributes no union-key words; empty global aggregate is the empty answer set). 268-case conformance green.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green. No `sorry`/`admit`.

## Constraints

- Semantics identical: R1 (`CountAcrossRulesAccepted`) and R2 (`dnf_rekey_transparent`) unchanged. Count remains nullary (no dummy `over`).
- No C5 split.
- No Program vocabulary. May land with or after lean-008; Dedup unification does **not** wait on `CFind` existing as the spec type. Conformance maps into Aggregates, not the reverse.
