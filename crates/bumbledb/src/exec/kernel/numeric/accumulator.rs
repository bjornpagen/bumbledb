//! Exact integer multiples of 2^-1074, rounded only at an output boundary.
//!
//! Bound: every finite binary64 magnitude is at most
//! `(2^53 - 1) * 2^2045 < 2^2098` in scaled units. For n <= 2^64-1,
//! the triangle inequality gives `abs(total) < 2^(2098+64) = 2^2162`.
//! A 34-limb signed magnitude has 2176 magnitude bits, so neither a valid push
//! nor a count-checked merge can overflow its finite storage. This is the
//! implementation bound argument, not a completed Lean proof.

use super::FloatCardinalityOverflow;
use bumbledb_theory::F64;
use core::cmp::Ordering;
use core::num::NonZeroU64;

const LIMBS: usize = 34;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Finite {
    negative: bool,
    // Little-endian limbs; zero's sign is always positive.
    limbs: [u64; LIMBS],
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "fixed inline accumulator storage avoids an allocation per float group"
)]
enum Total {
    Finite(Finite),
    PositiveInfinity,
    NegativeInfinity,
    NaN,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "empty/nonempty is a semantic distinction, not a reason to heap-allocate finite groups"
)]
enum State {
    #[default]
    Empty,
    NonEmpty {
        count: NonZeroU64,
        total: Total,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ExactF64Accumulator {
    state: State,
}

impl ExactF64Accumulator {
    pub(super) fn push(&mut self, value: F64) -> Result<(), FloatCardinalityOverflow> {
        let total = if value.is_nan() {
            Total::NaN
        } else if value == F64::INFINITY {
            Total::PositiveInfinity
        } else if value == F64::NEG_INFINITY {
            Total::NegativeInfinity
        } else {
            Total::Finite(Finite::from_value(value))
        };
        self.merge(&Self {
            state: State::NonEmpty {
                count: NonZeroU64::MIN,
                total,
            },
        })
    }

    /// Partitions must be disjoint after binding deduplication. Merge is not
    /// idempotent, and this numerical state carries no binding provenance.
    /// Cardinality failure leaves the original accumulator unchanged.
    pub(super) fn merge(&mut self, other: &Self) -> Result<(), FloatCardinalityOverflow> {
        let State::NonEmpty {
            count: other_count,
            total: other_total,
        } = &other.state
        else {
            return Ok(());
        };
        match &mut self.state {
            State::Empty => self.state = other.state.clone(),
            State::NonEmpty { count, total } => {
                let merged_count = count
                    .checked_add(other_count.get())
                    .ok_or(FloatCardinalityOverflow)?;
                total.merge(other_total);
                *count = merged_count;
            }
        }
        Ok(())
    }

    pub(super) fn sum(&self) -> Option<F64> {
        self.round(NonZeroU64::MIN)
    }

    pub(super) fn mean(&self) -> Option<F64> {
        match &self.state {
            State::Empty => None,
            State::NonEmpty { count, .. } => self.round(*count),
        }
    }

    fn round(&self, divisor: NonZeroU64) -> Option<F64> {
        let State::NonEmpty { total, .. } = &self.state else {
            return None;
        };
        Some(match total {
            Total::Finite(finite) => finite.round(divisor.get()),
            Total::PositiveInfinity => F64::INFINITY,
            Total::NegativeInfinity => F64::NEG_INFINITY,
            Total::NaN => F64::NAN,
        })
    }
}

impl Total {
    fn merge(&mut self, other: &Self) {
        match (&mut *self, other) {
            (Self::NaN, _) => {}
            (_, Self::NaN)
            | (Self::PositiveInfinity, Self::NegativeInfinity)
            | (Self::NegativeInfinity, Self::PositiveInfinity) => *self = Self::NaN,
            (Self::Finite(left), Self::Finite(right)) => left.add(right),
            (Self::Finite(_), Self::PositiveInfinity) => *self = Self::PositiveInfinity,
            (Self::Finite(_), Self::NegativeInfinity) => *self = Self::NegativeInfinity,
            (Self::PositiveInfinity | Self::NegativeInfinity, _) => {}
        }
    }
}

impl Finite {
    fn from_value(value: F64) -> Self {
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let fraction = bits & 0x000f_ffff_ffff_ffff;
        let (significand, shift) = if exponent == 0 {
            (fraction, 0)
        } else {
            (fraction | (1 << 52), exponent - 1)
        };
        let mut limbs = [0; LIMBS];
        let limb = shift / 64;
        let offset = shift % 64;
        limbs[limb] = significand << offset;
        if offset != 0 {
            limbs[limb + 1] = significand >> (64 - offset);
        }
        Self {
            negative: bits >> 63 != 0,
            limbs,
        }
    }

    fn add(&mut self, other: &Self) {
        if self.negative == other.negative {
            let mut carry = false;
            for (left, right) in self.limbs.iter_mut().zip(other.limbs) {
                let (sum, carry_a) = left.overflowing_add(right);
                let (sum, carry_b) = sum.overflowing_add(u64::from(carry));
                *left = sum;
                carry = carry_a || carry_b;
            }
            debug_assert!(!carry, "count-proved 2162-bit finite bound");
        } else {
            match self.limbs.iter().rev().cmp(other.limbs.iter().rev()) {
                Ordering::Equal => {
                    self.limbs = [0; LIMBS];
                    self.negative = false;
                }
                Ordering::Greater => subtract(&mut self.limbs, &other.limbs),
                Ordering::Less => {
                    let mut magnitude = other.limbs;
                    subtract(&mut magnitude, &self.limbs);
                    self.limbs = magnitude;
                    self.negative = other.negative;
                }
            }
        }
    }

    fn round(&self, divisor: u64) -> F64 {
        let (quotient, remainder) = divide(&self.limbs, divisor);
        let bit_length = quotient
            .iter()
            .rposition(|&word| word != 0)
            .map_or(0, |index| {
                index * 64 + (64 - quotient[index].leading_zeros() as usize)
            });
        let mut shift = bit_length.saturating_sub(53);
        let mut mantissa = shifted_word(&quotient, shift);
        let round_up = if shift == 0 {
            let twice = u128::from(remainder) * 2;
            twice > u128::from(divisor) || (twice == u128::from(divisor) && mantissa & 1 != 0)
        } else {
            let half_bit = shift - 1;
            let half_present = quotient[half_bit / 64] & (1 << (half_bit % 64)) != 0;
            let below_half = quotient[..half_bit / 64].iter().any(|&word| word != 0)
                || quotient[half_bit / 64] & ((1 << (half_bit % 64)) - 1) != 0;
            half_present && (below_half || remainder != 0 || mantissa & 1 != 0)
        };
        mantissa += u64::from(round_up);
        if mantissa == 1 << 53 {
            mantissa >>= 1;
            shift += 1;
        }
        let bits = if mantissa < 1 << 52 {
            mantissa // subnormal; rounding can also produce exact zero
        } else if shift >= 2046 {
            0x7ff0_0000_0000_0000
        } else {
            ((shift as u64 + 1) << 52) | (mantissa & 0x000f_ffff_ffff_ffff)
        };
        F64::from_bits(bits | (u64::from(self.negative) << 63))
    }
}

fn subtract(left: &mut [u64; LIMBS], right: &[u64; LIMBS]) {
    let mut borrow = false;
    for (left, right) in left.iter_mut().zip(right) {
        let (difference, borrow_a) = left.overflowing_sub(*right);
        let (difference, borrow_b) = difference.overflowing_sub(u64::from(borrow));
        *left = difference;
        borrow = borrow_a || borrow_b;
    }
    debug_assert!(!borrow, "the larger magnitude is the minuend");
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "remainder < divisor proves each base-2^64 quotient digit and remainder fit u64"
)]
fn divide(value: &[u64; LIMBS], divisor: u64) -> ([u64; LIMBS], u64) {
    if divisor == 1 {
        return (*value, 0);
    }
    let mut quotient = [0; LIMBS];
    let mut remainder = 0;
    for (out, &word) in quotient.iter_mut().zip(value).rev() {
        let dividend = (u128::from(remainder) << 64) | u128::from(word);
        // remainder < divisor, hence this quotient digit always fits u64.
        *out = (dividend / u128::from(divisor)) as u64;
        remainder = (dividend % u128::from(divisor)) as u64;
    }
    (quotient, remainder)
}

fn shifted_word(value: &[u64; LIMBS], shift: usize) -> u64 {
    let index = shift / 64;
    let offset = shift % 64;
    let low = value[index] >> offset;
    if offset == 0 {
        low
    } else {
        low | (value.get(index + 1).copied().unwrap_or(0) << (64 - offset))
    }
}

#[cfg(test)]
mod tests;
