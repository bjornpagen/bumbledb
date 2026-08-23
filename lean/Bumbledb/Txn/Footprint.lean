import Bumbledb.Txn.DeltaRestriction

/-!
# Footprint — the conflict algebra (Level 2, the concurrency face)

Two batches built on one base commute exactly when their footprints
stay apart. This module gives that sentence its mathematics over the
existing transaction model: the statement-indexed footprint keys
(`FKey`, four classes F/K/C/W), the emission of a batch's footprint
from its net delta and the theory's statement roster (`footprint`),
the disjointness the fast path tests (`KeyDisjoint`), the W-class
interval test (`wIntervalTest`), and the five theorems the
replication design ships behind — L6 footprint soundness, L7
footprint stability (the strengthening of the delta-restriction
theorems this file sits beside; the republish-without-re-judgment
license), L8 commutativity, L9 component independence, and L10 replay
idempotence (the recovery theorem). `Delta`, `Delta.applyTo`,
`judge`, `holds`, and the den-transfer lemmas of `DeltaRestriction`
are consumed, never restated.

## Footprint keys are raw-value projections (the fkey narrowing)

The driver keys every footprint entry by a 32-byte blake3 of the
statement identity and the tagged raw values of the statement's own
projection, and fact identity by the relation and the full raw row.
The hash is identity REPRESENTATION — collision-freeness is its
contract — so the model keys obligations by the value the hash names:
the statement index plus the projected raw value tuple. A `Fact` here
IS its raw value assignment, so fact identity (`FKey.F`) is the pair
of relation and fact — equal keys mean equal values by construction,
with no store-relative aliasing in reach.

## The det projection is the star-guard, per statement

Every statement's obligation instance is named by one full projection
each row computes from its own raw values. For a scalar key that is
the determinant; for a POINTWISE key it is the scalar prefix
(`keyDet`) — the obligation is the group, and two same-group inserts
carrying distinct intervals must share a key or L7 is false. A
containment keys by the target determinant, which is the source
projection's value space (`srcDet`/`tgtDet`; the coverage reading
keys by the two scalar group prefixes). A capacity statement keys by
the parent determinant, reached from a child through the source
projection and from a parent row through the target projection.

## The emissions, per class

* **F** — every fact the net delta writes, either mode
  (`Delta.touches`).
* **K** — for every declared key statement of a touched relation, the
  touched rows' det projections (`Delta.projected` at `keyDet`).
* **C** — `need`: an ADDED φ-source row's target det; `support±`: a
  touched ψ-target row's det (`cTouch`). A deleted source row emits
  nothing: a withdrawn demand can only relax a containment, and the
  silence is load-bearing for the acceptance form below.
* **W** — a touched φ-child row's parent det and a touched ψ-parent
  row's own det (`wTouch`).

## Sharing is mode-blind, and the hypothesis is strict

`KeyDisjoint` demands no shared key of ANY class — commute cells
included. The mode (insert vs delete, need vs support) is the
commutativity matrices' cell coordinate, not part of the key, so two
batches inserting one byte-identical row SHARE its F key and are
outside the hypothesis: the second batch's op would evaporate against
the first's effect, breaking op-effect independence — L6's
evaporation conclusion is false without the exclusion. Under full
disjointness the W rider ("shared W parents additionally pass the
interval test") holds vacuously: no W parent is shared.

## L7 is the acceptance form (recorded weakening)

L7 as proved: a batch ACCEPTED at its base is accepted at the
winner-moved base, with the identical net effect — the verdict the
loser algebra republishes and the publish law reads. The
rejected-verdict converse is refused, with this countermodel: base
holds one φ-source row and its ψ-target supporter; σ deletes the
source (no C emission — the silence above), δ deletes the supporter
(`support−`). Footprints disjoint in every class, yet δ is rejected
at base and accepted at base ⊕ σ. The asymmetry is deliberate in the
algebra — a σ source-delete only shrinks the demand roster a
`support−` obligation reads — and harmless to the protocol: a
rejected batch returns to its host and never publishes, so nothing
downstream ever spends rejection stability.

## The W interval test, and the two hypotheses

Set semantics can evaporate a child op against the final base, so a
batch's effective measure delta at any reachable base lies inside the
interval bounded by its child inserts above and its child deletes
below. `WInterval` carries the endpoints, `WInterval.bounds` pins
them to the actual child sets (carried AND checked — the
recompute-verify habit), and `wIntervalTest` is the worst-case
endpoint test against the base group's slack.
`wIntervalTest_admitsMeasure` is the test's verdict-carrying core: at
one parent group, a passing test yields the final state's window
admission outright — floor and ceiling — with no re-judgment. Two
hypotheses therefore exist side by side. `KeyDisjoint` is the strict
form — no shared key of any class — and carries the five NAMED
theorems, L6 included (a tested shared parent moves a measure the
loser reads, which is precisely what L6's instance-preservation
conclusions rule out). `TestedDisjoint` is the loser algebra's real
fast-path hypothesis — the boolean classes strictly apart, every
shared W parent licensed per det by untouched parent rows and a
passing per-row test — and `L7_tested`/`L8_tested` ship the optimism
path under it; `KeyDisjoint.tested` embeds the strict form as the
vacuous-license case, and `L7`/`L8` are the embeddings spent.

## Narrowings recorded (law 5: narrow and record)

* **The net delta is the emission source.** `Delta` is the coalesced
  set pair, so F entries read `adds`/`removes` directly; an op list
  reaches this model through its net pair, exactly as the write path
  coalesces before judgment.
* **Emission is closed-roster-blind.** The driver skips closed
  relations (sealed rows never change); the model emits for any
  touched relation. Wider emission only strengthens the hypothesis,
  and the write surface refuses closed writes outright
  (`ClosedRelationWrite`, the `Txn.lean` narrowing), so admitted
  deltas never touch a closed relation and the two emissions
  coincide. Every den-level transfer lemma cases on the closed
  roster anyway, so even an unadmitted delta breaks nothing.
* **`need` is φ-qualified.** Only a φ-satisfying source insert
  demands a witness, so only it emits. The driver may emit
  selection-blind; that too is only wider.
* **The K class is emitted per DECLARED key statement** — the
  statement roster is descriptor data, and an undeclared key
  constrains nothing.
* **L7 carries the winner's and the loser's own acceptance as
  hypotheses**, never the base state's: both arrive free in the
  protocol (the winner published; the loser judged locally), and the
  proof spends nothing else.
-/

namespace Bumbledb
namespace Txn

/-! ## The touched facts and the net effect of a batch -/

/-- The facts one batch writes at `R`: its net insert and delete
sets, mode-blind — the F-class key roster, and the touched notion
every emission below projects. -/
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

/-- The effective inserts of a batch at one base: the added facts the
base does not already hold. The publish law reads this — a commit is
state-changing exactly when an effective set is nonempty. -/
def Delta.effAdds (d : Delta) (J : Instance) (R : RelId) : Set Fact :=
  fun f => f ∈ d.adds R ∧ f ∉ J R

/-- The effective deletes of a batch at one base: the removed facts
the base holds and the batch does not re-add (an add of a removed
fact lands present — the net survivor, `apply`'s own reading). -/
def Delta.effRemoves (d : Delta) (J : Instance) (R : RelId) :
    Set Fact :=
  fun f => f ∈ d.removes R ∧ f ∉ d.adds R ∧ f ∈ J R

/-- A batch whose effects the state already contains: every add is
present, every remove is absent. L10's hypothesis — the state a crash
window replays into after its own commit landed. -/
def Delta.containedIn (d : Delta) (I : Instance) : Prop :=
  (∀ R f, f ∈ d.adds R → f ∈ I R) ∧
    ∀ R f, f ∈ d.removes R → f ∉ I R

namespace Footprint

/-! ## Presence and denotation transfer at untouched facts -/

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

/-- Raw presence under an outer layer is stable when the INNER layer
does not touch the fact. -/
theorem applyTo_inner_untouched {s d : Delta} {R : RelId} {f : Fact}
    (h : ¬ s.touches R f) (I : Instance) :
    f ∈ d.applyTo (s.applyTo I) R ↔ f ∈ d.applyTo I R := by
  constructor
  · intro hf
    rcases mem_applyTo.mp hf with ⟨h1, h2⟩ | h1
    · exact mem_applyTo.mpr (Or.inl ⟨(applyTo_untouched h I).mp h1, h2⟩)
    · exact mem_applyTo.mpr (Or.inr h1)
  · intro hf
    rcases mem_applyTo.mp hf with ⟨h1, h2⟩ | h1
    · exact mem_applyTo.mpr
        (Or.inl ⟨(applyTo_untouched h I).mpr h1, h2⟩)
    · exact mem_applyTo.mpr (Or.inr h1)

/-- Denotation membership is stable at an untouched fact — the
`den_untouched_iff` move keyed by `Delta.touches`. -/
theorem den_untouched {T : Theory} {I : Instance} {d : Delta}
    {R : RelId} {f : Fact} (h : ¬ d.touches R f) :
    f ∈ T.den (d.applyTo I) R ↔ f ∈ T.den I R :=
  den_untouched_iff (Delta.not_adds_of_not_touches h)
    (Delta.not_removes_of_not_touches h)

/-- Denotation membership under an outer layer is stable when the
inner layer does not touch the fact — how a witness read at one base
carries to the winner-moved base. -/
theorem den_inner_untouched {T : Theory} {s d : Delta} {R : RelId}
    {f : Fact} (h : ¬ s.touches R f) (I : Instance) :
    f ∈ T.den (d.applyTo (s.applyTo I)) R ↔
      f ∈ T.den (d.applyTo I) R := by
  cases hc : T.closed R with
  | some ext =>
    rw [den_closed_constant hc (d.applyTo (s.applyTo I)) (d.applyTo I)]
  | none =>
    simp only [Theory.den, hc]
    exact applyTo_inner_untouched h I

/-- An added fact stands in the batch's final denotation over EVERY
base — the insert is the net survivor whatever the base held. -/
theorem den_added_carries {T : Theory} {d : Delta} {R : RelId}
    {f : Fact} {J J' : Instance} (hf : f ∈ T.den (d.applyTo J) R)
    (ha : f ∈ d.adds R) : f ∈ T.den (d.applyTo J') R := by
  cases hc : T.closed R with
  | some ext =>
    rw [← den_closed_constant hc (d.applyTo J) (d.applyTo J')]
    exact hf
  | none =>
    simp only [Theory.den, hc] at hf ⊢
    exact mem_applyTo.mpr (Or.inr ha)

/-- A fact that survives the batch at one base and is not the batch's
own insert is not net-removed, so it survives at any base holding
it — the move that walks a base survivor from the winner-moved final
back to the loser's own final. -/
theorem den_survivor_carries {T : Theory} {d : Delta} {R : RelId}
    {f : Fact} {J J' : Instance} (hf : f ∈ T.den (d.applyTo J) R)
    (hna : f ∉ d.adds R) (hpre : f ∈ T.den J' R) :
    f ∈ T.den (d.applyTo J') R := by
  cases hc : T.closed R with
  | some ext =>
    rw [← den_closed_constant hc J' (d.applyTo J')]
    exact hpre
  | none =>
    simp only [Theory.den, hc] at hf hpre ⊢
    rcases mem_applyTo.mp hf with ⟨_, hnr⟩ | hadd
    · exact mem_applyTo.mpr (Or.inl ⟨hpre, hnr⟩)
    · exact absurd hadd hna

/-- A fact whose det projection the batch's footprint does not name
is untouched — the contrapositive every K-disjointness argument
spends: a touched fact projects its own det. -/
theorem untouched_of_det_silent {d : Delta} {R : RelId}
    {P : List FieldId} {f : Fact}
    (h : f.project P ∉ d.projected R P) : ¬ d.touches R f :=
  fun ht => h ⟨f, ht, rfl⟩

/-! ## The footprint keys -/

/-- The det projection of one key statement — the star-guard that
names its obligation instances: the determinant for a scalar key, the
scalar PREFIX for a pointwise key (the obligation is the group; two
same-group rows with distinct intervals must share the key). -/
def keyDet (T : Theory) (R : RelId) (X : List FieldId) :
    List FieldId :=
  match T.header.intervalSplit R X with
  | some (S, _) => S
  | none => X

/-- The source-side key projection of one containment: the full
source projection under the scalar reading, the scalar group prefix
under the coverage reading — the same dispatch `deltaCheck` runs. -/
def srcDet (T : Theory) (src tgt : Atom) : List FieldId :=
  match T.header.intervalSplit src.relation src.projection,
        T.header.intervalSplit tgt.relation tgt.projection with
  | some (S, _), some _ => S
  | _, _ => src.projection

/-- The target-side key projection of one containment — the target
determinant naming the obligation instance, group-prefixed under the
coverage reading. -/
def tgtDet (T : Theory) (src tgt : Atom) : List FieldId :=
  match T.header.intervalSplit src.relation src.projection,
        T.header.intervalSplit tgt.relation tgt.projection with
  | some _, some (U, _) => U
  | _, _ => tgt.projection

/-- The C-class emission of one containment statement at one target
det: `need` — an ADDED φ-source row references the group — or
`support±` — a touched ψ-target row establishes or supported it. A
deleted source row emits nothing (module doc: the silence is the
acceptance form's reason). -/
def cTouch (d : Delta) (Rs : RelId) (φ : Selection)
    (Ps : List FieldId) (Rt : RelId) (ψ : Selection)
    (Pt : List FieldId) (t : List Value) : Prop :=
  (∃ f, f ∈ d.adds Rs ∧ φ.satisfies f ∧ f.project Ps = t) ∨
    ∃ g, d.touches Rt g ∧ ψ.satisfies g ∧ g.project Pt = t

/-- The W-class emission of one capacity statement at one parent det:
a touched φ-child row projecting to the parent, or a touched ψ-parent
row itself (`parent±`). -/
def wTouch (d : Delta) (src tgt : Atom) (t : List Value) : Prop :=
  (∃ f, d.touches src.relation f ∧ src.selection.satisfies f ∧
      f.project src.projection = t) ∨
    ∃ g, d.touches tgt.relation g ∧ tgt.selection.satisfies g ∧
      g.project tgt.projection = t

/-- A footprint key: the statement-indexed projection of raw row
values the 32-byte fkey names (module doc). The mode — insert vs
delete, need vs support, the signed W delta — is the commutativity
matrices' cell coordinate, deliberately NOT part of the key: sharing
is mode-blind, which is the strictness L6 needs. -/
inductive FKey where
  /-- Fact identity: the relation and the full raw row. -/
  | F (R : RelId) (f : Fact)
  /-- A key statement's obligation: its det projection's raw tuple. -/
  | K (R : RelId) (X : List FieldId) (det : List Value)
  /-- A containment's obligation: the target det's raw tuple. -/
  | C (src tgt : Atom) (det : List Value)
  /-- A capacity's obligation: the parent det's raw tuple. -/
  | W (tgt : Atom) (wt : Weight) (w : CapWindow) (src : Atom)
      (det : List Value)

/-- The footprint of one batch under one theory: a pure function of
the net delta and the statement roster — no state, no engine, no
intern table. Verification is recomputation: any holder of the ops
rebuilds this same set. -/
def footprint (T : Theory) (d : Delta) : Set FKey := fun k =>
  match k with
  | .F R f => d.touches R f
  | .K R X det =>
    Statement.functionality R X ∈ T.statements ∧
      det ∈ d.projected R (keyDet T R X)
  | .C src tgt det =>
    Statement.containment src tgt ∈ T.statements ∧
      cTouch d src.relation src.selection (srcDet T src tgt)
        tgt.relation tgt.selection (tgtDet T src tgt) det
  | .W tgt wt w src det =>
    Statement.capacity tgt wt w src ∈ T.statements ∧
      wTouch d src tgt det

/-- Full key disjointness — the loser algebra's fast-path test: the
two batches share no footprint key of any class, commute cells
included. Deliberately stronger than "no CONFLICT cell" (module doc);
under it the W rider is vacuous. -/
def KeyDisjoint (T : Theory) (a b : Delta) : Prop :=
  ∀ k, k ∈ footprint T a → k ∉ footprint T b

/-- Disjointness is symmetric — the intersection either loser
computes is one set. -/
theorem KeyDisjoint.symm {T : Theory} {a b : Delta}
    (h : KeyDisjoint T a b) : KeyDisjoint T b a :=
  fun k hkb hka => h k hka hkb

/-! ## The W interval test — the measured class's arithmetic

The W class is quantitative: evaporation against the final base can
cancel any child op, so a batch's effective measure delta at any
reachable base lies in the interval its child inserts bound above and
its child deletes bound below. The test compares worst-case endpoint
sums against the base group's slack; passing it carries the window
verdict to the moved base with no re-judgment
(`wIntervalTest_admitsMeasure`). -/

/-- The φ-child inserts of one batch at one parent det. -/
def childAdds (d : Delta) (src : Atom) (t : List Value) : Set Fact :=
  fun f => f ∈ d.adds src.relation ∧ src.selection.satisfies f ∧
    f.project src.projection = t

/-- The φ-child deletes of one batch at one parent det. -/
def childRemoves (d : Delta) (src : Atom) (t : List Value) :
    Set Fact :=
  fun f => f ∈ d.removes src.relation ∧ src.selection.satisfies f ∧
    f.project src.projection = t

/-- One batch's evaporation interval at one parent: `up` bounds the
effective delta above (every child insert lands), `down` bounds it
below (every child delete lands). The published Δ with the F± sums
re-derives exactly these endpoints. -/
structure WInterval where
  /-- The maximal effective gain: total weight of the child inserts. -/
  up : Nat
  /-- The maximal effective loss: total weight of the child deletes. -/
  down : Nat

/-- The endpoints are carried AND checked: each bounds the measure of
the batch's actual child set — the recompute-verify habit at the
arithmetic layer. -/
def WInterval.bounds (iv : WInterval) (wt : Weight) (d : Delta)
    (src : Atom) (t : List Value) : Prop :=
  Set.MeasureAtMost wt.apply (childAdds d src t) iv.up ∧
    Set.MeasureAtMost wt.apply (childRemoves d src t) iv.down

/-- The interval test at one parent group of base measure `m` under
the resolved window: the worst-case combined gain respects the
ceiling and the worst-case combined loss respects the floor — the
verdict-flip boundary, exactly. -/
def wIntervalTest (win : Window) (m : Nat) (a b : WInterval) : Prop :=
  (∀ hi, win.hi = some hi → m + (a.up + b.up) ≤ hi) ∧
    win.lo + (a.down + b.down) ≤ m

/-- Splitting one duplicate-free list by a predicate: two
duplicate-free halves inside the host, one carrying the predicate and
one refuting it, whose mapped sums recompose the host's under every
weight. The one list engine both interval-test bounds walk. -/
theorem nodup_split_by (P : α → Prop) :
    ∀ l : List α, l.Nodup →
      ∃ l₁ l₂ : List α, l₁.Nodup ∧ l₂.Nodup ∧
        (∀ x, x ∈ l₁ → x ∈ l ∧ P x) ∧
        (∀ x, x ∈ l₂ → x ∈ l ∧ ¬ P x) ∧
        ∀ w : α → Nat,
          natSum (l.map w) = natSum (l₁.map w) + natSum (l₂.map w)
  | [], _ => by
    refine ⟨[], [], List.Pairwise.nil, List.Pairwise.nil, ?_, ?_,
      fun _ => rfl⟩
    · intro x hx
      cases hx
    · intro x hx
      cases hx
  | a :: l, hnd => by
    obtain ⟨hne, hnd'⟩ := List.pairwise_cons.mp hnd
    obtain ⟨l₁, l₂, hnd₁, hnd₂, hm₁, hm₂, hsum⟩ :=
      nodup_split_by P l hnd'
    rcases Classical.em (P a) with hp | hp
    · refine ⟨a :: l₁, l₂,
        List.Pairwise.cons (fun y hy => hne y (hm₁ y hy).1) hnd₁,
        hnd₂, ?_, ?_, ?_⟩
      · intro x hx
        rcases List.mem_cons.mp hx with rfl | hx'
        · exact ⟨List.mem_cons_self, hp⟩
        · exact ⟨List.mem_cons_of_mem a (hm₁ x hx').1, (hm₁ x hx').2⟩
      · intro x hx
        exact ⟨List.mem_cons_of_mem a (hm₂ x hx).1, (hm₂ x hx).2⟩
      · intro w
        have h := hsum w
        rw [List.map_cons, List.map_cons]
        show w a + natSum (l.map w) =
          w a + natSum (l₁.map w) + natSum (l₂.map w)
        omega
    · refine ⟨l₁, a :: l₂, hnd₁,
        List.Pairwise.cons (fun y hy => hne y (hm₂ y hy).1) hnd₂,
        ?_, ?_, ?_⟩
      · intro x hx
        exact ⟨List.mem_cons_of_mem a (hm₁ x hx).1, (hm₁ x hx).2⟩
      · intro x hx
        rcases List.mem_cons.mp hx with rfl | hx'
        · exact ⟨List.mem_cons_self, hp⟩
        · exact ⟨List.mem_cons_of_mem a (hm₂ x hx').1, (hm₂ x hx').2⟩
      · intro w
        have h := hsum w
        rw [List.map_cons, List.map_cons]
        show w a + natSum (l.map w) =
          natSum (l₁.map w) + (w a + natSum (l₂.map w))
        omega

/-- A ceiling transfers down a subset — the universal bound's free
monotonicity. -/
theorem measureAtMost_subset {w : α → Nat} {A B : Set α} {m : Nat}
    (hsub : ∀ x, x ∈ A → x ∈ B) (h : Set.MeasureAtMost w B m) :
    Set.MeasureAtMost w A m :=
  fun l hnd hmem => h l hnd fun x hx => hsub x (hmem x hx)

/-- A ceiling loosens to any larger bound. -/
theorem measureAtMost_mono {w : α → Nat} {A : Set α} {n m : Nat}
    (h : Set.MeasureAtMost w A n) (hle : n ≤ m) :
    Set.MeasureAtMost w A m :=
  fun l hnd hmem => Nat.le_trans (h l hnd hmem) hle

/-- Two ceilings add across a union: any duplicate-free member list
splits into the two sides, and the halves' sums bound separately. -/
theorem measureAtMost_union {w : α → Nat} {A B : Set α} {a b : Nat}
    (hA : Set.MeasureAtMost w A a) (hB : Set.MeasureAtMost w B b) :
    Set.MeasureAtMost w (fun x => x ∈ A ∨ x ∈ B) (a + b) := by
  intro l hnd hsub
  obtain ⟨l₁, l₂, hnd₁, hnd₂, hm₁, hm₂, hsum⟩ :=
    nodup_split_by (fun x => x ∈ A) l hnd
  have h₁ := hA l₁ hnd₁ fun x hx => (hm₁ x hx).2
  have h₂ := hB l₂ hnd₂ fun x hx =>
    (hsub x (hm₂ x hx).1).resolve_left (hm₂ x hx).2
  have h := hsum w
  omega

/-- A final-state child either stood in the base group or is one
batch's own insert — the ceiling side's set sandwich. -/
theorem childGroup_final_split {T : Theory} {I : Instance}
    {s d : Delta} {src : Atom} {t : List Value} {f : Fact}
    (hf : f ∈ ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
      src.selection src.projection t) :
    f ∈ ChildGroup (T.den I src.relation) src.selection
        src.projection t ∨
      (f ∈ childAdds s src t ∨ f ∈ childAdds d src t) := by
  obtain ⟨hfd, hφ, hp⟩ := mem_childGroup.mp hf
  rcases den_final_pre_or_added hfd with h1 | h1
  · rcases den_final_pre_or_added h1 with h2 | h2
    · exact Or.inl (mem_childGroup.mpr ⟨h2, hφ, hp⟩)
    · exact Or.inr (Or.inl ⟨h2, hφ, hp⟩)
  · exact Or.inr (Or.inr ⟨h1, hφ, hp⟩)

/-- A base child neither batch deletes survives to the final group —
the floor side's witness transfer. -/
theorem childGroup_keeper_carries {T : Theory} {I : Instance}
    {s d : Delta} {src : Atom} {t : List Value} {f : Fact}
    (hf : f ∈ ChildGroup (T.den I src.relation) src.selection
      src.projection t)
    (hs : f ∉ childRemoves s src t) (hd : f ∉ childRemoves d src t) :
    f ∈ ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
      src.selection src.projection t := by
  obtain ⟨hf0, hφ, hp⟩ := mem_childGroup.mp hf
  have h1 : f ∈ T.den (s.applyTo I) src.relation := by
    rcases den_pre_final_or_removed (d := s) hf0 with h | h
    · exact h
    · exact absurd ⟨h, hφ, hp⟩ hs
  have h2 : f ∈ T.den (d.applyTo (s.applyTo I)) src.relation := by
    rcases den_pre_final_or_removed (d := d) h1 with h | h
    · exact h
    · exact absurd ⟨h, hφ, hp⟩ hd
  exact mem_childGroup.mpr ⟨h2, hφ, hp⟩

/-- **The interval test carries the window verdict.** At one parent
group of exact base measure `m`, with both batches' endpoints honest
(`WInterval.bounds`), a passing test yields the FINAL state's window
admission — floor and ceiling — after both batches land in either
order's final state: the ceiling because the final group sits inside
base ∪ inserts and ceilings add across unions; the floor because the
base floor witness filtered of both delete sets survives whole and
loses at most the delete endpoints. This is the arithmetic that
settles a shared W parent without re-judgment — the tested
hypothesis spends it (`capacity_stable_tested`), and the
strict-disjoint theorems never need it (no parent is shared). -/
theorem wIntervalTest_admitsMeasure {T : Theory} {I : Instance}
    {s d : Delta} {src : Atom} {wt : Weight} {win : Window}
    {t : List Value} {m : Nat} {a b : WInterval}
    (hm : Set.ExactMeasure wt.apply
      (ChildGroup (T.den I src.relation) src.selection
        src.projection t) m)
    (ha : a.bounds wt s src t) (hb : b.bounds wt d src t)
    (htest : wIntervalTest win m a b) :
    win.admitsMeasure wt.apply
      (ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
        src.selection src.projection t) := by
  constructor
  · obtain ⟨l, hnd, hmem, hle⟩ := hm.1
    obtain ⟨l₁, l₂, hnd₁, hnd₂, hm₁, hm₂, hsum⟩ :=
      nodup_split_by
        (fun f => f ∈ childRemoves s src t ∨ f ∈ childRemoves d src t)
        l hnd
    have hdrop : natSum (l₁.map wt.apply) ≤ a.down + b.down :=
      measureAtMost_union ha.2 hb.2 l₁ hnd₁ fun x hx => (hm₁ x hx).2
    refine ⟨l₂, hnd₂, ?_, ?_⟩
    · intro x hx
      exact childGroup_keeper_carries (hmem x (hm₂ x hx).1)
        (fun hin => (hm₂ x hx).2 (Or.inl hin))
        (fun hin => (hm₂ x hx).2 (Or.inr hin))
    · have h := hsum wt.apply
      have h2 := htest.2
      omega
  · intro hi hhi
    have hup : Set.MeasureAtMost wt.apply
        (fun f => f ∈ childAdds s src t ∨ f ∈ childAdds d src t)
        (a.up + b.up) := measureAtMost_union ha.1 hb.1
    have hall : Set.MeasureAtMost wt.apply
        (fun f => f ∈ ChildGroup (T.den I src.relation) src.selection
            src.projection t ∨
          (f ∈ childAdds s src t ∨ f ∈ childAdds d src t))
        (m + (a.up + b.up)) := by
      intro l hnd hsub
      obtain ⟨l₁, l₂, hnd₁, hnd₂, hm₁, hm₂, hsum⟩ :=
        nodup_split_by
          (fun f => f ∈ ChildGroup (T.den I src.relation)
            src.selection src.projection t) l hnd
      have h₁ := hm.2 l₁ hnd₁ fun x hx => (hm₁ x hx).2
      have h₂ := hup l₂ hnd₂ fun x hx =>
        (hsub x (hm₂ x hx).1).resolve_left (hm₂ x hx).2
      have h := hsum wt.apply
      omega
    exact measureAtMost_mono
      (measureAtMost_subset (fun f hf => childGroup_final_split hf)
        hall)
      (htest.1 hi hhi)

/-! ## Placement — where a final-state fact's judgment reads land -/

/-- Two final-state facts sharing one det place together: at the
winner's state when the winner touched the det (the loser then
cannot have — K disjointness), at the loser's own final when it did
not — so a pair judgment transfers whole to a state whose verdict is
already in hand. -/
theorem pair_placement {T : Theory} {I : Instance} {s d : Delta}
    {R : RelId} {P : List FieldId}
    (hdisj : ∀ t, t ∈ s.projected R P → t ∉ d.projected R P)
    {f g : Fact}
    (hf : f ∈ T.den (d.applyTo (s.applyTo I)) R)
    (hg : g ∈ T.den (d.applyTo (s.applyTo I)) R)
    (hproj : f.project P = g.project P) :
    (f ∈ T.den (d.applyTo I) R ∧ g ∈ T.den (d.applyTo I) R) ∨
      (f ∈ T.den (s.applyTo I) R ∧ g ∈ T.den (s.applyTo I) R) := by
  by_cases hσ : f.project P ∈ s.projected R P
  · have hδf : ¬ d.touches R f := untouched_of_det_silent (hdisj _ hσ)
    have hσg : g.project P ∈ s.projected R P := hproj ▸ hσ
    have hδg : ¬ d.touches R g := untouched_of_det_silent (hdisj _ hσg)
    exact Or.inr ⟨(den_untouched hδf).mp hf, (den_untouched hδg).mp hg⟩
  · have hplace : ∀ h : Fact,
        h ∈ T.den (d.applyTo (s.applyTo I)) R →
        h.project P = f.project P → h ∈ T.den (d.applyTo I) R := by
      intro h hh hp
      by_cases hha : h ∈ d.adds R
      · exact den_added_carries hh hha
      · rcases den_final_pre_or_added hh with h1 | h1
        · have hnt : ¬ s.touches R h :=
            untouched_of_det_silent (fun hx => hσ (hp ▸ hx))
          exact den_survivor_carries hh hha ((den_untouched hnt).mp h1)
        · exact absurd h1 hha
    exact Or.inl ⟨hplace f hf rfl, hplace g hg hproj.symm⟩

/-- A final-state φ-source places at the loser's own final or at the
winner's state, and in either case every ψ-row of its target det is
untouched by the OTHER batch — so the witness the placed state's
verdict supplies transfers to the two-batch final. The case engine of
both containment readings. -/
theorem source_placement {T : Theory} {I : Instance} {s d : Delta}
    {Rs : RelId} {φ : Selection} {Ps : List FieldId} {Rt : RelId}
    {ψ : Selection} {Pt : List FieldId}
    (hdisj : ∀ t, cTouch s Rs φ Ps Rt ψ Pt t →
      ¬ cTouch d Rs φ Ps Rt ψ Pt t)
    {f : Fact} (hf : f ∈ T.den (d.applyTo (s.applyTo I)) Rs)
    (hφ : φ.satisfies f) :
    (f ∈ T.den (d.applyTo I) Rs ∧
        ∀ g, ψ.satisfies g → g.project Pt = f.project Ps →
          ¬ s.touches Rt g) ∨
      (f ∈ T.den (s.applyTo I) Rs ∧
        ∀ g, ψ.satisfies g → g.project Pt = f.project Ps →
          ¬ d.touches Rt g) := by
  by_cases hfa : f ∈ d.adds Rs
  · have hσ : ¬ cTouch s Rs φ Ps Rt ψ Pt (f.project Ps) := fun hc =>
      hdisj _ hc (Or.inl ⟨f, hfa, hφ, rfl⟩)
    refine Or.inl ⟨den_added_carries hf hfa, ?_⟩
    intro g hψ hp hto
    exact hσ (Or.inr ⟨g, hto, hψ, hp⟩)
  · rcases den_final_pre_or_added hf with hf1 | hf1
    · by_cases hsd : ∃ g, d.touches Rt g ∧ ψ.satisfies g ∧
          g.project Pt = f.project Ps
      · have hσ : ¬ cTouch s Rs φ Ps Rt ψ Pt (f.project Ps) :=
          fun hc => hdisj _ hc (Or.inr hsd)
        rcases den_final_pre_or_added hf1 with hf0 | hf0
        · refine Or.inl ⟨den_survivor_carries hf hfa hf0, ?_⟩
          intro g hψ hp hto
          exact hσ (Or.inr ⟨g, hto, hψ, hp⟩)
        · exact absurd (Or.inl ⟨f, hf0, hφ, rfl⟩) hσ
      · refine Or.inr ⟨hf1, ?_⟩
        intro g hψ hp hto
        exact hsd ⟨g, hto, hψ, hp⟩
    · exact absurd hf1 hfa

/-! ## Per-form stability — each judgment transfers whole -/

/-- Scalar-key stability: the two-batch final is keyed, given the
loser's own final and the winner's state keyed — every colliding pair
places whole in one of them (`pair_placement`). -/
theorem functionality_stable {T : Theory} {I : Instance} {s d : Delta}
    {R : RelId} {X : List FieldId}
    (hdisj : ∀ t, t ∈ s.projected R X → t ∉ d.projected R X)
    (hacc : Functionality (T.den (d.applyTo I) R) X)
    (hwin : Functionality (T.den (s.applyTo I) R) X) :
    Functionality (T.den (d.applyTo (s.applyTo I)) R) X := by
  intro f g hf hg hproj
  rcases pair_placement hdisj hf hg hproj with ⟨h1, h2⟩ | ⟨h1, h2⟩
  · exact hacc f g h1 h2 hproj
  · exact hwin f g h1 h2 hproj

/-- Pointwise-key stability: same placement at the scalar prefix —
the group is the obligation, so the K det is the prefix and a
same-group pair transfers with its interval reads intact. -/
theorem pointwise_stable {T : Theory} {I : Instance} {s d : Delta}
    {R : RelId} {S : List FieldId} {i : FieldId}
    (hdisj : ∀ t, t ∈ s.projected R S → t ∉ d.projected R S)
    (hacc : PointwiseKey (T.den (d.applyTo I) R) S i)
    (hwin : PointwiseKey (T.den (s.applyTo I) R) S i) :
    PointwiseKey (T.den (d.applyTo (s.applyTo I)) R) S i := by
  intro f g hf hg hproj hne x hx
  rcases pair_placement hdisj hf hg hproj with ⟨h1, h2⟩ | ⟨h1, h2⟩
  · exact hacc f g h1 h2 hproj hne x hx
  · exact hwin f g h1 h2 hproj hne x hx

/-- Containment stability: every final φ-source places
(`source_placement`), the placed state's own verdict supplies the
witness, and the placement's untouched guard carries the witness to
the two-batch final. -/
theorem containment_stable {T : Theory} {I : Instance} {s d : Delta}
    {Rs : RelId} {φ : Selection} {Ps : List FieldId} {Rt : RelId}
    {ψ : Selection} {Pt : List FieldId}
    (hdisj : ∀ t, cTouch s Rs φ Ps Rt ψ Pt t →
      ¬ cTouch d Rs φ Ps Rt ψ Pt t)
    (hacc : Containment (T.den (d.applyTo I) Rs) φ Ps
      (T.den (d.applyTo I) Rt) ψ Pt)
    (hwin : Containment (T.den (s.applyTo I) Rs) φ Ps
      (T.den (s.applyTo I) Rt) ψ Pt) :
    Containment (T.den (d.applyTo (s.applyTo I)) Rs) φ Ps
      (T.den (d.applyTo (s.applyTo I)) Rt) ψ Pt := by
  intro f hf hφ
  rcases source_placement hdisj hf hφ with ⟨h1, hguard⟩ | ⟨h1, hguard⟩
  · obtain ⟨g, hg, hψ, hproj⟩ := hacc f h1 hφ
    exact ⟨g, (den_inner_untouched (hguard g hψ hproj) I).mpr hg, hψ,
      hproj⟩
  · obtain ⟨g, hg, hψ, hproj⟩ := hwin f h1 hφ
    exact ⟨g, (den_untouched (hguard g hψ hproj)).mpr hg, hψ, hproj⟩

/-- Coverage stability: the pointwise containment rides the same
placement at the group prefixes, the covering point carried through
unchanged. -/
theorem coverage_stable {T : Theory} {I : Instance} {s d : Delta}
    {Rs : RelId} {φ : Selection} {S : List FieldId} {i : FieldId}
    {Rt : RelId} {ψ : Selection} {U : List FieldId} {j : FieldId}
    (hdisj : ∀ t, cTouch s Rs φ S Rt ψ U t →
      ¬ cTouch d Rs φ S Rt ψ U t)
    (hacc : Coverage (T.den (d.applyTo I) Rs) φ S i
      (T.den (d.applyTo I) Rt) ψ U j)
    (hwin : Coverage (T.den (s.applyTo I) Rs) φ S i
      (T.den (s.applyTo I) Rt) ψ U j) :
    Coverage (T.den (d.applyTo (s.applyTo I)) Rs) φ S i
      (T.den (d.applyTo (s.applyTo I)) Rt) ψ U j := by
  intro f hf hφ x hx
  rcases source_placement hdisj hf hφ with ⟨h1, hguard⟩ | ⟨h1, hguard⟩
  · obtain ⟨g, hg, hψ, hproj, hxg⟩ := hacc f h1 hφ x hx
    exact ⟨g, (den_inner_untouched (hguard g hψ hproj) I).mpr hg, hψ,
      hproj, hxg⟩
  · obtain ⟨g, hg, hψ, hproj, hxg⟩ := hwin f h1 hφ x hx
    exact ⟨g, (den_untouched (hguard g hψ hproj)).mpr hg, hψ, hproj,
      hxg⟩

/-- The capacity verdict at one parent whose det the loser's W
footprint does not name: the parent row and its whole child group are
loser-untouched, so the winner's own verdict is the final one. -/
theorem capacity_verdict_delta_silent {T : Theory} {I : Instance}
    {s d : Delta} {tgt : Atom} {wt : Weight} {w : CapWindow}
    {src : Atom}
    (hwin : CapacityLaw (T.den (s.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (s.applyTo I) tgt.relation) tgt.selection
      tgt.projection)
    {p : Fact} (hp : p ∈ T.den (d.applyTo (s.applyTo I)) tgt.relation)
    (hψ : tgt.selection.satisfies p)
    (hδt : ¬ wTouch d src tgt (p.project tgt.projection)) :
    (w.resolve p).admitsMeasure wt.apply
      (ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
        src.selection src.projection (p.project tgt.projection)) := by
  have hpd : ¬ d.touches tgt.relation p := fun hto =>
    hδt (Or.inr ⟨p, hto, hψ, rfl⟩)
  have hgrp :
      ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
        src.selection src.projection (p.project tgt.projection) =
      ChildGroup (T.den (s.applyTo I) src.relation) src.selection
        src.projection (p.project tgt.projection) := by
    funext q
    refine propext ⟨fun hq => ?_, fun hq => ?_⟩
    · obtain ⟨hqd, hqφ, hqp⟩ := mem_childGroup.mp hq
      have hnt : ¬ d.touches src.relation q := fun hto =>
        hδt (Or.inl ⟨q, hto, hqφ, hqp⟩)
      exact mem_childGroup.mpr ⟨(den_untouched hnt).mp hqd, hqφ, hqp⟩
    · obtain ⟨hqd, hqφ, hqp⟩ := mem_childGroup.mp hq
      have hnt : ¬ d.touches src.relation q := fun hto =>
        hδt (Or.inl ⟨q, hto, hqφ, hqp⟩)
      exact mem_childGroup.mpr ⟨(den_untouched hnt).mpr hqd, hqφ, hqp⟩
  rw [hgrp]
  exact hwin p ((den_untouched hpd).mp hp) hψ

/-- The capacity verdict at one parent whose det the winner's W
footprint does not name: the parent row and its whole child group are
winner-untouched, so the loser's own verdict is the final one. -/
theorem capacity_verdict_sigma_silent {T : Theory} {I : Instance}
    {s d : Delta} {tgt : Atom} {wt : Weight} {w : CapWindow}
    {src : Atom}
    (hacc : CapacityLaw (T.den (d.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (d.applyTo I) tgt.relation) tgt.selection tgt.projection)
    {p : Fact} (hp : p ∈ T.den (d.applyTo (s.applyTo I)) tgt.relation)
    (hψ : tgt.selection.satisfies p)
    (hσt : ¬ wTouch s src tgt (p.project tgt.projection)) :
    (w.resolve p).admitsMeasure wt.apply
      (ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
        src.selection src.projection (p.project tgt.projection)) := by
  have hps : ¬ s.touches tgt.relation p := fun hto =>
    hσt (Or.inr ⟨p, hto, hψ, rfl⟩)
  have hgrp :
      ChildGroup (T.den (d.applyTo (s.applyTo I)) src.relation)
        src.selection src.projection (p.project tgt.projection) =
      ChildGroup (T.den (d.applyTo I) src.relation) src.selection
        src.projection (p.project tgt.projection) := by
    funext q
    refine propext ⟨fun hq => ?_, fun hq => ?_⟩
    · obtain ⟨hqd, hqφ, hqp⟩ := mem_childGroup.mp hq
      have hnt : ¬ s.touches src.relation q := fun hto =>
        hσt (Or.inl ⟨q, hto, hqφ, hqp⟩)
      exact mem_childGroup.mpr
        ⟨(den_inner_untouched hnt I).mp hqd, hqφ, hqp⟩
    · obtain ⟨hqd, hqφ, hqp⟩ := mem_childGroup.mp hq
      have hnt : ¬ s.touches src.relation q := fun hto =>
        hσt (Or.inl ⟨q, hto, hqφ, hqp⟩)
      exact mem_childGroup.mpr
        ⟨(den_inner_untouched hnt I).mpr hqd, hqφ, hqp⟩
  rw [hgrp]
  exact hacc p ((den_inner_untouched hps I).mp hp) hψ

/-- Capacity stability, strict form: a parent whose det the loser's W
footprint names is winner-untouched whole — parent row and child
group both — so the loser's own verdict transfers; any other parent
is loser-untouched whole and the winner's verdict transfers. The
dependent bound resolves against the parent row itself, which is the
same fact on both sides. -/
theorem capacity_stable {T : Theory} {I : Instance} {s d : Delta}
    {tgt : Atom} {wt : Weight} {w : CapWindow} {src : Atom}
    (hdisj : ∀ t, wTouch s src tgt t → ¬ wTouch d src tgt t)
    (hacc : CapacityLaw (T.den (d.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (d.applyTo I) tgt.relation) tgt.selection tgt.projection)
    (hwin : CapacityLaw (T.den (s.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (s.applyTo I) tgt.relation) tgt.selection
      tgt.projection) :
    CapacityLaw (T.den (d.applyTo (s.applyTo I)) src.relation)
      src.selection src.projection wt w
      (T.den (d.applyTo (s.applyTo I)) tgt.relation) tgt.selection
      tgt.projection := by
  intro p hp hψ
  by_cases hδt : wTouch d src tgt (p.project tgt.projection)
  · exact capacity_verdict_sigma_silent hacc hp hψ
      (fun hc => hdisj _ hc hδt)
  · exact capacity_verdict_delta_silent hwin hp hψ hδt

/-- The loser algebra's REAL W hypothesis at one capacity statement:
a shared parent det is licensed when neither batch writes a ψ-parent
row at it and, at every base parent row of the det, the interval test
passes with honest endpoints against the base group's exact
measure — the `parent±`-conflicts-with-motion rule and the endpoint
arithmetic, packaged as the per-det license. -/
def wTested (T : Theory) (I : Instance) (s d : Delta) (tgt : Atom)
    (wt : Weight) (w : CapWindow) (src : Atom) : Prop :=
  ∀ t, wTouch s src tgt t → wTouch d src tgt t →
    (∀ g, tgt.selection.satisfies g → g.project tgt.projection = t →
      ¬ s.touches tgt.relation g ∧ ¬ d.touches tgt.relation g) ∧
    ∀ p, p ∈ T.den I tgt.relation → tgt.selection.satisfies p →
      p.project tgt.projection = t →
      ∃ (m : Nat) (a b : WInterval),
        Set.ExactMeasure wt.apply
          (ChildGroup (T.den I src.relation) src.selection
            src.projection t) m ∧
        WInterval.bounds a wt s src t ∧
        WInterval.bounds b wt d src t ∧
        wIntervalTest (w.resolve p) m a b

/-- Capacity stability, tested form: shared parent dets are settled
by the interval arithmetic (`wIntervalTest_admitsMeasure`), every
other det by whichever batch is silent there — the measured class
commuting quantitatively, exactly where the boolean classes commute
by disjointness. -/
theorem capacity_stable_tested {T : Theory} {I : Instance}
    {s d : Delta} {tgt : Atom} {wt : Weight} {w : CapWindow}
    {src : Atom} (htest : wTested T I s d tgt wt w src)
    (hacc : CapacityLaw (T.den (d.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (d.applyTo I) tgt.relation) tgt.selection tgt.projection)
    (hwin : CapacityLaw (T.den (s.applyTo I) src.relation)
      src.selection src.projection wt w
      (T.den (s.applyTo I) tgt.relation) tgt.selection
      tgt.projection) :
    CapacityLaw (T.den (d.applyTo (s.applyTo I)) src.relation)
      src.selection src.projection wt w
      (T.den (d.applyTo (s.applyTo I)) tgt.relation) tgt.selection
      tgt.projection := by
  intro p hp hψ
  by_cases hδt : wTouch d src tgt (p.project tgt.projection)
  · by_cases hσt : wTouch s src tgt (p.project tgt.projection)
    · obtain ⟨hguard, hrows⟩ := htest _ hσt hδt
      obtain ⟨hpσ, hpδ⟩ := hguard p hψ rfl
      have hp1 : p ∈ T.den (s.applyTo I) tgt.relation :=
        (den_untouched hpδ).mp hp
      have hp0 : p ∈ T.den I tgt.relation := (den_untouched hpσ).mp hp1
      obtain ⟨m, a, b, hm, ha, hb, ht⟩ := hrows p hp0 hψ rfl
      exact wIntervalTest_admitsMeasure hm ha hb ht
    · exact capacity_verdict_sigma_silent hacc hp hψ hσt
  · exact capacity_verdict_delta_silent hwin hp hψ hδt

/-- The loser algebra's fast-path hypothesis, whole: no shared key of
the three boolean classes — F, K, C, commute cells included — and
every shared W parent det licensed by the interval test
(`wTested`). `KeyDisjoint` embeds (`KeyDisjoint.tested`): with no W
key shared the license is vacuous. -/
structure TestedDisjoint (T : Theory) (I : Instance) (s d : Delta) :
    Prop where
  /-- No fact is written by both batches. -/
  fact : ∀ R f, s.touches R f → ¬ d.touches R f
  /-- No key statement's det is touched by both batches. -/
  key : ∀ R X, Statement.functionality R X ∈ T.statements →
    ∀ t, t ∈ s.projected R (keyDet T R X) →
      t ∉ d.projected R (keyDet T R X)
  /-- No containment's target det carries entries from both batches. -/
  cont : ∀ src tgt, Statement.containment src tgt ∈ T.statements →
    ∀ t, cTouch s src.relation src.selection (srcDet T src tgt)
        tgt.relation tgt.selection (tgtDet T src tgt) t →
      ¬ cTouch d src.relation src.selection (srcDet T src tgt)
        tgt.relation tgt.selection (tgtDet T src tgt) t
  /-- Every capacity statement's shared parent dets pass the interval
  test with untouched parent rows. -/
  cap : ∀ tgt wt w src, Statement.capacity tgt wt w src ∈
    T.statements → wTested T I s d tgt wt w src

/-- Full key disjointness is the tested hypothesis with the W license
vacuous — the strict theorems are the tested theorems' special
case. -/
theorem KeyDisjoint.tested {T : Theory} {s d : Delta}
    (hdisj : KeyDisjoint T s d) (I : Instance) :
    TestedDisjoint T I s d where
  fact := fun R f hs' hd' => hdisj (.F R f) hs' hd'
  key := fun R X hst t hs' hd' =>
    hdisj (.K R X t) ⟨hst, hs'⟩ ⟨hst, hd'⟩
  cont := fun src tgt hst t hs' hd' =>
    hdisj (.C src tgt t) ⟨hst, hs'⟩ ⟨hst, hd'⟩
  cap := fun tgt wt w src hst t hs' hd' =>
    absurd ⟨hst, hd'⟩ (hdisj (.W tgt wt w src t) ⟨hst, hs'⟩)

/-- One statement's stability under the tested hypothesis —
`Statement.judgment`'s dispatch, arm for arm, each form fed its own
slice: the boolean classes by disjointness, the measured class by
disjointness or the interval test. -/
theorem statement_stable_tested {T : Theory} {I : Instance}
    {s d : Delta} (hdisj : TestedDisjoint T I s d) {st : Statement}
    (hst : st ∈ T.statements) (hacc : st.judgment T (d.applyTo I))
    (hwin : st.judgment T (s.applyTo I)) :
    st.judgment T (d.applyTo (s.applyTo I)) := by
  cases st with
  | functionality R X =>
    have hk := hdisj.key R X hst
    cases hsplit : T.header.intervalSplit R X with
    | none =>
      simp only [keyDet, hsplit] at hk
      simp only [Statement.judgment, hsplit] at hacc hwin ⊢
      exact ⟨hacc.1, functionality_stable hk hacc.2 hwin.2⟩
    | some p =>
      obtain ⟨S, i⟩ := p
      simp only [keyDet, hsplit] at hk
      simp only [Statement.judgment, hsplit] at hacc hwin ⊢
      exact ⟨hacc.1, pointwise_stable hk hacc.2 hwin.2⟩
  | containment src tgt =>
    have hc := hdisj.cont src tgt hst
    cases hs' : T.header.intervalSplit src.relation
        src.projection with
    | some p =>
      obtain ⟨S, i⟩ := p
      cases ht' : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        obtain ⟨U, j⟩ := q
        simp only [srcDet, tgtDet, hs', ht'] at hc
        simp only [Statement.judgment, hs', ht'] at hacc hwin ⊢
        exact ⟨hacc.1, coverage_stable hc hacc.2 hwin.2⟩
      | none =>
        simp only [srcDet, tgtDet, hs', ht'] at hc
        simp only [Statement.judgment, hs', ht'] at hacc hwin ⊢
        exact ⟨hacc.1, containment_stable hc hacc.2 hwin.2⟩
    | none =>
      cases ht' : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        simp only [srcDet, tgtDet, hs', ht'] at hc
        simp only [Statement.judgment, hs', ht'] at hacc hwin ⊢
        exact ⟨hacc.1, containment_stable hc hacc.2 hwin.2⟩
      | none =>
        simp only [srcDet, tgtDet, hs', ht'] at hc
        simp only [Statement.judgment, hs', ht'] at hacc hwin ⊢
        exact ⟨hacc.1, containment_stable hc hacc.2 hwin.2⟩
  | capacity tgt wt w src =>
    have hw' := hdisj.cap tgt wt w src hst
    simp only [Statement.judgment] at hacc hwin ⊢
    exact capacity_stable_tested hw' hacc hwin

/-- One statement's stability under full key disjointness — the
tested dispatch at the vacuous W license. -/
theorem statement_stable {T : Theory} {I : Instance} {s d : Delta}
    (hdisj : KeyDisjoint T s d) {st : Statement}
    (hst : st ∈ T.statements) (hacc : st.judgment T (d.applyTo I))
    (hwin : st.judgment T (s.applyTo I)) :
    st.judgment T (d.applyTo (s.applyTo I)) :=
  statement_stable_tested (hdisj.tested I) hst hacc hwin

/-! ## L6 — footprint soundness -/

/-- The rows of one key obligation instance: the facts sharing the
det projection `t` — the star-guard's group, selection-free because a
key constrains the whole relation. -/
def obligationRows (A : Set Fact) (P : List FieldId)
    (t : List Value) : Set Fact :=
  fun f => f ∈ A ∧ f.project P = t

/-- A det the batch's K footprint does not name keeps its whole
obligation instance across the batch's application. -/
theorem obligationRows_untouched {T : Theory} {I : Instance}
    {s : Delta} {R : RelId} {P : List FieldId} {t : List Value}
    (hun : t ∉ s.projected R P) :
    obligationRows (T.den (s.applyTo I) R) P t =
      obligationRows (T.den I R) P t := by
  funext f
  refine propext ⟨fun h => ?_, fun h => ?_⟩
  · obtain ⟨hf, hp⟩ := h
    have hnt : ¬ s.touches R f :=
      untouched_of_det_silent (fun hx => hun (hp ▸ hx))
    exact ⟨(den_untouched hnt).mp hf, hp⟩
  · obtain ⟨hf, hp⟩ := h
    have hnt : ¬ s.touches R f :=
      untouched_of_det_silent (fun hx => hun (hp ▸ hx))
    exact ⟨(den_untouched hnt).mpr hf, hp⟩

/-- A det at which the batch touches no selected row keeps its whole
selected group — the witness-roster and child-group stability move. -/
theorem childGroup_untouched {T : Theory} {I : Instance} {s : Delta}
    {R : RelId} {ψ : Selection} {P : List FieldId} {t : List Value}
    (hun : ∀ g, s.touches R g → ψ.satisfies g → g.project P = t →
      False) :
    ChildGroup (T.den (s.applyTo I) R) ψ P t =
      ChildGroup (T.den I R) ψ P t := by
  funext g
  refine propext ⟨fun h => ?_, fun h => ?_⟩
  · obtain ⟨hg, hψ, hp⟩ := mem_childGroup.mp h
    have hnt : ¬ s.touches R g := fun ht => hun g ht hψ hp
    exact mem_childGroup.mpr ⟨(den_untouched hnt).mp hg, hψ, hp⟩
  · obtain ⟨hg, hψ, hp⟩ := mem_childGroup.mp h
    have hnt : ¬ s.touches R g := fun ht => hun g ht hψ hp
    exact mem_childGroup.mpr ⟨(den_untouched hnt).mpr hg, hψ, hp⟩

/-- **L6 — footprint soundness.** Under full key disjointness the
winner's application (a) moves no key obligation instance the
loser's K footprint names, (b) moves no ψ-witness roster any of the
loser's C dets name, (c) moves neither child group nor parent roster
at any of the loser's W dets, (d) writes no fact the loser writes,
and (e) evaporates none of the loser's ops — every touched fact's
presence at the moved base is its presence at the old one. The
hypothesis is deliberately stronger than "no CONFLICT cell": a shared
commute-cell F key (two inserts of one row) makes (e) false — the
second insert's op evaporates against the first's effect — and the
shared-W-parent rider holds vacuously because no W key is shared at
all (a TESTED shared parent moves a measure the loser reads, so it
lives in `TestedDisjoint`/`L7_tested`, never here). The one roster this
theorem deliberately does NOT stabilize is a `support−` obligation's
DEMAND side: a winner source-delete is footprint-silent and may
shrink it — which relaxes, never breaks, an accepted verdict (the
acceptance form, module doc). -/
theorem L6 {T : Theory} (I : Instance) {s d : Delta}
    (hdisj : KeyDisjoint T s d) :
    (∀ R X, Statement.functionality R X ∈ T.statements →
      ∀ t, t ∈ d.projected R (keyDet T R X) →
        obligationRows (T.den (s.applyTo I) R) (keyDet T R X) t =
          obligationRows (T.den I R) (keyDet T R X) t) ∧
    (∀ src tgt, Statement.containment src tgt ∈ T.statements →
      ∀ t, cTouch d src.relation src.selection (srcDet T src tgt)
          tgt.relation tgt.selection (tgtDet T src tgt) t →
        ChildGroup (T.den (s.applyTo I) tgt.relation) tgt.selection
            (tgtDet T src tgt) t =
          ChildGroup (T.den I tgt.relation) tgt.selection
            (tgtDet T src tgt) t) ∧
    (∀ tgt wt w src, Statement.capacity tgt wt w src ∈ T.statements →
      ∀ t, wTouch d src tgt t →
        ChildGroup (T.den (s.applyTo I) src.relation) src.selection
            src.projection t =
          ChildGroup (T.den I src.relation) src.selection
            src.projection t ∧
        ChildGroup (T.den (s.applyTo I) tgt.relation) tgt.selection
            tgt.projection t =
          ChildGroup (T.den I tgt.relation) tgt.selection
            tgt.projection t) ∧
    (∀ R f, d.touches R f → ¬ s.touches R f) ∧
    (∀ R f, d.touches R f → (f ∈ s.applyTo I R ↔ f ∈ I R)) := by
  have hwrites : ∀ R f, d.touches R f → ¬ s.touches R f :=
    fun R f htd hts => hdisj (.F R f) hts htd
  refine ⟨?_, ?_, ?_, hwrites, ?_⟩
  · intro R X hst t ht
    exact obligationRows_untouched
      (fun hmem => hdisj (.K R X t) ⟨hst, hmem⟩ ⟨hst, ht⟩)
  · intro src tgt hst t ht
    exact childGroup_untouched (fun g hto hψ hp =>
      hdisj (.C src tgt t) ⟨hst, Or.inr ⟨g, hto, hψ, hp⟩⟩ ⟨hst, ht⟩)
  · intro tgt wt w src hst t ht
    constructor
    · exact childGroup_untouched (fun q hto hφ hp =>
        hdisj (.W tgt wt w src t) ⟨hst, Or.inl ⟨q, hto, hφ, hp⟩⟩
          ⟨hst, ht⟩)
    · exact childGroup_untouched (fun g hto hψ hp =>
        hdisj (.W tgt wt w src t) ⟨hst, Or.inr ⟨g, hto, hψ, hp⟩⟩
          ⟨hst, ht⟩)
  · intro R f htd
    exact applyTo_untouched (hwrites R f htd) I

/-! ## L7 — footprint stability (the load-bearing theorem) -/

/-- L7 under the tested hypothesis — the loser algebra's REAL
fast-path license: boolean-class disjointness plus per-parent
interval tests carry an accepted verdict and its net effect to the
winner-moved base whole. `L7` below is this theorem at the vacuous W
license. -/
theorem L7_tested {T : Theory} {I : Instance} {s d : Delta}
    (hdisj : TestedDisjoint T I s d) (hwin : holds T (s.applyTo I))
    (hacc : holds T (d.applyTo I)) :
    holds T (d.applyTo (s.applyTo I)) ∧
      d.effAdds (s.applyTo I) = d.effAdds I ∧
      d.effRemoves (s.applyTo I) = d.effRemoves I := by
  refine ⟨fun st hst =>
    statement_stable_tested hdisj hst (hacc st hst) (hwin st hst),
    ?_, ?_⟩
  · funext R f
    refine propext ⟨fun h => ?_, fun h => ?_⟩
    · obtain ⟨hfa, hnp⟩ := h
      have hnt : ¬ s.touches R f := fun hts =>
        hdisj.fact R f hts (Or.inl hfa)
      exact ⟨hfa, fun hp => hnp ((applyTo_untouched hnt I).mpr hp)⟩
    · obtain ⟨hfa, hnp⟩ := h
      have hnt : ¬ s.touches R f := fun hts =>
        hdisj.fact R f hts (Or.inl hfa)
      exact ⟨hfa, fun hp => hnp ((applyTo_untouched hnt I).mp hp)⟩
  · funext R f
    refine propext ⟨fun h => ?_, fun h => ?_⟩
    · obtain ⟨hfr, hna, hp⟩ := h
      have hnt : ¬ s.touches R f := fun hts =>
        hdisj.fact R f hts (Or.inr hfr)
      exact ⟨hfr, hna, (applyTo_untouched hnt I).mp hp⟩
    · obtain ⟨hfr, hna, hp⟩ := h
      have hnt : ¬ s.touches R f := fun hts =>
        hdisj.fact R f hts (Or.inr hfr)
      exact ⟨hfr, hna, (applyTo_untouched hnt I).mpr hp⟩

/-- **L7 — footprint stability.** Under full key disjointness, a
batch ACCEPTED at its base is accepted at the winner-moved base —
`holds` transfers whole through `statement_stable` — and its net
effect there is its net effect at the old base, effective inserts and
deletes both: the strengthening of the delta-restriction theorems
that licenses republish-without-re-judgment and keeps the publish law
true at the moved base. The hypotheses are exactly the protocol's
free facts: the winner's batch was accepted (it published) and the
loser's was accepted (it judged locally); the base state itself is
never consulted. The rejected-verdict converse is REFUSED — the
module doc carries the two-delete countermodel — and never spent: a
rejected batch returns to its host and no republish path exists for
it. `L7_tested` is the same theorem under the relaxed W license. -/
theorem L7 {T : Theory} {I : Instance} {s d : Delta}
    (hdisj : KeyDisjoint T s d) (hwin : holds T (s.applyTo I))
    (hacc : holds T (d.applyTo I)) :
    holds T (d.applyTo (s.applyTo I)) ∧
      d.effAdds (s.applyTo I) = d.effAdds I ∧
      d.effRemoves (s.applyTo I) = d.effRemoves I :=
  L7_tested (hdisj.tested I) hwin hacc

/-- L7 at the lifecycle: a loser that committed `d` on the same base
the winner committed `s` on re-commits `d` on the winner's state and
is ACCEPTED, landing exactly the two-batch final — the
republish-without-re-judgment verdict, `commit`-shaped. -/
theorem republish_verdict_stable {T : Theory} {b w l : State T}
    {s d : Delta} (hdisj : KeyDisjoint T s d)
    (hwin : commit b s = .ok w) (hlose : commit b d = .ok l) :
    ∃ r : State T, commit w d = .ok r ∧
      r.inst = d.applyTo (s.applyTo b.inst) := by
  have h₁ : w.inst = s.applyTo b.inst := by
    rw [commit_ok_inst hwin, apply_eq_applyTo]
  have h₂ : l.inst = d.applyTo b.inst := by
    rw [commit_ok_inst hlose, apply_eq_applyTo]
  have hw : holds T (s.applyTo b.inst) := h₁ ▸ w.models
  have hl : holds T (d.applyTo b.inst) := h₂ ▸ l.models
  have hh := (L7 hdisj hw hl).1
  refine ⟨⟨d.applyTo (s.applyTo b.inst), hh⟩, ?_, rfl⟩
  unfold commit
  have happ : apply w d = d.applyTo (s.applyTo b.inst) := by
    rw [apply_eq_applyTo, h₁]
  rw [happ, judge_holds hh]

/-! ## L8 — commutativity -/

/-- Apply order is invisible when no fact is written by both
batches — the F class alone carries commutativity. -/
theorem applyTo_comm_of_writes_apart {s d : Delta}
    (hfd : ∀ R f, s.touches R f → ¬ d.touches R f) (I : Instance) :
    d.applyTo (s.applyTo I) = s.applyTo (d.applyTo I) := by
  funext R f
  refine propext ?_
  by_cases hσ : s.touches R f
  · have hδ : ¬ d.touches R f := hfd R f hσ
    exact (applyTo_untouched hδ (s.applyTo I)).trans
      (applyTo_inner_untouched hδ I).symm
  · exact (applyTo_inner_untouched hσ I).trans
      (applyTo_untouched hσ (d.applyTo I)).symm

/-- **L8 — commutativity.** Under full key disjointness either apply
order yields the identical final state — set-level instance equality,
by cases on which batch touches each fact (never both: the F class).
The engine's canonical plan order carries this equality down to
stored bytes; that half is representation, pinned by the replication
conformance lanes, not restated here. -/
theorem L8 {T : Theory} (I : Instance) {s d : Delta}
    (hdisj : KeyDisjoint T s d) :
    d.applyTo (s.applyTo I) = s.applyTo (d.applyTo I) :=
  applyTo_comm_of_writes_apart
    (fun R f hs' hd' => hdisj (.F R f) hs' hd') I

/-- L8 under the tested hypothesis: the interval test moves measures,
never rows, so the state equality is untouched by the W relaxation. -/
theorem L8_tested {T : Theory} {I : Instance} {s d : Delta}
    (hdisj : TestedDisjoint T I s d) :
    d.applyTo (s.applyTo I) = s.applyTo (d.applyTo I) :=
  applyTo_comm_of_writes_apart hdisj.fact I

/-- L8 at the lifecycle: the two commit orders on one committed base
share one final instance. -/
theorem apply_commutes {T : Theory} (b : State T) {s d : Delta}
    (hdisj : KeyDisjoint T s d) :
    d.applyTo (apply b s) = s.applyTo (apply b d) := by
  rw [apply_eq_applyTo, apply_eq_applyTo]
  exact L8 b.inst hdisj

/-! ## L9 — component independence -/

/-- The relations one statement consults — the braid derivation's
edge set: a statement is one hyperedge over its relations. -/
def stmtRels : Statement → List RelId
  | .functionality R _ => [R]
  | .containment src tgt => [src.relation, tgt.relation]
  | .capacity tgt _ _ src => [src.relation, tgt.relation]

/-- Statements never span braid components: every theory statement's
consulted relations share one component — the braid derivation's
defining guarantee, taken as the hypothesis it is. -/
def ComponentClosed (comp : RelId → Nat) (T : Theory) : Prop :=
  ∀ st, st ∈ T.statements → ∀ R R', R ∈ stmtRels st →
    R' ∈ stmtRels st → comp R = comp R'

/-- A batch writing inside one braid component. -/
def LocalTo (comp : RelId → Nat) (d : Delta) (c : Nat) : Prop :=
  ∀ R f, d.touches R f → comp R = c

/-- **L9 — component independence.** Two batches local to distinct
braid components have disjoint footprints by construction: every key
of every class is anchored to a touched relation, and a statement's
relations never leave its component — so cross-component commits are
concurrent with no intersection ever computed. The trivial corollary,
stated because the protocol prices it. -/
theorem L9 {T : Theory} {comp : RelId → Nat} {s d : Delta}
    {c c' : Nat} (hT : ComponentClosed comp T) (hs : LocalTo comp s c)
    (hd : LocalTo comp d c') (hne : c ≠ c') : KeyDisjoint T s d := by
  intro k hks hkd
  cases k with
  | F R f => exact hne ((hs R f hks).symm.trans (hd R f hkd))
  | K R X det =>
    obtain ⟨_, f, hf, _⟩ := hks
    obtain ⟨_, g, hg, _⟩ := hkd
    exact hne ((hs R f hf).symm.trans (hd R g hg))
  | C src tgt det =>
    obtain ⟨hst, hcs⟩ := hks
    obtain ⟨_, hcd⟩ := hkd
    have hrel : comp src.relation = comp tgt.relation :=
      hT _ hst src.relation tgt.relation List.mem_cons_self
        (List.mem_cons_of_mem _ List.mem_cons_self)
    have h1 : comp src.relation = c ∨ comp tgt.relation = c := by
      rcases hcs with ⟨f, hf, _, _⟩ | ⟨g, hg, _, _⟩
      · exact Or.inl (hs _ f (Or.inl hf))
      · exact Or.inr (hs _ g hg)
    have h2 : comp src.relation = c' ∨ comp tgt.relation = c' := by
      rcases hcd with ⟨f, hf, _, _⟩ | ⟨g, hg, _, _⟩
      · exact Or.inl (hd _ f (Or.inl hf))
      · exact Or.inr (hd _ g hg)
    rcases h1 with h1 | h1 <;> rcases h2 with h2 | h2
    · exact hne (h1.symm.trans h2)
    · exact hne (h1.symm.trans (hrel.trans h2))
    · exact hne (h1.symm.trans (hrel.symm.trans h2))
    · exact hne (h1.symm.trans h2)
  | W tgt wt w src det =>
    obtain ⟨hst, hcs⟩ := hks
    obtain ⟨_, hcd⟩ := hkd
    have hrel : comp src.relation = comp tgt.relation :=
      hT _ hst src.relation tgt.relation List.mem_cons_self
        (List.mem_cons_of_mem _ List.mem_cons_self)
    have h1 : comp src.relation = c ∨ comp tgt.relation = c := by
      rcases hcs with ⟨f, hf, _, _⟩ | ⟨g, hg, _, _⟩
      · exact Or.inl (hs _ f hf)
      · exact Or.inr (hs _ g hg)
    have h2 : comp src.relation = c' ∨ comp tgt.relation = c' := by
      rcases hcd with ⟨f, hf, _, _⟩ | ⟨g, hg, _, _⟩
      · exact Or.inl (hd _ f hf)
      · exact Or.inr (hd _ g hg)
    rcases h1 with h1 | h1 <;> rcases h2 with h2 | h2
    · exact hne (h1.symm.trans h2)
    · exact hne (h1.symm.trans (hrel.trans h2))
    · exact hne (h1.symm.trans (hrel.symm.trans h2))
    · exact hne (h1.symm.trans h2)

/-! ## L10 — replay idempotence -/

/-- A contained batch applies to nothing: `(base ∖ removes) ∪ adds`
is `base` when every add is present and every remove absent. -/
theorem applyTo_contained {d : Delta} {I : Instance}
    (h : d.containedIn I) : d.applyTo I = I := by
  funext R f
  refine propext ⟨fun hf => ?_, fun hf => ?_⟩
  · rcases hf with ⟨hf, _⟩ | hf
    · exact hf
    · exact h.1 R f hf
  · exact Or.inl ⟨hf, fun hr => h.2 R f hr hf⟩

/-- One op whose facts the state already disposes: an insert of
present facts, a delete of absent facts. -/
def opDisposed (I : Instance) : Op → Prop
  | .insert R fs => ∀ f, f ∈ fs → f ∈ I R
  | .delete R fs => ∀ f, f ∈ fs → f ∉ I R

/-- Inserting present facts is the identity — each singleton step
lands a fact the instance already holds. -/
theorem insertFacts_disposed {R : RelId} {I : Instance} :
    ∀ {fs : List Fact}, (∀ f, f ∈ fs → f ∈ I R) →
      insertFacts R fs I = I
  | [], _ => rfl
  | f :: fs, h => by
    show insertFacts R fs (fun R' g => g ∈ I R' ∨ (R' = R ∧ g = f)) = I
    have heq : (fun R' g => g ∈ I R' ∨ (R' = R ∧ g = f)) = I := by
      funext R' g
      refine propext ⟨fun hg => ?_, Or.inl⟩
      rcases hg with hg | ⟨rfl, rfl⟩
      · exact hg
      · exact h g List.mem_cons_self
    rw [heq]
    exact insertFacts_disposed fun g hg =>
      h g (List.mem_cons_of_mem f hg)

/-- Deleting absent facts is the identity. -/
theorem deleteFacts_disposed {R : RelId} {I : Instance} :
    ∀ {fs : List Fact}, (∀ f, f ∈ fs → f ∉ I R) →
      deleteFacts R fs I = I
  | [], _ => rfl
  | f :: fs, h => by
    show deleteFacts R fs
      (fun R' g => g ∈ I R' ∧ ¬(R' = R ∧ g = f)) = I
    have heq : (fun R' g => g ∈ I R' ∧ ¬(R' = R ∧ g = f)) = I := by
      funext R' g
      refine propext ⟨fun hg => hg.1, fun hg => ⟨hg, ?_⟩⟩
      rintro ⟨rfl, rfl⟩
      exact h g List.mem_cons_self hg
    rw [heq]
    exact deleteFacts_disposed fun g hg =>
      h g (List.mem_cons_of_mem f hg)

/-- **Every op net-disposes**: a whole op sequence the state already
disposes applies to the identical state — each op is the identity, so
the fold is. -/
theorem applyOps_disposed {I : Instance} :
    ∀ {ops : List Op}, (∀ op, op ∈ ops → opDisposed I op) →
      applyOps I ops = I
  | [], _ => rfl
  | op :: rest, h => by
    have hop : op.apply I = I := by
      cases op with
      | insert R fs => exact insertFacts_disposed (h _ List.mem_cons_self)
      | delete R fs => exact deleteFacts_disposed (h _ List.mem_cons_self)
    show applyOps (op.apply I) rest = I
    rw [hop]
    exact applyOps_disposed fun op' hop' =>
      h op' (List.mem_cons_of_mem op hop')

/-- The op-sequence face of L10: a disposed sequence commits to the
SAME state, accepted — the verdict is the pre-state's own commitment,
never a fresh judgment. -/
theorem commitOps_disposed {T : Theory} (b : State T) {ops : List Op}
    (h : ∀ op, op ∈ ops → opDisposed b.inst op) :
    commitOps b ops = .ok b := by
  unfold commitOps
  rw [applyOps_disposed h, judge_holds b.models]

/-- **L10 — replay idempotence.** Re-applying a batch whose effects
the state already contains yields the identical state (`apply` is the
identity), an empty effective delta (nothing effective to insert or
delete — the no-op arm's own reading, so the engine never reaches
judgment: the verdict below is discharged by the state's OWN
commitment, `State.models`, with the delta never consulted), and an
accepted verdict whose state is the pre-state ITSELF — so the
state-changing predicate is false and the generation does not
advance (`crate::GenerationId` moves only on state change; a re-landed
slot is a proven no-op). The theorem the recovery design stands on:
crash windows heal by replaying forward, because replaying
backward-overlap is harmless — `replay_heals` is exactly that
composition, and `commitOps_disposed`/`applyOps_disposed` carry the
per-op reading (every op net-disposes). -/
theorem L10 {T : Theory} (b : State T) {d : Delta}
    (h : d.containedIn b.inst) :
    apply b d = b.inst ∧
      (∀ R f, f ∉ d.effAdds b.inst R) ∧
      (∀ R f, f ∉ d.effRemoves b.inst R) ∧
      commit b d = .ok b := by
  have happ : apply b d = b.inst := by
    rw [apply_eq_applyTo]
    exact applyTo_contained h
  refine ⟨happ, ?_, ?_, ?_⟩
  · rintro R f ⟨hfa, hnp⟩
    exact hnp (h.1 R f hfa)
  · rintro R f ⟨hfr, _, hp⟩
    exact h.2 R f hfr hp
  · unfold commit
    rw [happ, judge_holds b.models]

/-- The recovery composition: a coalesced batch that COMMITTED is
contained in its own post-state, so replaying it there is L10's
no-op — identical state, accepted verdict, no generation advance.
The crash window between a local commit and its publish heals by
replaying forward. -/
theorem replay_heals {T : Theory} {b b' : State T} {d : Delta}
    (hco : ∀ R f, f ∈ d.adds R → f ∉ d.removes R)
    (h : commit b d = .ok b') :
    apply b' d = b'.inst ∧ commit b' d = .ok b' := by
  have hcon : d.containedIn b'.inst := by
    rw [commit_ok_inst h, apply_eq_applyTo]
    constructor
    · intro R f hf
      exact Or.inr hf
    · intro R f hf hmem
      rcases hmem with ⟨_, hnr⟩ | hadd
      · exact hnr hf
      · exact hco R f hadd hf
  obtain ⟨h1, _, _, h4⟩ := L10 b' hcon
  exact ⟨h1, h4⟩

end Footprint
end Txn
end Bumbledb
