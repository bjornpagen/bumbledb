import Std

namespace PrefixRetraction

def Cube : Nat → Type
  | 0 => Unit
  | n + 1 => Bool × Cube n

def feasible : (n : Nat) → (Cube n → Bool) → Bool
  | 0, S => S ()
  | n + 1, S => feasible n (fun x => S (false, x)) || feasible n (fun x => S (true, x))

def choose (n : Nat) (S : Cube (n+1) → Bool) (b : Bool) : Bool :=
  if feasible n (fun x => S (b, x)) then b else !b

def repair : (n : Nat) → (Cube n → Bool) → Cube n → Cube n
  | 0, _, _ => ()
  | n + 1, S, (b, x) =>
      let c := choose n S b
      (c, repair n (fun y => S (c, y)) x)

def samePrefix : (n k : Nat) → Cube n → Cube n → Prop
  | 0, _, _, _ => True
  | _ + 1, 0, _, _ => True
  | n + 1, k + 1, x, y => x.1 = y.1 ∧ samePrefix n k x.2 y.2

def splice : (n k : Nat) → Cube n → Cube n → Cube n
  | 0, _, _, _ => ()
  | _ + 1, 0, _, y => y
  | n + 1, k + 1, x, y => (x.1, splice n k x.2 y.2)

theorem feasible_iff (n : Nat) (S : Cube n → Bool) :
    feasible n S = true ↔ ∃ w, S w = true := by
  induction n with
  | zero =>
    constructor
    · intro h; exact ⟨(), h⟩
    · rintro ⟨⟨⟩, h⟩; exact h
  | succ n ih =>
    simp only [feasible, Bool.or_eq_true, ih]
    constructor
    · rintro (⟨x, h⟩ | ⟨x, h⟩)
      · exact ⟨(false, x), h⟩
      · exact ⟨(true, x), h⟩
    · rintro ⟨⟨b, x⟩, h⟩
      cases b
      · exact Or.inl ⟨x, h⟩
      · exact Or.inr ⟨x, h⟩

theorem chosen_feasible (n : Nat) (S : Cube (n+1) → Bool)
    (h : feasible (n+1) S = true) (b : Bool) :
    feasible n (fun x => S (choose n S b, x)) = true := by
  cases b <;>
    cases h0 : feasible n (fun x => S (false, x)) <;>
    cases h1 : feasible n (fun x => S (true, x)) <;>
    simp_all [feasible, choose]

theorem repair_lands (n : Nat) (S : Cube n → Bool)
    (h : feasible n S = true) (w : Cube n) : S (repair n S w) = true := by
  induction n with
  | zero => exact h
  | succ n ih =>
    exact ih (fun y => S (choose n S w.1, y)) (chosen_feasible n S h w.1) w.2

theorem repair_fixes (n : Nat) (S : Cube n → Bool) (w : Cube n)
    (h : S w = true) : repair n S w = w := by
  induction n with
  | zero => cases w; rfl
  | succ n ih =>
    rcases w with ⟨b, x⟩
    have live : feasible n (fun x => S (b, x)) = true := (feasible_iff n _).mpr ⟨x, h⟩
    simp only [repair, choose, live, ↓reduceIte]
    exact congrArg (fun y => (b, y)) (ih (fun y => S (b, y)) x h)

theorem prefix_sym (n k : Nat) (x y : Cube n) : samePrefix n k x y → samePrefix n k y x := by
  induction n generalizing k with
  | zero => exact id
  | succ n ih =>
    cases k with
    | zero => exact id
    | succ k =>
      intro h
      exact ⟨h.1.symm, ih k x.2 y.2 h.2⟩

theorem prefix_preserved (n k : Nat) (S : Cube n → Bool) (x y : Cube n)
    (h : samePrefix n k x y) : samePrefix n k (repair n S x) (repair n S y) := by
  induction n generalizing k with
  | zero => trivial
  | succ n ih =>
    cases k with
    | zero => trivial
    | succ k =>
      rcases x with ⟨a, x⟩
      rcases y with ⟨b, y⟩
      have same : a = b := h.1
      subst b
      exact ⟨rfl, ih k (fun z => S (choose n S a, z)) x y h.2⟩

theorem splice_prefix (n k : Nat) (x y : Cube n) : samePrefix n k (splice n k x y) x := by
  induction n generalizing k with
  | zero => trivial
  | succ n ih =>
    cases k with
    | zero => trivial
    | succ k => exact ⟨rfl, ih k x.2 y.2⟩

/-- A legal suffix can be supplied behind any raw prefix that decodes to its
    legal prefix. No unique physical witness is required. -/
theorem repair_covers_prefix_fibre (n k : Nat) (S : Cube n → Bool) (x y : Cube n)
    (hy : S y = true) (h : samePrefix n k (repair n S x) y) :
    repair n S (splice n k x y) = y := by
  induction n generalizing k with
  | zero => cases y; rfl
  | succ n ih =>
    cases k with
    | zero => exact repair_fixes (n+1) S y hy
    | succ k =>
      rcases x with ⟨a, x⟩
      rcases y with ⟨b, y⟩
      simp only [repair, samePrefix] at h
      simp only [splice, repair]
      rw [h.1]
      apply congrArg (fun z => (b, z))
      apply ih k (fun z => S (b, z)) x y hy
      simpa only [h.1] using h.2

/-- Direct raw suffix abstraction computes exact supported abstraction at the
    decoded retained prefix. This is the complete-fibre/base-change contract. -/
theorem prefix_exists (n k : Nat) (S : Cube n → Bool)
    (nonempty : feasible n S = true) (A : Cube n → Prop) (x : Cube n) :
    (∃ v, samePrefix n k v x ∧ A (repair n S v)) ↔
      ∃ y, S y = true ∧ samePrefix n k y (repair n S x) ∧ A y := by
  constructor
  · rintro ⟨v, hp, ha⟩
    exact ⟨repair n S v, repair_lands n S nonempty v, prefix_preserved n k S v x hp, ha⟩
  · rintro ⟨y, hy, hp, ha⟩
    refine ⟨splice n k x y, splice_prefix n k x y, ?_⟩
    rw [repair_covers_prefix_fibre n k S x y hy (prefix_sym n k y (repair n S x) hp)]
    exact ha

/-- A supported membership FD from the retained legal prefix is equivalent to
    physical dependence on only that raw prefix after completion. This chooses
    a readout hierarchy, not a least dependency set among all coordinate sets. -/
theorem prefix_dependency_reflected (n k : Nat) (S : Cube n → Bool)
    (nonempty : feasible n S = true) (A : Cube n → Prop) :
    (∀ x y, samePrefix n k x y → (A (repair n S x) ↔ A (repair n S y))) ↔
      (∀ x y, S x = true → S y = true → samePrefix n k x y → (A x ↔ A y)) := by
  constructor
  · intro raw x y hx hy hp
    have h := raw x y hp
    simpa only [repair_fixes n S x hx, repair_fixes n S y hy] using h
  · intro supported x y hp
    exact supported (repair n S x) (repair n S y)
      (repair_lands n S nonempty x) (repair_lands n S nonempty y)
      (prefix_preserved n k S x y hp)

#print axioms feasible_iff
#print axioms repair_lands
#print axioms repair_fixes
#print axioms prefix_preserved
#print axioms repair_covers_prefix_fibre
#print axioms prefix_exists
#print axioms prefix_dependency_reflected

end PrefixRetraction
