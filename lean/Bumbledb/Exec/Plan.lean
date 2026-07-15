import Bumbledb.Query.Denotation

/-!
# Exec/Plan — the Free Join plan formalism (Level 1)

The paper's plan definition (Wang–Willsey–Suciu, *Free Join*,
arXiv:2301.10841 §3.2) mechanized at the mathematical level, with
bumbledb's one deviation modeled as bumbledb rules it
(`docs/architecture/40-execution.md` § the paper's core): a plan is
DATA — a list of nodes, each a set of subatoms (occurrence index plus
a subset of that occurrence's bound variables) — and execution is
iterated indexed selection over sets. The mechanism fence holds
whole: no COLT, no tries, no batching, no pipeline — those stay in
the docs; what is admissible here is exactly the semantic content of
"a validated plan computes the rule's denotation".

## The shape of the model

* **Validity is the doc's sentence, clause for clause** (`PlanValid`):
  the nodes partition every positive occurrence's bound variables
  (`coversVar` + `onceVar`), every occurrence is placed (`complete` —
  a zero-variable occurrence enters as a zero-arity subatom, the
  doc's nonemptiness-gate cover; without placement its constant
  bindings would never be probed), per node no two subatoms share an
  occurrence (`occDisjoint` — validity quantifies over OCCURRENCES,
  self-joins are ordinary), and each node holds a cover subatom whose
  variables are EXACTLY the node's new variables (`covered` —
  bumbledb's deviation; the paper's looser "containing all new
  variables" is `PaperCovered`, and `PlanValid.paper` records that
  the deviation only shrinks the admitted plan set).
* **Semantics is a fold over nodes producing a binding set**
  (`runPlan`/`planBindings`): at each node the bindings extend on the
  node's new variables and survive exactly when EVERY subatom is
  consistent — some fact of its occurrence matches the occurrence's
  checked prefix plus the subatom's own variables (`Consistent`, the
  sibling probe and the cover iteration as one `∃`). Sets, not
  streams: set semantics is the carrier. The totalization device:
  binding sets are sets of TOTAL assignments closed off the bound
  variables, seeded with every assignment — a "partial binding" is
  the class of its total extensions, so no partial-map machinery
  enters and projection through `Safe` finds is well-defined.
* **The cover clause is the enumerability licence, not a soundness
  premise** (`cover_drives_extension`): under validity, every
  surviving extension's new values are drawn from a cover subatom's
  matching fact — the node step enumerates a finite fact set, never
  guesses over the value universe. The set-equality theorems spend
  `occDisjoint` (per-subatom probes compose into one per-occurrence
  fact only when a node holds at most one subatom per occurrence),
  `coversVar`, and `complete`; `covered` is spent by the licence and
  `onceVar` by nothing here — it is the partition's at-most-once
  half, stated because the doc's "partitions" says it, and the
  executor's non-redundancy (each variable probed once) is its
  mechanism-side face.
* **THE soundness theorem** (`valid_plan_sound`): for any valid plan
  of a `WellTyped` rule, the plan's answers equal `ruleAnswers` —
  negated atoms and conditions applied as POST-FILTERS over the
  completed binding set (`planAnswers`), which is the D2/residual-
  placement licence at the math level: the executor attaches
  residuals and anti-probes at the earliest node where their
  variables are bound (mechanism, docs-side); that this equals
  filtering the completed set equals `derives` is what makes any
  placement sound. `WellTyped` is spent only through its
  measure-free-bindings half (a measure binding would mention a
  variable no subatom carries); `Safe` is NOT a premise of the
  equality — acceptance supplies it, and it is what makes the answer
  set finite and the projection meaningful (`antijoin_over_active_
  domain`), but the set equality holds without it.
* **Plannability** (`every_rule_plannable`): EVERY rule — a fortiori
  every safe rule — has a valid plan, constructively: the left-deep
  one-variable-per-node plan (`leftDeepPlan`) opens every occurrence
  with a zero-arity gate subatom (the doc's zero-arity cover), then
  binds the rule's positive variables in first-binding order
  (`dedupFirst r.positiveVars`), one node per variable, each node
  carrying one single-variable subatom per occurrence binding it —
  every such subatom's variable set is exactly the node's one new
  variable, so GJ-style single-variable covers qualify under the
  strict rule, exactly the doc's "loses nothing" argument. No
  admissible rule is unexecutable.
* **The wrong-cover countermodel lives in `Countermodels.lean`**
  (`loose_cover_rebinds`): `looseNodeStep` is the paper's reading —
  the chosen cover may carry already-bound variables and iterating it
  draws values for ALL its variables from its facts, REBINDING the
  bound ones without re-checking the occurrence that bound them
  (earlier nodes are never revisited) — and the triangle instance
  shows a paper-valid plan whose loose execution emits a tuple
  outside the denotation. That is 40-execution.md's audit-found
  deviation, until now prose plus a Rust regression test only.

## Narrowings recorded (law 5: narrow and record)

* **Occurrence indices, not occurrence values.** A subatom addresses
  its atom by position in `r.atoms` (`Subatom.occ : Nat`) — the
  self-join discipline demands occurrence identity, and position IS
  occurrence identity for a list-carried body.
* **`occDisjoint` concludes subatom EQUALITY.** Two subatoms of one
  node sharing an occurrence are forced equal (not absent): a
  duplicated subatom is semantically idle, and the list carrier makes
  "the same subatom twice" unrefusable at this level — the engine's
  validator refuses the duplicate as mechanism.
* **The program cut is not re-modeled.** Plans are stated over
  `Rule` (the degenerate one-predicate program); the recursive arm
  executes each rule's plan against the current tables, so the
  per-rule theorem is the one the fixpoint evaluator spends
  rule-by-rule. When the engine's recursion discharge campaign lands,
  the `PRule` restatement rides the same change (the gate law).
-/

namespace Bumbledb.Query

/-! ## Plan syntax — the paper's §3.2 as data -/

/-- One subatom: a positive atom occurrence (by position in
`r.atoms` — occurrence identity, so self-joins are ordinary) plus the
subset of that occurrence's bound variables this node processes. -/
structure Subatom where
  occ : Nat
  vars : List VarId
deriving DecidableEq

/-- One plan node: its subatoms. -/
abbrev PlanNode : Type := List Subatom

/-- A plan for one rule: the node list, executed in order. -/
abbrev Plan : Type := List PlanNode

/-- The variables a node's subatoms carry. -/
def nodeVars (n : PlanNode) : List VarId :=
  n.flatMap Subatom.vars

/-- The variables a plan prefix has bound. -/
def planVars (P : Plan) : List VarId :=
  P.flatMap nodeVars

/-- The variables occurrence `i` has been probed on across a prefix:
the union of its subatoms' variable sets — the occurrence's checked
trie prefix, as a variable list. -/
def checkedVars (P : Plan) (i : Nat) : List VarId :=
  P.flatMap fun n => (n.filter fun s => s.occ == i).flatMap Subatom.vars

/-- A node's NEW variables against a prefix: mentioned by the node,
not yet bound. -/
def NewVar (pre : Plan) (n : PlanNode) (v : VarId) : Prop :=
  v ∈ nodeVars n ∧ v ∉ planVars pre

/-- A prefix touches occurrence `i` when some node carries one of its
subatoms. -/
def Touches (P : Plan) (i : Nat) : Prop :=
  ∃ n, n ∈ P ∧ ∃ s, s ∈ n ∧ s.occ = i

/-- Membership in `nodeVars`, unfolded. -/
theorem mem_nodeVars {n : PlanNode} {v : VarId} :
    v ∈ nodeVars n ↔ ∃ s, s ∈ n ∧ v ∈ s.vars :=
  List.mem_flatMap

/-- Membership in `planVars`, unfolded. -/
theorem mem_planVars {P : Plan} {v : VarId} :
    v ∈ planVars P ↔ ∃ n, n ∈ P ∧ v ∈ nodeVars n :=
  List.mem_flatMap

/-- Membership in `checkedVars`, unfolded to the subatom witness. -/
theorem mem_checkedVars {P : Plan} {i : Nat} {v : VarId} :
    v ∈ checkedVars P i ↔
      ∃ n, n ∈ P ∧ ∃ s, s ∈ n ∧ s.occ = i ∧ v ∈ s.vars := by
  unfold checkedVars
  constructor
  · intro h
    obtain ⟨n, hn, hv⟩ := List.mem_flatMap.mp h
    obtain ⟨s, hs, hvs⟩ := List.mem_flatMap.mp hv
    obtain ⟨hsn, hocc⟩ := List.mem_filter.mp hs
    exact ⟨n, hn, s, hsn, by simpa using hocc, hvs⟩
  · rintro ⟨n, hn, s, hsn, hocc, hvs⟩
    exact List.mem_flatMap.mpr ⟨n, hn, List.mem_flatMap.mpr
      ⟨s, List.mem_filter.mpr ⟨hsn, by simpa using hocc⟩, hvs⟩⟩

/-- A checked variable is a bound variable — probes never outrun the
prefix. -/
theorem checkedVars_subset_planVars {P : Plan} {i : Nat} {v : VarId}
    (h : v ∈ checkedVars P i) : v ∈ planVars P := by
  obtain ⟨n, hn, s, hs, _, hv⟩ := mem_checkedVars.mp h
  exact mem_planVars.mpr ⟨n, hn, mem_nodeVars.mpr ⟨s, hs, hv⟩⟩

/-! ## Validity — the doc's definition, clause for clause -/

/-- The shared partition clauses (both cover rules sit on these):
subatoms are scoped to their occurrence's bound variables, every
occurrence is placed, every bound variable of every occurrence is
carried by one of its subatoms in EXACTLY one node, and per node no
two subatoms share an occurrence. -/
structure PlanPartition (r : Rule) (P : Plan) : Prop where
  occScoped : ∀ n, n ∈ P → ∀ s, s ∈ n →
    ∃ a, r.atoms[s.occ]? = some a ∧ ∀ v, v ∈ s.vars → v ∈ a.boundVars
  complete : ∀ i, i < r.atoms.length → Touches P i
  coversVar : ∀ i a, r.atoms[i]? = some a → ∀ v, v ∈ a.boundVars →
    ∃ n, n ∈ P ∧ ∃ s, s ∈ n ∧ s.occ = i ∧ v ∈ s.vars
  onceVar : ∀ (i : Nat) (v : VarId) (k₁ k₂ : Nat) (n₁ n₂ : PlanNode),
    P[k₁]? = some n₁ → P[k₂]? = some n₂ →
    (∃ s, s ∈ n₁ ∧ s.occ = i ∧ v ∈ s.vars) →
    (∃ s, s ∈ n₂ ∧ s.occ = i ∧ v ∈ s.vars) → k₁ = k₂
  occDisjoint : ∀ n, n ∈ P → ∀ s₁, s₁ ∈ n → ∀ s₂, s₂ ∈ n →
    s₁.occ = s₂.occ → s₁ = s₂

/-- **Plan validity — bumbledb's rule** (`40-execution.md` § the
paper's core): the partition clauses plus a cover per node whose
variables are EXACTLY the node's new variables. -/
structure PlanValid (r : Rule) (P : Plan) : Prop
    extends PlanPartition r P where
  covered : ∀ k n, P[k]? = some n →
    ∃ s, s ∈ n ∧ ∀ v, v ∈ s.vars ↔ NewVar (P.take k) n v

/-- The PAPER's cover rule (its Definition's "containing all new
variables"): a cover need only CONTAIN the node's new variables — it
may also carry already-bound ones. The refused reading; the
countermodel is `Countermodels.loose_cover_rebinds`. -/
def PaperCovered (P : Plan) : Prop :=
  ∀ k n, P[k]? = some n →
    ∃ s, s ∈ n ∧ ∀ v, NewVar (P.take k) n v → v ∈ s.vars

/-- Plan validity under the paper's looser cover rule. -/
structure PaperPlanValid (r : Rule) (P : Plan) : Prop
    extends PlanPartition r P where
  covered : PaperCovered P

/-- The deviation only SHRINKS the admitted plan set: every
bumbledb-valid plan is paper-valid (exactly-the-new-variables
contains the new variables). The converse fails —
`Countermodels.loose_plan_not_valid` is the witness. -/
theorem PlanValid.paper {r : Rule} {P : Plan} (h : PlanValid r P) :
    PaperPlanValid r P :=
  { h.toPlanPartition with
    covered := fun k n hk => by
      obtain ⟨s, hs, hiff⟩ := h.covered k n hk
      exact ⟨s, hs, fun v hv => (hiff v).mpr hv⟩ }

/-! ## The prefix-restricted matching judgment -/

/-- `MatchesOn f a σ ρ V`: every binding whose term mentions only
variables in `V` selects — the matching equation restricted to a
checked prefix. Constant bindings (params, literals, param sets)
mention no variables, so they are checked from the first probe: the
per-atom filter applies at the source, exactly the filtered-view
reading. `MatchesOn` at full variable coverage is `Matches`
(`MatchesOn.matches`). -/
def MatchesOn (f : Fact) (a : Atom) (σ : Assignment) (ρ : ParamEnv)
    (V : List VarId) : Prop :=
  ∀ b, b ∈ a.bindings → (∀ v, v ∈ b.2.vars → v ∈ V) →
    Term.selects ρ σ b.2 (f b.1)

/-- A full match restricts to any prefix. -/
theorem Matches.matchesOn {f : Fact} {a : Atom} {σ : Assignment}
    {ρ : ParamEnv} (h : Matches f a σ ρ) (V : List VarId) :
    MatchesOn f a σ ρ V :=
  fun b hb _ => h b hb

/-- Restriction is antitone: a match on a wider prefix matches on any
narrower one. -/
theorem MatchesOn.anti {f : Fact} {a : Atom} {σ : Assignment}
    {ρ : ParamEnv} {V V' : List VarId}
    (hsub : ∀ v, v ∈ V → v ∈ V') (h : MatchesOn f a σ ρ V') :
    MatchesOn f a σ ρ V :=
  fun b hb hg => h b hb fun v hv => hsub v (hg v hv)

/-- The restricted judgment reads the assignment on `V` alone. -/
theorem MatchesOn.agree {f : Fact} {a : Atom} {σ σ' : Assignment}
    {ρ : ParamEnv} {V : List VarId}
    (hag : ∀ v, v ∈ V → σ v = σ' v) (h : MatchesOn f a σ ρ V) :
    MatchesOn f a σ' ρ V :=
  fun b hb hg =>
    (selects_congr fun v hv => hag v (hg v hv)).mp (h b hb hg)

/-- At full coverage of the atom's bound variables — and with no
measure bindings, the `WellTyped` half this file spends — the
restricted judgment IS the matching equation. -/
theorem MatchesOn.matches {f : Fact} {a : Atom} {σ : Assignment}
    {ρ : ParamEnv} {V : List VarId}
    (hnm : ∀ b, b ∈ a.bindings → ¬ b.2.isMeasure)
    (hcov : ∀ v, v ∈ a.boundVars → v ∈ V)
    (h : MatchesOn f a σ ρ V) : Matches f a σ ρ := by
  intro b hb
  refine h b hb fun v hv => hcov v ?_
  cases hb2 : b.2 with
  | var w =>
    rw [hb2] at hv
    simp only [Term.vars, List.mem_singleton] at hv
    subst hv
    exact List.mem_flatMap.mpr
      ⟨b, hb, by rw [hb2]; simp [Term.bindingVars]⟩
  | measure w =>
    exact absurd (show b.2.isMeasure by rw [hb2]; trivial) (hnm b hb)
  | param p => rw [hb2] at hv; simp [Term.vars] at hv
  | paramSet p => rw [hb2] at hv; simp [Term.vars] at hv
  | lit c => rw [hb2] at hv; simp [Term.vars] at hv

/-! ## Plan semantics — the fold over nodes -/

/-- One subatom's consistency against a prefix: some fact of its
occurrence's relation matches on the occurrence's checked prefix plus
the subatom's own variables. This one `∃` is BOTH executor moves:
iterating a cover (its variables are the node's new ones, so the `∃`
enumerates the extensions) and probing a sibling (its new variables
are already placed, so the `∃` is the membership test). -/
def Consistent (r : Rule) (I : Instance) (ρ : ParamEnv) (pre : Plan)
    (s : Subatom) (σ : Assignment) : Prop :=
  ∃ a, r.atoms[s.occ]? = some a ∧ ∃ f, f ∈ I a.relation ∧
    MatchesOn f a σ ρ (checkedVars pre s.occ ++ s.vars)

/-- One node's step: the bindings extend on the node's new variables
and survive exactly when every subatom is consistent. Under validity
the extension is DRAWN from a cover subatom's matching facts — the
cover sits among the subatoms and its variables are exactly the new
ones, so its consistency clause is the draw; `cover_drives_extension`
records the licence. -/
def nodeStep (r : Rule) (I : Instance) (ρ : ParamEnv) (pre : Plan)
    (n : PlanNode) (S : Set Assignment) : Set Assignment :=
  fun σ' => ∃ σ, σ ∈ S ∧ (∀ v, ¬ NewVar pre n v → σ' v = σ v) ∧
    ∀ s, s ∈ n → Consistent r I ρ pre s σ'

/-- The fold: run the remaining nodes against a prefix. -/
def runPlan (r : Rule) (I : Instance) (ρ : ParamEnv) :
    Plan → Plan → Set Assignment → Set Assignment
  | _, [], S => S
  | pre, n :: rest, S =>
    runPlan r I ρ (pre ++ [n]) rest (nodeStep r I ρ pre n S)

/-- The plan's binding set: every node folded from the empty prefix,
seeded with every assignment (the totalization device — the seed is
"nothing bound yet", and each stage's set is closed off the bound
variables). -/
def planBindings (r : Rule) (P : Plan) (I : Instance) (ρ : ParamEnv) :
    Set Assignment :=
  runPlan r I ρ [] P fun _ => True

/-- The plan's answers: the completed binding set, POST-FILTERED by
the negated atoms (the anti-join, as the executor's anti-probe) and
the condition trees (residuals), projected through the finds —
`valid_plan_sound` is the equality with `ruleAnswers`, the
residual-placement licence. -/
def planAnswers (C : Classify) (r : Rule) (P : Plan) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, σ ∈ planBindings r P I ρ ∧
    (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
    (∀ c, c ∈ r.conditions → Condition.holds C ρ σ c) ∧
    t = r.finds.map σ

/-! ## The fold invariant and the set-equality spine -/

/-- Pointwise-equal sets are equal — the extensionality step the
fold rewrites ride. -/
private theorem setExt {α : Type} {s t : Set α}
    (h : ∀ x, x ∈ s ↔ x ∈ t) : s = t :=
  funext fun x => propext (h x)

/-- The invariant the fold maintains: every TOUCHED occurrence holds
a fact matching on its checked prefix. Untouched occurrences carry no
obligation yet — which is exactly why validity's `complete` clause is
load-bearing: an unplaced occurrence's constant filters would never
apply. -/
def probed (r : Rule) (I : Instance) (ρ : ParamEnv) (P : Plan) :
    Set Assignment :=
  fun σ => ∀ i a, r.atoms[i]? = some a → Touches P i →
    ∃ f, f ∈ I a.relation ∧ MatchesOn f a σ ρ (checkedVars P i)

/-- **The one-node lemma**: stepping the invariant set through a node
lands exactly on the invariant of the extended prefix. The
`occDisjoint` premise is load-bearing: two subatoms of one occurrence
in one node would each produce their OWN matching fact, and the two
`∃`s do not compose into the one fact the extended prefix demands. -/
theorem nodeStep_probed {r : Rule} {I : Instance} {ρ : ParamEnv}
    {pre : Plan} {n : PlanNode}
    (hdisj : ∀ s₁, s₁ ∈ n → ∀ s₂, s₂ ∈ n → s₁.occ = s₂.occ → s₁ = s₂)
    (hscope : ∀ s, s ∈ n → ∃ a, r.atoms[s.occ]? = some a) :
    nodeStep r I ρ pre n (probed r I ρ pre)
      = probed r I ρ (pre ++ [n]) := by
  refine setExt fun σ' => ?_
  constructor
  · rintro ⟨σ, hσ, hag, hcons⟩
    intro i a hia htouch
    by_cases hin : ∃ s, s ∈ n ∧ s.occ = i
    · obtain ⟨s₀, hs₀, hocc₀⟩ := hin
      obtain ⟨a', ha', f, hf, hm⟩ := hcons s₀ hs₀
      rw [hocc₀] at ha' hm
      rw [hia] at ha'
      obtain rfl : a' = a := (Option.some.inj ha').symm
      refine ⟨f, hf, hm.anti fun v hv => ?_⟩
      obtain ⟨m, hmem, s, hs, hocc, hvs⟩ := mem_checkedVars.mp hv
      rcases List.mem_append.mp hmem with hmp | hmn
      · exact List.mem_append.mpr
          (Or.inl (mem_checkedVars.mpr ⟨m, hmp, s, hs, hocc, hvs⟩))
      · rw [List.mem_singleton.mp hmn] at hs
        have : s = s₀ := hdisj s hs s₀ hs₀ (hocc.trans hocc₀.symm)
        rw [this] at hvs
        exact List.mem_append.mpr (Or.inr hvs)
    · have htpre : Touches pre i := by
        obtain ⟨m, hmem, s, hs, hocc⟩ := htouch
        rcases List.mem_append.mp hmem with hmp | hmn
        · exact ⟨m, hmp, s, hs, hocc⟩
        · rw [List.mem_singleton.mp hmn] at hs
          exact absurd ⟨s, hs, hocc⟩ hin
      obtain ⟨f, hf, hm⟩ := hσ i a hia htpre
      have hm' : MatchesOn f a σ' ρ (checkedVars pre i) :=
        hm.agree fun v hv =>
          (hag v fun hnew =>
            hnew.2 (checkedVars_subset_planVars hv)).symm
      refine ⟨f, hf, hm'.anti fun v hv => ?_⟩
      obtain ⟨m, hmem, s, hs, hocc, hvs⟩ := mem_checkedVars.mp hv
      rcases List.mem_append.mp hmem with hmp | hmn
      · exact mem_checkedVars.mpr ⟨m, hmp, s, hs, hocc, hvs⟩
      · rw [List.mem_singleton.mp hmn] at hs
        exact absurd ⟨s, hs, hocc⟩ hin
  · intro hσ'
    have hpre : σ' ∈ probed r I ρ pre := by
      intro i a hia ht
      obtain ⟨m, hmem, s, hs, hocc⟩ := ht
      obtain ⟨f, hf, hm⟩ := hσ' i a hia
        ⟨m, List.mem_append.mpr (Or.inl hmem), s, hs, hocc⟩
      refine ⟨f, hf, hm.anti fun v hv => ?_⟩
      obtain ⟨m', hm', s', hs', hocc', hvs'⟩ := mem_checkedVars.mp hv
      exact mem_checkedVars.mpr
        ⟨m', List.mem_append.mpr (Or.inl hm'), s', hs', hocc', hvs'⟩
    refine ⟨σ', hpre, fun v _ => rfl, fun s hs => ?_⟩
    obtain ⟨a, ha⟩ := hscope s hs
    obtain ⟨f, hf, hm⟩ := hσ' s.occ a ha
      ⟨n, List.mem_append.mpr (Or.inr (List.mem_singleton.mpr rfl)),
        s, hs, rfl⟩
    refine ⟨a, ha, f, hf, hm.anti fun v hv => ?_⟩
    rcases List.mem_append.mp hv with hvc | hvs
    · obtain ⟨m', hm', s', hs', hocc', hvs'⟩ := mem_checkedVars.mp hvc
      exact mem_checkedVars.mpr
        ⟨m', List.mem_append.mpr (Or.inl hm'), s', hs', hocc', hvs'⟩
    · exact mem_checkedVars.mpr
        ⟨n, List.mem_append.mpr (Or.inr (List.mem_singleton.mpr rfl)),
          s, hs, rfl, hvs⟩

/-- The fold walks the invariant from any prefix to its end. -/
theorem runPlan_probed {r : Rule} {I : Instance} {ρ : ParamEnv} :
    ∀ rest pre : Plan,
      (∀ n, n ∈ rest →
        (∀ s₁, s₁ ∈ n → ∀ s₂, s₂ ∈ n → s₁.occ = s₂.occ → s₁ = s₂) ∧
        (∀ s, s ∈ n → ∃ a, r.atoms[s.occ]? = some a)) →
      runPlan r I ρ pre rest (probed r I ρ pre)
        = probed r I ρ (pre ++ rest)
  | [], pre, _ => by rw [runPlan, List.append_nil]
  | n :: rest, pre, h => by
    rw [runPlan,
      nodeStep_probed (h n List.mem_cons_self).1
        (h n List.mem_cons_self).2,
      runPlan_probed rest (pre ++ [n])
        (fun m hm => h m (List.mem_cons_of_mem _ hm)),
      List.append_assoc]
    rfl

/-- **The binding-set theorem**: a valid plan's fold computes exactly
the assignments whose every positive atom holds a matching fact — the
positive-body judgment of `derives`, whole. Spends `complete` +
`coversVar` (the final prefix checks every binding of every
occurrence), `occDisjoint` (probe composition), `scoped` (occurrence
indices resolve), and `WellTyped`'s measure-free half. -/
theorem planBindings_positive {r : Rule} {P : Plan} {I : Instance}
    {ρ : ParamEnv} (hv : PlanValid r P) (hwt : r.WellTyped) :
    ∀ σ, σ ∈ planBindings r P I ρ ↔
      ∀ a, a ∈ r.atoms → ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ := by
  have hstart : (fun _ => True : Set Assignment) = probed r I ρ [] := by
    refine setExt fun σ => ?_
    constructor
    · rintro - i a - ⟨m, hm, -⟩
      exact absurd hm (List.not_mem_nil)
    · intro _
      trivial
  have hrun := runPlan_probed (r := r) (I := I) (ρ := ρ) P []
    fun n hn =>
      ⟨hv.occDisjoint n hn,
        fun s hs => ⟨(hv.occScoped n hn s hs).choose,
          (hv.occScoped n hn s hs).choose_spec.1⟩⟩
  intro σ
  unfold planBindings
  rw [hstart, hrun]
  constructor
  · intro h a ha
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.mp ha
    obtain ⟨hlt, -⟩ := List.getElem?_eq_some_iff.mp hi
    obtain ⟨f, hf, hm⟩ := h i a hi (by simpa using hv.complete i hlt)
    refine ⟨f, hf, hm.matches (fun b hb => hwt.1 a (Or.inl ha) b hb)
      fun v hvv => ?_⟩
    obtain ⟨m, hmm, s, hss, hocc, hvs⟩ := hv.coversVar i a hi v hvv
    exact mem_checkedVars.mpr ⟨m, by simpa using hmm, s, hss, hocc, hvs⟩
  · intro h i a hi _
    obtain ⟨f, hf, hm⟩ := h a (List.mem_of_getElem? hi)
    exact ⟨f, hf, hm.matchesOn _⟩

/-! ## THE soundness theorem -/

/-- **Free Join plan soundness** — for a `WellTyped` rule and ANY
valid plan, the plan's answers equal the rule's denotation: the fold
computes the positive-atom join (`planBindings_positive`), and the
negated atoms and condition trees applied as post-filters over the
completed binding set reconstruct `derives` exactly — the
D2/residual-placement licence at the math level (where the executor
attaches them earlier is mechanism; that the completed-set filter is
the denotation is what makes any placement sound). `Safe` is not a
premise of the equality — acceptance supplies it for finiteness and
projection meaning, and the doc records the narrowing. -/
theorem valid_plan_sound {C : Classify} {r : Rule} {P : Plan}
    {I : Instance} {ρ : ParamEnv} (hv : PlanValid r P)
    (hwt : r.WellTyped) :
    ∀ t, t ∈ planAnswers C r P I ρ ↔ t ∈ ruleAnswers C r I ρ := by
  intro t
  constructor
  · rintro ⟨σ, hb, hneg, hcond, rfl⟩
    exact mem_ruleAnswers.mpr
      ⟨σ, ⟨(planBindings_positive hv hwt σ).mp hb, hneg, hcond⟩, rfl⟩
  · intro ht
    obtain ⟨σ, ⟨hpos, hneg, hcond⟩, rfl⟩ := mem_ruleAnswers.mp ht
    exact ⟨σ, (planBindings_positive hv hwt σ).mpr hpos, hneg, hcond,
      rfl⟩

/-- **The enumerability licence** (why the cover clause exists):
under validity, every extension surviving a node draws its new values
from a COVER subatom's matching fact — the cover's variables are
exactly the node's new variables, so its consistency clause pins
`σ'` on the new variables to a fact of a finite extension. The node
step never guesses over the value universe; iteration is the
mechanism face (COLT, dynamic cover choice — docs-side, whole). -/
theorem cover_drives_extension {r : Rule} {I : Instance}
    {ρ : ParamEnv} {P : Plan} {k : Nat} {n : PlanNode}
    {S : Set Assignment} {σ' : Assignment} (hv : PlanValid r P)
    (hk : P[k]? = some n)
    (hσ : σ' ∈ nodeStep r I ρ (P.take k) n S) :
    ∃ s, s ∈ n ∧ (∀ v, v ∈ s.vars ↔ NewVar (P.take k) n v) ∧
      Consistent r I ρ (P.take k) s σ' := by
  obtain ⟨s, hs, hcov⟩ := hv.covered k n hk
  obtain ⟨-, -, -, hcons⟩ := hσ
  exact ⟨s, hs, hcov, hcons s hs⟩

/-! ## Plannability — every rule has a valid plan -/

/-- Keep-first deduplication, seen-set form (structural recursion):
emit each element at its FIRST occurrence, skipping seen ones. -/
def dedupFirstAux (seen : List VarId) : List VarId → List VarId
  | [] => []
  | v :: vs =>
    if v ∈ seen then dedupFirstAux seen vs
    else v :: dedupFirstAux (v :: seen) vs

/-- Keep-first deduplication: the variable order of the left-deep
plan is first-binding order. -/
def dedupFirst (l : List VarId) : List VarId :=
  dedupFirstAux [] l

/-- Membership through the seen-set: emitted iff present and not yet
seen. -/
theorem mem_dedupFirstAux : ∀ (l seen : List VarId) (w : VarId),
    w ∈ dedupFirstAux seen l ↔ w ∈ l ∧ w ∉ seen
  | [], seen, w => by simp [dedupFirstAux]
  | v :: vs, seen, w => by
    rw [dedupFirstAux]
    by_cases hv : v ∈ seen
    · rw [if_pos hv, mem_dedupFirstAux vs seen w, List.mem_cons]
      constructor
      · rintro ⟨h1, h2⟩
        exact ⟨Or.inr h1, h2⟩
      · rintro ⟨rfl | h1, h2⟩
        · exact absurd hv h2
        · exact ⟨h1, h2⟩
    · rw [if_neg hv]
      simp only [List.mem_cons, mem_dedupFirstAux vs (v :: seen) w]
      constructor
      · rintro (rfl | ⟨h1, h2⟩)
        · exact ⟨Or.inl rfl, hv⟩
        · exact ⟨Or.inr h1, fun hs => h2 (Or.inr hs)⟩
      · rintro ⟨rfl | h1, h2⟩
        · exact Or.inl rfl
        · by_cases hwv : w = v
          · exact Or.inl hwv
          · exact Or.inr ⟨h1, fun hc => hc.elim hwv h2⟩

/-- Deduplication preserves membership. -/
theorem mem_dedupFirst (l : List VarId) (w : VarId) :
    w ∈ dedupFirst l ↔ w ∈ l := by
  rw [dedupFirst, mem_dedupFirstAux]
  exact ⟨fun h => h.1, fun h => ⟨h, List.not_mem_nil⟩⟩

/-- The seen-set form holds each element at ONE position. -/
theorem dedupFirstAux_getElem?_inj :
    ∀ (l seen : List VarId) (j₁ j₂ : Nat) (w : VarId),
      (dedupFirstAux seen l)[j₁]? = some w →
      (dedupFirstAux seen l)[j₂]? = some w → j₁ = j₂
  | [], _, j₁, j₂, w => by simp [dedupFirstAux]
  | v :: vs, seen, j₁, j₂, w => by
    rw [dedupFirstAux]
    by_cases hv : v ∈ seen
    · rw [if_pos hv]
      exact dedupFirstAux_getElem?_inj vs seen j₁ j₂ w
    · rw [if_neg hv]
      have hhead : ∀ j : Nat,
          (dedupFirstAux (v :: seen) vs)[j]? = some v → False := by
        intro j hj
        have hmem := List.mem_of_getElem? hj
        rw [mem_dedupFirstAux] at hmem
        exact hmem.2 List.mem_cons_self
      intro h₁ h₂
      match j₁, j₂, h₁, h₂ with
      | 0, 0, _, _ => rfl
      | 0, j₂ + 1, h₁, h₂ =>
        rw [List.getElem?_cons_zero] at h₁
        rw [List.getElem?_cons_succ] at h₂
        obtain rfl : v = w := Option.some.inj h₁
        exact absurd h₂ fun h => hhead j₂ h
      | j₁ + 1, 0, h₁, h₂ =>
        rw [List.getElem?_cons_zero] at h₂
        rw [List.getElem?_cons_succ] at h₁
        obtain rfl : v = w := Option.some.inj h₂
        exact absurd h₁ fun h => hhead j₁ h
      | j₁ + 1, j₂ + 1, h₁, h₂ =>
        rw [List.getElem?_cons_succ] at h₁ h₂
        exact congrArg (· + 1)
          (dedupFirstAux_getElem?_inj vs (v :: seen) j₁ j₂ w h₁ h₂)

/-- A deduplicated list holds each element at ONE position — the
at-most-once half of the left-deep plan's partition. -/
theorem dedupFirst_getElem?_inj {l : List VarId} {j₁ j₂ : Nat}
    {v : VarId} (h₁ : (dedupFirst l)[j₁]? = some v)
    (h₂ : (dedupFirst l)[j₂]? = some v) : j₁ = j₂ :=
  dedupFirstAux_getElem?_inj l [] j₁ j₂ v h₁ h₂

/-- The gate segment: every occurrence opens with a zero-arity
subatom — the doc's zero-arity cover (a nonemptiness gate collapses
to one entry), and what places constant-only occurrences. -/
def gatePlan (r : Rule) : Plan :=
  (List.range r.atoms.length).map fun i => [⟨i, []⟩]

/-- The node binding variable `v`: one single-variable subatom per
occurrence that binds `v`. -/
def varNode (r : Rule) (v : VarId) : PlanNode :=
  (List.range r.atoms.length).filterMap fun i =>
    (r.atoms[i]?).bind fun a =>
      if v ∈ a.boundVars then some ⟨i, [v]⟩ else none

/-- Membership in `varNode`, unfolded to the occurrence witness. -/
theorem mem_varNode {r : Rule} {v : VarId} {s : Subatom} :
    s ∈ varNode r v ↔
      ∃ i a, r.atoms[i]? = some a ∧ v ∈ a.boundVars ∧
        s = ⟨i, [v]⟩ := by
  unfold varNode
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨i, -, hs⟩
    cases ha : r.atoms[i]? with
    | none => rw [ha, Option.bind_none] at hs; cases hs
    | some a =>
      rw [ha, Option.bind_some] at hs
      by_cases hb : v ∈ a.boundVars
      · rw [if_pos hb] at hs
        exact ⟨i, a, ha, hb, (Option.some.inj hs).symm⟩
      · rw [if_neg hb] at hs
        cases hs
  · rintro ⟨i, a, ha, hb, rfl⟩
    obtain ⟨hlt, -⟩ := List.getElem?_eq_some_iff.mp ha
    refine ⟨i, List.mem_range.mpr hlt, ?_⟩
    rw [ha, Option.bind_some, if_pos hb]

/-- **The left-deep one-variable-per-node plan**: the gate segment,
then one node per positive variable in first-binding order. Each
variable node's cover is any of its single-variable subatoms — its
variable set is exactly the node's one new variable, so the strict
cover rule is satisfied by construction (the doc's "GJ-style
single-variable covers all qualify"). -/
def leftDeepPlan (r : Rule) : Plan :=
  gatePlan r ++ (dedupFirst r.positiveVars).map (varNode r)

/-- Where a variable-carrying subatom can live in the left-deep plan:
only at the variable's own node — position `gates + j` where `j` is
the variable's one dedup position. -/
theorem leftDeepPlan_var_position {r : Rule} {k : Nat} {m : PlanNode}
    {v : VarId} (hk : (leftDeepPlan r)[k]? = some m)
    (hv : ∃ s, s ∈ m ∧ v ∈ s.vars) :
    ∃ j, k = r.atoms.length + j ∧
      (dedupFirst r.positiveVars)[j]? = some v := by
  obtain ⟨s, hs, hvs⟩ := hv
  have hglen : (gatePlan r).length = r.atoms.length := by
    simp [gatePlan]
  by_cases hlt : k < r.atoms.length
  · exfalso
    rw [leftDeepPlan, List.getElem?_append_left (by omega : k < (gatePlan r).length)] at hk
    unfold gatePlan at hk
    rw [List.getElem?_map] at hk
    cases hr : (List.range r.atoms.length)[k]? with
    | none => rw [hr] at hk; cases hk
    | some i =>
      rw [hr] at hk
      obtain rfl : [(⟨i, []⟩ : Subatom)] = m := Option.some.inj hk
      rw [List.mem_singleton.mp hs] at hvs
      exact absurd hvs (List.not_mem_nil)
  · rw [leftDeepPlan, List.getElem?_append_right (by omega : (gatePlan r).length ≤ k),
      List.getElem?_map] at hk
    cases hw : (dedupFirst r.positiveVars)[k - (gatePlan r).length]? with
    | none => rw [hw] at hk; cases hk
    | some w =>
      rw [hw] at hk
      obtain rfl : varNode r w = m := Option.some.inj hk
      obtain ⟨i, a, -, -, rfl⟩ := mem_varNode.mp hs
      obtain rfl : v = w := List.mem_singleton.mp hvs
      exact ⟨k - (gatePlan r).length, by omega, hw⟩

/-- Membership in a `take` prefix, at a position below the cut. -/
theorem mem_take_getElem? {α : Type} {l : List α} {k : Nat} {x : α}
    (h : x ∈ l.take k) : ∃ j, j < k ∧ l[j]? = some x := by
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.mp h
  by_cases hlt : j < k
  · rw [List.getElem?_take_of_lt hlt] at hj
    exact ⟨j, hlt, hj⟩
  · rw [List.getElem?_take_eq_none (by omega)] at hj
    cases hj

/-- **Plannability, constructive**: the left-deep plan is valid — for
EVERY rule, safe or not (validity reads only the positive atoms; the
narrowing is recorded in the module doc). -/
theorem leftDeepPlan_valid (r : Rule) : PlanValid r (leftDeepPlan r) := by
  have hglen : (gatePlan r).length = r.atoms.length := by
    simp [gatePlan]
  have hmem_gate : ∀ n, n ∈ gatePlan r →
      ∃ i, i < r.atoms.length ∧ n = [⟨i, []⟩] := by
    intro n hn
    obtain ⟨i, hi, rfl⟩ := List.mem_map.mp hn
    exact ⟨i, List.mem_range.mp hi, rfl⟩
  refine
    { occScoped := ?_, complete := ?_, coversVar := ?_, onceVar := ?_,
      occDisjoint := ?_, covered := ?_ }
  · -- scoped
    intro n hn s hs
    rcases List.mem_append.mp hn with hg | hv
    · obtain ⟨i, hlt, rfl⟩ := hmem_gate n hg
      rw [List.mem_singleton.mp hs]
      exact ⟨r.atoms[i], List.getElem?_eq_getElem hlt,
        fun v hv => absurd hv (List.not_mem_nil)⟩
    · obtain ⟨w, -, rfl⟩ := List.mem_map.mp hv
      obtain ⟨i, a, ha, hb, rfl⟩ := mem_varNode.mp hs
      exact ⟨a, ha, fun v hvv => by
        rw [List.mem_singleton.mp hvv]; exact hb⟩
  · -- complete
    intro i hlt
    exact ⟨[⟨i, []⟩],
      List.mem_append.mpr (Or.inl (List.mem_map.mpr
        ⟨i, List.mem_range.mpr hlt, rfl⟩)),
      ⟨i, []⟩, List.mem_singleton.mpr rfl, rfl⟩
  · -- coversVar
    intro i a hia v hv
    have hpos : v ∈ r.positiveVars :=
      List.mem_flatMap.mpr ⟨a, List.mem_of_getElem? hia, hv⟩
    refine ⟨varNode r v,
      List.mem_append.mpr (Or.inr (List.mem_map.mpr
        ⟨v, (mem_dedupFirst _ v).mpr hpos, rfl⟩)),
      ⟨i, [v]⟩, mem_varNode.mpr ⟨i, a, hia, hv, rfl⟩, rfl,
      List.mem_singleton.mpr rfl⟩
  · -- onceVar
    intro i v k₁ k₂ n₁ n₂ h₁ h₂ hs₁ hs₂
    obtain ⟨j₁, rfl, hd₁⟩ := leftDeepPlan_var_position h₁
      (hs₁.imp fun s hs => ⟨hs.1, hs.2.2⟩)
    obtain ⟨j₂, rfl, hd₂⟩ := leftDeepPlan_var_position h₂
      (hs₂.imp fun s hs => ⟨hs.1, hs.2.2⟩)
    rw [dedupFirst_getElem?_inj hd₁ hd₂]
  · -- occDisjoint
    intro n hn s₁ hs₁ s₂ hs₂ hocc
    rcases List.mem_append.mp hn with hg | hv
    · obtain ⟨i, -, rfl⟩ := hmem_gate n hg
      rw [List.mem_singleton.mp hs₁, List.mem_singleton.mp hs₂]
    · obtain ⟨w, -, rfl⟩ := List.mem_map.mp hv
      obtain ⟨i₁, a₁, -, -, rfl⟩ := mem_varNode.mp hs₁
      obtain ⟨i₂, a₂, -, -, rfl⟩ := mem_varNode.mp hs₂
      simp only at hocc
      rw [hocc]
  · -- covered
    intro k n hk
    by_cases hlt : k < r.atoms.length
    · have hkg : (leftDeepPlan r)[k]? = some [⟨k, []⟩] := by
        rw [leftDeepPlan,
          List.getElem?_append_left (by omega : k < (gatePlan r).length)]
        unfold gatePlan
        rw [List.getElem?_map, List.getElem?_range hlt]
        rfl
      rw [hk] at hkg
      obtain rfl := Option.some.inj hkg
      refine ⟨⟨k, []⟩, List.mem_singleton.mpr rfl, fun v => ?_⟩
      constructor
      · intro hv
        exact absurd hv (List.not_mem_nil)
      · rintro ⟨hv, -⟩
        obtain ⟨s, hs, hvs⟩ := mem_nodeVars.mp hv
        rw [List.mem_singleton.mp hs] at hvs
        exact absurd hvs (List.not_mem_nil)
    · have hkv : (leftDeepPlan r)[k]? =
          ((dedupFirst r.positiveVars).map (varNode r))[k - (gatePlan r).length]? := by
        rw [leftDeepPlan,
          List.getElem?_append_right (by omega : (gatePlan r).length ≤ k)]
      rw [hk, List.getElem?_map] at hkv
      cases hw : (dedupFirst r.positiveVars)[k - (gatePlan r).length]? with
      | none => rw [hw] at hkv; cases hkv
      | some w =>
        rw [hw] at hkv
        obtain rfl : n = varNode r w := Option.some.inj hkv
        have hwpos : w ∈ r.positiveVars :=
          (mem_dedupFirst _ w).mp (List.mem_of_getElem? hw)
        obtain ⟨a, hamem, hwa⟩ := List.mem_flatMap.mp hwpos
        obtain ⟨i₀, hi₀⟩ := List.mem_iff_getElem?.mp hamem
        refine ⟨⟨i₀, [w]⟩, mem_varNode.mpr ⟨i₀, a, hi₀, hwa, rfl⟩,
          fun v => ?_⟩
        constructor
        · intro hv
          obtain rfl : v = w := List.mem_singleton.mp hv
          refine ⟨mem_nodeVars.mpr ⟨⟨i₀, [v]⟩,
            mem_varNode.mpr ⟨i₀, a, hi₀, hwa, rfl⟩,
            List.mem_singleton.mpr rfl⟩, fun hbound => ?_⟩
          obtain ⟨m, hmem, hwm⟩ := mem_planVars.mp hbound
          obtain ⟨j, hjk, hj⟩ := mem_take_getElem? hmem
          obtain ⟨j', rfl, hd⟩ := leftDeepPlan_var_position hj
            (mem_nodeVars.mp hwm)
          have := dedupFirst_getElem?_inj hd hw
          omega
        · rintro ⟨hv, -⟩
          obtain ⟨s, hs, hvs⟩ := mem_nodeVars.mp hv
          obtain ⟨i', a', -, -, rfl⟩ := mem_varNode.mp hs
          exact hvs

/-- **Plannability**: every rule has at least one valid plan — no
admissible rule is unexecutable. -/
theorem every_rule_plannable (r : Rule) : ∃ P, PlanValid r P :=
  ⟨leftDeepPlan r, leftDeepPlan_valid r⟩

/-! ## The paper's loose node step — the refused reading

Modeled here so the countermodel can execute it: under the paper's
cover rule the chosen cover may carry already-bound variables, and
iterating it draws values for ALL its variables from its matching
facts — REBINDING the bound ones. The node's own subatoms are probed
against the rebound assignment, but earlier nodes' occurrences are
NEVER revisited (the executor does not re-check a finished node), so
a rebound variable escapes the occurrence that bound it.
`Countermodels.loose_cover_rebinds` is the triangle instance where
this differs from the denotation. -/

/-- The paper-rule node step: some cover containing the new variables
drives the extension over its WHOLE variable set. -/
def looseNodeStep (r : Rule) (I : Instance) (ρ : ParamEnv)
    (pre : Plan) (n : PlanNode) (S : Set Assignment) :
    Set Assignment :=
  fun σ' => ∃ σ, σ ∈ S ∧ ∃ c, c ∈ n ∧
    (∀ v, NewVar pre n v → v ∈ c.vars) ∧
    (∀ v, v ∉ c.vars → σ' v = σ v) ∧
    ∀ s, s ∈ n → Consistent r I ρ pre s σ'

/-- The loose fold. -/
def looseRun (r : Rule) (I : Instance) (ρ : ParamEnv) :
    Plan → Plan → Set Assignment → Set Assignment
  | _, [], S => S
  | pre, n :: rest, S =>
    looseRun r I ρ (pre ++ [n]) rest (looseNodeStep r I ρ pre n S)

/-- The loose binding set. -/
def looseBindings (r : Rule) (P : Plan) (I : Instance)
    (ρ : ParamEnv) : Set Assignment :=
  looseRun r I ρ [] P fun _ => True

/-- The loose answers — same post-filters, loose fold. -/
def looseAnswers (C : Classify) (r : Rule) (P : Plan) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  fun t => ∃ σ, σ ∈ looseBindings r P I ρ ∧
    (∀ a, a ∈ r.negated → ¬ ∃ f, f ∈ I a.relation ∧ Matches f a σ ρ) ∧
    (∀ c, c ∈ r.conditions → Condition.holds C ρ σ c) ∧
    t = r.finds.map σ

end Bumbledb.Query
