//! Canonical binary64 value identity. Numerical execution lives in the engine.

use core::cmp::Ordering;
use core::fmt;

mod cast;
pub(crate) mod exact;
pub use cast::F64CastError;

pub(crate) const SIGN: u64 = 1 << 63;
pub(crate) const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
pub(crate) const FRACTION: u64 = 0x000f_ffff_ffff_ffff;
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
    /// The largest finite value, `(2 - 2⁻⁵²) × 2¹⁰²³`.
    pub const MAX_FINITE: Self = Self(0x7fef_ffff_ffff_ffff);
    /// The most negative finite value.
    pub const MIN_FINITE: Self = Self(0xffef_ffff_ffff_ffff);
    /// The smallest positive value: the minimum subnormal, `2⁻¹⁰⁷⁴`.
    pub const MIN_POSITIVE_SUBNORMAL: Self = Self(1);
    /// The smallest positive normal value, `2⁻¹⁰²²`.
    pub const MIN_POSITIVE_NORMAL: Self = Self(0x0010_0000_0000_0000);

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

#[cfg(test)]
mod tests {
    use super::{F64, F64ParseError};
    use crate::F64CastError;

    /// F-GOLDEN: the checked-in boundary-class bit fixtures.
    const GOLDEN_CANONICAL: &[u64] = &[
        0x0000_0000_0000_0000, // +0 (the only zero)
        0x0000_0000_0000_0001, // smallest subnormal
        0x000f_ffff_ffff_ffff, // largest subnormal
        0x0010_0000_0000_0000, // smallest normal
        0x3ff0_0000_0000_0000, // 1.0
        0x3ff0_0000_0000_0001, // nextUp(1.0)
        0xbff0_0000_0000_0000, // -1.0
        0x4340_0000_0000_0000, // 2^53 (exact integer boundary)
        0x4340_0000_0000_0001, // first inexact integer neighborhood
        0x7fef_ffff_ffff_ffff, // MAX_FINITE
        0xffef_ffff_ffff_ffff, // MIN_FINITE
        0x7ff0_0000_0000_0000, // +Infinity
        0xfff0_0000_0000_0000, // -Infinity
        0x7ff8_0000_0000_0000, // the canonical quiet NaN
    ];

    /// Noncanonical encodings and the class each must normalize to.
    const GOLDEN_NONCANONICAL: &[(u64, u64)] = &[
        (0x8000_0000_0000_0000, 0),                     // -0 → +0
        (0x7ff0_0000_0000_0001, 0x7ff8_0000_0000_0000), // signaling NaN
        (0xfff0_0000_0000_0001, 0x7ff8_0000_0000_0000), // negative signaling NaN
        (0xfff8_0000_0000_0000, 0x7ff8_0000_0000_0000), // negative quiet NaN
        (0x7fff_ffff_ffff_ffff, 0x7ff8_0000_0000_0000), // max payload NaN
        (0x7ff8_0000_0000_0001, 0x7ff8_0000_0000_0000), // payload-bearing quiet NaN
    ];

    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// F-CANON: canonicalization is idempotent over golden classes and
    /// random 64-bit patterns, and every NaN class and both zeros collapse.
    #[test]
    fn canonicalization_is_idempotent_and_total() {
        for &bits in GOLDEN_CANONICAL {
            assert_eq!(F64::from_bits(bits).to_bits(), bits, "{bits:#018x}");
            assert_eq!(F64::from_canonical_bits(bits), Ok(F64::from_bits(bits)));
        }
        for &(bad, good) in GOLDEN_NONCANONICAL {
            assert_eq!(F64::from_bits(bad).to_bits(), good, "{bad:#018x}");
            assert_eq!(
                F64::from_canonical_bits(bad),
                Err(F64ParseError::NonCanonicalBits { bits: bad }),
                "{bad:#018x} must refuse as wire input"
            );
        }
        let mut state = 7;
        for _ in 0..65_536 {
            let bits = splitmix(&mut state);
            let once = F64::from_bits(bits);
            assert_eq!(F64::from_bits(once.to_bits()), once, "idempotent");
            // The canonical image always re-parses as canonical.
            assert_eq!(F64::from_canonical_bits(once.to_bits()), Ok(once));
        }
    }

    #[test]
    fn nan_equals_nan_and_zeros_collapse_in_the_database_domain() {
        assert_eq!(F64::from(f64::NAN), F64::NAN);
        assert_eq!(F64::from(-f64::NAN), F64::NAN);
        assert_eq!(F64::from(0.0), F64::from(-0.0));
        assert_eq!(F64::from(-0.0), F64::ZERO);
    }

    /// F-ORDER: the total order is antisymmetric/transitive/consistent with
    /// equality, byte order equals logical order, and the placement is
    /// -Infinity < negative finite < 0 < positive finite < +Infinity < NaN.
    #[test]
    fn total_order_agrees_with_order_bytes_and_the_selected_placement() {
        let ladder = [
            F64::NEG_INFINITY,
            F64::MIN_FINITE,
            F64::from(-1.0),
            F64::from(-f64::MIN_POSITIVE),
            F64::from_bits(0x8000_0000_0000_0001), // -min subnormal
            F64::ZERO,
            F64::MIN_POSITIVE_SUBNORMAL,
            F64::MIN_POSITIVE_NORMAL,
            F64::from(1.0),
            F64::MAX_FINITE,
            F64::INFINITY,
            F64::NAN,
        ];
        for (i, a) in ladder.iter().enumerate() {
            for (j, b) in ladder.iter().enumerate() {
                assert_eq!(a.cmp(b), i.cmp(&j), "ladder {i} vs {j}");
                assert_eq!(
                    a.to_order_bytes().cmp(&b.to_order_bytes()),
                    i.cmp(&j),
                    "byte order equals logical order at {i} vs {j}"
                );
            }
        }
        assert!(F64::NAN > F64::INFINITY, "NaN sorts last, deliberately");
        let mut state = 11;
        for _ in 0..16_384 {
            let a = F64::from_bits(splitmix(&mut state));
            let b = F64::from_bits(splitmix(&mut state));
            assert_eq!(a.cmp(&b), a.to_order_key().cmp(&b.to_order_key()));
            assert_eq!(a == b, a.cmp(&b).is_eq(), "order consistent with Eq");
        }
    }

    #[test]
    fn order_key_and_payload_byte_codecs_roundtrip_and_reject_holes() {
        let mut state = 13;
        for _ in 0..16_384 {
            let value = F64::from_bits(splitmix(&mut state));
            assert_eq!(F64::from_order_key(value.to_order_key()), Ok(value));
            assert_eq!(F64::from_order_bytes(value.to_order_bytes()), Ok(value));
            assert_eq!(F64::from_canonical_be_bytes(value.to_be_bytes()), Ok(value));
        }
        // The order key of -0 (a hole in the canonical image) refuses.
        let neg_zero_key = 0x8000_0000_0000_0000_u64 ^ u64::MAX;
        assert!(F64::from_order_key(neg_zero_key).is_err());
        // Payload decoding refuses length and canonicality separately.
        assert_eq!(
            F64::try_from(&[0u8; 7][..]),
            Err(F64ParseError::InvalidLength { actual: 7 })
        );
        assert!(F64::from_canonical_be_bytes(0x8000_0000_0000_0000_u64.to_be_bytes()).is_err());
    }

    /// Explicit cast boundaries near 2^53 and the integer limits (C01).
    #[test]
    fn cast_boundaries_are_exact_or_refuse() {
        // 2^53 is exact; 2^53 + 1 is not.
        assert_eq!(
            F64::from_u64_exact(1u64 << 53),
            Ok(F64::from(9_007_199_254_740_992.0))
        );
        assert_eq!(
            F64::from_u64_exact((1u64 << 53) + 1),
            Err(F64CastError::Inexact)
        );
        // Correctly rounded lossy conversion still succeeds and ties to even.
        assert_eq!(F64::from_u64((1u64 << 53) + 1), F64::from_u64(1u64 << 53));
        // u64::MAX rounds to 2^64; the exact cast refuses.
        assert_eq!(
            F64::from_u64(u64::MAX),
            F64::from(18_446_744_073_709_551_616.0)
        );
        assert_eq!(F64::from_u64_exact(u64::MAX), Err(F64CastError::Inexact));
        // i64::MIN is a power of two: exact both ways.
        assert_eq!(
            F64::from_i64_exact(i64::MIN).and_then(F64::to_i64_exact),
            Ok(i64::MIN)
        );
        assert_eq!(F64::from_i64_exact(i64::MAX), Err(F64CastError::Inexact));
        // Back-casts refuse each failure class distinctly.
        assert_eq!(F64::NAN.to_i64_exact(), Err(F64CastError::NonFinite));
        assert_eq!(F64::INFINITY.to_u64_exact(), Err(F64CastError::NonFinite));
        assert_eq!(F64::from(0.5).to_i64_exact(), Err(F64CastError::Fractional));
        assert_eq!(
            F64::from(-1.0).to_u64_exact(),
            Err(F64CastError::OutOfRange)
        );
        assert_eq!(
            F64::from(9.3e18).to_i64_exact(),
            Err(F64CastError::OutOfRange)
        );
        // Zero is zero in every direction.
        assert_eq!(F64::ZERO.to_u64_exact(), Ok(0));
        assert_eq!(F64::from(-0.0).to_i64_exact(), Ok(0));
        // Negation stays in the quotient: -0 → +0, -NaN → NaN.
        assert_eq!(F64::ZERO.negated(), F64::ZERO);
        assert_eq!(F64::NAN.negated(), F64::NAN);
        assert_eq!(F64::from(1.5).negated(), F64::from(-1.5));
    }

    /// The in-crate independent cast oracle (P11 review concern, confirmed):
    /// integer-only round-to-nearest-ties-to-even of an exact integer to a
    /// binary64 payload, structured as the bench `verify::f64_oracle`
    /// technique (twice-the-remainder dyadic comparison over the exact
    /// value, carry into the next binade), sharing no code with the
    /// production casts and executing no host float instruction. The bench
    /// oracle remains the third implementation; these bits let the cast
    /// differentials fail even if production and the host shared one bug.
    mod cast_oracle {
        /// The nearest binary64 payload for an exact u64 integer.
        pub(super) fn from_u64_bits(value: u64) -> u64 {
            if value == 0 {
                return 0;
            }
            let width = 64 - value.leading_zeros(); // 1..=64
            if width <= 53 {
                // Exactly representable: unbiased exponent width−1, the
                // leading bit normalized to position 52 and then hidden.
                let mantissa = value << (53 - width);
                return (u64::from(1022 + width) << 52) | (mantissa & ((1u64 << 52) - 1));
            }
            let step = width - 53;
            let kept = value >> step;
            let remainder = value & ((1u64 << step) - 1);
            // Round the exact dyadic value kept + remainder/2^step to the
            // nearest integer, ties to even: compare 2·remainder with 2^step.
            let twice = u128::from(remainder) << 1;
            let denominator = 1u128 << step;
            let up = match twice.cmp(&denominator) {
                core::cmp::Ordering::Greater => true,
                core::cmp::Ordering::Equal => kept & 1 == 1,
                core::cmp::Ordering::Less => false,
            };
            let kept = kept + u64::from(up);
            if kept == 1 << 53 {
                encode53(1 << 52, step + 1)
            } else {
                encode53(kept, step)
            }
        }

        /// Encode `mantissa × 2^step` with `2^52 ≤ mantissa < 2^53` —
        /// biased exponent `1023 + 52 + step`, always finite here.
        fn encode53(mantissa: u64, step: u32) -> u64 {
            debug_assert!((1u64 << 52..1u64 << 53).contains(&mantissa));
            (u64::from(1023 + 52 + step) << 52) | (mantissa & ((1u64 << 52) - 1))
        }

        pub(super) fn from_i64_bits(value: i64) -> u64 {
            let magnitude = from_u64_bits(value.unsigned_abs());
            if value < 0 && magnitude != 0 {
                magnitude | (1u64 << 63)
            } else {
                magnitude
            }
        }

        /// Is the integer exactly representable in binary64? (No discarded
        /// remainder below the 53-bit significand window.)
        pub(super) fn u64_is_exact(value: u64) -> bool {
            if value == 0 {
                return true;
            }
            let width = 64 - value.leading_zeros();
            width <= 53 || value & ((1u64 << (width - 53)) - 1) == 0
        }

        pub(super) fn i64_is_exact(value: i64) -> bool {
            u64_is_exact(value.unsigned_abs())
        }
    }

    /// F-GOLDEN (cast half): pinned, hand-computed conversion bits at the
    /// precision boundaries — independent constants that hold even if the
    /// production cast, the host conversion and the oracle all changed.
    #[test]
    fn integer_cast_golden_bits_pin_the_boundaries() {
        // (input, expected canonical bits)
        let unsigned_goldens: &[(u64, u64)] = &[
            (0, 0x0000_0000_0000_0000),
            (1, 0x3ff0_0000_0000_0000),
            ((1 << 53) - 1, 0x433f_ffff_ffff_ffff), // largest exact odd
            (1 << 53, 0x4340_0000_0000_0000),       // 2^53
            ((1 << 53) + 1, 0x4340_0000_0000_0000), // tie → even (down)
            ((1 << 53) + 2, 0x4340_0000_0000_0001), // exact
            ((1 << 53) + 3, 0x4340_0000_0000_0002), // tie → even (up)
            (u64::MAX, 0x43f0_0000_0000_0000),      // rounds to 2^64
            (u64::MAX - 1024, 0x43ef_ffff_ffff_ffff), // last below 2^64
            (1 << 63, 0x43e0_0000_0000_0000),       // 2^63
        ];
        for &(input, expected) in unsigned_goldens {
            assert_eq!(F64::from_u64(input).to_bits(), expected, "prod {input}");
            assert_eq!(
                cast_oracle::from_u64_bits(input),
                expected,
                "oracle {input}"
            );
            #[expect(clippy::cast_precision_loss, reason = "the host is a subject here")]
            let host = input as f64;
            assert_eq!(host.to_bits(), expected, "host {input}");
        }
        let signed_goldens: &[(i64, u64)] = &[
            (i64::MIN, 0xc3e0_0000_0000_0000), // -2^63, a power of two
            (-1, 0xbff0_0000_0000_0000),
            (-((1 << 53) + 1), 0xc340_0000_0000_0000), // tie → even
            (i64::MAX, 0x43e0_0000_0000_0000),         // rounds to 2^63
        ];
        for &(input, expected) in signed_goldens {
            assert_eq!(F64::from_i64(input).to_bits(), expected, "prod {input}");
            assert_eq!(
                cast_oracle::from_i64_bits(input),
                expected,
                "oracle {input}"
            );
            #[expect(clippy::cast_precision_loss, reason = "the host is a subject here")]
            let host = input as f64;
            assert_eq!(host.to_bits(), expected, "host {input}");
        }
    }

    /// Three-way differential over random u64/i64 values: production,
    /// the host conversion (the hardware subject) and the independent
    /// integer oracle must all agree bit for bit — a shared host
    /// assumption can no longer mask a production bug.
    #[test]
    fn integer_conversion_differential_against_host_and_oracle() {
        let mut state = 17;
        for _ in 0..16_384 {
            let raw = splitmix(&mut state);
            #[expect(clippy::cast_precision_loss, reason = "the host is a subject")]
            let host_u = F64::from(raw as f64);
            let oracle_u = F64::from_bits(cast_oracle::from_u64_bits(raw));
            assert_eq!(F64::from_u64(raw), host_u, "u64 host {raw}");
            assert_eq!(F64::from_u64(raw), oracle_u, "u64 oracle {raw}");
            let signed = raw.cast_signed();
            #[expect(clippy::cast_precision_loss, reason = "the host is a subject")]
            let host_i = F64::from(signed as f64);
            let oracle_i = F64::from_bits(cast_oracle::from_i64_bits(signed));
            assert_eq!(F64::from_i64(signed), host_i, "i64 host {signed}");
            assert_eq!(F64::from_i64(signed), oracle_i, "i64 oracle {signed}");
        }
    }

    /// Structured boundary sweep with the oracle as the expectation:
    /// around every binade edge and tie pattern from 2^50 to the top,
    /// production equals the oracle, and the EXACT casts succeed exactly
    /// when the oracle's discarded remainder is zero (an independent
    /// characterization of `from_*_exact`, not a host round-trip).
    #[test]
    fn cast_boundary_sweep_and_exactness_against_the_oracle() {
        let mut probes: Vec<u64> = Vec::new();
        for width in 50..=63u32 {
            let base = 1u64 << width;
            for offset in [0u64, 1, 2, 3] {
                probes.push(base - 1 + offset);
            }
            if width >= 53 {
                // Representable spacing in the binade [2^width, 2^(width+1)).
                let step = 1u64 << (width - 52);
                // The half step (an exact tie), just below and just above.
                probes.push(base + step / 2 - 1);
                probes.push(base + step / 2);
                probes.push(base + step / 2 + 1);
                // The tie onto an odd kept significand.
                probes.push(base + step + step / 2);
            }
        }
        probes.push(u64::MAX);
        for &value in &probes {
            let expected = cast_oracle::from_u64_bits(value);
            assert_eq!(F64::from_u64(value).to_bits(), expected, "u64 {value:#x}");
            assert_eq!(
                F64::from_u64_exact(value).is_ok(),
                cast_oracle::u64_is_exact(value),
                "u64 exactness {value:#x}"
            );
            if cast_oracle::u64_is_exact(value) {
                assert_eq!(
                    F64::from_u64_exact(value).and_then(F64::to_u64_exact),
                    Ok(value)
                );
            }
            let signed = value.cast_signed();
            assert_eq!(
                F64::from_i64(signed).to_bits(),
                cast_oracle::from_i64_bits(signed),
                "i64 {signed}"
            );
            assert_eq!(
                F64::from_i64_exact(signed).is_ok(),
                cast_oracle::i64_is_exact(signed),
                "i64 exactness {signed}"
            );
        }
        // Random exactness agreement, both signs.
        let mut state = 23;
        for _ in 0..16_384 {
            let raw = splitmix(&mut state);
            assert_eq!(
                F64::from_u64_exact(raw).is_ok(),
                cast_oracle::u64_is_exact(raw),
                "u64 {raw}"
            );
            let signed = raw.cast_signed();
            assert_eq!(
                F64::from_i64_exact(signed).is_ok(),
                cast_oracle::i64_is_exact(signed),
                "i64 {signed}"
            );
        }
    }
}
