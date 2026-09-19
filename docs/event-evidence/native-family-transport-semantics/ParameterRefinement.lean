import Std

/-!
Guard refinement changes a finite presentation of the same actual worlds. The
forgetful map below is a projection between logical codes, not permission to
forget the real parameter. Descent is accepted only when lifting the proposed
coarse Event recovers the complete fine Event. Surjectivity and commutation with
actual worlds are constructor obligations, not assertions about the Rust kernel.
-/
namespace ParameterRefinement

variable {C F W : Type}

def Lift (forget : F → C) (event : C → Prop) : F → Prop :=
  fun fine => event (forget fine)

def Envelope (forget : F → C) (event : F → Prop) : C → Prop :=
  fun coarse => ∃ fine, forget fine = coarse ∧ event fine

theorem lift_preserves_denotation (forget : F → C)
    (old : W → C) (refined : W → F)
    (sameWorld : ∀ w, forget (refined w) = old w)
    (event : C → Prop) (w : W) :
    Lift forget event (refined w) ↔ event (old w) := by
  simp only [Lift, sameWorld]

theorem lift_reflects_inclusion (forget : F → C)
    (onto : ∀ c, ∃ f, forget f=c) (a b : C → Prop) :
    (∀ f, Lift forget a f → Lift forget b f) ↔ (∀ c, a c → b c) := by
  constructor
  · intro h c ha
    obtain ⟨f,hf⟩ := onto c
    have lifted : Lift forget a f := by simpa only [Lift, hf] using ha
    exact hf ▸ h f lifted
  · intro h f ha
    exact h (forget f) ha

theorem lift_preserves_complement (forget : F → C) (event : C → Prop) (f : F) :
    Lift forget (fun c => ¬event c) f ↔ ¬Lift forget event f := Iff.rfl

theorem envelope_is_extensive (forget : F → C) (event : F → Prop) (f : F)
    (h : event f) : Lift forget (Envelope forget event) f :=
  ⟨f,rfl,h⟩

theorem envelope_after_lift (forget : F → C)
    (onto : ∀ c, ∃ f, forget f=c) (event : C → Prop) (c : C) :
    Envelope forget (Lift forget event) c ↔ event c := by
  constructor
  · rintro ⟨f,hf,he⟩
    exact hf ▸ he
  · intro he
    obtain ⟨f,hf⟩ := onto c
    exact ⟨f,hf,by simpa only [Lift, hf] using he⟩

theorem exact_descent_iff_guard_invariant (forget : F → C) (event : F → Prop) :
    (∀ f, Lift forget (Envelope forget event) f ↔ event f) ↔
    (∀ a b, forget a=forget b → (event a ↔ event b)) := by
  constructor
  · intro exact a b same
    constructor
    · intro ha
      exact (exact b).mp ⟨a,same,ha⟩
    · intro hb
      exact (exact a).mp ⟨b,same.symm,hb⟩
  · intro invariant f
    constructor
    · rintro ⟨a,same,ha⟩
      exact (invariant a f same).mp ha
    · exact envelope_is_extensive forget event f

theorem descent_is_unique (forget : F → C)
    (onto : ∀ c, ∃ f, forget f=c) (a b : C → Prop)
    (same : ∀ f, Lift forget a f ↔ Lift forget b f) (c : C) : a c ↔ b c := by
  obtain ⟨f,hf⟩ := onto c
  exact hf ▸ same f

theorem new_guard_cannot_descend :
    ¬∃ event : Unit → Prop, ∀ b : Bool, event () ↔ b=true := by
  rintro ⟨event,h⟩
  have positive := (h true).mpr rfl
  have impossible := (h false).mp positive
  contradiction

/-- For a fixed actual parameter, a new guard has one value. The outcome roster
and each density are unchanged, so no extra multiplicity enters contraction. -/
theorem density_roster_unchanged {G H O K : Type}
    (oldGuard : W → G) (newGuard : W → H)
    (density : G → O → K) (outcomes : List O) (w : W) :
    outcomes.map (fun o => (fun pair : G × H => density pair.1 o)
      (oldGuard w,newGuard w)) = outcomes.map (density (oldGuard w)) := rfl

end ParameterRefinement

#print axioms ParameterRefinement.lift_preserves_denotation
#print axioms ParameterRefinement.lift_reflects_inclusion
#print axioms ParameterRefinement.lift_preserves_complement
#print axioms ParameterRefinement.envelope_is_extensive
#print axioms ParameterRefinement.envelope_after_lift
#print axioms ParameterRefinement.exact_descent_iff_guard_invariant
#print axioms ParameterRefinement.descent_is_unique
#print axioms ParameterRefinement.new_guard_cannot_descend
#print axioms ParameterRefinement.density_roster_unchanged
