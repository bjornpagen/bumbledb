import Std

/-!
Dependence relative to admitted worlds is an ordinary functional dependency.
Its sufficient coordinate sets need not have a least element. Rectangular
support restores closure under intersection; no probability law is involved.
-/
namespace SupportedDependence

variable {ι α β : Type}
abbrev World (ι α : Type) := ι → α

def DependsOn (support : World ι α → Prop) (f : World ι α → β)
    (coordinates : ι → Prop) : Prop :=
  ∀ a b, support a → support b →
    (∀ i, coordinates i → a i = b i) → f a = f b

def splice (coordinates : ι → Prop) [DecidablePred coordinates]
    (a b : World ι α) : World ι α :=
  fun i => if coordinates i then a i else b i

def Rectangular (support : World ι α → Prop) (coordinates : ι → Prop)
    [DecidablePred coordinates] : Prop :=
  ∀ a b, support a → support b → support (splice coordinates a b)

/-- Logical domains may contain any legal values; all binary codes need not
    be admitted. This theorem says nothing about a probability factorization. -/
theorem product_support_rectangular (legal : ι → α → Prop)
    (coordinates : ι → Prop) [DecidablePred coordinates] :
    Rectangular (fun w => ∀ i, legal i (w i)) coordinates := by
  intro a b ha hb i
  by_cases h : coordinates i
  · simpa [splice, h] using ha i
  · simpa [splice, h] using hb i

/-- The missing step on coupled support is the legality of a mixed world. -/
theorem intersection_when_rectangular (support : World ι α → Prop)
    (f : World ι α → β) (d e : ι → Prop) [DecidablePred d]
    (closed : Rectangular support d)
    (hd : DependsOn support f d) (he : DependsOn support f e) :
    DependsOn support f (fun i => d i ∧ e i) := by
  intro a b ha hb agrees
  have hm := closed a b ha hb
  have left : f a = f (splice d a b) := by
    apply hd a (splice d a b) ha hm
    intro i hi
    simp [splice, hi]
  have right : f (splice d a b) = f b := by
    apply he (splice d a b) b hm hb
    intro i hi
    by_cases hdi : d i
    · simpa [splice, hdi] using agrees i ⟨hdi, hi⟩
    · simp [splice, hdi]
  exact left.trans right

def copied (w : World Bool Bool) : Prop := w false = w true
def first (w : World Bool Bool) : Bool := w false

private theorem copied_first : DependsOn copied first (fun i => i = false) := by
  intro a b _ _ agrees
  exact agrees false rfl

private theorem copied_second : DependsOn copied first (fun i => i = true) := by
  intro a b ha hb agrees
  exact ha.trans ((agrees true rfl).trans hb.symm)

private theorem copied_not_constant :
    ¬ DependsOn copied first (fun _ => False) := by
  intro h
  have bad := h (fun _ => false) (fun _ => true) rfl rfl
    (by intro i impossible; exact False.elim impossible)
  cases bad

/-- Either copy determines the event, but their empty intersection does not. -/
theorem copied_has_incomparable_dependencies :
    DependsOn copied first (fun i => i = false) ∧
    DependsOn copied first (fun i => i = true) ∧
    ¬ DependsOn copied first (fun _ => False) :=
  ⟨copied_first, copied_second, copied_not_constant⟩

/-- A support-relative "unique least dependency mask" cannot be a general
    Event invariant. A canonical raw function can still have a least mask. -/
theorem no_least_supported_dependency :
    ¬ ∃ d : Bool → Prop, DependsOn copied first d ∧
      ∀ e, DependsOn copied first e → ∀ i, d i → e i := by
  rintro ⟨d, hd, least⟩
  apply copied_not_constant
  intro a b ha hb _
  apply hd a b ha hb
  intro i hi
  have hfalse := least (fun j => j = false) copied_first i hi
  have htrue := least (fun j => j = true) copied_second i hi
  have bad : false = true := hfalse.symm.trans htrue
  cases bad

#print axioms product_support_rectangular
#print axioms intersection_when_rectangular
#print axioms copied_has_incomparable_dependencies
#print axioms no_least_supported_dependency

end SupportedDependence
