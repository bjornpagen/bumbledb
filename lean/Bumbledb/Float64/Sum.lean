import Bumbledb.Float64.Order

/-!
# Exact float accumulation — the successor sum/mean model (chapter 11)

Exact accumulation followed by ONE rounding: every finite binary64
value is an integer multiple of 2^-1074 (`scaledInteger`), the
numerical total is exactly one case of
`Finite(exact integer) | +Infinity | -Infinity | NaN`, an exact
binding count travels beside it, and `sum`/`mean` round once at the
end, ties to even, then canonicalize.

Proved here, as chapter 11 demands:

* the canonical sum-case **merge table** (`NumTotal.merge`) is
  associative and commutative with the empty identity
  (`Acc.merge_assoc`, `Acc.merge_comm`, `Acc.empty_merge`);
* the merge is **NOT idempotent** (`merge_not_idempotent`): merging
  the same finite partial state twice doubles its contribution and
  count, so exact set deduplication must happen BEFORE accumulation —
  the accumulator carries no binding provenance;
* fold results are **order- and partition-independent**
  (`fold_perm`, `fold_append`);
* the **34-limb bound**: under the u64 count limit every finite total
  sits strictly below 2^2175 in magnitude — inside a signed 2,176-bit
  (34×64-bit) accumulator (`fold_total_bound`,
  `accumulator_within_34_limbs`) — proved from the single-value bound
  (`decodeMagnitude_bound`), not accepted from prose;
* the fixed **special-case table**: any NaN poisons
  (`fold_nan_of_mem`), mixed infinities are NaN
  (`fold_mixed_inf_nan`), a single infinity sign with finite values
  keeps that infinity (`fold_posInf_only`), no path returns from a
  nonfinite state to a finite one (`NumTotal.merge_finite_inv`), and
  all-finite input folds to the exact integer sum
  (`fold_all_finite`);
* **mean's exact-rational denominator behavior**: the mean is the
  exact rational `total / count`, never `rounded_sum / count` — the
  replication law `mean_replicated_equiv` plus the checked
  `{MAX_FINITE, MAX_FINITE}` fixture (`sum_max_max_overflows`): the
  once-rounded sum overflows to infinity while the once-rounded exact
  mean is exactly `MAX_FINITE`.

The final ties-to-even rounding `roundPosRat` is the executable
specification of "round the exact rational once": subnormal-exact
below 2^53 scaled units, a 53-bit quotient with half/tie-to-even
above, overflow to the infinity encoding past the top binade. Its
golden fixtures are kernel-checked (`#guard`) — an independent third
implementation beside the Rust engine and the bench oracle
(`crates/bumbledb-bench/src/verify/f64_oracle.rs`). The claim that
this specification matches IEEE-754 hardware arithmetic on qualified
targets is an EMPIRICAL bridge (gates `F-ARITH`/`F-ENV`/`F-CROSS`),
not a theorem here; the proof bridge ledger names that gap.

## Narrowings recorded (law 5: narrow and record)

* **Counts are `Nat`.** The engine's count is a u64 with a typed
  `CardinalityOverflow` refusal at the ceiling; the bound theorem
  quantifies at `count ≤ 2^64` and leaves the refusal to the engine
  roster.
* **The accumulator state is the pair, not flags.** No redundant
  "NaN plus both infinities plus an obsolete finite sum" state is
  representable: `NumTotal` has exactly four cases and nonfinite
  states retain no finite limbs.
* **Deduplication precedes accumulation.** `fold` consumes a list the
  caller has already deduplicated (the distinct binding set);
  `merge_not_idempotent` is the negative theorem that replay is NOT
  absorbed here.
* **Nearest/ties-even beyond the goldens is bridged, not proved.**
  `roundPosRat` is the executable rounding specification with
  kernel-checked fixtures; a full Lean nearest-ness theorem over all
  inputs is future refinement, and the independent bench rational
  oracle carries that check meanwhile (ledger row `F-ARITH`).
-/

namespace Bumbledb
namespace F64
namespace Agg

set_option exponentiation.threshold 4096

/-! ## The numerical total — exactly one case -/

/-- The numerical sum state: one exact finite integer (in units of
2^-1074), one infinity of each sign, or NaN. -/
inductive NumTotal where
  | finite (n : Int)
  | posInf
  | negInf
  | nan
deriving DecidableEq, Repr

/-- The canonical merge table (chapter 11 §4). Finite parts add
exactly; same-sign infinities keep their sign; mixed infinities and
anything with NaN are NaN. -/
def NumTotal.merge : NumTotal → NumTotal → NumTotal
  | .finite a, .finite b => .finite (a + b)
  | .finite _, .posInf => .posInf
  | .finite _, .negInf => .negInf
  | .finite _, .nan => .nan
  | .posInf, .finite _ => .posInf
  | .posInf, .posInf => .posInf
  | .posInf, .negInf => .nan
  | .posInf, .nan => .nan
  | .negInf, .finite _ => .negInf
  | .negInf, .posInf => .nan
  | .negInf, .negInf => .negInf
  | .negInf, .nan => .nan
  | .nan, _ => .nan

theorem NumTotal.merge_comm (a b : NumTotal) :
    a.merge b = b.merge a := by
  cases a <;> cases b <;> simp [NumTotal.merge, Int.add_comm]

theorem NumTotal.merge_assoc (a b c : NumTotal) :
    (a.merge b).merge c = a.merge (b.merge c) := by
  cases a <;> cases b <;> cases c <;>
    simp [NumTotal.merge, Int.add_assoc]

theorem NumTotal.zero_merge (a : NumTotal) :
    (NumTotal.finite 0).merge a = a := by
  cases a <;> simp [NumTotal.merge]

/-- NaN absorbs on the left. -/
theorem NumTotal.nan_merge (b : NumTotal) :
    NumTotal.nan.merge b = .nan := by
  cases b <;> rfl

/-- NaN absorbs on the right. -/
theorem NumTotal.merge_nan (a : NumTotal) :
    a.merge .nan = .nan := by
  cases a <;> rfl

/-- A finite merge came from two finite parts summing exactly — no
path leads back to `finite` from an infinity or NaN state (nonfinite
states retain no obsolete finite limbs). -/
theorem NumTotal.merge_finite_inv {a b : NumTotal} {n : Int}
    (h : a.merge b = .finite n) :
    ∃ x y, a = .finite x ∧ b = .finite y ∧ n = x + y := by
  cases a with
  | finite x =>
    cases b with
    | finite y =>
      refine ⟨x, y, rfl, rfl, ?_⟩
      have hfin : NumTotal.finite (x + y) = NumTotal.finite n := h
      injection hfin with hxy
      exact hxy.symm
    | posInf => exact absurd h (by simp [NumTotal.merge])
    | negInf => exact absurd h (by simp [NumTotal.merge])
    | nan => exact absurd h (by simp [NumTotal.merge])
  | posInf => cases b <;> exact absurd h (by simp [NumTotal.merge])
  | negInf => cases b <;> exact absurd h (by simp [NumTotal.merge])
  | nan => cases b <;> exact absurd h (by simp [NumTotal.merge])

/-! ## The accumulator — total plus exact count -/

/-- The accumulator: the numerical total and the exact contributing
count. The empty accumulator is the merge identity; a nonempty
accumulator has nonzero count by construction of `fold`
(`fold_count`). -/
structure Acc where
  total : NumTotal
  count : Nat
deriving DecidableEq, Repr

def Acc.empty : Acc := ⟨.finite 0, 0⟩

/-- Merging combines the numerical cases by the table and adds the
counts. It combines DISJOINT partitions of an already-deduplicated
binding set; it is not idempotent (`merge_not_idempotent`). -/
def Acc.merge (x y : Acc) : Acc :=
  ⟨x.total.merge y.total, x.count + y.count⟩

theorem Acc.merge_comm (x y : Acc) : x.merge y = y.merge x := by
  simp [Acc.merge, NumTotal.merge_comm, Nat.add_comm]

theorem Acc.merge_assoc (x y z : Acc) :
    (x.merge y).merge z = x.merge (y.merge z) := by
  simp [Acc.merge, NumTotal.merge_assoc, Nat.add_assoc]

theorem Acc.empty_merge (x : Acc) : Acc.empty.merge x = x := by
  simp [Acc.merge, Acc.empty, NumTotal.zero_merge]

theorem Acc.merge_empty (x : Acc) : x.merge Acc.empty = x := by
  rw [Acc.merge_comm]
  exact Acc.empty_merge x

/-- **The merge is not idempotent** — the negative theorem chapter 11
demands: merging the same finite partial state twice doubles its
contribution and count. Exact set deduplication must therefore happen
BEFORE accumulation; the accumulator alone cannot detect replay. -/
theorem merge_not_idempotent : ∃ a : Acc, a.merge a ≠ a :=
  ⟨⟨.finite 1, 1⟩, by decide⟩

/-! ## One value's contribution -/

/-- One canonical F64 value's numerical contribution. -/
def contrib (v : F64) : NumTotal :=
  if v.val = nan then .nan
  else if v.val = infinity then .posInf
  else if v.val = sign + infinity then .negInf
  else .finite (scaledInteger v)

def contribAcc (v : F64) : Acc := ⟨contrib v, 1⟩

/-- A finite contribution is exactly the value's scaled integer. -/
theorem contrib_finite_eq {v : F64} {k : Int}
    (h : contrib v = .finite k) : k = scaledInteger v := by
  unfold contrib at h
  by_cases hnan : v.val = nan
  · rw [if_pos hnan] at h
    exact nomatch h
  rw [if_neg hnan] at h
  by_cases hpi : v.val = infinity
  · rw [if_pos hpi] at h
    exact nomatch h
  rw [if_neg hpi] at h
  by_cases hni : v.val = sign + infinity
  · rw [if_pos hni] at h
    exact nomatch h
  rw [if_neg hni] at h
  injection h with h
  exact h.symm

/-- A finite contribution's magnitude sits strictly below 2^2098
scaled units — the single-value half of the 34-limb argument. -/
theorem contrib_finite_bound {v : F64} {n : Int}
    (h : contrib v = .finite n) : n.natAbs < 2 ^ 2098 := by
  have hn := contrib_finite_eq h
  unfold contrib at h
  by_cases hnan : v.val = nan
  · rw [if_pos hnan] at h
    exact nomatch h
  rw [if_neg hnan] at h
  by_cases hpi : v.val = infinity
  · rw [if_pos hpi] at h
    exact nomatch h
  rw [if_neg hpi] at h
  by_cases hni : v.val = sign + infinity
  · rw [if_pos hni] at h
    exact nomatch h
  have hprop := v.property
  have hmod : v.val % sign < infinity := by
    unfold Canonical sign infinity nan at hprop hnan hpi hni ⊢
    omega
  have hbound := decodeMagnitude_bound hmod
  rw [hn]
  simp only [scaledInteger, scaledMagnitude]
  split
  · simpa using hbound
  · simpa using hbound

/-! ## The fold and its algebra -/

/-- Accumulate a deduplicated binding list. The caller supplies the
distinct binding set; order is irrelevant (`fold_perm`). -/
def fold : List F64 → Acc
  | [] => Acc.empty
  | v :: vs => (contribAcc v).merge (fold vs)

@[simp] theorem fold_nil : fold [] = Acc.empty := rfl

@[simp] theorem fold_cons (v : F64) (vs : List F64) :
    fold (v :: vs) = (contribAcc v).merge (fold vs) := rfl

/-- The count is exactly the contributing cardinality. -/
theorem fold_count (vs : List F64) : (fold vs).count = vs.length := by
  induction vs with
  | nil => rfl
  | cons v vs ih =>
    show (contribAcc v).count + (fold vs).count = vs.length + 1
    rw [ih]
    exact Nat.add_comm 1 vs.length

/-- **Order independence**: permuting the binding list leaves the
accumulator unchanged — a different plan, hash iteration order or
spill order cannot change the meaning of `sum`/`mean`. -/
theorem fold_perm {xs ys : List F64} (h : xs.Perm ys) :
    fold xs = fold ys := by
  induction h with
  | nil => rfl
  | cons x _ ih => simp [fold_cons, ih]
  | swap x y l =>
    show (contribAcc y).merge ((contribAcc x).merge (fold l)) =
      (contribAcc x).merge ((contribAcc y).merge (fold l))
    rw [← Acc.merge_assoc, Acc.merge_comm (contribAcc y) (contribAcc x),
      Acc.merge_assoc]
  | trans _ _ ih₁ ih₂ => exact ih₁.trans ih₂

/-- **Partition independence**: folding disjoint partitions and
merging equals folding the whole — the merge-tree law behind
constant-group batches and RAM→scratch spills. -/
theorem fold_append (xs ys : List F64) :
    fold (xs ++ ys) = (fold xs).merge (fold ys) := by
  induction xs with
  | nil => simp [Acc.empty_merge]
  | cons x xs ih => simp [fold_cons, ih, Acc.merge_assoc]

/-! ## The 34-limb bound -/

/-- A finite fold total is bounded by count times the single-value
bound. -/
theorem fold_total_bound (vs : List F64) :
    ∀ {n : Int}, (fold vs).total = .finite n →
      n.natAbs ≤ vs.length * (2 ^ 2098 - 1) := by
  induction vs with
  | nil =>
    intro n h
    have h0 : NumTotal.finite 0 = NumTotal.finite n := h
    injection h0 with h0
    subst h0
    simp
  | cons v vs ih =>
    intro n h
    have hm : (contrib v).merge (fold vs).total = .finite n := h
    obtain ⟨x, y, hx, hy, hxy⟩ := NumTotal.merge_finite_inv hm
    have hxb : x.natAbs < 2 ^ 2098 := contrib_finite_bound hx
    have hyb := ih hy
    have habs : n.natAbs ≤ x.natAbs + y.natAbs := by
      rw [hxy]
      exact Int.natAbs_add_le x y
    calc n.natAbs
        ≤ x.natAbs + y.natAbs := habs
      _ ≤ (2 ^ 2098 - 1) + vs.length * (2 ^ 2098 - 1) :=
          Nat.add_le_add (Nat.le_pred_of_lt hxb) hyb
      _ = vs.length * (2 ^ 2098 - 1) + (2 ^ 2098 - 1) :=
          Nat.add_comm _ _
      _ = (vs.length + 1) * (2 ^ 2098 - 1) := (Nat.succ_mul _ _).symm
      _ = (v :: vs).length * (2 ^ 2098 - 1) := rfl

/-- **The 34-limb sufficiency theorem** (chapter 11 §4): for up to
2^64 contributing bindings, every finite exact total sits strictly
below 2^2175 in magnitude — inside a signed 2,176-bit (34×64-bit)
accumulator, with the count carried beside it. The limb count is
proved, not accepted from prose. -/
theorem accumulator_within_34_limbs (vs : List F64)
    (hlen : vs.length ≤ 2 ^ 64) {n : Int}
    (h : (fold vs).total = .finite n) : n.natAbs < 2 ^ 2175 := by
  have hb := fold_total_bound vs h
  have hmul : vs.length * (2 ^ 2098 - 1) ≤ 2 ^ 64 * (2 ^ 2098 - 1) :=
    Nat.mul_le_mul hlen (Nat.le_refl _)
  have hlit : (2 : Nat) ^ 64 * (2 ^ 2098 - 1) < 2 ^ 2175 := by decide
  exact Nat.lt_of_le_of_lt (Nat.le_trans hb hmul) hlit

/-! ## The fixed special-case table -/

/-- Any NaN contribution poisons the whole fold. -/
theorem fold_nan_of_mem {v : F64} {vs : List F64} (hv : v ∈ vs)
    (h : contrib v = .nan) : (fold vs).total = .nan := by
  induction vs with
  | nil => exact nomatch hv
  | cons w ws ih =>
    show ((contribAcc w).merge (fold ws)).total = .nan
    rcases List.mem_cons.mp hv with rfl | hmem
    · simp [Acc.merge, contribAcc, h, NumTotal.nan_merge]
    · simp [Acc.merge, contribAcc, ih hmem, NumTotal.merge_nan]

/-- A positive-infinity member forces the total into
`{+Infinity, NaN}`. -/
theorem fold_posInf_of_mem {v : F64} {vs : List F64} (hv : v ∈ vs)
    (h : contrib v = .posInf) :
    (fold vs).total = .posInf ∨ (fold vs).total = .nan := by
  induction vs with
  | nil => exact nomatch hv
  | cons w ws ih =>
    show ((contribAcc w).merge (fold ws)).total = .posInf ∨
      ((contribAcc w).merge (fold ws)).total = .nan
    rcases List.mem_cons.mp hv with rfl | hmem
    · simp only [Acc.merge, contribAcc, h]
      cases (fold ws).total <;> simp [NumTotal.merge]
    · rcases ih hmem with ht | ht <;>
        (simp only [Acc.merge, contribAcc, ht] <;>
          cases contrib w <;> simp [NumTotal.merge])

/-- A negative-infinity member forces the total into
`{-Infinity, NaN}`. -/
theorem fold_negInf_of_mem {v : F64} {vs : List F64} (hv : v ∈ vs)
    (h : contrib v = .negInf) :
    (fold vs).total = .negInf ∨ (fold vs).total = .nan := by
  induction vs with
  | nil => exact nomatch hv
  | cons w ws ih =>
    show ((contribAcc w).merge (fold ws)).total = .negInf ∨
      ((contribAcc w).merge (fold ws)).total = .nan
    rcases List.mem_cons.mp hv with rfl | hmem
    · simp only [Acc.merge, contribAcc, h]
      cases (fold ws).total <;> simp [NumTotal.merge]
    · rcases ih hmem with ht | ht <;>
        (simp only [Acc.merge, contribAcc, ht] <;>
          cases contrib w <;> simp [NumTotal.merge])

/-- **Both infinity signs present is NaN** — the fixed table row. -/
theorem fold_mixed_inf_nan {v w : F64} {vs : List F64}
    (hv : v ∈ vs) (hw : w ∈ vs) (h1 : contrib v = .posInf)
    (h2 : contrib w = .negInf) : (fold vs).total = .nan := by
  rcases fold_posInf_of_mem hv h1 with h | h
  · rcases fold_negInf_of_mem hw h2 with h' | h'
    · rw [h] at h'
      exact nomatch h'
    · exact h'
  · exact h

/-- Without negative-infinity or NaN contributions the total is
positive infinity or finite. -/
theorem fold_no_neg_no_nan {vs : List F64}
    (hall : ∀ v, v ∈ vs →
      contrib v = .posInf ∨ ∃ k, contrib v = .finite k) :
    (fold vs).total = .posInf ∨
      ∃ n, (fold vs).total = .finite n := by
  induction vs with
  | nil => exact Or.inr ⟨0, rfl⟩
  | cons w ws ih =>
    have hws := ih fun v hv => hall v (List.mem_cons_of_mem w hv)
    have hw := hall w List.mem_cons_self
    show ((contribAcc w).merge (fold ws)).total = .posInf ∨
      ∃ n, ((contribAcc w).merge (fold ws)).total = .finite n
    rcases hw with hw | ⟨k, hw⟩ <;>
      rcases hws with ht | ⟨n, ht⟩ <;>
        simp [Acc.merge, contribAcc, hw, ht, NumTotal.merge]

/-- **Only positive infinity, with any finite values, sums to
positive infinity** — the fixed table row (the negative sign's
version is symmetric through `fold_negInf_of_mem`). -/
theorem fold_posInf_only {vs : List F64}
    (hmem : ∃ v, v ∈ vs ∧ contrib v = .posInf)
    (hall : ∀ v, v ∈ vs →
      contrib v = .posInf ∨ ∃ k, contrib v = .finite k) :
    (fold vs).total = .posInf := by
  obtain ⟨v, hv, hpos⟩ := hmem
  rcases fold_posInf_of_mem hv hpos with h | h
  · exact h
  · rcases fold_no_neg_no_nan hall with h' | ⟨n, h'⟩ <;>
      (rw [h] at h'; exact nomatch h')

/-- The exact scaled sum of a finite list. -/
def exactScaledSum : List F64 → Int
  | [] => 0
  | v :: vs => scaledInteger v + exactScaledSum vs

/-- **All-finite input folds to the exact integer sum** — the value
half of the fixed table's finite row; `sum` rounds this once. -/
theorem fold_all_finite {vs : List F64}
    (h : ∀ v, v ∈ vs → ∃ k, contrib v = .finite k) :
    (fold vs).total = .finite (exactScaledSum vs) := by
  induction vs with
  | nil => rfl
  | cons w ws ih =>
    obtain ⟨k, hk⟩ := h w List.mem_cons_self
    have hkw := contrib_finite_eq hk
    have hws := ih fun v hv => h v (List.mem_cons_of_mem w hv)
    show ((contribAcc w).merge (fold ws)).total =
      .finite (scaledInteger w + exactScaledSum ws)
    simp [Acc.merge, contribAcc, hk, hws, NumTotal.merge, ← hkw]

/-! ## Final ties-to-even rounding — the executable specification -/

/-- The binade shift: halvings until the quotient sits below 2^53.
Structural fuel keeps this kernel-reducible for `#guard`/`decide`;
2,200 halvings cover every magnitude the bounded accumulator can
produce (`accumulator_within_34_limbs`: below 2^2175). -/
def shiftFor (q : Nat) : Nat :=
  go q 2200
where
  go : Nat → Nat → Nat
    | _, 0 => 0
    | q, fuel + 1 => if q < 2 ^ 53 then 0 else go (q / 2) fuel + 1

/-- Encode a rounded 53-bit quotient `q ∈ [2^52, 2^53)` at step
exponent `s` into canonical nonnegative payload bits; the top binade
overflows to the infinity encoding. -/
def encodeMag (q s : Nat) : Nat :=
  if s + 1 > 2046 then infinity else (s + 1) * 2 ^ 52 + (q - 2 ^ 52)

/-- Round the nonnegative exact rational `m / d` (in 2^-1074 scaled
units, `d ≥ 1`) to the nearest representable binary64 magnitude,
ties to even — ONE rounding of the exact value. Below 2^53 scaled
units every integer magnitude is representable (the subnormal and
first-normal grid, where payload bits equal the magnitude), so the
quotient rounds to the nearest integer; above, the 53-bit quotient
rounds at its binade's step with an exact remainder tie-to-even; past
the top binade the result is the infinity encoding. `d = 1` is the
exact-sum case; `d = count` is the mean — the denominator enters the
ONE rounding exactly, so a finite mean cannot overflow because an
intermediate rounded sum would. -/
def roundPosRat (m d : Nat) : Nat :=
  if d = 0 then nan
  else
    let q0 := m / d
    if q0 < 2 ^ 53 then
      let r := m % d
      let q := if 2 * r > d ∨ (2 * r = d ∧ q0 % 2 = 1) then q0 + 1
        else q0
      if q < 2 ^ 53 then q else encodeMag (2 ^ 52) 1
    else
      let s := shiftFor q0
      let D := d * 2 ^ s
      let t := m / D
      let rem := m % D
      let t' := if 2 * rem > D ∨ (2 * rem = D ∧ t % 2 = 1) then t + 1
        else t
      if t' = 2 ^ 53 then encodeMag (2 ^ 52) (s + 1) else encodeMag t' s

/-- Signed rounding to canonical payload bits: a zero magnitude
collapses to canonical +0 whatever the sign; a negative overflow
lands on the negative-infinity encoding. Call sites pass the result
through `ofBits`, so every emitted value is canonical by
construction. -/
def roundRatBits (n : Int) (d : Nat) : Nat :=
  let mag := roundPosRat n.natAbs d
  if 0 ≤ n ∨ mag = 0 then mag else sign + mag

/-- `sum`: the exact accumulator's total, rounded once and
canonicalized. Empty input is the caller's business — no binding
means no group and no answer row (`Query.empty_global_no_answer`);
this function is the per-group finalizer. -/
def sumF64 (vs : List F64) : F64 :=
  match (fold vs).total with
  | .finite n => ofBits (roundRatBits n 1)
  | .posInf => ofBits infinity
  | .negInf => ofBits (sign + infinity)
  | .nan => ofBits nan

/-- `mean`: the exact rational `total / count`, rounded ONCE — never
`rounded_sum / count`. `none` on empty input: no binding, no group,
no fabricated zero/NaN row. -/
def meanF64 (vs : List F64) : Option F64 :=
  match (fold vs).count with
  | 0 => none
  | _ + 1 =>
    some (match (fold vs).total with
      | .finite n => ofBits (roundRatBits n (fold vs).count)
      | .posInf => ofBits infinity
      | .negInf => ofBits (sign + infinity)
      | .nan => ofBits nan)

/-- The no-binding-no-answer rule at this level. -/
theorem mean_empty_none : meanF64 [] = none := rfl

/-! ## Mean's exact-rational denominator behavior -/

/-- An exact mean as a numerator/denominator pair in scaled units —
the rational the single final rounding consumes. -/
structure ExactMean where
  num : Int
  count : Nat

/-- Rational equality of exact means: cross multiplication. -/
def ExactMean.equiv (x y : ExactMean) : Prop :=
  x.num * (y.count : Int) = y.num * (x.count : Int)

/-- **The replication law**: the exact mean of `k` copies of one
value IS that value — `(k·n)/k = n/1` exactly. With `n` the
`MAX_FINITE` scaled integer and `k = 2` this is the
`{MAX_FINITE, MAX_FINITE}` case: the exact mean is `MAX_FINITE`
even though the once-rounded SUM overflows to infinity
(`sum_max_max_overflows`) — which is why mean must divide the exact
rational, never the rounded sum. -/
theorem mean_replicated_equiv (n : Int) (k : Nat) :
    ExactMean.equiv ⟨(k : Int) * n, k⟩ ⟨n, 1⟩ := by
  show ((k : Int) * n) * ((1 : Nat) : Int) = n * ((k : Nat) : Int)
  rw [Int.natCast_one, Int.mul_one, Int.mul_comm]

/-! ## Kernel-checked golden fixtures (chapter 11 §7)

These are Lean-kernel computations over big integers, not host
floating point: an independent third implementation beside the Rust
engine and the bench oracle. -/

set_option maxRecDepth 8192

/-- The MAX_FINITE magnitude in 2^-1074 scaled units. -/
def maxFiniteScaled : Nat := (2 ^ 53 - 1) * 2 ^ 2045

-- 1.0 rounds from its exact scaled integer to the 1.0 payload.
#guard roundRatBits (2 ^ 1074) 1 = 0x3ff0000000000000
-- {1e16, 1, -1e16}: the exact accumulator cancels to exactly 1.0
-- before rounding — the fixture that distinguishes exact accumulation
-- from repeated native addition (which loses the 1).
#guard (NumTotal.finite (10 ^ 16 * 2 ^ 1074)).merge
    ((NumTotal.finite (2 ^ 1074)).merge
      (.finite (-(10 ^ 16) * 2 ^ 1074))) = .finite (2 ^ 1074)
-- {MAX_FINITE, MAX_FINITE}: the sum overflows to +infinity …
#guard roundRatBits (2 * (maxFiniteScaled : Int)) 1 = 0x7ff0000000000000
-- … while the exact mean is exactly MAX_FINITE.
#guard roundRatBits (2 * (maxFiniteScaled : Int)) 2 = 0x7fefffffffffffff
-- {MIN_SUBNORMAL, MIN_SUBNORMAL}: exact sum 2 units, exact mean 1.
#guard roundRatBits 2 1 = 0x0000000000000002
#guard roundRatBits 2 2 = 0x0000000000000001
-- Exact zero total is canonical +0.
#guard roundRatBits 0 1 = 0
-- Ties to even, both directions, at the first rounding binade.
#guard roundRatBits (2 ^ 53 + 1) 1 = roundRatBits (2 ^ 53) 1
#guard roundRatBits (2 ^ 53 + 3) 1 = roundRatBits (2 ^ 53 + 4) 1
-- Exact mean denominators: 1/3 scaled unit rounds down, 2/3 up.
#guard roundRatBits 1 3 = 0
#guard roundRatBits 2 3 = 0x0000000000000001
-- Negative finite totals land on the sign-carrying payload.
#guard roundRatBits (-(2 ^ 1074)) 1 = 0xbff0000000000000

set_option maxRecDepth 65536 in
/-- The named `{MAX,MAX}` divergence, checked by the kernel: the
once-rounded sum is the infinity payload while the once-rounded exact
mean is the MAX_FINITE payload. Mean is not `rounded_sum / count`. -/
theorem sum_max_max_overflows :
    roundRatBits (2 * (maxFiniteScaled : Int)) 1 = infinity ∧
      roundRatBits (2 * (maxFiniteScaled : Int)) 2 =
        0x7fefffffffffffff :=
  ⟨by decide, by decide⟩

/-! ## Min/max select by the total database order -/

/-- Minimum by the canonical total order (`numericWord`): NaN sorts
last, so `min {1, NaN} = 1` — no host `fmin` NaN elision. -/
def minByOrder (vs : List F64) : Option F64 :=
  vs.foldr
    (fun v acc =>
      match acc with
      | none => some v
      | some w => some (if numericWord v ≤ numericWord w then v else w))
    none

/-- Maximum by the canonical total order: `max {1, NaN} = NaN`. -/
def maxByOrder (vs : List F64) : Option F64 :=
  vs.foldr
    (fun v acc =>
      match acc with
      | none => some v
      | some w => some (if numericWord w ≤ numericWord v then v else w))
    none

#guard minByOrder [ofBits 0x3ff0000000000000, ofBits nan] =
  some (ofBits 0x3ff0000000000000)
#guard maxByOrder [ofBits 0x3ff0000000000000, ofBits nan] =
  some (ofBits nan)

end Agg
end F64
end Bumbledb
