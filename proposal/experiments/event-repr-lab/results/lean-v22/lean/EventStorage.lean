import Std

namespace EventStorage

/-- Fact IDs are already distinct whole facts in one scalar determinant group.
    Adding a bottom-valued row adds a fact ID, but no world incidence. -/
def addEmpty {F W : Type} (incidence : F → W → Prop) : Option F → W → Prop
  | none, _ => False
  | some f, w => incidence f w

def covered {F W : Type} (incidence : F → W → Prop) (w : W) : Prop :=
  ∃ f, incidence f w

def conflict {F W : Type} (incidence : F → W → Prop) (w : W) : Prop :=
  ∃ a b, a ≠ b ∧ incidence a w ∧ incidence b w

def pointwiseKey {F W : Type} (incidence : F → W → Prop) : Prop :=
  ∀ a b w, incidence a w → incidence b w → a = b

theorem coverage_empty_row {F W : Type} (incidence : F → W → Prop) :
    ∀ w, covered (addEmpty incidence) w ↔ covered incidence w := by
  intro w
  constructor
  · rintro ⟨a, ha⟩
    cases a with
    | none => exact False.elim ha
    | some f => exact ⟨f, ha⟩
  · rintro ⟨f, hf⟩
    exact ⟨some f, hf⟩

theorem conflict_empty_row {F W : Type} (incidence : F → W → Prop) :
    ∀ w, conflict (addEmpty incidence) w ↔ conflict incidence w := by
  intro w
  constructor
  · rintro ⟨a, b, different, ha, hb⟩
    cases a with
    | none => exact False.elim ha
    | some f =>
      cases b with
      | none => exact False.elim hb
      | some g =>
        exact ⟨f, g, fun h => different (congrArg some h), ha, hb⟩
  · rintro ⟨a, b, different, ha, hb⟩
    exact ⟨some a, some b, fun h => different (Option.some.inj h), ha, hb⟩

/-- This is the missing premise when reusing the interval planner witness.
    Equal complete regions identify a fact only when that region has a point. -/
theorem complete_key_nonempty {F W : Type} (incidence : F → W → Prop)
    (key : pointwiseKey incidence) (a b : F)
    (same : ∀ w, incidence a w ↔ incidence b w) (inhabited : ∃ w, incidence a w) : a = b := by
  obtain ⟨w, ha⟩ := inhabited
  exact key a b w ha ((same w).mp ha)

theorem empty_key_counterexample :
    let incidence : Bool → Unit → Prop := fun _ _ => False
    pointwiseKey incidence ∧ (∀ w, incidence false w ↔ incidence true w) ∧ false ≠ true := by
  exact ⟨fun _ _ _ ha _ => False.elim ha, fun _ => Iff.rfl, Bool.false_ne_true⟩

/-- Inverse image preserves bottom. Owner/alignment validation is a separate
    precondition, not erased by this denotational identity. -/
theorem lift_empty {S T : Type} (f : T → S) :
    ∀ t, (fun _ : S => False) (f t) ↔ False := by
  intro _
  exact Iff.rfl

theorem projection_empty {S T : Type} (R : S → T → Prop) :
    ∀ t, (∃ s, R s t ∧ False) ↔ False := by
  intro t
  constructor
  · rintro ⟨_, _, h⟩
    exact h
  · intro h
    exact False.elim h

/-- All(R,bottom) is the dead-end region, not bottom in general. -/
theorem universal_empty {S T : Type} (R : S → T → Prop) :
    ∀ s, (∀ t, R s t → False) ↔ ¬ (∃ t, R s t) := by
  intro s
  constructor
  · intro h ⟨t, ht⟩
    exact h t ht
  · intro h t ht
    exact h ⟨t, ht⟩

/-- A pointwise inclusion with an empty source requires no target witness.
    It therefore cannot imply an ordinary scalar foreign-key fact exists. -/
theorem empty_source_no_target :
    (∀ _ : Unit, False → (∃ _ : Empty, True)) ∧ ¬ (∃ _ : Empty, True) := by
  constructor
  · intro _ h
    exact False.elim h
  · rintro ⟨f, _⟩
    exact nomatch f

/-- On a singleton legal world, a Boolean represents its entire Event.
    Filtering bottom-valued rows before complement loses a full-valued row. -/
theorem dropping_empty_changes_complement :
    (([false].filter id).map Bool.not = []) ∧
    ([false].map Bool.not = [true]) ∧ ([] : List Bool) ≠ [true] := by decide

theorem nonempty_owner_distinguishes_constants {W : Type} (world : W) :
    ¬ (∀ _ : W, False ↔ True) := by
  intro h
  exact (h world).mpr True.intro

end EventStorage

#print axioms EventStorage.coverage_empty_row
#print axioms EventStorage.conflict_empty_row
#print axioms EventStorage.complete_key_nonempty
#print axioms EventStorage.empty_key_counterexample
#print axioms EventStorage.lift_empty
#print axioms EventStorage.projection_empty
#print axioms EventStorage.universal_empty
#print axioms EventStorage.empty_source_no_target
#print axioms EventStorage.dropping_empty_changes_complement
#print axioms EventStorage.nonempty_owner_distinguishes_constants
