import Std

/-!
An exact finite support certificate: containment plus equal cardinality.
The list is a proof witness for finiteness, not a runtime enumeration plan.
Duplicates are permitted because both predicates use the same universe list;
a duplicate-free enumeration gives ordinary set cardinality.
-/
namespace FiniteAdmission

theorem finite_admission_iff {W : Type} (universe : List W)
    (covers : ∀ w, w ∈ universe) (actual declared : W → Bool) :
    (∀ w, actual w = declared w) ↔
      ((∀ w, actual w → declared w) ∧
       (universe.filter actual).length = (universe.filter declared).length) := by
  constructor
  · intro equal
    have functions : actual = declared := funext equal
    subst declared
    exact ⟨fun _ h => h, rfl⟩
  · rintro ⟨contained, population⟩
    have keep : (universe.filter actual).filter declared = universe.filter actual :=
      List.filter_eq_self.mpr (fun w hw => contained w (List.mem_filter.mp hw).2)
    have sub : universe.filter actual <+ universe.filter declared :=
      List.sublist_filter_iff.mpr ⟨universe.filter actual, List.filter_sublist, keep.symm⟩
    have equal := sub.eq_of_length population
    intro w
    apply Bool.eq_iff_iff.mpr
    have membership : w ∈ universe.filter actual ↔ w ∈ universe.filter declared := by
      rw [equal]
    simpa only [List.mem_filter, covers w, true_and] using membership

theorem cardinality_alone_is_insufficient :
    (([false, true].filter (fun b => !b)).length =
      ([false, true].filter (fun b => b)).length) ∧
    ¬ (∀ b : Bool, (!b) = b) := by decide

#print axioms finite_admission_iff
#print axioms cardinality_alone_is_insufficient
end FiniteAdmission
