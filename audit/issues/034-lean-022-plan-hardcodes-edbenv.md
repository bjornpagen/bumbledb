# lean-022: Plan denotes `edbEnv` — the per-rule theorem cannot be spent on interiors

- **Severity:** medium
- **Tree:** lean
- **Status:** FIXED(835f1b5e)
- **Source:** audit/lean-rest.md M1
- **Depends on:** lean-005 (same restatement: theorems about a rule take `Rule` + an environment, not a hardcoded EDB instance)
- **Conflicts with:** none. Plan staying *over `Rule`* is recorded essential (`Exec/Plan.lean:100-102`) and is not this issue.

## The bug

`lean/Bumbledb/Exec/Plan.lean:298-301` — consistency reads the EDB-only environment:

```lean
def Consistent (r : Rule) (I : Instance) (ρ : ParamEnv) (pre : Plan)
    (s : Subatom) (σ : Assignment) : Prop :=
  ∃ a, r.atoms[s.occ]? = some a ∧ ∃ f, f ∈ edbEnv I a.source ∧
    MatchesOn f a σ ρ (checkedVars pre s.occ ++ s.vars)
```

`runPlan` / `nodeStep` / `planBindings` / `planAnswers` take `I : Instance` only. The flagship theorem then locks that in:

```498:501:lean/Bumbledb/Exec/Plan.lean
theorem valid_plan_sound {C : Classify} {r : Rule} {P : Plan}
    {I : Instance} {ρ : ParamEnv} (hv : PlanValid r P)
    (hwt : r.WellTyped) :
    ∀ t, t ∈ planAnswers C r P I ρ ↔ t ∈ ruleAnswers C r (edbEnv I) ρ := by
```

`ruleAnswers` is already parameterized by `F : AtomSource → Set Fact`. Plan hardcodes the EDB-only instantiation. A valid plan of an interior-atom rule is "sound" against unread interiors (empty), which is not `evalQuery`'s meaning of that rule.

This is not lean-005. lean-005 is theorems that wrap a `Query` and then ignore interiors/rec. Plan already takes `Rule`; it is missing the environment parameter C4 requires of every rule-list theorem.

## Why it's wrong

Insight 2: two denotations of one rule (Plan's `edbEnv` vs Reach's `sourceDen`) will drift — Reach cannot spend `valid_plan_sound` against interior tables without a second plan evaluator. Insight 16: the essential fact is "a validated plan computes the rule's denotation"; baking the pre-cut Program/EDB coordinate into the definition is accidental.

`Subatom.occ : Nat` plus `r.atoms[s.occ]?` stays (C5: identities dense, theorems carry named premises — `PlanValid.occScoped` is spent). Do not Fin-index occurrences.

## The fix

Per `audit/CONTRACT.md §C4` ("theorems about a rule list take `List Rule` + an environment") and §C5 (R-DENSE: dual coordinates die, Fin refused):

- `Consistent` / `nodeStep` / `runPlan` / `planBindings` / `planAnswers` take `F : AtomSource → Set Fact` (or `I` plus `InteriorEnv`, feeding `sourceDen`).
- `valid_plan_sound`: `planAnswers C r P F ρ = ruleAnswers C r F ρ` under `PlanValid` / `WellTyped`.
- Reach spends it at `sourceDen I V`. The EDB case is `F := edbEnv I`, not the definition.
- Plan stays over `Rule`. No Query wrapper. No `Plan` nodes that mention interiors (the recorded narrowing stands).

## Acceptance criteria

- [x] Gone: `rg -n 'edbEnv I a.source' lean/Bumbledb/Exec/Plan.lean` → no matches in `Consistent`; `valid_plan_sound` quantifies over `F` (or `InteriorEnv`), not solely `edbEnv`.
- [x] Unchanged: `valid_plan_sound`, `every_rule_plannable`, `PlanValid.paper`, `Countermodels.loose_cover_rebinds` survive with the same content (plan answers = rule answers; every rule has a valid plan; paper cover is looser). 268-case conformance green (Plan is not a corpus decoder).
- [x] Commands green: `cd lean && lake build`; `./scripts/lean.sh` fully green. No `sorry`/`admit`.

## Constraints

- Semantics identical. C5: `Subatom.occ` stays `Nat`; `PlanValid` stays named premises. No Fin-telescope.
- No C5 split on this finding.
- No Program vocabulary. Coordinate with lean-021 (`keyProbeEval` takes the same `F`).
- Do not grow interior-aware plan nodes (recorded: Reach instantiates the per-rule theorem).
