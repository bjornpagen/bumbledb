use crate::error::Result;
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, StatKind};
use crate::storage::stored_u64;
use bumbledb_theory::schema::RelationId;

/// `S` get: the relation's exact row count — the planner's statistic.
/// Missing means no state-changing commit ever touched the relation: 0.
/// # Errors
/// `Lmdb` on storage failure, `Corruption` on a malformed counter value.
pub fn row_count(txn: &ReadTxn<'_>, rel: RelationId) -> Result<u64> {
    let key = keys::stat_key(rel, StatKind::RowCount);
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => stored_u64(bytes, "S row count"),
        None => Ok(0),
    }
}
