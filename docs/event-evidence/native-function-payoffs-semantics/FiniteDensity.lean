import Std

/-!
The finite density contraction reference. Rational densities are represented by
nonnegative integer numerators with a shared positive denominator. Every finite
rational family admits that presentation; constructing its common denominator,
Rust arbitrary precision arithmetic, graph counts and BEVT parsing remain native
obligations. Worlds below are the original legal worlds, not decoder aliases.
The same theorem applies at a fixed parameter, but never sums parameter guards.
-/
namespace FiniteDensity

def sum {A : Type} : List A → (A → Nat) → Nat
  | [], _ => 0
  | x :: xs, f => f x + sum xs f

theorem sum_congr {A : Type} (xs : List A) (f g : A → Nat)
    (same : ∀ x, x ∈ xs → f x = g x) : sum xs f = sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [sum]
    rw [same x (by simp), ih (fun y hy => same y (by simp [hy]))]

theorem sum_add {A : Type} (xs : List A) (f g : A → Nat) :
    sum xs (fun x => f x + g x) = sum xs f + sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih]; omega

theorem sum_zero {A : Type} (xs : List A) : sum xs (fun _ => 0) = 0 := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih, Nat.zero_add]

theorem sum_scale {A : Type} (xs : List A) (f : A → Nat) (k : Nat) :
    sum xs (fun x => f x * k) = sum xs f * k := by
  induction xs with
  | nil => simp only [sum, Nat.zero_mul]
  | cons x xs ih => simp only [sum, ih, Nat.add_mul]

theorem sum_swap {A B : Type} (xs : List A) (ys : List B) (f : A → B → Nat) :
    sum xs (fun x => sum ys (f x)) = sum ys (fun y => sum xs (fun x => f x y)) := by
  induction xs with
  | nil => exact (sum_zero ys).symm
  | cons x xs ih => simp only [sum, sum_add, ih]

def count {W : Type} (ws : List W) (p : W → Bool) := sum ws (fun w => if p w then 1 else 0)
def mass {W : Type} (ws : List W) (density : W → Nat) (p : W → Bool) :=
  sum ws (fun w => if p w then density w else 0)
def density {W C : Type} (cs : List C) (region : C → W → Bool) (value : C → Nat) (w : W) :=
  sum cs (fun c => if region c w then value c else 0)
def contract {W C : Type} (ws : List W) (cs : List C)
    (region : C → W → Bool) (value : C → Nat) (p : W → Bool) :=
  sum cs (fun c => count ws (fun w => region c w && p w) * value c)

/-- Symbolic counts times per-world density agree with enumerated joint mass.
Disjointness is needed to interpret cells as a piecewise function; this additive
identity also exposes why overlapping constructor inputs would double count. -/
theorem contraction_exact {W C : Type} (ws : List W) (cs : List C)
    (region : C → W → Bool) (value : C → Nat) (p : W → Bool) :
    contract ws cs region value p = mass ws (density cs region value) p := by
  unfold contract count
  have scaled : ∀ c, sum ws (fun w => if region c w && p w then 1 else 0) * value c =
      sum ws (fun w => if region c w && p w then value c else 0) := by
    intro c
    rw [← sum_scale]
    apply sum_congr
    intro w _
    cases region c w <;> cases p w <;> simp
  rw [sum_congr cs _ _ (fun c _ => scaled c), sum_swap]
  apply sum_congr
  intro w _
  change sum cs (fun c => if region c w && p w then value c else 0) =
    if p w then density cs region value w else 0
  cases hp : p w with
  | false => simpa only [hp, Bool.and_false, Bool.false_eq_true, ite_false] using sum_zero cs
  | true => simp [density]

theorem disjoint_union_counts {W : Type} (ws : List W) (a b : W → Bool)
    (disjoint : ∀ w, w ∈ ws → ¬(a w = true ∧ b w = true)) :
    count ws (fun w => a w || b w) = count ws a + count ws b := by
  unfold count
  rw [← sum_add]
  apply sum_congr
  intro w hw
  cases ha : a w <;> cases hb : b w <;> simp [ha, hb]
  exact False.elim (disjoint w hw ⟨ha, hb⟩)

/-- Merging disjoint equal-density regions preserves their contribution. -/
theorem merge_equal_density {W : Type} (ws : List W) (a b p : W → Bool) (k : Nat)
    (disjoint : ∀ w, w ∈ ws → ¬(a w = true ∧ b w = true)) :
    count ws (fun w => (a w || b w) && p w) * k =
      count ws (fun w => a w && p w) * k + count ws (fun w => b w && p w) * k := by
  have split : (fun w => (a w || b w) && p w) =
      (fun w => (a w && p w) || (b w && p w)) := by
    funext w; cases a w <;> cases b w <;> cases p w <;> rfl
  rw [split, disjoint_union_counts, Nat.add_mul]
  intro w hw h
  apply disjoint w hw
  cases ha : a w <;> cases hb : b w <;> simp_all

theorem omit_zero_density {W : Type} (ws : List W) (p : W → Bool) : count ws p * 0 = 0 := by
  exact Nat.mul_zero _

theorem conditional_complement {W : Type} (ws : List W) (d : W → Nat) (p g : W → Bool) :
    mass ws d (fun w => p w && g w) + mass ws d (fun w => !(p w) && g w) = mass ws d g := by
  unfold mass
  rw [← sum_add]
  apply sum_congr
  intro w _
  cases hp : p w <;> cases hg : g w <;> simp [hp, hg]

structure Law (W : Type) where
  worlds : List W
  unique : worlds.Nodup
  complete : ∀ w, w ∈ worlds
  denominator : Nat
  positive : 0 < denominator
  density : W → Nat
  normalized : sum worlds density = denominator

def observe {W : Type} (law : Law W) (p g : W → Bool) : Option (Nat × Nat) :=
  let evidence := mass law.worlds law.density g
  if evidence = 0 then none else some (mass law.worlds law.density (fun w => p w && g w), evidence)

theorem full_mass {W : Type} (law : Law W) :
    mass law.worlds law.density (fun _ => true) = law.denominator := by
  simpa [mass] using law.normalized

theorem zero_evidence {W : Type} (law : Law W) (p g : W → Bool)
    (zero : mass law.worlds law.density g = 0) : observe law p g = none := by
  simp [observe, zero]

theorem positive_evidence {W : Type} (law : Law W) (p g : W → Bool)
    (positive : 0 < mass law.worlds law.density g) :
    observe law p g = some (mass law.worlds law.density (fun w => p w && g w),
      mass law.worlds law.density g) := by
  simp [observe, Nat.ne_of_gt positive]

theorem repeated_evidence {W : Type} (law : Law W) (p g : W → Bool) :
    observe law p (fun w => g w && g w) = observe law p g := by
  have same : (fun w => g w && g w) = g := by funext w; cases g w <;> rfl
  rw [same]

/-- Every skipped stochastic bit has two outcomes, even when a reduced law and
Event have no node mentioning it. Deterministic guards do not use this roster. -/
theorem skipped_outcome_multiplicity {W : Type} (ws : List W) (d : W → Nat) (p : W → Bool) :
    mass (ws.flatMap fun w => [(w, false), (w, true)]) (fun w => d w.1) (fun w => p w.1) =
      2 * mass ws d p := by
  induction ws with
  | nil => rfl
  | cons w ws ih =>
    simp only [List.flatMap_cons, List.cons_append, List.nil_append, mass, sum] at *
    rw [ih]
    omega

/-- A legal zero-density world is still distinguishable structurally. -/
theorem zero_mass_nonempty :
    mass [false, true] (fun w => if w then 1 else 0) (fun w => !w) = 0 ∧
      (∃ w : Bool, (!w) = true) := by
  exact ⟨rfl, ⟨false, rfl⟩⟩

/-- Same one-coordinate marginals, different joint laws: multiplication of
marginals would be an unjustified independence assumption. Both totals are 4. -/
theorem equal_marginals_different_joint :
    let ws := [(false, false), (false, true), (true, false), (true, true)]
    let correlated := fun w : Bool × Bool => if w.1 == w.2 then 2 else 0
    let uniform := fun _ : Bool × Bool => 1
    mass ws correlated (·.1) = mass ws uniform (·.1) ∧
    mass ws correlated (·.2) = mass ws uniform (·.2) ∧
    mass ws correlated (fun w => w.1 && w.2) ≠ mass ws uniform (fun w => w.1 && w.2) := by
  decide

/-- Normalization is checked against an authored denominator, not repaired by
replacing it with whatever sum the constructor happened to receive. -/
theorem unnormalized_refused (total denominator : Nat) (bad : total ≠ denominator) :
    ¬(total = denominator ∧ 0 < denominator) := by
  intro h; exact bad h.1

structure Context (S W M : Type) where
  source : S
  support : W → Bool
  designated : Option M

/-- Even a constant Event retains its designated law in its context. -/
theorem changed_measurement_distinct {S W M : Type} (s : S) (h : W → Bool) (a b : Option M)
    (different : a ≠ b) : (Context.mk s h a) ≠ (Context.mk s h b) := by
  intro same; exact different (congrArg Context.designated same)

theorem missing_not_impossible : (none : Option (Option Nat)) ≠ some none := by decide

end FiniteDensity

#print axioms FiniteDensity.sum_congr
#print axioms FiniteDensity.sum_add
#print axioms FiniteDensity.sum_zero
#print axioms FiniteDensity.sum_scale
#print axioms FiniteDensity.sum_swap
#print axioms FiniteDensity.contraction_exact
#print axioms FiniteDensity.disjoint_union_counts
#print axioms FiniteDensity.merge_equal_density
#print axioms FiniteDensity.omit_zero_density
#print axioms FiniteDensity.conditional_complement
#print axioms FiniteDensity.full_mass
#print axioms FiniteDensity.zero_evidence
#print axioms FiniteDensity.positive_evidence
#print axioms FiniteDensity.repeated_evidence
#print axioms FiniteDensity.skipped_outcome_multiplicity
#print axioms FiniteDensity.zero_mass_nonempty
#print axioms FiniteDensity.equal_marginals_different_joint
#print axioms FiniteDensity.unnormalized_refused
#print axioms FiniteDensity.changed_measurement_distinct
#print axioms FiniteDensity.missing_not_impossible
