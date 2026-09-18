import Std

/-!
Structural support maps and exact possibility readouts. These proofs do not
authorize quantifier base change or preservation of a designated probability law.
-/
namespace ReadoutMaps

abbrev Region (Ω : Type) := Ω → Bool
def Possible {Ω : Type} (S A : Region Ω) : Prop :=
  ∃ w, S w = true ∧ A w = true

def pull {X Y : Type} (f : X → Y) (A : Region Y) : Region X := fun x => A (f x)

def MapsSupport {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y) : Prop :=
  ∀ x, SX x = true → SY (f x) = true

def CoversSupport {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y) : Prop :=
  ∀ y, SY y = true → ∃ x, SX x = true ∧ f x = y

def PreservesPossibility {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y) : Prop :=
  ∀ A, Possible SX (pull f A) ↔ Possible SY A

/-- The support image must equal the target support, exactly. No cardinality or
    finiteness assumption is required; singleton predicates prove necessity. -/
theorem possibility_iff_exact_support_image {X Y : Type}
    (f : X → Y) (SX : Region X) (SY : Region Y) :
    PreservesPossibility f SX SY ↔ MapsSupport f SX SY ∧ CoversSupport f SX SY := by
  classical
  constructor
  · intro preserves
    constructor
    · intro x hx
      let atom : Region Y := fun y => decide (y = f x)
      have witness : Possible SX (pull f atom) := ⟨x, hx, by simp [pull, atom]⟩
      obtain ⟨y, hy, ha⟩ := (preserves atom).mp witness
      have same : y = f x := by simpa [atom] using ha
      simpa [same] using hy
    · intro y hy
      let atom : Region Y := fun z => decide (z = y)
      have witness : Possible SY atom := ⟨y, hy, by simp [atom]⟩
      obtain ⟨x, hx, ha⟩ := (preserves atom).mpr witness
      exact ⟨x, hx, by simpa [pull, atom] using ha⟩
  · rintro ⟨maps, covers⟩ A
    constructor
    · rintro ⟨x, hx, ha⟩
      exact ⟨f x, maps x hx, ha⟩
    · rintro ⟨y, hy, ha⟩
      obtain ⟨x, hx, same⟩ := covers y hy
      exact ⟨x, hx, by simpa [pull, same] using ha⟩

def Cell {Ω : Type} (S A B : Region Ω) (a b : Bool) : Prop :=
  ∃ w, S w = true ∧ A w = a ∧ B w = b

theorem occupancy_preserved {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y)
    (maps : MapsSupport f SX SY) (covers : CoversSupport f SX SY)
    (A B : Region Y) (a b : Bool) :
    Cell SX (pull f A) (pull f B) a b ↔ Cell SY A B a b := by
  constructor
  · rintro ⟨x, hx, ha, hb⟩
    exact ⟨f x, maps x hx, ha, hb⟩
  · rintro ⟨y, hy, ha, hb⟩
    obtain ⟨x, hx, same⟩ := covers y hy
    exact ⟨x, hx, by simpa [pull, same] using ha, by simpa [pull, same] using hb⟩

theorem equality_reflected {X Y : Type} (f : X → Y) (SX : Region X) (SY : Region Y)
    (maps : MapsSupport f SX SY) (covers : CoversSupport f SX SY) (A B : Region Y) :
    (∀ x, SX x = true → pull f A x = pull f B x) ↔
      (∀ y, SY y = true → A y = B y) := by
  constructor
  · intro h y hy
    obtain ⟨x, hx, same⟩ := covers y hy
    simpa [pull, same] using h x hx
  · intro h x hx
    exact h (f x) (maps x hx)

def neg {Ω : Type} (S A : Region Ω) : Region Ω := fun w => S w && !(A w)
def lift {X Y : Type} (f : X → Y) (SX : Region X) (A : Region Y) : Region X :=
  fun x => SX x && A (f x)

/-- Totality on admitted support suffices for complement; no surjectivity is
    needed unless equality/possibility must also be reflected. -/
theorem relative_complement_preserved {X Y : Type}
    (f : X → Y) (SX : Region X) (SY : Region Y) (maps : MapsSupport f SX SY)
    (A : Region Y) : neg SX (lift f SX A) = lift f SX (neg SY A) := by
  funext x
  cases hx : SX x
  · simp [neg, lift, hx]
  · simp [neg, lift, hx, maps x hx]

/-- Fixing a bit is a legal restriction, but can delete an old witness. -/
theorem fixing_coordinate_can_remove_possibility :
    Possible (fun _ : Bool => true) (fun b => b) ∧
    ¬ Possible (fun _ : Unit => true) (pull (fun _ => false) (fun b => b)) := by
  constructor
  · exact ⟨true, rfl, rfl⟩
  · rintro ⟨x, _, h⟩
    simp [pull] at h

#print axioms possibility_iff_exact_support_image
#print axioms occupancy_preserved
#print axioms equality_reflected
#print axioms relative_complement_preserved
#print axioms fixing_coordinate_can_remove_possibility

end ReadoutMaps
