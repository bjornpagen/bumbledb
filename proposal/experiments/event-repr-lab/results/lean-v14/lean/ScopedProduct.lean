import Std

/-!
Exact fusion of staged support-relative operations, including arbitrary support
and non-preserving coordinate bijections. This does not grant relation-algebra
laws to a coupled triple support or certify probability-law transport.
-/
namespace ScopedProduct

abbrev Predicate (W : Type) := W → Prop
def meet {W : Type} (A B : Predicate W) : Predicate W := fun w => A w ∧ B w
def pull {W : Type} (m : W → W) (A : Predicate W) : Predicate W := fun w => A (m w)
def eliminate {W : Type} (R : W → W → Prop) (A : Predicate W) : Predicate W :=
  fun t => ∃ w, R t w ∧ A w

def staged {W : Type} (S A B : Predicate W) (a b o : W → W)
    (R : W → W → Prop) : Predicate W :=
  meet S (pull o (meet S (eliminate R
    (meet (meet S (pull a (meet S A))) (meet S (pull b (meet S B)))))))

def guarded {W : Type} (S A B : Predicate W) (a ai b o : W → W)
    (R : W → W → Prop) : Predicate W :=
  meet S (meet (pull o S) (pull o (eliminate R
    (meet (pull a (meet (meet S A) (pull ai S))) (pull b (meet S B))))))

/-- Absorb the common witness support into one operand through its inverse
    map, and retain the post-projection support in output coordinates. -/
theorem guarded_equals_staged {W : Type} (S A B : Predicate W)
    (a ai b o : W → W) (left_inverse : ∀ w, ai (a w) = w)
    (R : W → W → Prop) (t : W) :
    staged S A B a b o R t ↔ guarded S A B a ai b o R t := by
  simp only [staged, guarded, meet, pull, eliminate, left_inverse]
  constructor
  · rintro ⟨st, sot, w, hr, ⟨sw, sa, ha⟩, ⟨_, sb, hb⟩⟩
    exact ⟨st, sot, w, hr, ⟨⟨sa, ha⟩, sw⟩, sb, hb⟩
  · rintro ⟨st, sot, w, hr, ⟨⟨sa, ha⟩, sw⟩, sb, hb⟩
    exact ⟨st, sot, w, hr, ⟨sw, sa, ha⟩, sw, sb, hb⟩

theorem full_support_specialization {W : Type} (A B : Predicate W)
    (a ai b o : W → W) (R : W → W → Prop) (t : W) :
    guarded (fun _ => True) A B a ai b o R t ↔
      ∃ w, R (o t) w ∧ A (a w) ∧ B (b w) := by
  simp [guarded, meet, pull, eliminate]

/-- A checked support-reflecting input read map and support-preserving output
    read map make the intermediate gates redundant. Full binary support is
    sufficient but unnecessary. One input supplies the common witness gate. -/
theorem support_certificates_remove_gates {W : Type} (S A B : Predicate W)
    (a b o : W → W) (R : W → W → Prop)
    (input_reflects : ∀ w, S (a w) → S w)
    (output_preserves : ∀ t, S t → S (o t)) (t : W) :
    staged S A B a b o R t ↔
      meet S (pull o (eliminate R
        (meet (pull a (meet S A)) (pull b (meet S B))))) t := by
  constructor
  · rintro ⟨st, _, w, hr, ⟨_, sa, ha⟩, ⟨_, sb, hb⟩⟩
    exact ⟨st, w, hr, ⟨sa, ha⟩, sb, hb⟩
  · rintro ⟨st, w, hr, ⟨sa, ha⟩, sb, hb⟩
    have sw := input_reflects w sa
    exact ⟨st, output_preserves t st, w, hr, ⟨sw, sa, ha⟩, sw, sb, hb⟩

private def one (w : Bool × Bool) : Prop := w = (true, false)
private def swap (w : Bool × Bool) : Bool × Bool := (w.2, w.1)

/-- Omitting clipping before output renaming can resurrect a forbidden world. -/
theorem output_support_is_necessary :
    ¬ staged one one one id id swap (fun _ _ => True) (true, false) ∧
    (meet one (pull swap (eliminate (fun _ _ => True)
      (meet (meet one one) (meet one one))))) (true, false) := by
  constructor
  · intro h
    have bad := h.2.1
    change (false, true) = (true, false) at bad
    cases bad
  · exact ⟨rfl, (true, false), trivial, ⟨rfl, rfl⟩, rfl, rfl⟩

/-- Omitting the common witness support admits an outside-support witness. -/
theorem witness_support_is_necessary :
    ¬ staged one one one swap swap id (fun _ _ => True) (true, false) ∧
    (meet one (meet one (eliminate (fun _ _ => True)
      (meet (pull swap (meet one one)) (pull swap (meet one one)))))) (true, false) := by
  constructor
  · rintro ⟨_, _, w, _, ⟨hw, hs, _⟩, _⟩
    change w = (true, false) at hw
    subst w
    change (false, true) = (true, false) at hs
    cases hs
  · exact ⟨rfl, rfl, (false, true), trivial, ⟨rfl, rfl⟩, rfl, rfl⟩

#print axioms guarded_equals_staged
#print axioms full_support_specialization
#print axioms support_certificates_remove_gates
#print axioms output_support_is_necessary
#print axioms witness_support_is_necessary

end ScopedProduct
