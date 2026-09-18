import Std

namespace ConstraintCounting

def Bits : Nat → Type
  | 0 => Unit
  | n+1 => Bool × Bits n

def total : (n : Nat) → (Bits n → Int) → Int
  | 0, f => f ()
  | n+1, f => total n (fun w => f (false,w)) + total n (fun w => f (true,w))

def sign (b : Bool) : Int := if b then -1 else 1
def indicator (b : Bool) : Int := if b then 1 else 0

def dot : (n : Nat) → Bits n → Bits n → Bool
  | 0, _, _ => false
  | n+1, (a,c), (b,y) => (a && b) ^^ dot n c y

def allZero : (n : Nat) → Bits n → Bool
  | 0, _ => true
  | n+1, (a,c) => !a && allZero n c

theorem total_congr (n : Nat) (f g : Bits n → Int) (h : ∀ w, f w = g w) :
    total n f = total n g := by
  induction n with
  | zero => exact h ()
  | succ n ih =>
    simp only [total]
    rw [ih _ _ (fun w => h (false,w)), ih _ _ (fun w => h (true,w))]

theorem total_add (n : Nat) (f g : Bits n → Int) :
    total n (fun w => f w + g w) = total n f + total n g := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [total, ih]; omega

theorem total_neg (n : Nat) (f : Bits n → Int) :
    total n (fun w => -(f w)) = -(total n f) := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [total, ih]; omega

theorem total_scale (n : Nat) (k : Int) (f : Bits n → Int) :
    total n (fun w => k * f w) = k * total n f := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [total, ih, Int.mul_add]

theorem total_one (n : Nat) : total n (fun _ => 1) = (2 : Int)^n := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [total, ih, Int.pow_succ]; omega

theorem sign_not (b : Bool) : sign (!b) = -(sign b) := by cases b <;> decide

/-- Character orthogonality, for any number of constraint coefficients. -/
theorem character_orthogonality (n : Nat) (c : Bits n) :
    total n (fun y => sign (dot n c y)) =
      (2 : Int)^n * indicator (allZero n c) := by
  induction n with
  | zero => cases c; rfl
  | succ n ih =>
    rcases c with ⟨a,c⟩
    cases a with
    | false =>
      simp only [total, dot, Bool.false_and, Bool.false_xor, allZero,
        Bool.not_false, Bool.true_and, ih, Int.pow_succ]
      cases allZero n c <;> simp [indicator] <;> omega
    | true =>
      simp only [total, dot, Bool.true_and, Bool.false_xor, Bool.true_xor,
        sign_not, total_neg, allZero, Bool.not_true, Bool.false_and]
      simp [indicator] <;> omega

/-- Every failed constraint cancels in the multiplier sum. The result holds
    for arbitrary constraints, not only the quadratic constraints of circuits. -/
theorem constraint_gap (n m : Nat) (constraints : Bits n → Bits m) :
    total n (fun x => total m (fun y => sign (dot m (constraints x) y))) =
      (2 : Int)^m * total n (fun x => indicator (allZero m (constraints x))) := by
  calc
    _ = total n (fun x => (2 : Int)^m * indicator (allZero m (constraints x))) :=
      total_congr n _ _ (fun x => character_orthogonality m (constraints x))
    _ = _ := total_scale n _ _

theorem zero_count_from_gap (n : Nat) (f : Bits n → Bool) :
    2 * total n (fun x => indicator (!(f x))) =
      (2 : Int)^n + total n (fun x => sign (f x)) := by
  have localIdentity : ∀ x, 2 * indicator (!(f x)) = 1 + sign (f x) := by
    intro x; cases f x <;> decide
  calc
    _ = total n (fun x => 2 * indicator (!(f x))) := (total_scale n 2 _).symm
    _ = total n (fun x => 1 + sign (f x)) := total_congr n _ _ localIdentity
    _ = _ := by rw [total_add, total_one]

/-- Doubled zero count avoids division and includes the zero-width case. -/
theorem constraint_zero_count (n m : Nat) (constraints : Bits n → Bits m) :
    2 * total n (fun x => total m (fun y => indicator (!(dot m (constraints x) y)))) =
      (2 : Int)^m * (2 : Int)^n +
      (2 : Int)^m * total n (fun x => indicator (allZero m (constraints x))) := by
  rw [← total_scale]
  calc
    _ = total n (fun x => (2 : Int)^m + total m (fun y => sign (dot m (constraints x) y))) :=
      total_congr n _ _ (fun x => zero_count_from_gap m _)
    _ = _ := by
      rw [total_add, constraint_gap]
      have h : total n (fun _ => (2 : Int)^m) = (2 : Int)^m * (2 : Int)^n := by
        simpa only [Int.mul_one, total_one] using total_scale n ((2 : Int)^m) (fun _ => 1)
      rw [h]

theorem and_gate_constraint (a b z : Bool) :
    (z ^^ (a && b)) = false ↔ z = (a && b) := by
  cases a <;> cases b <;> cases z <;> decide

theorem not_gate_constraint (a z : Bool) :
    (z ^^ true ^^ a) = false ↔ z = !a := by
  cases a <;> cases z <;> decide

theorem output_constraint (z : Bool) :
    (true ^^ z) = false ↔ z = true := by cases z <;> decide

end ConstraintCounting

#print axioms ConstraintCounting.character_orthogonality
#print axioms ConstraintCounting.constraint_gap
#print axioms ConstraintCounting.zero_count_from_gap
#print axioms ConstraintCounting.constraint_zero_count
#print axioms ConstraintCounting.and_gate_constraint
#print axioms ConstraintCounting.not_gate_constraint
#print axioms ConstraintCounting.output_constraint
