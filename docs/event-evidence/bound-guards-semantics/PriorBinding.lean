import Std

/-!
Reference semantics for explicit Beta binding of univariate source observations.
The moment recurrence is algebraic. The finite-exception theorem states its
analytic premises explicitly: a total-function integral ignores changes on a
finite set and agrees with polynomial moments. This Std-only module does NOT
prove Beta measure construction, non-atomicity, analytic moment identities,
Rust polynomial division/sign solving, BESC/SDK replay or resource accounting.
-/
namespace PriorBinding

def moment (a b : Rat) : Nat → Rat
  | 0 => 1
  | n+1 => moment a b n * ((a+n)/(a+b+n))

abbrev Polynomial := List (Rat × Nat)
def integrate (a b : Rat) : Polynomial → Rat
  | [] => 0
  | (c,n)::rest => c * moment a b n + integrate a b rest

theorem normalized (a b : Rat) : integrate a b [(1,0)] = 1 := by
  simp [integrate, moment, Rat.add_zero]

theorem integrate_append (a b : Rat) (p q : Polynomial) :
    integrate a b (p++q) = integrate a b p + integrate a b q := by
  induction p with
  | nil => simp [integrate, Rat.zero_add]
  | cons x xs ih => simp only [List.cons_append, integrate, ih]; grind

theorem integrate_scale (a b c : Rat) (p : Polynomial) :
    integrate a b (p.map (fun (x,n) => (c*x,n))) = c * integrate a b p := by
  induction p with
  | nil => simp [integrate]
  | cons x xs ih => simp only [List.map_cons, integrate, ih]; grind

def conditional (numerator evidence : Rat) : Option Rat :=
  if evidence=0 then none else some (numerator/evidence)

theorem defined_exactly (n z : Rat) :
    (conditional n z).isSome = true ↔ z ≠ 0 := by
  simp only [conditional]; split <;> simp_all

theorem impossible_even_when_numerator_zero : conditional 0 0 = none := rfl

structure Observation (S O : Type) where
  source : S
  original : O
  numerator : Polynomial
  evidence : Polynomial
  alpha : Rat
  beta : Rat

def observe {S O : Type} (o : Observation S O) : Option Rat :=
  conditional (integrate o.alpha o.beta o.numerator) (integrate o.alpha o.beta o.evidence)

theorem observation_uses_raw_integrals {S O : Type} (o : Observation S O)
    (positive : integrate o.alpha o.beta o.evidence ≠ 0) :
    observe o = some (integrate o.alpha o.beta o.numerator / integrate o.alpha o.beta o.evidence) := by
  simp [observe, conditional, positive]

theorem shared_draws : integrate 1 1 [(1,2)] = 1/3 := by decide +kernel
theorem independent_allocations : integrate 1 1 [(1,1)] * integrate 1 1 [(1,1)] = 1/4 := by
  decide +kernel
theorem shared_is_not_independent :
    integrate 1 1 [(1,2)] ≠ integrate 1 1 [(1,1)] * integrate 1 1 [(1,1)] := by decide +kernel
theorem posterior_second_head :
    conditional (integrate 1 1 [(1,2)]) (integrate 1 1 [(1,1)]) = some (2/3) := by decide +kernel
theorem averaging_the_conditional_is_wrong :
    conditional (integrate 1 1 [(1,2)]) (integrate 1 1 [(1,1)]) ≠ some (integrate 1 1 [(1,1)]) := by
  decide +kernel
theorem endpoint_conditional_remains_undefined : conditional (0*0) 0 = none := rfl

section Certificates
variable {T : Type}

/-- Original partial functions are not identified modulo null sets. A finite
exception roster certifies agreement only, and totality is a separate premise. -/
structure Certificate (domain : T → Prop) (original : T → Option Rat) (polynomial : T → Rat) where
  exceptions : List T
  total : ∀ x, domain x → ∃ y, original x = some y
  agrees : ∀ x, domain x → x ∉ exceptions → original x = some (polynomial x)

theorem holes_refuse {domain : T → Prop} {f : T → Option Rat} {p : T → Rat}
    (x : T) (inside : domain x) (hole : f x = none) : ¬ Nonempty (Certificate domain f p) := by
  intro ⟨certificate⟩
  obtain ⟨y, hy⟩ := certificate.total x inside
  rw [hole] at hy
  contradiction

/-- Soundness relative to an integral whose finite-null and polynomial-moment
contracts have been established elsewhere. Neither premise is hidden as a
global axiom, and this theorem does not construct that analytic integral. -/
theorem finite_exception_soundness (domain : T → Prop) (f : T → Option Rat) (p : T → Rat)
    (certificate : Certificate domain f p) (actual : T → Rat)
    (realizes : ∀ x, domain x → f x = some (actual x))
    (integral : (T → Rat) → Rat) (momentValue : Rat)
    (finiteNull : ∀ (a b : T → Rat) (exceptions : List T),
      (∀ x, domain x → x ∉ exceptions → a x=b x) → integral a=integral b)
    (polynomialMoment : integral p=momentValue) : integral actual=momentValue := by
  rw [←polynomialMoment]
  apply finiteNull actual p certificate.exceptions
  intro x hx he
  have h := certificate.agrees x hx he
  rw [realizes x hx] at h
  exact Option.some.inj h

end Certificates

def endpoint (p : Rat) : Option Rat := some (if p=0 then 1 else 0)

def endpointCertificate : Certificate (fun (_ : Rat) => True) endpoint (fun _ => 0) where
  exceptions := [0]
  total := by intro x _; exact ⟨_, rfl⟩
  agrees := by intro x _ h; simp only [List.mem_singleton] at h; simp [endpoint, h]

theorem endpoint_value_survives : endpoint 0 = some 1 := by decide +kernel
theorem endpoint_differs_from_zero : endpoint ≠ (fun _ => some 0) := by
  intro h
  have := congrFun h 0
  simp [endpoint] at this
theorem endpoint_polynomial_integral_zero : integrate 1 1 [] = 0 := rfl
theorem finite_exceptions_need_no_holes : ∀ x, (endpoint x).isSome = true := by
  intro x; simp [endpoint]

end PriorBinding

#print axioms PriorBinding.normalized
#print axioms PriorBinding.integrate_append
#print axioms PriorBinding.integrate_scale
#print axioms PriorBinding.defined_exactly
#print axioms PriorBinding.impossible_even_when_numerator_zero
#print axioms PriorBinding.observation_uses_raw_integrals
#print axioms PriorBinding.shared_draws
#print axioms PriorBinding.independent_allocations
#print axioms PriorBinding.shared_is_not_independent
#print axioms PriorBinding.posterior_second_head
#print axioms PriorBinding.averaging_the_conditional_is_wrong
#print axioms PriorBinding.endpoint_conditional_remains_undefined
#print axioms PriorBinding.holes_refuse
#print axioms PriorBinding.finite_exception_soundness
#print axioms PriorBinding.endpoint_value_survives
#print axioms PriorBinding.endpoint_differs_from_zero
#print axioms PriorBinding.endpoint_polynomial_integral_zero
#print axioms PriorBinding.finite_exceptions_need_no_holes
