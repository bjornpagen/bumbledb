import Std

/-!
Family-valued outcome functions and likelihood updates. A parameter is held fixed
in every finite sum; coefficients may depend on it. Finite kernel multiplicities
can therefore act on exact rational-function coefficients without introducing a
prior or summing parameter guards. Native correspondence must establish whole-cell
map admission, faithful outcome rosters, defined coefficients on active cells and
the total zero-default representation. This is not verification of the Rust graph,
solver, arithmetic, resources, transport or database compiler.
-/
namespace FamilyFunctions

section Algebra
variable {S T Θ K : Type} [Lean.Grind.Field K]

def sum : List S → (S → K) → K
  | [], _ => 0
  | x::xs, f => f x+sum xs f

theorem sum_congr (xs : List S) (f g : S → K) (h : ∀ x, f x=g x) : sum xs f=sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum,h,ih]
theorem sum_congr_on (xs : List S) (f g : S → K)
    (same : ∀ x, x ∈ xs → f x=g x) : sum xs f=sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [sum]
    rw [same x (by simp), ih (fun y hy => same y (by simp [hy]))]
theorem sum_zero (xs : List S) : sum xs (fun _ => (0 : K))=0 := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum,ih]; grind
theorem sum_add (xs : List S) (f g : S → K) :
    sum xs (fun x => f x+g x)=sum xs f+sum xs g := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind
theorem sum_scale (xs : List S) (f : S → K) (k : K) :
    sum xs (fun x => k*f x)=k*sum xs f := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind
theorem sum_div (xs : List S) (f : S → K) (k : K) :
    sum xs (fun x => f x/k)=sum xs f/k := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind
theorem sum_swap (xs : List S) (ys : List T) (f : S → T → K) :
    sum xs (fun x => sum ys (f x))=sum ys (fun y => sum xs (fun x => f x y)) := by
  induction xs with
  | nil => exact (sum_zero ys).symm
  | cons x xs ih => simp only [sum,sum_add,ih]

theorem sum_pick [DecidableEq T] (ys : List T) (a : T) (v : K)
    (unique : ys.Nodup) (member : a ∈ ys) :
    sum ys (fun y => if a=y then v else 0)=v := by
  induction ys with
  | nil => simp at member
  | cons y ys ih =>
    have hn := List.nodup_cons.mp unique
    by_cases same : a=y
    · subst a
      have tail : sum ys (fun z => if y=z then v else 0)=0 := by
        calc
          _ = sum ys (fun _ => 0) := by
            apply sum_congr_on
            intro z member
            have different : y≠z := by intro h; exact hn.1 (h ▸ member)
            simp [different]
          _ = 0 := sum_zero ys
      simp only [sum,ite_true,tail]; grind
    · have member : a ∈ ys := (List.mem_cons.mp member).resolve_left same
      simp only [sum,same,ite_false,ih hn.2 member]; grind

def image [DecidableEq T] (xs : List S) (f : S → T) (value : S → K) (y : T) : K :=
  sum xs (fun x => if f x=y then value x else 0)

theorem image_parameter_basis [DecidableEq T] (xs : List S) (f : S → T)
    (inside : S → Bool) (coefficient : Θ → K) (theta : Θ) (y : T) :
    image xs f (fun x => if inside x then coefficient theta else 0) y =
      coefficient theta * image xs f (fun x => if inside x then 1 else 0) y := by
  unfold image
  rw [←sum_scale]
  apply sum_congr
  intro x
  by_cases h : f x=y <;> cases hi : inside x <;> simp [h] <;> grind

theorem image_add [DecidableEq T] (xs : List S) (f : S → T) (a b : S → K) (y : T) :
    image xs f (fun x => a x+b x) y=image xs f a y+image xs f b y := by
  unfold image
  rw [←sum_add]
  apply sum_congr
  intro x
  by_cases h : f x=y <;> simp [h] <;> grind

theorem weighted_pairing [DecidableEq T] (xs : List S) (ys : List T) (f : S → T)
    (w : S → K) (v : T → K) (unique : ys.Nodup) (covered : ∀ x, f x ∈ ys) :
    sum ys (fun y => v y*image xs f w y)=sum xs (fun x => v (f x)*w x) := by
  calc
    _ = sum ys (fun y => sum xs (fun x => if f x=y then v y*w x else 0)) := by
      apply sum_congr
      intro y
      unfold image
      rw [←sum_scale]
      apply sum_congr
      intro x
      by_cases h : f x=y <;> simp [h] <;> grind
    _ = sum xs (fun x => sum ys (fun y => if f x=y then v y*w x else 0)) := sum_swap ys xs _
    _ = _ := by
      apply sum_congr
      intro x
      calc
        _ = sum ys (fun y => if f x=y then v (f x)*w x else 0) := by
          apply sum_congr
          intro y
          by_cases h : f x=y <;> simp [h]
        _ = _ := sum_pick ys (f x) _ unique (covered x)

theorem image_total [DecidableEq T] (xs : List S) (ys : List T) (f : S → T)
    (w : S → K) (unique : ys.Nodup) (covered : ∀ x, f x ∈ ys) :
    sum ys (image xs f w)=sum xs w := by
  calc
    _ = sum ys (fun y => 1*image xs f w y) := by apply sum_congr; intro y; grind
    _ = sum xs (fun x => 1*w x) := weighted_pairing xs ys f w (fun _ => 1) unique covered
    _ = _ := by apply sum_congr; intro x; grind

theorem likelihood_normalized (xs : List S) (density factor : S → K)
    (nonzero : sum xs (fun x => density x*factor x)≠0) :
    sum xs (fun x => density x*factor x/sum xs (fun z => density z*factor z))=1 := by
  rw [sum_div]
  grind

theorem likelihood_scale (xs : List S) (density factor : S → K) (k : K) (x : S)
    (scale : k≠0) (nonzero : sum xs (fun z => density z*factor z)≠0) :
    density x*(k*factor x)/sum xs (fun z => density z*(k*factor z)) =
      density x*factor x/sum xs (fun z => density z*factor z) := by
  have norm : sum xs (fun z => density z*(k*factor z))=k*sum xs (fun z => density z*factor z) := by
    calc
      _ = sum xs (fun z => k*(density z*factor z)) := by apply sum_congr; intro z; grind
      _ = _ := sum_scale xs _ k
  rw [norm]
  grind

theorem missing_parameter_image_is_zero [DecidableEq T] (xs : List S) (f : S → T)
    (value : Θ → S → K) (domain : Θ → Bool) (theta : Θ) (y : T)
    (outside : domain theta=false) :
    (if domain theta then image xs f (value theta) y else 0)=0 := by simp [outside]

theorem zero_default_does_not_cover_an_active_hole (active : Bool) (value : Option K)
    (present : active=true) (hole : value=none) :
    (if active then value else some 0)=none := by simp [present,hole]

/- The parameter is fixed throughout each theorem. No measure on it is used.
The finite roster must correspond to actual outcomes of that parameter fibre. -/
theorem image_pullback_factor [DecidableEq T] (xs : List S) (f : S → T)
    (a : S → K) (b : T → K) (y : T) :
    image xs f (fun x => a x*b (f x)) y=b y*image xs f a y := by
  unfold image
  rw [←sum_scale]
  apply sum_congr
  intro x
  by_cases h : f x=y <;> simp [h] <;> grind

theorem channel_preserves_marginal [DecidableEq T] (xs : List S) (f : S → T)
    (channel : S → K) (prior : T → K) (rows : ∀ y, image xs f channel y=1) (y : T) :
    image xs f (fun x => channel x*prior (f x)) y=prior y := by
  rw [image_pullback_factor,rows]; grind

theorem channel_closure_normalized [DecidableEq T] (xs : List S) (ys : List T)
    (f : S → T) (channel : S → K) (prior : T → K)
    (unique : ys.Nodup) (covered : ∀ x, f x ∈ ys)
    (rows : ∀ y, image xs f channel y=1) (unit : sum ys prior=1) :
    sum xs (fun x => channel x*prior (f x))=1 := by
  rw [←image_total xs ys f _ unique covered]
  calc
    _ = sum ys prior := sum_congr ys _ _ (channel_preserves_marginal xs f channel prior rows)
    _ = 1 := unit

theorem fibre_mean_recovers_factor [DecidableEq T] (xs : List S) (f : S → T)
    (value : T → K) (y : T) (positive_count : image xs f (fun _ => (1 : K)) y≠0) :
    image xs f (fun x => value (f x)) y/image xs f (fun _ => (1 : K)) y=value y := by
  have factored := image_pullback_factor xs f (fun _ => (1 : K)) value y
  simp only [Lean.Grind.Semiring.one_mul] at factored
  rw [factored]
  grind

omit [Lean.Grind.Field K] in
theorem exact_descent_is_unique (f : S → T) (onto : ∀ y, ∃ x, f x=y)
    (a b : T → K) (same : ∀ x, a (f x)=b (f x)) : ∀ y, a y=b y := by
  intro y
  obtain ⟨x,h⟩ := onto y
  simpa [h] using same x

def replacementFactor [DecidableEq K] (target old : K) : K :=
  if target=0 then 0 else target/old

theorem jeffrey_cell_target [DecidableEq T] [DecidableEq K]
    (xs : List S) (f : S → T) (density : S → K) (target : T → K)
    (supported : ∀ y, target y≠0 → image xs f density y≠0) (y : T) :
    image xs f (fun x => density x*replacementFactor (target (f x)) (image xs f density (f x))) y=target y := by
  rw [image_pullback_factor xs f density (fun y => replacementFactor (target y) (image xs f density y))]
  unfold replacementFactor
  by_cases h : target y=0
  · simp only [h,ite_true]; grind
  · simp only [h,ite_false]
    have hm := supported y h
    grind

theorem jeffrey_normalized [DecidableEq T] [DecidableEq K]
    (xs : List S) (ys : List T) (f : S → T) (density : S → K) (target : T → K)
    (unique : ys.Nodup) (covered : ∀ x, f x ∈ ys)
    (supported : ∀ y, target y≠0 → image xs f density y≠0) (unit : sum ys target=1) :
    sum xs (fun x => density x*replacementFactor (target (f x)) (image xs f density (f x)))=1 := by
  rw [←image_total xs ys f _ unique covered]
  calc
    _ = sum ys target := sum_congr ys _ _ (jeffrey_cell_target xs f density target supported)
    _ = 1 := unit

theorem jeffrey_zero_target_skips_conditional [DecidableEq K] (density old : K) :
    density*replacementFactor (0 : K) old=0 := by
  simp only [replacementFactor,ite_true]; grind

theorem jeffrey_pointwise_idempotent [DecidableEq K] (density target old : K) :
    (density*replacementFactor target old)*replacementFactor target target =
      density*replacementFactor target old := by
  unfold replacementFactor
  by_cases h : target=0 <;> simp only [h,ite_true,ite_false] <;> grind

theorem unsupported_positive_target_cannot_be_normalized [DecidableEq K]
    (target : K) (nonzero : target≠0) :
    replacementFactor target (0 : K)*0≠target := by grind

end Algebra

theorem negative_zero_prior_factor_still_invalid :
    let density : Bool → Rat := fun b => if b then 0 else 1
    let factor : Bool → Rat := fun b => if b then -1 else 1
    sum [false,true] (fun b => density b*factor b)=1 ∧ ¬(∀ b, 0≤factor b) := by
  decide +kernel

end FamilyFunctions

#print axioms FamilyFunctions.sum_congr
#print axioms FamilyFunctions.sum_congr_on
#print axioms FamilyFunctions.sum_zero
#print axioms FamilyFunctions.sum_add
#print axioms FamilyFunctions.sum_scale
#print axioms FamilyFunctions.sum_div
#print axioms FamilyFunctions.sum_swap
#print axioms FamilyFunctions.sum_pick
#print axioms FamilyFunctions.image_parameter_basis
#print axioms FamilyFunctions.image_add
#print axioms FamilyFunctions.weighted_pairing
#print axioms FamilyFunctions.image_total
#print axioms FamilyFunctions.likelihood_normalized
#print axioms FamilyFunctions.likelihood_scale
#print axioms FamilyFunctions.missing_parameter_image_is_zero
#print axioms FamilyFunctions.zero_default_does_not_cover_an_active_hole
#print axioms FamilyFunctions.negative_zero_prior_factor_still_invalid
#print axioms FamilyFunctions.image_pullback_factor
#print axioms FamilyFunctions.channel_preserves_marginal
#print axioms FamilyFunctions.channel_closure_normalized
#print axioms FamilyFunctions.fibre_mean_recovers_factor
#print axioms FamilyFunctions.exact_descent_is_unique
#print axioms FamilyFunctions.jeffrey_cell_target
#print axioms FamilyFunctions.jeffrey_normalized
#print axioms FamilyFunctions.jeffrey_zero_target_skips_conditional
#print axioms FamilyFunctions.jeffrey_pointwise_idempotent
#print axioms FamilyFunctions.unsupported_positive_target_cannot_be_normalized
