import Std

namespace FactorizedPack

def Pack {I : Type} (f : I → Prop) : Prop := ∃ i, f i

theorem independent_product {I J K : Type} (a : I → Prop) (b : J → Prop) (c : K → Prop) :
    (∃ i j k, a i ∧ b j ∧ c k) ↔ Pack a ∧ Pack b ∧ Pack c := by
  constructor
  · rintro ⟨i,j,k,ha,hb,hc⟩; exact ⟨⟨i,ha⟩,⟨j,hb⟩,⟨k,hc⟩⟩
  · rintro ⟨⟨i,ha⟩,⟨j,hb⟩,⟨k,hc⟩⟩; exact ⟨i,j,k,ha,hb,hc⟩

def First {I J K : Type} (r : I → J → K → Prop) (i : I) := ∃ j k, r i j k
def Second {I J K : Type} (r : I → J → K → Prop) (j : J) := ∃ i k, r i j k
def Third {I J K : Type} (r : I → J → K → Prop) (k : K) := ∃ i j, r i j k

def Rectangular {I J K : Type} (r : I → J → K → Prop) : Prop :=
  ∀ i j k, First r i → Second r j → Third r k → r i j k

def CanFactor {I J K : Type} (r : I → J → K → Prop) : Prop :=
  ∀ (a : I → Prop) (b : J → Prop) (c : K → Prop),
    (∃ i j k, r i j k ∧ a i ∧ b j ∧ c k) ↔
      (∃ i, First r i ∧ a i) ∧ (∃ j, Second r j ∧ b j) ∧ (∃ k, Third r k ∧ c k)

/-- Necessary and sufficient for arbitrary row-local Event factors. A common
    output group or pairwise nonempty marginals alone is not this certificate. -/
theorem factorization_iff_rectangular {I J K : Type} (r : I → J → K → Prop) :
    CanFactor r ↔ Rectangular r := by
  constructor
  · intro factor i j k hi hj hk
    obtain ⟨i',j',k',hr,ei,ej,ek⟩ := (factor (fun x => x=i) (fun x => x=j) (fun x => x=k)).mpr
      ⟨⟨i,hi,rfl⟩,⟨j,hj,rfl⟩,⟨k,hk,rfl⟩⟩
    subst i'; subst j'; subst k'; exact hr
  · intro rectangle a b c
    constructor
    · rintro ⟨i,j,k,hr,ha,hb,hc⟩
      exact ⟨⟨i,⟨j,k,hr⟩,ha⟩,⟨j,⟨i,k,hr⟩,hb⟩,⟨k,⟨i,j,hr⟩,hc⟩⟩
    · rintro ⟨⟨i,hi,ha⟩,⟨j,hj,hb⟩,⟨k,hk,hc⟩⟩
      exact ⟨i,j,k,rectangle i j k hi hj hk,ha,hb,hc⟩

/-- Complements stay inside each branch's union. They are predicates on the
    same world; this transformation does not split the world witness. -/
theorem clover {I J K W : Type} (a : I → W → Prop) (b : J → W → Prop)
    (c : K → W → Prop) (w : W) :
    (∃ i j k, a i w ∧ ¬(b j w ∨ c k w)) ↔
      (∃ i, a i w) ∧ (∃ j, ¬b j w) ∧ (∃ k, ¬c k w) := by
  constructor
  · rintro ⟨i,j,k,ha,hbc⟩
    exact ⟨⟨i,ha⟩,⟨j,fun hb => hbc (Or.inl hb)⟩,⟨k,fun hc => hbc (Or.inr hc)⟩⟩
  · rintro ⟨⟨i,ha⟩,⟨j,hb⟩,⟨k,hc⟩⟩
    exact ⟨i,j,k,ha,fun h => h.elim hb hc⟩

/-- Group existence is separate from whether its Event has a possible world. -/
theorem group_presence {I J K : Type} :
    (∃ (_ : I) (_ : J) (_ : K), True) ↔ Nonempty I ∧ Nonempty J ∧ Nonempty K := by
  constructor
  · rintro ⟨i,j,k,_⟩; exact ⟨⟨i⟩,⟨j⟩,⟨k⟩⟩
  · rintro ⟨⟨i⟩,⟨j⟩,⟨k⟩⟩; exact ⟨i,j,k,True.intro⟩

theorem coupled_counterexample :
    (¬ ∃ i j k : Bool, (i=j ∧ j=k) ∧ i=false ∧ j=false ∧ k=true) ∧
    ((∃ i : Bool, i=false) ∧ (∃ j : Bool, j=false) ∧ (∃ k : Bool, k=true)) := by decide

theorem present_empty :
    (∃ (_ : Unit) (_ : Unit) (_ : Unit), True) ∧
    (¬ ∃ (_ : Unit) (_ : Unit) (_ : Unit), False) := by
  constructor
  · exact ⟨(),(),(),True.intro⟩
  · rintro ⟨_,_,_,h⟩; exact h

end FactorizedPack
#print axioms FactorizedPack.independent_product
#print axioms FactorizedPack.factorization_iff_rectangular
#print axioms FactorizedPack.clover
#print axioms FactorizedPack.group_presence
#print axioms FactorizedPack.coupled_counterexample
#print axioms FactorizedPack.present_empty
