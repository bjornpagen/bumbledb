import Std

/-!
Finite descriptor reconstruction obligations. Readouts determine a legal map
only under an injective legal target encoding. Import must re-establish support,
surjectivity, joint fibres and the actual environment readout. Orientation changes
roles; it cannot be discarded when the stored region bytes are unchanged.

The byte grammar/parser and Rust constructors must instantiate these premises.
These laws do not verify BEDC/BEVT serialization or native memory bounds.
-/
namespace Descriptors

def LegalMap {X Y : Type} (source : X → Prop) (target : Y → Prop) (f : X → Y) :=
  ∀ x, source x → target (f x)

def Onto {X Y : Type} (source : X → Prop) (target : Y → Prop) (f : X → Y) :=
  ∀ y, target y → ∃ x, source x ∧ f x = y

/-- The imported target code can have raw aliases outside legal support. -/
def LegalCode {Y C : Type} (target : Y → Prop) (code : Y → C) :=
  ∀ a, target a → ∀ b, target b → code a = code b → a = b

theorem readouts_reconstruct_the_legal_map {X Y C : Type}
    (source : X → Prop) (target : Y → Prop) (code : Y → C)
    (f g : X → Y) (encoding : LegalCode target code)
    (original : LegalMap source target f) (imported : LegalMap source target g)
    (readouts : ∀ x, source x → code (f x) = code (g x)) :
    ∀ x, source x → f x = g x := by
  intro x legal
  exact encoding (f x) (original x legal) (g x) (imported x legal) (readouts x legal)

theorem reconstruction_preserves_onto {X Y : Type}
    (source : X → Prop) (target : Y → Prop) (f g : X → Y)
    (same : ∀ x, source x → f x = g x) :
    Onto source target f ↔ Onto source target g := by
  constructor
  · intro onto y legal
    obtain ⟨x, hx, image⟩ := onto y legal
    exact ⟨x, hx, (same x hx).symm.trans image⟩
  · intro onto y legal
    obtain ⟨x, hx, image⟩ := onto y legal
    exact ⟨x, hx, (same x hx).trans image⟩

theorem reconstruction_preserves_pullback {X Y : Type}
    (source : X → Prop) (f g : X → Y) (same : ∀ x, source x → f x = g x)
    (region : Y → Prop) (x : X) (legal : source x) : region (f x) ↔ region (g x) := by
  rw [same x legal]

theorem reconstruction_preserves_image {X Y : Type}
    (source : X → Prop) (f g : X → Y) (same : ∀ x, source x → f x = g x)
    (region : X → Prop) (y : Y) :
    (∃ x, source x ∧ region x ∧ f x = y) ↔ (∃ x, source x ∧ region x ∧ g x = y) := by
  constructor
  · rintro ⟨x, hx, rx, image⟩; exact ⟨x, hx, rx, (same x hx).symm.trans image⟩
  · rintro ⟨x, hx, rx, image⟩; exact ⟨x, hx, rx, (same x hx).trans image⟩

/-- Without support admission, an illegal decoded output can have the same code.
Both readouts are equal; the maps themselves differ. -/
theorem target_support_check_cannot_be_replaced_by_code_equality :
    LegalCode (fun b : Bool => b = false) (fun _ => ()) ∧
    (fun _ : Unit => ()) = (fun _ : Unit => ()) ∧
    (fun _ : Unit => false) ≠ (fun _ : Unit => true) := by
  constructor
  · intro a ha b hb _; exact ha.trans hb.symm
  · constructor
    · rfl
    · intro equal; have := congrFun equal (); contradiction

def FullPair {A B E : Type} (left : A → Prop) (right : B → Prop)
    (aenv : A → E) (benv : B → E) (a : A) (b : B) :=
  left a ∧ right b ∧ aenv a = benv b

theorem original_support_and_environment_determine_product {A B E : Type}
    (left left' : A → Prop) (right right' : B → Prop)
    (aenv aenv' : A → E) (benv benv' : B → E)
    (la : ∀ a, left a ↔ left' a) (lb : ∀ b, right b ↔ right' b)
    (ea : ∀ a, left a → aenv a = aenv' a)
    (eb : ∀ b, right b → benv b = benv' b) (a : A) (b : B) :
    FullPair left right aenv benv a b ↔ FullPair left' right' aenv' benv' a b := by
  constructor
  · rintro ⟨ha, hb, same⟩
    exact ⟨(la a).mp ha, (lb b).mp hb, (ea a ha).symm.trans (same.trans (eb b hb))⟩
  · rintro ⟨ha, hb, same⟩
    have ha' := (la a).mpr ha
    have hb' := (lb b).mpr hb
    exact ⟨ha', hb', (ea a ha').trans (same.trans (eb b hb').symm)⟩

theorem converse_preserves_full_product {A B E : Type}
    (left : A → Prop) (right : B → Prop) (aenv : A → E) (benv : B → E)
    (a : A) (b : B) :
    FullPair left right aenv benv a b ↔ FullPair right left benv aenv b a := by
  constructor
  · rintro ⟨ha, hb, same⟩; exact ⟨hb, ha, same.symm⟩
  · rintro ⟨hb, ha, same⟩; exact ⟨ha, hb, same.symm⟩

def oriented (reversed : Bool) (region : Bool → Bool → Bool) (a b : Bool) :=
  if reversed then region b a else region a b

theorem forgetting_orientation_changes_meaning :
    oriented true (fun a b => a && !b) false true = true ∧
    oriented false (fun a b => a && !b) false true = false := by decide

theorem individual_onto_maps_do_not_certify_a_joint_square :
    Onto (fun _ : Bool => True) (fun _ : Bool => True) id ∧
    ¬ Onto (fun _ : Bool => True) (fun _ : Bool × Bool => True) (fun x => (x, x)) := by
  constructor
  · intro y _; exact ⟨y, trivial, rfl⟩
  · intro onto
    obtain ⟨x, _, image⟩ := onto (false, true) trivial
    have first := congrArg Prod.fst image
    have second := congrArg Prod.snd image
    exact Bool.noConfusion (first.symm.trans second)

theorem onto_environment_maps_can_still_disagree :
    Onto (fun _ : Bool => True) (fun _ : Bool => True) id ∧
    Onto (fun _ : Bool => True) (fun _ : Bool => True) Bool.not ∧
    (id : Bool → Bool) ≠ Bool.not := by
  constructor
  · intro y _; exact ⟨y, trivial, rfl⟩
  · constructor
    · intro y _; exact ⟨!y, trivial, Bool.not_not y⟩
    · intro equal; have := congrFun equal false; contradiction

theorem changed_environment_can_change_product_support :
    FullPair (fun _ : Bool => True) (fun _ : Bool => True) id id false false ∧
    ¬ FullPair (fun _ : Bool => True) (fun _ : Bool => True) id Bool.not false false := by
  simp [FullPair]

#print axioms Descriptors.readouts_reconstruct_the_legal_map
#print axioms Descriptors.reconstruction_preserves_onto
#print axioms Descriptors.reconstruction_preserves_pullback
#print axioms Descriptors.reconstruction_preserves_image
#print axioms Descriptors.target_support_check_cannot_be_replaced_by_code_equality
#print axioms Descriptors.original_support_and_environment_determine_product
#print axioms Descriptors.converse_preserves_full_product
#print axioms Descriptors.forgetting_orientation_changes_meaning
#print axioms Descriptors.individual_onto_maps_do_not_certify_a_joint_square
#print axioms Descriptors.onto_environment_maps_can_still_disagree
#print axioms Descriptors.changed_environment_can_change_product_support

end Descriptors
