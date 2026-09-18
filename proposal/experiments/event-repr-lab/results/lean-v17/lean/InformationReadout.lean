import Std

/-!
Information readouts over arbitrary admitted-world types. A constrained scope
is represented by its subtype of legal worlds, so complement is relative to
that scope. No finiteness, probability law, coordinate order or decoder is assumed.
-/
namespace InformationReadout

abbrev Region (Ω : Type) := Ω → Prop
def Possible {Ω O : Type} (f : Ω → O) (A : Region Ω) : Region Ω :=
  fun w => ∃ v, f v = f w ∧ A v
def Guaranteed {Ω O : Type} (f : Ω → O) (A : Region Ω) : Region Ω :=
  fun w => ∀ v, f v = f w → A v
def Stable {Ω O : Type} (f : Ω → O) (A : Region Ω) : Prop :=
  ∀ v w, f v = f w → (A v ↔ A w)
def Refines {Ω O P : Type} (f : Ω → O) (g : Ω → P) : Prop :=
  ∀ v w, f v = f w → g v = g w

theorem sandwich {Ω O : Type} (f : Ω → O) (A : Region Ω) (w : Ω) :
    (Guaranteed f A w → A w) ∧ (A w → Possible f A w) := by
  exact ⟨fun h => h w rfl, fun h => ⟨w, rfl, h⟩⟩

theorem complement_duality {Ω O : Type} (f : Ω → O) (A : Region Ω) (w : Ω) :
    Guaranteed f A w ↔ ¬ Possible f (fun v => ¬ A v) w := by
  classical
  constructor
  · intro h hp
    obtain ⟨v, hv, hn⟩ := hp
    exact hn (h v hv)
  · intro h v hv
    by_cases ha : A v
    · exact ha
    · exact False.elim (h ⟨v, hv, ha⟩)

theorem possible_stable {Ω O : Type} (f : Ω → O) (A : Region Ω) :
    Stable f (Possible f A) := by
  intro v w h
  constructor
  · rintro ⟨x, hx, ha⟩
    exact ⟨x, hx.trans h, ha⟩
  · rintro ⟨x, hx, ha⟩
    exact ⟨x, hx.trans h.symm, ha⟩

theorem guaranteed_stable {Ω O : Type} (f : Ω → O) (A : Region Ω) :
    Stable f (Guaranteed f A) := by
  intro v w h
  constructor
  · intro hv x hx
    exact hv x (hx.trans h.symm)
  · intro hw x hx
    exact hw x (hx.trans h)

theorem possible_fixed_iff_dependency {Ω O : Type} (f : Ω → O) (A : Region Ω) :
    (∀ w, Possible f A w ↔ A w) ↔ Stable f A := by
  constructor
  · intro fixed v w h
    exact (fixed v).symm.trans ((possible_stable f A v w h).trans (fixed w))
  · intro stable w
    constructor
    · rintro ⟨v, h, ha⟩
      exact (stable v w h).mp ha
    · exact fun ha => ⟨w, rfl, ha⟩

theorem guaranteed_fixed_iff_dependency {Ω O : Type} (f : Ω → O) (A : Region Ω) :
    (∀ w, Guaranteed f A w ↔ A w) ↔ Stable f A := by
  constructor
  · intro fixed v w h
    exact (fixed v).symm.trans ((guaranteed_stable f A v w h).trans (fixed w))
  · intro stable w
    constructor
    · exact fun h => h w rfl
    · intro ha v h
      exact (stable v w h).mpr ha

/-- Possible is the least observable superset; Guaranteed the greatest
    observable subset. These are exact bounds in the Event containment order. -/
theorem best_observable_bounds {Ω O : Type} (f : Ω → O) (A B : Region Ω)
    (stable : Stable f B) :
    ((∀ w, Possible f A w → B w) ↔ (∀ w, A w → B w)) ∧
    ((∀ w, B w → Guaranteed f A w) ↔ (∀ w, B w → A w)) := by
  constructor
  · constructor
    · intro h w ha
      exact h w ⟨w, rfl, ha⟩
    · intro h w hp
      obtain ⟨v, hv, ha⟩ := hp
      exact (stable v w hv).mp (h v ha)
  · constructor
    · intro h w hb
      exact h w hb w rfl
    · intro h w hb v hv
      exact h v ((stable v w hv).mpr hb)

theorem possible_group_union {Ω O I : Type} (f : Ω → O) (A : I → Region Ω) (w : Ω) :
    Possible f (fun v => ∃ i, A i v) w ↔ ∃ i, Possible f (A i) w := by
  constructor
  · rintro ⟨v, hv, i, ha⟩
    exact ⟨i, v, hv, ha⟩
  · rintro ⟨i, v, hv, ha⟩
    exact ⟨v, hv, i, ha⟩

theorem guaranteed_group_intersection {Ω O I : Type}
    (f : Ω → O) (A : I → Region Ω) (w : Ω) :
    Guaranteed f (fun v => ∀ i, A i v) w ↔ ∀ i, Guaranteed f (A i) w := by
  exact ⟨fun h i v hv => h v hv i, fun h v hv i => h i v hv⟩

/-- A filter can cross this readout for every operand exactly when its
    membership is determined by the retained observation. -/
theorem possible_meet_iff_dependency {Ω O : Type} (f : Ω → O) (B : Region Ω) :
    (∀ A : Region Ω, ∀ w,
      Possible f (fun v => A v ∧ B v) w ↔ (Possible f A w ∧ B w)) ↔ Stable f B := by
  constructor
  · intro law
    apply (possible_fixed_iff_dependency f B).mp
    intro w
    constructor
    · rintro ⟨v, hv, hb⟩
      exact ((law (fun _ => True) w).mp ⟨v, hv, True.intro, hb⟩).2
    · intro hb
      exact ⟨w, rfl, hb⟩
  · intro stable A w
    constructor
    · rintro ⟨v, hv, ha, hb⟩
      exact ⟨⟨v, hv, ha⟩, (stable v w hv).mp hb⟩
    · rintro ⟨⟨v, hv, ha⟩, hb⟩
      exact ⟨v, hv, ha, (stable v w hv).mpr hb⟩

theorem guaranteed_join_iff_dependency {Ω O : Type} (f : Ω → O) (B : Region Ω) :
    (∀ A : Region Ω, ∀ w,
      Guaranteed f (fun v => A v ∨ B v) w ↔ (Guaranteed f A w ∨ B w)) ↔ Stable f B := by
  classical
  constructor
  · intro law
    apply (guaranteed_fixed_iff_dependency f B).mp
    intro w
    constructor
    · intro h
      exact h w rfl
    · intro hb v hv
      exact ((law (fun _ => False) w).mpr (Or.inr hb) v hv).resolve_left id
  · intro stable A w
    constructor
    · intro h
      by_cases hb : B w
      · exact Or.inr hb
      · exact Or.inl (fun v hv => (h v hv).resolve_right
          (fun bv => hb ((stable v w hv).mp bv)))
    · rintro (ha | hb) v hv
      · exact Or.inl (ha v hv)
      · exact Or.inr ((stable v w hv).mpr hb)

/-- The order on information readouts is exactly the FD between observations,
    not a relation on their cardinalities or probabilities. -/
theorem observation_order_iff_fd {Ω O P : Type} (f : Ω → O) (g : Ω → P) :
    (∀ A : Region Ω, ∀ w, Possible f A w → Possible g A w) ↔ Refines f g := by
  constructor
  · intro h v w same
    obtain ⟨x, hx, rfl⟩ := h (fun x => x = v) w ⟨v, same, rfl⟩
    exact hx
  · intro h A w hp
    obtain ⟨v, hv, ha⟩ := hp
    exact ⟨v, h v w hv, ha⟩

theorem coarser_absorbs_finer {Ω O P : Type} (f : Ω → O) (g : Ω → P)
    (refines : Refines f g) (A : Region Ω) (w : Ω) :
    (Possible g (Possible f A) w ↔ Possible g A w) ∧
    (Possible f (Possible g A) w ↔ Possible g A w) ∧
    (Guaranteed g (Guaranteed f A) w ↔ Guaranteed g A w) ∧
    (Guaranteed f (Guaranteed g A) w ↔ Guaranteed g A w) := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · constructor
    · rintro ⟨v, hv, x, hx, ha⟩
      exact ⟨x, (refines x v hx).trans hv, ha⟩
    · rintro ⟨v, hv, ha⟩
      exact ⟨v, hv, v, rfl, ha⟩
  · constructor
    · rintro ⟨v, hv, x, hx, ha⟩
      exact ⟨x, hx.trans (refines v w hv), ha⟩
    · intro h
      exact ⟨w, rfl, h⟩
  · constructor
    · intro h v hv
      exact h v hv v rfl
    · intro h v hv x hx
      exact h x ((refines x v hx).trans hv)
  · constructor
    · intro h
      exact h w rfl
    · intro h v hv x hx
      exact h x (hx.trans (refines v w hv))

/-- Independently possible alternatives need not have a joint witness; a union
    can be guaranteed even though neither term is guaranteed separately. -/
theorem grouping_counterexamples :
    let f : Bool → Unit := fun _ => ()
    let A : Region Bool := fun b => b = true
    let B : Region Bool := fun b => b = false
    (∀ w, Possible f A w ∧ Possible f B w ∧
      ¬ Possible f (fun v => A v ∧ B v) w) ∧
    (∀ w, Guaranteed f (fun v => A v ∨ B v) w ∧
      ¬ Guaranteed f A w ∧ ¬ Guaranteed f B w) := by
  dsimp
  constructor
  · intro w
    refine ⟨⟨true, rfl, rfl⟩, ⟨false, rfl, rfl⟩, ?_⟩
    rintro ⟨v, _, ha, hb⟩
    cases ha.symm.trans hb
  · intro w
    refine ⟨?_, ?_, ?_⟩
    · intro v _
      cases v <;> simp
    · intro h
      have bad := h false rfl
      cases bad
    · intro h
      have bad := h true rfl
      cases bad

#print axioms sandwich
#print axioms complement_duality
#print axioms possible_stable
#print axioms guaranteed_stable
#print axioms possible_fixed_iff_dependency
#print axioms guaranteed_fixed_iff_dependency
#print axioms best_observable_bounds
#print axioms possible_group_union
#print axioms guaranteed_group_intersection
#print axioms possible_meet_iff_dependency
#print axioms guaranteed_join_iff_dependency
#print axioms observation_order_iff_fd
#print axioms coarser_absorbs_finer
#print axioms grouping_counterexamples

end InformationReadout
