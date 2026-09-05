import Bumbledb.Float64

/-!
# Canonical binary64 physical/numerical order refinement

The numerical denotation decodes the exponent and significand to exact
integer units. The physical key is the independent sign/complement mapping.
-/

namespace Bumbledb.F64

-- Binary64's exact denominator needs powers up to 2^2099; these remain
-- kernel-checked integer computations, not a native numerical oracle.
set_option exponentiation.threshold 4096

/-- The exponent/significand interpretation strictly increases with every
signless payload, including the subnormal-to-normal boundary. -/
theorem decodeMagnitude_strictMono {a b : Nat} (hab : a < b) :
    decodeMagnitude a < decodeMagnitude b := by
  have hfa := Nat.mod_lt a (show 0 < 2 ^ 52 by decide)
  have hfb := Nat.mod_lt b (show 0 < 2 ^ 52 by decide)
  have hea := Nat.div_add_mod a (2 ^ 52)
  have heb := Nat.div_add_mod b (2 ^ 52)
  have he : a / 2 ^ 52 ≤ b / 2 ^ 52 := Nat.div_le_div_right (Nat.le_of_lt hab)
  unfold decodeMagnitude
  dsimp only
  by_cases hea0 : a / 2 ^ 52 = 0
  · rw [if_pos hea0]
    by_cases heb0 : b / 2 ^ 52 = 0
    · rw [if_pos heb0]
      omega
    · rw [if_neg heb0]
      have hp : 1 ≤ 2 ^ (b / 2 ^ 52 - 1) := Nat.one_le_pow _ _ (by decide)
      have hm := Nat.mul_le_mul_left (2 ^ 52 + b % 2 ^ 52) hp
      simp only [Nat.mul_one] at hm
      omega
  · rw [if_neg hea0]
    have heb0 : b / 2 ^ 52 ≠ 0 := by omega
    rw [if_neg heb0]
    by_cases heq : a / 2 ^ 52 = b / 2 ^ 52
    · rw [heq]
      apply Nat.mul_lt_mul_of_pos_right (by omega)
      exact Nat.pow_pos (by decide)
    · have haexp : a / 2 ^ 52 = (a / 2 ^ 52 - 1) + 1 := by omega
      have hstep : 2 ^ (a / 2 ^ 52) = 2 ^ (a / 2 ^ 52 - 1) * 2 := by
        calc
          _ = 2 ^ ((a / 2 ^ 52 - 1) + 1) := congrArg (2 ^ ·) haexp
          _ = _ := Nat.pow_succ _ _
      have hpow : 2 ^ (a / 2 ^ 52) ≤ 2 ^ (b / 2 ^ 52 - 1) :=
        Nat.pow_le_pow_right (by decide) (by omega)
      calc
        (2 ^ 52 + a % 2 ^ 52) * 2 ^ (a / 2 ^ 52 - 1)
          < (2 ^ 52 * 2) * 2 ^ (a / 2 ^ 52 - 1) :=
            Nat.mul_lt_mul_of_pos_right (by omega) (Nat.pow_pos (by decide))
        _ = 2 ^ 52 * 2 ^ (a / 2 ^ 52) := by
          rw [hstep]
          omega
        _ ≤ 2 ^ 52 * 2 ^ (b / 2 ^ 52 - 1) := Nat.mul_le_mul_left _ hpow
        _ ≤ (2 ^ 52 + b % 2 ^ 52) * 2 ^ (b / 2 ^ 52 - 1) :=
          Nat.mul_le_mul_right _ (by omega)

set_option exponentiation.threshold 4096 in
set_option maxRecDepth 4096 in
/-- The infinity encoding is exactly the excluded finite-magnitude bound
when interpreted by the exponent/significand equation. -/
theorem decodeMagnitude_infinity : decodeMagnitude infinity = 2 ^ 2098 := by
  change 2 ^ 52 * 2 ^ 2046 = 2 ^ 2098
  exact (Nat.pow_add 2 52 2046).symm

/-- Every finite payload contributes fewer than 2^2098 smallest-subnormal
units. This is the single-value bound used by the exact accumulator. -/
theorem decodeMagnitude_bound {m : Nat} (hm : m < infinity) :
    decodeMagnitude m < 2 ^ 2098 := by
  rw [← decodeMagnitude_infinity]
  exact decodeMagnitude_strictMono hm

theorem decodeMagnitude_pos {m : Nat} (hm : 0 < m) : 0 < decodeMagnitude m :=
  decodeMagnitude_strictMono hm

private theorem numericWord_positive (v : Bumbledb.F64) (hv : v.val < infinity) :
    numericWord v = 2 ^ 2098 + decodeMagnitude v.val := by
  have hsign : v.val < sign := by unfold sign infinity at *; omega
  have hn : v.val ≠ nan := by unfold nan infinity at *; omega
  have hni : v.val ≠ sign + infinity := by unfold sign infinity at *; omega
  unfold numericWord
  rw [if_neg hn, if_neg hni, if_neg (Nat.ne_of_lt hv)]
  unfold scaledInteger scaledMagnitude
  rw [if_pos hsign, Nat.mod_eq_of_lt hsign]
  omega

private theorem numericWord_negative (v : Bumbledb.F64)
    (hv : sign < v.val ∧ v.val < sign + infinity) :
    numericWord v = 2 ^ 2098 - decodeMagnitude (v.val - sign) := by
  have hmag : v.val % sign = v.val - sign := by
    unfold sign infinity at *
    omega
  have hbound : decodeMagnitude (v.val - sign) < 2 ^ 2098 :=
    decodeMagnitude_bound (by unfold sign infinity at *; omega)
  have hn : v.val ≠ nan := by unfold sign nan at *; omega
  have hni : v.val ≠ sign + infinity := Nat.ne_of_lt hv.2
  have hpi : v.val ≠ infinity := by unfold sign infinity at *; omega
  have hsign : ¬v.val < sign := Nat.not_lt_of_ge (Nat.le_of_lt hv.1)
  unfold numericWord
  rw [if_neg hn, if_neg hni, if_neg hpi]
  unfold scaledInteger scaledMagnitude
  rw [if_neg hsign, hmag]
  omega

private theorem decodeMagnitude_le_bound {m : Nat} (hm : m ≤ infinity) :
    decodeMagnitude m ≤ 2 ^ 2098 := by
  rcases Nat.lt_or_eq_of_le hm with hm | rfl
  · exact Nat.le_of_lt (decodeMagnitude_bound hm)
  · exact Nat.le_of_eq decodeMagnitude_infinity

private theorem numericWord_nonnegative (v : Bumbledb.F64) (hv : v.val ≤ infinity) :
    numericWord v = 2 ^ 2098 + decodeMagnitude v.val := by
  rcases Nat.lt_or_eq_of_le hv with hv | hv
  · exact numericWord_positive v hv
  · have hn : v.val ≠ nan := by unfold infinity nan at *; omega
    have hni : v.val ≠ sign + infinity := by unfold sign infinity at *; omega
    unfold numericWord
    rw [if_neg hn, if_neg hni, if_pos hv, hv, decodeMagnitude_infinity]

private theorem numericWord_negative_inclusive (v : Bumbledb.F64)
    (hv : sign < v.val ∧ v.val ≤ sign + infinity) :
    numericWord v = 2 ^ 2098 - decodeMagnitude (v.val - sign) := by
  rcases Nat.lt_or_eq_of_le hv.2 with hi | hi
  · exact numericWord_negative v ⟨hv.1, hi⟩
  · have hn : v.val ≠ nan := by unfold sign nan at *; omega
    unfold numericWord
    rw [if_neg hn, if_pos hi, hi, Nat.add_sub_cancel_left, decodeMagnitude_infinity]

private theorem negative_class (v : Bumbledb.F64) (hv : ¬v.val < sign) :
    sign < v.val ∧ v.val ≤ sign + infinity := by
  have hc := v.property
  unfold Canonical sign infinity nan at *
  omega

private theorem positive_class (v : Bumbledb.F64) (hv : v.val < sign) :
    v.val ≤ infinity ∨ v.val = nan := by
  have hc := v.property
  unfold Canonical sign infinity nan at *
  omega

private theorem numericWord_negative_lt (v : Bumbledb.F64) (hv : ¬v.val < sign) :
    numericWord v < 2 ^ 2098 := by
  have hc := negative_class v hv
  rw [numericWord_negative_inclusive v hc]
  have hm := decodeMagnitude_pos (m := v.val - sign) (by omega)
  omega

private theorem numericWord_positive_ge (v : Bumbledb.F64) (hv : v.val < sign) :
    2 ^ 2098 ≤ numericWord v := by
  rcases positive_class v hv with hc | hc
  · rw [numericWord_nonnegative v hc]
    omega
  · unfold numericWord
    rw [if_pos hc]
    omega

private theorem numericWord_positive_mono (a b : Bumbledb.F64)
    (ha : a.val < sign) (hb : b.val < sign) (hab : a.val < b.val) :
    numericWord a < numericWord b := by
  rcases positive_class a ha with hca | hca
  · rcases positive_class b hb with hcb | hcb
    · rw [numericWord_nonnegative a hca, numericWord_nonnegative b hcb]
      exact Nat.add_lt_add_left (decodeMagnitude_strictMono hab) _
    · rw [numericWord_nonnegative a hca]
      have hbound := decodeMagnitude_le_bound hca
      unfold numericWord
      rw [if_pos hcb]
      omega
  · have hcb := positive_class b hb
    unfold infinity nan at *
    omega

/-- The physical sign/complement key strictly preserves the independent
exact numerical order, over every canonical payload (not only samples). -/
theorem orderKey_lt_numericWord (a b : Bumbledb.F64)
    (hab : orderKey a < orderKey b) : numericWord a < numericWord b := by
  have habound := a.property.1
  have hbbound := b.property.1
  unfold orderKey at hab
  by_cases ha : a.val < sign <;> by_cases hb : b.val < sign
  · rw [if_pos ha, if_pos hb] at hab
    exact numericWord_positive_mono a b ha hb (by omega)
  · rw [if_pos ha, if_neg hb] at hab
    unfold sign at *
    omega
  · exact Nat.lt_of_lt_of_le (numericWord_negative_lt a ha) (numericWord_positive_ge b hb)
  · rw [if_neg ha, if_neg hb] at hab
    have hca := negative_class a ha
    have hcb := negative_class b hb
    rw [numericWord_negative_inclusive a hca, numericWord_negative_inclusive b hcb]
    have hraw : b.val - sign < a.val - sign := by omega
    have hm := decodeMagnitude_strictMono hraw
    have hmabound := decodeMagnitude_le_bound (m := a.val - sign) (by omega)
    omega

/-- The numerical oracle and physical index order agree in both directions. -/
theorem orderKey_lt_iff (a b : Bumbledb.F64) : orderKey a < orderKey b ↔ a < b := by
  constructor
  · exact orderKey_lt_numericWord a b
  · intro hab
    rcases Nat.lt_trichotomy (orderKey a) (orderKey b) with hk | hk | hk
    · exact hk
    · have heq := (orderKey_injective a b).mp hk
      subst b
      exact False.elim (lt_irrefl a hab)
    · exact False.elim (Nat.lt_asymm hab (orderKey_lt_numericWord b a hk))

theorem orderKey_le_iff (a b : Bumbledb.F64) : orderKey a ≤ orderKey b ↔ a ≤ b := by
  have hlt := orderKey_lt_iff b a
  change orderKey a ≤ orderKey b ↔ numericWord a ≤ numericWord b
  change orderKey b < orderKey a ↔ numericWord b < numericWord a at hlt
  omega

theorem le_antisymm (a b : Bumbledb.F64) (hab : a ≤ b) (hba : b ≤ a) : a = b := by
  apply (orderKey_injective a b).mp
  exact Nat.le_antisymm ((orderKey_le_iff a b).mpr hab) ((orderKey_le_iff b a).mpr hba)

theorem numericWord_injective (a b : Bumbledb.F64) : numericWord a = numericWord b ↔ a = b := by
  constructor
  · intro h
    apply le_antisymm a b <;> change numericWord _ ≤ numericWord _ <;> omega
  · exact congrArg numericWord

end Bumbledb.F64
