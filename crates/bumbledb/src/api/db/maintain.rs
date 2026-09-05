//! Maintenance surface: compaction, size and generation reads.

use std::path::Path;

use super::{Db, embedded_work};
use crate::error::{Error, Result};
use crate::storage::GenerationId;
use crate::storage::store::{MapPolicy, Store, UnindexedRows};

impl<S> Db<S> {
    /// Compact into a fresh store at `dest` (which must not exist): one
    /// coherent source snapshot supplies rows, host records, attachment and
    /// generation together (ENG-003 by construction), and the destination
    /// adopts them in one durable transaction — a crash leaves `dest`
    /// absent, empty-staged, or complete.
    /// # Errors
    /// `DestinationExists`, storage failure, or stopped work.
    pub fn compact(&self, dest: &Path) -> Result<()> {
        let work = embedded_work()?;
        let snapshot = self.store.snapshot(&work).map_err(Error::from_store)?;
        let policy = MapPolicy::default();
        let target =
            Store::create(dest, self.schema.as_ref(), policy).map_err(Error::from_store)?;
        target
            .adopt_snapshot(&snapshot, &UnindexedRows, &work)
            .map_err(Error::from_store)?;
        drop(target);
        crate::obs::event(
            crate::obs::names::COMPACT_DURABLE,
            crate::obs::TraceArgs::Count(1),
        );
        Ok(())
    }

    /// Populated file bytes of the store (not the virtual map, not resident
    /// memory — see the C04 map report for the distinct quantities).
    /// # Errors
    /// Storage failure.
    pub fn disk_size(&self) -> Result<u64> {
        let work = embedded_work()?;
        let report = self.store.map_report(&work).map_err(Error::from_store)?;
        Ok(report.populated_file_bytes)
    }

    /// The committed generation.
    /// # Errors
    /// Storage failure.
    pub fn generation(&self) -> Result<GenerationId> {
        let work = embedded_work()?;
        self.store
            .committed_generation(&work)
            .map_err(Error::from_store)
    }
}
