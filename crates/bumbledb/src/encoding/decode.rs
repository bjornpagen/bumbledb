//! The decode side: canonical per-type decoders, field slicing, and the
//! corruption-checked field decoder.

use super::{FactLayout, FixedBytesValue, I64_SIGN_BIT, IntervalElement, TypeDesc, ValueRef};
use crate::error::CorruptionError;
use bumbledb_theory::Interval;

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

/// Decodes a fixed-width interval's stored START word (either element
/// domain: both encodings are order-preserving u64 words, and the bias
/// is additive, so `start_word + w` IS the encoded end), validating the
/// Q2 bound `start + w < MAX_END` in the word domain — both ceilings
/// encode to `u64::MAX`. Returns the `(start_word, end_word)` pair.
///
/// # Errors
///
/// [`CorruptionError::InvalidFixedIntervalStart`] when the stored start
/// sits at or past the bound — the derived end would reach the ceiling
/// (ray territory, unconstructible in the fixed family) or overflow.
pub const fn decode_fixed_interval_start(
    bytes: [u8; 8],
    width: u64,
) -> Result<(u64, u64), CorruptionError> {
    let start_word = u64::from_be_bytes(bytes);
    match start_word.checked_add(width) {
        Some(end_word) if end_word < u64::MAX => Ok((start_word, end_word)),
        _ => Err(CorruptionError::InvalidFixedIntervalStart(bytes)),
    }
}

/// Decodes a `bytes<len>` field's word-padded encoding, validating the
/// pad: `padded` is the field's `⌈len/8⌉ × 8` stored bytes, and every
/// byte past `len` must be zero — the pad is encoding, not data, so a
/// nonzero pad byte is corruption exactly like a non-0/1 Bool byte.
///
/// # Errors
///
/// [`CorruptionError::NonzeroFixedBytesPad`] on any nonzero trailing pad
/// byte (carrying the offending trailing word).
pub fn decode_fixed_bytes(padded: &[u8], len: u16) -> Result<FixedBytesValue, CorruptionError> {
    debug_assert_eq!(padded.len(), super::fixed_bytes_words(len) * 8);
    let len = usize::from(len);
    // A nonzero pad byte implies at least one stored word, so the
    // `last_chunk` arm of the chain always holds when the first does —
    // the offending trailing word rides the error.
    if padded[len..].iter().any(|&byte| byte != 0)
        && let Some(&tail) = padded.last_chunk()
    {
        return Err(CorruptionError::NonzeroFixedBytesPad(tail));
    }
    Ok(FixedBytesValue::new(&padded[..len]))
}

/// Splits an interval encoding's `start ‖ end` into its 8-byte halves
/// (readers: the interval decoders here, the image's word-pair fill, and
/// the image tests' expectations).
pub(crate) const fn split_halves(bytes: [u8; 16]) -> ([u8; 8], [u8; 8]) {
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

/// [`field_bytes`] with the width in the type: one word-width field's
/// canonical 8 bytes. The one surviving fixed-width determinant for word
/// fields — a field's width is a runtime layout fact the slice type
/// cannot carry, so every word-field consumer funnels through this
/// single check instead of checking locally.
///
/// # Panics
///
/// Only on a programmer-invariant violation: the addressed field is not
/// word-width (callers' fields are schema-validated U64/I64/String or a
/// one-word `bytes<N ≤ 8>`).
#[must_use]
pub fn field_word_bytes(fact_bytes: &[u8], layout: &FactLayout, field_idx: usize) -> [u8; 8] {
    <[u8; 8]>::try_from(field_bytes(fact_bytes, layout, field_idx))
        .expect("word-width field: the layout derives the width")
}

/// Decodes one field of an encoded fact.
///
/// # Errors
///
/// [`CorruptionError`] on a Bool byte that is not `0x00`/`0x01`, a
/// `bytes<N>` field with a nonzero pad byte, or an Interval whose
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
    let word = || field_word_bytes(fact_bytes, layout, field_idx);
    match layout.field_type(field_idx) {
        TypeDesc::Bool => decode_bool(bytes[0]).map(ValueRef::Bool),
        TypeDesc::U64 => Ok(ValueRef::U64(decode_u64(word()))),
        TypeDesc::I64 => Ok(ValueRef::I64(decode_i64(word()))),
        TypeDesc::String => Ok(ValueRef::String(decode_u64(word()))),
        TypeDesc::FixedBytes { len } => decode_fixed_bytes(bytes, len).map(ValueRef::FixedBytes),
        TypeDesc::Interval {
            element,
            width: None,
        } => {
            // The 16-byte width is layout-derived — the same
            // single-determinant ruling as [`field_word_bytes`], inline for
            // the one wide shape.
            let bytes: [u8; 16] = bytes
                .try_into()
                .expect("interval field: the layout derives the width");
            match element {
                IntervalElement::U64 => decode_interval_u64(bytes).map(|(start, end)| {
                    ValueRef::IntervalU64(
                        Interval::<u64>::new(start, end)
                            .expect("decode_interval_u64 accepted these bounds"),
                    )
                }),
                IntervalElement::I64 => decode_interval_i64(bytes).map(|(start, end)| {
                    ValueRef::IntervalI64(
                        Interval::<i64>::new(start, end)
                            .expect("decode_interval_i64 accepted these bounds"),
                    )
                }),
            }
        }
        TypeDesc::Interval {
            element,
            width: Some(w),
        } => {
            // One stored word: the start; the end re-derives from the
            // TYPE's width. The Q2 bound is the corruption check —
            // `decode_fixed_interval_start` validates it in the
            // order-preserving word domain, where the bias is additive.
            let (start_word, end_word) = decode_fixed_interval_start(word(), w)?;
            Ok(match element {
                IntervalElement::U64 => ValueRef::FixedIntervalU64(
                    Interval::<u64>::new(start_word, end_word)
                        .expect("the Q2 bound implies start < end"),
                ),
                IntervalElement::I64 => {
                    let decode = |word: u64| (word ^ I64_SIGN_BIT).cast_signed();
                    ValueRef::FixedIntervalI64(
                        Interval::<i64>::new(decode(start_word), decode(end_word))
                            .expect("the Q2 bound implies start < end"),
                    )
                }
            })
        }
    }
}

/// Decodes canonical fact bytes into owned dynamic [`Value`]s — the one
/// body behind the write transaction's point-read decode
/// (`WriteTx::get_dyn`), the snapshot's point-read and export decodes
/// (`Snapshot::get_dyn` / `Snapshot::scan`), and the commit boundary's
/// rejection decode (`storage/commit/write.rs`); only intern resolution
/// differs by context (pending-first inside a write transaction, the
/// committed dictionary on a snapshot, pending-then-committed at
/// rejection), so the resolver is the parameter.
pub(crate) fn decode_values(
    fact: &[u8],
    layout: &FactLayout,
    mut resolve_str: impl FnMut(u64) -> crate::error::Result<Box<[u8]>>,
) -> crate::error::Result<Vec<bumbledb_theory::Value>> {
    use bumbledb_theory::Value;
    (0..layout.field_count())
        .map(|idx| {
            Ok(match decode_field(fact, layout, idx)? {
                ValueRef::Bool(v) => Value::Bool(v),
                ValueRef::U64(v) => Value::U64(v),
                ValueRef::I64(v) => Value::I64(v),
                ValueRef::String(id) => Value::String(resolve_str(id)?),
                ValueRef::FixedBytes(value) => Value::FixedBytes(value.as_bytes().into()),
                // A fixed-width field decodes to the same checked host
                // interval — the end was derived from the type's width.
                ValueRef::IntervalU64(interval) | ValueRef::FixedIntervalU64(interval) => {
                    Value::IntervalU64(interval)
                }
                ValueRef::IntervalI64(interval) | ValueRef::FixedIntervalI64(interval) => {
                    Value::IntervalI64(interval)
                }
            })
        })
        .collect()
}
