# lean-002: `Rec` is an untyped SCC record — replace with structural `LinearRec`

- **Severity:** high
- **Tree:** lean
- **Status:** FIXED(c8af2915)
- **Source:** audit/lean.md H2
- **Depends on:** none (foundation; lands as ONE change with lean-001)
- **Conflicts with:** lean-010, lean-011, lean-015, lean-017 (same theorems; land after)

## The bug

`lean/Bumbledb/Query/Syntax.lean:271-279` — a linear rec is two untyped rule lists and a `Nat`:

```lean
def Rec : Type := Nat × List Rule × List Rule

def Rec.arity (r : Rec) : Nat := r.1
def Rec.base (r : Rec) : List Rule := r.2.1
def Rec.rec (r : Rec) : List Rule := r.2.2
```

`lean/Bumbledb/Query/Syntax.lean:458-464` — linearity is counted, not carried:

```lean
def Rule.selfCount (r : Rule) (self : InteriorId) : Nat :=
  (r.atoms.filter fun a => decide (a.source = .interior self)).length

def Rule.hasNegatedSelf (r : Rule) (self : InteriorId) : Prop :=
  ∃ a, a ∈ r.negated ∧ a.source = .interior self
```

`lean/Bumbledb/Query/Syntax.lean:485-490` — shotgun validation; note `¬ r.hasNegatedSelf self` is strictly implied by `r.negated = []` two lines later:

```lean
      rec.base ≠ [] ∧ rec.rec ≠ [] ∧
      (∀ r, r ∈ rec.base → r.selfCount self = 0 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ ¬ r.hasNegatedSelf self) ∧
      (∀ r, r ∈ rec.base ++ rec.rec → r.negated = [])
```

`lean/Bumbledb/Exec/Reach.lean:228-231` — every lemma re-buys the proof as a premise:

```lean
theorem reachOp_mono {C : Classify} {rec : Rec} {self : InteriorId}
    {I : Instance} {W : InteriorEnv} {ρ : ParamEnv}
    (hlin : ∀ r, r ∈ rec.rec → r.selfCount self = 1 ∧ r.negated = []) :
```

`lean/Bumbledb/Exec/Reach.lean:965-972` — `recLinear_arms` re-parses the Prop and throws away nonempty-base, base-not-self, and `hasNegatedSelf`. `lean/Bumbledb/Countermodels.lean:1406-1409` — the illegal state is writable:

```lean
def oddRec : Query.Rec := Query.Rec.mk 0 [] [oddRecRule]
def oddQuery : Query.Query := Query.Query.mk [] (some oddRec) 0 []
```

## Why it's wrong

Empty base, empty step, a step arm with zero self-atoms (`p ← ¬p` — `oddRec`), nonlinear arms, and negation-in-rec are all representable, so `recLinear` validates over the product and every downstream lemma either re-checks (`hlin` premises) or re-parses the Prop and discards conjuncts (`recLinear_arms`) — King's validate-then-forget, Insight 6. `hasNegatedSelf` is a leftover of "no self-negation" (stratified) that `NegationInRec` made redundant — a guard on a state a tighter type deletes (Insight 4).

## The fix

Per `audit/CONTRACT.md §C4`, in `lean/Bumbledb/Query/Syntax.lean`:

```lean
structure RecRule where            -- base arm: negation unrepresentable, no self field
  finds : List VarId
  atoms : List Atom
  conditions : List Condition

structure RecStep where            -- step arm: THE unique positive self-atom, structural
  finds : List VarId
  selfBindings : List (FieldId × Term)
  atoms : List Atom                -- non-self atoms after the decoder split
  conditions : List Condition

structure LinearRec where          -- nonempty by type; field named `rec` is unavailable (recursor)
  base : RecRule × List RecRule    -- or an equivalent structural nonempty encoding
  step : RecStep × List RecStep
```

C5 constraint (do not "fix" this with `recSelf` / `NonSelfAtom` / Fin): `AtomSource` stays `edb | interior InteriorId`. A hand-built `RecStep.atoms` can still mention `.interior self`. Linearity for *accepted* queries is the decoder parse plus `selfBindings` being the one self occurrence the evaluators use.

**Lowering (required, or `reachOp_empty` / `evalRule` break):**

```lean
def RecRule.toRule (r : RecRule) : Rule :=
  { finds := r.finds, atoms := r.atoms, negated := [], conditions := r.conditions }

def RecStep.toRule (self : InteriorId) (r : RecStep) : Rule :=
  { finds := r.finds
    atoms := { source := .interior self, bindings := r.selfBindings } :: r.atoms
    negated := []
    conditions := r.conditions }
```

`reachOp` / `reachStep` / `evalLinearReach` feed `toRule` into the existing `rulesAnswers` / `evalList` machinery. Do NOT drop `selfBindings` from the atom list — `reachOp_empty` is "positive self against ∅ derives nothing", which needs that atom to exist. Empty `selfBindings` is still a legal self atom (nonemptiness gate).

**Decoder (`lean/Main.lean:385-391 decodeRec`, JSON keys `base`/`rec` unchanged):** parse the frozen object into `LinearRec`. Let `selfId := interiors.length`. For each JSON base rule: refuse nonempty `negated` (`NegationInRec`), refuse any positive atom with `interior` id = `selfId` (`SelfInBase`), refuse empty `base`. For each JSON step rule: refuse nonempty `negated`; partition positive atoms into the unique `interior` atom whose id = `selfId` (its bindings become `selfBindings`) vs the rest (`RecStep.atoms`); missing self (`RecArmMissingSelf`) or extra self (`NonlinearRecArm`) is a decode error. None of the 22 checked-in reach cases fail this. Coordinate with lean-008.

- Semantics for accepted queries identical: base evaluates against EDB + finished interiors; step evaluates with the reconstructed self atom bound to the accumulating set (denotation) / delta (Level 1).
- `reachOp`/`reachDen`/`evalLinearReach`/`reachStep`/`recDom`/`recCands` (`Reach.lean:196-205, 359-375, 482-493`) take `LinearRec`; `reachOp_mono` loses `hlin` (monotonicity is `negated = []` from the type — today's `reachOp_mono` already spends only `hlin.2`, not `selfCount`); `reachOp_empty` loses `hpos` (self atom is `selfBindings`); `evalLinearReach_eq_lfp` and `reach_den_finite` (`Reach.lean:659-743`) lose `hlin`.
- DELETE: `Rule.selfCount`, `Rule.hasNegatedSelf`, `Query.recLinear`, `recLinear_arms`, `selfCount_eq_one_mem` (lean-017 is this deletion), `Rec` and its accessors, `oddRec`/`oddQuery` as *syntax* (Countermodels keeps the operator-level walls — see lean-015; `oddOp` CANNOT stay `reachOp C oddRec` once `Rec` dies).
- `Bridge.lean` `@Query.reachOp_mono` (`:558-561`) restates without a linearity premise; do not delete the row. NonlinearRecArm / NegationInRec stay the engine instruments.

## Acceptance criteria

- [x] Unrepresentable/gone: `rg -nw 'selfCount|hasNegatedSelf|recLinear|recLinear_arms|selfCount_eq_one_mem|oddRec' lean --glob '!conformance/cases/**'` → no matches; `rg -n 'def Rec ' lean/Bumbledb/Query/Syntax.lean` → no matches.
- [x] Unchanged behavior: `lake exe conformance conformance/cases` → 268 cases, 0 disagreements (the 22 reach cases decode into `LinearRec` and evaluate identically); `evalLinearReach_eq_lfp` and `reach_den_finite` survive with the same content minus the linearity premise.
- [x] New locks: `reachOp_mono` restated WITHOUT a linearity premise (monotonicity from the structural self) — its Bridge row updated; a decoder test path exercised by the corpus (the 22 reach cases are the lock).
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`; no `axiom`.

## Constraints

- Semantics identical for accepted queries; walls unchanged (`odd_not_monotone` stays as the operator-level wall — the wall is real, only its syntax inhabitant dies); OPEN refusals unchanged.
- Field name `rec` is unavailable (recursor collision) — `base`/`step` per CONTRACT §C4; JSON keys (`base`, `rec`) unchanged, decoder maps spelling → field.
- C5: do not add `recSelf` / `NonSelfAtom` / Fin-indexed atoms. Extra `.interior self` in `RecStep.atoms` stays a decoder-refused hand-construction; evaluators read self only through `selfBindings`.
- Must land as one change with lean-001. No Program vocabulary; no new caps. `toRule` must include the reconstructed self atom or `reachOp_empty` is false.
