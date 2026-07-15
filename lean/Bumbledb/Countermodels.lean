import Bumbledb.Query.Aggregates
import Bumbledb.Exec.Sweep
import Bumbledb.Exec.Dedup
import Bumbledb.Exec.Rewrites
import Bumbledb.Exec.Fixpoint
import Bumbledb.Exec.Plan
import Bumbledb.Txn
import Bumbledb.Txn.DeltaRestriction
import Bumbledb.Admission

/-!
# Countermodels — the design scratchpad (PRD 02 onward, grows all campaign)

Anything refused or bounded gets its countermodel here, ported or
new — countermodel-first is a covenant law, and the scratchpad is
part of the spec.

## PRD 02 residents

* `empty_interval_vacuous` — over a RAW bounds pair, because the
  in-tree `Interval` cannot be empty: it carries `h : start < «end»`
  as a field, and that unrepresentability is the POINT. The raw shape
  is what the artifact's `empty_nat_interval_has_no_points` warned
  about: an empty point set satisfies ANY coverage obligation
  vacuously, so an engine that admitted raw bounds would let a fact
  denote nothing and every dependency judgment over it hold for free.
  `crate::Interval::new` returning `Option` (parse, don't validate)
  is the mechanism that keeps this countermodel outside the tree —
  in-tree, `Bumbledb.interval_nonempty` is a theorem.

* The str-order refusal — `StrId` is an opaque intern id with
  decidable equality ONLY. No `LT`/`LE`/`Ord` instance exists for the
  intern domain: an intern id is a per-database allocation accident,
  so an order on ids would order the interning history, not the
  values. This is a deliberate absence (a typing fact), machine-
  checked below with `#check_failure`: the instance searches FAIL,
  and the build breaks if anyone ever adds the instances.

## PRD 03 residents

* `bare_eq_not_unique` — the two-row countermodel (port): bare `==`
  (mutual containment) holds between a one-fact source and a two-fact
  target sharing one projected key value, while the target projection
  is NOT a key. Bare projected view equality is not unique
  correspondence; the key premises of `KeyBackedEquality` are load-
  bearing, which is why each `==` direction must independently pass
  `resolve_target_key` (the ==-reverse-key locks).

## PRD 04 residents

* `unsafe_rule_infinite` — the unsafe rule whose "denotation" is
  INFINITE: one head variable bound by no positive atom, one
  nonemptiness-gate atom, a one-fact instance — and the answer set
  contains one tuple per intern id, so no list can enumerate it.
  This is what `Safe` (positive range restriction) refuses, and why
  `antijoin_over_active_domain` carries `Safe` as a hypothesis:
  negation and projection are only meaningful over the active domain,
  never over the infinite complement. The validator's mechanisms:
  `NegatedVariableUnbound`, `ComparisonOnlyVariable`, and the
  find-side binding check.

## PRD 05 resident

* `sql_zero_row_from_no_binding` — the refused SQL reading of the
  empty global aggregate (the seed artifact's `sum [] = 0`): a model
  that ALWAYS emits one row manufactures, over a rule that derives
  nothing, an answer with NO deriving witness — an answer from no
  binding, which the engine-faithful `Query.aggAnswers` cannot
  express (it demands the witness; `Query.empty_global_no_answer`).
  The artifact-divergence note in `Query/Aggregates.lean` records
  why the engine's contract is the authority.

## PRD 04 resident (continued)

* `one_way_overhang` — the [0,10)/[0,20) overshoot (port): one-way
  coverage of a [0,10) source by a [0,20) target HOLDS (with the
  target vacuously pointwise-keyed — a disjoint cover), while exact
  partition FAILS at point 15, which the target covers outside the
  source's support. The tiling over-read's killer: coverage is
  support INCLUSION (`coverage_is_support_inclusion`), never
  equality; exact partition needs both directions
  (`exact_partition_iff`), which is exactly recipe 26's five-statement
  construction and its commit matrix's one-way-overhang-accepted row.

## PRD 06 resident

* `sweep_premise_load_bearing` — the sweep's REQUIRED premise
  countermodel: an unordered segment list that jointly covers its
  source window while the one-pass walk convicts — the false REJECT,
  the "wrong verdict without erroring". Two recorded boundaries: the
  false-ACCEPT direction is NOT constructible
  (`Exec.sweep_never_false_accepts` is premise-free), and violating
  `Disjoint` alone cannot produce a wrong verdict
  (`Exec.sweep_complete_of_ordered` spends only `Ordered`) — see the
  section note and `Exec/Sweep.lean`'s module doc.

## PRD 09 residents

* `per_op_judgment_wrong` — the FinalStateView seam's formal
  justification: a two-deletion transaction (parent and child of a
  containment) whose FINAL state holds, whose two op orders reach the
  SAME final state, and whose parent-first order transiently violates
  mid-sequence. A per-operation judge would reject one order of a
  valid transaction — which is why judgment reads one final state
  (`Txn.judge`'s signature; `Txn.final_state_judgment_order_free`)
  and why `judgment.rs::FinalStateView` is a type, not a discipline.

* `incremental_verdict_needs_holds` — the delta-restricted judgment's
  load-bearing premise (`Txn/DeltaRestriction.lean`): WITHOUT
  `holds(pre)`, the restricted verdict accepts a violating final
  state — a pre-existing key violation in an untouched binding
  survives an empty delta whose touched set is empty, so every
  restricted check passes vacuously while the final state does not
  hold. Inside the lifecycle the premise is free (`State.models`);
  outside it, this countermodel is exactly why `Db::verify_store`
  exists — the sweeper re-runs both judgment forms globally, owning
  the class no incremental check can see ("an incremental form wrong
  once, long ago, preserved by every commit since" —
  `docs/architecture/60-validation.md` § the store sweeper, the
  division of authority).

* `stale_but_sound` — the maintenance protocol's freshness gap: a
  committed state (it `holds` its theory) whose derived relation is
  SOUND (its containment backs every derived fact — vacuously, here)
  yet STALE: the parent fact's derived copy never landed. No
  dependency statement can demand catch-up, so freshness is not a
  property of any committed state — it is host discipline (the
  `write_from` witness loop), exactly
  `Txn.derived_soundness_vs_freshness`'s other half.

## PRD 07 resident

* `distinct_premise_load_bearing` — the unkeyed double-count: one
  positive occurrence whose bound fields cover NO key, two DISTINCT
  facts (same bound amount, different unbound payload) collapsing to
  ONE full binding, and a `Sum` that double-counts under seen-set
  elision — 200 where the distinct binding set sums 100. The
  bag-semantics accident `DistinctWitness` forecloses, made concrete:
  `Query.distinct_witness_licence`'s premise
  (`Query.BoundFieldsCoverKey`) cannot be dropped, which is why
  `provably_distinct` is the only mint of the witness and
  `AggregateSink::without_seen_set` demands it by value.

## PRD 08 residents

* `elimination_needs_containment` — dropping a containment-backed
  atom WITHOUT the containment premise changes answers: a two-atom
  rule in full `Query.ElimStep` shape (every syntactic elimination
  condition holds) over an instance where the source fact has no
  target witness — the survivor answers, the original does not. Why
  elimination consults the THEORY (`plan/ground.rs::removable` scans
  `schema.containments()`), never just the shapes: the shape conditions
  are checkable at prepare, the existence guarantee is the statement's
  alone.

* `latch_miss_not_static` — the latch's two constructors are not
  interchangeable: a rule empty at one instance through a selection
  miss (`Query.EmptyAt.selectionMiss` — the `PendingIntern` dictionary
  miss, `Ok(false)`) ANSWERS at another instance, so the miss verdict
  can never be promoted to the plan-level `Program::Empty` — which is
  exactly the design decision `api/prepared/bind.rs`'s latch encodes
  (the miss short-circuits one execution; only the fold's refutation
  deletes the rule).

## Spec-fidelity F3 residents (the FieldSet split locks)

* `split_permuted_some` / `split_two_intervals_none` /
  `split_all_scalar_none` — the shape locks of the set-canonical
  `Header.intervalSplit` (the F3 fix): a written-order reading gave
  `[interval, scalar]` the classical judgment where the engine
  deliberately canonicalizes on the field SET (`resolve_target_key`
  counts interval positions as a set; `key_permutation` bridges
  statement order to key order), and it gave `[interval, interval]`
  a pointwise reading with an interval inside the "scalar" prefix.
  The three concrete headers pin the corrected split: one interval
  field splits to `some` at ANY written position, two intervals and
  all-scalar split to `none`.

## Extension residents (cardinality windows, E3)

* `unit_window_two_children` — the `1..1` window refuted by one
  parent with two distinct children (the bare-`==` rows, reread):
  the upper bound is load-bearing, not decorative — a window is never
  just its floor (`window_floor_containment` is the floor's half).

* `disjunctive_window_not_literal_conjunction` — why E3's literal
  sets are FIRST-CLASS, not per-literal sugar: the `1..1` window over
  `payload ∈ {true, false}` accepts each one-child relation and
  rejects their union, while ANY conjunction of per-literal windows
  accepting both one-child relations accepts the union too
  (`selTrue_group_union` / `selFalse_group_union` — each literal's
  child group transfers whole). Counts over a union do not decompose;
  the admitted count-vectors of a union window are not a product set.

* `joined_window_blast` — the E1 shape (a window over a joined pair
  of atoms) has NO oracle-bounded enforcement plan, and the refusal
  is BY REPRESENTATION (the admission-calculus resident; the
  countermodel section below).

* `joined_window_form_uninhabitable` — the blast composed against
  the acceptance gate's type (`Admission.lean: AdmissibleForm`): the
  E1 shape at its own grouping discipline has NO oracle-plan field —
  two runs whose touched consultations agree while the judgment
  differs, so "prohibitively expensive" is a type error (the section
  at the end of this file).

## The Free Join wrong-cover resident (the plan formalism)

* `loose_cover_rebinds` — the paper's looser cover rule ("containing
  all new variables", Free Join §3.2), refuted on the triangle query:
  a plan the paper's definition accepts (`loose_plan_paper_valid`)
  and bumbledb's exactly-new-variables rule refuses
  (`loose_plan_not_valid`) whose loose execution REBINDS an
  already-bound variable from the cover's facts without re-checking
  the occurrence that bound it, emitting a tuple outside the rule's
  denotation. This is `docs/architecture/40-execution.md` § the
  paper's core — the audit-found deviation paragraph — mechanized;
  until now it was prose plus a Rust regression test. The valid-plan
  side is `Exec/Plan.lean: valid_plan_sound`.
-/

namespace Bumbledb.Countermodels

/-! ## The empty-interval countermodel (raw bounds pair) -/

/-- A RAW bounds pair — the shape the in-tree `Interval` refuses to
be: no `h : start < «end»` field, so `start ≥ «end»` (an empty point
set) is representable. -/
structure RawInterval where
  start : Nat
  «end» : Nat

/-- The same half-open reading as `Interval.points`, over the raw
pair. -/
def RawInterval.points (iv : RawInterval) : Set Nat :=
  fun x => iv.start ≤ x ∧ x < iv.«end»

/-- The reversed raw pair `⟨10, 5⟩` denotes the empty set — the
artifact's `empty_nat_interval_has_no_points` shape, restated against
the in-tree `points` reading. -/
theorem raw_interval_no_points :
    ∀ x : Nat, x ∉ (RawInterval.mk 10 5).points := by
  intro x hx
  obtain ⟨hlo, hhi⟩ := hx
  exact absurd (Nat.lt_of_le_of_lt hlo hhi) (by decide)

/-- **The countermodel.** An empty point set satisfies ANY pointwise
coverage obligation vacuously: were empty intervals representable,
every dependency judgment quantifying over an interval's points would
hold for free on them. Unrepresentable in-tree — `Interval` carries
`h : start < «end»`, which is the point (`Bumbledb.interval_nonempty`
is the in-tree theorem; `crate::Interval::new` is the mechanism). -/
theorem empty_interval_vacuous (P : Nat → Prop) :
    ∀ x ∈ (RawInterval.mk 10 5).points, P x := by
  intro x hx
  exact absurd hx (raw_interval_no_points x)

/-! ## The str-order deliberate absence, machine-checked

`#check_failure` succeeds exactly when elaboration fails: each line
below is a build-breaking guard that no order instance ever appears
on the intern domain (the `#guard_msgs (drop info)` wrapper only
silences the expected failure-to-synthesize report). Equality stays
decidable — that instance resolves. -/

example : DecidableEq StrId := inferInstance

#guard_msgs (drop info) in
#check_failure (inferInstance : LT StrId)

#guard_msgs (drop info) in
#check_failure (inferInstance : LE StrId)

#guard_msgs (drop info) in
#check_failure (inferInstance : Ord StrId)

/-! ## The bare-`==` countermodel (PRD 03)

A one-fact source and a two-fact target: field 0 carries the shared
key value in every fact, field 1 the payload that makes the two
target facts distinct. -/

/-- The shared projected key value. -/
def keyVal : Value := ⟨.u64, ⟨0, by omega⟩⟩

/-- The Bool observer that discriminates the two target payloads. -/
def Value.asBool : Value → Bool
  | { type := .bool, val := b } => b
  | _ => false

/-- The source fact (and first target fact): payload `true`. -/
def rowTrue : Fact := fun i => if i.id = 1 then ⟨.bool, true⟩ else keyVal

/-- The second target fact: payload `false`, same projected key. -/
def rowFalse : Fact := fun i => if i.id = 1 then ⟨.bool, false⟩ else keyVal

/-- The one-fact source. -/
def oneSource : Set Fact := fun f => f = rowTrue

/-- The two-fact target, both rows sharing the projected key. -/
def twoTarget : Set Fact := fun f => f = rowTrue ∨ f = rowFalse

/-- The projection both sides use: field 0, the shared key. -/
def keyProj : List FieldId := [⟨0⟩]

/-- The two rows are distinct facts — the payload observer refuses.
Shared by the bare-`==`, unit-window and disjunctive-window
countermodels. -/
theorem rowTrue_ne_rowFalse : rowTrue ≠ rowFalse := fun heq => by
  have hb : (true : Bool) = false :=
    congrArg (fun f : Fact => Value.asBool (f ⟨1⟩)) heq
  cases hb

/-- **The countermodel (port).** Bare `==` holds — the views are both
`{[keyVal]}` — while the target projection is NOT a key: `rowTrue`
and `rowFalse` agree on it and differ. Unique correspondence needs
the `KeyBackedEquality` premises; `keyed_eq_unique_correspondence`
is exactly what this model refutes without them. Bridge: why each
lowered `==` direction independently passes `resolve_target_key`. -/
theorem bare_eq_not_unique :
    ContainsEq oneSource Selection.empty keyProj
      twoTarget Selection.empty keyProj ∧
    ¬ Functionality twoTarget keyProj := by
  refine ⟨⟨?_, ?_⟩, ?_⟩
  · intro f hf _
    have hf' : f = rowTrue := hf
    exact ⟨rowTrue, Or.inl rfl, Selection.empty_satisfies _,
      by rw [hf']⟩
  · intro g hg _
    have hg' : g = rowTrue ∨ g = rowFalse := hg
    refine ⟨rowTrue, rfl, Selection.empty_satisfies _, ?_⟩
    cases hg' with
    | inl h => rw [h]
    | inr h => rw [h]; rfl
  · intro h
    have heq : rowTrue = rowFalse :=
      h rowTrue rowFalse (Or.inl rfl) (Or.inr rfl) rfl
    have hb : (true : Bool) = false :=
      congrArg (fun f : Fact => Value.asBool (f ⟨1⟩)) heq
    cases hb

/-! ## The one-way-overhang countermodel (PRD 03)

The [0,10)/[0,20) overshoot, ported onto the in-tree `Interval U64`
(nonempty by construction — the raw-pair vacuity above is exactly
what these facts CANNOT exhibit). One scalar group (empty prefix),
interval position at field 0. -/

/-- The source span: `[0, 10)`. -/
def domSpan : Value :=
  ⟨.interval .u64, ⟨⟨0, by omega⟩, ⟨10, by omega⟩, by decide⟩⟩

/-- The overshooting target span: `[0, 20)`. -/
def tileSpan : Value :=
  ⟨.interval .u64, ⟨⟨0, by omega⟩, ⟨20, by omega⟩, by decide⟩⟩

/-- The one-fact source relation. -/
def domFact : Fact := fun _ => domSpan
/-- The one-fact overshooting target relation. -/
def tileFact : Fact := fun _ => tileSpan

def domRel : Set Fact := fun f => f = domFact
def tileRel : Set Fact := fun f => f = tileFact

/-- The single overshooting tile is pointwise-keyed vacuously: no two
distinct selected target facts exist — the "disjoint cover" half of
the port's `IsTilingOf`. -/
theorem overhang_tile_pointwise_key :
    PointwiseKey (Selected tileRel Selection.empty) [] ⟨0⟩ :=
  fun f g hf hg _ hne _ _ _ =>
    hne ((show f = tileFact from hf.1).trans
      (show g = tileFact from hg.1).symm)

/-- **The countermodel (port).** One-way coverage of `[0, 10)` by
`[0, 20)` HOLDS — target overhang is legal, coverage is support
INCLUSION only (`coverage_is_support_inclusion`) — while exact
partition FAILS: the tile covers point 15 outside the source's
support. The tiling over-read's killer, now in-tree. Bridge:
`Checker::check_coverage` walks only the demanded source interval;
recipe 26's commit matrix locks the one-way-overhang-accepted and
reverse-overhang-rejected rows (`r26_exact_partition_commit_matrix`). -/
theorem one_way_overhang :
    Coverage domRel Selection.empty [] ⟨0⟩
      tileRel Selection.empty [] ⟨0⟩ ∧
    ¬ ExactPartition domRel Selection.empty [] ⟨0⟩
      tileRel Selection.empty [] ⟨0⟩ := by
  constructor
  · intro f hf _ x hx
    have hf' : f = domFact := hf
    subst hf'
    refine ⟨tileFact, rfl, Selection.empty_satisfies _, rfl, ?_⟩
    cases x with
    | u64 y =>
      have h1 : (0 : Nat) ≤ y.val := hx.1
      have h2 : y.val < 10 := hx.2
      exact ⟨h1, show y.val < 20 by omega⟩
    | i64 y => exact False.elim hx
  · intro hex
    have htile : (Point.u64 ⟨15, by omega⟩) ∈
        Support tileRel Selection.empty [] ⟨0⟩ [] :=
      ⟨tileFact, rfl, Selection.empty_satisfies _, rfl,
        by decide, by decide⟩
    have hdom := (hex.2 [] (Point.u64 ⟨15, by omega⟩)).mpr htile
    obtain ⟨f, hf, -, -, hx⟩ := mem_support.mp hdom
    have hf' : f = domFact := hf
    subst hf'
    exact absurd hx.2 (by decide)

/-! ## The Q2 ray-exclusion (Tier-2 literal types)

Why `interval<E, w>`'s carrier bound is STRICT (`start + w < maxEnd`,
never `≤`): the ceiling is not a point — `«end» = maxEnd` denotes the
unbounded ray `[start, ∞)` (`ray_is_unbounded_tail`) — so a "unit
slot" written at the ceiling would not be one point wide; it would be
an unbounded tail with NO measure, refuting the fixed family's
constant-measure theorem (`fixed_measure_const_u64`). The strict
bound makes that value UNCONSTRUCTIBLE in the fixed carrier: rays
stay exclusive to the general type, by construction rather than by
check.

**The Tier-3 refusal, recorded here with its boundary.** The
admission rule (`docs/architecture/10-data-model.md`) admits a type
parameter iff it CHANGES THE ENCODING — `w` does (one word instead
of two). A parameter that merely CHECKS values — a refinement
`u64 where v < 100`, a CHECK constraint in type costume — changes no
byte of the encoding, so it is not a type: two "types" differing
only in an admitted predicate would carry identical encodings, and
"types are encodings" would be false. Predicate parameters stay
refused (Tier 3); the constraint vocabulary is the statement layer's,
and a per-value check that is not a dependency is a value the host
refuses to construct. -/

/-- The GENERAL slot written against the ceiling, `[maxEnd − 1,
maxEnd)` — representable in the general type, where it is honestly a
RAY. -/
def ceilingSlot : Interval U64 :=
  ⟨⟨2 ^ 64 - 2, by omega⟩, U64.maxEnd, by
    show (2 ^ 64 - 2 : Nat) < U64.maxEnd.val
    have h : U64.maxEnd.val = 2 ^ 64 - 1 := rfl
    omega⟩

/-- **The countermodel's first half:** the ceiling slot is a ray with
no measure — one "point" of spelling, an unbounded tail of
denotation. -/
theorem ceiling_slot_denotes_an_unbounded_tail :
    ceilingSlot.isRay ∧ ceilingSlot.measure = none :=
  ⟨rfl, measure_ray_none _ rfl⟩

/-- **The countermodel's second half (Q2, spent):** no `interval<u64, 1>`
value starts at `maxEnd − 1` — the strict carrier bound makes the
ceiling slot unconstructible in the fixed family, so every
fixed-width value keeps the constant measure `w`.
Bridge: `crate::Interval::fixed` returns `None` there. -/
theorem unit_slot_at_ceiling_unconstructible :
    ¬ ∃ v : FixedU64 1, v.val.val = U64.maxEnd.val - 1 := by
  rintro ⟨v, hv⟩
  have hb := v.property
  have h : U64.maxEnd.val = 2 ^ 64 - 1 := rfl
  omega

/-! ## The unsafe-rule countermodel (PRD 04)

One rule: `finds [v₀]`, one zero-binding gate atom, nothing else. The
head variable is bound by NO positive atom — the rule is unsafe — and
over a one-fact instance its answer set holds one tuple per intern id:
an infinite family no list enumerates. -/

/-- The gate fact: any single fact will do. -/
def gateFact : Fact := fun _ => ⟨.bool, false⟩

/-- A one-fact instance: every relation holds exactly the gate fact
(only the gate atom's relation is ever read). -/
def gateInstance : Instance := fun _ => fun f => f = gateFact

/-- The unsafe rule: project `v₀`, gate on a relation, bind nothing —
`v₀ ∈ allVars` (a find) but `positiveVars = []`. -/
def unsafeRule : Query.Rule where
  finds := [⟨0⟩]
  atoms := [{ relation := ⟨0⟩, bindings := [] }]
  negated := []
  conditions := []

/-- The rule is UNSAFE: its head variable has no positive binding —
exactly what the validator's find-side binding check refuses. -/
theorem unsafe_rule_not_safe : ¬ Query.Safe unsafeRule :=
  Query.membership_only_unsafe
    (Query.mem_allVars.mpr (Or.inl (List.mem_singleton.mpr rfl)))
    (fun h => by
      rcases Query.mem_positiveVars.mp h with ⟨a, ha, hv⟩
      rcases List.mem_singleton.mp ha with rfl
      simp [Query.Atom.boundVars] at hv)

/-- One answer per intern id: the unconstrained head variable takes
EVERY value. -/
theorem unsafe_rule_answers (C : Query.Classify) (ρ : Query.ParamEnv)
    (n : Nat) :
    [(⟨.str, ⟨n⟩⟩ : Value)] ∈
      Query.ruleAnswers C unsafeRule gateInstance ρ := by
  refine Query.mem_ruleAnswers.mpr
    ⟨fun _ => ⟨.str, ⟨n⟩⟩, ⟨?_, ?_, ?_⟩, rfl⟩
  · intro a ha
    rcases List.mem_singleton.mp ha with rfl
    exact ⟨gateFact, rfl, fun b hb => by cases hb⟩
  · intro a ha
    cases ha
  · intro t ht
    cases ht

/-- The head intern id of a singleton str answer — the observer the
infinitude argument counts with. -/
def headStrId : List Value → Option Nat
  | [{ type := .str, val := s }] => some s.id
  | _ => none

/-- Every member of a `Nat` list is bounded by its `foldr max`. -/
theorem le_foldr_max : ∀ (l : List Nat) (n : Nat), n ∈ l →
    n ≤ l.foldr Nat.max 0
  | a :: l, n, h => by
    rcases List.mem_cons.mp h with rfl | h
    · exact Nat.le_max_left _ _
    · exact Nat.le_trans (le_foldr_max l n h) (Nat.le_max_right _ _)

/-- **The countermodel.** The unsafe rule's "denotation" is INFINITE:
no list enumerates its answer set — any candidate list misses the
intern id one past its maximum. This is the theorem-shaped reason
`Safe` exists and is a HYPOTHESIS of `antijoin_over_active_domain`
and `eval_sound`: without positive range restriction there is no
active domain to evaluate over, and the anti-join's complement
reading would be this infinity. Bridge:
`ValidationError::NegatedVariableUnbound` /
`ComparisonOnlyVariable` / `MembershipOnlyVariable` — the acceptance
boundary that keeps this rule unwritable downstream. -/
theorem unsafe_rule_infinite (C : Query.Classify) (ρ : Query.ParamEnv) :
    ¬ (Query.ruleAnswers C unsafeRule gateInstance ρ).Finite := by
  rintro ⟨l, hl⟩
  have hmem := unsafe_rule_answers C ρ ((l.filterMap headStrId).foldr Nat.max 0 + 1)
  have hinl := (hl _).mp hmem
  have hid : (l.filterMap headStrId).foldr Nat.max 0 + 1 ∈
      l.filterMap headStrId :=
    List.mem_filterMap.mpr ⟨_, hinl, rfl⟩
  exact Nat.not_succ_le_self _ (le_foldr_max _ _ hid)

/-! ## The SQL zero-row countermodel (PRD 05)

The artifact-divergence's refused reading, as a model: a global
aggregate that ALWAYS emits one row — folding the possibly-empty
binding set, SQL's ungrouped-aggregate behavior (`SUM` of nothing is
`0`). Over an instance where the rule derives NOTHING, it
manufactures an answer with no deriving witness. -/

/-- The refused reading: one row, always — the fold of the (possibly
empty) binding set. -/
def sqlGlobalAgg (C : Query.Classify) (r : Query.Rule) (I : Instance)
    (ρ : Query.ParamEnv) (fold : Set Query.Assignment → Value) :
    Set Query.AnswerTuple :=
  fun t => t = [fold (Query.bindingSet C r I ρ)]

/-- The empty instance: no facts anywhere. -/
def emptyInstance : Instance := fun _ => fun _ => False

/-- A rule that derives nothing over the empty instance: its one
positive atom demands a fact, and there are none. -/
def gateRule : Query.Rule where
  finds := []
  atoms := [{ relation := ⟨0⟩, bindings := [] }]
  negated := []
  conditions := []

theorem gateRule_derives_nothing (C : Query.Classify)
    (ρ : Query.ParamEnv) :
    ∀ σ, ¬ Query.derives C gateRule emptyInstance ρ σ := by
  rintro σ ⟨hatoms, -, -⟩
  obtain ⟨f, hf, -⟩ := hatoms _ (List.mem_singleton.mpr rfl)
  exact hf

/-- **The countermodel.** The SQL zero-row reading manufactures an
answer over the EMPTY binding set — a row with no deriving witness —
while the engine-faithful `Query.aggAnswers` is empty
(`Query.empty_global_no_answer`): an answer must trace to a binding,
and the artifact's `sum [] = 0` is refused. Bridge:
`exec/sink/aggregate/finalize.rs` ("Empty input yields zero rows");
the SQL-divergence oracle rule in `60-validation.md`. -/
theorem sql_zero_row_from_no_binding (C : Query.Classify)
    (ρ : Query.ParamEnv) (fold : Set Query.Assignment → Value)
    (keys : List Query.KeyTerm)
    (foldRow : List (Option Value) → Set Query.Assignment →
      Query.AnswerTuple) :
    ([fold (Query.bindingSet C gateRule emptyInstance ρ)] ∈
      sqlGlobalAgg C gateRule emptyInstance ρ fold) ∧
    (∀ t, t ∉ Query.aggAnswers C gateRule emptyInstance ρ keys foldRow) :=
  ⟨rfl, Query.empty_global_no_answer (gateRule_derives_nothing C ρ)⟩

/-! ## The sweep-premise countermodel (PRD 06)

The REQUIRED premise countermodel of `Exec/Sweep.lean`: the one-pass
coverage walk returns a WRONG VERDICT — without erroring — the moment
its `Ordered` premise is violated. The claims `[5, 9), [1, 5)` (start
order broken) jointly cover the source window `[1, 9)`, yet the walk
opens its frontier at 1, meets start 5 first, reads a gap, and
convicts: a FALSE REJECT.

Two recorded boundaries of the countermodel (the design findings of
PRD 06, `Exec/Sweep.lean` module doc):

* **The false-ACCEPT direction is NOT constructible.** The PRD asked
  for both directions "if constructible";
  `Exec.sweep_never_false_accepts` proves acceptance sound with NO
  premises at all — the frontier only ever advances across points a
  consumed segment holds — so a violated premise can only convict the
  innocent, never acquit the guilty. The checker's failure mode off
  its witness is spurious `CommitRejected`, never a silently accepted
  violation.
* **Violating `Disjoint` alone cannot produce a wrong verdict.**
  Completeness needs only `Ordered` (`Exec.sweep_complete_of_ordered`)
  — max-frontier tracking subsumes overlap, exactly the Rust module's
  claim. `Disjoint` licences the predecessor-seek entry below the
  fold's altitude (`judgment.rs::check_coverage`), and it is what the
  verifier's `pointwise_overlap_is_found_by_the_ordered_walk` fixture
  guards: the ordered walk is also how a broken disjointness premise
  is DETECTED, so the witness must stay minted at key acceptance.

This is the audit's "wrong verdict without erroring" made concrete —
the theorem-shaped reason `check_coverage` demands the
`DisjointDeterminantProof` token (order + disjointness minted at
pointwise-key acceptance) before entering the walk. -/

/-- The later claim `[5, 9)` — listed FIRST: the order violation. -/
def segLate : Interval U64 := ⟨⟨5, by omega⟩, ⟨9, by omega⟩, by decide⟩

/-- The earlier claim `[1, 5)` — listed second. -/
def segEarly : Interval U64 := ⟨⟨1, by omega⟩, ⟨5, by omega⟩, by decide⟩

/-- The premise-violating segment list: start-sorted it is NOT. -/
def unorderedSegs : List (Interval U64) := [segLate, segEarly]

/-- The source window `[1, 9)` the two claims jointly cover. -/
def coveringSrc : Interval U64 := ⟨⟨1, by omega⟩, ⟨9, by omega⟩, by decide⟩

/-- **The countermodel (`sweep_premise_load_bearing`).** On the
unordered list the premise-free denotation HOLDS (the claims cover
the window) while the walk's verdict is `false` — the false reject,
kernel-evaluated. The `Ordered` premise of
`Exec.sweep_covered_sound_complete` is load-bearing; see the section
note for why this is the ONLY constructible wrong-verdict direction.
Bridge: `DisjointDeterminantProof` + `judgment.rs::check_coverage`;
the verifier's `pointwise_overlap_is_found_by_the_ordered_walk`
fixture. -/
theorem sweep_premise_load_bearing :
    ¬ Exec.Ordered unorderedSegs ∧
      (∀ x ∈ coveringSrc.points, x ∈ unionPoints unorderedSegs) ∧
      Exec.sweepCovered coveringSrc unorderedSegs = false := by
  refine ⟨?_, ?_, by decide⟩
  · intro h
    have h51 := (List.pairwise_cons.mp h).1 segEarly (List.mem_cons_self ..)
    exact absurd h51 (by decide)
  · intro x hx
    have hx' : coveringSrc.start ≤ x ∧ x < coveringSrc.«end» := hx
    by_cases h5 : x < segEarly.«end»
    · exact ⟨segEarly, List.mem_cons_of_mem _ (List.mem_cons_self ..),
        hx'.1, h5⟩
    · exact ⟨segLate, List.mem_cons_self ..,
        LinearElem.le_of_not_lt h5, hx'.2⟩

/-- The same claims start-sorted flip the verdict to the truth — the
sort (LMDB key order for the checker, `sort_unstable` for Pack) is
exactly what the premise buys. -/
example : Exec.sweepCovered coveringSrc [segEarly, segLate] = true := by
  decide

/-! ## The per-op-judgment countermodel (PRD 09)

One containment `child([0]) <= parent([0])` over an all-scalar
header, one linking fact in both relations, one transaction deleting
both. The shared theory also hosts `stale_but_sound` below. -/

/-- The parent relation. -/
def parentRel : RelId := ⟨0⟩
/-- The child (dependent, or derived) relation. -/
def childRel : RelId := ⟨1⟩
/-- The one linking fact, present in both relations. -/
def linkFact : Fact := fun _ => ⟨.bool, true⟩

/-- An all-scalar header: every projection splits to `none`. -/
def pcHeader : Header := ⟨fun _ => [.bool]⟩

/-- `child([0]) <= parent([0])` — the child needs its parent. -/
def pcStatement : Statement :=
  .containment ⟨childRel, [⟨0⟩], Selection.empty⟩
    ⟨parentRel, [⟨0⟩], Selection.empty⟩

/-- The one-statement theory (no closed relations). -/
def pcTheory : Theory := ⟨pcHeader, fun _ => none, [pcStatement]⟩

/-- The relations are distinct — the deletes touch different rows. -/
theorem child_ne_parent : childRel ≠ parentRel := by decide

/-- The starting instance: every relation holds exactly the linking
fact (only `parentRel` and `childRel` are ever judged). -/
def pcInst : Instance := fun _ => fun f => f = linkFact

/-- The parent-first deletion order. -/
def parentFirst : List Txn.Op :=
  [.delete parentRel linkFact, .delete childRel linkFact]

/-- The child-first deletion order. -/
def childFirst : List Txn.Op :=
  [.delete childRel linkFact, .delete parentRel linkFact]

/-- The two orders reach the SAME final state — deletion of distinct
rows is commutative set algebra. -/
theorem per_op_orders_agree :
    Txn.applyOps pcInst parentFirst = Txn.applyOps pcInst childFirst := by
  funext R g
  refine propext ⟨?_, ?_⟩
  · rintro ⟨⟨h1, h2⟩, h3⟩
    exact ⟨⟨h1, h3⟩, h2⟩
  · rintro ⟨⟨h1, h2⟩, h3⟩
    exact ⟨⟨h1, h3⟩, h2⟩

/-- The final state holds: both rows are gone, and the containment is
vacuous over the emptied child. -/
theorem per_op_final_holds :
    holds pcTheory (Txn.applyOps pcInst parentFirst) := by
  intro st hst
  cases List.mem_singleton.mp hst
  intro f hf _
  exact absurd ⟨rfl, hf.1.1⟩ hf.2

/-- Mid-sequence, parent-first, the state VIOLATES: the parent is gone
while the child survives — the transient orphan. -/
theorem per_op_mid_violates :
    ¬ holds pcTheory
      (Txn.applyOps pcInst [.delete parentRel linkFact]) := by
  intro h
  have hj := h pcStatement (List.mem_singleton.mpr rfl)
  obtain ⟨g, hg, -, -⟩ :=
    hj linkFact ⟨rfl, fun hpc => child_ne_parent hpc.1⟩
      (Selection.empty_satisfies _)
  exact hg.2 ⟨rfl, hg.1⟩

/-- **The countermodel (item 8).** A delta that is VALID as a final
state but transiently violates mid-sequence: deleting parent and child
holds either way as one final state — the two op orders agree — yet
the parent-first prefix violates the containment. A per-operation
judge would reject one op order of a valid transaction, and which
order the host writes is semantically arbitrary; that is why judgment
is final-state (`Txn.judge` takes ONE instance;
`Txn.final_state_judgment_order_free`) and why per-operation checking
is wrong, not merely slow. Bridge: `judgment.rs::FinalStateView`
("operation order is no longer representable here") — the
constitution's seam, formally justified. -/
theorem per_op_judgment_wrong :
    holds pcTheory (Txn.applyOps pcInst parentFirst) ∧
    Txn.applyOps pcInst parentFirst = Txn.applyOps pcInst childFirst ∧
    ¬ holds pcTheory
      (Txn.applyOps pcInst [.delete parentRel linkFact]) :=
  ⟨per_op_final_holds, per_op_orders_agree, per_op_mid_violates⟩

/-! ## The stale-but-sound countermodel (PRD 09)

The same theory, read as a maintenance pair: `childRel` a derived
relation the host maintains as a copy of `parentRel`, the containment
its soundness constraint. -/

/-- The stale committed state: the parent fact landed, its derived
copy never did. -/
def staleInst : Instance := fun R g => R = parentRel ∧ g = linkFact

/-- **The countermodel (item 6's other half).** A committed state with
a stale-but-sound derived relation: `staleInst` HOLDS the theory (the
derived relation's containment is vacuously sound — every derived fact
is backed, there being none), while the parent fact's derived copy is
missing — the state is stale against the host's derivation contract
`child = copy of parent`. `holds` is the whole of committedness
(`Txn.committed_states_model`), so no committed state can attest
freshness: soundness is the engine's judgment, freshness is host
discipline — the `write_from` witness loop, and the formal
host-discipline gap of constitution PRD 20's maintenance protocol. -/
theorem stale_but_sound :
    holds pcTheory staleInst ∧
    linkFact ∈ staleInst parentRel ∧ linkFact ∉ staleInst childRel := by
  refine ⟨?_, ⟨rfl, rfl⟩, fun h => child_ne_parent h.1⟩
  intro st hst
  cases List.mem_singleton.mp hst
  intro f hf _
  exact absurd hf.1 child_ne_parent

/-! ## The delta-restriction premise countermodel (wave 2)

`Txn/DeltaRestriction.lean`'s restriction theorems all assume the
PRE-state holds the statement. This is that premise's countermodel:
the two-row fixture (`rowTrue`/`rowFalse` — same key projection,
distinct facts) as a pre-instance under a one-key theory, judged by
the delta-restricted check against an EMPTY delta. The touched set is
empty, so every restricted check passes vacuously — and the final
state (the pre-state, unchanged) violates the key. -/

/-- The one-key theory's header: field 0 scalar `u64` (the shared
key), field 1 `bool` (the discriminating payload) — all-scalar, so
the FD reads classically. -/
def fdHeader : Header := ⟨fun _ => [.u64, .bool]⟩

/-- The one-key theory: `R(field0) -> R`, nothing else. -/
def fdTheory : Theory :=
  ⟨fdHeader, fun _ => none, [.functionality ⟨0⟩ keyProj]⟩

/-- The VIOLATING pre-instance: both rows stand, agreeing on the key
projection while distinct — no `Txn.State` can carry it, which is the
type-level half of this countermodel. -/
def violInstance : Instance := fun _ => twoTarget

/-- The empty delta: nothing added, nothing removed — no binding is
touched. -/
def emptyDelta : Txn.Delta :=
  ⟨fun _ => fun _ => False, fun _ => fun _ => False⟩

/-- **The countermodel (the load-bearing premise).** WITHOUT
`holds(pre)`, the delta-restricted verdict accepts a violating final
state: every statement's restricted check passes over the violating
pre-instance and the empty delta (the touched determinant set is
empty), while the final state does not hold — the pre-existing
violation in an untouched binding survives, unjudged. Inside the
lifecycle the premise is free (`Txn.State.models`); outside it, this
is exactly why `Db::verify_store` exists: the sweeper re-runs both
judgment forms globally over the full committed state, owning the
class no incremental check can see
(`docs/architecture/60-validation.md` § the store sweeper — the
division of authority the delta-restricted judgment implies). -/
theorem incremental_verdict_needs_holds :
    (∀ st, st ∈ fdTheory.statements →
      Txn.deltaCheck fdTheory violInstance emptyDelta st) ∧
    ¬ holds fdTheory (emptyDelta.applyTo violInstance) := by
  have hsplit : fdTheory.header.intervalSplit ⟨0⟩ keyProj = none := rfl
  constructor
  · intro st hst
    cases List.mem_singleton.mp hst
    simp only [Txn.deltaCheck, hsplit]
    intro f g hf hg htouch hproj
    obtain ⟨f', hf', -⟩ := Txn.mem_projected.mp htouch
    rcases hf' with h | h
    · exact False.elim h
    · exact False.elim h
  · intro h
    have hj := h _ (List.mem_singleton.mpr rfl)
    simp only [Statement.judgment, hsplit] at hj
    have hT : rowTrue ∈
        fdTheory.den (emptyDelta.applyTo violInstance) ⟨0⟩ :=
      Or.inl ⟨Or.inl rfl, fun hf => hf⟩
    have hF : rowFalse ∈
        fdTheory.den (emptyDelta.applyTo violInstance) ⟨0⟩ :=
      Or.inl ⟨Or.inr rfl, fun hf => hf⟩
    exact rowTrue_ne_rowFalse (hj rowTrue rowFalse hT hF rfl)

/-! ## The unkeyed double-count countermodel (PRD 07)

The `DistinctWitness` premise, load-bearing. One positive occurrence
binds only the amount field; the relation carries two facts agreeing
there and differing at the UNBOUND payload field — so the bound
fields cover no key, the two distinct facts produce ONE full binding
(`amount ↦ 100`), the elided stream repeats the key, and a `Sum`
folded without the seen-set answers 200 where the distinct binding
set sums 100. Contrast the doc example "two postings of amount 100 to
one account are two distinct bindings (their fresh ids differ)" —
that holds when the fresh id IS bound; here it is not, and the
seen-set is what keeps the collapse honest. -/

/-- The shared bound value: amount 100. -/
def amount : Value := ⟨.u64, ⟨100, by omega⟩⟩

/-- The first posting: amount at field 0, payload `true` at the
unbound field 1. -/
def postingA : Fact := fun i =>
  if i.id = 1 then ⟨.bool, true⟩ else amount

/-- The second posting: same amount, payload `false`. -/
def postingB : Fact := fun i =>
  if i.id = 1 then ⟨.bool, false⟩ else amount

/-- The two-fact relation. -/
def postingRel : Set Fact := fun f => f = postingA ∨ f = postingB

/-- The instance: every relation reads the posting pair (only the
occurrence's relation is ever consulted). -/
def postingInstance : Instance := fun _ => postingRel

/-- The unkeyed occurrence: only the amount field is bound. -/
def unkeyedAtom : Query.Atom :=
  { relation := ⟨0⟩, bindings := [(⟨0⟩, .var ⟨0⟩)] }

/-- The rule around it — the body a `Sum(amount)` head folds. -/
def unkeyedRule : Query.Rule where
  finds := [⟨0⟩]
  atoms := [unkeyedAtom]
  negated := []
  conditions := []

/-- The ONE binding both facts produce. -/
def dupAssign : Query.Assignment := fun _ => amount

/-- The two postings are distinct facts — they differ at the unbound
payload field. -/
theorem postingA_ne_postingB : postingA ≠ postingB := fun heq => by
  have hb : (true : Bool) = false :=
    congrArg (fun f : Fact => Value.asBool (f ⟨1⟩)) heq
  cases hb

/-- Both distinct facts are matched by the one binding — two fact
tuples, one full binding: exactly the duplicate the binding seen-set
exists to absorb. -/
theorem both_facts_one_binding (ρ : Query.ParamEnv) :
    Query.MatchSelection unkeyedRule postingInstance ρ dupAssign
      (fun _ => postingA) ∧
    Query.MatchSelection unkeyedRule postingInstance ρ dupAssign
      (fun _ => postingB) := by
  constructor <;> intro a ha <;> rcases List.mem_singleton.mp ha with rfl
  · refine ⟨Or.inl rfl, ?_⟩
    intro b hb
    rcases List.mem_singleton.mp hb with rfl
    rfl
  · refine ⟨Or.inr rfl, ?_⟩
    intro b hb
    rcases List.mem_singleton.mp hb with rfl
    rfl

/-- The premise FAILS: the occurrence's bound fields cover no key —
any covered field list lives at field 0, where the two distinct facts
agree, so no `Functionality` over it can hold. -/
theorem unkeyed_no_cover :
    ¬ Query.BoundFieldsCoverKey unkeyedRule postingInstance := by
  intro h
  obtain ⟨K, hkey, hpin⟩ := h unkeyedAtom (List.mem_singleton.mpr rfl)
  have hall : ∀ i, i ∈ K → postingA i = postingB i := by
    intro i hi
    obtain ⟨t, hb, -⟩ := hpin i hi
    have hfield : i = ⟨0⟩ :=
      congrArg Prod.fst (List.mem_singleton.mp hb)
    subst hfield
    rfl
  exact postingA_ne_postingB
    (hkey postingA postingB (Or.inl rfl) (Or.inr rfl)
      ((Fact.project_eq_iff postingA postingB K).mpr hall))

/-- The Sum observer: a key row's u64 payload. -/
def headU64 : List Value → Nat
  | { type := .u64, val := x } :: _ => x.val
  | _ => 0

/-- Sum over the emitted key rows. -/
def sumHead (rows : List (List Value)) : Nat :=
  natSum (rows.map headU64)

/-- The elided key stream: both fact tuples emit the one binding's
key. -/
def dupStream : List (List Value) := [[amount], [amount]]

/-- The distinct set of the stream is the one key. -/
theorem dedup_dupStream : Query.dedup dupStream = [[amount]] := by
  show Query.dedup [[amount], [amount]] = [[amount]]
  unfold Query.dedup
  rw [if_pos (List.mem_singleton.mpr rfl)]
  unfold Query.dedup
  rw [if_neg (fun h : [amount] ∈ ([] : List (List Value)) => nomatch h)]
  rfl

/-- **The countermodel (PRD 07).** `distinct_premise_load_bearing` —
the `DistinctWitness` premise cannot be dropped: the unkeyed
occurrence's two distinct facts collapse to one full binding
(`both_facts_one_binding`, `postingA_ne_postingB`), no key is covered
(`unkeyed_no_cover` — `provably_distinct` refuses this rule, so
`AggregateSink::without_seen_set` is unreachable for it), and the
`Sum` of the elided stream DOUBLE-COUNTS: 200 against the distinct
binding set's 100. The normative fold domain is the distinct set
(`Query.agg_over_distinct_bindings`); elision without the premise is
bag semantics by accident. -/
theorem distinct_premise_load_bearing (ρ : Query.ParamEnv) :
    (Query.MatchSelection unkeyedRule postingInstance ρ dupAssign
        (fun _ => postingA) ∧
      Query.MatchSelection unkeyedRule postingInstance ρ dupAssign
        (fun _ => postingB) ∧
      postingA ≠ postingB) ∧
    ¬ Query.BoundFieldsCoverKey unkeyedRule postingInstance ∧
    sumHead dupStream = 200 ∧
    sumHead (Query.dedup dupStream) = 100 := by
  refine ⟨⟨(both_facts_one_binding ρ).1, (both_facts_one_binding ρ).2,
    postingA_ne_postingB⟩, unkeyed_no_cover, rfl, ?_⟩
  rw [dedup_dupStream]
  rfl

/-! ## The elimination-needs-containment countermodel (PRD 08)

Two atoms joined on their id fields, in FULL elimination shape — every
syntactic condition of `Query.ElimStep` holds — but over an instance
with no containment: the source relation holds one fact, the target
relation is empty. The survivor rule answers where the original
cannot. -/

/-- The source atom `A(0: v₀)`. -/
def elimSrc : Query.Atom :=
  { relation := ⟨0⟩, bindings := [(⟨0⟩, .var ⟨0⟩)] }

/-- The target atom `B(0: v₀)` — the drop candidate. -/
def elimTgt : Query.Atom :=
  { relation := ⟨1⟩, bindings := [(⟨0⟩, .var ⟨0⟩)] }

/-- The two-atom rule: `finds v₀ where A(0: v₀), B(0: v₀)`. -/
def elimRule : Query.Rule where
  finds := [⟨0⟩]
  atoms := [elimSrc, elimTgt]
  negated := []
  conditions := []

/-- The survivor: the target dropped. -/
def elimSurvivor : Query.Rule where
  finds := [⟨0⟩]
  atoms := [elimSrc]
  negated := []
  conditions := []

/-- The one source fact — an orphan: no target row shares its id. -/
def orphanFact : Fact := fun _ => ⟨.bool, true⟩

/-- The instance: relation 0 holds the orphan, everything else is
empty — the containment `A(0) <= B(0)` FAILS here. -/
def elimInstance : Instance := fun R => fun f =>
  R.id = 0 ∧ f = orphanFact

/-- Every syntactic elimination condition holds of the pair — the
shape alone cannot see the missing witness. -/
theorem elim_step_holds :
    Query.ElimStep elimRule elimSurvivor elimSrc elimTgt [⟨0⟩] [⟨0⟩]
      Selection.empty Selection.empty where
  atoms_split := ⟨[elimSrc], [], rfl, rfl⟩
  finds_eq := rfl
  negated_eq := rfl
  conditions_eq := rfl
  source := List.mem_singleton.mpr rfl
  join_covers := by
    intro p hp
    rcases List.mem_singleton.mp hp with rfl
    exact ⟨⟨0⟩, List.mem_singleton.mpr rfl, List.mem_singleton.mpr rfl⟩
  carries_phi := fun s hs => by cases hs
  target_bindings := by
    intro bd hbd
    rcases List.mem_singleton.mp hbd with rfl
    exact Or.inl ⟨⟨0⟩, rfl⟩
  var_functional := by
    intro i j v hi hj
    exact (congrArg Prod.fst (List.mem_singleton.mp hi)).trans
      (congrArg Prod.fst (List.mem_singleton.mp hj)).symm
  join_or_dead := by
    intro i v hb
    have h1 := List.mem_singleton.mp hb
    left
    refine ⟨(⟨0⟩, ⟨0⟩), List.mem_singleton.mpr rfl,
      (congrArg Prod.fst h1).symm, ?_⟩
    have hv : Query.Term.var v = Query.Term.var (⟨0⟩ : Query.VarId) :=
      congrArg Prod.snd h1
    rw [hv]
    exact List.mem_singleton.mpr rfl

/-- The containment premise FAILS: the orphan has no target witness. -/
theorem elim_no_containment :
    ¬ Containment (elimInstance elimSrc.relation) Selection.empty
      [⟨0⟩] (elimInstance elimTgt.relation) Selection.empty [⟨0⟩] := by
  intro h
  obtain ⟨g, hg, -, -⟩ :=
    h orphanFact ⟨rfl, rfl⟩ (Selection.empty_satisfies _)
  exact absurd hg.1 (by decide)

/-- The survivor answers: the orphan derives it. -/
theorem elim_survivor_answers (C : Query.Classify)
    (ρ : Query.ParamEnv) :
    [orphanFact ⟨0⟩] ∈
      Query.ruleAnswers C elimSurvivor elimInstance ρ := by
  refine Query.mem_ruleAnswers.mpr
    ⟨fun _ => orphanFact ⟨0⟩, ⟨?_, ?_, ?_⟩, rfl⟩
  · intro a ha
    rcases List.mem_singleton.mp ha with rfl
    refine ⟨orphanFact, ⟨rfl, rfl⟩, ?_⟩
    intro bd hbd
    rcases List.mem_singleton.mp hbd with rfl
    rfl
  · intro a ha
    cases ha
  · intro c hc
    cases hc

/-- The original rule answers NOTHING: its target atom demands a fact
the empty target relation does not hold. -/
theorem elim_rule_empty (C : Query.Classify) (ρ : Query.ParamEnv) :
    ∀ t, t ∉ Query.ruleAnswers C elimRule elimInstance ρ := by
  intro t ht
  obtain ⟨σ, ⟨hatoms, -, -⟩, -⟩ := Query.mem_ruleAnswers.mp ht
  obtain ⟨f, hf, -⟩ := hatoms elimTgt
    (List.mem_cons_of_mem _ (List.mem_singleton.mpr rfl))
  exact absurd hf.1 (by decide)

/-- **The countermodel (PRD 08).** `elimination_needs_containment` —
the elimination shape holds, the containment premise fails, and
dropping the atom CHANGES answers: the survivor emits the orphan's
tuple, the original emits nothing. Why the elimination consults the
theory's statements and `elimination_sound` carries `Containment` as
a hypothesis — the syntactic conditions license the transfer, only
the statement licenses existence. -/
theorem elimination_needs_containment (C : Query.Classify)
    (ρ : Query.ParamEnv) :
    Query.ElimStep elimRule elimSurvivor elimSrc elimTgt [⟨0⟩] [⟨0⟩]
      Selection.empty Selection.empty ∧
    ¬ Containment (elimInstance elimSrc.relation) Selection.empty
      [⟨0⟩] (elimInstance elimTgt.relation) Selection.empty [⟨0⟩] ∧
    ∃ t, t ∈ Query.ruleAnswers C elimSurvivor elimInstance ρ ∧
      t ∉ Query.ruleAnswers C elimRule elimInstance ρ :=
  ⟨elim_step_holds, elim_no_containment,
    [orphanFact ⟨0⟩], elim_survivor_answers C ρ, elim_rule_empty C ρ _⟩

/-! ## The latch-miss countermodel (PRD 08) -/

/-- **The countermodel (PRD 08).** `latch_miss_not_static` — the
selection miss is PER-INSTANCE: the one-atom rule is empty at the
empty instance through `Query.EmptyAt.selectionMiss` (the dictionary
miss's abstract face), yet ANSWERS at the orphan instance — so the
miss verdict can never be promoted to the instance-independent
refutation, which is the latch's two-constructor design decision made
checkable. -/
theorem latch_miss_not_static (C : Query.Classify)
    (ρ : Query.ParamEnv) :
    Query.EmptyAt C ρ elimSurvivor emptyInstance ∧
    ∃ t, t ∈ Query.ruleAnswers C elimSurvivor elimInstance ρ :=
  ⟨.selectionMiss elimSrc (List.mem_singleton.mpr rfl)
      (fun _ hf _ _ => hf),
    [orphanFact ⟨0⟩], elim_survivor_answers C ρ⟩

/-! ## The FieldSet split locks (spec-fidelity F3)

Three concrete headers pin `Header.intervalSplit`'s set-canonical
reading — the shapes the written-order reading got wrong, locked
against regression. -/

/-- A header with the interval written FIRST: `[interval u64, bool]`
— the permuted shape the engine canonicalizes and a written-order
split misread as classical. -/
def permHeader : Header := ⟨fun _ => [.interval .u64, .bool]⟩

/-- A header with TWO interval positions — the gate-refused shape a
written-order split misread as pointwise (an interval inside the
"scalar" prefix). -/
def twoIntervalHeader : Header :=
  ⟨fun _ => [.interval .u64, .interval .u64]⟩

/-- **The permuted-shape lock.** `[interval, scalar]` splits to
`some ([scalar], interval)` — the pointwise reading at ANY written
position, exactly the engine's FieldSet canonicalization
(`judgment.rs` enforces coverage in permuted determinant order). -/
theorem split_permuted_some :
    permHeader.intervalSplit ⟨0⟩ [⟨0⟩, ⟨1⟩] = some ([⟨1⟩], ⟨0⟩) := rfl

/-- **The several-interval lock.** `[interval, interval]` splits to
`none` — under the set-canonical definition "every other shape splits
to `none`" is TRUE (the D2 spec error, closed). -/
theorem split_two_intervals_none :
    twoIntervalHeader.intervalSplit ⟨0⟩ [⟨0⟩, ⟨1⟩] = none := rfl

/-- **The all-scalar lock**, concrete (the general theorem is
`Header.intervalSplit_scalar`): a scalar projection splits to
`none` — the classical-judgment arm. -/
theorem split_all_scalar_none :
    pcHeader.intervalSplit ⟨0⟩ [⟨0⟩] = none := rfl

/-! ## The violated unit window (extension 1, `Cardinality.lean`)

One parent, two distinct children sharing its key — the bare-`==`
model's rows, reread as a parent/child pair. -/

/-- The one-fact parent: every field the shared key. -/
def winParent : Fact := fun _ => keyVal

/-- The one-fact parent relation. -/
def winParents : Set Fact := fun f => f = winParent

/-- **The countermodel (port).** The `1..1` window FAILS on one
parent with two distinct children: the two-element duplicate-free
member list breaks the ceiling. The upper bound is load-bearing — a
window is never just its floor, which is why `1..1` says strictly
more than the reverse containment (`window_floor_containment`). -/
theorem unit_window_two_children :
    ¬ CardinalityWindow twoTarget Selection.empty keyProj
        (Window.mk 1 (some 1)) winParents Selection.empty keyProj := by
  intro h
  have hmost := (h winParent rfl (Selection.empty_satisfies _)).2 1 rfl
  refine Set.not_atMost_one_of_two ?_ ?_ rowTrue_ne_rowFalse hmost
  · exact ⟨Or.inl rfl, Selection.empty_satisfies _, rfl⟩
  · exact ⟨Or.inr rfl, Selection.empty_satisfies _, rfl⟩

/-! ## The indivisible disjunctive window (E3)

Why literal SETS are first-class rather than per-literal sugar:
counts over a union do not decompose. The `1..1` window over the
disjunctive selection `payload ∈ {true, false}` accepts each
one-child relation and rejects their union — while ANY conjunction of
per-literal windows that accepts both one-child relations must accept
the union too, because each literal's child group in the union is
exactly its group in one of the accepted relations. No conjunction of
per-literal windows expresses the disjunctive window. -/

/-- The `true` payload literal. -/
def valTrue : Value := ⟨.bool, true⟩

/-- The `false` payload literal. -/
def valFalse : Value := ⟨.bool, false⟩

/-- The disjunctive selection: the payload field carries `true` OR
`false` — one binding, a two-literal set. -/
def orSel : Selection := ⟨[(⟨1⟩, [valTrue, valFalse])]⟩

/-- The `true` per-literal restriction (a singleton set — the old
equality binding). -/
def selTrue : Selection := ⟨[(⟨1⟩, [valTrue])]⟩

/-- The `false` per-literal restriction. -/
def selFalse : Selection := ⟨[(⟨1⟩, [valFalse])]⟩

/-- The one-fact `false`-child relation (`oneSource` is its `true`
sibling). -/
def oneFalse : Set Fact := fun f => f = rowFalse

/-- The `true` literal's child group in the union is exactly its
group in the `true`-child relation: the `false` child fails the
singleton selection. -/
theorem selTrue_group_union (t : List Value) :
    ChildGroup twoTarget selTrue keyProj t
      = ChildGroup oneSource selTrue keyProj t := by
  funext f
  refine propext ⟨?_, ?_⟩
  · rintro ⟨hf, hsel, hproj⟩
    have hf' : f = rowTrue ∨ f = rowFalse := hf
    rcases hf' with rfl | rfl
    · exact ⟨rfl, hsel, hproj⟩
    · have hv : rowFalse ⟨1⟩ = valTrue :=
        Selection.satisfies_singleton hsel (List.mem_singleton.mpr rfl)
      have hb : (false : Bool) = true := congrArg Value.asBool hv
      cases hb
  · rintro ⟨hf, hsel, hproj⟩
    exact ⟨Or.inl hf, hsel, hproj⟩

/-- The `false` literal's child group in the union is exactly its
group in the `false`-child relation — the mirror split. -/
theorem selFalse_group_union (t : List Value) :
    ChildGroup twoTarget selFalse keyProj t
      = ChildGroup oneFalse selFalse keyProj t := by
  funext f
  refine propext ⟨?_, ?_⟩
  · rintro ⟨hf, hsel, hproj⟩
    have hf' : f = rowTrue ∨ f = rowFalse := hf
    rcases hf' with rfl | rfl
    · have hv : rowTrue ⟨1⟩ = valFalse :=
        Selection.satisfies_singleton hsel (List.mem_singleton.mpr rfl)
      have hb : (true : Bool) = false := congrArg Value.asBool hv
      cases hb
    · exact ⟨rfl, hsel, hproj⟩
  · rintro ⟨hf, hsel, hproj⟩
    exact ⟨Or.inr hf, hsel, hproj⟩

/-- **The countermodel (E3).** The `1..1` window over the disjunctive
selection accepts each one-child relation and rejects their union —
while ANY pair of per-literal windows accepting both one-child
relations also accepts the union (each literal's group transfers
whole, `selTrue_group_union` / `selFalse_group_union`). A count over
a union is not any conjunction of per-literal counts: the admitted
count-vectors of a union window are not a product set. This is what
makes the literal set a first-class selection form rather than
lowering sugar. -/
theorem disjunctive_window_not_literal_conjunction :
    (CardinalityWindow oneSource orSel keyProj (Window.mk 1 (some 1))
        winParents Selection.empty keyProj ∧
      CardinalityWindow oneFalse orSel keyProj (Window.mk 1 (some 1))
        winParents Selection.empty keyProj ∧
      ¬ CardinalityWindow twoTarget orSel keyProj (Window.mk 1 (some 1))
        winParents Selection.empty keyProj) ∧
    (∀ wt wf : Window,
      (CardinalityWindow oneSource selTrue keyProj wt winParents
          Selection.empty keyProj ∧
        CardinalityWindow oneSource selFalse keyProj wf winParents
          Selection.empty keyProj) →
      (CardinalityWindow oneFalse selTrue keyProj wt winParents
          Selection.empty keyProj ∧
        CardinalityWindow oneFalse selFalse keyProj wf winParents
          Selection.empty keyProj) →
      (CardinalityWindow twoTarget selTrue keyProj wt winParents
          Selection.empty keyProj ∧
        CardinalityWindow twoTarget selFalse keyProj wf winParents
          Selection.empty keyProj)) := by
  refine ⟨⟨?_, ?_, ?_⟩, ?_⟩
  · -- the true child alone: the union count is one
    intro g hg hψ
    have hg' : g = winParent := hg
    subst hg'
    refine ⟨⟨[rowTrue],
      List.Pairwise.cons (fun x hx => nomatch hx) List.Pairwise.nil,
      ?_, Nat.le_refl 1⟩, ?_⟩
    · intro a ha
      rcases List.mem_singleton.mp ha with rfl
      refine ⟨rfl, ?_, rfl⟩
      intro bd hbd
      rcases List.mem_singleton.mp hbd with rfl
      exact List.mem_cons_self ..
    · intro m hm
      injection hm with hm
      subst hm
      exact Set.atMost_one_of_subsingleton fun a b ha hb =>
        (show a = rowTrue from ha.1).trans
          (show b = rowTrue from hb.1).symm
  · -- the false child alone: the union count is one
    intro g hg hψ
    have hg' : g = winParent := hg
    subst hg'
    refine ⟨⟨[rowFalse],
      List.Pairwise.cons (fun x hx => nomatch hx) List.Pairwise.nil,
      ?_, Nat.le_refl 1⟩, ?_⟩
    · intro a ha
      rcases List.mem_singleton.mp ha with rfl
      refine ⟨rfl, ?_, rfl⟩
      intro bd hbd
      rcases List.mem_singleton.mp hbd with rfl
      exact List.mem_cons_of_mem _ (List.mem_singleton.mpr rfl)
    · intro m hm
      injection hm with hm
      subst hm
      exact Set.atMost_one_of_subsingleton fun a b ha hb =>
        (show a = rowFalse from ha.1).trans
          (show b = rowFalse from hb.1).symm
  · -- the union: the disjunctive count is two — the ceiling breaks
    intro h
    have hmost :=
      (h winParent rfl (Selection.empty_satisfies _)).2 1 rfl
    refine Set.not_atMost_one_of_two ?_ ?_ rowTrue_ne_rowFalse hmost
    · refine ⟨Or.inl rfl, ?_, rfl⟩
      intro bd hbd
      rcases List.mem_singleton.mp hbd with rfl
      exact List.mem_cons_self ..
    · refine ⟨Or.inr rfl, ?_, rfl⟩
      intro bd hbd
      rcases List.mem_singleton.mp hbd with rfl
      exact List.mem_cons_of_mem _ (List.mem_singleton.mpr rfl)
  · -- any per-literal conjunction accepting both singles accepts the
    -- union: each literal's group transfers whole
    intro wt wf h10 h01
    constructor
    · intro g hg hψ
      rw [selTrue_group_union]
      exact h10.1 g hg hψ
    · intro g hg hψ
      rw [selFalse_group_union]
      exact h01.2 g hg hψ

/-! ## The recursion walls (Exec/Fixpoint)

Two countermodels fence the stratified fixpoint's two premises:

* **The odd loop** — `p ← ¬p`, negation through the predicate's own
  SCC. Not stratified (`odd_not_stratified`), and honestly so: the
  operator is NOT monotone (`odd_not_monotone`), its naive rounds
  oscillate (`odd_rounds_oscillate` — the empty table derives, the
  derived table underives), and NO table is a fixpoint
  (`odd_no_fixpoint`) — there is no consistent semantics to assign,
  which is why `Exec/Fixpoint.lean: stratumOp_mono` carries
  `StratifiedBy` as its premise rather than as a convention.

* **The successor operator** — value invention in a rule head,
  modeled at the OPERATOR level because it is unrepresentable in
  `PRule` syntax (heads are projected variables — the creation
  quarantine): `succOp X = {0} ∪ {m + 1 | m ∈ X}` is monotone
  (`succOp_monotone`) yet its naive chain ascends forever
  (`succ_chain_ascends`) and every prefixed point is infinite
  (`succ_prefixed_infinite` — no list enumerates it). Termination's
  premise (heads project BOUND variables, so candidates live on the
  finite active domain — `Exec/Fixpoint.lean: program_den_finite`)
  is load-bearing, exactly the chain-window fence
  (`docs/architecture/20-query-ir.md` § the chain-window fence). -/

/-- The odd loop's one atom: the program's own predicate, negated,
zero bindings (the nonemptiness gate). -/
def oddAtom : Query.PAtom := ⟨.idb ⟨0⟩, []⟩

/-- The odd loop's one rule: `p ← ¬p`. Safe (no variables at all) —
safety is not the broken premise here. -/
def oddRule : Query.PRule := ⟨[], [], [oddAtom], []⟩

/-- The odd loop: one zero-arity predicate whose only rule negates
itself. -/
def oddProgram : Query.Program := ⟨[⟨0, [oddRule]⟩], ⟨0⟩⟩

/-- **No stratum witness exists**: the self-negation edge demands
`strat p < strat p`. -/
theorem odd_not_stratified : ¬ oddProgram.Stratified := by
  rintro ⟨strat, h⟩
  have hedge : (⟨⟨0⟩, .negated⟩ : Query.Edge) ∈ oddRule.edges := by
    decide
  have := (h 0 ⟨0, [oddRule]⟩ rfl oddRule (List.mem_singleton.mpr rfl)
    _ hedge).2 rfl
  exact Nat.lt_irrefl _ this

/-- The odd loop's stratum-0 operator (any classifier, instance, and
parameter environment — the program reads none of them). -/
def oddOp (C : Query.Classify) (I : Instance) (ρ : Query.ParamEnv) :
    Query.PredSets → Query.PredSets :=
  Query.stratumOp C oddProgram (fun _ => 0) I ρ 0 (fun _ _ => False)

/-- An empty table derives: nothing matches the negated atom. -/
theorem odd_step_of_empty (C : Query.Classify) (I : Instance)
    (ρ : Query.ParamEnv) {X : Query.PredSets}
    (hX : ∀ t, ¬ t ∈ X ⟨0⟩) : [] ∈ oddOp C I ρ X ⟨0⟩ := by
  refine ⟨rfl, ⟨0, [oddRule]⟩, rfl, oddRule,
    List.mem_singleton.mpr rfl, fun _ => ⟨.bool, false⟩,
    ⟨?_, ?_, ?_⟩, rfl⟩
  · intro a ha
    exact absurd ha (by simp [oddRule])
  · intro a ha hex
    have haa : a = oddAtom := by simpa [oddRule] using ha
    subst haa
    obtain ⟨f, hf, -⟩ := hex
    obtain ⟨t, ht, -⟩ := hf
    rw [Query.stratumSets_at rfl] at ht
    exact hX t ht
  · intro t ht
    exact absurd ht (by simp [oddRule])

/-- A nonempty table underives: the derived fact refutes the very
rule that derived it. -/
theorem odd_step_of_nonempty (C : Query.Classify) (I : Instance)
    (ρ : Query.ParamEnv) {X : Query.PredSets} {t₀ : Query.AnswerTuple}
    (h0 : t₀ ∈ X ⟨0⟩) : ∀ t, ¬ t ∈ oddOp C I ρ X ⟨0⟩ := by
  rintro t ⟨-, d, hd, r, hr, σ, ⟨-, hneg, -⟩, -⟩
  have hdq : d = ⟨0, [oddRule]⟩ := (Option.some.inj hd).symm
  subst hdq
  have hrr : r = oddRule := List.mem_singleton.mp hr
  subst hrr
  refine hneg oddAtom (List.mem_singleton.mpr rfl)
    ⟨Query.tupleFact t₀, ⟨t₀, ?_, rfl⟩, ?_⟩
  · rw [Query.stratumSets_at rfl]
    exact h0
  · intro b hb
    exact absurd hb (by simp [oddAtom])

/-- **The naive rounds oscillate**: round one derives the head from
the empty table; round two, fed round one's table, underives it —
no round-robin ever settles. -/
theorem odd_rounds_oscillate (C : Query.Classify) (I : Instance)
    (ρ : Query.ParamEnv) :
    [] ∈ oddOp C I ρ (fun _ _ => False) ⟨0⟩ ∧
      ¬ [] ∈ oddOp C I ρ (oddOp C I ρ (fun _ _ => False)) ⟨0⟩ := by
  have h1 := odd_step_of_empty C I ρ
    (X := fun _ _ => False) (fun _ ht => ht)
  exact ⟨h1, odd_step_of_nonempty C I ρ h1 []⟩

/-- **No consistent semantics**: no table is a fixed point of the
odd loop's operator — an empty answer derives the head, a nonempty
one refutes its own derivation. -/
theorem odd_no_fixpoint (C : Query.Classify) (I : Instance)
    (ρ : Query.ParamEnv) :
    ∀ X : Query.PredSets,
      ¬ (∀ t, t ∈ oddOp C I ρ X ⟨0⟩ ↔ t ∈ X ⟨0⟩) := by
  intro X hfix
  by_cases hX : ∃ t, t ∈ X ⟨0⟩
  · obtain ⟨t₀, ht₀⟩ := hX
    exact odd_step_of_nonempty C I ρ ht₀ t₀ ((hfix t₀).mpr ht₀)
  · have hempty : ∀ t, ¬ t ∈ X ⟨0⟩ := fun t ht => hX ⟨t, ht⟩
    exact hempty [] ((hfix []).mp (odd_step_of_empty C I ρ hempty))

/-- **The non-monotonicity witness**: growing the table SHRINKS the
operator's output — exactly what `Exec/Fixpoint.lean:
stratumOp_mono`'s stratification premise rules out. -/
theorem odd_not_monotone (C : Query.Classify) (I : Instance)
    (ρ : Query.ParamEnv) : ¬ Query.MonoP (oddOp C I ρ) := by
  intro hm
  have h1 : [] ∈ oddOp C I ρ (fun _ _ => False) ⟨0⟩ :=
    odd_step_of_empty C I ρ (fun _ ht => ht)
  have h2 := hm (fun _ _ => False) (oddOp C I ρ (fun _ _ => False))
    (fun _ _ ht => absurd ht (fun h => h)) ⟨0⟩ [] h1
  exact odd_step_of_nonempty C I ρ h1 [] h2

/-- The successor operator: a head-creating rule's immediate
consequence (`p(0)`; `p(m + 1) ← p(m)`), writable only at the
operator level — `PRule` heads cannot create values. -/
def succOp : Set Nat → Set Nat :=
  fun X n => n = 0 ∨ ∃ m, m ∈ X ∧ n = m + 1

/-- The successor operator is monotone — stratification is NOT the
broken premise here; head creation is. -/
theorem succOp_monotone {X Y : Set Nat} (h : ∀ n, n ∈ X → n ∈ Y) :
    ∀ n, n ∈ succOp X → n ∈ succOp Y := by
  rintro n (rfl | ⟨m, hm, rfl⟩)
  · exact Or.inl rfl
  · exact Or.inr ⟨m, h m hm, rfl⟩

/-- Round `k` of the successor chain stays below `k` … -/
theorem succ_chain_bound :
    ∀ k n, n ∈ Query.naiveIter succOp k → n < k
  | 0, _, h => absurd h (fun h => h)
  | k + 1, n, h => by
    rcases h with h | h
    · exact Nat.lt_succ_of_lt (succ_chain_bound k n h)
    · rcases h with rfl | ⟨m, hm, rfl⟩
      · exact Nat.zero_lt_succ k
      · exact Nat.succ_lt_succ (succ_chain_bound k m hm)

/-- … and every round grows: the ascending chain never stabilizes —
`n` arrives exactly at round `n + 1`. Termination's premise is
load-bearing. -/
theorem succ_chain_ascends :
    ∀ n, n ∈ Query.naiveIter succOp (n + 1) ∧
      ¬ n ∈ Query.naiveIter succOp n := by
  intro n
  constructor
  · induction n with
    | zero => exact Or.inr (Or.inl rfl)
    | succ n ih => exact Or.inr (Or.inr ⟨n, ih, rfl⟩)
  · intro h
    exact Nat.lt_irrefl n (succ_chain_bound n n h)

/-- Every prefixed point of the successor operator holds every
natural. -/
theorem succ_prefixed_all (X : Set Nat)
    (hpre : ∀ n, n ∈ succOp X → n ∈ X) : ∀ n, n ∈ X := by
  intro n
  induction n with
  | zero => exact hpre 0 (Or.inl rfl)
  | succ n ih => exact hpre (n + 1) (Or.inr ⟨n, ih, rfl⟩)

/-- **The infinite ascending chain's wall**: no prefixed point of the
successor operator is finite — any candidate list misses the value
one past its maximum. The safety theorem
(`Exec/Fixpoint.lean: program_den_finite`) survives on exactly the
premise this operator breaks: heads project bound variables, never
created ones. -/
theorem succ_prefixed_infinite (X : Set Nat)
    (hpre : ∀ n, n ∈ succOp X → n ∈ X) : ¬ X.Finite := by
  rintro ⟨l, hl⟩
  have hmem : l.foldr Nat.max 0 + 1 ∈ l :=
    (hl _).mp (succ_prefixed_all X hpre _)
  have := le_foldr_max l _ hmem
  omega

/-! ## The join-blast countermodel (the admission calculus, E1)

The E1 shape — a window over a JOINED pair of atoms — has no
oracle-bounded enforcement plan, and the refusal is BY
REPRESENTATION, twice over, the strongest kind: `Statement`'s sides
are single `Atom`s (one `RelId` each — a joined side is unwritable),
and every `EnforcementPlan` evaluation answers from one oracle
(`Oracle.plan_answers_sound`) whose fact surface the per-form
conformance pins hold to ONE stored relation's denotation — the pins,
not the evaluation lemma alone, are what keep a join surface out
(the gate-type composition is `joined_window_form_uninhabitable`
below). This countermodel is the refusal's mathematical face: the
touched-group license every sanctioned plan spends — the
untouched-implies-unchanged lemmas of `Txn/DeltaRestriction.lean`
(`cardinality_untouched_group_eq` for the single-atom window) — is
FALSE for the joined shape. One inserted fact on one join side
changes the joined child set at parent groups NEITHER relation's
delta projects to at the grouping, one group per matching join
partner: deciding the shape costs consultations proportional to the
JOIN, not to the touched groups, which is exactly the blast radius
the acceptance gate's cost law refuses
(`docs/architecture/30-dependencies.md` § the acceptance gate). The
two-parent witness below is the seed: the delta projects to no
parent tag, the pre-state joined sets at both tags are empty, and
both gain a pair from the one insert. -/

/-- The joined-window model's grouping projection: the parent tag at
field 1. -/
def blastGrp : List FieldId := [⟨1⟩]

/-- The pre-instance: relation 0 (the A side) holds the two join
facts — shared join key `keyVal` at field 0, distinct parent tags at
field 1 (`rowTrue`/`rowFalse` reread) — and relation 1 (the B side)
is empty. -/
def blastPre : Instance := fun R f =>
  R = parentRel ∧ (f = rowTrue ∨ f = rowFalse)

/-- The delta: ONE B-side fact lands — `winParent` (the join key at
every field, so its parent-tag projection is `[keyVal]`, matching
neither A-side tag). -/
def blastDelta : Txn.Delta :=
  ⟨fun R f => R = childRel ∧ f = winParent, fun _ _ => False⟩

/-- The E1 shape's would-be child set at parent tag `t`: the (a, b)
pairs joined on field 0 whose A side projects to `t` — the object a
joined window would count per parent. -/
def JoinedChildren (A B : Set Fact) (t : List Value) :
    Set (Fact × Fact) :=
  fun p => p.1 ∈ A ∧ p.2 ∈ B ∧ p.1 ⟨0⟩ = p.2 ⟨0⟩ ∧
    p.1.project blastGrp = t

/-- The join key is not the `true` tag — the two value types
differ. -/
theorem keyVal_ne_valTrue : keyVal ≠ valTrue :=
  fun h => nomatch congrArg Value.type h

/-- The join key is not the `false` tag. -/
theorem keyVal_ne_valFalse : keyVal ≠ valFalse :=
  fun h => nomatch congrArg Value.type h

/-- **The countermodel (E1).** The joined shape's blast radius: the
delta's grouping projection touches NO parent tag on either relation
(the first three conjuncts — the touched-group license has nothing to
re-check), both parent tags' joined child sets are empty in the
pre-state, and BOTH gain a pair in the final state from the one
inserted B-fact. Untouched-implies-unchanged fails at every parent a
join partner reaches — consultations proportional to the join, the
cost law's refused shape. -/
theorem joined_window_blast :
    (∀ t, t ∉ blastDelta.projected parentRel blastGrp) ∧
    [valTrue] ∉ blastDelta.projected childRel blastGrp ∧
    [valFalse] ∉ blastDelta.projected childRel blastGrp ∧
    (∀ p, p ∉ JoinedChildren (blastPre parentRel)
      (blastPre childRel) [valTrue]) ∧
    (∀ p, p ∉ JoinedChildren (blastPre parentRel)
      (blastPre childRel) [valFalse]) ∧
    (rowTrue, winParent) ∈ JoinedChildren
      (blastDelta.applyTo blastPre parentRel)
      (blastDelta.applyTo blastPre childRel) [valTrue] ∧
    (rowFalse, winParent) ∈ JoinedChildren
      (blastDelta.applyTo blastPre parentRel)
      (blastDelta.applyTo blastPre childRel) [valFalse] := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rintro t ⟨f, hf | hf, -⟩
    · exact child_ne_parent hf.1.symm
    · exact hf
  · rintro ⟨f, hf | hf, hproj⟩
    · obtain ⟨-, rfl⟩ := hf
      have h1 : ([keyVal] : List Value) = [valTrue] := hproj
      injection h1 with h2 _
      exact keyVal_ne_valTrue h2
    · exact hf
  · rintro ⟨f, hf | hf, hproj⟩
    · obtain ⟨-, rfl⟩ := hf
      have h1 : ([keyVal] : List Value) = [valFalse] := hproj
      injection h1 with h2 _
      exact keyVal_ne_valFalse h2
    · exact hf
  · rintro p ⟨-, hB, -, -⟩
    exact child_ne_parent hB.1
  · rintro p ⟨-, hB, -, -⟩
    exact child_ne_parent hB.1
  · exact ⟨Or.inl ⟨⟨rfl, Or.inl rfl⟩, fun h => h⟩,
      Or.inr ⟨rfl, rfl⟩, rfl, rfl⟩
  · exact ⟨Or.inl ⟨⟨rfl, Or.inr rfl⟩, fun h => h⟩,
      Or.inr ⟨rfl, rfl⟩, rfl, rfl⟩

/-! ## The Free Join wrong-cover countermodel (the plan formalism)

The paper's cover Definition ("containing all new variables") lets a
subatom that ALSO carries an already-bound variable be iterated. On
skewed data the executor then REBINDS the bound variable from the
cover's facts without re-checking the occurrence that bound it —
earlier nodes are never revisited. The triangle query below is the
`docs/architecture/40-execution.md` § the-paper's-core deviation
paragraph, mechanized: `R = {(1,2)}`, `S = {(3,4)}`, `T = {(1,4)}`;
the loose plan iterates `R` whole, then lets `S`'s subatom `(b, c)`
cover node 2 (whose one new variable is `c`), rebinding `b` from 2 to
3 and emitting `(1, 3, 4)` — but `R(1,3)` is not a fact, so the tuple
is outside the denotation. Bumbledb's exactly-new-variables rule
refuses the plan (`loose_plan_not_valid` — node 2 has no cover), and
every plan it accepts computes the denotation
(`Exec/Plan.lean: valid_plan_sound`). -/

/-- The triangle's variables: `a`, `b`, `c`. -/
def triA : Query.VarId := ⟨0⟩
/-- Triangle variable `b`. -/
def triB : Query.VarId := ⟨1⟩
/-- Triangle variable `c`. -/
def triC : Query.VarId := ⟨2⟩

/-- The three edge relations. -/
def triR : RelId := ⟨0⟩
/-- Edge relation `S`. -/
def triS : RelId := ⟨1⟩
/-- Edge relation `T`. -/
def triT : RelId := ⟨2⟩

/-- `R(a, b)`. -/
def triAtomR : Query.Atom :=
  ⟨triR, [(⟨0⟩, .var triA), (⟨1⟩, .var triB)]⟩
/-- `S(b, c)`. -/
def triAtomS : Query.Atom :=
  ⟨triS, [(⟨0⟩, .var triB), (⟨1⟩, .var triC)]⟩
/-- `T(a, c)`. -/
def triAtomT : Query.Atom :=
  ⟨triT, [(⟨0⟩, .var triA), (⟨1⟩, .var triC)]⟩

/-- The triangle rule: find `(a, b, c)` from `R(a,b), S(b,c),
T(a,c)` — safe, well-typed, condition-free. -/
def triRule : Query.Rule :=
  ⟨[triA, triB, triC], [triAtomR, triAtomS, triAtomT], [], []⟩

/-- The four `u64` values the instance spends. -/
def tri1 : Value := ⟨.u64, ⟨1, by omega⟩⟩
/-- Value 2. -/
def tri2 : Value := ⟨.u64, ⟨2, by omega⟩⟩
/-- Value 3. -/
def tri3 : Value := ⟨.u64, ⟨3, by omega⟩⟩
/-- Value 4. -/
def tri4 : Value := ⟨.u64, ⟨4, by omega⟩⟩

/-- `R`'s one fact: `(1, 2)`. -/
def triFactR : Fact := fun i => if i = ⟨0⟩ then tri1 else tri2
/-- `S`'s one fact: `(3, 4)`. -/
def triFactS : Fact := fun i => if i = ⟨0⟩ then tri3 else tri4
/-- `T`'s one fact: `(1, 4)`. -/
def triFactT : Fact := fun i => if i = ⟨0⟩ then tri1 else tri4

/-- The skewed instance: one fact per edge relation, and NO triangle
(`R(1,3)` would be needed to close one through `S`'s fact). -/
def triInst : Instance := fun R f =>
  (R = triR ∧ f = triFactR) ∨ (R = triS ∧ f = triFactS) ∨
    (R = triT ∧ f = triFactT)

/-- The wrong-cover plan: node 1 iterates `R` whole (variables
`a, b`); node 2 joins `S` and `T`, its one new variable `c`. `S`'s
subatom carries `(b, c)` and `T`'s `(a, c)` — each contains the new
variable, NEITHER is exactly it. -/
def triLoosePlan : Query.Plan :=
  [[⟨0, [triA, triB]⟩], [⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩]]

/-- The rebound assignment the loose execution emits:
`a = 1, b = 3, c = 4`. -/
def triLooseOut : Query.Assignment := fun v =>
  if v.id = 0 then tri1 else if v.id = 1 then tri3 else tri4

/-- The node-1 binding the loose execution extends: `a = 1, b = 2`
(and `c` already at its final value — the totalization device). -/
def triMid : Query.Assignment := fun v =>
  if v.id = 1 then tri2 else triLooseOut v

/-- The loose plan satisfies the PAPER's validity whole — partition,
placement, occurrence-disjointness, and the paper's cover rule. -/
theorem loose_plan_paper_valid : Query.PaperPlanValid triRule triLoosePlan := by
  refine
    { occScoped := ?_, complete := ?_, coversVar := ?_, onceVar := ?_,
      occDisjoint := ?_, covered := ?_ }
  · intro n hn s hs
    rcases List.mem_cons.mp hn with rfl | hn'
    · rw [List.mem_singleton.mp hs]
      exact ⟨triAtomR, rfl, fun v hv => hv⟩
    rcases List.mem_cons.mp hn' with rfl | hn''
    · rcases List.mem_cons.mp hs with rfl | hs'
      · exact ⟨triAtomS, rfl, fun v hv => hv⟩
      rcases List.mem_cons.mp hs' with rfl | hs''
      · exact ⟨triAtomT, rfl, fun v hv => hv⟩
      · exact absurd hs'' List.not_mem_nil
    · exact absurd hn'' List.not_mem_nil
  · intro i hi
    match i, hi with
    | 0, _ =>
      exact ⟨[⟨0, [triA, triB]⟩], List.mem_cons_self,
        ⟨0, [triA, triB]⟩, List.mem_cons_self, rfl⟩
    | 1, _ =>
      exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
        List.mem_cons_of_mem _ List.mem_cons_self,
        ⟨1, [triB, triC]⟩, List.mem_cons_self, rfl⟩
    | 2, _ =>
      exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
        List.mem_cons_of_mem _ List.mem_cons_self,
        ⟨2, [triA, triC]⟩,
        List.mem_cons_of_mem _ List.mem_cons_self, rfl⟩
  · intro i a hia v hv
    match i, hia with
    | 0, hia =>
      obtain rfl : triAtomR = a := Option.some.inj hia
      rcases List.mem_cons.mp hv with rfl | hv'
      · exact ⟨[⟨0, [triA, triB]⟩], List.mem_cons_self,
          ⟨0, [triA, triB]⟩, List.mem_cons_self, rfl,
          List.mem_cons_self⟩
      rcases List.mem_cons.mp hv' with rfl | hv''
      · exact ⟨[⟨0, [triA, triB]⟩], List.mem_cons_self,
          ⟨0, [triA, triB]⟩, List.mem_cons_self, rfl,
          List.mem_cons_of_mem _ List.mem_cons_self⟩
      · exact absurd hv'' List.not_mem_nil
    | 1, hia =>
      obtain rfl : triAtomS = a := Option.some.inj hia
      rcases List.mem_cons.mp hv with rfl | hv'
      · exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
          List.mem_cons_of_mem _ List.mem_cons_self,
          ⟨1, [triB, triC]⟩, List.mem_cons_self, rfl,
          List.mem_cons_self⟩
      rcases List.mem_cons.mp hv' with rfl | hv''
      · exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
          List.mem_cons_of_mem _ List.mem_cons_self,
          ⟨1, [triB, triC]⟩, List.mem_cons_self, rfl,
          List.mem_cons_of_mem _ List.mem_cons_self⟩
      · exact absurd hv'' List.not_mem_nil
    | 2, hia =>
      obtain rfl : triAtomT = a := Option.some.inj hia
      rcases List.mem_cons.mp hv with rfl | hv'
      · exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
          List.mem_cons_of_mem _ List.mem_cons_self,
          ⟨2, [triA, triC]⟩,
          List.mem_cons_of_mem _ List.mem_cons_self, rfl,
          List.mem_cons_self⟩
      rcases List.mem_cons.mp hv' with rfl | hv''
      · exact ⟨[⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩],
          List.mem_cons_of_mem _ List.mem_cons_self,
          ⟨2, [triA, triC]⟩,
          List.mem_cons_of_mem _ List.mem_cons_self, rfl,
          List.mem_cons_of_mem _ List.mem_cons_self⟩
      · exact absurd hv'' List.not_mem_nil
  · have hpos : ∀ (k : Nat) (n : Query.PlanNode) (i : Nat) (v : Query.VarId),
        triLoosePlan[k]? = some n →
        (∃ s, s ∈ n ∧ s.occ = i ∧ v ∈ s.vars) →
        (k = 0 ∧ i = 0) ∨ (k = 1 ∧ (i = 1 ∨ i = 2)) := by
      intro k n i v hk hs
      match k, hk with
      | 0, hk =>
        obtain rfl := Option.some.inj hk
        obtain ⟨s, hs, hocc, -⟩ := hs
        rw [List.mem_singleton.mp hs] at hocc
        exact Or.inl ⟨rfl, hocc.symm⟩
      | 1, hk =>
        obtain rfl := Option.some.inj hk
        obtain ⟨s, hs, hocc, -⟩ := hs
        rcases List.mem_cons.mp hs with rfl | hs'
        · exact Or.inr ⟨rfl, Or.inl hocc.symm⟩
        rcases List.mem_cons.mp hs' with rfl | hs''
        · exact Or.inr ⟨rfl, Or.inr hocc.symm⟩
        · exact absurd hs'' List.not_mem_nil
    intro i v k₁ k₂ n₁ n₂ h₁ h₂ e₁ e₂
    rcases hpos k₁ n₁ i v h₁ e₁ with ⟨rfl, hi₁⟩ | ⟨rfl, hi₁⟩ <;>
      rcases hpos k₂ n₂ i v h₂ e₂ with ⟨rfl, hi₂⟩ | ⟨rfl, hi₂⟩
    · rfl
    · rcases hi₂ with h | h <;>
        exact absurd (hi₁.symm.trans h) (by decide)
    · rcases hi₁ with h | h <;>
        exact absurd (h.symm.trans hi₂) (by decide)
    · rfl
  · intro n hn s₁ hs₁ s₂ hs₂ hocc
    rcases List.mem_cons.mp hn with rfl | hn'
    · rw [List.mem_singleton.mp hs₁, List.mem_singleton.mp hs₂]
    rcases List.mem_cons.mp hn' with rfl | hn''
    · rcases List.mem_cons.mp hs₁ with rfl | hs₁' <;>
        rcases List.mem_cons.mp hs₂ with rfl | hs₂'
      · rfl
      · rcases List.mem_cons.mp hs₂' with rfl | h
        · exact absurd hocc (by decide)
        · exact absurd h List.not_mem_nil
      · rcases List.mem_cons.mp hs₁' with rfl | h
        · exact absurd hocc (by decide)
        · exact absurd h List.not_mem_nil
      · rcases List.mem_cons.mp hs₁' with rfl | h
        · rcases List.mem_cons.mp hs₂' with rfl | h'
          · rfl
          · exact absurd h' List.not_mem_nil
        · exact absurd h List.not_mem_nil
    · exact absurd hn'' List.not_mem_nil
  · intro k n hk
    match k, hk with
    | 0, hk =>
      obtain rfl := Option.some.inj hk
      exact ⟨⟨0, [triA, triB]⟩, List.mem_cons_self,
        fun v hv => And.left hv⟩
    | 1, hk =>
      obtain rfl := Option.some.inj hk
      refine ⟨⟨1, [triB, triC]⟩, List.mem_cons_self, fun v hv => ?_⟩
      obtain ⟨h1, h2⟩ := hv
      rcases List.mem_cons.mp h1 with rfl | h1'
      · exact List.mem_cons_self
      rcases List.mem_cons.mp h1' with rfl | h1''
      · exact List.mem_cons_of_mem _ List.mem_cons_self
      rcases List.mem_cons.mp h1'' with rfl | h1'''
      · exact absurd (by decide :
          triA ∈ Query.planVars (triLoosePlan.take 1)) h2
      rcases List.mem_cons.mp h1''' with rfl | h1''''
      · exact List.mem_cons_of_mem _ List.mem_cons_self
      · exact absurd h1'''' List.not_mem_nil

/-- Bumbledb's exactly-new-variables rule REFUSES the loose plan:
node 2's one new variable is `c`, and both subatoms carry a bound
variable beside it — no cover exists. -/
theorem loose_plan_not_valid : ¬ Query.PlanValid triRule triLoosePlan := by
  intro hv
  obtain ⟨s, hs, hiff⟩ :=
    hv.covered 1 [⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩] rfl
  rcases List.mem_cons.mp hs with rfl | hs'
  · obtain ⟨-, hnb⟩ := (hiff triB).mp List.mem_cons_self
    exact hnb (by decide)
  rcases List.mem_cons.mp hs' with rfl | hs''
  · obtain ⟨-, hnb⟩ := (hiff triA).mp List.mem_cons_self
    exact hnb (by decide)
  · exact absurd hs'' List.not_mem_nil

/-- **The countermodel.** The loose execution of the paper-valid plan
emits `(1, 3, 4)` — `b` REBOUND from `S`'s fact, `R` never
re-checked — and the denotation refuses it: no fact `R(1, 3)` exists.
The paper's cover rule is unsound under never-revisit execution;
bumbledb's exactly-new-variables restriction
(`Exec/Plan.lean: PlanValid.covered`) is what `valid_plan_sound`
stands on, and the engine pins the same instance as a Rust
regression test (`40-execution.md` § the paper's core, the deviation
paragraph). -/
theorem loose_cover_rebinds (C : Query.Classify) (ρ : Query.ParamEnv) :
    [tri1, tri3, tri4]
        ∈ Query.looseAnswers C triRule triLoosePlan triInst ρ ∧
    [tri1, tri3, tri4] ∉ Query.ruleAnswers C triRule triInst ρ := by
  constructor
  · refine ⟨triLooseOut, ?_, ?_, ?_, ?_⟩
    · show triLooseOut ∈ Query.looseNodeStep triRule triInst ρ
        [[⟨0, [triA, triB]⟩]] [⟨1, [triB, triC]⟩, ⟨2, [triA, triC]⟩]
        (Query.looseNodeStep triRule triInst ρ []
          [⟨0, [triA, triB]⟩] fun _ => True)
      refine ⟨triMid, ?_, ⟨1, [triB, triC]⟩, List.mem_cons_self,
        ?_, ?_, ?_⟩
      · -- node 1: R's subatom covers itself, binding a = 1, b = 2
        refine ⟨triMid, trivial, ⟨0, [triA, triB]⟩,
          List.mem_cons_self, fun v hv => And.left hv,
          fun v _ => rfl, ?_⟩
        intro s hs
        rw [List.mem_singleton.mp hs]
        refine ⟨triAtomR, rfl, triFactR, Or.inl ⟨rfl, rfl⟩, ?_⟩
        intro b hb _
        rcases List.mem_cons.mp hb with rfl | hb'
        · exact (by decide : triMid triA = triFactR ⟨0⟩)
        rcases List.mem_cons.mp hb' with rfl | hb''
        · exact (by decide : triMid triB = triFactR ⟨1⟩)
        · exact absurd hb'' List.not_mem_nil
      · -- the paper cover: S's subatom contains the new variable c
        intro v hv
        obtain ⟨h1, h2⟩ := hv
        rcases List.mem_cons.mp h1 with rfl | h1'
        · exact List.mem_cons_self
        rcases List.mem_cons.mp h1' with rfl | h1''
        · exact List.mem_cons_of_mem _ List.mem_cons_self
        rcases List.mem_cons.mp h1'' with rfl | h1'''
        · exact absurd (by decide :
            triA ∈ Query.planVars [[⟨0, [triA, triB]⟩]]) h2
        rcases List.mem_cons.mp h1''' with rfl | h1''''
        · exact List.mem_cons_of_mem _ List.mem_cons_self
        · exact absurd h1'''' List.not_mem_nil
      · -- the REBIND: off the cover's variables the binding is kept
        intro v hv
        have hb : ¬ v.id = 1 := by
          intro h
          apply hv
          have : v = triB := by
            cases v
            simp only at h
            rw [h]
            rfl
          rw [this]
          exact List.mem_cons_self
        show triLooseOut v = if v.id = 1 then tri2 else triLooseOut v
        rw [if_neg hb]
      · -- node 2's probes: S and T both consistent with (1, 3, 4)
        intro s hs
        rcases List.mem_cons.mp hs with rfl | hs'
        · refine ⟨triAtomS, rfl, triFactS, Or.inr (Or.inl ⟨rfl, rfl⟩), ?_⟩
          intro b hb _
          rcases List.mem_cons.mp hb with rfl | hb'
          · exact (by decide : triLooseOut triB = triFactS ⟨0⟩)
          rcases List.mem_cons.mp hb' with rfl | hb''
          · exact (by decide : triLooseOut triC = triFactS ⟨1⟩)
          · exact absurd hb'' List.not_mem_nil
        rcases List.mem_cons.mp hs' with rfl | hs''
        · refine ⟨triAtomT, rfl, triFactT, Or.inr (Or.inr ⟨rfl, rfl⟩), ?_⟩
          intro b hb _
          rcases List.mem_cons.mp hb with rfl | hb'
          · exact (by decide : triLooseOut triA = triFactT ⟨0⟩)
          rcases List.mem_cons.mp hb' with rfl | hb''
          · exact (by decide : triLooseOut triC = triFactT ⟨1⟩)
          · exact absurd hb'' List.not_mem_nil
        · exact absurd hs'' List.not_mem_nil
    · intro a ha
      exact absurd ha List.not_mem_nil
    · intro c hc
      exact absurd hc List.not_mem_nil
    · exact (by decide : ([tri1, tri3, tri4] : List Value)
        = [triLooseOut triA, triLooseOut triB, triLooseOut triC])
  · intro hmem
    obtain ⟨σ, hder, hproj⟩ := Query.mem_ruleAnswers.mp hmem
    obtain ⟨hpos, -, -⟩ := hder
    have hproj' : [tri1, tri3, tri4] = [σ triA, σ triB, σ triC] := hproj
    injection hproj' with h1 hrest
    injection hrest with h2 hrest2
    obtain ⟨f, hf, hm⟩ := hpos triAtomR List.mem_cons_self
    rcases hf with ⟨-, rfl⟩ | ⟨habs, -⟩ | ⟨habs, -⟩
    · have hsel : σ triB = triFactR ⟨1⟩ :=
        hm (⟨1⟩, .var triB) (List.mem_cons_of_mem _ List.mem_cons_self)
      rw [← h2] at hsel
      exact absurd hsel (by decide)
    · exact absurd habs (by decide)
    · exact absurd habs (by decide)

/-! ## The E1 shape is UNINHABITABLE (the admission calculus, closed)

`joined_window_blast` above is the blast radius as data; this section
composes it against the acceptance gate's TYPE
(`Admission.lean: AdmissibleForm`): the E1 joined-window shape,
pinned at its own declared discipline — the joined judgment as the
`Judgment` field, the two joined relations as the consulted surfaces,
the parent-tag grouping as both surface projections — admits NO
`plan_decides` term. The argument is the blast's two-run reading: the
same delta over two pre-states (`blastPre` and the empty instance),
both holding the joined window, whose final states agree on EVERY
consultation at every delta-derived touched key (`touched_delta_
bounded` forces the keys; the only delta fact projects to `[keyVal]`,
and both parent-side consultations there are empty) — yet the joined
judgment is FALSE in one final state and TRUE in the other
(`joined_window_blast`'s two gained pairs). A verdict function of the
touched consultations would have to be both, so the field is empty:
"prohibitively expensive" is a type error, not an opinion.

Constructions local to this refutation: `listOracle` (a conforming
oracle over any listed fact set, trivial position order — the
countermodels' oracle builder; its filter decides tuple equality
through the eval machinery's `DecidableEq Value`,
`Query/Denotation.lean`), the open theory `admTheory` (every relation
open, so the denotation is the instance), and the two concrete oracle
families `blastOracle1`/`blastOracle2`.

The pins are the E1 shape's own declaration, not a loophole: a window
form groups parents by its grouping projection, and those are the
surfaces its plan may key — the same discipline `cardinalityForm`
(`Admission.lean`) inhabits successfully at one atom. Degenerate
groupings that scan a whole relation are refused by the gate's
acceptance rules on the docs side (`Admission.lean`'s recorded
narrowing), which is why the countermodel pins the grouping. -/

/-- Every list is pairwise-related under the trivial relation — the
countermodel oracles' order obligation. -/
theorem pairwise_trivial {α : Type} : ∀ l : List α,
    l.Pairwise (fun _ _ => True)
  | [] => List.Pairwise.nil
  | _ :: rest =>
    List.Pairwise.cons (fun _ _ => trivial) (pairwise_trivial rest)

/-- One group of a listed fact set, by filtered projection. -/
def groupList (L : List Fact) (proj : List FieldId)
    (t : List Value) : List Fact :=
  L.filter fun f => decide (f.project proj = t)

/-- Membership in a filtered group. -/
theorem mem_groupList {L : List Fact} {proj : List FieldId}
    {t : List Value} {f : Fact} :
    f ∈ groupList L proj t ↔ f ∈ L ∧ f.project proj = t := by
  unfold groupList
  rw [List.mem_filter]
  constructor
  · rintro ⟨h1, h2⟩
    exact ⟨h1, of_decide_eq_true h2⟩
  · rintro ⟨h1, h2⟩
    exact ⟨h1, decide_eq_true h2⟩

/-- A conforming oracle over any LISTED fact set, at the trivial
position order: consultation filters the list, and the neighbor reads
answer the group's head (extremal vacuously — every position relates
to every other). The countermodels' oracle builder. -/
def listOracle (A : Set Fact) (L : List Fact)
    (hmem : ∀ f, f ∈ A ↔ f ∈ L) (hnd : L.Nodup)
    (proj : List FieldId) :
    Oracle.OrderedOracle (List Value) Unit Fact (fun _ _ => True) where
  facts := A
  groupOf := fun f => f.project proj
  posOf := fun _ => ()
  consult := groupList L proj
  consult_mem := by
    intro g f
    constructor
    · intro h
      obtain ⟨h1, h2⟩ := mem_groupList.mp h
      exact ⟨(hmem f).mpr h1, h2⟩
    · rintro ⟨h1, h2⟩
      exact mem_groupList.mpr ⟨(hmem f).mp h1, h2⟩
  consult_nodup := fun _ => hnd.filter _
  consult_ordered := fun _ => pairwise_trivial _
  pred := fun g _ =>
    match groupList L proj g with
    | [] => none
    | f :: _ => some f
  succ := fun g _ =>
    match groupList L proj g with
    | [] => none
    | f :: _ => some f
  pred_mem := by
    intro g p f h
    have hf : f ∈ groupList L proj g := by
      revert h
      cases hL : groupList L proj g with
      | nil => intro h; exact nomatch h
      | cons a rest =>
        intro h
        have ha : a = f := Option.some.inj h
        rw [← ha]
        exact List.mem_cons_self ..
    obtain ⟨h1, h2⟩ := mem_groupList.mp hf
    exact ⟨(hmem f).mpr h1, h2, trivial⟩
  pred_greatest := fun _ _ _ _ _ _ _ _ => trivial
  pred_none := by
    intro g p hnone f hfacts hgrp _
    have hf : f ∈ groupList L proj g :=
      mem_groupList.mpr ⟨(hmem f).mp hfacts, hgrp⟩
    revert hnone
    cases hL : groupList L proj g with
    | nil =>
      rw [hL] at hf
      exact fun _ => nomatch hf
    | cons a rest => exact fun hnone => nomatch hnone
  succ_mem := by
    intro g p f h
    have hf : f ∈ groupList L proj g := by
      revert h
      cases hL : groupList L proj g with
      | nil => intro h; exact nomatch h
      | cons a rest =>
        intro h
        have ha : a = f := Option.some.inj h
        rw [← ha]
        exact List.mem_cons_self ..
    obtain ⟨h1, h2⟩ := mem_groupList.mp hf
    exact ⟨(hmem f).mpr h1, h2, trivial⟩
  succ_least := fun _ _ _ _ _ _ _ _ => trivial
  succ_none := by
    intro g p hnone f hfacts hgrp _
    have hf : f ∈ groupList L proj g :=
      mem_groupList.mpr ⟨(hmem f).mp hfacts, hgrp⟩
    revert hnone
    cases hL : groupList L proj g with
    | nil =>
      rw [hL] at hf
      exact fun _ => nomatch hf
    | cons a rest => exact fun hnone => nomatch hnone

/-- The empty window `0..0`: no joined pair per parent — the E1
declaration under refutation. -/
def joinedWindow : Window := ⟨0, some 0⟩

/-- The empty window holds of an empty set. -/
theorem window_admits_empty {α : Type} {s : Set α}
    (hempty : ∀ a, a ∉ s) : joinedWindow.admits s := by
  refine ⟨Set.atLeast_zero s, ?_⟩
  intro m _ l _ hmem
  cases l with
  | nil => exact Nat.zero_le m
  | cons a l' => exact absurd (hmem a (List.mem_cons_self ..)) (hempty a)

/-- The empty window refuses any inhabited set. -/
theorem window_refuses_inhabited {α : Type} {s : Set α} {a : α}
    (ha : a ∈ s) : ¬ joinedWindow.admits s := by
  rintro ⟨-, hup⟩
  have hle := hup 0 rfl [a]
    (List.Pairwise.cons (fun b hb => nomatch hb) List.Pairwise.nil)
    (fun b hb => by rcases List.mem_singleton.mp hb with rfl; exact ha)
  have h1 : (1 : Nat) ≤ 0 := hle
  omega

/-- The E1 judgment under refutation: at every parent tag, no joined
child pair — `joinedWindow` over `JoinedChildren`, the joined shape's
would-be denotation. -/
def joinedWindowJudgment (T : Theory) (I : Instance) : Prop :=
  ∀ t, joinedWindow.admits
    (JoinedChildren (T.den I parentRel) (T.den I childRel) t)

/-- The open theory of the refutation: every relation open, so the
denotation IS the instance (`admTheory_den`). -/
def admTheory : Theory := ⟨⟨fun _ => []⟩, fun _ => none, []⟩

/-- An open theory denotes the instance itself. -/
theorem admTheory_den (I : Instance) (R : RelId) :
    admTheory.den I R = I R :=
  rfl

/-- The second run's pre-instance: empty. -/
def blastEmpty : Instance := fun _ _ => False

/-- `blastPre`'s child side is empty. -/
theorem blastPre_child_empty : ∀ f, f ∉ blastPre childRel :=
  fun _ h => child_ne_parent h.1

/-- Run 1's final parent side lists exactly the two tagged rows. -/
theorem final_blast_parent_mem (f : Fact) :
    f ∈ admTheory.den (blastDelta.applyTo blastPre) parentRel ↔
      f ∈ [rowTrue, rowFalse] := by
  rw [admTheory_den]
  constructor
  · rintro (⟨⟨-, h⟩, -⟩ | ⟨hc, -⟩)
    · rcases h with rfl | rfl
      · exact List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
    · exact absurd hc.symm child_ne_parent
  · intro h
    rcases List.mem_cons.mp h with rfl | h
    · exact Or.inl ⟨⟨rfl, Or.inl rfl⟩, fun hf => hf⟩
    · rcases List.mem_singleton.mp h with rfl
      exact Or.inl ⟨⟨rfl, Or.inr rfl⟩, fun hf => hf⟩

/-- Either run's final child side lists exactly the inserted fact —
the pre-state child side is empty in both. -/
theorem final_blast_child_mem (I : Instance)
    (hI : ∀ f, f ∉ I childRel) (f : Fact) :
    f ∈ admTheory.den (blastDelta.applyTo I) childRel ↔
      f ∈ [winParent] := by
  rw [admTheory_den]
  constructor
  · rintro (⟨h, -⟩ | ⟨-, rfl⟩)
    · exact absurd h (hI f)
    · exact List.mem_cons_self ..
  · intro h
    rcases List.mem_singleton.mp h with rfl
    exact Or.inr ⟨rfl, rfl⟩

/-- Run 2's final parent side is empty — the delta writes only the
child relation. -/
theorem final_blastEmpty_parent_empty :
    ∀ f, f ∉ admTheory.den (blastDelta.applyTo blastEmpty) parentRel := by
  rintro f (⟨h, -⟩ | ⟨hc, -⟩)
  · exact h
  · exact child_ne_parent hc.symm

/-- The two tagged rows are distinct facts. -/
theorem nodup_pair : ([rowTrue, rowFalse] : List Fact).Nodup :=
  List.Pairwise.cons
    (fun _ hb => by
      rcases List.mem_singleton.mp hb with rfl
      exact rowTrue_ne_rowFalse)
    (List.Pairwise.cons (fun _ hb => nomatch hb) List.Pairwise.nil)

/-- A singleton fact list is duplicate-free. -/
theorem nodup_single : ([winParent] : List Fact).Nodup :=
  List.Pairwise.cons (fun _ hb => nomatch hb) List.Pairwise.nil

/-- Neither tagged row projects to the delta-derived key: the touched
consultation of run 1's parent oracle is EMPTY — exactly the blast's
first three conjuncts read at the filter. -/
theorem pair_group_keyVal_empty :
    groupList [rowTrue, rowFalse] blastGrp [keyVal] = [] := by
  unfold groupList
  refine List.filter_eq_nil_iff.mpr fun f hf => ?_
  intro hd
  have hproj := of_decide_eq_true hd
  rcases List.mem_cons.mp hf with rfl | hf
  · have h1 : ([valTrue] : List Value) = [keyVal] := hproj
    injection h1 with h2 _
    exact keyVal_ne_valTrue h2.symm
  · rcases List.mem_singleton.mp hf with rfl
    have h1 : ([valFalse] : List Value) = [keyVal] := hproj
    injection h1 with h2 _
    exact keyVal_ne_valFalse h2.symm

/-- Run 1's oracle family: the final state over `blastPre`. -/
def blastOracle1 : Bool →
    Oracle.OrderedOracle (List Value) Unit Fact (fun _ _ => True)
  | true =>
    listOracle (admTheory.den (blastDelta.applyTo blastPre) parentRel)
      [rowTrue, rowFalse] final_blast_parent_mem nodup_pair blastGrp
  | false =>
    listOracle (admTheory.den (blastDelta.applyTo blastPre) childRel)
      [winParent] (final_blast_child_mem blastPre blastPre_child_empty)
      nodup_single blastGrp

/-- Run 2's oracle family: the final state over the empty
pre-instance. -/
def blastOracle2 : Bool →
    Oracle.OrderedOracle (List Value) Unit Fact (fun _ _ => True)
  | true =>
    listOracle (admTheory.den (blastDelta.applyTo blastEmpty) parentRel)
      []
      (fun f => ⟨fun h => absurd h (final_blastEmpty_parent_empty f),
        fun h => nomatch h⟩)
      List.Pairwise.nil blastGrp
  | false =>
    listOracle (admTheory.den (blastDelta.applyTo blastEmpty) childRel)
      [winParent] (final_blast_child_mem blastEmpty fun _ h => h)
      nodup_single blastGrp

/-- **The E1 shape has no `AdmissibleForm` term** — the oracle-plan
field is uninhabitable at the shape's own discipline: the joined
judgment over the two joined relations, both keyed at the parent-tag
grouping. The two runs' touched consultations agree list for list
while the final judgments differ (`joined_window_blast`'s gained
pair), so `plan_decides` would convict and acquit one verdict — the
acceptance gate's cost law as a type error. -/
theorem joined_window_form_uninhabitable :
    ¬ ∃ F : Admission.AdmissibleForm Unit Bool,
      (∀ T I, F.Judgment () T I ↔ joinedWindowJudgment T I) ∧
      (∀ T I, F.surface () true T I = T.den I parentRel) ∧
      (∀ T I, F.surface () false T I = T.den I childRel) ∧
      F.surfaceProj () true = blastGrp ∧
      F.surfaceProj () false = blastGrp := by
  rintro ⟨F, hJ, hsp, hsc, hpp, hpc⟩
  -- the two conforming families
  have hfacts1 : ∀ ix, (blastOracle1 ix).facts =
      F.surface () ix admTheory (blastDelta.applyTo blastPre) := by
    intro ix
    cases ix
    · exact (hsc admTheory (blastDelta.applyTo blastPre)).symm
    · exact (hsp admTheory (blastDelta.applyTo blastPre)).symm
  have hkeys1 : ∀ ix f, (blastOracle1 ix).groupOf f =
      f.project (F.surfaceProj () ix) := by
    intro ix f
    cases ix
    · rw [hpc]; exact rfl
    · rw [hpp]; exact rfl
  have hfacts2 : ∀ ix, (blastOracle2 ix).facts =
      F.surface () ix admTheory (blastDelta.applyTo blastEmpty) := by
    intro ix
    cases ix
    · exact (hsc admTheory (blastDelta.applyTo blastEmpty)).symm
    · exact (hsp admTheory (blastDelta.applyTo blastEmpty)).symm
  have hkeys2 : ∀ ix f, (blastOracle2 ix).groupOf f =
      f.project (F.surfaceProj () ix) := by
    intro ix f
    cases ix
    · rw [hpc]; exact rfl
    · rw [hpp]; exact rfl
  have h1 := F.plan_decides () admTheory blastPre blastDelta Unit
    (fun _ _ => True) blastOracle1 hfacts1 hkeys1
  have h2 := F.plan_decides () admTheory blastEmpty blastDelta Unit
    (fun _ _ => True) blastOracle2 hfacts2 hkeys2
  -- every touched key is the one delta fact's projection
  have htouch : ∀ t, t ∈ F.Touched () blastDelta → t = [keyVal] := by
    intro t ht
    obtain ⟨ix, R, f, hf, hproj⟩ :=
      F.touched_delta_bounded () blastDelta t ht
    have hfw : f = winParent := by
      rcases hf with hf | hf
      · exact hf.2
      · exact hf.elim
    subst hfw
    cases ix
    · rw [hpc] at hproj
      exact hproj.symm
    · rw [hpp] at hproj
      exact hproj.symm
  -- the touched consultations agree across the runs
  have hcons : ∀ ix, (blastOracle1 ix).consult [keyVal] =
      (blastOracle2 ix).consult [keyVal] := by
    intro ix
    cases ix
    · rfl
    · show groupList [rowTrue, rowFalse] blastGrp [keyVal] =
        groupList [] blastGrp [keyVal]
      rw [pair_group_keyVal_empty]
      rfl
  have hans : ∀ t, t ∈ F.Touched () blastDelta →
      (fun ix => ((F.probe () ix).toPlan t).answers (blastOracle1 ix)) =
      (fun ix =>
        ((F.probe () ix).toPlan t).answers (blastOracle2 ix)) := by
    intro t ht
    funext ix
    rw [Admission.ProbeShape.toPlan_answers,
      Admission.ProbeShape.toPlan_answers, htouch t ht]
    exact hcons ix
  -- run 2: pre and final both hold, so the delta check passes
  have hpre2 : F.Judgment () admTheory blastEmpty :=
    (hJ admTheory blastEmpty).mpr fun t =>
      window_admits_empty fun pr hpr => hpr.1
  have hfin2 : F.Judgment () admTheory
      (blastDelta.applyTo blastEmpty) :=
    (hJ admTheory (blastDelta.applyTo blastEmpty)).mpr fun t =>
      window_admits_empty fun pr hpr =>
        final_blastEmpty_parent_empty pr.1 hpr.1
  have hdc2 : F.DeltaCheck () admTheory blastEmpty blastDelta :=
    (F.delta_restricts () admTheory blastEmpty blastDelta hpre2).mp
      hfin2
  -- transfer the verdicts to run 1, whose consultations agree
  have hlhs1 : ∀ t, t ∈ F.Touched () blastDelta →
      F.Verdict () blastDelta t
        (fun ix =>
          ((F.probe () ix).toPlan t).answers (blastOracle1 ix)) := by
    intro t ht
    rw [hans t ht]
    exact h2.mpr hdc2 t ht
  have hdc1 := h1.mp hlhs1
  -- run 1: pre holds, so the delta check forces the final judgment —
  -- which the gained joined pair refutes
  have hpre1 : F.Judgment () admTheory blastPre :=
    (hJ admTheory blastPre).mpr fun t =>
      window_admits_empty fun pr hpr =>
        blastPre_child_empty pr.2 hpr.2.1
  have hfin1 : F.Judgment () admTheory
      (blastDelta.applyTo blastPre) :=
    (F.delta_restricts () admTheory blastPre blastDelta hpre1).mpr
      hdc1
  have hjw := (hJ admTheory (blastDelta.applyTo blastPre)).mp hfin1
  exact window_refuses_inhabited
    joined_window_blast.2.2.2.2.2.1 (hjw [valTrue])

end Bumbledb.Countermodels
