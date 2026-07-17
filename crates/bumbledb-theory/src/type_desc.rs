//! Encoding-level type descriptions and the [`ValueType`] lowering.
//!
//! [`TypeDesc`] is pure layout vocabulary — the widths and derivations a
//! type's canonical bytes obey — with no reach into the fact codec: the
//! encoders, decoders, and corruption checks that consume it are engine
//! machinery (`bumbledb::encoding`). It lives theory-side because the
//! lowering `ValueType::type_desc` is an inherent judgment of the
//! structural type (a type IS its encoding —
//! `docs/architecture/10-data-model.md`).

use crate::schema::{IntervalElement, ValueType};

/// Encoding-level description of a field's type: exactly what is needed to
/// size, encode, and corruption-check its bytes. No names anywhere — a type
/// is an encoding and nothing else (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeDesc {
    /// 1 byte, strictly `0x00` or `0x01`.
    Bool,
    /// 8 bytes, big-endian (order-preserving).
    U64,
    /// 8 bytes, sign-flipped big-endian (order-preserving).
    I64,
    /// 8 bytes in facts: the interned dictionary id, big-endian.
    String,
    /// `⌈len/8⌉ × 8` bytes in facts: the `len` raw bytes themselves,
    /// zero-padded to the word boundary — the pad is encoding, not data
    /// (a nonzero trailing pad byte is corruption). Identity = bytes; no
    /// dictionary indirection ever (`docs/architecture/10-data-model.md`).
    FixedBytes {
        /// Declared width in bytes, `1..=MAX_FIXED_BYTES` (the engine's
        /// `encoding::MAX_FIXED_BYTES`, 64).
        len: u16,
    },
    /// The interval family. General (`width: None`): 16 bytes,
    /// `start ‖ end`, each half in the element's order-preserving
    /// encoding, strictly `start < end`. Fixed (`width: Some(w)`,
    /// `interval<E, w>`): 8 bytes — the START half only; the width is
    /// the type's, so the end derives as `start + w` at decode, and a
    /// stored start at or past the Q2 bound (`start + w < MAX_END`) is
    /// corruption (the engine's
    /// `error::CorruptionError::InvalidFixedIntervalStart`).
    Interval {
        /// The element domain: one of the two orderable scalars.
        element: IntervalElement,
        /// `Some(w)`: the fixed width — the encoding is one word.
        width: Option<u64>,
    },
}

impl TypeDesc {
    /// Encoded width in bytes: 1 for `Bool`, 16 for a general
    /// `Interval` and 8 for a fixed-width one (the width halving — the
    /// end is the type's to derive, so storing it would be
    /// transcription), the word-padded `⌈len/8⌉ × 8` for `FixedBytes`,
    /// 8 for everything else.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            // A fixed-width interval is one word — the start; the end
            // is the type's to derive (the width halving).
            Self::U64 | Self::I64 | Self::String | Self::Interval { width: Some(_), .. } => 8,
            Self::FixedBytes { len } => (len as usize).div_ceil(8) * 8,
            Self::Interval { width: None, .. } => 16,
        }
    }
}

impl ValueType {
    /// The encoding-level description this type lowers to.
    #[must_use]
    pub fn type_desc(&self) -> TypeDesc {
        match self {
            Self::Bool => TypeDesc::Bool,
            Self::U64 => TypeDesc::U64,
            Self::I64 => TypeDesc::I64,
            Self::String => TypeDesc::String,
            Self::FixedBytes { len } => TypeDesc::FixedBytes { len: *len },
            Self::Interval { element, width } => TypeDesc::Interval {
                element: *element,
                width: *width,
            },
        }
    }
}
