-- narration: the unit of addition
def answer (xs : List Nat) : Nat :=
  xs.foldl (· + ·) 0 /- trailing fold -/

/-
  Public contract sentence that will be tightened.
-/
def name (s : String) : String := s
