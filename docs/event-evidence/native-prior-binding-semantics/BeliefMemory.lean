import Std

/-!
Exact possibility memory for public action/observation histories. A state here
already contains every hidden coordinate and persistent environment affecting
the supplied transition model. This is support semantics, not Bayesian mass.
The native BFS, interning, symbolic maps and finite code compilation remain
tested correspondence obligations; no Rust refinement is asserted.
-/
namespace BeliefMemory

abbrev Region (S : Type) := S → Prop
abbrev Step (S A : Type) := S → A → S → Prop
def inhabited {S : Type} (B : Region S) := ∃ s, B s
def enabled {S A : Type} (R : Step S A) (B : Region S) (a : A) :=
  inhabited B ∧ ∀ s, B s → ∃ t, R s a t
def update {S A O : Type} (R : Step S A) (C : O → Region S)
    (B : Region S) (a : A) (o : O) : Region S :=
  fun t => C o t ∧ ∃ s, B s ∧ R s a t
def known {S : Type} (B G : Region S) := ∀ s, B s → G s

theorem update_is_exact_post_intersection {S A O : Type} (R : Step S A)
    (C : O → Region S) (B : Region S) (a : A) (o : O) (t : S) :
    update R C B a o t ↔ C o t ∧ ∃ s, B s ∧ R s a t := Iff.rfl

theorem actual_successor_remains_possible {S A O : Type} (R : Step S A)
    (C : O → Region S) (B : Region S) (a : A) (o : O) (s t : S)
    (prior : B s) (edge : R s a t) (observed : C o t) :
    update R C B a o t := ⟨observed, s, prior, edge⟩

theorem every_possible_successor_has_a_witness {S A O : Type} (R : Step S A)
    (C : O → Region S) (B : Region S) (a : A) (o : O) :
    ∀ t, update R C B a o t → ∃ s, B s ∧ R s a t := by
  exact fun _ h => h.2

theorem uniform_enabledness_is_greatest_sound_permission {S A : Type}
    (R : Step S A) (B : Region S) (P : A → Prop)
    (nonempty : inhabited B) (sound : ∀ a, P a → ∀ s, B s → ∃ t, R s a t) :
    ∀ a, P a → enabled R B a := by
  exact fun a allowed => ⟨nonempty, sound a allowed⟩

theorem empty_belief_has_no_enabled_action {S A : Type} (R : Step S A) (a : A) :
    ¬ enabled R (fun _ => False) a := by
  rintro ⟨⟨_, h⟩, _⟩
  exact h

theorem one_disabled_world_blocks_the_action {S A : Type} (R : Step S A)
    (B : Region S) (a : A) (s : S) (present : B s) (dead : ∀ t, ¬ R s a t) :
    ¬ enabled R B a := by
  rintro ⟨_, all⟩
  obtain ⟨t, edge⟩ := all s present
  exact dead t edge

theorem enabled_action_has_an_inhabited_observation {S A O : Type} (R : Step S A)
    (C : O → Region S) (covers : ∀ s, ∃ o, C o s)
    (B : Region S) (a : A) (live : enabled R B a) :
    ∃ o, inhabited (update R C B a o) := by
  obtain ⟨s, present⟩ := live.1
  obtain ⟨t, edge⟩ := live.2 s present
  obtain ⟨o, seen⟩ := covers t
  exact ⟨o, t, seen, s, present, edge⟩

theorem different_observations_have_disjoint_successors {S A O : Type}
    (R : Step S A) (C : O → Region S)
    (disjoint : ∀ i j s, C i s → C j s → i = j)
    (B : Region S) (a : A) (i j : O) (different : i ≠ j) :
    ∀ t, update R C B a i t → ¬ update R C B a j t := by
  exact fun t left right => different (disjoint i j t left.1 right.1)

theorem observation_splits_cover_exactly_the_post {S A O : Type}
    (R : Step S A) (C : O → Region S) (covers : ∀ s, ∃ o, C o s)
    (B : Region S) (a : A) (t : S) :
    (∃ o, update R C B a o t) ↔ ∃ s, B s ∧ R s a t := by
  constructor
  · exact fun ⟨_, h⟩ => h.2
  · intro h
    obtain ⟨o, seen⟩ := covers t
    exact ⟨o, seen, h⟩

theorem equal_beliefs_have_equal_updates {S A O : Type} (R : Step S A)
    (C : O → Region S) (B D : Region S) (same : ∀ s, B s ↔ D s)
    (a : A) (o : O) : ∀ t, update R C B a o t ↔ update R C D a o t := by
  intro t
  constructor
  · rintro ⟨seen, s, present, edge⟩
    exact ⟨seen, s, (same s).mp present, edge⟩
  · rintro ⟨seen, s, present, edge⟩
    exact ⟨seen, s, (same s).mpr present, edge⟩

theorem equal_beliefs_have_equal_permissions {S A : Type} (R : Step S A)
    (B D : Region S) (same : ∀ s, B s ↔ D s) (a : A) :
    enabled R B a ↔ enabled R D a := by
  constructor
  · rintro ⟨⟨s, present⟩, all⟩
    exact ⟨⟨s, (same s).mp present⟩, fun t p => all t ((same t).mpr p)⟩
  · rintro ⟨⟨s, present⟩, all⟩
    exact ⟨⟨s, (same s).mpr present⟩, fun t p => all t ((same t).mp p)⟩

def history {S A O : Type} (R : Step S A) (C : O → Region S)
    (B : Region S) : List (A × O) → Region S
  | [] => B
  | (a, o) :: rest => history R C (update R C B a o) rest

inductive Run {S A O : Type} (R : Step S A) (C : O → Region S) :
    S → List (A × O) → S → Prop
  | nil (s) : Run R C s [] s
  | cons {s u t a o rest} : R s a u → C o u → Run R C u rest t →
      Run R C s ((a, o) :: rest) t

theorem memory_is_exactly_the_worlds_consistent_with_the_entire_history
    {S A O : Type} (R : Step S A) (C : O → Region S) (h : List (A × O)) :
    ∀ (B : Region S) t, history R C B h t ↔ ∃ s, B s ∧ Run R C s h t := by
  induction h with
  | nil =>
    intro B t
    constructor
    · exact fun present => ⟨t, present, Run.nil t⟩
    · rintro ⟨s, present, path⟩
      cases path
      exact present
  | cons ao rest ih =>
    obtain ⟨a, o⟩ := ao
    intro B t
    change history R C (update R C B a o) rest t ↔ _
    rw [ih]
    constructor
    · rintro ⟨u, ⟨seen, s, present, edge⟩, path⟩
      exact ⟨s, present, Run.cons edge seen path⟩
    · rintro ⟨s, present, path⟩
      cases path with
      | cons edge seen tail => exact ⟨_, ⟨seen, s, present, edge⟩, tail⟩

theorem actual_history_never_reaches_an_empty_belief {S A O : Type}
    (R : Step S A) (C : O → Region S) (B : Region S) (s t : S)
    (h : List (A × O)) (present : B s) (path : Run R C s h t) :
    inhabited (history R C B h) := by
  exact ⟨t, (memory_is_exactly_the_worlds_consistent_with_the_entire_history R C h B t).mpr
    ⟨s, present, path⟩⟩

theorem initial_observation_is_evidence_intersection {S O : Type}
    (G : Region S) (C : O → Region S) (o : O) (s : S) :
    (fun t => G t ∧ C o t) s ↔ G s ∧ C o s := Iff.rfl

theorem retained_environment_witness_cannot_change {S A O E : Type}
    (R : Step S A) (C : O → Region S) (env : S → E)
    (preserves : ∀ s a t, R s a t → env s = env t)
    (B : Region S) (a : A) (o : O) :
    ∀ t, update R C B a o t → ∃ s, B s ∧ R s a t ∧ env s = env t := by
  rintro t ⟨_, s, present, edge⟩
  exact ⟨s, present, edge, preserves s a t edge⟩

theorem known_goal_is_true_for_every_consistent_actual_world {S A O : Type}
    (R : Step S A) (C : O → Region S) (B G : Region S)
    (h : List (A × O)) (certain : known (history R C B h) G) :
    ∀ s t, B s → Run R C s h t → G t := by
  intro s t present path
  exact certain t ((memory_is_exactly_the_worlds_consistent_with_the_entire_history R C h B t).mpr
    ⟨s, present, path⟩)

theorem all_observation_successors_safe_iff_all_actual_successors_safe
    {S A O : Type} (R : Step S A) (C : O → Region S)
    (covers : ∀ s, ∃ o, C o s) (B G : Region S) (a : A) :
    (∀ o, inhabited (update R C B a o) → known (update R C B a o) G) ↔
    (∀ s t, B s → R s a t → G t) := by
  constructor
  · intro all s t present edge
    obtain ⟨o, seen⟩ := covers t
    have next : update R C B a o t := ⟨seen, s, present, edge⟩
    exact all o ⟨t, next⟩ t next
  · intro all o _ t next
    obtain ⟨s, present, edge⟩ := next.2
    exact all s t present edge

theorem same_screen_does_not_identify_memories :
    (fun _ : Bool => ()) false = (fun _ : Bool => ()) true ∧
    ¬ (∀ s : Bool, (s = false) ↔ (s = true)) := by
  refine ⟨rfl, ?_⟩
  intro same
  exact Bool.false_ne_true ((same false).mp rfl)

#print axioms BeliefMemory.update_is_exact_post_intersection
#print axioms BeliefMemory.actual_successor_remains_possible
#print axioms BeliefMemory.every_possible_successor_has_a_witness
#print axioms BeliefMemory.uniform_enabledness_is_greatest_sound_permission
#print axioms BeliefMemory.empty_belief_has_no_enabled_action
#print axioms BeliefMemory.one_disabled_world_blocks_the_action
#print axioms BeliefMemory.enabled_action_has_an_inhabited_observation
#print axioms BeliefMemory.different_observations_have_disjoint_successors
#print axioms BeliefMemory.observation_splits_cover_exactly_the_post
#print axioms BeliefMemory.equal_beliefs_have_equal_updates
#print axioms BeliefMemory.equal_beliefs_have_equal_permissions
#print axioms BeliefMemory.memory_is_exactly_the_worlds_consistent_with_the_entire_history
#print axioms BeliefMemory.actual_history_never_reaches_an_empty_belief
#print axioms BeliefMemory.initial_observation_is_evidence_intersection
#print axioms BeliefMemory.retained_environment_witness_cannot_change
#print axioms BeliefMemory.known_goal_is_true_for_every_consistent_actual_world
#print axioms BeliefMemory.all_observation_successors_safe_iff_all_actual_successors_safe
#print axioms BeliefMemory.same_screen_does_not_identify_memories

end BeliefMemory
