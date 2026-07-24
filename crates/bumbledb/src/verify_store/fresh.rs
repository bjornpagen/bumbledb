//! The `Q` pass (finding 033): one cursor over `Q | relation | field`,
//! reconciled against the `F`-scan fresh tallies — the never-reissue
//! high-water, verified offline. Every committed fresh value must sit
//! strictly below the stored next-value (the ratchet law,
//! `docs/architecture/50-storage.md` § key layout;
//! `lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`), or
//! `alloc()` re-issues an id the host already holds. A tallied fresh
//! field with no stored entry reads as zero, exactly as the `S` pass
//! treats absent counters. The one legal exemption: an explicit
//! `u64::MAX` fresh value leaves the sequence exhausted with
//! `next == value == u64::MAX` — never convicted.

use std::collections::BTreeSet;

use crate::error::Result;
use crate::storage::keys;
use bumbledb_theory::schema::{FieldId, Generation, RelationId};

use super::{StoreFinding, Sweep, namespace};

/// Whether a stored (or absent-as-zero) next-value fails the ratchet law
/// against the tallied maximum — the exhausted-sequence exemption
/// applied in one place.
fn ratchet_broken(stored: u64, max_fresh: u64) -> bool {
    max_fresh != u64::MAX && stored <= max_fresh
}

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let mut seen: BTreeSet<(RelationId, FieldId)> = BTreeSet::new();
    for entry in namespace(s.data, txn, keys::NS_FRESH)? {
        let (key, value) = entry?;
        let Some((rel, field)) = keys::parse_fresh_key(key) else {
            s.malformed(key, "Q key length");
            continue;
        };
        let Some(relation) = s.schema.relation_checked(rel) else {
            s.malformed(key, "Q key relation");
            continue;
        };
        // Closed relations appear in no namespace — the entry's very
        // existence is the finding (the F pass's exemption, mirrored).
        if relation.is_closed() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            continue;
        }
        let fresh_field = relation
            .fields()
            .get(usize::from(field.0))
            .is_some_and(|descriptor| descriptor.generation == Generation::Fresh);
        if !fresh_field {
            s.malformed(key, "Q key field");
            continue;
        }
        let Ok(bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "Q next value");
            continue;
        };
        let stored = u64::from_le_bytes(bytes);
        seen.insert((rel, field));
        if let Some(&max_fresh) = s.max_fresh.get(&(rel, field))
            && ratchet_broken(stored, max_fresh)
        {
            s.push(StoreFinding::FreshNextValueLow {
                relation: rel,
                field,
                stored,
                max_fresh,
            });
        }
    }
    // A tallied fresh field with no stored entry reads as zero: rows on
    // disk convict the absent sequence exactly as absent S counters are
    // convicted.
    let absent: Vec<StoreFinding> = s
        .max_fresh
        .iter()
        .filter(|(spot, max_fresh)| !seen.contains(spot) && ratchet_broken(0, **max_fresh))
        .map(
            |(&(relation, field), &max_fresh)| StoreFinding::FreshNextValueLow {
                relation,
                field,
                stored: 0,
                max_fresh,
            },
        )
        .collect();
    s.findings.extend(absent);
    Ok(())
}
