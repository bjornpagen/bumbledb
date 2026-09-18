import Std

namespace DirectConditional

def choose (s h l : Bool) : Bool := if s then h else l

inductive Term (V : Type) where
  | literal : Bool → Term V
  | variable : V → Term V
  | conditional : Term V → Term V → Term V → Term V

def eval {V : Type} (t : Term V) (w : V → Bool) : Bool :=
  match t with
  | .literal b => b
  | .variable v => w v
  | .conditional s h l => choose (eval s w) (eval h w) (eval l w)

/-- Inserted coordinate functions are not recursively substituted. -/
def substitute {V W : Type} (d : V → Term W) (t : Term V) : Term W :=
  match t with
  | .literal b => .literal b
  | .variable v => d v
  | .conditional s h l =>
      .conditional (substitute d s) (substitute d h) (substitute d l)

theorem simultaneous_substitution {V W : Type} (d : V → Term W)
    (t : Term V) (w : W → Bool) :
    eval (substitute d t) w = eval t (fun v => eval (d v) w) := by
  induction t with
  | literal b => rfl
  | variable v => rfl
  | conditional s h l ihs ihh ihl => simp only [substitute, eval, ihs, ihh, ihl]

theorem substitution_composes {U V W : Type} (d : U → Term V)
    (e : V → Term W) (t : Term U) :
    substitute e (substitute d t) = substitute (fun v => substitute e (d v)) t := by
  induction t with
  | literal b => rfl
  | variable v => rfl
  | conditional s h l ihs ihh ihl => simp only [substitute, ihs, ihh, ihl]

theorem identity_substitution {V : Type} (t : Term V) :
    substitute Term.variable t = t := by
  induction t with
  | literal b => rfl
  | variable v => rfl
  | conditional s h l ihs ihh ihl => simp only [substitute, ihs, ihh, ihl]

/-- Exactly the selector/output polarity rules used by the ternary memo key. -/
theorem conditional_polarities (s h l : Bool) :
    choose (!s) h l = choose s l h ∧
    choose s (!h) (!l) = !(choose s h l) ∧
    choose s h h = h ∧
    choose s h l = ((s && h) || ((!s) && l)) := by
  cases s <;> cases h <;> cases l <;> decide

/-- A single common Shannon split for all three operands. -/
theorem shannon_conditional (x sl sh hl hh ll lh : Bool) :
    choose (choose x sh sl) (choose x hh hl) (choose x lh ll) =
      choose x (choose sh hh lh) (choose sl hl ll) := by
  cases x <;> rfl

/-- Even a two-coordinate exchange is broken by sequential replacement. -/
theorem sequential_is_not_simultaneous :
    let swap : Bool → Term Bool := fun v => .variable (!v)
    let first : Bool → Term Bool := fun v => if v then .variable true else .variable true
    let second : Bool → Term Bool := fun v => if v then .variable false else .variable false
    let t : Term Bool := .variable false
    eval (substitute swap t) id ≠ eval (substitute second (substitute first t)) id := by
  decide

end DirectConditional

#print axioms DirectConditional.simultaneous_substitution
#print axioms DirectConditional.substitution_composes
#print axioms DirectConditional.identity_substitution
#print axioms DirectConditional.conditional_polarities
#print axioms DirectConditional.shannon_conditional
#print axioms DirectConditional.sequential_is_not_simultaneous
