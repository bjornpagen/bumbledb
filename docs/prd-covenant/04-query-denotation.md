# PRD 04 — Query denotation: the matching equation and the answer set

**Depends on:** 03.
**Modules:** `lean/Bumbledb/Query/Syntax.lean`, `Query/Denotation.lean`,
`Countermodels.lean` additions.
**Authority:** `20-query-ir.md`'s semantic content (builds its
replacement — the matching-equation block, negation, union, answer
identity all move HERE); the artifact's query-model section (port
targets); the language law (answers, PointIn, conditions).
**Representation move:** Level 0 for queries. The normative denotation
the executor, the naive model, AND the conformance lane (PRD 13) are
all measured against.

## Context (decided shape)

Syntax.lean — a faithful abstraction of the IR (not the notation):
- `Term := var VarId | param ParamId | paramSet ParamId | lit Value |
  measure VarId` — mirroring `ir.rs` post-constitution.
- `Atom` (relation + field→term bindings), `Condition` (the
  ConditionTree sum: leaf comparison / and / or), `Rule` (atoms,
  negated, conditions, finds), `Query` (head + rules).
- Comparisons: `Eq Ne Lt Le Gt Ge`, `PointIn`, `Allen mask` — the
  typed legality rules as a `WellTyped` predicate (the validator's
  spec).

Denotation.lean — definitions:
- `Assignment`, `ParamEnv`; `matches : Fact → Atom → Assignment →
  ParamEnv → Prop` — THE matching equation (port
  `MatchTerms`/`AtomMatches`: unbound var binds; bound var demands
  equality; param/lit select; paramSet selects membership).
- `Safe` — positive range restriction: every negated-atom, order-
  comparison, and head variable is bound by a positive atom (port
  `PositivelyRangeRestricted`; the spec of `NegatedVariableUnbound`).
- `ruleAnswers : Rule → Instance → ParamEnv → Set AnswerTuple` — body
  environments filtered by conditions and anti-joins, projected
  through finds. `queryAnswers` = union over rules.
- `lower : Condition → List (List Comparison)` — the DNF lowering.

Theorems:
1. `repeated_var_unifies` (port) — same-fact equality within an atom;
   cross-atom repeats denote joins (state both).
2. `param_selects_not_binds` (port) + the membership-only-variable
   refusal as a Safety consequence.
3. `antijoin_over_active_domain` — negation denotes ¬∃ over the finite
   relation extension, NEVER the infinite complement; requires `Safe`
   (the theorem that makes the safety rule load-bearing: state a
   countermodel where an unsafe rule's "denotation" would be infinite
   — `Countermodels.lean`).
4. `safety_order_independent` — `Safe` is invariant under any
   permutation of a rule's items (the order-independence lock's spec).
5. `dnf_preserves_denotation` — `body (A ∧ (B ∨ C)) = body (A∧B) ∪
   body (A∧C)` generalized: `lower` preserves `ruleAnswers` (the
   normalize pass's contract — Bridge row: ir/normalize's DNF).
6. `union_idempotent` (port `ruleUnion_set_idempotent`) — duplicate
   rules, duplicate derivations, one answer.
7. `answer_identity_canonical` — two environments producing the same
   projected head tuple are one answer (the seen-set's spec).
8. `pointIn_unfold` (port) and `allen_mask_denotation` (port shape):
   `Allen iv mask jv ↔ classify iv jv ∈ mask` with `classify` abstract
   here (PRD 05 refines it).
9. `snapshot_single` stated at this level as: denotation is a function
   of ONE Instance (no mixed-instance evaluation exists in the model —
   a structural note, made checkable by the signature itself).

## Technical direction

Port the artifact's raw-query model, then RESHAPE to the current IR
(the artifact predates ConditionTree/answers). `Set`-valued
denotations; decidable instances added only where PRD 13 needs them
(matching over concrete Tiny instances — provide `decide`-friendly
`List`-backed evaluation functions alongside the `Set` specs, with
`eval_sound : x ∈ evalList ↔ x ∈ queryAnswers` as the internal
refinement theorem — this is PRD 13's foundation and belongs here).

## Passing criteria

- `[shape]` All nine theorems + the unsafe-rule countermodel present
  and checked; the executable `evalList` with its soundness theorem;
  zero sorry/axioms; `scripts/lean.sh` 0.
- `[shape]` `Safe` is a hypothesis of `antijoin_over_active_domain`,
  not ambient (grep the signature).
- `[shape]` Names obey the language law (answers, PointIn, conditions
  — grep for `Row`/`Contains`-as-membership → zero).
- `[gate]` CI green.

## Doc amendments

None yet — PRD 11 deletes against these names.
