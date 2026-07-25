import Bumbledb.Cardinality

/-!
# Capacity — the aggregate containment (the capacity cutover, spec statement)

The capacity statement `B(Y | ψ) <=[w]{lo..hi} A(X | φ)`
(`docs/design/capacity-laws.md`): for every ψ-selected target fact,
the MEASURE of its child group — Σ weight over the deduplicated
φ-selected source facts sharing its projected key tuple — lies in the
window, whose bounds resolve against the target's own row. Counting
is the unit-weight corollary it always was
(`length = sum ∘ map(const 1)` — `natSum_map_const_one`,
`cardinality_is_unit_capacity`), so the `<={lo..hi}` count utterance
survives character for character as the unit instance while
everything count-only beneath it retires.

The completed operator family, every rung the same partition law:

* `==`   — the `{1}` window (`keyed_eq_unit_window`)
* `<=`   — existence (`window_floor_containment`)
* `<={lo..hi}`    — unit weight, the count instance
* `<=[w]{lo..hi}` — the weighted capacity statement (this module)

## Measuring without finiteness (ruling C7)

A weighted law is irreducibly numeric, but no finiteness token is
spent: the measure bounds are the WITNESS-STYLE pair
`Set.MeasureAtLeast` / `Set.MeasureAtMost` — "some duplicate-free
list of members weighs at least `n`" and "every duplicate-free list
of members weighs at most `m`" — sound because ℕ-weights are
non-negative, total over arbitrary fact sets (an infinite group of
positive weights fails every finite ceiling, exactly as it should).
The numeric fold lives at the enumeration boundary, as it does for
counts: over one duplicate-free enumeration both bounds collapse to
one `natSum` (`measureAtLeast_iff_enum` / `measureAtMost_iff_enum`),
riding the weighted pigeonhole `nodup_subset_natSum_le`. The count
spellings `Set.AtLeast`/`Set.AtMost` survive as the `const 1`
corollaries (`measureAtLeast_unit_iff` / `measureAtMost_unit_iff`).

`natSum` moved UPSTREAM to this module from the query aggregates
(one definition serves the capacity denotation, the checked-sum
lemmas, and the countermodels); the sum is unbounded ℕ — the
engine's u128 accumulator claim is tied down by
`natSum_le_length_mul` (`Query/Aggregates.lean`), and truncation is
unrepresentable end to end (ruling C3).

## The pigeonhole, consolidated (dossier § 7 item 14)

The count pigeonhole is proved twice downstream in local styles
(`Decide.length_le_of_nodup_of_subset`, classical erase;
`Oracle.nodup_subset_length_le`, split-at-occurrence). Its weighted
successor is owed to BOTH consumers; this module rules the budgeting
question by consolidation: `nodup_subset_natSum_le` is proved ONCE
here, upstream of both, in the split-at-occurrence style (no
decidable equality — `Fact` has none), on the owed helper
`natSum_append`. `Decide.capacityB_iff` and
`Oracle.capacity_plan_decides` both spend it through the enumeration
collapse.

## Syntax resolved against rows (rulings C1, C4, C6)

* `Weight` (`Schema.lean`, C4 — the total three-case sum) reads
  through `Weight.apply`: `unit ↦ const 1`; `field i ↦` the u64
  payload of the SOURCE row's field; `durationOf i ↦` the interval
  measure. Signed weights are refused at the gate (polarity), never
  here.
* `Bound` resolves by NAME against the TARGET's whole field roster
  (C1 — the projection tuple stays the pure grouping key), through
  `Bound.resolve` over the fact the judge already holds.
* `CapWindow.resolve` lands the syntactic window in the literal
  `Window` — the `{lo..hi}` object that SURVIVES the cutover (ruling
  C16): admission is stated over the resolved form
  (`Window.admitsMeasure`), so "one walk decides a window" keeps its
  shape with sum in place of length. A dependent floor is
  unrepresentable (C6: `CapWindow.lo : Nat`).

## Narrowings recorded (law 5: narrow and record)

* **The value readings are junk-total.** `Value.u64Nat` and
  `Value.durationNat` read 0 off every gate-refused shape (a non-u64
  weight field, a scalar in duration position) — the recorded
  junk-total default (`Dependencies.lean`'s precedent for gate-refused
  shapes): `CapacityLaw` is consumed on accepted theories only,
  where the weight typing arm of the acceptance gate has already
  refused every shape the junk value could distinguish.
* **A ray reads measure 0 here; the ENGINE refuses the commit**
  (ruling C10: a ray-valued Duration weight or bound at judge time is
  a typed commit refusal naming the row — the R6 precedent, a ray has
  no finite measure). The junk value is unobservable on
  accepted-and-judged commits; the refusal is gate mechanism this
  level does not restate (`measure_ray_none` is the law it enforces).
* **Acceptance is not restated.** `Y` a key of `B`, the weight-typing
  roster (signed/non-u64 refusals, the path-weight refusal naming the
  pinned-column idiom), dependent-bound typing, and the dimension
  gate (C18) are validator mechanism; `CapacityLaw` carries none of
  them as conjuncts — exactly the containment discipline.
* **Bound subsumption is pointwise over resolved windows**
  (`CapWindow.Subsumes`): the syntactic Bound-comparison relation the
  respelling detector needs is a recorded seam (dossier § 7 item 16),
  not stated here.

## The build-lane ledger (the L4 handoff)

Discharged in this flush (forced by `Statement.capacity`'s totality
— every dispatch over `Statement` must carry the new arm, so the
obligations the dossier § 2 names land with the statement):
`Decide.capacityB_iff`, `Oracle.capacity_plan_decides`,
`Oracle.capacity_plan_consultations` (C16's successor name),
`Txn.capacity_delta_restriction`, and the Lean-side Duration
extractor (`Value.durationNat`).

Discharged by the build lane since: **the C12 clip lemma**
(`natSum_prefix_le` below — prefix monotonicity of non-negative
running sums — spent by `Oracle.capacity_ceiling_exit_sound` /
`Oracle.capacity_floor_exit_sound`, the named soundness of the
engine's early-exit walk: ceiling exits at `sum > hi`, floor at
`sum ≥ lo`); **the C11 Admission form** (`Admission.capacityForm` —
the bounded-quantification verdict proved sufficient: it quantifies
over the probed false-surface parent bucket and resolves each
answer's window at that answer, so `AdmissibleForm`'s Verdict type
generalizes for no one).

OWED to the build lane, recorded by name:

* **The C12 Bridge row**: the clip theorems' ledger row, the analog
  of `Exec.sweep_early_exit_sound`'s — it re-pins with the capacity
  engine anchors, so it rides the count-path deletion.
* **The count path's deletion**: `Cardinality.lean`,
  `Statement.cardinality`, `Decide.cardinalityB`,
  `Oracle.WindowPlanned`, `Txn.cardinalityDeltaCheck`, and
  `Admission.cardinalityForm` retire into unit-instance corollaries —
  `cardinality_is_unit_capacity` is the license that the deletion
  loses nothing. `ChildGroup` and the `Set.AtLeast`/`Set.AtMost`
  unit lemmas move into this module when their home dies.
* **The corpus re-baseline (C8)**: `judgment-window-*` re-keys to
  `judgment-capacity-*`; `Main.lean`'s decoder gains the `"capacity"`
  key and the `"cardinality"` key dies, in ONE commit with the Rust
  serializer and the corpus files (this flush leaves the old decode
  path alive and UNREACHABLE from the new statement, so the corpus
  replays byte-identically).
* **Bridge rows** for the capacity theorems, with engine anchors
  re-pinned to the capacity-side symbols (`validate_capacity`,
  `check_capacity`, `CapacityStatement`) once they exist — a row now
  would dangle the census.
* **The Subsumption restatement**: `window_floor_containment`'s floor
  half generalizes to ANY weight (a positive measure floor inhabits
  the group), but the `{1..*}` BAN fires on the Count instance only —
  a weight-0 row satisfies containment but not the sum floor; the
  `Countermodels.lean` residents re-derive over Count-instance
  capacity laws.
-/

namespace Bumbledb

/-! ## The sum — unbounded ℕ, one definition for every altitude -/

/-- The mathematical sum: the capacity measure's fold, and the sum
the query-side checked forms are measured against
(`checkedSum_sound`, `Query/Aggregates.lean` — this definition moved
upstream at the capacity cutover). Unbounded ℕ: the engine
accumulates in u128 and the witnessed measure crosses whole (ruling
C3), so no ceiling is modeled. -/
def natSum : List Nat → Nat
  | [] => 0
  | x :: xs => x + natSum xs

/-- The sum splits over append — the helper the split-at-occurrence
pigeonhole walks through (the dossier's owed `natSum_append`). -/
theorem natSum_append : ∀ l₁ l₂ : List Nat,
    natSum (l₁ ++ l₂) = natSum l₁ + natSum l₂
  | [], _ => (Nat.zero_add _).symm
  | x :: l₁, l₂ => by
    show x + natSum (l₁ ++ l₂) = x + natSum l₁ + natSum l₂
    rw [natSum_append l₁ l₂]
    omega

/-- **Prefix monotonicity of non-negative running sums** — the C12
clip lemma: a walk's running sum never exceeds the whole walk's sum,
so the engine's early exit is sound in both polarities — a ceiling
walk may convict the moment the running sum passes `hi` (the suffix
can only raise it: `capacity_ceiling_exit_sound`, `Oracle.lean`), and
a floor-only walk may accept the moment it reaches `lo`
(`capacity_floor_exit_sound`). The design's § 4 early-exit claim is
cited here, not asserted. -/
theorem natSum_prefix_le (l₁ l₂ : List Nat) :
    natSum l₁ ≤ natSum (l₁ ++ l₂) := by
  rw [natSum_append]
  exact Nat.le_add_right _ _

/-- Unit weights sum to the length — `length = sum ∘ map(const 1)`,
the design's one-line reason counting is the unit-weight corollary. -/
theorem natSum_map_const_one : ∀ l : List α,
    natSum (l.map fun _ => 1) = l.length
  | [] => rfl
  | _ :: l => by
    show 1 + natSum (l.map fun _ => 1) = l.length + 1
    rw [natSum_map_const_one l]
    omega

/-- **The weighted pigeonhole**: a duplicate-free list of members of
another list weighs no more than it. Split-at-occurrence style — no
decidable equality demanded (`Fact` has none) — consolidating the two
count pigeonholes' weighted successor in ONE home (module doc; both
downstream consumers spend this). Only the SUBLIST needs `Nodup`:
the host may repeat, its sum only grows. -/
theorem nodup_subset_natSum_le (w : α → Nat) :
    ∀ (l enum : List α), l.Nodup → (∀ a, a ∈ l → a ∈ enum) →
      natSum (l.map w) ≤ natSum (enum.map w)
  | [], _, _, _ => Nat.zero_le _
  | a :: rest, enum, hnd, hsub => by
    obtain ⟨s, t, rfl⟩ := List.append_of_mem (hsub a List.mem_cons_self)
    obtain ⟨hne, hnd'⟩ := List.pairwise_cons.mp hnd
    have hsub' : ∀ x, x ∈ rest → x ∈ s ++ t := by
      intro x hx
      rcases List.mem_append.mp (hsub x (List.mem_cons_of_mem a hx))
        with h | h
      · exact List.mem_append.mpr (.inl h)
      · rcases List.mem_cons.mp h with rfl | h'
        · exact absurd rfl (hne x hx)
        · exact List.mem_append.mpr (.inr h')
    have hrest := nodup_subset_natSum_le w rest (s ++ t) hnd' hsub'
    rw [List.map_append, natSum_append] at hrest
    rw [List.map_cons, List.map_append, List.map_cons, natSum_append]
    show w a + natSum (rest.map w) ≤
      natSum (s.map w) + (w a + natSum (t.map w))
    omega

/-! ## List-witnessed measure bounds (ruling C7) -/

/-- `s` weighs at least `n` under `w`: some duplicate-free list of
members reaches measure `n`. The floor is an EXISTENCE claim, so it
is a witness — no finiteness token is spent (the `Set.AtLeast`
discipline, weighted). -/
def Set.MeasureAtLeast (w : α → Nat) (s : Set α) (n : Nat) : Prop :=
  ∃ l : List α, l.Nodup ∧ (∀ a, a ∈ l → a ∈ s) ∧ n ≤ natSum (l.map w)

/-- `s` weighs at most `m` under `w`: every duplicate-free list of
members stays within measure `m`. The ceiling is UNIVERSAL — an
infinite group of positive weights fails every finite bound, exactly
as it should. Sound as a pair with the floor because ℕ-weights are
non-negative. -/
def Set.MeasureAtMost (w : α → Nat) (s : Set α) (m : Nat) : Prop :=
  ∀ l : List α, l.Nodup → (∀ a, a ∈ l → a ∈ s) → natSum (l.map w) ≤ m

/-- Exactly measure `n`: both bounds at `n`. -/
def Set.ExactMeasure (w : α → Nat) (s : Set α) (n : Nat) : Prop :=
  s.MeasureAtLeast w n ∧ s.MeasureAtMost w n

/-- Every set weighs at least zero — the empty witness
(weight-independently: `natSum [] = 0`). -/
theorem Set.measureAtLeast_zero (w : α → Nat) (s : Set α) :
    s.MeasureAtLeast w 0 := by
  refine ⟨[], List.Pairwise.nil, ?_, Nat.le_refl 0⟩
  intro a ha
  cases ha

/-- **The count floor is the unit-weight floor** — `Set.AtLeast`
survives as the `const 1` corollary. -/
theorem Set.measureAtLeast_unit_iff (s : Set α) (n : Nat) :
    s.MeasureAtLeast (fun _ => 1) n ↔ s.AtLeast n := by
  constructor
  · rintro ⟨l, hnd, hmem, hlen⟩
    exact ⟨l, hnd, hmem, by rwa [natSum_map_const_one] at hlen⟩
  · rintro ⟨l, hnd, hmem, hlen⟩
    exact ⟨l, hnd, hmem, by rwa [natSum_map_const_one]⟩

/-- **The count ceiling is the unit-weight ceiling** — `Set.AtMost`
survives as the `const 1` corollary. -/
theorem Set.measureAtMost_unit_iff (s : Set α) (m : Nat) :
    s.MeasureAtMost (fun _ => 1) m ↔ s.AtMost m := by
  constructor
  · intro h l hnd hmem
    have := h l hnd hmem
    rwa [natSum_map_const_one] at this
  · intro h l hnd hmem
    rw [natSum_map_const_one]
    exact h l hnd hmem

/-! ## The enumeration collapse — one walk decides a measure

Over one duplicate-free enumeration of the set both witness-style
bounds collapse to ONE `natSum` — the weighted successor of the
count bounds' length collapse, and the shape both executable
consumers spend (`Decide.capacityB_iff`,
`Oracle.capacity_plan_decides`). -/

/-- The floor over one enumeration: `MeasureAtLeast n` is
`n ≤ natSum`. -/
theorem measureAtLeast_iff_enum {s : Set α} {l : List α}
    (hmem : ∀ a, a ∈ l ↔ a ∈ s) (hnd : l.Nodup) (w : α → Nat)
    (n : Nat) :
    s.MeasureAtLeast w n ↔ n ≤ natSum (l.map w) := by
  constructor
  · rintro ⟨l', hnd', hsub, hlen⟩
    exact Nat.le_trans hlen (nodup_subset_natSum_le w l' l hnd'
      fun a ha => (hmem a).mpr (hsub a ha))
  · intro h
    exact ⟨l, hnd, fun a ha => (hmem a).mp ha, h⟩

/-- The ceiling over one enumeration: `MeasureAtMost m` is
`natSum ≤ m`. -/
theorem measureAtMost_iff_enum {s : Set α} {l : List α}
    (hmem : ∀ a, a ∈ l ↔ a ∈ s) (hnd : l.Nodup) (w : α → Nat)
    (m : Nat) :
    s.MeasureAtMost w m ↔ natSum (l.map w) ≤ m := by
  constructor
  · intro h
    exact h l hnd fun a ha => (hmem a).mp ha
  · intro h l' hnd' hsub
    exact Nat.le_trans (nodup_subset_natSum_le w l' l hnd'
      fun a ha => (hmem a).mpr (hsub a ha)) h

/-! ## Measure admission over the resolved window -/

/-- The measure judgment over one group at a RESOLVED (literal)
window: the group's measure under `wt` lies in `[w.lo, w.hi]` — the
floor always demanded, a ceiling only where spelled (`none` is `*`).
`Window.admits`' weighted successor; the count reading is the
`const 1` instance. -/
def Window.admitsMeasure (w : Window) (wt : α → Nat) (s : Set α) :
    Prop :=
  s.MeasureAtLeast wt w.lo ∧ ∀ m, w.hi = some m → s.MeasureAtMost wt m

/-- Widening is sound for measures: whatever a window admits under a
weight, any window subsuming it admits — `Window.admits_of_subsumes`
weighted, verbatim in shape. -/
theorem Window.admitsMeasure_of_subsumes {w w' : Window}
    {wt : α → Nat} {s : Set α} (h : w.admitsMeasure wt s)
    (hsub : Window.Subsumes w' w) : w'.admitsMeasure wt s := by
  obtain ⟨l, hnd, hmem, hlen⟩ := h.1
  refine ⟨⟨l, hnd, hmem, Nat.le_trans hsub.1 hlen⟩, ?_⟩
  intro m' hm'
  obtain ⟨m, hm, hle⟩ := hsub.2 m' hm'
  exact fun l' hnd' hmem' => Nat.le_trans (h.2 m hm l' hnd' hmem') hle

/-- **The default posture is vacuous, weight-independently.** The
`0..*` window admits every group's measure under every weight. -/
theorem measure_zero_star_admits (wt : α → Nat) (s : Set α) :
    (Window.mk 0 none).admitsMeasure wt s := by
  refine ⟨Set.measureAtLeast_zero wt s, ?_⟩
  intro m hm
  cases hm

/-- **The point window is exact measure** — `n..n` degenerates to
exactly-measure-`n`; exact COUNT is the `const 1` reading. -/
theorem measure_point_admits_iff (wt : α → Nat) (n : Nat)
    (s : Set α) :
    (Window.mk n (some n)).admitsMeasure wt s ↔
      s.ExactMeasure wt n := by
  constructor
  · intro h
    exact ⟨h.1, h.2 n rfl⟩
  · intro h
    refine ⟨h.1, fun m hm => ?_⟩
    injection hm with hm
    exact hm ▸ h.2

/-! ## Reading syntax against rows — weights and bounds resolve -/

/-- The u64 payload of a value, junk-total: every non-u64 shape reads
0 (recorded narrowing — the acceptance gate's weight/bound typing has
already refused every shape this default could distinguish). -/
def Value.u64Nat : Value → Nat
  | { type := .u64, val := v } => v.val
  | _ => 0

/-- The interval measure of a value, junk-total — the Duration
extractor at the denotation altitude: a general interval reads
`«end» − start` through `Interval.measure` with the RAY defaulting to
0 (recorded narrowing; ruling C10 makes a ray-valued Duration weight
or bound a typed COMMIT refusal at the engine's law site, so the
junk value is unobservable on judged commits — `measure_ray_none` is
the law it enforces); a fixed-width value reads its constant width
(`fixed_measure_const_u64`); every non-interval shape reads 0. -/
def Value.durationNat : Value → Nat
  | { type := .interval .u64, val := iv } => iv.measure.getD 0
  | { type := .interval .i64, val := iv } => iv.measure.getD 0
  | { type := .intervalFixed _ w, val := _ } => w
  | _ => 0

/-- The weight's reading: the measure of one SOURCE fact
(`Weight.unit` is `const 1` — the count instance, definitionally). -/
def Weight.apply : Weight → Fact → Nat
  | .unit => fun _ => 1
  | .field i => fun f => (f i).u64Nat
  | .durationOf i => fun f => (f i).durationNat

/-- A bound's resolution against the TARGET's row (ruling C1: by
name against the whole roster — at this altitude, any field of the
fact). Literals are the degenerate constant case of the same
production. -/
def Bound.resolve : Bound → Fact → Nat
  | .lit n, _ => n
  | .targetField i, g => (g i).u64Nat
  | .targetDuration i, g => (g i).durationNat

/-- A capacity window resolves per target row to the LITERAL window —
the `{lo..hi}` object that survives the cutover (ruling C16); every
admission judgment reads this resolved form. -/
def CapWindow.resolve (w : CapWindow) (g : Fact) : Window :=
  ⟨w.lo, w.hi.map fun b => b.resolve g⟩

/-- `w'` subsumes `w`, pointwise over resolved windows: at every
target row, the resolved `w'` is at least as permissive. Dependent
bounds make the SYNTACTIC comparison a recorded seam (module doc);
the pointwise relation is the one the monotonicity theorem spends. -/
def CapWindow.Subsumes (w' w : CapWindow) : Prop :=
  ∀ g : Fact, Window.Subsumes (w'.resolve g) (w.resolve g)

/-- **The default posture is universal.** `0..*` subsumes every
capacity window at every row: a spelled statement is always a
strengthening of the default, never a repair of it. -/
theorem capWindow_star_subsumes (w : CapWindow) :
    CapWindow.Subsumes ⟨0, none⟩ w :=
  fun g => star_subsumes (w.resolve g)

/-! ## The capacity judgment -/

/-- The capacity judgment `B(Y | ψ) <=[wt]{w} A(X | φ)`: for every
selected target fact, the child group's measure under the weight lies
in the window resolved against that target's own row. The ONE
structural novelty over `CardinalityWindow` is `w.resolve g` — the
dependent-bound read at the quantified parent. ACCEPTANCE IS NOT
HERE: `Y` a key of `B` and the whole weight/bound typing roster are
acceptance premises, carried as hypotheses where a theorem spends
them — exactly the containment discipline. -/
def CapacityLaw (A : Set Fact) (φ : Selection) (X : List FieldId)
    (wt : Weight) (w : CapWindow) (B : Set Fact) (ψ : Selection)
    (Y : List FieldId) : Prop :=
  ∀ g, g ∈ B → ψ.satisfies g →
    (w.resolve g).admitsMeasure wt.apply
      (ChildGroup A φ X (g.project Y))

/-- **Behavior under the empty parent denotation.** Every capacity
law holds when no parent fact is selected — capacity constrains
measures PER PARENT and never manufactures a parent; existence
obligations are containments' alone (weight- and bound-independent). -/
theorem capacity_of_empty_parent {A : Set Fact} {φ : Selection}
    {X : List FieldId} {wt : Weight} {w : CapWindow} {B : Set Fact}
    {ψ : Selection} {Y : List FieldId}
    (hB : ∀ g, g ∈ B → ¬ ψ.satisfies g) :
    CapacityLaw A φ X wt w B ψ Y :=
  fun g hg hψ => absurd hψ (hB g hg)

/-- **Window monotonicity.** The judgment is preserved by widening
the window — pointwise over resolved bounds, per parent via
`Window.admitsMeasure_of_subsumes`. -/
theorem capacity_window_mono {A : Set Fact} {φ : Selection}
    {X : List FieldId} {wt : Weight} {w w' : CapWindow}
    {B : Set Fact} {ψ : Selection} {Y : List FieldId}
    (h : CapacityLaw A φ X wt w B ψ Y)
    (hsub : CapWindow.Subsumes w' w) :
    CapacityLaw A φ X wt w' B ψ Y :=
  fun g hg hψ =>
    Window.admitsMeasure_of_subsumes (h g hg hψ) (hsub g)

/-- **The `0..*` statement says nothing, under every weight** — the
default posture holds of every instance, so an unspelled bound never
gates a commit (vacuity is weight-independent). -/
theorem capacity_zero_star (A : Set Fact) (φ : Selection)
    (X : List FieldId) (wt : Weight) (B : Set Fact) (ψ : Selection)
    (Y : List FieldId) :
    CapacityLaw A φ X wt ⟨0, none⟩ B ψ Y :=
  fun _ _ _ => measure_zero_star_admits wt.apply _

/-- **The literal point window is exact measure**, per parent — the
general point theorem speaks of exact MEASURE; the exact-count
reading is the unit-weight instance. -/
theorem capacity_point_exact {A : Set Fact} {φ : Selection}
    {X : List FieldId} {wt : Weight} {n : Nat} {B : Set Fact}
    {ψ : Selection} {Y : List FieldId} :
    CapacityLaw A φ X wt ⟨n, some (.lit n)⟩ B ψ Y ↔
      ∀ g, g ∈ B → ψ.satisfies g →
        (ChildGroup A φ X (g.project Y)).ExactMeasure wt.apply n := by
  constructor
  · intro h g hg hψ
    exact (measure_point_admits_iff wt.apply n _).mp (h g hg hψ)
  · intro h g hg hψ
    exact (measure_point_admits_iff wt.apply n _).mpr (h g hg hψ)

/-- **The count window IS the unit-weight capacity law** — the
ladder's top rung closing over its bottom: a literal-bound capacity
statement at unit weight says exactly what the cardinality window
said, so the build lane's deletion of the count path loses nothing
and the corpus's count cases re-encode with every verdict
unchanged. -/
theorem cardinality_is_unit_capacity {A : Set Fact} {φ : Selection}
    {X : List FieldId} {w : Window} {B : Set Fact} {ψ : Selection}
    {Y : List FieldId} :
    CapacityLaw A φ X .unit ⟨w.lo, w.hi.map .lit⟩ B ψ Y ↔
      CardinalityWindow A φ X w B ψ Y := by
  unfold CapacityLaw CardinalityWindow
  refine forall_congr' fun g => forall_congr' fun _ =>
    forall_congr' fun _ => ?_
  have hres : (CapWindow.mk w.lo (w.hi.map Bound.lit)).resolve g = w := by
    cases w with
    | mk lo hi =>
      cases hi with
      | none => rfl
      | some m => rfl
  rw [hres]
  unfold Window.admitsMeasure Window.admits
  constructor
  · rintro ⟨h1, h2⟩
    exact ⟨(Set.measureAtLeast_unit_iff _ _).mp h1,
      fun m hm => (Set.measureAtMost_unit_iff _ _).mp (h2 m hm)⟩
  · rintro ⟨h1, h2⟩
    exact ⟨(Set.measureAtLeast_unit_iff _ _).mpr h1,
      fun m hm => (Set.measureAtMost_unit_iff _ _).mpr (h2 m hm)⟩

end Bumbledb
