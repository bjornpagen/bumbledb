use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::keys;

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
