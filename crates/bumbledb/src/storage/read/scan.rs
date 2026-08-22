use crate::encoding::FactView;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use bumbledb_theory::schema::RelationId;

use super::check_width::check_width;

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
/// appear. A wrong-width fact yields `Err(Corruption)`; the caller is
/// expected to stop at the first error (hard error, never a skip).
/// A **closed** relation never touches the cursor: its facts are the
/// sealed extension's canonical bytes, yielded in declaration order (row
/// id = declaration index) straight from the theory.
/// NOT delegated to [`scan_from`]`(rel, 0)` (the cleanup-0.5.0 kill-6
/// sketch, aborted-with-reason at this site): the prefix cursor and the
/// Holes from deletes are absent keys, not tombstones — they simply do not
/// # Errors
/// `Lmdb` on cursor-open failure; per-item `Corruption` on an `F` key
/// that is not the codec's fixed 13-byte shape — a corrupt key is data,
/// never a panic.
pub fn scan<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &'txn Schema,
    rel: RelationId,
) -> Result<impl Iterator<Item = Result<(u64, FactView<'txn, 'txn>)>>> {
    if let Some(extension) = schema
        .relation_checked(rel)
        .and_then(|r| r.body().closed_rows())
    {
        let layout = schema.relation(rel).layout();
        return Ok(Scan::Closed(extension.iter().enumerate().map(
            move |(row_id, row)| Ok((row_id as u64, layout.encoded(&row.fact))),
        )));
    }
    let mut key: KeyBuf = [0; MAX_KEY];
    let prefix = keys::fact_prefix(&mut key, rel);
    let iter = txn.env().data().prefix_iter(txn.raw(), prefix)?;
    Ok(Scan::Store(parse_facts(schema, rel, iter)))
}

/// [`scan`]'s suffix sibling: the same `F` cursor, opened at
/// `fact_key(rel, from_row_id)` instead of the prefix start — the image
/// append path's tail scan. Row ids are the monotone high-water allocator's, so a
/// scan from a base image's build-time high-water
/// ([`crate::storage::catalog::CatalogRead::row_id_high_water`], read in
/// this same transaction) yields
/// is `fact_key(rel, u64::MAX)` inclusive — every 13-byte key in between
/// shares the `F | rel` prefix by byte order, and any longer key inside
/// exactly the rows committed after that base. The range's upper bound
/// the theory and is never appended to (the cache branches before either
/// # Errors
/// As [`scan`]: `Lmdb` on cursor-open failure; per-item `Corruption` on
/// a mis-shaped key or wrong-width fact, fused on the first error.
#[cfg_attr(not(test), allow(dead_code))]
pub fn scan_from<'txn>(
    txn: &'txn ReadTxn<'_>,
    schema: &'txn Schema,
    rel: RelationId,
    from_row_id: u64,
) -> Result<impl Iterator<Item = Result<(u64, FactView<'txn, 'txn>)>>> {
    debug_assert!(
        schema
            .relation_checked(rel)
            .and_then(|r| r.body().closed_rows())
            .is_none(),
        "closed relations synthesize from the theory and never append"
    );
    let lo = keys::fact_key(rel, from_row_id);
    let hi = keys::fact_key(rel, u64::MAX);
    let bounds: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>) = (
        std::ops::Bound::Included(&lo),
        std::ops::Bound::Included(&hi),
    );
    let iter = txn.env().data().range(txn.raw(), &bounds)?;
    Ok(parse_facts(schema, rel, iter))
}

/// Fused on error: after the first corruption the iterator yields nothing more
/// — "never a skip" is structural, not a caller obligation (a caller ignoring
/// an Err cannot resume past it).
fn parse_facts<'txn>(
    schema: &'txn Schema,
    rel: RelationId,
    iter: impl Iterator<Item = std::result::Result<(&'txn [u8], &'txn [u8]), heed::Error>>,
) -> impl Iterator<Item = Result<(u64, FactView<'txn, 'txn>)>> {
    let mut dead = false;
    iter.map_while(move |entry| {
        if dead {
            return None;
        }
        let item: Result<(u64, FactView<'txn, 'txn>)> = try {
            let (raw_key, bytes) = entry.map_err(Error::from)?;

            let (_, row_id) = keys::parse_fact_key(raw_key).ok_or(Error::Corruption(
                CorruptionError::MalformedValue("F key length"),
            ))?;
            (row_id, check_width(schema, rel, row_id, bytes)?)
        };
        dead = item.is_err();
        Some(item)
    })
}
