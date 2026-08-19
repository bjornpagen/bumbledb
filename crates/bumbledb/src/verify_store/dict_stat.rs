//! The `_dict` pass (findings 004/078): one cursor over the reverse map
//! plus one point-get per referenced id — the sweeper's knowledge is the
//! engine's knowledge, so every corruption the runtime can convict
//! (`Corruption(DanglingInternId)`) or silently suffer (a rebound
//! forward entry, a regressed next-id arming reverse-map reuse) is a
//! finding here. Entries no live fact references stay the accepted leak
//! (`docs/architecture/50-storage.md`: dictionary entries are never
//! removed) — an informational statistic, never a finding.

use std::ops::Bound;

use crate::encoding::InternId;
use crate::error::{CorruptionError, Result};
use crate::storage::catalog::{Bounds, CatalogMap, CatalogRead, ReadCursor};
use crate::storage::dict;

use super::Sweep;

pub(super) fn dangling<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<u64> {
    let mut dangling = 0u64;
    let catalog = s.catalog;
    let lo = [dict::REVERSE];
    let hi = [dict::REVERSE + 1];
    let mut range = catalog.range(
        CatalogMap::Dictionary,
        Bounds {
            start: Bound::Included(&lo),
            end: Bound::Excluded(&hi),
        },
    )?;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        let key = entry.key;
        let raw = entry.value;
        match key.get(1..).and_then(|rest| <[u8; 8]>::try_from(rest).ok()) {
            Some(id) if key.len() == 9 && key[0] == dict::REVERSE => {
                let id = InternId::from_raw(u64::from_be_bytes(id));
                if id.is_sentinel() {
                    s.malformed(key, "dict reverse sentinel");
                    continue;
                }
                // The counter bound (078): a reverse id at or beyond the
                // `_meta` next-id is the regressed-counter state that
                // arms silent reuse — `RowIdHighWaterLow`'s dictionary
                // sibling.
                if id >= s.dict_next_id {
                    s.corrupt(CorruptionError::DictNextIdLow {
                        stored: s.dict_next_id,
                        reverse_id: id,
                    });
                }
                // Forward/reverse coherence (004): the stored raw bytes
                // must hash to a forward entry mapping back to this id —
                // one blake3 per entry, the price the M pass already
                // pays per entry. A rebound forward entry silently
                // redirects every selection literal on the string.
                let forward = s.catalog.dict_lookup(raw)?;
                if forward != Some(id) {
                    s.corrupt(CorruptionError::DictForwardDesync {
                        intern_id: id,
                        forward,
                    });
                }
                if !s.referenced_interns.contains(&id) {
                    dangling += 1;
                }
            }
            _ => s.malformed(key, "dict reverse id"),
        }
    }
    // The liveness direction (004): every id a live fact references must
    // resolve — the exact corruption the runtime types as
    // `DanglingInternId`, convicted offline instead of at the next
    // export. The F pass built the set; this is its second consumer.
    let referenced: Vec<InternId> = s.referenced_interns.iter().copied().collect();
    for id in referenced {
        if s.catalog
            .get(CatalogMap::Dictionary, &dict::reverse_key(id))?
            .is_none()
        {
            s.corrupt(CorruptionError::DanglingInternId(id));
        }
    }
    Ok(dangling)
}
