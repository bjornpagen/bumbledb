import Bumbledb.Subsumption
import Bumbledb.Txn
import Bumbledb.Exec.Sweep
import Bumbledb.Exec.Fixpoint

/-!
# Decide — the finite-instance judgment, decided (Level 1)

`holds T I` is Level 0's final-state judgment over arbitrary fact
sets; on finite, row-listed instances it is DECIDABLE, and this
module is that fact made executable: one Boolean checker per
statement form, each proved sound AND complete against its
`Statement.judgment` denotation (`funcB_iff`, `pointwiseKeyB_iff`,
`containB_iff`, `coverageB_iff`, `cardinalityB_iff`,
`orderMarkB_iff`, `rankedB_iff`), composed into the per-statement
dispatcher (`Statement.checkB`, `Statement.checkB_iff`), the
whole-theory executable judge (`holdsB`, `holdsB_iff_holds`, the
derived `decideJudgment`/`decideHolds`), and the two-phase
`Txn.judgeB` — key phase then statement phase, mirroring `Txn.judge`
and proved to agree with its verdict and its violation sets, phase
for phase, with NO instance-side premise beyond the merge: under the
hop-key rule's acceptance form (`RankKeysDeclared`, a fact of the
theory alone) the agreement covers every row instance, hop-key
violators included (`Txn.judgeB_agrees_of_declared`; the key phase
is chain-premise-free, `Txn.mem_keyViolationsB`, and a clean key
phase derives the semantic premise, `rankKeysHold_of_clean_keys`,
that the conditioned form `Txn.judgeB_agrees` spends together with
`Txn.mem_statementViolationsB`).

Placement recorded: the file sits at the tree root beside `Txn.lean`
because its subject is the WRITE-side judgment — statements,
theories, the commit judge — Level-1 work by the refinement chain (an
executable form proved equal to its denotation), but not a query
stage, so it does not live under `Exec/`.

## The carrier — rows, because fact identity must be decided

The finite carrier is `RowInstance`: per-relation fact lists whose
facts are given as VALUE ROWS — the conformance lane's interchange
shape (`Conformance.decodeFact` reads exactly this row form), denoted
through the machinery that already exists: `Query.tupleFact` (the
executable evaluator's tuple-fact reading, `Exec/Fixpoint.lean`) and
`Query.ListInstance` (the eval machinery's association-list world,
`Query/Denotation.lean`) — `RowInstance.world` / `RowInstance.den`
reuse both, never duplicate them. Rows rather than bare `Fact`
functions is forced, not stylistic: `Fact` is `FieldId → Value` with
no decidable equality, and the key and order judgments CONCLUDE fact
equality — a row's finite support is what makes tuple-fact equality
decidable (`rowEqB_iff`: agreement on the shared index range, filler
beyond it). Recorded narrowing: the checkers judge instances whose
facts are row-denoted — exactly the conformance corpus's world shape.

## The premises — acceptance enters as hypotheses (the tree's rule)

* **`WorldCarriesClosed`** — the world carries each sealed ground
  roster at its relation (the conformance lane's merge:
  `Conformance.decodeCase` appends the ground axioms into the world).
  Under it, `Theory.den` reads every relation through the row lists
  (`theoryDen_denotes`). A sealed `GroundExtension` is a `Fact` list,
  not a row list, so its members' equality is not decidable in place;
  the merge premise is the honest boundary, and it is what the lane
  already does.
* **`RankKeysHold`** — every declared `by` chain's hop relation is
  keyed on the hop's key field, semantically, on the judged instance:
  the ranked checker reads ranks through the deterministic list probe
  (`chainEvalL`), and `chain_eval_deterministic`
  (`Subsumption.lean`) is what licenses that function reading of the
  relational `chainEval` — `chainEvalL_complete` spends it hop by
  hop. This is the ranked form's acceptance rule spent semantically,
  carried as a hypothesis wherever a theorem needs it, never a
  conjunct of any denotation — `Dependencies.lean`'s discipline,
  unchanged. The premise fails on exactly the instances that violate
  a declared rank-hop key, so nothing conditioned on it says
  anything there — which is why the TWO-PHASE agreement does not
  stop at it: `RankKeysDeclared` is the same rule in acceptance form
  (the hop key is a DECLARED scalar functionality statement), an
  instance-free premise under which the agreement is total — a
  hop-key violator convicts in the chain-premise-free key phase on
  both sides, and a clean key phase derives `RankKeysHold`
  (`rankKeysHold_of_clean_keys`).

## The sweep is spent where union coverage demands it

`coverageB` runs the PROVED sweep: `Exec.sweepCovered` over
`sortByStart` — soundness premise-free (`sweep_never_false_accepts`),
completeness from the sort's start order
(`sweep_complete_of_ordered` + `pairwise_sortByStart`) — because
per-point coverage by a UNION of target segments is not a pairwise
fact. The pointwise KEY checker is pairwise value-level disjointness
(`pointsDisjointB_iff`, two boundary comparisons per element domain)
— the shape of `pointwise_key_disjoint`; the engine's sorted
neighbor probe (`Applier::probe_neighbors`) is mechanism below this
altitude, recorded in the Bridge rows that already exist.

## Third oracle, write side — discharged

The conformance lane runs a judgment arm: `lake exe conformance`
dispatches `judgment-*.json` cases (`lean/Main.lean`) to `Txn.judgeB`
over `(theory, instance, delta)` documents serialized by
`crates/bumbledb-bench/src/conformance/judgment.rs`, comparing the
verdict and the per-phase violation sets against what the engine and
the naive model agreed on — engine verdict vs naive verdict vs this
judge, the write-side third oracle. `Bridge.lean` carries the rows
(`holdsB_iff_holds`, `Txn.judgeB_agrees_of_declared`); the corpus
format lives in `lean/conformance/README.md` § judgment cases.

## Narrowings recorded (law 5: narrow and record)

* The row carrier and the two premises — above.
* `Txn.judgeB` returns `Option (List Statement)`: `none` accepts; a
  rejection's LIST may repeat a statement the theory declares twice.
  Agreement with `Txn.judge` is stated as membership equality with
  the violation SETS — `Txn.lean`'s own recorded narrowing (a set
  carries no duplicates or order by construction) applied to the
  executable face.
* Decidability lands as premise-carrying named `def`s
  (`decideJudgment`, `decideHolds`), never `instance`s: the premises
  are per-theory semantic facts instance resolution cannot see.
* The interval checkers are stated per element domain (`U64`/`I64`
  concretely) — the tree's precedent (`encode_interval_order` and its
  U64 companion): an abstract order class would buy generality no
  third domain spends. The two-conjunct shape of `pointsDisjointB`
  and `coverRowB` (a u64 arm AND an i64 arm, each vacuously true on
  the other domain) keeps the checkers total with no typing premise —
  the same totalization move as `Value.points`.
* `orderMarkB`'s downward-closure clause enumerates `1..k` for each
  attained position `k` (`List.range'`) — executable and exact
  against `OrdinalGroup.closed`; the count is data-dependent, and
  cost is not this file's subject (law 3: abstract cost lives in
  `Oracle.lean`, measured cost in the docs).
-/

namespace Bumbledb

/-! ## Boolean helpers -/

/-- The Boolean implication reading of `!x || y` — the shape every
guarded checker clause takes. -/
theorem impB_iff {x y : Bool} : (!x || y) = true ↔ (x = true → y = true) := by
  cases x <;> cases y <;> simp

/-- Conjunction verdicts split — pinned locally so the file never
leans on a lemma-name accident of the core library. -/
theorem andB_iff {x y : Bool} : (x && y) = true ↔ x = true ∧ y = true := by
  cases x <;> cases y <;> simp

/-- Disjunction verdicts split. -/
theorem orB_iff {x y : Bool} : (x || y) = true ↔ x = true ∨ y = true := by
  cases x <;> cases y <;> simp

/-! ## Rows — decidable tuple-fact identity -/

/-- A row: one stored fact as data — the conformance lane's fact
shape, denoted through `Query.tupleFact`. -/
abbrev Row : Type := List Value

/-- Decidable tuple-fact equality: two rows denote one fact exactly
when they agree on every index of the shared range (beyond both
lengths each side reads the filler, so the range check is the whole
fact). -/
def rowEqB (a b : Row) : Bool :=
  (List.range (max a.length b.length)).all fun n =>
    decide (Query.tupleFact a ⟨n⟩ = Query.tupleFact b ⟨n⟩)

/-- `rowEqB` decides tuple-fact identity — the finite support cashed:
inside the shared range the check is fieldwise, beyond it both facts
read `fillerValue`. -/
theorem rowEqB_iff {a b : Row} :
    rowEqB a b = true ↔ Query.tupleFact a = Query.tupleFact b := by
  constructor
  · intro h
    funext i
    obtain ⟨n⟩ := i
    by_cases hn : n < max a.length b.length
    · exact of_decide_eq_true
        (List.all_eq_true.mp h n (List.mem_range.mpr hn))
    · have hmax : max a.length b.length ≤ n := Nat.le_of_not_lt hn
      have ha : a.length ≤ n :=
        Nat.le_trans (Nat.le_max_left a.length b.length) hmax
      have hb : b.length ≤ n :=
        Nat.le_trans (Nat.le_max_right a.length b.length) hmax
      show (a[n]?).getD Query.fillerValue = (b[n]?).getD Query.fillerValue
      rw [List.getElem?_eq_none ha, List.getElem?_eq_none hb]
  · intro h
    refine List.all_eq_true.mpr fun n _ => decide_eq_true ?_
    rw [h]

/-! ## The finite-instance carrier -/

/-- A finite instance as per-relation ROW lists — the conformance
lane's world shape, denoted through the eval machinery's
`Query.ListInstance` (reused, not duplicated: `RowInstance.world`). -/
structure RowInstance where
  /-- The relation extensions, as rows. -/
  rels : List (RelId × List Row)

/-- One relation's rows (first entry wins; a missing relation is
empty — `Query.ListInstance.facts`'s convention). -/
def RowInstance.rows (W : RowInstance) (R : RelId) : List Row :=
  match W.rels.find? fun e => e.1 == R with
  | some e => e.2
  | none => []

/-- The `ListInstance` a row instance denotes — each row read as its
tuple-fact. -/
def RowInstance.world (W : RowInstance) : Query.ListInstance :=
  ⟨W.rels.map fun e => (e.1, e.2.map Query.tupleFact)⟩

/-- The instance a row instance denotes — through the eval
machinery's own denotation. -/
def RowInstance.den (W : RowInstance) : Instance :=
  W.world.den

/-- The list-level engine of `RowInstance.facts_eq`: the mapped
world's association lookup is the row lookup, mapped. -/
theorem findMap_aux (R : RelId) : ∀ rels : List (RelId × List Row),
    (match (rels.map fun e : RelId × List Row => (e.1, e.2.map Query.tupleFact)).find?
        (fun e => e.1 == R) with
     | some e => e.2
     | none => []) =
    (match rels.find? (fun e : RelId × List Row => e.1 == R) with
     | some e => e.2
     | none => []).map Query.tupleFact
  | [] => rfl
  | e :: rest => by
    simp only [List.map_cons, List.find?]
    cases hb : e.1 == R with
    | true => rfl
    | false => exact findMap_aux R rest

/-- The world's fact lists are the row lists, tuple-fact for row. -/
theorem RowInstance.facts_eq (W : RowInstance) (R : RelId) :
    W.world.facts R = (W.rows R).map Query.tupleFact :=
  findMap_aux R W.rels

/-- Membership in a row instance's denotation: some listed row
denotes the fact. -/
theorem RowInstance.mem_den (W : RowInstance) (R : RelId) (f : Fact) :
    f ∈ W.den R ↔ ∃ r, r ∈ W.rows R ∧ f = Query.tupleFact r := by
  show f ∈ W.world.facts R ↔ _
  rw [RowInstance.facts_eq, List.mem_map]
  exact exists_congr fun r => and_congr_right fun _ => eq_comm

/-- `L` lists `A`: the row list enumerates the fact set, tuple-fact
for row — the enumeration premise every checker theorem consumes. -/
def Denotes (L : List Row) (A : Set Fact) : Prop :=
  ∀ f, f ∈ A ↔ ∃ r, r ∈ L ∧ f = Query.tupleFact r

/-- The world carries each sealed ground roster at its relation — the
conformance lane's merge (`Conformance.decodeCase` appends the ground
axioms into the world), as the premise it is. -/
def WorldCarriesClosed (T : Theory) (W : RowInstance) : Prop :=
  ∀ R ext, T.closed R = some ext → ∀ f : Fact, f ∈ ext.facts ↔ f ∈ W.den R

/-- Under the merge premise, the theory-side denotation of EVERY
relation — closed or open — is listed by the world's rows. -/
theorem theoryDen_denotes {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) (R : RelId) :
    Denotes (W.rows R) (T.den W.den R) := by
  intro f
  unfold Theory.den
  cases hc : T.closed R with
  | some ext => exact (hclosed R ext hc f).trans (W.mem_den R f)
  | none => exact W.mem_den R f

/-! ## Selections, decided -/

/-- σ over a row, decided: every binding's field carries a member of
its literal set. -/
def satisfiesB (φ : Selection) (r : Row) : Bool :=
  φ.bindings.all fun b => decide (Query.tupleFact r b.1 ∈ b.2)

theorem satisfiesB_iff {φ : Selection} {r : Row} :
    satisfiesB φ r = true ↔ φ.satisfies (Query.tupleFact r) := by
  unfold satisfiesB Selection.satisfies
  rw [List.all_eq_true]
  exact forall_congr' fun b => forall_congr' fun _ => decide_eq_true_iff

/-! ## Counting — distinct facts of a row list

`Set.AtLeast`/`Set.AtMost` (`Cardinality.lean`) are list-witnessed
bounds; on a row-listed set both collapse to one number: the length
of a duplicate-free enumeration. -/

/-- A duplicate-free list confined to another list is no longer —
the pigeonhole the count bounds ride (classical erase; the proof is
a `Prop`, so decidable equality is summoned, never demanded). -/
theorem length_le_of_nodup_of_subset {α : Type u} :
    ∀ {l enum : List α}, l.Nodup → (∀ a, a ∈ l → a ∈ enum) →
      l.length ≤ enum.length
  | [], _, _, _ => Nat.zero_le _
  | a :: l, enum, hnd, hsub => by
    haveI : DecidableEq α := fun x y => Classical.propDecidable _
    have hmem : a ∈ enum := hsub a (List.mem_cons_self ..)
    have hnd' := List.nodup_cons.mp hnd
    have hsub' : ∀ x, x ∈ l → x ∈ enum.erase a := fun x hx =>
      (List.mem_erase_of_ne fun heq => hnd'.1 (by rw [← heq]; exact hx)).mpr
        (hsub x (List.mem_cons_of_mem a hx))
    have hlen := length_le_of_nodup_of_subset hnd'.2 hsub'
    have herase : (enum.erase a).length = enum.length - 1 :=
      List.length_erase_of_mem hmem
    have hpos : 0 < enum.length := List.length_pos_of_mem hmem
    show l.length + 1 ≤ enum.length
    omega

/-- On an enumerated set, the floor bound is a length compare. -/
theorem atLeast_iff_enum_length {α : Type u} {s : Set α}
    {enum : List α} (hnd : enum.Nodup)
    (hmem : ∀ a, a ∈ s ↔ a ∈ enum) (n : Nat) :
    s.AtLeast n ↔ n ≤ enum.length := by
  constructor
  · rintro ⟨l, hnd', hsub, hlen⟩
    exact Nat.le_trans hlen (length_le_of_nodup_of_subset hnd'
      fun a ha => (hmem a).mp (hsub a ha))
  · intro h
    exact ⟨enum, hnd, fun a ha => (hmem a).mpr ha, h⟩

/-- On an enumerated set, the ceiling bound is a length compare. -/
theorem atMost_iff_enum_length {α : Type u} {s : Set α}
    {enum : List α} (hnd : enum.Nodup)
    (hmem : ∀ a, a ∈ s ↔ a ∈ enum) (m : Nat) :
    s.AtMost m ↔ enum.length ≤ m := by
  constructor
  · intro h
    exact h enum hnd fun a ha => (hmem a).mpr ha
  · intro h l hnd' hsub
    exact Nat.le_trans (length_le_of_nodup_of_subset hnd'
      fun a ha => (hmem a).mp (hsub a ha)) h

/-- One representative row per denoted fact (keep-last under
`rowEqB`) — the duplicate-free enumeration the count bounds read. -/
def dedupFacts : List Row → List Row
  | [] => []
  | r :: rest =>
    if rest.any fun r' => rowEqB r r' then dedupFacts rest
    else r :: dedupFacts rest

theorem dedupFacts_subset : ∀ {l : List Row} {r : Row},
    r ∈ dedupFacts l → r ∈ l
  | r' :: rest, r, h => by
    unfold dedupFacts at h
    by_cases hc : (rest.any fun x => rowEqB r' x) = true
    · rw [if_pos hc] at h
      exact List.mem_cons_of_mem _ (dedupFacts_subset h)
    · rw [if_neg hc] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (dedupFacts_subset h')

/-- Deduplication drops no denoted fact. -/
theorem mem_map_dedupFacts : ∀ {l : List Row} {f : Fact},
    f ∈ (dedupFacts l).map Query.tupleFact ↔ f ∈ l.map Query.tupleFact
  | [], _ => Iff.rfl
  | r :: rest, f => by
    unfold dedupFacts
    by_cases hc : (rest.any fun x => rowEqB r x) = true
    · rw [if_pos hc]
      rw [mem_map_dedupFacts (l := rest)]
      constructor
      · intro h
        exact List.mem_cons_of_mem _ h
      · intro h
        rcases List.mem_cons.mp
          (show f ∈ Query.tupleFact r :: rest.map Query.tupleFact
            from h) with rfl | h'
        · obtain ⟨r', hr', heq⟩ := List.any_eq_true.mp hc
          exact List.mem_map.mpr ⟨r', hr', (rowEqB_iff.mp heq).symm⟩
        · exact h'
    · rw [if_neg hc]
      show f ∈ Query.tupleFact r :: (dedupFacts rest).map Query.tupleFact
        ↔ f ∈ Query.tupleFact r :: rest.map Query.tupleFact
      rw [List.mem_cons, List.mem_cons, mem_map_dedupFacts (l := rest)]

/-- The deduplicated rows denote pairwise-distinct facts. -/
theorem nodup_map_dedupFacts : ∀ (l : List Row),
    ((dedupFacts l).map Query.tupleFact).Nodup
  | [] => List.Pairwise.nil
  | r :: rest => by
    unfold dedupFacts
    by_cases hc : (rest.any fun x => rowEqB r x) = true
    · rw [if_pos hc]
      exact nodup_map_dedupFacts rest
    · rw [if_neg hc]
      show (Query.tupleFact r ::
        (dedupFacts rest).map Query.tupleFact).Nodup
      refine List.nodup_cons.mpr ⟨?_, nodup_map_dedupFacts rest⟩
      intro hmem
      obtain ⟨r', hr', heq⟩ := List.mem_map.mp hmem
      exact hc (List.any_eq_true.mpr
        ⟨r', dedupFacts_subset hr', rowEqB_iff.mpr heq.symm⟩)

/-! ## Functionality (scalar) -/

/-- The scalar-key checker: no two listed rows agree on the
determinant projection without denoting one fact. -/
def funcB (L : List Row) (X : List FieldId) : Bool :=
  L.all fun a => L.all fun b =>
    !decide ((Query.tupleFact a).project X = (Query.tupleFact b).project X)
      || rowEqB a b

/-- `funcB` decides `Functionality` on the listed denotation. -/
theorem funcB_iff {L : List Row} {A : Set Fact} (hA : Denotes L A)
    (X : List FieldId) :
    funcB L X = true ↔ Functionality A X := by
  unfold funcB Functionality
  rw [List.all_eq_true]
  constructor
  · intro h f g hf hg hproj
    obtain ⟨a, ha, rfl⟩ := (hA f).mp hf
    obtain ⟨b, hb, rfl⟩ := (hA g).mp hg
    have h1 := List.all_eq_true.mp (h a ha) b hb
    rw [impB_iff] at h1
    exact rowEqB_iff.mp (h1 (decide_eq_true hproj))
  · intro h a ha
    refine List.all_eq_true.mpr fun b hb => ?_
    rw [impB_iff]
    intro hproj
    exact rowEqB_iff.mpr (h _ _ ((hA _).mpr ⟨a, ha, rfl⟩)
      ((hA _).mpr ⟨b, hb, rfl⟩) (of_decide_eq_true hproj))

/-! ## The pointwise readings — points, decided per element domain -/

/-- A `u64`-tagged point of a value's point-family is a point of the
`Interval U64` it carries — the one inversion every interval checker
walks (its `i64` twin below); scalar values and the other domain
carry no `u64` points. -/
theorem mem_points_u64 (v : Value) (x : U64) :
    Point.u64 x ∈ v.points ↔
      ∃ iv, v.intervalU64 = some iv ∧ x ∈ iv.points := by
  obtain ⟨t, val⟩ := v
  cases t with
  | bool => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | u64 => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | i64 => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | str => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | fixedBytes n => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | interval e =>
    cases e with
    | u64 =>
      constructor
      · intro h
        exact ⟨val, rfl, h⟩
      · rintro ⟨iv, hiv, hx⟩
        have heq : val = iv := Option.some.inj hiv
        subst heq
        exact hx
    | i64 => exact ⟨fun h => (nomatch h),
        by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩

/-- The `i64` twin of `mem_points_u64`. -/
theorem mem_points_i64 (v : Value) (x : I64) :
    Point.i64 x ∈ v.points ↔
      ∃ iv, v.intervalI64 = some iv ∧ x ∈ iv.points := by
  obtain ⟨t, val⟩ := v
  cases t with
  | bool => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | u64 => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | i64 => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | str => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | fixedBytes n => exact ⟨fun h => (nomatch h),
      by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
  | interval e =>
    cases e with
    | u64 => exact ⟨fun h => (nomatch h),
        by rintro ⟨iv, hiv, -⟩; exact nomatch hiv⟩
    | i64 =>
      constructor
      · intro h
        exact ⟨val, rfl, h⟩
      · rintro ⟨iv, hiv, hx⟩
        have heq : val = iv := Option.some.inj hiv
        subst heq
        exact hx

/-- Half-open interval disjointness, decided: one ends at or before
the other starts. -/
def ivDisjointB {α : Type} [LT α] [LE α] [DecidableLE α]
    (iv jv : Interval α) : Bool :=
  decide (iv.«end» ≤ jv.start) || decide (jv.«end» ≤ iv.start)

/-- The two boundary comparisons decide point disjointness over
`U64`: sound by half-open arithmetic, complete because two
overlapping nonempty intervals share the later start. -/
theorem ivDisjointB_iff_u64 (iv jv : Interval U64) :
    ivDisjointB iv jv = true ↔
      ∀ x : U64, x ∈ iv.points → x ∉ jv.points := by
  unfold ivDisjointB
  rw [orB_iff, decide_eq_true_iff, decide_eq_true_iff]
  constructor
  · intro h x hx hw
    have h1 : iv.start.val ≤ x.val := hx.1
    have h2 : x.val < iv.«end».val := hx.2
    have h3 : jv.start.val ≤ x.val := hw.1
    have h4 : x.val < jv.«end».val := hw.2
    rcases h with h | h
    · have h5 : iv.«end».val ≤ jv.start.val := h
      omega
    · have h5 : jv.«end».val ≤ iv.start.val := h
      omega
  · intro h
    by_cases h1 : iv.«end» ≤ jv.start
    · exact Or.inl h1
    · refine Or.inr ?_
      by_cases h2 : jv.«end» ≤ iv.start
      · exact h2
      · exfalso
        have h1' : ¬ iv.«end».val ≤ jv.start.val := h1
        have h2' : ¬ jv.«end».val ≤ iv.start.val := h2
        by_cases h3 : iv.start.val ≤ jv.start.val
        · exact h jv.start
            ⟨h3, show jv.start.val < iv.«end».val by omega⟩
            ⟨Nat.le_refl jv.start.val, jv.h⟩
        · exact h iv.start ⟨Nat.le_refl iv.start.val, iv.h⟩
            ⟨show jv.start.val ≤ iv.start.val by omega,
             show iv.start.val < jv.«end».val by omega⟩

/-- The `i64` twin of `ivDisjointB_iff_u64`. -/
theorem ivDisjointB_iff_i64 (iv jv : Interval I64) :
    ivDisjointB iv jv = true ↔
      ∀ x : I64, x ∈ iv.points → x ∉ jv.points := by
  unfold ivDisjointB
  rw [orB_iff, decide_eq_true_iff, decide_eq_true_iff]
  constructor
  · intro h x hx hw
    have h1 : iv.start.val ≤ x.val := hx.1
    have h2 : x.val < iv.«end».val := hx.2
    have h3 : jv.start.val ≤ x.val := hw.1
    have h4 : x.val < jv.«end».val := hw.2
    rcases h with h | h
    · have h5 : iv.«end».val ≤ jv.start.val := h
      omega
    · have h5 : jv.«end».val ≤ iv.start.val := h
      omega
  · intro h
    by_cases h1 : iv.«end» ≤ jv.start
    · exact Or.inl h1
    · refine Or.inr ?_
      by_cases h2 : jv.«end» ≤ iv.start
      · exact h2
      · exfalso
        have h1' : ¬ iv.«end».val ≤ jv.start.val := h1
        have h2' : ¬ jv.«end».val ≤ iv.start.val := h2
        by_cases h3 : iv.start.val ≤ jv.start.val
        · exact h jv.start
            ⟨h3, show jv.start.val < iv.«end».val by omega⟩
            ⟨Int.le_refl jv.start.val, jv.h⟩
        · exact h iv.start ⟨Int.le_refl iv.start.val, iv.h⟩
            ⟨show jv.start.val ≤ iv.start.val by omega,
             show iv.start.val < jv.«end».val by omega⟩

/-- Point-family disjointness of two VALUES, decided: a u64 arm and
an i64 arm, each vacuous off its domain (the `Value.points`
totalization move) — scalar values and cross-domain pairs share no
point by construction. -/
def pointsDisjointB (v w : Value) : Bool :=
  (match v.intervalU64, w.intervalU64 with
   | some iv, some jv => ivDisjointB iv jv
   | _, _ => true) &&
  (match v.intervalI64, w.intervalI64 with
   | some iv, some jv => ivDisjointB iv jv
   | _, _ => true)

/-- `pointsDisjointB` decides disjointness of the point-families. -/
theorem pointsDisjointB_iff (v w : Value) :
    pointsDisjointB v w = true ↔
      ∀ x, x ∈ v.points → x ∉ w.points := by
  unfold pointsDisjointB
  constructor
  · intro h x hx hw
    obtain ⟨h1, h2⟩ := andB_iff.mp h
    cases x with
    | u64 y =>
      obtain ⟨iv, hiv, hy⟩ := (mem_points_u64 v y).mp hx
      obtain ⟨jv, hjv, hy'⟩ := (mem_points_u64 w y).mp hw
      rw [hiv, hjv] at h1
      exact (ivDisjointB_iff_u64 iv jv).mp h1 y hy hy'
    | i64 y =>
      obtain ⟨iv, hiv, hy⟩ := (mem_points_i64 v y).mp hx
      obtain ⟨jv, hjv, hy'⟩ := (mem_points_i64 w y).mp hw
      rw [hiv, hjv] at h2
      exact (ivDisjointB_iff_i64 iv jv).mp h2 y hy hy'
  · intro h
    refine andB_iff.mpr ⟨?_, ?_⟩
    · cases hiv : v.intervalU64 with
      | none => rfl
      | some iv =>
        cases hjv : w.intervalU64 with
        | none => rfl
        | some jv =>
          refine (ivDisjointB_iff_u64 iv jv).mpr fun y hy hy' => ?_
          exact h (.u64 y) ((mem_points_u64 v y).mpr ⟨iv, hiv, hy⟩)
            ((mem_points_u64 w y).mpr ⟨jv, hjv, hy'⟩)
    · cases hiv : v.intervalI64 with
      | none => rfl
      | some iv =>
        cases hjv : w.intervalI64 with
        | none => rfl
        | some jv =>
          refine (ivDisjointB_iff_i64 iv jv).mpr fun y hy hy' => ?_
          exact h (.i64 y) ((mem_points_i64 v y).mpr ⟨iv, hiv, hy⟩)
            ((mem_points_i64 w y).mpr ⟨jv, hjv, hy'⟩)

/-! ## Functionality (pointwise) -/

/-- The pointwise-key checker: within a scalar group, two rows
denoting distinct facts carry disjoint interval positions — the
pairwise reading of `pointwise_key_disjoint` (the engine's sorted
neighbor probe is mechanism below this altitude). -/
def pointwiseKeyB (L : List Row) (S : List FieldId) (i : FieldId) : Bool :=
  L.all fun a => L.all fun b =>
    !decide ((Query.tupleFact a).project S = (Query.tupleFact b).project S)
      || (rowEqB a b ||
          pointsDisjointB (Query.tupleFact a i) (Query.tupleFact b i))

/-- `pointwiseKeyB` decides `PointwiseKey` on the listed denotation. -/
theorem pointwiseKeyB_iff {L : List Row} {A : Set Fact} (hA : Denotes L A)
    (S : List FieldId) (i : FieldId) :
    pointwiseKeyB L S i = true ↔ PointwiseKey A S i := by
  unfold pointwiseKeyB PointwiseKey
  rw [List.all_eq_true]
  constructor
  · intro h f g hf hg hproj hne x hxf hxg
    obtain ⟨a, ha, rfl⟩ := (hA f).mp hf
    obtain ⟨b, hb, rfl⟩ := (hA g).mp hg
    have h1 := List.all_eq_true.mp (h a ha) b hb
    rw [impB_iff] at h1
    rcases orB_iff.mp (h1 (decide_eq_true hproj)) with h2 | h2
    · exact hne (rowEqB_iff.mp h2)
    · exact (pointsDisjointB_iff _ _).mp h2 x hxf hxg
  · intro h a ha
    refine List.all_eq_true.mpr fun b hb => ?_
    rw [impB_iff]
    intro hproj
    by_cases heq : rowEqB a b = true
    · exact orB_iff.mpr (Or.inl heq)
    · refine orB_iff.mpr (Or.inr ?_)
      refine (pointsDisjointB_iff _ _).mpr fun x hx hx' => ?_
      exact h _ _ ((hA _).mpr ⟨a, ha, rfl⟩) ((hA _).mpr ⟨b, hb, rfl⟩)
        (of_decide_eq_true hproj)
        (fun hfg => heq (rowEqB_iff.mpr hfg)) x hx hx'

/-! ## Containment (scalar) -/

/-- The scalar containment checker: every selected source row has a
selected target row with the same projected tuple. -/
def containB (LA : List Row) (φ : Selection) (X : List FieldId)
    (LB : List Row) (ψ : Selection) (Y : List FieldId) : Bool :=
  LA.all fun a => !satisfiesB φ a ||
    LB.any fun b => satisfiesB ψ b &&
      decide ((Query.tupleFact b).project Y = (Query.tupleFact a).project X)

/-- `containB` decides `Containment` on the listed denotations. -/
theorem containB_iff {LA LB : List Row} {A B : Set Fact}
    (hA : Denotes LA A) (hB : Denotes LB B) (φ : Selection)
    (X : List FieldId) (ψ : Selection) (Y : List FieldId) :
    containB LA φ X LB ψ Y = true ↔ Containment A φ X B ψ Y := by
  unfold containB Containment
  rw [List.all_eq_true]
  constructor
  · intro h f hfA hfφ
    obtain ⟨a, ha, rfl⟩ := (hA f).mp hfA
    have h1 := h a ha
    rw [impB_iff] at h1
    obtain ⟨b, hb, hcond⟩ := List.any_eq_true.mp
      (h1 (satisfiesB_iff.mpr hfφ))
    obtain ⟨h2, h3⟩ := andB_iff.mp hcond
    exact ⟨Query.tupleFact b, (hB _).mpr ⟨b, hb, rfl⟩,
      satisfiesB_iff.mp h2, of_decide_eq_true h3⟩
  · intro h a ha
    rw [impB_iff]
    intro hφ
    obtain ⟨g, hgB, hgψ, hgY⟩ := h (Query.tupleFact a)
      ((hA _).mpr ⟨a, ha, rfl⟩) (satisfiesB_iff.mp hφ)
    obtain ⟨b, hb, rfl⟩ := (hB g).mp hgB
    exact List.any_eq_true.mpr ⟨b, hb, andB_iff.mpr
      ⟨satisfiesB_iff.mpr hgψ, decide_eq_true hgY⟩⟩

/-! ## Coverage — the sweep, spent -/

/-- Union coverage of one source interval by a segment list, decided
by the PROVED sweep (`Exec.sweepCovered` over the start sort):
soundness needs no premise, completeness rides the sort's order. -/
def coveredB {α : Type} [LT α] [LE α] [LinearElem α] [DecidableLT α]
    [DecidableLE α] (src : Interval α) (segs : List (Interval α)) : Bool :=
  Exec.sweepCovered src (sortByStart segs)

/-- `coveredB` decides union coverage — `sweep_never_false_accepts`
(sound, premise-free) and `sweep_complete_of_ordered` +
`pairwise_sortByStart` (complete), with `mem_sortByStart` carrying
membership across the sort. -/
theorem coveredB_iff {α : Type} [LT α] [LE α] [LinearElem α]
    [DecidableLT α] [DecidableLE α] (src : Interval α)
    (segs : List (Interval α)) :
    coveredB src segs = true ↔
      ∀ x, x ∈ src.points → x ∈ unionPoints segs := by
  constructor
  · intro h x hx
    have hx' : src.start ≤ x ∧ x < src.«end» := hx
    have h' : Exec.sweepFrom src.«end» src.start (sortByStart segs) =
        true := h
    obtain ⟨jv, hjv, hxj⟩ :=
      Exec.sweep_never_false_accepts h' x hx'.1 hx'.2
    exact ⟨jv, mem_sortByStart.mp hjv, hxj⟩
  · intro h
    show Exec.sweepFrom src.«end» src.start (sortByStart segs) = true
    refine Exec.sweep_complete_of_ordered (pairwise_sortByStart segs)
      fun x hsx hxe => ?_
    obtain ⟨jv, hjv, hxj⟩ := h x ⟨hsx, hxe⟩
    exact ⟨jv, mem_sortByStart.mpr hjv, hxj⟩

/-- One source row's coverage verdict: the same-group selected target
segments must cover its interval position — a u64 arm and an i64
arm, each vacuous off its domain. -/
def coverRowB (LB : List Row) (ψ : Selection) (U : List FieldId)
    (j : FieldId) (S : List FieldId) (i : FieldId) (a : Row) : Bool :=
  (match (Query.tupleFact a i).intervalU64 with
   | some iv =>
     coveredB iv ((LB.filter fun b => satisfiesB ψ b &&
         decide ((Query.tupleFact b).project U =
           (Query.tupleFact a).project S)).filterMap
       fun b => (Query.tupleFact b j).intervalU64)
   | none => true) &&
  (match (Query.tupleFact a i).intervalI64 with
   | some iv =>
     coveredB iv ((LB.filter fun b => satisfiesB ψ b &&
         decide ((Query.tupleFact b).project U =
           (Query.tupleFact a).project S)).filterMap
       fun b => (Query.tupleFact b j).intervalI64)
   | none => true)

/-- The coverage checker: every selected source row passes its
per-row sweep. -/
def coverageB (LA : List Row) (φ : Selection) (S : List FieldId)
    (i : FieldId) (LB : List Row) (ψ : Selection) (U : List FieldId)
    (j : FieldId) : Bool :=
  LA.all fun a => !satisfiesB φ a || coverRowB LB ψ U j S i a

/-- `coverageB` decides `Coverage` on the listed denotations. -/
theorem coverageB_iff {LA LB : List Row} {A B : Set Fact}
    (hA : Denotes LA A) (hB : Denotes LB B) (φ : Selection)
    (S : List FieldId) (i : FieldId) (ψ : Selection)
    (U : List FieldId) (j : FieldId) :
    coverageB LA φ S i LB ψ U j = true ↔ Coverage A φ S i B ψ U j := by
  unfold coverageB coverRowB Coverage
  rw [List.all_eq_true]
  constructor
  · intro h f hfA hfφ x hx
    obtain ⟨a, ha, rfl⟩ := (hA f).mp hfA
    have h1 := h a ha
    rw [impB_iff] at h1
    obtain ⟨hu, hi⟩ := andB_iff.mp
      (h1 (satisfiesB_iff.mpr hfφ))
    cases x with
    | u64 y =>
      obtain ⟨iv, hiv, hy⟩ := (mem_points_u64 _ y).mp hx
      rw [hiv] at hu
      obtain ⟨jv, hjv, hyj⟩ := (coveredB_iff iv _).mp hu y hy
      obtain ⟨b, hbg, hbs⟩ := List.mem_filterMap.mp hjv
      obtain ⟨hbL, hbc⟩ := List.mem_filter.mp hbg
      obtain ⟨hbψ, hbU⟩ := andB_iff.mp hbc
      exact ⟨Query.tupleFact b, (hB _).mpr ⟨b, hbL, rfl⟩,
        satisfiesB_iff.mp hbψ, of_decide_eq_true hbU,
        (mem_points_u64 _ y).mpr ⟨jv, hbs, hyj⟩⟩
    | i64 y =>
      obtain ⟨iv, hiv, hy⟩ := (mem_points_i64 _ y).mp hx
      rw [hiv] at hi
      obtain ⟨jv, hjv, hyj⟩ := (coveredB_iff iv _).mp hi y hy
      obtain ⟨b, hbg, hbs⟩ := List.mem_filterMap.mp hjv
      obtain ⟨hbL, hbc⟩ := List.mem_filter.mp hbg
      obtain ⟨hbψ, hbU⟩ := andB_iff.mp hbc
      exact ⟨Query.tupleFact b, (hB _).mpr ⟨b, hbL, rfl⟩,
        satisfiesB_iff.mp hbψ, of_decide_eq_true hbU,
        (mem_points_i64 _ y).mpr ⟨jv, hbs, hyj⟩⟩
  · intro h a ha
    rw [impB_iff]
    intro hφ
    refine andB_iff.mpr ⟨?_, ?_⟩
    · cases hiv : (Query.tupleFact a i).intervalU64 with
      | none => rfl
      | some iv =>
        refine (coveredB_iff iv _).mpr fun y hy => ?_
        obtain ⟨g, hgB, hgψ, hgU, hyg⟩ := h (Query.tupleFact a)
          ((hA _).mpr ⟨a, ha, rfl⟩) (satisfiesB_iff.mp hφ)
          (.u64 y) ((mem_points_u64 _ y).mpr ⟨iv, hiv, hy⟩)
        obtain ⟨b, hb, rfl⟩ := (hB g).mp hgB
        obtain ⟨jv, hjv, hyj⟩ := (mem_points_u64 _ y).mp hyg
        exact ⟨jv, List.mem_filterMap.mpr ⟨b, List.mem_filter.mpr
          ⟨hb, andB_iff.mpr
            ⟨satisfiesB_iff.mpr hgψ, decide_eq_true hgU⟩⟩, hjv⟩, hyj⟩
    · cases hiv : (Query.tupleFact a i).intervalI64 with
      | none => rfl
      | some iv =>
        refine (coveredB_iff iv _).mpr fun y hy => ?_
        obtain ⟨g, hgB, hgψ, hgU, hyg⟩ := h (Query.tupleFact a)
          ((hA _).mpr ⟨a, ha, rfl⟩) (satisfiesB_iff.mp hφ)
          (.i64 y) ((mem_points_i64 _ y).mpr ⟨iv, hiv, hy⟩)
        obtain ⟨b, hb, rfl⟩ := (hB g).mp hgB
        obtain ⟨jv, hjv, hyj⟩ := (mem_points_i64 _ y).mp hyg
        exact ⟨jv, List.mem_filterMap.mpr ⟨b, List.mem_filter.mpr
          ⟨hb, andB_iff.mpr
            ⟨satisfiesB_iff.mpr hgψ, decide_eq_true hgU⟩⟩, hjv⟩, hyj⟩

/-! ## Cardinality windows -/

/-- The distinct child count of one parent tuple: qualifying rows,
one representative per denoted fact. -/
def childCountB (LA : List Row) (φ : Selection) (X : List FieldId)
    (t : List Value) : Nat :=
  (dedupFacts (LA.filter fun a => satisfiesB φ a &&
    decide ((Query.tupleFact a).project X = t))).length

/-- The window verdict at one count. -/
def windowB (w : Window) (n : Nat) : Bool :=
  decide (w.lo ≤ n) &&
    match w.hi with
    | none => true
    | some m => decide (n ≤ m)

/-- The cardinality-window checker: every selected parent row's child
count sits in the window. -/
def cardinalityB (LA : List Row) (φ : Selection) (X : List FieldId)
    (w : Window) (LB : List Row) (ψ : Selection) (Y : List FieldId) :
    Bool :=
  LB.all fun b => !satisfiesB ψ b ||
    windowB w (childCountB LA φ X ((Query.tupleFact b).project Y))

/-- The child group of a parent tuple is enumerated by the
deduplicated qualifying rows. -/
theorem childGroup_enum {L : List Row} {A : Set Fact} (hA : Denotes L A)
    (φ : Selection) (X : List FieldId) (t : List Value) :
    ∀ f, f ∈ ChildGroup A φ X t ↔
      f ∈ (dedupFacts (L.filter fun a => satisfiesB φ a &&
        decide ((Query.tupleFact a).project X = t))).map
          Query.tupleFact := by
  intro f
  rw [mem_map_dedupFacts]
  constructor
  · rintro ⟨hfA, hfφ, hft⟩
    obtain ⟨r, hr, rfl⟩ := (hA f).mp hfA
    refine List.mem_map.mpr ⟨r, List.mem_filter.mpr ⟨hr, ?_⟩, rfl⟩
    exact andB_iff.mpr
      ⟨satisfiesB_iff.mpr hfφ, decide_eq_true hft⟩
  · intro h
    obtain ⟨r, hr, rfl⟩ := List.mem_map.mp h
    obtain ⟨hrL, hcond⟩ := List.mem_filter.mp hr
    obtain ⟨h1, h2⟩ := andB_iff.mp hcond
    exact ⟨(hA _).mpr ⟨r, hrL, rfl⟩, satisfiesB_iff.mp h1,
      of_decide_eq_true h2⟩

/-- The window verdict at an enumerated group's length is the
window's judgment — the two count bounds collapsed to compares. -/
theorem windowB_iff {s : Set Fact} {enum : List Fact}
    (hnd : enum.Nodup) (hmem : ∀ f, f ∈ s ↔ f ∈ enum) (w : Window) :
    windowB w enum.length = true ↔ w.admits s := by
  unfold windowB Window.admits
  rw [andB_iff, decide_eq_true_iff]
  constructor
  · rintro ⟨h1, h2⟩
    refine ⟨(atLeast_iff_enum_length hnd hmem w.lo).mpr h1,
      fun m hm => ?_⟩
    rw [hm] at h2
    exact (atMost_iff_enum_length hnd hmem m).mpr
      (of_decide_eq_true h2)
  · rintro ⟨h1, h2⟩
    refine ⟨(atLeast_iff_enum_length hnd hmem w.lo).mp h1, ?_⟩
    cases hhi : w.hi with
    | none => rfl
    | some m =>
      exact decide_eq_true
        ((atMost_iff_enum_length hnd hmem m).mp (h2 m hhi))

/-- `cardinalityB` decides `CardinalityWindow` on the listed
denotations. -/
theorem cardinalityB_iff {LA LB : List Row} {A B : Set Fact}
    (hA : Denotes LA A) (hB : Denotes LB B) (φ : Selection)
    (X : List FieldId) (w : Window) (ψ : Selection)
    (Y : List FieldId) :
    cardinalityB LA φ X w LB ψ Y = true ↔
      CardinalityWindow A φ X w B ψ Y := by
  unfold cardinalityB CardinalityWindow
  rw [List.all_eq_true]
  constructor
  · intro h g hg hψ
    obtain ⟨b, hb, rfl⟩ := (hB g).mp hg
    have h1 := h b hb
    rw [impB_iff] at h1
    have h2 := h1 (satisfiesB_iff.mpr hψ)
    refine (windowB_iff (nodup_map_dedupFacts _)
      (childGroup_enum hA φ X ((Query.tupleFact b).project Y)) w).mp ?_
    rw [List.length_map]
    exact h2
  · intro h b hb
    rw [impB_iff]
    intro hψ
    have h1 := (windowB_iff (nodup_map_dedupFacts _)
      (childGroup_enum hA φ X ((Query.tupleFact b).project Y)) w).mpr
      (h (Query.tupleFact b) ((hB _).mpr ⟨b, hb, rfl⟩)
        (satisfiesB_iff.mp hψ))
    rw [List.length_map] at h1
    exact h1

/-! ## Order marks -/

/-- The plain order-mark checker: per listed row's group — 1-based,
per-group ordinal uniqueness, and downward closure (`1..k` enumerated
below each attained `k`). -/
def orderMarkB (L : List Row) (pos : FieldId) (G : List FieldId) : Bool :=
  L.all fun a =>
    decide (1 ≤ (Query.tupleFact a pos).ordinal) &&
    (L.all fun b =>
      !decide ((Query.tupleFact a).project G =
        (Query.tupleFact b).project G) ||
      (!decide ((Query.tupleFact a pos).ordinal =
          (Query.tupleFact b pos).ordinal) ||
        rowEqB a b)) &&
    ((List.range' 1 (Query.tupleFact a pos).ordinal).all fun n =>
      L.any fun b =>
        decide ((Query.tupleFact b).project G =
          (Query.tupleFact a).project G) &&
        decide ((Query.tupleFact b pos).ordinal = n))

/-- `orderMarkB` decides `OrderMark` on the listed denotation: a
group not realized by a listed row is empty, and an empty group is
ordinally disciplined vacuously. -/
theorem orderMarkB_iff {L : List Row} {A : Set Fact} (hA : Denotes L A)
    (pos : FieldId) (G : List FieldId) :
    orderMarkB L pos G = true ↔ OrderMark A pos G := by
  unfold orderMarkB OrderMark
  rw [List.all_eq_true]
  constructor
  · intro h t
    refine ⟨?_, ?_, ?_⟩
    · intro f g hf hg hord
      obtain ⟨hfA, hft⟩ := hf
      obtain ⟨hgA, hgt⟩ := hg
      obtain ⟨a, ha, rfl⟩ := (hA f).mp hfA
      obtain ⟨b, hb, rfl⟩ := (hA g).mp hgA
      obtain ⟨hxy, -⟩ := andB_iff.mp (h a ha)
      obtain ⟨-, h2⟩ := andB_iff.mp hxy
      have h2' := List.all_eq_true.mp h2 b hb
      rw [impB_iff] at h2'
      have h3 := h2' (decide_eq_true (hft.trans hgt.symm))
      rw [impB_iff] at h3
      exact rowEqB_iff.mp (h3 (decide_eq_true hord))
    · intro f hf
      obtain ⟨hfA, hft⟩ := hf
      obtain ⟨a, ha, rfl⟩ := (hA f).mp hfA
      exact of_decide_eq_true
        ((andB_iff.mp
          ((andB_iff.mp (h a ha)).1)).1)
    · intro f hf n h1 hn
      obtain ⟨hfA, hft⟩ := hf
      obtain ⟨a, ha, rfl⟩ := (hA f).mp hfA
      have hcl := (andB_iff.mp (h a ha)).2
      have hmem : n ∈ List.range' 1 (Query.tupleFact a pos).ordinal :=
        List.mem_range'_1.mpr ⟨h1, by omega⟩
      obtain ⟨b, hb, hcond⟩ := List.any_eq_true.mp
        (List.all_eq_true.mp hcl n hmem)
      obtain ⟨hbG, hbn⟩ := andB_iff.mp hcond
      exact ⟨Query.tupleFact b,
        ⟨(hA _).mpr ⟨b, hb, rfl⟩, (of_decide_eq_true hbG).trans hft⟩,
        of_decide_eq_true hbn⟩
  · intro h a ha
    have hf : Query.tupleFact a ∈
        GroupOf A G ((Query.tupleFact a).project G) :=
      ⟨(hA _).mpr ⟨a, ha, rfl⟩, rfl⟩
    refine andB_iff.mpr ⟨andB_iff.mpr ⟨?_, ?_⟩, ?_⟩
    · exact decide_eq_true
        ((h ((Query.tupleFact a).project G)).based _ hf)
    · refine List.all_eq_true.mpr fun b hb => ?_
      rw [impB_iff]
      intro hproj
      rw [impB_iff]
      intro hord
      have hg : Query.tupleFact b ∈
          GroupOf A G ((Query.tupleFact a).project G) :=
        ⟨(hA _).mpr ⟨b, hb, rfl⟩, (of_decide_eq_true hproj).symm⟩
      exact rowEqB_iff.mpr
        ((h ((Query.tupleFact a).project G)).unique _ _ hf hg
          (of_decide_eq_true hord))
    · refine List.all_eq_true.mpr fun n hn => ?_
      obtain ⟨h1, h2⟩ := List.mem_range'_1.mp hn
      obtain ⟨g, hg, hgo⟩ :=
        (h ((Query.tupleFact a).project G)).closed _ hf n h1
          (by omega)
      obtain ⟨hgA, hgt⟩ := hg
      obtain ⟨b, hb, rfl⟩ := (hA g).mp hgA
      exact List.any_eq_true.mpr ⟨b, hb, andB_iff.mpr
        ⟨decide_eq_true hgt, decide_eq_true hgo⟩⟩

/-! ## Ranked order marks — the chain probe, key-justified -/

/-- The executable chain evaluation: probe each hop's ROW LIST for
the running value at the key field, read the payload, continue.
`chainEval` (`Order.lean`) is deliberately relational; this function
reading is licensed by the hop key premises through
`chain_eval_deterministic` — `chainEvalL_complete` spends it. -/
def chainEvalL (W : RowInstance) : List RankHop → Value → Option Value
  | [], v => some v
  | hop :: rest, v =>
    match (W.rows hop.relation).find? fun b =>
        decide (Query.tupleFact b hop.key = v) with
    | some b => chainEvalL W rest (Query.tupleFact b hop.read)
    | none => none

/-- The probe is sound with NO key premise: a found row is a
relational witness. -/
theorem chainEvalL_sound {T : Theory} {W : RowInstance}
    (hden : ∀ R, Denotes (W.rows R) (T.den W.den R)) :
    ∀ (hops : List RankHop) (v w : Value),
      chainEvalL W hops v = some w → chainEval T W.den hops v w
  | [], v, w, h => Option.some.inj h
  | hop :: rest, v, w, h => by
    unfold chainEvalL at h
    cases hf : (W.rows hop.relation).find? fun b =>
        decide (Query.tupleFact b hop.key = v) with
    | none =>
      rw [hf] at h
      exact nomatch h
    | some b =>
      rw [hf] at h
      have hb' := List.find?_some hf
      exact ⟨Query.tupleFact b hop.read,
        ⟨Query.tupleFact b,
          (hden hop.relation _).mpr ⟨b, List.mem_of_find?_eq_some hf, rfl⟩,
          of_decide_eq_true hb', rfl⟩,
        chainEvalL_sound hden rest _ w h⟩

/-- The probe is complete under the hop key premises: the relational
chain forces the probe to succeed, and `chain_eval_deterministic`
(`Subsumption.lean`), spent one hop at a time, makes the found row's
payload THE chain value — the function reading, justified. -/
theorem chainEvalL_complete {T : Theory} {W : RowInstance}
    (hden : ∀ R, Denotes (W.rows R) (T.den W.den R)) :
    ∀ (hops : List RankHop),
      (∀ hop, hop ∈ hops →
        Functionality (T.den W.den hop.relation) [hop.key]) →
      ∀ (v w : Value), chainEval T W.den hops v w →
        chainEvalL W hops v = some w
  | [], _, v, w, h => congrArg some h
  | hop :: rest, hkeys, v, w, h => by
    obtain ⟨u, huhop, hrest⟩ := h
    obtain ⟨g, hgden, hgkey, hgread⟩ := huhop
    obtain ⟨r0, hr0, hg0⟩ := (hden hop.relation g).mp hgden
    cases hf : (W.rows hop.relation).find? fun b =>
        decide (Query.tupleFact b hop.key = v) with
    | none =>
      exact absurd (decide_eq_true (show Query.tupleFact r0 hop.key = v
          by rw [← hg0]; exact hgkey))
        (List.find?_eq_none.mp hf r0 hr0)
    | some b =>
      have hb' := List.find?_some hf
      have hbkey : Query.tupleFact b hop.key = v := of_decide_eq_true hb'
      have hbmem := List.mem_of_find?_eq_some hf
      have hkey1 : ∀ h', h' ∈ [hop] →
          Functionality (T.den W.den h'.relation) [h'.key] := by
        intro h' hh'
        rw [List.mem_singleton] at hh'
        rw [hh']
        exact hkeys hop (List.mem_cons_self ..)
      have hu : u = Query.tupleFact b hop.read :=
        chain_eval_deterministic [hop] hkey1 v u
          (Query.tupleFact b hop.read)
          ⟨u, ⟨g, hgden, hgkey, hgread⟩, rfl⟩
          ⟨Query.tupleFact b hop.read,
            ⟨Query.tupleFact b,
              (hden hop.relation _).mpr ⟨b, hbmem, rfl⟩, hbkey, rfl⟩,
            rfl⟩
      simp only [chainEvalL]
      rw [hf]
      show chainEvalL W rest (Query.tupleFact b hop.read) = some w
      rw [← hu]
      exact chainEvalL_complete hden rest
        (fun h' hh' => hkeys h' (List.mem_cons_of_mem hop hh')) u w hrest

/-- The rank-monotonicity checker: within a group, a strictly smaller
probed rank sits strictly earlier. -/
def rankMonoB (W : RowInstance) (L : List Row) (pos : FieldId)
    (G : List FieldId) (c : RankChain) : Bool :=
  L.all fun a => L.all fun b =>
    !decide ((Query.tupleFact a).project G =
      (Query.tupleFact b).project G) ||
    (match chainEvalL W c.hops (Query.tupleFact a c.link),
           chainEvalL W c.hops (Query.tupleFact b c.link) with
     | some wa, some wb =>
       !decide (wa.ordinal < wb.ordinal) ||
       decide ((Query.tupleFact a pos).ordinal <
         (Query.tupleFact b pos).ordinal)
     | _, _ => true)

/-- The ranked checker decides `RankedOrderMark` under the hop key
premises (the form's acceptance rule, spent semantically — the
hypothesis, never a denotation conjunct). -/
theorem rankedB_iff {T : Theory} {W : RowInstance}
    (hden : ∀ R, Denotes (W.rows R) (T.den W.den R)) {R : RelId}
    {pos : FieldId} {G : List FieldId} {c : RankChain}
    (hkeys : ∀ hop, hop ∈ c.hops →
      Functionality (T.den W.den hop.relation) [hop.key]) :
    (orderMarkB (W.rows R) pos G && rankMonoB W (W.rows R) pos G c) =
        true ↔
      RankedOrderMark T W.den (T.den W.den R) pos G c := by
  rw [andB_iff]
  constructor
  · rintro ⟨h1, h2⟩
    refine ⟨(orderMarkB_iff (hden R) pos G).mp h1, ?_⟩
    intro t f g hf hg rf rg hrf hrg hlt
    obtain ⟨hfA, hft⟩ := hf
    obtain ⟨hgA, hgt⟩ := hg
    obtain ⟨a, ha, rfl⟩ := (hden R f).mp hfA
    obtain ⟨b, hb, rfl⟩ := (hden R g).mp hgA
    obtain ⟨w1, hw1, hw1o⟩ := hrf
    obtain ⟨w2, hw2, hw2o⟩ := hrg
    have he1 := chainEvalL_complete hden c.hops hkeys _ _ hw1
    have he2 := chainEvalL_complete hden c.hops hkeys _ _ hw2
    have h3 := List.all_eq_true.mp
      (List.all_eq_true.mp (show (W.rows R).all _ = true from h2) a ha)
      b hb
    rw [impB_iff] at h3
    have h4 := h3 (decide_eq_true (hft.trans hgt.symm))
    rw [he1, he2] at h4
    have h5 : (!decide (w1.ordinal < w2.ordinal) ||
        decide ((Query.tupleFact a pos).ordinal <
          (Query.tupleFact b pos).ordinal)) = true := h4
    rw [impB_iff] at h5
    exact of_decide_eq_true
      (h5 (decide_eq_true (show w1.ordinal < w2.ordinal by
        rw [hw1o, hw2o]; exact hlt)))
  · intro h
    refine ⟨(orderMarkB_iff (hden R) pos G).mpr h.mark, ?_⟩
    refine List.all_eq_true.mpr fun a ha =>
      List.all_eq_true.mpr fun b hb => ?_
    rw [impB_iff]
    intro hproj
    cases h1 : chainEvalL W c.hops (Query.tupleFact a c.link) with
    | none => rfl
    | some wa =>
      cases h2 : chainEvalL W c.hops (Query.tupleFact b c.link) with
      | none => rfl
      | some wb =>
        show (!decide (wa.ordinal < wb.ordinal) ||
          decide ((Query.tupleFact a pos).ordinal <
            (Query.tupleFact b pos).ordinal)) = true
        rw [impB_iff]
        intro hlt
        refine decide_eq_true ?_
        exact h.mono ((Query.tupleFact a).project G) _ _
          ⟨(hden R _).mpr ⟨a, ha, rfl⟩, rfl⟩
          ⟨(hden R _).mpr ⟨b, hb, rfl⟩, (of_decide_eq_true hproj).symm⟩
          wa.ordinal wb.ordinal
          ⟨wa, chainEvalL_sound hden c.hops _ _ h1, rfl⟩
          ⟨wb, chainEvalL_sound hden c.hops _ _ h2, rfl⟩
          (of_decide_eq_true hlt)

/-! ## The per-statement dispatcher -/

/-- One statement's checker — `Statement.judgment`'s dispatch,
executable: the same `intervalSplit` reads select the same arms. -/
def Statement.checkB (T : Theory) (W : RowInstance) : Statement → Bool
  | .functionality R X =>
    match T.header.intervalSplit R X with
    | some (S, i) => pointwiseKeyB (W.rows R) S i
    | none => funcB (W.rows R) X
  | .containment src tgt =>
    match T.header.intervalSplit src.relation src.projection,
          T.header.intervalSplit tgt.relation tgt.projection with
    | some (S, i), some (U, j) =>
      coverageB (W.rows src.relation) src.selection S i
        (W.rows tgt.relation) tgt.selection U j
    | _, _ =>
      containB (W.rows src.relation) src.selection src.projection
        (W.rows tgt.relation) tgt.selection tgt.projection
  | .cardinality src w tgt =>
    cardinalityB (W.rows src.relation) src.selection src.projection w
      (W.rows tgt.relation) tgt.selection tgt.projection
  | .order R pos G none => orderMarkB (W.rows R) pos G
  | .order R pos G (some c) =>
    orderMarkB (W.rows R) pos G && rankMonoB W (W.rows R) pos G c

/-- `checkB` decides `Statement.judgment` on the row-denoted
instance, under the merge premise — plus, for a RANKED order
statement only, its hop key premises (the acceptance rule, spent). -/
theorem Statement.checkB_iff {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) {st : Statement}
    (hkeys : ∀ R pos G c, st = .order R pos G (some c) →
      ∀ hop, hop ∈ c.hops →
        Functionality (T.den W.den hop.relation) [hop.key]) :
    st.checkB T W = true ↔ st.judgment T W.den := by
  cases st with
  | functionality R X =>
    cases hsplit : T.header.intervalSplit R X with
    | some si =>
      obtain ⟨S, i⟩ := si
      simp only [Statement.checkB, Statement.judgment, hsplit]
      exact pointwiseKeyB_iff (theoryDen_denotes hclosed R) S i
    | none =>
      simp only [Statement.checkB, Statement.judgment, hsplit]
      exact funcB_iff (theoryDen_denotes hclosed R) X
  | containment src tgt =>
    cases hs : T.header.intervalSplit src.relation src.projection with
    | some si =>
      obtain ⟨S, i⟩ := si
      cases ht : T.header.intervalSplit tgt.relation tgt.projection with
      | some uj =>
        obtain ⟨U, j⟩ := uj
        simp only [Statement.checkB, Statement.judgment, hs, ht]
        exact coverageB_iff (theoryDen_denotes hclosed src.relation)
          (theoryDen_denotes hclosed tgt.relation) src.selection S i
          tgt.selection U j
      | none =>
        simp only [Statement.checkB, Statement.judgment, hs, ht]
        exact containB_iff (theoryDen_denotes hclosed src.relation)
          (theoryDen_denotes hclosed tgt.relation) src.selection
          src.projection tgt.selection tgt.projection
    | none =>
      cases ht : T.header.intervalSplit tgt.relation tgt.projection with
      | some uj =>
        simp only [Statement.checkB, Statement.judgment, hs, ht]
        exact containB_iff (theoryDen_denotes hclosed src.relation)
          (theoryDen_denotes hclosed tgt.relation) src.selection
          src.projection tgt.selection tgt.projection
      | none =>
        simp only [Statement.checkB, Statement.judgment, hs, ht]
        exact containB_iff (theoryDen_denotes hclosed src.relation)
          (theoryDen_denotes hclosed tgt.relation) src.selection
          src.projection tgt.selection tgt.projection
  | cardinality src w tgt =>
    simp only [Statement.checkB, Statement.judgment]
    exact cardinalityB_iff (theoryDen_denotes hclosed src.relation)
      (theoryDen_denotes hclosed tgt.relation) src.selection
      src.projection w tgt.selection tgt.projection
  | order R pos G ranking =>
    cases ranking with
    | none =>
      simp only [Statement.checkB, Statement.judgment]
      exact orderMarkB_iff (theoryDen_denotes hclosed R) pos G
    | some c =>
      simp only [Statement.checkB, Statement.judgment]
      exact rankedB_iff (theoryDen_denotes hclosed)
        (hkeys R pos G c rfl)

/-! ## The whole-theory executable judge -/

/-- The declared-chain key premise, theory-wide: every hop of every
DECLARED `by` chain is key-backed on the judged instance — the ranked
form's acceptance rule, spent semantically (`rank_of_deterministic`'s
premise, quantified over the theory's statements). -/
def RankKeysHold (T : Theory) (I : Instance) : Prop :=
  ∀ R pos G c, Statement.order R pos G (some c) ∈ T.statements →
    ∀ hop, hop ∈ c.hops → Functionality (T.den I hop.relation) [hop.key]

/-- The theory-wide premise, restricted to one declared statement —
what `Statement.checkB_iff` consumes. -/
theorem rankKeys_at {T : Theory} {I : Instance}
    (hkeys : RankKeysHold T I) {st : Statement}
    (hst : st ∈ T.statements) :
    ∀ R pos G c, st = .order R pos G (some c) →
      ∀ hop, hop ∈ c.hops →
        Functionality (T.den I hop.relation) [hop.key] :=
  fun R pos G c heq hop hh => hkeys R pos G c (heq ▸ hst) hop hh

/-- The declared-chain key premise in its ACCEPTANCE form — a fact of
the THEORY, no instance mentioned: every declared `by` chain's hop
key is itself a DECLARED scalar functionality statement. This is the
ranked form's gate rule as the schema states it; the semantic
`RankKeysHold` is derived from it exactly when the key phase is
clean (`rankKeysHold_of_clean_keys`), which is what frees the
two-phase agreement of any instance-side premise
(`Txn.judgeB_agrees_of_declared`). -/
def RankKeysDeclared (T : Theory) : Prop :=
  ∀ R pos G c, Statement.order R pos G (some c) ∈ T.statements →
    ∀ hop, hop ∈ c.hops →
      Statement.functionality hop.relation [hop.key] ∈ T.statements ∧
      T.header.intervalSplit hop.relation [hop.key] = none

/-- A clean key phase spends the declared hop keys into the semantic
premise: with no key violation, every declared scalar functionality
statement's judgment holds of the judged instance — so every
declared hop key IS a `Functionality` there. -/
theorem rankKeysHold_of_clean_keys {T : Theory} {I : Instance}
    (hdecl : RankKeysDeclared T)
    (hk : ¬ (Txn.keyViolationSet T I).Nonempty) :
    RankKeysHold T I := by
  intro R pos G c hst hop hh
  obtain ⟨hfmem, hscalar⟩ := hdecl R pos G c hst hop hh
  have hjudg : (Statement.functionality hop.relation
      [hop.key]).judgment T I :=
    Classical.byContradiction fun hj => hk ⟨_, ⟨hfmem, hj⟩, rfl⟩
  simp only [Statement.judgment, hscalar] at hjudg
  exact hjudg

/-- **`holdsB` — the whole-theory executable judge**: every declared
statement's checker accepts. -/
def holdsB (T : Theory) (W : RowInstance) : Bool :=
  T.statements.all fun st => st.checkB T W

/-- **`holdsB` decides `holds`** on the row-denoted instance, under
the merge premise and the declared-chain key premise. -/
theorem holdsB_iff_holds {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W)
    (hkeys : RankKeysHold T W.den) :
    holdsB T W = true ↔ holds T W.den := by
  unfold holdsB holds
  rw [List.all_eq_true]
  exact forall_congr' fun st => forall_congr' fun hst =>
    Statement.checkB_iff hclosed (rankKeys_at hkeys hst)

/-- One statement's judgment, decided — `Decidable` by the checker
(premise-carrying named def; recorded narrowing in the module doc). -/
def decideJudgment {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) (st : Statement)
    (hkeys : ∀ R pos G c, st = .order R pos G (some c) →
      ∀ hop, hop ∈ c.hops →
        Functionality (T.den W.den hop.relation) [hop.key]) :
    Decidable (st.judgment T W.den) :=
  decidable_of_iff (st.checkB T W = true)
    (Statement.checkB_iff hclosed hkeys)

/-- **`Decidable (holds T I)` on finite instances** — the module's
headline, as a term: the whole-theory judgment is decided by
`holdsB`. -/
def decideHolds {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W)
    (hkeys : RankKeysHold T W.den) :
    Decidable (holds T W.den) :=
  decidable_of_iff (holdsB T W = true) (holdsB_iff_holds hclosed hkeys)

/-! ## The two-phase executable judge -/

namespace Txn

/-- The key phase's citations, executable: the declared functionality
statements the checker refutes. -/
def keyViolationsB (T : Theory) (W : RowInstance) : List Statement :=
  T.statements.filter fun st => st.isKey && !st.checkB T W

/-- The statement phase's citations, executable: the declared non-key
statements the checker refutes. -/
def statementViolationsB (T : Theory) (W : RowInstance) : List Statement :=
  T.statements.filter fun st => !st.isKey && !st.checkB T W

/-- **`judgeB` — the executable two-phase judge**, mirroring
`Txn.judge`: any key violation rejects with the complete violated-key
list and the statement phase never runs; else any statement violation
rejects with the complete non-key list; else accept (`none`). -/
def judgeB (T : Theory) (W : RowInstance) : Option (List Statement) :=
  match keyViolationsB T W with
  | [] =>
    match statementViolationsB T W with
    | [] => none
    | v => some v
  | v => some v

/-- The hop-key argument `Statement.checkB_iff` takes, vacuous for a
KEY statement: `isKey` selects the functionality constructor, never
an order statement, so the ranked premise is unreachable — which is
why the key phase consumes no chain premise. -/
theorem keyStatement_vacuous_hkeys {T : Theory} {I : Instance}
    {st : Statement} (hk : st.isKey = true) :
    ∀ R pos G c, st = .order R pos G (some c) →
      ∀ hop, hop ∈ c.hops →
        Functionality (T.den I hop.relation) [hop.key] := by
  intro R pos G c heq
  rw [heq] at hk
  exact nomatch hk

/-- The executable key citations are exactly `Txn.keyViolationSet`,
membership for membership — NO chain premise: a key statement is a
functionality statement, whose checker consumes the merge premise
only. The phase that convicts a hop-key-violating instance must not
itself assume hop keys, and does not. -/
theorem mem_keyViolationsB {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) {st : Statement} :
    st ∈ keyViolationsB T W ↔ st ∈ Txn.keyViolationSet T W.den := by
  unfold keyViolationsB
  constructor
  · intro h
    obtain ⟨hmem, hcond⟩ := List.mem_filter.mp h
    obtain ⟨h1, h2⟩ := andB_iff.mp hcond
    refine ⟨⟨hmem, fun hj => ?_⟩, h1⟩
    rw [(Statement.checkB_iff hclosed
      (keyStatement_vacuous_hkeys h1)).mpr hj] at h2
    exact nomatch h2
  · rintro ⟨⟨hmem, hj⟩, hk⟩
    refine List.mem_filter.mpr ⟨hmem, andB_iff.mpr ⟨hk, ?_⟩⟩
    cases hc : st.checkB T W with
    | false => rfl
    | true =>
      exact absurd
        ((Statement.checkB_iff hclosed
          (keyStatement_vacuous_hkeys hk)).mp hc) hj

/-- The executable statement citations are exactly
`Txn.statementViolationSet`, membership for membership. -/
theorem mem_statementViolationsB {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W)
    (hkeys : RankKeysHold T W.den) {st : Statement} :
    st ∈ statementViolationsB T W ↔
      st ∈ Txn.statementViolationSet T W.den := by
  unfold statementViolationsB
  constructor
  · intro h
    obtain ⟨hmem, hcond⟩ := List.mem_filter.mp h
    obtain ⟨h1, h2⟩ := andB_iff.mp hcond
    refine ⟨⟨hmem, fun hj => ?_⟩, ?_⟩
    · rw [(Statement.checkB_iff hclosed
        (rankKeys_at hkeys hmem)).mpr hj] at h2
      exact nomatch h2
    · cases hk : st.isKey with
      | false => rfl
      | true =>
        rw [hk] at h1
        exact nomatch h1
  · rintro ⟨⟨hmem, hj⟩, hk⟩
    refine List.mem_filter.mpr ⟨hmem, andB_iff.mpr ⟨?_, ?_⟩⟩
    · rw [hk]
      rfl
    · cases hc : st.checkB T W with
      | false => rfl
      | true =>
        exact absurd
          ((Statement.checkB_iff hclosed
            (rankKeys_at hkeys hmem)).mp hc) hj

/-- **The two-phase agreement**: `judgeB` and `Txn.judge` render one
verdict — accept together (and the accepted state is the judged
instance), or reject in the SAME phase, the executable citation list
and the model's violation set agreeing member for member
(`mem_keyViolationsB` / `mem_statementViolationsB`). -/
theorem judgeB_agrees {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W)
    (hkeys : RankKeysHold T W.den) :
    (judgeB T W = none ∧
      ∃ h, Txn.judge T W.den = .ok ⟨W.den, h⟩) ∨
    (judgeB T W = some (keyViolationsB T W) ∧
      Txn.judge T W.den = .reject (Txn.keyViolationSet T W.den)) ∨
    (judgeB T W = some (statementViolationsB T W) ∧
      Txn.judge T W.den =
        .reject (Txn.statementViolationSet T W.den)) := by
  by_cases hh : holds T W.den
  · refine Or.inl ⟨?_, hh, Txn.judge_holds hh⟩
    have hkey : keyViolationsB T W = [] := by
      refine List.filter_eq_nil_iff.mpr fun st hst => ?_
      intro hcond
      obtain ⟨-, h2⟩ := andB_iff.mp hcond
      rw [(Statement.checkB_iff hclosed
        (rankKeys_at hkeys hst)).mpr (hh st hst)] at h2
      exact nomatch h2
    have hstmt : statementViolationsB T W = [] := by
      refine List.filter_eq_nil_iff.mpr fun st hst => ?_
      intro hcond
      obtain ⟨-, h2⟩ := andB_iff.mp hcond
      rw [(Statement.checkB_iff hclosed
        (rankKeys_at hkeys hst)).mpr (hh st hst)] at h2
      exact nomatch h2
    unfold judgeB
    rw [hkey, hstmt]
  · by_cases hk : (Txn.keyViolationSet T W.den).Nonempty
    · refine Or.inr (Or.inl ⟨?_, Txn.judge_key_preempts hh hk⟩)
      obtain ⟨st, hstv⟩ := hk
      have hne : keyViolationsB T W ≠ [] :=
        List.ne_nil_of_mem ((mem_keyViolationsB hclosed).mpr hstv)
      unfold judgeB
      cases hkv : keyViolationsB T W with
      | nil => exact absurd hkv hne
      | cons a l => rfl
    · refine Or.inr (Or.inr ⟨?_, Txn.judge_statement_phase hh hk⟩)
      have hkey : keyViolationsB T W = [] := by
        rcases hkv : keyViolationsB T W with _ | ⟨a, l⟩
        · rfl
        · exact absurd ⟨a, (mem_keyViolationsB hclosed).mp
            (hkv ▸ List.mem_cons_self ..)⟩ hk
      have hex : ∃ st, st ∈ Txn.violationSet T W.den :=
        Classical.byContradiction fun hne =>
          hh fun st hst => Classical.byContradiction fun hj =>
            hne ⟨st, hst, hj⟩
      obtain ⟨st, hv⟩ := hex
      have hne : statementViolationsB T W ≠ [] :=
        List.ne_nil_of_mem
          ((mem_statementViolationsB hclosed hkeys).mpr
            (Txn.statement_phase_all hk hv))
      unfold judgeB
      rw [hkey]
      cases hsv : statementViolationsB T W with
      | nil => exact absurd hsv hne
      | cons a l => rfl

/-- **The two-phase agreement, instance-premise-free.** Under the
merge premise and the ACCEPTANCE form of the hop-key rule
(`RankKeysDeclared` — a fact of the theory, not of the judged
instance), `judgeB` and `Txn.judge` render one verdict on EVERY row
instance, hop-key-violating instances included: an instance breaking
a declared rank-hop key convicts in the key phase on both sides (the
hop key is a declared functionality statement and the key phase
consumes no chain premise — `mem_keyViolationsB`), and a clean key
phase yields the semantic premise (`rankKeysHold_of_clean_keys`)
that the conditioned agreement (`judgeB_agrees`) spends. -/
theorem judgeB_agrees_of_declared {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W)
    (hdecl : RankKeysDeclared T) :
    (judgeB T W = none ∧
      ∃ h, Txn.judge T W.den = .ok ⟨W.den, h⟩) ∨
    (judgeB T W = some (keyViolationsB T W) ∧
      Txn.judge T W.den = .reject (Txn.keyViolationSet T W.den)) ∨
    (judgeB T W = some (statementViolationsB T W) ∧
      Txn.judge T W.den =
        .reject (Txn.statementViolationSet T W.den)) := by
  by_cases hk : (Txn.keyViolationSet T W.den).Nonempty
  · have hh : ¬ holds T W.den := by
      obtain ⟨st, ⟨⟨hmem, hj⟩, -⟩⟩ := hk
      exact fun h => hj (h st hmem)
    refine Or.inr (Or.inl ⟨?_, Txn.judge_key_preempts hh hk⟩)
    obtain ⟨st, hstv⟩ := hk
    have hne : keyViolationsB T W ≠ [] :=
      List.ne_nil_of_mem ((mem_keyViolationsB hclosed).mpr hstv)
    unfold judgeB
    cases hkv : keyViolationsB T W with
    | nil => exact absurd hkv hne
    | cons a l => rfl
  · exact judgeB_agrees hclosed (rankKeysHold_of_clean_keys hdecl hk)

end Txn

end Bumbledb
