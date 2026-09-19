import Std

/-!
Exact weighted fibres and normalized conditional channels. Nonnegative rational
weights are represented by integer numerators and retained common denominators.
The world rosters are explicit; application to native sources requires unique,
exhaustive original legal worlds. No decoder aliases or stochastic guard bits
occur. This does not verify Rust's raw scalar partitions, arithmetic library,
map dependency extraction, arena ownership or memoization.
-/
namespace WeightedMaps

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

theorem sum_zero {A : Type} (xs : List A) : sum xs (fun _ => 0) = 0 := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih, Nat.zero_add]

theorem sum_add {A : Type} (xs : List A) (f g : A → Nat) :
    sum xs (fun x => f x + g x) = sum xs f + sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum, ih]; omega

theorem sum_scale {A : Type} (xs : List A) (k : Nat) (f : A → Nat) :
    sum xs (fun x => k * f x) = k * sum xs f := by
  induction xs with
  | nil => simp only [sum, Nat.mul_zero]
  | cons x xs ih => simp only [sum, ih, Nat.mul_add]

theorem sum_swap {A B : Type} (xs : List A) (ys : List B) (f : A → B → Nat) :
    sum xs (fun x => sum ys (f x)) = sum ys (fun y => sum xs (fun x => f x y)) := by
  induction xs with
  | nil => exact (sum_zero ys).symm
  | cons x xs ih => simp only [sum, sum_add, ih]

theorem sum_pick {A : Type} [DecidableEq A] (xs : List A) (a : A) (v : A → Nat)
    (unique : xs.Nodup) (member : a ∈ xs) :
    sum xs (fun x => if a = x then v x else 0) = v a := by
  induction xs with
  | nil => simp at member
  | cons x xs ih =>
    have hn := List.nodup_cons.mp unique
    by_cases same : a = x
    · subst a
      have tail : sum xs (fun y => if x = y then v y else 0) = 0 := by
        calc
          _ = sum xs (fun _ => 0) := by
            apply sum_congr
            intro y hy
            have different : x ≠ y := by intro h; exact hn.1 (h ▸ hy)
            simp [different]
          _ = 0 := sum_zero xs
      simp [sum, tail]
    · have member : a ∈ xs := (List.mem_cons.mp member).resolve_left same
      simp only [sum, same, ite_false, Nat.zero_add]
      exact ih hn.2 member

def push {S T : Type} [DecidableEq T] (ws : List S) (f : S → T) (weight : S → Nat) (y : T) :=
  sum ws (fun x => if f x = y then weight x else 0)

/-- Weighted image preserves addition, while Event image uses existential OR. -/
theorem push_add {S T : Type} [DecidableEq T] (ws : List S) (f : S → T) (a b : S → Nat) (y : T) :
    push ws f (fun x => a x + b x) y = push ws f a y + push ws f b y := by
  unfold push
  rw [← sum_add]
  apply sum_congr
  intro x _
  by_cases h : f x = y <;> simp [h]

/-- Pulling a scalar back through f then multiplying commutes with its fibre sum. -/
theorem fibre_factor {S T : Type} [DecidableEq T] (ws : List S) (f : S → T)
    (a : T → Nat) (b : S → Nat) (y : T) :
    push ws f (fun x => a (f x) * b x) y = a y * push ws f b y := by
  unfold push
  rw [← sum_scale]
  apply sum_congr
  intro x _
  by_cases h : f x = y <;> simp [h]

theorem weighted_pairing {S T : Type} [DecidableEq T] (xs : List S) (ys : List T)
    (f : S → T) (w : S → Nat) (v : T → Nat)
    (unique : ys.Nodup) (legal : ∀ x, x ∈ xs → f x ∈ ys) :
    sum ys (fun y => v y * push xs f w y) = sum xs (fun x => v (f x) * w x) := by
  unfold push
  have scaled : ∀ y, v y * sum xs (fun x => if f x = y then w x else 0) =
      sum xs (fun x => if f x = y then v y * w x else 0) := by
    intro y
    rw [← sum_scale]
    apply sum_congr
    intro x _
    by_cases h : f x = y <;> simp [h]
  rw [sum_congr ys _ _ (fun y _ => scaled y), sum_swap]
  apply sum_congr
  intro x hx
  exact sum_pick ys (f x) (fun y => v y * w x) unique (legal x hx)

theorem total_mass_preserved {S T : Type} [DecidableEq T] (xs : List S) (ys : List T)
    (f : S → T) (w : S → Nat) (unique : ys.Nodup) (legal : ∀ x, x ∈ xs → f x ∈ ys) :
    sum ys (push xs f w) = sum xs w := by
  simpa only [Nat.one_mul] using weighted_pairing xs ys f w (fun _ => 1) unique legal

/-- Every old parent world retains its prior marginal under a normalized channel. -/
theorem channel_preserves_prior {S T : Type} [DecidableEq T] (xs : List S) (f : S → T)
    (prior : T → Nat) (kernel : S → Nat) (denominator : Nat)
    (normalized : ∀ y, push xs f kernel y = denominator) (y : T) :
    push xs f (fun x => prior (f x) * kernel x) y = prior y * denominator := by
  rw [fibre_factor, normalized y]

theorem closed_channel_normalized {S T : Type} [DecidableEq T] (xs : List S) (ys : List T)
    (f : S → T) (prior : T → Nat) (kernel : S → Nat) (pDen kDen : Nat)
    (unique : ys.Nodup) (legal : ∀ x, x ∈ xs → f x ∈ ys)
    (priorNormalized : sum ys prior = pDen)
    (kernelNormalized : ∀ y, push xs f kernel y = kDen) :
    sum xs (fun x => prior (f x) * kernel x) = pDen * kDen := by
  rw [← total_mass_preserved xs ys f _ unique legal]
  rw [sum_congr ys _ _ (fun y _ => channel_preserves_prior xs f prior kernel kDen kernelNormalized y)]
  have commute : sum ys (fun y => prior y * kDen) = sum ys (fun y => kDen * prior y) := by
    apply sum_congr; intro y _; exact Nat.mul_comm _ _
  rw [commute, sum_scale, priorNormalized, Nat.mul_comm]

/-- A bit may be summed as soon as the remaining readout no longer depends on
it. This is the local law required by the native suffix-dependency schedule. -/
theorem early_elimination {S T : Type} [DecidableEq T] (ws : List S) (f : S → T)
    (weight : Bool → S → Nat) (y : T) :
    push ws f (weight false) y + push ws f (weight true) y =
      push ws f (fun x => weight false x + weight true x) y := by
  exact (push_add ws f _ _ y).symm

theorem skipped_outcome_doubles (mass : Nat) : sum [false, true] (fun _ => mass) = 2 * mass := by
  simp only [sum]; omega

/-- Summation is not idempotent. The Boolean image implementation's repeated
existential elimination cannot be copied into weighted image unchanged. -/
theorem repeated_sum_is_wrong :
    sum [false, true] (fun _ => sum [false, true] (fun _ => 1)) ≠
      sum [false, true] (fun _ => 1) := by decide

theorem existential_image_loses_multiplicity :
    (∃ _ : Bool, True) ∧ sum [false, true] (fun _ => 1) = 2 := by
  exact ⟨⟨false, trivial⟩, rfl⟩

/-- A zero-prior row can hide a malformed channel if only the closed total is
checked. Both parent worlds are structurally admitted here. -/
theorem closed_normalization_is_too_weak :
    let prior : Bool → Nat := fun b => if b then 0 else 1
    let channel : Bool → Nat := fun b => if b then 0 else 1
    sum [false, true] (fun b => prior b * channel b) = 1 ∧ channel true ≠ 1 := by decide

/-- A copied outcome and two fresh conditionally independent outcomes have
incompatible joint laws even for the same exact fair source. -/
theorem copy_is_not_resample :
    let worlds := [(false, false), (false, true), (true, false), (true, true)]
    let copy : Bool × Bool → Nat := fun w => if w.1 == w.2 then 2 else 0
    let fresh : Bool × Bool → Nat := fun _ => 1
    sum worlds (fun w => if w.1 && w.2 then copy w else 0) ≠
      sum worlds (fun w => if w.1 && w.2 then fresh w else 0) := by decide

end WeightedMaps

#print axioms WeightedMaps.sum_congr
#print axioms WeightedMaps.sum_zero
#print axioms WeightedMaps.sum_add
#print axioms WeightedMaps.sum_scale
#print axioms WeightedMaps.sum_swap
#print axioms WeightedMaps.sum_pick
#print axioms WeightedMaps.push_add
#print axioms WeightedMaps.fibre_factor
#print axioms WeightedMaps.weighted_pairing
#print axioms WeightedMaps.total_mass_preserved
#print axioms WeightedMaps.channel_preserves_prior
#print axioms WeightedMaps.closed_channel_normalized
#print axioms WeightedMaps.early_elimination
#print axioms WeightedMaps.skipped_outcome_doubles
#print axioms WeightedMaps.repeated_sum_is_wrong
#print axioms WeightedMaps.existential_image_loses_multiplicity
#print axioms WeightedMaps.closed_normalization_is_too_weak
#print axioms WeightedMaps.copy_is_not_resample
