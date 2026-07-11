//! The encode side: canonical per-type encoders and the fact encoder.

use super::{fixed_bytes_words, FactLayout, IntervalElement, TypeDesc, ValueRef, I64_SIGN_BIT};
use crate::value::Value;

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
/// `start < end` is a programmer invariant here (`debug_assert!`): the
/// public [`Interval`](crate::Interval) type makes the violation
/// unconstructible.
#[must_use]
pub fn encode_interval_u64(start: u64, end: u64) -> [u8; 16] {
    debug_assert!(start < end);
    concat_halves(encode_u64(start), encode_u64(end))
}

/// Encodes an Interval over I64 as `start ‖ end`, each half [`encode_i64`].
/// The same `(start, end)` lexicographic-sort and `start < end` contracts
/// as [`encode_interval_u64`].
#[must_use]
pub fn encode_interval_i64(start: i64, end: i64) -> [u8; 16] {
    debug_assert!(start < end);
    concat_halves(encode_i64(start), encode_i64(end))
}

fn concat_halves(start: [u8; 8], end: [u8; 8]) -> [u8; 16] {
    let mut out = [0; 16];
    out[..8].copy_from_slice(&start);
    out[8..].copy_from_slice(&end);
    out
}

/// Appends the canonical `bytes<N>` encoding: the N raw bytes themselves,
/// zero-padded to the word boundary (`⌈N/8⌉ × 8` bytes) — the pad is
/// encoding, not data. Injective for a fixed N, and memcmp order over the
/// padded bytes equals byte order over the values (uniform width, zero
/// tail), which is all the guard B-tree needs — order *operations* stay
/// refused at the query surface.
pub fn encode_fixed_bytes(raw: &[u8], out: &mut Vec<u8>) {
    let width = fixed_bytes_words(u16::try_from(raw.len()).expect("validated: N <= 64")) * 8;
    out.extend_from_slice(raw);
    out.resize(out.len() + width - raw.len(), 0);
}

/// Appends the canonical encoding of a self-encoding literal — every
/// [`Value`] variant whose canonical bytes are a pure function of the value.
/// The one definition site for selection-literal encoding: the commit
/// judgment's pre-encoded σ literals and the schema fingerprint's canonical
/// encoding both call this, so the two can never drift apart.
/// `String` is the deliberate exception — its fact encoding is a
/// per-database intern id, not a function of the value — so each consumer
/// resolves it at its own boundary before calling. `FixedBytes` is
/// self-encoding: the raw bytes, word-padded, inline in the fact.
///
/// # Panics
///
/// On `String` — programmer invariant: callers peel the interned
/// variant first.
pub fn encode_literal(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Bool(v) => out.push(encode_bool(*v)),
        // The canonical Enum encoding: the one-byte declaration-order
        // ordinal (`TypeDesc::Enum`).
        Value::Enum(ordinal) => out.push(*ordinal),
        Value::U64(v) => out.extend_from_slice(&encode_u64(*v)),
        Value::I64(v) => out.extend_from_slice(&encode_i64(*v)),
        Value::FixedBytes(raw) => encode_fixed_bytes(raw, out),
        Value::IntervalU64(start, end) => {
            out.extend_from_slice(&encode_interval_u64(*start, *end));
        }
        Value::IntervalI64(start, end) => {
            out.extend_from_slice(&encode_interval_i64(*start, *end));
        }
        Value::String(_) => {
            unreachable!("interned literals resolve at their consumer's boundary")
        }
        // A mask is not a field type; nothing storable carries one.
        Value::AllenMask(_) => unreachable!("mask values never encode"),
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
            ValueRef::FixedBytes(value) => {
                debug_assert!(matches!(
                    desc,
                    TypeDesc::FixedBytes { len } if usize::from(len) == value.len()
                ));
                out.extend_from_slice(value.padded());
            }
            ValueRef::IntervalU64(start, end) => {
                debug_assert_eq!(
                    desc,
                    TypeDesc::Interval {
                        element: IntervalElement::U64
                    }
                );
                out.extend_from_slice(&encode_interval_u64(start, end));
            }
            ValueRef::IntervalI64(start, end) => {
                debug_assert_eq!(
                    desc,
                    TypeDesc::Interval {
                        element: IntervalElement::I64
                    }
                );
                out.extend_from_slice(&encode_interval_i64(start, end));
            }
        }
    }
}
