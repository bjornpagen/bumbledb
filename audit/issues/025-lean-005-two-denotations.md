# lean-005: the proof library still lives on the pre-cut `rulesAnswers ∘ edbEnv` denotation

- **Severity:** high
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md H5
- **Depends on:** none for the List-Rule restatements (Dedup/Rewrites); the Theorem 9 half depends on lean-001
- **Conflicts with:** lean-016 (DUPLICATE of this issue's RewriteStep restatement)

## The bug

The cut made `evalQuery` the denotation (`Exec/Reach.lean:745` — "`evalQuery` — the denotation of a Query"), but the flagship theorems still quantify over a `Query` and then read only `q.rules` under the OLD plain denotation:

`lean/Bumbledb/Query/Denotation.lean:1023-1026` — Theorem 9:

```lean
theorem snapshot_single {q : Query} {I J : Instance} (C : Classify)
    (ρ : ParamEnv) (h : ∀ R, R ∈ q.relations → I R = J R) :
    ∀ t, t ∈ rulesAnswers C q.rules (edbEnv I) ρ ↔
      t ∈ rulesAnswers C q.rules (edbEnv J) ρ := by
```

with `Query.relations` (`Denotation.lean:1010-1012`) reading `q.rules.flatMap` only — interiors and rec invisible, so the theorem's headline ("the denotation reads ONE instance") is proved for a denotation that is no longer THE denotation.

`lean/Bumbledb/Exec/Dedup.lean:329-333` — Theorem 1 takes a `{q : Query}` it uses only for `q.rules`:

```lean
theorem seenfold_is_set_semantics {C : Classify} {q : Query}
    {I : Instance} {ρ : ParamEnv} {l : List AnswerTuple}
    (henum : ∀ t, t ∈ l ↔ t ∈ rulesAnswers C q.rules (edbEnv I) ρ) :
```

Same shape: `union_regime_head_projection` (`Dedup.lean:611-622`), and the whole rewrite theory — `lean/Bumbledb/Exec/Rewrites.lean:2306-2312`:

```lean
inductive RewriteStep (T : Theory) (C : Classify) :
    Query → Query → Prop where
  | ground {n : Nat} {pre post : List Rule} {r r' : Rule}
      (h : groundRewrite T r = .inl r') :
      RewriteStep T C (Query.plain n (pre ++ r :: post)) (Query.plain n (pre ++ r' :: post))
```

— every constructor wraps `Query.plain n (…)`, threading a phantom arity `n`, and `step_preserves` (`Rewrites.lean:2456-2459`) concludes over `rulesAnswers q.rules (edbEnv I)`.

## Why it's wrong

Two denotations inhabit the library: `evalQuery` (the real one) and `rulesAnswers ∘ edbEnv` (the pre-cut one), connected only by the `evalQuery_plain` shim. Theorems stated over a `Query` that inspect only `q.rules` are theorems about a *rule list* wearing a query costume — the quantifier advertises generality the statement does not have (Insight 3: the representation should say what the theorem is about). The `Query.plain n` wrapping in `RewriteStep` forces an irrelevant arity through 2000 lines of rewrite theory.

## The fix

Per `audit/CONTRACT.md §C4` ("One rule-list theory"):

- Restate the Dedup wrappers that currently take `{q : Query}` and then only read `q.rules` over `List Rule` + an environment:
  - theorems: `seenfold_is_set_semantics` (`Dedup.lean:329`), `union_regime_head_projection` (`:611`), `disjoint_witness_licence` (`:578`), `syntactic_disjointness_sound` (`:725` — Bridge row `@Query.syntactic_disjointness_sound` at `Bridge.lean:437` must keep resolving).
  - defs: `DisjointArms` (`Dedup.lean:515-518`, `q.rules.Pairwise`) and `ProvablyDisjointRules` (`:677-679`). `disjoint_flatten` (`:523`) is already over `List Rule` — that is the target shape. Callers pass `q.rules`.
- `RewriteStep : Theory → Classify → List Rule → List Rule → Prop`; constructors relate `pre ++ r :: post` to `pre ++ r' :: post` directly; the `{n : Nat}` dies (this IS lean-016). `step_preserves` (`Rewrites.lean:2456`) and `rewrites_compose` conclude over `rulesAnswers C rules (edbEnv I) ρ` with `rules : List Rule`.
- Theorem 9: restate over the real denotation (after lean-001) — `Query.relations` walks ALL rule lists (interiors, rec arms via `LinearRec`'s `toRule` lists, main), and `snapshot_single` concludes `∀ t, t ∈ evalQuery C q I ρ ↔ t ∈ evalQuery C q J ρ`. The proof lifts the per-rule-list congruence through the interior fold and `reachDen` (use `rulesAnswers_congr` / `sourceDen_congr` already in `Reach.lean`). Interior atoms that are not stored relations still contribute no `RelId` to `relations`; agreement on mentioned stored relations plus identical interior/rec tables (determined by those relations) is the content.
- Bridge rows keep theorem NAMES resolving (`snapshot_single`, `seenfold_is_set_semantics`, `disjoint_witness_licence`, `syntactic_disjointness_sound`, `step_preserves`). Mechanism columns unchanged.

## Acceptance criteria

- [ ] Gone: `rg -n 'RewriteStep.*Query → Query|Query\\.plain n' lean/Bumbledb/Exec/Rewrites.lean` → no matches; `rg -n '\\{q : Query\\}' lean/Bumbledb/Exec/Dedup.lean` → no matches; `DisjointArms` / `ProvablyDisjointRules` take `List Rule` (or equivalent), not `Query`.
- [ ] Real Theorem 9: `rg -n 'evalQuery' lean/Bumbledb/Query/Denotation.lean` (or the file `snapshot_single` moves to) shows `snapshot_single` concluding over `evalQuery`; `Query.relations` covers interior and rec rules (`rg -n 'def Query.relations' -A 6 lean` shows all three lists).
- [ ] Unchanged: theorem NAMES survive (`seenfold_is_set_semantics`, `union_regime_head_projection`, `snapshot_single`, `disjoint_witness_licence`, `syntactic_disjointness_sound`, `step_preserves`, `rewrites_compose`) — Bridge rows keep resolving; 268-case conformance green.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`; `./scripts/spec-census.sh` green (census tokens follow the restatements).

## Constraints

- Semantics identical — each restatement must be propositionally the same fact modulo the binder change; no assertion weakened.
- The Dedup/Rewrites half may land before lean-001 (it removes `Query` dependencies and makes the sum change smaller); the Theorem 9 half lands after lean-001.
