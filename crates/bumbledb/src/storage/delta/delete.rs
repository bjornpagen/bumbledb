use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::env::ReadTxn;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records a delete. Returns whether the final state changes (an
    /// idempotent no-op if the fact is absent).
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed membership probe.
    pub fn delete(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<bool> {
        let hash = fact_hash(fact_bytes);
        let changed = self.present(view, rel, &hash)?;
        if changed {
            let slice = self.arena.alloc(fact_bytes);
            self.facts.insert((rel, hash), (slice, Disposition::Delete));
            *self.row_count_delta.entry(rel).or_insert(0) -= 1;
        }
        Ok(changed)
    }
}
