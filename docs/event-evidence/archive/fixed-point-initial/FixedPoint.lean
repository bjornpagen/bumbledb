import Std

/-!
An executable bounded reference iteration over an exhaustive finite world roster.
The roster counts legal states, not decision-diagram nodes or machine aliases.
No axiom asserts convergence: strict inclusion consumes a unit of finite rank.
The Rust fixed-point evaluator, finite-presentation admission and variance checker
must separately refine these premises. Source environments remain fixed.
-/
namespace FixedPoint

abbrev Region (W : Type) := W → Bool
def included {W : Type} (a b : Region W) := ∀ w, a w = true → b w = true
def monotone {W : Type} (F : Region W → Region W) :=
  ∀ a b, included a b → included (F a) (F b)
def empty {W : Type} : Region W := fun _ => false
def complement {W : Type} (a : Region W) : Region W := fun w => !(a w)

def iterate {W : Type} (F : Region W → Region W) : Nat → Region W
  | 0 => empty
  | n + 1 => F (iterate F n)

private theorem equal_count_agrees {W : Type} (a b : Region W)
    (inc : included a b) (xs : List W)
    (same : xs.countP a = xs.countP b) : ∀ w ∈ xs, a w = b w := by
  induction xs with
  | nil => simp
  | cons x xs ih =>
    have tail_le : xs.countP a ≤ xs.countP b :=
      List.countP_mono_left (fun w _ => inc w)
    have head : a x = b x := by
      have h := inc x
      simp only [List.countP_cons] at same
      cases ha : a x <;> cases hb : b x <;> simp_all <;> omega
    have tail : xs.countP a = xs.countP b := by
      simp only [List.countP_cons, head] at same
      omega
    intro w member
    rcases List.mem_cons.mp member with rfl | rest
    · exact head
    · exact ih tail w rest

/-- A complete roster gives a strict finite rank for proper inclusion.
    Duplicates only loosen the bound; no quotient or sample may omit a world. -/
theorem strict_rank {W : Type} (worlds : List W)
    (complete : ∀ w, w ∈ worlds) (a b : Region W)
    (inc : included a b) (different : a ≠ b) :
    worlds.countP a < worlds.countP b := by
  have le : worlds.countP a ≤ worlds.countP b :=
    List.countP_mono_left (fun w _ => inc w)
  have ne : worlds.countP a ≠ worlds.countP b := by
    intro same
    apply different
    funext w
    exact equal_count_agrees a b inc worlds same w (complete w)
  omega

theorem ascending {W : Type} (F : Region W → Region W) (mono : monotone F)
    (n : Nat) : included (iterate F n) (iterate F (n + 1)) := by
  induction n with
  | zero => intro w h; cases h
  | succ n ih => exact mono _ _ ih

theorem below_prefixed {W : Type} (F : Region W → Region W) (mono : monotone F)
    (p : Region W) (closed : included (F p) p) (n : Nat) :
    included (iterate F n) p := by
  induction n with
  | zero => intro w h; cases h
  | succ n ih => exact fun w h => closed w (mono _ _ ih w h)

private theorem equality_persists {W : Type} (F : Region W → Region W)
    (n k : Nat) (same : iterate F n = iterate F (n + 1)) :
    iterate F (n + k) = iterate F (n + k + 1) := by
  induction k with
  | zero => simpa using same
  | succ k ih => exact congrArg F ih

/-- N legal entries suffice for the least fixed point; one further application
    detects equality. This is a semantic bound, not a feasible resource budget. -/
theorem stabilizes {W : Type} (worlds : List W) (complete : ∀ w, w ∈ worlds)
    (F : Region W → Region W) (mono : monotone F) :
    iterate F worlds.length = iterate F (worlds.length + 1) := by
  by_contra unstopped
  have strict : ∀ n, n ≤ worlds.length →
      worlds.countP (iterate F n) < worlds.countP (iterate F (n + 1)) := by
    intro n within
    apply strict_rank worlds complete _ _ (ascending F mono n)
    intro same
    have h := equality_persists F n (worlds.length - n) same
    have index : n + (worlds.length - n) = worlds.length := by omega
    exact unstopped (by simpa only [index] using h)
  have grows : ∀ n, n ≤ worlds.length + 1 → n ≤ worlds.countP (iterate F n) := by
    intro n
    induction n with
    | zero => intro _; omega
    | succ n ih =>
      intro within
      have smaller := ih (by omega)
      have step := strict n (by omega)
      omega
  have lower := grows (worlds.length + 1) (by omega)
  have upper := List.countP_le_length (p := iterate F (worlds.length + 1)) (l := worlds)
  omega

theorem least_fixed_point {W : Type} (worlds : List W)
    (complete : ∀ w, w ∈ worlds) (F : Region W → Region W) (mono : monotone F) :
    F (iterate F worlds.length) = iterate F worlds.length ∧
      (∀ p, included (F p) p → included (iterate F worlds.length) p) := by
  exact ⟨(stabilizes worlds complete F mono).symm,
    fun p closed => below_prefixed F mono p closed worlds.length⟩

private theorem complement_involutive {W : Type} (a : Region W) :
    complement (complement a) = a := by
  funext w
  exact Bool.not_not (a w)

private theorem complement_reverses {W : Type} (a b : Region W) :
    included (complement a) (complement b) ↔ included b a := by
  unfold included complement
  constructor <;> intro h w
  · have at := h w
    cases ha : a w <;> cases hb : b w <;> simp_all
  · have at := h w
    cases ha : a w <;> cases hb : b w <;> simp_all

def dual {W : Type} (F : Region W → Region W) (a : Region W) : Region W :=
  complement (F (complement a))

theorem dual_monotone {W : Type} (F : Region W → Region W) (mono : monotone F) :
    monotone (dual F) := by
  intro a b inc
  apply (complement_reverses _ _).mpr
  apply mono
  apply (complement_reverses _ _).mpr
  exact inc

/-- Greatest fixed points require the same finite bound via complement duality. -/
theorem greatest_fixed_point {W : Type} (worlds : List W)
    (complete : ∀ w, w ∈ worlds) (F : Region W → Region W) (mono : monotone F) :
    let result := complement (iterate (dual F) worlds.length)
    F result = result ∧ (∀ p, included p (F p) → included p result) := by
  have fixed := least_fixed_point worlds complete (dual F) (dual_monotone F mono)
  constructor
  · have h := congrArg complement fixed.1
    simpa only [dual, complement_involutive] using h
  · intro p closed
    have pre : included (dual F (complement p)) (complement p) := by
      unfold dual
      rw [complement_involutive]
      exact (complement_reverses _ _).mpr closed
    have bound := fixed.2 (complement p) pre
    apply (complement_reverses _ _).mp
    simpa only [complement_involutive] using bound

/-- Every fixed environment uses the same finite roster/bound. The environment
    type may be infinite because this operator never mixes its fibres. -/
theorem shared_environment_stabilizes {Env W : Type} (worlds : List W)
    (complete : ∀ w, w ∈ worlds) (F : Env → Region W → Region W)
    (mono : ∀ env, monotone (F env)) :
    (fun env => iterate (F env) worlds.length) =
      (fun env => iterate (F env) (worlds.length + 1)) := by
  funext env
  exact stabilizes worlds complete (F env) (mono env)

end FixedPoint

#print axioms FixedPoint.strict_rank
#print axioms FixedPoint.ascending
#print axioms FixedPoint.below_prefixed
#print axioms FixedPoint.stabilizes
#print axioms FixedPoint.least_fixed_point
#print axioms FixedPoint.dual_monotone
#print axioms FixedPoint.greatest_fixed_point
#print axioms FixedPoint.shared_environment_stabilizes
