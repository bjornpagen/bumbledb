import Std

/-!
Query predicates retain a partial truth function over one shared parameter
assignment. Quantification is explicit and does not assert independence or
choose a prior. These are denotational laws, not a proof of the native solver,
BENP encoding, query registry or spill implementation.
-/
namespace PredicateQueries

def strict (op : Bool → Bool → Bool) (a b : Option Bool) : Option Bool := do
  let x ← a
  let y ← b
  pure (op x y)

def negate (a : Option Bool) : Option Bool := a.map Bool.not

def possibly {Θ : Type} (p : Θ → Option Bool) : Prop := ∃ θ, p θ = some true

def always {Θ : Type} (p : Θ → Option Bool) : Prop := ∀ θ, p θ = some true

def total {Θ : Type} (p : Θ → Option Bool) : Prop := ∀ θ, ∃ b, p θ = some b

theorem strict_defined_iff (op : Bool → Bool → Bool) (a b : Option Bool) :
    (∃ c, strict op a b = some c) ↔ (∃ x, a = some x) ∧ (∃ y, b = some y) := by
  cases a <;> cases b <;> simp [strict]

theorem strict_left_hole (op : Bool → Bool → Bool) (b : Option Bool) :
    strict op none b = none := rfl

theorem strict_right_hole (op : Bool → Bool → Bool) (a : Option Bool) :
    strict op a none = none := by cases a <;> rfl

theorem constant_true_preserves_holes :
    strict (fun _ _ => true) (some true) none = none := rfl

theorem negate_twice (a : Option Bool) : negate (negate a) = a := by
  cases a with
  | none => rfl
  | some b => cases b <;> rfl

theorem negation_preserves_totality {Θ : Type} (p : Θ → Option Bool) :
    total (fun θ => negate (p θ)) ↔ total p := by
  constructor
  · intro h θ
    obtain ⟨b, hb⟩ := h θ
    have := congrArg negate hb
    rw [negate_twice] at this
    exact ⟨!b, this⟩
  · intro h θ
    obtain ⟨b, hb⟩ := h θ
    exact ⟨!b, by simp [negate, hb]⟩

theorem always_is_total {Θ : Type} (p : Θ → Option Bool) (h : always p) : total p :=
  fun θ => ⟨true, h θ⟩

theorem always_is_possible_on_inhabited_domain {Θ : Type} (p : Θ → Option Bool)
    (inhabited : Nonempty Θ) (h : always p) : possibly p := by
  obtain ⟨θ⟩ := inhabited
  exact ⟨θ, h θ⟩

theorem no_false_plus_total_is_always {Θ : Type} (p : Θ → Option Bool)
    (defined : total p) (noFalse : ¬ possibly (fun θ => negate (p θ))) : always p := by
  intro θ
  obtain ⟨b, hb⟩ := defined θ
  cases b with
  | false => exact False.elim (noFalse ⟨θ, by simp [negate, hb]⟩)
  | true => exact hb

theorem a_hole_is_neither_true_nor_false :
    ¬ possibly (fun _ : Unit => none) ∧
    ¬ possibly (fun _ : Unit => negate none) ∧
    ¬ always (fun _ : Unit => none) := by
  simp [possibly, always, negate]

theorem conjunction_needs_one_shared_witness {Θ : Type} (p q : Θ → Option Bool) :
    possibly (fun θ => strict Bool.and (p θ) (q θ)) ↔
      ∃ θ, p θ = some true ∧ q θ = some true := by
  constructor
  · rintro ⟨θ, h⟩
    refine ⟨θ, ?_⟩
    cases hp : p θ with
    | none => simp [hp, strict] at h
    | some a =>
      cases hq : q θ with
      | none => simp [hp, hq, strict] at h
      | some b =>
        cases a <;> cases b <;> simp_all [strict]
  · rintro ⟨θ, hp, hq⟩
    exact ⟨θ, by simp [strict, hp, hq]⟩

theorem separately_possible_does_not_mean_jointly_possible :
    possibly (fun b : Bool => some b) ∧
    possibly (fun b : Bool => some (!b)) ∧
    ¬ possibly (fun b : Bool => strict Bool.and (some b) (some (!b))) := by
  refine ⟨⟨true, rfl⟩, ⟨false, rfl⟩, ?_⟩
  rintro ⟨b, h⟩
  cases b <;> simp [strict] at h

theorem strict_or_does_not_erase_an_undefined_operand :
    possibly (fun _ : Unit => some true) ∧
    ¬ possibly (fun _ : Unit => strict Bool.or (some true) none) := by
  exact ⟨⟨(), rfl⟩, by simp [possibly, strict]⟩

theorem always_conjunction {Θ : Type} (p q : Θ → Option Bool) :
    always (fun θ => strict Bool.and (p θ) (q θ)) ↔ always p ∧ always q := by
  have pointwise : ∀ θ, strict Bool.and (p θ) (q θ) = some true ↔
      p θ = some true ∧ q θ = some true := by
    intro θ
    cases p θ with
    | none => simp [strict]
    | some a =>
      cases q θ with
      | none => simp [strict]
      | some b => cases a <;> cases b <;> decide
  simp only [always, pointwise, forall_and]

/- Exact region membership is total on its explicit ambient domain. The
complete region is separately retained as a derivation origin. -/
def region {Θ : Type} (ambient : Θ → Prop) (r : Θ → Bool) :
    {θ // ambient θ} → Option Bool := fun θ => some (r θ.val)

theorem region_is_total {Θ : Type} (d : Θ → Prop) (r : Θ → Bool) :
    total (region d r) := fun θ => ⟨r θ.val, rfl⟩

theorem region_possible_iff_intersection {Θ : Type} (d : Θ → Prop) (r : Θ → Bool) :
    possibly (region d r) ↔ ∃ θ, d θ ∧ r θ = true := by
  constructor
  · rintro ⟨⟨θ, hd⟩, h⟩
    exact ⟨θ, hd, Option.some.inj h⟩
  · rintro ⟨θ, hd, h⟩
    exact ⟨⟨θ, hd⟩, by simp [region, h]⟩

theorem region_always_iff_inclusion {Θ : Type} (d : Θ → Prop) (r : Θ → Bool) :
    always (region d r) ↔ ∀ θ, d θ → r θ = true := by
  constructor
  · intro h θ hd
    exact Option.some.inj (h ⟨θ, hd⟩)
  · intro h θ
    simp [region, h θ.val θ.property]

theorem region_complement {Θ : Type} (d : Θ → Prop) (r : Θ → Bool)
    (θ : {θ // d θ}) :
    negate (region d r θ) = region d (fun t => !(r t)) θ := rfl

theorem region_boolean_homomorphism {Θ : Type} (d : Θ → Prop)
    (r s : Θ → Bool) (op : Bool → Bool → Bool) (θ : {θ // d θ}) :
    strict op (region d r θ) (region d s θ) =
      region d (fun t => op (r t) (s t)) θ := rfl

theorem same_truth_on_domain_does_not_identify_region_origin :
    (∀ θ : {b : Bool // b = true}, region (fun b => b = true) id θ =
      region (fun b => b = true) (fun _ => true) θ) ∧
    (id : Bool → Bool) ≠ (fun _ => true) := by
  constructor
  · intro θ
    simp [region, θ.property]
  · intro h
    have impossible := congrFun h false
    cases impossible

#print axioms PredicateQueries.strict_defined_iff
#print axioms PredicateQueries.strict_left_hole
#print axioms PredicateQueries.strict_right_hole
#print axioms PredicateQueries.constant_true_preserves_holes
#print axioms PredicateQueries.negate_twice
#print axioms PredicateQueries.negation_preserves_totality
#print axioms PredicateQueries.always_is_total
#print axioms PredicateQueries.always_is_possible_on_inhabited_domain
#print axioms PredicateQueries.no_false_plus_total_is_always
#print axioms PredicateQueries.a_hole_is_neither_true_nor_false
#print axioms PredicateQueries.conjunction_needs_one_shared_witness
#print axioms PredicateQueries.separately_possible_does_not_mean_jointly_possible
#print axioms PredicateQueries.strict_or_does_not_erase_an_undefined_operand
#print axioms PredicateQueries.always_conjunction
#print axioms PredicateQueries.region_is_total
#print axioms PredicateQueries.region_possible_iff_intersection
#print axioms PredicateQueries.region_always_iff_inclusion
#print axioms PredicateQueries.region_complement
#print axioms PredicateQueries.region_boolean_homomorphism
#print axioms PredicateQueries.same_truth_on_domain_does_not_identify_region_origin
end PredicateQueries
