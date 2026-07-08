use crate::error::{CorruptionError, Error, Result};
use crate::schema::RelationId;
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, StatKind};

/// `S` get: the relation's exact row count — the planner's statistic.
/// Missing means no state-changing commit ever touched the relation: 0.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed counter value.
pub fn row_count(txn: &ReadTxn<'_>, rel: RelationId) -> Result<u64> {
    let mut key = [0u8; keys::STAT_KEY_LEN];
    let len = keys::stat_key(&mut key, rel, StatKind::RowCount);
    debug_assert_eq!(len, key.len());
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            Error::Corruption(CorruptionError::MalformedValue("S row count"))
        })?)),
        None => Ok(0),
    }
}
