import Std

/-!
An exact finite support certificate: containment plus equal cardinality.
The list is a proof witness for finiteness, not a runtime enumeration plan.
Duplicates are permitted because both predicates use the same worlds list;
a duplicate-free enumeration gives ordinary set cardinality.
-/
namespace FiniteAdmission

theorem finite_admission_iff {W : Type} (worlds : List W)
    (covers : ∀ w, w ∈ worlds) (actual declared : W → Bool) :
    (∀ w, actual w = declared w) ↔
      ((∀ w, actual w → declared w) ∧
       (worlds.filter actual).length = (worlds.filter declared).length) := by
  constructor
  · intro equal
    have functions : actual = declared := funext equal
    subst declared
    exact ⟨fun _ h => h, rfl⟩
  · rintro ⟨contained, population⟩
    have keep : (worlds.filter actual).filter declared = worlds.filter actual :=
      List.filter_eq_self.mpr (fun w hw => contained w (List.mem_filter.mp hw).2)
    have sub : List.Sublist (worlds.filter actual) (worlds.filter declared) :=
      List.sublist_filter_iff.mpr ⟨worlds.filter actual, List.filter_sublist, keep.symm⟩
    have equal := sub.eq_of_length population
    intro w
    apply Bool.eq_iff_iff.mpr
    have membership : w ∈ worlds.filter actual ↔ w ∈ worlds.filter declared := by
      rw [equal]
    simpa only [List.mem_filter, covers w, true_and] using membership

theorem cardinality_alone_is_insufficient :
    (([false, true].filter (fun b => !b)).length =
      ([false, true].filter (fun b => b)).length) ∧
    ¬ (∀ b : Bool, (!b) = b) := by decide

#print axioms finite_admission_iff
#print axioms cardinality_alone_is_insufficient
end FiniteAdmission
