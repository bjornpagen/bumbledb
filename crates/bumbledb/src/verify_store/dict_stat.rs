//! The `_dict` pass (findings 004/078): one cursor over the reverse map
//! plus one point-get per referenced id — the sweeper's knowledge is the
//! engine's knowledge, so every corruption the runtime can convict
//! (`Corruption(DanglingInternId)`) or silently suffer (a rebound
//! forward entry, a regressed next-id arming reverse-map reuse) is a
//! finding here. Entries no live fact references stay the accepted leak
//! (`docs/architecture/50-storage.md`: dictionary entries are never
//! removed) — an informational statistic, never a finding.

use crate::error::Result;
use crate::storage::dict::{self, ReverseEntry};

use super::{StoreFinding, Sweep};

pub(super) fn dangling(s: &mut Sweep<'_, '_>) -> Result<u64> {
    let mut dangling = 0u64;
    for entry in dict::reverse_entries(s.txn)? {
        match entry? {
            ReverseEntry::Id(id, raw) => {
                // The counter bound (078): a reverse id at or beyond the
                // `_meta` next-id is the regressed-counter state that
                // arms silent reuse — `RowIdHighWaterLow`'s dictionary
                // sibling.
                if id >= s.dict_next_id {
                    s.push(StoreFinding::DictNextIdLow {
                        stored: s.dict_next_id,
                        reverse_id: id,
                    });
                }
                // Forward/reverse coherence (004): the stored raw bytes
                // must hash to a forward entry mapping back to this id —
                // one blake3 per entry, the price the M pass already
                // pays per entry. A rebound forward entry silently
                // redirects every selection literal on the string.
                let forward = dict::lookup(s.txn, raw)?;
                if forward != Some(id) {
                    s.push(StoreFinding::DictForwardDesync {
                        intern_id: id,
                        forward,
                    });
                }
                if !s.referenced_interns.contains(&id) {
                    dangling += 1;
                }
            }
            ReverseEntry::Malformed(key) => s.malformed(key, "dict reverse id"),
        }
    }
    // The liveness direction (004): every id a live fact references must
    // resolve — the exact corruption the runtime types as
    // `DanglingInternId`, convicted offline instead of at the next
    // export. The F pass built the set; this is its second consumer.
    for &id in &s.referenced_interns {
        if !dict::has_reverse(s.txn, id)? {
            s.findings
                .push(StoreFinding::DanglingInternId { intern_id: id });
        }
    }
    Ok(dangling)
}
