use crate::encoding::FactView;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

use super::check_width::check_width;

/// `F` get: the canonical bytes of the fact at `row_id`, borrowed from the
/// LMDB page, width-proved against the relation layout.
/// # Errors
/// `Corruption(MissingFact)` when the row is absent — a row id obtained
/// from `M`/`U` in the same snapshot must resolve; `Corruption
/// (WrongFactWidth)` when the stored value does not match the schema's
/// fact width. Never a skip.
pub fn fetch<'txn, 's>(
    txn: &'txn ReadTxn<'_>,
    schema: &'s Schema,
    rel: RelationId,
    row_id: u64,
) -> Result<FactView<'txn, 's>> {
    let key = keys::fact_key(rel, row_id);
    let bytes = txn
        .env()
        .data()
        .get(txn.raw(), &key)?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation: rel,
            row_id,
        }))?;
    check_width(schema, rel, row_id, bytes)
}

/// The committed point-read leg over a composed determinant key
/// ([`super::begin_determinant_key`] + determinant bytes): `U` probe →
/// `F` fetch — one body behind `ReadInstance::{get, get_dyn}` and
/// `WriteTx`'s committed arm.
/// # Errors
/// As [`fetch`], plus `Corruption` on a malformed `U` row-id value.
pub fn fact_for_key<'txn, 's>(
    txn: &'txn ReadTxn<'_>,
    schema: &'s Schema,
    rel: RelationId,
    key: &[u8],
) -> Result<Option<FactView<'txn, 's>>> {
    match super::determinant_row::determinant_row_for_key(txn, key)? {
        Some(row_id) => fetch(txn, schema, rel, row_id).map(Some),
        None => Ok(None),
    }
}

/// `F` get by row id, missing honestly — the fresh-row point probe: a fresh-keyed determinant IS the row id, the
/// auto-key maintains no `U` tree, so the probe reads `F` directly —
/// one B-tree descent. Absence is a miss, never corruption: no index
/// entry witnessed the row (contrast [`fetch`], whose row id came from
/// `M`/`U` in the same snapshot).
/// # Errors
/// `Lmdb` on storage failure; `Corruption(WrongFactWidth)` on a stored
/// fact not matching the schema's width.
pub fn fact_at<'txn, 's>(
    txn: &'txn ReadTxn<'_>,
    schema: &'s Schema,
    rel: RelationId,
    row_id: u64,
) -> Result<Option<FactView<'txn, 's>>> {
    let key = keys::fact_key(rel, row_id);
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => check_width(schema, rel, row_id, bytes).map(Some),
        None => Ok(None),
    }
}
