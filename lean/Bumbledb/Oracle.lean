import Bumbledb.Txn.DeltaRestriction
import Bumbledb.Exec.Sweep

/-!
# Oracle — the order-oracle plan calculus (the admission calculus, wave 2)

"The determinant index can answer it" as mathematics. The acceptance
gate's law — a statement is accepted only if the checker has an
enforcement plan whose consultations are bounded by the touched data
(`docs/architecture/30-dependencies.md` § the acceptance gate) — was
prose. This module is that law as theorems: an abstract ordered
oracle, enforcement plans as witness terms over it, and per accepted
form the two halves — the plan's verdict EQUALS the form's
delta-restricted check (`Txn/DeltaRestriction.lean`, wave 2a), and
the consultation count is a `Nat`, proved. Law 3's abstract-cost
scoping (owner ruling 2026-07-14) is the license: consultation counts
over an abstract ordered oracle are admissible mathematics; anything
with units stays in the docs.

## Placement (recorded)

Level 1 — beside `Exec/*`: the calculus models the judge's
algorithmic essence (which questions it asks an ordered set, and how
many), never its mechanism. `Bumbledb.lean` imports it after the
delta-restriction module whose checks it decides.

## The abstract ordered oracle

`OrderedOracle K P β ple` is an ordered-set oracle, full stop: a fact
set, a group key (`groupOf : β → K`) and a within-group position
(`posOf : β → P`) under an abstract order `ple` — the two-level key
of a determinant consultation, with the order itself abstract (which
bytes realize it is representation, unmodeled). Operations with
specification lemmas: `consult` (the facts of one group, sound,
complete, duplicate-free, in position order), `pred`/`succ`
(within-group neighbors at a position: members, extremal), and
nothing else. The scalar forms consume only the exact-key face
(`consult`); the pointwise forms consume the ordered face
(`pred`/`succ`, the position-ordered walk). Theorems quantify over
every CONFORMING oracle — the determinant index is the oracle, and
conformance of the concrete index is engine mechanism, priced here
and discharged there.

## Enforcement plans as witness terms

`EnforcementPlan` is the closed inductive of sanctioned probe shapes:
`pointProbe` (one lookup), `neighborProbe` (predecessor + successor),
`prefixWalk` (one entry seek + the ordered walk of one group/window),
`memberTest` (a sealed finite-set test — zero oracle consultations:
the closed-target containment's compiled member set).
`EnforcementPlan.answers` is a plan's evaluation against an oracle;
`EnforcementPlan.consultations` is its price; `plan_answers_sound`
says every answer is a fact of the ONE consulted oracle — the
calculus never merges answer sets. That theorem alone is NOT the
join fence (an `OrderedOracle`'s fact set is unconstrained here):
the E1 refusal's mathematical face is the per-form conformance pins
(`o.facts = T.den … R`, one stored relation per oracle) composed
into the gate type's uninhabitability
(`Admission.AdmissibleForm`,
`Countermodels.joined_window_form_uninhabitable`, over the blast
countermodel `Countermodels.joined_window_blast`). This generalizes
the `DisjointDeterminantProof` discipline: acceptance of a statement
form = the existence of a plan term whose evaluation decides the
form's delta-restricted check (`planObligation`, `acceptance_gate`).

## The per-form theorems (correctness + the proved count)

* **Scalar FD** — one point probe per touched determinant tuple
  (`fdPlan`, `fd_plan_consultations` = 1): the collision-free verdict
  over the probed buckets IS `fdDeltaCheck` (`fd_plan_decides`).
* **Pointwise FD** — two neighbor probes per touched fact
  (`neighborPlan`, `neighbor_plan_consultations` = 2): under the
  group-disjointness premise — the applier's premise chain, exactly
  what the pre-state key supplies per group — clearing both neighbors
  IS clearing the whole group (`neighbor_probe_decides`, the sandwich
  argument at the sweep's altitude).
* **Scalar IND, source** — one target-key point probe per added
  source fact (`indSourcePlan`, `ind_source_plan_consultations` = 1);
  the ψ read is of the probed bucket itself — the oracle answers
  whole facts, so the engine's establishing-fact `F` get is
  representation below this altitude.
* **Scalar IND, target** — per removed ψ-target fact, one
  re-establishment probe on the KEYED target index
  (`indReestablishProbe`, `ind_reestablish_consultations` = 1) plus
  one reverse-demand WALK of the source bucket (`indReverseWalk`,
  `ind_reverse_walk_consultations` = 1 + the walked bucket's length):
  the source projection is many-to-one by design — no acceptance
  rule keys it — so a unit price there would understate (exactly the
  keyed-bucket honesty criterion below), and the walk is priced at
  what it reads. Both arms together decide `containmentDeltaCheck`
  (`containment_plan_decides`, evaluating exactly these named
  terms); a CLOSED target sharpens the whole source arm to a sealed
  member test at zero consultations (`member_test_decides`,
  `member_test_consultations`).
* **Coverage** — one entry seek + a prefix walk of the group
  (`coveragePlan`, `coverage_plan_consultations` = 1 + walk length):
  the walked segments feed `Exec/Sweep.lean`'s one-pass verdict, and
  under the target-key disjointness premise the sweep verdict IS the
  point-subset denotation (`coverage_walk_decides`, spending
  `sweep_covered_sound_complete`).
* **Window** — a prefix walk of each touched parent's child group
  (`windowPlan`): the length-window verdict over one duplicate-free
  enumeration IS the count-window judgment
  (`window_admits_iff_enum` via the pigeonhole lemma
  `nodup_subset_length_le`), deciding `cardinalityDeltaCheck`
  (`cardinality_plan_decides`); total consultations = touched parents
  + total touched-group sizes, as an equation
  (`window_plan_consultations`).

## The acceptance premises, spent

Correctness above never needs the target-key premise — the key
licenses the PRICE: `point_probe_honest` (a keyed group's bucket is a
subsingleton, so one descent answers it) and
`accepted_target_key_prices_the_probe` (`TargetKeyAccepted` + `holds`
spent through `accepted_target_key_spent` — the gate's "an accepted
statement is a measured promise" made literal: acceptance is what
makes the unit cost honest). The premise enters as a hypothesis,
never a conjunct — the acceptance ≠ denotation discipline unchanged.
The `holds`-at-final-state hypothesis is the two-phase judge's phase
order (`Txn.lean`): keys convict in the key phase before any
statement probe runs.

## The acceptance-gate theorem

`planObligation` dispatches every `Statement` constructor to its
plan-decision obligation, arm for arm with `Txn.deltaCheck`;
`acceptance_gate` proves every statement HAS its plan. The pointwise
arms' obligations are the probe-decision laws at the sweep's altitude
(the narrowing below); every scalar arm's obligation is full
fact-level fidelity to its delta-restricted check.

## Narrowings recorded (law 5: narrow and record)

* **The pointwise arms live at the interval altitude.** The
  fact-level reading rides `Value.points` through positional
  structural typing — delegated exactly as `Exec/Sweep.lean`'s module
  doc delegates σ and typing above the fold. Their delta composition
  is `pointwise_delta_restriction` / `coverage_delta_restriction`
  (already proved, wave 2a); this module supplies the per-probe and
  per-walk verdicts those checks run.
* **The neighbor-probe theorem is per-insert over the standing
  group.** The applier probes each inserted fact against the index as
  it stands, earlier inserts included — the multi-insert composition
  is that Level-2 sequencing, unmodeled here; the theorem is the one
  insert's decision, which is the probe's whole algorithmic content.
* **The coverage plan walks the whole group.** The engine clips the
  walk to the touched window via the entry seek;
  `Exec/Sweep.lean: sweep_ignores_spent_segments` is the clipping
  license, and a wider walk only re-reads more — the same
  superset-narrowing as `touchedParents` (wave 2a).
* **Touched enumerations are sets.** The engine's deduplicated
  scan-order lists are representation — the `violationSet` narrowing
  (`Txn.lean`), unchanged. The window count theorem takes the touched
  parents as any list because the price is per-parent-walk, whatever
  order the engine visits them.
* **The reverse-demand walk reads the whole source bucket.** The
  engine's reverse index is φ-gated per statement and its scan stops
  at the first surviving demand — clipping, representation below
  this altitude: a shorter engine read only reads less, and the
  model prices the whole walked bucket — the same superset-narrowing
  as `touchedParents` (wave 2a).
* **Unit probe cost is the keyed-bucket honesty.** `pointProbe` costs
  1 because acceptance demands probe-ability — the exact-field-set
  target-key rule; `accepted_target_key_prices_the_probe` is that
  demand spent. On an unkeyed group the abstract count would
  understate, which is exactly why the gate refuses unkeyed targets.

## The window plan: acceptance and checker discharged

The engine ACCEPTS the window form at declaration (2026-07-14:
`StatementDescriptor::Cardinality`; the gate arm
`validate_cardinality` in `schema/validate.rs` resolves exactly the
key this plan prices — the window's target key — so "the plan is
sealable" is what acceptance checks, and the `Bridge.lean`
acceptance row cites the plan theorem). The Rust CHECKER runs the
plan as stated: `windowPlan`'s per-touched-parent child-group walk
is `storage/commit/judgment.rs::Checker::check_window` — the
enforcement discharge row cites the delta-restriction theorem this
plan decides. The FD, containment, and coverage plans price
mechanisms the ledger already carries (`Applier`, `judgment.rs`, the
sweep); those arms add no rows.
-/

namespace Bumbledb
namespace Oracle

/-! ## The abstract ordered oracle -/

/-- The ordered-set oracle: a fact set indexed by a group key and a
within-group position under the abstract order `ple`. The operations
are consultation (`consult` — one group, complete, duplicate-free, in
position order) and the within-group neighbors (`pred`/`succ` —
members, extremal at the probed position). Nothing else exists at
this altitude: which bytes realize the order, and how a concrete
index descends, is representation. -/
structure OrderedOracle (K P β : Type) (ple : P → P → Prop) where
  /-- The indexed fact set. -/
  facts : Set β
  /-- The group key of one fact. -/
  groupOf : β → K
  /-- The within-group position of one fact. -/
  posOf : β → P
  /-- The ordered walk of one group. -/
  consult : K → List β
  /-- Consultation is sound and complete for the group. -/
  consult_mem : ∀ g f, f ∈ consult g ↔ f ∈ facts ∧ groupOf f = g
  /-- Consultation enumerates each fact once. -/
  consult_nodup : ∀ g, (consult g).Nodup
  /-- Consultation walks in position order. -/
  consult_ordered : ∀ g,
    (consult g).Pairwise (fun a b => ple (posOf a) (posOf b))
  /-- The within-group predecessor at a position. -/
  pred : K → P → Option β
  /-- The within-group successor at a position. -/
  succ : K → P → Option β
  /-- A predecessor answer is a group member at or below the probe. -/
  pred_mem : ∀ g p f, pred g p = some f →
    f ∈ facts ∧ groupOf f = g ∧ ple (posOf f) p
  /-- A predecessor answer is greatest among members at or below. -/
  pred_greatest : ∀ g p f, pred g p = some f →
    ∀ h, h ∈ facts → groupOf h = g → ple (posOf h) p →
      ple (posOf h) (posOf f)
  /-- An empty predecessor answer means no member sits at or below. -/
  pred_none : ∀ g p, pred g p = none →
    ∀ h, h ∈ facts → groupOf h = g → ¬ ple (posOf h) p
  /-- A successor answer is a group member at or above the probe. -/
  succ_mem : ∀ g p f, succ g p = some f →
    f ∈ facts ∧ groupOf f = g ∧ ple p (posOf f)
  /-- A successor answer is least among members at or above. -/
  succ_least : ∀ g p f, succ g p = some f →
    ∀ h, h ∈ facts → groupOf h = g → ple p (posOf h) →
      ple (posOf f) (posOf h)
  /-- An empty successor answer means no member sits at or above. -/
  succ_none : ∀ g p, succ g p = none →
    ∀ h, h ∈ facts → groupOf h = g → ¬ ple p (posOf h)

/-! ## Enforcement plans — the sanctioned probe shapes -/

/-- A plan term: the closed inductive of probe shapes the acceptance
gate sanctions. There is no join shape — deliberately, by
representation; the E1 refusal that representation feeds is the gate
type's uninhabitability
(`Countermodels.joined_window_form_uninhabitable`, over the blast
countermodel `Countermodels.joined_window_blast`). -/
inductive EnforcementPlan (K P : Type) where
  /-- One lookup at a group key. -/
  | pointProbe (group : K)
  /-- Predecessor + successor at a within-group position. -/
  | neighborProbe (group : K) (pos : P)
  /-- One entry seek + the ordered walk of one group/window. -/
  | prefixWalk (group : K)
  /-- A sealed finite-set test: zero oracle consultations. -/
  | memberTest

variable {K P P' β : Type} {ple : P → P → Prop} {ple' : P' → P' → Prop}

/-- A plan's evaluation: the answers the oracle hands back. -/
def EnforcementPlan.answers (o : OrderedOracle K P β ple) :
    EnforcementPlan K P → List β
  | .pointProbe g => o.consult g
  | .neighborProbe g p => (o.pred g p).toList ++ (o.succ g p).toList
  | .prefixWalk g => o.consult g
  | .memberTest => []

/-- A plan's price: oracle operations as a `Nat`. A point probe is
one descent; a neighbor probe is two; a prefix walk is the entry seek
plus one read per walked fact; a member test consults nothing. -/
def EnforcementPlan.consultations (o : OrderedOracle K P β ple) :
    EnforcementPlan K P → Nat
  | .pointProbe _ => 1
  | .neighborProbe _ _ => 2
  | .prefixWalk g => 1 + (o.consult g).length
  | .memberTest => 0

/-- The price of a plan list — the per-form aggregate counts. -/
def costs (o : OrderedOracle K P β ple)
    (ps : List (EnforcementPlan K P)) : Nat :=
  (ps.map (EnforcementPlan.consultations o)).foldr (· + ·) 0

/-- Membership in a neighbor probe's answers, unfolded to the two
option-valued operations. -/
theorem mem_neighborAnswers {o : OrderedOracle K P β ple} {g : K}
    {p : P} {b : β} :
    b ∈ (EnforcementPlan.neighborProbe g p).answers o ↔
      o.pred g p = some b ∨ o.succ g p = some b := by
  show b ∈ (o.pred g p).toList ++ (o.succ g p).toList ↔ _
  rw [List.mem_append]
  constructor
  · rintro (h | h)
    · left
      cases hp : o.pred g p with
      | none => rw [hp] at h; cases h
      | some a =>
        rw [hp] at h
        rcases List.mem_singleton.mp h with rfl
        rfl
    · right
      cases hs : o.succ g p with
      | none => rw [hs] at h; cases h
      | some a =>
        rw [hs] at h
        rcases List.mem_singleton.mp h with rfl
        rfl
  · rintro (h | h)
    · exact Or.inl (by rw [h]; exact List.mem_singleton.mpr rfl)
    · exact Or.inr (by rw [h]; exact List.mem_singleton.mpr rfl)

/-- **One oracle per evaluation.** Every answer of every plan is a
fact of the ONE consulted oracle — an evaluation never merges answer
sets. This is exactly what it says and no more: nothing constrains
an oracle's fact set, so this theorem alone fences out no join. The
E1 negative face lives where the surfaces are pinned — the per-form
conformance hypotheses (`o.facts = T.den … R`, one stored relation
per oracle) and the gate type's uninhabitability
(`Admission.AdmissibleForm`,
`Countermodels.joined_window_form_uninhabitable`). -/
theorem plan_answers_sound (o : OrderedOracle K P β ple) :
    ∀ (p : EnforcementPlan K P) (b : β), b ∈ p.answers o → b ∈ o.facts
  | .pointProbe k, b, hb => ((o.consult_mem k b).mp hb).1
  | .prefixWalk k, b, hb => ((o.consult_mem k b).mp hb).1
  | .neighborProbe k p, b, hb => by
    rcases mem_neighborAnswers.mp hb with h | h
    · exact (o.pred_mem k p b h).1
    · exact (o.succ_mem k p b h).1
  | .memberTest, _, hb => nomatch hb

/-- **The keyed bucket is a subsingleton** — why a point probe's unit
cost is honest: on a keyed index (the probe-ability acceptance rule)
one descent answers the whole bucket. -/
theorem point_probe_honest (o : OrderedOracle K P β ple)
    (hkey : ∀ a b, a ∈ o.facts → b ∈ o.facts →
      o.groupOf a = o.groupOf b → a = b) (k : K) :
    ∀ a b, a ∈ o.consult k → b ∈ o.consult k → a = b := by
  intro a b ha hb
  obtain ⟨haf, hak⟩ := (o.consult_mem k a).mp ha
  obtain ⟨hbf, hbk⟩ := (o.consult_mem k b).mp hb
  exact hkey a b haf hbf (hak.trans hbk.symm)

/-- **Acceptance prices the probe.** `TargetKeyAccepted` + `holds`
(the theory-side acceptance premise, spent through
`accepted_target_key_spent`) make the target oracle's every bucket a
subsingleton — the gate's "an accepted statement is a measured
promise" made literal: acceptance is exactly what licenses the unit
consultation count. The `holds`-at-judged-state hypothesis is the
two-phase judge's phase order (`Txn.lean`: keys convict before any
statement probe runs). Serves the containment AND window forms — one
target-key rule, one price theorem. -/
theorem accepted_target_key_prices_the_probe {T : Theory}
    {I : Instance} (hI : holds T I) {tgt : Atom}
    (hacc : TargetKeyAccepted T tgt)
    (hscalar : ∀ i, i ∈ tgt.projection →
      T.header.isInterval tgt.relation i = false)
    (o : OrderedOracle (List Value) P Fact ple)
    (hfacts : o.facts = T.den I tgt.relation)
    (hkey : ∀ g, o.groupOf g = g.project tgt.projection) :
    ∀ t a b, a ∈ o.consult t → b ∈ o.consult t → a = b := by
  have hfun := accepted_target_key_spent hI hacc hscalar
  intro t
  refine point_probe_honest o ?_ t
  intro a b ha hb hg
  rw [hfacts] at ha hb
  rw [hkey a, hkey b] at hg
  exact hfun a b ha hb hg

/-! ## Counting lemmas — windows over one enumeration

The count-window judgment (`Cardinality.lean`) is stated as the two
list-witnessed bounds; over one duplicate-free enumeration of the set
both bounds collapse to the enumeration's LENGTH — the pigeonhole
lemma below is the whole argument, and no finiteness token is spent
anywhere else. -/

/-- The pigeonhole lemma: a duplicate-free list of members of another
list is no longer than it. Elementary — no decidable equality, no
erasure: split the host at the head's occurrence. -/
theorem nodup_subset_length_le :
    ∀ (l l' : List β), l.Nodup → (∀ a, a ∈ l → a ∈ l') →
      l.length ≤ l'.length
  | [], _, _, _ => Nat.zero_le _
  | a :: rest, l', hnd, hsub => by
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
    have hlen := nodup_subset_length_le rest (s ++ t) hnd' hsub'
    simp only [List.length_append, List.length_cons] at hlen ⊢
    omega

/-- The floor over one enumeration: `AtLeast n` is `n ≤ length`. -/
theorem atLeast_iff_enum {s : Set β} {l : List β}
    (hmem : ∀ a, a ∈ l ↔ a ∈ s) (hnd : l.Nodup) (n : Nat) :
    s.AtLeast n ↔ n ≤ l.length := by
  constructor
  · rintro ⟨w, hwnd, hwmem, hwlen⟩
    exact Nat.le_trans hwlen (nodup_subset_length_le w l hwnd
      fun a ha => (hmem a).mpr (hwmem a ha))
  · intro h
    exact ⟨l, hnd, fun a ha => (hmem a).mp ha, h⟩

/-- The ceiling over one enumeration: `AtMost m` is `length ≤ m`. -/
theorem atMost_iff_enum {s : Set β} {l : List β}
    (hmem : ∀ a, a ∈ l ↔ a ∈ s) (hnd : l.Nodup) (m : Nat) :
    s.AtMost m ↔ l.length ≤ m := by
  constructor
  · intro h
    exact h l hnd fun a ha => (hmem a).mp ha
  · intro h w hwnd hwmem
    exact Nat.le_trans (nodup_subset_length_le w l hwnd
      fun a ha => (hmem a).mpr (hwmem a ha)) h

/-- The length-window verdict a walk's answers decide. -/
def windowVerdict (w : Window) (ans : List β) : Prop :=
  w.lo ≤ ans.length ∧ ∀ m, w.hi = some m → ans.length ≤ m

/-- **One walk decides a window.** The count-window judgment over a
set equals the length-window verdict over any duplicate-free
enumeration of it — the whole counting question is one prefix walk's
answer length. -/
theorem window_admits_iff_enum {s : Set β} {l : List β}
    (hmem : ∀ a, a ∈ l ↔ a ∈ s) (hnd : l.Nodup) (w : Window) :
    w.admits s ↔ windowVerdict w l := by
  unfold Window.admits windowVerdict
  rw [atLeast_iff_enum hmem hnd]
  constructor
  · rintro ⟨h1, h2⟩
    exact ⟨h1, fun m hm => (atMost_iff_enum hmem hnd m).mp (h2 m hm)⟩
  · rintro ⟨h1, h2⟩
    exact ⟨h1, fun m hm => (atMost_iff_enum hmem hnd m).mpr (h2 m hm)⟩

/-- A duplicate-free list's mere-membership pairwise property — the
bridge from set-level pairwise facts to `List.Pairwise`. -/
theorem pairwise_of_nodup {l : List β} {D : β → β → Prop}
    (hnd : l.Nodup) (h : ∀ a b, a ∈ l → b ∈ l → a ≠ b → D a b) :
    l.Pairwise D := by
  induction l with
  | nil => exact List.Pairwise.nil
  | cons a rest ih =>
    obtain ⟨hne, hnd'⟩ := List.pairwise_cons.mp hnd
    exact List.Pairwise.cons
      (fun b hb => h a b List.mem_cons_self
        (List.mem_cons_of_mem a hb) (hne b hb))
      (ih hnd' fun x y hx hy =>
        h x y (List.mem_cons_of_mem a hx) (List.mem_cons_of_mem a hy))

/-! ## The verdict readings -/

/-- A collision-free bucket: the FD point probe's verdict. -/
def collisionFree (ans : List β) : Prop :=
  ∀ a b, a ∈ ans → b ∈ ans → a = b

/-- A ψ-witness among the answers: the containment probes' verdict. -/
def witnessed (ψ : Selection) (ans : List Fact) : Prop :=
  ∃ g, g ∈ ans ∧ ψ.satisfies g

/-- A surviving φ-demand among the answers: the reverse probe's
verdict. -/
def demanded (φ : Selection) (ans : List Fact) : Prop :=
  ∃ f, f ∈ ans ∧ φ.satisfies f

/-! ## The plan terms, per form -/

/-- Scalar FD: one point probe at the touched determinant tuple. -/
def fdPlan (t : List Value) : EnforcementPlan (List Value) P :=
  .pointProbe t

/-- IND source arm: one point probe of the KEYED target index at the
added source fact's projected tuple; the ψ read is of the probed
bucket itself (the oracle answers whole facts — the engine's
establishing-fact `F` get is representation below this altitude). -/
def indSourcePlan (t : List Value) : EnforcementPlan (List Value) P :=
  .pointProbe t

/-- IND target arm, first probe: re-establishment — one point probe
of the KEYED target index at the removed tuple. -/
def indReestablishProbe (t : List Value) :
    EnforcementPlan (List Value) P :=
  .pointProbe t

/-- IND target arm, second read: the reverse demand — a WALK of the
source index's bucket at the removed tuple. The source projection is
many-to-one by design (no acceptance rule keys it), so a unit price
would understate; the walk is priced at what it reads. -/
def indReverseWalk (t : List Value) : EnforcementPlan (List Value) P :=
  .prefixWalk t

/-- Window: one prefix walk of a touched parent's child group. -/
def windowPlan (t : List Value) : EnforcementPlan (List Value) P :=
  .prefixWalk t

/-- Pointwise FD: the neighbor probe at the inserted interval's
start. -/
def neighborPlan {α : Type} [LT α] (g : K) (iv : Interval α) :
    EnforcementPlan K α :=
  .neighborProbe g iv.start

/-- Coverage: the entry seek + the ordered walk of one group. -/
def coveragePlan {α : Type} (g : K) : EnforcementPlan K α :=
  .prefixWalk g

/-! ## The consultation counts, proved -/

/-- Scalar FD: one consultation per touched fact. -/
theorem fd_plan_consultations (o : OrderedOracle (List Value) P Fact ple)
    (t : List Value) : (fdPlan (P := P) t).consultations o = 1 := rfl

/-- IND source: one target-index descent per added source fact — the
unit price the target-key acceptance premise licenses
(`accepted_target_key_prices_the_probe`). -/
theorem ind_source_plan_consultations
    (o : OrderedOracle (List Value) P Fact ple) (t : List Value) :
    (indSourcePlan (P := P) t).consultations o = 1 := rfl

/-- IND target, re-establishment: one keyed target-index descent per
removed target fact. -/
theorem ind_reestablish_consultations
    (o : OrderedOracle (List Value) P Fact ple) (t : List Value) :
    (indReestablishProbe (P := P) t).consultations o = 1 := rfl

/-- IND target, reverse demand: the entry seek plus one read per
walked source-bucket member — the honest price of an UNKEYED
grouping, never a flat count. -/
theorem ind_reverse_walk_consultations
    (o : OrderedOracle (List Value) P Fact ple) (t : List Value) :
    (indReverseWalk (P := P) t).consultations o =
      1 + (o.consult t).length := rfl

/-- Pointwise FD: two consultations per touched fact. -/
theorem neighbor_plan_consultations {α : Type} [LT α]
    {qle : α → α → Prop} (o : OrderedOracle K α (Interval α) qle)
    (g : K) (iv : Interval α) :
    (neighborPlan g iv).consultations o = 2 := rfl

/-- Coverage: one entry seek + one read per walked segment. -/
theorem coverage_plan_consultations {α : Type} [LT α]
    {qle : α → α → Prop} (o : OrderedOracle K α (Interval α) qle)
    (g : K) :
    (coveragePlan (α := α) g).consultations o =
      1 + (o.consult g).length := rfl

/-- The sealed member test consults nothing. -/
theorem member_test_consultations (o : OrderedOracle K P β ple) :
    (EnforcementPlan.memberTest : EnforcementPlan K P).consultations o
      = 0 := rfl

/-- **The window bound, as an equation**: over any touched-parent
list, total consultations = touched parents + total touched-group
sizes — one seek per parent, one read per group member. -/
theorem window_plan_consultations
    (o : OrderedOracle (List Value) P Fact ple)
    (parents : List (List Value)) :
    costs o (parents.map (windowPlan (P := P))) =
      parents.length +
        (parents.map fun t => (o.consult t).length).foldr (· + ·) 0 := by
  induction parents with
  | nil => rfl
  | cons t ps ih =>
    simp only [costs, windowPlan, EnforcementPlan.consultations,
      List.map_cons, List.foldr_cons, List.length_cons] at ih ⊢
    omega

/-! ## Form 1 — the scalar FD -/

/-- The scalar-FD plan obligation: against every conforming oracle
over the relation's determinant index, the collision-free verdict
over the touched buckets decides `fdDeltaCheck`. -/
def FdPlanned (T : Theory) (I : Instance) (d : Txn.Delta) (R : RelId)
    (X : List FieldId) : Prop :=
  ∀ (P : Type) (ple : P → P → Prop)
    (o : OrderedOracle (List Value) P Fact ple),
    o.facts = T.den (d.applyTo I) R →
    (∀ f, o.groupOf f = f.project X) →
    ((∀ t, t ∈ d.projected R X →
        collisionFree ((fdPlan (P := P) t).answers o)) ↔
      Txn.fdDeltaCheck T I d R X)

/-- **The scalar-FD plan theorem** (one point probe per touched
fact, `fd_plan_consultations`): the probed buckets' collision
freedom IS the delta-restricted key check. -/
theorem fd_plan_decides (T : Theory) (I : Instance) (d : Txn.Delta)
    (R : RelId) (X : List FieldId) : FdPlanned T I d R X := by
  intro P ple o hfacts hkey
  have hmem : ∀ (t : List Value) (f : Fact),
      f ∈ (fdPlan (P := P) t).answers o ↔
        f ∈ T.den (d.applyTo I) R ∧ f.project X = t := by
    intro t f
    show f ∈ o.consult t ↔ _
    rw [o.consult_mem t f, hfacts, hkey f]
  constructor
  · intro h f g hf hg ht hproj
    exact h (f.project X) ht f g ((hmem _ f).mpr ⟨hf, rfl⟩)
      ((hmem _ g).mpr ⟨hg, hproj.symm⟩)
  · intro hc t ht a b ha hb
    obtain ⟨haf, hat⟩ := (hmem t a).mp ha
    obtain ⟨hbf, hbt⟩ := (hmem t b).mp hb
    exact hc a b haf hbf (by rw [hat]; exact ht) (hat.trans hbt.symm)

/-! ## Form 3 — the scalar IND (both arms) -/

/-- The containment plan obligation: against every conforming pair of
oracles — the source-projection index and the target-key index — the
two probe arms decide `containmentDeltaCheck`: per added source fact
a ψ-witness probe of the keyed target index (`indSourcePlan`), per
removed target fact a re-establishment probe (`indReestablishProbe`)
or the absence of a surviving reverse demand in the walked source
bucket (`indReverseWalk`) — the decided terms ARE the priced terms,
minted once and consumed here. -/
def ContainmentPlanned (T : Theory) (I : Instance) (d : Txn.Delta)
    (src tgt : Atom) : Prop :=
  ∀ (P P' : Type) (ple : P → P → Prop) (ple' : P' → P' → Prop)
    (oS : OrderedOracle (List Value) P Fact ple)
    (oT : OrderedOracle (List Value) P' Fact ple'),
    oS.facts = T.den (d.applyTo I) src.relation →
    (∀ f, oS.groupOf f = f.project src.projection) →
    oT.facts = T.den (d.applyTo I) tgt.relation →
    (∀ g, oT.groupOf g = g.project tgt.projection) →
    (((∀ f, f ∈ d.adds src.relation →
        f ∈ T.den (d.applyTo I) src.relation →
        src.selection.satisfies f →
        witnessed tgt.selection
          ((indSourcePlan (P := P')
            (f.project src.projection)).answers oT)) ∧
      (∀ g, g ∈ d.removes tgt.relation →
        tgt.selection.satisfies g →
        witnessed tgt.selection
          ((indReestablishProbe (P := P')
            (g.project tgt.projection)).answers oT) ∨
        ¬ demanded src.selection
          ((indReverseWalk (P := P)
            (g.project tgt.projection)).answers oS))) ↔
      Txn.containmentDeltaCheck T I d src tgt)

/-- **The containment plan theorem** (source arm: one keyed target
probe per added source fact, `ind_source_plan_consultations`; target
arm: one keyed re-establishment probe plus one reverse-demand walk of
the source bucket per removed ψ-target fact,
`ind_reestablish_consultations` / `ind_reverse_walk_consultations`):
both probe arms together ARE the delta-restricted containment check —
the theorem evaluates the same named terms the count theorems
price. -/
theorem containment_plan_decides (T : Theory) (I : Instance)
    (d : Txn.Delta) (src tgt : Atom) :
    ContainmentPlanned T I d src tgt := by
  intro P P' ple ple' oS oT hSf hSk hTf hTk
  have hTmem : ∀ (t : List Value) (g : Fact), g ∈ oT.consult t ↔
      g ∈ T.den (d.applyTo I) tgt.relation ∧
        g.project tgt.projection = t := by
    intro t g
    rw [oT.consult_mem t g, hTf, hTk g]
  have hSmem : ∀ (t : List Value) (f : Fact), f ∈ oS.consult t ↔
      f ∈ T.den (d.applyTo I) src.relation ∧
        f.project src.projection = t := by
    intro t f
    rw [oS.consult_mem t f, hSf, hSk f]
  have hwit : ∀ t : List Value,
      witnessed tgt.selection
        ((indSourcePlan (P := P') t).answers oT) ↔
      ∃ g, g ∈ T.den (d.applyTo I) tgt.relation ∧
        tgt.selection.satisfies g ∧ g.project tgt.projection = t := by
    intro t
    constructor
    · rintro ⟨g, hg, hψ⟩
      obtain ⟨hgf, hgt⟩ := (hTmem t g).mp hg
      exact ⟨g, hgf, hψ, hgt⟩
    · rintro ⟨g, hgf, hψ, hgt⟩
      exact ⟨g, (hTmem t g).mpr ⟨hgf, hgt⟩, hψ⟩
  constructor
  · rintro ⟨hsrc, htgt⟩
    constructor
    · intro f hadd hfin hφ
      exact (hwit _).mp (hsrc f hadd hfin hφ)
    · intro f hfin hφ hmem
      obtain ⟨⟨g, hgrem, hgψ, hgproj⟩, hnohold⟩ := hmem
      rcases htgt g hgrem hgψ with hw | hnd
      · obtain ⟨g', hg'f, hg'ψ, hg'p⟩ := (hwit _).mp hw
        exact hnohold ⟨g', hg'f, hg'ψ, hg'p.trans hgproj⟩
      · exact hnd ⟨f, (hSmem _ f).mpr ⟨hfin, hgproj.symm⟩, hφ⟩
  · intro hc
    constructor
    · intro f hadd hfin hφ
      exact (hwit _).mpr (hc.1 f hadd hfin hφ)
    · intro g hgrem hgψ
      by_cases hw : witnessed tgt.selection
          ((indReestablishProbe (P := P')
            (g.project tgt.projection)).answers oT)
      · exact Or.inl hw
      · refine Or.inr ?_
        rintro ⟨f, hf, hφ⟩
        obtain ⟨hfin, hfp⟩ := (hSmem _ f).mp hf
        refine hc.2 f hfin hφ ⟨⟨g, hgrem, hgψ, hfp.symm⟩, ?_⟩
        rintro ⟨g', hg'f, hg'ψ, hg'p⟩
        exact hw ((hwit _).mpr ⟨g', hg'f, hg'ψ, hg'p.trans hfp⟩)

/-- **The closed-target sharpening** (`memberTest`,
`member_test_consultations` = 0): a containment into a closed
relation is decided by the sealed extension itself — the enforcement
plan is the answer set, zero oracle consultations (the compiled
member set of the acceptance gate). The no-removes hypothesis is the
engine's closed-write refusal (`ClosedRelationWrite` — the `Txn.lean`
narrowing), which makes the target arm vacuous. -/
theorem member_test_decides {T : Theory} {I : Instance}
    {d : Txn.Delta} {src tgt : Atom} {ext : GroundExtension}
    (hclosed : T.closed tgt.relation = some ext)
    (hnorem : ∀ g, g ∉ d.removes tgt.relation) :
    (∀ f, f ∈ d.adds src.relation →
        f ∈ T.den (d.applyTo I) src.relation →
        src.selection.satisfies f →
        ∃ g, g ∈ ext.facts ∧ tgt.selection.satisfies g ∧
          g.project tgt.projection = f.project src.projection) ↔
      Txn.containmentDeltaCheck T I d src tgt := by
  have hden : T.den (d.applyTo I) tgt.relation =
      fun g => g ∈ ext.facts := by
    unfold Theory.den
    rw [hclosed]
  constructor
  · intro h
    constructor
    · intro f hadd hfin hφ
      obtain ⟨g, hg, hψ, hp⟩ := h f hadd hfin hφ
      refine ⟨g, ?_, hψ, hp⟩
      rw [hden]
      exact hg
    · intro f _ _ hmem
      obtain ⟨⟨g, hgrem, -, -⟩, -⟩ := hmem
      exact hnorem g hgrem
  · intro hc f hadd hfin hφ
    obtain ⟨g, hgf, hψ, hp⟩ := hc.1 f hadd hfin hφ
    rw [hden] at hgf
    exact ⟨g, hgf, hψ, hp⟩

/-! ## Form 5 — the cardinality window -/

/-- The window plan obligation: against every conforming oracle over
the σ-selected source's parent-key index, the per-touched-parent walk
verdict decides `cardinalityDeltaCheck`. -/
def WindowPlanned (T : Theory) (I : Instance) (d : Txn.Delta)
    (src : Atom) (w : Window) (tgt : Atom) : Prop :=
  ∀ (P : Type) (ple : P → P → Prop)
    (o : OrderedOracle (List Value) P Fact ple),
    o.facts = Selected (T.den (d.applyTo I) src.relation)
      src.selection →
    (∀ f, o.groupOf f = f.project src.projection) →
    ((∀ g, g ∈ T.den (d.applyTo I) tgt.relation →
        tgt.selection.satisfies g →
        g.project tgt.projection ∈ Txn.touchedParents d src tgt →
        windowVerdict w
          ((windowPlan (P := P) (g.project tgt.projection)).answers o)) ↔
      Txn.cardinalityDeltaCheck T I d src w tgt)

/-- **The window plan theorem** (a prefix walk of each touched
parent's child group; the bound is `window_plan_consultations` —
consultations = touched parents + total touched-group sizes): the
length-window verdict over each walked group IS the delta-restricted
window check. -/
theorem cardinality_plan_decides (T : Theory) (I : Instance)
    (d : Txn.Delta) (src : Atom) (w : Window) (tgt : Atom) :
    WindowPlanned T I d src w tgt := by
  intro P ple o hfacts hkey
  have hgrp : ∀ (t : List Value) (a : Fact),
      a ∈ (windowPlan (P := P) t).answers o ↔
        a ∈ ChildGroup (T.den (d.applyTo I) src.relation)
          src.selection src.projection t := by
    intro t a
    show a ∈ o.consult t ↔ _
    rw [o.consult_mem t a, hfacts, hkey a]
    exact ⟨fun ⟨⟨h1, h2⟩, h3⟩ => ⟨h1, h2, h3⟩,
      fun ⟨h1, h2, h3⟩ => ⟨⟨h1, h2⟩, h3⟩⟩
  have hadm : ∀ t : List Value,
      windowVerdict w ((windowPlan (P := P) t).answers o) ↔
        w.admits (ChildGroup (T.den (d.applyTo I) src.relation)
          src.selection src.projection t) :=
    fun t => (window_admits_iff_enum (hgrp t) (o.consult_nodup t) w).symm
  exact ⟨fun h g hg hψ ht => (hadm _).mp (h g hg hψ ht),
    fun h g hg hψ ht => (hadm _).mpr (h g hg hψ ht)⟩

/-! ## Form 2 — the pointwise FD (the sweep's altitude) -/

/-- The pointwise-FD plan obligation, at the interval altitude: for
every conforming oracle over start-positioned intervals, clearing the
two probed neighbors decides clearing the whole group — under the
group-disjointness premise, which is exactly what the pre-state key
supplies per group (the applier's premise chain). -/
def NeighborPlanned : Prop :=
  ∀ (α : Type) [LT α] [LE α] [LinearElem α] (K : Type)
    (o : OrderedOracle K α (Interval α) (· ≤ ·)),
    (∀ jv, o.posOf jv = jv.start) →
    ∀ (g : K),
      (∀ a b, a ∈ o.facts → o.groupOf a = g → b ∈ o.facts →
        o.groupOf b = g → a ≠ b →
        ∀ x, x ∈ a.points → x ∉ b.points) →
      ∀ iv : Interval α,
        ((∀ jv, jv ∈ (neighborPlan g iv).answers o →
            ∀ x, x ∈ iv.points → x ∉ jv.points) ↔
          (∀ jv, jv ∈ o.facts → o.groupOf jv = g →
            ∀ x, x ∈ iv.points → x ∉ jv.points))

/-- **The neighbor-probe theorem** (two consultations,
`neighbor_plan_consultations`): under per-group disjointness, an
interval clearing its predecessor and successor clears the whole
group — the sandwich argument: a farther overlap either passes
through the probed neighbor's points or forces the neighbor inside
another member, refuting the group premise. -/
theorem neighbor_probe_decides : NeighborPlanned := by
  intro α _ _ _ K o hpos g hdisj iv
  constructor
  · intro hclear jv hjf hjg x hxiv hxjv
    have hxiv' : iv.start ≤ x ∧ x < iv.«end» := hxiv
    have hxjv' : jv.start ≤ x ∧ x < jv.«end» := hxjv
    rcases LinearElem.le_total jv.start iv.start with hle | hle
    · -- the predecessor side
      cases hp : o.pred g iv.start with
      | none =>
        exact o.pred_none g iv.start hp jv hjf hjg
          (by rw [hpos jv]; exact hle)
      | some p =>
        obtain ⟨hpf, hpg, hpk⟩ := o.pred_mem g iv.start p hp
        rw [hpos p] at hpk
        have hclearp := hclear p (mem_neighborAnswers.mpr (.inl hp))
        by_cases hpj : p = jv
        · subst hpj
          exact hclearp x hxiv hxjv
        · have hjp : jv.start ≤ p.start := by
            have h1 := o.pred_greatest g iv.start p hp jv hjf hjg
              (by rw [hpos jv]; exact hle)
            rw [hpos jv, hpos p] at h1
            exact h1
          by_cases hxp : x < p.«end»
          · exact hclearp x hxiv ⟨LinearElem.le_trans hpk hxiv'.1, hxp⟩
          · have hpe : p.«end» ≤ x := LinearElem.le_of_not_lt hxp
            exact hdisj p jv hpf hpg hjf hjg hpj p.start
              ⟨LinearElem.le_refl _, p.h⟩
              ⟨hjp, LinearElem.lt_trans
                (LinearElem.lt_of_lt_of_le p.h hpe) hxjv'.2⟩
    · -- the successor side
      cases hs : o.succ g iv.start with
      | none =>
        exact o.succ_none g iv.start hs jv hjf hjg
          (by rw [hpos jv]; exact hle)
      | some q =>
        obtain ⟨hqf, hqg, hqk⟩ := o.succ_mem g iv.start q hs
        rw [hpos q] at hqk
        have hclearq := hclear q (mem_neighborAnswers.mpr (.inr hs))
        by_cases hqj : q = jv
        · subst hqj
          exact hclearq x hxiv hxjv
        · have hjq : q.start ≤ jv.start := by
            have h1 := o.succ_least g iv.start q hs jv hjf hjg
              (by rw [hpos jv]; exact hle)
            rw [hpos jv, hpos q] at h1
            exact h1
          by_cases hxq : x < q.«end»
          · exact hclearq x hxiv
              ⟨LinearElem.le_trans hjq hxjv'.1, hxq⟩
          · have hqe : q.«end» ≤ x := LinearElem.le_of_not_lt hxq
            exact hclearq q.start
              ⟨hqk, LinearElem.lt_trans
                (LinearElem.lt_of_lt_of_le q.h hqe) hxiv'.2⟩
              ⟨LinearElem.le_refl _, q.h⟩
  · intro hall jv hjv x hxiv hxjv
    rcases mem_neighborAnswers.mp hjv with h | h
    · obtain ⟨hf, hg, -⟩ := o.pred_mem g iv.start jv h
      exact hall jv hf hg x hxiv hxjv
    · obtain ⟨hf, hg, -⟩ := o.succ_mem g iv.start jv h
      exact hall jv hf hg x hxiv hxjv

/-! ## Form 4 — coverage (the sweep's altitude) -/

/-- The coverage plan obligation, at the interval altitude: for every
conforming oracle over one group's start-positioned segments, the
walk-fed sweep verdict decides the point-subset denotation — under
the target-key disjointness premise, the `DisjointDeterminantProof`
discipline generalized. -/
def CoveragePlanned : Prop :=
  ∀ (α : Type) [LT α] [LE α] [LinearElem α] [DecidableLT α]
    [DecidableLE α] (K : Type)
    (o : OrderedOracle K α (Interval α) (· ≤ ·)),
    (∀ jv, o.posOf jv = jv.start) →
    ∀ (g : K),
      (∀ a b, a ∈ o.facts → o.groupOf a = g → b ∈ o.facts →
        o.groupOf b = g → a ≠ b →
        ∀ x, x ∈ a.points → x ∉ b.points) →
      ∀ src : Interval α,
        (Exec.sweepCovered src ((coveragePlan (α := α) g).answers o)
            = true ↔
          ∀ x, x ∈ src.points →
            ∃ jv, jv ∈ o.facts ∧ o.groupOf jv = g ∧ x ∈ jv.points)

/-- **The coverage walk theorem** (one entry seek + one read per
walked segment, `coverage_plan_consultations`): the ordered walk
hands `Exec/Sweep.lean`'s one-pass fold exactly the group, in start
order; under the group-disjointness premise the sweep verdict IS the
point-subset denotation (`sweep_covered_sound_complete` spent — the
plan calculus consuming the `DisjointDeterminantProof` theorem). -/
theorem coverage_walk_decides : CoveragePlanned := by
  intro α _ _ _ _ _ K o hpos g hdisj src
  have hmemseg : ∀ jv : Interval α, jv ∈ o.consult g ↔
      jv ∈ o.facts ∧ o.groupOf jv = g := fun jv => o.consult_mem g jv
  have hord : Exec.Ordered (o.consult g) := by
    refine (o.consult_ordered g).imp ?_
    intro a b hab
    rw [hpos a, hpos b] at hab
    exact hab
  have hdisjL : Exec.Disjoint (o.consult g) := by
    refine pairwise_of_nodup (o.consult_nodup g) ?_
    intro a b ha hb hne
    obtain ⟨haf, hag⟩ := (hmemseg a).mp ha
    obtain ⟨hbf, hbg⟩ := (hmemseg b).mp hb
    exact hdisj a b haf hag hbf hbg hne
  have hsweep := Exec.sweep_covered_sound_complete src (o.consult g)
    ⟨hord, hdisjL⟩
  constructor
  · intro hv x hx
    obtain ⟨jv, hjv, hxjv⟩ := hsweep.mp hv x hx
    obtain ⟨hjf, hjg⟩ := (hmemseg jv).mp hjv
    exact ⟨jv, hjf, hjg, hxjv⟩
  · intro hv
    refine hsweep.mpr fun x hx => ?_
    obtain ⟨jv, hjf, hjg, hxjv⟩ := hv x hx
    exact ⟨jv, (hmemseg jv).mpr ⟨hjf, hjg⟩, hxjv⟩

/-! ## The acceptance-gate theorem -/

/-- Every statement's plan obligation — `Txn.deltaCheck`'s dispatch,
arm for arm, each form replaced by the existence-and-correctness of
its plan. The scalar arms carry full fact-level fidelity to their
delta-restricted checks; the pointwise arms carry the probe-decision
laws at the sweep's altitude (module doc, narrowings). -/
def planObligation (T : Theory) (I : Instance) (d : Txn.Delta) :
    Statement → Prop
  | .functionality R X =>
    match T.header.intervalSplit R X with
    | some _ => NeighborPlanned
    | none => FdPlanned T I d R X
  | .containment src tgt =>
    match T.header.intervalSplit src.relation src.projection,
          T.header.intervalSplit tgt.relation tgt.projection with
    | some _, some _ => CoveragePlanned
    | _, _ => ContainmentPlanned T I d src tgt
  | .cardinality src w tgt => WindowPlanned T I d src w tgt

/-- **The acceptance-gate theorem.** Every `Statement` constructor
HAS its plan term: each accepted form's delta-restricted check is
decided by an evaluation of sanctioned probes with a proved
consultation count — "an accepted statement is a measured promise"
(`docs/architecture/30-dependencies.md` § the acceptance gate) made
literal. The acceptance premises price the probes
(`accepted_target_key_prices_the_probe`); the E1 shape has no plan
term at the gate's own type
(`Countermodels.joined_window_form_uninhabitable`, over the blast
countermodel `Countermodels.joined_window_blast`). -/
theorem acceptance_gate (T : Theory) (I : Instance) (d : Txn.Delta) :
    ∀ st : Statement, planObligation T I d st := by
  intro st
  cases st with
  | functionality R X =>
    cases h : T.header.intervalSplit R X with
    | some p =>
      simp only [planObligation, h]
      exact neighbor_probe_decides
    | none =>
      simp only [planObligation, h]
      exact fd_plan_decides T I d R X
  | containment src tgt =>
    cases hs : T.header.intervalSplit src.relation src.projection with
    | some p =>
      cases ht : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        simp only [planObligation, hs, ht]
        exact coverage_walk_decides
      | none =>
        simp only [planObligation, hs, ht]
        exact containment_plan_decides T I d src tgt
    | none =>
      cases ht : T.header.intervalSplit tgt.relation
          tgt.projection with
      | some q =>
        simp only [planObligation, hs, ht]
        exact containment_plan_decides T I d src tgt
      | none =>
        simp only [planObligation, hs, ht]
        exact containment_plan_decides T I d src tgt
  | cardinality src w tgt =>
    exact cardinality_plan_decides T I d src w tgt

end Oracle
end Bumbledb
