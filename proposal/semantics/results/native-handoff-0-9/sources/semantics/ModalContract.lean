import Std

/-!
Public relational operators over already-admitted legal state types. At a
fixed source environment, endpoint types are its legal fibres; this module
never existentially eliminates that environment. Representation/support/role
admission is a separate obligation proved in the laboratory modules.
-/
namespace ModalContract

abbrev Rel (S T : Type) := S → T → Prop
def comp {S T U : Type} (R : Rel S T) (Q : Rel T U) : Rel S U :=
  fun s u => ∃ t, R s t ∧ Q t u
def converse {S T : Type} (R : Rel S T) : Rel T S := fun t s => R s t
def domain {S T : Type} (R : Rel S T) (s : S) := ∃ t, R s t
def may {S T : Type} (R : Rel S T) (P : T → Prop) (s : S) := ∃ t, R s t ∧ P t
def all {S T : Type} (R : Rel S T) (P : T → Prop) (s : S) := ∀ t, R s t → P t
def must {S T : Type} (R : Rel S T) (P : T → Prop) (s : S) := domain R s ∧ all R P s
def leftResidual {S T U : Type} (R : Rel S T) (V : Rel S U) : Rel T U :=
  fun t u => ∀ s, R s t → V s u
def rightResidual {S T U : Type} (V : Rel S U) (Q : Rel T U) : Rel S T :=
  fun s t => ∀ u, Q t u → V s u
def included {S T : Type} (R Q : Rel S T) := ∀ s t, R s t → Q s t

theorem may_composition {S T U : Type} (R : Rel S T) (Q : Rel T U)
    (P : U → Prop) (s : S) : may (comp R Q) P s ↔ may R (may Q P) s := by
  constructor
  · rintro ⟨u, ⟨t, r, q⟩, p⟩; exact ⟨t, r, u, q, p⟩
  · rintro ⟨t, r, u, q, p⟩; exact ⟨u, ⟨t, r, q⟩, p⟩

theorem all_composition {S T U : Type} (R : Rel S T) (Q : Rel T U)
    (P : U → Prop) (s : S) : all (comp R Q) P s ↔ all R (all Q P) s := by
  constructor
  · intro h t r u q; exact h u ⟨t, r, q⟩
  · intro h u pair; obtain ⟨t, r, q⟩ := pair; exact h t r u q

/-- The missing premise for rewriting a nonvacuous box across composition. -/
theorem must_composition_exact {S T U : Type} (R : Rel S T) (Q : Rel T U)
    (P : U → Prop) (s : S) :
    must R (must Q P) s ↔ must (comp R Q) P s ∧ all R (domain Q) s := by
  constructor
  · rintro ⟨⟨t, r⟩, h⟩
    obtain ⟨u, q⟩ := (h t r).1
    exact ⟨⟨⟨u, t, r, q⟩, fun v pair => by
      obtain ⟨m, rm, qm⟩ := pair
      exact (h m rm).2 v qm⟩, fun m rm => (h m rm).1⟩
  · rintro ⟨⟨⟨u, t, r, _⟩, h⟩, enabled⟩
    exact ⟨⟨t, r⟩, fun m rm => ⟨enabled m rm, fun v qm => h v ⟨m, rm, qm⟩⟩⟩

theorem dead_end_distinction {S T : Type} (R : Rel S T) (P : T → Prop)
    (s : S) (dead : ¬ domain R s) : all R P s ∧ ¬ must R P s := by
  exact ⟨fun t r => False.elim (dead ⟨t, r⟩), fun h => dead h.1⟩

def branches : Rel Unit Bool := fun _ _ => True
def liveOnly : Rel Bool Unit := fun b _ => b = false

theorem must_composition_counterexample :
    must (comp branches liveOnly) (fun _ => True) () ∧
    ¬ must branches (must liveOnly (fun _ => True)) () := by
  constructor
  · exact ⟨⟨(), false, True.intro, rfl⟩, fun _ _ => True.intro⟩
  · intro h
    obtain ⟨_, impossible⟩ := (h.2 true True.intro).1
    exact Bool.noConfusion impossible

theorem left_adjunction {S T U : Type} (R : Rel S T) (Q : Rel T U) (V : Rel S U) :
    included (comp R Q) V ↔ included Q (leftResidual R V) := by
  constructor
  · intro h t u q s r; exact h s u ⟨t, r, q⟩
  · intro h s u pair; obtain ⟨t, r, q⟩ := pair; exact h t u q s r

theorem right_adjunction {S T U : Type} (R : Rel S T) (Q : Rel T U) (V : Rel S U) :
    included (comp R Q) V ↔ included R (rightResidual V Q) := by
  constructor
  · intro h s t r u q; exact h s u ⟨t, r, q⟩
  · intro h s u pair; obtain ⟨t, r, q⟩ := pair; exact h s t r u q

/-- Good must already include action enabledness and the requested safety goal. -/
def permitted {O S A : Type} (I : Rel O S) (Good : Rel S A) : Rel O A :=
  fun o a => domain I o ∧ leftResidual (converse I) Good o a

theorem permission_exact {O S A : Type} (I : Rel O S) (Good : Rel S A) (o : O) (a : A) :
    permitted I Good o a ↔ (∃ s, I o s) ∧ (∀ s, I o s → Good s a) := by
  exact Iff.rfl

theorem permission_sound {O S A : Type} (I : Rel O S) (Good : Rel S A) :
    included (comp (converse I) (permitted I Good)) Good := by
  intro s a pair
  obtain ⟨o, obs, permission⟩ := pair
  exact permission.2 s obs

theorem permission_greatest {O S A : Type} (I : Rel O S) (Good : Rel S A)
    (P : Rel O A) (inhabited : ∀ o a, P o a → domain I o)
    (safe : included (comp (converse I) P) Good) : included P (permitted I Good) := by
  intro o a p
  exact ⟨inhabited o a p, fun s obs => safe s a ⟨o, obs, p⟩⟩

theorem uniform_action_counterexample :
    (∀ s : Bool, ∃ a : Bool, s = a) ∧ ¬ (∃ a : Bool, ∀ s : Bool, s = a) := by
  constructor
  · intro s; exact ⟨s, rfl⟩
  · rintro ⟨a, h⟩
    exact Bool.false_ne_true ((h false).trans (h true).symm)

/-- Finite-path semantics. Finiteness/termination of an implementation is
    additional to these laws, even though these laws hold for any state type. -/
inductive Star {S : Type} (R : Rel S S) : Rel S S
  | refl (s) : Star R s s
  | step {s t u} : R s t → Star R t u → Star R s u

theorem star_transitive {S : Type} (R : Rel S S) {s t u : S}
    (a : Star R s t) (b : Star R t u) : Star R s u := by
  induction a with
  | refl => exact b
  | step r _ ih => exact Star.step r (ih b)

theorem star_least {S : Type} (R Q : Rel S S)
    (refl : ∀ s, Q s s) (trans : ∀ s t u, Q s t → Q t u → Q s u)
    (contains : included R Q) : included (Star R) Q := by
  intro s t path
  induction path with
  | refl s => exact refl s
  | step r _ ih => exact trans _ _ _ (contains _ _ r) ih

theorem star_may_unfold {S : Type} (R : Rel S S) (P : S → Prop) (s : S) :
    may (Star R) P s ↔ P s ∨ may R (may (Star R) P) s := by
  constructor
  · rintro ⟨t, path, p⟩
    cases path with
    | refl => exact Or.inl p
    | step r tail => exact Or.inr ⟨_, r, _, tail, p⟩
  · intro h
    rcases h with p | ⟨t, r, u, path, p⟩
    · exact ⟨s, Star.refl s, p⟩
    · exact ⟨u, Star.step r path, p⟩

theorem star_all_unfold {S : Type} (R : Rel S S) (P : S → Prop) (s : S) :
    all (Star R) P s ↔ P s ∧ all R (all (Star R) P) s := by
  constructor
  · intro h; exact ⟨h s (Star.refl s), fun _ r _ path => h _ (Star.step r path)⟩
  · rintro ⟨p, h⟩ t path
    cases path with
    | refl => exact p
    | step r tail => exact h _ r _ tail

end ModalContract
#print axioms ModalContract.may_composition
#print axioms ModalContract.all_composition
#print axioms ModalContract.must_composition_exact
#print axioms ModalContract.dead_end_distinction
#print axioms ModalContract.must_composition_counterexample
#print axioms ModalContract.left_adjunction
#print axioms ModalContract.right_adjunction
#print axioms ModalContract.permission_exact
#print axioms ModalContract.permission_sound
#print axioms ModalContract.permission_greatest
#print axioms ModalContract.uniform_action_counterexample
#print axioms ModalContract.star_transitive
#print axioms ModalContract.star_least
#print axioms ModalContract.star_may_unfold
#print axioms ModalContract.star_all_unfold
