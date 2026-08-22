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

pub(crate) use decode::FieldDecodeError;
pub use decode::{
    decode_bool, decode_bool_at, decode_field, decode_fixed_bytes, decode_fixed_interval_start,
    decode_i64, decode_interval_i64, decode_interval_u64, decode_u64, field_bytes,
    field_word_bytes,
};
pub(crate) use decode::{decode_values, decode_values_keyed_into, interval_words, split_halves};
pub use encode::{append_field, encode_bool, encode_fact, encode_i64, encode_literal, encode_u64};
// The two-half interval encoders' production users live inside this
// module (the type-aware `encode_literal` and `encode_fact` arms); the
// crate-wide re-export survives for the byte-level test fixtures (the
// i64 twin's fixtures import it from `encode` directly).
#[cfg(test)]
pub(crate) use encode::encode_interval_u64;
pub use fact_hash::fact_hash;

// A type IS its encoding: layout vocabulary is [`ValueType`] itself.
pub use bumbledb_theory::schema::ValueType;

/// Dictionary intern id. Ids allocate from 0; [`InternId::SENTINEL`] is
/// never minted — the miss token on query-word paths and the one owner of
/// the `u64::MAX` reserved value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternId(u64);

impl InternId {
    /// Never-minted miss token (`u64::MAX`).
    pub const SENTINEL: Self = Self(u64::MAX);

    /// Wraps a stored or minted id, including [`Self::SENTINEL`].
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw word stored in facts and `_dict`.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Whether this is the never-minted miss token.
    #[must_use]
    pub const fn is_sentinel(self) -> bool {
        self.0 == Self::SENTINEL.0
    }
}

// `IntervalElement` rides along for the codec submodules (`decode`
// addresses it as `super::IntervalElement`).
use bumbledb_theory::{Interval, schema::IntervalElement};

/// The `bytes<N>` width ceiling: 64 bytes = 8 words = two cache lines of
/// key material — digests in the wild are 16/20/32/64
/// (`docs/architecture/10-data-model.md`). Schema validation rejects
/// widths outside `1..=MAX_FIXED_BYTES` with a typed `SchemaError`.
pub const MAX_FIXED_BYTES: usize = 64;

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
            "bytes<N> widths are 1..=64"
        );
        let mut bytes = [0u8; MAX_FIXED_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        Self {
            bytes,
            len: u8::try_from(raw.len()).expect("len <= 64"),
        }
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
/// the dictionary's job (docs/architecture/50-storage.md). Bytes payloads
/// carry no width — `N` lives on the layout's [`ValueType::FixedBytes`].
/// Every variant is fixed-width, so the type is `Copy` and carries no
/// borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueRef {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Intern id of a UTF-8 string.
    String(InternId),
    /// A `bytes<N>` payload. Width is the layout arm, not this buffer;
    /// unused tail is zero by construction.
    Bytes([u8; MAX_FIXED_BYTES]),
    /// Nonempty interval over U64.
    IntervalU64(Interval<u64>),
    /// Nonempty interval over I64.
    IntervalI64(Interval<i64>),
}

impl ValueRef {
    /// Wraps raw `bytes<N>` bytes. `N` is the layout's
    /// [`ValueType::FixedBytes`]; this buffer does not carry a second
    /// width.
    ///
    /// # Panics
    ///
    /// As [`FixedBytesValue::new`] — schema-typed callers only.
    #[must_use]
    pub fn bytes(raw: &[u8]) -> Self {
        assert!(
            !raw.is_empty() && raw.len() <= MAX_FIXED_BYTES,
            "bytes<N> widths are 1..=64"
        );
        let mut bytes = [0u8; MAX_FIXED_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        Self::Bytes(bytes)
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
    fields: Box<[(usize, ValueType)]>,
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
    pub fn field_type(&self, field_idx: usize) -> ValueType {
        self.fields[field_idx].1
    }

    /// Parses `bytes` as a fact of this layout. Storage reads use this so a
    /// wrong-width slice becomes [`FactView`] or absence, never a later
    /// index panic.
    #[must_use]
    pub(crate) fn view<'bytes, 'layout>(
        &'layout self,
        bytes: &'bytes [u8],
    ) -> Option<FactView<'bytes, 'layout>> {
        (bytes.len() == self.fact_width).then_some(FactView {
            bytes,
            layout: self,
        })
    }

    /// View over bytes this layout produced — encode path, sealed rows,
    /// tests. Width is a programmer invariant (`encode_fact` writes
    /// `fact_width` bytes).
    #[must_use]
    pub(crate) fn encoded<'bytes, 'layout>(
        &'layout self,
        bytes: &'bytes [u8],
    ) -> FactView<'bytes, 'layout> {
        debug_assert_eq!(bytes.len(), self.fact_width);
        FactView {
            bytes,
            layout: self,
        }
    }
}

/// Canonical fact bytes whose width has been proved against their layout.
///
/// Storage reads parse into this; field readers and determinant slicers
/// consume it, so a wrong-width slice cannot reach an index panic in
/// release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FactView<'bytes, 'layout> {
    bytes: &'bytes [u8],
    layout: &'layout FactLayout,
}

impl<'bytes, 'layout> FactView<'bytes, 'layout> {
    /// The proved fact bytes.
    #[must_use]
    pub(crate) const fn bytes(self) -> &'bytes [u8] {
        self.bytes
    }

    /// The layout this view was proved against.
    #[must_use]
    pub(crate) const fn layout(self) -> &'layout FactLayout {
        self.layout
    }
}
