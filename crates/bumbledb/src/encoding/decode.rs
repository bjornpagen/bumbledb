//! The decode side: canonical per-type decoders, field slicing, and the
//! corruption-checked field decoder.

use super::{
    FactView, FixedBytesValue, I64_SIGN_BIT, InternId, IntervalElement, ValueRef, ValueType,
};
use crate::error::{CorruptionError, Error};
use bumbledb_theory::Interval;

/// The four corruption classes [`decode_field`] can produce. Broader
/// [`CorruptionError`] variants are not field-decode failures; the sweeper
/// matches this type exhaustively instead of proving a subset with
/// `unreachable!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldDecodeError {
    InvalidBool(u8),
    NonzeroFixedBytesPad([u8; 8]),
    InvalidInterval([u8; 16]),
    InvalidFixedIntervalStart([u8; 8]),
}

impl From<FieldDecodeError> for CorruptionError {
    fn from(err: FieldDecodeError) -> Self {
        match err {
            FieldDecodeError::InvalidBool(byte) => Self::InvalidBool(byte),
            FieldDecodeError::NonzeroFixedBytesPad(word) => Self::NonzeroFixedBytesPad(word),
            FieldDecodeError::InvalidInterval(bytes) => Self::InvalidInterval(bytes),
            FieldDecodeError::InvalidFixedIntervalStart(bytes) => {
                Self::InvalidFixedIntervalStart(bytes)
            }
        }
    }
}

impl From<FieldDecodeError> for Error {
    fn from(err: FieldDecodeError) -> Self {
        Self::Corruption(err.into())
    }
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

/// Decodes an Interval-over-U64's `start ‖ end` bytes into the checked
/// host type — construction is the validation boundary (parse, don't
/// validate): the `start < end` proof rides the returned [`Interval`],
/// so no consumer re-derives it.
///
/// # Errors
///
/// [`FieldDecodeError::InvalidInterval`] when `start >= end` — a stored
/// empty or inverted interval denotes nothing, exactly as corrupt as a
/// non-0/1 Bool byte.
pub fn decode_interval_u64(bytes: [u8; 16]) -> Result<Interval<u64>, FieldDecodeError> {
    let (start_bytes, end_bytes) = split_halves(bytes);
    Interval::new(decode_u64(start_bytes), decode_u64(end_bytes))
        .ok_or(FieldDecodeError::InvalidInterval(bytes))
}

/// Decodes an Interval-over-I64's `start ‖ end` bytes into the checked
/// host type, as [`decode_interval_u64`].
///
/// # Errors
///
/// [`FieldDecodeError::InvalidInterval`], as [`decode_interval_u64`].
pub fn decode_interval_i64(bytes: [u8; 16]) -> Result<Interval<i64>, FieldDecodeError> {
    let (start_bytes, end_bytes) = split_halves(bytes);
    Interval::new(decode_i64(start_bytes), decode_i64(end_bytes))
        .ok_or(FieldDecodeError::InvalidInterval(bytes))
}

/// Decodes a fixed-width interval's stored START word (either element
/// domain: both encodings are order-preserving u64 words, and the bias
/// is additive, so `start_word + w` IS the encoded end), validating the
/// Q2 bound `start + w < MAX_END` in the word domain — both ceilings
/// encode to `u64::MAX`. Returns the `(start_word, end_word)` pair.
///
/// # Errors
///
/// [`FieldDecodeError::InvalidFixedIntervalStart`] when the stored start
/// sits at or past the bound — the derived end would reach the ceiling
/// (ray territory, unconstructible in the fixed family) or overflow.
pub const fn decode_fixed_interval_start(
    bytes: [u8; 8],
    width: u64,
) -> Result<(u64, u64), FieldDecodeError> {
    let start_word = u64::from_be_bytes(bytes);
    match start_word.checked_add(width) {
        Some(end_word) if end_word < u64::MAX => Ok((start_word, end_word)),
        _ => Err(FieldDecodeError::InvalidFixedIntervalStart(bytes)),
    }
}

/// Decodes a `bytes<len>` field's word-padded encoding, validating the
/// pad: `padded` is the field's `⌈len/8⌉ × 8` stored bytes, and every
/// byte past `len` must be zero — the pad is encoding, not data, so a
/// nonzero pad byte is corruption exactly like a non-0/1 Bool byte.
///
/// # Errors
///
/// [`FieldDecodeError::NonzeroFixedBytesPad`] on any nonzero trailing pad
/// byte (carrying the offending trailing word).
pub fn decode_fixed_bytes(padded: &[u8], len: u16) -> Result<FixedBytesValue, FieldDecodeError> {
    debug_assert_eq!(padded.len(), super::fixed_bytes_words(len) * 8);
    let len = usize::from(len);
    // A nonzero pad byte implies at least one stored word, so the
    // `last_chunk` arm of the chain always holds when the first does —
    // the offending trailing word rides the error.
    if padded[len..].iter().any(|&byte| byte != 0)
        && let Some(&tail) = padded.last_chunk()
    {
        return Err(FieldDecodeError::NonzeroFixedBytesPad(tail));
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

/// Slices one field's bytes out of a width-proved fact in O(1).
#[must_use]
pub fn field_bytes<'bytes>(fact: FactView<'bytes, '_>, field_idx: usize) -> &'bytes [u8] {
    let (offset, desc) = fact.layout.fields[field_idx];
    &fact.bytes[offset..offset + desc.width()]
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
pub fn field_word_bytes(fact: FactView<'_, '_>, field_idx: usize) -> [u8; 8] {
    <[u8; 8]>::try_from(field_bytes(fact, field_idx))
        .expect("word-width field: the layout derives the width")
}

/// Decodes one field of a width-proved fact.
///
/// # Errors
///
/// [`FieldDecodeError`] on a Bool byte that is not `0x00`/`0x01`, a
/// `bytes<N>` field with a nonzero pad byte, or an Interval whose
/// `start >= end` — never a skip, never a default.
pub fn decode_field(
    fact: FactView<'_, '_>,
    field_idx: usize,
) -> Result<ValueRef, FieldDecodeError> {
    let bytes = field_bytes(fact, field_idx);
    let word = || field_word_bytes(fact, field_idx);
    match fact.layout.field_type(field_idx) {
        ValueType::Bool => match bytes[0] {
            0x00 => Ok(ValueRef::Bool(false)),
            0x01 => Ok(ValueRef::Bool(true)),
            other => Err(FieldDecodeError::InvalidBool(other)),
        },
        ValueType::U64 => Ok(ValueRef::U64(decode_u64(word()))),
        ValueType::I64 => Ok(ValueRef::I64(decode_i64(word()))),
        ValueType::String => Ok(ValueRef::String(InternId::from_raw(decode_u64(word())))),
        ValueType::FixedBytes { len } => decode_fixed_bytes(bytes, len).map(ValueRef::FixedBytes),
        ValueType::Interval { element } => {
            let bytes: [u8; 16] = bytes
                .try_into()
                .expect("interval field: the layout derives the width");
            match element {
                IntervalElement::U64 => decode_interval_u64(bytes).map(ValueRef::IntervalU64),
                IntervalElement::I64 => decode_interval_i64(bytes).map(ValueRef::IntervalI64),
            }
        }
        ValueType::FixedInterval { element, width: w } => {
            let (start_word, end_word) = decode_fixed_interval_start(word(), w)?;
            Ok(match element {
                IntervalElement::U64 => ValueRef::IntervalU64(
                    Interval::<u64>::new(start_word, end_word)
                        .expect("the Q2 bound implies start < end"),
                ),
                IntervalElement::I64 => ValueRef::IntervalI64(
                    Interval::<i64>::new(
                        decode_i64(start_word.to_be_bytes()),
                        decode_i64(end_word.to_be_bytes()),
                    )
                    .expect("the Q2 bound implies start < end"),
                ),
            })
        }
    }
}

/// Decodes canonical fact bytes into owned dynamic [`Value`]s — the one
/// body behind the write transaction's point-read decode
/// (`WriteTx::get_dyn`), the snapshot's point-read and export decodes
/// (`ReadInstance::get_dyn` / `ReadInstance::scan`), and the commit boundary's
/// rejection decode (`storage/commit/write.rs`); only intern resolution
/// differs by context (pending-first inside a write transaction, the
/// committed dictionary on a snapshot, pending-then-committed at
/// rejection), so the resolver is the parameter.
pub(crate) fn decode_values(
    fact: FactView<'_, '_>,
    resolve_str: impl FnMut(u64) -> crate::error::Result<Box<str>>,
) -> crate::error::Result<Vec<bumbledb_theory::Value>> {
    decode_values_keyed(fact, &[], &[], resolve_str)
}

/// [`decode_values`] for the keyed point-read hit (`ReadInstance::get_dyn` /
/// `WriteTx::get_dyn`): fields the key statement's projection fixed take
/// the caller's supplied values — the `U` probe matched the determinant
/// byte-for-byte and hash equality is fact equality
/// (`docs/architecture/10-data-model.md`), so the stored field IS the
/// supplied one, and resolving it through the reverse dictionary would
/// re-derive the input with an extra storage descent. Non-projected
/// fields decode as [`decode_values`] does.
pub(crate) fn decode_values_keyed(
    fact: FactView<'_, '_>,
    projection: &[bumbledb_theory::schema::FieldId],
    key_values: &[bumbledb_theory::Value],
    resolve_str: impl FnMut(u64) -> crate::error::Result<Box<str>>,
) -> crate::error::Result<Vec<bumbledb_theory::Value>> {
    let mut out = Vec::new();
    decode_values_keyed_into(fact, projection, key_values, resolve_str, &mut out)?;
    Ok(out)
}

/// [`decode_values_keyed`] into a caller-provided buffer — the pooled
/// point-read decode (`ReadInstance::get_dyn_into` / `WriteTx::get_dyn_into`):
/// the values `Vec` is the caller's, its capacity retained across gets,
/// so a warm keyed get's allocator traffic shrinks to the variable-width
/// payload boxes alone. Clears `out` first; a decode error leaves the
/// written prefix (the caller treats `Err` as no result).
pub(crate) fn decode_values_keyed_into(
    fact: FactView<'_, '_>,
    projection: &[bumbledb_theory::schema::FieldId],
    key_values: &[bumbledb_theory::Value],
    mut resolve_str: impl FnMut(u64) -> crate::error::Result<Box<str>>,
    out: &mut Vec<bumbledb_theory::Value>,
) -> crate::error::Result<()> {
    use bumbledb_theory::Value;
    debug_assert_eq!(projection.len(), key_values.len());
    out.clear();
    out.reserve(fact.layout.field_count());
    for idx in 0..fact.layout.field_count() {
        if let Some(pos) = projection.iter().position(|f| usize::from(f.0) == idx) {
            out.push(key_values[pos].clone());
            continue;
        }
        out.push(match decode_field(fact, idx)? {
            ValueRef::Bool(v) => Value::Bool(v),
            ValueRef::U64(v) => Value::U64(v),
            ValueRef::I64(v) => Value::I64(v),
            ValueRef::String(id) => Value::String(resolve_str(id.raw())?),
            ValueRef::FixedBytes(value) => Value::FixedBytes(value.as_bytes().into()),
            ValueRef::IntervalU64(interval) => Value::IntervalU64(interval),
            ValueRef::IntervalI64(interval) => Value::IntervalI64(interval),
        });
    }
    Ok(())
}
