import Bumbledb.Query.Denotation

/-!
# Exec/SemiNaive — naive and semi-naive iteration as mechanism

These chains are engine-correctness facts about an operator, not the
reach denotation (`reachDen = lfpS` in `Exec/Reach.lean`). Namespace
stays `Bumbledb.Query` so `Query.naiveIter` / `@Query.semi_naive_agrees`
keep resolving.
-/

namespace Bumbledb.Query

theorem setExt {α : Type u} {s t : Set α} (h : ∀ a, s a ↔ t a) :
    s = t :=
  funext fun a => propext (h a)

/-- The naive chain: start empty, keep everything, add everything
the operator derives. -/
def naiveIter {α : Type u} (T : Set α → Set α) : Nat → Set α
  | 0 => fun _ => False
  | k + 1 => fun a => naiveIter T k a ∨ T (naiveIter T k) a

/-- The semi-naive chain: an accumulator and the frontier
`new = T(acc) \ acc`. -/
def semiNaiveIter {α : Type u} (T : Set α → Set α) :
    Nat → Set α × Set α
  | 0 => (fun _ => False, fun a => T (fun _ => False) a ∧ ¬ False)
  | k + 1 =>
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a,
     fun a => T (fun b => (semiNaiveIter T k).1 b ∨
         (semiNaiveIter T k).2 b) a ∧
       ¬ ((semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a))

theorem semiNaive_delta {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).2 =
      fun a => T (semiNaiveIter T k).1 a ∧ ¬ (semiNaiveIter T k).1 a
  | 0 => rfl
  | _ + 1 => rfl

/-- **Semi-naive agrees with naive**: iterating on
`new = T(acc) \ acc` walks exactly the naive chain. Instantiates at
`T:= reachOp C rec self I W ρ`. -/
theorem semi_naive_agrees {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).1 = naiveIter T k
  | 0 => rfl
  | k + 1 => by
    have ih := semi_naive_agrees T k
    show (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a)
      = naiveIter T (k + 1)
    rw [semiNaive_delta T k, ih]
    refine setExt fun a => ?_
    show naiveIter T k a ∨ (T (naiveIter T k) a ∧ ¬ naiveIter T k a) ↔
      naiveIter T k a ∨ T (naiveIter T k) a
    constructor
    · rintro (h | ⟨h, -⟩)
      · exact Or.inl h
      · exact Or.inr h
    · intro h
      by_cases hk : naiveIter T k a
      · exact Or.inl hk
      · rcases h with h | h
        · exact absurd h hk
        · exact Or.inr ⟨h, hk⟩

theorem semi_naive_same_fixpoint {α : Type u} (T : Set α → Set α)
    (k : Nat) :
    (fun a => (semiNaiveIter T k).1 a ∨ (semiNaiveIter T k).2 a) =
      naiveIter T (k + 1) :=
  semi_naive_agrees T (k + 1)

end Bumbledb.Query
