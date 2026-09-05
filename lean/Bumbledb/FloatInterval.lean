import Bumbledb.Float64.Sum
import Bumbledb.Query.Aggregates

/-!
# `Interval<F64>` — dense numeric ranges over canonical endpoints

The successor float interval (chapters 10 §2 and 11 §5): two canonical
binary64 endpoints on a **dense numeric line**, half-open, nonempty,
with `-Infinity` legal only as a missing lower bound and `+Infinity`
only as a missing upper bound, NaN refused, signed zero normalized
before validation, and `start < end` in the exact numeric endpoint
order. Points are rationals; finite endpoints embed as their exact
scaled integers (units of 2^-1074 — a uniform order-preserving rescale
of the numeric line; lengths are handled separately and exactly).

This dense denotation is the one that avoids the hidden contradiction
chapter 11 names: if points meant only representable finite floats,
`start < end` would accept the empty set `[-Infinity, -MAX_FINITE)`.
Here that interval is **provably nonempty** (`negInfRay_witness`), a
real gap exists between distinct adjacent representable endpoints
(`adjacent_endpoint_gap`, `gap_uncovered`), and half-open adjacency
still coalesces exactly (`join_points`). Do not mix this model with a
discrete representable-point model in membership, coverage or test
oracles.

Structure invariants are discharged by parsing, never validating:
`FInterval` carries non-NaN endpoints and the strict numeric order;
infinity placement is then a THEOREM, not a side condition
(`start_not_posInf`, `stop_not_negInf`). Equal endpoints — including
`[-0, +0)`, whose endpoints normalize to one canonical zero — are
unrepresentable (`zero_zero_refuses`).

The ordered execution words: the physical order key agrees with the
dense denotation on every finite probe
(`containsF64_iff_orderKey`), and a NONFINITE probe is false by
definition while the raw key comparison would accept `-Infinity` on a
left ray — the engine's finite-probe guard is load-bearing
(`neg_inf_probe_needs_guard`). The canonical endpoint order also
instantiates the shared `LinearElem` toolkit, so the existing proved
sweep/pack/coverage algebra (`Exec/Sweep.lean`) applies to float
endpoints verbatim — no second temporal engine.

Bounded length is the exact endpoint difference rounded ONCE
(`FInterval.measure`, reusing `Agg.roundRatBits`); a nonfinite bound
is `unbounded` (`UnboundedMeasure`) and a finite difference whose
single rounding overflows is `overflow` (`MeasureOverflow`) — two
distinct outcomes, per chapter 11. There is **no `FixedInterval<F64>`
model**: no width/compression constructor exists here to prove things
about, deliberately.

## Narrowings recorded (law 5: narrow and record)

* **Points are ℚ, not ℝ.** Exact rational order suffices for every
  supported proof; no real-number enumeration is modeled or needed.
* **The scale is uniform.** Embedding finite endpoints at their scaled
  integers (not value/2^1074) is an order isomorphism of the line;
  membership, adjacency, coalescing and gap theorems are
  scale-invariant. Lengths are computed in exact scaled integers and
  rounded once — never through the embedded points.
* **Rounding correctness beyond kernel goldens is bridged.** The
  measure's single rounding reuses `Agg.roundRatBits`; its IEEE
  correspondence is the `F-ARITH`/`F-INTERVAL` empirical gate, named
  in the proof bridge ledger.
-/

namespace Bumbledb
open F64

/-! ## Finite-value classification -/

/-- A strictly finite canonical value: not NaN, not either
infinity. -/
def F64.IsFinite (v : F64) : Prop :=
  v.val ≠ nan ∧ v.val ≠ infinity ∧ v.val ≠ sign + infinity

instance (v : F64) : Decidable v.IsFinite := by
  unfold F64.IsFinite
  infer_instance

/-- A strictly finite value's contribution/scaled-integer reading. -/
theorem F64.contrib_of_isFinite {v : F64} (h : v.IsFinite) :
    Agg.contrib v = .finite (scaledInteger v) := by
  unfold Agg.contrib
  rw [if_neg h.1, if_neg h.2.1, if_neg h.2.2]

/-- A strictly finite value's magnitude sits below 2^2098 scaled
units. -/
theorem F64.scaled_abs_lt {v : F64} (h : v.IsFinite) :
    (scaledInteger v).natAbs < 2 ^ 2098 :=
  Agg.contrib_finite_bound (F64.contrib_of_isFinite h)

set_option exponentiation.threshold 4096

/-- The numeric word of a strictly finite value, unfolded. -/
theorem numericWord_of_isFinite {v : F64} (h : v.IsFinite) :
    numericWord v = ((2 ^ 2098 : Int) + scaledInteger v).toNat := by
  unfold numericWord
  rw [if_neg h.1, if_neg h.2.2, if_neg h.2.1]

/-- The numeric word of the negative-infinity payload. -/
theorem numericWord_negInf {v : F64} (hn : v.val ≠ nan)
    (h : v.val = sign + infinity) : numericWord v = 0 := by
  unfold numericWord
  rw [if_neg hn, if_pos h]

/-- The numeric word of the positive-infinity payload. -/
theorem numericWord_posInf {v : F64} (h : v.val = infinity) :
    numericWord v = 2 ^ 2099 := by
  have hn : v.val ≠ nan := by
    rw [h]
    unfold infinity nan
    omega
  have hni : v.val ≠ sign + infinity := by
    rw [h]
    unfold sign infinity
    omega
  unfold numericWord
  rw [if_neg hn, if_neg hni, if_pos h]

/-- Every non-NaN word sits at or below the positive-infinity rank. -/
theorem numericWord_le_of_not_nan {v : F64} (hn : v.val ≠ nan) :
    numericWord v ≤ 2 ^ 2099 := by
  by_cases hni : v.val = sign + infinity
  · rw [numericWord_negInf hn hni]
    exact Nat.zero_le _
  by_cases hpi : v.val = infinity
  · exact Nat.le_of_eq (numericWord_posInf hpi)
  · have hf : v.IsFinite := ⟨hn, hpi, hni⟩
    have hb := F64.scaled_abs_lt hf
    rw [numericWord_of_isFinite hf]
    omega

/-- A strictly finite word sits strictly below the positive-infinity
rank. -/
theorem numericWord_lt_posInf {v : F64} (h : v.IsFinite) :
    numericWord v < 2 ^ 2099 := by
  have hb := F64.scaled_abs_lt h
  rw [numericWord_of_isFinite h]
  omega

/-- On strictly finite values the numeric-word order IS the exact
scaled-integer order. -/
theorem numericWord_finite_lt_iff {u v : F64} (hu : u.IsFinite)
    (hv : v.IsFinite) :
    numericWord u < numericWord v ↔
      scaledInteger u < scaledInteger v := by
  have hub := F64.scaled_abs_lt hu
  have hvb := F64.scaled_abs_lt hv
  rw [numericWord_of_isFinite hu, numericWord_of_isFinite hv]
  omega

/-- On strictly finite values the numeric-word ≤ IS scaled-integer ≤. -/
theorem numericWord_finite_le_iff {u v : F64} (hu : u.IsFinite)
    (hv : v.IsFinite) :
    numericWord u ≤ numericWord v ↔
      scaledInteger u ≤ scaledInteger v := by
  have hub := F64.scaled_abs_lt hu
  have hvb := F64.scaled_abs_lt hv
  rw [numericWord_of_isFinite hu, numericWord_of_isFinite hv]
  omega

/-! ## The checked interval -/

/-- A checked float interval: canonical endpoints (the `F64` quotient
already collapsed signed zero and every NaN payload), NaN refused at
both bounds, and strict exact numeric endpoint order. Constructors
parse; no unchecked public constructor exists to model. -/
structure FInterval where
  /-- The inclusive lower endpoint payload. -/
  start : F64
  /-- The exclusive upper endpoint payload. -/
  stop : F64
  /-- NaN is forbidden at the lower bound. -/
  start_not_nan : start.val ≠ nan
  /-- NaN is forbidden at the upper bound. -/
  stop_not_nan : stop.val ≠ nan
  /-- Strict numeric endpoint order — the nonemptiness witness. -/
  lt : numericWord start < numericWord stop

/-- `+Infinity` cannot be a lower bound: nothing non-NaN sits above
it, so `start < stop` already refuses the placement — a theorem, not
a side condition. -/
theorem FInterval.start_not_posInf (i : FInterval) :
    i.start.val ≠ infinity := by
  intro h
  have hw := numericWord_posInf h
  have hle := numericWord_le_of_not_nan i.stop_not_nan
  have := i.lt
  omega

/-- `-Infinity` cannot be an upper bound: nothing sits below it. -/
theorem FInterval.stop_not_negInf (i : FInterval) :
    i.stop.val ≠ sign + infinity := by
  intro h
  have hw := numericWord_negInf i.stop_not_nan h
  have := i.lt
  omega

/-- `[-0, +0)` refuses: both endpoints normalize to one canonical
zero, so the strict order is unsatisfiable. -/
theorem zero_zero_refuses :
    ¬ numericWord (ofBits 0x8000000000000000) <
      numericWord (ofBits 0) := by decide

/-! ## The dense denotation -/

/-- The dense-line point set: rationals bounded below by a finite
start (inclusive) and above by a finite stop (exclusive); an infinity
payload bounds nothing. `[-Infinity, +Infinity)` is the whole line. -/
def FInterval.points (i : FInterval) : Set Rat := fun q =>
  (i.start.val = sign + infinity ∨
    ((scaledInteger i.start : Rat) ≤ q)) ∧
  (i.stop.val = infinity ∨ q < (scaledInteger i.stop : Rat))

/-! Rational order helpers (core `Rat` supplies the pieces). -/

private theorem rat_lt_of_lt_of_le {a b c : Rat} (h1 : a < b)
    (h2 : b ≤ c) : a < c := by
  obtain ⟨hab, hnba⟩ := Rat.lt_iff_le_and_not_ge.mp h1
  exact Rat.lt_iff_le_and_not_ge.mpr
    ⟨Rat.le_trans hab h2, fun hca => hnba (Rat.le_trans h2 hca)⟩

private theorem rat_lt_of_le_of_lt {a b c : Rat} (h1 : a ≤ b)
    (h2 : b < c) : a < c := by
  obtain ⟨hbc, hncb⟩ := Rat.lt_iff_le_and_not_ge.mp h2
  exact Rat.lt_iff_le_and_not_ge.mpr
    ⟨Rat.le_trans h1 hbc, fun hca => hncb (Rat.le_trans hca h1)⟩

private theorem rat_lt_irrefl (a : Rat) : ¬ a < a :=
  Rat.not_lt.mpr (Rat.le_refl)

/-- Every checked interval denotes a NONEMPTY dense point set — the
constructor invariant, covering finite intervals, rays and the whole
line. -/
theorem FInterval.nonempty (i : FInterval) : ∃ q, q ∈ i.points := by
  by_cases hs : i.start.val = sign + infinity
  · by_cases he : i.stop.val = infinity
    · exact ⟨0, Or.inl hs, Or.inl he⟩
    · refine ⟨(scaledInteger i.stop : Rat) - 1, Or.inl hs, Or.inr ?_⟩
      refine Rat.sub_lt_iff.mpr ?_
      calc (scaledInteger i.stop : Rat)
          = (scaledInteger i.stop : Rat) + 0 := (Rat.add_zero _).symm
        _ < (scaledInteger i.stop : Rat) + 1 :=
            Rat.add_lt_add_left.mpr (by decide)
  · have hsf : i.start.IsFinite :=
      ⟨i.start_not_nan, i.start_not_posInf, hs⟩
    refine ⟨(scaledInteger i.start : Rat),
      Or.inr Rat.le_refl, ?_⟩
    by_cases he : i.stop.val = infinity
    · exact Or.inl he
    · have hef : i.stop.IsFinite :=
        ⟨i.stop_not_nan, he, i.stop_not_negInf⟩
      exact Or.inr (Rat.intCast_lt_intCast.mpr
        ((numericWord_finite_lt_iff hsf hef).mp i.lt))

/-- **`[-Infinity, -MAX_FINITE)` is a valid nonempty left ray** — the
distinguishing dense-denotation fixture: no finite representable F64
point lies inside it, yet the dense line does. -/
def negInfRay : FInterval :=
  ⟨ofBits (sign + infinity), ofBits 0xffefffffffffffff,
    by decide, by decide, by decide⟩

set_option maxRecDepth 8192 in
/-- A concrete dense witness strictly below `-MAX_FINITE`. -/
theorem negInfRay_witness :
    ((-(Agg.maxFiniteScaled : Int) - 1 : Int) : Rat) ∈
      negInfRay.points := by
  refine ⟨Or.inl (by decide), Or.inr ?_⟩
  exact Rat.intCast_lt_intCast.mpr (by decide)

/-- `[a, nextUp a)` at finite `a` is a valid positive-width interval —
no successor arithmetic enters the algebra; the strict payload order
is the whole check. -/
def unitUlp : FInterval :=
  ⟨ofBits 0x3ff0000000000000, ofBits 0x3ff0000000000001,
    by decide, by decide, by decide⟩

/-! ## Membership: exact embedding, nonfinite probes false -/

/-- A membership probe: a strictly finite F64 point embeds exactly;
NaN and either infinity are false BY DEFINITION — a nonfinite scalar
probe is a well-defined nonmatch. -/
def FInterval.containsF64 (i : FInterval) (p : F64) : Prop :=
  p.IsFinite ∧ (scaledInteger p : Rat) ∈ i.points

/-- Nonfinite probes are false — definitional, recorded as the named
law. -/
theorem nonfinite_probe_false (i : FInterval) (p : F64)
    (h : ¬ p.IsFinite) : ¬ i.containsF64 p :=
  fun ⟨hf, _⟩ => h hf

/-- **The physical bridge**: on strictly finite probes the dense
denotation IS the physical order-key comparison — the non-NaN
order-key mapping executes membership without an epsilon and without
a discrete-point reinterpretation. -/
theorem containsF64_iff_orderKey (i : FInterval) (p : F64)
    (hp : p.IsFinite) :
    i.containsF64 p ↔
      (orderKey i.start ≤ orderKey p ∧
        orderKey p < orderKey i.stop) := by
  unfold FInterval.containsF64 FInterval.points
  constructor
  · rintro ⟨-, hlow, hup⟩
    constructor
    · by_cases hs : i.start.val = sign + infinity
      · show orderKey i.start ≤ orderKey p
        rw [orderKey_le_iff]
        show numericWord i.start ≤ numericWord p
        rw [numericWord_negInf i.start_not_nan hs]
        exact Nat.zero_le _
      · have hsf : i.start.IsFinite :=
          ⟨i.start_not_nan, i.start_not_posInf, hs⟩
        have hle : (scaledInteger i.start : Rat) ≤
            (scaledInteger p : Rat) := by
          rcases hlow with hs' | hle
          · exact absurd hs' hs
          · exact hle
        exact (orderKey_le_iff _ _).mpr
          ((numericWord_finite_le_iff hsf hp).mpr
            (Rat.intCast_le_intCast.mp hle))
    · by_cases he : i.stop.val = infinity
      · show orderKey p < orderKey i.stop
        rw [orderKey_lt_iff]
        show numericWord p < numericWord i.stop
        rw [numericWord_posInf he]
        exact numericWord_lt_posInf hp
      · have hef : i.stop.IsFinite :=
          ⟨i.stop_not_nan, he, i.stop_not_negInf⟩
        have hlt : (scaledInteger p : Rat) <
            (scaledInteger i.stop : Rat) := by
          rcases hup with he' | hlt
          · exact absurd he' he
          · exact hlt
        exact (orderKey_lt_iff _ _).mpr
          ((numericWord_finite_lt_iff hp hef).mpr
            (Rat.intCast_lt_intCast.mp hlt))
  · rintro ⟨hlow, hup⟩
    refine ⟨hp, ?_, ?_⟩
    · by_cases hs : i.start.val = sign + infinity
      · exact Or.inl hs
      · have hsf : i.start.IsFinite :=
          ⟨i.start_not_nan, i.start_not_posInf, hs⟩
        exact Or.inr (Rat.intCast_le_intCast.mpr
          ((numericWord_finite_le_iff hsf hp).mp
            ((orderKey_le_iff _ _).mp hlow)))
    · by_cases he : i.stop.val = infinity
      · exact Or.inl he
      · have hef : i.stop.IsFinite :=
          ⟨i.stop_not_nan, he, i.stop_not_negInf⟩
        exact Or.inr (Rat.intCast_lt_intCast.mpr
          ((numericWord_finite_lt_iff hp hef).mp
            ((orderKey_lt_iff _ _).mp hup)))

/-- **The finite-probe guard is load-bearing**: the raw order-key
comparison ADMITS a `-Infinity` probe on a left-unbounded interval,
while the dense denotation refuses every nonfinite probe. An engine
that executes membership as bare key comparisons without the
nonfinite guard returns the wrong answer on this fixture. -/
theorem neg_inf_probe_needs_guard :
    (orderKey negInfRay.start ≤ orderKey (ofBits (sign + infinity)) ∧
      orderKey (ofBits (sign + infinity)) < orderKey negInfRay.stop) ∧
    ¬ negInfRay.containsF64 (ofBits (sign + infinity)) := by
  constructor
  · exact ⟨by decide, by decide⟩
  · exact nonfinite_probe_false _ _ (by decide)

/-! ## Adjacency coalesces; representable-neighbor gaps do not -/

/-- Join two intervals sharing the middle endpoint. The shared
endpoint is strictly finite by the placement theorems, so the join is
well-formed. -/
def FInterval.join (i j : FInterval) (h : i.stop = j.start) :
    FInterval :=
  ⟨i.start, j.stop, i.start_not_nan, j.stop_not_nan, by
    have hj := j.lt
    rw [← h] at hj
    exact Nat.lt_trans i.lt hj⟩

/-- **Half-open adjacency coalesces exactly**: `[a,b) ∪ [b,c)` is
`[a,c)` on the dense line — no point is lost and none is invented. -/
theorem FInterval.join_points (i j : FInterval)
    (h : i.stop = j.start) (q : Rat) :
    q ∈ (i.join j h).points ↔ q ∈ i.points ∨ q ∈ j.points := by
  have hbf : i.stop.IsFinite :=
    ⟨i.stop_not_nan, by rw [h]; exact j.start_not_posInf,
      i.stop_not_negInf⟩
  have hnotinf : ¬ i.stop.val = infinity := hbf.2.1
  have hnotneg : ¬ j.start.val = sign + infinity := by
    rw [← h]
    exact hbf.2.2
  constructor
  · rintro ⟨hlow, hup⟩
    by_cases hqb : q < (scaledInteger i.stop : Rat)
    · exact Or.inl ⟨hlow, Or.inr hqb⟩
    · refine Or.inr ⟨Or.inr ?_, hup⟩
      rw [← h]
      exact Rat.not_lt.mp hqb
  · rintro (⟨hlow, hup⟩ | ⟨hlow, hup⟩)
    · refine ⟨hlow, ?_⟩
      rcases hup with he | hqb
      · exact absurd he hnotinf
      · by_cases hje : j.stop.val = infinity
        · exact Or.inl hje
        · have hjf : j.stop.IsFinite :=
            ⟨j.stop_not_nan, hje, j.stop_not_negInf⟩
          have hlt : scaledInteger j.start < scaledInteger j.stop :=
            (numericWord_finite_lt_iff (h ▸ hbf) hjf).mp j.lt
          refine Or.inr (rat_lt_of_lt_of_le hqb ?_)
          rw [h]
          exact Rat.le_of_lt (Rat.intCast_lt_intCast.mpr hlt)
    · refine ⟨?_, hup⟩
      rcases hlow with hs | hqc
      · exact absurd hs hnotneg
      · by_cases his : i.start.val = sign + infinity
        · exact Or.inl his
        · have hisf : i.start.IsFinite :=
            ⟨i.start_not_nan, i.start_not_posInf, his⟩
          have hlt : scaledInteger i.start < scaledInteger i.stop :=
            (numericWord_finite_lt_iff hisf hbf).mp i.lt
          refine Or.inr (Rat.le_trans ?_ hqc)
          refine Rat.le_of_lt (Rat.intCast_lt_intCast.mpr ?_)
          rw [← h]
          exact hlt

/-- **The dense gap between distinct endpoints**: between any two
distinct scaled-integer endpoint values a rational point lies strictly
between — even when no machine float does. -/
theorem adjacent_endpoint_gap {b c : Int} (h : b < c) :
    ∃ q : Rat, (b : Rat) < q ∧ q < (c : Rat) := by
  refine ⟨(b : Rat) + mkRat 1 2, ?_, ?_⟩
  · calc (b : Rat) = (b : Rat) + 0 := (Rat.add_zero _).symm
      _ < (b : Rat) + mkRat 1 2 := Rat.add_lt_add_left.mpr (by decide)
  · have h1 : ((b : Rat) + mkRat 1 2) < (b : Rat) + 1 :=
      Rat.add_lt_add_left.mpr (by decide)
    have h2 : ((b + 1 : Int) : Rat) ≤ (c : Rat) :=
      Rat.intCast_le_intCast.mpr (by omega)
    rw [Rat.intCast_add, Rat.intCast_one] at h2
    exact rat_lt_of_lt_of_le h1 h2

/-- **Never coalesce across a representable-neighbor gap**: when one
interval ends strictly below where the next begins (end `b`, next
start `nextUp(b)` included), a dense point escapes both — the pair
must NOT be coalesced merely because the bounds are adjacent machine
floats. -/
theorem gap_uncovered (i j : FInterval)
    (hif : ¬ i.stop.val = infinity)
    (hjs : ¬ j.start.val = sign + infinity)
    (hgap : scaledInteger i.stop < scaledInteger j.start) :
    ∃ q : Rat, q ∉ i.points ∧ q ∉ j.points := by
  refine ⟨(scaledInteger i.stop : Rat) + mkRat 1 2, ?_, ?_⟩
  · rintro ⟨-, hup⟩
    rcases hup with he | hlt
    · exact hif he
    · have hle : (scaledInteger i.stop : Rat) ≤
          (scaledInteger i.stop : Rat) + mkRat 1 2 := by
        calc (scaledInteger i.stop : Rat)
            = (scaledInteger i.stop : Rat) + 0 := (Rat.add_zero _).symm
          _ ≤ _ := Rat.add_le_add_left.mpr (by decide)
      exact rat_lt_irrefl _ (rat_lt_of_le_of_lt hle hlt)
  · rintro ⟨hlow, -⟩
    rcases hlow with hs | hle
    · exact hjs hs
    · have h1 : ((scaledInteger i.stop : Rat) + mkRat 1 2) <
          (scaledInteger i.stop : Rat) + 1 :=
        Rat.add_lt_add_left.mpr (by decide)
      have h2 : ((scaledInteger i.stop + 1 : Int) : Rat) ≤
          (scaledInteger j.start : Rat) :=
        Rat.intCast_le_intCast.mpr (by omega)
      rw [Rat.intCast_add, Rat.intCast_one] at h2
      exact rat_lt_irrefl _
        (rat_lt_of_le_of_lt hle (rat_lt_of_lt_of_le h1 h2))

/-- The concrete `nextUp` fixture: `1.0`'s payload and its successor
payload are distinct scaled integers, so `adjacent_endpoint_gap`
yields the dense point between them. -/
example : ∃ q : Rat,
    ((scaledInteger (ofBits 0x3ff0000000000000) : Rat) < q ∧
      q < (scaledInteger (ofBits 0x3ff0000000000001) : Rat)) :=
  adjacent_endpoint_gap (by decide)

/-! ## One ordered-endpoint algebra — the shared toolkit instance -/

/-- Canonical F64 endpoint order instantiates the shared linear-order
toolkit, so the PROVED generic sweep/pack/coverage algebra
(`Exec/Sweep.lean`) serves float intervals verbatim — Allen
relations, overlap, coalescing and coverage reuse ordered endpoints,
never a float-specific temporal engine and never raw payload bits fed
to an unsigned kernel. -/
instance : LinearElem F64 where
  lt_irrefl := F64.lt_irrefl
  lt_trans := fun {a b c} h h' => F64.lt_trans a b c h h'
  trichotomy a b := by
    rcases Nat.lt_trichotomy (numericWord a) (numericWord b) with
      h | h | h
    · exact .inl h
    · exact .inr (.inl ((numericWord_injective a b).mp h))
    · exact .inr (.inr h)
  le_iff a b := by
    show numericWord a ≤ numericWord b ↔
      numericWord a < numericWord b ∨ a = b
    rw [← numericWord_injective a b]
    omega

/-! ## Bounded length: one rounding, two distinct failures -/

/-- The measure outcome roster: `unbounded` for a nonfinite bound
(`UnboundedMeasure`), `overflow` for a bounded interval whose
once-rounded length exceeds the finite range (`MeasureOverflow`), or
the canonical rounded length payload. Distinct failures by type. -/
inductive FMeasure where
  | unbounded
  | overflow
  | length (bits : Nat)
deriving DecidableEq, Repr

/-- The exact scaled endpoint difference of a bounded interval. -/
def FInterval.exactLengthScaled (i : FInterval) : Int :=
  scaledInteger i.stop - scaledInteger i.start

/-- A bounded interval's exact length is strictly positive. -/
theorem FInterval.exactLength_pos (i : FInterval)
    (hs : i.start.val ≠ sign + infinity)
    (he : i.stop.val ≠ infinity) : 0 < i.exactLengthScaled := by
  have hsf : i.start.IsFinite :=
    ⟨i.start_not_nan, i.start_not_posInf, hs⟩
  have hef : i.stop.IsFinite :=
    ⟨i.stop_not_nan, he, i.stop_not_negInf⟩
  have := (numericWord_finite_lt_iff hsf hef).mp i.lt
  unfold FInterval.exactLengthScaled
  omega

/-- Bounded length: the exact endpoint difference rounded ONCE, ties
to even (`Agg.roundRatBits`), under the numerical guard on the
engine side. A rounded overflow to the infinity payload is
`overflow`; either nonfinite bound is `unbounded`. Length is a
numerical length of the dense range — never a count of representable
points, and never silently narrowed to an integer type. -/
def FInterval.measure (i : FInterval) : FMeasure :=
  if i.start.val = sign + infinity ∨ i.stop.val = infinity then
    .unbounded
  else
    let bits := Agg.roundRatBits i.exactLengthScaled 1
    if bits = infinity then .overflow else .length bits

set_option maxRecDepth 8192

-- A ray's measure is `unbounded`, never a finite number.
#guard negInfRay.measure = .unbounded

/-- `[-MAX_FINITE, +MAX_FINITE)`: bounded, but its once-rounded F64
length overflows — `overflow`, distinct from `unbounded`. -/
def wholeFiniteSpan : FInterval :=
  ⟨ofBits 0xffefffffffffffff, ofBits 0x7fefffffffffffff,
    by decide, by decide, by decide⟩

#guard wholeFiniteSpan.measure = .overflow

/-- `[1.0, 2.0)` measures exactly `1.0`. -/
def unitSpan : FInterval :=
  ⟨ofBits 0x3ff0000000000000, ofBits 0x4000000000000000,
    by decide, by decide, by decide⟩

#guard unitSpan.measure = .length 0x3ff0000000000000

-- `[1.0, nextUp 1.0)` has a positive exact length of one scaled ulp
-- unit at that binade — a valid interval whose length is NOT zero.
#guard unitUlp.measure = .length 0x3cb0000000000000

end Bumbledb
