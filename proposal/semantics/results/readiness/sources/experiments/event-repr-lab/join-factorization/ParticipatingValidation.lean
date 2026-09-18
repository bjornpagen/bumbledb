import Std

namespace ParticipatingValidation

/-- Validating the Cartesian join equals validating precisely the rows that
    participate. Unmatched branch values are not subject to evaluation. -/
theorem all_joined_iff_participating {I J K : Type}
    (a : I → Prop) (b : J → Prop) (c : K → Prop) :
    (∀ i j k, a i ∧ b j ∧ c k) ↔
      ((Nonempty J ∧ Nonempty K) → ∀ i, a i) ∧
      ((Nonempty I ∧ Nonempty K) → ∀ j, b j) ∧
      ((Nonempty I ∧ Nonempty J) → ∀ k, c k) := by
  constructor
  · intro h
    constructor
    · rintro ⟨⟨j⟩,⟨k⟩⟩ i; exact (h i j k).1
    · constructor
      · rintro ⟨⟨i⟩,⟨k⟩⟩ j; exact (h i j k).2.1
      · rintro ⟨⟨i⟩,⟨j⟩⟩ k; exact (h i j k).2.2
  · rintro ⟨ha,hb,hc⟩ i j k
    exact ⟨ha ⟨⟨j⟩,⟨k⟩⟩ i,hb ⟨⟨i⟩,⟨k⟩⟩ j,hc ⟨⟨i⟩,⟨j⟩⟩ k⟩

/-- Errors do not depend on the eventual Event accumulator. In particular,
    saturation is not a certificate to suppress later participating faults. -/
theorem fault_set_exact {I J K E : Type} (a : I → E → Prop)
    (b : J → E → Prop) (c : K → E → Prop) (e : E) :
    (∃ i j k, a i e ∨ b j e ∨ c k e) ↔
      ((∃ i, a i e) ∧ Nonempty J ∧ Nonempty K) ∨
      (Nonempty I ∧ (∃ j, b j e) ∧ Nonempty K) ∨
      (Nonempty I ∧ Nonempty J ∧ (∃ k, c k e)) := by
  constructor
  · rintro ⟨i,j,k,h⟩
    rcases h with h | h | h
    · exact Or.inl ⟨⟨i,h⟩,⟨j⟩,⟨k⟩⟩
    · exact Or.inr (Or.inl ⟨⟨i⟩,⟨j,h⟩,⟨k⟩⟩)
    · exact Or.inr (Or.inr ⟨⟨i⟩,⟨j⟩,⟨k,h⟩⟩)
  · intro h
    rcases h with ⟨⟨i,hi⟩,⟨j⟩,⟨k⟩⟩ | ⟨⟨i⟩,⟨j,hj⟩,⟨k⟩⟩ | ⟨⟨i⟩,⟨j⟩,⟨k,hk⟩⟩
    · exact ⟨i,j,k,Or.inl hi⟩
    · exact ⟨i,j,k,Or.inr (Or.inl hj)⟩
    · exact ⟨i,j,k,Or.inr (Or.inr hk)⟩

end ParticipatingValidation
#print axioms ParticipatingValidation.all_joined_iff_participating
#print axioms ParticipatingValidation.fault_set_exact
