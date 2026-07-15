import Bumbledb.Schema

/-!
# Cardinality — windows over parent-scoped counts (extension 1)

The first extension statement form:
`A(X | φ) in n..m per B(Y | ψ)` — for every fact `b ∈ σψ(B)`,
`|{ a ∈ σφ(A) : πX(a) = πY(b) }| ∈ [n, m]`. Real schemas are full of
laws that are counts; the two original forms have no counting
statement. This module is the denotation alone: the form's acceptance
rule (`Y` must be a key of `B` — the same exact-field-set rule
`resolve_target_key` applies to `<=` today, reused by design) stays
an ACCEPTANCE premise carried as a hypothesis where a theorem spends
it, never a conjunct here — the acceptance ≠ denotation discipline of
`Dependencies.lean`, unchanged.

## Counting without finiteness

`Set` is the `α → Prop` carrier, so counts are stated as the two
list-witnessed bounds `Set.AtLeast` / `Set.AtMost`: "some
duplicate-free list of members reaches `n`" and "every duplicate-free
list of members stays within `m`". No finiteness token is spent —
the lower bound is an existence witness, the upper bound is
universal, and both are total over arbitrary fact sets (an infinite
group fails every finite upper bound, exactly as it should).

## The no-limits posture

`hi = none` is the `*` spelling and the DEFAULT posture: the `0..*`
window is provably vacuous (`zero_star_admits`,
`cardinality_zero_star`), and every window widens into it
(`star_subsumes` spent through `cardinality_window_mono`). A spelled
window is always a strengthening of the default, never a repair of
it.

## Consistency with the existing vocabulary

The subsumption theorems (a floored window implies the reverse
containment; keyed `==` is the `1..1` window) spend `Containment`
and `KeyBackedEquality`, so they live downstream in
`Subsumption.lean` — this module sits upstream of
`Dependencies.lean`, which reads `CardinalityWindow` into
`Statement.judgment`.

## v0 refusals recorded

* **Window projections refuse interval-typed positions.** A window
  counts FACTS per parent; an interval position would make the count
  ambiguous between facts and points, and no sighted workload demands
  a pointwise count. Recorded trigger to revisit: a sighted
  counting-over-denotation workload (counting points, not rows).
  Until then the vocabulary refuses the shape at acceptance (the
  engine gate's typed error, CardinalityIntervalPosition — the
  discharge record below), and this module states nothing about it.
* **Window sides are single atoms, permanently (E1).** A join inside
  a window side would put join evaluation into the judge and break
  the linear per-statement cost model — the same acceptance-gate
  argument that keeps containment sides single atoms. Permanent, not
  a trigger.
* **Head-weighted repetition (E2) is not this vocabulary's form.** It
  is structurally encoded downstream (a consumer's refinement), never
  a statement form here.

## Countermodel

`Countermodels.unit_window_two_children` — the `1..1` window refuted
by one parent with two distinct children: the upper bound is
load-bearing, not decorative. `Countermodels.
disjunctive_window_not_literal_conjunction` — the window over a
disjunctive selection that no conjunction of per-literal windows can
express: why E3's literal sets are first-class, not sugar.

## Narrowings recorded (law 5: narrow and record)

* Counts are the two list-witnessed bounds, never a cardinal: the
  spelling is total over arbitrary fact sets and demands no
  finiteness token (module section above) — nothing here needs the
  count as a number.
* Acceptance's shape checks for the new form (the target-key rule,
  the interval refusal above) are validator mechanism this level does
  not restate; `CardinalityWindow` is consumed on accepted theories
  only, exactly as `holds` is.
* **Acceptance and enforcement discharged (2026-07-14).** The engine
  ACCEPTS the form at declaration: `StatementDescriptor::Cardinality`
  and the gate arm `validate_cardinality`
  (`crates/bumbledb/src/schema/validate.rs`) implement the acceptance
  premises above — the containment target-key rule reused, the v0
  interval refusal as the typed error `CardinalityIntervalPosition`,
  closed-side rules mirroring containment's (a statement between
  constants is decided at validate) — and the macro's `in lo..hi per`
  form lowers to it. The engine also JUDGES the window per commit:
  the statement-phase checker the plan calculus prices is
  `storage/commit/judgment.rs::check_windows` — per touched parent
  one keyed parent probe and one child-group walk, exactly
  `Oracle.cardinality_plan_decides`' shape — and `Db::verify_store`
  re-counts every parent globally. The `Bridge.lean` rows cite
  `Oracle.cardinality_plan_decides` (acceptance) and
  `Txn.cardinality_delta_restriction` (enforcement).
-/

namespace Bumbledb

/-! ## List-witnessed count bounds -/

/-- `s` has at least `n` members: some duplicate-free list of members
reaches length `n`. The lower bound is an EXISTENCE claim, so it is a
witness — no finiteness token is spent. -/
def Set.AtLeast (s : Set α) (n : Nat) : Prop :=
  ∃ l : List α, l.Nodup ∧ (∀ a, a ∈ l → a ∈ s) ∧ n ≤ l.length

/-- `s` has at most `m` members: every duplicate-free list of members
stays within length `m`. The upper bound is UNIVERSAL, so an infinite
group fails every finite bound — exactly as it should. -/
def Set.AtMost (s : Set α) (m : Nat) : Prop :=
  ∀ l : List α, l.Nodup → (∀ a, a ∈ l → a ∈ s) → l.length ≤ m

/-- Exactly `n` members: both bounds at `n`. -/
def Set.ExactCount (s : Set α) (n : Nat) : Prop :=
  s.AtLeast n ∧ s.AtMost n

/-- Every set has at least zero members — the empty witness. -/
theorem Set.atLeast_zero (s : Set α) : s.AtLeast 0 := by
  refine ⟨[], List.Pairwise.nil, ?_, Nat.le_refl 0⟩
  intro a ha
  cases ha

/-- A subsingleton counts at most one — the ceiling's elementary
builder, spent by the countermodels and the unit-window arguments. -/
theorem Set.atMost_one_of_subsingleton {s : Set α}
    (h : ∀ a b, a ∈ s → b ∈ s → a = b) : s.AtMost 1 := by
  intro l hnd hmem
  cases l with
  | nil => exact Nat.zero_le 1
  | cons a l' =>
    cases l' with
    | nil => exact Nat.le_refl 1
    | cons b l'' =>
      have hab : a = b := h a b (hmem a (List.mem_cons_self))
        (hmem b (List.mem_cons_of_mem a (List.mem_cons_self)))
      cases hnd with
      | cons hne _ => exact absurd hab (hne b (List.mem_cons_self))

/-- Two distinct members refute the unit ceiling — the two-element
duplicate-free list is the refutation witness. -/
theorem Set.not_atMost_one_of_two {s : Set α} {a b : α}
    (ha : a ∈ s) (hb : b ∈ s) (hne : a ≠ b) : ¬ s.AtMost 1 := by
  intro h
  have hnd : [a, b].Nodup :=
    List.Pairwise.cons
      (fun x hx => by
        cases hx with
        | head => exact hne
        | tail _ h' => cases h')
      (List.Pairwise.cons (fun x hx => by cases hx) List.Pairwise.nil)
  have hlen := h [a, b] hnd (fun x hx => by
    cases hx with
    | head => exact ha
    | tail _ hx' =>
      cases hx' with
      | head => exact hb
      | tail _ h'' => cases h'')
  exact Nat.not_succ_le_self 1 hlen

/-! ## Windows over one group -/

/-- The window judgment over one group: the count of `s` lies in
`[w.lo, w.hi]` — the lower bound always demanded, an upper bound only
where spelled (`hi = some m`; `none` is `*`). -/
def Window.admits (w : Window) (s : Set α) : Prop :=
  s.AtLeast w.lo ∧ ∀ m, w.hi = some m → s.AtMost m

/-- `w'` subsumes `w`: `w'` is at least as permissive — its floor is
no higher, and any ceiling it spells sits at or above one `w` already
spells. -/
def Window.Subsumes (w' w : Window) : Prop :=
  w'.lo ≤ w.lo ∧ ∀ m', w'.hi = some m' → ∃ m, w.hi = some m ∧ m ≤ m'

/-- Widening is sound: whatever a window admits, any window subsuming
it admits — the monotone move of the count calculus, mirroring
`selection_monotonicity`'s role for σ. -/
theorem Window.admits_of_subsumes {w w' : Window} {s : Set α}
    (h : w.admits s) (hsub : Window.Subsumes w' w) : w'.admits s := by
  obtain ⟨l, hnd, hmem, hlen⟩ := h.1
  refine ⟨⟨l, hnd, hmem, Nat.le_trans hsub.1 hlen⟩, ?_⟩
  intro m' hm'
  obtain ⟨m, hm, hle⟩ := hsub.2 m' hm'
  exact fun l' hnd' hmem' => Nat.le_trans (h.2 m hm l' hnd' hmem') hle

/-- **The default posture is vacuous.** The `0..*` window admits
every group: the floor is the empty witness, and there is no ceiling
to fail — the no-limits law's theorem form. -/
theorem zero_star_admits (s : Set α) : (Window.mk 0 none).admits s := by
  refine ⟨s.atLeast_zero, ?_⟩
  intro m hm
  cases hm

/-- **The default posture is universal.** `0..*` subsumes every
window: any spelled statement is a strengthening of the default,
never a repair of it. -/
theorem star_subsumes (w : Window) :
    Window.Subsumes (Window.mk 0 none) w := by
  refine ⟨Nat.zero_le w.lo, ?_⟩
  intro m' hm'
  cases hm'

/-- **The point window is exact count.** `n..n` degenerates to
exactly-`n`: the two bounds meet, and nothing else is being said. -/
theorem window_point_admits_iff (n : Nat) (s : Set α) :
    (Window.mk n (some n)).admits s ↔ s.ExactCount n := by
  constructor
  · intro h
    exact ⟨h.1, h.2 n rfl⟩
  · intro h
    refine ⟨h.1, fun m hm => ?_⟩
    injection hm with hm
    exact hm ▸ h.2

/-! ## The cardinality-window judgment -/

/-- The child group of one parent tuple: the selected source facts
whose projection `X` equals the parent's projected key tuple `t`. -/
def ChildGroup (A : Set Fact) (φ : Selection) (X : List FieldId)
    (t : List Value) : Set Fact :=
  fun f => f ∈ A ∧ φ.satisfies f ∧ f.project X = t

/-- Membership in a child group, unfolded — the definitional
reading. -/
theorem mem_childGroup {A : Set Fact} {φ : Selection}
    {X : List FieldId} {t : List Value} {f : Fact} :
    f ∈ ChildGroup A φ X t ↔
      f ∈ A ∧ φ.satisfies f ∧ f.project X = t :=
  Iff.rfl

/-- The cardinality-window judgment `A(X | φ) in w per B(Y | ψ)`:
for every selected parent fact, the child group's count lies in the
window. ACCEPTANCE IS NOT HERE: `Y` a key of `B` is the acceptance
premise (`TargetKeyAccepted`), carried as a hypothesis where a
theorem spends it — exactly the containment discipline. -/
def CardinalityWindow (A : Set Fact) (φ : Selection)
    (X : List FieldId) (w : Window) (B : Set Fact) (ψ : Selection)
    (Y : List FieldId) : Prop :=
  ∀ g, g ∈ B → ψ.satisfies g →
    w.admits (ChildGroup A φ X (g.project Y))

/-- **Behavior under the empty parent denotation.** Every window
holds when no parent fact is selected — windows constrain counts PER
PARENT and never manufacture a parent; existence obligations are
containments' alone, exactly as `functionality_of_empty` records for
keys. -/
theorem cardinality_of_empty_parent {A : Set Fact} {φ : Selection}
    {X : List FieldId} {w : Window} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} (hB : ∀ g, g ∈ B → ¬ ψ.satisfies g) :
    CardinalityWindow A φ X w B ψ Y :=
  fun g hg hψ => absurd hψ (hB g hg)

/-- **Window monotonicity.** The judgment is preserved by widening
the window — the count calculus' one monotone move, per parent via
`Window.admits_of_subsumes`. -/
theorem cardinality_window_mono {A : Set Fact} {φ : Selection}
    {X : List FieldId} {w w' : Window} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} (h : CardinalityWindow A φ X w B ψ Y)
    (hsub : Window.Subsumes w' w) :
    CardinalityWindow A φ X w' B ψ Y :=
  fun g hg hψ => Window.admits_of_subsumes (h g hg hψ) hsub

/-- **The `0..*` statement says nothing** — the default posture holds
of every instance, so an unspelled bound never gates a commit. -/
theorem cardinality_zero_star (A : Set Fact) (φ : Selection)
    (X : List FieldId) (B : Set Fact) (ψ : Selection)
    (Y : List FieldId) :
    CardinalityWindow A φ X (Window.mk 0 none) B ψ Y :=
  fun _ _ _ => zero_star_admits _

/-- **The `n..n` window is exact count**, per parent — the window
form degenerating to the exactly-`n` law counting schemas spell as
"exactly one" and its ranged siblings. -/
theorem cardinality_point_exact {A : Set Fact} {φ : Selection}
    {X : List FieldId} {n : Nat} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} :
    CardinalityWindow A φ X (Window.mk n (some n)) B ψ Y ↔
      ∀ g, g ∈ B → ψ.satisfies g →
        (ChildGroup A φ X (g.project Y)).ExactCount n := by
  constructor
  · intro h g hg hψ
    exact (window_point_admits_iff n _).mp (h g hg hψ)
  · intro h g hg hψ
    exact (window_point_admits_iff n _).mpr (h g hg hψ)

end Bumbledb
