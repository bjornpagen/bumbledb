import Std

/-!
Reference contract for numerical replay's explicit value stack. Operations and
source admission are abstract. In particular V may contain a partial value or
a checked error. This proves operand order, stack isolation and source-roster
retention; it does not verify the Rust byte parser, capacities or source codecs.
-/
namespace NumberReplay

inductive Expr (S U B : Type) where
  | leaf (source : S)
  | unary (operation : U) (value : Expr S U B)
  | binary (operation : B) (left right : Expr S U B)

inductive Instruction (S U B : Type) where
  | leaf (source : S)
  | unary (operation : U)
  | binary (operation : B)

def eval {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) : Expr S U B → V
  | .leaf source => leaf source
  | .unary operation value => unary operation (eval leaf unary binary value)
  | .binary operation left right =>
      binary operation (eval leaf unary binary left) (eval leaf unary binary right)

def code {S U B : Type} : Expr S U B → List (Instruction S U B)
  | .leaf source => [.leaf source]
  | .unary operation value => code value ++ [.unary operation]
  | .binary operation left right => code left ++ code right ++ [.binary operation]

def run {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) : List (Instruction S U B) → List V → Option (List V)
  | [], stack => some stack
  | .leaf source :: rest, stack => run leaf unary binary rest (leaf source :: stack)
  | .unary operation :: rest, value :: stack =>
      run leaf unary binary rest (unary operation value :: stack)
  | .binary operation :: rest, right :: left :: stack =>
      run leaf unary binary rest (binary operation left right :: stack)
  | _, _ => none

theorem run_append {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) (a b : List (Instruction S U B)) (stack : List V) :
    run leaf unary binary (a ++ b) stack =
      (run leaf unary binary a stack).bind (run leaf unary binary b) := by
  induction a generalizing stack with
  | nil => rfl
  | cons instruction rest ih =>
    cases instruction with
    | leaf source => simpa [run] using ih (leaf source :: stack)
    | unary operation =>
      cases stack with
      | nil => rfl
      | cons value stack => simpa [run] using ih (unary operation value :: stack)
    | binary operation =>
      cases stack with
      | nil => rfl
      | cons right stack =>
        cases stack with
        | nil => rfl
        | cons left stack => simpa [run] using ih (binary operation left right :: stack)

theorem replay_stack_isolation {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) (expression : Expr S U B) (stack : List V) :
    run leaf unary binary (code expression) stack =
      some (eval leaf unary binary expression :: stack) := by
  induction expression generalizing stack with
  | leaf source => rfl
  | unary operation value ih => simp [code, run_append, ih, run, eval]
  | binary operation left right ihLeft ihRight =>
      simp [code, run_append, ihLeft, ihRight, run, eval]

theorem replay_continuation {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) (expression : Expr S U B)
    (rest : List (Instruction S U B)) (stack : List V) :
    run leaf unary binary (code expression ++ rest) stack =
      run leaf unary binary rest (eval leaf unary binary expression :: stack) := by
  simp [run_append, replay_stack_isolation]

theorem replay_has_one_result {S U B V : Type} (leaf : S → V) (unary : U → V → V)
    (binary : B → V → V → V) (expression : Expr S U B) :
    run leaf unary binary (code expression) [] = some [eval leaf unary binary expression] :=
  replay_stack_isolation leaf unary binary expression []

def origins {S U B : Type} : Expr S U B → List S
  | .leaf source => [source]
  | .unary _ value => origins value
  | .binary _ left right => origins left ++ origins right

def instructionOrigins {S U B : Type} : Instruction S U B → List S
  | .leaf source => [source]
  | _ => []

theorem replay_retains_ordered_origins {S U B : Type} (expression : Expr S U B) :
    (code expression).flatMap instructionOrigins = origins expression := by
  induction expression with
  | leaf source => rfl
  | unary operation value ih => simp [code, instructionOrigins, origins, ih]
  | binary operation left right ihLeft ihRight =>
      simp [code, instructionOrigins, origins, ihLeft, ihRight]

theorem malformed_binary_is_not_undefined {S U B V : Type} (leaf : S → V)
    (unary : U → V → V) (binary : B → V → V → V) (operation : B) (value : V) :
    run leaf unary binary [.binary operation] [value] = none := rfl

theorem subtraction_order :
    run (id : Int → Int) (fun _ x => x) (fun _ a b => a - b)
      (code (.binary () (.leaf 7) (.leaf 2) : Expr Int Unit Unit)) [] = some [5] := rfl

theorem equal_values_do_not_identify_origins :
    eval (fun _ : Bool => (1 : Nat)) (fun _ x => x) (fun _ a b => a + b)
      (.leaf true : Expr Bool Unit Unit) =
    eval (fun _ : Bool => (1 : Nat)) (fun _ x => x) (fun _ a b => a + b)
      (.leaf false : Expr Bool Unit Unit) ∧
    origins (.leaf true : Expr Bool Unit Unit) ≠ origins (.leaf false : Expr Bool Unit Unit) := by
  simp [eval, origins]

#print axioms NumberReplay.run_append
#print axioms NumberReplay.replay_stack_isolation
#print axioms NumberReplay.replay_continuation
#print axioms NumberReplay.replay_has_one_result
#print axioms NumberReplay.replay_retains_ordered_origins
#print axioms NumberReplay.malformed_binary_is_not_undefined
#print axioms NumberReplay.subtraction_order
#print axioms NumberReplay.equal_values_do_not_identify_origins
end NumberReplay
