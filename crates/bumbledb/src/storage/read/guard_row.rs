use crate::error::Result;
use crate::schema::{RelationId, StatementId};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::row_id_value::row_id_value;

/// `U` probe: the row id holding a key statement's guard tuple, if any.
/// `guard` is the concatenated canonical encodings of the statement's
/// projected fields in statement projection order — the same bytes
/// [`keys::guard_bytes`] slices out of a stored fact.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn guard_row(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    statement: StatementId,
    guard: &[u8],
) -> Result<Option<u64>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::guard_key(&mut key, rel, statement, guard);
    row_id_value(txn.env().data().get(txn.raw(), &key[..len])?)
}
