use crate::error::Result;
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use bumbledb_theory::schema::{RelationId, StatementId};

use super::row_id_value::row_id_value;

/// `U` probe: the row id holding a key statement's determinant tuple, if any.
/// `determinant` is the concatenated canonical encodings of the statement's
/// projected fields in statement projection order — the same bytes
/// [`keys::determinant_image`] slices out of a stored fact.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn determinant_row(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    statement: StatementId,
    determinant: &[u8],
) -> Result<Option<u64>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::determinant_key(&mut key, rel, statement, determinant);
    row_id_value(txn.env().data().get(txn.raw(), &key[..len])?)
}
