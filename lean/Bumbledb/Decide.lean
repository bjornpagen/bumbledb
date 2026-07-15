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
`containB_iff`, `coverageB_iff`, `cardinalityB_iff`), composed into
the per-statement dispatcher (`Statement.checkB`,
`Statement.checkB_iff`), the whole-theory executable judge
(`holdsB`, `holdsB_iff_holds`, the derived
`decideJudgment`/`decideHolds`), and the two-phase `Txn.judgeB` —
key phase then statement phase, mirroring `Txn.judge` and proved to
agree with its verdict and its violation sets, phase for phase, with
NO instance-side premise beyond the merge (`Txn.judgeB_agrees`,
`Txn.mem_keyViolationsB`, `Txn.mem_statementViolationsB`).

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
no decidable equality, and the key judgments CONCLUDE fact
equality — a row's finite support is what makes tuple-fact equality
decidable (`rowEqB_iff`: agreement on the shared index range, filler
beyond it). Recorded narrowing: the checkers judge instances whose
facts are row-denoted — exactly the conformance corpus's world shape.

## The premise — acceptance enters as a hypothesis (the tree's rule)

* **`WorldCarriesClosed`** — the world carries each sealed ground
  roster at its relation (the conformance lane's merge:
  `Conformance.decodeCase` appends the ground axioms into the world).
  Under it, `Theory.den` reads every relation through the row lists
  (`theoryDen_denotes`). A sealed `GroundExtension` is a `Fact` list,
  not a row list, so its members' equality is not decidable in place;
  the merge premise is the honest boundary, and it is what the lane
  already does.

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
(`holdsB_iff_holds`, `Txn.judgeB_agrees`); the corpus
format lives in `lean/conformance/README.md` § judgment cases.

## Narrowings recorded (law 5: narrow and record)

* The row carrier and the merge premise — above.
* `Txn.judgeB` returns `Option (List Statement)`: `none` accepts; a
  rejection's LIST may repeat a statement the theory declares twice.
  Agreement with `Txn.judge` is stated as membership equality with
  the violation SETS — `Txn.lean`'s own recorded narrowing (a set
  carries no duplicates or order by construction) applied to the
  executable face.
* Decidability lands as premise-carrying named `def`s
  (`decideJudgment`, `decideHolds`), never `instance`s: the premise
  is a per-theory semantic fact instance resolution cannot see.
* The interval checkers are stated per element domain (`U64`/`I64`
  concretely) — the tree's precedent (`encode_interval_order` and its
  U64 companion): an abstract order class would buy generality no
  third domain spends. The two-conjunct shape of `pointsDisjointB`
  and `coverRowB` (a u64 arm AND an i64 arm, each vacuously true on
  the other domain) keeps the checkers total with no typing premise —
  the same totalization move as `Value.points`.
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

/-- `checkB` decides `Statement.judgment` on the row-denoted
instance, under the merge premise. -/
theorem Statement.checkB_iff {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) {st : Statement} :
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

/-! ## The whole-theory executable judge -/

/-- **`holdsB` — the whole-theory executable judge**: every declared
statement's checker accepts. -/
def holdsB (T : Theory) (W : RowInstance) : Bool :=
  T.statements.all fun st => st.checkB T W

/-- **`holdsB` decides `holds`** on the row-denoted instance, under
the merge premise. -/
theorem holdsB_iff_holds {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) :
    holdsB T W = true ↔ holds T W.den := by
  unfold holdsB holds
  rw [List.all_eq_true]
  exact forall_congr' fun st => forall_congr' fun _ =>
    Statement.checkB_iff hclosed

/-- One statement's judgment, decided — `Decidable` by the checker
(premise-carrying named def; recorded narrowing in the module doc). -/
def decideJudgment {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) (st : Statement) :
    Decidable (st.judgment T W.den) :=
  decidable_of_iff (st.checkB T W = true)
    (Statement.checkB_iff hclosed)

/-- **`Decidable (holds T I)` on finite instances** — the module's
headline, as a term: the whole-theory judgment is decided by
`holdsB`. -/
def decideHolds {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) :
    Decidable (holds T W.den) :=
  decidable_of_iff (holdsB T W = true) (holdsB_iff_holds hclosed)

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

/-- The executable key citations are exactly `Txn.keyViolationSet`,
membership for membership: a key statement is a functionality
statement, whose checker consumes the merge premise only. -/
theorem mem_keyViolationsB {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) {st : Statement} :
    st ∈ keyViolationsB T W ↔ st ∈ Txn.keyViolationSet T W.den := by
  unfold keyViolationsB
  constructor
  · intro h
    obtain ⟨hmem, hcond⟩ := List.mem_filter.mp h
    obtain ⟨h1, h2⟩ := andB_iff.mp hcond
    refine ⟨⟨hmem, fun hj => ?_⟩, h1⟩
    rw [(Statement.checkB_iff hclosed).mpr hj] at h2
    exact nomatch h2
  · rintro ⟨⟨hmem, hj⟩, hk⟩
    refine List.mem_filter.mpr ⟨hmem, andB_iff.mpr ⟨hk, ?_⟩⟩
    cases hc : st.checkB T W with
    | false => rfl
    | true =>
      exact absurd ((Statement.checkB_iff hclosed).mp hc) hj

/-- The executable statement citations are exactly
`Txn.statementViolationSet`, membership for membership. -/
theorem mem_statementViolationsB {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) {st : Statement} :
    st ∈ statementViolationsB T W ↔
      st ∈ Txn.statementViolationSet T W.den := by
  unfold statementViolationsB
  constructor
  · intro h
    obtain ⟨hmem, hcond⟩ := List.mem_filter.mp h
    obtain ⟨h1, h2⟩ := andB_iff.mp hcond
    refine ⟨⟨hmem, fun hj => ?_⟩, ?_⟩
    · rw [(Statement.checkB_iff hclosed).mpr hj] at h2
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
        exact absurd ((Statement.checkB_iff hclosed).mp hc) hj

/-- **The two-phase agreement**: `judgeB` and `Txn.judge` render one
verdict on EVERY row instance — accept together (and the accepted
state is the judged instance), or reject in the SAME phase, the
executable citation list and the model's violation set agreeing
member for member (`mem_keyViolationsB` /
`mem_statementViolationsB`). No premise beyond the merge. -/
theorem judgeB_agrees {T : Theory} {W : RowInstance}
    (hclosed : WorldCarriesClosed T W) :
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
      rw [(Statement.checkB_iff hclosed).mpr (hh st hst)] at h2
      exact nomatch h2
    have hstmt : statementViolationsB T W = [] := by
      refine List.filter_eq_nil_iff.mpr fun st hst => ?_
      intro hcond
      obtain ⟨-, h2⟩ := andB_iff.mp hcond
      rw [(Statement.checkB_iff hclosed).mpr (hh st hst)] at h2
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
          ((mem_statementViolationsB hclosed).mpr
            (Txn.statement_phase_all hk hv))
      unfold judgeB
      rw [hkey]
      cases hsv : statementViolationsB T W with
      | nil => exact absurd hsv hne
      | cons a l => rfl

end Txn

end Bumbledb
