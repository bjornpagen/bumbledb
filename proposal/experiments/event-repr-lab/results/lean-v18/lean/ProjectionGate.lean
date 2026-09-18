import Std

namespace ProjectionGate

/-- A membership FD lets A ignore Y. The exact remaining obligation for
    eliminating Y's support gate is joint witness coverage, not independence. -/
theorem elimination_iff_coverage {X Y O P : Type}
    (S : X → Y → Prop) (D : X → Prop)
    (fx : X → O) (fy : Y → P) (x₀ : X) (y₀ : Y)
    (projects : ∀ x y, S x y → D x) :
    (∀ A : X → Prop,
      (∃ x y, S x y ∧ A x ∧ fx x = fx x₀ ∧ fy y = fy y₀) ↔
      (∃ x, D x ∧ A x ∧ fx x = fx x₀)) ↔
    (∀ x, D x → fx x = fx x₀ → ∃ y, S x y ∧ fy y = fy y₀) := by
  constructor
  · intro allA x hd hx
    obtain ⟨v, y, hs, same, _, hy⟩ :=
      (allA (fun v => v = x)).mpr ⟨x, hd, rfl, hx⟩
    exact ⟨y, same ▸ hs, hy⟩
  · intro coverage A
    constructor
    · rintro ⟨x, y, hs, ha, hx, _⟩
      exact ⟨x, projects x y hs, ha, hx⟩
    · rintro ⟨x, hd, ha, hx⟩
      obtain ⟨y, hs, hy⟩ := coverage x hd hx
      exact ⟨x, y, hs, ha, hx, hy⟩

/-- On product support, the target's admitted omitted coordinate supplies the
    witness for any observation of that coordinate, including partial masks. -/
theorem product_gate_elimination {X Y O P : Type}
    (D : X → Prop) (J : Y → Prop) (A : X → Prop)
    (fx : X → O) (fy : Y → P) (x₀ : X) (y₀ : Y) (legal : J y₀) :
    (∃ x y, (D x ∧ J y) ∧ A x ∧ fx x = fx x₀ ∧ fy y = fy y₀) ↔
    (∃ x, D x ∧ A x ∧ fx x = fx x₀) := by
  apply (elimination_iff_coverage (fun x y => D x ∧ J y) D fx fy x₀ y₀
    (fun _ _ h => h.1)).mpr
  intro x hx _
  exact ⟨y₀, ⟨hx, legal⟩, rfl⟩

/-- Equality only on admitted outputs becomes equality of every completed code.
    Each environment has its own product domain, retained on both sides. -/
theorem completed_product_gate {E X Y O P W : Type}
    (D : E → X → Prop) (J : E → Y → Prop) (A : E → X → Prop)
    (fx : X → O) (fy : Y → P)
    (env : W → E) (dx : W → X) (dy : W → Y)
    (lands : ∀ w, J (env w) (dy w)) :
    ∀ w,
      (∃ x y, (D (env w) x ∧ J (env w) y) ∧ A (env w) x ∧
        fx x = fx (dx w) ∧ fy y = fy (dy w)) ↔
      (∃ x, D (env w) x ∧ A (env w) x ∧ fx x = fx (dx w)) := by
  intro w
  exact product_gate_elimination _ _ _ _ _ _ _ (lands w)

/-- x determines Event membership, but copied support prevents gate removal. -/
theorem coupled_support_counterexample :
    ¬ (∃ x y : Bool, x = y ∧ x = true ∧ y = false) ∧
    (∃ x : Bool, x = true) := by decide

/-- Products inside each environment are insufficient if that environment is
    hidden: the legal omitted witness must still match the visible readout. -/
theorem hidden_environment_counterexample :
    ¬ (∃ e y : Bool, y = e ∧ e = true ∧ y = false) ∧
    (∃ e : Bool, e = true) := by decide

end ProjectionGate

#print axioms ProjectionGate.elimination_iff_coverage
#print axioms ProjectionGate.product_gate_elimination
#print axioms ProjectionGate.completed_product_gate
#print axioms ProjectionGate.coupled_support_counterexample
#print axioms ProjectionGate.hidden_environment_counterexample
