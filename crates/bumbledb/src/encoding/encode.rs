//! The encode side: canonical per-type encoders and the fact encoder.

use super::{FactLayout, I64_SIGN_BIT, TypeDesc, ValueRef};
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
///
/// Because each half is order-preserving, the 16 bytes sort
/// lexicographically by `(start, end)` — load-bearing for the storage
/// layer's neighbor probes (`docs/architecture/50-storage.md`).
///
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
/// FIELD'S ENCODING — every [`Value`] variant whose canonical bytes are
/// a pure function of the value and its field type. A lowering plus
/// delegation, not a second per-field encoder: the `(value, desc)` pair
/// lowers to the [`ValueRef`] variant that already reifies the
/// fixed-width distinction, and [`append_key_field`] writes the bytes —
/// so the commit judgment's pre-encoded σ literals, the schema
/// fingerprint's canonical encoding, and the fact encoding agree by
/// construction, never by discipline. The `desc` parameter carries the
/// field's [`TypeDesc`] because a type is an encoding and the FIELD
/// owns it: the same checked interval value encodes as 16 bytes at a
/// general interval position and as its 8-byte start at a fixed-width
/// one (`interval<E, w>` — the width is the type's, so the end is
/// derived, never stored). `String` is the deliberate exception — its
/// fact encoding is a per-database intern id, not a function of the
/// value — so each consumer resolves it at its own boundary before
/// calling. `FixedBytes` is self-encoding: the raw bytes, word-padded,
/// inline in the fact.
///
/// # Panics
///
/// On `String` — programmer invariant: callers peel the interned
/// variant first.
pub fn encode_literal(value: &Value, desc: TypeDesc, out: &mut Vec<u8>) {
    let fixed_width = matches!(desc, TypeDesc::Interval { width: Some(_), .. });
    let value = match value {
        Value::Bool(v) => ValueRef::Bool(*v),
        Value::U64(v) => ValueRef::U64(*v),
        Value::I64(v) => ValueRef::I64(*v),
        Value::FixedBytes(raw) => ValueRef::fixed_bytes(raw),
        Value::IntervalU64(interval) if fixed_width => ValueRef::FixedIntervalU64(*interval),
        Value::IntervalU64(interval) => ValueRef::IntervalU64(*interval),
        Value::IntervalI64(interval) if fixed_width => ValueRef::FixedIntervalI64(*interval),
        Value::IntervalI64(interval) => ValueRef::IntervalI64(*interval),
        Value::String(_) => {
            unreachable!("interned literals resolve at their consumer's boundary")
        }
        // A mask is not a field type; nothing storable carries one.
        Value::AllenMask(_) => unreachable!("mask values never encode"),
    };
    append_key_field(value, out);
}

/// Appends the canonical encoding of ONE field value — the per-field
/// unit the fact encoder and the typed-key determinant path
/// (`api/db`'s `Key` trait) share. One definition site: a key value's
/// determinant bytes and the span `storage/keys::determinant_image`
/// slices out of a stored fact are the same encoding by construction
/// (the parity law, pinned by
/// `append_key_field_matches_determinant_image_slices`). `String`
/// carries a resolved intern id (resolution is the caller's boundary);
/// the fixed-width interval family writes ONE word — the start; the
/// width is the type's and the end derives at decode
/// (`docs/architecture/50-storage.md`).
pub fn append_key_field(value: ValueRef, out: &mut Vec<u8>) {
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
            out.extend_from_slice(&encode_u64(id));
        }
        ValueRef::FixedBytes(value) => {
            out.extend_from_slice(value.padded());
        }
        ValueRef::IntervalU64(interval) => {
            out.extend_from_slice(&encode_interval_u64(interval));
        }
        ValueRef::IntervalI64(interval) => {
            out.extend_from_slice(&encode_interval_i64(interval));
        }
        ValueRef::FixedIntervalU64(interval) => {
            out.extend_from_slice(&encode_u64(interval.start()));
        }
        ValueRef::FixedIntervalI64(interval) => {
            out.extend_from_slice(&encode_i64(interval.start()));
        }
    }
}

/// Appends the canonical encoding of a full fact to `out` — each field
/// through [`append_key_field`], so the fact encoding IS the field
/// encoding concatenated (no second per-field encoder can drift).
///
/// `values` match the layout positionally by construction: typed fact codegen
/// emits both from one schema declaration, while dynamic ingress builds the
/// refs only after `value_matches` has accepted the same layout walk. Decode
/// callers obtain each ref from that layout itself. No raw interval bounds can
/// reach this function.
pub fn encode_fact(values: &[ValueRef], layout: &FactLayout, out: &mut Vec<u8>) {
    out.reserve(layout.fact_width());
    for value in values {
        append_key_field(*value, out);
    }
}
