//! Exact arithmetic with explicit bit/work limits. No floating point rounding
//! enters normalization, source admission or conditional observations.
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::{Capacity, Control, Error, Result};

/// Limits apply to inputs and conservative intermediate bit bounds. They can
/// refuse a calculation whose reduced answer is small; they never round it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithmeticLimits {
    pub bits: usize,
    pub operations: usize,
}

impl Default for ArithmeticLimits {
    fn default() -> Self {
        Self {
            bits: 65_536,
            operations: 1_000_000,
        }
    }
}

/// Counts exact arithmetic primitives across one construction or observation.
/// Cancellation is polled around each bounded bigint primitive, not each limb.
pub struct ExactArithmetic<'a> {
    limits: ArithmeticLimits,
    operations: usize,
    control: &'a dyn Control,
}

impl<'a> ExactArithmetic<'a> {
    #[must_use]
    pub const fn new(limits: ArithmeticLimits, control: &'a dyn Control) -> Self {
        Self {
            limits,
            operations: 0,
            control,
        }
    }

    #[must_use]
    pub const fn operations(&self) -> usize {
        self.operations
    }

    /// The cancellation/work control shared by this arithmetic operation.
    #[must_use]
    pub fn control(&self) -> &'a dyn Control {
        self.control
    }

    pub(crate) fn validate(&mut self, value: &ExactRational) -> Result<()> {
        self.step(value.bits())
    }

    fn step(&mut self, bits: u64) -> Result<()> {
        self.control.checkpoint()?;
        if bits > u64::try_from(self.limits.bits).unwrap_or(u64::MAX) {
            return Err(Error::Capacity(Capacity::ArithmeticBits));
        }
        if self.operations >= self.limits.operations {
            return Err(Error::Capacity(Capacity::ArithmeticSteps));
        }
        self.operations += 1;
        Ok(())
    }

    fn finish(&self) -> Result<()> {
        self.control.checkpoint()
    }
}

/// A reduced arbitrary-precision rational with positive denominator.
/// Equality and hashing compare exact values. Wire encoding is canonical;
/// physical arithmetic allocation never defines source identity.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExactRational(BigRational);

impl std::fmt::Debug for ExactRational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExactRational")
            .field("negative", &self.0.is_negative())
            .field("numerator_bits", &self.0.numer().bits())
            .field("denominator_bits", &self.0.denom().bits())
            .finish()
    }
}

impl std::fmt::Display for ExactRational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u64> for ExactRational {
    fn from(value: u64) -> Self {
        Self(BigRational::from_integer(BigInt::from(value)))
    }
}
impl From<i64> for ExactRational {
    fn from(value: i64) -> Self {
        Self(BigRational::from_integer(BigInt::from(value)))
    }
}

impl ExactRational {
    #[must_use]
    pub fn zero() -> Self {
        Self(BigRational::zero())
    }
    #[must_use]
    pub fn one() -> Self {
        Self(BigRational::one())
    }
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    pub(crate) fn is_integer(&self) -> bool {
        self.0.denom().is_one()
    }

    pub(crate) fn denominator_value(&self, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        work.validate(self)?;
        Ok(Self(BigRational::from_integer(self.0.denom().clone())))
    }
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }
    #[must_use]
    pub fn is_probability(&self) -> bool {
        !self.is_negative() && self.0.numer() <= self.0.denom()
    }
    #[must_use]
    pub fn bits(&self) -> u64 {
        self.0.numer().bits().max(self.0.denom().bits())
    }

    /// Parse signed base-ten integers and normalize their ratio. Denominator
    /// zero refuses. Input extents are checked before bigint allocation.
    /// # Errors
    /// Malformed digits, zero denominator, resource limits or cancellation.
    pub fn fraction(
        numerator: &str,
        denominator: &str,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let numerator = integer(numerator, work)?;
        let denominator = integer(denominator, work)?;
        Self::normalize(numerator, denominator, work)
    }

    /// Exact finite decimal, optionally with an `e` exponent; no binary64
    /// intermediate. Exponents and digit extents obey the supplied bit budget.
    /// # Errors
    /// Invalid syntax, excessive precision/exponent or cancellation.
    pub fn decimal(text: &str, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        work.step(decimal_bits(text.len()))?;
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((mantissa, exponent)) => (
                mantissa,
                exponent
                    .parse::<i64>()
                    .map_err(|_| Error::InvalidRational)?,
            ),
            None => (text, 0),
        };
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if whole.is_empty()
            || matches!(whole, "+" | "-")
            || (mantissa.contains('.') && fraction.is_empty())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::InvalidRational);
        }
        let extent = whole
            .len()
            .checked_add(fraction.len())
            .ok_or(Error::Capacity(Capacity::ArithmeticBits))?;
        work.step(decimal_bits(extent))?;
        let digits = format!("{whole}{fraction}");
        let numerator = integer(&digits, work)?;
        let scale = i64::try_from(fraction.len())
            .ok()
            .and_then(|length| length.checked_sub(exponent))
            .ok_or(Error::Capacity(Capacity::ArithmeticBits))?;
        let power = u32::try_from(scale.unsigned_abs())
            .map_err(|_| Error::Capacity(Capacity::ArithmeticBits))?;
        let bound = numerator
            .bits()
            .saturating_add(u64::from(power).saturating_mul(4));
        work.step(bound)?;
        let factor = BigInt::from(10u8).pow(power);
        if scale >= 0 {
            Self::normalize(numerator, factor, work)
        } else {
            Self::normalize(numerator * factor, BigInt::one(), work)
        }
    }

    /// Preserve the exact value of a finite IEEE binary64 input. Decimal text
    /// must instead use `decimal` if its decimal value is the authored meaning.
    /// # Errors
    /// Nonfinite input, bit/work limits or cancellation.
    pub fn binary64(value: f64, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        if !value.is_finite() {
            return Err(Error::InvalidRational);
        }
        // Charge the exact reduced extent before allocating. Removing the
        // significand's trailing zeros also handles integral and subnormal values.
        let raw = value.to_bits();
        let exponent = ((raw >> 52) & 0x7ff) as i32;
        let significand = (raw & ((1u64 << 52) - 1)) | (u64::from(exponent != 0) << 52);
        let bits = if significand == 0 {
            1
        } else {
            let shift = if exponent == 0 {
                -1074
            } else {
                exponent - 1075
            };
            let zeros = significand.trailing_zeros();
            let shift = shift + zeros.cast_signed();
            let numerator = u64::from(64 - (significand >> zeros).leading_zeros());
            if shift >= 0 {
                numerator + u64::from(shift.unsigned_abs())
            } else {
                numerator.max(u64::from(shift.unsigned_abs()) + 1)
            }
        };
        work.step(bits)?;
        let result = Self(BigRational::from_float(value).ok_or(Error::InvalidRational)?);
        work.finish()?;
        Ok(result)
    }

    fn normalize(
        numerator: BigInt,
        denominator: BigInt,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        work.step(numerator.bits().max(denominator.bits()))?;
        if denominator.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let value = Self(BigRational::new(numerator, denominator));
        work.finish()?;
        Ok(value)
    }

    /// # Errors
    /// Explicit conservative intermediate-bit/work limits or cancellation.
    pub fn add(&self, other: &Self, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        self.binary(other, ArithmeticOp::Add, work)
    }
    /// # Errors
    /// As `add`.
    pub fn sub(&self, other: &Self, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        self.binary(other, ArithmeticOp::Subtract, work)
    }
    /// # Errors
    /// As `add`.
    pub fn mul(&self, other: &Self, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        self.binary(other, ArithmeticOp::Multiply, work)
    }
    /// # Errors
    /// Zero divisor, intermediate-bit/work limits or cancellation.
    pub fn div(&self, other: &Self, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        self.binary(other, ArithmeticOp::Divide, work)
    }

    /// Natural power by repeated squaring. The zero power is one, including
    /// for a zero base; the supplied base is still checked against the budget.
    /// # Errors
    /// Arithmetic resource limits or cancellation.
    pub fn pow(&self, mut exponent: u32, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        work.validate(self)?;
        let mut result = Self::one();
        let mut base = self.clone();
        while exponent != 0 {
            if exponent & 1 != 0 {
                result = result.mul(&base, work)?;
            }
            exponent >>= 1;
            if exponent != 0 {
                base = base.mul(&base, work)?;
            }
        }
        work.finish()?;
        Ok(result)
    }

    fn binary(
        &self,
        other: &Self,
        op: ArithmeticOp,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let [a, b, c, d] = [
            self.0.numer().bits(),
            self.0.denom().bits(),
            other.0.numer().bits(),
            other.0.denom().bits(),
        ];
        let bound = match op {
            ArithmeticOp::Add | ArithmeticOp::Subtract => a
                .saturating_add(d)
                .max(c.saturating_add(b))
                .saturating_add(1)
                .max(b.saturating_add(d)),
            ArithmeticOp::Multiply => a.saturating_add(c).max(b.saturating_add(d)),
            ArithmeticOp::Divide => a.saturating_add(d).max(b.saturating_add(c)),
        }
        .max(self.bits())
        .max(other.bits());
        work.step(bound)?;
        if matches!(op, ArithmeticOp::Divide) && other.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let value = match op {
            ArithmeticOp::Add => &self.0 + &other.0,
            ArithmeticOp::Subtract => &self.0 - &other.0,
            ArithmeticOp::Multiply => &self.0 * &other.0,
            ArithmeticOp::Divide => &self.0 / &other.0,
        };
        work.finish()?;
        Ok(Self(value))
    }

    /// Canonical BERA v1: header, sign, unsigned numerator and positive denominator
    /// as minimal big-endian integers with little-endian u64 byte lengths.
    /// # Errors
    /// Arithmetic extent/work limits, allocation or cancellation.
    pub fn to_bytes(&self, work: &mut ExactArithmetic<'_>) -> Result<Vec<u8>> {
        work.step(self.bits())?;
        let (_, mut numerator) = self.0.numer().to_bytes_be();
        if self.is_zero() {
            numerator.clear();
        }
        let (_, denominator) = self.0.denom().to_bytes_be();
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(22 + numerator.len() + denominator.len())?;
        bytes.extend_from_slice(b"BERA\x01");
        bytes.push(u8::from(self.is_negative()));
        for part in [&numerator, &denominator] {
            bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
            bytes.extend_from_slice(part);
        }
        work.finish()?;
        Ok(bytes)
    }

    /// Admit only canonical BERA bytes. No unreduced denominator, leading zero,
    /// negative zero, unknown version, trailing bytes or ambiguous encoding.
    /// # Errors
    /// Malformed/noncanonical values, extent/work limits or cancellation.
    pub fn from_bytes(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<Self> {
        if bytes.get(..5) != Some(b"BERA\x01") || bytes.get(5).is_none_or(|sign| *sign > 1) {
            return Err(Error::InvalidRational);
        }
        let mut offset = 6;
        let numerator = blob(bytes, &mut offset, work)?;
        let denominator = blob(bytes, &mut offset, work)?;
        if offset != bytes.len()
            || numerator.first() == Some(&0)
            || denominator.first().is_none_or(|b| *b == 0)
            || (numerator.is_empty() && bytes[5] != 0)
        {
            return Err(Error::InvalidRational);
        }
        let sign = if bytes[5] == 1 {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let value = Self::normalize(
            BigInt::from_bytes_be(sign, numerator),
            BigInt::from_bytes_be(Sign::Plus, denominator),
            work,
        )?;
        if value.to_bytes(work)? != bytes {
            return Err(Error::InvalidRational);
        }
        Ok(value)
    }
}

#[derive(Clone, Copy)]
enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

fn decimal_bits(digits: usize) -> u64 {
    (digits as u64).saturating_mul(4).max(1)
}

fn integer(text: &str, work: &mut ExactArithmetic<'_>) -> Result<BigInt> {
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
    work.step(decimal_bits(digits.len()))?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidRational);
    }
    BigInt::parse_bytes(text.as_bytes(), 10).ok_or(Error::InvalidRational)
}

fn blob<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    work: &mut ExactArithmetic<'_>,
) -> Result<&'a [u8]> {
    let raw = bytes
        .get(*offset..offset.saturating_add(8))
        .ok_or(Error::InvalidRational)?;
    let length = usize::try_from(u64::from_le_bytes(
        raw.try_into().map_err(|_| Error::InvalidRational)?,
    ))
    .map_err(|_| Error::Capacity(Capacity::ArithmeticBits))?;
    *offset += 8;
    let end = offset.checked_add(length).ok_or(Error::InvalidRational)?;
    let part = bytes.get(*offset..end).ok_or(Error::InvalidRational)?;
    let bits = part.first().map_or(0, |first| {
        (length as u64).saturating_mul(8) - u64::from(first.leading_zeros())
    });
    work.step(bits)?;
    *offset = end;
    Ok(part)
}
