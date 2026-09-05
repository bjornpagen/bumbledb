//! The independent binary64 oracle (P11, chapter 11 / gates `F-CANON`,
//! `F-ORDER`, `F-ARITH`, `F-AGG`).
//!
//! Everything here is computed from raw `u64` bit patterns with integer and
//! rational arithmetic ONLY: no production canonicalization, comparison,
//! hashing or accumulator helper is consulted, and no host floating-point
//! operation participates in an expected value. Host `f64` appears solely as
//! the SUBJECT of the differential tests (the hardware side `F-ARITH`
//! qualifies), never as the oracle.
//!
//! The model mirrors the kernel-checked Lean specification
//! (`lean/Bumbledb/Float64/Sum.lean`, `lean/Bumbledb/Float64.lean`) as a
//! third implementation: canonical quotient (one zero, one NaN), total order
//! key, exact scaled-integer decomposition (units of 2^-1074), the exact
//! sum/mean accumulator with the canonical merge table, and one final
//! round-to-nearest-ties-to-even of the exact dyadic rational.

/// The canonical quiet-NaN payload.
pub const NAN: u64 = 0x7ff8_0000_0000_0000;
/// The positive-infinity payload.
pub const INF: u64 = 0x7ff0_0000_0000_0000;
/// The sign bit.
pub const SIGN: u64 = 0x8000_0000_0000_0000;
/// The negative-infinity payload.
pub const NEG_INF: u64 = SIGN | INF;
/// The largest finite payload.
pub const MAX_FINITE: u64 = 0x7fef_ffff_ffff_ffff;

/// Collapse every zero to +0 and every NaN encoding to the canonical quiet
/// NaN — pure integer classification of the exponent/fraction fields.
#[must_use]
pub fn canonical(bits: u64) -> u64 {
    let magnitude = bits & !SIGN;
    if magnitude == 0 {
        0
    } else if magnitude > INF {
        NAN
    } else {
        bits
    }
}

/// Is the payload already canonical?
#[must_use]
pub fn is_canonical(bits: u64) -> bool {
    canonical(bits) == bits
}

/// The total-order key: `-Infinity < negative finite < 0 < positive finite <
/// +Infinity < NaN`, as an unsigned word — sign/complement mapping over
/// CANONICAL payloads (NaN is positive-side and largest by construction).
#[must_use]
pub fn order_key(bits: u64) -> u64 {
    debug_assert!(is_canonical(bits), "order keys read canonical payloads");
    if bits & SIGN == 0 { bits | SIGN } else { !bits }
}

/// Invert the total-order key back to the canonical payload.
#[must_use]
pub fn order_key_inverse(key: u64) -> u64 {
    if key & SIGN != 0 { key & !SIGN } else { !key }
}

/// The value class of a canonical payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Nan,
    PosInf,
    NegInf,
    Finite,
}

/// Classify a canonical payload.
#[must_use]
pub fn classify(bits: u64) -> Class {
    match bits {
        NAN => Class::Nan,
        INF => Class::PosInf,
        NEG_INF => Class::NegInf,
        _ => Class::Finite,
    }
}

/// A finite payload's (significand, binade) decomposition: the magnitude in
/// 2^-1074 scaled units is `sig << shift`, with `sig < 2^53`.
///
/// # Panics
/// Never: the 11-bit exponent field always fits `u32`.
#[must_use]
pub fn decompose(bits: u64) -> (bool, u64, u32) {
    debug_assert_eq!(classify(canonical(bits)), Class::Finite);
    let neg = bits & SIGN != 0;
    let magnitude = bits & !SIGN;
    // An 11-bit exponent field: the shift leaves at most 12 significant bits.
    let exponent = u32::try_from(magnitude >> 52).expect("an 11-bit exponent field");
    let fraction = magnitude & ((1u64 << 52) - 1);
    if exponent == 0 {
        (neg, fraction, 0)
    } else {
        (neg, (1u64 << 52) | fraction, exponent - 1)
    }
}

/// A little-endian unsigned big integer — the oracle's own arithmetic, kept
/// deliberately tiny: exactly the operations the exact accumulator and the
/// dyadic rounding need.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Wide {
    limbs: Vec<u64>,
}

impl Wide {
    #[must_use]
    pub fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    #[must_use]
    pub fn from_u64(v: u64) -> Self {
        let mut w = Self::zero();
        if v != 0 {
            w.limbs.push(v);
        }
        w
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "deliberate limb splitting of the u128"
    )]
    pub fn from_u128(v: u128) -> Self {
        let mut w = Self::zero();
        let lo = v as u64;
        let hi = (v >> 64) as u64;
        if hi != 0 {
            w.limbs = vec![lo, hi];
        } else if lo != 0 {
            w.limbs = vec![lo];
        }
        w
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// The bit length (0 for zero).
    ///
    /// # Panics
    /// Never: a limb's leading-zero count fits `usize`.
    #[must_use]
    pub fn bit_len(&self) -> usize {
        match self.limbs.last() {
            None => 0,
            Some(top) => {
                self.limbs.len() * 64 - usize::try_from(top.leading_zeros()).expect("small")
            }
        }
    }

    #[must_use]
    pub fn cmp_wide(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match self.limbs.len().cmp(&other.limbs.len()) {
            Ordering::Equal => {}
            ord => return ord,
        }
        for i in (0..self.limbs.len()).rev() {
            match self.limbs[i].cmp(&other.limbs[i]) {
                Ordering::Equal => {}
                ord => return ord,
            }
        }
        Ordering::Equal
    }

    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        let mut limbs = Vec::with_capacity(self.limbs.len().max(other.limbs.len()) + 1);
        let mut carry = 0u64;
        for i in 0..self.limbs.len().max(other.limbs.len()) {
            let a = self.limbs.get(i).copied().unwrap_or(0);
            let b = other.limbs.get(i).copied().unwrap_or(0);
            let (s1, c1) = a.overflowing_add(b);
            let (s2, c2) = s1.overflowing_add(carry);
            limbs.push(s2);
            carry = u64::from(c1) + u64::from(c2);
        }
        if carry != 0 {
            limbs.push(carry);
        }
        let mut w = Self { limbs };
        w.trim();
        w
    }

    /// `self - other`; the caller guarantees `other <= self`.
    ///
    /// # Panics
    /// On underflow — an oracle bug, never tolerated silently.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        assert_ne!(
            self.cmp_wide(other),
            std::cmp::Ordering::Less,
            "oracle subtraction underflow"
        );
        let mut limbs = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0u64;
        for i in 0..self.limbs.len() {
            let a = self.limbs[i];
            let b = other.limbs.get(i).copied().unwrap_or(0);
            let (d1, b1) = a.overflowing_sub(b);
            let (d2, b2) = d1.overflowing_sub(borrow);
            limbs.push(d2);
            borrow = u64::from(b1) + u64::from(b2);
        }
        assert_eq!(borrow, 0, "oracle subtraction underflow");
        let mut w = Self { limbs };
        w.trim();
        w
    }

    #[must_use]
    pub fn shl(&self, bits: usize) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        let limb_shift = bits / 64;
        let bit_shift = bits % 64;
        let mut limbs = vec![0u64; limb_shift];
        if bit_shift == 0 {
            limbs.extend_from_slice(&self.limbs);
        } else {
            let mut carry = 0u64;
            for &limb in &self.limbs {
                limbs.push((limb << bit_shift) | carry);
                carry = limb >> (64 - bit_shift);
            }
            if carry != 0 {
                limbs.push(carry);
            }
        }
        let mut w = Self { limbs };
        w.trim();
        w
    }

    /// Shift right, returning the quotient; the shifted-out low bits are the
    /// caller's to reconstruct exactly via `sub` when needed.
    #[must_use]
    pub fn shr(&self, bits: usize) -> Self {
        let limb_shift = bits / 64;
        if limb_shift >= self.limbs.len() {
            return Self::zero();
        }
        let bit_shift = bits % 64;
        let mut limbs = Vec::with_capacity(self.limbs.len() - limb_shift);
        if bit_shift == 0 {
            limbs.extend_from_slice(&self.limbs[limb_shift..]);
        } else {
            for i in limb_shift..self.limbs.len() {
                let lo = self.limbs[i] >> bit_shift;
                let hi = self
                    .limbs
                    .get(i + 1)
                    .map_or(0, |next| next << (64 - bit_shift));
                limbs.push(lo | hi);
            }
        }
        let mut w = Self { limbs };
        w.trim();
        w
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "deliberate limb splitting of the carry product"
    )]
    pub fn mul_u64(&self, factor: u64) -> Self {
        if factor == 0 || self.is_zero() {
            return Self::zero();
        }
        let mut limbs = Vec::with_capacity(self.limbs.len() + 1);
        let mut carry = 0u128;
        for &limb in &self.limbs {
            let prod = u128::from(limb) * u128::from(factor) + carry;
            limbs.push(prod as u64);
            carry = prod >> 64;
        }
        if carry != 0 {
            limbs.push(carry as u64);
        }
        let mut w = Self { limbs };
        w.trim();
        w
    }

    /// Single-limb division: `(self / d, self % d)`.
    ///
    /// # Panics
    /// On a zero divisor — an oracle bug.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the running remainder is strictly below the u64 divisor"
    )]
    pub fn divmod_u64(&self, d: u64) -> (Self, u64) {
        assert_ne!(d, 0, "oracle division by zero");
        let mut quotient = vec![0u64; self.limbs.len()];
        let mut rem = 0u128;
        for i in (0..self.limbs.len()).rev() {
            let acc = (rem << 64) | u128::from(self.limbs[i]);
            quotient[i] = (acc / u128::from(d)) as u64;
            rem = acc % u128::from(d);
        }
        let mut q = Self { limbs: quotient };
        q.trim();
        (q, rem as u64)
    }

    /// The low `bits` bits as a `Wide`.
    #[must_use]
    pub fn low_bits(&self, bits: usize) -> Self {
        let limb_count = bits.div_ceil(64);
        let mut limbs: Vec<u64> = self.limbs.iter().copied().take(limb_count).collect();
        let partial = bits % 64;
        if partial != 0 && limbs.len() == limb_count {
            let mask = (1u64 << partial) - 1;
            if let Some(top) = limbs.last_mut() {
                *top &= mask;
            }
        }
        let mut w = Self { limbs };
        w.trim();
        w
    }

    /// The value as `u64`; the caller guarantees it fits.
    ///
    /// # Panics
    /// If the value exceeds `u64` — an oracle bug.
    #[must_use]
    pub fn to_u64(&self) -> u64 {
        assert!(self.limbs.len() <= 1, "oracle narrowing overflow");
        self.limbs.first().copied().unwrap_or(0)
    }
}

/// A finite payload's exact magnitude in 2^-1074 scaled units.
#[must_use]
pub fn scaled_magnitude(bits: u64) -> Wide {
    let (_, sig, shift) = decompose(bits);
    Wide::from_u64(sig).shl(shift as usize)
}

/// Round the exact nonnegative dyadic rational `num / (den << shift)`
/// (in 2^-1074 scaled units, `den >= 1`) to the nearest representable
/// binary64 magnitude payload, ties to even; overflow past the top binade is
/// the infinity payload. ONE rounding of the exact value — `den = count` is
/// the mean, `shift` carries multiplication/division scale, and `den = 1,
/// shift = 0` is the exact sum.
///
/// # Panics
/// On `den == 0` — the oracle denominator is at least one.
#[must_use]
pub fn round_dyadic(num: &Wide, den: u64, shift: usize) -> u64 {
    assert!(den >= 1, "oracle denominator");
    // q1 = num / den exactly, with single-limb remainder.
    let (q1, r1) = num.divmod_u64(den);
    // q0 = floor(value) = q1 >> shift; exact remainder over den << shift is
    // (q1 mod 2^shift) * den + r1.
    let q0 = q1.shr(shift);
    let rem = q1.low_bits(shift).mul_u64(den).add(&Wide::from_u64(r1));
    let den_wide = Wide::from_u64(den).shl(shift);
    if q0.bit_len() <= 53 {
        // Every integer scaled magnitude below 2^53 is representable
        // (subnormals and the first normal binade): round to the nearest
        // integer, ties to even.
        let q0v = q0.to_u64();
        if q0v < (1u64 << 53) {
            let twice = rem.mul_u64(2);
            let up = match twice.cmp_wide(&den_wide) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Equal => q0v & 1 == 1,
                std::cmp::Ordering::Less => false,
            };
            let q = q0v + u64::from(up);
            if q < (1u64 << 53) {
                return q;
            }
            return encode_magnitude(1u64 << 52, 1);
        }
    }
    // The 53-bit quotient at the binade's step.
    let s = q0.bit_len() - 53;
    let t = q0.shr(s);
    // Exact remainder against den << (shift + s):
    // num - t * (den << (shift + s)).
    let step_den = Wide::from_u64(den).shl(shift + s);
    let consumed = t.mul_u64(den).shl(shift + s);
    let rem_total = num.sub(&consumed);
    let twice = rem_total.mul_u64(2);
    let tv = t.to_u64();
    let up = match twice.cmp_wide(&step_den) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => tv & 1 == 1,
        std::cmp::Ordering::Less => false,
    };
    let tv = tv + u64::from(up);
    if tv == 1u64 << 53 {
        encode_magnitude(1u64 << 52, s + 1)
    } else {
        encode_magnitude(tv, s)
    }
}

/// Encode a 53-bit quotient `q in [2^52, 2^53)` at step exponent `s`;
/// past the top binade the result is the infinity payload.
#[must_use]
fn encode_magnitude(q: u64, s: usize) -> u64 {
    debug_assert!((1u64 << 52..1u64 << 53).contains(&q));
    let exponent = s + 1;
    if exponent > 2046 {
        return INF;
    }
    ((u64::try_from(exponent).expect("bounded exponent")) << 52) | (q - (1u64 << 52))
}

/// Apply a sign to a rounded magnitude payload, collapsing negative zero.
#[must_use]
pub fn signed_bits(neg: bool, magnitude_bits: u64) -> u64 {
    if magnitude_bits == 0 || !neg {
        magnitude_bits
    } else {
        SIGN | magnitude_bits
    }
}

/// The exact numerical total: one case, no redundant flag combinations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Total {
    Finite { neg: bool, mag: Wide },
    PosInf,
    NegInf,
    Nan,
}

impl Total {
    #[must_use]
    pub fn zero() -> Self {
        Total::Finite {
            neg: false,
            mag: Wide::zero(),
        }
    }

    /// The canonical merge table (chapter 11 §4): finite parts add exactly
    /// (signed magnitude arithmetic), same-sign infinities keep their sign,
    /// mixed infinities and anything with NaN are NaN.
    #[must_use]
    pub fn merge(&self, other: &Total) -> Total {
        use Total::{Finite, Nan, NegInf, PosInf};
        match (self, other) {
            (Nan, _) | (_, Nan) | (PosInf, NegInf) | (NegInf, PosInf) => Nan,
            (PosInf, _) | (_, PosInf) => PosInf,
            (NegInf, _) | (_, NegInf) => NegInf,
            (Finite { neg: n1, mag: m1 }, Finite { neg: n2, mag: m2 }) => {
                if n1 == n2 {
                    Finite {
                        neg: *n1,
                        mag: m1.add(m2),
                    }
                } else {
                    match m1.cmp_wide(m2) {
                        std::cmp::Ordering::Equal => Total::zero(),
                        std::cmp::Ordering::Greater => Finite {
                            neg: *n1,
                            mag: m1.sub(m2),
                        },
                        std::cmp::Ordering::Less => Finite {
                            neg: *n2,
                            mag: m2.sub(m1),
                        },
                    }
                }
            }
        }
    }
}

/// The accumulator: exact total plus exact count. Merging is associative and
/// commutative and NOT idempotent — deduplication precedes accumulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acc {
    pub total: Total,
    pub count: u64,
}

impl Acc {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            total: Total::zero(),
            count: 0,
        }
    }

    /// # Panics
    /// On a merged cardinality past `u64` (unreachable for real groups).
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            total: self.total.merge(&other.total),
            count: self
                .count
                .checked_add(other.count)
                .expect("oracle cardinality overflow"),
        }
    }

    /// One canonical value's contribution.
    #[must_use]
    pub fn of_bits(bits: u64) -> Self {
        let bits = canonical(bits);
        let total = match classify(bits) {
            Class::Nan => Total::Nan,
            Class::PosInf => Total::PosInf,
            Class::NegInf => Total::NegInf,
            Class::Finite => Total::Finite {
                neg: bits & SIGN != 0,
                mag: scaled_magnitude(bits),
            },
        };
        Self { total, count: 1 }
    }
}

/// Fold an already-deduplicated binding list.
#[must_use]
pub fn fold(values: &[u64]) -> Acc {
    values
        .iter()
        .fold(Acc::empty(), |acc, &bits| acc.merge(&Acc::of_bits(bits)))
}

/// `sum`: the exact total rounded once, canonical. The empty group is the
/// CALLER's business — no binding, no group, no answer row.
#[must_use]
pub fn sum_bits(values: &[u64]) -> u64 {
    finalize(&fold(values).total, 1)
}

/// `mean`: the exact rational total/count rounded ONCE — never
/// `rounded_sum / count`. `None` on empty input.
#[must_use]
pub fn mean_bits(values: &[u64]) -> Option<u64> {
    let acc = fold(values);
    if acc.count == 0 {
        return None;
    }
    Some(finalize(&acc.total, acc.count))
}

fn finalize(total: &Total, den: u64) -> u64 {
    match total {
        Total::Nan => NAN,
        Total::PosInf => INF,
        Total::NegInf => NEG_INF,
        Total::Finite { neg, mag } => signed_bits(*neg, round_dyadic(mag, den, 0)),
    }
}

/// Correctly rounded reference addition of two canonical payloads — exact
/// scaled integers, one rounding. IEEE special cases per the table.
#[must_use]
pub fn ref_add(a: u64, b: u64) -> u64 {
    let acc = Acc::of_bits(a).merge(&Acc::of_bits(b));
    finalize(&acc.total, 1)
}

/// Correctly rounded reference subtraction.
#[must_use]
pub fn ref_sub(a: u64, b: u64) -> u64 {
    ref_add(a, ref_neg(b))
}

/// Negation with the canonical zero-sign collapse.
#[must_use]
pub fn ref_neg(a: u64) -> u64 {
    match classify(canonical(a)) {
        Class::Nan => NAN,
        Class::PosInf => NEG_INF,
        Class::NegInf => INF,
        Class::Finite => canonical(canonical(a) ^ SIGN),
    }
}

/// Correctly rounded reference multiplication: significand product with the
/// exact dyadic scale, one rounding.
#[must_use]
pub fn ref_mul(a: u64, b: u64) -> u64 {
    let a = canonical(a);
    let b = canonical(b);
    match (classify(a), classify(b)) {
        (Class::Nan, _) | (_, Class::Nan) => return NAN,
        (Class::PosInf | Class::NegInf, _) | (_, Class::PosInf | Class::NegInf) => {
            let a_zero = classify(a) == Class::Finite && a & !SIGN == 0;
            let b_zero = classify(b) == Class::Finite && b & !SIGN == 0;
            if a_zero || b_zero {
                return NAN; // 0 * inf
            }
            let neg = (a & SIGN != 0) != (b & SIGN != 0);
            return if neg { NEG_INF } else { INF };
        }
        (Class::Finite, Class::Finite) => {}
    }
    let (na, sa, ea) = decompose(a);
    let (nb, sb, eb) = decompose(b);
    let neg = na != nb;
    if sa == 0 || sb == 0 {
        return 0; // canonical zero
    }
    // value = sa*sb * 2^(ea+eb) * 2^-2148 = (sa*sb << (ea+eb)) / 2^1074
    // in scaled units.
    let product = Wide::from_u128(u128::from(sa) * u128::from(sb));
    let scale = ea as usize + eb as usize;
    let bits = if scale >= 1074 {
        round_dyadic(&product.shl(scale - 1074), 1, 0)
    } else {
        round_dyadic(&product, 1, 1074 - scale)
    };
    signed_bits(neg, bits)
}

/// Correctly rounded reference division: exact dyadic ratio of significands,
/// one rounding.
#[must_use]
pub fn ref_div(a: u64, b: u64) -> u64 {
    let a = canonical(a);
    let b = canonical(b);
    match (classify(a), classify(b)) {
        (Class::Nan, _)
        | (_, Class::Nan)
        | (Class::PosInf | Class::NegInf, Class::PosInf | Class::NegInf) => return NAN,
        (Class::PosInf | Class::NegInf, Class::Finite) => {
            let neg = (a & SIGN != 0) != (b & SIGN != 0);
            return if neg { NEG_INF } else { INF };
        }
        (Class::Finite, Class::PosInf | Class::NegInf) => {
            // finite / inf = canonical zero (sign collapses).
            return 0;
        }
        (Class::Finite, Class::Finite) => {}
    }
    let (na, sa, ea) = decompose(a);
    let (nb, sb, eb) = decompose(b);
    let neg = na != nb;
    if sb == 0 {
        if sa == 0 {
            return NAN; // 0 / 0
        }
        return if neg { NEG_INF } else { INF }; // x / 0
    }
    if sa == 0 {
        return 0;
    }
    // Real value: (sa * 2^(ea-1074)) / (sb * 2^(eb-1074)); in 2^-1074
    // scaled units that is (sa << (ea + 1074)) / (sb << eb) — an exact
    // dyadic rational the rounding routine consumes whole, in ONE rounding.
    let num = Wide::from_u64(sa).shl(ea as usize + 1074);
    let bits = round_dyadic(&num, sb, eb as usize);
    signed_bits(neg, bits)
}

#[cfg(test)]
mod tests;
