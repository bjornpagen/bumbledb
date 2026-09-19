import Std

/-!
Algebraic and order obligations of the native real-root checker. The field laws
are generic; interval laws work over any linear order, not a rational grid.
They justify remainder/GCD zero tests and the isolation/refinement/equality
protocol once root membership and isolating intervals have been established.

This module does NOT prove Sturm's theorem, Cauchy's root bound, the analytic
constant-sign theorem, Euclidean algorithm termination, or correspondence to
Rust's dense coefficients. Native differential tests cover those implementation
obligations against an independent exact SymPy oracle. No real-algebraic solver
correctness claim follows merely from these reference lemmas.
-/
namespace AlgebraicRoots

section Field
variable {K : Type} [Lean.Grind.Field K]

theorem remainder_at_root (a quotient divisor remainder : K)
    (division : a = quotient * divisor + remainder) (root : divisor = 0) :
    a = remainder := by grind

theorem negative_remainder_at_middle_root (a quotient b c : K)
    (division : a = quotient*b-c) (middle : b=0) : a = -c := by grind

theorem common_factor_root (f h g a b : K)
    (factorF : f=a*g) (factorH : h=b*g) (root : g=0) : f=0 ∧ h=0 := by grind

theorem bezout_common_root (f h g u v : K)
    (bezout : g=u*f+v*h) (rootF : f=0) (rootH : h=0) : g=0 := by grind

theorem gcd_zero_iff (f h g a b u v : K)
    (factorF : f=a*g) (factorH : h=b*g) (bezout : g=u*f+v*h) :
    g=0 ↔ f=0 ∧ h=0 := by
  constructor
  · exact common_factor_root f h g a b factorF factorH
  · intro roots; exact bezout_common_root f h g u v bezout roots.1 roots.2

end Field

section Order
variable {K : Type} [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]

def Isolates (root : K → Prop) (lower upper value : K) : Prop :=
  root value ∧ lower < value ∧ value < upper ∧
    ∀ other, root other → lower < other → other < upper → other=value

omit [LE K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K] in
theorem isolating_value_unique (root : K → Prop) (l u a b : K)
    (ha : Isolates root l u a) (hb : Isolates root l u b) : a=b := by
  exact (ha.2.2.2 b hb.1 hb.2.1 hb.2.2.1).symm

theorem endpoints_excluded (root : K → Prop) (l u a : K)
    (h : Isolates root l u a) : a≠l ∧ a≠u := by
  have := h.2.1; have := h.2.2.1; grind

theorem disjoint_intervals_order (rootA rootB : K → Prop) (l r s u a b : K)
    (ha : Isolates rootA l r a) (hb : Isolates rootB s u b) (separated : r≤s) : a<b := by
  have := ha.2.2.1; have := hb.2.1; grind

omit [LE K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K] in
theorem common_root_in_overlap (rootA rootB common : K → Prop) (l r s u a b c : K)
    (ha : Isolates rootA l r a) (hb : Isolates rootB s u b)
    (factor : ∀ x, common x → rootA x ∧ rootB x)
    (hc : common c) (la : l<c) (ua : c<r) (lb : s<c) (ub : c<u) : a=b := by
  have ea := ha.2.2.2 c (factor c hc).1 la ua
  have eb := hb.2.2.2 c (factor c hc).2 lb ub
  exact ea.symm.trans eb

omit [LE K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K] in
theorem interior_hit_is_exact (root : K → Prop) (l u a m : K)
    (ha : Isolates root l u a) (hit : root m) (lower : l<m) (upper : m<u) : a=m := by
  exact (ha.2.2.2 m hit lower upper).symm

theorem refine_left (root : K → Prop) (l u a m : K)
    (ha : Isolates root l u a) (left : a<m) (middle : m<u) : Isolates root l m a := by
  refine ⟨ha.1, ha.2.1, left, ?_⟩
  intro b hb lb bm
  apply ha.2.2.2 b hb lb
  grind

theorem refine_right (root : K → Prop) (l u a m : K)
    (ha : Isolates root l u a) (right : m<a) (middle : l<m) : Isolates root m u a := by
  refine ⟨ha.1, right, ha.2.2.1, ?_⟩
  intro b hb mb bu
  apply ha.2.2.2 b hb _ bu
  grind

theorem no_left_root_selects_right (root : K → Prop) (l u a m : K)
    (ha : Isolates root l u a) (noLeft : ∀ b, root b → l<b → ¬b<m)
    (notMiddle : ¬root m) : m<a := by
  have h := noLeft a ha.1 ha.2.1
  have different : a≠m := by intro equal; exact notMiddle (equal ▸ ha.1)
  grind

end Order

def variation (a b : Ordering) : Nat :=
  if a == .eq || b == .eq || a == b then 0 else 1

/-- At a zero middle remainder, nonzero opposite neighbors contribute exactly
one variation on either side of the middle's sign change. -/
theorem opposite_neighbors (middle : Ordering) :
    (match middle with
      | .eq => variation .lt .gt
      | _ => variation .lt middle + variation middle .gt) = 1 ∧
    (match middle with
      | .eq => variation .gt .lt
      | _ => variation .gt middle + variation middle .lt) = 1 := by
  cases middle <;> decide

theorem same_endpoint_signs_are_insufficient :
    let f := fun x : Rat => 25*x*x-70*x+48
    0<f 1 ∧ 0<f 2 ∧ f (3/2)<0 := by decide +kernel

end AlgebraicRoots

#print axioms AlgebraicRoots.remainder_at_root
#print axioms AlgebraicRoots.negative_remainder_at_middle_root
#print axioms AlgebraicRoots.common_factor_root
#print axioms AlgebraicRoots.bezout_common_root
#print axioms AlgebraicRoots.gcd_zero_iff
#print axioms AlgebraicRoots.isolating_value_unique
#print axioms AlgebraicRoots.endpoints_excluded
#print axioms AlgebraicRoots.disjoint_intervals_order
#print axioms AlgebraicRoots.common_root_in_overlap
#print axioms AlgebraicRoots.interior_hit_is_exact
#print axioms AlgebraicRoots.refine_left
#print axioms AlgebraicRoots.refine_right
#print axioms AlgebraicRoots.no_left_root_selects_right
#print axioms AlgebraicRoots.opposite_neighbors
#print axioms AlgebraicRoots.same_endpoint_signs_are_insufficient
