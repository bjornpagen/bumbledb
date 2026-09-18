import Std

namespace SharedExit

def select (s lo hi : Bool) : Bool := if s then hi else lo

/-- The two operands must share the selector, not merely some marginal law. -/
theorem apply_shared (op : Bool → Bool → Bool) (s a b c d : Bool) :
    op (select s a b) (select s c d) = select s (op a c) (op b d) := by
  cases s <;> rfl

theorem split_selector (s t lo hi : Bool) :
    select (s || t) lo hi = select s (select t lo hi) hi := by
  cases s <;> cases t <;> rfl

theorem complement (s lo hi : Bool) :
    (!(select s lo hi)) = select s (!lo) (!hi) := by cases s <;> rfl

theorem normalize (s lo hi p : Bool) :
    select s (lo ^^ p) (hi ^^ p) = (select s lo hi ^^ p) := by cases s <;> rfl

def any {α : Type} (xs : List α) (w : α → Bool) : Bool :=
  match xs with
  | [] => false
  | x::rest => w x || any rest w

/-- Sparse selector coordinates need not be adjacent in an ambient order. -/
theorem sparse_split {α : Type} (xs ys : List α) (w : α → Bool) (lo hi : Bool) :
    select (any (xs ++ ys) w) lo hi =
      select (any xs w) (select (any ys w) lo hi) hi := by
  have union : any (xs ++ ys) w = (any xs w || any ys w) := by
    induction xs with
    | nil => rfl
    | cons x rest ih => simp only [List.cons_append,any,ih,Bool.or_assoc]
  rw [union,split_selector]

/-- Eliminating an inhabited independent selector with both values possible.
    The tail world may be empty or infinite. It is shared by each branch. -/
theorem exists_hidden {α : Type} (s : Bool) (lo hi : α → Bool) :
    (∃ h : Bool, ∃ y, select (s || h) (lo y) (hi y) = true) ↔
      (if s then (∃ y, hi y = true) else ((∃ y, lo y = true) ∨ (∃ y, hi y = true))) := by
  cases s <;> simp [select,Bool.exists_bool]

theorem no_hidden_selector {α : Type} (s : Bool) (lo hi : α → Bool) :
    (∃ y, select s (lo y) (hi y) = true) ↔
      (if s then (∃ y, hi y = true) else (∃ y, lo y = true)) := by
  cases s <;> rfl

/-- Equal cofactors remove the coordinate; unequal arbitrary children retain it. -/
theorem essential_selector {α : Type} (lo hi : α → Bool) :
    (∃ y, select false (lo y) (hi y) ≠ select true (lo y) (hi y)) ↔
      (∃ y, lo y ≠ hi y) := by rfl

def Bits : Nat → Type
  | 0 => Unit
  | n+1 => Bool × Bits n
def bitAny : (n : Nat) → Bits n → Bool
  | 0, _ => false
  | n+1, (b,rest) => b || bitAny n rest
def total : (n : Nat) → (Bits n → Nat) → Nat
  | 0, f => f ()
  | n+1, f => total n (fun w => f (false,w)) + total n (fun w => f (true,w))

theorem total_constant (n k : Nat) : total n (fun _ => k) = 2^n*k := by
  induction n with
  | zero => simp [total]
  | succ n ih => simp only [total,ih,Nat.pow_succ,Nat.mul_two,Nat.add_mul]

theorem count_plus_high (n lo hi : Nat) :
    total n (fun w => if bitAny n w then hi else lo) + hi = lo + 2^n*hi := by
  induction n with
  | zero => simp [total,bitAny]
  | succ n ih =>
    simp only [total,bitAny,Bool.false_or,Bool.true_or,ite_true,total_constant,
      Nat.pow_succ,Nat.mul_two,Nat.add_mul]
    omega

/-- Lo/hi are counts aligned to the same remaining raw coordinates. The
    selector is a full Boolean cube, not samples from an arbitrary joint law. -/
theorem count_selector (n lo hi : Nat) :
    total n (fun w => if bitAny n w then hi else lo) = lo + (2^n-1)*hi := by
  have h := count_plus_high n lo hi
  have positive : ∀ k : Nat, 1 ≤ 2^k := by
    intro k; induction k with
    | zero => decide
    | succ k ih => simp only [Nat.pow_succ,Nat.mul_two]; omega
  have pos := positive n
  have mul : (2^n-1)*hi + hi = 2^n*hi := by
    have e := congrArg (fun k => k*hi) (Nat.sub_add_cancel pos)
    simpa only [Nat.add_mul,Nat.one_mul] using e
  omega

/-- Knowing each selector is possible does not make them the same selector. -/
theorem unequal_selectors_obstruction :
    (select false false true && select true false true) ≠
      select false (false && false) (true && true) ∨
    (select false false true || select true false true) ≠
      select false (false || false) (true || true) := by decide

end SharedExit

#print axioms SharedExit.apply_shared
#print axioms SharedExit.split_selector
#print axioms SharedExit.complement
#print axioms SharedExit.normalize
#print axioms SharedExit.sparse_split
#print axioms SharedExit.exists_hidden
#print axioms SharedExit.no_hidden_selector
#print axioms SharedExit.essential_selector
#print axioms SharedExit.count_selector
#print axioms SharedExit.unequal_selectors_obstruction
