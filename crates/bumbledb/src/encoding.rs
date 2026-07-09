//! Canonical per-type encodings and the fact codec (docs/architecture/10-data-model.md).
//!
//! The byte-level truth of the whole system: everything above stores, hashes,
//! and compares exactly these bytes. Canonical means injective
//! (`docs/architecture/10-data-model.md`): one value, one byte string, so
//! value equality is `fact_bytes` equality.

mod decode;
mod encode;
mod fact_hash;
mod layout;
#[cfg(test)]
mod tests;

pub use decode::{decode_bool, decode_enum, decode_field, decode_u64, field_bytes};
pub use encode::{
    encode_bool, encode_fact, encode_i64, encode_interval_i64, encode_interval_u64, encode_literal,
    encode_u64,
};
pub use fact_hash::fact_hash;

use crate::schema::IntervalElement;

/// Encoding-level description of a field's type: exactly what is needed to
/// size, encode, and corruption-check its bytes. No names anywhere — a type
/// is an encoding and nothing else (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeDesc {
    /// 1 byte, strictly `0x00` or `0x01`.
    Bool,
    /// 1 byte, declaration-order ordinal into a closed variant list.
    Enum {
        /// Number of declared variants; valid ordinals are `0..variant_count`.
        variant_count: u16,
    },
    /// 8 bytes, big-endian (order-preserving).
    U64,
    /// 8 bytes, sign-flipped big-endian (order-preserving).
    I64,
    /// 8 bytes in facts: the interned dictionary id, big-endian.
    String,
    /// 8 bytes in facts: the interned dictionary id, big-endian.
    Bytes,
    /// 16 bytes: `start ‖ end`, each half in the element's order-preserving
    /// encoding, strictly `start < end`.
    Interval {
        /// The element domain: one of the two orderable scalars.
        element: IntervalElement,
    },
}

impl TypeDesc {
    /// Encoded width in bytes: 1 for `Bool`/`Enum`, 16 for `Interval`,
    /// 8 for everything else.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool | Self::Enum { .. } => 1,
            Self::U64 | Self::I64 | Self::String | Self::Bytes => 8,
            Self::Interval { .. } => 16,
        }
    }
}

/// A decoded field value at the encoding layer.
///
/// `String`/`Bytes` carry intern ids here; resolving an id to raw bytes is
/// the dictionary's job (docs/architecture/50-storage.md). Every variant is a fixed-width scalar, so
/// the type is `Copy` and carries no borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueRef {
    Bool(bool),
    Enum(u8),
    U64(u64),
    I64(i64),
    /// Intern id of a UTF-8 string.
    String(u64),
    /// Intern id of a byte sequence.
    Bytes(u64),
    /// Interval over U64: `(start, end)`, strictly `start < end`.
    IntervalU64(u64, u64),
    /// Interval over I64: `(start, end)`, strictly `start < end`.
    IntervalI64(i64, i64),
}

const I64_SIGN_BIT: u64 = 1 << 63;

/// The byte layout of one relation's facts, computed from its ordered field
/// types: per-field offset and width, and the total fact width.
///
/// Facts are dense — each offset is exactly the sum of the preceding widths,
/// with no padding anywhere: unaligned loads are near-free on the target
/// machine, so intra-row alignment would be pure waste
/// (`docs/architecture/10-data-model.md`, `00-product.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactLayout {
    /// Per-field `(offset, type)` in declaration order.
    fields: Box<[(usize, TypeDesc)]>,
    fact_width: usize,
}

impl FactLayout {
    /// Total encoded width of one fact in bytes.
    #[must_use]
    pub const fn fact_width(&self) -> usize {
        self.fact_width
    }

    /// Number of fields in the layout.
    #[must_use]
    pub const fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Byte offset of the field at `field_idx`.
    #[must_use]
    pub fn field_offset(&self, field_idx: usize) -> usize {
        self.fields[field_idx].0
    }

    /// Type of the field at `field_idx`.
    #[must_use]
    pub fn field_type(&self, field_idx: usize) -> TypeDesc {
        self.fields[field_idx].1
    }
}
