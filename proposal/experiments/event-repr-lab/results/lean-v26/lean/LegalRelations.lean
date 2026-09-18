import Std

/-!
Ordinary relations over legal domains, possibly varying with a retained
environment. Neither the ambient bit cube nor a probability law is required.
A role certificate is separate: a binary relation cannot read a third face.
-/
namespace LegalRelations

variable {E V : Type}
abbrev Rel (E V : Type) := E → V → V → Prop
abbrev Event3 (E V : Type) := E → V → V → V → Prop

def Supported (D : E → V → Prop) (R : Rel E V) : Prop :=
  ∀ e x y, R e x y → D e x ∧ D e y
def support (D : E → V → Prop) (e : E) (x y z : V) : Prop :=
  D e x ∧ D e y ∧ D e z
def lift (D : E → V → Prop) (R : Rel E V) : Event3 E V :=
  fun e x y z => support D e x y z ∧ R e x y
def comp (D : E → V → Prop) (R Q : Rel E V) : Rel E V :=
  fun e x z => D e x ∧ D e z ∧ ∃ y, D e y ∧ R e x y ∧ Q e y z
def ident (D : E → V → Prop) : Rel E V := fun e x y => D e x ∧ x = y
def residual (D : E → V → Prop) (R T : Rel E V) : Rel E V :=
  fun e y z => D e y ∧ D e z ∧ ∀ x, D e x → R e x y → T e x z
def Le (R Q : Rel E V) : Prop := ∀ e x y, R e x y → Q e x y

/-- The same retained environment is read at every stage. Legal domains may
    vary with it, and may be empty or omit arbitrary presentation codes. -/
theorem composition_associative (D : E → V → Prop) (R Q T : Rel E V)
    (e : E) (x z : V) :
    comp D (comp D R Q) T e x z ↔ comp D R (comp D Q T) e x z := by
  constructor
  · rintro ⟨hx, hz, y, hy, ⟨_, _, m, hm, hr, hq⟩, ht⟩
    exact ⟨hx, hz, m, hm, hr, hm, hz, y, hy, hq, ht⟩
  · rintro ⟨hx, hz, m, hm, hr, ⟨_, _, y, hy, hq, ht⟩⟩
    exact ⟨hx, hz, y, hy, ⟨hx, hy, m, hm, hr, hq⟩, ht⟩

theorem identity_left (D : E → V → Prop) (R : Rel E V)
    (hr : Supported D R) (e : E) (x y : V) :
    comp D (ident D) R e x y ↔ R e x y := by
  constructor
  · rintro ⟨_, _, m, _, ⟨_, same⟩, h⟩
    simpa only [← same] using h
  · intro h
    obtain ⟨hx, hy⟩ := hr e x y h
    exact ⟨hx, hy, x, hx, ⟨hx, rfl⟩, h⟩

theorem identity_right (D : E → V → Prop) (R : Rel E V)
    (hr : Supported D R) (e : E) (x y : V) :
    comp D R (ident D) e x y ↔ R e x y := by
  constructor
  · rintro ⟨_, _, m, _, h, ⟨_, same⟩⟩
    simpa only [same] using h
  · intro h
    obtain ⟨hx, hy⟩ := hr e x y h
    exact ⟨hx, hy, y, hy, h, hy, rfl⟩

theorem residual_adjunction (D : E → V → Prop) (R Q T : Rel E V)
    (hq : Supported D Q) :
    Le (comp D R Q) T ↔ Le Q (residual D R T) := by
  constructor
  · intro h e y z q
    obtain ⟨hy, hz⟩ := hq e y z q
    exact ⟨hy, hz, fun x hx r => h e x z ⟨hx, hz, y, hy, r, q⟩⟩
  · intro h e x z composed
    obtain ⟨hx, _, y, _, r, q⟩ := composed
    exact (h e y z q).2.2 x hx r

/-- Literal support-clipped stages of the three-face lowering: join XY/YZ,
    hide Y, clip, then swap output Y/Z. -/
def staged (D : E → V → Prop) (R Q : Rel E V) : Event3 E V :=
  fun e x y z => support D e x y z ∧ support D e x z y ∧
    ∃ m, support D e x m y ∧ R e x m ∧ Q e m y

theorem staged_is_lifted_composition (D : E → V → Prop)
    (R Q : Rel E V) (e : E) (x y z : V) :
    staged D R Q e x y z ↔ lift D (comp D R Q) e x y z := by
  constructor
  · rintro ⟨⟨hx, hy, hz⟩, _, m, ⟨_, hm, _⟩, hr, hq⟩
    exact ⟨⟨hx, hy, hz⟩, hx, hy, m, hm, hr, hq⟩
  · rintro ⟨⟨hx, hy, hz⟩, _, _, m, hm, hr, hq⟩
    exact ⟨⟨hx, hy, hz⟩, ⟨hx, hz, hy⟩, m, ⟨hx, hm, hy⟩, hr, hq⟩

def Cylindrical (D : E → V → Prop) (F : Event3 E V) : Prop :=
  ∀ e x y z t, D e x → D e y → D e z → D e t →
    (F e x y z ↔ F e x y t)
def Supported3 (D : E → V → Prop) (F : Event3 E V) : Prop :=
  ∀ e x y z, F e x y z → support D e x y z
def forgetThird (D : E → V → Prop) (F : Event3 E V) : Event3 E V :=
  fun e x y z => support D e x y z ∧ ∃ t, D e t ∧ F e x y t

/-- The general role certificate is a functional dependency on a readout.
    Unlike the relation-composition laws, this equivalence needs no product
    support. It works with arbitrary coupled legal worlds. -/
theorem readout_fixed_iff_dependency {W X : Type} (S A : W → Prop)
    (q : W → X) (supported : ∀ w, A w → S w) :
    (∀ w, (S w ∧ ∃ v, S v ∧ q v = q w ∧ A v) ↔ A w) ↔
      (∀ w v, S w → S v → q w = q v → (A w ↔ A v)) := by
  constructor
  · intro fixed w v sw sv same
    constructor
    · intro aw; exact (fixed v).mp ⟨sv, w, sw, same, aw⟩
    · intro av; exact (fixed w).mp ⟨sw, v, sv, same.symm, av⟩
  · intro dependency w
    constructor
    · rintro ⟨sw, v, sv, same, av⟩
      exact (dependency v w sv sw same).mp av
    · intro aw
      have sw := supported w aw
      exact ⟨sw, w, sw, rfl, aw⟩

/-- Scoped Full passes every such role check, regardless of its underlying
    support. Checking it cannot certify that the support is a product. -/
theorem full_fixed_does_not_test_support {W X : Type} (S : W → Prop)
    (q : W → X) (w : W) :
    (S w ∧ ∃ v, S v ∧ q v = q w) ↔ S w := by
  constructor
  · exact And.left
  · intro sw; exact ⟨sw, w, sw, rfl⟩

/-- A scoped existential fixed-point check certifies the binary role using
    the existing Event algebra. It does not demand a raw mask excluding Z. -/
theorem exists_fixed_iff_cylindrical (D : E → V → Prop) (F : Event3 E V)
    (hs : Supported3 D F) :
    (∀ e x y z, forgetThird D F e x y z ↔ F e x y z) ↔ Cylindrical D F := by
  constructor
  · intro fixed e x y z t hx hy hz ht
    constructor
    · intro h
      exact (fixed e x y t).mp ⟨⟨hx, hy, ht⟩, z, hz, h⟩
    · intro h
      exact (fixed e x y z).mp ⟨⟨hx, hy, hz⟩, t, ht, h⟩
  · intro cylinder e x y z
    constructor
    · rintro ⟨⟨hx, hy, hz⟩, t, ht, h⟩
      exact (cylinder e x y t z hx hy ht hz).mp h
    · intro h
      have legal := hs e x y z h
      exact ⟨legal, z, legal.2.2, h⟩

def composeEvents (D : E → V → Prop) (F G : Event3 E V) : Event3 E V :=
  fun e x y z => support D e x y z ∧ support D e x z y ∧
    ∃ m, support D e x m y ∧ F e x m y ∧ G e m y x

theorem lifted_events_compose (D : E → V → Prop) (R Q : Rel E V)
    (e : E) (x y z : V) :
    composeEvents D (lift D R) (lift D Q) e x y z ↔
      lift D (comp D R Q) e x y z := by
  constructor
  · rintro ⟨⟨hx, hy, hz⟩, _, m, ⟨_, hm, _⟩, ⟨_, hr⟩, ⟨_, hq⟩⟩
    exact ⟨⟨hx, hy, hz⟩, hx, hy, m, hm, hr, hq⟩
  · rintro ⟨⟨hx, hy, hz⟩, _, _, m, hm, hr, hq⟩
    exact ⟨⟨hx, hy, hz⟩, ⟨hx, hz, hy⟩, m, ⟨hx, hm, hy⟩,
      ⟨⟨hx, hm, hy⟩, hr⟩, ⟨⟨hm, hy, hx⟩, hq⟩⟩

/-- With full Boolean support, treating F(x,y,z)=z as a binary XY relation
    makes F;Id read y. At this world the two disagree. -/
theorem ambient_support_does_not_certify_role :
    let D := fun (_ : Unit) (_ : Bool) => True
    let F := fun (_ : Unit) (_ _ z : Bool) => z = true
    composeEvents D F (lift D (ident D)) () false true false ∧
      ¬ F () false true false := by
  simp [composeEvents, support, lift, ident]

#print axioms composition_associative
#print axioms identity_left
#print axioms identity_right
#print axioms residual_adjunction
#print axioms staged_is_lifted_composition
#print axioms exists_fixed_iff_cylindrical
#print axioms readout_fixed_iff_dependency
#print axioms full_fixed_does_not_test_support
#print axioms lifted_events_compose
#print axioms ambient_support_does_not_certify_role

end LegalRelations
