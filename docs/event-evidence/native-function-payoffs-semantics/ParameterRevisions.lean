import Std

/-!
Parameter-domain inclusion and pointwise family conditioning. A finite outcome
roster enumerates outcomes at one actual parameter, never guard valuations. The
native constructor must establish nonnegative normalized prior laws, exact cells,
support correspondence and faithful arithmetic. This reference does not verify
the solver, canonical bytes, arena transfer, source constructors or query compiler.
-/
namespace ParameterRevisions

section Domains
variable {T O G H : Type}

def includeDomain (source target : T → Prop) (inside : ∀ t, source t → target t)
    (world : {t // source t} × O) : {t // target t} × O :=
  (⟨world.1.val,inside world.1.val world.1.property⟩,world.2)

theorem inclusion_keeps_parameter (source target : T → Prop)
    (inside : ∀ t, source t → target t) (world : {t // source t} × O) :
    (includeDomain source target inside world).1.val=world.1.val := rfl

/-- Whole target cells, not merely matching guard codes, prevent importing a
witness at another parameter outside the source's admitted domain. -/
theorem whole_cell_preserves_domain
    (source target : T → Prop) (a : T → G) (b : T → H) (g : G) (h : H)
    (exactCell : ∀ t, (source t ∧ a t=g) ↔ (target t ∧ b t=h))
    (t : T) (admitted : target t) (guard : b t=h) : source t :=
  ((exactCell t).mpr ⟨admitted,guard⟩).1

theorem no_image_outside_source_domain
    (source : T → Prop) (relation : T → O → O → Prop) (t : T) (o : O)
    (outside : ¬source t) : ¬∃ x, source t ∧ relation t x o := by
  rintro ⟨_,h,_⟩
  exact outside h

theorem coarse_code_is_insufficient :
    let domain : Bool → Prop := fun b => b=true
    let guard : Bool → Unit := fun _ => ()
    (∃ b, domain b ∧ guard b=guard false) ∧ ¬domain false := by
  exact ⟨⟨true,rfl,rfl⟩,by decide⟩

end Domains

section Fields
variable {O T K : Type} [Lean.Grind.Field K]

def sum : List O → (O → K) → K
  | [], _ => 0
  | x::xs, f => f x + sum xs f

theorem sum_congr (xs : List O) (f g : O → K) (same : ∀ x, f x=g x) :
    sum xs f=sum xs g := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [sum,same,ih]

theorem sum_div (xs : List O) (f : O → K) (z : K) :
    sum xs (fun x => f x/z) = sum xs f/z := by
  induction xs with
  | nil => simp only [sum]; grind
  | cons x xs ih => simp only [sum,ih]; grind

def mass (xs : List O) (density : O → K) (evidence : O → Bool) : K :=
  sum xs (fun o => if evidence o then density o else 0)

def posterior (xs : List O) (density : O → K) (evidence : O → Bool) (o : O) : K :=
  (if evidence o then density o else 0) / mass xs density evidence

noncomputable def family (domain : T → Prop) (xs : List O)
    (density : T → O → K) (evidence : T → O → Bool) (t : T) : Option (O → K) := by
  classical
  exact if domain t ∧ mass xs (density t) (evidence t)≠0
    then some (posterior xs (density t) (evidence t)) else none

theorem family_defined_exactly (domain : T → Prop) (xs : List O)
    (density : T → O → K) (evidence : T → O → Bool) (t : T) :
    (family domain xs density evidence t).isSome=true ↔
      domain t ∧ mass xs (density t) (evidence t)≠0 := by
  classical
  simp only [family]
  split <;> simp_all

theorem zero_evidence_excludes_parameter (domain : T → Prop) (xs : List O)
    (density : T → O → K) (evidence : T → O → Bool) (t : T)
    (zero : mass xs (density t) (evidence t)=0) :
    family domain xs density evidence t=none := by
  simp [family,zero]

theorem posterior_normalized (xs : List O) (density : O → K) (evidence : O → Bool)
    (nonzero : mass xs density evidence≠0) : sum xs (posterior xs density evidence)=1 := by
  unfold posterior
  rw [sum_div]
  change mass xs density evidence / mass xs density evidence=1
  grind

theorem posterior_zero_outside_evidence (xs : List O) (density : O → K)
    (evidence : O → Bool) (o : O) (no : evidence o=false) :
    posterior xs density evidence o=0 := by
  simp only [posterior,no,Bool.false_eq_true,if_false]
  grind

theorem repeated_evidence (xs : List O) (density : O → K) (evidence : O → Bool)
    (nonzero : mass xs density evidence≠0) (o : O) :
    posterior xs (posterior xs density evidence) evidence o = posterior xs density evidence o := by
  have again : mass xs (posterior xs density evidence) evidence=1 := by
    calc
      _ = sum xs (posterior xs density evidence) := by
        apply sum_congr
        intro x
        cases h : evidence x
        · simp only [Bool.false_eq_true,if_false]
          exact (posterior_zero_outside_evidence xs density evidence x h).symm
        · simp only [if_true]
      _ = 1 := posterior_normalized xs density evidence nonzero
  change (if evidence o then posterior xs density evidence o else 0)/
    mass xs (posterior xs density evidence) evidence = posterior xs density evidence o
  rw [again]
  cases h : evidence o
  · rw [posterior_zero_outside_evidence xs density evidence o h]
    simp only [Bool.false_eq_true,if_false]; grind
  · simp only [if_true]; grind

theorem support_is_not_evidence (legal : O → Prop) (xs : List O)
    (density : O → K) (evidence : O → Bool) (o : O)
    (allowed : legal o) (no : evidence o=false) :
    legal o ∧ posterior xs density evidence o=0 :=
  ⟨allowed,posterior_zero_outside_evidence xs density evidence o no⟩

end Fields

theorem positive_mass_iff_nonzero (z : Rat) (nonnegative : 0≤z) : 0<z ↔ z≠0 := by grind

theorem posterior_nonnegative {O : Type} (xs : List O) (density : O → Rat)
    (evidence : O → Bool) (positive : 0<mass xs density evidence)
    (nonnegative : ∀ o, 0≤density o) (o : O) : 0≤posterior xs density evidence o := by
  have numerator : 0≤(if evidence o then density o else 0) := by
    cases evidence o <;> simp only [Bool.false_eq_true,if_false,if_true]
    · exact Rat.le_refl
    · exact nonnegative o
  exact Rat.mul_nonneg numerator (Rat.le_of_lt (Rat.inv_pos.mpr positive))

theorem possible_event_can_have_no_posterior :
    let density : Bool → Rat := fun b => if b then 0 else 1
    let evidence : Bool → Bool := id
    (∃ b, evidence b=true) ∧ mass [false,true] density evidence=0 := by
  exact ⟨⟨true,rfl⟩,by decide +kernel⟩

end ParameterRevisions

#print axioms ParameterRevisions.inclusion_keeps_parameter
#print axioms ParameterRevisions.whole_cell_preserves_domain
#print axioms ParameterRevisions.no_image_outside_source_domain
#print axioms ParameterRevisions.coarse_code_is_insufficient
#print axioms ParameterRevisions.sum_congr
#print axioms ParameterRevisions.sum_div
#print axioms ParameterRevisions.family_defined_exactly
#print axioms ParameterRevisions.zero_evidence_excludes_parameter
#print axioms ParameterRevisions.posterior_normalized
#print axioms ParameterRevisions.posterior_zero_outside_evidence
#print axioms ParameterRevisions.repeated_evidence
#print axioms ParameterRevisions.support_is_not_evidence
#print axioms ParameterRevisions.positive_mass_iff_nonzero
#print axioms ParameterRevisions.posterior_nonnegative
#print axioms ParameterRevisions.possible_event_can_have_no_posterior
