import Bumbledb.Txn.DeltaRestriction

/-!
# Support — mutable consulted relations (the ASS-001 successor)

The retired braid module (`Txn/Braids.lean`, deleted) proved locality
from `ComponentClosed`: a partition of ALL consulted relations —
closed targets included — into components. The Rust support
derivation never consulted closed relations' edges, so the theorem's
premise was strictly stronger than the runtime premise (audit
ASS-001). This module replaces it with the support the engine
actually computes: **the mutable consulted relations of one
statement** — its consulted relations minus the theory's closed
(ground-axiom) relations, whose denotations are constants of the
theory (`den_closed_constant`).

The successor theorems:

* `judgment_stable_outside_mutable_support` — a delta that touches no
  relation in one statement's MUTABLE support leaves that statement's
  judgment unchanged, while every closed denotation remains fixed
  (`den_closed_stable`). Shared closed vocabulary therefore never
  merges two mutable supports (`closed_not_mutable`,
  `disjoint_mutable_locality`).
* `holds_stable_outside_mutable_support` — the whole-theory face.
* Under the new log design these theorems justify **scoped
  admission/planning work only**. They are NOT a distributed commit
  lane, causal read cut, or publication independence claim: tenant
  history is ordered by the log's single authority (chapter 20), and
  no per-component publication exists to be certified. The braid
  theorems L9/L10 are retired with their premises, not relabeled.

This module also carries the one-command normalization laws the
successor delta contract fixes (chapter 10 §1, CONC-01):

* `normalize_applyTo` — canonicalizing `(A, D)` to `(A, D \ A)`
  changes nothing: `apply` already reads add-wins.
* `normalize_idempotent` — normalization is idempotent.
* `add_wins` — the same exact fact on both sides of ONE command lands
  present; this is a same-command tie rule, never cross-command
  conflict resolution (sequential commands keep their order:
  `applyTo_comm_of_disjoint` states the only raw commutation and its
  doc records what it does NOT license).
* `applyTo_comm_of_disjoint` — for two normalized deltas with no
  cross add/remove conflicts, the raw SET transformations commute.
  This is deliberately stated at its real strength: it says nothing
  about admission outcomes, exact-state witnesses or receipts — a
  disjoint pair can still interact through a shared capacity law
  (chapter 02; the union counterexamples live in the bench admission
  model, `crates/bumbledb-bench/src/naive/successor/`).

## Narrowings recorded (law 5: narrow and record)

* **Support is a relation LIST, filtered.** The engine's support
  derivation deduplicates and indexes; the model keeps the statement's
  own consulted list filtered by closedness — same set.
* **`touches` is mode-blind.** A delta touches `R` at `f` when `f` is
  in either net set at `R`; the engine's plan reads the same coalesced
  pair.
* **No component function survives.** Locality is stated per
  statement against its own mutable support; any grouping of
  statements into components is planner mechanism the theorem does
  not consult.
-/

namespace Bumbledb
namespace Txn

/-! ## The touched facts of a batch -/

/-- The facts one batch writes at `R`: its net insert and delete
sets, mode-blind — the touched notion every locality argument
projects. -/
def Delta.touches (d : Delta) (R : RelId) (f : Fact) : Prop :=
  f ∈ d.adds R ∨ f ∈ d.removes R

/-- An untouched fact is outside the insert set. -/
theorem Delta.not_adds_of_not_touches {d : Delta} {R : RelId}
    {f : Fact} (h : ¬ d.touches R f) : f ∉ d.adds R :=
  fun hx => h (Or.inl hx)

/-- An untouched fact is outside the delete set. -/
theorem Delta.not_removes_of_not_touches {d : Delta} {R : RelId}
    {f : Fact} (h : ¬ d.touches R f) : f ∉ d.removes R :=
  fun hx => h (Or.inr hx)

namespace Support

/-! ## Congruence — one relation's slice decides everything read
there -/

/-- Raw presence is stable at an untouched fact — one apply layer. -/
theorem applyTo_untouched {d : Delta} {R : RelId} {f : Fact}
    (h : ¬ d.touches R f) (I : Instance) :
    f ∈ d.applyTo I R ↔ f ∈ I R := by
  rw [mem_applyTo]
  constructor
  · rintro (⟨h1, _⟩ | h2)
    · exact h1
    · exact absurd h2 (Delta.not_adds_of_not_touches h)
  · intro h1
    exact Or.inl ⟨h1, Delta.not_removes_of_not_touches h⟩

/-- Application at one relation reads only that relation's pre-state:
equal pre-slices at `R` land equal final slices at `R`. -/
theorem applyTo_congr_at {d : Delta} {J J' : Instance} {R : RelId}
    (h : J R = J' R) : d.applyTo J R = d.applyTo J' R := by
  funext f
  simp only [Delta.applyTo]
  rw [h]

/-- The denotation at one relation reads only that relation's
instance slice — a closed relation reads its sealed axioms either
way. -/
theorem den_congr {T : Theory} {J J' : Instance} {R : RelId}
    (h : J R = J' R) : T.den J R = T.den J' R := by
  cases hc : T.closed R with
  | some ext => exact den_closed_constant hc J J'
  | none =>
    simp only [Theory.den, hc]
    exact h

/-! ## The mutable consulted support -/

/-- The relations one statement consults — one hyperedge over its
relations. This is the statement's whole read/write footprint; the
mutable support below filters it. -/
def stmtRels : Statement → List RelId
  | .functionality R _ => [R]
  | .containment src tgt => [src.relation, tgt.relation]
  | .capacity tgt _ _ src => [src.relation, tgt.relation]

/-- **The mutable consulted relations of one statement**: its
consulted relations that are NOT closed under the theory. This is the
support the Rust derivation actually computes — closed (ground-axiom)
relations denote theory constants and contribute no mutable edge.
Bridge: the successor support derivation in core admission planning
(chapter 13 §4); the old `ComponentClosed` premise over all consulted
relations is retired. -/
def mutableRels (T : Theory) (st : Statement) : List RelId :=
  (stmtRels st).filter (fun R => (T.closed R).isNone)

/-- A closed relation is never in any statement's mutable support —
the reason shared closed vocabulary cannot merge two mutable
components. -/
theorem closed_not_mutable {T : Theory} {st : Statement} {R : RelId}
    {ext : GroundExtension} (h : T.closed R = some ext) :
    R ∉ mutableRels T st := by
  intro hmem
  have hnone := (List.mem_filter.mp hmem).2
  rw [h] at hnone
  exact nomatch hnone

/-- A mutable-support member is consulted and open. -/
theorem mutable_mem_iff {T : Theory} {st : Statement} {R : RelId} :
    R ∈ mutableRels T st ↔ R ∈ stmtRels st ∧ T.closed R = none := by
  constructor
  · intro h
    have hpair := List.mem_filter.mp h
    exact ⟨hpair.1, Option.isNone_iff_eq_none.mp hpair.2⟩
  · intro ⟨hmem, hopen⟩
    refine List.mem_filter.mpr ⟨hmem, ?_⟩
    rw [hopen]
    rfl

/-! ## Closed denotations remain fixed -/

/-- Every closed denotation is fixed across ANY delta application —
no touching hypothesis needed: the sealed extension is a theory
constant. -/
theorem den_closed_stable {T : Theory} {R : RelId}
    {ext : GroundExtension} (h : T.closed R = some ext)
    (d : Delta) (I : Instance) :
    T.den (d.applyTo I) R = T.den I R :=
  den_closed_constant h (d.applyTo I) I

/-! ## The judgment congruence -/

/-- One statement's judgment reads the denotation at its own
consulted relations and nothing else — arm for arm over
`Statement.judgment`'s dispatch. -/
theorem judgment_congr {T : Theory} {J J' : Instance} {st : Statement}
    (h : ∀ R, R ∈ stmtRels st → T.den J R = T.den J' R) :
    st.judgment T J ↔ st.judgment T J' := by
  cases st with
  | functionality R X =>
    have hR := h R List.mem_cons_self
    simp only [Statement.judgment]
    rw [hR]
  | containment src tgt =>
    have hsrc := h src.relation List.mem_cons_self
    have htgt := h tgt.relation
      (List.mem_cons_of_mem _ List.mem_cons_self)
    simp only [Statement.judgment]
    rw [hsrc, htgt]
  | capacity tgt wt w src =>
    have hsrc := h src.relation List.mem_cons_self
    have htgt := h tgt.relation
      (List.mem_cons_of_mem _ List.mem_cons_self)
    simp only [Statement.judgment]
    rw [hsrc, htgt]

/-! ## The mutable-support theorem -/

/-- A delta avoiding a statement's mutable support: it touches no
fact of any relation in `mutableRels T st`. Touching a CLOSED
consulted relation is deliberately allowed by this hypothesis — the
denotation ignores it (`den_closed_stable`); the engine's surface
separately refuses closed writes, which only shrinks the reachable
deltas. -/
def AvoidsMutable (d : Delta) (T : Theory) (st : Statement) : Prop :=
  ∀ R, R ∈ mutableRels T st → ∀ f, ¬ d.touches R f

/-- **The mutable-support theorem (ASS-001 successor).** A delta that
touches no relation of one statement's mutable consulted support
leaves that statement's judgment unchanged — over ANY instance, with
every closed denotation fixed. The old braid theorem demanded a
component function closed over ALL consulted relations, closed
targets included; the runtime support never consulted those edges.
This statement's premise IS the runtime premise. Bridge: the
successor incremental admission planner's support derivation; tested
per accepted statement form by the independent staged/admission
models (`crates/bumbledb-bench/src/naive/successor/`). -/
theorem judgment_stable_outside_mutable_support {T : Theory}
    {d : Delta} {st : Statement} (h : AvoidsMutable d T st)
    (I : Instance) :
    st.judgment T (d.applyTo I) ↔ st.judgment T I := by
  refine judgment_congr fun R hR => ?_
  cases hc : T.closed R with
  | some ext => exact den_closed_stable hc d I
  | none =>
    refine den_congr ?_
    funext f
    refine propext (applyTo_untouched ?_ I)
    exact h R (mutable_mem_iff.mpr ⟨hR, hc⟩) f

/-- The whole-theory face: a delta outside EVERY statement's mutable
support leaves the theory's judgment unchanged. This is the license
for scoped admission work — and only that: it is not a publication
order, causal cut, or per-component commit lane. -/
theorem holds_stable_outside_mutable_support {T : Theory} {d : Delta}
    (h : ∀ st, st ∈ T.statements → AvoidsMutable d T st)
    (I : Instance) : holds T (d.applyTo I) ↔ holds T I := by
  constructor
  · intro hh st hst
    exact (judgment_stable_outside_mutable_support (h st hst) I).mp
      (hh st hst)
  · intro hh st hst
    exact (judgment_stable_outside_mutable_support (h st hst) I).mpr
      (hh st hst)

/-- A batch writing only inside one relation list. -/
def LocalTo (d : Delta) (rs : List RelId) : Prop :=
  ∀ R f, d.touches R f → R ∈ rs

/-- **Disjoint mutable supports are semantically independent** — the
shared-closed-vocabulary corollary: a delta local to one statement's
mutable support leaves any statement with a DISJOINT mutable support
unmoved, even when the two statements share closed consulted
relations. Two mutable components need not merge because their
statements cite one sealed vocabulary. -/
theorem disjoint_mutable_locality {T : Theory} {d : Delta}
    {st st' : Statement} (hloc : LocalTo d (mutableRels T st))
    (hdisj : ∀ R, R ∈ mutableRels T st → R ∉ mutableRels T st')
    (I : Instance) :
    st'.judgment T (d.applyTo I) ↔ st'.judgment T I := by
  refine judgment_stable_outside_mutable_support ?_ I
  intro R hR f ht
  exact hdisj R (hloc R f ht) hR

/-! ## One-command normalization (the delta tie rule, CONC-01) -/

/-- Canonical one-command normalization: `(A, D)` becomes
`(A, D \ A)`. Chapter 10 §1 fixes this as the ONE normal form every
generated/dynamic/wire/scratch/replay path uses. -/
def Delta.normalize (d : Delta) : Delta :=
  ⟨d.adds, fun R f => f ∈ d.removes R ∧ f ∉ d.adds R⟩

/-- Normalization changes nothing: `apply` already reads add-wins, so
`(S \ D) ∪ A = (S \ (D \ A)) ∪ A`. Input iterator/call order cannot
choose the result — the sets carry no order to read. -/
theorem normalize_applyTo (d : Delta) (I : Instance) :
    (Delta.normalize d).applyTo I = d.applyTo I := by
  funext R f
  refine propext ?_
  simp only [Delta.applyTo, Delta.normalize]
  constructor
  · rintro (⟨hi, hnr⟩ | ha)
    · by_cases ha : f ∈ d.adds R
      · exact Or.inr ha
      · exact Or.inl ⟨hi, fun hr => hnr ⟨hr, ha⟩⟩
    · exact Or.inr ha
  · rintro (⟨hi, hnr⟩ | ha)
    · exact Or.inl ⟨hi, fun ⟨hr, _⟩ => hnr hr⟩
    · exact Or.inr ha

/-- Normalization is idempotent — the canonical form is a fixed
point. -/
theorem normalize_idempotent (d : Delta) :
    Delta.normalize (Delta.normalize d) = Delta.normalize d := by
  show Delta.mk _ _ = Delta.mk _ _
  refine congrArg (Delta.mk d.adds) ?_
  funext R f
  refine propext ⟨fun h => ⟨h.1.1, h.2⟩, fun h => ⟨⟨h.1, h.2⟩, h.2⟩⟩

/-- A normalized delta removes nothing it adds. -/
theorem normalize_no_conflict (d : Delta) (R : RelId) (f : Fact) :
    f ∈ (Delta.normalize d).adds R → f ∉ (Delta.normalize d).removes R :=
  fun ha ⟨_, hna⟩ => hna ha

/-- **Add wins inside one command**: the exact same fact spelled on
both sides of ONE atomic command lands present. This is a
normalization rule inside one command — never last-writer-wins or
add-wins conflict resolution BETWEEN independently published
commands, whose order stays the log's. -/
theorem add_wins (d : Delta) (I : Instance) {R : RelId} {f : Fact}
    (h : f ∈ d.adds R) : f ∈ d.applyTo I R :=
  Or.inr h

/-- Adding a fact already present is an ordinary no-op at that fact. -/
theorem add_present_noop (d : Delta) (I : Instance) {R : RelId}
    {f : Fact} (hi : f ∈ I R) (hnr : f ∉ d.removes R) :
    f ∈ d.applyTo I R :=
  Or.inl ⟨hi, hnr⟩

/-- Removing an absent fact is an ordinary no-op at that fact. -/
theorem remove_absent_noop (d : Delta) (I : Instance) {R : RelId}
    {f : Fact} (hni : f ∉ I R) (hna : f ∉ d.adds R) :
    f ∉ d.applyTo I R := by
  rintro (⟨hi, _⟩ | ha)
  · exact hni hi
  · exact hna ha

/-! ## Raw commutation, stated at its real strength (CONC-01) -/

/-- **Raw set-transformation commutation.** For two deltas with no
cross insert/delete conflicts (`A₁ ∩ D₂ = ∅` and `A₂ ∩ D₁ = ∅`), the
two application orders produce ONE final instance. This is sufficient
for the raw set transformations to commute and for NOTHING more: it
does not commute independently observed admission outcomes,
exact-state preconditions, receipts or capacity interactions —
disjoint reservations still meet through a shared capacity law
(chapter 02 §counterexamples; executable fixtures in
`crates/bumbledb-bench/src/naive/successor/admission.rs`). In 1.0 the
externally observable order remains the authority order; no public
"probably commutative" flag exists to spend this theorem. -/
theorem applyTo_comm_of_disjoint {d₁ d₂ : Delta}
    (h₁₂ : ∀ R f, f ∈ d₁.adds R → f ∉ d₂.removes R)
    (h₂₁ : ∀ R f, f ∈ d₂.adds R → f ∉ d₁.removes R) (I : Instance) :
    d₂.applyTo (d₁.applyTo I) = d₁.applyTo (d₂.applyTo I) := by
  funext R f
  refine propext ?_
  simp only [Delta.applyTo]
  constructor
  · rintro (⟨⟨hi, hr1⟩ | ha1, hr2⟩ | ha2)
    · exact Or.inl ⟨Or.inl ⟨hi, hr2⟩, hr1⟩
    · exact Or.inr ha1
    · exact Or.inl ⟨Or.inr ha2, h₂₁ R f ha2⟩
  · rintro (⟨⟨hi, hr2⟩ | ha2, hr1⟩ | ha1)
    · exact Or.inl ⟨Or.inl ⟨hi, hr1⟩, hr2⟩
    · exact Or.inr ha2
    · exact Or.inl ⟨Or.inr ha1, h₁₂ R f ha1⟩

end Support
end Txn
end Bumbledb
