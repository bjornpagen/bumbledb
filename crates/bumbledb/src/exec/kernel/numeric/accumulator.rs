//! Exact SUM and AVG over binary64: Neal's small superaccumulator (R. M.
//! Neal, "Fast exact summation using small and large superaccumulators",
//! arXiv:1505.05571). A value adds its 53-bit significand, split at a 32-bit
//! boundary chosen by its exponent, into two of 67 signed 64-bit chunks;
//! carries propagate lazily, at most every 1023 adds. The exact total rounds
//! once, at output, to nearest even.

use super::FloatCardinalityOverflow;
use bumbledb_theory::F64;
use core::num::NonZeroU64;

const CHUNKS: usize = 67;
const LOW_MASK: u64 = (1 << 32) - 1;
/// Normalized chunks are below 2^32 and one add moves a chunk by less than
/// 2^52, so 1023 adds keep every chunk below 2^62.
const ADDS_PER_PROPAGATION: u32 = 1023;
const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
const FRACTION: u64 = 0x000f_ffff_ffff_ffff;
/// Magnitude limbs at output: totals of up to `u64::MAX` values stay below
/// 2^2162 units of 2^-1074.
const LIMBS: usize = 34;

/// The superaccumulator. Chunk `i` weighs `2^(32 i - 1075)`.
#[derive(Clone, Debug)]
struct Chunks {
    chunk: [i64; CHUNKS],
    until_propagate: u32,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "the chunks live inline in each group's accumulator; no allocation per group"
)]
enum Total {
    Finite(Chunks),
    PositiveInfinity,
    NegativeInfinity,
    NaN,
}

#[derive(Clone, Debug, Default)]
#[expect(
    clippy::large_enum_variant,
    reason = "empty and nonempty are a semantic distinction, not a reason to box"
)]
enum State {
    #[default]
    Empty,
    NonEmpty {
        count: NonZeroU64,
        total: Total,
    },
}

/// The exact sum and count of the pushed binary64 values. Merging is not
/// idempotent: partitions must be disjoint.
#[derive(Clone, Debug, Default)]
pub(crate) struct ExactF64Accumulator {
    state: State,
}

impl PartialEq for ExactF64Accumulator {
    fn eq(&self, other: &Self) -> bool {
        match (&self.state, &other.state) {
            (State::Empty, State::Empty) => true,
            (
                State::NonEmpty { count, total },
                State::NonEmpty {
                    count: other_count,
                    total: other_total,
                },
            ) => {
                count == other_count
                    && match (total, other_total) {
                        (Total::Finite(a), Total::Finite(b)) => a.magnitude() == b.magnitude(),
                        (Total::PositiveInfinity, Total::PositiveInfinity)
                        | (Total::NegativeInfinity, Total::NegativeInfinity)
                        | (Total::NaN, Total::NaN) => true,
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}

impl Eq for ExactF64Accumulator {}

impl ExactF64Accumulator {
    pub(crate) fn push(&mut self, value: F64) -> Result<(), FloatCardinalityOverflow> {
        match &mut self.state {
            State::Empty => {
                self.state = State::NonEmpty {
                    count: NonZeroU64::MIN,
                    total: Total::of(value),
                };
            }
            State::NonEmpty { count, total } => {
                *count = count.checked_add(1).ok_or(FloatCardinalityOverflow)?;
                total.add(value.to_bits());
            }
        }
        Ok(())
    }

    /// Adds `value` `count` times without a loop over the multiplicity.
    pub(crate) fn push_repeated(
        &mut self,
        value: F64,
        count: u64,
    ) -> Result<(), FloatCardinalityOverflow> {
        let Some(count) = NonZeroU64::new(count) else {
            return Ok(());
        };
        let mut total = Total::Finite(Chunks::new());
        total.add_repeated(value.to_bits(), count.get());
        self.merge_total(count, &total)
    }

    /// Pushes every value of `keys`, given as F64 order keys, through four
    /// lane-private superaccumulators.
    pub(crate) fn push_keys(
        &mut self,
        keys: impl ExactSizeIterator<Item = u64>,
    ) -> Result<(), FloatCardinalityOverflow> {
        let Some(count) = NonZeroU64::new(keys.len() as u64) else {
            return Ok(());
        };
        let mut lanes = [Chunks::new(), Chunks::new(), Chunks::new(), Chunks::new()];
        let mut special: Option<Total> = None;
        for (i, key) in keys.enumerate() {
            let bits = order_key_to_bits(key);
            if bits & EXPONENT == EXPONENT {
                let value = Total::special(bits);
                special = Some(match special {
                    None => value,
                    Some(mut total) => {
                        total.merge(&value);
                        total
                    }
                });
            } else {
                lanes[i & 3].add_finite(bits);
            }
        }
        let [mut total, b, c, d] = lanes;
        for lane in [b, c, d] {
            total.merge(&lane);
        }
        let mut total = Total::Finite(total);
        if let Some(special) = special {
            total.merge(&special);
        }
        self.merge_total(count, &total)
    }

    /// Adds a disjoint partition. A cardinality failure leaves `self`
    /// unchanged.
    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), FloatCardinalityOverflow> {
        match &other.state {
            State::Empty => Ok(()),
            State::NonEmpty { count, total } => self.merge_total(*count, total),
        }
    }

    fn merge_total(
        &mut self,
        other_count: NonZeroU64,
        other_total: &Total,
    ) -> Result<(), FloatCardinalityOverflow> {
        match &mut self.state {
            State::Empty => {
                self.state = State::NonEmpty {
                    count: other_count,
                    total: other_total.clone(),
                };
            }
            State::NonEmpty { count, total } => {
                *count = count
                    .checked_add(other_count.get())
                    .ok_or(FloatCardinalityOverflow)?;
                total.merge(other_total);
            }
        }
        Ok(())
    }

    pub(crate) fn sum(&self) -> Option<F64> {
        self.round(NonZeroU64::MIN)
    }

    /// The exact total divided by the exact count, rounded once.
    pub(crate) fn mean(&self) -> Option<F64> {
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
            Total::Finite(chunks) => chunks.magnitude().round(divisor.get()),
            Total::PositiveInfinity => F64::INFINITY,
            Total::NegativeInfinity => F64::NEG_INFINITY,
            Total::NaN => F64::NAN,
        })
    }

    #[cfg(test)]
    fn count(&self) -> u64 {
        match &self.state {
            State::Empty => 0,
            State::NonEmpty { count, .. } => count.get(),
        }
    }

    /// The exact finite total, if any.
    #[cfg(test)]
    fn magnitude(&self) -> Option<Magnitude> {
        match &self.state {
            State::NonEmpty {
                total: Total::Finite(chunks),
                ..
            } => Some(chunks.magnitude()),
            _ => None,
        }
    }
}

impl Total {
    fn of(value: F64) -> Self {
        let mut total = Self::Finite(Chunks::new());
        total.add(value.to_bits());
        total
    }

    /// Adds one canonical binary64.
    fn add(&mut self, bits: u64) {
        if bits & EXPONENT == EXPONENT {
            self.merge(&Self::special(bits));
        } else if let Self::Finite(chunks) = self {
            chunks.add_finite(bits);
        }
    }

    /// The total of one infinity or NaN.
    fn special(bits: u64) -> Self {
        if bits & FRACTION != 0 {
            Self::NaN
        } else if bits >> 63 == 0 {
            Self::PositiveInfinity
        } else {
            Self::NegativeInfinity
        }
    }

    fn add_repeated(&mut self, bits: u64, count: u64) {
        if bits & EXPONENT == EXPONENT {
            self.add(bits);
        } else if let Self::Finite(chunks) = self {
            chunks.add_finite_repeated(bits, count);
        }
    }

    fn merge(&mut self, other: &Self) {
        match (&mut *self, other) {
            (Self::NaN, _) => {}
            (_, Self::NaN)
            | (Self::PositiveInfinity, Self::NegativeInfinity)
            | (Self::NegativeInfinity, Self::PositiveInfinity) => *self = Self::NaN,
            (Self::Finite(left), Self::Finite(right)) => left.merge(right),
            (Self::Finite(_), Self::PositiveInfinity) => *self = Self::PositiveInfinity,
            (Self::Finite(_), Self::NegativeInfinity) => *self = Self::NegativeInfinity,
            (Self::PositiveInfinity | Self::NegativeInfinity, _) => {}
        }
    }
}

/// A finite binary64's significand and its chunk placement: the value is
/// `significand * 2^(exponent - 1075)` with `exponent >= 1`, so it starts at
/// bit `exponent % 32` of chunk `exponent / 32`.
fn split(bits: u64) -> (bool, u64, u32, usize) {
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & FRACTION;
    let (significand, exponent) = if biased == 0 {
        (fraction, 1)
    } else {
        (fraction | (1 << 52), biased)
    };
    (
        bits >> 63 != 0,
        significand,
        u32::try_from(exponent & 31).expect("five bits"),
        usize::try_from(exponent >> 5).expect("six bits"),
    )
}

impl Chunks {
    const fn new() -> Self {
        Self {
            chunk: [0; CHUNKS],
            until_propagate: ADDS_PER_PROPAGATION,
        }
    }

    #[expect(
        clippy::cast_possible_wrap,
        reason = "both parts of a significand are below 2^53"
    )]
    fn add_finite(&mut self, bits: u64) {
        let (negative, significand, low_exp, high_exp) = split(bits);
        let low = ((significand << low_exp) & LOW_MASK) as i64;
        let high = (significand >> (32 - low_exp)) as i64;
        let sign = -i64::from(negative);
        self.chunk[high_exp] += (low ^ sign) - sign;
        self.chunk[high_exp + 1] += (high ^ sign) - sign;
        self.until_propagate -= 1;
        if self.until_propagate == 0 {
            self.propagate();
        }
    }

    /// Adds `count * value`: the up to 117-bit product spreads over 32-bit
    /// digits of consecutive chunks, and the top chunk takes every remaining
    /// high bit (below 2^52, since the top chunk is at least three above the
    /// value's first).
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "masked 32-bit digits, and a top remainder below 2^52"
    )]
    fn add_finite_repeated(&mut self, bits: u64, count: u64) {
        let (negative, significand, low_exp, high_exp) = split(bits);
        let product = u128::from(significand) * u128::from(count);
        let sign = -i64::from(negative);
        for (k, index) in (high_exp..CHUNKS).enumerate() {
            let digit = if k == 0 {
                ((product as u64) << low_exp) & LOW_MASK
            } else {
                let shift = 32 * k as u32 - low_exp;
                if shift >= 128 {
                    break;
                }
                let rest = product >> shift;
                if index == CHUNKS - 1 {
                    rest as u64
                } else {
                    rest as u64 & LOW_MASK
                }
            } as i64;
            self.chunk[index] += (digit ^ sign) - sign;
        }
        self.propagate();
    }

    /// Leaves chunks `0..66` in `[0, 2^32)` and the signed rest in the top
    /// chunk.
    fn propagate(&mut self) {
        for i in 0..CHUNKS - 1 {
            let c = self.chunk[i];
            self.chunk[i] = c & LOW_MASK.cast_signed();
            self.chunk[i + 1] += c >> 32;
        }
        self.until_propagate = ADDS_PER_PROPAGATION;
    }

    fn merge(&mut self, other: &Self) {
        let mut other = other.clone();
        other.propagate();
        self.propagate();
        for (left, right) in self.chunk.iter_mut().zip(other.chunk) {
            *left += right;
        }
        self.propagate();
    }

    /// The exact total as sign and magnitude in units of 2^-1074.
    #[expect(
        clippy::cast_sign_loss,
        reason = "propagated chunks below the top are 32-bit digits; the top is two's complement"
    )]
    fn magnitude(&self) -> Magnitude {
        let mut chunks = self.clone();
        chunks.propagate();
        let top = chunks.chunk[CHUNKS - 1];
        // Two's complement over 35 limbs, in units of 2^-1075.
        let mut words = [0u64; LIMBS + 1];
        for (word, pair) in words
            .iter_mut()
            .zip(chunks.chunk[..CHUNKS - 1].as_chunks::<2>().0)
        {
            *word = pair[0] as u64 | (pair[1] as u64) << 32;
        }
        words[LIMBS - 1] = top.cast_unsigned();
        words[LIMBS] = if top < 0 { u64::MAX } else { 0 };
        let negative = top < 0;
        if negative {
            let mut carry = true;
            for word in &mut words {
                let (value, overflow) = (!*word).overflowing_add(u64::from(carry));
                *word = value;
                carry = overflow;
            }
        }
        // Every term is even in units of 2^-1075: halve into 2^-1074 units.
        let mut limbs = [0u64; LIMBS];
        for (i, limb) in limbs.iter_mut().enumerate() {
            *limb = (words[i] >> 1) | (words[i + 1] << 63);
        }
        debug_assert_eq!(words[0] & 1, 0, "terms are even");
        debug_assert_eq!(words[LIMBS] >> 1, 0, "totals stay below 2^2162");
        Magnitude {
            negative: negative && limbs.iter().any(|&limb| limb != 0),
            limbs,
        }
    }
}

/// An exact finite total: sign and little-endian magnitude limbs in units of
/// 2^-1074. Zero is positive.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Magnitude {
    negative: bool,
    limbs: [u64; LIMBS],
}

impl Magnitude {
    /// `self / divisor`, rounded once to nearest even.
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

fn order_key_to_bits(key: u64) -> u64 {
    if key >> 63 == 0 {
        !key
    } else {
        key ^ (1 << 63)
    }
}

#[cfg(test)]
mod limbs;
#[cfg(test)]
mod tests;
