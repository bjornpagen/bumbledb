import Std

namespace LocalCompletion

/-- A completed predicate is already invariant under each independent
    idempotent face decoder, even when the predicate couples the two faces. -/
theorem separately_invariant {X Y : Type} (dx : X → X) (dy : Y → Y)
    (A : X → Y → Prop) (idy : ∀ y, dy (dy y) = dy y)
    (complete : ∀ x y, A (dx x) (dy y) ↔ A x y) :
    ∀ x y, A x (dy y) ↔ A x y := by
  intro x y
  have h := complete x (dy y)
  rw [idy y] at h
  exact h.symm.trans (complete x y)

/-- R may contain a domain gate and arbitrary observation/transition
    constraints on X. It must not introduce a new constraint on Y. -/
theorem existential_preserves_invariance {X Y : Type}
    (dy : Y → Y) (A : X → Y → Prop) (R : X → X → Prop)
    (stable : ∀ x y, A x (dy y) ↔ A x y) :
    ∀ x y, (∃ v, R v x ∧ A v (dy y)) ↔ (∃ v, R v x ∧ A v y) := by
  intro x y
  constructor
  · rintro ⟨v, hr, ha⟩
    exact ⟨v, hr, (stable v y).mp ha⟩
  · rintro ⟨v, hr, ha⟩
    exact ⟨v, hr, (stable v y).mpr ha⟩

theorem universal_preserves_invariance {X Y : Type}
    (dy : Y → Y) (A : X → Y → Prop) (R : X → X → Prop)
    (stable : ∀ x y, A x (dy y) ↔ A x y) :
    ∀ x y, (∀ v, R v x → A v (dy y)) ↔ (∀ v, R v x → A v y) := by
  intro x y
  constructor
  · intro h v hr
    exact (stable v y).mp (h v hr)
  · intro h v hr
    exact (stable v y).mpr (h v hr)

/-- Complete the affected face only. This is equality of predicates on all raw
    codes, stronger than agreement only on admitted outputs. -/
theorem local_completion_exact {X Y : Type}
    (dx : X → X) (dy : Y → Y) (A : X → Y → Prop) (R : X → X → Prop)
    (idy : ∀ y, dy (dy y) = dy y)
    (complete : ∀ x y, A (dx x) (dy y) ↔ A x y) :
    ∀ x y, (∃ v, R v (dx x) ∧ A v (dy y)) ↔
      (∃ v, R v (dx x) ∧ A v y) := by
  intro x y
  exact existential_preserves_invariance dy A R
    (separately_invariant dx dy A idy complete) (dx x) y

/-- A raw environment is repaired first. Independent face decoders are indexed
    by that resolved environment; no invariance across different laws is used. -/
theorem resolved_environment_invariance {E X Y : Type}
    (de : E → E) (dx : E → X → X) (dy : E → Y → Y)
    (A : E → X → Y → Prop) (ide : ∀ e, de (de e) = de e)
    (idy : ∀ e y, dy e (dy e y) = dy e y)
    (complete : ∀ e x y, A (de e) (dx (de e) x) (dy (de e) y) ↔ A e x y) :
    ∀ e x y, A (de e) x (dy (de e) y) ↔ A (de e) x y := by
  intro e
  apply separately_invariant (dx (de e)) (dy (de e)) (A (de e)) (idy (de e))
  intro x y
  have h := complete (de e) x y
  rw [ide e] at h
  exact h

/-- Equality on every raw environment, including empty/invalid environment
    codes, provided the environment remains observed during quantification. -/
theorem indexed_local_completion_exact {E X Y : Type}
    (de : E → E) (dx : E → X → X) (dy : E → Y → Y)
    (A : E → X → Y → Prop) (R : E → X → X → Prop)
    (ide : ∀ e, de (de e) = de e) (idy : ∀ e y, dy e (dy e y) = dy e y)
    (complete : ∀ e x y, A (de e) (dx (de e) x) (dy (de e) y) ↔ A e x y) :
    ∀ e x y,
      (∃ v, R (de e) v (dx (de e) x) ∧ A (de e) v (dy (de e) y)) ↔
      (∃ v, R (de e) v (dx (de e) x) ∧ A (de e) v y) := by
  intro e x y
  exact existential_preserves_invariance (dy (de e)) (A (de e)) (R (de e))
    (resolved_environment_invariance de dx dy A ide idy complete e) (dx (de e) x) y

/-- A remaining legal-domain gate on the untouched face breaks the premise:
    legal Y is {false}, its decoder returns false, and A is constantly true. -/
theorem retained_gate_obstruction :
    let A : Bool → Bool → Prop := fun _ _ => True
    let dy : Bool → Bool := fun _ => false
    let projected : Bool → Prop := fun y => ∃ x, A x y ∧ y = false
    (∀ x y, A x (dy y) ↔ A x y) ∧
    projected (dy true) ∧ ¬ projected true := by decide

/-- On copied support x=y, completion of Y depends on X. Leaving the observed
    raw Y unchanged produces a different representative outside that support. -/
theorem coupled_decoder_obstruction :
    let S : Bool → Bool → Prop := fun x y => x = y
    let A : Bool → Bool → Prop := fun x _ => x = true
    let repairedY : Bool → Bool → Bool := fun x _ => x
    let projected : Bool → Prop := fun y => ∃ v, S v y ∧ A v y
    (∀ x y, S x (repairedY x y)) ∧
    (∀ x y, S x y → repairedY x y = y) ∧
    (∀ x y, A x (repairedY x y) ↔ A x y) ∧
    projected (repairedY true false) ∧ ¬ projected false := by decide

end LocalCompletion

#print axioms LocalCompletion.separately_invariant
#print axioms LocalCompletion.existential_preserves_invariance
#print axioms LocalCompletion.universal_preserves_invariance
#print axioms LocalCompletion.local_completion_exact
#print axioms LocalCompletion.resolved_environment_invariance
#print axioms LocalCompletion.indexed_local_completion_exact
#print axioms LocalCompletion.retained_gate_obstruction
#print axioms LocalCompletion.coupled_decoder_obstruction
