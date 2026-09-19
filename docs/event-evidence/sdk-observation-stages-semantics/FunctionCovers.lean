import Std

/-!
Local function gluing: coverage and pointwise compatibility are the functional
dependency of an observable on its evidence region. This reference is independent
of probability, so zero-mass worlds receive the same obligations. It does not
verify Rust masks, exact-family equality, allocation, transport or Free Join.
-/
namespace FunctionCovers

structure Patch (W V : Type) where
  region : W → Prop
  value : W → V

def Covers {W V : Type} (parent : W → Prop) (patches : List (Patch W V)) : Prop :=
  ∀ w, parent w → ∃ p, p ∈ patches ∧ p.region w
def Compatible {W V : Type} (parent : W → Prop) (patches : List (Patch W V)) : Prop :=
  ∀ a, a ∈ patches → ∀ b, b ∈ patches → ∀ w,
    parent w → a.region w → b.region w → a.value w=b.value w
def Represents {W V : Type} (parent : W → Prop) (patches : List (Patch W V)) (f : W → V) : Prop :=
  ∀ p, p ∈ patches → ∀ w, parent w → p.region w → f w=p.value w

noncomputable def mask {W V : Type} (zero : V) (parent : W → Prop) (f : W → V) : W → V := by
  classical
  exact fun w => if parent w then f w else zero

theorem mask_inside {W V : Type} (z : V) (p : W → Prop) (f : W → V) (w : W) (h : p w) :
    mask z p f w=f w := by simp [mask,h]
theorem mask_outside {W V : Type} (z : V) (p : W → Prop) (f : W → V) (w : W) (h : ¬p w) :
    mask z p f w=z := by simp [mask,h]
theorem mask_idempotent {W V : Type} (z : V) (p : W → Prop) (f : W → V) :
    mask z p (mask z p f) = mask z p f := by
  funext w
  classical
  by_cases h : p w <;> simp [mask,h]

theorem empty_cover_iff {W V : Type} (parent : W → Prop) :
    Covers parent ([] : List (Patch W V)) ↔ ∀ w, ¬parent w := by
  simp [Covers]

theorem zero_is_not_missing_coverage : ¬Covers (fun (_ : Unit) => True) ([] : List (Patch Unit Nat)) := by
  simp [Covers]

theorem representation_requires_agreement {W V : Type} (parent : W → Prop)
    (patches : List (Patch W V)) (f : W → V) (rep : Represents parent patches f) :
    Compatible parent patches := by
  intro a ha b hb w hp hwa hwb
  exact (rep a ha w hp hwa).symm.trans (rep b hb w hp hwb)

theorem unique_on_parent {W V : Type} (parent : W → Prop) (patches : List (Patch W V))
    (covered : Covers parent patches) (f g : W → V)
    (hf : Represents parent patches f) (hg : Represents parent patches g) :
    ∀ w, parent w → f w=g w := by
  intro w hp
  obtain ⟨p, member, active⟩ := covered w hp
  exact (hf p member w hp active).trans (hg p member w hp active).symm

noncomputable def glue {W V : Type} (zero : V) (parent : W → Prop) (patches : List (Patch W V)) : W → V := by
  classical
  exact fun w => if h : parent w ∧ ∃ p, p ∈ patches ∧ p.region w then
    (Classical.choose h.2).value w else zero

theorem glue_represents {W V : Type} (z : V) (parent : W → Prop) (patches : List (Patch W V))
    (compatible : Compatible parent patches) : Represents parent patches (glue z parent patches) := by
  intro p member w inside active
  have h : parent w ∧ ∃ p, p ∈ patches ∧ p.region w := ⟨inside,p,member,active⟩
  simp only [glue, dif_pos h]
  have selected := Classical.choose_spec h.2
  exact compatible _ selected.1 p member w inside selected.2 active

theorem glue_outside {W V : Type} (z : V) (parent : W → Prop) (patches : List (Patch W V))
    (w : W) (outside : ¬parent w) : glue z parent patches w=z := by
  simp [glue,outside]

theorem unique_zero_extension {W V : Type} (z : V) (parent : W → Prop) (patches : List (Patch W V))
    (covered : Covers parent patches) (compatible : Compatible parent patches) (f : W → V)
    (rep : Represents parent patches f) (outside : ∀ w, ¬parent w → f w=z) :
    f=glue z parent patches := by
  funext w
  classical
  by_cases h : parent w
  · exact unique_on_parent parent patches covered f _ rep (glue_represents z parent patches compatible) w h
  · rw [outside w h,glue_outside z parent patches w h]

theorem covers_same_members {W V : Type} (parent : W → Prop) (a b : List (Patch W V))
    (same : ∀ p, p ∈ a ↔ p ∈ b) : Covers parent a ↔ Covers parent b := by
  simp only [Covers]
  constructor <;> intro covered w hw
  · obtain ⟨p,hp,active⟩ := covered w hw; exact ⟨p,(same p).mp hp,active⟩
  · obtain ⟨p,hp,active⟩ := covered w hw; exact ⟨p,(same p).mpr hp,active⟩

theorem represents_same_members {W V : Type} (parent : W → Prop) (a b : List (Patch W V))
    (same : ∀ p, p ∈ a ↔ p ∈ b) (f : W → V) : Represents parent a f ↔ Represents parent b f := by
  constructor
  · intro h p member; exact h p ((same p).mpr member)
  · intro h p member; exact h p ((same p).mp member)

theorem glue_order_independent {W V : Type} (z : V) (parent : W → Prop) (a b : List (Patch W V))
    (same : ∀ p, p ∈ a ↔ p ∈ b) (covered : Covers parent a)
    (ca : Compatible parent a) (cb : Compatible parent b) : glue z parent a=glue z parent b := by
  apply unique_zero_extension z parent b ((covers_same_members parent a b same).mp covered) cb
  · exact (represents_same_members parent a b same _).mp (glue_represents z parent a ca)
  · exact glue_outside z parent a

theorem duplicate_does_not_add_value {W V : Type} (z : V) (parent : W → Prop) (patches : List (Patch W V))
    (covered : Covers parent patches) (compatible : Compatible parent patches) :
    glue z parent (patches++patches)=glue z parent patches := by
  have same : ∀ p, p ∈ (patches++patches) ↔ p ∈ patches := by simp
  apply glue_order_independent z parent (patches++patches) patches same
  · exact (covers_same_members parent (patches++patches) patches same).mpr covered
  · intro a ha b hb w hp hwa hwb
    exact compatible a ((same a).mp ha) b ((same b).mp hb) w hp hwa hwb
  · exact compatible

theorem incompatible_overlap_has_no_representation {W V : Type} (parent : W → Prop)
    (patches : List (Patch W V)) (conflict : ¬Compatible parent patches) :
    ¬∃ f, Represents parent patches f := by
  rintro ⟨f,rep⟩
  exact conflict (representation_requires_agreement parent patches f rep)

end FunctionCovers

#print axioms FunctionCovers.mask_inside
#print axioms FunctionCovers.mask_outside
#print axioms FunctionCovers.mask_idempotent
#print axioms FunctionCovers.empty_cover_iff
#print axioms FunctionCovers.zero_is_not_missing_coverage
#print axioms FunctionCovers.representation_requires_agreement
#print axioms FunctionCovers.unique_on_parent
#print axioms FunctionCovers.glue_represents
#print axioms FunctionCovers.glue_outside
#print axioms FunctionCovers.unique_zero_extension
#print axioms FunctionCovers.covers_same_members
#print axioms FunctionCovers.represents_same_members
#print axioms FunctionCovers.glue_order_independent
#print axioms FunctionCovers.duplicate_does_not_add_value
#print axioms FunctionCovers.incompatible_overlap_has_no_representation
