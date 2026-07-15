use super::{RelationId, Snapshot, ValueRef, WriteTx};
use crate::encoding::decode_field;
use crate::error::{CorruptionError, Error, FactShapeError, Result};
use crate::storage::dict;

/// The typed path's fixed-width interval boundary: checks the host's
/// checked interval against the field's declared width — exactly the
/// `value_matches` rule (width equality and the Q2 ray bar; the width
/// is the type, so a wide or narrow value is a type mismatch) — and
/// wraps it as the one-word-encoding [`ValueRef`].
///
/// # Errors
///
/// [`FactShapeError::TypeMismatch`] when the interval's width is not the
/// declared `width` or its end is the domain ceiling (a ray).
pub fn fixed_interval_u64(
    relation: RelationId,
    field: crate::schema::FieldId,
    interval: crate::Interval<u64>,
    width: u64,
) -> Result<ValueRef> {
    if interval.end() - interval.start() == width && !interval.is_ray() {
        Ok(ValueRef::FixedIntervalU64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}

/// The `i64` sibling of [`fixed_interval_u64`].
///
/// # Errors
///
/// As [`fixed_interval_u64`].
pub fn fixed_interval_i64(
    relation: RelationId,
    field: crate::schema::FieldId,
    interval: crate::Interval<i64>,
    width: u64,
) -> Result<ValueRef> {
    if interval.end().abs_diff(interval.start()) == width && !interval.is_ray() {
        Ok(ValueRef::FixedIntervalI64(interval))
    } else {
        Err(FactShapeError::TypeMismatch { relation, field }.into())
    }
}

/// Write-context interning: novel values mint provisional ids flushed
/// at commit.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_write<S>(tx: &mut WriteTx<'_, S>, value: &str) -> Result<u64> {
    tx.delta.intern_str(&tx.view, value)
}

/// Delete-context resolution: pending id, else committed id, else
/// `None` — the fact cannot exist; nothing minted.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_delete<S>(tx: &WriteTx<'_, S>, value: &str) -> Result<Option<u64>> {
    tx.delta.resolve_str(&tx.view, value)
}

/// Read-context lookup: `None` means never interned — the fact cannot
/// exist.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_read<S>(snap: &Snapshot<'_, S>, value: &str) -> Result<Option<u64>> {
    dict::lookup_str(&snap.txn, value)
}

/// Resolves an intern id to a `&str` view of the committed dictionary
/// (decode boundary): mmap pages, transaction-stable by LMDB `CoW`. UTF-8
/// is validated here, without a copy (parse, don't validate).
///
/// # Errors
///
/// `Corruption` on a dangling id or non-UTF-8 stored bytes.
pub fn resolve_string<'a, S>(snap: &'a Snapshot<'_, S>, id: u64) -> Result<&'a str> {
    let raw = dict::resolve(&snap.txn, id)?;
    std::str::from_utf8(raw).map_err(|_| Error::Corruption(CorruptionError::NonUtf8Intern(id)))
}

/// Write-context sibling of [`resolve_string`], for the point-read decode:
/// provisional ids minted this transaction resolve through the delta's
/// pending map (borrowed from its arena — the read-your-writes source),
/// committed ids through the dictionary's mmap pages.
///
/// # Errors
///
/// `Corruption` on a dangling id or non-UTF-8 stored bytes.
pub fn resolve_string_write<'a, S>(tx: &'a WriteTx<'_, S>, id: u64) -> Result<&'a str> {
    let raw = match tx.delta.pending_raw(id) {
        Some(raw) => raw,
        None => dict::resolve(&tx.view, id)?,
    };
    std::str::from_utf8(raw).map_err(|_| Error::Corruption(CorruptionError::NonUtf8Intern(id)))
}

/// Appends the canonical fact bytes for a write-context encode.
pub fn encode_write_fact<S>(
    tx: &WriteTx<'_, S>,
    rel: RelationId,
    values: &[ValueRef],
    out: &mut Vec<u8>,
) {
    crate::encoding::encode_fact(values, tx.schema.relation(rel).layout(), out);
}

/// Appends the canonical fact bytes for a read-context encode.
pub fn encode_read_fact<S>(
    snap: &Snapshot<'_, S>,
    rel: RelationId,
    values: &[ValueRef],
    out: &mut Vec<u8>,
) {
    crate::encoding::encode_fact(values, snap.schema.relation(rel).layout(), out);
}

/// Decodes one field of canonical fact bytes.
///
/// # Errors
///
/// `Corruption` on undecodable bytes.
pub fn decode<S>(
    snap: &Snapshot<'_, S>,
    rel: RelationId,
    fact: &[u8],
    idx: usize,
) -> Result<ValueRef> {
    Ok(decode_field(fact, snap.schema.relation(rel).layout(), idx)?)
}

/// Write-context sibling of [`decode`] (the layout comes from the write
/// transaction's schema).
///
/// # Errors
///
/// `Corruption` on undecodable bytes.
pub fn decode_write<S>(
    tx: &WriteTx<'_, S>,
    rel: RelationId,
    fact: &[u8],
    idx: usize,
) -> Result<ValueRef> {
    Ok(decode_field(fact, tx.schema.relation(rel).layout(), idx)?)
}
