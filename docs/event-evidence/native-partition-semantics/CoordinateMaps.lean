import Std

/-!
The native map contract: total legal-world functions, Boolean substitution,
existential/universal image, and early source-coordinate abstraction. The raw
support check precedes target completion. Legal types below contain only admitted
worlds; `checksSupport` separately describes their admission premise.

These are denotational and reference-kernel proofs, not a refinement of Rust's
arena, traversal, locks, memo limits, or byte representation. No measure occurs.
-/
namespace CoordinateMaps

abbrev Region (X : Type) := X → Prop
def incl {X : Type} (A B : Region X) : Prop := ∀ x, A x → B x
def pull {X Y : Type} (f : X → Y) (B : Region Y) : Region X := fun x => B (f x)
def image {X Y : Type} (f : X → Y) (A : Region X) : Region Y :=
  fun y => ∃ x, A x ∧ f x = y
def allImage {X Y : Type} (f : X → Y) (A : Region X) : Region Y :=
  fun y => ∀ x, f x = y → A x
def mustImage {X Y : Type} (f : X → Y) (A : Region X) : Region Y :=
  fun y => (∃ x, f x = y) ∧ allImage f A y
def onto {X Y : Type} (f : X → Y) : Prop := ∀ y, ∃ x, f x = y

def checksSupport {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y) : Prop :=
  ¬ ∃ x, SX x ∧ ¬ SY (f x)

theorem support_check_exact {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y) :
    checksSupport f SX SY ↔ ∀ x, SX x → SY (f x) := by
  classical
  simp [checksSupport]

theorem pull_boolean {X Y : Type} (f : X → Y) (a b : Y → Bool)
    (operation : Bool → Bool → Bool) (x : X) :
    operation (a (f x)) (b (f x)) = (fun y => operation (a y) (b y)) (f x) := rfl

theorem pull_complement {X Y : Type} (f : X → Y) (B : Region Y) :
    pull f (fun y => ¬ B y) = fun x => ¬ pull f B x := rfl

theorem pull_composition {X Y Z : Type} (f : X → Y) (g : Y → Z) (C : Region Z) :
    pull (g ∘ f) C = pull f (pull g C) := rfl

theorem image_left_adjoint {X Y : Type} (f : X → Y) (A : Region X) (B : Region Y) :
    incl (image f A) B ↔ incl A (pull f B) := by
  constructor
  · intro h x hx
    exact h (f x) ⟨x, hx, rfl⟩
  · intro h y ⟨x, hx, same⟩
    exact same ▸ h x hx

theorem universal_right_adjoint {X Y : Type} (f : X → Y) (A : Region X) (B : Region Y) :
    incl (pull f B) A ↔ incl B (allImage f A) := by
  constructor
  · intro h y hy x same
    exact h x (by simpa [pull, same] using hy)
  · intro h x hx
    exact h (f x) hx x rfl

theorem image_union {X Y : Type} (f : X → Y) (A B : Region X) (y : Y) :
    image f (fun x => A x ∨ B x) y ↔ image f A y ∨ image f B y := by
  constructor
  · rintro ⟨x, (ha | hb), same⟩
    · exact Or.inl ⟨x, ha, same⟩
    · exact Or.inr ⟨x, hb, same⟩
  · rintro (⟨x, ha, same⟩ | ⟨x, hb, same⟩)
    · exact ⟨x, Or.inl ha, same⟩
    · exact ⟨x, Or.inr hb, same⟩

theorem image_composition {X Y Z : Type} (f : X → Y) (g : Y → Z)
    (A : Region X) (z : Z) : image (g ∘ f) A z ↔ image g (image f A) z := by
  constructor
  · rintro ⟨x, hx, same⟩
    exact ⟨f x, ⟨x, hx, rfl⟩, same⟩
  · rintro ⟨y, ⟨x, hx, same⟩, target⟩
    exact ⟨x, hx, by simpa [Function.comp_def, same] using target⟩

theorem image_pull_intersection {X Y : Type} (f : X → Y)
    (A : Region X) (B : Region Y) (y : Y) :
    image f (fun x => A x ∧ pull f B x) y ↔ image f A y ∧ B y := by
  constructor
  · rintro ⟨x, ⟨ha, hb⟩, same⟩
    exact ⟨⟨x, ha, same⟩, same ▸ hb⟩
  · rintro ⟨⟨x, ha, same⟩, hb⟩
    exact ⟨x, ⟨ha, by simpa [pull, same] using hb⟩, same⟩

theorem universal_duality {X Y : Type} (f : X → Y) (A : Region X) (y : Y) :
    allImage f A y ↔ ¬ image f (fun x => ¬ A x) y := by
  classical
  constructor
  · intro h ⟨x, no, same⟩
    exact no (h x same)
  · intro h x same
    apply Classical.byContradiction
    intro no
    exact h ⟨x, no, same⟩

theorem must_is_range_and_all {X Y : Type} (f : X → Y) (A : Region X) (y : Y) :
    mustImage f A y ↔ image f (fun _ => True) y ∧ allImage f A y := by
  simp [mustImage, image]

theorem full_image_iff_onto {X Y : Type} (f : X → Y) :
    (∀ y, image f (fun _ => True) y) ↔ onto f := by
  simp [image, onto]

theorem onto_reflects_inclusion {X Y : Type} (f : X → Y) (surjective : onto f)
    (A B : Region Y) : incl (pull f A) (pull f B) ↔ incl A B := by
  constructor
  · intro h y hy
    obtain ⟨x, same⟩ := surjective y
    have hx : pull f A x := by simpa [pull, same] using hy
    simpa [pull, same] using h x hx
  · intro h x hx
    exact h (f x) hx

theorem inclusion_reflection_requires_onto {X Y : Type} (f : X → Y)
    (reflects : ∀ A B : Region Y, incl (pull f A) (pull f B) → incl A B) : onto f := by
  intro y
  have result := reflects (fun _ => True) (fun y => ∃ x, f x = y)
    (fun x _ => ⟨x, rfl⟩) y trivial
  exact result

theorem onto_preserves_occupancy {X Y : Type} (f : X → Y) (surjective : onto f)
    (A B : Y → Bool) (a b : Bool) :
    (∃ x, A (f x) = a ∧ B (f x) = b) ↔ (∃ y, A y = a ∧ B y = b) := by
  constructor
  · rintro ⟨x, ha, hb⟩
    exact ⟨f x, ha, hb⟩
  · rintro ⟨y, ha, hb⟩
    obtain ⟨x, same⟩ := surjective y
    exact ⟨x, same.symm ▸ ha, same.symm ▸ hb⟩

/- A small substitution reference: readouts replace variables simultaneously.
   Native local tables and split nodes must refine this same evaluation law. -/
inductive Formula (V : Type) where
  | constant : Bool → Formula V
  | bit : V → Formula V
  | ite : Formula V → Formula V → Formula V → Formula V

def eval {V : Type} (assignment : V → Bool) : Formula V → Bool
  | .constant b => b
  | .bit v => assignment v
  | .ite c h l => if eval assignment c then eval assignment h else eval assignment l

def substitute {U V : Type} (readout : V → Formula U) : Formula V → Formula U
  | .constant b => .constant b
  | .bit v => readout v
  | .ite c h l => .ite (substitute readout c) (substitute readout h) (substitute readout l)

theorem substitution_exact {U V : Type} (readout : V → Formula U)
    (assignment : U → Bool) (formula : Formula V) :
    eval assignment (substitute readout formula) =
      eval (fun v => eval assignment (readout v)) formula := by
  induction formula with
  | constant b => rfl
  | bit v => rfl
  | ite c h l ic ih il => simp only [substitute, eval, ic, ih, il]

/- `same x w` identifies worlds agreeing on the retained source coordinates.
   Input A has ALREADY been masked by original support. Intermediate raw worlds
   may be decoder aliases, but the theorem retains an actual A-witness. -/
def abstract {X : Type} (same : X → X → Prop) (A : Region X) : Region X :=
  fun x => ∃ w, same x w ∧ A w

theorem early_abstraction_preserves_image {X Y : Type} (f : X → Y)
    (same : X → X → Prop) (reflexive : ∀ x, same x x)
    (readoutIndependent : ∀ x w, same x w → f x = f w)
    (A : Region X) (y : Y) : image f (abstract same A) y ↔ image f A y := by
  constructor
  · rintro ⟨x, ⟨w, equivalent, present⟩, output⟩
    exact ⟨w, present, (readoutIndependent x w equivalent).symm.trans output⟩
  · rintro ⟨x, present, output⟩
    exact ⟨x, ⟨x, reflexive x, present⟩, output⟩

theorem abstraction_moves_past_future_readout {X : Type} (same : X → X → Prop)
    (A Q : Region X) (independent : ∀ x w, same x w → (Q x ↔ Q w)) (x : X) :
    abstract same (fun w => A w ∧ Q w) x ↔ abstract same A x ∧ Q x := by
  constructor
  · rintro ⟨w, equivalent, ha, hq⟩
    exact ⟨⟨w, equivalent, ha⟩, (independent x w equivalent).mpr hq⟩
  · rintro ⟨⟨w, equivalent, ha⟩, hq⟩
    exact ⟨w, equivalent, ha, (independent x w equivalent).mp hq⟩

/-- Recursive image splitting on a list of Boolean readouts. Coordinates can
    occur in any chosen order; arbitrary readouts can be repeated or constant. -/
def splitImage {X : Type} : List (X → Bool) → Region X → List Bool → Prop
  | [], A, values => values = [] ∧ ∃ x, A x
  | _ :: _, _, [] => False
  | bit :: rest, A, value :: values =>
      splitImage rest (fun x => A x ∧ bit x = value) values

theorem recursive_image_exact {X : Type} (readouts : List (X → Bool))
    (A : Region X) (values : List Bool) :
    splitImage readouts A values ↔ ∃ x, A x ∧ readouts.map (fun bit => bit x) = values := by
  induction readouts generalizing A values with
  | nil =>
    simp only [splitImage, List.map_nil]
    constructor
    · rintro ⟨rfl, x, hx⟩
      exact ⟨x, hx, rfl⟩
    · rintro ⟨x, hx, same⟩
      exact ⟨same.symm, x, hx⟩
  | cons bit rest ih =>
    cases values with
    | nil => simp [splitImage]
    | cons value values =>
      simp only [splitImage, ih, List.map_cons, List.cons.injEq]
      constructor
      · rintro ⟨x, ⟨hx, hb⟩, tail⟩
        exact ⟨x, hx, hb, tail⟩
      · rintro ⟨x, hx, hb, tail⟩
        exact ⟨x, ⟨hx, hb⟩, tail⟩

/-- This is the last-use rule used by the native image kernel: after splitting,
    abstract only coordinates on which every remaining readout is constant. -/
theorem recursive_image_early_abstraction {X : Type} (readouts : List (X → Bool))
    (same : X → X → Prop) (reflexive : ∀ x, same x x)
    (independent : ∀ x w, same x w →
      readouts.map (fun bit => bit x) = readouts.map (fun bit => bit w))
    (A : Region X) (values : List Bool) :
    splitImage readouts (abstract same A) values ↔ splitImage readouts A values := by
  rw [recursive_image_exact, recursive_image_exact]
  exact early_abstraction_preserves_image
    (fun x => readouts.map (fun bit => bit x)) same reflexive independent A values

/- The four-cell examples prevent tempting but invalid rewrites. -/
theorem image_does_not_preserve_intersection :
    let f : Bool → Unit := fun _ => ()
    image f (fun b => b = true) () ∧ image f (fun b => b = false) () ∧
    ¬ image f (fun b => b = true ∧ b = false) () := by
  refine ⟨⟨true, rfl, rfl⟩, ⟨false, rfl, rfl⟩, ?_⟩
  rintro ⟨b, ⟨ht, hf⟩, _⟩
  cases ht.symm.trans hf

theorem unreachable_fibres_separate_all_and_must :
    let f : Unit → Bool := fun _ => false
    allImage f (fun _ => False) true ∧ ¬ mustImage f (fun _ => False) true := by
  simp [allImage, mustImage]

theorem restriction_can_erase_possibility :
    (∃ b : Bool, b = true) ∧ ¬ ∃ u : Unit, pull (fun _ => false) (fun b => b = true) u := by
  simp [pull]

theorem completed_target_full_cannot_check_support :
    let f : Unit → Bool := fun _ => false
    let targetSupport : Region Bool := fun b => b = true
    (∀ u : Unit, (fun _ : Bool => True) (f u)) ∧
      ¬ checksSupport f (fun _ => True) targetSupport := by
  simp [checksSupport]

theorem aliases_cannot_supply_legal_image_witnesses :
    image (id : Bool → Bool) (fun _ => True) false ∧
    ¬ image (id : Bool → Bool) (fun b => b = true ∧ True) false := by
  simp [image]

theorem forgetting_a_future_readout_changes_image :
    let same : Bool → Bool → Prop := fun _ _ => True
    image (id : Bool → Bool) (abstract same (fun b => b = true)) false ∧
    ¬ image (id : Bool → Bool) (fun b => b = true) false := by
  simp [image, abstract]

#print axioms CoordinateMaps.support_check_exact
#print axioms CoordinateMaps.pull_boolean
#print axioms CoordinateMaps.pull_complement
#print axioms CoordinateMaps.pull_composition
#print axioms CoordinateMaps.image_left_adjoint
#print axioms CoordinateMaps.universal_right_adjoint
#print axioms CoordinateMaps.image_union
#print axioms CoordinateMaps.image_composition
#print axioms CoordinateMaps.image_pull_intersection
#print axioms CoordinateMaps.universal_duality
#print axioms CoordinateMaps.must_is_range_and_all
#print axioms CoordinateMaps.full_image_iff_onto
#print axioms CoordinateMaps.onto_reflects_inclusion
#print axioms CoordinateMaps.inclusion_reflection_requires_onto
#print axioms CoordinateMaps.onto_preserves_occupancy
#print axioms CoordinateMaps.substitution_exact
#print axioms CoordinateMaps.early_abstraction_preserves_image
#print axioms CoordinateMaps.abstraction_moves_past_future_readout
#print axioms CoordinateMaps.recursive_image_exact
#print axioms CoordinateMaps.recursive_image_early_abstraction
#print axioms CoordinateMaps.image_does_not_preserve_intersection
#print axioms CoordinateMaps.unreachable_fibres_separate_all_and_must
#print axioms CoordinateMaps.restriction_can_erase_possibility
#print axioms CoordinateMaps.completed_target_full_cannot_check_support
#print axioms CoordinateMaps.aliases_cannot_supply_legal_image_witnesses
#print axioms CoordinateMaps.forgetting_a_future_readout_changes_image

end CoordinateMaps
