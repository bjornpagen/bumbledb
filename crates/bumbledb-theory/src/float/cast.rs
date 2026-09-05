//! Integer-only, explicitly named numeric conversions. These do not depend on
//! a process's floating-point rounding mode or flush-to-zero controls.

use super::F64;

/// A refused explicit numeric cast; no saturating or truncating cast is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F64CastError {
    NonFinite,
    Fractional,
    OutOfRange,
    Inexact,
}

impl core::fmt::Display for F64CastError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "a nonfinite F64 has no exact integer value",
            Self::Fractional => "the F64 value is not an integer",
            Self::OutOfRange => "the integer is outside the target domain",
            Self::Inexact => "the integer cannot be represented exactly as F64",
        })
    }
}

impl std::error::Error for F64CastError {}

impl F64 {
    /// Correctly rounded conversion, nearest with ties to even. Large integers
    /// may lose precision; use [`Self::from_u64_exact`] to refuse that loss.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        let exponent = value.ilog2();
        let (mut mantissa, mut exponent) = if exponent <= 52 {
            (value << (52 - exponent), exponent)
        } else {
            let shift = exponent - 52;
            let upper = value >> shift;
            let lower = value & ((1 << shift) - 1);
            let half = 1 << (shift - 1);
            let increment = lower > half || (lower == half && upper & 1 != 0);
            (upper + u64::from(increment), exponent)
        };
        if mantissa == 1 << 53 {
            mantissa >>= 1;
            exponent += 1;
        }
        Self::from_bits((u64::from(exponent + 1023) << 52) | (mantissa & super::FRACTION))
    }

    /// Correctly rounded signed conversion, nearest with ties to even.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        let magnitude = Self::from_u64(value.unsigned_abs());
        if value < 0 {
            magnitude.negated()
        } else {
            magnitude
        }
    }

    /// # Errors
    /// [`F64CastError::Inexact`] if binary64 would lose integer information.
    pub fn from_u64_exact(value: u64) -> Result<Self, F64CastError> {
        let result = Self::from_u64(value);
        if result.to_u64_exact() == Ok(value) {
            Ok(result)
        } else {
            Err(F64CastError::Inexact)
        }
    }

    /// # Errors
    /// [`F64CastError::Inexact`] if binary64 would lose integer information.
    pub fn from_i64_exact(value: i64) -> Result<Self, F64CastError> {
        let result = Self::from_i64(value);
        if result.to_i64_exact() == Ok(value) {
            Ok(result)
        } else {
            Err(F64CastError::Inexact)
        }
    }

    /// # Errors
    /// Refuses nonfinite, fractional, negative, or out-of-range values.
    pub fn to_u64_exact(self) -> Result<u64, F64CastError> {
        let magnitude = self.integer_magnitude()?;
        if self.to_bits() & super::SIGN != 0 {
            Err(F64CastError::OutOfRange)
        } else {
            Ok(magnitude)
        }
    }

    /// # Errors
    /// Refuses nonfinite, fractional, or out-of-range values.
    pub fn to_i64_exact(self) -> Result<i64, F64CastError> {
        let magnitude = self.integer_magnitude()?;
        if self.to_bits() & super::SIGN == 0 {
            i64::try_from(magnitude).map_err(|_| F64CastError::OutOfRange)
        } else if magnitude == 1 << 63 {
            Ok(i64::MIN)
        } else {
            i64::try_from(magnitude)
                .map(|v| -v)
                .map_err(|_| F64CastError::OutOfRange)
        }
    }

    /// Negation in the quotient domain. In particular negating zero produces
    /// positive zero, and negating NaN produces the same canonical NaN.
    #[must_use]
    pub const fn negated(self) -> Self {
        Self::from_bits(self.to_bits() ^ super::SIGN)
    }

    fn integer_magnitude(self) -> Result<u64, F64CastError> {
        if !self.is_finite() {
            return Err(F64CastError::NonFinite);
        }
        let bits = self.to_bits();
        if bits == 0 {
            return Ok(0);
        }
        let biased = (bits & super::EXPONENT) >> 52;
        if biased < 1023 {
            return Err(F64CastError::Fractional);
        }
        if biased > 1086 {
            return Err(F64CastError::OutOfRange);
        }
        let exponent = biased - 1023;
        let mantissa = (bits & super::FRACTION) | (1 << 52);
        if exponent >= 52 {
            Ok(mantissa << (exponent - 52))
        } else {
            let shift = 52 - exponent;
            if mantissa & ((1 << shift) - 1) != 0 {
                return Err(F64CastError::Fractional);
            }
            Ok(mantissa >> shift)
        }
    }
}
