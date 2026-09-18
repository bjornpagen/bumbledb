import Std

/-!
Finite rational laws represented by nonnegative integer masses and one positive
total. Fractions below retain numerator and denominator; they are exact rational
observations, not canonical arithmetic encodings. This proves the finite-law
boundary, not polynomial integration or a general semialgebraic solver.
-/
namespace FiniteMeasure

def sum {A : Type} (xs : List A) (f : A → Nat) : Nat :=
  match xs with
  | [] => 0
  | x :: rest => f x + sum rest f

theorem sum_congr {A : Type} (xs : List A) (f g : A → Nat)
    (h : ∀ x, x ∈ xs → f x = g x) : sum xs f = sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [sum]
    rw [h x (by simp), ih (fun y hy => h y (by simp [hy]))]

theorem sum_add {A : Type} (xs : List A) (f g : A → Nat) :
    sum xs (fun x => f x + g x) = sum xs f + sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih]; omega

theorem sum_mul {A : Type} (xs : List A) (k : Nat) (f : A → Nat) :
    sum xs (fun x => k * f x) = k * sum xs f := by
  induction xs with
  | nil => simp [sum]
  | cons x xs ih => simp [sum, ih, Nat.mul_add]

theorem sum_swap {A B : Type} (xs : List A) (ys : List B) (f : A → B → Nat) :
    sum xs (fun x => sum ys (f x)) = sum ys (fun y => sum xs (fun x => f x y)) := by
  induction xs with
  | nil =>
    change 0 = sum ys (fun _ => 0)
    induction ys with
    | nil => rfl
    | cons y ys ih => exact ih
  | cons x xs ih =>
    change sum ys (f x) + sum xs (fun x => sum ys (f x)) =
      sum ys (fun y => f x y + sum xs (fun x => f x y))
    rw [sum_add, ih]

def mass {W : Type} (ws : List W) (weight : W → Nat) (p : W → Bool) : Nat :=
  sum ws (fun w => if p w then weight w else 0)

structure Law (W : Type) where
  worlds : List W
  unique : worlds.Nodup
  complete : ∀ w, w ∈ worlds
  weight : W → Nat
  positive : 0 < sum worlds weight

def observe {W : Type} (law : Law W) (p evidence : W → Bool) : Option (Nat × Nat) :=
  let denominator := mass law.worlds law.weight evidence
  if denominator = 0 then none
  else some (mass law.worlds law.weight (fun w => p w && evidence w), denominator)

theorem complement_mass {W : Type} (ws : List W) (weight : W → Nat) (p : W → Bool) :
    mass ws weight p + mass ws weight (fun w => !(p w)) = sum ws weight := by
  unfold mass
  rw [← sum_add]
  apply sum_congr
  intro w _
  cases hp : p w <;> simp [hp]

theorem conditional_complement {W : Type} (ws : List W) (weight : W → Nat)
    (p g : W → Bool) :
    mass ws weight (fun w => p w && g w) + mass ws weight (fun w => !(p w) && g w) =
      mass ws weight g := by
  unfold mass
  rw [← sum_add]
  apply sum_congr
  intro w _
  cases hp : p w <;> cases hg : g w <;> simp [hp, hg]

theorem intersection_bound {W : Type} (ws : List W) (weight : W → Nat)
    (p g : W → Bool) : mass ws weight (fun w => p w && g w) ≤ mass ws weight g := by
  induction ws with
  | nil => exact Nat.le_refl 0
  | cons w ws ih =>
    have point : (if p w && g w then weight w else 0) ≤
        (if g w then weight w else 0) := by
      cases hp : p w <;> cases hg : g w <;> simp [hp, hg]
    exact Nat.add_le_add point ih

theorem partial_partition_mass {W : Type} (ws : List W) (weight : W → Nat)
    (branches : List (W → Bool)) (parent : W → Bool)
    (partition : ∀ w, w ∈ ws → sum branches (fun p => if p w then 1 else 0) =
      (if parent w then 1 else 0)) :
    sum branches (mass ws weight) = mass ws weight parent := by
  unfold mass
  rw [sum_swap]
  apply sum_congr
  intro w hw
  have scaled : sum branches (fun p => if p w then weight w else 0) =
      sum branches (fun p => weight w * (if p w then 1 else 0)) := by
    apply sum_congr
    intro p _
    cases hp : p w <;> simp [hp]
  rw [scaled, sum_mul, partition w hw]
  cases hp : parent w <;> simp [hp]

theorem partition_mass {W : Type} (ws : List W) (weight : W → Nat)
    (branches : List (W → Bool))
    (partition : ∀ w, w ∈ ws → sum branches (fun p => if p w then 1 else 0) = 1) :
    sum branches (mass ws weight) = sum ws weight := by
  unfold mass
  rw [sum_swap]
  apply sum_congr
  intro w hw
  have scaled : sum branches (fun p => if p w then weight w else 0) =
      sum branches (fun p => weight w * (if p w then 1 else 0)) := by
    apply sum_congr
    intro p _
    cases p w <;> simp
  rw [scaled, sum_mul, partition w hw, Nat.mul_one]

theorem repeated_evidence {W : Type} (law : Law W) (p g : W → Bool) :
    observe law p (fun w => g w && g w) = observe law p g := by
  have same : (fun w => g w && g w) = g := by funext w; cases g w <;> rfl
  rw [same]

theorem zero_evidence {W : Type} (law : Law W) (p g : W → Bool)
    (zero : mass law.worlds law.weight g = 0) : observe law p g = none := by
  simp [observe, zero]

theorem positive_evidence {W : Type} (law : Law W) (p g : W → Bool)
    (positive : 0 < mass law.worlds law.weight g) :
    observe law p g = some (mass law.worlds law.weight (fun w => p w && g w),
      mass law.worlds law.weight g) := by
  simp [observe, Nat.ne_of_gt positive]

theorem zero_mass_can_be_possible :
    let ws := [false, true]
    let weight := fun b : Bool => if b then 1 else 0
    let p := fun b : Bool => !b
    0 < sum ws weight ∧ mass ws weight p = 0 ∧ (∃ w, w ∈ ws ∧ p w = true) := by
  decide

theorem full_mass_need_not_be_full :
    let ws := [false, true]
    let weight := fun b : Bool => if b then 1 else 0
    mass ws weight id = sum ws weight ∧ (∃ w, w ∈ ws ∧ id w = false) := by
  decide

end FiniteMeasure
#print axioms FiniteMeasure.sum_congr
#print axioms FiniteMeasure.sum_add
#print axioms FiniteMeasure.sum_mul
#print axioms FiniteMeasure.sum_swap
#print axioms FiniteMeasure.complement_mass
#print axioms FiniteMeasure.conditional_complement
#print axioms FiniteMeasure.intersection_bound
#print axioms FiniteMeasure.partial_partition_mass
#print axioms FiniteMeasure.partition_mass
#print axioms FiniteMeasure.repeated_evidence
#print axioms FiniteMeasure.zero_evidence
#print axioms FiniteMeasure.positive_evidence
#print axioms FiniteMeasure.zero_mass_can_be_possible
#print axioms FiniteMeasure.full_mass_need_not_be_full
