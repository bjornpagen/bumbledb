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

pub use decode::{decode_bool, decode_field, decode_u64, field_bytes};
pub use encode::{
    encode_bool, encode_fact, encode_fixed_bytes, encode_i64, encode_interval_i64,
    encode_interval_u64, encode_literal, encode_u64,
};
pub use fact_hash::fact_hash;

use crate::schema::IntervalElement;

/// The `bytes<N>` width ceiling: 64 bytes = 8 words = two cache lines of
/// key material — digests in the wild are 16/20/32/64
/// (`docs/architecture/10-data-model.md`). Schema validation rejects
/// widths outside `1..=MAX_FIXED_BYTES` with a typed `SchemaError`.
pub const MAX_FIXED_BYTES: usize = 64;

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
        /// Declared width in bytes, `1..=MAX_FIXED_BYTES`.
        len: u16,
    },
    /// 16 bytes: `start ‖ end`, each half in the element's order-preserving
    /// encoding, strictly `start < end`.
    Interval {
        /// The element domain: one of the two orderable scalars.
        element: IntervalElement,
    },
}

impl TypeDesc {
    /// Encoded width in bytes: 1 for `Bool`, 16 for `Interval`, the
    /// word-padded `⌈len/8⌉ × 8` for `FixedBytes`, 8 for everything else.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            Self::U64 | Self::I64 | Self::String => 8,
            Self::FixedBytes { len } => (len as usize).div_ceil(8) * 8,
            Self::Interval { .. } => 16,
        }
    }
}

/// The word count of a `bytes<len>` value's padded encoding: `⌈len/8⌉`.
#[must_use]
pub const fn fixed_bytes_words(len: u16) -> usize {
    (len as usize).div_ceil(8)
}

/// One `bytes<N>` value at the encoding layer: the raw bytes inline in a
/// fixed 64-byte buffer (`Copy`, borrow-free — the fixed-width law), pad
/// beyond `len` zero by construction so derived equality is value
/// equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedBytesValue {
    bytes: [u8; MAX_FIXED_BYTES],
    len: u8,
}

impl FixedBytesValue {
    /// Wraps `raw` (the value's exact declared width).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a width outside
    /// `1..=MAX_FIXED_BYTES` — schema validation makes such fields
    /// undeclarable, and every caller is schema-typed.
    #[must_use]
    pub fn new(raw: &[u8]) -> Self {
        assert!(
            !raw.is_empty() && raw.len() <= MAX_FIXED_BYTES,
            "validated schema: bytes<N> widths are 1..=64"
        );
        let mut bytes = [0u8; MAX_FIXED_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        Self {
            bytes,
            len: u8::try_from(raw.len()).expect("len <= 64"),
        }
    }

    /// The value's `len` raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    /// The declared width in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// The canonical word-padded encoding: `⌈len/8⌉ × 8` bytes, the raw
    /// bytes zero-padded (pad already zero by construction).
    #[must_use]
    pub fn padded(&self) -> &[u8] {
        &self.bytes[..fixed_bytes_words(u16::from(self.len)) * 8]
    }
}

/// A decoded field value at the encoding layer.
///
/// `String` carries an intern id here; resolving an id to raw bytes is
/// the dictionary's job (docs/architecture/50-storage.md). `FixedBytes`
/// carries its value whole — bytes<N> values are inline, never interned.
/// Every variant is fixed-width, so the type is `Copy` and carries no
/// borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueRef {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Intern id of a UTF-8 string.
    String(u64),
    /// A `bytes<N>` value, inline.
    FixedBytes(FixedBytesValue),
    /// Interval over U64: `(start, end)`, strictly `start < end`.
    IntervalU64(u64, u64),
    /// Interval over I64: `(start, end)`, strictly `start < end`.
    IntervalI64(i64, i64),
}

impl ValueRef {
    /// Wraps raw `bytes<N>` bytes (the macro codegen's constructor).
    ///
    /// # Panics
    ///
    /// As [`FixedBytesValue::new`] — schema-typed callers only.
    #[must_use]
    pub fn fixed_bytes(raw: &[u8]) -> Self {
        Self::FixedBytes(FixedBytesValue::new(raw))
    }
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
