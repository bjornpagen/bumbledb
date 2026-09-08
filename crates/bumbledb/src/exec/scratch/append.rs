//! Streaming named-map append: the one production write visitor.
//! Callers stream rows here; they do not collect a whole-stage `Vec`.

use super::{SPILL_BATCH, ScratchMapId, ScratchRelation, ScratchWriteBatch, work_error};
use crate::error::Result;

/// Streaming append onto an existing [`ScratchRelation`].
///
/// Each [`Self::append`] stages one write and flushes a bounded batch onto
/// the substrate. Failure returns immediately;
/// Drop aborts the uncommitted tail. There is no infallible callback twin.
pub struct ScratchAppend<'a> {
    relation: &'a mut ScratchRelation,
    batch: ScratchWriteBatch,
}

impl<'a> ScratchAppend<'a> {
    /// Bind the visitor to a live relation and its cancellation context.
    #[must_use]
    pub fn new(relation: &'a mut ScratchRelation) -> Self {
        Self {
            relation,
            batch: ScratchWriteBatch::new(),
        }
    }

    /// Stage one named-map write. Flushes a bounded batch
    /// (same size as the RAM→disk copy). A refusal does not keep collecting.
    ///
    /// # Errors
    /// Cancellation, allocation failure, or scratch I/O. The failing
    /// entry is not retained; call [`Self::finish`] only after a complete
    /// stream. Drop aborts the uncommitted tail.
    pub fn append(&mut self, map: ScratchMapId, key: &[u8], value: &[u8]) -> Result<()> {
        self.relation.work.checkpoint().map_err(work_error)?;
        self.batch.put(map, key, value)?;
        if self.batch.pending_entries() >= usize::from(SPILL_BATCH) {
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

    fn flush(&mut self) -> Result<()> {
        let batch = std::mem::take(&mut self.batch);
        if batch.pending_entries() == 0 {
            Ok(())
        } else {
            batch.commit(self.relation)
        }
    }
}
