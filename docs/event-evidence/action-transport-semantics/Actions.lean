import Std

/-!
Controlled finite transitions, inhabited uniform permissions and retained
strategy witnesses. These reference types are already legal endpoint fibres
over one shared environment. Native product/map admission, canonical kernels,
fixed-point execution and layer extraction remain correspondence obligations.

Winning reachability must retain actions that decrease a natural rank, not
arbitrary actions inside the winning set. Safety requires enabled continuation.
Fully observed state strategies do not infer a hidden actor's information/memory.
-/
namespace Actions

abbrev Region (S : Type) := S → Prop
abbrev Rel (S A : Type) := S → A → Prop
abbrev Step (S A T : Type) := S → A → T → Prop
def enabled {S A T : Type} (R : Step S A T) : Rel S A := fun s a => ∃ t, R s a t
def good {S A T : Type} (R : Step S A T) (E : Region T) : Rel S A :=
  fun s a => enabled R s a ∧ ∀ t, R s a t → E t
def cpre {S A T : Type} (R : Step S A T) (E : Region T) : Region S :=
  fun s => ∃ a, good R E s a

theorem flattened_must_then_image {S A T : Type} (R : Step S A T) (E : Region T) (s : S) :
    (∃ pair : S × A, pair.1 = s ∧
      (∃ t, R pair.1 pair.2 t) ∧ (∀ t, R pair.1 pair.2 t → E t)) ↔ cpre R E s := by
  constructor
  · rintro ⟨⟨state, action⟩, rfl, live, all⟩
    exact ⟨action, live, all⟩
  · rintro ⟨action, live, all⟩
    exact ⟨(s, action), rfl, live, all⟩

theorem cpre_monotone {S A T : Type} (R : Step S A T) (E F : Region T)
    (inc : ∀ t, E t → F t) : ∀ s, cpre R E s → cpre R F s := by
  rintro s ⟨a, live, all⟩
  exact ⟨a, live, fun t edge => inc t (all t edge)⟩

theorem no_action_for_empty_goal {S A T : Type} (R : Step S A T) (s : S) :
    ¬ cpre R (fun _ => False) s := by
  rintro ⟨a, ⟨t, edge⟩, all⟩
  exact all t edge

theorem initial_reach_step_is_goal {S A : Type} (R : Step S A S) (G : Region S) :
    ∀ s, (G s ∨ cpre R (fun _ => False) s) ↔ G s := by
  intro s
  exact ⟨fun h => h.resolve_right (no_action_for_empty_goal R s), Or.inl⟩

theorem enabled_retains_environment {S A T E : Type} (R : Step S A T)
    (stateEnv : S → E) (actionEnv : A → E) (outcomeEnv : T → E)
    (preserves : ∀ s a t, R s a t → stateEnv s = actionEnv a ∧ stateEnv s = outcomeEnv t) :
    ∀ s a, enabled R s a → stateEnv s = actionEnv a := by
  rintro s a ⟨t, edge⟩
  exact (preserves s a t edge).1

def permitted {O S A : Type} (I : Rel O S) (Good : Rel S A) : Rel O A :=
  fun o a => (∃ s, I o s) ∧ ∀ s, I o s → Good s a

theorem permissions_are_inhabited_and_sound {O S A : Type} (I : Rel O S)
    (Good : Rel S A) :
    (∀ o a, permitted I Good o a → ∃ s, I o s) ∧
    (∀ s o a, I o s → permitted I Good o a → Good s a) := by
  exact ⟨fun _ _ p => p.1, fun s _ _ seen p => p.2 s seen⟩

theorem permissions_are_greatest {O S A : Type} (I : Rel O S) (Good : Rel S A)
    (Q : Rel O A) (inhabited : ∀ o a, Q o a → ∃ s, I o s)
    (sound : ∀ s o a, I o s → Q o a → Good s a) :
    ∀ o a, Q o a → permitted I Good o a := by
  exact fun o a q => ⟨inhabited o a q, fun s seen => sound s o a seen q⟩

theorem different_hidden_actions_do_not_form_a_uniform_choice :
    (∀ s : Bool, ∃ a : Bool, s = a) ∧ ¬ (∃ a : Bool, ∀ s : Bool, s = a) := by
  exact ⟨fun s => ⟨s, rfl⟩, fun ⟨_, all⟩ => Bool.false_ne_true ((all false).trans (all true).symm)⟩

def earlier {S : Type} (layer : Nat → Region S) (n : Nat) : Region S :=
  fun s => ∃ k, k < n ∧ layer k s
def rankPolicy {S A : Type} (R : Step S A S) (layer : Nat → Region S) : Rel S A :=
  fun s a => ∃ n, 0 < n ∧ layer n s ∧ good R (earlier layer n) s a

theorem first_entry_admits_an_earlier_action {S A : Type} (R : Step S A S)
    (G previous next : Region S) (goalAlready : ∀ s, G s → previous s)
    (step : ∀ s, next s ↔ G s ∨ cpre R previous s) :
    ∀ s, next s → ¬ previous s → ∃ a, good R previous s a := by
  intro s present fresh
  rcases (step s).mp present with goal | action
  · exact (fresh (goalAlready s goal)).elim
  · exact action

theorem ranked_choices_are_enabled {S A : Type} (R : Step S A S)
    (layer : Nat → Region S) : ∀ s a, rankPolicy R layer s a → enabled R s a := by
  exact fun _ _ ⟨_, _, _, available, _⟩ => available

theorem ranked_choices_strictly_decrease {S A : Type} (R : Step S A S)
    (layer : Nat → Region S) (Win : Region S) (rank : S → Nat)
    (meaning : ∀ n s, layer n s → Win s ∧ rank s = n) :
    ∀ s a t, rankPolicy R layer s a → R s a t → Win t ∧ rank t < rank s := by
  rintro s a t ⟨n, _, present, _, all⟩ edge
  obtain ⟨k, below, target⟩ := all t edge
  have sourceRank := meaning n s present
  have dest := meaning k t target
  exact ⟨dest.1, by omega⟩

theorem ranked_policy_domain {S A : Type} (R : Step S A S)
    (layer : Nat → Region S) (G Win : Region S)
    (covers : ∀ s, Win s ↔ ∃ n, layer n s)
    (unique : ∀ i j s, layer i s → layer j s → i = j)
    (zero : ∀ s, layer 0 s ↔ G s)
    (progress : ∀ n s, 0 < n → layer n s → ∃ a, good R (earlier layer n) s a) :
    ∀ s, (∃ a, rankPolicy R layer s a) ↔ Win s ∧ ¬ G s := by
  intro s
  constructor
  · rintro ⟨a, n, positive, present, _⟩
    refine ⟨(covers s).mpr ⟨n, present⟩, ?_⟩
    intro goal
    have eq := unique n 0 s present ((zero s).mpr goal)
    omega
  · rintro ⟨won, notGoal⟩
    obtain ⟨n, present⟩ := (covers s).mp won
    have positive : 0 < n := by
      have nonzero : n ≠ 0 := fun eq => notGoal ((zero s).mp (eq ▸ present))
      omega
    obtain ⟨a, available⟩ := progress n s positive present
    exact ⟨a, n, positive, present, available⟩

/-- Every policy choice and every legal outcome finishes within the fuel.
    Goal states may stop; outside the goal both policy and actions are enabled. -/
inductive TerminatesWithin {S A : Type} (R : Step S A S) (P : Rel S A) (G : Region S) : Nat → S → Prop
  | goal {n s} : G s → TerminatesWithin R P G n s
  | next {n s} : (∃ a, P s a) → (∀ a, P s a → enabled R s a) →
      (∀ a t, P s a → R s a t → TerminatesWithin R P G n t) →
      TerminatesWithin R P G (n + 1) s

theorem decreasing_rank_guarantees_every_policy_run {S A : Type} (R : Step S A S)
    (P : Rel S A) (G Win : Region S) (rank : S → Nat)
    (total : ∀ s, Win s → ¬ G s → ∃ a, P s a)
    (available : ∀ s a, P s a → enabled R s a)
    (decreases : ∀ s a t, P s a → R s a t → Win t ∧ rank t < rank s) :
    ∀ s, Win s → TerminatesWithin R P G (rank s) s := by
  classical
  have bounded : ∀ n s, rank s ≤ n → Win s → TerminatesWithin R P G n s := by
    intro n
    induction n with
    | zero =>
      intro s bound won
      by_cases goal : G s
      · exact TerminatesWithin.goal goal
      · obtain ⟨a, allowed⟩ := total s won goal
        obtain ⟨t, edge⟩ := available s a allowed
        have strict := (decreases s a t allowed edge).2
        omega
    | succ n ih =>
      intro s bound won
      by_cases goal : G s
      · exact TerminatesWithin.goal goal
      · refine TerminatesWithin.next (total s won goal) (available s) ?_
        intro a t allowed edge
        have dest := decreases s a t allowed edge
        exact ih t (by omega) dest.1
  exact fun s won => bounded (rank s) s (Nat.le_refl _) won

theorem policy_restriction_retains_progress {S A : Type} (R : Step S A S)
    (P Q : Rel S A) (G Win : Region S) (rank : S → Nat)
    (subset : ∀ s a, Q s a → P s a)
    (total : ∀ s, Win s → ¬ G s → ∃ a, Q s a)
    (available : ∀ s a, P s a → enabled R s a)
    (decreases : ∀ s a t, P s a → R s a t → Win t ∧ rank t < rank s) :
    ∀ s, Win s → TerminatesWithin R Q G (rank s) s := by
  exact decreasing_rank_guarantees_every_policy_run R Q G Win rank total
    (fun s a q => available s a (subset s a q))
    (fun s a t q edge => decreases s a t (subset s a q) edge)

theorem every_bounded_winning_policy_is_inside_a_reach_prefixed_point {S A : Type}
    (R : Step S A S) (P : Rel S A) (G X : Region S)
    (closed : ∀ s, G s ∨ cpre R X s → X s) :
    ∀ n s, TerminatesWithin R P G n s → X s := by
  intro n s terminates
  induction terminates with
  | goal goal => exact closed _ (Or.inl goal)
  | next chosen available _ ih =>
    obtain ⟨a, allowed⟩ := chosen
    exact closed _ (Or.inr ⟨a, available a allowed, fun t edge => ih a t allowed edge⟩)

def safePolicy {S A : Type} (R : Step S A S) (Win : Region S) : Rel S A :=
  fun s a => Win s ∧ good R Win s a

theorem safe_policy_covers_the_postfixed_region {S A : Type} (R : Step S A S)
    (Win : Region S) (closed : ∀ s, Win s → cpre R Win s) :
    ∀ s, (∃ a, safePolicy R Win s a) ↔ Win s := by
  intro s
  constructor
  · exact fun ⟨_, won, _⟩ => won
  · intro won
    obtain ⟨a, good⟩ := closed s won
    exact ⟨a, won, good⟩

theorem safe_policy_always_has_an_enabled_next_step {S A : Type} (R : Step S A S)
    (Win : Region S) (closed : ∀ s, Win s → cpre R Win s) :
    ∀ s, Win s → ∃ a t, safePolicy R Win s a ∧ R s a t ∧ Win t := by
  intro s won
  obtain ⟨a, ⟨t, edge⟩, all⟩ := closed s won
  exact ⟨a, t, ⟨won, ⟨t, edge⟩, all⟩, edge, all t edge⟩

inductive PolicyPath {S A : Type} (R : Step S A S) (P : Rel S A) : S → S → Prop
  | refl (s) : PolicyPath R P s s
  | step {s t u a} : P s a → R s a t → PolicyPath R P t u → PolicyPath R P s u

theorem safe_policy_preserves_the_invariant_on_every_path {S A : Type}
    (R : Step S A S) (P : Rel S A) (Win E : Region S)
    (within : ∀ s, Win s → E s)
    (policy : ∀ s a, P s a → safePolicy R Win s a) :
    ∀ s t, Win s → PolicyPath R P s t → E t := by
  intro s t won path
  induction path with
  | refl s => exact within s won
  | step allowed edge _ ih => exact ih ((policy _ _ allowed).2.2 _ edge)

theorem safety_greatest_contains_every_enabled_invariant {S A : Type}
    (R : Step S A S) (Win E : Region S)
    (greatest : ∀ X : Region S, (∀ s, X s → E s ∧ cpre R X s) → ∀ s, X s → Win s)
    (X : Region S) (P : Rel S A) (within : ∀ s, X s → E s)
    (total : ∀ s, X s → ∃ a, P s a)
    (sound : ∀ s a, P s a → good R X s a) : ∀ s, X s → Win s := by
  apply greatest X
  intro s present
  obtain ⟨a, allowed⟩ := total s present
  exact ⟨within s present, a, sound s a allowed⟩

/- Replay is an extensional source/role obligation. These theorems assume
   equivalent legal Step and objective meanings; they do not verify a codec. -/
theorem replay_preserves_controlled_predecessor {S A T : Type}
    (R Q : Step S A T) (E F : Region T)
    (edges : ∀ s a t, R s a t ↔ Q s a t)
    (objective : ∀ t, E t ↔ F t) :
    ∀ s, cpre R E s ↔ cpre Q F s := by
  have equalR : R = Q := funext fun s => funext fun a => funext fun t => propext (edges s a t)
  have equalE : E = F := funext fun t => propext (objective t)
  subst Q; subst F
  exact fun _ => Iff.rfl

def reachIterate {S A : Type} (R : Step S A S) (G : Region S) : Nat → Region S
  | 0 => fun _ => False
  | n + 1 => fun s => G s ∨ cpre R (reachIterate R G n) s

def safeIterate {S A : Type} (R : Step S A S) (E : Region S) : Nat → Region S
  | 0 => fun _ => True
  | n + 1 => fun s => E s ∧ cpre R (safeIterate R E n) s

theorem replay_preserves_reach_iterates {S A : Type}
    (R Q : Step S A S) (G H : Region S)
    (edges : ∀ s a t, R s a t ↔ Q s a t)
    (objective : ∀ s, G s ↔ H s) :
    ∀ n s, reachIterate R G n s ↔ reachIterate Q H n s := by
  intro n
  induction n with
  | zero => exact fun _ => Iff.rfl
  | succ n ih =>
    intro s
    exact or_congr (objective s)
      (replay_preserves_controlled_predecessor R Q _ _ edges ih s)

theorem replay_preserves_safety_iterates {S A : Type}
    (R Q : Step S A S) (E F : Region S)
    (edges : ∀ s a t, R s a t ↔ Q s a t)
    (objective : ∀ s, E s ↔ F s) :
    ∀ n s, safeIterate R E n s ↔ safeIterate Q F n s := by
  intro n
  induction n with
  | zero => exact fun _ => Iff.rfl
  | succ n ih =>
    intro s
    exact and_congr (objective s)
      (replay_preserves_controlled_predecessor R Q _ _ edges ih s)

/-- These are exactly the subset and domain-equality conditions checked after
    reconstructing a reach policy. Missing winning states cannot pass. -/
theorem checked_restriction_retains_termination_bound {S A : Type}
    (R : Step S A S) (P Q : Rel S A) (G Win : Region S) (rank : S → Nat)
    (subset : ∀ s a, Q s a → P s a)
    (domain : ∀ s, (∃ a, Q s a) ↔ ∃ a, P s a)
    (total : ∀ s, Win s → ¬ G s → ∃ a, P s a)
    (available : ∀ s a, P s a → enabled R s a)
    (decreases : ∀ s a t, P s a → R s a t → Win t ∧ rank t < rank s) :
    ∀ s, Win s → TerminatesWithin R Q G (rank s) s := by
  exact policy_restriction_retains_progress R P Q G Win rank subset
    (fun s won notGoal => (domain s).mpr (total s won notGoal)) available decreases

/-- A checked safety restriction cannot become a dead end; every retained
    choice remains enabled and all its outcomes remain winning. -/
theorem checked_safety_restriction_retains_continuation {S A : Type}
    (R : Step S A S) (Win : Region S) (Q : Rel S A)
    (closed : ∀ s, Win s → cpre R Win s)
    (subset : ∀ s a, Q s a → safePolicy R Win s a)
    (domain : ∀ s, (∃ a, Q s a) ↔ ∃ a, safePolicy R Win s a) :
    (∀ s, Win s → ∃ a, Q s a) ∧ (∀ s a, Q s a → good R Win s a) := by
  refine ⟨?_, fun s a chosen => (subset s a chosen).2⟩
  intro s won
  exact (domain s).mpr ((safe_policy_covers_the_postfixed_region R Win closed s).mpr won)

#print axioms Actions.replay_preserves_controlled_predecessor
#print axioms Actions.replay_preserves_reach_iterates
#print axioms Actions.replay_preserves_safety_iterates
#print axioms Actions.checked_restriction_retains_termination_bound
#print axioms Actions.checked_safety_restriction_retains_continuation

#print axioms Actions.flattened_must_then_image
#print axioms Actions.cpre_monotone
#print axioms Actions.no_action_for_empty_goal
#print axioms Actions.initial_reach_step_is_goal
#print axioms Actions.enabled_retains_environment
#print axioms Actions.permissions_are_inhabited_and_sound
#print axioms Actions.permissions_are_greatest
#print axioms Actions.different_hidden_actions_do_not_form_a_uniform_choice
#print axioms Actions.first_entry_admits_an_earlier_action
#print axioms Actions.ranked_choices_are_enabled
#print axioms Actions.ranked_choices_strictly_decrease
#print axioms Actions.ranked_policy_domain
#print axioms Actions.decreasing_rank_guarantees_every_policy_run
#print axioms Actions.policy_restriction_retains_progress
#print axioms Actions.every_bounded_winning_policy_is_inside_a_reach_prefixed_point
#print axioms Actions.safe_policy_covers_the_postfixed_region
#print axioms Actions.safe_policy_always_has_an_enabled_next_step
#print axioms Actions.safe_policy_preserves_the_invariant_on_every_path
#print axioms Actions.safety_greatest_contains_every_enabled_invariant

end Actions
