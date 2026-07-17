use crate::error::{CorruptionError, Error, Result};
use crate::schema::{Relation, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use bumbledb_theory::schema::RelationId;

use super::check_width::check_width;

/// The two scan sources behind one iterator type: the `F` cursor for an
/// ordinary relation, the sealed extension for a closed one — virtual
/// storage, the store holds zero closed-relation bytes
/// (`docs/architecture/50-storage.md` § virtual relations).
enum Scan<S, C> {
    Store(S),
    Closed(C),
}

impl<T, S: Iterator<Item = T>, C: Iterator<Item = T>> Iterator for Scan<S, C> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            Self::Store(iter) => iter.next(),
            Self::Closed(iter) => iter.next(),
        }
    }
}

/// One `F`-prefix cursor over a relation's live facts in `row_id` order.
/// Holes from deletes are absent keys, not tombstones — they simply do not
/// appear. A wrong-width fact yields `Err(Corruption)`; the caller is
/// expected to stop at the first error (hard error, never a skip).
///
/// A **closed** relation never touches the cursor: its facts are the
/// sealed extension's canonical bytes, yielded in declaration order (row
/// id = declaration index) straight from the theory.
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
    if let Some(extension) = schema.relation_checked(rel).and_then(Relation::extension) {
        return Ok(Scan::Closed(
            extension
                .iter()
                .enumerate()
                .map(|(row_id, row)| Ok((row_id as u64, &*row.fact))),
        ));
    }
    let mut key: KeyBuf = [0; MAX_KEY];
    let len = keys::fact_prefix(&mut key, rel);
    let iter = txn.env().data().prefix_iter(txn.raw(), &key[..len])?;
    // Fused on error: after the first corruption the iterator yields
    // nothing more — "never a skip" is structural, not a caller
    // obligation (a caller ignoring an Err cannot resume past it).
    let mut dead = false;
    Ok(Scan::Store(iter.map_while(move |entry| {
        if dead {
            return None;
        }
        let item: Result<(u64, &[u8])> = try {
            let (raw_key, bytes) = entry.map_err(Error::from)?;
            // F | relation(4) | row_id(8): fixed 13-byte shape — the
            // parser's split chain is the length check, and a
            // mis-shaped key is corruption, typed.
            let (_, row_id) = keys::parse_fact_key(raw_key).ok_or(Error::Corruption(
                CorruptionError::MalformedValue("F key length"),
            ))?;
            check_width(schema, rel, row_id, bytes)?;
            (row_id, bytes)
        };
        dead = item.is_err();
        Some(item)
    })))
}
