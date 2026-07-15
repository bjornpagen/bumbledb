import Bumbledb.Txn

/-!
# DeltaRestriction — the incremental judgment (Level 2, wave 2)

The commit pipeline judges every statement RESTRICTED to
delta-touched bindings against the final state
(`docs/architecture/30-dependencies.md` § enforcement). Until now
that soundness was one prose sentence — "sound because an untouched
binding cannot change a judgment's truth". This module is that
sentence as mathematics: per statement form, a TOUCHED notion (data),
a delta-restricted check quantifying only over the touched
bindings/groups/windows, and THE theorem — over a pre-state that
holds the statement, the final state satisfies the statement IFF the
delta-restricted check passes. The composition across the whole
theory (`delta_restricted_commit_sound`) is the commit pipeline's
soundness, whole.

## The touched notions, per form (item 1 — data)

* **Scalar FD** — determinant tuples some delta fact of the relation
  projects to (`Delta.projected`, read at the determinant list).
  Bridge: `storage/commit/applier.rs::Applier` probes exactly the
  inserted determinant images.
* **Pointwise FD** — the scalar-prefix groups of the delta facts
  (`Delta.projected` at the scalar prefix); the neighbor probe runs
  within the touched group. Bridge: `Applier::probe_neighbors`.
* **Scalar IND, source side** — the ADDED source facts inside φ (the
  net insert set is the delta's `adds` by representation — the
  coalesced pair). Bridge: `storage/commit/judgment.rs::check_source`.
* **Scalar IND, target side** — the removed target key tuples NOT
  re-established (`removedTargetKeys`): a ψ-satisfying holder was
  removed and no ψ-satisfying holder stands in the final state.
  Re-establishment is ψ-QUALIFIED per statement — a re-landed tuple
  whose establishing fact fails ψ does not re-establish — mirroring
  `storage/commit/judgment.rs::check_target`'s discipline (the plan's
  plain set difference drops the empty-ψ re-land; one `F` get per
  re-landed tuple answers the ψ-carrying dependents).
* **Coverage** — the touched WINDOW per scalar group
  (`touchedWindow`): every point a delta fact of either side
  contributes to the group. Bridge: `check_target`'s affected-source
  coverage walk re-runs only across disestablished segments;
  `Checker::check_coverage` walks only the demanded source interval.
* **Cardinality window** — the touched parent keys
  (`touchedParents`): every parent key tuple any delta source fact
  projects to, plus the delta's ψ-selected parents themselves.

## The load-bearing premise (item 4)

Every restriction theorem assumes the PRE-state holds the statement.
Without it the delta-restricted verdict accepts a violating final
state — a pre-existing violation in an untouched binding survives
untouched (`Countermodels.incremental_verdict_needs_holds`). Inside
the lifecycle the premise is free (`State.models`; every committed
state was judged whole at its own commit), and OUTSIDE it the
division of authority is `Db::verify_store`'s: the sweeper re-runs
both judgment forms globally, catching exactly the class no
incremental check can see (`docs/architecture/60-validation.md` § the
store sweeper). The checks here quantify over RAW instances, not
`State`, precisely so the countermodel can exhibit the missing
premise.

## Discharged (2026-07-14): the window CHECK

The engine both ACCEPTS the window form at declaration
(`StatementDescriptor::Cardinality`, the gate arm in
`schema/validate.rs`) and ENFORCES it per commit: the
delta-restricted check this module states is the Rust checker's
consultation plan, implemented as stated — the touched-parent set is
`storage/commit/plan.rs`'s window derivation
(`cardinality_delta_restriction`'s ledger row), and
`storage/commit/judgment.rs::check_windows` judges exactly that set
against the final state.

## Narrowings recorded (law 5: narrow and record)

* **Touched notions are SETS.** The engine's plan is deduplicated
  lists in scan order — iteration order and dedup are representation
  mechanism, exactly the `violationSet` narrowing (`Txn.lean`).
* **`touchedParents` ignores φ on its source half.** Every delta
  source fact touches its parent key tuple, φ-satisfying or not — a
  SUPERSET of the minimal touched set (a non-φ fact never changes a
  child group). Wider touched only re-checks more groups; minimality
  is checker mechanism.
* **`Delta.projected` includes removes the FD forms never spend.**
  The scalar and pointwise FD restriction proofs refute only the
  adds clause (`untouched_fact_pre` — an added fact touches its own
  tuple; a removed fact leaves only pre-state facts behind, already
  keyed by the premise), and the Applier probes inserted determinant
  images only — the removes clause is a strict superset on those two
  forms, kept for the one shape of the definition. Same license as
  `touchedParents`: wider touched only re-checks more.
* **`touchedWindow` is a strict superset on two of its four
  clauses.** The backward proof spends only source-ADDS (an added
  demand) and target-REMOVES (`coverage_untouched_point` — withdrawn
  supply); removed-source points (a withdrawn demand) and
  added-target points (new supply) cannot break coverage, and the
  engine consults neither (`check_source` + the disestablished
  segments). Kept for the one shape of the definition; wider touched
  only re-checks more — minimality is checker mechanism.
* **The IND source arm carries a final-state membership hypothesis.**
  An added fact of a CLOSED relation never reaches the denotation
  (`den_closed_constant`), so the arm judges added facts that stand
  in the final state; the engine refuses closed writes at the surface
  (`ClosedRelationWrite` — the `Txn.lean` narrowing), making the
  hypothesis vacuous on every write the surface admits.
-/

namespace Bumbledb
namespace Txn

/-! ## The raw-instance final state -/

/-- The final instance of one delta over a RAW pre-instance —
`apply` freed of the `State` wrapper, so the restriction theorems can
quantify over non-modeling pre-states (the countermodel's whole
point: the `holds` premise is load-bearing, and `State` cannot carry
a violating instance). `apply s d = d.applyTo s.inst` definitionally
(`apply_eq_applyTo`). -/
def Delta.applyTo (d : Delta) (I : Instance) : Instance :=
  fun R f => (f ∈ I R ∧ f ∉ d.removes R) ∨ f ∈ d.adds R

/-- `apply` is `applyTo` at the state's instance — the bridge the
composition theorem walks from raw instances back to the lifecycle. -/
theorem apply_eq_applyTo {T : Theory} (s : State T) (d : Delta) :
    apply s d = d.applyTo s.inst :=
  rfl

/-- Membership in the final instance, unfolded — the definitional
reading. -/
theorem mem_applyTo {I : Instance} {d : Delta} {R : RelId} {f : Fact} :
    f ∈ d.applyTo I R ↔ (f ∈ I R ∧ f ∉ d.removes R) ∨ f ∈ d.adds R :=
  Iff.rfl

/-! ## The three membership moves every form spends -/

/-- A final-state fact is a pre-state fact or an added fact — the
inspection that makes every restricted check's case split. A closed
relation's denotation ignores the delta (`den_closed_constant`), so
its facts are always the pre side. -/
theorem den_final_pre_or_added {T : Theory} {I : Instance} {d : Delta}
    {R : RelId} {f : Fact} (h : f ∈ T.den (d.applyTo I) R) :
    f ∈ T.den I R ∨ f ∈ d.adds R := by
  cases hc : T.closed R with
  | some ext =>
    refine Or.inl ?_
    simp only [Theory.den, hc] at h ⊢
    exact h
  | none =>
    simp only [Theory.den, hc] at h ⊢
    rcases mem_applyTo.mp h with ⟨h1, -⟩ | h2
    · exact Or.inl h1
    · exact Or.inr h2

/-- A pre-state fact survives to the final state or was removed —
how an untouched witness is carried forward. -/
theorem den_pre_final_or_removed {T : Theory} {I : Instance}
    {d : Delta} {R : RelId} {f : Fact} (h : f ∈ T.den I R) :
    f ∈ T.den (d.applyTo I) R ∨ f ∈ d.removes R := by
  cases hc : T.closed R with
  | some ext =>
    refine Or.inl ?_
    simp only [Theory.den, hc] at h ⊢
    exact h
  | none =>
    simp only [Theory.den, hc] at h ⊢
    by_cases hr : f ∈ d.removes R
    · exact Or.inr hr
    · exact Or.inl (mem_applyTo.mpr (Or.inl ⟨h, hr⟩))

/-- A fact the delta neither adds nor removes stands in the final
denotation exactly when it stood in the pre denotation — the
untouched-implies-unchanged move, fact-level. -/
theorem den_untouched_iff {T : Theory} {I : Instance} {d : Delta}
    {R : RelId} {f : Fact} (ha : f ∉ d.adds R) (hr : f ∉ d.removes R) :
    f ∈ T.den (d.applyTo I) R ↔ f ∈ T.den I R := by
  constructor
  · intro h
    rcases den_final_pre_or_added h with h' | h'
    · exact h'
    · exact absurd h' ha
  · intro h
    rcases den_pre_final_or_removed h with h' | h'
    · exact h'
    · exact absurd h' hr

/-! ## The projected touched tuples (the FD forms) -/

/-- The delta-projected tuples of relation `R` at field list `X` —
the per-form touched data of two forms: the scalar FD (`X` the
determinant: touched determinant tuples) and the pointwise FD (`X`
the scalar prefix: touched groups). -/
def Delta.projected (d : Delta) (R : RelId) (X : List FieldId) :
    Set (List Value) :=
  fun t => ∃ f, (f ∈ d.adds R ∨ f ∈ d.removes R) ∧ f.project X = t

/-- Membership in the projected touched tuples, unfolded. -/
theorem mem_projected {d : Delta} {R : RelId} {X : List FieldId}
    {t : List Value} :
    t ∈ d.projected R X ↔
      ∃ f, (f ∈ d.adds R ∨ f ∈ d.removes R) ∧ f.project X = t :=
  Iff.rfl

/-- A final-state fact whose projected tuple is untouched is a
PRE-state fact — the shared untouched-implies-pre move of both FD
forms (an added fact touches its own tuple). -/
theorem untouched_fact_pre {T : Theory} {I : Instance} {d : Delta}
    {R : RelId} {X : List FieldId} {f : Fact}
    (hf : f ∈ T.den (d.applyTo I) R)
    (hun : f.project X ∉ d.projected R X) : f ∈ T.den I R := by
  rcases den_final_pre_or_added hf with h | h
  · exact h
  · exact absurd ⟨f, Or.inl h, rfl⟩ hun

/-! ## Form 1 — the scalar FD -/

/-- The delta-restricted scalar-FD check: injectivity demanded only
at TOUCHED determinant tuples, judged against the final state.
Bridge: `storage/commit/applier.rs::Applier` — the insert scan probes
exactly the inserted determinant images. -/
def fdDeltaCheck (T : Theory) (I : Instance) (d : Delta) (R : RelId)
    (X : List FieldId) : Prop :=
  ∀ f g, f ∈ T.den (d.applyTo I) R → g ∈ T.den (d.applyTo I) R →
    f.project X ∈ d.projected R X → f.project X = g.project X → f = g

/-- **The scalar-FD restriction theorem.** Over a pre-state holding
the key, the final state is keyed IFF the delta-restricted check
passes: an untouched determinant tuple's facts are pre-state facts
(`untouched_fact_pre`), so their agreement is the pre-state key's. -/
theorem fd_delta_restriction {T : Theory} {I : Instance} {d : Delta}
    {R : RelId} {X : List FieldId}
    (hpre : Functionality (T.den I R) X) :
    Functionality (T.den (d.applyTo I) R) X ↔
      fdDeltaCheck T I d R X := by
  constructor
  · intro h f g hf hg _ hproj
    exact h f g hf hg hproj
  · intro hc f g hf hg hproj
    by_cases ht : f.project X ∈ d.projected R X
    · exact hc f g hf hg ht hproj
    · exact hpre f g (untouched_fact_pre hf ht)
        (untouched_fact_pre hg (hproj ▸ ht)) hproj

/-! ## Form 2 — the pointwise FD -/

/-- The delta-restricted pointwise-FD check: per-group interval
disjointness demanded only at TOUCHED scalar-prefix groups, judged
against the final state. Bridge: `Applier::probe_neighbors` — the
ordered-neighbor probe runs within the inserted fact's group. -/
def pointwiseDeltaCheck (T : Theory) (I : Instance) (d : Delta)
    (R : RelId) (S : List FieldId) (i : FieldId) : Prop :=
  ∀ f g, f ∈ T.den (d.applyTo I) R → g ∈ T.den (d.applyTo I) R →
    f.project S ∈ d.projected R S → f.project S = g.project S →
    f ≠ g → ∀ x, x ∈ (f i).points → x ∉ (g i).points

/-- **The pointwise-FD restriction theorem.** An untouched group's
facts are pre-state facts, so their disjointness is the pre-state
key's — the same shape as `fd_delta_restriction`, over point sets. -/
theorem pointwise_delta_restriction {T : Theory} {I : Instance}
    {d : Delta} {R : RelId} {S : List FieldId} {i : FieldId}
    (hpre : PointwiseKey (T.den I R) S i) :
    PointwiseKey (T.den (d.applyTo I) R) S i ↔
      pointwiseDeltaCheck T I d R S i := by
  constructor
  · intro h f g hf hg _ hgroup hne
    exact h f g hf hg hgroup hne
  · intro hc f g hf hg hgroup hne
    by_cases ht : f.project S ∈ d.projected R S
    · exact hc f g hf hg ht hgroup hne
    · exact hpre f g (untouched_fact_pre hf ht)
        (untouched_fact_pre hg (hgroup ▸ ht)) hgroup hne

/-! ## Form 3 — the scalar IND (containment) -/

/-- The disestablished target keys of one containment statement: the
projected key tuples some removed ψ-satisfying target fact held, with
NO ψ-satisfying holder standing in the final state. Re-establishment
is ψ-QUALIFIED — a re-landed tuple whose establishing fact fails ψ
does not re-establish — mirroring
`storage/commit/judgment.rs::check_target` (the plan's plain set
difference drops the empty-ψ re-land; the ψ-carrying dependents read
the establishing fact). -/
def removedTargetKeys (T : Theory) (I : Instance) (d : Delta)
    (tgt : Atom) : Set (List Value) :=
  fun t =>
    (∃ g, g ∈ d.removes tgt.relation ∧ tgt.selection.satisfies g ∧
      g.project tgt.projection = t) ∧
    ¬ ∃ g, g ∈ T.den (d.applyTo I) tgt.relation ∧
      tgt.selection.satisfies g ∧ g.project tgt.projection = t

/-- The delta-restricted containment check, two arms mirroring the
engine's direction partition: the SOURCE side probes every added
source fact inside φ for its final-state witness
(`judgment.rs::check_source`); the TARGET side demands no surviving
φ-source still requires a disestablished key tuple
(`judgment.rs::check_target`). -/
def containmentDeltaCheck (T : Theory) (I : Instance) (d : Delta)
    (src tgt : Atom) : Prop :=
  (∀ f, f ∈ d.adds src.relation →
    f ∈ T.den (d.applyTo I) src.relation →
    src.selection.satisfies f →
    ∃ g, g ∈ T.den (d.applyTo I) tgt.relation ∧
      tgt.selection.satisfies g ∧
      g.project tgt.projection = f.project src.projection) ∧
  (∀ f, f ∈ T.den (d.applyTo I) src.relation →
    src.selection.satisfies f →
    f.project src.projection ∉ removedTargetKeys T I d tgt)

/-- **Untouched implies unchanged, containment form.** A PRE-state
source fact whose target key tuple is not disestablished keeps a
final-state witness: its pre-state witness either survives or was
removed — and a removed witness with an undisestablished tuple forces
a ψ-satisfying re-establisher, which is the new witness. -/
theorem containment_untouched_witness {T : Theory} {I : Instance}
    {d : Delta} {src tgt : Atom}
    (hpre : Containment (T.den I src.relation) src.selection
      src.projection (T.den I tgt.relation) tgt.selection
      tgt.projection)
    {f : Fact} (hf : f ∈ T.den I src.relation)
    (hφ : src.selection.satisfies f)
    (hun : f.project src.projection ∉ removedTargetKeys T I d tgt) :
    ∃ g, g ∈ T.den (d.applyTo I) tgt.relation ∧
      tgt.selection.satisfies g ∧
      g.project tgt.projection = f.project src.projection := by
  obtain ⟨g₀, hg₀, hψ, hproj⟩ := hpre f hf hφ
  rcases den_pre_final_or_removed hg₀ with hg₁ | hrem
  · exact ⟨g₀, hg₁, hψ, hproj⟩
  · exact Classical.byContradiction fun hno =>
      hun ⟨⟨g₀, hrem, hψ, hproj⟩, hno⟩

/-- **The containment restriction theorem.** Over a pre-state holding
the containment, the final state holds it IFF both restricted arms
pass: an added source fact is the source arm's; a pre-existing source
fact keeps its witness unless its key tuple was disestablished
(`containment_untouched_witness`), which is the target arm's. -/
theorem containment_delta_restriction {T : Theory} {I : Instance}
    {d : Delta} {src tgt : Atom}
    (hpre : Containment (T.den I src.relation) src.selection
      src.projection (T.den I tgt.relation) tgt.selection
      tgt.projection) :
    Containment (T.den (d.applyTo I) src.relation) src.selection
      src.projection (T.den (d.applyTo I) tgt.relation) tgt.selection
      tgt.projection ↔
      containmentDeltaCheck T I d src tgt := by
  constructor
  · intro h
    constructor
    · intro f _ hf hφ
      exact h f hf hφ
    · intro f hf hφ hmem
      obtain ⟨g, hg, hψ, hproj⟩ := h f hf hφ
      exact hmem.2 ⟨g, hg, hψ, hproj⟩
  · intro hc f hf hφ
    rcases den_final_pre_or_added hf with hf₀ | hadd
    · exact containment_untouched_witness hpre hf₀ hφ (hc.2 f hf hφ)
    · exact hc.1 f hadd hf hφ

/-! ## Form 4 — coverage (the pointwise IND) -/

/-- The touched window of one scalar group under a delta: every point
a delta fact of EITHER side contributes to the group — the source
relation's delta points (a new demand or a withdrawn one) and the
target relation's delta points (new or withdrawn supply). Bridge:
`check_target` re-walks disestablished segments;
`Checker::check_coverage` walks the demanded source interval. -/
def touchedWindow (d : Delta) (Ra : RelId) (S : List FieldId)
    (i : FieldId) (Rb : RelId) (U : List FieldId) (j : FieldId)
    (t : List Value) : Set Point :=
  fun x =>
    (∃ f, (f ∈ d.adds Ra ∨ f ∈ d.removes Ra) ∧ f.project S = t ∧
      x ∈ (f i).points) ∨
    (∃ g, (g ∈ d.adds Rb ∨ g ∈ d.removes Rb) ∧ g.project U = t ∧
      x ∈ (g j).points)

/-- The delta-restricted coverage check: the covering witness is
demanded only at points inside the group's TOUCHED window, judged
against the final state. -/
def coverageDeltaCheck (T : Theory) (I : Instance) (d : Delta)
    (Ra : RelId) (φ : Selection) (S : List FieldId) (i : FieldId)
    (Rb : RelId) (ψ : Selection) (U : List FieldId) (j : FieldId) :
    Prop :=
  ∀ f, f ∈ T.den (d.applyTo I) Ra → φ.satisfies f →
    ∀ x, x ∈ (f i).points →
      x ∈ touchedWindow d Ra S i Rb U j (f.project S) →
      ∃ g, g ∈ T.den (d.applyTo I) Rb ∧ ψ.satisfies g ∧
        g.project U = f.project S ∧ x ∈ (g j).points

/-- **Untouched implies unchanged, coverage form.** A pre-state
source fact's point outside the touched window keeps its pre-state
covering witness: a removed witness would have put the point INSIDE
the window. -/
theorem coverage_untouched_point {T : Theory} {I : Instance}
    {d : Delta} {Ra : RelId} {φ : Selection} {S : List FieldId}
    {i : FieldId} {Rb : RelId} {ψ : Selection} {U : List FieldId}
    {j : FieldId}
    (hpre : Coverage (T.den I Ra) φ S i (T.den I Rb) ψ U j)
    {f : Fact} {x : Point} (hf : f ∈ T.den I Ra)
    (hφ : φ.satisfies f) (hx : x ∈ (f i).points)
    (hun : x ∉ touchedWindow d Ra S i Rb U j (f.project S)) :
    ∃ g, g ∈ T.den (d.applyTo I) Rb ∧ ψ.satisfies g ∧
      g.project U = f.project S ∧ x ∈ (g j).points := by
  obtain ⟨g₀, hg₀, hψ, hproj, hxg⟩ := hpre f hf hφ x hx
  rcases den_pre_final_or_removed hg₀ with hg₁ | hrem
  · exact ⟨g₀, hg₁, hψ, hproj, hxg⟩
  · exact absurd (Or.inr ⟨g₀, Or.inr hrem, hproj, hxg⟩) hun

/-- **The coverage restriction theorem.** Over a pre-state holding
the coverage, the final state holds it IFF the touched-window check
passes: an added source fact's points lie inside their group's
touched window by construction, and an untouched point keeps its
pre-state witness (`coverage_untouched_point`). -/
theorem coverage_delta_restriction {T : Theory} {I : Instance}
    {d : Delta} {Ra : RelId} {φ : Selection} {S : List FieldId}
    {i : FieldId} {Rb : RelId} {ψ : Selection} {U : List FieldId}
    {j : FieldId}
    (hpre : Coverage (T.den I Ra) φ S i (T.den I Rb) ψ U j) :
    Coverage (T.den (d.applyTo I) Ra) φ S i
      (T.den (d.applyTo I) Rb) ψ U j ↔
      coverageDeltaCheck T I d Ra φ S i Rb ψ U j := by
  constructor
  · intro h f hf hφ x hx _
    exact h f hf hφ x hx
  · intro hc f hf hφ x hx
    by_cases ht : x ∈ touchedWindow d Ra S i Rb U j (f.project S)
    · exact hc f hf hφ x hx ht
    · rcases den_final_pre_or_added hf with hf₀ | hadd
      · exact coverage_untouched_point hpre hf₀ hφ hx ht
      · exact absurd (Or.inl ⟨f, Or.inl hadd, rfl, hx⟩) ht

/-! ## Form 5 — the cardinality window -/

/-- The touched parent keys of one window statement: every parent key
tuple any delta source fact projects to (a count that may have
moved), plus the delta's ψ-selected parents themselves (a group newly
constrained or released). -/
def touchedParents (d : Delta) (src tgt : Atom) : Set (List Value) :=
  fun t =>
    t ∈ d.projected src.relation src.projection ∨
    ∃ g, (g ∈ d.adds tgt.relation ∨ g ∈ d.removes tgt.relation) ∧
      tgt.selection.satisfies g ∧ g.project tgt.projection = t

/-- The delta-restricted window check: the window judged only at
TOUCHED parent keys, against the final state's child groups. -/
def cardinalityDeltaCheck (T : Theory) (I : Instance) (d : Delta)
    (src : Atom) (w : Window) (tgt : Atom) : Prop :=
  ∀ g, g ∈ T.den (d.applyTo I) tgt.relation →
    tgt.selection.satisfies g →
    g.project tgt.projection ∈ touchedParents d src tgt →
    w.admits (ChildGroup (T.den (d.applyTo I) src.relation)
      src.selection src.projection (g.project tgt.projection))

/-- **Untouched implies unchanged, window form.** An untouched parent
key's child group is the SAME fact set in the final state: a delta
source fact projecting to it would have touched it. -/
theorem cardinality_untouched_group_eq {T : Theory} {I : Instance}
    {d : Delta} {src tgt : Atom} {t : List Value}
    (hun : t ∉ touchedParents d src tgt) :
    ChildGroup (T.den (d.applyTo I) src.relation) src.selection
      src.projection t =
      ChildGroup (T.den I src.relation) src.selection
        src.projection t := by
  funext f
  refine propext ⟨?_, ?_⟩
  · intro h
    obtain ⟨hf, hφ, hproj⟩ := mem_childGroup.mp h
    refine mem_childGroup.mpr ⟨?_, hφ, hproj⟩
    rcases den_final_pre_or_added hf with h' | h'
    · exact h'
    · exact absurd (Or.inl ⟨f, Or.inl h', hproj⟩) hun
  · intro h
    obtain ⟨hf, hφ, hproj⟩ := mem_childGroup.mp h
    refine mem_childGroup.mpr ⟨?_, hφ, hproj⟩
    rcases den_pre_final_or_removed hf with h' | h'
    · exact h'
    · exact absurd (Or.inl ⟨f, Or.inr h', hproj⟩) hun

/-- **The window restriction theorem.** Over a pre-state
holding the window, the final state holds it IFF the touched-parents
check passes: an untouched parent is a pre-state parent whose child
group is unchanged (`cardinality_untouched_group_eq`), so its window
verdict is the pre-state's. -/
theorem cardinality_delta_restriction {T : Theory} {I : Instance}
    {d : Delta} {src : Atom} {w : Window} {tgt : Atom}
    (hpre : CardinalityWindow (T.den I src.relation) src.selection
      src.projection w (T.den I tgt.relation) tgt.selection
      tgt.projection) :
    CardinalityWindow (T.den (d.applyTo I) src.relation)
      src.selection src.projection w
      (T.den (d.applyTo I) tgt.relation) tgt.selection
      tgt.projection ↔
      cardinalityDeltaCheck T I d src w tgt := by
  constructor
  · intro h g hg hψ _
    exact h g hg hψ
  · intro hc g hg hψ
    by_cases ht : g.project tgt.projection ∈ touchedParents d src tgt
    · exact hc g hg hψ ht
    · have hg₀ : g ∈ T.den I tgt.relation := by
        rcases den_final_pre_or_added hg with h' | h'
        · exact h'
        · exact absurd (Or.inr ⟨g, Or.inl h', hψ, rfl⟩) ht
      rw [cardinality_untouched_group_eq ht]
      exact hpre g hg₀ hψ

/-! ## The per-statement dispatch and the composition theorem -/
/-- One statement's delta-restricted check — `Statement.judgment`'s
dispatch, arm for arm, each form replaced by its restricted check.
This is the consultation plan the commit pipeline runs INSTEAD of the
full judgment; `statement_delta_restriction` is the license. -/
def deltaCheck (T : Theory) (I : Instance) (d : Delta) :
    Statement → Prop
  | .functionality R X =>
    match T.header.intervalSplit R X with
    | some (S, i) => pointwiseDeltaCheck T I d R S i
    | none => fdDeltaCheck T I d R X
  | .containment src tgt =>
    match T.header.intervalSplit src.relation src.projection,
          T.header.intervalSplit tgt.relation tgt.projection with
    | some (S, i), some (U, j) =>
      coverageDeltaCheck T I d src.relation src.selection S i
        tgt.relation tgt.selection U j
    | _, _ => containmentDeltaCheck T I d src tgt
  | .cardinality src w tgt => cardinalityDeltaCheck T I d src w tgt

/-- **The per-statement restriction theorem.** Over a pre-state
holding one statement, the final state satisfies the statement IFF
its delta-restricted check passes — every form, through one
dispatch. -/
theorem statement_delta_restriction (T : Theory) (I : Instance)
    (d : Delta) (st : Statement) (hpre : st.judgment T I) :
    st.judgment T (d.applyTo I) ↔ deltaCheck T I d st := by
  cases st with
  | functionality R X =>
    cases hsplit : T.header.intervalSplit R X with
    | none =>
      simp only [Statement.judgment, deltaCheck, hsplit] at hpre ⊢
      exact fd_delta_restriction hpre
    | some p =>
      obtain ⟨S, i⟩ := p
      simp only [Statement.judgment, deltaCheck, hsplit] at hpre ⊢
      exact pointwise_delta_restriction hpre
  | containment src tgt =>
    cases hs : T.header.intervalSplit src.relation src.projection with
    | some p =>
      obtain ⟨S, i⟩ := p
      cases ht : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        obtain ⟨U, j⟩ := q
        simp only [Statement.judgment, deltaCheck, hs, ht] at hpre ⊢
        exact coverage_delta_restriction hpre
      | none =>
        simp only [Statement.judgment, deltaCheck, hs, ht] at hpre ⊢
        exact containment_delta_restriction hpre
    | none =>
      cases ht : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        simp only [Statement.judgment, deltaCheck, hs, ht] at hpre ⊢
        exact containment_delta_restriction hpre
      | none =>
        simp only [Statement.judgment, deltaCheck, hs, ht] at hpre ⊢
        exact containment_delta_restriction hpre
  | cardinality src w tgt =>
    simp only [Statement.judgment, deltaCheck] at hpre ⊢
    exact cardinality_delta_restriction hpre

/-- The exact form over the lifecycle: a committed state's final
state models the theory IFF every declared statement's
delta-restricted check passes — the pre-state's `holds` is the
`State`'s own commitment, spent statement by statement. -/
theorem delta_restriction_exact {T : Theory} (s : State T)
    (d : Delta) :
    holds T (apply s d) ↔
      ∀ st, st ∈ T.statements → deltaCheck T s.inst d st := by
  constructor
  · intro h st hst
    exact (statement_delta_restriction T s.inst d st
      (s.models st hst)).mp (h st hst)
  · intro h st hst
    exact (statement_delta_restriction T s.inst d st
      (s.models st hst)).mpr (h st hst)

/-- **The composition theorem — the commit pipeline's soundness,
whole (item 3).** Pre-state models the theory (the `State`'s
commitment) and every statement's delta-restricted check passes: the
final state models the theory. This is the one prose sentence of
`docs/architecture/30-dependencies.md` § enforcement as mathematics —
the incremental judgment convicts exactly what the full judgment
convicts, so running only the restricted checks at commit loses
nothing. Bridge: `storage/commit/judgment.rs::judge` +
`storage/commit/apply.rs::apply` run every form's restricted check
(FD, containment, and window — module doc § discharged), and
equivalent-under-premise
rather than literally these; the two recorded coincidences: (1) the
Applier's FD probe covers only inserted determinant images while
`Delta.projected` also spans remove-touched tuples — a superset that
only re-checks more (the narrowing below); (2) `check_target`'s
re-establishment tests exact re-landed determinant bytes while
`removedTargetKeys` accepts any ψ-satisfying final holder — the two
coincide because the key phase has already made the target bucket a
subsingleton (keys convict before any statement probe runs,
`Txn.lean`'s phase order). `Db::verify_store` owns the
missing-premise class (`docs/architecture/60-validation.md` § the
store sweeper; `Countermodels.incremental_verdict_needs_holds`). -/
theorem delta_restricted_commit_sound {T : Theory} (s : State T)
    (d : Delta)
    (h : ∀ st, st ∈ T.statements → deltaCheck T s.inst d st) :
    holds T (apply s d) :=
  (delta_restriction_exact s d).mpr h

/-- The pipeline corollary: passing delta-restricted checks means the
commit ACCEPTS, and the accepted state is the delta's final state —
the restricted judge and `commit` agree on the accept path. -/
theorem delta_restricted_pipeline_accepts {T : Theory} (s : State T)
    (d : Delta)
    (h : ∀ st, st ∈ T.statements → deltaCheck T s.inst d st) :
    commit s d = .ok ⟨apply s d, delta_restricted_commit_sound s d h⟩ :=
  judge_holds (delta_restricted_commit_sound s d h)

end Txn
end Bumbledb
