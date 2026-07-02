//! Canonical per-type encodings and the fact codec (PRD 01).
//!
//! The byte-level truth of the whole system: everything above stores, hashes,
//! and compares exactly these bytes. Canonical means injective and unique
//! (`docs/architecture/10-data-model.md`): one value, one byte string, so
//! value equality is `fact_bytes` equality.

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
}

impl TypeDesc {
    /// Encoded width in bytes: 1 for `Bool`/`Enum`, 8 for everything else.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool | Self::Enum { .. } => 1,
            Self::U64 | Self::I64 | Self::String | Self::Bytes => 8,
        }
    }
}

/// A decoded field value at the encoding layer.
///
/// `String`/`Bytes` carry intern ids here; resolving an id to raw bytes is
/// the dictionary's job (PRD 05). Every variant is a fixed-width scalar, so
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
}

use crate::error::CorruptionError;

const I64_SIGN_BIT: u64 = 1 << 63;

/// Encodes a Bool as its canonical single byte.
#[must_use]
pub const fn encode_bool(value: bool) -> u8 {
    value as u8
}

/// Decodes a canonical Bool byte.
///
/// # Errors
///
/// [`CorruptionError::InvalidBool`] on any byte other than `0x00`/`0x01`.
pub const fn decode_bool(byte: u8) -> Result<bool, CorruptionError> {
    match byte {
        0x00 => Ok(false),
        0x01 => Ok(true),
        other => Err(CorruptionError::InvalidBool(other)),
    }
}

/// Decodes an Enum ordinal byte, range-checking it against the variant list.
///
/// # Errors
///
/// [`CorruptionError::EnumOrdinalOutOfRange`] when `ordinal >= variant_count`.
pub const fn decode_enum(ordinal: u8, variant_count: u16) -> Result<u8, CorruptionError> {
    if (ordinal as u16) < variant_count {
        Ok(ordinal)
    } else {
        Err(CorruptionError::EnumOrdinalOutOfRange {
            ordinal,
            variant_count,
        })
    }
}

/// Encodes a U64 as big-endian bytes (lexicographic order = numeric order).
#[must_use]
pub const fn encode_u64(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

/// Decodes big-endian U64 bytes.
#[must_use]
pub const fn decode_u64(bytes: [u8; 8]) -> u64 {
    u64::from_be_bytes(bytes)
}

/// Encodes an I64 as sign-flipped big-endian bytes: flipping the sign bit
/// biases the value so lexicographic byte order equals numeric order.
#[must_use]
pub const fn encode_i64(value: i64) -> [u8; 8] {
    (value.cast_unsigned() ^ I64_SIGN_BIT).to_be_bytes()
}

/// Decodes sign-flipped big-endian I64 bytes.
#[must_use]
pub const fn decode_i64(bytes: [u8; 8]) -> i64 {
    (u64::from_be_bytes(bytes) ^ I64_SIGN_BIT).cast_signed()
}

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
    /// Computes the layout for the given field types in declaration order.
    #[must_use]
    pub fn new(field_types: &[TypeDesc]) -> Self {
        let mut offset = 0;
        let fields = field_types
            .iter()
            .map(|&desc| {
                let slot = (offset, desc);
                offset += desc.width();
                slot
            })
            .collect();
        Self {
            fields,
            fact_width: offset,
        }
    }

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

/// Appends the canonical encoding of a full fact to `out`.
///
/// `values` must match the layout's field types positionally — that is a
/// programmer invariant of the typed callers above this layer, checked by
/// `debug_assert!` on this hot path.
pub fn encode_fact(values: &[ValueRef], layout: &FactLayout, out: &mut Vec<u8>) {
    debug_assert_eq!(values.len(), layout.field_count());
    out.reserve(layout.fact_width());
    for (value, &(_, desc)) in values.iter().zip(&layout.fields) {
        match *value {
            ValueRef::Bool(v) => {
                debug_assert_eq!(desc, TypeDesc::Bool);
                out.push(encode_bool(v));
            }
            ValueRef::Enum(ordinal) => {
                debug_assert!(matches!(
                    desc,
                    TypeDesc::Enum { variant_count } if u16::from(ordinal) < variant_count
                ));
                out.push(ordinal);
            }
            ValueRef::U64(v) => {
                debug_assert_eq!(desc, TypeDesc::U64);
                out.extend_from_slice(&encode_u64(v));
            }
            ValueRef::I64(v) => {
                debug_assert_eq!(desc, TypeDesc::I64);
                out.extend_from_slice(&encode_i64(v));
            }
            ValueRef::String(id) => {
                debug_assert_eq!(desc, TypeDesc::String);
                out.extend_from_slice(&encode_u64(id));
            }
            ValueRef::Bytes(id) => {
                debug_assert_eq!(desc, TypeDesc::Bytes);
                out.extend_from_slice(&encode_u64(id));
            }
        }
    }
}

/// Slices one field's bytes out of an encoded fact in O(1).
#[must_use]
pub fn field_bytes<'a>(fact_bytes: &'a [u8], layout: &FactLayout, field_idx: usize) -> &'a [u8] {
    debug_assert_eq!(fact_bytes.len(), layout.fact_width());
    let (offset, desc) = layout.fields[field_idx];
    &fact_bytes[offset..offset + desc.width()]
}

/// Decodes one field of an encoded fact.
///
/// # Errors
///
/// [`CorruptionError`] on a Bool byte that is not `0x00`/`0x01` or an Enum
/// ordinal outside the declared variant list — never a skip, never a default.
///
/// # Panics
///
/// Only on a programmer-invariant violation: `fact_bytes` not matching the
/// layout's width (callers slice facts produced under the same layout).
pub fn decode_field(
    fact_bytes: &[u8],
    layout: &FactLayout,
    field_idx: usize,
) -> Result<ValueRef, CorruptionError> {
    let bytes = field_bytes(fact_bytes, layout, field_idx);
    let word = |b: &[u8]| decode_u64(b.try_into().expect("8-byte field slice"));
    match layout.field_type(field_idx) {
        TypeDesc::Bool => decode_bool(bytes[0]).map(ValueRef::Bool),
        TypeDesc::Enum { variant_count } => {
            decode_enum(bytes[0], variant_count).map(ValueRef::Enum)
        }
        TypeDesc::U64 => Ok(ValueRef::U64(word(bytes))),
        TypeDesc::I64 => Ok(ValueRef::I64(decode_i64(
            bytes.try_into().expect("8-byte field slice"),
        ))),
        TypeDesc::String => Ok(ValueRef::String(word(bytes))),
        TypeDesc::Bytes => Ok(ValueRef::Bytes(word(bytes))),
    }
}

/// Fact identity: the full 32-byte blake3 of the canonical fact bytes.
///
/// Never truncated (v5 truncated to 16 bytes — post-mortem §00). Hash
/// equality is treated as fact equality; collisions are an accepted axiom
/// recorded in `docs/architecture/10-data-model.md`.
#[must_use]
pub fn fact_hash(fact_bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(fact_bytes).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_round_trip_and_strictness() {
        assert_eq!(encode_bool(false), 0x00);
        assert_eq!(encode_bool(true), 0x01);
        assert_eq!(decode_bool(0x00), Ok(false));
        assert_eq!(decode_bool(0x01), Ok(true));
        // Any other byte is corruption, never a distinct "true".
        for byte in [0x02, 0x7f, 0xff] {
            assert_eq!(decode_bool(byte), Err(CorruptionError::InvalidBool(byte)));
        }
    }

    #[test]
    fn enum_ordinal_range_check() {
        assert_eq!(decode_enum(0, 3), Ok(0));
        assert_eq!(decode_enum(2, 3), Ok(2));
        assert_eq!(
            decode_enum(3, 3),
            Err(CorruptionError::EnumOrdinalOutOfRange {
                ordinal: 3,
                variant_count: 3
            })
        );
        // 256 variants: every u8 ordinal is valid.
        assert_eq!(decode_enum(255, 256), Ok(255));
    }

    #[test]
    fn u64_round_trip_extremes() {
        for v in [0, 1, u64::MAX, u64::MAX - 1, 1 << 63, (1 << 63) - 1] {
            assert_eq!(decode_u64(encode_u64(v)), v);
        }
    }

    #[test]
    fn i64_round_trip_extremes() {
        for v in [0, 1, -1, i64::MAX, i64::MIN, i64::MIN + 1, i64::MAX - 1] {
            assert_eq!(decode_i64(encode_i64(v)), v);
        }
    }

    #[test]
    fn u64_order_preservation() {
        let samples = [
            0u64,
            1,
            2,
            255,
            256,
            65_535,
            1 << 32,
            (1 << 63) - 1,
            1 << 63,
            u64::MAX,
        ];
        for pair in samples.windows(2) {
            assert!(pair[0] < pair[1]);
            assert!(
                encode_u64(pair[0]) < encode_u64(pair[1]),
                "encode({}) must sort below encode({})",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn i64_order_preservation_across_sign_boundary() {
        let samples = [
            i64::MIN,
            i64::MIN + 1,
            -65_536,
            -256,
            -2,
            -1,
            0,
            1,
            2,
            256,
            65_536,
            i64::MAX - 1,
            i64::MAX,
        ];
        for pair in samples.windows(2) {
            assert!(pair[0] < pair[1]);
            assert!(
                encode_i64(pair[0]) < encode_i64(pair[1]),
                "encode({}) must sort below encode({})",
                pair[0],
                pair[1]
            );
        }
    }

    /// A mixed 1/8-byte layout: Bool, Enum, U64, I64, String, Bytes.
    fn mixed_layout() -> FactLayout {
        FactLayout::new(&[
            TypeDesc::Bool,
            TypeDesc::Enum { variant_count: 3 },
            TypeDesc::U64,
            TypeDesc::I64,
            TypeDesc::String,
            TypeDesc::Bytes,
        ])
    }

    #[test]
    fn layout_offsets_are_cumulative_widths_with_no_padding() {
        let layout = mixed_layout();
        assert_eq!(layout.field_count(), 6);
        // 1 + 1 + 8 + 8 + 8 + 8 — 1-byte fields sit flush against 8-byte ones.
        assert_eq!(layout.field_offset(0), 0);
        assert_eq!(layout.field_offset(1), 1);
        assert_eq!(layout.field_offset(2), 2);
        assert_eq!(layout.field_offset(3), 10);
        assert_eq!(layout.field_offset(4), 18);
        assert_eq!(layout.field_offset(5), 26);
        assert_eq!(layout.fact_width(), 34);
    }

    fn mixed_values() -> Vec<ValueRef> {
        vec![
            ValueRef::Bool(true),
            ValueRef::Enum(2),
            ValueRef::U64(u64::MAX),
            ValueRef::I64(i64::MIN),
            ValueRef::String(7),
            ValueRef::Bytes(9),
        ]
    }

    #[test]
    fn encode_fact_matches_independent_field_encodings() {
        let layout = mixed_layout();
        let mut fact = Vec::new();
        encode_fact(&mixed_values(), &layout, &mut fact);
        assert_eq!(fact.len(), layout.fact_width());

        let mut expected = vec![0x01, 0x02];
        expected.extend_from_slice(&encode_u64(u64::MAX));
        expected.extend_from_slice(&encode_i64(i64::MIN));
        expected.extend_from_slice(&encode_u64(7));
        expected.extend_from_slice(&encode_u64(9));
        assert_eq!(fact, expected);
    }

    #[test]
    fn field_bytes_slices_equal_independent_encodings() {
        let layout = mixed_layout();
        let mut fact = Vec::new();
        encode_fact(&mixed_values(), &layout, &mut fact);

        assert_eq!(field_bytes(&fact, &layout, 0), &[0x01]);
        assert_eq!(field_bytes(&fact, &layout, 1), &[0x02]);
        assert_eq!(field_bytes(&fact, &layout, 2), encode_u64(u64::MAX));
        assert_eq!(field_bytes(&fact, &layout, 3), encode_i64(i64::MIN));
        assert_eq!(field_bytes(&fact, &layout, 4), encode_u64(7));
        assert_eq!(field_bytes(&fact, &layout, 5), encode_u64(9));
    }

    #[test]
    fn decode_field_round_trips_every_type() {
        let layout = mixed_layout();
        let values = mixed_values();
        let mut fact = Vec::new();
        encode_fact(&values, &layout, &mut fact);
        for (idx, expected) in values.iter().enumerate() {
            assert_eq!(decode_field(&fact, &layout, idx), Ok(*expected));
        }
    }

    #[test]
    fn decode_field_surfaces_corruption() {
        let layout = mixed_layout();
        let mut fact = Vec::new();
        encode_fact(&mixed_values(), &layout, &mut fact);
        fact[0] = 0x02; // corrupt the Bool
        assert_eq!(
            decode_field(&fact, &layout, 0),
            Err(CorruptionError::InvalidBool(0x02))
        );
        fact[0] = 0x01;
        fact[1] = 0x03; // corrupt the Enum ordinal (variant_count = 3)
        assert_eq!(
            decode_field(&fact, &layout, 1),
            Err(CorruptionError::EnumOrdinalOutOfRange {
                ordinal: 3,
                variant_count: 3
            })
        );
    }

    #[test]
    fn nullary_fact_layout_and_hash() {
        // Nullary relations are legal (10-data-model): the empty fact encodes
        // to zero bytes and still has a well-defined identity hash.
        let layout = FactLayout::new(&[]);
        assert_eq!(layout.fact_width(), 0);
        let mut fact = Vec::new();
        encode_fact(&[], &layout, &mut fact);
        assert!(fact.is_empty());
        assert_eq!(fact_hash(&fact), *blake3::hash(b"").as_bytes());
    }

    #[test]
    fn fact_hash_is_full_32_byte_blake3() {
        let bytes = b"arbitrary fact bytes";
        let hash = fact_hash(bytes);
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, *blake3::hash(bytes).as_bytes());
        assert_ne!(fact_hash(b"a"), fact_hash(b"b"));
    }
}
