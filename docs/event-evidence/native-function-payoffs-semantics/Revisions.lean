import Std

/-!
Finite rational revision and signed expectation. Lists below enumerate the
original legal worlds; normalized probability interpretations additionally require
nonnegative densities and exhaustive, nonrepeating rosters. The algebraic laws
hold for signed rational functions too. Division identities state their nonzero
premises explicitly; zero targets never need a prior conditional. Rat's total
zero division is not used as an implementation of impossible observations.

The indexed partition is a total bucket function. Native correspondence must
establish that the checked disjoint/covering Event roster denotes that function,
including its empty buckets. This file does not verify Rust arithmetic, region
contraction, map construction, ownership, codecs or query aggregate admission.
-/
namespace Revisions

def sum {A : Type} : List A → (A → Rat) → Rat
  | [], _ => 0
  | x :: xs, f => f x + sum xs f

theorem sum_congr {A : Type} (xs : List A) (f g : A → Rat)
    (same : ∀ x, x ∈ xs → f x = g x) : sum xs f = sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [sum]
    rw [same x (by simp), ih (fun y hy => same y (by simp [hy]))]

theorem sum_zero {A : Type} (xs : List A) : sum xs (fun _ => 0) = 0 := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih]; grind

theorem sum_add {A : Type} (xs : List A) (f g : A → Rat) :
    sum xs (fun x => f x + g x) = sum xs f + sum xs g := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum, ih]; grind

theorem sum_scale {A : Type} (xs : List A) (f : A → Rat) (k : Rat) :
    sum xs (fun x => f x * k) = sum xs f * k := by
  induction xs with
  | nil => simp [sum]
  | cons x xs ih => simp only [sum, ih]; grind

theorem sum_div {A : Type} (xs : List A) (f : A → Rat) (k : Rat) :
    sum xs (fun x => f x / k) = sum xs f / k := by
  simpa only [Rat.div_def] using sum_scale xs f k⁻¹


theorem sum_swap {A B : Type} (xs : List A) (ys : List B) (f : A → B → Rat) :
    sum xs (fun x => sum ys (f x)) = sum ys (fun y => sum xs (fun x => f x y)) := by
  induction xs with
  | nil => exact (sum_zero ys).symm
  | cons x xs ih => simp only [sum, sum_add, ih]

theorem sum_nonnegative {A : Type} (xs : List A) (f : A → Rat)
    (positive : ∀ x, x ∈ xs → 0 ≤ f x) : 0 ≤ sum xs f := by
  induction xs with
  | nil => exact Rat.le_refl
  | cons x xs ih =>
    exact Rat.add_nonneg (positive x (by simp)) (ih (fun y hy => positive y (by simp [hy])))

theorem sum_pick {A : Type} [DecidableEq A] (xs : List A) (a : A) (v : Rat)
    (unique : xs.Nodup) (member : a ∈ xs) :
    sum xs (fun x => if a = x then v else 0) = v := by
  induction xs with
  | nil => simp at member
  | cons x xs ih =>
    have hn := List.nodup_cons.mp unique
    by_cases same : a = x
    · subst a
      have tail : sum xs (fun y => if x = y then v else 0) = 0 := by
        calc
          _ = sum xs (fun _ => 0) := by
            apply sum_congr
            intro y hy
            have different : x ≠ y := by intro h; exact hn.1 (h ▸ hy)
            simp [different]
          _ = 0 := sum_zero xs
      simp only [sum, ite_true, tail, Rat.add_zero]
    · have member : a ∈ xs := (List.mem_cons.mp member).resolve_left same
      simp only [sum, same, ite_false, Rat.zero_add]
      exact ih hn.2 member

def normalizer {W : Type} (ws : List W) (prior factor : W → Rat) :=
  sum ws (fun w => prior w * factor w)
def pearl {W : Type} (ws : List W) (prior factor : W → Rat) (w : W) :=
  prior w * factor w / normalizer ws prior factor

def likelihood {W : Type} (ws : List W) (prior factor : W → Rat) : Option (W → Rat) :=
  if normalizer ws prior factor = 0 then none else some (pearl ws prior factor)

theorem impossible_likelihood {W : Type} (ws : List W) (p l : W → Rat)
    (zero : normalizer ws p l = 0) : likelihood ws p l = none := by
  simp [likelihood, zero]

theorem pearl_normalized {W : Type} (ws : List W) (p l : W → Rat)
    (defined : normalizer ws p l ≠ 0) : sum ws (pearl ws p l) = 1 := by
  unfold pearl
  rw [sum_div]
  change normalizer ws p l / normalizer ws p l = 1
  simp only [Rat.div_def, Rat.mul_inv_cancel _ defined]


theorem pearl_nonnegative {W : Type} (ws : List W) (p l : W → Rat)
    (prior : ∀ w, 0 ≤ p w) (factor : ∀ w, 0 ≤ l w)
    (defined : normalizer ws p l ≠ 0) (w : W) : 0 ≤ pearl ws p l w := by
  have mass : 0 ≤ normalizer ws p l :=
    sum_nonnegative ws _ (fun x _ => Rat.mul_nonneg (prior x) (factor x))
  have positive := Rat.lt_of_le_of_ne mass (Ne.symm defined)
  exact Rat.mul_nonneg (Rat.mul_nonneg (prior w) (factor w))
    (Rat.le_of_lt (Rat.inv_pos.mpr positive))

/-- Retaining scale matters for the receipt although it does not change a
successful posterior. In particular the normalizer need not be at most one. -/
theorem likelihood_scale {W : Type} (ws : List W) (p l : W → Rat) (k : Rat)
    (scale : k ≠ 0) (defined : normalizer ws p l ≠ 0) (w : W) :
    pearl ws p (fun x => k * l x) w = pearl ws p l w := by
  have norm : normalizer ws p (fun x => k * l x) = normalizer ws p l * k := by
    unfold normalizer
    rw [← sum_scale]
    apply sum_congr; intro x _; grind
  unfold pearl
  rw [norm]
  grind

theorem sequential_normalizer {W : Type} (ws : List W) (p a b : W → Rat) :
    normalizer ws (pearl ws p a) b =
      normalizer ws p (fun w => a w * b w) / normalizer ws p a := by
  simp only [normalizer, pearl]
  rw [← sum_div]
  apply sum_congr; intro x _; grind

/-- Sequential reweighting uses the joint factor, only where both operations
are defined. Fixed likelihoods commute; their interpretation is not inferred. -/
theorem sequential_likelihood {W : Type} (ws : List W) (p a b : W → Rat)
    (first : normalizer ws p a ≠ 0)
    (joint : normalizer ws p (fun w => a w * b w) ≠ 0) (w : W) :
    pearl ws (pearl ws p a) b w = pearl ws p (fun x => a x * b x) w := by
  change pearl ws p a w * b w / normalizer ws (pearl ws p a) b = pearl ws p (fun x => a x * b x) w
  rw [sequential_normalizer]
  unfold pearl
  grind

def indicator {W : Type} (g : W → Bool) (w : W) : Rat := if g w then 1 else 0

theorem repeated_event {W : Type} (g : W → Bool) (w : W) :
    indicator g w * indicator g w = indicator g w := by
  cases h : g w <;> simp [indicator, h]

theorem zero_mass_still_possible :
    normalizer [false, true] (fun b => if b then 1 else 0) (indicator (!·)) = 0 ∧
      (∃ w : Bool, (!w) = true) := by
  exact ⟨by decide +kernel, ⟨false, rfl⟩⟩

def cellMass {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (c : C) := sum ws (fun w => if bucket w = c then p w else 0)
def jeffrey {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat) (w : W) :=
  if q (bucket w) = 0 then 0 else q (bucket w) * p w / cellMass ws bucket p (bucket w)

/-- The admitted branch explicitly handles zero targets, even on empty cells. -/
theorem zero_target {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat) (w : W) (zero : q (bucket w) = 0) :
    jeffrey ws bucket p q w = 0 := by simp [jeffrey, zero]

theorem jeffrey_cell {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat) (c : C)
    (supported : q c ≠ 0 → cellMass ws bucket p c ≠ 0) :
    cellMass ws bucket (jeffrey ws bucket p q) c = q c := by
  by_cases zero : q c = 0
  · unfold cellMass
    rw [sum_congr ws _ (fun _ => 0)]
    · rw [sum_zero, zero]
    · intro w _; by_cases h : bucket w = c <;> simp [jeffrey, h, zero]
  · have same : cellMass ws bucket (jeffrey ws bucket p q) c =
        sum ws (fun w => (if bucket w = c then p w else 0) * (q c / cellMass ws bucket p c)) := by
      apply sum_congr
      intro w _
      by_cases h : bucket w = c <;> simp [jeffrey, h, zero] <;> grind
    rw [same, sum_scale]
    change cellMass ws bucket p c * (q c / cellMass ws bucket p c) = q c
    have nz := supported zero
    grind


theorem partition_total {W C : Type} [DecidableEq C] (ws : List W) (cs : List C)
    (bucket : W → C) (p : W → Rat) (unique : cs.Nodup)
    (complete : ∀ w, w ∈ ws → bucket w ∈ cs) :
    sum cs (cellMass ws bucket p) = sum ws p := by
  unfold cellMass
  rw [sum_swap]
  apply sum_congr
  intro w hw
  exact sum_pick cs (bucket w) (p w) unique (complete w hw)

theorem jeffrey_normalized {W C : Type} [DecidableEq C] (ws : List W) (cs : List C)
    (bucket : W → C) (p : W → Rat) (q : C → Rat) (unique : cs.Nodup)
    (complete : ∀ w, w ∈ ws → bucket w ∈ cs)
    (supported : ∀ c, q c ≠ 0 → cellMass ws bucket p c ≠ 0)
    (normalized : sum cs q = 1) : sum ws (jeffrey ws bucket p q) = 1 := by
  rw [← partition_total ws cs bucket _ unique complete]
  rw [sum_congr cs _ q (fun c _ => jeffrey_cell ws bucket p q c (supported c)), normalized]

theorem jeffrey_nonnegative {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat) (prior : ∀ w, 0 ≤ p w) (target : ∀ c, 0 ≤ q c)
    (supported : ∀ c, q c ≠ 0 → cellMass ws bucket p c ≠ 0) (w : W) :
    0 ≤ jeffrey ws bucket p q w := by
  by_cases zero : q (bucket w) = 0
  · rw [zero_target ws bucket p q w zero]; exact Rat.le_refl
  · have mass : 0 ≤ cellMass ws bucket p (bucket w) := by
      apply sum_nonnegative
      intro x _
      split <;> first | exact prior x | exact Rat.le_refl
    have positive := Rat.lt_of_le_of_ne mass (Ne.symm (supported _ zero))
    simp only [jeffrey, zero, ite_false]
    exact Rat.mul_nonneg (Rat.mul_nonneg (target _) (prior w))
      (Rat.le_of_lt (Rat.inv_pos.mpr positive))

/-- Every retained within-cell world ratio is unchanged. Summing these ratios
establishes the same statement for arbitrary Events inside that cell. -/
theorem jeffrey_within_cell {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat) (w : W)
    (positive : q (bucket w) ≠ 0)
    (_defined : cellMass ws bucket p (bucket w) ≠ 0) :
    jeffrey ws bucket p q w / q (bucket w) = p w / cellMass ws bucket p (bucket w) := by
  simp only [jeffrey, positive, ite_false]
  grind

theorem jeffrey_idempotent {W C : Type} [DecidableEq C] (ws : List W) (bucket : W → C)
    (p : W → Rat) (q : C → Rat)
    (supported : ∀ c, q c ≠ 0 → cellMass ws bucket p c ≠ 0) (w : W) :
    jeffrey ws bucket (jeffrey ws bucket p q) q w = jeffrey ws bucket p q w := by
  by_cases zero : q (bucket w) = 0
  · simp [jeffrey, zero]
  · change (if q (bucket w) = 0 then 0 else q (bucket w) * jeffrey ws bucket p q w / cellMass ws bucket (jeffrey ws bucket p q) (bucket w)) = jeffrey ws bucket p q w
    simp only [zero, ite_false]
    rw [jeffrey_cell ws bucket p q (bucket w) (supported (bucket w))]
    grind

def numerator {W : Type} (ws : List W) (p payoff : W → Rat) (g : W → Bool) :=
  sum ws (fun w => if g w then p w * payoff w else 0)
def expect {W : Type} (ws : List W) (p payoff : W → Rat) (g : W → Bool) : Option Rat :=
  let evidence := numerator ws p (fun _ => 1) g
  if evidence = 0 then none else some (numerator ws p payoff g / evidence)

theorem expectation_numerator_add {W : Type} (ws : List W) (p a b : W → Rat) (g : W → Bool) :
    numerator ws p (fun w => a w + b w) g = numerator ws p a g + numerator ws p b g := by
  unfold numerator
  rw [← sum_add]
  apply sum_congr
  intro w _; cases g w <;> simp <;> grind


/-- The signed finite-function numerator can be contracted by Event cells.
Disjointness makes these cells a piecewise function; the additive identity also
makes the danger of accidentally overlapping distinct values explicit. -/
theorem signed_cell_contraction {W C : Type} (ws : List W) (cs : List C)
    (p : W → Rat) (region : C → W → Bool) (value : C → Rat) (g : W → Bool) :
    numerator ws p (fun w => sum cs (fun c => if region c w then value c else 0)) g =
      sum cs (fun c => numerator ws p (fun _ => value c) (fun w => region c w && g w)) := by
  unfold numerator
  rw [sum_swap]
  apply sum_congr
  intro w _
  cases hg : g w with
  | false => simp only [hg, Bool.false_eq_true, ite_false, Bool.and_false]; exact (sum_zero cs).symm
  | true =>
    simp only [hg, ite_true, Bool.and_true]
    rw [Rat.mul_comm, ← sum_scale]
    apply sum_congr
    intro c _
    cases region c w <;> simp only [ite_true, Bool.false_eq_true, ite_false] <;> grind

theorem zero_evidence_not_zero_expectation {W : Type} (ws : List W) (p v : W → Rat) (g : W → Bool)
    (zero : numerator ws p (fun _ => 1) g = 0) : expect ws p v g = none := by
  simp [expect, zero]

theorem signed_expectation :
    expect [false, true] (fun _ => (1 : Rat)/2) (fun b => if b then -4 else 2) (fun _ => true) = some (-1) := by
  decide +kernel

theorem posterior_is_not_likelihood :
    let ws := [false, true]
    let p : Bool → Rat := fun b => if b then 1/5 else 4/5
    let q : Bool → Rat := fun b => if b then 4/5 else 1/5
    jeffrey ws id p q true = 4/5 ∧ pearl ws p q true = 1/2 := by decide +kernel

/-- Total division would fabricate a result where the prior conditional is
undefined. Admission must instead refuse this positive target. -/
theorem positive_target_on_zero_prior :
    let p : Bool → Rat := fun b => if b then 0 else 1
    cellMass [false, true] id p true = 0 ∧ (1/2 : Rat) ≠ 0 := by decide +kernel

theorem overlapping_jeffrey_noncommutes :
    let ws := [(false,false), (true,false), (false,true), (true,true)]
    let p : Bool × Bool → Rat := fun w => if w.1 then (if w.2 then 4/10 else 2/10) else (if w.2 then 3/10 else 1/10)
    let q : Bool → Rat := fun b => if b then 3/4 else 1/4
    jeffrey ws Prod.snd (jeffrey ws Prod.fst p q) q (true,true) ≠
      jeffrey ws Prod.fst (jeffrey ws Prod.snd p q) q (true,true) := by decide +kernel

end Revisions

#print axioms Revisions.sum_congr
#print axioms Revisions.sum_zero
#print axioms Revisions.sum_add
#print axioms Revisions.sum_scale
#print axioms Revisions.sum_div
#print axioms Revisions.impossible_likelihood
#print axioms Revisions.pearl_normalized
#print axioms Revisions.likelihood_scale
#print axioms Revisions.sequential_normalizer
#print axioms Revisions.sequential_likelihood
#print axioms Revisions.repeated_event
#print axioms Revisions.zero_mass_still_possible
#print axioms Revisions.zero_target
#print axioms Revisions.jeffrey_cell
#print axioms Revisions.jeffrey_within_cell
#print axioms Revisions.jeffrey_idempotent
#print axioms Revisions.expectation_numerator_add
#print axioms Revisions.zero_evidence_not_zero_expectation
#print axioms Revisions.signed_expectation
#print axioms Revisions.posterior_is_not_likelihood
#print axioms Revisions.positive_target_on_zero_prior
#print axioms Revisions.overlapping_jeffrey_noncommutes

#print axioms Revisions.sum_swap

#print axioms Revisions.sum_nonnegative

#print axioms Revisions.sum_pick

#print axioms Revisions.pearl_nonnegative

#print axioms Revisions.partition_total

#print axioms Revisions.jeffrey_normalized

#print axioms Revisions.jeffrey_nonnegative

#print axioms Revisions.signed_cell_contraction
