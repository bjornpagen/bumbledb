def answer (xs : List Nat) : Nat :=
  xs.foldl (· + ·) 0

/-- Semantics the signature cannot carry. -/
def name (s : String) : String := s
