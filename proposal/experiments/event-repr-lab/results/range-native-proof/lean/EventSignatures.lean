import Std

/-!
Support-relative Event signatures. These theorems describe denotations, not the
Rust arena implementation. Only Lean's installed standard library is required.
-/
namespace EventSignatures

abbrev Region (Ω : Type) := Ω → Bool
abbrev BinaryOp := Bool → Bool → Bool

def Occupied {Ω : Type} (S A B : Region Ω) (a b : Bool) : Prop :=
  ∃ w, S w = true ∧ A w = a ∧ B w = b

def Possible {Ω : Type} (S A : Region Ω) : Prop :=
  ∃ w, S w = true ∧ A w = true

def Full {Ω : Type} (S A : Region Ω) : Prop :=
  ∀ w, S w = true → A w = true

def Apply {Ω : Type} (op : BinaryOp) (A B : Region Ω) : Region Ω :=
  fun w => op (A w) (B w)

theorem possible_apply_iff {Ω : Type} (S A B : Region Ω) (op : BinaryOp) :
    Possible S (Apply op A B) ↔
      ∃ a b, Occupied S A B a b ∧ op a b = true := by
  constructor
  · intro ⟨w, hs, hop⟩
    exact ⟨A w, B w, ⟨w, hs, rfl, rfl⟩, hop⟩
  · intro ⟨a, b, ⟨w, hs, ha, hb⟩, hop⟩
    exact ⟨w, hs, by simpa [Apply, ha, hb] using hop⟩

theorem full_apply_iff {Ω : Type} (S A B : Region Ω) (op : BinaryOp) :
    Full S (Apply op A B) ↔
      ∀ a b, Occupied S A B a b → op a b = true := by
  constructor
  · intro h a b ⟨w, hs, ha, hb⟩
    simpa [Apply, ha, hb] using h w hs
  · intro h w hs
    exact h (A w) (B w) ⟨w, hs, rfl, rfl⟩

theorem signature_nonempty {Ω : Type} (S A B : Region Ω)
    (h : ∃ w, S w = true) : ∃ a b, Occupied S A B a b := by
  obtain ⟨w, hw⟩ := h
  exact ⟨A w, B w, w, hw, rfl, rfl⟩

theorem swap {Ω : Type} (S A B : Region Ω) (a b : Bool) :
    Occupied S B A a b ↔ Occupied S A B b a := by
  constructor <;> intro ⟨w, hs, ha, hb⟩ <;> exact ⟨w, hs, hb, ha⟩

theorem complement_left {Ω : Type} (S A B : Region Ω) (a b : Bool) :
    Occupied S (fun w => !(A w)) B a b ↔ Occupied S A B (!a) b := by
  simp only [Occupied]
  constructor
  · intro ⟨w, hs, ha, hb⟩
    exact ⟨w, hs, by cases a <;> cases h : A w <;> simp_all, hb⟩
  · intro ⟨w, hs, ha, hb⟩
    exact ⟨w, hs, by simp [ha], hb⟩

theorem complement_right {Ω : Type} (S A B : Region Ω) (a b : Bool) :
    Occupied S A (fun w => !(B w)) a b ↔ Occupied S A B a (!b) := by
  rw [swap, complement_left, swap]

def CellOp (a b : Bool) : BinaryOp := fun x y => decide (x = a ∧ y = b)

theorem possible_cell_iff {Ω : Type} (S A B : Region Ω) (a b : Bool) :
    Possible S (Apply (CellOp a b) A B) ↔ Occupied S A B a b := by
  simp [Possible, Apply, CellOp, Occupied]

/-- Exactly the information needed for possibility of every binary expression. -/
theorem signature_iff_all_binary_tests {Ω : Type} (S A B C D : Region Ω) :
    (∀ a b, Occupied S A B a b ↔ Occupied S C D a b) ↔
      (∀ op, Possible S (Apply op A B) ↔ Possible S (Apply op C D)) := by
  constructor
  · intro h op
    rw [possible_apply_iff, possible_apply_iff]
    constructor
    · intro ⟨a, b, hab, hop⟩
      exact ⟨a, b, (h a b).mp hab, hop⟩
    · intro ⟨a, b, hcd, hop⟩
      exact ⟨a, b, (h a b).mpr hcd, hop⟩
  · intro h a b
    simpa only [possible_cell_iff] using h (CellOp a b)

/- A finite counterexample to strong composition over the fifteen classes. -/
def support : Region (Fin 3) := fun _ => true
def singleton : Region (Fin 3) := fun w => decide (w.val = 0)
def doubleton : Region (Fin 3) := fun w => decide (w.val < 2)

def Matches (pattern : BinaryOp) (A B : Region (Fin 3)) : Prop :=
  ∀ a b, Occupied support A B a b ↔ pattern a b = true

-- Bits 0,2,3 and bits 0,1,3 respectively, using cell index 2*a+b.
def pattern13 : BinaryOp := fun a b => a || !b
def pattern11 : BinaryOp := fun a b => !a || b

def Composition (A C : Region (Fin 3)) : Prop :=
  ∃ B, Matches pattern13 A B ∧ Matches pattern11 B C

theorem same_endpoint_signature :
    ∀ a b, Occupied support singleton singleton a b ↔
      Occupied support doubleton doubleton a b := by
  unfold Occupied support singleton doubleton
  decide

theorem singleton_cannot_split : ¬Composition singleton singleton := by
  intro ⟨B, hab, _⟩
  obtain ⟨x, _, hx, hbx⟩ := (hab true false).mpr (by decide)
  obtain ⟨y, _, hy, hby⟩ := (hab true true).mpr (by decide)
  simp only [singleton, decide_eq_true_eq] at hx hy
  have hxy : x = y := Fin.ext (hx.trans hy.symm)
  subst y
  simp_all

theorem doubleton_can_split : Composition doubleton doubleton := by
  refine ⟨singleton, ?_, ?_⟩
  · unfold Matches Occupied support singleton doubleton pattern13
    decide
  · unfold Matches Occupied support singleton doubleton pattern11
    decide

/-- No decision based solely on the fifteen endpoint classes can be exact here. -/
theorem composition_distinguishes_same_signature :
    (∀ a b, Occupied support singleton singleton a b ↔
      Occupied support doubleton doubleton a b) ∧
    ¬Composition singleton singleton ∧ Composition doubleton doubleton :=
  ⟨same_endpoint_signature, singleton_cannot_split, doubleton_can_split⟩

/-- Even an arbitrary (possibly noncomputable) readout of the signature fails. -/
theorem no_signature_factorization :
    ¬∃ readout : (Bool → Bool → Prop) → Prop,
      ∀ A C, Composition A C ↔ readout (Occupied support A C) := by
  intro ⟨readout, h⟩
  have heq : Occupied support singleton singleton =
      Occupied support doubleton doubleton := by
    funext a b
    exact propext (same_endpoint_signature a b)
  have hlarge := (h doubleton doubleton).mp doubleton_can_split
  rw [← heq] at hlarge
  exact singleton_cannot_split ((h singleton singleton).mpr hlarge)

#print axioms possible_apply_iff
#print axioms full_apply_iff
#print axioms signature_nonempty
#print axioms swap
#print axioms complement_left
#print axioms complement_right
#print axioms signature_iff_all_binary_tests
#print axioms composition_distinguishes_same_signature
#print axioms no_signature_factorization

end EventSignatures
