import Std

/-!
The denotations selected by the native monotone programs. Finite detection and
extremality are proved in FixedPoint.lean; these laws identify the resulting
least/greatest solutions with paths, safety and well-founded forcing. Relations
below contain legal states and preserve any declared shared environment.
-/
namespace Closure

abbrev Rel (S : Type) := S → S → Prop
abbrev Region (S : Type) := S → Prop
inductive Path {S : Type} (R : Rel S) : Rel S where
  | refl (s) : Path R s s
  | step {s t u} : R s t → Path R t u → Path R s u

def step {S : Type} (R Q : Rel S) : Rel S :=
  fun s u => s = u ∨ ∃ t, R s t ∧ Q t u

theorem path_unfold {S : Type} (R : Rel S) (s u : S) :
    Path R s u ↔ step R (Path R) s u := by
  constructor
  · intro path
    cases path with
    | refl => exact Or.inl rfl
    | step edge tail => exact Or.inr ⟨_, edge, tail⟩
  · rintro (rfl | ⟨t, edge, tail⟩)
    · exact Path.refl _
    · exact Path.step edge tail

theorem path_is_least_preclosed {S : Type} (R Q : Rel S)
    (closed : ∀ s u, step R Q s u → Q s u) : ∀ s u, Path R s u → Q s u := by
  intro s u path
  induction path with
  | refl s => exact closed s s (Or.inl rfl)
  | step edge _ ih => exact closed _ _ (Or.inr ⟨_, edge, ih⟩)

theorem least_closure_program_is_path {S : Type} (R Q : Rel S)
    (closed : ∀ s u, step R Q s u → Q s u)
    (least : ∀ V : Rel S, (∀ s u, step R V s u → V s u) → ∀ s u, Q s u → V s u) :
    ∀ s u, Q s u ↔ Path R s u := by
  intro s u
  exact ⟨least (Path R) (fun s u => (path_unfold R s u).mpr) s u,
    path_is_least_preclosed R Q closed s u⟩

theorem paths_preserve_environment {S E : Type} (R : Rel S) (environment : S → E)
    (preserves : ∀ s t, R s t → environment s = environment t) :
    ∀ s t, Path R s t → environment s = environment t := by
  intro s t path
  induction path with
  | refl => rfl
  | step edge _ ih => exact (preserves _ _ edge).trans ih

def Reach {S : Type} (R : Rel S) (E : Region S) : Region S :=
  fun s => ∃ t, Path R s t ∧ E t
def reachStep {S : Type} (R : Rel S) (E X : Region S) : Region S :=
  fun s => E s ∨ ∃ t, R s t ∧ X t

theorem reach_is_preclosed {S : Type} (R : Rel S) (E : Region S) :
    ∀ s, reachStep R E (Reach R E) s → Reach R E s := by
  intro s h
  rcases h with goal | ⟨t, edge, u, path, goal⟩
  · exact ⟨s, Path.refl s, goal⟩
  · exact ⟨u, Path.step edge path, goal⟩

theorem reach_is_least_preclosed {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, reachStep R E P s → P s) : ∀ s, Reach R E s → P s := by
  rintro s ⟨t, path, goal⟩
  induction path with
  | refl s => exact closed s (Or.inl goal)
  | step edge _ ih => exact closed _ (Or.inr ⟨_, edge, ih goal⟩)

theorem least_reach_program_is_reach {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, reachStep R E P s → P s)
    (least : ∀ V : Region S, (∀ s, reachStep R E V s → V s) → ∀ s, P s → V s) :
    ∀ s, P s ↔ Reach R E s := by
  intro s
  exact ⟨least (Reach R E) (reach_is_preclosed R E) s,
    reach_is_least_preclosed R E P closed s⟩

def Safe {S : Type} (R : Rel S) (E : Region S) : Region S :=
  fun s => ∀ t, Path R s t → E t
def safeStep {S : Type} (R : Rel S) (E X : Region S) : Region S :=
  fun s => E s ∧ ∀ t, R s t → X t

theorem safe_is_postclosed {S : Type} (R : Rel S) (E : Region S) :
    ∀ s, Safe R E s → safeStep R E (Safe R E) s := by
  intro s safe
  exact ⟨safe s (Path.refl s), fun _ edge _ path => safe _ (Path.step edge path)⟩

theorem safe_is_greatest_postclosed {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, P s → safeStep R E P s) : ∀ s, P s → Safe R E s := by
  intro s present t path
  revert present
  induction path with
  | refl s => exact fun present => (closed s present).1
  | step edge _ ih => exact fun present => ih ((closed _ present).2 _ edge)

theorem greatest_safe_program_is_safe {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, P s → safeStep R E P s)
    (greatest : ∀ V : Region S, (∀ s, V s → safeStep R E V s) → ∀ s, V s → P s) :
    ∀ s, P s ↔ Safe R E s := by
  intro s
  exact ⟨safe_is_greatest_postclosed R E P closed s,
    greatest (Safe R E) (safe_is_postclosed R E) s⟩

/-- A finite proof tree reaches a goal or requires an enabled next step with
    every successor again forced. Dead ends outside the goal have no constructor. -/
inductive Forces {S : Type} (R : Rel S) (E : Region S) : Region S where
  | goal {s} : E s → Forces R E s
  | next {s} : (∃ t, R s t) → (∀ t, R s t → Forces R E t) → Forces R E s

def forceStep {S : Type} (R : Rel S) (E X : Region S) : Region S :=
  fun s => E s ∨ ((∃ t, R s t) ∧ ∀ t, R s t → X t)

theorem forcing_is_preclosed {S : Type} (R : Rel S) (E : Region S) :
    ∀ s, forceStep R E (Forces R E) s → Forces R E s := by
  intro s h
  rcases h with goal | ⟨enabled, all⟩
  · exact Forces.goal goal
  · exact Forces.next enabled all

theorem forcing_is_least_preclosed {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, forceStep R E P s → P s) : ∀ s, Forces R E s → P s := by
  intro s forced
  induction forced with
  | goal goal => exact closed _ (Or.inl goal)
  | next enabled _ ih => exact closed _ (Or.inr ⟨enabled, ih⟩)

theorem least_force_program_is_forcing {S : Type} (R : Rel S) (E P : Region S)
    (closed : ∀ s, forceStep R E P s → P s)
    (least : ∀ V : Region S, (∀ s, forceStep R E V s → V s) → ∀ s, P s → V s) :
    ∀ s, P s ↔ Forces R E s := by
  intro s
  exact ⟨least (Forces R E) (forcing_is_preclosed R E) s,
    forcing_is_least_preclosed R E P closed s⟩

theorem a_permanent_avoiding_cycle_is_not_forced :
    ¬ Forces (fun (_ _ : Unit) => True) (fun _ => False) () := by
  have impossible : ∀ s : Unit,
      Forces (fun (_ _ : Unit) => True) (fun _ => False) s → False := by
    intro s forced
    induction forced with
    | goal impossible => exact impossible
    | next _ _ ih => exact ih () True.intro
  exact impossible ()

#print axioms Closure.path_unfold
#print axioms Closure.path_is_least_preclosed
#print axioms Closure.least_closure_program_is_path
#print axioms Closure.paths_preserve_environment
#print axioms Closure.reach_is_preclosed
#print axioms Closure.reach_is_least_preclosed
#print axioms Closure.least_reach_program_is_reach
#print axioms Closure.safe_is_postclosed
#print axioms Closure.safe_is_greatest_postclosed
#print axioms Closure.greatest_safe_program_is_safe
#print axioms Closure.forcing_is_preclosed
#print axioms Closure.forcing_is_least_preclosed
#print axioms Closure.least_force_program_is_forcing
#print axioms Closure.a_permanent_avoiding_cycle_is_not_forced

end Closure
