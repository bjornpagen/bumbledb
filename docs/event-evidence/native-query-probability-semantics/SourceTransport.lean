import Std

/-!
Fixed-law source transport obligations. Decoded regions, scalar values and named
contexts must have their original meaning on the same legal worlds. The first
laws show the resulting function/fibre/likelihood contractions are unchanged.
Revision import recomputes the entire receipt and result; the replay laws then
separate false claims from operational failure and mathematical impossibility.

These are reference denotations, not a proof of BESC/BEVT/BERA parsing, Rust
arithmetic, graph/arena reconstruction, cancellation or retained memory. Kernel
normalization and revision validity are established by the replayed constructors,
not by a wire tag or by equality of posteriors alone.
-/
namespace SourceTransport

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

def scalar {C W : Type} (cs : List C) (region : C → W → Bool)
    (value : C → Rat) (w : W) := sum cs (fun c => if region c w then value c else 0)

theorem scalar_reconstruction {C W : Type} (cs : List C)
    (region decoded : C → W → Bool) (value decodedValue : C → Rat) (w : W)
    (sameRegion : ∀ c, c ∈ cs → region c w = decoded c w)
    (sameValue : ∀ c, c ∈ cs → value c = decodedValue c) :
    scalar cs region value w = scalar cs decoded decodedValue w := by
  apply sum_congr
  intro c hc
  rw [sameRegion c hc, sameValue c hc]

theorem zero_default_reconstructed {C W : Type} (cs : List C)
    (region : C → W → Bool) (value : C → Rat) (w : W)
    (absent : ∀ c, c ∈ cs → region c w = false) : scalar cs region value w = 0 := by
  unfold scalar
  induction cs with
  | nil => rfl
  | cons c cs ih =>
    simp only [sum, absent c (by simp), Bool.false_eq_true, ite_false, Rat.zero_add]
    exact ih (fun d hd => absent d (by simp [hd]))

def fibre {X Y : Type} [DecidableEq Y] (xs : List X) (parent : X → Y)
    (density : X → Rat) (y : Y) := sum xs (fun x => if parent x = y then density x else 0)

theorem fibre_reconstruction {X Y : Type} [DecidableEq Y] (xs : List X)
    (parent decodedParent : X → Y) (density decodedDensity : X → Rat) (y : Y)
    (sameParent : ∀ x, x ∈ xs → parent x = decodedParent x)
    (sameDensity : ∀ x, x ∈ xs → density x = decodedDensity x) :
    fibre xs parent density y = fibre xs decodedParent decodedDensity y := by
  apply sum_congr
  intro x hx
  rw [sameParent x hx, sameDensity x hx]

theorem kernel_normalization_reconstructed {X Y : Type} [DecidableEq Y]
    (xs : List X) (parent decodedParent : X → Y) (density decodedDensity : X → Rat)
    (sameParent : ∀ x, x ∈ xs → parent x = decodedParent x)
    (sameDensity : ∀ x, x ∈ xs → density x = decodedDensity x)
    (normalized : ∀ y, fibre xs parent density y = 1) :
    ∀ y, fibre xs decodedParent decodedDensity y = 1 := by
  intro y
  rw [← fibre_reconstruction xs parent decodedParent density decodedDensity y sameParent sameDensity]
  exact normalized y

/-- Both the normalizer and an optional posterior are retained. Zero evidence
is a successful, owned impossible result, represented by the inner `none`. -/
def likelihood {W : Type} (ws : List W) (prior factor : W → Rat) :
    Rat × Option (List Rat) :=
  let z := sum ws (fun w => prior w * factor w)
  (z, if z = 0 then none else some (ws.map fun w => prior w * factor w / z))

theorem likelihood_reconstruction {W : Type} (ws : List W)
    (prior decodedPrior factor decodedFactor : W → Rat)
    (samePrior : ∀ w, w ∈ ws → prior w = decodedPrior w)
    (sameFactor : ∀ w, w ∈ ws → factor w = decodedFactor w) :
    likelihood ws prior factor = likelihood ws decodedPrior decodedFactor := by
  have products : ∀ w, w ∈ ws → prior w * factor w = decodedPrior w * decodedFactor w := by
    intro w hw
    rw [samePrior w hw, sameFactor w hw]
  have totals := sum_congr ws _ _ products
  have maps : ∀ z : Rat,
      ws.map (fun w => prior w * factor w / z) =
      ws.map (fun w => decodedPrior w * decodedFactor w / z) := by
    intro z
    apply List.map_congr_left
    intro w hw
    rw [products w hw]
  unfold likelihood
  simp only [totals, maps]

/-- Positions, including empty cells, are retained. Roster equality is stronger
than equality of a set of target probabilities or of unsupported cell labels. -/
def unsupported : List (Nat × Rat × Rat) → List Nat
  | [] => []
  | (i, old, target) :: cs =>
      if old = 0 ∧ target ≠ 0 then i :: unsupported cs else unsupported cs

theorem unsupported_cons_retains_position (i : Nat) (old target : Rat)
    (cs : List (Nat × Rat × Rat)) (zero : old = 0) (positive : target ≠ 0) :
    unsupported ((i, old, target) :: cs) = i :: unsupported cs := by
  simp [unsupported, zero, positive]

def admit {I E O : Type} [DecidableEq O] (replay : I → Except E O)
    (input : I) (claimed : O) : Except (Option E) O :=
  match replay input with
  | .error e => .error (some e)
  | .ok actual => if actual = claimed then .ok actual else .error none

theorem admission_exact {I E O : Type} [DecidableEq O] (replay : I → Except E O)
    (input : I) (claimed : O) :
    admit replay input claimed = .ok claimed ↔ replay input = .ok claimed := by
  cases h : replay input with
  | error e => simp [admit, h]
  | ok actual =>
    by_cases same : actual = claimed <;> simp [admit, h, same]

theorem replay_failure_preserved {I E O : Type} [DecidableEq O]
    (replay : I → Except E O) (input : I) (claimed : O) (e : E)
    (failed : replay input = .error e) : admit replay input claimed = .error (some e) := by
  simp [admit, failed]

theorem forged_claim_rejected {I E O : Type} [DecidableEq O]
    (replay : I → Except E O) (input : I) (claimed actual : O)
    (replayed : replay input = .ok actual) (different : actual ≠ claimed) :
    admit replay input claimed = .error none := by
  simp [admit, replayed, different]

theorem zero_mass_channel_rows_cannot_be_skipped :
    sum [false, true] (fun b => if b then 0 else 1) = 1 ∧
    fibre [false, true] id (fun b => if b then 0 else 1) true ≠ 1 := by decide +kernel

theorem same_posterior_does_not_validate_a_scaled_receipt :
    (likelihood [false, true] (fun _ => 1/2) (fun _ => 1)).2 =
      (likelihood [false, true] (fun _ => 1/2) (fun _ => 2)).2 ∧
    (likelihood [false, true] (fun _ => 1/2) (fun _ => 1)).1 ≠
      (likelihood [false, true] (fun _ => 1/2) (fun _ => 2)).1 := by decide +kernel

theorem omitted_empty_target_changes_receipt :
    unsupported [(0, 0, 1/2), (1, 1, 0), (2, 0, 1/2)] = [0, 2] ∧
    unsupported [(0, 0, 1/2), (1, 1, 0)] = [0] := by decide +kernel

theorem replay_error_is_not_mathematical_impossibility :
    (Except.error (some ()) : Except (Option Unit) (Option Rat)) ≠ .ok none := by intro h; cases h

#print axioms SourceTransport.sum_congr
#print axioms SourceTransport.scalar_reconstruction
#print axioms SourceTransport.zero_default_reconstructed
#print axioms SourceTransport.fibre_reconstruction
#print axioms SourceTransport.kernel_normalization_reconstructed
#print axioms SourceTransport.likelihood_reconstruction
#print axioms SourceTransport.unsupported_cons_retains_position
#print axioms SourceTransport.admission_exact
#print axioms SourceTransport.replay_failure_preserved
#print axioms SourceTransport.forged_claim_rejected
#print axioms SourceTransport.zero_mass_channel_rows_cannot_be_skipped
#print axioms SourceTransport.same_posterior_does_not_validate_a_scaled_receipt
#print axioms SourceTransport.omitted_empty_target_changes_receipt
#print axioms SourceTransport.replay_error_is_not_mathematical_impossibility

end SourceTransport
