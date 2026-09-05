//! Streaming charged named-map append: the one production write visitor.
//! Callers stream rows here; they do not collect a whole-stage `Vec`.

use super::{
    charge, entry_retained, work_error, ScratchMapId, ScratchRelation, ScratchWriteBatch,
    SPILL_BATCH,
};
use crate::error::Result;
use crate::work::ByteReservation;

/// Streaming charged append onto an existing [`ScratchRelation`].
///
/// Each [`Self::append`] reserves before owning another staged copy and
/// flushes a bounded batch onto the substrate. Failure returns immediately;
/// Drop aborts the uncommitted tail. There is no infallible callback twin.
pub struct ScratchAppend<'a> {
    relation: &'a mut ScratchRelation,
    batch: ScratchWriteBatch,
    staging: Vec<ByteReservation>,
    staging_bytes: usize,
    staging_charged: usize,
}

impl<'a> ScratchAppend<'a> {
    /// Bind the visitor to a live relation (and its execute ledger).
    #[must_use]
    pub fn new(relation: &'a mut ScratchRelation) -> Self {
        Self {
            relation,
            batch: ScratchWriteBatch::new(),
            staging: Vec::new(),
            staging_bytes: 0,
            staging_charged: 0,
        }
    }

    /// Charge, then stage one named-map write. Flushes a bounded batch
    /// (same size as the RAM→disk copy). A refusal does not keep collecting.
    ///
    /// # Errors
    /// Stopped work, reservation refusal, or scratch I/O. The failing
    /// entry is not retained; call [`Self::finish`] only after a complete
    /// stream. Drop aborts the uncommitted tail.
    pub fn append(&mut self, map: ScratchMapId, key: &[u8], value: &[u8]) -> Result<()> {
        self.reserve_staging(entry_retained(key, value))?;
        self.batch.put(map, key, value)?;
        if self.batch.pending_entries() as usize >= SPILL_BATCH {
            self.flush()?;
        }
        Ok(())
    }

    /// Commit the last partial batch. Consume this visitor after a
    /// successful stream; do not call after a failed [`Self::append`].
    ///
    /// # Errors
    /// As [`ScratchWriteBatch::commit`].
    pub fn finish(mut self) -> Result<()> {
        self.flush()
    }

    fn reserve_staging(&mut self, grown: usize) -> Result<()> {
        self.relation.work.step(1).map_err(work_error)?;
        charge(
            &self.relation.work,
            &mut self.staging_bytes,
            &mut self.staging_charged,
            &mut self.staging,
            grown,
        )
    }

    fn flush(&mut self) -> Result<()> {
        let batch = std::mem::replace(&mut self.batch, ScratchWriteBatch::new());
        let result = if batch.pending_entries() == 0 {
            Ok(())
        } else {
            batch.commit(self.relation)
        };
        self.staging.clear();
        self.staging_bytes = 0;
        self.staging_charged = 0;
        result
    }
}
