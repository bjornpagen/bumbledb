use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::env::ReadTxn;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records a delete, netted against committed state
    /// (docs/architecture/50-storage.md): absent + no pending entry is a
    /// redundant delete and records nothing; a pending `Insert` proves the
    /// fact committed-absent, so the delete *cancels* it (net no-op — the
    /// entry is removed, never overwritten); only a genuinely committed
    /// fact records `Delete`. Returns whether the final state changes.
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
        match self
            .facts
            .get(&(rel, hash))
            .map(|(_, disposition)| *disposition)
        {
            Some(Disposition::Delete) => Ok(false),
            Some(Disposition::Insert) => {
                // The pending Insert proves the fact committed-absent:
                // cancel it.
                self.facts.remove(&(rel, hash));
                self.record_determinants(rel, fact_bytes, None);
                *self.row_count_delta.entry(rel).or_insert(0) -= 1;
                Ok(true)
            }
            None => {
                if crate::storage::read::fact_row_by_hash(view, rel, &hash)?.is_none() {
                    return Ok(false); // absent: a redundant delete.
                }
                let slice = self.arena.alloc(fact_bytes);
                self.facts.insert((rel, hash), (slice, Disposition::Delete));
                self.record_determinants(rel, fact_bytes, None);
                *self.row_count_delta.entry(rel).or_insert(0) -= 1;
                Ok(true)
            }
        }
    }
}
