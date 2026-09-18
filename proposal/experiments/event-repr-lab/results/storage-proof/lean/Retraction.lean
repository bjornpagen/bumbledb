import Std

namespace Retraction

structure Decoder (W : Type) (S : W → Prop) where
  run : W → W
  lands : ∀ w, S (run w)
  fixes : ∀ w, S w → run w = w

def encode {W : Type} {S : W → Prop} (d : Decoder W S) (A : W → Bool) :=
  fun w => A (d.run w)

theorem exact_identity {W : Type} {S : W → Prop} (d : Decoder W S)
    (A B : W → Bool) :
    (∀ w, encode d A w = encode d B w) ↔ (∀ w, S w → A w = B w) := by
  constructor
  · intro h w hw
    simpa only [encode, d.fixes w hw] using h w
  · intro h w
    exact h (d.run w) (d.lands w)

theorem boolean_homomorphism {W : Type} {S : W → Prop} (d : Decoder W S)
    (op : Bool → Bool → Bool) (A B : W → Bool) (w : W) :
    encode d (fun v => op (A v) (B v)) w = op (encode d A w) (encode d B w) := rfl

theorem complement_homomorphism {W : Type} {S : W → Prop} (d : Decoder W S)
    (A : W → Bool) (w : W) : encode d (fun v => !(A v)) w = !(encode d A w) := rfl

theorem normalization_idempotent {W : Type} {S : W → Prop} (d : Decoder W S)
    (A : W → Bool) (w : W) : encode d (encode d A) w = encode d A w := by
  simp only [encode, d.fixes (d.run w) (d.lands w)]

/-- A completed Event is exactly a predicate whose membership is functionally
    determined by the decoded world. This is the storage invariant as an FD. -/
theorem normalized_iff_fibre_dependency {W : Type} {S : W → Prop}
    (d : Decoder W S) (A : W → Bool) :
    (∀ w, encode d A w = A w) ↔
      (∀ x y, d.run x = d.run y → A x = A y) := by
  constructor
  · intro h x y same
    exact (h x).symm.trans ((congrArg A same).trans (h y))
  · intro h w
    exact h (d.run w) w (d.fixes (d.run w) (d.lands w))

theorem possibility_reflected {W : Type} {S : W → Prop} (d : Decoder W S)
    (A : W → Bool) : (∃ w, encode d A w = true) ↔ ∃ w, S w ∧ A w = true := by
  constructor
  · rintro ⟨w, hw⟩
    exact ⟨d.run w, d.lands w, hw⟩
  · rintro ⟨w, hs, hw⟩
    exact ⟨w, by simpa only [encode, d.fixes w hs] using hw⟩

theorem commuting_substitution {W : Type} {S : W → Prop} (d : Decoder W S)
    (f : W → W) (commutes : ∀ w, d.run (f w) = f (d.run w))
    (A : W → Bool) (w : W) :
    encode d (fun v => A (f v)) w = encode d A (f w) := by
  simp only [encode, commutes]

/-- A fixed physical code must decode to a fixed legal world if decoding is
    to commute with the map. This obstruction is independent of repair policy. -/
theorem fixed_point_obstruction {W : Type} {S : W → Prop} (f : W → W)
    (w : W) (fixed : f w = w)
    (noLegalFixed : ∀ s, S s → f s ≠ s) :
    ¬ ∃ d : Decoder W S, ∀ v, d.run (f v) = f (d.run v) := by
  rintro ⟨d, commutes⟩
  exact noLegalFixed (d.run w) (d.lands w)
    ((commutes w).symm.trans (congrArg d.run fixed))

theorem whole_face_exists {V : Type} (D : V → Prop) (d : Decoder V D)
    (A : V → Prop) : (∃ v, A (d.run v)) ↔ ∃ v, D v ∧ A v := by
  constructor
  · rintro ⟨v, h⟩
    exact ⟨d.run v, d.lands v, h⟩
  · rintro ⟨v, hd, h⟩
    exact ⟨v, by simpa only [d.fixes v hd] using h⟩

theorem whole_face_forall {V : Type} (D : V → Prop) (d : Decoder V D)
    (A : V → Prop) : (∀ v, A (d.run v)) ↔ ∀ v, D v → A v := by
  constructor
  · intro h v hd
    simpa only [d.fixes v hd] using h v
  · intro h v
    exact h (d.run v) (d.lands v)

/-- In each retained context, ignoring a complete decoded face is exactly
    independence of that face on legal worlds: the relation-role FD. -/
theorem face_dependency_reflected {C V : Type} (D : C → V → Prop)
    (d : ∀ c, Decoder V (D c)) (A : C → V → Bool) :
    (∀ c x y, A c ((d c).run x) = A c ((d c).run y)) ↔
      (∀ c x y, D c x → D c y → A c x = A c y) := by
  constructor
  · intro h c x y hx hy
    simpa only [(d c).fixes x hx, (d c).fixes y hy] using h c x y
  · intro h c x y
    exact h c ((d c).run x) ((d c).run y) ((d c).lands x) ((d c).lands y)

/-- For each retained environment, the same decoder is used on both endpoint
    faces and their middle witness. Environment decoding can precede this law. -/
theorem composition_preserved {V : Type} (D : V → Prop) (d : Decoder V D)
    (R Q : V → V → Prop) (x z : V) :
    (∃ y, R (d.run x) (d.run y) ∧ Q (d.run y) (d.run z)) ↔
    (D (d.run x) ∧ D (d.run z) ∧
      ∃ y, D y ∧ R (d.run x) y ∧ Q y (d.run z)) := by
  constructor
  · intro h
    exact ⟨d.lands x, d.lands z,
      (whole_face_exists D d (fun y => R (d.run x) y ∧ Q y (d.run z))).mp h⟩
  · intro h
    exact (whole_face_exists D d (fun y => R (d.run x) y ∧ Q y (d.run z))).mpr h.2.2

/-- The unit in the completed relation algebra is the decoder's kernel,
    which can relate unequal physical codes. -/
theorem decoded_identity_right {V : Type} (D : V → Prop) (d : Decoder V D)
    (R : V → V → Prop) (x z : V) :
    (∃ y, R (d.run x) (d.run y) ∧ d.run y = d.run z) ↔ R (d.run x) (d.run z) := by
  constructor
  · rintro ⟨y, h, equal⟩
    simpa only [equal] using h
  · intro h
    exact ⟨z, h, rfl⟩

abbrev Code := Bool × Bool -- low bit, high bit
def legal (w : Code) : Bool := !(w.1 && w.2)
def repair (w : Code) : Code := if legal w then w else (false, false)
def zero (w : Code) : Bool := !(w.1 || w.2)

theorem no_commuting_decoder_for_legal_pair :
    ¬ ∃ d : Decoder Code (fun w => w.1 ≠ w.2),
      ∀ w, d.run (w.2, w.1) = ((d.run w).2, (d.run w).1) := by
  apply fixed_point_obstruction (fun w : Code => (w.2, w.1)) (false, false) rfl
  intro s hs same
  exact hs (congrArg Prod.fst same).symm

theorem partial_projection_counterexample :
    (∃ low, zero (repair (low, true)) = true) ∧
      ¬ (∃ low, legal (low, true) = true ∧ zero (low, true) = true) := by decide

theorem alias_count_counterexample :
    let codes : List Code := [(false,false),(true,false),(false,true),(true,true)]
    (codes.filter (fun w => legal w && zero w)).length = 1 ∧
    (codes.filter (fun w => zero (repair w))).length = 2 := by decide

theorem encoded_unit_is_not_raw_equality :
    repair (true,true) = repair (false,false) ∧
      (true,true) ≠ (false,false) := by decide

#print axioms exact_identity
#print axioms boolean_homomorphism
#print axioms complement_homomorphism
#print axioms normalization_idempotent
#print axioms normalized_iff_fibre_dependency
#print axioms possibility_reflected
#print axioms commuting_substitution
#print axioms fixed_point_obstruction
#print axioms no_commuting_decoder_for_legal_pair
#print axioms whole_face_exists
#print axioms whole_face_forall
#print axioms face_dependency_reflected
#print axioms composition_preserved
#print axioms decoded_identity_right
#print axioms partial_projection_counterexample
#print axioms alias_count_counterexample
#print axioms encoded_unit_is_not_raw_equality

end Retraction
