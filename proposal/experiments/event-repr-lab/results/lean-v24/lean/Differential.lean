import Std

namespace Differential

def davio (x a d : Bool) : Bool := a ^^ (x && d)

theorem decomposition (f : Bool → Bool) (x : Bool) :
    davio x (f false) (f false ^^ f true) = f x := by
  cases x <;> cases h0 : f false <;> cases h1 : f true <;>
    rfl

theorem unique_coefficients (a d b e : Bool) :
    (∀ x, davio x a d = davio x b e) ↔ a = b ∧ d = e := by
  cases a <;> cases d <;> cases b <;> cases e <;> decide

theorem complement_preserves_difference (x a d : Bool) :
    !(davio x a d) = davio x (!a) d := by
  cases x <;> cases a <;> cases d <;> decide

theorem xor_coefficients (x a d b e : Bool) :
    davio x a d ^^ davio x b e = davio x (a ^^ b) (d ^^ e) := by
  cases x <;> cases a <;> cases d <;> cases b <;> cases e <;> decide

theorem product_coefficients (x a d b e : Bool) :
    (davio x a d && davio x b e) =
      davio x (a && b) ((a && e) ^^ (b && d) ^^ (d && e)) := by
  cases x <;> cases a <;> cases d <;> cases b <;> cases e <;> decide

theorem exists_root (a d : Bool) :
    (∃ x, davio x a d = true) ↔ (a || d) = true := by
  cases a <;> cases d <;> decide

theorem forall_root (a d : Bool) :
    (∀ x, davio x a d = true) ↔ (a && !d) = true := by
  cases a <;> cases d <;> decide

/-- Hiding y in y XOR x*true gives full. Hiding y separately in the two
    coefficient functions gives true XOR x*true, which is false at x=true. -/
theorem coefficient_projection_obstruction :
    (∃ y, davio true y true = true) ∧ davio true true true ≠ true := by decide

def bit (x : Bool) : Nat := if x then 1 else 0

/-- The two Shannon cofactors have overlap-sensitive total population.
    Summing this identity over remaining assignments requires the intersection
    count of the coefficient functions, not just their separate counts. -/
theorem cofactor_population (a d : Bool) :
    bit (davio false a d) + bit (davio true a d) =
      2 * bit a + bit d - 2 * bit (a && d) := by
  cases a <;> cases d <;> decide

inductive Basis where
  | shannon | positive | negative

def World : Nat → Type
  | 0 => Unit
  | n+1 => Bool × World n

def Code : Nat → Type
  | 0 => Bool
  | n+1 => Code n × Code n

def encodeTree (schedule : Nat → Basis) : (n : Nat) → (World n → Bool) → Code n
  | 0, f => f ()
  | n+1, f =>
    let lo := fun w => f (false,w)
    let hi := fun w => f (true,w)
    match schedule n with
    | .shannon => (encodeTree schedule n lo, encodeTree schedule n hi)
    | .positive => (encodeTree schedule n lo, encodeTree schedule n (fun w => lo w ^^ hi w))
    | .negative => (encodeTree schedule n hi, encodeTree schedule n (fun w => lo w ^^ hi w))

def evaluateTree (schedule : Nat → Basis) : (n : Nat) → Code n → World n → Bool
  | 0, value, _ => value
  | n+1, (a,b), (x,w) =>
    match schedule n with
    | .shannon => if x then evaluateTree schedule n b w else evaluateTree schedule n a w
    | .positive => davio x (evaluateTree schedule n a w) (evaluateTree schedule n b w)
    | .negative => davio (!x) (evaluateTree schedule n a w) (evaluateTree schedule n b w)

/-- Every Boolean function has a coefficient tree under any fixed mixed basis
    schedule. This is a full tree theorem, not a verification of Rust interning. -/
theorem tree_roundtrip (schedule : Nat → Basis) :
    ∀ n (f : World n → Bool) w,
      evaluateTree schedule n (encodeTree schedule n f) w = f w := by
  intro n
  induction n with
  | zero => intro f w; cases w; rfl
  | succ n ih =>
    intro f ⟨x,w⟩
    cases h : schedule n with
    | shannon =>
      cases x <;> simp [encodeTree, evaluateTree, h, ih]
    | positive =>
      simp only [encodeTree, evaluateTree, h, ih]
      exact decomposition (fun b => f (b,w)) x
    | negative =>
      simp only [encodeTree, evaluateTree, h, ih]
      cases x <;> cases h0 : f (false,w) <;> cases h1 : f (true,w) <;>
        rfl

/-- Equal functions have identical full coefficient trees for that schedule.
    Shared/omitted nodes in a finite arena need additional refinement invariants. -/
theorem tree_injective (schedule : Nat → Basis) :
    ∀ n (a b : Code n),
      (∀ w, evaluateTree schedule n a w = evaluateTree schedule n b w) → a = b := by
  intro n
  induction n with
  | zero => intro a b same; exact same ()
  | succ n ih =>
    intro ⟨a0,a1⟩ ⟨b0,b1⟩ same
    have coefficients : ∀ w,
        evaluateTree schedule n a0 w = evaluateTree schedule n b0 w ∧
        evaluateTree schedule n a1 w = evaluateTree schedule n b1 w := by
      intro w
      cases h : schedule n with
      | shannon =>
        constructor
        · simpa [evaluateTree,h] using same (false,w)
        · simpa [evaluateTree,h] using same (true,w)
      | positive =>
        apply (unique_coefficients _ _ _ _).mp
        intro x
        simpa [evaluateTree,h] using same (x,w)
      | negative =>
        apply (unique_coefficients _ _ _ _).mp
        intro x
        simpa [evaluateTree,h] using same (!x,w)
    have left := ih a0 b0 (fun w => (coefficients w).1)
    have right := ih a1 b1 (fun w => (coefficients w).2)
    cases left
    cases right
    rfl

end Differential

#print axioms Differential.decomposition
#print axioms Differential.unique_coefficients
#print axioms Differential.complement_preserves_difference
#print axioms Differential.xor_coefficients
#print axioms Differential.product_coefficients
#print axioms Differential.exists_root
#print axioms Differential.forall_root
#print axioms Differential.coefficient_projection_obstruction
#print axioms Differential.cofactor_population
#print axioms Differential.tree_roundtrip
#print axioms Differential.tree_injective
