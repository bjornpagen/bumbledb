//! Canonical binary64 value identity. Numerical execution lives in the engine.

use core::cmp::Ordering;
use core::fmt;

mod cast;
pub use cast::F64CastError;

const SIGN: u64 = 1 << 63;
const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
const FRACTION: u64 = 0x000f_ffff_ffff_ffff;
const NAN: u64 = 0x7ff8_0000_0000_0000;

/// A binary64 value with one NaN and one zero representation.
///
/// All NaN signs/payloads become `0x7ff8000000000000`; both zeros become
/// positive zero. Equality and hashing use these canonical bits. The total
/// database order is negative infinity, finite numbers, positive infinity,
/// then NaN. In particular, NaN equals itself.
///
/// Host construction normalizes; canonical byte/bit decoding rejects other
/// representations. This type deliberately offers no arithmetic operators:
/// constructing a value cannot establish how the host calculated it.
///
/// ```compile_fail
/// use bumbledb_theory::F64;
/// let invalid = F64(0x8000_0000_0000_0000); // The payload is private.
/// ```
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct F64(u64);

impl F64 {
    /// The only zero value.
    pub const ZERO: Self = Self(0);
    /// The only NaN value.
    pub const NAN: Self = Self(NAN);
    /// Positive infinity.
    pub const INFINITY: Self = Self(EXPONENT);
    /// Negative infinity.
    pub const NEG_INFINITY: Self = Self(SIGN | EXPONENT);

    /// Normalize a host binary64 bit image without executing floating arithmetic.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        if bits & EXPONENT == EXPONENT && bits & FRACTION != 0 {
            Self::NAN
        } else if bits & !SIGN == 0 {
            Self::ZERO
        } else {
            Self(bits)
        }
    }

    /// Decode an already canonical binary64 bit image.
    ///
    /// # Errors
    /// Returns [`F64ParseError::NonCanonicalBits`] for negative zero or an
    /// alternative NaN encoding. Use [`Self::from_bits`] for host normalization.
    pub const fn from_canonical_bits(bits: u64) -> Result<Self, F64ParseError> {
        if Self::from_bits(bits).0 == bits {
            Ok(Self(bits))
        } else {
            Err(F64ParseError::NonCanonicalBits { bits })
        }
    }

    /// Return the canonical binary64 payload bits, not the index order key.
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// Return the canonical host float. This conversion performs no arithmetic.
    #[must_use]
    pub const fn to_f64(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// Encode the canonical payload as exactly eight big-endian bytes.
    ///
    /// Payload bytes are not lexicographically ordered numerical index keys.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    /// Decode exactly eight canonical big-endian payload bytes.
    ///
    /// # Errors
    /// Returns [`F64ParseError::NonCanonicalBits`] for noncanonical payloads.
    pub const fn from_canonical_be_bytes(bytes: [u8; 8]) -> Result<Self, F64ParseError> {
        Self::from_canonical_bits(u64::from_be_bytes(bytes))
    }

    /// Return an unsigned key whose order is the database's total value order.
    #[must_use]
    pub const fn to_order_key(self) -> u64 {
        if self.0 & SIGN == 0 {
            self.0 ^ SIGN
        } else {
            !self.0
        }
    }

    /// Decode an unsigned total-order key, refusing noncanonical holes.
    ///
    /// # Errors
    /// Returns [`F64ParseError::NonCanonicalBits`] if the inverse mapping
    /// contains negative zero or an alternative NaN encoding.
    pub const fn from_order_key(key: u64) -> Result<Self, F64ParseError> {
        let bits = if key & SIGN == 0 { !key } else { key ^ SIGN };
        Self::from_canonical_bits(bits)
    }

    /// Return lexicographically ordered bytes for an index, not payload bytes.
    #[must_use]
    pub const fn to_order_bytes(self) -> [u8; 8] {
        self.to_order_key().to_be_bytes()
    }

    /// Decode exactly eight big-endian index-order bytes.
    ///
    /// # Errors
    /// Returns [`F64ParseError::NonCanonicalBits`] for a noncanonical order key.
    pub const fn from_order_bytes(bytes: [u8; 8]) -> Result<Self, F64ParseError> {
        Self::from_order_key(u64::from_be_bytes(bytes))
    }

    /// Whether this is the single canonical NaN.
    #[must_use]
    pub const fn is_nan(self) -> bool {
        self.0 == NAN
    }

    /// Whether this is finite, including zero and subnormal values.
    #[must_use]
    pub const fn is_finite(self) -> bool {
        self.0 & EXPONENT != EXPONENT
    }

    /// Whether this is either infinity, excluding NaN.
    #[must_use]
    pub const fn is_infinite(self) -> bool {
        self.0 & !SIGN == EXPONENT
    }
}

impl From<f64> for F64 {
    fn from(value: f64) -> Self {
        Self::from_bits(value.to_bits())
    }
}

impl From<F64> for f64 {
    fn from(value: F64) -> Self {
        value.to_f64()
    }
}

impl TryFrom<&[u8]> for F64 {
    type Error = F64ParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let payload = bytes.try_into().map_err(|_| F64ParseError::InvalidLength {
            actual: bytes.len(),
        })?;
        Self::from_canonical_be_bytes(payload)
    }
}

impl Ord for F64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_order_key().cmp(&other.to_order_key())
    }
}

impl PartialOrd for F64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for F64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F64(0x{:016x})", self.0)
    }
}

/// Failure to decode a canonical binary64 payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F64ParseError {
    /// A binary64 payload must contain exactly eight bytes.
    InvalidLength { actual: usize },
    /// The payload is negative zero or a noncanonical NaN encoding.
    NonCanonicalBits { bits: u64 },
}

impl fmt::Display for F64ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual } => {
                write!(f, "F64 requires 8 payload bytes, got {actual}")
            }
            Self::NonCanonicalBits { .. } => f.write_str("noncanonical F64 payload"),
        }
    }
}

impl std::error::Error for F64ParseError {}
