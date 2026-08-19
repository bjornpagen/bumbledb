//! The `Q` pass (finding 033): one cursor over `Q | relation | field`,
//! reconciled against the `F`-scan fresh tallies — the never-reissue
//! high-water, verified offline. Every committed fresh value must sit
//! strictly below the stored next-value (the ratchet law,
//! `docs/architecture/50-storage.md` § key layout;
//! `lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`), or
//! `reserve()` re-issues an id the host already holds. A tallied fresh
//! field with no stored entry reads as zero, exactly as the `S` pass
//! treats absent counters. The one legal exemption: an explicit
//! `u64::MAX` fresh value leaves the sequence exhausted with
//! `next == value == u64::MAX` — never convicted.

use std::collections::BTreeSet;

use crate::error::{CorruptionError, Result};
use crate::storage::catalog::CatalogRead;
use crate::storage::keys;
use bumbledb_theory::schema::{FieldId, Generation, RelationId};

use super::{StoreFinding, Sweep, for_namespace};

/// Whether a stored (or absent-as-zero) next-value fails the ratchet law
/// against the tallied maximum — the exhausted-sequence exemption
/// applied in one place. The exemption keys on the STORED value being
/// exhausted, never on the tally alone: a row holding an explicit
/// `u64::MAX` makes the tally `MAX`, but the mark advance that admitted
/// it saturated the stored next-value to `MAX` too — so any stored
/// value below `MAX` under a `MAX` tally is a genuine regression
/// (`reserve()` would re-issue every id between it and the ceiling), not
/// the legal exhausted shape `next == value == u64::MAX`.
fn ratchet_broken(stored: u64, max_fresh: u64) -> bool {
    stored != u64::MAX && stored <= max_fresh
}

pub(super) fn sweep<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<()> {
    let mut seen: BTreeSet<(RelationId, FieldId)> = BTreeSet::new();
    for_namespace(s.catalog, keys::Namespace::Fresh, |key, value| {
        let Some((rel, field)) = keys::parse_fresh_key(key) else {
            s.malformed(key, "Q key length");
            return Ok(());
        };
        let Some(relation) = s.schema.relation_checked(rel) else {
            s.malformed(key, "Q key relation");
            return Ok(());
        };
        // Closed relations appear in no namespace — the entry's very
        // existence is the finding (the F pass's exemption, mirrored).
        if relation.body().closed_rows().is_some() {
            s.corrupt(CorruptionError::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            return Ok(());
        }
        let fresh_field = relation
            .fields()
            .get(usize::from(field.0))
            .is_some_and(|descriptor| descriptor.generation == Generation::Fresh);
        if !fresh_field {
            s.malformed(key, "Q key field");
            return Ok(());
        }
        let Ok(bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "Q next value");
            return Ok(());
        };
        let stored = u64::from_le_bytes(bytes);
        seen.insert((rel, field));
        if let Some(&max_fresh) = s.max_fresh.get(&(rel, field))
            && ratchet_broken(stored, max_fresh)
        {
            s.corrupt(CorruptionError::FreshNextValueLow {
                relation: rel,
                field,
                stored,
                max_fresh,
            });
        }
        Ok(())
    })?;
    // A tallied fresh field with no stored entry reads as zero: rows on
    // disk convict the absent sequence exactly as absent S counters are
    // convicted.
    let absent: Vec<StoreFinding> = s
        .max_fresh
        .iter()
        .filter(|(spot, max_fresh)| !seen.contains(spot) && ratchet_broken(0, **max_fresh))
        .map(|(&(relation, field), &max_fresh)| {
            StoreFinding::Corruption(CorruptionError::FreshNextValueLow {
                relation,
                field,
                stored: 0,
                max_fresh,
            })
        })
        .collect();
    s.findings.extend(absent);
    Ok(())
}
