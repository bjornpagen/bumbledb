import Bumbledb.Query.Aggregates
import Bumbledb.Exec.Sweep
import Bumbledb.Txn

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

* `stale_but_sound` — the maintenance protocol's freshness gap: a
  committed state (it `holds` its theory) whose derived relation is
  SOUND (its containment backs every derived fact — vacuously, here)
  yet STALE: the parent fact's derived copy never landed. No
  dependency statement can demand catch-up, so freshness is not a
  property of any committed state — it is host discipline (the
  `write_from` witness loop), exactly
  `Txn.derived_soundness_vs_freshness`'s other half.

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
    (keys : List Query.VarId)
    (foldRow : List Value → Set Query.Assignment → Query.AnswerTuple) :
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

end Bumbledb.Countermodels
