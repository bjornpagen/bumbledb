import Std

/-!
Reference obligations of canonical real-algebraic identity. The finite-search
lemmas expose interpolation recovery and integer-factor completeness as premises;
they do not prove Gauss's lemma, Lagrange interpolation or Rust enumeration.
The key theorem assumes the standard uniqueness of a monic minimal polynomial
and injectivity of a polynomial's ordered distinct real-root roster. Neither a
timeout nor a partially searched grid supplies those premises. Independent exact
native fixtures check the actual factor/ordinal implementation against SymPy.
-/
namespace AlgebraicIdentity

section Field
variable {K : Type} [Lean.Grind.Field K]

theorem root_in_one_factor (f a b : K) (split : f = a*b) (root : f=0) :
    a=0 ∨ b=0 := by grind

theorem select_other_factor (f a b : K) (split : f=a*b) (root : f=0)
    (notLeft : a≠0) : b=0 := by grind

theorem nonzero_evaluation_excludes_zero_factor (f a b : K)
    (split : f=a*b) (nonzero : f≠0) : a≠0 ∧ b≠0 := by grind

theorem global_factor_sign (f a b : K) (split : f=a*b) :
    f=(-a)*(-b) := by grind

end Field

/-- A nontrivial factorization always has a factor among the searched degrees. -/
theorem one_factor_has_small_degree (a b : Nat) (ha : 0<a) (hb : 0<b) :
    (0<a ∧ a≤(a+b)/2) ∨ (0<b ∧ b≤(a+b)/2) := by omega

section FiniteSearch
variable {P I V : Type}

def Grid (allowed : I → V → Prop) (values : I → V) : Prop :=
  ∀ i, allowed i (values i)

/-- Every proper integer factor is recovered by an enumerated value tuple.
The substantive number-theoretic premises remain visible at this boundary. -/
theorem complete_grid_recovers_factor
    (proper : P → Prop) (evaluate : P → I → V)
    (allowed : I → V → Prop) (interpolate : (I → V) → P)
    (divides : ∀ p, proper p → Grid allowed (evaluate p))
    (recovery : ∀ p, proper p → interpolate (evaluate p)=p)
    (p : P) (hp : proper p) :
    ∃ values, Grid allowed values ∧ interpolate values=p := by
  exact ⟨evaluate p, divides p hp, recovery p hp⟩

theorem exhaustive_negative_proves_no_factor
    (proper : P → Prop) (evaluate : P → I → V)
    (allowed : I → V → Prop) (interpolate : (I → V) → P)
    (divides : ∀ p, proper p → Grid allowed (evaluate p))
    (recovery : ∀ p, proper p → interpolate (evaluate p)=p)
    (exhausted : ∀ values, Grid allowed values → ¬proper (interpolate values)) :
    ∀ p, ¬proper p := by
  intro p hp
  obtain ⟨values, hv, he⟩ := complete_grid_recovers_factor proper evaluate allowed interpolate divides recovery p hp
  exact exhausted values hv (he ▸ hp)

/-- Visiting only part of a grid cannot establish the exhaustive premise. -/
theorem partial_search_is_insufficient :
    let hit : Bool → Prop := fun x => x=true
    (¬hit false) ∧ ¬(∀ x, ¬hit x) := by decide

end FiniteSearch

section NumericIdentity
variable {K P : Type}

structure Description (K P : Type) where
  polynomial : P
  ordinal : Nat
  value : K

def Admitted (minimal : P → K → Prop) (ordinal : P → K → Nat)
    (description : Description K P) : Prop :=
  minimal description.polynomial description.value ∧
  description.ordinal = ordinal description.polynomial description.value

/-- Equal numeric values have equal canonical polynomial/ordinal keys, once
minimal-polynomial uniqueness and ordered-root rank injectivity are established. -/
theorem key_iff_value
    (minimal : P → K → Prop) (ordinal : P → K → Nat)
    (unique : ∀ p q x, minimal p x → minimal q x → p=q)
    (rankInjective : ∀ p x y, minimal p x → minimal p y →
      ordinal p x=ordinal p y → x=y)
    (a b : Description K P) (ha : Admitted minimal ordinal a)
    (hb : Admitted minimal ordinal b) :
    (a.polynomial=b.polynomial ∧ a.ordinal=b.ordinal) ↔ a.value=b.value := by
  constructor
  · intro h
    apply rankInjective a.polynomial a.value b.value ha.1
    · exact h.1 ▸ hb.1
    · rw [←ha.2, h.2, hb.2, h.1]
  · intro values
    have polynomials : a.polynomial=b.polynomial := by
      apply unique a.polynomial b.polynomial a.value ha.1
      exact values ▸ hb.1
    exact ⟨polynomials, by rw [ha.2, hb.2, polynomials, values]⟩

end NumericIdentity

end AlgebraicIdentity

#print axioms AlgebraicIdentity.root_in_one_factor
#print axioms AlgebraicIdentity.select_other_factor
#print axioms AlgebraicIdentity.nonzero_evaluation_excludes_zero_factor
#print axioms AlgebraicIdentity.global_factor_sign
#print axioms AlgebraicIdentity.one_factor_has_small_degree
#print axioms AlgebraicIdentity.complete_grid_recovers_factor
#print axioms AlgebraicIdentity.exhaustive_negative_proves_no_factor
#print axioms AlgebraicIdentity.partial_search_is_insufficient
#print axioms AlgebraicIdentity.key_iff_value
