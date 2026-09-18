import Std

/-!
Denotational proof of the anchored representation. This does not establish
canonicality of a Rust node interner or verify the machine implementation.
-/
namespace Anchored

abbrev Region (Ω : Type) := Ω → Bool

def root {Ω : Type} (S A : Region Ω) (anchor : Ω) : Region Ω :=
  fun w => S w && Bool.xor (A w) (A anchor)

def decode {Ω : Type} (S r : Region Ω) (polarity : Bool) : Region Ω :=
  fun w => S w && Bool.xor (r w) polarity

def neg {Ω : Type} (S A : Region Ω) : Region Ω := fun w => S w && !(A w)

theorem root_excludes_anchor {Ω : Type} (S A : Region Ω) (anchor : Ω) :
    root S A anchor anchor = false := by
  cases h : A anchor <;> simp [root, h]

theorem root_excludes_outside {Ω : Type} (S A : Region Ω) (anchor w : Ω)
    (h : S w = false) : root S A anchor w = false := by
  simp [root, h]

theorem decode_root {Ω : Type} (S A : Region Ω) (anchor w : Ω) :
    decode S (root S A anchor) (A anchor) w = (S w && A w) := by
  cases hs : S w <;> cases ha : A w <;> cases hp : A anchor <;>
    simp [decode, root, hs, ha, hp]

theorem complement_same_root {Ω : Type} (S A : Region Ω) (anchor w : Ω)
    (h : S anchor = true) : root S (neg S A) anchor w = root S A anchor w := by
  cases hs : S w <;> cases ha : A w <;> cases hp : A anchor <;>
    simp [root, neg, h, hs, ha, hp]

theorem complement_flips_polarity {Ω : Type} (S A : Region Ω) (anchor : Ω)
    (h : S anchor = true) : neg S A anchor = !(A anchor) := by
  simp [neg, h]

/-- Equality of represented data is exactly equality on admitted support. -/
theorem exact_identity {Ω : Type} (S A B : Region Ω) (anchor : Ω)
    (h : S anchor = true) :
    ((∀ w, root S A anchor w = root S B anchor w) ∧ A anchor = B anchor) ↔
      (∀ w, S w = true → A w = B w) := by
  constructor
  · intro ⟨hr, hp⟩ w hs
    have hd : decode S (root S A anchor) (A anchor) w =
        decode S (root S B anchor) (B anchor) w := by
      simp only [decode, hr w, hp]
    simpa only [decode_root, hs, Bool.true_and] using hd
  · intro hab
    have hp := hab anchor h
    refine ⟨?_, hp⟩
    intro w
    cases hs : S w
    · simp [root, hs]
    · simp [root, hs, hab w hs, hp]

def adjusted (f : Bool → Bool → Bool) (p q : Bool) : Bool → Bool → Bool :=
  fun x y => Bool.xor (f (Bool.xor x p) (Bool.xor y q)) (f p q)

theorem adjusted_zero (f : Bool → Bool → Bool) (p q : Bool) :
    adjusted f p q false false = false := by
  simp [adjusted]

/-- One raw Apply suffices: its truth table is forced to map 00 to false. -/
theorem binary_apply_root {Ω : Type} (S A B : Region Ω) (anchor w : Ω)
    (f : Bool → Bool → Bool) :
    adjusted f (A anchor) (B anchor) (root S A anchor w) (root S B anchor w) =
      root S (fun x => f (A x) (B x)) anchor w := by
  cases hs : S w <;> cases ha : A w <;> cases hb : B w <;>
    cases hp : A anchor <;> cases hq : B anchor <;>
    simp [adjusted, root, hs, ha, hb, hp, hq]

#print axioms root_excludes_anchor
#print axioms root_excludes_outside
#print axioms decode_root
#print axioms complement_same_root
#print axioms complement_flips_polarity
#print axioms exact_identity
#print axioms adjusted_zero
#print axioms binary_apply_root

end Anchored
