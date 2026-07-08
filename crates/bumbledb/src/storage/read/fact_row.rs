use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::env::ReadTxn;
use crate::storage::keys;

use super::row_id_value::row_id_value;

/// `M` probe: the row id of a fact, if it is live.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn fact_row(txn: &ReadTxn<'_>, rel: RelationId, fact_bytes: &[u8]) -> Result<Option<u64>> {
    fact_row_by_hash(txn, rel, &fact_hash(fact_bytes))
}

/// `M` probe by a caller-computed hash — the delta already hashed the fact
/// for its own map key; blake3 is the record path's most expensive CPU
/// step and must not run twice.
///
/// # Errors
///
/// As [`fact_row`].
pub fn fact_row_by_hash(
    txn: &ReadTxn<'_>,
    rel: RelationId,
    hash: &[u8; 32],
) -> Result<Option<u64>> {
    // Right-sized stack buffer: this probe runs once per user write
    // operation — zeroing 511 bytes for a 37-byte key was measurable
    // waste (the codec header promises no oversized zeroing).
    let mut key = [0u8; keys::MEMBERSHIP_KEY_LEN];
    let len = keys::membership_key(&mut key, rel, hash);
    debug_assert_eq!(len, key.len());
    row_id_value(txn.env().data().get(txn.raw(), &key)?)
}
