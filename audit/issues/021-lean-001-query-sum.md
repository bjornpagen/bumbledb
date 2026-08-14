# lean-001: `Query` is a Program product with an `Option Rec` hole — make it a two-arm sum

- **Severity:** high
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md H1
- **Depends on:** lean-002 (must land as ONE change with it — the `reach` constructor carries `LinearRec`)
- **Conflicts with:** lean-003, lean-004, lean-006, lean-007, lean-009 (same files; land after or fold in per INDEX order)

## The bug

`lean/Bumbledb/Query/Syntax.lean:289-297` — the "sum" is a product with an `Option` hole and a floating arity:

```lean
def Query : Type := List Interior × Option Rec × Nat × List Rule

def Query.interiors (q : Query) : List Interior := q.1
def Query.rec (q : Query) : Option Rec := q.2.1
def Query.arity (q : Query) : Nat := q.2.2.1
def Query.rules (q : Query) : List Rule := q.2.2.2
```

`lean/Bumbledb/Query/Syntax.lean:299-305` — "plain" is a *predicate* reconstructing what a constructor would carry:

```lean
def Query.plain (arity : Nat) (rules : List Rule) : Query :=
  Query.mk [] none arity rules

/-- A query with empty interiors and no rec. -/
def Query.Plain (q : Query) : Prop :=
  q.interiors = [] ∧ q.rec = none
```

`lean/Bumbledb/Query/Syntax.lean:482-484` — validation with a vacuous none-arm:

```lean
def Query.recLinear (q : Query) : Prop :=
  match q.rec with
  | none => True
```

`lean/Bumbledb/Exec/Reach.lean:747-757` and `770-779` — every evaluator independently re-matches `q.rec` and rebuilds the rec's identity:

```lean
/-- Match `q.rec` only. Do not match `recId` beside it. -/
def evalQuery (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    Set AnswerTuple :=
  let V := evalInteriors C q I ρ
  let V' :=
    match q.rec with
    | none => V
    | some rec =>
        let self : InteriorId := ⟨q.interiors.length⟩
        V.update self (reachDen C rec self I V ρ)
  rulesAnswers C q.rules (sourceDen I V') ρ
```

## Why it's wrong

The language has exactly two shapes — rec-absent and rec-present — but the type is a product of four independent fields, so absence-of-rec is a *value* (`none`) inside every query rather than a constructor. Every consumer branches on it (Insight 5: null-in-the-type forces a check at every use), `Query.Plain` is a Prop reconstructing what a constructor would make structural (Insight 4: illegal/uninformative states get guarded everywhere instead of being unrepresentable), and `recLinear`'s `none => True` arm is vacuous validation. The comment "Do not match `recId`" is a posted guard against the dual coordinate the product created.

## The fix

Per `audit/CONTRACT.md §C4`. In `lean/Bumbledb/Query/Syntax.lean`:

```lean
inductive Query where
  | cq    (interiors : List Interior) (arity : Nat) (rules : List Rule)
  | reach (interiors : List Interior) (r : LinearRec) (arity : Nat) (rules : List Rule)
```

Contract delta from the audit's sketch: TWO arms, not three — interiors are ordinary, possibly-empty data in both arms (Dijkstra's empty prefix; engine F37 records `Query::single` as the same ruling). A constructor cannot be named `rec` (recursor collision — the recorded language constraint); `LinearRec` comes from lean-002.

- Total accessors `Query.interiors` / `Query.arity` / `Query.rules` by match are fine. There is NO `Query.rec` accessor. Keep `arity` on both constructors in this commit (C4's sketch); lean-006 deletes it if still unread.
- `evalQuery` / `evalQueryList` (`lean/Bumbledb/Exec/Reach.lean`) become one function by constructor cases; the `reach` case is the ONE site that computes the rec's id (`⟨interiors.length⟩`) and passes it down.
- DELETE: `Query.Plain`, `Query.plain` (a test-local abbreviation for `.cq []` is acceptable only if it earns its keep), `Query.recId` (`Syntax.lean:318-321`, dead by its own comments), `Query.derivedCount` as an `isSome` flag (`Syntax.lean:324-325` — per-constructor lengths if still needed), `evalQuery_plain` and `evalQueryList_plain` (`Reach.lean:1050-1063` — plain is a constructor case, not a shim equation), the `match q.rec` in both evaluators.
- Update the JSON decoder (`lean/Main.lean:400-408 decodeReachQuery`, `393-398 decodeRecOpt`) to build `.cq` / `.reach` from the SAME frozen JSON (`rec` absent-or-null → `.cq`); corpus unchanged. Coordinate with lean-008.
- Restate `evalQuery_sound` (`Reach.lean:977-1048`) by constructor cases; its `recLinear` premise dies with lean-002. Per-lane `Safe`/`WellTyped` premises are lean-009.
- Restate `evalQuery_empty_rules` over the total `Query.rules` accessor (same conclusion: empty main denotes ∅). Rewrite the Bridge caption (`Bridge.lean:583-586`) from warning-tone ("the rec is never the answer") to structural-tone: main's `rulesAnswers` over `[]` is the empty union; the reach arm's finished table is an environment entry, never the conclusion. Keep the theorem name (engine `EmptyRuleSet` maps to it).
- Rewrite the Syntax module doc that still teaches the product embedding (`Syntax.lean:94-96` "today's query plus two empty fields (`Query.plain` / `evalQuery_plain`)") — C7 forbids that embedding. Present tense: empty-prefix `.cq` is a CQ; `.reach` carries `LinearRec`.
- **Bridge rows that name deleted theorems must RETARGET, never vanish (C8).** `@Query.evalQuery_plain` (`Bridge.lean:548-551`) currently cites the shim. Replace the Lean half with a constructor-case lemma (`.cq` with empty interiors equals `rulesAnswers` over `edbEnv`) or unfold `evalQuery` at that case under the same name-or-successor; keep the obligation row and its mechanism/instrument tokens. `@Query.evalQuery_sound` / `@Query.evalQuery_empty_rules` stay, restated.

## Acceptance criteria

- [ ] Unrepresentable/gone: `rg -nw 'Query\.Plain|Query\.plain|recId|evalQuery_plain|evalQueryList_plain' lean --glob '!conformance/cases/**'` → no matches; `rg -n 'match q\.rec|q\.rec\.isSome|Option Rec' lean` → no matches.
- [ ] Unchanged behavior: `lean/conformance/cases/**` byte-identical (`git status` clean there); `evalQuery_sound`, `evalQuery_empty_rules`, `reach_den_finite` survive restated with the same mathematical content (agreement of the listed evaluator with the set denotation; empty main denotes ∅).
- [ ] Bridge honest: `rg -n '@Query.evalQuery_plain' lean/Bumbledb/Bridge.lean` → no matches (row retargeted, not deleted); `rg -n 'evalQuery_empty_rules' lean/Bumbledb/Bridge.lean` still a row; every remaining `@` reference elaborates.
- [ ] Module doc: `rg -n 'two empty fields|Query\\.plain' lean/Bumbledb/Query/Syntax.lean` → no matches.
- [ ] New locks: none required beyond the surviving theorems; the sum itself is the lock.
- [ ] Commands green: `cd lean && lake build` (zero errors); `lake exe conformance conformance/cases` → 268 cases, 0 disagreements; no `sorry`/`admit` tokens anywhere under lean/; no `axiom` declarations.

## Constraints

- Semantics identical: `reachDen = lfpS` unchanged; the recorded phantom-read semantics (out-of-range interior reads empty) preserved; the 268 JSON cases frozen.
- Lean language: no field/constructor named `rec`; use `cq`/`reach` and (in `LinearRec`) `base`/`step` per CONTRACT §C4.
- No Program vocabulary; no new caps. `Bridge.lean` rows move honestly with every rename (never deleted to dodge a build failure).
- Must land as one change with lean-002; lean-009's `allRules` deletion and lean-012 (duplicate of this issue) ride along. lean-018 is DUPLICATE of this restatement of `evalQuery_empty_rules`.
- Do not invent a third `Query` constructor for CQ (C1 / engine-037): interiors are a possibly-empty prefix on both arms.
