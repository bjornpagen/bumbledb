use super::{RelationId, Snapshot, ValueRef, WriteTx};
use crate::encoding::decode_field;
use crate::error::{CorruptionError, Error, Result};
use crate::storage::dict;

/// Write-context interning: novel values mint provisional ids flushed
/// at commit.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_write(tx: &mut WriteTx<'_>, value: &str) -> Result<u64> {
    tx.delta.intern_str(&tx.view, value)
}

/// Write-context interning for bytes.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_bytes_write(tx: &mut WriteTx<'_>, value: &[u8]) -> Result<u64> {
    tx.delta.intern_bytes(&tx.view, value)
}

/// Delete-context resolution: pending id, else committed id, else
/// `None` — the fact cannot exist; nothing minted.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_delete(tx: &WriteTx<'_>, value: &str) -> Result<Option<u64>> {
    tx.delta.resolve_str(&tx.view, value)
}

/// Delete-context resolution for bytes.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_bytes_delete(tx: &WriteTx<'_>, value: &[u8]) -> Result<Option<u64>> {
    tx.delta.resolve_bytes(&tx.view, value)
}

/// Read-context lookup: `None` means never interned — the fact cannot
/// exist.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_str_read(snap: &Snapshot<'_>, value: &str) -> Result<Option<u64>> {
    dict::lookup_str(&snap.txn, value)
}

/// Read-context lookup for bytes.
///
/// # Errors
///
/// Storage errors from the dictionary reads.
pub fn intern_bytes_read(snap: &Snapshot<'_>, value: &[u8]) -> Result<Option<u64>> {
    dict::lookup_bytes(&snap.txn, value)
}

/// Resolves an intern id to an owned `String` (decode boundary).
///
/// # Errors
///
/// `Corruption` on a dangling id or non-UTF-8 stored bytes.
pub fn resolve_string(snap: &Snapshot<'_>, id: u64) -> Result<String> {
    let raw = dict::resolve(&snap.txn, id, dict::TAG_STRING)?;
    String::from_utf8(raw.to_vec())
        .map_err(|_| Error::Corruption(CorruptionError::NonUtf8Intern(id)))
}

/// Resolves an intern id to owned bytes (decode boundary).
///
/// # Errors
///
/// `Corruption` on a dangling id.
pub fn resolve_bytes(snap: &Snapshot<'_>, id: u64) -> Result<Vec<u8>> {
    Ok(dict::resolve(&snap.txn, id, dict::TAG_BYTES)?.to_vec())
}

/// Appends the canonical fact bytes for a write-context encode.
pub fn encode_write_fact(
    tx: &WriteTx<'_>,
    rel: RelationId,
    values: &[ValueRef],
    out: &mut Vec<u8>,
) {
    crate::encoding::encode_fact(values, tx.schema.relation(rel).layout(), out);
}

/// Appends the canonical fact bytes for a read-context encode.
pub fn encode_read_fact(
    snap: &Snapshot<'_>,
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
pub fn decode(snap: &Snapshot<'_>, rel: RelationId, fact: &[u8], idx: usize) -> Result<ValueRef> {
    Ok(decode_field(fact, snap.schema.relation(rel).layout(), idx)?)
}
