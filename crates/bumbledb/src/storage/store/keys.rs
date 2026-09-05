//! Fixed-width physical keys for the `_core_data` namespaces.
//!
//! Every key here is bounded and tiny (≤ 29 bytes): no text, no determinant
//! bytes, no whole tuple ever enters an LMDB key, so LMDB's key-size limit
//! (511 bytes for the pinned build) cannot truncate application data. Long
//! logical keys live in row values and are compared exactly inside their
//! fingerprint bucket.
//!
//! Namespaces (first byte):
//!
//! | Tag | Key | Value |
//! | --- | --- | --- |
//! | `0x01` row | `[tag, relation u32, row u64]` | canonical row bytes |
//! | `0x02` membership | `[tag, relation u32, fp 16, row u64]` | empty |
//! | `0x03` determinant | `[tag, statement u16, fp 16, row u64]` | empty |
//!
//! The membership index moves the local row id into the key (chapter 41):
//! all colliding rows are enumerable and individually deletable, and the
//! same fingerprint in two relations can never alias. The determinant
//! namespace is a deliberate multimap — semantic uniqueness is judged, not
//! enforced by key shape.

use bumbledb_theory::schema::{RelationId, StatementId};

use super::error::{StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::RowId;

/// Bounded opaque host-record key width: the pinned LMDB build's usual
/// 511-byte key limit minus the one-byte namespace tag.
pub const HOST_KEY_MAX: usize = 510;

pub(crate) const TAG_ROW: u8 = 0x01;
pub(crate) const TAG_MEMBERSHIP: u8 = 0x02;
pub(crate) const TAG_DETERMINANT: u8 = 0x03;

pub(crate) const ROW_KEY_LEN: usize = 1 + 4 + 8;
pub(crate) const MEMBERSHIP_KEY_LEN: usize = 1 + 4 + FP_LEN + 8;
pub(crate) const DETERMINANT_KEY_LEN: usize = 1 + 2 + FP_LEN + 8;

pub(crate) fn row_key(relation: RelationId, row: RowId) -> [u8; ROW_KEY_LEN] {
    let mut key = [0u8; ROW_KEY_LEN];
    key[0] = TAG_ROW;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key[5..13].copy_from_slice(&row.0.to_be_bytes());
    key
}

pub(crate) fn row_prefix(relation: RelationId) -> [u8; 5] {
    let mut key = [0u8; 5];
    key[0] = TAG_ROW;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key
}

pub(crate) fn membership_key(
    relation: RelationId,
    fp: &[u8; FP_LEN],
    row: RowId,
) -> [u8; MEMBERSHIP_KEY_LEN] {
    let mut key = [0u8; MEMBERSHIP_KEY_LEN];
    key[0] = TAG_MEMBERSHIP;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key[5..5 + FP_LEN].copy_from_slice(fp);
    key[5 + FP_LEN..].copy_from_slice(&row.0.to_be_bytes());
    key
}

/// One collision bucket: every membership entry sharing this prefix is a
/// candidate whose full canonical bytes must be compared.
pub(crate) fn membership_bucket(relation: RelationId, fp: &[u8; FP_LEN]) -> [u8; 5 + FP_LEN] {
    let mut key = [0u8; 5 + FP_LEN];
    key[0] = TAG_MEMBERSHIP;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key[5..].copy_from_slice(fp);
    key
}

pub(crate) fn determinant_key(
    statement: StatementId,
    fp: &[u8; FP_LEN],
    row: RowId,
) -> [u8; DETERMINANT_KEY_LEN] {
    let mut key = [0u8; DETERMINANT_KEY_LEN];
    key[0] = TAG_DETERMINANT;
    key[1..3].copy_from_slice(&statement.0.to_be_bytes());
    key[3..3 + FP_LEN].copy_from_slice(fp);
    key[3 + FP_LEN..].copy_from_slice(&row.0.to_be_bytes());
    key
}

pub(crate) fn determinant_bucket(statement: StatementId, fp: &[u8; FP_LEN]) -> [u8; 3 + FP_LEN] {
    let mut key = [0u8; 3 + FP_LEN];
    key[0] = TAG_DETERMINANT;
    key[1..3].copy_from_slice(&statement.0.to_be_bytes());
    key[3..].copy_from_slice(fp);
    key
}

pub(crate) fn row_id_from_suffix(key: &[u8], expected_len: usize) -> StoreResult<RowId> {
    if key.len() != expected_len {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "index key width",
        )));
    }
    let suffix: [u8; 8] = key[expected_len - 8..].try_into().map_err(|_| {
        StoreError::Corruption(StoreCorruption::MalformedKey("index key row suffix"))
    })?;
    Ok(RowId(u64::from_be_bytes(suffix)))
}

pub(crate) fn row_id_from_row_key(key: &[u8]) -> StoreResult<RowId> {
    row_id_from_suffix(key, ROW_KEY_LEN)
}
