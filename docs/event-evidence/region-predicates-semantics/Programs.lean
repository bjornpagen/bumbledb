import Std

/-!
The finite variance checker and typed Event-program reference. Boolean
constructors enumerate possible before/after truth assignments, exactly as the
native checker does. Fixed map/modal instructions instantiate order-preserving
transforms; the theorem requires that contract explicitly. The native instruction
set admits no arbitrary callback as such a transform.

This reference unfolds an acyclic program as a typed expression. Rust's checked
indices, pruning, ownership, packed truth functions and iterative evaluation
must refine it; this is not an extraction or a proof of those implementation parts.
-/
namespace Programs

structure Variance where
  up : Bool
  down : Bool

def independent : Variance := ⟨false, false⟩
def increasing : Variance := ⟨true, false⟩
def flipped (v : Variance) : Variance := ⟨v.down, v.up⟩
def permits (v : Variance) (a b : Bool) : Bool :=
  if a == b then true else if b then v.up else v.down

def binary (u v : Variance) (op : Bool → Bool → Bool) : Variance where
  up := decide (∃ a b c d : Bool, permits u a b = true ∧ permits v c d = true ∧
    op a c = false ∧ op b d = true)
  down := decide (∃ a b c d : Bool, permits u a b = true ∧ permits v c d = true ∧
    op a c = true ∧ op b d = false)

def conditional (c h l : Bool) := if c then h else l
def ternary (u v z : Variance) : Variance where
  up := decide (∃ a b c d e f : Bool, permits u a b = true ∧ permits v c d = true ∧
    permits z e f = true ∧ conditional a c e = false ∧ conditional b d f = true)
  down := decide (∃ a b c d e f : Bool, permits u a b = true ∧ permits v c d = true ∧
    permits z e f = true ∧ conditional a c e = true ∧ conditional b d f = false)

theorem permits_iff_bounds (v : Variance) (a b : Bool) :
    permits v a b = true ↔
      (v.up = false → b = true → a = true) ∧
      (v.down = false → a = true → b = true) := by
  rcases v with ⟨up, down⟩
  cases up <;> cases down <;> cases a <;> cases b <;> decide

theorem complement_sound (v : Variance) (a b : Bool) (h : permits v a b = true) :
    permits (flipped v) (!a) (!b) = true := by
  rcases v with ⟨up, down⟩
  cases up <;> cases down <;> cases a <;> cases b <;> simp_all [permits, flipped]

theorem binary_sound (u v : Variance) (op : Bool → Bool → Bool) (a b c d : Bool)
    (left : permits u a b = true) (right : permits v c d = true) :
    permits (binary u v op) (op a c) (op b d) = true := by
  cases old : op a c <;> cases new : op b d
  · rfl
  · change (binary u v op).up = true
    exact decide_eq_true ⟨a, b, c, d, left, right, old, new⟩
  · change (binary u v op).down = true
    exact decide_eq_true ⟨a, b, c, d, left, right, old, new⟩
  · rfl

theorem ternary_sound (u v z : Variance) (a b c d e f : Bool)
    (condition : permits u a b = true) (high : permits v c d = true)
    (low : permits z e f = true) :
    permits (ternary u v z) (conditional a c e) (conditional b d f) = true := by
  cases old : conditional a c e <;> cases new : conditional b d f
  · rfl
  · change (ternary u v z).up = true
    exact decide_eq_true ⟨a, b, c, d, e, f, condition, high, low, old, new⟩
  · change (ternary u v z).down = true
    exact decide_eq_true ⟨a, b, c, d, e, f, condition, high, low, old, new⟩
  · rfl

abbrev Region (W : Type) := W → Bool
def included {W : Type} (a b : Region W) := ∀ w, a w = true → b w = true
def Monotone {X Y : Type} (f : Region X → Region Y) :=
  ∀ a b, included a b → included (f a) (f b)

theorem pullback_is_monotone {X Y : Type} (f : X → Y) :
    Monotone (fun (a : Region Y) x => a (f x)) := by
  intro a b inc x
  exact inc (f x)

/-- Image, possibility, May and Post instantiate an existential readout. -/
theorem existential_readout_is_monotone {X Y : Type} (R : Y → X → Prop)
    (readout : Region X → Region Y)
    (exact : ∀ a y, readout a y = true ↔ ∃ x, R y x ∧ a x = true) : Monotone readout := by
  intro a b inc y present
  obtain ⟨x, related, ha⟩ := (exact a y).mp present
  exact (exact b y).mpr ⟨x, related, inc x ha⟩

/-- Universal image, information certainty and All instantiate this contract. -/
theorem universal_readout_is_monotone {X Y : Type} (R : Y → X → Prop)
    (readout : Region X → Region Y)
    (exact : ∀ a y, readout a y = true ↔ ∀ x, R y x → a x = true) : Monotone readout := by
  intro a b inc y present
  exact (exact b y).mpr (fun x related => inc x ((exact a y).mp present x related))

/-- Must and nonvacuous universal image use a fixed enabledness mask. -/
theorem enabled_readout_is_monotone {X Y : Type} (f : Region X → Region Y)
    (mono : Monotone f) (enabled : Region Y) :
    Monotone (fun a y => enabled y && f a y) := by
  intro a b inc y present
  have parts := Bool.and_eq_true_iff.mp present
  exact Bool.and_eq_true_iff.mpr ⟨parts.1, mono a b inc y parts.2⟩

theorem monotone_transports_variance {X Y : Type} (f : Region X → Region Y)
    (mono : Monotone f) (v : Variance) (a b : Region X)
    (changes : ∀ x, permits v (a x) (b x) = true) (y : Y) :
    permits v (f a y) (f b y) = true := by
  apply (permits_iff_bounds _ _ _).mpr
  constructor
  · intro noUp hb
    apply mono b a (fun x hx => ((permits_iff_bounds v (a x) (b x)).mp (changes x)).1 noUp hx) y hb
  · intro noDown ha
    apply mono a b (fun x hx => ((permits_iff_bounds v (a x) (b x)).mp (changes x)).2 noDown hx) y ha

inductive Expr {C : Type} (W : C → Type) (input : C) : C → Type where
  | variable : Expr W input input
  | constant {o : C} (value : Region (W o)) : Expr W input o
  | neg {o : C} (value : Expr W input o) : Expr W input o
  | binary {o : C} (op : Bool → Bool → Bool) (left right : Expr W input o) : Expr W input o
  | ite {o : C} (condition high low : Expr W input o) : Expr W input o
  | transform {a b : C} (f : Region (W a) → Region (W b)) (mono : Monotone f)
      (value : Expr W input a) : Expr W input b

def variance {C : Type} {W : C → Type} {i o : C} : Expr W i o → Variance
  | .variable => increasing
  | .constant _ => independent
  | .neg v => flipped (variance v)
  | .binary op a b => binary (variance a) (variance b) op
  | .ite c h l => ternary (variance c) (variance h) (variance l)
  | .transform _ _ v => variance v

def evaluate {C : Type} {W : C → Type} {i o : C} : Expr W i o → Region (W i) → Region (W o)
  | .variable, input => input
  | .constant value, _ => value
  | .neg value, input => fun w => !(evaluate value input w)
  | .binary op a b, input => fun w => op (evaluate a input w) (evaluate b input w)
  | .ite c h l, input => fun w => conditional (evaluate c input w) (evaluate h input w) (evaluate l input w)
  | .transform f _ v, input => f (evaluate v input)

theorem program_variance_sound {C : Type} {W : C → Type} {i o : C}
    (expr : Expr W i o) (a b : Region (W i)) (inc : included a b) :
    ∀ w, permits (variance expr) (evaluate expr a w) (evaluate expr b w) = true := by
  induction expr with
  | «variable» =>
    intro w
    apply (permits_iff_bounds _ _ _).mpr
    exact ⟨fun impossible => Bool.noConfusion impossible, fun _ => inc w⟩
  | constant value => intro w; simp [evaluate, permits]
  | neg value ih => exact fun w => complement_sound _ _ _ (ih w)
  | binary op left right ihl ihr => exact fun w => binary_sound _ _ op _ _ _ _ (ihl w) (ihr w)
  | ite c h l ihc ihh ihl => exact fun w => ternary_sound _ _ _ _ _ _ _ _ _ (ihc w) (ihh w) (ihl w)
  | transform f mono value ih => exact monotone_transports_variance f mono _ _ _ ih

theorem sealed_program_is_monotone {C : Type} {W : C → Type} {i o : C}
    (expr : Expr W i o) (positive : (variance expr).down = false) : Monotone (evaluate expr) := by
  intro a b inc w present
  exact ((permits_iff_bounds _ _ _).mp (program_variance_sound expr a b inc w)).2 positive present

theorem negation_is_refused :
    (variance (Expr.neg (Expr.variable : Expr (fun _ : Unit => Unit) () ()))).down = true := by rfl

theorem double_negation_is_admitted :
    (variance (Expr.neg (Expr.neg (Expr.variable : Expr (fun _ : Unit => Unit) () ())))).down = false := by rfl

#print axioms Programs.permits_iff_bounds
#print axioms Programs.complement_sound
#print axioms Programs.binary_sound
#print axioms Programs.ternary_sound
#print axioms Programs.pullback_is_monotone
#print axioms Programs.existential_readout_is_monotone
#print axioms Programs.universal_readout_is_monotone
#print axioms Programs.enabled_readout_is_monotone
#print axioms Programs.monotone_transports_variance
#print axioms Programs.program_variance_sound
#print axioms Programs.sealed_program_is_monotone
#print axioms Programs.negation_is_refused
#print axioms Programs.double_negation_is_admitted

end Programs
