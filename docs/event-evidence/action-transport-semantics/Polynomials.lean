import Std

/-!
Reference laws for named-parameter polynomial arithmetic at rational assignments.
Expanded terms need not be normalized here: the append/product/merge equations
justify transformations of their denotation. These proofs do not verify Rust's
sorting, sparse normal-form uniqueness, bigint implementation, resource limits or
BEPL codec, and do not decide equality on constrained real parameter domains.

Beta moments below are the algebraic recurrence used by the restricted explicit
prior constructor. No analytic integration theorem is claimed in this module.
-/
namespace Polynomials

def power (x : Rat) : Nat → Rat
  | 0 => 1
  | n + 1 => power x n * x

theorem power_add (x : Rat) (m n : Nat) : power x (m+n) = power x m * power x n := by
  induction n with
  | zero => simp [power]
  | succ n ih => simp only [power]; grind

def monomial {P : Type} (assignment : P → Rat) : List (P × Nat) → Rat
  | [] => 1
  | (p,n) :: xs => power (assignment p) n * monomial assignment xs

theorem monomial_append {P : Type} (a : P → Rat) (xs ys : List (P × Nat)) :
    monomial a (xs ++ ys) = monomial a xs * monomial a ys := by
  induction xs with
  | nil => simp [monomial]
  | cons x xs ih => simp only [List.cons_append, monomial, ih]; grind

theorem repeated_name_adds_degree {P : Type} (a : P → Rat) (p : P) (m n : Nat) :
    monomial a [(p,m), (p,n)] = monomial a [(p,m+n)] := by
  simp [monomial, power_add]

abbrev Term (P : Type) := Rat × List (P × Nat)
abbrev Polynomial (P : Type) := List (Term P)

def evaluate {P : Type} (a : P → Rat) : Polynomial P → Rat
  | [] => 0
  | (coefficient, powers) :: xs => coefficient * monomial a powers + evaluate a xs

theorem evaluate_append {P : Type} (a : P → Rat) (xs ys : Polynomial P) :
    evaluate a (xs ++ ys) = evaluate a xs + evaluate a ys := by
  induction xs with
  | nil => simp only [List.nil_append, evaluate, Rat.zero_add]
  | cons x xs ih => simp only [List.cons_append, evaluate, ih]; grind

theorem merge_coefficients {P : Type} (a : P → Rat) (powers : List (P × Nat))
    (c d : Rat) (rest : Polynomial P) :
    evaluate a ((c,powers) :: (d,powers) :: rest) =
      evaluate a ((c+d,powers) :: rest) := by
  simp only [evaluate]; grind

theorem remove_zero_coefficient {P : Type} (a : P → Rat) (powers : List (P × Nat))
    (rest : Polynomial P) : evaluate a ((0,powers) :: rest) = evaluate a rest := by
  simp only [evaluate, Rat.zero_mul, Rat.zero_add]

def multiplyTerm {P : Type} (x : Term P) (ys : Polynomial P) : Polynomial P :=
  ys.map (fun y => (x.1 * y.1, x.2 ++ y.2))

theorem evaluate_multiplyTerm {P : Type} (a : P → Rat) (x : Term P) (ys : Polynomial P) :
    evaluate a (multiplyTerm x ys) = x.1 * monomial a x.2 * evaluate a ys := by
  induction ys with
  | nil => simp [evaluate, multiplyTerm]
  | cons y ys ih =>
    simp only [multiplyTerm, List.map_cons, evaluate, monomial_append] at *
    rw [ih]
    grind

def multiply {P : Type} (xs ys : Polynomial P) : Polynomial P :=
  xs.flatMap (fun x => multiplyTerm x ys)

theorem evaluate_multiply {P : Type} (a : P → Rat) (xs ys : Polynomial P) :
    evaluate a (multiply xs ys) = evaluate a xs * evaluate a ys := by
  induction xs with
  | nil => simp [multiply, evaluate]
  | cons x xs ih =>
    simp only [multiply, List.flatMap_cons, evaluate_append, evaluate_multiplyTerm] at *
    rw [ih]
    simp only [evaluate]
    grind

theorem monomial_rename {P Q : Type} (a : Q → Rat) (rename : P → Q) (xs : List (P × Nat)) :
    monomial a (xs.map (fun (p,n) => (rename p,n))) = monomial (a ∘ rename) xs := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp [monomial, ih]

def polynomialPower {P : Type} (xs : Polynomial P) : Nat → Polynomial P
  | 0 => [(1,[])]
  | n+1 => multiply (polynomialPower xs n) xs

theorem evaluate_power {P : Type} (a : P → Rat) (xs : Polynomial P) (n : Nat) :
    evaluate a (polynomialPower xs n) = power (evaluate a xs) n := by
  induction n with
  | zero => simp [polynomialPower, evaluate, monomial, power, Rat.add_zero]
  | succ n ih => simp only [polynomialPower, evaluate_multiply, ih, power]

def substituteMonomial {P Q : Type} (replacement : P → Polynomial Q) :
    List (P × Nat) → Polynomial Q
  | [] => [(1,[])]
  | (p,n) :: xs => multiply (polynomialPower (replacement p) n) (substituteMonomial replacement xs)

theorem evaluate_substituteMonomial {P Q : Type} (a : Q → Rat)
    (replacement : P → Polynomial Q) (xs : List (P × Nat)) :
    evaluate a (substituteMonomial replacement xs) =
      monomial (fun p => evaluate a (replacement p)) xs := by
  induction xs with
  | nil => simp [substituteMonomial, evaluate, monomial, Rat.add_zero]
  | cons x xs ih => simp only [substituteMonomial, evaluate_multiply, evaluate_power, ih, monomial]

def substitute {P Q : Type} (replacement : P → Polynomial Q) (xs : Polynomial P) : Polynomial Q :=
  xs.flatMap (fun x => multiply [(x.1,[])] (substituteMonomial replacement x.2))

/-- Replacements are evaluated at the original assignment, not recursively
replaced a second time. This also covers explicit parameter renaming/sharing. -/
theorem evaluate_substitute {P Q : Type} (a : Q → Rat)
    (replacement : P → Polynomial Q) (xs : Polynomial P) :
    evaluate a (substitute replacement xs) =
      evaluate (fun p => evaluate a (replacement p)) xs := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [substitute, List.flatMap_cons, evaluate_append, evaluate_multiply,
      evaluate_substituteMonomial] at *
    rw [ih]
    simp only [evaluate, monomial, Rat.mul_one, Rat.add_zero]

theorem two_draws_normalize (p : Rat) :
    p*p + p*(1-p) + (1-p)*p + (1-p)*(1-p) = 1 := by grind

theorem discard_fresh_draw (p : Rat) : p*p + p*(1-p) = p := by grind

theorem shared_bias_evidence (p : Rat) :
    p*(1-p) + (1-p)*p = 2*p*(1-p) := by grind

theorem conditional_fairness (p : Rat) (positiveEvidence : 2*p*(1-p) ≠ 0) :
    p*(1-p) / (2*p*(1-p)) = 1/2 := by
  have factor : p*(1-p) = (1/2) * (2*p*(1-p)) := by grind
  rw [factor, Rat.mul_div_cancel positiveEvidence]

theorem fair_filter_has_endpoint_failures :
    (2*(0:Rat)*(1-0) = 0) ∧ (2*(1:Rat)*(1-1) = 0) := by decide +kernel

theorem unrelated_biases_are_not_fair :
    (1/4:Rat)*(1-3/4) ≠ (1-1/4)*(3/4) := by decide +kernel

theorem copy_and_two_draws_differ : (1/2:Rat) ≠ (1/2:Rat)*(1/2) := by decide +kernel

def betaMoment (alpha beta : Rat) : Nat → Rat
  | 0 => 1
  | n + 1 => betaMoment alpha beta n * ((alpha + n) / (alpha + beta + n))

theorem beta_successor (alpha beta : Rat) (n : Nat) :
    betaMoment alpha beta (n+1) =
      betaMoment alpha beta n * ((alpha+n)/(alpha+beta+n)) := rfl

theorem shared_prior_second_moment : betaMoment 1 1 2 = 1/3 := by decide +kernel

theorem separate_prior_second_moment :
    betaMoment 1 1 1 * betaMoment 1 1 1 = 1/4 := by decide +kernel

theorem shared_prior_is_not_two_allocations :
    betaMoment 1 1 2 ≠ betaMoment 1 1 1 * betaMoment 1 1 1 := by decide +kernel

end Polynomials

#print axioms Polynomials.power_add
#print axioms Polynomials.monomial_append
#print axioms Polynomials.repeated_name_adds_degree
#print axioms Polynomials.evaluate_append
#print axioms Polynomials.merge_coefficients
#print axioms Polynomials.remove_zero_coefficient
#print axioms Polynomials.evaluate_multiplyTerm
#print axioms Polynomials.evaluate_multiply
#print axioms Polynomials.monomial_rename
#print axioms Polynomials.evaluate_power
#print axioms Polynomials.evaluate_substituteMonomial
#print axioms Polynomials.evaluate_substitute
#print axioms Polynomials.two_draws_normalize
#print axioms Polynomials.discard_fresh_draw
#print axioms Polynomials.shared_bias_evidence
#print axioms Polynomials.conditional_fairness
#print axioms Polynomials.fair_filter_has_endpoint_failures
#print axioms Polynomials.unrelated_biases_are_not_fair
#print axioms Polynomials.copy_and_two_draws_differ
#print axioms Polynomials.beta_successor
#print axioms Polynomials.shared_prior_second_moment
#print axioms Polynomials.separate_prior_second_moment
#print axioms Polynomials.shared_prior_is_not_two_allocations
