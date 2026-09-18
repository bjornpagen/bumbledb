import Std

/-!
The finite reference for the native Event admission accumulator in one scalar
determinant group. Each fact appears once, after canonical whole-fact deduplication.
All incidences have already been checked in one common source/support context.
No probability or positive-mass premise occurs. The Boolean accumulator can be
executed, but these theorems do not verify Rust's arena, byte maps or row traversal.
-/
namespace Admission

structure Summary (W : Type) where
  coverage : W → Bool
  conflict : W → Bool

def add {W : Type} (s : Summary W) (event : W → Bool) : Summary W :=
  ⟨fun w => s.coverage w || event w,
   fun w => s.conflict w || (s.coverage w && event w)⟩

def summarize {F W : Type} (incidence : F → W → Bool) : List F → Summary W
  | [] => ⟨fun _ => false, fun _ => false⟩
  | f :: fs => add (summarize incidence fs) (incidence f)

theorem coverage_exact {F W : Type} (incidence : F → W → Bool)
    (facts : List F) (w : W) :
    (summarize incidence facts).coverage w = true ↔
      ∃ f ∈ facts, incidence f w = true := by
  induction facts with
  | nil => simp [summarize]
  | cons f fs ih =>
    simp only [summarize, add, Bool.or_eq_true, ih, List.mem_cons]
    constructor
    · rintro (⟨g, member, present⟩ | present)
      · exact ⟨g, Or.inr member, present⟩
      · exact ⟨f, Or.inl rfl, present⟩
    · rintro ⟨g, (rfl | member), present⟩
      · exact Or.inr present
      · exact Or.inl ⟨g, member, present⟩

/-- The conflict summary records overlap between distinct *whole facts*.
    Without the Nodup premise, inserting one fact twice creates a false conflict. -/
theorem conflict_exact {F W : Type} (incidence : F → W → Bool)
    (facts : List F) (unique : facts.Nodup) (w : W) :
    (summarize incidence facts).conflict w = true ↔
      ∃ a ∈ facts, ∃ b ∈ facts, a ≠ b ∧ incidence a w = true ∧ incidence b w = true := by
  induction facts with
  | nil => simp [summarize]
  | cons f fs ih =>
    have fresh := (List.nodup_cons.mp unique).1
    have tail := (List.nodup_cons.mp unique).2
    simp only [summarize, add, Bool.or_eq_true, Bool.and_eq_true]
    rw [ih tail, coverage_exact]
    constructor
    · rintro (⟨a, ha, b, hb, different, ea, eb⟩ | ⟨⟨a, ha, ea⟩, ef⟩)
      · exact ⟨a, List.mem_cons_of_mem _ ha, b, List.mem_cons_of_mem _ hb, different, ea, eb⟩
      · exact ⟨a, List.mem_cons_of_mem _ ha, f, List.mem_cons_self,
          fun equal => fresh (equal ▸ ha), ea, ef⟩
    · rintro ⟨a, ha, b, hb, different, ea, eb⟩
      rcases List.mem_cons.mp ha with rfl | inTailA
      · rcases List.mem_cons.mp hb with rfl | inTailB
        · exact False.elim (different rfl)
        · exact Or.inr ⟨⟨b, inTailB, eb⟩, ea⟩
      · rcases List.mem_cons.mp hb with rfl | inTailB
        · exact Or.inr ⟨⟨a, inTailA, ea⟩, eb⟩
        · exact Or.inl ⟨a, inTailA, b, inTailB, different, ea, eb⟩

def pointwiseKey {F W : Type} (incidence : F → W → Bool) (facts : List F) : Prop :=
  ∀ a ∈ facts, ∀ b ∈ facts, ∀ w, incidence a w = true → incidence b w = true → a = b

theorem key_iff_no_conflicts {F W : Type} (incidence : F → W → Bool)
    (facts : List F) (unique : facts.Nodup) :
    pointwiseKey incidence facts ↔ ∀ w, (summarize incidence facts).conflict w = false := by
  constructor
  · intro key w
    cases h : (summarize incidence facts).conflict w with
    | false => rfl
    | true =>
      obtain ⟨a, ha, b, hb, different, ea, eb⟩ := (conflict_exact incidence facts unique w).mp h
      exact False.elim (different (key a ha b hb w ea eb))
  · intro clear a ha b hb w ea eb
    apply Classical.byContradiction
    intro different
    have conflict := (conflict_exact incidence facts unique w).mpr ⟨a, ha, b, hb, different, ea, eb⟩
    rw [clear w] at conflict
    cases conflict

theorem containment_iff_union {S T W : Type}
    (source : S → W → Bool) (target : T → W → Bool) (targets : List T) (s : S) :
    (∀ w, source s w = true → (summarize target targets).coverage w = true) ↔
      (∀ w, source s w = true → ∃ t ∈ targets, target t w = true) := by
  simp only [coverage_exact]

/-- Native traversal order may vary; the complete denotation cannot. -/
theorem order_independent {F W : Type} (incidence : F → W → Bool)
    (a b : List F) (ua : a.Nodup) (ub : b.Nodup)
    (same : ∀ f, f ∈ a ↔ f ∈ b) : summarize incidence a = summarize incidence b := by
  have cover : (summarize incidence a).coverage = (summarize incidence b).coverage := by
    funext w
    apply Bool.eq_iff_iff.mpr
    simp only [coverage_exact, same]
  have conflict : (summarize incidence a).conflict = (summarize incidence b).conflict := by
    funext w
    apply Bool.eq_iff_iff.mpr
    simp only [conflict_exact incidence a ua, conflict_exact incidence b ub, same]
  cases ha : summarize incidence a
  cases hb : summarize incidence b
  simp_all

/-- Exactly the rows meeting the conflict region have a distinct competitor.
    Empty and unrelated rows in the same scalar group must not be cited. -/
theorem citation_exact {F W : Type} (incidence : F → W → Bool)
    (facts : List F) (unique : facts.Nodup) (f : F) (member : f ∈ facts) :
    (∃ w, incidence f w = true ∧ (summarize incidence facts).conflict w = true) ↔
      ∃ g ∈ facts, f ≠ g ∧ ∃ w, incidence f w = true ∧ incidence g w = true := by
  constructor
  · rintro ⟨w, ef, conflict⟩
    obtain ⟨a, ha, b, hb, different, ea, eb⟩ := (conflict_exact incidence facts unique w).mp conflict
    by_cases equal : f = a
    · exact ⟨b, hb, fun h => different (equal.symm.trans h), w, ef, eb⟩
    · exact ⟨a, ha, equal, w, ef, ea⟩
  · rintro ⟨g, hg, different, w, ef, eg⟩
    exact ⟨w, ef, (conflict_exact incidence facts unique w).mpr ⟨f, member, g, hg, different, ef, eg⟩⟩

theorem empty_contribution {W : Type} (summary : Summary W) :
    add summary (fun _ => false) = summary := by
  cases summary
  simp [add]

theorem empty_requires_no_target {W : Type} (w : W) :
    ((summarize (fun _ : Empty => fun _ : W => false) []).coverage w = false) ∧
      (false = true → (summarize (fun _ : Empty => fun _ : W => false) []).coverage w = true) := by
  simp [summarize]

/-- Contributors must be recomputed on deletion. Subtracting the removed
    region from an old union loses any surviving contributor's shared points. -/
theorem subtracting_coverage_is_not_deletion :
    let incidence : Bool → Unit → Bool := fun _ _ => true
    (summarize incidence [false]).coverage () = true ∧
    ((summarize incidence [true, false]).coverage () && !(incidence true ())) = false := by
  decide

theorem duplicate_fact_creates_false_conflict :
    let incidence : Unit → Unit → Bool := fun _ _ => true
    (summarize incidence [()]).conflict () = false ∧
    (summarize incidence [(), ()]).conflict () = true := by
  decide

end Admission

#print axioms Admission.coverage_exact
#print axioms Admission.conflict_exact
#print axioms Admission.key_iff_no_conflicts
#print axioms Admission.containment_iff_union
#print axioms Admission.order_independent
#print axioms Admission.citation_exact
#print axioms Admission.empty_contribution
#print axioms Admission.empty_requires_no_target
#print axioms Admission.subtracting_coverage_is_not_deletion
#print axioms Admission.duplicate_fact_creates_false_conflict
