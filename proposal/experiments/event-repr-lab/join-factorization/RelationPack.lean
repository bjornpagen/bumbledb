import Std

namespace RelationPack

/-- Row-index reduction can move outside relational composition while keeping
    the intermediate world witness y shared. Interfaces are typed X,Y,Z. -/
theorem composition {I J X Y Z : Type}
    (r : I → X → Y → Prop) (s : J → Y → Z → Prop) (x : X) (z : Z) :
    (∃ i j y, r i x y ∧ s j y z) ↔
      (∃ y, (∃ i, r i x y) ∧ (∃ j, s j y z)) := by
  constructor
  · rintro ⟨i,j,y,hr,hs⟩; exact ⟨y,⟨i,hr⟩,⟨j,hs⟩⟩
  · rintro ⟨y,⟨i,hr⟩,⟨j,hs⟩⟩; exact ⟨i,j,y,hr,hs⟩

def residual {X Y Z : Type} (r : X → Y → Prop) (t : X → Z → Prop) (y : Y) (z : Z) :=
  ∀ x, r x y → t x z

/-- A union in the antecedent becomes intersection of residual obligations. -/
theorem residual_union {I X Y Z : Type}
    (r : I → X → Y → Prop) (t : X → Z → Prop) (y : Y) (z : Z) :
    residual (fun x y => ∃ i, r i x y) t y z ↔
      ∀ i, residual (r i) t y z := by
  constructor
  · intro h i x hr; exact h x ⟨i,hr⟩
  · intro h x hr; obtain ⟨i,hi⟩ := hr; exact h i x hi

/-- An intersection in the consequent also becomes intersected obligations. -/
theorem residual_intersection {J X Y Z : Type}
    (r : X → Y → Prop) (t : J → X → Z → Prop) (y : Y) (z : Z) :
    residual r (fun x z => ∀ j, t j x z) y z ↔
      ∀ j, residual r (t j) y z := by
  constructor
  · intro h j x hr; exact h x hr j
  · intro h x hr j; exact h j x hr

/-- Separately possible transitions do not certify a shared intermediate world. -/
theorem separate_witness_counterexample :
    (∃ y : Bool, y=false) ∧ (∃ y : Bool, y=true) ∧
      (¬ ∃ y : Bool, y=false ∧ y=true) := by decide

end RelationPack
#print axioms RelationPack.composition
#print axioms RelationPack.residual_union
#print axioms RelationPack.residual_intersection
#print axioms RelationPack.separate_witness_counterexample
