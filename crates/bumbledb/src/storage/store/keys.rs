//! Fixed-width physical keys for the `_core_data` namespaces.
//!
//! Row and membership keys are fixed-width. Determinant routing bytes vary
//! by compiled encoding: exact-bounded scalars (≤16 bytes) or a 16-byte
//! fingerprint, plus an optional ordered interval tail. No text or whole
//! tuple ever enters an LMDB key. Determinant entries persist
//! [`ProjectionId`], never a restated statement id of a shared index.

use super::error::{StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::RowId;
use crate::schema::ProjectionId;

/// Bounded opaque host-record key width: LMDB key limit minus namespace tag.
pub const HOST_KEY_MAX: usize = 510;

pub(crate) const TAG_ROW: u8 = 0x01;
pub(crate) const TAG_MEMBERSHIP: u8 = 0x02;
pub(crate) const TAG_DETERMINANT: u8 = 0x03;

pub(crate) const ROW_KEY_LEN: usize = 1 + 4 + 8;
pub(crate) const MEMBERSHIP_KEY_LEN: usize = 1 + 4 + FP_LEN + 8;
/// Minimum determinant key: tag + projection id + row surrogate.
/// Routing and an optional interval tail sit between those fields.
pub(crate) const DETERMINANT_KEY_MIN_LEN: usize = 1 + 2 + 8;

const PROJECTION_OFF: usize = 1;
const ROUTING_OFF: usize = 3;

pub(crate) fn row_key(relation: bumbledb_theory::schema::RelationId, row: RowId) -> [u8; ROW_KEY_LEN] {
    let mut key = [0u8; ROW_KEY_LEN];
    key[0] = TAG_ROW;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key[5..13].copy_from_slice(&row.0.to_be_bytes());
    key
}

pub(crate) fn row_prefix(relation: bumbledb_theory::schema::RelationId) -> [u8; 5] {
    let mut key = [0u8; 5];
    key[0] = TAG_ROW;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key
}

pub(crate) fn membership_key(
    relation: bumbledb_theory::schema::RelationId,
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

pub(crate) fn membership_bucket(
    relation: bumbledb_theory::schema::RelationId,
    fp: &[u8; FP_LEN],
) -> [u8; 5 + FP_LEN] {
    let mut key = [0u8; 5 + FP_LEN];
    key[0] = TAG_MEMBERSHIP;
    key[1..5].copy_from_slice(&relation.0.to_be_bytes());
    key[5..].copy_from_slice(fp);
    key
}

/// One determinant bucket prefix: tag + interned projection + routing bytes.
pub(crate) fn determinant_bucket(projection: ProjectionId, routing: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(ROUTING_OFF + routing.len());
    key.push(TAG_DETERMINANT);
    key.extend_from_slice(&projection.0.to_be_bytes());
    key.extend_from_slice(routing);
    key
}

/// One determinant index entry: bucket + optional ordered tail + row id.
pub(crate) fn determinant_key(
    projection: ProjectionId,
    routing: &[u8],
    tail: Option<&[u8]>,
    row: RowId,
) -> Vec<u8> {
    let mut key = determinant_bucket(projection, routing);
    if let Some(tail) = tail {
        key.extend_from_slice(tail);
    }
    key.extend_from_slice(&row.0.to_be_bytes());
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

pub(crate) fn row_id_from_determinant_key(key: &[u8]) -> StoreResult<RowId> {
    if key.len() < DETERMINANT_KEY_MIN_LEN || key.first() != Some(&TAG_DETERMINANT) {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "determinant key width",
        )));
    }
    let suffix: [u8; 8] = key[key.len() - 8..].try_into().map_err(|_| {
        StoreError::Corruption(StoreCorruption::MalformedKey("determinant row suffix"))
    })?;
    Ok(RowId(u64::from_be_bytes(suffix)))
}

pub(crate) fn projection_of_determinant_key(key: &[u8]) -> StoreResult<ProjectionId> {
    if key.len() < DETERMINANT_KEY_MIN_LEN || key.first() != Some(&TAG_DETERMINANT) {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "determinant key width",
        )));
    }
    Ok(ProjectionId(u16::from_be_bytes(
        key[PROJECTION_OFF..ROUTING_OFF].try_into().map_err(|_| {
            StoreError::Corruption(StoreCorruption::MalformedKey("determinant projection"))
        })?,
    )))
}

/// Routing bytes plus optional interval tail (everything between the
/// projection id and the row surrogate).
pub(crate) fn payload_of_determinant_key(key: &[u8]) -> StoreResult<&[u8]> {
    if key.len() < DETERMINANT_KEY_MIN_LEN || key.first() != Some(&TAG_DETERMINANT) {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "determinant key width",
        )));
    }
    Ok(&key[ROUTING_OFF..key.len() - 8])
}
