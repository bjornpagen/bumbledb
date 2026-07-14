import Bumbledb.Values

/-!
# Countermodels — the design scratchpad (PRD 02 onward, grows all campaign)

Anything refused or bounded gets its countermodel here, ported or
new — countermodel-first is a covenant law, and the scratchpad is
part of the spec.

## PRD 02 residents

* `empty_interval_vacuous` — over a RAW bounds pair, because the
  in-tree `Interval` cannot be empty: it carries `h : start < «end»`
  as a field, and that unrepresentability is the POINT. The raw shape
  is what the artifact's `empty_nat_interval_has_no_points` warned
  about: an empty point set satisfies ANY coverage obligation
  vacuously, so an engine that admitted raw bounds would let a fact
  denote nothing and every dependency judgment over it hold for free.
  `crate::Interval::new` returning `Option` (parse, don't validate)
  is the mechanism that keeps this countermodel outside the tree —
  in-tree, `Bumbledb.interval_nonempty` is a theorem.

* The str-order refusal — `StrId` is an opaque intern id with
  decidable equality ONLY. No `LT`/`LE`/`Ord` instance exists for the
  intern domain: an intern id is a per-database allocation accident,
  so an order on ids would order the interning history, not the
  values. This is a deliberate absence (a typing fact), machine-
  checked below with `#check_failure`: the instance searches FAIL,
  and the build breaks if anyone ever adds the instances.
-/

namespace Bumbledb.Countermodels

/-! ## The empty-interval countermodel (raw bounds pair) -/

/-- A RAW bounds pair — the shape the in-tree `Interval` refuses to
be: no `h : start < «end»` field, so `start ≥ «end»` (an empty point
set) is representable. -/
structure RawInterval where
  start : Nat
  «end» : Nat

/-- The same half-open reading as `Interval.points`, over the raw
pair. -/
def RawInterval.points (iv : RawInterval) : Set Nat :=
  fun x => iv.start ≤ x ∧ x < iv.«end»

/-- The reversed raw pair `⟨10, 5⟩` denotes the empty set — the
artifact's `empty_nat_interval_has_no_points` shape, restated against
the in-tree `points` reading. -/
theorem raw_interval_no_points :
    ∀ x : Nat, x ∉ (RawInterval.mk 10 5).points := by
  intro x hx
  obtain ⟨hlo, hhi⟩ := hx
  exact absurd (Nat.lt_of_le_of_lt hlo hhi) (by decide)

/-- **The countermodel.** An empty point set satisfies ANY pointwise
coverage obligation vacuously: were empty intervals representable,
every dependency judgment quantifying over an interval's points would
hold for free on them. Unrepresentable in-tree — `Interval` carries
`h : start < «end»`, which is the point (`Bumbledb.interval_nonempty`
is the in-tree theorem; `crate::Interval::new` is the mechanism). -/
theorem empty_interval_vacuous (P : Nat → Prop) :
    ∀ x ∈ (RawInterval.mk 10 5).points, P x := by
  intro x hx
  exact absurd hx (raw_interval_no_points x)

/-! ## The str-order deliberate absence, machine-checked

`#check_failure` succeeds exactly when elaboration fails: each line
below is a build-breaking guard that no order instance ever appears
on the intern domain (the `#guard_msgs (drop info)` wrapper only
silences the expected failure-to-synthesize report). Equality stays
decidable — that instance resolves. -/

example : DecidableEq StrId := inferInstance

#guard_msgs (drop info) in
#check_failure (inferInstance : LT StrId)

#guard_msgs (drop info) in
#check_failure (inferInstance : LE StrId)

#guard_msgs (drop info) in
#check_failure (inferInstance : Ord StrId)

end Bumbledb.Countermodels
