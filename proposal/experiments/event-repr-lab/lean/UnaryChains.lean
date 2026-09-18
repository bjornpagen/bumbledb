import Std

namespace UnaryChains

inductive Letter where
  | guard | guardNot | reverseGuard | saturate | xor

def act (l : Letter) (x g : Bool) : Bool :=
  match l with
  | .guard => x && g
  | .guardNot => x && !g
  | .reverseGuard => !x && g
  | .saturate => x || g
  | .xor => x ^^ g

def cofactors (l : Letter) (g : Bool) : Bool × Bool :=
  match l with
  | .guard => (false,g)
  | .guardNot => (false,!g)
  | .reverseGuard => (g,false)
  | .saturate => (g,true)
  | .xor => (g,!g)

theorem reify_step (l : Letter) (x g : Bool) :
    act l x g = if x then (cofactors l g).2 else (cofactors l g).1 := by
  cases l <;> cases x <;> cases g <;> decide

/-- At a fixed coordinate value every letter is one of four unary maps.
    Composition is a two-bit operation, independent of the Event owner. -/
abbrev Transform := Bool × Bool
def eval (t : Transform) (g : Bool) : Bool := if g then t.2 else t.1
def compose (a b : Transform) : Transform := (eval a b.1,eval a b.2)
def identity : Transform := (false,true)

theorem compose_eval (a b : Transform) (g : Bool) :
    eval (compose a b) g = eval a (eval b g) := by cases g <;> rfl

theorem compose_associative (a b c : Transform) :
    compose (compose a b) c = compose a (compose b c) := by
  exact Prod.ext (compose_eval a b c.1) (compose_eval a b c.2)

theorem identity_left (a : Transform) : compose identity a = a := by
  rcases a with ⟨a,b⟩; cases a <;> cases b <;> rfl

theorem identity_right (a : Transform) : compose a identity = a := by rfl

def compileLetter (l : Letter) (x : Bool) : Transform := (act l x false,act l x true)
def run : List (Letter × Bool) → Bool → Bool
  | [], g => g
  | (l,x)::rest, g => act l x (run rest g)
def compile : List (Letter × Bool) → Transform
  | [] => identity
  | (l,x)::rest => compose (compileLetter l x) (compile rest)

theorem compile_exact (word : List (Letter × Bool)) (g : Bool) :
    eval (compile word) g = run word g := by
  induction word with
  | nil => cases g <;> rfl
  | cons head tail ih =>
    rcases head with ⟨l,x⟩
    simp only [compile, compose_eval, ih, run]
    cases run tail g <;> rfl

theorem run_append (a b : List (Letter × Bool)) (g : Bool) :
    run (a ++ b) g = run a (run b g) := by
  induction a with
  | nil => rfl
  | cons head tail ih => rcases head with ⟨l,x⟩; simp only [List.cons_append,run,ih]

def Bits : Nat → Type
  | 0 => Unit
  | n+1 => Bool × Bits n
def population : (n : Nat) → (Bits n → Bool) → Nat
  | 0, f => if f () then 1 else 0
  | n+1, f => population n (fun w => f (false,w)) + population n (fun w => f (true,w))

theorem population_false (n : Nat) : population n (fun _ => false) = 0 := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [population,ih,Nat.add_zero]

theorem population_true (n : Nat) : population n (fun _ => true) = 2^n := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [population,ih,Nat.pow_succ]; omega

theorem population_complement (n : Nat) (f : Bits n → Bool) :
    population n f + population n (fun w => !(f w)) = 2^n := by
  induction n with
  | zero => cases h : f () <;> simp only [population,h] <;> decide
  | succ n ih =>
    have lo := ih (fun w => f (false,w))
    have hi := ih (fun w => f (true,w))
    simp only [population,Nat.pow_succ]
    omega

def transfer (l : Letter) (size count : Nat) : Nat :=
  match l with
  | .guard | .reverseGuard => count
  | .guardNot => size-count
  | .saturate => size+count
  | .xor => size

/-- Each label's counting action depends only on tail population and cube size,
    because these are ordinary Shannon cofactors. This assumes a fresh uniform
    independent Boolean coordinate, not an arbitrary designated source law. -/
theorem population_step (l : Letter) (n : Nat) (f : Bits n → Bool) :
    population (n+1) (fun w => act l w.1 (f w.2)) =
      transfer l (2^n) (population n f) := by
  have h := population_complement n f
  cases l with
  | guard => simp only [population,act,transfer,Bool.false_and,Bool.true_and,population_false,Nat.zero_add]
  | reverseGuard => simp only [population,act,transfer,Bool.not_false,Bool.not_true,
      Bool.false_and,Bool.true_and,population_false,Nat.add_zero]
  | guardNot =>
    simp only [population,act,transfer,Bool.false_and,Bool.true_and,population_false,Nat.zero_add]
    change population n (fun w => !(f w)) = 2^n - population n f
    omega
  | saturate =>
    simp only [population,act,transfer,Bool.false_or,Bool.true_or,population_true]
    exact Nat.add_comm _ _
  | xor =>
    simpa only [population,act,transfer,Bool.false_xor,Bool.true_xor] using h

end UnaryChains

#print axioms UnaryChains.reify_step
#print axioms UnaryChains.compose_eval
#print axioms UnaryChains.compose_associative
#print axioms UnaryChains.identity_left
#print axioms UnaryChains.identity_right
#print axioms UnaryChains.compile_exact
#print axioms UnaryChains.run_append
#print axioms UnaryChains.population_complement
#print axioms UnaryChains.population_step
