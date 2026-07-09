//! The `_dict` pass: one cursor over the reverse map, counting entries no
//! live fact references — the accepted leak
//! (`docs/architecture/50-storage.md`: dictionary entries are never
//! removed). An informational statistic, never a finding.

use crate::error::Result;
use crate::storage::dict::{self, ReverseId};

use super::Sweep;

pub(super) fn dangling(s: &mut Sweep<'_, '_>) -> Result<u64> {
    let mut dangling = 0u64;
    for entry in dict::reverse_ids(s.txn)? {
        match entry? {
            ReverseId::Id(id) => {
                if !s.referenced_interns.contains(&id) {
                    dangling += 1;
                }
            }
            ReverseId::Malformed(key) => s.malformed(key, "dict reverse id"),
        }
    }
    Ok(dangling)
}
