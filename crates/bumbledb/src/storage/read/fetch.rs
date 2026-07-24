use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

use super::check_width::check_width;

/// `F` get: the canonical bytes of the fact at `row_id`, borrowed from the
/// LMDB page.
///
/// # Errors
///
/// `Corruption(MissingFact)` when the row is absent — a row id obtained
/// from `M`/`U` in the same snapshot must resolve; `Corruption
/// (WrongFactWidth)` when the stored value does not match the schema's
/// fact width. Never a skip.
pub fn fetch<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    row_id: u64,
) -> Result<&'txn [u8]> {
    let mut key = [0u8; keys::FACT_KEY_LEN];
    let len = keys::fact_key(&mut key, rel, row_id);
    debug_assert_eq!(len, key.len());
    let bytes = txn
        .env()
        .data()
        .get(txn.raw(), &key[..len])?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation: rel,
            row_id,
        }))?;
    check_width(schema, rel, row_id, bytes)?;
    Ok(bytes)
}

/// The committed point-read leg over a composed determinant key
/// ([`super::begin_determinant_key`] + determinant bytes): `U` probe →
/// `F` fetch — one body behind `Snapshot::{get, get_dyn}` and
/// `WriteTx`'s committed arm.
///
/// # Errors
///
/// As [`fetch`], plus `Corruption` on a malformed `U` row-id value.
pub fn fact_for_key<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    key: &[u8],
) -> Result<Option<&'txn [u8]>> {
    match super::determinant_row::determinant_row_for_key(txn, key)? {
        Some(row_id) => fetch(txn, schema, rel, row_id).map(Some),
        None => Ok(None),
    }
}

/// `F` get by row id, missing honestly — the fresh-row point probe (the
/// one id allocator, `docs/architecture/50-storage.md` § key layout;
/// ruled 2026-07-23, R16): a fresh-keyed determinant IS the row id, the
/// auto-key maintains no `U` tree, so the probe reads `F` directly —
/// one B-tree descent. Absence is a miss, never corruption: no index
/// entry witnessed the row (contrast [`fetch`], whose row id came from
/// `M`/`U` in the same snapshot).
///
/// # Errors
///
/// `Lmdb` on storage failure; `Corruption(WrongFactWidth)` on a stored
/// fact not matching the schema's width.
pub fn fact_at<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    row_id: u64,
) -> Result<Option<&'txn [u8]>> {
    let mut key = [0u8; keys::FACT_KEY_LEN];
    let len = keys::fact_key(&mut key, rel, row_id);
    debug_assert_eq!(len, key.len());
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => {
            check_width(schema, rel, row_id, bytes)?;
            Ok(Some(bytes))
        }
        None => Ok(None),
    }
}
