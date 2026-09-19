import Std

/-!
Reference contract for completed numerical derivations at a query boundary.
D includes the expression and its full observation/source/evidence leaves.
An injective canonical encoding is a premise. No theorem equates numerical
values with derivation identity or verifies the Rust registry, Free Join,
spilling, N-API, or a particular arithmetic primitive's actual cost.
-/
namespace NumberQueries

def Injective {A B : Type} (f : A → B) := ∀ a b, f a = f b → a = b

def group {R K : Type} [DecidableEq K] (rows : List R) (key : R → K) (selected : K) :=
  rows.filter (fun r => decide (key r = selected))

theorem canonical_group_preserves_members {R D T : Type} [DecidableEq D] [DecidableEq T]
    (rows : List R) (derive : R → D) (encode : D → T) (faithful : Injective encode) (d : D) :
    group rows (fun r => encode (derive r)) (encode d) = group rows derive d := by
  unfold group
  have same : ∀ r, encode (derive r) = encode d ↔ derive r = d :=
    fun r => ⟨faithful (derive r) d, fun h => congrArg encode h⟩
  simp only [same]

theorem canonical_group_preserves_count {R D T : Type} [DecidableEq D] [DecidableEq T]
    (rows : List R) (derive : R → D) (encode : D → T) (faithful : Injective encode) (d : D) :
    (group rows (fun r => encode (derive r)) (encode d)).length =
      (group rows derive d).length := by
  rw [canonical_group_preserves_members rows derive encode faithful d]

/-- A later stage sees the same derivation after decoding its canonical token. -/
theorem stage_roundtrip_preserves_group {R D T : Type} [DecidableEq D]
    (rows : List R) (derive : R → D) (encode : D → T) (decode : T → D)
    (roundtrip : ∀ d, decode (encode d) = d) (selected : D) :
    group rows (fun r => decode (encode (derive r))) selected = group rows derive selected := by
  simp only [roundtrip]

/-- Per-row placeholders split one numerical group before an aggregate runs. -/
theorem fresh_tokens_change_counts :
    (group [0, 1] (fun _ : Nat => (7 : Nat)) 7).length = 2 ∧
      (group [0, 1] (id : Nat → Nat) 0).length = 1 := by decide

/-- Equal scalar results do not authorize erasing the complete derivation key. -/
theorem value_group_can_merge_distinct_derivations :
    (group [false, true] (fun _ : Bool => (1 : Nat)) 1).length = 2 ∧
      (group [false, true] (id : Bool → Bool) false).length = 1 := by decide

/-- Sequential primitive charging, including the work already done at refusal.
Cancellation/bit-bound failures are outside this arithmetic-step abstraction. -/
def spend (limit used cost : Nat) : Nat × Bool :=
  (min limit (used + cost), decide (used + cost ≤ limit))

theorem successful_spend_retains_cost (limit used cost : Nat) (fits : used + cost ≤ limit) :
    spend limit used cost = (used + cost, true) := by
  simp [spend, Nat.min_eq_right fits, fits]

theorem successful_stages_accumulate (limit used first second : Nat)
    (fits : used + first + second ≤ limit) :
    spend limit (spend limit used first).1 second = spend limit used (first + second) := by
  have firstFits : used + first ≤ limit := by omega
  rw [successful_spend_retains_cost limit used first firstFits]
  simp only [spend, Nat.add_assoc]

theorem failure_keeps_spent_prefix (limit used cost : Nat) (exhausted : limit < used + cost) :
    spend limit used cost = (limit, false) := by
  simp [spend, Nat.min_eq_left (Nat.le_of_lt exhausted), Nat.not_le_of_lt exhausted]

theorem failure_does_not_refund_next (limit used cost next : Nat)
    (exhausted : limit < used + cost) (positive : 0 < next) :
    (spend limit (spend limit used cost).1 next).2 = false := by
  rw [failure_keeps_spent_prefix limit used cost exhausted]
  simp [spend]
  omega

theorem resetting_stage_budget_admits_forbidden_work :
    (spend 3 0 2).2 = true ∧ (spend 3 0 2).2 = true ∧
      (spend 3 (spend 3 0 2).1 2).2 = false := by decide

#print axioms NumberQueries.canonical_group_preserves_members
#print axioms NumberQueries.canonical_group_preserves_count
#print axioms NumberQueries.stage_roundtrip_preserves_group
#print axioms NumberQueries.fresh_tokens_change_counts
#print axioms NumberQueries.value_group_can_merge_distinct_derivations
#print axioms NumberQueries.successful_spend_retains_cost
#print axioms NumberQueries.successful_stages_accumulate
#print axioms NumberQueries.failure_keeps_spent_prefix
#print axioms NumberQueries.failure_does_not_refund_next
#print axioms NumberQueries.resetting_stage_budget_admits_forbidden_work
end NumberQueries
