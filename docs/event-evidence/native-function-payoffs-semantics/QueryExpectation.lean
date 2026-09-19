import Std

/-!
Reference semantics of query payoff rosters: relational witnesses union by
exact value; unique coverage is structural and relative to evidence. Nothing
here proves the Rust collector, registry, scratch storage or exact contraction.
-/
namespace QueryExpectation

abbrev Region (W : Type) := W → Prop
abbrev Cells (W : Type) := Rat → Region W

def cells {B G W : Type} (rows : B → Prop) (group : B → G) (payoff : B → Rat)
    (when : B → Region W) (g : G) : Cells W :=
  fun v w => ∃ row, rows row ∧ group row = g ∧ payoff row = v ∧ when row w

def present {B G : Type} (rows : B → Prop) (group : B → G) (g : G) : Prop :=
  ∃ row, rows row ∧ group row = g

def valid {W : Type} (given : Region W) (roster : Cells W) : Prop :=
  ∀ w, given w → ∃ v, roster v w ∧ ∀ y, roster y w → y = v

def clip {W : Type} (given : Region W) (roster : Cells W) : Cells W :=
  fun v w => roster v w ∧ given w

theorem union_arms {B G W : Type} (a b : B → Prop) (g : B → G)
    (v : B → Rat) (r : B → Region W) (key : G) (value : Rat) (w : W) :
    cells (fun row => a row ∨ b row) g v r key value w ↔
      cells a g v r key value w ∨ cells b g v r key value w := by
  simp only [cells]
  constructor
  · rintro ⟨row, ha | hb, hg, hv, hr⟩
    · exact Or.inl ⟨row, ha, hg, hv, hr⟩
    · exact Or.inr ⟨row, hb, hg, hv, hr⟩
  · rintro (⟨row, ha, hg, hv, hr⟩ | ⟨row, hb, hg, hv, hr⟩)
    · exact ⟨row, Or.inl ha, hg, hv, hr⟩
    · exact ⟨row, Or.inr hb, hg, hv, hr⟩

theorem duplicate_witnesses_idempotent {B G W : Type} (rows : B → Prop)
    (g : B → G) (v : B → Rat) (r : B → Region W) (key : G) :
    cells (fun row => rows row ∨ rows row) g v r key = cells rows g v r key := by
  funext value w
  simp

theorem enumeration_irrelevant {B G W : Type} (a b : B → Prop) (same : ∀ row, a row ↔ b row)
    (g : B → G) (v : B → Rat) (r : B → Region W) (key : G) :
    cells a g v r key = cells b g v r key := by
  funext value w
  simp only [cells, same]

theorem clip_preserves_admission {W : Type} (given : Region W) (roster : Cells W) :
    valid given (clip given roster) ↔ valid given roster := by
  simp only [valid, clip]
  constructor
  · intro h w hw
    obtain ⟨v, ⟨hv, _⟩, unique⟩ := h w hw
    exact ⟨v, hv, fun y hy => unique y ⟨hy, hw⟩⟩
  · intro h w hw
    obtain ⟨v, hv, unique⟩ := h w hw
    exact ⟨v, ⟨hv, hw⟩, fun y hy => unique y hy.1⟩

theorem outside_evidence_irrelevant {W : Type} (given : Region W) (a b : Cells W)
    (same : ∀ w, given w → ∀ v, a v w ↔ b v w) : valid given a ↔ valid given b := by
  simp only [valid]
  constructor
  · intro h w hw
    obtain ⟨v, hv, unique⟩ := h w hw
    exact ⟨v, (same w hw v).mp hv, fun y hy => unique y ((same w hw y).mpr hy)⟩
  · intro h w hw
    obtain ⟨v, hv, unique⟩ := h w hw
    exact ⟨v, (same w hw v).mpr hv, fun y hy => unique y ((same w hw y).mp hy)⟩

theorem gap_refuses {W : Type} (given : Region W) (roster : Cells W) (w : W)
    (inside : given w) (missing : ∀ v, ¬ roster v w) : ¬ valid given roster := by
  intro admitted
  obtain ⟨v, hv, _⟩ := admitted w inside
  exact missing v hv

theorem distinct_value_overlap_refuses {W : Type} (given : Region W) (roster : Cells W)
    (w : W) (a b : Rat) (inside : given w) (ha : roster a w) (hb : roster b w)
    (different : a ≠ b) : ¬ valid given roster := by
  intro admitted
  obtain ⟨v, _, unique⟩ := admitted w inside
  exact different ((unique a ha).trans (unique b hb).symm)

theorem equal_value_union_is_one_payoff {W : Type} (given a b : Region W) (value : Rat)
    (covered : ∀ w, given w → a w ∨ b w) :
    valid given (fun v w => v = value ∧ (a w ∨ b w)) := by
  intro w hw
  exact ⟨value, ⟨rfl, covered w hw⟩, fun _ h => h.1⟩

theorem empty_evidence_is_structurally_valid {W : Type} (roster : Cells W) :
    valid (fun _ => False) roster := by
  intro _ h
  exact False.elim h

theorem empty_input_has_no_group {B G : Type} (group : B → G) (g : G) :
    ¬ present (fun _ => False) group g := by
  simp [present]

theorem an_empty_region_still_establishes_group {B G W : Type} (rows : B → Prop)
    (group : B → G) (row : B) (joined : rows row) (when : B → Region W)
    (_empty : ∀ w, ¬ when row w) : present rows group (group row) :=
  ⟨row, joined, rfl⟩

/-- A stable token carried beside ordinary folds neither splits nor merges groups. -/
theorem stable_tokens_preserve_grouping {G T : Type} (token : G → T) (a b : G) :
    (a, token a) = (b, token b) ↔ a = b := by
  constructor
  · exact fun h => congrArg Prod.fst h
  · intro h; cases h; rfl

/-- Reference payoff functions are partial: admission never invents zero. -/
noncomputable def selected {W : Type} (given : Region W) (roster : Cells W)
    (admitted : valid given roster) (w : W) (hw : given w) : Rat :=
  Classical.choose (admitted w hw)

theorem selected_is_supplied {W : Type} (given : Region W) (roster : Cells W)
    (admitted : valid given roster) (w : W) (hw : given w) :
    roster (selected given roster admitted w hw) w :=
  (Classical.choose_spec (admitted w hw)).1

def conditional (n d : Rat) : Option Rat := if d = 0 then none else some (n / d)

theorem impossible_exactly_zero_evidence (n d : Rat) : conditional n d = none ↔ d = 0 := by
  simp [conditional]

-- Law puts mass only on false; omitting true remains a structural gap.
theorem zero_mass_gap_is_not_zero_payoff :
    let law : Bool → Rat := fun w => if w then 0 else 1
    let roster : Cells Bool := fun v w => v = 7 ∧ w = false
    law true = 0 ∧ ¬ valid (fun _ => True) roster := by
  constructor
  · rfl
  · apply gap_refuses (w := true) (inside := True.intro)
    intro v h
    cases h.2

def sharedDrawPayoff (p : Rat) : Option Rat := conditional (2*p + 6*p*p) p

theorem parameter_hole_survives : sharedDrawPayoff 0 = none := by decide +kernel
theorem parameter_expectation_is_signed_scalar : sharedDrawPayoff (1/3) = some 4 := by decide +kernel
theorem shared_parameter_not_prior_averaged : sharedDrawPayoff (1/3) ≠ sharedDrawPayoff 1 := by decide +kernel
theorem zero_payoff_impossible_is_not_zero : conditional 0 0 = none := by decide +kernel
theorem signed_value_can_be_negative : conditional (-3) (1/2) = some (-6) := by decide +kernel


/-- All participating expressions must be defined, even on empty regions. -/
def payoffDefined {B G : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → Option Rat) (key : G) : Prop :=
  ∀ row, rows row → group row=key → ∃ value, payoff row=some value

theorem undefined_participant_refuses {B G : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → Option Rat) (key : G) (row : B)
    (joined : rows row) (member : group row=key) (undefined : payoff row=none) :
    ¬payoffDefined rows group payoff key := by
  intro admitted
  obtain ⟨value, defined⟩ := admitted row joined member
  rw [undefined] at defined
  cases defined

theorem empty_region_does_not_hide_zero_divisor {B G W : Type} (rows : B → Prop)
    (group : B → G) (numerator denominator : B → Rat) (when : B → Region W)
    (key : G) (row : B) (joined : rows row) (member : group row=key)
    (_empty : ∀ w, ¬when row w) (zero : denominator row=0) :
    ¬payoffDefined rows group (fun r => conditional (numerator r) (denominator r)) key := by
  apply undefined_participant_refuses rows group _ key row joined member
  simp [conditional,zero]

theorem equal_presentations_preserve_cells {B G W : Type} (rows : B → Prop)
    (group : B → G) (a b : B → Rat) (when : B → Region W) (key : G)
    (same : ∀ row, rows row → group row=key → a row=b row) :
    cells rows group a when key=cells rows group b when key := by
  funext value w
  apply propext
  constructor
  · rintro ⟨row,hr,hg,hv,hw⟩
    exact ⟨row,hr,hg,(same row hr hg).symm.trans hv,hw⟩
  · rintro ⟨row,hr,hg,hv,hw⟩
    exact ⟨row,hr,hg,(same row hr hg).trans hv,hw⟩

theorem equivalent_fraction_presentations : (1/2 : Rat) = 2/4 := by decide +kernel
theorem negative_denominator_is_exact : conditional 2 (-3) = some (-2/3) := by decide +kernel
theorem zero_over_zero_is_not_zero_payoff : conditional 0 0 ≠ some 0 := by decide +kernel

/-- A local observable supplies a value at each actual world in its region. -/
def functionCells {B G W : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) : Cells W :=
  fun v w => ∃ row, rows row ∧ group row=key ∧ when row w ∧ payoff row w=v

def localCoverage {B G W : Type} (rows : B → Prop) (group : B → G)
    (when : B → Region W) (key : G) (given : Region W) : Prop :=
  ∀ w, given w → ∃ row, rows row ∧ group row=key ∧ when row w

def localAgreement {B G W : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) (given : Region W) : Prop :=
  ∀ a, rows a → group a=key → ∀ b, rows b → group b=key → ∀ w,
    given w → when a w → when b w → payoff a w=payoff b w

/-- Structural coverage and a pointwise FD are exactly payoff admission. -/
theorem function_admission_iff {B G W : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) (given : Region W) :
    valid given (functionCells rows group payoff when key) ↔
      localCoverage rows group when key given ∧ localAgreement rows group payoff when key given := by
  constructor
  · intro h
    constructor
    · intro w hw
      obtain ⟨v, ⟨row,hr,hg,hwv,_⟩, _⟩ := h w hw
      exact ⟨row,hr,hg,hwv⟩
    · intro a ha hga b hb hgb w hw hwa hwb
      obtain ⟨v,_,unique⟩ := h w hw
      exact (unique (payoff a w) ⟨a,ha,hga,hwa,rfl⟩).trans
        (unique (payoff b w) ⟨b,hb,hgb,hwb,rfl⟩).symm
  · rintro ⟨covered,agrees⟩ w hw
    obtain ⟨row,hr,hg,active⟩ := covered w hw
    refine ⟨payoff row w, ⟨row,hr,hg,active,rfl⟩, ?_⟩
    rintro y ⟨other,ho,hgo,ao,eq⟩
    exact eq.symm.trans (agrees other ho hgo row hr hg w hw ao active)

theorem scalar_is_constant_function_patch {B G W : Type} (rows : B → Prop)
    (group : B → G) (payoff : B → Rat) (when : B → Region W) (key : G) :
    functionCells rows group (fun row _ => payoff row) when key = cells rows group payoff when key := by
  funext v w
  apply propext
  constructor <;> rintro ⟨row,hr,hg,a,b⟩ <;> exact ⟨row,hr,hg,b,a⟩

theorem function_union_arms {B G W : Type} (a b : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) (v : Rat) (w : W) :
    functionCells (fun row => a row ∨ b row) group payoff when key v w ↔
      functionCells a group payoff when key v w ∨ functionCells b group payoff when key v w := by
  constructor
  · rintro ⟨row,ha | hb,hg,hr,hv⟩
    · exact Or.inl ⟨row,ha,hg,hr,hv⟩
    · exact Or.inr ⟨row,hb,hg,hr,hv⟩
  · rintro (⟨row,ha,hg,hr,hv⟩ | ⟨row,hb,hg,hr,hv⟩)
    · exact ⟨row,Or.inl ha,hg,hr,hv⟩
    · exact ⟨row,Or.inr hb,hg,hr,hv⟩

theorem function_duplicate_witnesses {B G W : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) :
    functionCells (fun row => rows row ∨ rows row) group payoff when key =
      functionCells rows group payoff when key := by
  funext v w
  simp

/-- Promotion or replacement only needs equality on supplied actual worlds. -/
theorem local_function_replacement {B G W : Type} (rows : B → Prop) (group : B → G)
    (a b : B → W → Rat) (when : B → Region W) (key : G) (given : Region W)
    (same : ∀ row, rows row → group row=key → ∀ w, given w → when row w → a row w=b row w) :
    valid given (functionCells rows group a when key) ↔ valid given (functionCells rows group b when key) := by
  apply outside_evidence_irrelevant
  intro w hw v
  constructor
  · rintro ⟨row,hr,hg,active,hv⟩
    exact ⟨row,hr,hg,active,(same row hr hg w hw active).symm.trans hv⟩
  · rintro ⟨row,hr,hg,active,hv⟩
    exact ⟨row,hr,hg,active,(same row hr hg w hw active).trans hv⟩

theorem selected_function_matches_each_patch {B G W : Type} (rows : B → Prop) (group : B → G)
    (payoff : B → W → Rat) (when : B → Region W) (key : G) (given : Region W)
    (h : valid given (functionCells rows group payoff when key))
    (row : B) (hr : rows row) (hg : group row=key) (w : W) (hw : given w) (active : when row w) :
    selected given (functionCells rows group payoff when key) h w hw = payoff row w := by
  obtain ⟨v,_,unique⟩ := h w hw
  exact (unique _ (selected_is_supplied given _ h w hw)).trans
    (unique _ ⟨row,hr,hg,active,rfl⟩).symm

#print axioms QueryExpectation.function_admission_iff
#print axioms QueryExpectation.scalar_is_constant_function_patch
#print axioms QueryExpectation.function_union_arms
#print axioms QueryExpectation.function_duplicate_witnesses
#print axioms QueryExpectation.local_function_replacement
#print axioms QueryExpectation.selected_function_matches_each_patch

#print axioms QueryExpectation.union_arms
#print axioms QueryExpectation.duplicate_witnesses_idempotent
#print axioms QueryExpectation.enumeration_irrelevant
#print axioms QueryExpectation.clip_preserves_admission
#print axioms QueryExpectation.outside_evidence_irrelevant
#print axioms QueryExpectation.gap_refuses
#print axioms QueryExpectation.distinct_value_overlap_refuses
#print axioms QueryExpectation.equal_value_union_is_one_payoff
#print axioms QueryExpectation.empty_evidence_is_structurally_valid
#print axioms QueryExpectation.empty_input_has_no_group
#print axioms QueryExpectation.an_empty_region_still_establishes_group
#print axioms QueryExpectation.stable_tokens_preserve_grouping
#print axioms QueryExpectation.selected_is_supplied
#print axioms QueryExpectation.impossible_exactly_zero_evidence
#print axioms QueryExpectation.zero_mass_gap_is_not_zero_payoff
#print axioms QueryExpectation.parameter_hole_survives
#print axioms QueryExpectation.parameter_expectation_is_signed_scalar
#print axioms QueryExpectation.shared_parameter_not_prior_averaged
#print axioms QueryExpectation.zero_payoff_impossible_is_not_zero
#print axioms QueryExpectation.signed_value_can_be_negative

#print axioms QueryExpectation.undefined_participant_refuses
#print axioms QueryExpectation.empty_region_does_not_hide_zero_divisor
#print axioms QueryExpectation.equal_presentations_preserve_cells
#print axioms QueryExpectation.equivalent_fraction_presentations
#print axioms QueryExpectation.negative_denominator_is_exact
#print axioms QueryExpectation.zero_over_zero_is_not_zero_payoff

end QueryExpectation
