//! The encode side: canonical per-type encoders and the fact encoder.

use super::{FactLayout, TypeDesc, ValueRef, I64_SIGN_BIT};

/// Encodes a Bool as its canonical single byte.
#[must_use]
pub const fn encode_bool(value: bool) -> u8 {
    value as u8
}

/// Encodes a U64 as big-endian bytes (lexicographic order = numeric order).
#[must_use]
pub const fn encode_u64(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

/// Encodes an I64 as sign-flipped big-endian bytes: flipping the sign bit
/// biases the value so lexicographic byte order equals numeric order.
#[must_use]
pub const fn encode_i64(value: i64) -> [u8; 8] {
    (value.cast_unsigned() ^ I64_SIGN_BIT).to_be_bytes()
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
