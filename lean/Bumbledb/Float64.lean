/-!
# Canonical binary64 identity and exact numerical comparison

Payload normalization is integer arithmetic; it never executes a host float.
Finite comparison independently interprets the significand and exponent as an
exact integer multiple of 2^-1074. This is not an opaque byte-order oracle.

This module proves normalization/identity and the bounded physical key's
injectivity. `Float64/Order.lean` proves the physical-key/exact-numerical-order
refinement. The numerical instruction bridge and accumulator/rounding proofs
remain separate obligations; no theorem here claims those bridges complete.
-/

namespace Bumbledb

namespace F64

def sign : Nat := 2 ^ 63
def infinity : Nat := 0x7ff0000000000000
def nan : Nat := 0x7ff8000000000000

/-- One zero and one NaN, with all finite magnitudes and both infinities. -/
def Canonical (bits : Nat) : Prop :=
  bits < 2 ^ 64 ∧ bits ≠ sign ∧
    (bits ≤ infinity ∨ (sign < bits ∧ bits ≤ sign + infinity) ∨ bits = nan)

instance (bits : Nat) : Decidable (Canonical bits) := by
  unfold Canonical
  infer_instance

/-- Host normalization, including the fixed-width raw input boundary. -/
def normalizeBits (raw : Nat) : Nat :=
  let bits := raw % 2 ^ 64
  if bits = sign then 0
  else if (infinity < bits ∧ bits < sign) ∨ sign + infinity < bits then nan
  else bits

theorem canonical_normalize (raw : Nat) : Canonical (normalizeBits raw) := by
  have hbound : raw % 2 ^ 64 < 2 ^ 64 := Nat.mod_lt _ (by decide)
  unfold normalizeBits Canonical sign infinity nan at *
  dsimp only
  split <;> (try split) <;> omega

theorem normalize_canonical (bits : Nat) (h : Canonical bits) :
    normalizeBits bits = bits := by
  unfold normalizeBits
  rw [Nat.mod_eq_of_lt h.1]
  unfold Canonical sign infinity nan at *
  dsimp only
  split <;> (try split) <;> omega

theorem normalize_idempotent (raw : Nat) :
    normalizeBits (normalizeBits raw) = normalizeBits raw :=
  normalize_canonical _ (canonical_normalize raw)

end F64

/-- A canonical binary64 payload, not the host's nonreflexive Float equality. -/
abbrev F64 := { bits : Nat // F64.Canonical bits }

namespace F64

def ofBits (bits : Nat) : F64 := ⟨normalizeBits bits, canonical_normalize bits⟩

/-- Strict wire parsing refuses other zeros/NaNs and over-width words. -/
def parse (bits : Nat) : Option F64 :=
  if h : Canonical bits then some ⟨bits, h⟩ else none

theorem parse_payload (value : F64) : parse value.val = some value := by
  simp [parse, value.property]

/-- The physical unsigned order key, with subtraction as the complement of
a bounded negative payload. This is separate from the numerical oracle below. -/
def orderKey (value : F64) : Nat :=
  if value.val < sign then value.val + sign else 2 ^ 64 - 1 - value.val

theorem orderKey_injective (a b : F64) : orderKey a = orderKey b ↔ a = b := by
  constructor
  · intro heq
    apply Subtype.ext
    have ha := a.property.1
    have hb := b.property.1
    unfold orderKey sign at heq
    split at heq <;> split at heq <;> omega
  · intro heq
    exact congrArg orderKey heq

/-- Decode a signless finite payload into units of the smallest positive
subnormal. It is defined for all natural words, independently of bounds. -/
def decodeMagnitude (magnitude : Nat) : Nat :=
  let exponent := magnitude / 2 ^ 52
  let fraction := magnitude % 2 ^ 52
  if exponent = 0 then fraction else (2 ^ 52 + fraction) * 2 ^ (exponent - 1)

/-- Exact finite magnitude in units of the smallest positive subnormal. -/
def scaledMagnitude (bits : Nat) : Nat := decodeMagnitude (bits % sign)

/-- Finite denotation; callers classify the two infinities and NaN first. -/
def scaledInteger (value : F64) : Int :=
  let magnitude : Int := scaledMagnitude value.val
  if value.val < sign then magnitude else -magnitude

/-- Exact numerical order with disjoint ranks for -infinity, finite values,
+infinity and NaN. Finite magnitudes are strictly below 2^2098 scaled units;
the physical 64-bit order-key implementation is not used by this oracle. -/
def numericWord (value : F64) : Nat :=
  if value.val = nan then 2 ^ 2099 + 1
  else if value.val = sign + infinity then 0
  else if value.val = infinity then 2 ^ 2099
  else ((2 ^ 2098 : Int) + scaledInteger value).toNat

instance : LT F64 := ⟨fun a b => numericWord a < numericWord b⟩
instance : LE F64 := ⟨fun a b => numericWord a ≤ numericWord b⟩
instance : DecidableLT F64 := fun a b => inferInstanceAs (Decidable (numericWord a < numericWord b))
instance : DecidableLE F64 := fun a b => inferInstanceAs (Decidable (numericWord a ≤ numericWord b))

theorem lt_irrefl (a : F64) : ¬ a < a := Nat.lt_irrefl _
theorem lt_trans (a b c : F64) (hab : a < b) (hbc : b < c) : a < c := Nat.lt_trans hab hbc
theorem le_total (a b : F64) : a ≤ b ∨ b ≤ a := Nat.le_total _ _

#guard normalizeBits 0x8000000000000000 = 0
#guard normalizeBits 0x7ff0000000000001 = nan
#guard normalizeBits 0xfff0000000000001 = nan
#guard (parse 0x8000000000000000).isNone
#guard (parse 0x7ff0000000000001).isNone
#guard (parse 0xfff8000000000000).isNone
#guard (parse (2 ^ 64)).isNone
#guard (parse nan).isSome
#guard ofBits 0xfff0000000000000 < ofBits 0xffefffffffffffff
#guard ofBits 0xffefffffffffffff < ofBits 0x8000000000000001
#guard ofBits 0x8000000000000001 < ofBits 0
#guard ofBits 0 < ofBits 1
#guard ofBits 0x000fffffffffffff < ofBits 0x0010000000000000
#guard ofBits 0x3ff0000000000000 < ofBits 0x3ff0000000000001
#guard ofBits 0x7fefffffffffffff < ofBits infinity
#guard ofBits infinity < ofBits nan
#guard scaledInteger (ofBits 1) = 1
#guard scaledInteger (ofBits 0x8000000000000001) = -1
#guard scaledInteger (ofBits 0x3ff0000000000000) = 2 ^ 1074

end F64
end Bumbledb
