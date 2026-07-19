use crate::error::Result;
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, StatKind};
use crate::storage::stored_u64;
use bumbledb_theory::schema::RelationId;

/// `S` get: the relation's row-id high-water — the next row id the commit
/// pipeline would assign (`storage/commit/applier.rs::next_row_id`; missing
/// means no row was ever committed: 0). Monotone across committed states
/// forever — deletes never touch it, and the sweeper convicts a store
/// where a live row id reaches it (`RowIdHighWaterLow`) — so every row
/// visible in this snapshot has id strictly below the returned value, and
/// every row a later commit adds has id at or above it. That boundary is
/// what the image append path scans from ([`crate::image::append`]): read
/// in the same transaction as the image build, it is snapshot-consistent
/// by construction.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed counter value.
pub fn row_id_high_water(txn: &ReadTxn<'_>, rel: RelationId) -> Result<u64> {
    let mut key = [0u8; keys::STAT_KEY_LEN];
    let len = keys::stat_key(&mut key, rel, StatKind::RowIdHighWater);
    debug_assert_eq!(len, key.len());
    match txn.env().data().get(txn.raw(), &key)? {
        Some(bytes) => stored_u64(bytes, "S row-id high-water"),
        None => Ok(0),
    }
}
