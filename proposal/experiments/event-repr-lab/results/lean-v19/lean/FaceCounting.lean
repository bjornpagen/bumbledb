import Std

namespace FaceCounting

def total {X : Type} : List X → (X → Nat) → Nat
  | [], _ => 0
  | x :: xs, f => f x + total xs f

def count {X : Type} (xs : List X) (A : X → Bool) : Nat :=
  total xs (fun x => if A x then 1 else 0)

def pairs {X Y : Type} (xs : List X) (ys : List Y) (A : X → Y → Bool) : Nat :=
  total xs (fun x => count ys (A x))

theorem total_constant {X : Type} (xs : List X) (n : Nat) :
    total xs (fun _ => n) = xs.length * n := by
  induction xs with
  | nil => simp [total]
  | cons x xs ih => simp [total, ih, Nat.add_mul, Nat.add_comm]

theorem total_scale {X : Type} (xs : List X) (f : X → Nat) (m : Nat) :
    total xs (fun x => m * f x) = m * total xs f := by
  induction xs with
  | nil => simp [total]
  | cons x xs ih => simp [total, ih, Nat.mul_add]

/-- Lists witness the finite factors. For duplicate-free lists this is set
    cardinality; with duplicates the same statement is about multiplicity. -/
theorem omitted_factor_count {X Y : Type} (xs : List X) (ys : List Y)
    (A : X → Bool) :
    pairs xs ys (fun x _ => A x) = ys.length * count xs A := by
  simp only [pairs, count, total_constant]
  exact total_scale xs (fun x => if A x then 1 else 0) ys.length

/-- Environment partitions retain their own domain size. Empty environments
    contribute zero, rather than being mapped to a nonempty decoder context. -/
theorem indexed_factor_count {E X Y : Type} (es : List E)
    (xs : E → List X) (ys : E → List Y) (A : E → X → Bool) :
    total es (fun e => pairs (xs e) (ys e) (fun x _ => A e x)) =
      total es (fun e => (ys e).length * count (xs e) (A e)) := by
  simp only [omitted_factor_count]

/-- Expand into all raw omitted codes, divide their exact multiplicity out,
    then substitute the legal fibre size. The legal and raw factors can differ. -/
theorem raw_fibre_replacement {X Y Z : Type} (xs : List X)
    (raw : List Y) (legal : List Z) (A : X → Bool)
    (nonempty : raw.length > 0) :
    pairs xs raw (fun x _ => A x) / raw.length * legal.length =
      pairs xs legal (fun x _ => A x) := by
  rw [omitted_factor_count, omitted_factor_count]
  rw [Nat.mul_div_cancel_left _ nonempty]
  exact Nat.mul_comm _ _

/-- A coupled support is not a product. Dropping its missing pairs would double
    this count even though the Event ignores the omitted coordinate. -/
theorem coupled_support_counterexample :
    pairs [false, true] [false, true] (fun x y => x == y) = 2 ∧
    ([false, true] : List Bool).length * count [false, true] (fun _ => true) = 4 := by
  decide

#print axioms omitted_factor_count
#print axioms indexed_factor_count
#print axioms raw_fibre_replacement
#print axioms coupled_support_counterexample
end FaceCounting
