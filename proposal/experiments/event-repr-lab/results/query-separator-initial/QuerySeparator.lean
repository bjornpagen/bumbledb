import Std

namespace QuerySeparator

def Agree {V D : Type} (s : V → Prop) (a b : V → D) := ∀ v, s v → a v = b v
def Local {V D : Type} (s : V → Prop) (p : (V → D) → Prop) :=
  ∀ a b, Agree s a b → (p a ↔ p b)
def patch {V D : Type} (left : V → Bool) (a b : V → D) : V → D :=
  fun v => if left v then a v else b v

theorem patch_left {V D : Type} (left : V → Bool) (a b : V → D) :
    Agree (fun v => left v = true) (patch left a b) a := by
  intro v h; simp [patch,h]

theorem patch_right {V D : Type} (left right : V → Bool) (keys : V → Prop)
    (a b reference : V → D)
    (overlap : ∀ v, left v = true → right v = true → keys v)
    (ha : Agree keys a reference) (hb : Agree keys b reference) :
    Agree (fun v => right v = true) (patch left a b) b := by
  intro v hr
  cases h : left v with
  | false => simp [patch,h]
  | true =>
    have hk := overlap v h hr
    simpa [patch,h] using (ha v hk).trans (hb v hk).symm

theorem patch_keys {V D : Type} (left : V → Bool) (keys : V → Prop)
    (a b reference : V → D) (ha : Agree keys a reference) (hb : Agree keys b reference) :
    Agree keys (patch left a b) reference := by
  intro v hk
  cases h : left v with
  | false => simpa [patch,h] using hb v hk
  | true => simpa [patch,h] using ha v hk

/-- A scalar separator allows compatible branch assignments to be glued.
    Predicate locality is the semantic premise supplied by variable incidence
    for a pure positive conjunctive query. No probability claim is involved. -/
theorem branch_existence {V D : Type} (left right : V → Bool) (keys : V → Prop)
    (p q : (V → D) → Prop) (reference : V → D)
    (lp : Local (fun v => left v = true) p)
    (lq : Local (fun v => right v = true) q)
    (overlap : ∀ v, left v = true → right v = true → keys v) :
    (∃ a, Agree keys a reference ∧ p a ∧ q a) ↔
      (∃ a, Agree keys a reference ∧ p a) ∧ (∃ b, Agree keys b reference ∧ q b) := by
  constructor
  · rintro ⟨a,hk,hp,hq⟩; exact ⟨⟨a,hk,hp⟩,⟨a,hk,hq⟩⟩
  · rintro ⟨⟨a,ha,hp⟩,⟨b,hb,hq⟩⟩
    exact ⟨patch left a b, patch_keys left keys a b reference ha hb,
      (lp _ _ (patch_left left a b)).mpr hp,
      (lq _ _ (patch_right left right keys a b reference overlap ha hb)).mpr hq⟩

/-- The universal residual aggregate preserves both quantifier directions. -/
theorem residual_pair {I J X Y Z : Type} (r : I → X → Y → Prop)
    (t : J → X → Z → Prop) (y : Y) (z : Z) :
    (∀ i j x, r i x y → t j x z) ↔
      (∀ x, (∃ i, r i x y) → ∀ j, t j x z) := by
  constructor
  · rintro h x ⟨i,hi⟩ j; exact h i j x hi
  · intro h i j x hi; exact h x ⟨i,hi⟩ j

/-- Individually possible branches in distinct environments cannot compose. -/
theorem environment_counterexample :
    (∃ e : Bool, e=false) ∧ (∃ e : Bool, e=true) ∧
      (¬ ∃ e : Bool, e=false ∧ e=true) := by decide

end QuerySeparator
#print axioms QuerySeparator.patch_left
#print axioms QuerySeparator.patch_right
#print axioms QuerySeparator.patch_keys
#print axioms QuerySeparator.branch_existence
#print axioms QuerySeparator.residual_pair
#print axioms QuerySeparator.environment_counterexample
