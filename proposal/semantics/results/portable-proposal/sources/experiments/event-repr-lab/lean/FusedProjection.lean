import Std

namespace FusedProjection

/-- Source witnesses are eliminated before the target coordinate map is read.
    R retains the joint context; its witnesses cannot be split per operand. -/
def contract {S T : Type} (R : S → T → Prop) (A : S → Prop) (t : T) : Prop :=
  ∃ s, R s t ∧ A s

/-- At a hidden source bit, both source cofactors share the same target. -/
theorem hidden_split {X T : Type} (R : X → T → Prop) (A : Bool → X → Prop) :
    ∀ t, (∃ b x, R x t ∧ A b x) ↔
      ((∃ x, R x t ∧ A false x) ∨ (∃ x, R x t ∧ A true x)) := by
  intro t
  constructor
  · rintro ⟨b, x, hr, ha⟩
    cases b with
    | false => exact Or.inl ⟨x, hr, ha⟩
    | true => exact Or.inr ⟨x, hr, ha⟩
  · rintro (⟨x, hr, ha⟩ | ⟨x, hr, ha⟩)
    · exact ⟨false, x, hr, ha⟩
    · exact ⟨true, x, hr, ha⟩

/-- An observed source bit equals a target decoder function. That function may
    mention coordinates which have the same names as hidden source bits. -/
theorem observed_split {X T : Type} (R : X → T → Prop)
    (A : Bool → X → Prop) (select : T → Bool) :
    ∀ t, (∃ b x, b = select t ∧ R x t ∧ A b x) ↔
      (if select t then (∃ x, R x t ∧ A true x) else (∃ x, R x t ∧ A false x)) := by
  intro t
  cases select t
  · constructor
    · rintro ⟨b, x, hb, hr, ha⟩
      subst b
      exact ⟨x, hr, ha⟩
    · rintro ⟨x, hr, ha⟩
      exact ⟨false, x, rfl, hr, ha⟩
  · constructor
    · rintro ⟨b, x, hb, hr, ha⟩
      subst b
      exact ⟨x, hr, ha⟩
    · rintro ⟨x, hr, ha⟩
      exact ⟨true, x, rfl, hr, ha⟩

/-- Once no remaining source coordinate needs elimination or substitution,
    the residual function is returned directly. -/
theorem graph_substitution {S T : Type} (d : T → S) (A : S → Prop) :
    ∀ t, contract (fun s t => s = d t) A t ↔ A (d t) := by
  intro t
  constructor
  · rintro ⟨s, hs, ha⟩
    subst s
    exact ha
  · intro ha
    exact ⟨d t, rfl, ha⟩

theorem absent_hidden_bit (A : Prop) : (∃ _ : Bool, A) ↔ A := by
  constructor
  · rintro ⟨_, ha⟩
    exact ha
  · intro ha
    exact ⟨false, ha⟩

theorem contract_union {S T : Type} (R : S → T → Prop) (A B : S → Prop) :
    ∀ t, contract R (fun s => A s ∨ B s) t ↔ contract R A t ∨ contract R B t := by
  intro t
  constructor
  · rintro ⟨s, hr, ha | hb⟩
    · exact Or.inl ⟨s, hr, ha⟩
    · exact Or.inr ⟨s, hr, hb⟩
  · rintro (⟨s, hr, ha⟩ | ⟨s, hr, hb⟩)
    · exact ⟨s, hr, Or.inl ha⟩
    · exact ⟨s, hr, Or.inr hb⟩

/-- Completion alone commutes with complement. Completion after existential
    projection does not, so its memo must retain the full signed source root. -/
theorem complement_memo_obstruction :
    (∃ b : Bool, ¬ (b = true)) ∧ ¬ (¬ (∃ b : Bool, b = true)) := by decide

/-- Both operands must keep the same hidden source witness. -/
theorem separate_witness_obstruction :
    ((∃ b : Bool, b = false) ∧ (∃ b : Bool, b = true)) ∧
    ¬ (∃ b : Bool, b = false ∧ b = true) := by decide

/-- Re-abstracting a hidden *target* bit introduced by a selector changes the
    answer. Source elimination must never visit inserted decoder functions. -/
theorem target_abstraction_obstruction :
    let A : Bool → Bool → Prop := fun _ y => y = true
    (∃ b v, A v b) ∧ ¬ (∃ v, A v false) := by decide

end FusedProjection

#print axioms FusedProjection.hidden_split
#print axioms FusedProjection.observed_split
#print axioms FusedProjection.graph_substitution
#print axioms FusedProjection.absent_hidden_bit
#print axioms FusedProjection.contract_union
#print axioms FusedProjection.complement_memo_obstruction
#print axioms FusedProjection.separate_witness_obstruction
#print axioms FusedProjection.target_abstraction_obstruction
