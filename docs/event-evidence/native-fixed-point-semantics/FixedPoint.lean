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
  apply Classical.byContradiction
  intro unstopped
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
  · have pointwise := h w
    cases ha : a w <;> cases hb : b w <;> simp_all
  · have pointwise := h w
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

/-- One state per environment is not enough if iteration mixes environments.
    This monotone shift grows one new natural-number environment on every step. -/
def shift (a : Region Nat) : Region Nat
  | 0 => true
  | n + 1 => a n

private theorem shift_iterate (n w : Nat) : iterate shift n w = decide (w < n) := by
  induction n generalizing w with
  | zero => simp [iterate, empty]
  | succ n ih =>
    cases w with
    | zero => simp [iterate, shift]
    | succ w => simpa [iterate, shift] using ih w

theorem environment_mixing_counterexample :
    monotone shift ∧ ∀ n, iterate shift n ≠ iterate shift (n + 1) := by
  constructor
  · intro a b inc w present
    cases w with
    | zero => rfl
    | succ w => exact inc w present
  · intro n equal
    have at_boundary := congrFun equal n
    simp only [shift_iterate] at at_boundary
    simp at at_boundary

theorem negation_is_not_a_fixed_point_program :
    ¬ monotone (complement (W := Unit)) := by
  intro mono
  have inc : included (empty (W := Unit)) (fun _ => true) := by
    intro w impossible
    cases impossible
  have impossible := mono _ _ inc () rfl
  cases impossible

/-! Native finite-carrier and early-stopping reference. The Rust loop uses
canonical equality after alignment and an exact legal-support count. These
premises are explicit here. Cancellation/kernel/resource refusal is represented
by no mathematical result; a successful stop is never a truncated approximant.
The roster below is a proof object, not a required runtime enumeration. -/

def legalRoster (n : Nat) (support : Fin n → Bool) : List { w : Fin n // support w = true } :=
  (List.finRange n).filterMap (fun w => if h : support w = true then some ⟨w, h⟩ else none)

theorem legal_roster_complete (n : Nat) (support : Fin n → Bool)
    (w : { x : Fin n // support x = true }) : w ∈ legalRoster n support := by
  apply List.mem_filterMap.mpr
  exact ⟨w.val, List.mem_finRange w.val, by simp [w.property]⟩

theorem legal_roster_counts_original_support (n : Nat) (support : Fin n → Bool) :
    (legalRoster n support).length = (List.finRange n).countP support := by
  simp only [legalRoster, List.length_filterMap_eq_countP]
  apply List.countP_congr
  intro w _
  by_cases h : support w = true <;> simp [h]

theorem legal_roster_bounded_by_codes (n : Nat) (support : Fin n → Bool) :
    (legalRoster n support).length ≤ n := by
  rw [legal_roster_counts_original_support]
  simpa using List.countP_le_length (p := support) (l := List.finRange n)

def advance {W : Type} (F : Region W → Region W) (start : Region W) : Nat → Region W
  | 0 => start
  | n + 1 => F (advance F start n)

private theorem advance_next {W : Type} (F : Region W → Region W) (start : Region W) (n : Nat) :
    advance F (F start) n = advance F start (n + 1) := by
  induction n with
  | zero => rfl
  | succ n ih => exact congrArg F ih

private theorem advance_empty {W : Type} (F : Region W → Region W) (n : Nat) :
    advance F empty n = iterate F n := by
  induction n with
  | zero => rfl
  | succ n ih => exact congrArg F ih

private theorem advance_top_dual {W : Type} (F : Region W → Region W) (n : Nat) :
    advance F (fun _ => true) n = complement (iterate (dual F) n) := by
  induction n with
  | zero => rfl
  | succ n ih =>
    rw [advance, ih]
    change _ = complement (complement (F (complement (iterate (dual F) n))))
    rw [complement_involutive]

def detect {W : Type} (F : Region W → Region W) (same : Region W → Region W → Bool)
    (start : Region W) : Nat → Option (Nat × Region W)
  | 0 => none
  | fuel + 1 =>
    let next := F start
    if same start next then some (1, next)
    else (detect F same next fuel).map (fun (steps, value) => (steps + 1, value))

/-- Every successful stop identifies an actual iterate and an actual fixed
    point, with the detection application included in its bounded step count. -/
theorem detection_sound {W : Type} (F : Region W → Region W)
    (same : Region W → Region W → Bool) (exact : ∀ a b, same a b = true ↔ a = b)
    (fuel : Nat) (start result : Region W) (steps : Nat)
    (found : detect F same start fuel = some (steps, result)) :
    0 < steps ∧ steps ≤ fuel ∧ result = advance F start steps ∧ F result = result := by
  induction fuel generalizing start result steps with
  | zero => simp [detect] at found
  | succ fuel ih =>
    simp only [detect] at found
    split at found
    · rename_i equal
      have pair := Option.some.inj found
      have hs := congrArg Prod.fst pair
      have hr := congrArg Prod.snd pair
      dsimp at hs hr
      subst steps result
      exact ⟨by omega, by omega, rfl, (congrArg F ((exact _ _).mp equal)).symm⟩
    · cases previous : detect F same (F start) fuel with
      | none => simp [previous] at found
      | some answer =>
        obtain ⟨k, value⟩ := answer
        simp only [previous, Option.map_some, Option.some.injEq, Prod.mk.injEq] at found
        obtain ⟨rfl, rfl⟩ := found
        obtain ⟨positive, bounded, refined, fixed⟩ := ih _ _ _ previous
        exact ⟨by omega, by omega, refined.trans (advance_next F start k), fixed⟩

/-- A fixed iterate at index n is detected within n+1 applications. -/
theorem detection_complete {W : Type} (F : Region W → Region W)
    (same : Region W → Region W → Bool) (exact : ∀ a b, same a b = true ↔ a = b)
    (n : Nat) (start : Region W) (fixed : F (advance F start n) = advance F start n) :
    ∃ steps result, detect F same start (n + 1) = some (steps, result) := by
  induction n generalizing start with
  | zero =>
    have equal : same start (F start) = true := (exact _ _).mpr fixed.symm
    exact ⟨1, F start, by simp [detect, equal]⟩
  | succ n ih =>
    by_cases equal : same start (F start) = true
    · exact ⟨1, F start, by simp [detect, equal]⟩
    · have shifted : F (advance F (F start) n) = advance F (F start) n := by
        simpa only [advance_next] using fixed
      obtain ⟨steps, result, found⟩ := ih _ shifted
      refine ⟨steps + 1, result, ?_⟩
      rw [detect]
      simp only [if_neg equal, found, Option.map_some]

theorem detected_least_is_extremal {W : Type} (F : Region W → Region W)
    (mono : monotone F) (same : Region W → Region W → Bool)
    (exact : ∀ a b, same a b = true ↔ a = b) (fuel steps : Nat) (result : Region W)
    (found : detect F same empty fuel = some (steps, result)) :
    F result = result ∧ ∀ p, included (F p) p → included result p := by
  obtain ⟨_, _, refined, fixed⟩ := detection_sound F same exact fuel empty result steps found
  refine ⟨fixed, ?_⟩
  intro p closed
  rw [refined, advance_empty]
  exact below_prefixed F mono p closed steps

theorem detected_greatest_is_extremal {W : Type} (F : Region W → Region W)
    (mono : monotone F) (same : Region W → Region W → Bool)
    (exact : ∀ a b, same a b = true ↔ a = b) (fuel steps : Nat) (result : Region W)
    (found : detect F same (fun _ => true) fuel = some (steps, result)) :
    F result = result ∧ ∀ p, included p (F p) → included p result := by
  obtain ⟨_, _, refined, fixed⟩ := detection_sound F same exact fuel _ result steps found
  refine ⟨fixed, ?_⟩
  intro p closed
  have pre : included (dual F (complement p)) (complement p) := by
    unfold dual
    rw [complement_involutive]
    exact (complement_reverses _ _).mpr closed
  have bound := below_prefixed (dual F) (dual_monotone F mono) (complement p) pre steps
  have reversed := (complement_reverses (complement p) (iterate (dual F) steps)).mpr bound
  simpa only [refined, advance_top_dual, complement_involutive] using reversed

theorem finite_detection_completes {W : Type} (worlds : List W)
    (complete : ∀ w, w ∈ worlds) (F : Region W → Region W) (mono : monotone F)
    (same : Region W → Region W → Bool) (exact : ∀ a b, same a b = true ↔ a = b) :
    (∃ steps result, detect F same empty (worlds.length + 1) = some (steps, result)) ∧
    (∃ steps result, detect F same (fun _ => true) (worlds.length + 1) = some (steps, result)) := by
  constructor
  · apply detection_complete F same exact worlds.length
    rw [advance_empty]
    exact (stabilizes worlds complete F mono).symm
  · apply detection_complete F same exact worlds.length
    rw [advance_top_dual]
    exact (greatest_fixed_point worlds complete F mono).1

theorem greatest_iteration_descends {W : Type} (F : Region W → Region W)
    (mono : monotone F) (n : Nat) :
    included (advance F (fun _ => true) (n + 1)) (advance F (fun _ => true) n) := by
  rw [advance_top_dual, advance_top_dual]
  exact (complement_reverses _ _).mpr (ascending (dual F) (dual_monotone F mono) n)

end FixedPoint

#print axioms FixedPoint.strict_rank
#print axioms FixedPoint.ascending
#print axioms FixedPoint.below_prefixed
#print axioms FixedPoint.stabilizes
#print axioms FixedPoint.least_fixed_point
#print axioms FixedPoint.dual_monotone
#print axioms FixedPoint.greatest_fixed_point
#print axioms FixedPoint.shared_environment_stabilizes
#print axioms FixedPoint.environment_mixing_counterexample
#print axioms FixedPoint.negation_is_not_a_fixed_point_program
#print axioms FixedPoint.legal_roster_complete
#print axioms FixedPoint.legal_roster_counts_original_support
#print axioms FixedPoint.legal_roster_bounded_by_codes
#print axioms FixedPoint.detection_sound
#print axioms FixedPoint.detection_complete
#print axioms FixedPoint.detected_least_is_extremal
#print axioms FixedPoint.detected_greatest_is_extremal
#print axioms FixedPoint.finite_detection_completes
#print axioms FixedPoint.greatest_iteration_descends
