import LeanQuerySemantics

set_option autoImplicit false

universe u v w x

/-!
  GPT55DependencyTheory.lean

  Consolidated edge extension for Bumbledb's dependency and query semantics.
  This file intentionally imports the prior two artifacts and adds only
  definitions/theorems/countermodels, with every proof term completed.
-/

namespace GPT55DependencyTheory

open DependencyTheory
open LeanQuerySemantics

/- ===================================================================== -/
/- A. Dependency semantics: exact statement strength                     -/
/- ===================================================================== -/

/-- Ordinary selected containment is exactly subset inclusion of projected
    selected views.  This is often the cleanest public mathematical notation:
    `View R p keyR ⊆ View S q keyS`. -/
theorem contains_iff_view_subset {α : Type u} {β : Type v} {κ : Type w}
    (R : Rel α) (p : Sel α) (keyR : α → κ)
    (S : Rel β) (q : Sel β) (keyS : β → κ) :
    Contains R p keyR S q keyS ↔
      ∀ k : κ, View R p keyR k → View S q keyS k := by
  constructor
  · intro h k hv
    obtain ⟨a, hRa, hpa, hka⟩ := hv
    obtain ⟨b, hSb, hqb, hkb⟩ := h a hRa hpa
    exact ⟨b, hSb, hqb, hkb.trans hka⟩
  · intro h a hRa hpa
    have hv : View R p keyR (keyR a) := ⟨a, hRa, hpa, rfl⟩
    obtain ⟨b, hSb, hqb, hkb⟩ := h (keyR a) hv
    exact ⟨b, hSb, hqb, hkb⟩

/-- Selected equality is exactly equality of projected selected views. -/
theorem containsEq_iff_view_ext {α : Type u} {β : Type v} {κ : Type w}
    (R : Rel α) (p : Sel α) (keyR : α → κ)
    (S : Rel β) (q : Sel β) (keyS : β → κ) :
    ContainsEq R p keyR S q keyS ↔
      ∀ k : κ, View R p keyR k ↔ View S q keyS k := by
  constructor
  · intro h k
    constructor
    · exact (contains_iff_view_subset R p keyR S q keyS).mp h.forward k
    · exact (contains_iff_view_subset S q keyS R p keyR).mp h.backward k
  · intro h
    constructor
    · exact (contains_iff_view_subset R p keyR S q keyS).mpr (fun k hv => (h k).mp hv)
    · exact (contains_iff_view_subset S q keyS R p keyR).mpr (fun k hv => (h k).mpr hv)

/-- A public-facing name for Bumbledb's accepted `==` when it is meant to
    denote not just mutual projected inclusion but unique key-backed
    correspondence.  This is a conjunction, not a new primitive law. -/
structure KeyBackedEquality {α : Type u} {β : Type v} {κ : Type w}
    (R : Rel α) (p : Sel α) (keyR : α → κ)
    (S : Rel β) (q : Sel β) (keyS : β → κ) : Prop where
  eq : ContainsEq R p keyR S q keyS
  source_key : Key (Selected R p) keyR
  target_key : Key (Selected S q) keyS

/-- Key-backed equality gives a unique target witness for each selected
    source witness. -/
theorem KeyBackedEquality.unique_target {α : Type u} {β : Type v} {κ : Type w}
    {R : Rel α} {p : Sel α} {keyR : α → κ}
    {S : Rel β} {q : Sel β} {keyS : β → κ}
    (h : KeyBackedEquality R p keyR S q keyS)
    {a : α} (hRa : R a) (hpa : p a) :
    ∃ b : β, (S b ∧ q b ∧ keyS b = keyR a) ∧
      ∀ b' : β, (S b' ∧ q b' ∧ keyS b' = keyR a) → b' = b := by
  obtain ⟨b, hSb, hqb, hkb⟩ := h.eq.forward a hRa hpa
  refine ⟨b, ⟨hSb, hqb, hkb⟩, ?_⟩
  intro b' hb'
  exact h.target_key.uniqueness ⟨hb'.1, hb'.2.1⟩ ⟨hSb, hqb⟩ (hb'.2.2.trans hkb.symm)

/-- Key-backed equality gives a unique source witness for each selected
    target witness. -/
theorem KeyBackedEquality.unique_source {α : Type u} {β : Type v} {κ : Type w}
    {R : Rel α} {p : Sel α} {keyR : α → κ}
    {S : Rel β} {q : Sel β} {keyS : β → κ}
    (h : KeyBackedEquality R p keyR S q keyS)
    {b : β} (hSb : S b) (hqb : q b) :
    ∃ a : α, (R a ∧ p a ∧ keyR a = keyS b) ∧
      ∀ a' : α, (R a' ∧ p a' ∧ keyR a' = keyS b) → a' = a := by
  obtain ⟨a, hRa, hpa, hka⟩ := h.eq.backward b hSb hqb
  refine ⟨a, ⟨hRa, hpa, hka⟩, ?_⟩
  intro a' ha'
  exact h.source_key.uniqueness ⟨ha'.1, ha'.2.1⟩ ⟨hRa, hpa⟩ (ha'.2.2.trans hka.symm)

/-- Countermodel carrier for non-unique witnesses under bare `ContainsEq`. -/
inductive Two where | left | right deriving DecidableEq

open Two

/-- A singleton source. -/
def oneSource : Rel Unit := fun _ => True

/-- A two-row target with both rows present. -/
def twoTarget : Rel Two := fun _ => True

/-- Both target rows project to the same key. -/
def twoKey (_ : Two) : Nat := 0

/-- Bare projected equality holds. -/
theorem bare_containsEq_nonunique :
    ContainsEq oneSource (fun _ => True) (fun _ : Unit => 0)
      twoTarget (fun _ => True) twoKey := by
  constructor
  · intro a _ _
    exact ⟨left, trivial, trivial, rfl⟩
  · intro b _ _
    exact ⟨(), trivial, trivial, rfl⟩

/-- The same bare equality does not make the target key unique. -/
theorem bare_containsEq_target_not_key : ¬ Key (Selected twoTarget (fun _ => True)) twoKey := by
  intro h
  have hEq : (left : Two) = right := h.uniqueness ⟨trivial, trivial⟩ ⟨trivial, trivial⟩ rfl
  cases hEq

/-- Selection strengthening on the source preserves containment. -/
theorem contains_source_selection_strengthen {α : Type u} {β : Type v} {κ : Type w}
    {R : Rel α} {p p' : Sel α} {keyR : α → κ}
    {S : Rel β} {q : Sel β} {keyS : β → κ}
    (h : Contains R p keyR S q keyS) (hp : ∀ a, p' a → p a) :
    Contains R p' keyR S q keyS := by
  intro a hRa hp'a
  exact h a hRa (hp a hp'a)

/-- Selection weakening on the target preserves containment. -/
theorem contains_target_selection_weaken {α : Type u} {β : Type v} {κ : Type w}
    {R : Rel α} {p : Sel α} {keyR : α → κ}
    {S : Rel β} {q q' : Sel β} {keyS : β → κ}
    (h : Contains R p keyR S q keyS) (hq : ∀ b, q b → q' b) :
    Contains R p keyR S q' keyS := by
  intro a hRa hpa
  obtain ⟨b, hSb, hqb, hk⟩ := h a hRa hpa
  exact ⟨b, hSb, hq b hqb, hk⟩

/- ===================================================================== -/
/- B. Interval dependency edge: exact partition vs disjoint cover         -/
/- ===================================================================== -/

/-- Pointwise support of a selected interval fact family for scalar `s`. -/
def IntervalSupport {α : Type u} {κ : Type v} {β : Type w} [LinOrd β]
    (F : IntervalFacts α κ β) (p : Sel α) (s : κ) (x : β) : Prop :=
  ∃ a : α, F.R a ∧ p a ∧ F.scalar a = s ∧ x ∈ F.ival a

/-- Pointwise containment is subset inclusion of interval supports. -/
theorem intervalContains_iff_support_subset
    {α : Type u} {βt : Type v} {κ : Type w} {pt : Type x} [LinOrd pt]
    (F : IntervalFacts α κ pt) (p : Sel α)
    (G : IntervalFacts βt κ pt) (q : Sel βt) :
    IntervalContains F p G q ↔
      ∀ s x, IntervalSupport F p s x → IntervalSupport G q s x := by
  constructor
  · intro h s x hs
    obtain ⟨a, hRa, hpa, hscalar, hx⟩ := hs
    obtain ⟨b, hGb, hqb, hscalarg, hxg⟩ := h a hRa hpa x hx
    exact ⟨b, hGb, hqb, hscalar ▸ hscalarg, hxg⟩
  · intro h a hRa hpa x hx
    have hs : IntervalSupport F p (F.scalar a) x := ⟨a, hRa, hpa, rfl, hx⟩
    obtain ⟨b, hGb, hqb, hscalar, hxg⟩ := h (F.scalar a) x hs
    exact ⟨b, hGb, hqb, hscalar, hxg⟩

/-- Exact point partition: pointwise-keyed target plus equal source/target
    support.  This is the mathematically honest strengthening of a mere
    disjoint cover. -/
def ExactPointPartition {α : Type u} {βt : Type v} {κ : Type w} {pt : Type x} [LinOrd pt]
    (Domain : IntervalFacts α κ pt) (p : Sel α)
    (Tiles : IntervalFacts βt κ pt) (q : Sel βt) : Prop :=
  Tiles.PointwiseKey ∧
  ∀ s x, IntervalSupport Domain p s x ↔ IntervalSupport Tiles q s x

/-- `ExactTilingOf` from the previous artifact is equivalent to the support
    equality formulation plus target disjointness. -/
theorem exactTiling_iff_exactPointPartition
    {α : Type u} {βt : Type v} {κ : Type w} {pt : Type x} [LinOrd pt]
    (Domain : IntervalFacts α κ pt) (p : Sel α)
    (Tiles : IntervalFacts βt κ pt) (q : Sel βt) :
    LeanQuerySemantics.ExactTilingOf Domain p Tiles q ↔
      ExactPointPartition Domain p Tiles q := by
  constructor
  · intro h
    refine ⟨h.1.1, ?_⟩
    intro s x
    constructor
    · exact (intervalContains_iff_support_subset Domain p Tiles q).mp h.1.2 s x
    · exact (intervalContains_iff_support_subset Tiles q Domain p).mp h.2 s x
  · intro h
    refine ⟨⟨h.1, ?_⟩, ?_⟩
    · exact (intervalContains_iff_support_subset Domain p Tiles q).mpr
        (fun s x hs => (h.2 s x).mp hs)
    · exact (intervalContains_iff_support_subset Tiles q Domain p).mpr
        (fun s x hs => (h.2 s x).mpr hs)

structure NIntervalFact where
  id : Nat
  lo : Nat
  hi : Nat
  deriving DecidableEq

def nifInterval (f : NIntervalFact) : Interval Nat := ⟨f.lo, f.hi⟩

def domainOne : Rel NIntervalFact := fun f => f = ⟨0, 0, 10⟩
def tileOvershoot : Rel NIntervalFact := fun f => f = ⟨0, 0, 20⟩

def domainFacts : IntervalFacts NIntervalFact Nat Nat where
  R := domainOne
  scalar := NIntervalFact.id
  ival := nifInterval

def overshootFacts : IntervalFacts NIntervalFact Nat Nat where
  R := tileOvershoot
  scalar := NIntervalFact.id
  ival := nifInterval

/-- A single overshooting tile is pointwise-keyed vacuously: there are no
    two distinct rows in the selected target. -/
theorem overshoot_pointwiseKey : overshootFacts.PointwiseKey := by
  intro a b ha hb _ hneq x _ _
  unfold overshootFacts tileOvershoot at ha hb
  cases ha
  cases hb
  exact hneq rfl

/-- Concrete countermodel: disjoint cover holds while exact point partition
    fails, because the tile covers point 15 outside the declared domain. -/
theorem overshoot_isTiling_not_exact :
    IsTilingOf domainFacts (fun _ => True) overshootFacts (fun _ => True) ∧
    ¬ ExactPointPartition domainFacts (fun _ => True) overshootFacts (fun _ => True) := by
  constructor
  · constructor
    · exact overshoot_pointwiseKey
    · intro a ha _ x hx
      unfold domainFacts domainOne at ha
      cases ha
      refine ⟨⟨0, 0, 20⟩, rfl, trivial, rfl, ?_⟩
      exact ⟨hx.1, Nat.lt_trans hx.2 (by decide)⟩
  · intro hex
    have htile : IntervalSupport overshootFacts (fun _ => True) 0 15 := by
      refine ⟨⟨0, 0, 20⟩, rfl, trivial, rfl, ?_⟩
      exact ⟨by decide, by decide⟩
    have hdom : IntervalSupport domainFacts (fun _ => True) 0 15 := (hex.2 0 15).mpr htile
    obtain ⟨a, ha, _, hscalar, hx⟩ := hdom
    unfold domainFacts domainOne at ha
    cases ha
    exact (show ¬ 15 < 10 from by decide) hx.2

/-- The abstract Lean interval allows empty/reversed intervals; this tiny
    fact is what makes coverage obligations over such facts vacuous unless
    Rust/schema validation rejects them. -/
theorem empty_nat_interval_has_no_points : ¬ ∃ x : Nat, x ∈ (⟨10, 5⟩ : Interval Nat) := by
  intro hx
  obtain ⟨x, hlo, hhi⟩ := hx
  exact (show ¬ 10 < 5 from by decide) (Nat.lt_of_le_of_lt hlo hhi)

/- ===================================================================== -/
/- C. Raw query syntax, resolved binders, and set semantics              -/
/- ===================================================================== -/

abbrev Var := Nat
abbrev Param := Nat
abbrev RelName := Nat
abbrev Val := Nat
abbrev Row := List Val
abbrev Assignment := Var → Val
abbrev ParamEnv := Param → Val
abbrev VarSet := Var → Prop

inductive Term where
  | var : Var → Term
  | const : Val → Term
  | param : Param → Term
  deriving DecidableEq

namespace Term

def eval (ρ : ParamEnv) (σ : Assignment) : Term → Val
  | var v => σ v
  | const c => c
  | param p => ρ p

def Vars : Term → VarSet
  | var v => fun x => x = v
  | const _ => fun _ => False
  | param _ => fun _ => False

@[simp] theorem vars_var (v x : Var) : Vars (var v) x ↔ x = v := Iff.rfl
@[simp] theorem vars_const (c : Val) (x : Var) : Vars (const c) x ↔ False := Iff.rfl
@[simp] theorem vars_param (p : Param) (x : Var) : Vars (param p) x ↔ False := Iff.rfl

end Term

structure RawAtom where
  rel : RelName
  args : List Term
  deriving DecidableEq

inductive CmpOp where
  | eq | ne | lt | le
  deriving DecidableEq

inductive AllenOp where
  | before | meets | overlaps | during | starts | finishes | equals
  deriving DecidableEq

structure RawInterval where
  lo : Term
  hi : Term
  deriving DecidableEq

inductive RawCmp where
  | bin : CmpOp → Term → Term → RawCmp
  | member : Term → RawInterval → RawCmp
  | allen : AllenOp → RawInterval → RawInterval → RawCmp
  deriving DecidableEq

inductive Literal where
  | pos : RawAtom → Literal
  | neg : RawAtom → Literal
  | cmp : RawCmp → Literal
  deriving DecidableEq

structure RawClause where
  head : List Term
  body : List Literal
  deriving DecidableEq

structure Database where
  rel : RelName → List Row

/-- The finite active domain actually present in a database snapshot. -/
def flattenRows : List Row → List Val
  | [] => []
  | r :: rs => r ++ flattenRows rs

def Database.activeDomain (db : Database) : List RelName → List Val
  | [] => []
  | r :: rs => flattenRows (db.rel r) ++ Database.activeDomain db rs

def termListVars : List Term → VarSet
  | [] => fun _ => False
  | t :: ts => fun x => Term.Vars t x ∨ termListVars ts x

def atomVars (a : RawAtom) : VarSet := termListVars a.args

def intervalVars (i : RawInterval) : VarSet := fun x => Term.Vars i.lo x ∨ Term.Vars i.hi x

def cmpVars : RawCmp → VarSet
  | RawCmp.bin _ t u => fun x => Term.Vars t x ∨ Term.Vars u x
  | RawCmp.member t i => fun x => Term.Vars t x ∨ intervalVars i x
  | RawCmp.allen _ i j => fun x => intervalVars i x ∨ intervalVars j x

def litVars : Literal → VarSet
  | Literal.pos a => atomVars a
  | Literal.neg a => atomVars a
  | Literal.cmp c => cmpVars c

def positiveVars : List Literal → VarSet
  | [] => fun _ => False
  | Literal.pos a :: rest => fun x => atomVars a x ∨ positiveVars rest x
  | _ :: rest => positiveVars rest

def allLitVars : List Literal → VarSet
  | [] => fun _ => False
  | l :: rest => fun x => litVars l x ∨ allLitVars rest x

def WellScoped (bound : VarSet) (c : RawClause) : Prop :=
  ∀ v, (termListVars c.head v ∨ allLitVars c.body v) → bound v

/-- Positive range restriction: every variable used anywhere in the clause
    appears in at least one positive relational atom.  This is intentionally
    stronger than mere lexical well-scopedness. -/
def PositivelyRangeRestricted (c : RawClause) : Prop :=
  ∀ v, (termListVars c.head v ∨ allLitVars c.body v) → positiveVars c.body v

theorem positive_range_restriction_implies_wellscoped (c : RawClause) :
    PositivelyRangeRestricted c → WellScoped (positiveVars c.body) c := by
  intro h v hv
  exact h v hv

/-- Matching a term vector against a row.  Repeated variables are naturally
    handled by repeated calls to `Term.eval`. -/
def MatchTerms (ρ : ParamEnv) (σ : Assignment) : List Term → Row → Prop
  | [], [] => True
  | t :: ts, v :: vs => Term.eval ρ σ t = v ∧ MatchTerms ρ σ ts vs
  | _, _ => False

def AtomMatches (db : Database) (ρ : ParamEnv) (σ : Assignment) (a : RawAtom) : Prop :=
  ∃ row : Row, row ∈ db.rel a.rel ∧ MatchTerms ρ σ a.args row

def cmpOpEval : CmpOp → Val → Val → Prop
  | CmpOp.eq, a, b => a = b
  | CmpOp.ne, a, b => a ≠ b
  | CmpOp.lt, a, b => a < b
  | CmpOp.le, a, b => a ≤ b

def intervalEval (ρ : ParamEnv) (σ : Assignment) (i : RawInterval) : Interval Nat :=
  ⟨Term.eval ρ σ i.lo, Term.eval ρ σ i.hi⟩

def AllenEval (op : AllenOp) (i j : Interval Nat) : Prop :=
  match op with
  | AllenOp.before => i.hi ≤ j.lo
  | AllenOp.meets => i.hi = j.lo
  | AllenOp.overlaps => i.lo < j.lo ∧ j.lo < i.hi ∧ i.hi < j.hi
  | AllenOp.during => j.lo ≤ i.lo ∧ i.hi ≤ j.hi
  | AllenOp.starts => i.lo = j.lo ∧ i.hi ≤ j.hi
  | AllenOp.finishes => j.lo ≤ i.lo ∧ i.hi = j.hi
  | AllenOp.equals => i.lo = j.lo ∧ i.hi = j.hi

def CmpHolds (ρ : ParamEnv) (σ : Assignment) : RawCmp → Prop
  | RawCmp.bin op t u => cmpOpEval op (Term.eval ρ σ t) (Term.eval ρ σ u)
  | RawCmp.member t i => Term.eval ρ σ t ∈ intervalEval ρ σ i
  | RawCmp.allen op i j => AllenEval op (intervalEval ρ σ i) (intervalEval ρ σ j)

def LiteralHolds (db : Database) (ρ : ParamEnv) (σ : Assignment) : Literal → Prop
  | Literal.pos a => AtomMatches db ρ σ a
  | Literal.neg a => ¬ AtomMatches db ρ σ a
  | Literal.cmp c => CmpHolds ρ σ c

def BodyHolds (db : Database) (ρ : ParamEnv) (σ : Assignment) (body : List Literal) : Prop :=
  body.foldr (fun lit acc => LiteralHolds db ρ σ lit ∧ acc) True

def ClauseDenote (db : Database) (ρ : ParamEnv) (c : RawClause) : Rel Row :=
  fun out => ∃ σ : Assignment, BodyHolds db ρ σ c.body ∧ MatchTerms ρ σ c.head out

def RuleUnion (rels : List (Rel Row)) : Rel Row :=
  fun row => ∃ R : Rel Row, R ∈ rels ∧ R row

theorem ruleUnion_set_idempotent (R : Rel Row) (row : Row) :
    RuleUnion [R, R] row ↔ R row := by
  constructor
  · intro h
    obtain ⟨S, hmem, hs⟩ := h
    have hSR : S = R := by simpa using hmem
    rw [hSR] at hs
    exact hs
  · intro h
    exact ⟨R, List.mem_cons_self, h⟩

/-- Repeated variable punning law for raw atom matching. -/
theorem repeated_var_forces_equal (ρ : ParamEnv) (σ : Assignment) (x : Var) (a b : Val) :
    MatchTerms ρ σ [Term.var x, Term.var x] [a, b] → a = b := by
  intro h
  exact h.1.symm.trans h.2.1

/-- Constants in atom positions are matched literally. -/
theorem constant_match_forces_value (ρ : ParamEnv) (σ : Assignment) (c v : Val) :
    MatchTerms ρ σ [Term.const c] [v] → v = c := by
  intro h
  exact h.1.symm

/-- Parameters are not bound variables; they are read from the parameter
    environment. -/
theorem parameter_match_forces_value (ρ : ParamEnv) (σ : Assignment) (p : Param) (v : Val) :
    MatchTerms ρ σ [Term.param p] [v] → v = ρ p := by
  intro h
  exact h.1.symm

theorem point_membership_unfold (ρ : ParamEnv) (σ : Assignment) (t lo hi : Term) :
    CmpHolds ρ σ (RawCmp.member t ⟨lo, hi⟩) ↔
      Term.eval ρ σ lo ≤ Term.eval ρ σ t ∧ Term.eval ρ σ t < Term.eval ρ σ hi :=
  Iff.rfl

theorem allen_meets_unfold (ρ : ParamEnv) (σ : Assignment) (i j : RawInterval) :
    CmpHolds ρ σ (RawCmp.allen AllenOp.meets i j) ↔
      Term.eval ρ σ i.hi = Term.eval ρ σ j.lo :=
  Iff.rfl

/-- Positive atoms range only over rows present in the finite snapshot. -/
theorem atom_match_row_from_snapshot (db : Database) (ρ : ParamEnv) (σ : Assignment) (a : RawAtom) :
    AtomMatches db ρ σ a → ∃ row, row ∈ db.rel a.rel ∧ MatchTerms ρ σ a.args row :=
  fun h => h

/- ===================================================================== -/
/- D. Finite aggregation, overflow, Pack, snapshots, closed folding      -/
/- ===================================================================== -/

inductive AggOp where | count | sum | min | max deriving DecidableEq

/-- Aggregation is defined over an explicit finite list of input values.
    Empty global aggregate behavior is explicit: count = 0, sum = 0,
    min/max = none. -/
def aggEval : AggOp → List Nat → Option Nat
  | AggOp.count, xs => some xs.length
  | AggOp.sum, xs => some (xs.foldl (fun acc x => acc + x) 0)
  | AggOp.min, [] => none
  | AggOp.min, x :: xs => some (xs.foldl Nat.min x)
  | AggOp.max, [] => none
  | AggOp.max, x :: xs => some (xs.foldl Nat.max x)

@[simp] theorem count_empty : aggEval AggOp.count [] = some 0 := rfl
@[simp] theorem sum_empty : aggEval AggOp.sum [] = some 0 := rfl
@[simp] theorem min_empty : aggEval AggOp.min [] = none := rfl
@[simp] theorem max_empty : aggEval AggOp.max [] = none := rfl

/-- Checked addition for bounded integer implementations.  Lean's `Nat`
    has no overflow; this function models the Rust obligation explicitly. -/
def checkedAdd (limit a b : Nat) : Option Nat :=
  if a + b ≤ limit then some (a + b) else none

def checkedSum (limit : Nat) : List Nat → Option Nat
  | [] => some 0
  | x :: xs =>
      match checkedSum limit xs with
      | none => none
      | some s => checkedAdd limit x s

theorem checkedAdd_sound {limit a b s : Nat} :
    checkedAdd limit a b = some s → s = a + b ∧ s ≤ limit := by
  unfold checkedAdd
  split
  · rename_i hle
    intro hsome
    cases hsome
    exact ⟨rfl, hle⟩
  · intro hnone
    cases hnone

/-- A packed value is a finite set representation, not intrinsically a
    deterministic byte representation unless `canonical` is supplied. -/
structure Pack (α : Type u) where
  elems : List α
  nodup : elems.Nodup

/-- Extensional equality of packs: the mathematically relevant set equality. -/
def Pack.ExtEq {α : Type u} (p q : Pack α) : Prop := ∀ a, a ∈ p.elems ↔ a ∈ q.elems

/-- Deterministic/canonical representation is an extra obligation, not a
    consequence of set semantics. -/
structure CanonicalPack (α : Type u) extends Pack α where
  canonical : ∀ q : Pack α, Pack.ExtEq toPack q → toPack.elems = q.elems

/-- Snapshot facts carry a generation. -/
structure Versioned (α : Type u) where
  value : α
  gen : Nat
  deriving DecidableEq

def SnapshotRel {α : Type u} (R : Rel (Versioned α)) (snap : Nat) : Rel (Versioned α) :=
  fun v => R v ∧ v.gen ≤ snap

/-- Folding a closed relation at a snapshot is just closed psi-selection. -/
def Closed.snapshot {α : Type u} {R : Rel (Versioned α)}
    (c : Closed R) (snap : Nat) : Closed (SnapshotRel R snap) where
  carrier := c.psiSelect (fun v => v.gen ≤ snap)
  nodup := c.psiSelect_nodup (fun v => v.gen ≤ snap)
  exact := fun v => by
    rw [c.psiSelect_correct]
    rfl

/- ===================================================================== -/
/- E. Stratified negation skeleton                                      -/
/- ===================================================================== -/

structure RuleSig where
  head : RelName
  posDeps : List RelName
  negDeps : List RelName

/-- A stratum assignment: positive dependencies may stay in the same stratum;
    negative dependencies must be strictly lower. -/
def Stratified (rules : List RuleSig) (stratum : RelName → Nat) : Prop :=
  ∀ r, r ∈ rules →
    (∀ p, p ∈ r.posDeps → stratum p ≤ stratum r.head) ∧
    (∀ n, n ∈ r.negDeps → stratum n < stratum r.head)

/-- Stratification rules out direct negative self-dependency. -/
theorem stratified_no_direct_negative_self (rules : List RuleSig) (s : RelName → Nat)
    (h : Stratified rules s) {r : RuleSig} (hr : r ∈ rules) :
    r.head ∉ r.negDeps := by
  intro hmem
  have hlt : s r.head < s r.head := (h r hr).2 r.head hmem
  exact Nat.lt_irrefl (s r.head) hlt

end GPT55DependencyTheory
