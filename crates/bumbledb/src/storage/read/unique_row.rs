use crate::error::Result;
use crate::schema::{ConstraintId, RelationId};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::row_id_value::row_id_value;

/// `U` probe: the row id holding a unique key, if any. `key_bytes` is the
/// concatenated canonical encodings of the constrained fields in constraint
/// field order.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn unique_row(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    constraint: ConstraintId,
    key_bytes: &[u8],
) -> Result<Option<u64>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::unique_key(&mut key, rel, constraint, key_bytes);
    row_id_value(txn.env().data().get(txn.raw(), &key[..len])?)
}
