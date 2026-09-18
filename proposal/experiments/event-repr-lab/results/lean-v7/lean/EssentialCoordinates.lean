import Std

/-!
Essential coordinates are the unique least dependency set for a predicate on a
finite binary product. They describe Boolean dependence, not admitted support or
a probability law. This file does not verify Rust normalization or interning.
-/
namespace EssentialCoordinates

variable {ι : Type} [DecidableEq ι]
abbrev World (ι : Type) := ι → Bool

def put (w : World ι) (i : ι) (v : Bool) : World ι :=
  fun j => if j = i then v else w j

def Essential (f : World ι → Bool) (i : ι) : Prop :=
  ∃ w v, f (put w i v) ≠ f w

def DependsOnly (f : World ι → Bool) (S : ι → Prop) : Prop :=
  ∀ a b, (∀ i, S i → a i = b i) → f a = f b

theorem put_self (w : World ι) (i : ι) : put w i (w i) = w := by
  funext j
  by_cases h : j = i <;> simp [put, h]

theorem put_twice (w : World ι) (i : ι) (u v : Bool) :
    put (put w i u) i v = put w i v := by
  funext j
  by_cases h : j = i <;> simp [put, h]

theorem put_commute (w : World ι) (i j : ι) (u v : Bool) (hne : i ≠ j) :
    put (put w i u) j v = put (put w j v) i u := by
  funext k
  by_cases hi : k = i
  · subst k; simp [put, hne]
  · by_cases hj : k = j
    · subst k; simp [put, hi]
    · simp [put, hi, hj]

/-- A cofactor can only lose dependence, including on coordinates other than
    the pinned coordinate. This is a statement about a full binary product. -/
theorem cofactor_essential_subset (f : World ι → Bool) (i j : ι) (v : Bool) :
    Essential (fun w => f (put w i v)) j → j ≠ i ∧ Essential f j := by
  rintro ⟨w, u, changed⟩
  have hji : j ≠ i := by
    intro same
    subst j
    apply changed
    simp only [put_twice]
  refine ⟨hji, put w i v, u, ?_⟩
  simpa only [put_commute w j i u v hji] using changed

private def selector (w : World (Fin 3)) : Bool :=
  if w 0 then w 1 else w 2

/-- F = if x then y else z uses all three coordinates, but pinning x=true
    reduces it exactly to y. Original dependence minus pins need not be exact. -/
theorem cofactor_can_remove_other_dependence :
    Essential selector 0 ∧ Essential selector 1 ∧ Essential selector 2 ∧
    (∀ w, selector (put w 0 true) = w 1) ∧
    ¬ Essential (fun w => selector (put w 0 true)) 2 := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · exact ⟨(fun i => decide (i = 1)), true, by decide⟩
  · exact ⟨(fun i => decide (i = 0)), true, by decide⟩
  · exact ⟨(fun _ => false), true, by decide⟩
  · intro w; simp [selector, put]
  · rintro ⟨w, v, changed⟩
    apply changed
    simp [selector, put]

#print axioms cofactor_essential_subset
#print axioms cofactor_can_remove_other_dependence

theorem cofactor_characterization (f : World ι → Bool) (i : ι) :
    Essential f i ↔ ∃ w, f (put w i false) ≠ f (put w i true) := by
  constructor
  · rintro ⟨w, v, changed⟩
    refine ⟨w, ?_⟩
    intro same
    cases hw : w i <;> cases v <;>
      (have original := congrArg f (put_self w i)
       simp only [hw] at original
       simp_all)
  · rintro ⟨w, changed⟩
    refine ⟨put w i true, false, ?_⟩
    simpa only [put_twice] using changed

def existsAt (f : World ι → Bool) (i : ι) : World ι → Bool :=
  fun w => f (put w i false) || f (put w i true)

theorem quantified_coordinate_inessential (f : World ι → Bool) (i : ι) :
    ¬ Essential (existsAt f i) i := by
  rintro ⟨w, v, changed⟩
  apply changed
  simp [existsAt, put_twice]

theorem inessential_put (f : World ι → Bool) (i : ι)
    (h : ¬ Essential f i) (w : World ι) (v : Bool) :
    f (put w i v) = f w := by
  apply Classical.byContradiction
  intro hn
  exact h ⟨w, v, hn⟩

def overwrite (a b : World ι) : List ι → World ι
  | [] => a
  | i :: is => put (overwrite a b is) i (b i)

theorem overwrite_at (a b : World ι) (is : List ι) (j : ι) :
    overwrite a b is j = if j ∈ is then b j else a j := by
  induction is with
  | nil => simp [overwrite]
  | cons i is ih =>
    by_cases h : j = i
    · subst j; simp [overwrite, put]
    · simp [overwrite, put, h, ih]

theorem overwrite_agrees (a b : World ι) (is : List ι) (S : ι → Prop)
    (h : ∀ i, S i → a i = b i) :
    ∀ i, S i → overwrite a b is i = b i := by
  intro i hi
  rw [overwrite_at]
  split
  · rfl
  · exact h i hi

theorem overwrite_preserves (f : World ι → Bool) (a b : World ι) (is : List ι)
    (h : ∀ i, Essential f i → a i = b i) : f (overwrite a b is) = f a := by
  induction is with
  | nil => rfl
  | cons i is ih =>
    have step : f (put (overwrite a b is) i (b i)) = f (overwrite a b is) := by
      by_cases hi : Essential f i
      · have same := (overwrite_agrees a b is (Essential f) h i hi).symm
        rw [same, put_self]
      · exact inessential_put f i hi _ _
    exact step.trans ih

/-- The explicit covering list is the finiteness hypothesis; no infinite-product
    assertion is hidden here. Repeated coordinates in the list are harmless. -/
theorem essential_sufficient (f : World ι → Bool) (coordinates : List ι)
    (covers : ∀ i, i ∈ coordinates) : DependsOnly f (Essential f) := by
  intro a b h
  have done : overwrite a b coordinates = b := by
    funext i
    simp [overwrite_at, covers i]
  have hp := overwrite_preserves f a b coordinates h
  rw [done] at hp
  exact hp.symm

theorem essential_minimum (f : World ι → Bool) (S : ι → Prop)
    (h : DependsOnly f S) : ∀ i, Essential f i → S i := by
  intro i ⟨w, v, changed⟩
  apply Classical.byContradiction
  intro hi
  apply changed
  apply h
  intro j hj
  have different : j ≠ i := by
    intro same
    subst j
    exact hi hj
  simp [put, different]

theorem support_iff_contains_essential (f : World ι → Bool) (S : ι → Prop)
    (coordinates : List ι) (covers : ∀ i, i ∈ coordinates) :
    DependsOnly f S ↔ ∀ i, Essential f i → S i := by
  constructor
  · exact essential_minimum f S
  · intro contains a b agrees
    apply essential_sufficient f coordinates covers a b
    intro i hi
    exact agrees i (contains i hi)

theorem complement_essential (f : World ι → Bool) (i : ι) :
    Essential (fun w => !(f w)) i ↔ Essential f i := by
  constructor
  · rintro ⟨w, v, different⟩
    refine ⟨w, v, ?_⟩
    intro same
    exact different (congrArg Bool.not same)
  · rintro ⟨w, v, different⟩
    refine ⟨w, v, ?_⟩
    cases h₁ : f (put w i v) <;> cases h₂ : f w <;> simp_all

/-- Fixing every omitted coordinate to false is an exact local-table decoder. -/
theorem canonical_projection (f : World ι → Bool) (S : ι → Prop)
    [DecidablePred S] (contains : ∀ i, Essential f i → S i)
    (coordinates : List ι) (covers : ∀ i, i ∈ coordinates) (w : World ι) :
    f (fun i => if S i then w i else false) = f w := by
  apply (support_iff_contains_essential f S coordinates covers).mpr contains
  intro i hi
  simp [hi]

#print axioms essential_sufficient
#print axioms cofactor_characterization
#print axioms quantified_coordinate_inessential
#print axioms essential_minimum
#print axioms support_iff_contains_essential
#print axioms complement_essential
#print axioms canonical_projection

end EssentialCoordinates
