# lean-023: `HeadSlot` is a fourth encoding of the head-shape row

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean-rest.md M2
- **Depends on:** lean-008 (`CFind` is the thin head wrapper around `Query`; unify onto it)
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

Per `audit/CONTRACT.md §C4` (one decoder / one query type; head-shape is a wrapper around `Query`, not a parallel IR) and §C6's find-shape sum (Count is its own kind):

- Keep `AggOp` as the op inventory. After lean-008, `CFind` is the one head type the union law reads.
- `HeadSlot` becomes a *function* of that type (`HeadSlot.of : CFind → …` or a view of `AggOp` + `KeyTerm`), not a constructor family. Delete the independent inductive.
- `KeyTerm` may remain as the grouping-key view (`CFind.var`/`measure`) if it earns its keep as a function; it must not be a second source of truth.

## Acceptance criteria

- [ ] Gone: `rg -n 'inductive HeadSlot' lean` → no matches; union-key theorems (`headRow`, `keysOf`, `union_regime_agg_heads`) take `List CFind` or `List AggOp` (plus key positions), not a parallel inductive.
- [ ] Unchanged: `union_regime_agg_heads`, `agg_over_distinct_bindings`, `empty_global_no_answer` survive with the same content (Count contributes no union-key words; empty global aggregate is the empty answer set). 268-case conformance green.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green. No `sorry`/`admit`.

## Constraints

- Semantics identical: R1 (`CountAcrossRulesAccepted`) and R2 (`dnf_rekey_transparent`) unchanged. Count remains nullary (no dummy `over`).
- No C5 split.
- No Program vocabulary. Lands after lean-008 so the one head type exists; may fold `KeyTerm` in the same commit.
