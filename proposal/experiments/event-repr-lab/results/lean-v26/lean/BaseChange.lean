import Std

/-!
Exact powerset base change. The four types stand for admitted worlds, not raw
presentations containing unsupported assignments. No law or multiplicity is
interpreted here. These are denotational conditions for an optimizer rewrite.
-/
namespace BaseChange

abbrev Region (X : Type) := X → Bool

def Commutes {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z) : Prop :=
  ∀ w, f (u w) = g (v w)

def CompleteFibres {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z) : Prop :=
  ∀ x y, f x = g y → ∃ w, u w = x ∧ v w = y

def Left {W X Y : Type} (u : W → X) (v : W → Y) (A : Region X) (y : Y) : Prop :=
  ∃ w, A (u w) = true ∧ v w = y

def Right {X Y Z : Type} (f : X → Z) (g : Y → Z) (A : Region X) (y : Y) : Prop :=
  ∃ x, A x = true ∧ f x = g y

def Exact {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z) : Prop :=
  ∀ A y, Left u v A y ↔ Right f g A y

abbrev Relation (X Y : Type) := X → Y → Prop
def graph {X Y : Type} (f : X → Y) : Relation X Y := fun x y => f x = y
def converse {X Y : Type} (R : Relation X Y) : Relation Y X := fun y x => R x y
def compose {X Y Z : Type} (R : Relation X Y) (Q : Relation Y Z) : Relation X Z :=
  fun x z => ∃ y, R x y ∧ Q y z

/-- The optimizer certificate is an equation in the ordinary relation algebra:
    Converse(U);V = F;Converse(G), for the graphs of the four maps. -/
theorem relational_certificate_iff {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z) :
    (∀ x y, compose (converse (graph u)) (graph v) x y ↔
      compose (graph f) (converse (graph g)) x y) ↔
      Commutes u v f g ∧ CompleteFibres u v f g := by
  constructor
  · intro equal
    constructor
    · intro w
      obtain ⟨z, hx, hy⟩ := (equal (u w) (v w)).mp ⟨w, rfl, rfl⟩
      exact hx.trans hy.symm
    · intro x y compatible
      exact (equal x y).mpr ⟨f x, rfl, compatible.symm⟩
  · rintro ⟨square, complete⟩ x y
    constructor
    · rintro ⟨w, hx, hy⟩
      change u w = x at hx
      change v w = y at hy
      have compatible := square w
      rw [hx, hy] at compatible
      exact ⟨f x, rfl, compatible.symm⟩
    · rintro ⟨z, hx, hy⟩
      exact complete x y (hx.trans hy.symm)

theorem automatic_inclusion {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z)
    (square : Commutes u v f g) (A : Region X) (y : Y) :
    Left u v A y → Right f g A y := by
  rintro ⟨w, ha, same⟩
  exact ⟨u w, ha, by simpa [same] using square w⟩

/-- Existence of every compatible lift suffices; uniqueness is unnecessary. -/
theorem exact_iff_complete_fibres {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z)
    (square : Commutes u v f g) :
    Exact u v f g ↔ CompleteFibres u v f g := by
  classical
  constructor
  · intro exact x y compatible
    let atom : Region X := fun z => decide (z = x)
    have witness : Right f g atom y := ⟨x, by simp [atom], compatible⟩
    obtain ⟨w, ha, same⟩ := (exact atom y).mpr witness
    exact ⟨w, by simpa [atom] using ha, same⟩
  · intro complete A y
    constructor
    · exact automatic_inclusion u v f g square A y
    · rintro ⟨x, ha, compatible⟩
      obtain ⟨w, hx, hy⟩ := complete x y compatible
      exact ⟨w, by simpa [hx] using ha, hy⟩

/-- The same square supports universal as well as existential elimination. -/
theorem universal_base_change {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z)
    (square : Commutes u v f g) (complete : CompleteFibres u v f g)
    (A : Region X) (y : Y) :
    (∀ w, v w = y → A (u w) = true) ↔
      (∀ x, f x = g y → A x = true) := by
  constructor
  · intro h x compatible
    obtain ⟨w, hx, hy⟩ := complete x y compatible
    simpa [hx] using h w hy
  · intro h w same
    exact h (u w) (by simpa [same] using square w)

/-- Preserve the nonempty-fibre guard as well, as required by nonvacuous Must. -/
theorem nonvacuous_base_change {W X Y Z : Type}
    (u : W → X) (v : W → Y) (f : X → Z) (g : Y → Z)
    (square : Commutes u v f g) (complete : CompleteFibres u v f g)
    (A : Region X) (y : Y) :
    ((∃ w, v w = y) ∧ ∀ w, v w = y → A (u w) = true) ↔
      ((∃ x, f x = g y) ∧ ∀ x, f x = g y → A x = true) := by
  constructor
  · rintro ⟨⟨w, same⟩, all⟩
    exact ⟨⟨u w, by simpa [same] using square w⟩,
      (universal_base_change u v f g square complete A y).mp all⟩
  · rintro ⟨⟨x, compatible⟩, all⟩
    obtain ⟨w, _, same⟩ := complete x y compatible
    exact ⟨⟨w, same⟩,
      (universal_base_change u v f g square complete A y).mpr all⟩

/-- Both projections of a copied-bit extension are surjective, but the square
    still lacks the off-diagonal fibres needed by the quantifier rewrite. -/
theorem copied_bit_counterexample :
    Commutes (id : Bool → Bool) id (fun _ => ()) (fun _ => ()) ∧
    (∀ x : Bool, ∃ w : Bool, id w = x) ∧
    (∀ y : Bool, ∃ w : Bool, id w = y) ∧
    Right (fun _ : Bool => ()) (fun _ : Bool => ()) id false ∧
    ¬ Left (id : Bool → Bool) id id false := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · intro w; rfl
  · intro x; exact ⟨x, rfl⟩
  · intro y; exact ⟨y, rfl⟩
  · exact ⟨true, rfl, rfl⟩
  · rintro ⟨w, ha, hy⟩
    simp only [id_eq] at ha hy
    rw [hy] at ha
    cases ha

/-- Complete existential fibres do not require the unique lifts of a set
    pullback. Their multiplicity may still matter to a designated law. -/
theorem complete_fibres_need_not_have_unique_lifts :
    CompleteFibres (fun _ : Bool => ()) (fun _ : Bool => ())
      (id : Unit → Unit) id ∧
    ∃ w₁ w₂ : Bool, w₁ ≠ w₂ ∧
      (fun _ : Bool => ()) w₁ = (fun _ : Bool => ()) w₂ ∧
      (fun _ : Bool => ()) w₁ = (fun _ : Bool => ()) w₂ := by
  constructor
  · intro x y _
    cases x; cases y
    exact ⟨false, rfl, rfl⟩
  · exact ⟨false, true, by decide, rfl, rfl⟩

#print axioms automatic_inclusion
#print axioms relational_certificate_iff
#print axioms exact_iff_complete_fibres
#print axioms universal_base_change
#print axioms nonvacuous_base_change
#print axioms copied_bit_counterexample
#print axioms complete_fibres_need_not_have_unique_lifts

end BaseChange
