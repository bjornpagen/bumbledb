import Std

/-!
Borrowed operands for relational product. W contains admitted joint worlds;
X and Y are operand presentations, T is the retained output presentation.
The joint image, rather than the separate marginal images, is the exact
structural contract. No measure, count, or Rust implementation is interpreted.
-/
namespace ViewProduct

abbrev Region (X : Type) := X → Bool
abbrev Compatibility (T X Y : Type) := T → X → Y → Prop

def Image {W T X Y : Type} (h : W → T) (u : W → X) (v : W → Y) :
    Compatibility T X Y :=
  fun t x y => ∃ w, h w = t ∧ u w = x ∧ v w = y

def Fused {W T X Y : Type} (h : W → T) (u : W → X) (v : W → Y)
    (A : Region X) (B : Region Y) (t : T) : Prop :=
  ∃ w, h w = t ∧ A (u w) = true ∧ B (v w) = true

def Contract {T X Y : Type} (C : Compatibility T X Y)
    (A : Region X) (B : Region Y) (t : T) : Prop :=
  ∃ x y, C t x y ∧ A x = true ∧ B y = true

/-- Read through both maps, intersect, and project, without constructing the
    renamed Events. This holds even for non-surjective or non-injective maps. -/
theorem contraction_through_image {W T X Y : Type}
    (h : W → T) (u : W → X) (v : W → Y)
    (A : Region X) (B : Region Y) (t : T) :
    Fused h u v A B t ↔ Contract (Image h u v) A B t := by
  constructor
  · rintro ⟨w, ht, ha, hb⟩
    exact ⟨u w, v w, ⟨w, ht, rfl, rfl⟩, ha, hb⟩
  · rintro ⟨x, y, ⟨w, ht, hx, hy⟩, ha, hb⟩
    exact ⟨w, ht, by simpa [hx] using ha, by simpa [hy] using hb⟩

/-- A substituted compatibility relation is exact for every operand pair iff
    it is the actual joint image, including the retained output coordinate. -/
theorem contraction_exact_iff_image {W T X Y : Type}
    (h : W → T) (u : W → X) (v : W → Y) (C : Compatibility T X Y) :
    (∀ A B t, Fused h u v A B t ↔ Contract C A B t) ↔
      (∀ t x y, Image h u v t x y ↔ C t x y) := by
  classical
  constructor
  · intro exact t x y
    let ax : Region X := fun z => decide (z = x)
    let byy : Region Y := fun z => decide (z = y)
    constructor
    · rintro ⟨w, ht, hx, hy⟩
      have fused : Fused h u v ax byy t :=
        ⟨w, ht, by simp [ax, hx], by simp [byy, hy]⟩
      obtain ⟨x', y', hc, ha, hb⟩ := (exact ax byy t).mp fused
      have xx : x' = x := by simpa [ax] using ha
      have yy : y' = y := by simpa [byy] using hb
      simpa [xx, yy] using hc
    · intro compatible
      have contracted : Contract C ax byy t :=
        ⟨x, y, compatible, by simp [ax], by simp [byy]⟩
      obtain ⟨w, ht, ha, hb⟩ := (exact ax byy t).mpr contracted
      exact ⟨w, ht, by simpa [ax] using ha, by simpa [byy] using hb⟩
  · intro same A B t
    rw [contraction_through_image]
    constructor
    · rintro ⟨x, y, hi, ha, hb⟩
      exact ⟨x, y, (same t x y).mp hi, ha, hb⟩
    · rintro ⟨x, y, hc, ha, hb⟩
      exact ⟨x, y, (same t x y).mpr hc, ha, hb⟩

/-- Ordinary relational composition is the full-product instance. The two
    views share Y; they must read the same Y witness. -/
theorem product_is_composition {X Y Z : Type}
    (R : Region (X × Y)) (Q : Region (Y × Z)) (x : X) (z : Z) :
    Fused (fun w : X × Y × Z => (w.1, w.2.2))
      (fun w => (w.1, w.2.1)) (fun w => w.2) R Q (x, z) ↔
      ∃ y, R (x, y) = true ∧ Q (y, z) = true := by
  constructor
  · rintro ⟨⟨x', y, z'⟩, same, hr, hq⟩
    have hx : x' = x := congrArg Prod.fst same
    have hz : z' = z := congrArg Prod.snd same
    exact ⟨y, by simpa [hx] using hr, by simpa [hz] using hq⟩
  · rintro ⟨y, hr, hq⟩
    exact ⟨(x, y, z), rfl, hr, hq⟩

/-- Surjective operand maps can still omit joint combinations. Replacing the
    actual joint image by the full product introduces a false witness. -/
theorem separate_surjectivity_is_insufficient :
    (∀ x : Bool, ∃ w : Bool, id w = x) ∧
    (∀ y : Bool, ∃ w : Bool, id w = y) ∧
    Contract (fun _ : Unit => fun _ _ : Bool => True) id Bool.not () ∧
    ¬ Fused (fun _ : Bool => ()) id id id Bool.not () := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro x; exact ⟨x, rfl⟩
  · intro y; exact ⟨y, rfl⟩
  · exact ⟨true, false, trivial, rfl, rfl⟩
  · rintro ⟨w, _, ha, hb⟩
    cases w <;> simp_all

/-- Identical operand predicates under different maps give different answers.
    Operand roots alone are therefore not a valid general product memo key. -/
theorem same_roots_different_maps :
    Fused (fun _ : Bool => ()) id id id id () ∧
    ¬ Fused (fun _ : Bool => ()) id Bool.not id id () := by
  constructor
  · exact ⟨true, rfl, rfl, rfl⟩
  · rintro ⟨w, _, ha, hb⟩
    cases w <;> simp_all

#print axioms contraction_through_image
#print axioms contraction_exact_iff_image
#print axioms product_is_composition
#print axioms separate_surjectivity_is_insufficient
#print axioms same_roots_different_maps

end ViewProduct
