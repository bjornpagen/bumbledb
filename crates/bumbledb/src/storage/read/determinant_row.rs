use crate::error::Result;
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::row_id_value::row_id_value;

/// Byte width of the `U | relation | statement` header a composed
/// determinant key carries before its determinant bytes.
pub const DETERMINANT_KEY_HEADER: usize = 1 + 4 + 2;

/// Appends the `U | relation | statement` header to a caller-owned key
/// buffer — the composed-key form of [`keys::determinant_key`]: the
/// caller encodes the determinant bytes (the concatenated canonical
/// encodings of the statement's projected fields in statement projection
/// order, the same bytes [`keys::determinant_image`] slices out of a
/// stored fact) directly behind the header, so the probe zeroes no
/// oversized scratch (post-mortem §25). Byte equality with the codec's
/// writer is pinned by `composed_determinant_key_matches_the_codec`.
pub fn begin_determinant_key(out: &mut Vec<u8>, rel: RelationId, statement: StatementId) {
    out.push(keys::NS_DETERMINANT);
    out.extend_from_slice(&rel.0.to_be_bytes());
    out.extend_from_slice(&statement.0.to_be_bytes());
}

/// `U` probe over a composed key ([`begin_determinant_key`] + the
/// caller-encoded determinant bytes) — the one determinant probe: every
/// reader owns a reusable buffer, so no fixed-size variant exists to
/// re-zero one.
///
/// # Errors
///
/// `Lmdb` on storage failure, `Corruption` on a malformed row-id value.
pub fn determinant_row_for_key(txn: &ReadTxn<'_>, key: &[u8]) -> Result<Option<u64>> {
    row_id_value(txn.env().data().get(txn.raw(), key)?)
}
