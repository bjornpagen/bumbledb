import Bumbledb.Query.Aggregates

/-!
# Exec/Sweep — the sweep as a fold (Level 1, PRD 06)

The engine's ONE shared sweep
(`crates/bumbledb/src/interval/sweep.rs::sweep`) modeled at its
algorithmic essence: a covered-frontier fold over a start-ordered
segment list. Two Rust consumers drive the one walk, and both get
their theorems here:

* **`storage/commit/judgment.rs::check_coverage`** — the containment
  judgment's coverage walk, which consumes a
  `DisjointDeterminantProof` token before walking.
  `sweep_covered_sound_complete` IS that token's theorem: under
  `Ordered ∧ Disjoint` — exactly what a pointwise key guarantees per
  prefix group (`pointwise_key_disjoint`, PRD 03) plus the
  determinant index's key order — the one-pass verdict equals the
  point-subset denotation.
* **Pack's finalize** (`exec/sink/aggregate/finalize.rs`, driving the
  windowless sweep) — `pack_is_the_sweep` proves this file's
  run-emitting fold IS PRD 05's `pack`: one fold, two consumers, the
  code-sharing claim the docs brag about, proved.

## The seam, recorded (the predecessor probe)

Rust's `check_coverage` enters the walk two-phase: an ordered-index
seek locates the one entry segment that can cover the source start `s`
(the ≥ probe, else the group predecessor), then the forward chain is
swept. At the fold level the walk is modeled from a frontier
INITIALIZED at `s` (Rust's `run = Some((s, s))`) over the whole
ordered group: a segment wholly at-or-before the frontier is a no-op
(`sweep_ignores_spent_segments`), so the seek only skips segments the
fold would ignore anyway — the two-phase entry is mechanism, not
semantics.

## Findings recorded (law 5)

* **`Disjoint` goes unspent at the fold level.** Completeness needs
  only `Ordered` (`sweep_complete_of_ordered`), and soundness needs
  NO premise at all (`sweep_never_false_accepts`) — mirroring the
  Rust module doc: overlapping inputs are legal, "tracking the
  *maximum* frontier subsumes disjoint chaining". The `Disjoint` half
  of the `DisjointDeterminantProof` premise licences what sits BELOW
  the fold's altitude: the predecessor-seek entry ("a predecessor
  that has ended proves nothing covers `s`" — decisive only because
  the group is disjoint and start-ordered). The premise stays visible
  in the statement — it is the token's meaning — and the finding is
  recorded here and in the countermodel section.
* **A false ACCEPT is not constructible.** The PRD's countermodel
  asks for both wrong-verdict directions "if constructible";
  `sweep_never_false_accepts` proves the accept direction sound with
  no premises whatsoever, so only the false REJECT exists
  (`Countermodels.sweep_premise_load_bearing` — the unordered list
  the verdict wrongly rejects).
* **The Pack tie order is closed by theorem, not argument.** The
  engine sorts claims lexicographically on `[start, end]`
  (`finalize.rs::finalize_into`); the spec's `sortByStart` reads
  starts alone. `sweepRuns_tie_order_irrelevant` proves the walk
  equal on any two start-ordered arrangements of one claim
  collection — the fidelity record's remaining transfer argument
  (equal-start tie order), now an equation composed from
  `sweepRuns_eq_pack_of_ordered` and PRD 05's
  `pack_input_order_irrelevant`.
* **`ray_needs_ray` carries the ceiling premise explicitly.** That
  `maxEnd` is the GREATEST element is a fact of the two real domains
  (`ceiling_greatest_u64` / `ceiling_greatest_i64`), not of PRD 02's
  `PointDomain` class; the theorem takes it as a visible hypothesis
  rather than widening the class.
* **The σ conjunct rides ABOVE the fold.** `check_coverage`'s full
  verdict is coverage AND every consumed segment satisfies ψ
  (`storage/commit/judgment.rs::GapAt::segment` delegating to
  `storage/commit/judgment.rs::check_segment`; the hook is
  `interval/sweep.rs::Continuation::segment`); `sweepCovered` models
  pure coverage. The σ semantics belong to the `Coverage` denotation
  (`Dependencies.lean`, which carries ψ); this file's fold is the
  coverage half only — delegated, not dropped.
* **The degenerate window is unstatable here.** Rust declares an
  empty window (`s = e`) vacuously covered (`interval/sweep.rs::sweep`:
  the run opens at `(s, s)`, the `frontier ≥ e` exit fires immediately);
  `Interval` carries `start < end`, so the shape cannot be written at
  this level — and it is unreachable through `check_coverage`, whose
  probe intervals are acceptance-gated valid intervals.
* **The windowed gap verdict is delegated to the continuation.** A
  windowed continuation that declined to convict would make `sweep`
  return accept on a gap (the windowed early return in
  `interval/sweep.rs::sweep`) where `sweepFrom`
  hard-codes `false`; the only windowed continuation, `GapAt`, always
  errs (`storage/commit/judgment.rs::GapAt::maximal`), so the
  divergence is unreachable —
  the spec does not determine `sweep` for a non-convicting windowed
  caller.

Mechanism fence: a sweep is a fold, nothing else — batching, buffers,
scratch, SIMD, pipelining, memos, and LMDB are banned from this file
forever.
-/

namespace Bumbledb

variable {α : Type} [LT α] [LE α] [LinearElem α] [DecidableLT α]
  [DecidableLE α]

omit [DecidableLT α] [DecidableLE α] in
/-- Antisymmetry of the element order — derived from `le_iff`, needed
by the spent-segment no-op lemma. -/
theorem LinearElem.le_antisymm {a b : α} (h : a ≤ b) (h' : b ≤ a) :
    a = b := by
  rcases (LinearElem.le_iff a b).mp h with hlt | rfl
  · exact absurd hlt (LinearElem.not_lt_of_le h')
  · rfl

namespace Exec

/-! ## The premises — the witness type's meaning

`Ordered ∧ Disjoint` is precisely what `DisjointDeterminantProof`
attests: the token is minted when a pointwise key is accepted
(`pointwise_key_disjoint` gives `Disjoint` per prefix group) and the
determinant index's key order gives `Ordered` — the checker's
`check_coverage` demands the token before entering the walk. -/

/-- `Ordered`: the segment list is start-sorted — what the
determinant index's key order gives `check_coverage`'s forward chain, and
what `pack`'s sort pass establishes for the finalize sweep. -/
def Ordered (l : List (Interval α)) : Prop :=
  l.Pairwise (fun a b => a.start ≤ b.start)

/-- `Disjoint`: the segments are pairwise point-disjoint — exactly
what a pointwise key guarantees per prefix group
(`pointwise_key_disjoint`, PRD 03), the half of the
`DisjointDeterminantProof` premise the key acceptance mints. -/
def Disjoint (l : List (Interval α)) : Prop :=
  l.Pairwise (fun a b => ∀ x : α, x ∈ a.points → x ∉ b.points)

/-! ## The windowed walk — `check_coverage`'s shape -/

/-- The covered-frontier walk under a window ending at `e`, entered
with frontier `f`: the fold image of `interval/sweep.rs::sweep` with
`window = Some((s, e))` and `run = (s, f)` (Rust initializes `f = s`;
`sweepCovered` below does the same). Per step, in Rust's order: the
early exit (`frontier ≥ e` — later input is moot), the gap verdict
(`start > frontier` — a stalled window frontier can never recover),
else consume and advance the frontier to the max (`frontier.max(end)`
— a contained segment must not shrink it). Exhaustion is the
gap-at-frontier verdict unless the window was already covered. -/
def sweepFrom (e : α) (f : α) : List (Interval α) → Bool
  | [] => decide (e ≤ f)
  | iv :: rest =>
    if e ≤ f then true
    else if f < iv.start then false
    else sweepFrom e (maxE f iv.«end») rest

/-- `sweepCovered` — the coverage verdict: is the source window
`[src.start, src.«end»)` jointly covered by the segment list? The
fold image of `judgment.rs::check_coverage`'s call into the shared
sweep (`sweep(segments, Some((source_start, source_end)), GapAt)`),
with the frontier opened at the source start — the
predecessor-initialized entry (see the seam note in the module doc:
the ordered-index seek skips only segments this fold ignores anyway,
`sweep_ignores_spent_segments`). -/
def sweepCovered (src : Interval α) (segs : List (Interval α)) : Bool :=
  sweepFrom src.«end» src.start segs

/-! ## The windowless walk — Pack's shape -/

/-- The run-emitting walk: the fold image of
`interval/sweep.rs::sweep` with `window = None`, carrying the open
run as an `Interval` (the run invariant `start < frontier` is the
type). A gap (`frontier < start`) emits the maximal run and opens a
new one; anything else joins the frontier; exhaustion emits the last
run — `Continuation::maximal`, twice. -/
def sweepRuns (run : Interval α) : List (Interval α) → List (Interval α)
  | [] => [run]
  | iv :: rest =>
    if run.«end» < iv.start then
      run :: sweepRuns iv rest
    else
      sweepRuns
        ⟨run.start, maxE run.«end» iv.«end»,
          LinearElem.lt_of_lt_of_le run.h (le_maxE_left run.«end» iv.«end»)⟩
        rest

/-- `sweepPack` — Pack as this file's sweep: sort by start (the
engine's `sort_unstable` pass in `finalize.rs`), then the windowless
run-emitting walk. `pack_is_the_sweep` proves it equal to PRD 05's
spec function `pack`. -/
def sweepPack (l : List (Interval α)) : List (Interval α) :=
  match sortByStart l with
  | [] => []
  | iv :: rest => sweepRuns iv rest

/-! ## Soundness — no premise at all

The accept verdict is trustworthy on ANY input: the frontier only
ever advances over consumed points, so `true` always means covered.
This is the recorded finding that a false ACCEPT is not constructible
(the countermodel's other direction, refused by theorem). -/

/-- **`sweep_never_false_accepts`.** An accepting walk really covered
`[f, e)` — with NO ordering or disjointness premise: the frontier
advances only across points some consumed segment holds. The reason
the countermodel (`Countermodels.sweep_premise_load_bearing`) can
only exhibit a false REJECT. Bridge: `interval/sweep.rs` ("tracking
the *maximum* frontier subsumes disjoint chaining"); the checker
never convicts the innocent even off its premises. -/
theorem sweep_never_false_accepts {e : α} :
    ∀ {l : List (Interval α)} {f : α}, sweepFrom e f l = true →
      ∀ x : α, f ≤ x → x < e → x ∈ unionPoints l
  | [], f, h, x, hfx, hxe => by
    have he : e ≤ f := of_decide_eq_true h
    exact absurd (LinearElem.lt_of_lt_of_le hxe (LinearElem.le_trans he hfx))
      (LinearElem.lt_irrefl x)
  | iv :: rest, f, h, x, hfx, hxe => by
    unfold sweepFrom at h
    by_cases hef : e ≤ f
    · exact absurd (LinearElem.lt_of_lt_of_le hxe (LinearElem.le_trans hef hfx))
        (LinearElem.lt_irrefl x)
    · rw [if_neg hef] at h
      by_cases hgap : f < iv.start
      · rw [if_pos hgap] at h
        cases h
      · rw [if_neg hgap] at h
        by_cases hxm : x < maxE f iv.«end»
        · rcases maxE_eq_or f iv.«end» with hm | hm
          · rw [hm] at hxm
            exact absurd hxm (LinearElem.not_lt_of_le hfx)
          · rw [hm] at hxm
            exact mem_unionPoints_cons.mpr
              (.inl ⟨LinearElem.le_trans (LinearElem.le_of_not_lt hgap) hfx, hxm⟩)
        · exact mem_unionPoints_cons.mpr
            (.inr (sweep_never_false_accepts h x
              (LinearElem.le_of_not_lt hxm) hxe))

/-! ## Completeness — `Ordered` is the load-bearing premise -/

/-- **`sweep_complete_of_ordered`.** Over a start-ordered list, a
truly covered window is accepted: the uncovered frontier point must
live in some remaining segment, and order forbids the head to gap
past it. `Disjoint` is NOT needed — the recorded finding (module
doc): overlaps are subsumed by max-frontier tracking, exactly the
Rust module's claim. Bridge: `interval/sweep.rs`, sampled by
`coverage_verdict_matches_the_naive_subset_check`. -/
theorem sweep_complete_of_ordered {e : α} :
    ∀ {l : List (Interval α)} {f : α}, Ordered l →
      (∀ x : α, f ≤ x → x < e → x ∈ unionPoints l) →
      sweepFrom e f l = true
  | [], f, _, hcov => by
    refine decide_eq_true (LinearElem.le_of_not_lt fun hfe => ?_)
    obtain ⟨iv, hiv, -⟩ := hcov f (LinearElem.le_refl f) hfe
    nomatch hiv
  | iv :: rest, f, hord, hcov => by
    obtain ⟨hhd, hord'⟩ := List.pairwise_cons.mp hord
    unfold sweepFrom
    by_cases hef : e ≤ f
    · rw [if_pos hef]
    · rw [if_neg hef]
      have hfe : f < e := by
        rcases LinearElem.trichotomy f e with h | rfl | h
        · exact h
        · exact absurd (LinearElem.le_refl f) hef
        · exact absurd (LinearElem.le_of_lt h) hef
      obtain ⟨jv, hjv, hjmem⟩ := hcov f (LinearElem.le_refl f) hfe
      have hjmem' : jv.start ≤ f ∧ f < jv.«end» := hjmem
      have hivs : iv.start ≤ f := by
        rcases List.mem_cons.mp hjv with rfl | hmem
        · exact hjmem'.1
        · exact LinearElem.le_trans (hhd jv hmem) hjmem'.1
      rw [if_neg (LinearElem.not_lt_of_le hivs)]
      refine sweep_complete_of_ordered hord' fun x hmx hxe => ?_
      have hfx : f ≤ x :=
        LinearElem.le_trans (le_maxE_left f iv.«end») hmx
      rcases mem_unionPoints_cons.mp (hcov x hfx hxe) with hxiv | hxrest
      · have hxiv' : iv.start ≤ x ∧ x < iv.«end» := hxiv
        exact absurd
          (LinearElem.lt_of_lt_of_le hxiv'.2
            (LinearElem.le_trans (le_maxE_right f iv.«end») hmx))
          (LinearElem.lt_irrefl x)
      · exact hxrest

/-! ## Theorem 1 — THE `DisjointDeterminantProof` theorem -/

/-- **Theorem 1 (`sweep_covered_sound_complete`).** Under
`Ordered ∧ Disjoint`, the one-pass verdict IS the denotation:
`sweepCovered src segs = true ↔ points src ⊆ ⋃ points segs`. The
premise is precisely the `DisjointDeterminantProof` token's meaning —
minted at pointwise-key acceptance, demanded by
`judgment.rs::check_coverage` before the walk. Finding (module doc):
only the `Ordered` half is spent at the fold's altitude
(`sweep_complete_of_ordered`; soundness is premise-free,
`sweep_never_false_accepts`); the `Disjoint` half licences the
predecessor-seek entry below it. Bridge: `DisjointDeterminantProof` +
`judgment.rs::check_coverage`; sampled by
`coverage_verdict_matches_the_naive_subset_check`. -/
theorem sweep_covered_sound_complete (src : Interval α)
    (segs : List (Interval α))
    (hprem : Ordered segs ∧ Disjoint segs) :
    sweepCovered src segs = true ↔
      ∀ x ∈ src.points, x ∈ unionPoints segs := by
  constructor
  · intro h x hx
    have hx' : src.start ≤ x ∧ x < src.«end» := hx
    exact sweep_never_false_accepts h x hx'.1 hx'.2
  · intro h
    exact sweep_complete_of_ordered hprem.1 fun x hsx hxe => h x ⟨hsx, hxe⟩

/-! ## Theorem 3 — the early exit's licence -/

omit [LinearElem α] in
/-- **Theorem 3 (`sweep_early_exit_sound`).** Once the frontier
passes the window end, the verdict is `true` on ANY remaining input —
so returning without consuming it (Rust's `return Ok(())` at the loop
head, "later input is moot") loses nothing. Bridge:
`interval/sweep.rs`'s early return; pinned by the second half of
`consumed_segments_are_handed_over_in_order_and_gaps_convict_first`
(input past a covered window is never consumed). -/
theorem sweep_early_exit_sound {e f : α} (hef : e ≤ f) :
    ∀ l : List (Interval α), sweepFrom e f l = true
  | [] => decide_eq_true hef
  | _ :: _ => by unfold sweepFrom; rw [if_pos hef]

/-- The seam lemma (module doc): a segment already inside the covered
run (`«end» ≤ frontier`) is a no-op, so dropping any spent prefix —
what `check_coverage`'s ordered-index predecessor seek does — never
changes the verdict. The seek's licence at the fold's altitude.
Bridge: `judgment.rs::check_coverage`'s entry location. -/
theorem sweep_ignores_spent_segments {e f : α} {iv : Interval α}
    (hspent : iv.«end» ≤ f) (l : List (Interval α)) :
    sweepFrom e f (iv :: l) = sweepFrom e f l := by
  have hivs : ¬ f < iv.start :=
    LinearElem.not_lt_of_le
      (LinearElem.le_of_lt (LinearElem.lt_of_lt_of_le iv.h hspent))
  have hmax : maxE f iv.«end» = f := by
    unfold maxE
    by_cases hle : f ≤ iv.«end»
    · rw [if_pos hle]
      exact LinearElem.le_antisymm hspent hle
    · rw [if_neg hle]
  by_cases hef : e ≤ f
  · rw [sweep_early_exit_sound hef (iv :: l), sweep_early_exit_sound hef l]
  · calc sweepFrom e f (iv :: l)
        = sweepFrom e (maxE f iv.«end») l := by
          show (if e ≤ f then true
            else if f < iv.start then false
            else sweepFrom e (maxE f iv.«end») l) = _
          rw [if_neg hef, if_neg hivs]
      _ = sweepFrom e f l := by rw [hmax]

/-! ## Theorem 4 — one fold, two consumers -/

/-- The windowless walk is PRD 05's coalescing fold, run for run —
the run-carrying state `⟨s, f, h⟩` is `coalesce`'s three arguments as
one `Interval`. -/
theorem sweepRuns_eq_coalesce :
    ∀ (l : List (Interval α)) (s f : α) (h : s < f),
      sweepRuns ⟨s, f, h⟩ l = coalesce s f h l
  | [], _, _, _ => rfl
  | iv :: rest, s, f, h => by
    unfold sweepRuns coalesce
    by_cases hgap : f < iv.start
    · rw [if_pos hgap, if_pos hgap,
        sweepRuns_eq_coalesce rest iv.start iv.«end» iv.h]
    · rw [if_neg hgap, if_neg hgap]
      exact sweepRuns_eq_coalesce rest s (maxE f iv.«end») _

/-- **Theorem 4 (`pack_is_the_sweep`).** `sweepPack = pack`: the
run-emitting sweep, given Pack's sort pass, IS PRD 05's spec function
— one fold, two consumers (`check_coverage` windowed, Pack's finalize
windowless), the code-sharing claim of `interval/sweep.rs` proved.
Every PRD 05 Pack spec (`pack_canonical`, `pack_extensional`,
`pack_adjacency`, `pack_lattice_closed`) transfers to the sweep
through this equation. Bridge: `interval/sweep.rs` +
`exec/sink/aggregate/finalize.rs`; sampled by
`packed_output_matches_the_naive_point_set`. -/
theorem pack_is_the_sweep (l : List (Interval α)) :
    sweepPack l = pack l := by
  unfold sweepPack pack packSorted
  cases sortByStart l with
  | nil => rfl
  | cons iv rest => exact sweepRuns_eq_coalesce rest iv.start iv.«end» iv.h

/-! ### The tie-order transfer — the sort seam, closed

The engine's sort pass is `sort_unstable` on lexicographic
`[start, end]` pairs (`finalize.rs::finalize_into`); the spec's
`sortByStart` reads starts alone. Both hand the walk a start-ordered
list, and the theorem below proves that is ALL the walk can see — the
tie order among equal starts was an argument in the fidelity record
and is now an equation. -/

/-- Over a start-ordered input the run-emitting walk IS `pack`:
`sortByStart` is the identity on it (`sortByStart_id_of_sorted`, the
sort seam) and the walk is `coalesce` (`sweepRuns_eq_coalesce`). -/
theorem sweepRuns_eq_pack_of_ordered {iv : Interval α}
    {rest : List (Interval α)} (hord : Ordered (iv :: rest)) :
    sweepRuns iv rest = pack (iv :: rest) := by
  unfold pack
  rw [sortByStart_id_of_sorted hord]
  exact sweepRuns_eq_coalesce rest iv.start iv.«end» iv.h

/-- **The tie-order transfer.** The run-emitting walk is invariant
across start-ordered arrangements: any two start-ordered lists
carrying the same claims — membership-equal, which every permutation
of one claim collection is — produce IDENTICAL runs. So the engine's
`[start, end]` lexicographic sort and the spec's start-only sort
cannot be told apart through Pack: the tie order among equal starts
is provably irrelevant. Composes `sweepRuns_eq_pack_of_ordered` with
PRD 05's input-order theorem (`pack_input_order_irrelevant` —
canonical-form uniqueness under `pack_canonical` +
`pack_extensional`). Bridge: `finalize.rs::finalize_into`'s sort pass
feeding `interval/sweep.rs::sweep`; sampled by
`packed_output_matches_the_naive_point_set`. -/
theorem sweepRuns_tie_order_irrelevant {iv₁ iv₂ : Interval α}
    {l₁ l₂ : List (Interval α)}
    (h₁ : Ordered (iv₁ :: l₁)) (h₂ : Ordered (iv₂ :: l₂))
    (hmem : ∀ jv, jv ∈ iv₁ :: l₁ ↔ jv ∈ iv₂ :: l₂) :
    sweepRuns iv₁ l₁ = sweepRuns iv₂ l₂ := by
  rw [sweepRuns_eq_pack_of_ordered h₁, sweepRuns_eq_pack_of_ordered h₂]
  refine pack_input_order_irrelevant _ _ fun x => ?_
  constructor
  · rintro ⟨jv, hjv, hx⟩
    exact ⟨jv, (hmem jv).mp hjv, hx⟩
  · rintro ⟨jv, hjv, hx⟩
    exact ⟨jv, (hmem jv).mpr hjv, hx⟩

/-! ## Theorem 5 — coverage to ∞ -/

/-- The frontier the whole list can ever reach: the running max of
segment ends above a base — proof material for `ray_needs_ray`. -/
def endsSup (a : α) : List (Interval α) → α
  | [] => a
  | iv :: rest => endsSup (maxE a iv.«end») rest

omit [DecidableLT α] in
theorem le_endsSup :
    ∀ (l : List (Interval α)) (a : α), a ≤ endsSup a l
  | [], a => LinearElem.le_refl a
  | iv :: rest, a =>
    LinearElem.le_trans (le_maxE_left a iv.«end») (le_endsSup rest _)

omit [DecidableLT α] in
theorem end_le_endsSup :
    ∀ (l : List (Interval α)) (a : α) {jv : Interval α}, jv ∈ l →
      jv.«end» ≤ endsSup a l
  | iv :: rest, a, jv, hjv => by
    rcases List.mem_cons.mp hjv with rfl | hmem
    · exact LinearElem.le_trans (le_maxE_right a jv.«end»)
        (le_endsSup rest _)
    · exact end_le_endsSup rest _ hmem

omit [LinearElem α] [DecidableLT α] in
theorem endsSup_lt {c : α} :
    ∀ (l : List (Interval α)) (a : α), a < c →
      (∀ iv ∈ l, iv.«end» < c) → endsSup a l < c
  | [], _, ha, _ => ha
  | iv :: rest, a, ha, hall => by
    refine endsSup_lt rest _ ?_
      fun jv hjv => hall jv (List.mem_cons_of_mem _ hjv)
    rcases maxE_eq_or a iv.«end» with hm | hm
    · rw [hm]; exact ha
    · rw [hm]; exact hall iv (List.mem_cons_self ..)

/-- The `u64` ceiling really is the greatest element — discharges
`ray_needs_ray`'s explicit ceiling premise for the real domain
(recorded narrowing: the fact lives here, not in `PointDomain`). -/
theorem ceiling_greatest_u64 (a : U64) :
    a ≤ (PointDomain.maxEnd : U64) := by
  show a.val ≤ 2 ^ 64 - 1
  have := a.property
  omega

/-- The `i64` companion of `ceiling_greatest_u64`. -/
theorem ceiling_greatest_i64 (a : I64) :
    a ≤ (PointDomain.maxEnd : I64) := by
  show a.val ≤ 2 ^ 63 - 1
  have := a.property
  omega

/-- **Theorem 5 (`ray_needs_ray`).** A source ray is covered only if
the segment list reaches a ray, stated contrapositively: a rayless
list is REJECTED (the frontier tops out at the finite `endsSup`,
strictly below ∞, and that point convicts by soundness). ∞ is the
largest end word, no special case — the "coverage to ∞" doc claim as
a lemma. The ceiling premise `hceil` is explicit (module doc;
discharged by `ceiling_greatest_u64` / `ceiling_greatest_i64`).
Bridge: `judgment.rs::check_coverage` ("a source ray demands coverage
to ∞ — satisfiable only by a chain reaching a target ray");
`rays_are_ordinary_largest_end_words` in `interval/sweep.rs`. -/
theorem ray_needs_ray [PointDomain α] (src : Interval α)
    (segs : List (Interval α))
    (hceil : ∀ a : α, a ≤ (PointDomain.maxEnd : α))
    (hray : src.isRay) (hnoray : ∀ iv ∈ segs, ¬ iv.isRay) :
    sweepCovered src segs = false := by
  cases hc : sweepCovered src segs with
  | false => rfl
  | true =>
    exfalso
    have hends : ∀ iv ∈ segs, iv.«end» < PointDomain.maxEnd := by
      intro iv hiv
      rcases (LinearElem.le_iff iv.«end» PointDomain.maxEnd).mp
        (hceil iv.«end») with hlt | heq
      · exact hlt
      · exact absurd heq (hnoray iv hiv)
    have hsm : src.start < (PointDomain.maxEnd : α) := hray ▸ src.h
    have hx1 : src.start ≤ endsSup src.start segs :=
      le_endsSup segs src.start
    have hx2 : endsSup src.start segs < (PointDomain.maxEnd : α) :=
      endsSup_lt segs src.start hsm hends
    have hc' : sweepFrom src.«end» src.start segs = true := hc
    obtain ⟨iv, hiv, hxmem⟩ :=
      sweep_never_false_accepts hc' (endsSup src.start segs) hx1
        (hray.symm ▸ hx2)
    have hxmem' : iv.start ≤ endsSup src.start segs ∧
        endsSup src.start segs < iv.«end» := hxmem
    exact absurd
      (LinearElem.lt_of_le_of_lt (end_le_endsSup segs src.start hiv)
        hxmem'.2)
      (LinearElem.lt_irrefl _)

/-! ## Theorem 6 — the half-open seam -/

/-- **Theorem 6 (`adjacent_segments_cover`).** Touching segments
cover across the seam: `a.«end» = b.start` shares no point yet leaves
no hole, so the walk accepts the composed window
`[a.start, b.«end»)` — half-open composition, the one adjacency law
("`start == frontier` continues a run", `interval/sweep.rs`, its home
and nowhere else). The denotational half is PRD 05's
`pack_adjacency`; this is the verdict half. Bridge:
`adjacency_continues_and_the_minimal_gap_breaks` in
`interval/sweep.rs`; the checker accepting a chain of exact tiles. -/
theorem adjacent_segments_cover (a b : Interval α)
    (hadj : a.«end» = b.start) :
    sweepCovered
      ⟨a.start, b.«end», LinearElem.lt_trans a.h (hadj.symm ▸ b.h)⟩
      [a, b] = true := by
  have hae : a.«end» < b.«end» := hadj.symm ▸ b.h
  have hab : a.start < b.«end» := LinearElem.lt_trans a.h hae
  have h1 : ¬ b.«end» ≤ a.start :=
    fun h => LinearElem.lt_irrefl _ (LinearElem.lt_of_lt_of_le hab h)
  have h2 : ¬ a.start < a.start := LinearElem.lt_irrefl _
  have hm1 : maxE a.start a.«end» = a.«end» := by
    unfold maxE
    rw [if_pos (LinearElem.le_of_lt a.h)]
  have h3 : ¬ b.«end» ≤ a.«end» :=
    fun h => LinearElem.lt_irrefl _ (LinearElem.lt_of_lt_of_le hae h)
  have h4 : ¬ a.«end» < b.start :=
    fun h => LinearElem.lt_irrefl b.start (hadj ▸ h)
  have hm2 : maxE a.«end» b.«end» = b.«end» := by
    unfold maxE
    rw [if_pos (LinearElem.le_of_lt hae)]
  show sweepFrom b.«end» a.start [a, b] = true
  unfold sweepFrom
  rw [if_neg h1, if_neg h2, hm1]
  unfold sweepFrom
  rw [if_neg h3, if_neg h4, hm2]
  exact decide_eq_true (LinearElem.le_refl b.«end»)

end Exec

end Bumbledb
