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
end PredicateGuards
