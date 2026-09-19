import Std

/-!
Grouped Event union. The carrier distinguishes an absent group from a present
empty Event. Canonical context selection is a minimum in a lawful total order;
Rust must implement this order on canonical full-space BEVT bytes. Claims retain
written-rule/variable identity before idempotent value aggregation. Computed keys
are partial: a key fault provides no arbitrary group, while later valid keys still
participate. These are reference laws, not a verification of scratch or Rust.
-/
namespace Pack

def unionOf {R W : Type} (event : R → W → Bool) : List R → W → Bool
  | [] => fun _ => false
  | r :: rs => fun w => event r w || unionOf event rs w

theorem union_exact {R W : Type} (event : R → W → Bool) (rows : List R) (w : W) :
    unionOf event rows w = true ↔ ∃ r ∈ rows, event r w = true := by
  induction rows with
  | nil => simp [unionOf]
  | cons r rs ih => simp [unionOf, ih]

theorem union_membership_invariant {R W : Type} (event : R → W → Bool)
    (a b : List R) (same : ∀ r, r ∈ a ↔ r ∈ b) : unionOf event a = unionOf event b := by
  funext w
  apply Bool.eq_iff_iff.mpr
  simp only [union_exact, same]

def value {R W : Type} (event : R → W → Bool) (rows : List R) : Option (W → Bool) :=
  if rows.isEmpty then none else some (unionOf event rows)

theorem absent_iff_no_rows {R W : Type} (event : R → W → Bool) (rows : List R) :
    value event rows = none ↔ rows = [] := by
  cases rows <;> simp [value]

theorem empty_seed_is_present {W : Type} :
    value (fun _ : Unit => fun _ : W => false) [()] = some (fun _ => false) := rfl

theorem duplicate_claim_is_idempotent {R W : Type} (event : R → W → Bool)
    (r : R) (rs : List R) : value event (r :: r :: rs) = value event (r :: rs) := by
  simp only [value, List.isEmpty_cons, Bool.false_eq_true, ↓reduceIte, Option.some.injEq]
  exact union_membership_invariant event _ _ (by intro x; simp)

theorem union_full_is_not_absent {W : Type} :
    value (fun _ : Unit => fun _ : W => true) [()] = some (fun _ => true) := by
  simp [value, unionOf]

section Context
variable {C : Type} [Min C] [LE C] [Std.IsLinearOrder C] [Std.LawfulOrderMin C]

def anchor {R : Type} (context : R → C) (rows : List R) : Option C :=
  (rows.map context).min?

theorem anchor_exact {R : Type} (context : R → C) (rows : List R) (c : C) :
    anchor context rows = some c ↔
      (∃ r ∈ rows, context r = c) ∧ ∀ r ∈ rows, c ≤ context r := by
  simp [anchor, List.min?_eq_some_iff]

omit [LE C] [Std.IsLinearOrder C] [Std.LawfulOrderMin C] in
theorem anchor_none_iff {R : Type} (context : R → C) (rows : List R) :
    anchor context rows = none ↔ rows = [] := by
  simp [anchor]

theorem anchor_membership_invariant {R : Type} (context : R → C)
    (a b : List R) (same : ∀ r, r ∈ a ↔ r ∈ b) : anchor context a = anchor context b := by
  cases ha : anchor context a with
  | none =>
    have empty := (anchor_none_iff context a).mp ha
    have emptyB : b = [] := by
      apply List.eq_nil_iff_forall_not_mem.mpr
      intro r member
      have := (same r).mpr member
      simp [empty] at this
    simp [anchor, emptyB]
  | some c =>
    symm
    apply (anchor_exact context b c).mpr
    simpa only [same] using (anchor_exact context a c).mp ha

def Fault {R : Type} (context : R → C) (rows : List R) (r : R) : Prop :=
  r ∈ rows ∧ ∃ c, anchor context rows = some c ∧ context r ≠ c

theorem faults_membership_invariant {R : Type} (context : R → C)
    (a b : List R) (same : ∀ r, r ∈ a ↔ r ∈ b) (r : R) :
    Fault context a r ↔ Fault context b r := by
  simp only [Fault, same, anchor_membership_invariant context a b same]

theorem clear_iff_common_context {R : Type} (context : R → C) (rows : List R) :
    (∀ r, ¬ Fault context rows r) ↔ ∀ a ∈ rows, ∀ b ∈ rows, context a = context b := by
  constructor
  · intro clear a ha b hb
    cases h : anchor context rows with
    | none => have := (anchor_none_iff context rows).mp h; simp [this] at ha
    | some c =>
      have eqc : ∀ r ∈ rows, context r = c := by
        intro r hr
        exact Classical.byContradiction (fun neq => clear r ⟨hr, c, h, neq⟩)
      exact (eqc a ha).trans (eqc b hb).symm
  · intro common r bad
    obtain ⟨member, c, expected, neq⟩ := bad
    obtain ⟨⟨other, present, same⟩, _⟩ := (anchor_exact context rows c).mp expected
    exact neq ((common r member other present).trans same)

end Context

theorem a_later_minimum_requires_retaining_earlier_claims :
    ¬ Fault (id : Nat → Nat) [2] 2 ∧ Fault (id : Nat → Nat) [2, 1] 2 := by simp [Fault, anchor, List.min?]

theorem first_visited_context_is_not_a_semantic_anchor :
    ([2, 1] : List Nat).head? ≠ [1, 2].head? ∧
      anchor id ([2, 1] : List Nat) = anchor id [1, 2] := by decide

theorem a_foreign_empty_claim_still_faults_after_full :
    unionOf (fun r : Nat => fun _ : Unit => r == 0) [0, 1] () = true ∧
      Fault (id : Nat → Nat) [0, 1] 1 := by simp [unionOf, Fault, anchor, List.min?]

/-- A claim's row identity includes written-rule provenance, not just its Event.
Rows 1 and 2 have the same Event/context but distinct written identities. -/
theorem value_dedup_can_destroy_fault_provenance :
    unionOf (fun _ : Nat => fun _ : Unit => true) [0, 1, 2] =
      unionOf (fun _ : Nat => fun _ : Unit => true) [0, 1] ∧
    Fault (fun r : Nat => if r = 0 then 0 else 1) [0, 1, 2] 2 ∧
    ¬ Fault (fun r : Nat => if r = 0 then 0 else 1) [0, 1] 2 := by
  constructor
  · funext w; rfl
  · simp [Fault, anchor, List.min?]

def members {R G : Type} [DecidableEq G] (key : R → Option G) (g : G) (rows : List R) :=
  rows.filter (fun r => decide (key r = some g))

theorem members_exact {R G : Type} [DecidableEq G]
    (key : R → Option G) (g : G) (rows : List R) (r : R) :
    r ∈ members key g rows ↔ r ∈ rows ∧ key r = some g := by simp [members]

theorem invalid_key_has_no_invented_group {R G : Type} [DecidableEq G]
    (key : R → Option G) (g : G) (rows : List R) (r : R) (invalid : key r = none) :
    r ∉ members key g rows := by simp [members_exact, invalid]

theorem prior_key_fault_cannot_suppress_later_participation {R G : Type} [DecidableEq G]
    (key : R → Option G) (g : G) (bad good : R) (invalid : key bad = none)
    (valid : key good = some g) : members key g [bad, good] = [good] := by
  simp [members, invalid, valid]

theorem group_reordering_preserves_context_and_faults {R G C : Type} [DecidableEq G]
    [Min C] [LE C] [Std.IsLinearOrder C] [Std.LawfulOrderMin C]
    (key : R → Option G) (context : R → C) (g : G) (a b : List R)
    (same : ∀ r, r ∈ a ↔ r ∈ b) (r : R) :
    Fault context (members key g a) r ↔ Fault context (members key g b) r := by
  apply faults_membership_invariant
  intro x
  simp only [members_exact, same]

#print axioms Pack.union_exact
#print axioms Pack.union_membership_invariant
#print axioms Pack.absent_iff_no_rows
#print axioms Pack.empty_seed_is_present
#print axioms Pack.duplicate_claim_is_idempotent
#print axioms Pack.union_full_is_not_absent
#print axioms Pack.anchor_exact
#print axioms Pack.anchor_none_iff
#print axioms Pack.anchor_membership_invariant
#print axioms Pack.faults_membership_invariant
#print axioms Pack.clear_iff_common_context
#print axioms Pack.a_later_minimum_requires_retaining_earlier_claims
#print axioms Pack.first_visited_context_is_not_a_semantic_anchor
#print axioms Pack.a_foreign_empty_claim_still_faults_after_full
#print axioms Pack.value_dedup_can_destroy_fault_provenance
#print axioms Pack.members_exact
#print axioms Pack.invalid_key_has_no_invented_group
#print axioms Pack.prior_key_fault_cannot_suppress_later_participation
#print axioms Pack.group_reordering_preserves_context_and_faults

end Pack
