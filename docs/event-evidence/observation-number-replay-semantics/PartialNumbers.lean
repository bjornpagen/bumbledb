import Std

/-!
Reference denotation of strict partial arithmetic and exact truth partitions.
Parameter solver correctness and correspondence to the Rust representation are
not proved here. The assignment type is arbitrary; univariate native support is
one instance, not a semantic restriction on the algebra.
-/
namespace PartialNumbers

def lift₂ {A B C : Type} (op : A → B → C) (a : Option A) (b : Option B) : Option C :=
  a.bind fun x => b.map fun y => op x y

def divide (a b : Option Rat) : Option Rat :=
  a.bind fun x => b.bind fun y => if y = 0 then none else some (x / y)

def defined {A : Type} (a : Option A) : Prop := ∃ x, a = some x

def holds (p : Option Bool) : Prop := p = some true
def fails (p : Option Bool) : Prop := p = some false
def missing (p : Option Bool) : Prop := p = none

def negate (p : Option Bool) : Option Bool := p.map (! ·)

def onDomain {W A : Type} (d : W → Bool) (f : W → Option A) : W → Option A :=
  fun w => if d w then f w else none

theorem binary_none_left {A B C : Type} (op : A → B → C) (b : Option B) :
    lift₂ op none b = none := rfl

theorem binary_none_right {A B C : Type} (op : A → B → C) (a : Option A) :
    lift₂ op a none = none := by cases a <;> rfl

theorem binary_some {A B C : Type} (op : A → B → C) (a : A) (b : B) :
    lift₂ op (some a) (some b) = some (op a b) := rfl

theorem binary_domain_intersection {A B C : Type} (op : A → B → C)
    (a : Option A) (b : Option B) : defined (lift₂ op a b) ↔ defined a ∧ defined b := by
  cases a <;> cases b <;> simp [lift₂, defined]

theorem constant_operation_keeps_holes {A B C : Type} (c : C) (a : Option A) (b : Option B) :
    defined (lift₂ (fun _ _ => c) a b) ↔ defined a ∧ defined b :=
  binary_domain_intersection _ _ _

theorem division_none_left (b : Option Rat) : divide none b = none := rfl

theorem division_none_right (a : Option Rat) : divide a none = none := by
  cases a <;> rfl

theorem division_by_zero (a : Option Rat) : divide a (some 0) = none := by
  cases a <;> simp [divide]

theorem division_defined_iff (a b : Option Rat) :
    defined (divide a b) ↔ ∃ x y, a = some x ∧ b = some y ∧ y ≠ 0 := by
  cases a <;> cases b <;> simp [divide, defined]

theorem zero_power_keeps_holes {A : Type} (a : Option A) :
    defined (a.map fun _ => (1 : Rat)) ↔ defined a := by
  cases a <;> simp [defined]

theorem identical_partial_values_need_not_be_totally_equal :
    (none : Option Rat) = none ∧ ¬ holds (lift₂ (fun a b : Rat => a == b) none none) := by
  simp [holds, lift₂]

theorem truth_cases_exhaustive (p : Option Bool) : holds p ∨ fails p ∨ missing p := by
  cases p with
  | none => simp [missing]
  | some b => cases b <;> simp [holds, fails]

theorem true_false_disjoint (p : Option Bool) : ¬ (holds p ∧ fails p) := by
  cases p <;> simp [holds, fails]

theorem true_undefined_disjoint (p : Option Bool) : ¬ (holds p ∧ missing p) := by
  cases p <;> simp [holds, missing]

theorem false_undefined_disjoint (p : Option Bool) : ¬ (fails p ∧ missing p) := by
  cases p <;> simp [fails, missing]

theorem negate_retains_undefined (p : Option Bool) : missing (negate p) ↔ missing p := by
  cases p <;> simp [missing, negate]

theorem negate_exchanges_truth (p : Option Bool) : holds (negate p) ↔ fails p := by
  cases p with
  | none => simp [holds, fails, negate]
  | some b => cases b <;> simp [holds, fails, negate]

theorem negation_involution (p : Option Bool) : negate (negate p) = p := by
  cases p with
  | none => rfl
  | some b => cases b <;> rfl

theorem all_boolean_lifts_are_strict (op : Bool → Bool → Bool) (a b : Option Bool) :
    defined (lift₂ op a b) ↔ defined a ∧ defined b := binary_domain_intersection _ _ _

theorem always_requires_total {W : Type} (p : W → Option Bool)
    (always : ∀ w, holds (p w)) : ∀ w, defined (p w) := by
  intro w
  exact ⟨true, always w⟩

theorem undefined_is_not_a_witness : ¬ holds none := by simp [holds]

theorem restriction_never_creates_a_value {W A : Type} (d : W → Bool)
    (f : W → Option A) (w : W) : defined (onDomain d f w) → defined (f w) := by
  simp only [onDomain]
  split <;> simp_all [defined]

theorem restriction_composition {W A : Type} (a b : W → Bool) (f : W → Option A) :
    onDomain a (onDomain b f) = onDomain (fun w => a w && b w) f := by
  funext w
  cases ha : a w <;> cases hb : b w <;> simp [onDomain, ha, hb]

/-- Retaining provenance is independent of the numerical denotation. -/
inductive Expr (Source : Type) where
  | literal
  | observed (source : Source)
  | binary (left right : Expr Source)
  | unary (value : Expr Source)

def origins {S : Type} : Expr S → List S
  | .literal => []
  | .observed source => [source]
  | .binary left right => origins left ++ origins right
  | .unary value => origins value

theorem binary_retains_origins {S : Type} (a b : Expr S) (s : S) :
    s ∈ origins (.binary a b) ↔ s ∈ origins a ∨ s ∈ origins b := by simp [origins]

theorem unary_retains_origins {S : Type} (a : Expr S) : origins (.unary a) = origins a := rfl

theorem arithmetic_never_invents_an_origin {S : Type} (a b : Expr S) (s : S)
    (ha : s ∉ origins a) (hb : s ∉ origins b) : s ∉ origins (.binary a b) := by
  simp [origins, ha, hb]

#print axioms PartialNumbers.binary_none_left
#print axioms PartialNumbers.binary_none_right
#print axioms PartialNumbers.binary_some
#print axioms PartialNumbers.binary_domain_intersection
#print axioms PartialNumbers.constant_operation_keeps_holes
#print axioms PartialNumbers.division_none_left
#print axioms PartialNumbers.division_none_right
#print axioms PartialNumbers.division_by_zero
#print axioms PartialNumbers.division_defined_iff
#print axioms PartialNumbers.zero_power_keeps_holes
#print axioms PartialNumbers.identical_partial_values_need_not_be_totally_equal
#print axioms PartialNumbers.truth_cases_exhaustive
#print axioms PartialNumbers.true_false_disjoint
#print axioms PartialNumbers.true_undefined_disjoint
#print axioms PartialNumbers.false_undefined_disjoint
#print axioms PartialNumbers.negate_retains_undefined
#print axioms PartialNumbers.negate_exchanges_truth
#print axioms PartialNumbers.negation_involution
#print axioms PartialNumbers.all_boolean_lifts_are_strict
#print axioms PartialNumbers.always_requires_total
#print axioms PartialNumbers.undefined_is_not_a_witness
#print axioms PartialNumbers.restriction_never_creates_a_value
#print axioms PartialNumbers.restriction_composition
#print axioms PartialNumbers.binary_retains_origins
#print axioms PartialNumbers.unary_retains_origins
#print axioms PartialNumbers.arithmetic_never_invents_an_origin
end PartialNumbers
