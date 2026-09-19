import Std

/-!
Reference semantics for explicit numerical truth guards. The refinement adds
deterministic truth bits to each existing world. It does not identify worlds,
condition a law or turn undefined truth into false. This checks the semantic
construction, not the native guard solver, BENP parser or Free Join lowering.
-/
namespace PredicateGuards

def code (p : Option Bool) : Bool × Bool := (p == some true, p == some false)
def decode (c : Bool × Bool) : Option Bool :=
  if c.1 then some true else if c.2 then some false else none

theorem decode_code (p : Option Bool) : decode (code p) = p := by
  cases p with
  | none => rfl
  | some b => cases b <;> rfl

theorem code_injective (a b : Option Bool) : code a = code b ↔ a = b := by
  constructor
  · intro h
    simpa [decode_code] using congrArg decode h
  · intro h; cases h; rfl

theorem impossible_double_truth (p : Option Bool) : code p ≠ (true, true) := by
  cases p with
  | none => decide
  | some b => cases b <;> decide

theorem undefined_is_ambient_remainder (p : Option Bool) :
    p = none ↔ code p = (false, false) := by
  cases p with
  | none => decide
  | some b => cases b <;> decide

theorem false_is_not_complement_of_true_at_a_hole :
    (code none).2 ≠ !(code none).1 := by decide

theorem total_needs_one_bit (b : Bool) : code (some b) = (b, !b) := by
  cases b <;> rfl

/-- The admitted worlds of the new presentation, retaining their old identity. -/
def Refined {W : Type} (p : W → Option Bool) :=
  { cell : W × (Bool × Bool) // cell.2 = code (p cell.1) }

def refine {W : Type} (p : W → Option Bool) (w : W) : Refined p :=
  ⟨(w, code (p w)), rfl⟩

def forget {W : Type} {p : W → Option Bool} (cell : Refined p) : W := cell.val.1

theorem forget_refine {W : Type} (p : W → Option Bool) (w : W) :
    forget (refine p w) = w := rfl

theorem refine_forget {W : Type} {p : W → Option Bool} (cell : Refined p) :
    refine p (forget cell) = cell := by
  apply Subtype.ext
  apply Prod.ext
  · rfl
  · exact cell.property.symm

theorem no_world_is_added {W : Type} {p : W → Option Bool} (cell : Refined p) :
    ∃ w, refine p w = cell := ⟨forget cell, refine_forget cell⟩

theorem no_world_is_merged {W : Type} (p : W → Option Bool) (a b : W)
    (h : refine p a = refine p b) : a = b := congrArg forget h

theorem truth_is_represented {W : Type} (p : W → Option Bool) (w : W) :
    decode (refine p w).val.2 = p w := decode_code (p w)

def lift {W : Type} {p : W → Option Bool} (e : W → Bool) : Refined p → Bool :=
  fun cell => e (forget cell)

theorem lifted_membership {W : Type} (p : W → Option Bool) (e : W → Bool) (w : W) :
    lift e (refine p w) = e w := rfl

theorem lift_every_truth_table {W : Type} {p : W → Option Bool}
    (op : Bool → Bool → Bool) (a b : W → Bool) :
    (lift (fun w => op (a w) (b w)) : Refined p → Bool) =
      (fun cell => op (lift a cell) (lift b cell)) := rfl

theorem lifted_event_has_same_witnesses {W : Type} (p : W → Option Bool) (e : W → Bool) :
    (∃ cell : Refined p, lift e cell = true) ↔ ∃ w, e w = true := by
  constructor
  · rintro ⟨cell, h⟩; exact ⟨forget cell, h⟩
  · rintro ⟨w, h⟩; exact ⟨refine p w, h⟩

theorem zero_weight_world_is_retained {W : Type} (p : W → Option Bool)
    (weight : W → Rat) (w : W) (zero : weight w = 0) :
    ∃ cell : Refined p, forget cell = w ∧ weight (forget cell) = 0 := by
  exact ⟨refine p w, rfl, zero⟩

/-- Finite contraction under transported weights, with the complete roster. -/
theorem contraction_is_preserved {W : Type} (p : W → Option Bool)
    (worlds : List W) (weight : W → Rat) (e : W → Bool) :
    ((worlds.map (refine p)).map
      (fun cell => if lift e cell then weight (forget cell) else 0)).sum =
    (worlds.map (fun w => if e w then weight w else 0)).sum := by
  simp [List.map_map, Function.comp_def, lift, forget, refine]

/-- No rounding a predicate that splits an existing logical cell. -/
def Resolves {W O : Type} (view : W → O) (p : W → Option Bool) : Prop :=
  ∀ a b, view a = view b → p a = p b

theorem unresolved_cell_refuses_factor {W O : Type} (view : W → O)
    (p : W → Option Bool) (a b : W) (same : view a = view b) (different : p a ≠ p b) :
    ¬ ∃ f : O → Option Bool, ∀ w, f (view w) = p w := by
  rintro ⟨f, exact⟩
  apply different
  rw [← exact a, ← exact b, same]

theorem refinement_resolves_truth {W O : Type} (view : W → O) (p : W → Option Bool) :
    Resolves (fun w => (view w, code (p w))) p := by
  intro a b h
  exact (code_injective _ _).mp (congrArg Prod.snd h)

theorem factor_implies_resolution {W O : Type} (view : W → O)
    (p : W → Option Bool) (f : O → Option Bool) (exact : ∀ w, f (view w) = p w) :
    Resolves view p := by
  intro a b h
  rw [← exact a, ← exact b, h]

/- Query guards expose the three cases as ordinary Event membership. These
laws cover the later Event join/union and the exact presentation descent gate.
They do not assert completeness of a bounded native solver. -/
def caseEvent (p : Option Bool) (tag : Option Bool) : Bool := p == tag

theorem query_cases_cover (p : Option Bool) :
    (caseEvent p (some true) || caseEvent p (some false) || caseEvent p none) = true := by
  cases p with
  | none => rfl
  | some b => cases b <;> rfl

theorem query_cases_are_disjoint (p : Option Bool) :
    (caseEvent p (some true) && caseEvent p (some false)) = false ∧
    (caseEvent p (some true) && caseEvent p none) = false ∧
    (caseEvent p (some false) && caseEvent p none) = false := by
  cases p with
  | none => decide
  | some b => cases b <;> decide

theorem query_partition_reconstructs_claim (p : Option Bool) (claim : Bool) :
    ((claim && caseEvent p (some true)) || (claim && caseEvent p (some false)) ||
      (claim && caseEvent p none)) = claim := by
  cases p with
  | none => cases claim <;> rfl
  | some b => cases b <;> cases claim <;> rfl

theorem event_descends_iff_constant_on_cells {W O : Type}
    (view : W → O) (e : W → Bool) :
    (∃ f : O → Bool, ∀ w, f (view w) = e w) ↔
      (∀ a b, view a = view b → e a = e b) := by
  constructor
  · rintro ⟨f, exact⟩ a b same
    rw [← exact a, ← exact b, same]
  · intro same
    classical
    refine ⟨fun o => decide (∃ w, view w = o ∧ e w = true), ?_⟩
    intro w
    cases hw : e w with
    | false =>
      have absent : ¬ ∃ v, view v = view w ∧ e v = true := by
        rintro ⟨v, hv, he⟩
        have h := same v w hv
        simp [he, hw] at h
      simp [absent]
    | true =>
      have present : ∃ v, view v = view w ∧ e v = true := ⟨w, rfl, hw⟩
      simp [present]

theorem exact_descent_is_unique_on_legal_cells {W O : Type}
    (view : W → O) (onto : ∀ o, ∃ w, view w = o) (a b : O → Bool)
    (same : ∀ w, a (view w) = b (view w)) : a = b := by
  funext o
  obtain ⟨w, hw⟩ := onto o
  simpa [hw] using same w

/-- A common presentation distinguishes worlds by each whole partial truth,
including its undefined case. It never substitutes separate existential answers. -/
def AgreeOn {W : Type} (roster : List (W → Option Bool)) (a b : W) : Prop :=
  ∀ p ∈ roster, p a = p b

theorem common_codes_exact {W : Type} (roster : List (W → Option Bool)) (a b : W) :
    roster.map (fun p => p a) = roster.map (fun p => p b) ↔ AgreeOn roster a b := by
  induction roster with
  | nil => simp [AgreeOn]
  | cons p ps ih => simp [AgreeOn] at ih ⊢

theorem common_roster_append {W : Type} (ps qs : List (W → Option Bool)) (a b : W) :
    AgreeOn (ps ++ qs) a b ↔ AgreeOn ps a b ∧ AgreeOn qs a b := by
  simp [AgreeOn, or_imp, forall_and]

theorem repeated_guard_changes_no_cells {W : Type} (p : W → Option Bool)
    (ps : List (W → Option Bool)) (a b : W) :
    AgreeOn (p :: p :: ps) a b ↔ AgreeOn (p :: ps) a b := by
  simp [AgreeOn]

theorem reordered_guards_change_no_cells {W : Type} (ps qs : List (W → Option Bool))
    (permutation : ps.Perm qs) (a b : W) : AgreeOn ps a b ↔ AgreeOn qs a b := by
  constructor
  · intro same p hp; exact same p (permutation.mem_iff.mpr hp)
  · intro same p hp; exact same p (permutation.mem_iff.mp hp)

theorem common_view_is_coarsest_refinement {W O V : Type} (old : W → O)
    (ps : List (W → Option Bool)) (view : W → V)
    (preserves : ∀ a b, view a = view b → old a = old b)
    (resolves : ∀ a b, view a = view b → AgreeOn ps a b) :
    ∀ a b, view a = view b →
      (old a, ps.map (fun p => p a)) = (old b, ps.map (fun p => p b)) := by
  intro a b same
  exact Prod.ext (preserves a b same) ((common_codes_exact ps a b).mpr (resolves a b same))

/-- Binding a source from a row keeps each row's legal world type. It does not
form a product, identify sources or create independent copies of a parameter. -/
def boundRefine {R : Type} {W : R → Type} (p : (r : R) → W r → Option Bool)
    (world : Sigma W) : Sigma (fun r => Refined (p r)) :=
  ⟨world.1, refine (p world.1) world.2⟩

def boundForget {R : Type} {W : R → Type} {p : (r : R) → W r → Option Bool}
    (world : Sigma (fun r => Refined (p r))) : Sigma W :=
  ⟨world.1, forget world.2⟩

theorem bound_forget_refine {R : Type} {W : R → Type}
    (p : (r : R) → W r → Option Bool) (world : Sigma W) :
    boundForget (boundRefine p world) = world := by cases world; rfl

theorem bound_refine_forget {R : Type} {W : R → Type}
    {p : (r : R) → W r → Option Bool} (world : Sigma (fun r => Refined (p r))) :
    boundRefine p (boundForget world) = world := by
  cases world with
  | mk r cell => simp only [boundRefine, boundForget, refine_forget]

theorem bound_sources_are_not_conflated {R : Type} {W : R → Type}
    (p : (r : R) → W r → Option Bool) (a b : Sigma W)
    (same : boundRefine p a = boundRefine p b) : a = b := by
  simpa only [bound_forget_refine] using congrArg boundForget same

theorem bound_truth_uses_its_row {R : Type} {W : R → Type}
    (p : (r : R) → W r → Option Bool) (r : R) (w : W r) :
    decode (boundRefine p ⟨r, w⟩).2.val.2 = p r w := decode_code (p r w)

/-- Restricting a source to a marker retains every legal world exactly when
the marker is full. This is about possibility, regardless of any law's mass. -/
theorem source_marker_covers_iff_full {W : Type} (marker : W → Bool) :
    (∀ w, ∃ cell : {x : W // marker x = true}, cell.val = w) ↔
      (∀ w, marker w = true) := by
  constructor
  · intro covers w
    obtain ⟨cell, same⟩ := covers w
    simpa only [same] using cell.property
  · intro full w
    exact ⟨⟨w, full w⟩, rfl⟩

theorem bound_contraction_keeps_each_source_law {R : Type} {W : R → Type}
    (p : (r : R) → W r → Option Bool) (worlds : List (Sigma W))
    (weight : Sigma W → Rat) (e : Sigma W → Bool) :
    ((worlds.map (boundRefine p)).map
      (fun cell => if e (boundForget cell) then weight (boundForget cell) else 0)).sum =
      (worlds.map (fun w => if e w then weight w else 0)).sum := by
  simp only [List.map_map, Function.comp_def, bound_forget_refine]

#print axioms PredicateGuards.decode_code
#print axioms PredicateGuards.code_injective
#print axioms PredicateGuards.impossible_double_truth
#print axioms PredicateGuards.undefined_is_ambient_remainder
#print axioms PredicateGuards.false_is_not_complement_of_true_at_a_hole
#print axioms PredicateGuards.total_needs_one_bit
#print axioms PredicateGuards.forget_refine
#print axioms PredicateGuards.refine_forget
#print axioms PredicateGuards.no_world_is_added
#print axioms PredicateGuards.no_world_is_merged
#print axioms PredicateGuards.truth_is_represented
#print axioms PredicateGuards.lifted_membership
#print axioms PredicateGuards.lift_every_truth_table
#print axioms PredicateGuards.lifted_event_has_same_witnesses
#print axioms PredicateGuards.zero_weight_world_is_retained
#print axioms PredicateGuards.contraction_is_preserved
#print axioms PredicateGuards.unresolved_cell_refuses_factor
#print axioms PredicateGuards.refinement_resolves_truth
#print axioms PredicateGuards.factor_implies_resolution
#print axioms PredicateGuards.query_cases_cover
#print axioms PredicateGuards.query_cases_are_disjoint
#print axioms PredicateGuards.query_partition_reconstructs_claim
#print axioms PredicateGuards.event_descends_iff_constant_on_cells
#print axioms PredicateGuards.exact_descent_is_unique_on_legal_cells
#print axioms PredicateGuards.common_codes_exact
#print axioms PredicateGuards.common_roster_append
#print axioms PredicateGuards.repeated_guard_changes_no_cells
#print axioms PredicateGuards.reordered_guards_change_no_cells
#print axioms PredicateGuards.common_view_is_coarsest_refinement
#print axioms PredicateGuards.bound_forget_refine
#print axioms PredicateGuards.bound_refine_forget
#print axioms PredicateGuards.bound_sources_are_not_conflated
#print axioms PredicateGuards.bound_truth_uses_its_row
#print axioms PredicateGuards.source_marker_covers_iff_full
#print axioms PredicateGuards.bound_contraction_keeps_each_source_law
end PredicateGuards
