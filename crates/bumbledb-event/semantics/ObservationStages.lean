import Std

/-!
Reference contract for staging owned observations. `Key` retains the source,
original evidence, and exact Event/function identity. Function identity may be
an arithmetic presentation; no numerical-equivalence oracle is assumed.

An injective registry encoding is an explicit premise, not a proof about the
Rust registry. These theorems do not verify native execution, spill, the macro,
codecs, allocation, or the concrete arithmetic cost model.
-/
namespace ObservationStages

inductive Key (Source Event Function : Type) where
  | probability (source : Source) (event given : Event)
  | expectation (source : Source) (function : Function) (given : Event)

def project {R V : Type} (rows : R → Prop) (value : R → V) : V → Prop :=
  fun v => ∃ r, rows r ∧ value r = v

def Injective {A B : Type} (f : A → B) : Prop := ∀ a b, f a = f b → a = b

theorem registry_equality_iff {V T : Type} (encode : V → T) (faithful : Injective encode)
    (a b : V) : encode a = encode b ↔ a = b :=
  ⟨faithful a b, fun h => congrArg encode h⟩

theorem registry_inequality_iff {V T : Type} (encode : V → T) (faithful : Injective encode)
    (a b : V) : encode a ≠ encode b ↔ a ≠ b := by
  constructor
  · intro h same
    exact h (congrArg encode same)
  · intro h same
    exact h (faithful a b same)

theorem equality_join_preserved {A B V T : Type} (left : A → Prop) (right : B → Prop)
    (lv : A → V) (rv : B → V) (encode : V → T) (faithful : Injective encode) (a : A) (b : B) :
    (left a ∧ right b ∧ encode (lv a) = encode (rv b)) ↔
      (left a ∧ right b ∧ lv a = rv b) := by
  simp only [registry_equality_iff encode faithful]

theorem antijoin_preserved {A B V T : Type} (left : A → Prop) (right : B → Prop)
    (lv : A → V) (rv : B → V) (encode : V → T) (faithful : Injective encode) (a : A) :
    (left a ∧ ¬ ∃ b, right b ∧ encode (lv a) = encode (rv b)) ↔
      (left a ∧ ¬ ∃ b, right b ∧ lv a = rv b) := by
  simp only [registry_equality_iff encode faithful]

theorem projection_preserved {R V T : Type} (rows : R → Prop) (value : R → V)
    (encode : V → T) (faithful : Injective encode) (v : V) :
    project rows (fun r => encode (value r)) (encode v) ↔ project rows value v := by
  simp only [project, registry_equality_iff encode faithful]

theorem projection_composition {R V U : Type} (rows : R → Prop) (f : R → V) (g : V → U) :
    project (project rows f) g = project rows (fun r => g (f r)) := by
  funext u
  apply propext
  constructor
  · rintro ⟨v, ⟨r, hr, rfl⟩, hu⟩
    exact ⟨r, hr, hu⟩
  · rintro ⟨r, hr, hu⟩
    exact ⟨f r, ⟨r, hr, rfl⟩, hu⟩

theorem projected_duplicates_coalesce {R V : Type} (rows : R → Prop) (value : R → V) :
    project (fun r => rows r ∨ rows r) value = project rows value := by
  simp

theorem stage_union {R V : Type} (a b : R → Prop) (value : R → V) (v : V) :
    project (fun r => a r ∨ b r) value v ↔ project a value v ∨ project b value v := by
  constructor
  · rintro ⟨r, ha | hb, hv⟩
    · exact Or.inl ⟨r, ha, hv⟩
    · exact Or.inr ⟨r, hb, hv⟩
  · rintro (⟨r, ha, hv⟩ | ⟨r, hb, hv⟩)
    · exact ⟨r, Or.inl ha, hv⟩
    · exact ⟨r, Or.inr hb, hv⟩

theorem grouping_preserves_binding_membership {R V T : Type} (rows : R → Prop) (value : R → V)
    (encode : V → T) (faithful : Injective encode) (v : V) :
    (fun r => rows r ∧ encode (value r) = encode v) = (fun r => rows r ∧ value r = v) := by
  funext r
  simp only [registry_equality_iff encode faithful]

theorem probability_identity_retains_evidence {S E F : Type} (s t : S) (a b g h : E) :
    (Key.probability s a g : Key S E F) = Key.probability t b h ↔ s = t ∧ a = b ∧ g = h := by
  simp

theorem expectation_identity_retains_function {S E F : Type} (s t : S) (f k : F) (g h : E) :
    (Key.expectation s f g : Key S E F) = Key.expectation t k h ↔ s = t ∧ f = k ∧ g = h := by
  simp

theorem observation_kinds_disjoint {S E F : Type} (s t : S) (a g h : E) (f : F) :
    (Key.probability s a g : Key S E F) ≠ Key.expectation t f h := by
  intro impossible
  cases impossible

/-- Copies preserve the entire interpretation, including undefined parameter values. -/
theorem decoded_observation_preserves_partial_value {V T P N : Type} (encode : V → T)
    (decode : T → V) (roundtrip : ∀ v, decode (encode v) = v)
    (interpret : V → P → Option N) (v : V) (p : P) :
    interpret (decode (encode v)) p = interpret v p := by
  rw [roundtrip]

theorem equal_numbers_insufficient :
    ∃ a b : Key Nat Nat Nat, a ≠ b ∧ (fun _ : Key Nat Nat Nat => (1 : Rat)) a =
      (fun _ : Key Nat Nat Nat => (1 : Rat)) b := by
  refine ⟨.probability 0 0 0, .probability 0 1 0, ?_, rfl⟩
  intro h
  cases h

/-- Admission precedes downstream transformations, including a consumer returning no rows. -/
def consume {E A B : Type} (producer : Except E A) (consumer : A → B) : Except E B :=
  match producer with
  | .error e => .error e
  | .ok rows => .ok (consumer rows)

theorem producer_error_survives_filter {E A B : Type} (error : E) (consumer : A → B) :
    consume (.error error) consumer = .error error := rfl

theorem admitted_values_pass_unchanged {E A : Type} (rows : A) :
    consume (.ok rows : Except E A) id = .ok rows := rfl

theorem stage_pipeline_associates {E A B C : Type} (producer : Except E A)
    (first : A → B) (next : B → C) :
    consume (consume producer first) next = consume producer (fun a => next (first a)) := by
  cases producer <;> rfl

/-- Work already spent is retained across the stage boundary. -/
theorem cumulative_budget_refuses_next (spent next limit : Nat)
    (exhausts : limit < spent + next) : ¬ spent + next ≤ limit := Nat.not_le_of_lt exhausts

#print axioms ObservationStages.registry_equality_iff
#print axioms ObservationStages.registry_inequality_iff
#print axioms ObservationStages.equality_join_preserved
#print axioms ObservationStages.antijoin_preserved
#print axioms ObservationStages.projection_preserved
#print axioms ObservationStages.projection_composition
#print axioms ObservationStages.projected_duplicates_coalesce
#print axioms ObservationStages.stage_union
#print axioms ObservationStages.grouping_preserves_binding_membership
#print axioms ObservationStages.probability_identity_retains_evidence
#print axioms ObservationStages.expectation_identity_retains_function
#print axioms ObservationStages.observation_kinds_disjoint
#print axioms ObservationStages.decoded_observation_preserves_partial_value
#print axioms ObservationStages.equal_numbers_insufficient
#print axioms ObservationStages.producer_error_survives_filter
#print axioms ObservationStages.admitted_values_pass_unchanged
#print axioms ObservationStages.stage_pipeline_associates
#print axioms ObservationStages.cumulative_budget_refuses_next
end ObservationStages
