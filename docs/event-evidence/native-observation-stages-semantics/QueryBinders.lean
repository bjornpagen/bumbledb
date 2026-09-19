import Std

/-!
Scoped query fixed points: extremal solutions vary monotonically with their
captured inputs, including when those inputs are outer bound predicates. The
reference uses predicates and the complete-lattice characterization; finite
stabilization and successful bounded detection are proved in FixedPoint.lean.
No theorem here claims refinement of Rust, de Bruijn transport bytes, JS handle
hygiene, cancellation, or allocation. Those remain native correspondence tests.
-/
namespace QueryBinders
abbrev Region (W : Type) := W → Prop
def included {W : Type} (a b : Region W) := ∀ w, a w → b w
def monotone {W : Type} (F : Region W → Region W) :=
  ∀ a b, included a b → included (F a) (F b)
def least {W : Type} (F : Region W → Region W) : Region W :=
  fun w => ∀ p, included (F p) p → p w
def greatest {W : Type} (F : Region W → Region W) : Region W :=
  fun w => ∃ p, included p (F p) ∧ p w

theorem least_below_prefixed {W : Type} (F : Region W → Region W)
    (p : Region W) (closed : included (F p) p) : included (least F) p :=
  fun _ present => present p closed

theorem least_prefixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    included (F (least F)) (least F) := by
  intro w present p closed
  exact closed w (mono _ _ (least_below_prefixed F p closed) w present)

theorem least_postfixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    included (least F) (F (least F)) :=
  least_below_prefixed F _ (mono _ _ (least_prefixed F mono))

theorem least_fixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    F (least F) = least F := by
  funext w
  exact propext ⟨least_prefixed F mono w, least_postfixed F mono w⟩

theorem postfixed_below_greatest {W : Type} (F : Region W → Region W)
    (p : Region W) (closed : included p (F p)) : included p (greatest F) :=
  fun _ present => ⟨p, closed, present⟩

theorem greatest_postfixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    included (greatest F) (F (greatest F)) := by
  rintro w ⟨p, closed, present⟩
  exact mono _ _ (postfixed_below_greatest F p closed) w (closed w present)

theorem greatest_prefixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    included (F (greatest F)) (greatest F) :=
  postfixed_below_greatest F _ (mono _ _ (greatest_postfixed F mono))

theorem greatest_fixed {W : Type} (F : Region W → Region W) (mono : monotone F) :
    F (greatest F) = greatest F := by
  funext w
  exact propext ⟨greatest_prefixed F mono w, greatest_postfixed F mono w⟩

/-- The outer predicate changes a whole inner operator pointwise. Its least
    solution preserves that direction; no distribution or resampling occurs. -/
theorem least_operator_covariant {W : Type} (F G : Region W → Region W)
    (ordered : ∀ p, included (F p) (G p)) (monoG : monotone G) :
    included (least F) (least G) := by
  apply least_below_prefixed
  exact fun w present => least_prefixed G monoG w (ordered _ w present)

theorem greatest_operator_covariant {W : Type} (F G : Region W → Region W)
    (ordered : ∀ p, included (F p) (G p)) (monoF : monotone F) :
    included (greatest F) (greatest G) := by
  apply postfixed_below_greatest
  exact fun w present => ordered _ w (greatest_postfixed F monoF w present)

/-- Independent or decreasing parameters follow the same order law by equality
    or by reversing the two assignments. Every binder still requires positivity
    in its own predicate, separately from its variance in outer predicates. -/
theorem nested_least_monotone {X W : Type} (body : Region X → Region W → Region W)
    (inner : ∀ x, monotone (body x))
    (outer : ∀ x y, included x y → ∀ p, included (body x p) (body y p)) :
    ∀ x y, included x y → included (least (body x)) (least (body y)) := by
  intro x y ordered
  exact least_operator_covariant _ _ (outer x y ordered) (inner y)

theorem nested_greatest_monotone {X W : Type} (body : Region X → Region W → Region W)
    (inner : ∀ x, monotone (body x))
    (outer : ∀ x y, included x y → ∀ p, included (body x p) (body y p)) :
    ∀ x y, included x y → included (greatest (body x)) (greatest (body y)) := by
  intro x y ordered
  exact greatest_operator_covariant _ _ (outer x y ordered) (inner x)

/-- Lexical extension shadows only depth zero; older predicates are shifted.
    Row bindings have a distinct type/lookup and do not occupy this stack. -/
def push {A : Type} (x : A) (env : Nat → A) : Nat → A
  | 0 => x
  | n + 1 => env n

theorem nearest_predicate {A : Type} (x : A) (env : Nat → A) : push x env 0 = x := rfl
theorem outer_predicate {A : Type} (x y : A) (env : Nat → A) :
    push y (push x env) 1 = x := rfl
theorem older_predicate {A : Type} (x y : A) (env : Nat → A) (n : Nat) :
    push y (push x env) (n + 2) = env n := rfl

abbrev Relation (X Y : Type) := X → Y → Prop
def subrelation {X Y : Type} (r s : Relation X Y) := ∀ x y, r x y → s x y
def may {X Y : Type} (r : Relation X Y) (p : Region Y) : Region X :=
  fun x => ∃ y, r x y ∧ p y
def all {X Y : Type} (r : Relation X Y) (p : Region Y) : Region X :=
  fun x => ∀ y, r x y → p y
def must {X Y : Type} (r : Relation X Y) (p : Region Y) : Region X :=
  fun x => (∃ y, r x y) ∧ all r p x

theorem may_covariant {X Y : Type} (r s : Relation X Y) (a b : Region Y)
    (relations : subrelation r s) (predicates : included a b) : included (may r a) (may s b) := by
  rintro x ⟨y, edge, present⟩
  exact ⟨y, relations x y edge, predicates y present⟩

theorem all_contravariant_relation_covariant_predicate {X Y : Type}
    (r s : Relation X Y) (a b : Region Y)
    (relations : subrelation r s) (predicates : included a b) : included (all s a) (all r b) :=
  fun x h y edge => predicates y (h y (relations x y edge))

theorem must_covariant_fixed_relation {X Y : Type} (r : Relation X Y) (a b : Region Y)
    (predicates : included a b) : included (must r a) (must r b) :=
  fun _ h => ⟨h.1, fun y edge => predicates y (h.2 y edge)⟩

/-- Growing a relation can enable a dead end, or add a bad successor. Neither
    direction is sound in general; the query checker must not call Must positive
    just because ordinary May and composition are positive. -/
theorem must_not_monotone_relation :
    ¬ included (must (fun (_ : Unit) y => y = false) (fun y => y = false))
        (must (fun (_ : Unit) (_ : Bool) => True) (fun y => y = false)) := by
  intro h
  have impossible := (h () ⟨⟨false, rfl⟩, fun _ edge => edge⟩).2 true trivial
  cases impossible

theorem must_not_antitone_relation :
    ¬ included (must (fun (_ : Unit) (_ : Unit) => True) (fun _ => True))
      (must (fun (_ : Unit) (_ : Unit) => False) (fun _ => True)) := by
  intro h
  obtain ⟨_, impossible⟩ := (h () ⟨⟨(), trivial⟩, fun _ _ => trivial⟩).1
  exact impossible

/-- Residuation reverses its constrained operand and preserves its bound. -/
def residual {X Y Z : Type} (r : Relation X Y) (bound : Relation X Z) : Relation Y Z :=
  fun y z => ∀ x, r x y → bound x z

theorem residual_variance {X Y Z : Type} (r s : Relation X Y) (a b : Relation X Z)
    (relations : subrelation r s) (bounds : subrelation a b) :
    subrelation (residual s a) (residual r b) :=
  fun y z h x edge => bounds x z (h x (relations x y edge))

end QueryBinders

#print axioms QueryBinders.least_below_prefixed
#print axioms QueryBinders.least_prefixed
#print axioms QueryBinders.least_postfixed
#print axioms QueryBinders.least_fixed
#print axioms QueryBinders.postfixed_below_greatest
#print axioms QueryBinders.greatest_postfixed
#print axioms QueryBinders.greatest_prefixed
#print axioms QueryBinders.greatest_fixed
#print axioms QueryBinders.least_operator_covariant
#print axioms QueryBinders.greatest_operator_covariant
#print axioms QueryBinders.nested_least_monotone
#print axioms QueryBinders.nested_greatest_monotone
#print axioms QueryBinders.nearest_predicate
#print axioms QueryBinders.outer_predicate
#print axioms QueryBinders.older_predicate
#print axioms QueryBinders.may_covariant
#print axioms QueryBinders.all_contravariant_relation_covariant_predicate
#print axioms QueryBinders.must_covariant_fixed_relation
#print axioms QueryBinders.must_not_monotone_relation
#print axioms QueryBinders.must_not_antitone_relation
#print axioms QueryBinders.residual_variance
