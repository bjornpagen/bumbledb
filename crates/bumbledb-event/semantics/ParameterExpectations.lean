import Std

/-!
Pointwise signed expectations for parameter families. One finite roster lists
outcomes at a single parameter assignment; guard values are never summed as
random alternatives. Native admission must establish normalized nonnegative
laws, complete outcome rosters, and the payoff's disjoint-cell representation.
The algebra below uses generic fields and does not verify Rust contraction,
the univariate solver, transport, or query-observable coverage.
-/
namespace ParameterExpectations

section Algebra
variable {O C T K : Type} [Lean.Grind.Field K]

def sum : List O → (O → K) → K
  | [], _ => 0
  | x::xs, f => f x + sum xs f

theorem sum_congr (xs : List O) (f g : O → K) (same : ∀ x, f x=g x) :
    sum xs f=sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum,same,ih]

theorem sum_zero (xs : List O) : sum xs (fun _ => (0 : K))=0 := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum,ih]; grind

theorem sum_add (xs : List O) (f g : O → K) :
    sum xs (fun x => f x+g x)=sum xs f+sum xs g := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind

theorem sum_scale (xs : List O) (f : O → K) (k : K) :
    sum xs (fun x => k*f x)=k*sum xs f := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind

theorem sum_mul (xs : List O) (f : O → K) (k : K) :
    sum xs (fun x => f x*k)=sum xs f*k := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind

theorem sum_swap (xs : List O) (cs : List C) (f : O → C → K) :
    sum xs (fun x => sum cs (f x))=sum cs (fun c => sum xs (fun x => f x c)) := by
  induction xs with
  | nil => exact (sum_zero cs).symm
  | cons x xs ih => simp only [sum,sum_add,ih]

def mass (xs : List O) (density : O → K) (evidence : O → Bool) : K :=
  sum xs (fun x => if evidence x then density x else 0)

def numerator (xs : List O) (density payoff : O → K) (evidence : O → Bool) : K :=
  sum xs (fun x => payoff x * (if evidence x then density x else 0))

theorem cell_contraction (xs : List O) (cs : List C) (density payoff : O → K)
    (evidence : O → Bool) (inside : C → O → Bool) (value : C → K)
    (presentation : ∀ x, payoff x=sum cs (fun c => if inside c x then value c else 0)) :
    numerator xs density payoff evidence =
      sum cs (fun c => value c*mass xs density (fun x => inside c x && evidence x)) := by
  calc
    _ = sum xs (fun x => sum cs (fun c =>
        (if inside c x then value c else 0)*(if evidence x then density x else 0))) := by
      apply sum_congr
      intro x
      rw [presentation x,sum_mul]
    _ = sum cs (fun c => sum xs (fun x =>
        (if inside c x then value c else 0)*(if evidence x then density x else 0))) :=
      sum_swap xs cs _
    _ = _ := by
      apply sum_congr
      intro c
      calc
        _ = sum xs (fun x => value c *
            (if inside c x && evidence x then density x else 0)) := by
          apply sum_congr
          intro x
          cases hi : inside c x <;> cases he : evidence x <;> simp <;> grind
        _ = _ := sum_scale xs _ (value c)

theorem numerator_linearity (xs : List O) (density a b : O → K) (evidence : O → Bool) :
    numerator xs density (fun x => a x+b x) evidence =
      numerator xs density a evidence+numerator xs density b evidence := by
  unfold numerator
  rw [←sum_add]
  apply sum_congr
  intro x
  grind

noncomputable def observe (domain : T → Prop) (xs : List O)
    (density payoff : T → O → K) (evidence : T → O → Bool) (t : T) : Option K := by
  classical
  exact if domain t ∧ mass xs (density t) (evidence t)≠0 then
    some (numerator xs (density t) (payoff t) (evidence t)/mass xs (density t) (evidence t))
  else none

theorem defined_exactly (domain : T → Prop) (xs : List O)
    (density payoff : T → O → K) (evidence : T → O → Bool) (t : T) :
    (observe domain xs density payoff evidence t).isSome=true ↔
      domain t ∧ mass xs (density t) (evidence t)≠0 := by
  classical
  simp only [observe]
  split <;> simp_all

theorem zero_evidence_undefined_even_for_zero_payoff (domain : T → Prop) (xs : List O)
    (density : T → O → K) (evidence : T → O → Bool) (t : T)
    (zero : mass xs (density t) (evidence t)=0) :
    observe domain xs density (fun _ _ => 0) evidence t=none := by
  simp [observe,zero]

theorem indicator_numerator (xs : List O) (density : O → K) (event evidence : O → Bool) :
    numerator xs density (fun x => if event x then 1 else 0) evidence =
      mass xs density (fun x => event x && evidence x) := by
  apply sum_congr
  intro x
  cases ha : event x <;> cases hg : evidence x <;> simp <;> grind

end Algebra

theorem signed_values_are_not_probabilities :
    numerator [()] (fun _ => (1 : Rat)) (fun _ => -2) (fun _ => true)= -2 ∧
    numerator [()] (fun _ => (1 : Rat)) (fun _ => 4) (fun _ => true)=4 := by
  decide +kernel

end ParameterExpectations

#print axioms ParameterExpectations.sum_congr
#print axioms ParameterExpectations.sum_zero
#print axioms ParameterExpectations.sum_add
#print axioms ParameterExpectations.sum_scale
#print axioms ParameterExpectations.sum_mul
#print axioms ParameterExpectations.sum_swap
#print axioms ParameterExpectations.cell_contraction
#print axioms ParameterExpectations.numerator_linearity
#print axioms ParameterExpectations.defined_exactly
#print axioms ParameterExpectations.zero_evidence_undefined_even_for_zero_payoff
#print axioms ParameterExpectations.indicator_numerator
#print axioms ParameterExpectations.signed_values_are_not_probabilities
