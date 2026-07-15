import Bumbledb.Oracle
import Bumbledb.Decide

/-!
# Admission — the acceptance gate as an inhabited structure

**The admission law.** A statement form — or a future operator that
wants statement-vocabulary standing — enters the vocabulary by
inhabiting `AdmissibleForm`. The type IS the acceptance gate's
checklist (`docs/architecture/30-dependencies.md` § the acceptance
gate), and an inhabitant is the whole mathematical case for
acceptance: reason about exact operators before writing Rust. Five
forms inhabit it below — the three fact-level forms
(`functionalityForm`, `containmentForm`, `cardinalityForm`) and the
two pointwise forms (`pointwiseForm`, `coverageForm`); each
instantiation pulls the campaign's waves together — the Level-0
denotation (`Dependencies.lean` / `Cardinality.lean`), the
executable judge (`Decide.lean`), the delta restriction
(`Txn/DeltaRestriction.lean`), and the order-oracle plan
(`Oracle.lean`) — so "this form is accepted" is the existence of ONE
term. The deliberately inadmissible E1 shape is UNINHABITABLE on the
oracle-plan field (`Countermodels.joined_window_form_uninhabitable`,
composing the blast countermodel `Countermodels.joined_window_blast`):
"prohibitively expensive" is a type error, not an opinion. The
query-side sibling is the recursion safety roster
(`Exec/Fixpoint.lean`, `docs/architecture/20-query-ir.md` § engine
recursion): one
doctrine on both paths — a feature's admission is a proof obligation,
never a vibe.

## The fields (the checklist, wave by wave)

* **`Judgment`** — the form's Level-0 denotation, per parameter.
* **`surface` / `surfaceProj`** — the consulted stored surfaces: which
  fact sets the form's judge reads, each with its grouping projection
  (the determinant-index key).
* **`quarantined`** — creation-quarantine compliance: the judgment
  factors through the consulted surfaces — two instances agreeing on
  every surface receive one verdict, so the judgment reads STORED
  values only (`docs/architecture/20-query-ir.md` § the creation
  quarantine, the write-side face).
* **`check` / `checkPremise` / `check_decides`** — the executable
  checker, sound AND complete against the denotation on row-listed
  instances under the merge premise plus the form's DECLARED checker
  premise (`Decide.lean`'s per-form checkers; every inhabitant below
  declares the trivial premise — the field exists so a form whose
  judge needs an acceptance rule must say so in the open).
* **`DeltaCheck` / `delta_restricts`** — the delta-restricted check
  and THE restriction theorem: over a holding pre-state, the final
  state satisfies the judgment IFF the restricted check passes
  (`Txn/DeltaRestriction.lean`).
* **`Touched` / `touched_delta_bounded`** — the touched keys, forced
  to be delta-derived: every probed key is the projection of some
  delta fact at a consulted surface's grouping, so the probe COUNT is
  bounded by the delta (the cost law's first half).
* **`probe` / `Verdict` / `plan_decides`** — the order-oracle plan:
  per consulted surface a sanctioned probe shape (`ProbeShape`, whose
  price is `Oracle.EnforcementPlan.consultations` — one descent for a
  point probe, the entry seek plus the walked group for a prefix
  walk), and per touched key a verdict over the probes' answers that,
  against EVERY conforming oracle family, decides `DeltaCheck` (the
  cost law's second half: nothing outside the touched groups is ever
  read). Law 3's abstract-cost scoping, spent exactly as in
  `Oracle.lean`.

The composition (`AdmissibleForm.gate_decides_final`): over a holding
pre-state, the touched verdicts against any conforming oracle family
ARE the final-state judgment — the commit pipeline's whole read
pattern, per form, as one theorem of the structure.

## Design records

* **Per-form parameterization, statement-agnostic (the design call
  recorded).** `AdmissibleForm Param Ix` carries the form's
  schema-level parameters (`Param`) and its consulted-surface index
  (`Ix`); it does NOT carry a `Statement` constructor. A new form
  inhabits the structure BEFORE its constructor lands — that is the
  admission law's point — and the E1 countermodel must be statable at
  all (a joined side is unwritable as an `Atom`, so a
  constructor-anchored structure could never even pose the question).
  The tie to the closed `Statement` vocabulary is the per-form
  `…_denotes` theorems below: under the form's split scope, the
  `Judgment` field IS `Statement.judgment`'s arm.
* **Quarantine as surface-extensionality (the formulation recorded).**
  The tasked candidate — invariance under value-preserving instance
  isomorphism — collapses here: selections read declared literal
  sets, so the only isomorphisms available preserve every stored
  value on every read position, i.e. they fix the surfaces pointwise.
  The cleaner statement is the factoring itself: the judgment is a
  function of the consulted surfaces' fact sets, nothing else — no
  minted value, no other relation, no host state can move a verdict.
* **The oracle-plan field's shape.** Every inhabitant is a
  single-key form, so the field is stated over
  `Oracle.OrderedOracle (List Value) P Fact` families indexed by `Ix`,
  every probe at the ONE touched key — the one-oracle-per-evaluation
  discipline (`Oracle.plan_answers_sound`) made structural, with the
  surfaces pinned per form (a verdict cannot read a join because no
  field gives it one). The per-form `Planned` theorems of
  `Oracle.lean` discharge the fact-level fields; the acceptance
  premises price the probes there
  (`Oracle.accepted_target_key_prices_the_probe`), unchanged. The
  `Verdict` field reads each surface at the ONE touched key, and
  `touched_delta_bounded` forces every touched key to be a delta
  fact's projection — the two fences whose joint exclusion makes
  `Countermodels.joined_window_form_uninhabitable` true; an
  answer-dependent (chase-shaped) read or an every-group escalation
  is structurally refused here, which is part of why the order-mark
  forms left the vocabulary (`docs/architecture/30-dependencies.md`
  § refused: order marks).

## Narrowings recorded (law 5: narrow and record)

* **The pointwise inhabitants enter at the walk shape.** Their
  verdicts read whole walked groups; the engine's finer reads — the
  two-neighbor probe (`Oracle.neighbor_probe_decides`) and the
  one-pass sweep (`Oracle.coverage_walk_decides`), both at the
  interval altitude — refine the walk below this structure's
  fact-altitude field, and a wider walk only re-reads more (the
  recorded superset license in `Oracle.lean`).
* **The type prices a walk at its walked group and no finer.** A
  degenerate grouping (an empty projection) would make one "group"
  the whole relation; the gate's ACCEPTANCE rules (the target-key and
  key-form demands, `docs/architecture/30-dependencies.md`) refuse
  such shapes at declaration, and that refusal is mechanism the docs
  own. This type states the honest abstract count for whatever
  grouping the form declares.
* **Verdicts read whole consults.** The two sanctioned shapes both
  answer with `consult` (`ProbeShape.toPlan_answers`). Engine-side
  clipping of a walk is representation
  (`Exec/Sweep.lean: sweep_ignores_spent_segments`, the recorded
  license in `Oracle.lean`).

## The window inhabitant: acceptance and enforcement discharged

The engine ACCEPTS the form at declaration (2026-07-14:
`StatementDescriptor::Cardinality`; the gate arm in
`schema/validate.rs` checks exactly this inhabitant's acceptance
premises — the window's target key and interval refusal) and JUDGES
it per commit: the checker and delta machinery this term describes
are `storage/commit/judgment.rs::check_windows` over
`storage/commit/plan.rs`'s touched sets. No
`Bridge.lean` row cites this module directly: the acceptance row
cites the plan theorem (`Oracle.lean`), the enforcement row cites
the delta-restriction theorem (`Txn/DeltaRestriction.lean`), and
the FD and containment fields' mechanisms are already ledgered by
the modules they come from.
-/

namespace Bumbledb
namespace Admission

/-! ## The sanctioned probe shapes -/

/-- The two exact-key probe shapes an admissible form's verdict may
read per consulted surface — the `Oracle.EnforcementPlan` terms that
answer at one key. -/
inductive ProbeShape where
  /-- One lookup at the touched key. -/
  | point
  /-- One entry seek + the ordered walk of the touched key's group. -/
  | walk

/-- The plan term a shape names at one key. -/
def ProbeShape.toPlan {K P : Type} :
    ProbeShape → K → Oracle.EnforcementPlan K P
  | .point, t => .pointProbe t
  | .walk, t => .prefixWalk t

/-- Both shapes answer with the key's consultation — the verdict's
one read. -/
theorem ProbeShape.toPlan_answers {K P β : Type} {ple : P → P → Prop}
    (o : Oracle.OrderedOracle K P β ple) (s : ProbeShape) (t : K) :
    (s.toPlan (P := P) t).answers o = o.consult t := by
  cases s <;> rfl

/-- The point probe's price: one descent
(`Oracle.EnforcementPlan.consultations`; honest on a keyed surface —
`Oracle.point_probe_honest`). -/
theorem ProbeShape.point_consultations {K P β : Type}
    {ple : P → P → Prop} (o : Oracle.OrderedOracle K P β ple) (t : K) :
    ((ProbeShape.point.toPlan (P := P) t).consultations o) = 1 :=
  rfl

/-- The prefix walk's price: the entry seek plus one read per walked
group member — the touched-window term of the gate's cost law. -/
theorem ProbeShape.walk_consultations {K P β : Type}
    {ple : P → P → Prop} (o : Oracle.OrderedOracle K P β ple) (t : K) :
    ((ProbeShape.walk.toPlan (P := P) t).consultations o) =
      1 + (o.consult t).length :=
  rfl

/-! ## The acceptance gate, as a type -/

/-- **The acceptance gate's checklist as a structure.** A statement
form is accepted exactly by exhibiting a term: its denotation, its
consulted surfaces, quarantine compliance, the executable judge, the
delta restriction, and the oracle plan — field by field the module
doc's law. `Param` is the form's schema-level parameter tuple; `Ix`
indexes the consulted surfaces. -/
structure AdmissibleForm (Param : Type) (Ix : Type) where
  /-- Level 0 — the form's judgment, per parameter. -/
  Judgment : Param → Theory → Instance → Prop
  /-- A consulted surface: the stored fact set the judge reads. -/
  surface : Param → Ix → Theory → Instance → Set Fact
  /-- The surface's grouping projection — the index key its probes
  descend by. -/
  surfaceProj : Param → Ix → List FieldId
  /-- Creation-quarantine compliance: the judgment factors through
  the consulted surfaces — stored values only. -/
  quarantined : ∀ p T I J,
    (∀ ix, surface p ix T I = surface p ix T J) →
    (Judgment p T I ↔ Judgment p T J)
  /-- The executable checker (`Decide.lean`'s altitude). -/
  check : Param → Theory → RowInstance → Bool
  /-- The checker's OWN acceptance premise, per instance — the
  discipline of the tree (`Decide.lean`: acceptance enters as a
  hypothesis, never a denotation conjunct), made a declared field so
  a form states loudly what its executable judge spends. Every
  inhabitant below declares the trivial premise; a form whose checker
  needs more (the ranked form's hop-key rule) must declare it here,
  in the open. -/
  checkPremise : Param → Theory → RowInstance → Prop
  /-- The checker is sound and complete against the denotation on
  row-listed instances, under the merge premise and the form's
  declared checker premise. -/
  check_decides : ∀ p T (W : RowInstance), WorldCarriesClosed T W →
    checkPremise p T W →
    (check p T W = true ↔ Judgment p T W.den)
  /-- The delta-restricted check (`Txn/DeltaRestriction.lean`'s
  altitude). -/
  DeltaCheck : Param → Theory → Instance → Txn.Delta → Prop
  /-- The restriction theorem: over a holding pre-state, the final
  state satisfies the judgment IFF the restricted check passes. -/
  delta_restricts : ∀ p T I (d : Txn.Delta), Judgment p T I →
    (Judgment p T (d.applyTo I) ↔ DeltaCheck p T I d)
  /-- The touched keys a delta licenses the judge to probe. -/
  Touched : Param → Txn.Delta → Set (List Value)
  /-- Every touched key is delta-derived — the probe count is bounded
  by the delta, the cost law's first half. -/
  touched_delta_bounded : ∀ p (d : Txn.Delta) t, t ∈ Touched p d →
    ∃ ix R f, (f ∈ d.adds R ∨ f ∈ d.removes R) ∧
      f.project (surfaceProj p ix) = t
  /-- The probe shape per consulted surface — the plan term, priced
  by `ProbeShape.point_consultations` / `walk_consultations`. -/
  probe : Param → Ix → ProbeShape
  /-- The per-touched-key verdict over the probes' answers. -/
  Verdict : Param → Txn.Delta → List Value → (Ix → List Fact) → Prop
  /-- The oracle-plan theorem: against EVERY conforming oracle
  family over the surfaces, the touched verdicts decide the
  delta-restricted check — nothing outside the touched groups is
  read, the cost law's second half. -/
  plan_decides : ∀ p T I (d : Txn.Delta) (P : Type)
    (ple : P → P → Prop)
    (o : Ix → Oracle.OrderedOracle (List Value) P Fact ple),
    (∀ ix, (o ix).facts = surface p ix T (d.applyTo I)) →
    (∀ ix f, (o ix).groupOf f = f.project (surfaceProj p ix)) →
    ((∀ t, t ∈ Touched p d →
        Verdict p d t
          (fun ix => ((probe p ix).toPlan t).answers (o ix))) ↔
      DeltaCheck p T I d)

/-- **The composition** — the gate's whole read pattern per form:
over a holding pre-state, the touched verdicts against any conforming
oracle family ARE the final-state judgment (`plan_decides` chained
through `delta_restricts`). -/
theorem AdmissibleForm.gate_decides_final {Param Ix : Type}
    (F : AdmissibleForm Param Ix) (p : Param) (T : Theory)
    (I : Instance) (d : Txn.Delta) (hpre : F.Judgment p T I)
    (P : Type) (ple : P → P → Prop)
    (o : Ix → Oracle.OrderedOracle (List Value) P Fact ple)
    (hfacts : ∀ ix, (o ix).facts = F.surface p ix T (d.applyTo I))
    (hkeys : ∀ ix f,
      (o ix).groupOf f = f.project (F.surfaceProj p ix)) :
    ((∀ t, t ∈ F.Touched p d →
        F.Verdict p d t
          (fun ix => ((F.probe p ix).toPlan t).answers (o ix))) ↔
      F.Judgment p T (d.applyTo I)) :=
  (F.plan_decides p T I d P ple o hfacts hkeys).trans
    (F.delta_restricts p T I d hpre).symm

/-! ## Inhabitant 1 — functionality, scalar

Denotation `Functionality` (`Dependencies.lean`); checker `funcB`
(`funcB_iff`); restriction `fd_delta_restriction`; plan
`fd_plan_decides` at one point probe per touched determinant tuple
(`fd_plan_consultations`). -/

/-- The scalar functionality form: `R(X) -> R` on an all-scalar
determinant. -/
def functionalityForm : AdmissibleForm (RelId × List FieldId) Unit where
  Judgment := fun p T I => Functionality (T.den I p.1) p.2
  surface := fun p _ T I => T.den I p.1
  surfaceProj := fun p _ => p.2
  quarantined := by
    intro p T I J h
    have hden : T.den I p.1 = T.den J p.1 := h ()
    show Functionality (T.den I p.1) p.2 ↔
      Functionality (T.den J p.1) p.2
    rw [hden]
  check := fun p T W => funcB (W.rows p.1) p.2
  checkPremise := fun _ _ _ => True
  check_decides := fun p T W hclosed _ =>
    funcB_iff (theoryDen_denotes hclosed p.1) p.2
  DeltaCheck := fun p T I d => Txn.fdDeltaCheck T I d p.1 p.2
  delta_restricts := fun p T I d hpre => Txn.fd_delta_restriction hpre
  Touched := fun p d => d.projected p.1 p.2
  touched_delta_bounded := by
    rintro p d t ⟨f, hf, hproj⟩
    exact ⟨(), p.1, f, hf, hproj⟩
  probe := fun _ _ => .point
  Verdict := fun _ _ _ ans => Oracle.collisionFree (ans ())
  plan_decides := by
    intro p T I d P ple o hfacts hkeys
    exact Oracle.fd_plan_decides T I d p.1 p.2 P ple (o ())
      (hfacts ()) (fun f => hkeys () f)

/-- Under the scalar scope, the form's judgment IS the statement
dispatcher's arm — the tie to the closed vocabulary. -/
theorem functionalityForm_denotes {T : Theory} {I : Instance}
    {R : RelId} {X : List FieldId}
    (hscalar : T.header.intervalSplit R X = none) :
    functionalityForm.Judgment (R, X) T I ↔
      (Statement.functionality R X).judgment T I := by
  show Functionality (T.den I R) X ↔ _
  simp only [Statement.judgment, hscalar]

/-! ## Inhabitant 2 — containment, scalar

Denotation `Containment` (`Dependencies.lean`); checker `containB`
(`containB_iff`); restriction `containment_delta_restriction`; plan
`containment_plan_decides` — per touched key one KEYED target-index
point probe (`ind_source_plan_consultations` /
`ind_reestablish_consultations`; the target-key acceptance premise
prices the unit probe, `accepted_target_key_prices_the_probe`) and
one WALK of the source-index bucket
(`ind_reverse_walk_consultations` — the source grouping is unkeyed
by design, so its read is priced at the walked bucket, never a flat
count). -/

/-- The scalar containment form: `A(X | φ) <= B(Y | ψ)` with scalar
projections. `Ix = Bool`: `true` the source surface, `false` the
target surface. -/
def containmentForm : AdmissibleForm (Atom × Atom) Bool where
  Judgment := fun p T I =>
    Containment (T.den I p.1.relation) p.1.selection p.1.projection
      (T.den I p.2.relation) p.2.selection p.2.projection
  surface := fun p ix T I =>
    match ix with
    | true => T.den I p.1.relation
    | false => T.den I p.2.relation
  surfaceProj := fun p ix =>
    match ix with
    | true => p.1.projection
    | false => p.2.projection
  quarantined := by
    intro p T I J h
    have hs : T.den I p.1.relation = T.den J p.1.relation := h true
    have ht : T.den I p.2.relation = T.den J p.2.relation := h false
    show Containment (T.den I p.1.relation) p.1.selection
        p.1.projection (T.den I p.2.relation) p.2.selection
        p.2.projection ↔
      Containment (T.den J p.1.relation) p.1.selection p.1.projection
        (T.den J p.2.relation) p.2.selection p.2.projection
    rw [hs, ht]
  check := fun p T W =>
    containB (W.rows p.1.relation) p.1.selection p.1.projection
      (W.rows p.2.relation) p.2.selection p.2.projection
  checkPremise := fun _ _ _ => True
  check_decides := fun p T W hclosed _ =>
    containB_iff (theoryDen_denotes hclosed p.1.relation)
      (theoryDen_denotes hclosed p.2.relation) p.1.selection
      p.1.projection p.2.selection p.2.projection
  DeltaCheck := fun p T I d => Txn.containmentDeltaCheck T I d p.1 p.2
  delta_restricts := fun p T I d hpre =>
    Txn.containment_delta_restriction hpre
  Touched := fun p d t =>
    (∃ f, f ∈ d.adds p.1.relation ∧ f.project p.1.projection = t) ∨
    (∃ g, g ∈ d.removes p.2.relation ∧ g.project p.2.projection = t)
  touched_delta_bounded := by
    rintro p d t (⟨f, hf, hproj⟩ | ⟨g, hg, hproj⟩)
    · exact ⟨true, p.1.relation, f, Or.inl hf, hproj⟩
    · exact ⟨false, p.2.relation, g, Or.inr hg, hproj⟩
  probe := fun _ ix =>
    match ix with
    | true => .walk
    | false => .point
  Verdict := fun p d t ans =>
    (∀ f, f ∈ ans true → f ∈ d.adds p.1.relation →
      p.1.selection.satisfies f →
      Oracle.witnessed p.2.selection (ans false)) ∧
    ((∃ g, g ∈ d.removes p.2.relation ∧ p.2.selection.satisfies g ∧
        g.project p.2.projection = t) →
      Oracle.witnessed p.2.selection (ans false) ∨
      ¬ Oracle.demanded p.1.selection (ans true))
  plan_decides := by
    intro p T I d P ple o hfacts hkeys
    have hplan := Oracle.containment_plan_decides T I d p.1 p.2 P P
      ple ple (o true) (o false) (hfacts true) (fun f => hkeys true f)
      (hfacts false) (fun g => hkeys false g)
    have hconS : ∀ (t : List Value) (f : Fact),
        f ∈ (o true).consult t ↔
          f ∈ T.den (d.applyTo I) p.1.relation ∧
            f.project p.1.projection = t := by
      intro t f
      rw [(o true).consult_mem t f, hfacts true, hkeys true f]
    refine Iff.trans ?_ hplan
    constructor
    · intro hv
      constructor
      · intro f hadd hfin hφ
        exact (hv (f.project p.1.projection)
            (Or.inl ⟨f, hadd, rfl⟩)).1 f
          ((hconS _ f).mpr ⟨hfin, rfl⟩) hadd hφ
      · intro g hrem hψ
        exact (hv (g.project p.2.projection)
          (Or.inr ⟨g, hrem, rfl⟩)).2 ⟨g, hrem, hψ, rfl⟩
    · intro ha t ht
      constructor
      · intro f hfans hadd hφ
        obtain ⟨hfin, hfp⟩ := (hconS t f).mp hfans
        have hw := ha.1 f hadd hfin hφ
        rw [hfp] at hw
        exact hw
      · rintro ⟨g, hrem, hψ, hgp⟩
        have hw := ha.2 g hrem hψ
        rw [hgp] at hw
        exact hw

/-- Under the scalar scope (no interval split on the source side),
the containment form's judgment IS the statement dispatcher's arm. -/
theorem containmentForm_denotes {T : Theory} {I : Instance}
    {src tgt : Atom}
    (hs : T.header.intervalSplit src.relation src.projection = none) :
    containmentForm.Judgment (src, tgt) T I ↔
      (Statement.containment src tgt).judgment T I := by
  show Containment (T.den I src.relation) src.selection src.projection
      (T.den I tgt.relation) tgt.selection tgt.projection ↔ _
  cases ht : T.header.intervalSplit tgt.relation tgt.projection with
  | none => simp only [Statement.judgment, hs, ht]
  | some q => simp only [Statement.judgment, hs, ht]

/-! ## Inhabitant 3 — the cardinality window (accepted and enforced)

Denotation `CardinalityWindow` (`Cardinality.lean`); checker
`cardinalityB` (`cardinalityB_iff`); restriction
`cardinality_delta_restriction`; plan `cardinality_plan_decides` —
per touched parent one target-key point probe and one prefix walk of
the child group (`window_plan_consultations`, the equation). -/

/-- The cardinality-window form: `A(X | φ) in w per B(Y | ψ)`.
`Ix = Bool`: `true` the σ-selected child surface, `false` the parent
surface. -/
def cardinalityForm : AdmissibleForm (Atom × Window × Atom) Bool where
  Judgment := fun p T I =>
    CardinalityWindow (T.den I p.1.relation) p.1.selection
      p.1.projection p.2.1 (T.den I p.2.2.relation) p.2.2.selection
      p.2.2.projection
  surface := fun p ix T I =>
    match ix with
    | true => Selected (T.den I p.1.relation) p.1.selection
    | false => T.den I p.2.2.relation
  surfaceProj := fun p ix =>
    match ix with
    | true => p.1.projection
    | false => p.2.2.projection
  quarantined := by
    intro p T I J h
    have hsel : Selected (T.den I p.1.relation) p.1.selection =
        Selected (T.den J p.1.relation) p.1.selection := h true
    have ht : T.den I p.2.2.relation = T.den J p.2.2.relation :=
      h false
    have hgrp : ∀ t, ChildGroup (T.den I p.1.relation) p.1.selection
        p.1.projection t =
          ChildGroup (T.den J p.1.relation) p.1.selection
            p.1.projection t := by
      intro t
      funext f
      apply propext
      constructor
      · rintro ⟨h1, h2, h3⟩
        have hf : f ∈ Selected (T.den J p.1.relation)
            p.1.selection := by
          rw [← hsel]
          exact ⟨h1, h2⟩
        exact ⟨hf.1, hf.2, h3⟩
      · rintro ⟨h1, h2, h3⟩
        have hf : f ∈ Selected (T.den I p.1.relation)
            p.1.selection := by
          rw [hsel]
          exact ⟨h1, h2⟩
        exact ⟨hf.1, hf.2, h3⟩
    show CardinalityWindow (T.den I p.1.relation) p.1.selection
        p.1.projection p.2.1 (T.den I p.2.2.relation)
        p.2.2.selection p.2.2.projection ↔
      CardinalityWindow (T.den J p.1.relation) p.1.selection
        p.1.projection p.2.1 (T.den J p.2.2.relation)
        p.2.2.selection p.2.2.projection
    constructor
    · intro hcw g hg hψ
      rw [← hgrp (g.project p.2.2.projection)]
      refine hcw g ?_ hψ
      rw [ht]
      exact hg
    · intro hcw g hg hψ
      rw [hgrp (g.project p.2.2.projection)]
      refine hcw g ?_ hψ
      rw [← ht]
      exact hg
  check := fun p T W =>
    cardinalityB (W.rows p.1.relation) p.1.selection p.1.projection
      p.2.1 (W.rows p.2.2.relation) p.2.2.selection p.2.2.projection
  checkPremise := fun _ _ _ => True
  check_decides := fun p T W hclosed _ =>
    cardinalityB_iff (theoryDen_denotes hclosed p.1.relation)
      (theoryDen_denotes hclosed p.2.2.relation) p.1.selection
      p.1.projection p.2.1 p.2.2.selection p.2.2.projection
  DeltaCheck := fun p T I d =>
    Txn.cardinalityDeltaCheck T I d p.1 p.2.1 p.2.2
  delta_restricts := fun p T I d hpre =>
    Txn.cardinality_delta_restriction hpre
  Touched := fun p d => Txn.touchedParents d p.1 p.2.2
  touched_delta_bounded := by
    rintro p d t (⟨f, hf, hproj⟩ | ⟨g, hg, _, hproj⟩)
    · exact ⟨true, p.1.relation, f, hf, hproj⟩
    · exact ⟨false, p.2.2.relation, g, hg, hproj⟩
  probe := fun _ ix =>
    match ix with
    | true => .walk
    | false => .point
  Verdict := fun p _ _ ans =>
    Oracle.witnessed p.2.2.selection (ans false) →
      Oracle.windowVerdict p.2.1 (ans true)
  plan_decides := by
    intro p T I d P ple o hfacts hkeys
    have hplan := Oracle.cardinality_plan_decides T I d p.1 p.2.1
      p.2.2 P ple (o true) (hfacts true) (fun f => hkeys true f)
    have hconT : ∀ (t : List Value) (g : Fact),
        g ∈ (o false).consult t ↔
          g ∈ T.den (d.applyTo I) p.2.2.relation ∧
            g.project p.2.2.projection = t := by
      intro t g
      rw [(o false).consult_mem t g, hfacts false, hkeys false g]
    refine Iff.trans ?_ hplan
    constructor
    · intro hv g hg hψ ht
      exact hv (g.project p.2.2.projection) ht
        ⟨g, (hconT _ g).mpr ⟨hg, rfl⟩, hψ⟩
    · intro hw t ht hwit
      obtain ⟨g, hgc, hψ⟩ := hwit
      obtain ⟨hgf, hgp⟩ := (hconT t g).mp hgc
      have hv := hw g hgf hψ (by rw [hgp]; exact ht)
      rw [hgp] at hv
      exact hv

/-- The window form's judgment IS the statement dispatcher's arm —
no split scope: window projections refuse interval positions at the
gate. -/
theorem cardinalityForm_denotes {T : Theory} {I : Instance}
    {src : Atom} {w : Window} {tgt : Atom} :
    cardinalityForm.Judgment (src, w, tgt) T I ↔
      (Statement.cardinality src w tgt).judgment T I :=
  Iff.rfl

/-! ## Inhabitant 4 — functionality, pointwise (the interval FD)

Denotation `PointwiseKey` (`Dependencies.lean`); checker
`pointwiseKeyB` (`pointwiseKeyB_iff`); restriction
`pointwise_delta_restriction`; plan: one prefix walk of the touched
scalar-prefix group, verdict pairwise point-disjointness over the
walked group. The engine's two-neighbor refinement of that walk is
`Oracle.neighbor_probe_decides` at the interval altitude — mechanism
below this structure's fact-altitude oracle field; a wider walk only
re-reads more (the recorded superset license, `Oracle.lean`). -/

/-- The pointwise functionality form: `R(S…, i) -> R` — the scalar
prefix `S` grouping and the interval position `i`, `intervalSplit`'s
image of the written determinant. -/
def pointwiseForm :
    AdmissibleForm (RelId × List FieldId × FieldId) Unit where
  Judgment := fun p T I => PointwiseKey (T.den I p.1) p.2.1 p.2.2
  surface := fun p _ T I => T.den I p.1
  surfaceProj := fun p _ => p.2.1
  quarantined := by
    intro p T I J h
    have hden : T.den I p.1 = T.den J p.1 := h ()
    show PointwiseKey (T.den I p.1) p.2.1 p.2.2 ↔
      PointwiseKey (T.den J p.1) p.2.1 p.2.2
    rw [hden]
  check := fun p T W => pointwiseKeyB (W.rows p.1) p.2.1 p.2.2
  checkPremise := fun _ _ _ => True
  check_decides := fun p T W hclosed _ =>
    pointwiseKeyB_iff (theoryDen_denotes hclosed p.1) p.2.1 p.2.2
  DeltaCheck := fun p T I d =>
    Txn.pointwiseDeltaCheck T I d p.1 p.2.1 p.2.2
  delta_restricts := fun p T I d hpre =>
    Txn.pointwise_delta_restriction hpre
  Touched := fun p d => d.projected p.1 p.2.1
  touched_delta_bounded := by
    rintro p d t ⟨f, hf, hproj⟩
    exact ⟨(), p.1, f, hf, hproj⟩
  probe := fun _ _ => .walk
  Verdict := fun p _ _ ans =>
    ∀ f g, f ∈ ans () → g ∈ ans () → f ≠ g →
      ∀ x, x ∈ (f p.2.2).points → x ∉ (g p.2.2).points
  plan_decides := by
    intro p T I d P ple o hfacts hkeys
    have hmem : ∀ (t : List Value) (f : Fact),
        f ∈ ((ProbeShape.walk.toPlan (P := P) t).answers (o ())) ↔
          f ∈ T.den (d.applyTo I) p.1 ∧ f.project p.2.1 = t := by
      intro t f
      rw [ProbeShape.toPlan_answers, (o ()).consult_mem t f,
        hfacts (), hkeys () f]
    constructor
    · intro hv f g hf hg ht hproj hne x hx
      exact hv (f.project p.2.1) ht f g
        ((hmem _ f).mpr ⟨hf, rfl⟩)
        ((hmem _ g).mpr ⟨hg, hproj.symm⟩) hne x hx
    · intro hc t ht f g hf hg hne x hx
      obtain ⟨hfd, hfp⟩ := (hmem t f).mp hf
      obtain ⟨hgd, hgp⟩ := (hmem t g).mp hg
      exact hc f g hfd hgd (by rw [hfp]; exact ht)
        (hfp.trans hgp.symm) hne x hx

/-- Under the pointwise scope (an interval split on the determinant),
the form's judgment IS the statement dispatcher's arm. -/
theorem pointwiseForm_denotes {T : Theory} {I : Instance} {R : RelId}
    {X S : List FieldId} {i : FieldId}
    (hsplit : T.header.intervalSplit R X = some (S, i)) :
    pointwiseForm.Judgment (R, S, i) T I ↔
      (Statement.functionality R X).judgment T I := by
  show PointwiseKey (T.den I R) S i ↔ _
  simp only [Statement.judgment, hsplit]

/-! ## Inhabitant 5 — containment, pointwise (coverage)

Denotation `Coverage` (`Dependencies.lean`); checker `coverageB`
(`coverageB_iff` — the proved sweep inside); restriction
`coverage_delta_restriction`; plan: one prefix walk per side of the
touched group, verdict the touched-window point covering over the
two walked groups. The engine's one-pass sweep of the walk is
`Oracle.coverage_walk_decides` at the interval altitude, under the
`DisjointDeterminantProof` premise — this inhabitant states the
covering verdict itself, the sweep being its executable reading. -/

/-- One side of the pointwise containment: relation, selection, the
scalar-prefix grouping, and the interval position —
`intervalSplit`'s image of one written atom. -/
abbrev CoverageSide : Type :=
  RelId × Selection × List FieldId × FieldId

/-- The coverage form: `A(S…, i | φ) <= B(U…, j | ψ)`. `Ix = Bool`:
`true` the source surface, `false` the target surface. -/
def coverageForm : AdmissibleForm (CoverageSide × CoverageSide) Bool where
  Judgment := fun p T I =>
    Coverage (T.den I p.1.1) p.1.2.1 p.1.2.2.1 p.1.2.2.2
      (T.den I p.2.1) p.2.2.1 p.2.2.2.1 p.2.2.2.2
  surface := fun p ix T I =>
    match ix with
    | true => T.den I p.1.1
    | false => T.den I p.2.1
  surfaceProj := fun p ix =>
    match ix with
    | true => p.1.2.2.1
    | false => p.2.2.2.1
  quarantined := by
    intro p T I J h
    have hs : T.den I p.1.1 = T.den J p.1.1 := h true
    have ht : T.den I p.2.1 = T.den J p.2.1 := h false
    show Coverage (T.den I p.1.1) p.1.2.1 p.1.2.2.1 p.1.2.2.2
        (T.den I p.2.1) p.2.2.1 p.2.2.2.1 p.2.2.2.2 ↔
      Coverage (T.den J p.1.1) p.1.2.1 p.1.2.2.1 p.1.2.2.2
        (T.den J p.2.1) p.2.2.1 p.2.2.2.1 p.2.2.2.2
    rw [hs, ht]
  check := fun p T W =>
    coverageB (W.rows p.1.1) p.1.2.1 p.1.2.2.1 p.1.2.2.2
      (W.rows p.2.1) p.2.2.1 p.2.2.2.1 p.2.2.2.2
  checkPremise := fun _ _ _ => True
  check_decides := fun p T W hclosed _ =>
    coverageB_iff (theoryDen_denotes hclosed p.1.1)
      (theoryDen_denotes hclosed p.2.1) p.1.2.1 p.1.2.2.1 p.1.2.2.2
      p.2.2.1 p.2.2.2.1 p.2.2.2.2
  DeltaCheck := fun p T I d =>
    Txn.coverageDeltaCheck T I d p.1.1 p.1.2.1 p.1.2.2.1 p.1.2.2.2
      p.2.1 p.2.2.1 p.2.2.2.1 p.2.2.2.2
  delta_restricts := fun p T I d hpre =>
    Txn.coverage_delta_restriction hpre
  Touched := fun p d t =>
    t ∈ d.projected p.1.1 p.1.2.2.1 ∨ t ∈ d.projected p.2.1 p.2.2.2.1
  touched_delta_bounded := by
    rintro p d t (⟨f, hf, hproj⟩ | ⟨g, hg, hproj⟩)
    · exact ⟨true, p.1.1, f, hf, hproj⟩
    · exact ⟨false, p.2.1, g, hg, hproj⟩
  probe := fun _ _ => .walk
  Verdict := fun p d t ans =>
    ∀ f, f ∈ ans true → p.1.2.1.satisfies f →
      ∀ x, x ∈ (f p.1.2.2.2).points →
        x ∈ Txn.touchedWindow d p.1.1 p.1.2.2.1 p.1.2.2.2 p.2.1
          p.2.2.2.1 p.2.2.2.2 t →
        ∃ g, g ∈ ans false ∧ p.2.2.1.satisfies g ∧
          x ∈ (g p.2.2.2.2).points
  plan_decides := by
    intro p T I d P ple o hfacts hkeys
    have hmemS : ∀ (t : List Value) (f : Fact),
        f ∈ ((ProbeShape.walk.toPlan (P := P) t).answers (o true)) ↔
          f ∈ T.den (d.applyTo I) p.1.1 ∧
            f.project p.1.2.2.1 = t := by
      intro t f
      rw [ProbeShape.toPlan_answers, (o true).consult_mem t f,
        hfacts true, hkeys true f]
    have hmemT : ∀ (t : List Value) (g : Fact),
        g ∈ ((ProbeShape.walk.toPlan (P := P) t).answers (o false)) ↔
          g ∈ T.den (d.applyTo I) p.2.1 ∧
            g.project p.2.2.2.1 = t := by
      intro t g
      rw [ProbeShape.toPlan_answers, (o false).consult_mem t g,
        hfacts false, hkeys false g]
    constructor
    · intro hv f hf hφ x hx hxw
      have ht : f.project p.1.2.2.1 ∈ d.projected p.1.1 p.1.2.2.1 ∨
          f.project p.1.2.2.1 ∈ d.projected p.2.1 p.2.2.2.1 := by
        rcases hxw with ⟨f', hf', hproj, -⟩ | ⟨g', hg', hproj, -⟩
        · exact Or.inl ⟨f', hf', hproj⟩
        · exact Or.inr ⟨g', hg', hproj⟩
      obtain ⟨g, hgans, hψ, hxg⟩ := hv (f.project p.1.2.2.1) ht f
        ((hmemS _ f).mpr ⟨hf, rfl⟩) hφ x hx hxw
      obtain ⟨hgd, hgp⟩ := (hmemT _ g).mp hgans
      exact ⟨g, hgd, hψ, hgp, hxg⟩
    · intro hc t ht f hfans hφ x hx hxw
      obtain ⟨hfd, hfp⟩ := (hmemS t f).mp hfans
      obtain ⟨g, hgd, hψ, hgp, hxg⟩ :=
        hc f hfd hφ x hx (by rw [hfp]; exact hxw)
      exact ⟨g, (hmemT t g).mpr ⟨hgd, hgp.trans hfp⟩, hψ, hxg⟩

/-- Under the pointwise scope (interval splits on both sides), the
coverage form's judgment IS the statement dispatcher's arm. -/
theorem coverageForm_denotes {T : Theory} {I : Instance}
    {src tgt : Atom} {S U : List FieldId} {i j : FieldId}
    (hs : T.header.intervalSplit src.relation src.projection =
      some (S, i))
    (ht : T.header.intervalSplit tgt.relation tgt.projection =
      some (U, j)) :
    coverageForm.Judgment
        ((src.relation, src.selection, S, i),
         (tgt.relation, tgt.selection, U, j)) T I ↔
      (Statement.containment src tgt).judgment T I := by
  show Coverage (T.den I src.relation) src.selection S i
      (T.den I tgt.relation) tgt.selection U j ↔ _
  simp only [Statement.judgment, hs, ht]

end Admission
end Bumbledb
