import Std

/-!
Probability heads retain source/event/evidence identity through relational
projection, then contract the admitted joint law. This is a reference contract;
it does not prove Rust registry alignment, Free Join, spill or worker transport.
World lists must enumerate legal outcomes once. Parameters index the law and
are never silently summed or assigned a prior.
-/
namespace QueryProbability

abbrev Region (W : Type) := W → Bool

structure Key (S W : Type) where
  source : S
  event : Region W
  given : Region W

def sum {W : Type} : List W → (W → Rat) → Rat
  | [], _ => 0
  | w :: ws, f => f w + sum ws f

def mass {W : Type} (ws : List W) (law : W → Rat) (event : Region W) : Rat :=
  sum ws (fun w => if event w then law w else 0)

def conditional (numerator evidenceMass : Rat) : Option Rat :=
  if evidenceMass = 0 then none else some (numerator / evidenceMass)

structure Observation (S W : Type) where
  key : Key S W
  numerator : Rat
  evidenceMass : Rat

def observe {S W : Type} (ws : List W) (law : S → W → Rat) (key : Key S W) : Observation S W :=
  ⟨key, mass ws (law key.source) (fun w => key.event w && key.given w),
    mass ws (law key.source) key.given⟩

def value {S W : Type} (answer : Observation S W) : Option Rat :=
  conditional answer.numerator answer.evidenceMass

theorem source_pair_retained {S W : Type} (ws : List W) (law : S → W → Rat) (key : Key S W) :
    (observe ws law key).key = key := rfl

theorem impossible_iff_zero_mass (n d : Rat) : conditional n d = none ↔ d = 0 := by
  simp [conditional]

theorem observation_equality_requires_source_pair {S W : Type} (a b : Observation S W)
    (same : a = b) : a.key = b.key := congrArg Observation.key same

/-- Relational projection deduplicates whole keys, not their numerical values. -/
def projected {B S W : Type} (bindings : B → Prop) (head : B → Key S W) (key : Key S W) : Prop :=
  ∃ b, bindings b ∧ head b = key

theorem arms_union {B S W : Type} (a b : B → Prop) (head : B → Key S W) (key : Key S W) :
    projected (fun row => a row ∨ b row) head key ↔ projected a head key ∨ projected b head key := by
  simp only [projected]
  constructor
  · rintro ⟨row, ha | hb, same⟩
    · exact Or.inl ⟨row, ha, same⟩
    · exact Or.inr ⟨row, hb, same⟩
  · rintro (⟨row, ha, same⟩ | ⟨row, hb, same⟩)
    · exact ⟨row, Or.inl ha, same⟩
    · exact ⟨row, Or.inr hb, same⟩

theorem duplicate_arm_idempotent {B S W : Type} (a : B → Prop) (head : B → Key S W) (key : Key S W) :
    projected (fun row => a row ∨ a row) head key ↔ projected a head key := by
  simp

theorem body_enumeration_irrelevant {B S W : Type} (a b : B → Prop)
    (same : ∀ row, a row ↔ b row) (head : B → Key S W) (key : Key S W) :
    projected a head key ↔ projected b head key := by
  simp only [projected, same]

theorem sum_add {W : Type} (ws : List W) (a b : W → Rat) :
    sum ws (fun w => a w + b w) = sum ws a + sum ws b := by
  induction ws with
  | nil => simp only [sum]; grind
  | cons w ws ih => simp only [sum, ih]; grind

theorem complementary_numerators {W : Type} (ws : List W) (law : W → Rat) (event given : Region W) :
    mass ws law (fun w => event w && given w) + mass ws law (fun w => !(event w) && given w) =
      mass ws law given := by
  unfold mass
  rw [←sum_add]
  congr 1
  funext w
  cases he : event w <;> cases hg : given w <;> simp [he, hg] <;> grind

def left : Key Unit Bool := ⟨(), id, fun _ => true⟩
def right : Key Unit Bool := ⟨(), Bool.not, fun _ => true⟩

theorem equal_numbers_do_not_identify_events :
    value (observe [false, true] (fun _ _ => 1/2) left) = some (1/2) ∧
    value (observe [false, true] (fun _ _ => 1/2) right) = some (1/2) ∧ left ≠ right := by
  refine ⟨by decide +kernel, by decide +kernel, ?_⟩
  intro h
  have bad := congrArg (fun k => k.event false) h
  cases bad

theorem nonempty_evidence_can_be_impossible :
    let key : Key Unit Bool := ⟨(), fun _ => true, id⟩
    (∃ w, key.given w = true) ∧
      value (observe [false, true] (fun _ w => if w then 0 else 1) key) = none := by
  exact ⟨⟨true, rfl⟩, by decide +kernel⟩

/-- A family remains pointwise: no averaging over the unknown parameter. -/
def twoDraws (p : Rat) : Option Rat := conditional (p*p) p

theorem shared_parameter_hole : twoDraws 0 = none := by decide +kernel
theorem shared_parameter_exact : twoDraws (1/3) = some (1/3) := by decide +kernel
theorem family_is_not_one_fixed_number : twoDraws (1/3) ≠ twoDraws (2/3) := by decide +kernel

inductive Refusal where | missingLaw

def admit {S W : Type} (ws : List W) (laws : S → Option (W → Rat)) (key : Key S W) :
    Except Refusal (Observation S W) :=
  match laws key.source with
  | none => .error .missingLaw
  | some law => .ok (observe ws (fun _ => law) key)

theorem missing_law_remains_error {S W : Type} (ws : List W) (key : Key S W) :
    admit ws (fun _ => none) key = .error .missingLaw := rfl

#print axioms QueryProbability.source_pair_retained
#print axioms QueryProbability.impossible_iff_zero_mass
#print axioms QueryProbability.observation_equality_requires_source_pair
#print axioms QueryProbability.arms_union
#print axioms QueryProbability.duplicate_arm_idempotent
#print axioms QueryProbability.body_enumeration_irrelevant
#print axioms QueryProbability.sum_add
#print axioms QueryProbability.complementary_numerators
#print axioms QueryProbability.equal_numbers_do_not_identify_events
#print axioms QueryProbability.nonempty_evidence_can_be_impossible
#print axioms QueryProbability.shared_parameter_hole
#print axioms QueryProbability.shared_parameter_exact
#print axioms QueryProbability.family_is_not_one_fixed_number
#print axioms QueryProbability.missing_law_remains_error

end QueryProbability
