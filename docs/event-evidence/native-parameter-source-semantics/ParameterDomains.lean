import Std

/-!
Exact finite logical cells over a potentially infinite parameter carrier, and
domain-retaining rational functions. A cell presentation must supply a witness
for EVERY cell and classify it back to that cell. Native correspondence must
establish root order, complete sign decomposition and rational/algebraic cell
witnesses. This does not prove the Sturm solver, Rust cell merging, root numeric
identity, budget accounting or source/BEVT integration.
-/
namespace ParameterDomains

structure Cells (W C : Type) where
  classify : W → C
  witness : C → W
  sectionLaw : ∀ c, classify (witness c) = c

def denotes {W C : Type} (p : Cells W C) (membership : C → Bool) (w : W) : Prop :=
  membership (p.classify w) = true

theorem cell_witness {W C : Type} (p : Cells W C) (membership : C → Bool) (c : C)
    (included : membership c = true) : denotes p membership (p.witness c) := by
  simp [denotes, p.sectionLaw, included]

theorem exact_emptiness {W C : Type} (p : Cells W C) (membership : C → Bool) :
    (¬∃ w, denotes p membership w) ↔ (∀ c, membership c = false) := by
  constructor
  · intro empty c
    cases value : membership c with
    | false => rfl
    | true => exact False.elim (empty ⟨p.witness c, cell_witness p membership c value⟩)
  · intro empty ⟨w, hw⟩
    simp [denotes, empty] at hw

theorem complement {W C : Type} (p : Cells W C) (membership : C → Bool) (w : W) :
    denotes p (fun c => !membership c) w ↔ ¬denotes p membership w := by
  simp [denotes]

theorem pointwise_binary {W C : Type} (p : Cells W C) (f : Bool → Bool → Bool)
    (a b : C → Bool) (w : W) :
    denotes p (fun c => f (a c) (b c)) w ↔ f (a (p.classify w)) (b (p.classify w)) = true := Iff.rfl

theorem exact_membership_equality {W C : Type} (p : Cells W C) (a b : C → Bool) :
    (∀ w, a (p.classify w) = b (p.classify w)) ↔ (∀ c, a c = b c) := by
  constructor
  · intro same c
    simpa only [p.sectionLaw] using same (p.witness c)
  · intro same w; exact same _

/-- Refinement must preserve the SAME actual parameter assignment. Independent
cell witnesses for the two operands cannot be combined as if they were one. -/
theorem common_refinement {W A B C : Type} (p : Cells W A) (q : Cells W B)
    (r : Cells W C) (left : C → A) (right : C → B)
    (agreeLeft : ∀ w, left (r.classify w) = p.classify w)
    (agreeRight : ∀ w, right (r.classify w) = q.classify w)
    (a : A → Bool) (b : B → Bool) (f : Bool → Bool → Bool) (w : W) :
    f (a (left (r.classify w))) (b (right (r.classify w))) =
      f (a (p.classify w)) (b (q.classify w)) := by rw [agreeLeft, agreeRight]

theorem redundant_boundary (left point right : Bool) (h1 : left=point) (h2 : point=right) :
    ∀ side : Fin 3, (if side.val=0 then left else if side.val=1 then point else right) = left := by
  intro side; simp [← h1, ← h2]

theorem point_membership_is_independent :
    let openRegion : Fin 3 → Bool := fun i => i.val != 1
    openRegion 0 = true ∧ openRegion 2 = true ∧ openRegion 1 = false := by decide

section PartialFunctions
variable {K : Type} [Lean.Grind.Field K]

noncomputable def observe (ambient inherited : K → Prop) (n d : K → K) (x : K) : Option K := by
  classical
  exact if ambient x ∧ inherited x ∧ d x ≠ 0 then some (n x / d x) else none

theorem defined_exactly (ambient inherited : K → Prop) (n d : K → K) (x : K) :
    (observe ambient inherited n d x).isSome = true ↔ ambient x ∧ inherited x ∧ d x ≠ 0 := by
  classical
  simp only [observe]
  split <;> simp_all

theorem zero_denominator_undefined (ambient inherited : K → Prop) (n d : K → K) (x : K)
    (zero : d x=0) : observe ambient inherited n d x = none := by
  simp [observe, zero]

theorem inherited_hole_preserved (ambient inherited : K → Prop) (n d : K → K) (x : K)
    (hole : ¬inherited x) : observe ambient inherited n d x = none := by
  simp [observe, hole]

theorem zero_numerator_does_not_erase_hole (ambient inherited : K → Prop) (d : K → K) (x : K)
    (hole : ¬inherited x) : observe ambient inherited (fun _ => 0) d x = none := by
  exact inherited_hole_preserved _ _ _ _ _ hole

theorem quotient_add (n m d e : K) (hd : d≠0) (he : e≠0) :
    n/d + m/e = (n*e+m*d)/(d*e) := by grind

theorem quotient_multiply (n m d e : K) (hd : d≠0) (he : e≠0) :
    (n/d)*(m/e) = (n*m)/(d*e) := by grind

theorem same_value_and_domain (ambient inherited : K → Prop) (n d m e : K → K)
    (defined : ∀ x, ambient x → inherited x → (d x≠0 ↔ e x≠0))
    (values : ∀ x, ambient x → inherited x → d x≠0 → n x / d x = m x / e x) (x : K) :
    observe ambient inherited n d x = observe ambient inherited m e x := by
  classical
  by_cases a : ambient x
  · by_cases i : inherited x
    · have same := defined x a i
      by_cases h : d x≠0
      · have h' := same.mp h
        simp [observe, a, i, h, h', values x a i h]
      · have h' : ¬e x≠0 := fun he => h (same.mpr he)
        simp [observe, a, i, h, h']
    · simp [observe, i]
  · simp [observe, a]

theorem inhabited_source_can_be_nowhere_defined (a : K) :
    (∃ _ : K, True) ∧ ∀ x : K, observe (fun _ => True) (fun _ => True) (fun _ => 1) (fun _ => 0) x = none := by
  refine ⟨⟨a, trivial⟩, ?_⟩
  intro x; exact zero_denominator_undefined _ _ _ _ _ rfl

end PartialFunctions
end ParameterDomains

#print axioms ParameterDomains.cell_witness
#print axioms ParameterDomains.exact_emptiness
#print axioms ParameterDomains.complement
#print axioms ParameterDomains.pointwise_binary
#print axioms ParameterDomains.exact_membership_equality
#print axioms ParameterDomains.common_refinement
#print axioms ParameterDomains.redundant_boundary
#print axioms ParameterDomains.point_membership_is_independent
#print axioms ParameterDomains.defined_exactly
#print axioms ParameterDomains.zero_denominator_undefined
#print axioms ParameterDomains.inherited_hole_preserved
#print axioms ParameterDomains.zero_numerator_does_not_erase_hole
#print axioms ParameterDomains.quotient_add
#print axioms ParameterDomains.quotient_multiply
#print axioms ParameterDomains.same_value_and_domain
#print axioms ParameterDomains.inhabited_source_can_be_nowhere_defined
