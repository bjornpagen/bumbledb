use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

use super::check_width::check_width;

/// One `F`-prefix cursor over a relation's live facts in `row_id` order.
/// Holes from deletes are absent keys, not tombstones — they simply do not
/// appear. A wrong-width fact yields `Err(Corruption)`; the caller is
/// expected to stop at the first error (hard error, never a skip).
///
/// # Errors
///
/// `Lmdb` on cursor-open failure; per-item `Corruption` on an `F` key
/// that is not the codec's fixed 13-byte shape — a corrupt key is data,
/// never a panic.
pub fn scan<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &'txn Schema,
    rel: RelationId,
) -> Result<impl Iterator<Item = Result<(u64, &'txn [u8])>>> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::fact_prefix(&mut key, rel);
    let iter = txn.env().data().prefix_iter(txn.raw(), &key[..len])?;
    // Fused on error: after the first corruption the iterator yields
    // nothing more — "never a skip" is structural, not a caller
    // obligation (a caller ignoring an Err cannot resume past it).
    let mut dead = false;
    Ok(iter.map_while(move |entry| {
        if dead {
            return None;
        }
        let item = (|| {
            let (raw_key, bytes) = entry?;
            // F | relation(4) | row_id(8): fixed 13-byte shape, checked
            // before slicing — a short key is corruption, typed.
            if raw_key.len() != keys::FACT_KEY_LEN {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "F key length",
                )));
            }
            let row_id = u64::from_be_bytes(raw_key[5..].try_into().expect("length checked above"));
            check_width(schema, rel, row_id, bytes)?;
            Ok((row_id, bytes))
        })();
        dead = item.is_err();
        Some(item)
    }))
}
