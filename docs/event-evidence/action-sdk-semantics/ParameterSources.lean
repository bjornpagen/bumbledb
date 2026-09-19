import Std

/-!
A sealed guard presentation denotes worlds (theta, outcome), retaining one real
parameter assignment. Feasible guard/outcome codes are a finite quotient, not
samples or an enumeration of parameter values. The correspondence below assumes
that guard feasibility and support uniformity were established by the constructor.
It does not verify Rust's source builder, arithmetic, wire parser or graph kernels.
-/
namespace ParameterSources

section Quotient
variable {Θ G Ω : Type} (domain : Θ → Prop) (guard : Θ → G)
variable (support : G → Ω → Prop)

def Feasible (g : G) : Prop := ∃ theta, domain theta ∧ guard theta=g

def RealWorld (theta : Θ) (outcome : Ω) : Prop :=
  domain theta ∧ support (guard theta) outcome

def LogicalCell (g : G) (outcome : Ω) : Prop :=
  Feasible domain guard g ∧ support g outcome

theorem world_has_cell (theta : Θ) (outcome : Ω)
    (h : RealWorld domain guard support theta outcome) :
    LogicalCell domain guard support (guard theta) outcome := by
  exact ⟨⟨theta, h.1, rfl⟩, h.2⟩

theorem every_cell_has_world (g : G) (outcome : Ω)
    (h : LogicalCell domain guard support g outcome) :
    ∃ theta, RealWorld domain guard support theta outcome ∧ guard theta=g := by
  obtain ⟨theta, hd, hg⟩ := h.1
  exact ⟨theta, ⟨hd, hg ▸ h.2⟩, hg⟩

theorem possible_iff_cell (event : G → Ω → Prop) :
    (∃ theta outcome, RealWorld domain guard support theta outcome ∧ event (guard theta) outcome) ↔
    (∃ g outcome, LogicalCell domain guard support g outcome ∧ event g outcome) := by
  constructor
  · rintro ⟨theta, outcome, hw, he⟩
    exact ⟨guard theta, outcome, world_has_cell domain guard support theta outcome hw, he⟩
  · rintro ⟨g, outcome, hc, he⟩
    obtain ⟨theta, hw, hg⟩ := every_cell_has_world domain guard support g outcome hc
    exact ⟨theta, outcome, hw, hg ▸ he⟩

theorem inclusion_iff_cell (a b : G → Ω → Prop) :
    (∀ theta outcome, RealWorld domain guard support theta outcome →
      a (guard theta) outcome → b (guard theta) outcome) ↔
    (∀ g outcome, LogicalCell domain guard support g outcome → a g outcome → b g outcome) := by
  constructor
  · intro h g outcome hc ha
    obtain ⟨theta, hw, hg⟩ := every_cell_has_world domain guard support g outcome hc
    exact hg ▸ h theta outcome hw (hg ▸ ha)
  · intro h theta outcome hw ha
    exact h (guard theta) outcome (world_has_cell domain guard support theta outcome hw) ha

theorem equality_iff_cell (a b : G → Ω → Prop) :
    (∀ theta outcome, RealWorld domain guard support theta outcome →
      (a (guard theta) outcome ↔ b (guard theta) outcome)) ↔
    (∀ g outcome, LogicalCell domain guard support g outcome → (a g outcome ↔ b g outcome)) := by
  constructor
  · intro h g outcome hc
    obtain ⟨theta, hw, hg⟩ := every_cell_has_world domain guard support g outcome hc
    exact hg ▸ h theta outcome hw
  · intro h theta outcome hw
    exact h (guard theta) outcome (world_has_cell domain guard support theta outcome hw)

/-- Restriction captures the subset of parameters still having an outcome. -/
theorem restricted_domain_has_fibre (event : G → Ω → Prop) (theta : Θ)
    (h : domain theta ∧ ∃ outcome, support (guard theta) outcome ∧ event (guard theta) outcome) :
    ∃ outcome, RealWorld domain guard support theta outcome ∧ event (guard theta) outcome := by
  obtain ⟨outcome, hs, he⟩ := h.2
  exact ⟨outcome, ⟨h.1, hs⟩, he⟩

end Quotient

section Maps
variable {Θ G H Ω Ψ : Type}

def KeepParameter (guard : Θ → G) (readout : G × Ω → H × Ψ) (world : Θ × Ω) : Θ × Ψ :=
  (world.1, (readout (guard world.1, world.2)).2)

theorem map_keeps_parameter (guard : Θ → G) (readout : G × Ω → H × Ψ) (world : Θ × Ω) :
    (KeepParameter guard readout world).1 = world.1 := rfl

theorem readout_commutes (guard : Θ → G) (targetGuard : Θ → H)
    (readout : G × Ω → H × Ψ) (theta : Θ) (outcome : Ω)
    (coherent : targetGuard theta = (readout (guard theta, outcome)).1) :
    (targetGuard theta, (KeepParameter guard readout (theta,outcome)).2) =
      readout (guard theta,outcome) := by
  simp only [KeepParameter]
  rw [coherent]

/-- The native map constructor checks this uniqueness through equality of exact
parameter cells. Matching coarse guard codes alone cannot provide it. -/
theorem image_preserves_actual_parameter
    (domain : Θ → Prop) (guard : Θ → G) (targetGuard : Θ → H)
    (support : G → Ω → Prop) (event : G → Ω → Prop)
    (readout : G × Ω → H × Ψ) (theta : Θ) (target : Ψ)
    (hd : domain theta)
    (uniqueGuard : ∀ g outcome, LogicalCell domain guard support g outcome →
      (readout (g,outcome)).1 = targetGuard theta → g=guard theta) :
    (∃ g outcome, LogicalCell domain guard support g outcome ∧ event g outcome ∧
      readout (g,outcome)=(targetGuard theta,target)) ↔
    (∃ outcome, support (guard theta) outcome ∧ event (guard theta) outcome ∧
      readout (guard theta,outcome)=(targetGuard theta,target)) := by
  constructor
  · rintro ⟨g, outcome, hc, he, hm⟩
    have hg := uniqueGuard g outcome hc (congrArg Prod.fst hm)
    exact ⟨outcome, hg ▸ hc.2, hg ▸ he, hg ▸ hm⟩
  · rintro ⟨outcome, hs, he, hm⟩
    exact ⟨guard theta, outcome, ⟨⟨theta,hd,rfl⟩,hs⟩,he,hm⟩

theorem guard_pattern_is_not_parameter_identity :
    let guard : Nat → Bool := fun _ => false
    guard 0=guard 1 ∧ (0 : Nat)≠1 := by decide

theorem one_logical_cell_can_have_arbitrarily_many_worlds (n : Nat) :
    let guard : Nat → Unit := fun _ => ()
    guard n=() ∧ guard (n+1)=() ∧ n≠n+1 := by simp

end Maps

section Laws
variable {K : Type} [Lean.Grind.Field K]

/-- A deterministic guard selects a row. It does not give both rows stochastic
multiplicity, including when their numeric values happen to agree. -/
theorem guard_selects_one (guard : Bool) (a b : K) :
    (if guard then a else 0) + (if guard then 0 else b) =
      (if guard then a else b) := by
  cases guard <;> simp <;> grind

theorem skipped_outcome_doubles (density : K) : density+density=2*density := by grind

end Laws

theorem shared_bias_zero_evidence (p : Rat) :
    2*p*(1-p)=0 ↔ p=0 ∨ p=1 := by grind

theorem shared_bias_fair_when_defined (p : Rat) (h : p≠0) (h1 : p≠1) :
    (p*(1-p))/(2*p*(1-p))=1/2 := by grind

theorem zero_mass_does_not_erase_outcome :
    let legal : Bool → Prop := fun _ => True
    let density : Bool → Rat := fun head => if head then 0 else 1
    legal true ∧ density true=0 ∧ density false+density true=1 := by decide +kernel

end ParameterSources

#print axioms ParameterSources.world_has_cell
#print axioms ParameterSources.every_cell_has_world
#print axioms ParameterSources.possible_iff_cell
#print axioms ParameterSources.inclusion_iff_cell
#print axioms ParameterSources.equality_iff_cell
#print axioms ParameterSources.restricted_domain_has_fibre
#print axioms ParameterSources.map_keeps_parameter
#print axioms ParameterSources.readout_commutes
#print axioms ParameterSources.image_preserves_actual_parameter
#print axioms ParameterSources.guard_pattern_is_not_parameter_identity
#print axioms ParameterSources.one_logical_cell_can_have_arbitrarily_many_worlds
#print axioms ParameterSources.guard_selects_one
#print axioms ParameterSources.skipped_outcome_doubles
#print axioms ParameterSources.shared_bias_zero_evidence
#print axioms ParameterSources.shared_bias_fair_when_defined
#print axioms ParameterSources.zero_mass_does_not_erase_outcome
