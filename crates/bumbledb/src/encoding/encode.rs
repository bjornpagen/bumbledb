//! The encode side: canonical per-type encoders and the fact encoder.

use super::{FactLayout, I64_SIGN_BIT, ValueRef, ValueType};
use bumbledb_theory::{Interval, Value};

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

/// Encodes an Interval over U64 as `start ‖ end`, each half [`encode_u64`].
/// Because each half is order-preserving, the 16 bytes sort
/// lexicographically by `(start, end)` — load-bearing for the storage
/// layer's neighbor probes.
/// The checked input type makes `start < end` unconstructible.
#[must_use]
pub fn encode_interval_u64(interval: Interval<u64>) -> [u8; 16] {
    let (start, end) = interval.bounds();
    concat_halves(encode_u64(start), encode_u64(end))
}

/// Encodes an Interval over I64 as `start ‖ end`, each half [`encode_i64`].
/// The same `(start, end)` lexicographic-sort contract as
/// [`encode_interval_u64`].
#[must_use]
pub fn encode_interval_i64(interval: Interval<i64>) -> [u8; 16] {
    let (start, end) = interval.bounds();
    concat_halves(encode_i64(start), encode_i64(end))
}

fn concat_halves(start: [u8; 8], end: [u8; 8]) -> [u8; 16] {
    let mut out = [0; 16];
    out[..8].copy_from_slice(&start);
    out[8..].copy_from_slice(&end);
    out
}

/// Appends the canonical encoding of a self-encoding literal AT ITS
/// FIELD'S ENCODING. The field's [`ValueType`] owns the width: the same
/// checked interval value encodes as 16 bytes at a general interval
/// position and as its 8-byte start at a fixed-width one.
/// # Panics
/// variant first.
/// On `String` — programmer invariant: callers peel the interned
pub fn encode_literal(value: &Value, ty: ValueType, out: &mut Vec<u8>) {
    let value = match value {
        Value::Bool(v) => ValueRef::Bool(*v),
        Value::U64(v) => ValueRef::U64(*v),
        Value::I64(v) => ValueRef::I64(*v),
        Value::FixedBytes(raw) => ValueRef::bytes(raw),
        Value::IntervalU64(interval) => ValueRef::IntervalU64(*interval),
        Value::IntervalI64(interval) => ValueRef::IntervalI64(*interval),
        Value::String(_) => {
            unreachable!("interned literals resolve at their consumer's boundary")
        }
    };
    append_field(value, ty, out);
}

/// Appends one field at the layout's type — width lives here, not on
/// [`ValueRef`]. A `bytes<N>` payload writes `N`'s padded width; a
/// general interval value at a fixed-width slot writes the start word.
pub fn append_field(value: ValueRef, ty: ValueType, out: &mut Vec<u8>) {
    match (value, ty) {
        (ValueRef::Bytes(buf), ValueType::FixedBytes { len }) => {
            out.extend_from_slice(&buf[..super::fixed_bytes_words(len) * 8]);
        }
        (ValueRef::IntervalU64(interval), ValueType::FixedInterval { .. }) => {
            out.extend_from_slice(&encode_u64(interval.start()));
        }
        (ValueRef::IntervalI64(interval), ValueType::FixedInterval { .. }) => {
            out.extend_from_slice(&encode_i64(interval.start()));
        }
        (value, _) => append_key_field(value, out),
    }
}

pub(crate) fn append_key_field(value: ValueRef, out: &mut Vec<u8>) {
    match value {
        ValueRef::Bool(v) => {
            out.push(encode_bool(v));
        }
        ValueRef::U64(v) => {
            out.extend_from_slice(&encode_u64(v));
        }
        ValueRef::I64(v) => {
            out.extend_from_slice(&encode_i64(v));
        }
        ValueRef::String(id) => {
            out.extend_from_slice(&encode_u64(id.raw()));
        }
        ValueRef::Bytes(_) => {
            panic!("bytes<N> field: append_field writes at the layout type")
        }
        ValueRef::IntervalU64(interval) => {
            out.extend_from_slice(&encode_interval_u64(interval));
        }
        ValueRef::IntervalI64(interval) => {
            out.extend_from_slice(&encode_interval_i64(interval));
        }
    }
}

/// through [`append_field`] at the layout's type, so a general interval
/// value at a fixed-width slot cannot silently write 16 bytes into 8.
pub fn encode_fact(values: &[ValueRef], layout: &FactLayout, out: &mut Vec<u8>) {
    debug_assert_eq!(values.len(), layout.field_count());
    out.reserve(layout.fact_width());
    for (idx, value) in values.iter().enumerate() {
        append_field(*value, layout.field_type(idx), out);
    }
}
