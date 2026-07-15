import Bumbledb.Query.Denotation

/-!
# Exec/Rewrites — the prepare-time rewrites, proved (Level 1, PRD 08)

Grounding as denotation-preserving partial evaluation, the key-probe
plan, statically-empty folds, and their composition — the formal arm
of the claim the rewrites fuzz target checks empirically: "rewrites
are semantics-preserving". Algorithmic essence only, per the mechanism
fence: grounding is substitution against ground axioms, a key probe is
one determinant get, static emptiness is a refuted condition.

## The Rust readings (READ-RUST-FIRST, file:line anchors)

* **The grounding loop** — `crates/bumbledb/src/plan/ground.rs:129-156`
  (`ground`: elimination and evaluation interleaved, one action per
  step); `:161-203` (`removable`, the deterministic statement-order
  scan). The elimination conditions: `:214-229`
  (`join_covers_full_key`, condition 1 — including
  `shared_vars_pair_positions_only`, `:221-227`), `:241-276`
  (`target_otherwise_unused`, condition 2 — Eq-constant filters within
  ψ, φ carried literally, non-Y fields dead), `:281-299`
  (`variables_join_or_dead`, condition 3), `:168`
  (`Enforcement::ScalarProbe`, condition 4 — the interval refusal),
  `:315-373` (`var_is_dead`). Chains and support: the module doc at
  `:66-81` and `chain_reaches` — an ELIMINATED occurrence may source a
  later elimination so long as its support chain avoids the candidate
  (`ChainedElimStep`, `chained_elimination_sound` below).
* **The grounding evaluator** —
  `crates/bumbledb/src/plan/ground/evaluate.rs:109-125` (`fold_step`,
  first foldable occurrence per call); `:128-199` (`fold_positive`:
  survivors, the membership attachment, the rule-death channel at
  `:179-190`); `:379-485` (`parse_resolvable` — params, pending
  interns, and measures refuse); `:576-590` (`surviving_ids`, the
  prepare-time σ over sealed rows); `:203-273` (`fold_negated`, the
  complement fold — unmodeled here, see the narrowings).
* **The key-probe lowering** —
  `crates/bumbledb/src/exec/dispatch/classify.rs:24-130` (`classify`:
  exactly one positive occurrence, no residuals, no measure or
  set-valued filters, by-value Eq bindings covering a declared key;
  non-key per-field filters — order compares, `Ne`, `FieldsCompare`,
  resolved `PointIn`/Allen — pass into `remaining_filters`, `:127`,
  `:165-188`, applied post-get,
  `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs:253`);
  `:134-161` (`key_probe_candidate`: the key statement — pointwise
  interval-final keys included — or the full-fact `M` fallback);
  `crates/bumbledb/src/api/prepared/build.rs:335`
  (`PreparedRule::KeyProbe` minted).
* **The subsumption sweep** — `plan/ground.rs::subsume` (classical
  UCQ minimization restricted to the normalized-form witness;
  `plan/ground.rs::subsumes`, the ordered-pair check), wired after
  grounding at `api/prepared/build.rs::ground_program`: the deleted
  rules are filtered out of the prepared program. Unmodeled — see the
  narrowings.
* **The statically-empty fold** —
  `crates/bumbledb/src/ir/normalize/fold.rs:78-96` (`fold`:
  participating occurrences only — a negated occurrence's contradiction
  is NOT emptiness); `:152-214` (the contradiction rules (a)-(f), every
  one judged on constants only).
* **The literal latch** —
  `crates/bumbledb/src/api/prepared/bind.rs:280-340`
  (`resolve_filters`: `Ok(false)` = a dictionary miss or empty set
  under an `Eq` filter of a POSITIVE occurrence — per-execution, never
  a plan verdict); `:348-365` (`resolve_selection_into`: the
  `PendingIntern` hit rewrites the template once; the miss
  short-circuits THIS execution only).
* **Repeated variables** —
  `crates/bumbledb/src/ir/normalize/normalize.rs:132-187` (`lower_atom`
  pass 1: the first domain binding per variable; later positions lower
  to `FieldsCompare` filters, which condition 2 refuses — the
  `var_functional` premise's anchor).

## Narrowings recorded (law 5: narrow and record)

* **The fold is modeled as the FULL substitution.** `groundRewrite`
  replaces a closed atom by the disjunction, over the σ-surviving
  sealed facts, of its bindings' equalities (`groundCondition`) — the
  `Const::WordSet` membership generalized. Rust's foldability
  conditions (at most one live variable, at the id position, payload
  dead — `evaluate.rs:20-58`) are plan-shape mechanism: its algebra
  attaches single-field memberships to siblings, so payload must be
  dead; the substitution carries payload constraints wholesale and
  needs no deadness premise. `Atom.foldableB` (vars and literals only)
  mirrors `parse_resolvable`'s refusals as the modeled acceptance;
  preservation never spends it — the param refusal is stage discipline
  (prepared plans carry resolved constants), not semantics.
* **The negated complement fold is unmodeled** (`fold_negated`,
  `evaluate.rs:203-273`): the complement rewrite needs the domain
  guarantee (`domain_within_ids`) and a negated membership the
  condition grammar cannot write; the modeled step grounds positive
  occurrences only.
* **Rule subsumption is unmodeled** (`plan/ground.rs::subsume`, wired
  at `api/prepared/build.rs::ground_program`): after grounding, a rule
  whose normalized body contains a sibling's — identical `finds`,
  identical participating-atom multiset with the keeper's per-atom
  filters ⊆ the candidate's, the keeper's residual sets ⊆ the
  candidate's, and every keeper negated atom present in the candidate
  — is DELETED from the prepared program: a sixth denotation-affecting
  rewrite `RewriteStep` does not name, so `rewrite_composition`'s
  chain never represents it. The witness implies candidate-answers ⊆
  keeper-answers under both the projection and head-projection union
  regimes (every conjunct is either shared verbatim or only shrinks
  the candidate's binding set), and deleting a rule whose answers a
  kept rule covers preserves the union — hand-verified, NOT proved;
  the general-form refusal (CQ-homomorphism minimization, NP-hard) is
  the engine's own recorded fence (`plan/ground.rs`'s module doc). The
  ground-off dual-pipeline differential (the fuzz rewrites target) and
  the prepare-level instruments
  (`the_dnf_residue_subsumes_the_filtered_rule`,
  `crates/bumbledb/src/plan/ground/tests.rs`;
  `crates/bumbledb/src/api/prepared/tests/ground.rs`) stand as the
  empirical check until a `subsume` step constructor and its
  rule-containment lemma land — the recorded narrowing, and the one
  live rewrite whose soundness rests on record rather than theorem.
* **`elimination_sound` is the projection-sink face.** For
  set-semantics `ruleAnswers`, containment existence plus deadness
  suffice; the key-ness of Y (condition 1's full-key demand,
  acceptance-side) is what the AGGREGATE face spends — key-ness keeps
  a dead non-key variable from multiplying the binding-set fold domain
  (`ground.rs:44-51`). The aggregate face lives in `Exec/Dedup.lean`
  where the fold domains live, as THREE named pieces (2026-07-14,
  the admission-calculus docket): `elim_extension_exists` (the
  containment's extension, binding-level) and
  `elim_extension_unique` (the target key pins it — key-ness and
  full-key coverage enter there as the `Functionality` and
  zip-coverage hypotheses) compose into
  `elimination_agg_fold_domain` (the distinct-full-binding domain of
  each group fiber projects bijectively onto the surviving slots),
  spent by `elimination_agg_domain_counts` (the key's payoff: the
  engine's full-slot-array domain and the surviving-slot domain
  carry the same counts) beside `elimination_agg_sound` (answer
  identity fiber for fiber, both folds at the surviving-slot
  reading — the containment's payoff, no key premise). The recorded
  scope is Dedup's module doc: no single abstract-fold statement
  spans both domains (different tuple widths), so "discharged" means
  exactly this pair, each claim one theorem. Here, only the
  projection face's full-key premise enters, through the join-value
  transfer.
* **`var_functional`**: an eliminable occurrence binds each variable at
  one field — repeated variables keep their first binding and lower the
  rest to `FieldsCompare` filters (`normalize.rs:174-187`), which
  `selections_within_psi` refuses (`ground.rs:252-258`). Without it the
  witness fact could disagree with itself (two dead fields, one
  variable), so the premise is load-bearing, not decorative.
* **The full-fact `M` path is unmodeled** (`classify.rs:153-160`):
  model facts carry junk fields beyond the arity (the PRD 03
  narrowing), so whole-fact identity is a storage fact below this
  level; `KeyProbeShape` models the statement-key arm. The
  closed-relation refusal (`classify.rs:94`) and the measure/set-filter
  refusals (`classify.rs:44-75`) are dispatch mechanism. Residual
  per-field filters are MODELED: the rule's conditions ride the probe
  and filter the at-most-one hit (`keyProbeEval`'s condition check =
  `remaining_filters` applied post-get) — the model's conditions are
  the general shape, the engine's per-field filters its instances.
* **Chained-source eliminations are modeled one link deep.**
  `chained_elimination_sound` composes a support pair — an ordinary
  elimination plus one elimination whose pairing source it discharged
  — into one answer-preserving step, with the acyclic-support premise
  (`hroot`: the chain roots in a surviving occurrence — exactly what
  `chain_reaches` forbids the negation of) and the engine's
  `shared_vars_pair_positions_only` conjunct (`shared_join`) named;
  `RewriteStep.eliminateChained` makes the pair an instance of the
  step relation. Deeper support forests (a discharged source whose own
  source is discharged, or cross-links through other discharged
  readers) iterate the same existence/uniqueness argument from the
  roots (`ground.rs:66-81`) but are not stated — the recorded
  narrowing. The narrowing also covers the SOURCE-ROLE roster: the
  engine's `removable` excludes only `Role::Negated` as a pairing
  source (`ground.rs:178`), so a `Role::Folded` occurrence is
  admissible where the modeled chain step names `Eliminated` sources
  only. The Folded case is practically unreachable — a folded
  source's binders all carry the attached `WordSet` membership
  filter, which condition 2's `selections_within_psi` refuses
  (Eq-to-ψ-literal filters only, `ground.rs:252-258`) — recorded, not
  proved.
* **`StaticallyEmpty` is the kill rule's soundness spec**: a condition
  refuted under EVERY environment. The Rust detector judges constants
  only (`fold.rs`, rules (a)-(f)); its completeness is not claimed.
  The "every verdict is an instance" claim is read through the
  FILTERS-TO-CONDITIONS mapping: rules (b)/(d) kill on
  occurrence-FILTER contradictions (`fold.rs:247-258`), and an
  occurrence's Eq filter is this model's equality-condition leaf (the
  shape `groundCondition` writes), so an Eq-conflict kill refutes the
  corresponding lifted condition — honest emptiness either way; the
  claim does NOT say every kill site literally inspects
  `Rule.conditions`.
-/

namespace Bumbledb.Query

/-! ## Agreement with the ground axioms

The instances item 1 quantifies over: those whose closed relations
carry exactly their sealed extensions — the committed states
`Theory.den` describes, met by construction on every readable snapshot
(`den_closed_constant` is why the rewrite may run at prepare). -/

/-- The instance carries the theory's ground axioms: every closed
relation's extension is the sealed fact list — membership for
membership. -/
def AgreesWithAxioms (T : Theory) (I : Instance) : Prop :=
  ∀ R ext, T.closed R = some ext → ∀ f, f ∈ I R ↔ f ∈ ext.facts

/-- On an agreeing instance the theory's denotation IS the instance:
`Theory.den` reads the sealed constant exactly where the instance
carries it. -/
theorem den_agrees {T : Theory} {I : Instance}
    (hax : AgreesWithAxioms T I) (R : RelId) :
    ∀ f, f ∈ T.den I R ↔ f ∈ I R := by
  intro f
  unfold Theory.den
  cases hcl : T.closed R with
  | none => exact Iff.rfl
  | some ext => exact (hax R ext hcl f).symm

/-- Containment transfers along membership-equivalent fact sets — how
`holds` (stated over `Theory.den`) is spent at the instance the
executor reads. -/
theorem containment_transfer {A A' B B' : Set Fact} {φ ψ : Selection}
    {X Y : List FieldId}
    (hA : ∀ f, f ∈ A ↔ f ∈ A') (hB : ∀ f, f ∈ B ↔ f ∈ B')
    (h : Containment A φ X B ψ Y) : Containment A' φ X B' ψ Y := by
  intro f hf hφ
  obtain ⟨g, hg, hψ, hp⟩ := h f ((hA f).mpr hf) hφ
  exact ⟨g, (hB g).mp hg, hψ, hp⟩

/-! ## `groundAtom` — substitution against ground axioms

Grounding's essence: a sealed atom's join step is a fold over the
FINITE extension, not over the instance — `bindAtom` against the
sealed rows, exactly the `evalList` join stage with the fact source
swapped for the constant. -/

/-- The satisfying extension of a closed atom under the current
partial assignment: every sealed fact's `bindAtom` extension.
Bridge: `plan/ground/evaluate.rs::surviving_ids` is this fold's
prepare-time constant half (the σ over sealed rows); the attached
membership is its per-binding residue. -/
def groundAtom (ρ : ParamEnv) (a : Atom) (ext : GroundExtension)
    (σ : PartialAssign) : List PartialAssign :=
  ext.facts.filterMap fun f => bindAtom ρ f a.bindings σ

/-- Membership in the grounded extension, unfolded. -/
theorem mem_groundAtom {ρ : ParamEnv} {a : Atom} {ext : GroundExtension}
    {σ σ' : PartialAssign} :
    σ' ∈ groundAtom ρ a ext σ ↔
      ∃ f, f ∈ ext.facts ∧ bindAtom ρ f a.bindings σ = some σ' :=
  List.mem_filterMap

/-- The grounded step IS the join step: over a world whose fact list
for the closed relation is the sealed extension, `evalList`'s per-atom
join stage computes exactly `groundAtom` — partial evaluation moves
the fold to prepare, it does not change it. -/
theorem groundAtom_join_step {W : ListInstance} {ρ : ParamEnv}
    {a : Atom} {ext : GroundExtension}
    (h : W.facts a.relation = ext.facts) (σs : List PartialAssign) :
    joinAtoms W ρ [a] σs = σs.flatMap (groundAtom ρ a ext) := by
  simp only [joinAtoms, h]
  rfl

/-! ## `groundRewrite` — fold to a finite contribution, or refute

The pass, one step (the Rust loop applies steps to a fixpoint —
`fold_step` is "one action per call"; composition is
`rewrite_composition`): find the first foldable closed atom, run σ
over its sealed rows, and either fold the atom into its finite
constant contribution (a disjunction of binding equalities over the
survivors — the membership set, generalized) or refute the rule
outright (`folded to ∅`). A vacuous fold (no bindings, survivors
nonempty) yields an always-true condition — the "satisfaction proved"
deletion arm, subsumed. -/

/-- The modeled foldability screen: bindings are variables and
literals only. Mirrors `parse_resolvable`'s refusals — params and
pending interns are stage-3 values a stage-2 pass must not judge,
measures raise per execution (`evaluate.rs:379-485`). Recorded: the
preservation theorem never spends this screen (the narrowing note). -/
def Atom.foldableB (a : Atom) : Bool :=
  a.bindings.all fun bd =>
    match bd with
    | (_, Term.var _) => true
    | (_, Term.lit _) => true
    | _ => false

/-- The prepare-time σ, one row: every literal binding agrees with the
sealed fact. Non-literal bindings pass — they are the residue the
grounded condition carries. Bridge: `surviving_ids`' row filter. -/
def litSatB (b : Atom) (f : Fact) : Bool :=
  b.bindings.all fun bd =>
    match bd with
    | (i, Term.lit c) => decide (c = f i)
    | _ => true

/-- A matching fact survives the literal screen: the screen checks a
subset of what the matching equation demands. -/
theorem litSat_of_matches {ρ : ParamEnv} {σ : Assignment} {b : Atom}
    {f : Fact} (h : Matches f b σ ρ) : litSatB b f = true := by
  refine List.all_eq_true.mpr fun bd hbd => ?_
  obtain ⟨i, t⟩ := bd
  have hs := h (i, t) hbd
  cases t with
  | lit c => exact decide_eq_true hs
  | var v => rfl
  | param p => rfl
  | paramSet p => rfl
  | measure v => rfl

/-- The folded contribution: the closed atom's matching equation as a
finite condition — one disjunct per surviving sealed fact, one
equality leaf per binding. `Const::WordSet` is the one-live-variable
projection of this shape; the empty survivor list gives `or []`,
the refutation. -/
def groundCondition (b : Atom) (facts : List Fact) : Condition :=
  .or (facts.map fun f =>
    .and (b.bindings.map fun bd => .leaf ⟨.eq, bd.2, .lit (f bd.1)⟩))

/-- An equality against a literal is exactly the term's selection —
the leaf the substitution writes means what the binding meant. -/
theorem holds_eq_lit {C : Classify} {ρ : ParamEnv} {σ : Assignment}
    {t : Term} {w : Value} :
    Comparison.holds C ρ σ ⟨.eq, t, .lit w⟩ ↔ Term.selects ρ σ t w := by
  constructor
  · rintro ⟨x, y, hx, hy, hxy⟩
    have hwy : w = y := hy
    have hxy' : x = y := hxy
    cases hwy
    cases hxy'
    exact hx
  · intro h
    exact ⟨w, w, h, rfl, rfl⟩

/-- The grounded condition holds exactly when some listed fact matches
the atom — substitution against ground axioms preserves the matching
equation, term kind by term kind (`holds_eq_lit` carries every
`Term.selects` arm, params and sets included). -/
theorem groundCondition_holds {C : Classify} {ρ : ParamEnv}
    {σ : Assignment} {b : Atom} {facts : List Fact} :
    Condition.holds C ρ σ (groundCondition b facts) ↔
      ∃ f, f ∈ facts ∧ Matches f b σ ρ := by
  simp only [groundCondition, Condition.holds]
  rw [Condition.anyHold_iff]
  constructor
  · rintro ⟨c, hc, hh⟩
    obtain ⟨f, hf, rfl⟩ := List.mem_map.mp hc
    refine ⟨f, hf, fun bd hbd => ?_⟩
    simp only [Condition.holds] at hh
    rw [Condition.allHold_iff] at hh
    have := hh _ (List.mem_map.mpr ⟨bd, hbd, rfl⟩)
    simp only [Condition.holds] at this
    exact holds_eq_lit.mp this
  · rintro ⟨f, hf, hm⟩
    refine ⟨_, List.mem_map.mpr ⟨f, hf, rfl⟩, ?_⟩
    simp only [Condition.holds]
    rw [Condition.allHold_iff]
    intro c hc
    obtain ⟨bd, hbd, rfl⟩ := List.mem_map.mp hc
    simp only [Condition.holds]
    exact holds_eq_lit.mpr (hm bd hbd)

/-- The refutation verdict: the rule whose closed atom's prepare-time
σ survived nothing. Bridge: `NormalizedQuery::dead` (`folded to ∅:
…`), deleted at prepare into `Program::Empty`. -/
structure Grounded where
  /-- The refuted rule. -/
  rule : Rule
  /-- The closed atom whose sealed σ-subset is empty. -/
  atom : Atom

/-- The scan: the first foldable closed atom, with its extension and
the remaining atoms — `fold_step`'s first-occurrence discipline
(`evaluate.rs:109-125`). -/
def groundSplit (T : Theory) :
    List Atom → Option (Atom × GroundExtension × List Atom)
  | [] => none
  | a :: rest =>
    match T.closed a.relation, a.foldableB with
    | some ext, true => some (a, ext, rest)
    | _, _ =>
      match groundSplit T rest with
      | some (b, e, l) => some (b, e, a :: l)
      | none => none

/-- What a successful scan pins: the found atom is closed with the
returned extension, and the input atoms are the found atom plus the
rest, membership for membership. -/
theorem groundSplit_spec (T : Theory) :
    ∀ (atoms : List Atom) (b : Atom) (ext : GroundExtension)
      (rest : List Atom), groundSplit T atoms = some (b, ext, rest) →
      T.closed b.relation = some ext ∧
        ∀ x, x ∈ atoms ↔ x = b ∨ x ∈ rest
  | [], _, _, _, h => by cases h
  | a :: atoms, b, ext, rest, h => by
    have recurse : ∀ l' : List Atom,
        groundSplit T atoms = some (b, ext, l') → rest = a :: l' →
        T.closed b.relation = some ext ∧
          ∀ x, x ∈ a :: atoms ↔ x = b ∨ x ∈ rest := by
      rintro l' heq' rfl
      obtain ⟨hc, hmem⟩ := groundSplit_spec T atoms b ext l' heq'
      refine ⟨hc, fun x => ?_⟩
      constructor
      · intro hx
        rcases List.mem_cons.mp hx with rfl | hx'
        · exact Or.inr (List.mem_cons_self ..)
        · rcases (hmem x).mp hx' with rfl | hx''
          · exact Or.inl rfl
          · exact Or.inr (List.mem_cons_of_mem _ hx'')
      · rintro (rfl | hx)
        · exact List.mem_cons_of_mem _ ((hmem x).mpr (Or.inl rfl))
        · rcases List.mem_cons.mp hx with rfl | hx'
          · exact List.mem_cons_self ..
          · exact List.mem_cons_of_mem _ ((hmem x).mpr (Or.inr hx'))
    cases hc : T.closed a.relation with
    | some ext' =>
      cases hf : a.foldableB with
      | true =>
        simp only [groundSplit, hc, hf, Option.some.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        exact ⟨hc, fun x => List.mem_cons⟩
      | false =>
        cases hg : groundSplit T atoms with
        | none =>
          simp only [groundSplit, hc, hf, hg] at h
          exact nomatch h
        | some x =>
          obtain ⟨b', e', l'⟩ := x
          simp only [groundSplit, hc, hf, hg, Option.some.injEq] at h
          obtain ⟨rfl, rfl, rfl⟩ := h
          exact recurse l' hg rfl
    | none =>
      cases hg : groundSplit T atoms with
      | none =>
        simp only [groundSplit, hc, hg] at h
        exact nomatch h
      | some x =>
        obtain ⟨b', e', l'⟩ := x
        simp only [groundSplit, hc, hg, Option.some.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        exact recurse l' hg rfl

/-- **The grounding pass, one step** — `Rule ⊕ Grounded`: the first
foldable closed atom either folds to its finite constant contribution
(the surviving-fact disjunction replaces the atom) or refutes the rule
(no sealed row survives σ — the `folded to ∅` verdict). No foldable
closed atom: the rule passes through. Bridge: `plan/ground/evaluate.rs::
fold_positive` (`Role::Folded`, the membership attachment, the
rule-death channel). -/
def groundRewrite (T : Theory) (r : Rule) : Rule ⊕ Grounded :=
  match groundSplit T r.atoms with
  | none => .inl r
  | some (b, ext, rest) =>
    match ext.facts.filter (litSatB b) with
    | [] => .inr ⟨r, b⟩
    | f :: fs =>
      .inl { finds := r.finds, atoms := rest, negated := r.negated,
             conditions := groundCondition b (f :: fs) :: r.conditions }

/-- The answers a ground outcome denotes: a rewritten rule's answers,
or the empty set for a refutation (`Program::Empty` — a dead rule is
deleted at prepare and plans nothing). -/
def groundAnswers (C : Classify) (o : Rule ⊕ Grounded) (I : Instance)
    (ρ : ParamEnv) : Set AnswerTuple :=
  match o with
  | .inl r => ruleAnswers C r I ρ
  | .inr _ => fun _ => False

/-- **Item 1 — `grounding_preserves_answers`.** On every instance
agreeing with the theory's ground axioms, the grounding step preserves
the rule's answers: the folded contribution means exactly what the
closed atom meant (closed extensions are instance-invariant — 03's
sealed constants, `den_closed_constant`), and the refutation verdict
is honest emptiness. Bridge: `plan/ground`, the `ground-off` dual
pipeline (the rewrites fuzz target's empirical arm of this exact
statement). -/
theorem grounding_preserves_answers {T : Theory} (C : Classify)
    (r : Rule) {I : Instance} (hax : AgreesWithAxioms T I)
    (ρ : ParamEnv) :
    ∀ t, t ∈ groundAnswers C (groundRewrite T r) I ρ ↔
      t ∈ ruleAnswers C r I ρ := by
  intro t
  unfold groundRewrite
  split
  · exact Iff.rfl
  · rename_i b ext rest hsp
    obtain ⟨hclosed, hmem⟩ := groundSplit_spec T r.atoms b ext rest hsp
    have hag : ∀ f, f ∈ I b.relation ↔ f ∈ ext.facts :=
      hax b.relation ext hclosed
    split
    · -- no survivor: the rule denotes nothing on any agreeing instance
      rename_i heq2
      simp only [groundAnswers]
      constructor
      · intro h
        exact h.elim
      · intro ht
        obtain ⟨σ, ⟨hatoms, -, -⟩, -⟩ := mem_ruleAnswers.mp ht
        obtain ⟨f, hf, hm⟩ := hatoms b ((hmem b).mpr (Or.inl rfl))
        have : f ∈ ext.facts.filter (litSatB b) :=
          List.mem_filter.mpr ⟨(hag f).mp hf, litSat_of_matches hm⟩
        rw [heq2] at this
        cases this
    · -- survivors: the folded condition is the atom, exactly
      rename_i f₀ fs heq2
      simp only [groundAnswers]
      have hcond : ∀ σ : Assignment,
          Condition.holds C ρ σ (groundCondition b (f₀ :: fs)) ↔
            ∃ f, f ∈ I b.relation ∧ Matches f b σ ρ := by
        intro σ
        rw [← heq2, groundCondition_holds]
        constructor
        · rintro ⟨f, hf, hm⟩
          exact ⟨f, (hag f).mpr (List.mem_filter.mp hf).1, hm⟩
        · rintro ⟨f, hf, hm⟩
          exact ⟨f, List.mem_filter.mpr
            ⟨(hag f).mp hf, litSat_of_matches hm⟩, hm⟩
      constructor
      · intro ht
        obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
        refine mem_ruleAnswers.mpr ⟨σ, ⟨?_, hneg, ?_⟩, rfl⟩
        · intro a ha
          rcases (hmem a).mp ha with rfl | ha'
          · exact (hcond σ).mp (hconds _ (List.mem_cons_self ..))
          · exact hatoms a ha'
        · exact fun c hc => hconds c (List.mem_cons_of_mem _ hc)
      · intro ht
        obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
        refine mem_ruleAnswers.mpr
          ⟨σ, ⟨fun a ha => hatoms a ((hmem a).mpr (Or.inr ha)), hneg,
            ?_⟩, rfl⟩
        intro c hc
        rcases List.mem_cons.mp hc with rfl | hc'
        · exact (hcond σ).mpr (hatoms b ((hmem b).mpr (Or.inl rfl)))
        · exact hconds c hc'

/-- The refutation verdict spent: a rule the grounding refuted answers
NOTHING on any agreeing instance — the `Program::Empty` face of
item 1, and the fold-death channel's soundness. -/
theorem ground_refuted_empty {T : Theory} {C : Classify} {r : Rule}
    {g : Grounded} (h : groundRewrite T r = .inr g) {I : Instance}
    (hax : AgreesWithAxioms T I) (ρ : ParamEnv) :
    ∀ t, t ∉ ruleAnswers C r I ρ := by
  intro t ht
  have := (grounding_preserves_answers C r hax ρ t).mpr ht
  rw [h] at this
  exact this

/-! ## Item 2 — elimination

The containment-implied atom drop, `Role::Eliminated(statement)`: an
accepted containment `A(X | φ) <= B(Y | ψ)` makes the query's join of
`A` to `B` on X→Y redundant when `B` contributes nothing else. The
shape below is `plan/ground.rs`'s conditions 1–3, definition for
definition; condition 4 (the interval refusal, `ScalarProbe`) enters
at `RewriteStep.eliminate` as the scalar-split premises, where the
theory's judgment is cashed. -/

deriving instance DecidableEq for Term

/-- The elimination shape — `removable`'s conditions over one rule:
`r'` is `r` with one occurrence of the target atom `b` dropped, the
source atom `a` survives, the query joins `a` to `b` exactly on the
statement's X→Y position pairs, `a` carries φ literally, `b` carries
nothing beyond variables and ψ-literals, each variable of `b` binds
one field, and every variable of `b` either joins through a projection
pair or is dead in the surviving rule. -/
structure ElimStep (r r' : Rule) (a b : Atom) (X Y : List FieldId)
    (φ ψ : Selection) : Prop where
  /-- The drop: one `b` occurrence removed, everything else kept. -/
  atoms_split : ∃ pre post, r.atoms = pre ++ b :: post ∧
    r'.atoms = pre ++ post
  finds_eq : r'.finds = r.finds
  negated_eq : r'.negated = r.negated
  conditions_eq : r'.conditions = r.conditions
  /-- The pairing source survives the drop (`a_idx ≠ b_idx`). -/
  source : a ∈ r'.atoms
  /-- Condition 1 — every X→Y position pair is join-covered by one
  shared variable (`join_covers_full_key`, the covering half). -/
  join_covers : ∀ p, p ∈ X.zip Y →
    ∃ v, (p.1, Term.var v) ∈ a.bindings ∧ (p.2, Term.var v) ∈ b.bindings
  /-- Condition 2 — the source occurrence carries φ literally
  (`source_carries_phi`): each φ binding's field is pinned by the
  atom to a MEMBER of the binding's literal set — a literal pin is at
  least as strong as the disjunction it discharges (singleton sets
  are the old exact-literal containment), never inference. -/
  carries_phi : ∀ s, s ∈ φ.bindings →
    ∃ c, c ∈ s.2 ∧ (s.1, Term.lit c) ∈ a.bindings
  /-- Condition 2 — the target carries only variables and
  ψ-SINGLETON literals (`selections_within_psi`: any other filter
  shape fails the containment; a non-singleton ψ binding pins no
  single witness value, so the shape refuses it). -/
  target_bindings : ∀ bd, bd ∈ b.bindings →
    (∃ v, bd.2 = Term.var v) ∨
      (∃ c, bd.2 = Term.lit c ∧ (bd.1, [c]) ∈ ψ.bindings)
  /-- The one-field-per-variable discipline: repeated variables lower
  to `FieldsCompare` filters (`normalize.rs:174-187`), which condition
  2 refuses — so an eliminable occurrence binds each variable once. -/
  var_functional : ∀ i j v, (i, Term.var v) ∈ b.bindings →
    (j, Term.var v) ∈ b.bindings → i = j
  /-- Condition 3 — every variable of `b` joins through a projection
  pair or is dead in the surviving rule (`variables_join_or_dead` +
  `var_is_dead`: dead = read by no find, atom, negated atom, or
  condition that survives). -/
  join_or_dead : ∀ i v, (i, Term.var v) ∈ b.bindings →
    (∃ p, p ∈ X.zip Y ∧ p.2 = i ∧ (p.1, Term.var v) ∈ a.bindings) ∨
      v ∉ r'.allVars

/-- Positionally equal projections agree on every zip pair — how the
containment witness's Y values transfer to the source's X values. -/
theorem project_eq_zip {f g : Fact} :
    ∀ {X Y : List FieldId}, f.project X = g.project Y →
      ∀ p, p ∈ X.zip Y → f p.1 = g p.2
  | [], _, _, p, hp => by cases hp
  | _ :: _, [], _, p, hp => by cases hp
  | x :: X, y :: Y, h, p, hp => by
    have hcons : f x :: f.project X = g y :: g.project Y := h
    injection hcons with h1 h2
    rcases List.mem_cons.mp hp with rfl | hp'
    · exact h1
    · exact project_eq_zip h2 p hp'

/-- The witness-extension assignment: live variables (anything the
surviving rule reads) keep σ's value; a dead variable bound in the
dropped atom takes the witness fact's field value. -/
def elimAssign (σ : Assignment) (r' : Rule) (b : Atom) (g : Fact) :
    Assignment := fun v =>
  if v ∈ r'.allVars then σ v
  else
    match b.bindings.find? (fun bd => decide (bd.2 = Term.var v)) with
    | some bd => g bd.1
    | none => σ v

/-- Live variables keep σ's value. -/
theorem elimAssign_live {σ : Assignment} {r' : Rule} {b : Atom}
    {g : Fact} {v : VarId} (h : v ∈ r'.allVars) :
    elimAssign σ r' b g v = σ v := by
  unfold elimAssign
  rw [if_pos h]

/-- A dead variable bound at field `i` of the dropped atom reads the
witness's value there — single-valued by `var_functional`. -/
theorem elimAssign_dead {σ : Assignment} {r' : Rule} {b : Atom}
    {g : Fact} {v : VarId} {i : FieldId} (hdead : v ∉ r'.allVars)
    (hvf : ∀ i j v', (i, Term.var v') ∈ b.bindings →
      (j, Term.var v') ∈ b.bindings → i = j)
    (hb : (i, Term.var v) ∈ b.bindings) :
    elimAssign σ r' b g v = g i := by
  unfold elimAssign
  rw [if_neg hdead]
  cases hfind : b.bindings.find? (fun bd => decide (bd.2 = Term.var v)) with
  | none =>
    have := List.find?_eq_none.mp hfind (i, Term.var v) hb
    simp at this
  | some bd =>
    have hpred := List.find?_some hfind
    have hbd2 : bd.2 = Term.var v := by
      simpa using hpred
    have hmem : (bd.1, Term.var v) ∈ b.bindings := by
      rw [← hbd2]
      exact List.mem_of_find?_eq_some hfind
    show g bd.1 = g i
    rw [hvf bd.1 i v hmem hb]

/-- **Item 2 — `elimination_sound`.** Under the elimination shape and
the theory's containment premise, dropping the target atom preserves
the rule's answers: existence rides the containment (the source fact
carries φ, so a ψ-satisfying witness with the same projected tuple
exists), the join pairs transfer the witness's Y values to the σ the
surviving rule already derives with, and the dead variables — read by
nothing that survives — are reassigned to the witness's fields.
Bridge: `Role::Eliminated(statement)`; the elimination differential
(`with_grounding_disabled`) is the empirical arm. -/
theorem elimination_sound {C : Classify} {I : Instance} {ρ : ParamEnv}
    {r r' : Rule} {a b : Atom} {X Y : List FieldId} {φ ψ : Selection}
    (hs : ElimStep r r' a b X Y φ ψ)
    (hcont : Containment (I a.relation) φ X (I b.relation) ψ Y) :
    ∀ t, t ∈ ruleAnswers C r' I ρ ↔ t ∈ ruleAnswers C r I ρ := by
  obtain ⟨pre, post, hr, hr'⟩ := hs.atoms_split
  have hsub : ∀ x : Atom, x ∈ r'.atoms → x ∈ r.atoms := by
    intro x hx
    rw [hr'] at hx
    rw [hr]
    rcases List.mem_append.mp hx with h | h
    · exact List.mem_append.mpr (Or.inl h)
    · exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ h))
  intro t
  constructor
  · -- the dropped rule's answers are the original's
    intro ht
    obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
    -- the source fact and its containment witness
    obtain ⟨fa, hfa, hma⟩ := hatoms a hs.source
    have hφ : φ.satisfies fa := by
      intro s hsmem
      obtain ⟨c, hcmem, hca⟩ := hs.carries_phi s hsmem
      exact (hma _ hca) ▸ hcmem
    obtain ⟨g, hg, hψ, hgproj⟩ := hcont fa hfa hφ
    -- the extension: live variables keep σ, dead ones read the witness
    have hlive : ∀ v, v ∈ r'.allVars → σ v = elimAssign σ r' b g v :=
      fun v hv => (elimAssign_live hv).symm
    -- the witness matches the dropped atom under the extension
    have hmb : Matches g b (elimAssign σ r' b g) ρ := by
      intro bd hbd
      rcases hs.target_bindings bd hbd with ⟨v, hv⟩ | ⟨c, hc, hcψ⟩
      · rw [hv]
        show elimAssign σ r' b g v = g bd.1
        have hvb : (bd.1, Term.var v) ∈ b.bindings := by
          rw [← hv]
          exact hbd
        rcases hs.join_or_dead bd.1 v hvb with ⟨p, hp, hpi, hpa⟩ | hdead
        · -- the join pair: σ' v = σ v = fa p.1 = g p.2 = g bd.1
          have hvlive : v ∈ r'.allVars := by
            refine mem_allVars.mpr (Or.inr (Or.inl ⟨a, hs.source, ?_⟩))
            exact List.mem_flatMap.mpr
              ⟨(p.1, Term.var v), hpa, by simp [Term.vars]⟩
          have h1 : elimAssign σ r' b g v = σ v := (hlive v hvlive).symm
          have h2 : σ v = fa p.1 := hma _ hpa
          have h3 : fa p.1 = g p.2 := project_eq_zip hgproj.symm p hp
          rw [h1, h2, h3, hpi]
        · exact elimAssign_dead hdead hs.var_functional hvb
      · rw [hc]
        show c = g bd.1
        exact (Selection.satisfies_singleton hψ hcψ).symm
    -- the original rule derives under the extension
    refine mem_ruleAnswers.mpr ⟨elimAssign σ r' b g, ⟨?_, ?_, ?_⟩, ?_⟩
    · intro x hx
      rw [hr] at hx
      rcases List.mem_append.mp hx with hx' | hx'
      · -- a survivor from `pre`
        have hxr' : x ∈ r'.atoms := by
          rw [hr']
          exact List.mem_append.mpr (Or.inl hx')
        obtain ⟨f, hf, hmf⟩ := hatoms x hxr'
        refine ⟨f, hf, (matches_congr fun v hv => ?_).mp hmf⟩
        exact hlive v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, hxr', hv⟩)))
      · rcases List.mem_cons.mp hx' with rfl | hx''
        · exact ⟨g, hg, hmb⟩
        · -- a survivor from `post`
          have hxr' : x ∈ r'.atoms := by
            rw [hr']
            exact List.mem_append.mpr (Or.inr hx'')
          obtain ⟨f, hf, hmf⟩ := hatoms x hxr'
          refine ⟨f, hf, (matches_congr fun v hv => ?_).mp hmf⟩
          exact hlive v (mem_allVars.mpr (Or.inr (Or.inl ⟨x, hxr', hv⟩)))
    · intro n hn
      have hn' : n ∈ r'.negated := hs.negated_eq ▸ hn
      rintro ⟨f, hf, hmf⟩
      refine hneg n hn' ⟨f, hf, (matches_congr fun v hv => ?_).mpr hmf⟩
      exact hlive v
        (mem_allVars.mpr (Or.inr (Or.inr (Or.inl ⟨n, hn', hv⟩))))
    · intro c hc
      have hc' : c ∈ r'.conditions := hs.conditions_eq ▸ hc
      refine (condHolds_congr C ρ σ (elimAssign σ r' b g) c
        fun v hv => ?_).mp (hconds c hc')
      exact hlive v
        (mem_allVars.mpr (Or.inr (Or.inr (Or.inr ⟨c, hc', hv⟩))))
    · -- the projected tuple is unchanged: finds are live
      rw [← hs.finds_eq]
      refine List.map_congr_left fun v hv => ?_
      exact hlive v (mem_allVars.mpr (Or.inl hv))
  · -- dropping constraints only grows the derivation set
    intro ht
    obtain ⟨σ, ⟨hatoms, hneg, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
    refine mem_ruleAnswers.mpr ⟨σ, ⟨fun x hx => hatoms x (hsub x hx),
      fun n hn => hneg n (hs.negated_eq ▸ hn),
      fun c hc => hconds c (hs.conditions_eq ▸ hc)⟩, ?_⟩
    rw [hs.finds_eq]

/-! ### Chained eliminations — the discharged source

The engine deliberately lets an already-`Eliminated` occurrence serve
as the pairing source of a later elimination (`ground.rs:66-81`): the
discharged occurrence's witness fact still exists, uniquely, and
satisfies its whole filter list, so the next containment applies to
THE WITNESS instead of a live atom. The discipline is the support
forest — `chain_reaches` refuses any source whose support chain
passes through the candidate. The composed step below models one
chain link; its premises name exactly what the engine checks. -/

/-- Answers read atoms by MEMBERSHIP only (`derives` quantifies
`∈ r.atoms`), so two rules with the same finds, negated atoms and
conditions and membership-equal atom lists answer alike — the licence
to reorder atom lists when replaying the engine's mark-based steps
(`Role` marks never move occurrences; the model's lists may). -/
theorem ruleAnswers_atoms_congr {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r s : Rule}
    (hatoms : ∀ x : Atom, x ∈ r.atoms ↔ x ∈ s.atoms)
    (hfinds : r.finds = s.finds) (hneg : r.negated = s.negated)
    (hconds : r.conditions = s.conditions) :
    ∀ t, t ∈ ruleAnswers C r I ρ ↔ t ∈ ruleAnswers C s I ρ := by
  intro t
  constructor
  · intro ht
    obtain ⟨σ, ⟨ha, hn, hc⟩, rfl⟩ := mem_ruleAnswers.mp ht
    exact mem_ruleAnswers.mpr ⟨σ,
      ⟨fun x hx => ha x ((hatoms x).mpr hx),
       fun x hx => hn x (hneg ▸ hx),
       fun x hx => hc x (hconds ▸ hx)⟩, by rw [hfinds]⟩
  · intro ht
    obtain ⟨σ, ⟨ha, hn, hc⟩, rfl⟩ := mem_ruleAnswers.mp ht
    exact mem_ruleAnswers.mpr ⟨σ,
      ⟨fun x hx => ha x ((hatoms x).mp hx),
       fun x hx => hn x (hneg ▸ hx),
       fun x hx => hc x (hconds ▸ hx)⟩, by rw [← hfinds]⟩

/-- The chained elimination shape — `removable`'s conditions with a
DISCHARGED pairing source: `r'` is `r` with one occurrence of the
target atom `c` dropped, and the pairing source `b` need NOT be an
atom of `r` (it was itself eliminated earlier; its witness fact
stands in). The fields are `ElimStep`'s with two changes: the
`source` membership is gone — that is the point — and `shared_join`,
the engine's `shared_vars_pair_positions_only` conjunct
(`ground.rs:221-227`) that plain `ElimStep` never needed, is
demanded: with the source discharged, a variable shared with it can
reach the witness only by riding a projection pair. -/
structure ChainedElimStep (r r' : Rule) (b c : Atom)
    (X Y : List FieldId) (φ ψ : Selection) : Prop where
  /-- The drop: one `c` occurrence removed, everything else kept. -/
  atoms_split : ∃ pre post, r.atoms = pre ++ c :: post ∧
    r'.atoms = pre ++ post
  finds_eq : r'.finds = r.finds
  negated_eq : r'.negated = r.negated
  conditions_eq : r'.conditions = r.conditions
  /-- Condition 1 — every X→Y position pair is join-covered by one
  variable shared between the discharged source and the target. -/
  join_covers : ∀ p, p ∈ X.zip Y →
    ∃ v, (p.1, Term.var v) ∈ b.bindings ∧ (p.2, Term.var v) ∈ c.bindings
  /-- Condition 2 — the discharged source carries φ literally: its
  filter list is a subset of the ψ its witness satisfies, so a φ
  binding pinned there transfers to the witness. -/
  carries_phi : ∀ s, s ∈ φ.bindings →
    ∃ w, w ∈ s.2 ∧ (s.1, Term.lit w) ∈ b.bindings
  /-- Condition 2 — the target carries only variables and ψ-singleton
  literals (`selections_within_psi`). -/
  target_bindings : ∀ bd, bd ∈ c.bindings →
    (∃ v, bd.2 = Term.var v) ∨
      (∃ w, bd.2 = Term.lit w ∧ (bd.1, [w]) ∈ ψ.bindings)
  /-- The one-field-per-variable discipline on the target. -/
  var_functional : ∀ i j v, (i, Term.var v) ∈ c.bindings →
    (j, Term.var v) ∈ c.bindings → i = j
  /-- Condition 3 — every variable of `c` joins through a projection
  pair to the discharged source, or is dead in the surviving rule
  (deadness judged with the source already discharged, exactly
  `var_is_dead`'s skip of discharged occurrences). -/
  join_or_dead : ∀ i v, (i, Term.var v) ∈ c.bindings →
    (∃ p, p ∈ X.zip Y ∧ p.2 = i ∧ (p.1, Term.var v) ∈ b.bindings) ∨
      v ∉ r'.allVars
  /-- Condition 1's second half — `shared_vars_pair_positions_only`:
  every variable shared between source and target pairs an X position
  with its corresponding Y position. -/
  shared_join : ∀ i j v, (i, Term.var v) ∈ b.bindings →
    (j, Term.var v) ∈ c.bindings →
    ∃ p, p ∈ X.zip Y ∧ p.1 = i ∧ p.2 = j

/-- **The composed step — `chained_elimination_sound`.** A support
pair — the ordinary elimination of `b` (source `a`, live) composed
with the elimination of `c` whose pairing source is the DISCHARGED
`b` — preserves the rule's answers under both containments, provided
the support chain roots in a surviving occurrence (`hroot`, the
acyclic-support premise: `chain_reaches` refuses exactly the
elimination that would sink this root). Proof by replay in witness
order: with `shared_join` named, eliminating `c` FIRST (against the
still-live `b`) is an ordinary `ElimStep` — a target variable read
only by the discharged source is join-classified, not dead — and the
elimination of `b` then follows on the smaller rule; `ruleAnswers`
reads atom lists by membership, so the reordering is invisible to the
denotation. Bridge: `plan/ground.rs::chain_reaches` and the support
forest (`ground.rs:66-81`); deeper forests iterate this argument from
the roots (the recorded narrowing, module doc). -/
theorem chained_elimination_sound {C : Classify} {I : Instance}
    {ρ : ParamEnv} {r r₁ r₂ : Rule} {a b c : Atom}
    {X₁ Y₁ X₂ Y₂ : List FieldId} {φ₁ ψ₁ φ₂ ψ₂ : Selection}
    (hs₁ : ElimStep r r₁ a b X₁ Y₁ φ₁ ψ₁)
    (hs₂ : ChainedElimStep r₁ r₂ b c X₂ Y₂ φ₂ ψ₂)
    (hroot : a ∈ r₂.atoms)
    (hcont₁ : Containment (I a.relation) φ₁ X₁ (I b.relation) ψ₁ Y₁)
    (hcont₂ : Containment (I b.relation) φ₂ X₂ (I c.relation) ψ₂ Y₂) :
    ∀ t, t ∈ ruleAnswers C r₂ I ρ ↔ t ∈ ruleAnswers C r I ρ := by
  -- The replay rules are built inline from `r₂`'s own items so every
  -- shared field agrees definitionally: drop `c` first (source `b`
  -- live), then `b` (source `a`, the surviving root).
  -- membership bookkeeping for the two recorded splits
  obtain ⟨pre₁, post₁, hr₁, hr₁'⟩ := hs₁.atoms_split
  obtain ⟨pre₂, post₂, hr₂, hr₂'⟩ := hs₂.atoms_split
  have hmem_r : ∀ x : Atom, x ∈ r.atoms ↔ x = b ∨ x ∈ r₁.atoms := by
    intro x
    rw [hr₁, hr₁']
    simp only [List.mem_append, List.mem_cons]
    constructor
    · rintro (h | rfl | h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl rfl
      · exact Or.inr (Or.inr h)
    · rintro (rfl | h | h)
      · exact Or.inr (Or.inl rfl)
      · exact Or.inl h
      · exact Or.inr (Or.inr h)
  have hmem_r₁ : ∀ x : Atom, x ∈ r₁.atoms ↔ x = c ∨ x ∈ r₂.atoms := by
    intro x
    rw [hr₂, hr₂']
    simp only [List.mem_append, List.mem_cons]
    constructor
    · rintro (h | rfl | h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl rfl
      · exact Or.inr (Or.inr h)
    · rintro (rfl | h | h)
      · exact Or.inr (Or.inl rfl)
      · exact Or.inl h
      · exact Or.inr (Or.inr h)
  -- the shared item equalities, chained through both steps
  have hfinds₂ : r₂.finds = r.finds := hs₂.finds_eq.trans hs₁.finds_eq
  have hneg₂ : r₂.negated = r.negated :=
    hs₂.negated_eq.trans hs₁.negated_eq
  have hconds₂ : r₂.conditions = r.conditions :=
    hs₂.conditions_eq.trans hs₁.conditions_eq
  -- step A: eliminate `c` against the still-live `b`
  have stepA : ElimStep
      ⟨r₂.finds, c :: b :: r₂.atoms, r₂.negated, r₂.conditions⟩
      ⟨r₂.finds, b :: r₂.atoms, r₂.negated, r₂.conditions⟩
      b c X₂ Y₂ φ₂ ψ₂ :=
    { atoms_split := ⟨[], b :: r₂.atoms, rfl, rfl⟩
      finds_eq := rfl
      negated_eq := rfl
      conditions_eq := rfl
      source := List.mem_cons_self ..
      join_covers := hs₂.join_covers
      carries_phi := hs₂.carries_phi
      target_bindings := hs₂.target_bindings
      var_functional := hs₂.var_functional
      join_or_dead := by
        intro i v hv
        rcases hs₂.join_or_dead i v hv with hjoin | hdead
        · exact Or.inl hjoin
        · by_cases hvb : ∃ j, (j, Term.var v) ∈ b.bindings
          · -- shared with the source: `shared_join` classifies it
            obtain ⟨j, hj⟩ := hvb
            obtain ⟨p, hp, hp1, hp2⟩ := hs₂.shared_join j i v hj hv
            exact Or.inl ⟨p, hp, hp2, hp1 ▸ hj⟩
          · -- read by nothing live and not by `b`: dead in `rB` too
            refine Or.inr fun hmem => ?_
            rcases mem_allVars.mp hmem with h1 | ⟨x, hx, hvx⟩ |
              ⟨x, hx, hvx⟩ | ⟨t', ht', hvt⟩
            · exact hdead (mem_allVars.mpr (Or.inl h1))
            · rcases List.mem_cons.mp hx with rfl | hx'
              · -- the `b` occurrence: its bindings are vars and
                -- literals (step 1's condition 2), so `v` would be a
                -- var binding of `b` — refused by `hvb`
                obtain ⟨bd, hbd, hvbd⟩ := List.mem_flatMap.mp hvx
                rcases hs₁.target_bindings bd hbd with ⟨v', hv'⟩ |
                  ⟨w, hw, -⟩
                · rw [hv'] at hvbd
                  have hvv : v = v' := by
                    simpa [Term.vars] using hvbd
                  refine hvb ⟨bd.1, ?_⟩
                  rw [show Term.var v = bd.2 from by rw [hv', ← hvv]]
                  exact hbd
                · rw [hw] at hvbd
                  simp [Term.vars] at hvbd
              · exact hdead (mem_allVars.mpr
                  (Or.inr (Or.inl ⟨x, hx', hvx⟩)))
            · exact hdead (mem_allVars.mpr
                (Or.inr (Or.inr (Or.inl ⟨x, hx, hvx⟩))))
            · exact hdead (mem_allVars.mpr
                (Or.inr (Or.inr (Or.inr ⟨t', ht', hvt⟩)))) }
  -- step B: eliminate `b` against the surviving root `a`
  have stepB : ElimStep
      ⟨r₂.finds, b :: r₂.atoms, r₂.negated, r₂.conditions⟩
      r₂ a b X₁ Y₁ φ₁ ψ₁ :=
    { atoms_split := ⟨[], r₂.atoms, rfl, rfl⟩
      finds_eq := rfl
      negated_eq := rfl
      conditions_eq := rfl
      source := hroot
      join_covers := hs₁.join_covers
      carries_phi := hs₁.carries_phi
      target_bindings := hs₁.target_bindings
      var_functional := hs₁.var_functional
      join_or_dead := by
        intro i v hv
        rcases hs₁.join_or_dead i v hv with hjoin | hdead
        · exact Or.inl hjoin
        · -- dead in `r₁` (more atoms) ⇒ dead in `r₂`
          refine Or.inr fun hmem => ?_
          rcases mem_allVars.mp hmem with h1 | ⟨x, hx, hvx⟩ |
            ⟨x, hx, hvx⟩ | ⟨t', ht', hvt⟩
          · exact hdead (mem_allVars.mpr
              (Or.inl (hs₂.finds_eq ▸ h1)))
          · exact hdead (mem_allVars.mpr (Or.inr (Or.inl
              ⟨x, (hmem_r₁ x).mpr (Or.inr hx), hvx⟩)))
          · exact hdead (mem_allVars.mpr (Or.inr (Or.inr (Or.inl
              ⟨x, hs₂.negated_eq ▸ hx, hvx⟩))))
          · exact hdead (mem_allVars.mpr (Or.inr (Or.inr (Or.inr
              ⟨t', hs₂.conditions_eq ▸ ht', hvt⟩)))) }
  -- the chain: r₂ ↔ (b live) ↔ (c and b live) ↔ r
  intro t
  have hAr : t ∈ ruleAnswers C
      ⟨r₂.finds, c :: b :: r₂.atoms, r₂.negated, r₂.conditions⟩ I ρ ↔
      t ∈ ruleAnswers C r I ρ := by
    refine ruleAnswers_atoms_congr
      (r := ⟨r₂.finds, c :: b :: r₂.atoms, r₂.negated, r₂.conditions⟩)
      (s := r) (fun x => ?_) hfinds₂ hneg₂ hconds₂ t
    constructor
    · intro hx
      rcases List.mem_cons.mp hx with rfl | hx'
      · exact (hmem_r x).mpr (Or.inr ((hmem_r₁ x).mpr (Or.inl rfl)))
      rcases List.mem_cons.mp hx' with rfl | hx''
      · exact (hmem_r x).mpr (Or.inl rfl)
      · exact (hmem_r x).mpr (Or.inr ((hmem_r₁ x).mpr (Or.inr hx'')))
    · intro hx
      rcases (hmem_r x).mp hx with rfl | hx'
      · exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
      rcases (hmem_r₁ x).mp hx' with rfl | hx''
      · exact List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx'')
  exact (elimination_sound stepB hcont₁ t).trans
    ((elimination_sound stepA hcont₂ t).trans hAr)

/-! ## Item 3 — the key probe

The shape `exec/dispatch/classify.rs` lowers to a point probe: exactly
one positive atom, no negated atoms, no residuals (`conditions = []`),
and by-value bindings — literals, or params resolved per probe —
covering a declared key's projection. Execution is then ONE
determinant get, not a scan; the theorem says the get equals the join
denotation, with the key's uniqueness
(`Dependencies.functionality_unique_witness`) carrying the
at-most-one half. -/

/-- A term bound BY VALUE — pinned to a constant an `Eq` filter can
carry: a literal, or a param resolved at bind (`classify.rs:77-87`;
`KeyProbePlan` resolves key constants per probe). Variables bind, sets
and measures are refused filter shapes (`classify.rs:44-75`). -/
def Term.pinned : Term → Prop
  | .lit _ => True
  | .param _ => True
  | _ => False

/-- The value a pinned term resolves to under the parameter
environment. -/
def Term.pinValue (ρ : ParamEnv) : Term → Option Value
  | .lit c => some c
  | .param p => some (ρ.scalar p)
  | _ => none

/-- A pinned term always resolves. -/
theorem Term.pinned_resolves {t : Term} (h : t.pinned) (ρ : ParamEnv) :
    ∃ c, t.pinValue ρ = some c := by
  cases t with
  | lit c => exact ⟨c, rfl⟩
  | param p => exact ⟨ρ.scalar p, rfl⟩
  | var v => exact absurd h (fun hf => hf)
  | paramSet p => exact absurd h (fun hf => hf)
  | measure v => exact absurd h (fun hf => hf)

/-- A pinned term selects exactly its resolved value. -/
theorem Term.pin_selects {ρ : ParamEnv} {σ : Assignment} {t : Term}
    {w c : Value} (hpv : t.pinValue ρ = some c)
    (hsel : Term.selects ρ σ t w) : w = c := by
  cases t with
  | lit c' =>
    cases hpv
    exact hsel.symm
  | param p =>
    cases hpv
    exact hsel.symm
  | var v => cases hpv
  | paramSet p => cases hpv
  | measure v => cases hpv

/-- The first pinned binding's value at a field — the probe's resolved
key constant for that position (`value_of`, `classify.rs:77-87`). -/
def pinAt (ρ : ParamEnv) : List (FieldId × Term) → FieldId → Option Value
  | [], _ => none
  | bd :: bds, i =>
    if bd.1 = i then
      match bd.2.pinValue ρ with
      | some c => some c
      | none => pinAt ρ bds i
    else pinAt ρ bds i

/-- A resolved pin comes from some pinned binding at that field. -/
theorem pinAt_spec {ρ : ParamEnv} {c : Value} :
    ∀ {bds : List (FieldId × Term)} {i : FieldId},
      pinAt ρ bds i = some c →
      ∃ t, (i, t) ∈ bds ∧ t.pinValue ρ = some c
  | [], _, h => by cases h
  | bd :: bds, i, h => by
    by_cases hi : bd.1 = i
    · rw [pinAt, if_pos hi] at h
      cases hpv : bd.2.pinValue ρ with
      | some c' =>
        rw [hpv] at h
        cases h
        refine ⟨bd.2, ?_, hpv⟩
        rw [← hi]
        exact List.mem_cons_self ..
      | none =>
        rw [hpv] at h
        obtain ⟨t, hmem, hval⟩ := pinAt_spec h
        exact ⟨t, List.mem_cons_of_mem _ hmem, hval⟩
    · rw [pinAt, if_neg hi] at h
      obtain ⟨t, hmem, hval⟩ := pinAt_spec h
      exact ⟨t, List.mem_cons_of_mem _ hmem, hval⟩

/-- A field carrying a pinned binding resolves some pin. -/
theorem pinAt_isSome {ρ : ParamEnv} {t : Term} {c : Value} :
    ∀ {bds : List (FieldId × Term)} {i : FieldId},
      (i, t) ∈ bds → t.pinValue ρ = some c →
      (pinAt ρ bds i).isSome
  | [], _, hmem, _ => by cases hmem
  | bd :: bds, i, hmem, hpv => by
    by_cases hi : bd.1 = i
    · rw [pinAt, if_pos hi]
      cases hpv' : bd.2.pinValue ρ with
      | some c' => rfl
      | none =>
        rcases List.mem_cons.mp hmem with heq | hmem'
        · -- the head IS the pinned binding: its pin resolves
          have : bd.2 = t := (congrArg Prod.snd heq).symm
          rw [this, hpv] at hpv'
          cases hpv'
        · exact pinAt_isSome hmem' hpv
    · rw [pinAt, if_neg hi]
      rcases List.mem_cons.mp hmem with heq | hmem'
      · exact absurd (congrArg Prod.fst heq).symm hi
      · exact pinAt_isSome hmem' hpv

/-- The probe predicate: the fact agrees with every resolved key pin —
the determinant-tuple equality one `U` get decides. -/
def probeHitB (ρ : ParamEnv) (a : Atom) (K : List FieldId)
    (f : Fact) : Bool :=
  K.all fun i =>
    match pinAt ρ a.bindings i with
    | some c => decide (f i = c)
    | none => false

/-- **`KeyProbeShape`** — what the lowering ACCEPTS
(`classify.rs:24-130`): one positive atom, nothing negated, and a key
statement of the theory whose every projection field is bound by
value. The rule's CONDITIONS are unconstrained: `classify` accepts
non-key per-field filters (order compares, `Ne`, `FieldsCompare`,
resolved `PointIn`/Allen) into `remaining_filters`, applied post-get
— the model carries them as the rule's conditions, checked on the one
hit (`keyProbeEval`); the engine's per-field filters are instances of
the general shape. The full-fact `M` fallback (`classify.rs:153-160`)
is a recorded narrowing (module doc). -/
structure KeyProbeShape (T : Theory) (r : Rule) (a : Atom)
    (K : List FieldId) : Prop where
  /-- Exactly one atom occurrence. -/
  atoms : r.atoms = [a]
  /-- Positive only — no anti-joins on the fast path. -/
  negated : r.negated = []
  /-- The key resolves against a declared functionality statement
  (`key_probe_candidate`; fresh auto-keys included). -/
  declared : Statement.functionality a.relation K ∈ T.statements
  /-- Every key field is bound by value (`value_of` finds an `Eq`
  constant for each). -/
  covered : ∀ i, i ∈ K → ∃ t, (i, t) ∈ a.bindings ∧ t.pinned

/-- The point-probe evaluation: ONE determinant get — the first (and,
under the key, only) tuple-equal fact — then the decoded fact's
bindings checked, the residual conditions applied to the hit
(`remaining_filters` post-get, `key_probe_fact.rs:253`), and the
finds projected, exactly the probe kernel's shape
(`exec/dispatch/execute_key_probe.rs`). Reuses the `evalList`
machinery: `bindAtom` is the decode-and-check step, `condHoldsB` the
residual filter. -/
def keyProbeEval (C : Classify) (W : ListInstance) (ρ : ParamEnv)
    (r : Rule) (a : Atom) (K : List FieldId) : List AnswerTuple :=
  match (W.facts a.relation).find? (probeHitB ρ a K) with
  | none => []
  | some f =>
    match bindAtom ρ f a.bindings [] with
    | some σp =>
      match r.conditions.all (condHoldsB C ρ (totalize σp)) with
      | true => [r.finds.map (totalize σp)]
      | false => []
    | none => []

/-- A matching fact hits the probe: the atom's pinned bindings force
the fact's key fields to the resolved constants. -/
theorem probeHit_of_matches {ρ : ParamEnv} {σ : Assignment} {a : Atom}
    {K : List FieldId} {f : Fact}
    (hcov : ∀ i, i ∈ K → ∃ t, (i, t) ∈ a.bindings ∧ t.pinned)
    (hm : Matches f a σ ρ) : probeHitB ρ a K f = true := by
  refine List.all_eq_true.mpr fun i hi => ?_
  obtain ⟨t, hmem, hpin⟩ := hcov i hi
  obtain ⟨c, hpv⟩ := t.pinned_resolves hpin ρ
  have hsome := pinAt_isSome hmem hpv
  obtain ⟨c', hc'⟩ := Option.isSome_iff_exists.mp hsome
  obtain ⟨t', hmem', hpv'⟩ := pinAt_spec hc'
  have : f i = c' := Term.pin_selects hpv' (hm _ hmem')
  simp only [hc']
  exact decide_eq_true this

/-- Two probe hits share the key tuple: both project to the resolved
pins. -/
theorem probeHit_project {ρ : ParamEnv} {a : Atom} {K : List FieldId}
    {f g : Fact} (hf : probeHitB ρ a K f = true)
    (hg : probeHitB ρ a K g = true) : f.project K = g.project K := by
  refine (Fact.project_eq_iff f g K).mpr fun i hi => ?_
  have hfi := List.all_eq_true.mp hf i hi
  have hgi := List.all_eq_true.mp hg i hi
  cases hc : pinAt ρ a.bindings i with
  | none =>
    rw [hc] at hfi
    cases hfi
  | some c =>
    rw [hc] at hfi hgi
    have h1 : f i = c := of_decide_eq_true hfi
    have h2 : g i = c := of_decide_eq_true hgi
    rw [h1, h2]

/-- **Item 3 — `keyprobe_equiv_join`.** Under the accepted shape and
the key's uniqueness, the point-probe evaluation equals the join
denotation, membership for membership: soundness is the probe's own
decode-and-check plus the residual conditions it verified on the hit;
completeness pins the probed fact through
`functionality_unique_witness` — any deriving fact hits the probe, the
key allows only one hit, so the ONE get finds exactly it, and the
deriving assignment's conditions transfer to the hit (the residual
filters only shrink the at-most-one candidate, never widen it).
`Safe` and the measure-free binding shape are the same two premises
`eval_sound` spends (the validator discharges both); `Safe` is also
what makes the residual conditions decidable on the probe's state —
every condition variable is bound by the one atom.
Bridge: `PreparedRule::KeyProbe`; `api/prepared/build.rs:335`;
`remaining_filters` (`classify.rs:127,165-188`). -/
theorem keyprobe_equiv_join {T : Theory} {C : Classify}
    {W : ListInstance} {ρ : ParamEnv} {r : Rule} {a : Atom}
    {K : List FieldId} (hshape : KeyProbeShape T r a K)
    (hkey : Functionality (W.den a.relation) K) (hsafe : Safe r)
    (hnm : ∀ bd, bd ∈ a.bindings → ¬ bd.2.isMeasure) :
    ∀ t, t ∈ keyProbeEval C W ρ r a K ↔ t ∈ ruleAnswers C r W.den ρ := by
  intro t
  constructor
  · -- the probe's answer derives
    intro ht
    unfold keyProbeEval at ht
    cases hfind : (W.facts a.relation).find? (probeHitB ρ a K) with
    | none =>
      simp only [hfind] at ht
      cases ht
    | some f =>
      simp only [hfind] at ht
      cases hbind : bindAtom ρ f a.bindings [] with
      | none =>
        simp only [hbind] at ht
        cases ht
      | some σp =>
        simp only [hbind] at ht
        cases hcnd : r.conditions.all (condHoldsB C ρ (totalize σp)) with
        | false =>
          simp only [hcnd] at ht
          cases ht
        | true =>
          simp only [hcnd] at ht
          have hteq : t = r.finds.map (totalize σp) :=
            List.mem_singleton.mp ht
          obtain ⟨-, hpins⟩ := bindAtom_sound a.bindings [] σp hbind
          refine mem_ruleAnswers.mpr ⟨totalize σp, ⟨?_, ?_, ?_⟩, hteq⟩
          · intro x hx
            rw [hshape.atoms] at hx
            rcases List.mem_singleton.mp hx with rfl
            exact ⟨f, List.mem_of_find?_eq_some hfind,
              fun bd hbd => (hpins bd hbd).selects⟩
          · intro n hn
            rw [hshape.negated] at hn
            cases hn
          · intro c hc
            exact (condHoldsB_iff C ρ (totalize σp) c).mp
              (List.all_eq_true.mp hcnd c hc)
  · -- every deriving answer is the probe's
    intro ht
    obtain ⟨σ, ⟨hatoms, -, hconds⟩, rfl⟩ := mem_ruleAnswers.mp ht
    obtain ⟨f, hfI, hm⟩ := hatoms a (by
      rw [hshape.atoms]
      exact List.mem_singleton.mpr rfl)
    have hf : f ∈ W.facts a.relation := hfI
    have hhit := probeHit_of_matches hshape.covered hm
    -- the one get finds exactly the deriving fact
    cases hfind : (W.facts a.relation).find? (probeHitB ρ a K) with
    | none => exact absurd hhit (List.find?_eq_none.mp hfind f hf)
    | some g =>
      have hghit := List.find?_some hfind
      have hgmem : g ∈ W.facts a.relation :=
        List.mem_of_find?_eq_some hfind
      have hgf : g = f :=
        functionality_unique_witness hkey (f.project K) f hfI rfl g
          hgmem (probeHit_project hghit hhit)
      subst hgf
      -- the decode-and-check step succeeds and agrees with σ
      obtain ⟨σp, hbind, hagp, -⟩ :=
        bindAtom_complete a.bindings []
          (fun v x hx => by cases hx) (fun bd hbd => hm bd hbd) hnm
      obtain ⟨-, hpins⟩ := bindAtom_sound a.bindings [] σp hbind
      -- `Safe` pins every rule variable to the one atom, so the
      -- probe's totalized state agrees with σ everywhere the rule
      -- reads
      have hagree : ∀ v, v ∈ r.allVars → totalize σp v = σ v := by
        intro v hv
        obtain ⟨x, hx, hbound⟩ := mem_positiveVars.mp (hsafe v hv)
        rw [hshape.atoms] at hx
        rw [List.mem_singleton.mp hx] at hbound
        obtain ⟨bd, hbd, hvbd⟩ := List.mem_flatMap.mp hbound
        have hpin := hpins bd hbd
        rw [Term.mem_bindingVars.mp hvbd] at hpin
        exact totalize_agrees hagp
          (by rw [show lookupVar σp v = some (g bd.1) from hpin]; rfl)
      -- the residual conditions transfer from σ to the hit
      have hcnd : r.conditions.all (condHoldsB C ρ (totalize σp))
          = true :=
        List.all_eq_true.mpr fun c hc =>
          (condHoldsB_iff C ρ (totalize σp) c).mpr
            ((condHolds_congr C ρ σ (totalize σp) c fun v hv =>
              (hagree v (mem_allVars.mpr
                (Or.inr (Or.inr (Or.inr ⟨c, hc, hv⟩))))).symm).mp
              (hconds c hc))
      unfold keyProbeEval
      simp only [hfind, hbind, hcnd]
      refine List.mem_singleton.mpr ?_
      refine (List.map_congr_left fun v hv => ?_).symm
      exact hagree v (mem_allVars.mpr (Or.inl hv))

/-- The key premise, spent from the theory: on a holding instance an
accepted scalar key over an open relation IS semantic functionality of
the instance's extension — how `keyprobe_equiv_join`'s `hkey` is
discharged by `holds` (03's acceptance-spending pattern,
`accepted_target_key_spent`). -/
theorem keyprobe_key_spent {T : Theory} {I : Instance}
    (hI : holds T I) {R : RelId} {K : List FieldId}
    (hdecl : Statement.functionality R K ∈ T.statements)
    (hscalar : T.header.intervalSplit R K = none)
    (hopen : T.closed R = none) : Functionality (I R) K := by
  have hj := hI _ hdecl
  simp only [Statement.judgment, hscalar] at hj
  unfold Theory.den at hj
  rw [hopen] at hj
  exact hj

/-- The pointwise face of the key premise: on a holding instance an
accepted key whose field set splits pointwise (one interval field, any
written position) is ALSO semantic functionality — a pointwise key
implies exact-tuple functionality, because two facts sharing the
scalar prefix with IDENTICAL intervals overlap pointwise
(`interval_nonempty` supplies the shared point), which `PointwiseKey`
forbids of distinct facts. The typing premise `hpts` — every stored
fact denotes at least one point at the split position — is PRD 03's
arity-respecting-facts narrowing made a named hypothesis, discharged
fieldwise by `Value.points_nonempty` on interval-typed columns. This
closes `keyprobe_equiv_join`'s `hkey` for interval-final pointwise
keys: the engine key-probes them too (`key_probe_candidate` accepts
ANY declared key, `classify.rs:134-151`; the lock
`pointwise_key_point_lookup_uses_key_probe_and_is_image_free`). -/
theorem keyprobe_pointwise_key_spent {T : Theory} {I : Instance}
    (hI : holds T I) {R : RelId} {K S : List FieldId} {i : FieldId}
    (hdecl : Statement.functionality R K ∈ T.statements)
    (hsplit : T.header.intervalSplit R K = some (S, i))
    (hopen : T.closed R = none)
    (hpts : ∀ f, f ∈ I R → ∃ p, p ∈ (f i).points) :
    Functionality (I R) K := by
  have hj := hI _ hdecl
  simp only [Statement.judgment, hsplit] at hj
  unfold Theory.den at hj
  rw [hopen] at hj
  obtain ⟨hfil, hS⟩ := Header.intervalSplit_some hsplit
  intro f g hf hg hproj
  -- the interval field is a member of the key's field set
  have hiK : i ∈ K := by
    have hmem : i ∈ K.filter (fun j => T.header.isInterval R j) := by
      rw [hfil]
      exact List.mem_singleton.mpr rfl
    exact (List.mem_filter.mp hmem).1
  -- key-tuple agreement restricts to the scalar prefix…
  have hSproj : f.project S = g.project S := by
    refine (Fact.project_eq_iff f g S).mpr fun j hj' => ?_
    refine (Fact.project_eq_iff f g K).mp hproj j ?_
    rw [hS] at hj'
    exact (List.mem_filter.mp hj').1
  -- …and pins the interval position exactly
  have hfg : f i = g i := (Fact.project_eq_iff f g K).mp hproj i hiK
  refine Classical.byContradiction fun hne => ?_
  obtain ⟨p, hp⟩ := hpts f hf
  exact hj f g hf hg hSproj hne p hp (hfg ▸ hp)

/-! ## Item 4 — static emptiness and the latch's two constructors -/

/-- **`StaticallyEmpty`** — the fold's kill rule
(`ir/normalize/fold.rs`, rules (a)-(f)): some condition of the rule
refutes under EVERY environment and assignment — the semantic content
of a contradiction judged on constants alone (params never fold, so a
detected refutation cannot depend on ρ; the ∀ρ∀σ form is what
"constants only" buys). The detector's completeness is not claimed —
the recorded narrowing. -/
def StaticallyEmpty (C : Classify) (r : Rule) : Prop :=
  ∃ c, c ∈ r.conditions ∧ ∀ (ρ : ParamEnv) (σ : Assignment),
    ¬ Condition.holds C ρ σ c

/-- **`statically_empty_sound`.** A refuted rule contributes the empty
answer set on EVERY instance — the verdict never consulted one.
Bridge: `Program::Empty` and the fold-death records (`NormalizedQuery::
dead`, deleted at prepare). -/
theorem statically_empty_sound {C : Classify} {r : Rule}
    (h : StaticallyEmpty C r) :
    ∀ (I : Instance) (ρ : ParamEnv) t, t ∉ ruleAnswers C r I ρ := by
  intro I ρ t ht
  obtain ⟨σ, ⟨-, -, hconds⟩, -⟩ := mem_ruleAnswers.mp ht
  obtain ⟨c, hc, href⟩ := h
  exact href ρ σ (hconds c hc)

/-- **The latch's two-constructor distinction, structural**
(`api/prepared/bind.rs:280-365`): an execution comes up empty for one
of two reasons, and they are DIFFERENT verdicts. `selectionMiss` is
`Ok(false)` — one positive atom's selection finds nothing in THIS
instance (the dictionary miss: an unresolved `PendingIntern` means no
fact of this snapshot carries the value) — per-execution, never a plan
verdict; a later instance may answer
(`Countermodels.latch_miss_not_static`). `refuted` is the fold's
instance-independent refutation — the plan itself is
`Program::Empty`. -/
inductive EmptyAt (C : Classify) (ρ : ParamEnv) (r : Rule)
    (I : Instance) : Prop where
  /-- The per-instance selection miss: one positive atom matches no
  fact of this instance under any assignment — this execution's `Eq`
  short-circuit, sound on positive occurrences only (a negated miss
  just rejects nothing). -/
  | selectionMiss (a : Atom) (ha : a ∈ r.atoms)
      (hmiss : ∀ f, f ∈ I a.relation → ∀ σ, ¬ Matches f a σ ρ)
  /-- The instance-independent refutation: the fold's verdict, which
  never read `I`. -/
  | refuted (h : StaticallyEmpty C r)

/-- Both constructors empty this execution's answers — the shared
face; only `refuted` transfers to every instance (the theorem after,
and the countermodel that the miss does not). -/
theorem emptyAt_no_answers {C : Classify} {ρ : ParamEnv} {r : Rule}
    {I : Instance} (h : EmptyAt C ρ r I) :
    ∀ t, t ∉ ruleAnswers C r I ρ := by
  intro t ht
  cases h with
  | selectionMiss a ha hmiss =>
    obtain ⟨σ, ⟨hatoms, -, -⟩, -⟩ := mem_ruleAnswers.mp ht
    obtain ⟨f, hf, hm⟩ := hatoms a ha
    exact hmiss f hf σ hm
  | refuted h => exact statically_empty_sound h I ρ t ht

/-- The refutation constructor is instance-INDEPENDENT: it verdicts
every instance at once — exactly what licenses deleting the rule from
the prepared program, where the miss licenses only this execution's
empty result. -/
theorem emptyAt_refuted_everywhere {C : Classify} {r : Rule}
    (h : StaticallyEmpty C r) :
    ∀ (I : Instance) (ρ : ParamEnv), EmptyAt C ρ r I :=
  fun _ _ => .refuted h

/-! ## Item 5 — the rewrites compose

The prepare pipeline's licence to chain: grounding steps, eliminations
and kills, in any order, any number — each preserves `queryAnswers` on
instances that hold the theory and agree with its ground axioms, so
any sequence does. The theorem falls out of items 1, 2 and 4 by
rewriting, which is the shape check on their statements. -/

/-- One prepare-time rewrite step on a program, at one rule. The
elimination step carries the THEORY-side premises: the declared
containment (in the statement's own `Bumbledb.Atom` shape) and
condition 4's scalar splits (`Enforcement::ScalarProbe` — the interval
refusal, `plan/ground.rs:168`); `holds` cashes them into the semantic
containment at execution. -/
inductive RewriteStep (T : Theory) (C : Classify) :
    Query → Query → Prop where
  /-- The grounding fold: one rule rewritten
  (`Role::Folded`, the membership attachment). -/
  | ground {n : Nat} {pre post : List Rule} {r r' : Rule}
      (h : groundRewrite T r = .inl r') :
      RewriteStep T C ⟨n, pre ++ r :: post⟩ ⟨n, pre ++ r' :: post⟩
  /-- The grounding refutation: the dead rule deleted at prepare
  (`folded to ∅`). -/
  | groundDead {n : Nat} {pre post : List Rule} {r : Rule}
      {g : Grounded} (h : groundRewrite T r = .inr g) :
      RewriteStep T C ⟨n, pre ++ r :: post⟩ ⟨n, pre ++ post⟩
  /-- The containment elimination (`Role::Eliminated(statement)`). -/
  | eliminate {n : Nat} {pre post : List Rule} {r r' : Rule}
      {a b : Atom} {X Y : List FieldId} {φ ψ : Selection}
      (hs : ElimStep r r' a b X Y φ ψ)
      (hdecl : Statement.containment ⟨a.relation, X, φ⟩
        ⟨b.relation, Y, ψ⟩ ∈ T.statements)
      (hsrc : T.header.intervalSplit a.relation X = none)
      (htgt : T.header.intervalSplit b.relation Y = none) :
      RewriteStep T C ⟨n, pre ++ r :: post⟩ ⟨n, pre ++ r' :: post⟩
  /-- The chained containment elimination — the discharged-source arm
  (`chain_reaches`, the support forest): the support pair and the
  elimination it licenses land as ONE composed step, the
  acyclic-support premise (`hroot`) named. -/
  | eliminateChained {n : Nat} {pre post : List Rule} {r r₁ r₂ : Rule}
      {a b c : Atom} {X₁ Y₁ X₂ Y₂ : List FieldId}
      {φ₁ ψ₁ φ₂ ψ₂ : Selection}
      (hs₁ : ElimStep r r₁ a b X₁ Y₁ φ₁ ψ₁)
      (hs₂ : ChainedElimStep r₁ r₂ b c X₂ Y₂ φ₂ ψ₂)
      (hroot : a ∈ r₂.atoms)
      (hdecl₁ : Statement.containment ⟨a.relation, X₁, φ₁⟩
        ⟨b.relation, Y₁, ψ₁⟩ ∈ T.statements)
      (hdecl₂ : Statement.containment ⟨b.relation, X₂, φ₂⟩
        ⟨c.relation, Y₂, ψ₂⟩ ∈ T.statements)
      (hsrc₁ : T.header.intervalSplit a.relation X₁ = none)
      (htgt₁ : T.header.intervalSplit b.relation Y₁ = none)
      (hsrc₂ : T.header.intervalSplit b.relation X₂ = none)
      (htgt₂ : T.header.intervalSplit c.relation Y₂ = none) :
      RewriteStep T C ⟨n, pre ++ r :: post⟩ ⟨n, pre ++ r₂ :: post⟩
  /-- The statically-empty kill (`NormalizedQuery::dead`). -/
  | kill {n : Nat} {pre post : List Rule} {r : Rule}
      (h : StaticallyEmpty C r) :
      RewriteStep T C ⟨n, pre ++ r :: post⟩ ⟨n, pre ++ post⟩

/-- Replacing one rule by an answer-equal rule preserves the query's
answers — the union reads members only. -/
theorem queryAnswers_congr_at {C : Classify} {I : Instance}
    {ρ : ParamEnv} {n : Nat} {pre post : List Rule} {r r' : Rule}
    (h : ∀ t, t ∈ ruleAnswers C r I ρ ↔ t ∈ ruleAnswers C r' I ρ) :
    ∀ t, t ∈ queryAnswers C ⟨n, pre ++ r :: post⟩ I ρ ↔
      t ∈ queryAnswers C ⟨n, pre ++ r' :: post⟩ I ρ := by
  intro t
  constructor
  · intro ht
    obtain ⟨x, hx, hta⟩ := mem_queryAnswers.mp ht
    rcases List.mem_append.mp hx with hx' | hx'
    · exact mem_queryAnswers.mpr
        ⟨x, List.mem_append.mpr (Or.inl hx'), hta⟩
    · rcases List.mem_cons.mp hx' with rfl | hx''
      · exact mem_queryAnswers.mpr
          ⟨r', List.mem_append.mpr (Or.inr (List.mem_cons_self ..)),
            (h t).mp hta⟩
      · exact mem_queryAnswers.mpr
          ⟨x, List.mem_append.mpr
            (Or.inr (List.mem_cons_of_mem _ hx'')), hta⟩
  · intro ht
    obtain ⟨x, hx, hta⟩ := mem_queryAnswers.mp ht
    rcases List.mem_append.mp hx with hx' | hx'
    · exact mem_queryAnswers.mpr
        ⟨x, List.mem_append.mpr (Or.inl hx'), hta⟩
    · rcases List.mem_cons.mp hx' with rfl | hx''
      · exact mem_queryAnswers.mpr
          ⟨r, List.mem_append.mpr (Or.inr (List.mem_cons_self ..)),
            (h t).mpr hta⟩
      · exact mem_queryAnswers.mpr
          ⟨x, List.mem_append.mpr
            (Or.inr (List.mem_cons_of_mem _ hx'')), hta⟩

/-- Deleting an answerless rule preserves the query's answers — the
union loses nothing it never had. -/
theorem queryAnswers_drop_at {C : Classify} {I : Instance}
    {ρ : ParamEnv} {n : Nat} {pre post : List Rule} {r : Rule}
    (h : ∀ t, t ∉ ruleAnswers C r I ρ) :
    ∀ t, t ∈ queryAnswers C ⟨n, pre ++ r :: post⟩ I ρ ↔
      t ∈ queryAnswers C ⟨n, pre ++ post⟩ I ρ := by
  intro t
  constructor
  · intro ht
    obtain ⟨x, hx, hta⟩ := mem_queryAnswers.mp ht
    rcases List.mem_append.mp hx with hx' | hx'
    · exact mem_queryAnswers.mpr
        ⟨x, List.mem_append.mpr (Or.inl hx'), hta⟩
    · rcases List.mem_cons.mp hx' with rfl | hx''
      · exact absurd hta (h t)
      · exact mem_queryAnswers.mpr
          ⟨x, List.mem_append.mpr (Or.inr hx''), hta⟩
  · intro ht
    obtain ⟨x, hx, hta⟩ := mem_queryAnswers.mp ht
    rcases List.mem_append.mp hx with hx' | hx'
    · exact mem_queryAnswers.mpr
        ⟨x, List.mem_append.mpr (Or.inl hx'), hta⟩
    · exact mem_queryAnswers.mpr
        ⟨x, List.mem_append.mpr (Or.inr (List.mem_cons_of_mem _ hx')),
          hta⟩

/-- One step preserves the query's answers on every instance that
holds the theory and agrees with its ground axioms — items 1, 2 and 4,
lifted to the program. -/
theorem step_preserves {T : Theory} {C : Classify} {q q' : Query}
    (hstep : RewriteStep T C q q') {I : Instance} {ρ : ParamEnv}
    (hI : holds T I) (hax : AgreesWithAxioms T I) :
    ∀ t, t ∈ queryAnswers C q I ρ ↔ t ∈ queryAnswers C q' I ρ := by
  cases hstep with
  | ground h =>
    rename_i r _
    refine queryAnswers_congr_at fun t => ?_
    have := grounding_preserves_answers C r hax ρ t
    rw [h] at this
    exact this.symm
  | groundDead h =>
    exact queryAnswers_drop_at fun t => ground_refuted_empty h hax ρ t
  | eliminate hs hdecl hsrc htgt =>
    refine queryAnswers_congr_at fun t => ?_
    have hj := hI _ hdecl
    simp only [Statement.judgment, hsrc, htgt] at hj
    exact (elimination_sound hs
      (containment_transfer (den_agrees hax _) (den_agrees hax _) hj)
      t).symm
  | eliminateChained hs₁ hs₂ hroot hdecl₁ hdecl₂ hsrc₁ htgt₁ hsrc₂ htgt₂ =>
    refine queryAnswers_congr_at fun t => ?_
    have hj₁ := hI _ hdecl₁
    have hj₂ := hI _ hdecl₂
    simp only [Statement.judgment, hsrc₁, htgt₁] at hj₁
    simp only [Statement.judgment, hsrc₂, htgt₂] at hj₂
    exact (chained_elimination_sound hs₁ hs₂ hroot
      (containment_transfer (den_agrees hax _) (den_agrees hax _) hj₁)
      (containment_transfer (den_agrees hax _) (den_agrees hax _) hj₂)
      t).symm
  | kill h =>
    exact queryAnswers_drop_at fun t => statically_empty_sound h I ρ t

/-- A rewrite sequence: any chain of the rewrites (grounding, kill,
elimination — the chained composed step included). -/
inductive Rewrites (T : Theory) (C : Classify) : Query → Query → Prop
  | refl (q : Query) : Rewrites T C q q
  | step {q q' q'' : Query} (h : RewriteStep T C q q')
      (rest : Rewrites T C q' q'') : Rewrites T C q q''

/-- **Item 5 — `rewrite_composition`.** ANY sequence of grounding,
elimination (chained-source pairs included) and kill steps preserves
`queryAnswers` on every instance holding the theory and agreeing with
its ground axioms — the prepare pipeline's licence to chain for the
MODELED steps (the subsumption deletion runs outside this chain — the
recorded narrowing in the module doc). Falls out of items 1, 2 and 4
by induction over the chain, one rewrite per step. -/
theorem rewrite_composition {T : Theory} {C : Classify} {q q' : Query}
    (h : Rewrites T C q q') {I : Instance} {ρ : ParamEnv}
    (hI : holds T I) (hax : AgreesWithAxioms T I) :
    ∀ t, t ∈ queryAnswers C q I ρ ↔ t ∈ queryAnswers C q' I ρ := by
  induction h with
  | refl q => exact fun t => Iff.rfl
  | step hstep _ ih =>
    exact fun t => (step_preserves hstep hI hax t).trans (ih t)

end Bumbledb.Query
