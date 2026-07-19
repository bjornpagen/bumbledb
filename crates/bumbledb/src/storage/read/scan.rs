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
/// NOT delegated to [`scan_from`]`(rel, 0)` (the cleanup-0.5.0 kill-6
/// sketch, aborted-with-reason at this site): the prefix cursor and the
/// range cursor differ observably on corrupt keys — a bare `F | rel`
/// prefix key sorts before `fact_key(rel, 0)`, so the range cursor
/// would silently skip what the audit pin requires this cursor to
/// convict (`read/tests.rs: a_short_f_key_is_typed_corruption_from_scan`).
/// The shared meaning — the per-entry parse, width check, and fuse — is
/// already one body ([`parse_facts`]); the two cursor-opens encode
/// different corruption envelopes. The row-level agreement over
/// well-formed keys is pinned
/// (`scan_from_zero_yields_exactly_scan_over_live_facts`).
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
    Ok(Scan::Store(parse_facts(schema, rel, iter)))
}

/// [`scan`]'s suffix sibling: the same `F` cursor, opened at
/// `fact_key(rel, from_row_id)` instead of the prefix start — the image
/// append path's tail scan (`docs/architecture/50-storage.md` § the
/// image cache). Row ids are the monotone high-water allocator's, so a
/// scan from a base image's build-time high-water
/// ([`super::row_id_high_water`], read in this same transaction) yields
/// exactly the rows committed after that base. The range's upper bound
/// is `fact_key(rel, u64::MAX)` inclusive — every 13-byte key in between
/// shares the `F | rel` prefix by byte order, and any longer key inside
/// the bounds is a mis-shaped `F` key, typed corruption exactly as in
/// [`scan`].
///
/// Ordinary relations only: a closed relation's image synthesizes from
/// the theory and is never appended to (the cache branches before either
/// scan).
///
/// # Errors
///
/// As [`scan`]: `Lmdb` on cursor-open failure; per-item `Corruption` on
/// a mis-shaped key or wrong-width fact, fused on the first error.
pub fn scan_from<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &'txn Schema,
    rel: RelationId,
    from_row_id: u64,
) -> Result<impl Iterator<Item = Result<(u64, &'txn [u8])>>> {
    debug_assert!(
        schema
            .relation_checked(rel)
            .and_then(Relation::extension)
            .is_none(),
        "closed relations synthesize from the theory and never append"
    );
    let mut lo = [0u8; keys::FACT_KEY_LEN];
    let lo_len = keys::fact_key(&mut lo, rel, from_row_id);
    debug_assert_eq!(lo_len, lo.len());
    let mut hi = [0u8; keys::FACT_KEY_LEN];
    let hi_len = keys::fact_key(&mut hi, rel, u64::MAX);
    debug_assert_eq!(hi_len, hi.len());
    let bounds: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>) = (
        std::ops::Bound::Included(&lo[..]),
        std::ops::Bound::Included(&hi[..]),
    );
    let iter = txn.env().data().range(txn.raw(), &bounds)?;
    Ok(parse_facts(schema, rel, iter))
}

/// The shared per-entry parse behind [`scan`] and [`scan_from`]: the
/// fixed 13-byte `F` key shape, the fact-width check, and the fuse.
/// Fused on error: after the first corruption the iterator yields
/// nothing more — "never a skip" is structural, not a caller
/// obligation (a caller ignoring an Err cannot resume past it).
fn parse_facts<'txn>(
    schema: &'txn Schema,
    rel: RelationId,
    iter: impl Iterator<Item = std::result::Result<(&'txn [u8], &'txn [u8]), heed::Error>>,
) -> impl Iterator<Item = Result<(u64, &'txn [u8])>> {
    let mut dead = false;
    iter.map_while(move |entry| {
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
    })
}
