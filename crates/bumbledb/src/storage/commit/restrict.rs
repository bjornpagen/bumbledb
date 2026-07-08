use std::collections::BTreeSet;

use crate::error::{CorruptionError, Error, FkViolation, Result};
use crate::obs;
use crate::schema::{ConstraintId, RelationId};
use crate::storage::env::WriteTxn;
use crate::storage::keys::{self, KeyBuf};

/// Phase 3b: Restrict — every unique key deleted in phase 1 and not
/// re-established in phase 2 must have no remaining referrer. "No committed
/// state contains a dangling reference": deleting a target and all its
/// referrers in one transaction passes, as it should.
pub(super) fn check_restrict(
    txn: &WriteTxn<'_>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    key: &mut KeyBuf,
    deleted_guards: &BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    inserted_guards: &BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
) -> Result<()> {
    let mut scanned = 0u64;
    let mut restrict_span = obs::span(obs::names::FK_RESTRICT, obs::Category::Commit);
    for (rel, cid, guard) in deleted_guards.difference(inserted_guards) {
        scanned += 1;
        let p_len = keys::restrict_prefix(key, *rel, *cid, guard);
        let mut iter = data.prefix_iter(txn.raw(), &key[..p_len])?;
        if let Some(entry) = iter.next() {
            let (surviving_key, ()) = entry.map(|(k, _)| (k, ()))?;
            // R | target_rel | constraint | guard | source_rel | source_row:
            // the referencing side is the 12 bytes after the prefix. A
            // scanned key of any other length is corrupt data, not a
            // programmer error — typed, so the commit aborts cleanly.
            if surviving_key.len() != p_len + 12 {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "R key length",
                )));
            }
            let tail = &surviving_key[p_len..];
            let source_relation = RelationId(u32::from_be_bytes(
                tail[..4].try_into().expect("length checked above"),
            ));
            let source_row =
                u64::from_be_bytes(tail[4..].try_into().expect("length checked above"));
            // Fetch the referrer's fact bytes inside the still-open txn:
            // errors name facts, never storage row ids
            // (docs/architecture/10-data-model.md). Cold path — the fetch
            // costs one get on an aborting commit.
            drop(iter);
            let f_len = keys::fact_key(key, source_relation, source_row);
            let fact_bytes: Box<[u8]> = data
                .get(txn.raw(), &key[..f_len])?
                .ok_or(Error::Corruption(CorruptionError::MissingFact {
                    relation: source_relation,
                    row_id: source_row,
                }))?
                .into();
            return Err(Error::ForeignKeyViolation {
                relation: *rel,
                constraint: *cid,
                violation: FkViolation::RemainingReference {
                    source_relation,
                    fact_bytes,
                },
            });
        }
    }
    restrict_span.set_args(scanned, 0);
    restrict_span.end();
    Ok(())
}
