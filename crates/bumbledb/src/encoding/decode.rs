//! The decode side: canonical per-type decoders, field slicing, and the
//! corruption-checked field decoder.

use super::{FactLayout, IntervalElement, TypeDesc, ValueRef, I64_SIGN_BIT};
use crate::error::CorruptionError;

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

/// Decodes big-endian U64 bytes.
#[must_use]
pub const fn decode_u64(bytes: [u8; 8]) -> u64 {
    u64::from_be_bytes(bytes)
}

/// Decodes sign-flipped big-endian I64 bytes.
#[must_use]
pub const fn decode_i64(bytes: [u8; 8]) -> i64 {
    (u64::from_be_bytes(bytes) ^ I64_SIGN_BIT).cast_signed()
}

/// Decodes an Interval-over-U64's `start ‖ end` bytes, validating strict
/// `start < end`.
///
/// # Errors
///
/// [`CorruptionError::InvalidInterval`] when `start >= end` — a stored
/// empty or inverted interval denotes nothing, exactly as corrupt as a
/// non-0/1 Bool byte.
pub const fn decode_interval_u64(bytes: [u8; 16]) -> Result<(u64, u64), CorruptionError> {
    let (start_bytes, end_bytes) = split_halves(bytes);
    let (start, end) = (decode_u64(start_bytes), decode_u64(end_bytes));
    if start < end {
        Ok((start, end))
    } else {
        Err(CorruptionError::InvalidInterval(bytes))
    }
}

/// Decodes an Interval-over-I64's `start ‖ end` bytes, validating strict
/// `start < end`.
///
/// # Errors
///
/// [`CorruptionError::InvalidInterval`], as [`decode_interval_u64`].
pub const fn decode_interval_i64(bytes: [u8; 16]) -> Result<(i64, i64), CorruptionError> {
    let (start_bytes, end_bytes) = split_halves(bytes);
    let (start, end) = (decode_i64(start_bytes), decode_i64(end_bytes));
    if start < end {
        Ok((start, end))
    } else {
        Err(CorruptionError::InvalidInterval(bytes))
    }
}

const fn split_halves(bytes: [u8; 16]) -> ([u8; 8], [u8; 8]) {
    let (mut start, mut end) = ([0; 8], [0; 8]);
    let mut i = 0;
    while i < 8 {
        start[i] = bytes[i];
        end[i] = bytes[i + 8];
        i += 1;
    }
    (start, end)
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
/// [`CorruptionError`] on a Bool byte that is not `0x00`/`0x01`, an Enum
/// ordinal outside the declared variant list, or an Interval whose
/// `start >= end` — never a skip, never a default.
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
        TypeDesc::Interval { element } => {
            let bytes: [u8; 16] = bytes.try_into().expect("16-byte field slice");
            match element {
                IntervalElement::U64 => {
                    decode_interval_u64(bytes).map(|(s, e)| ValueRef::IntervalU64(s, e))
                }
                IntervalElement::I64 => {
                    decode_interval_i64(bytes).map(|(s, e)| ValueRef::IntervalI64(s, e))
                }
            }
        }
    }
}
