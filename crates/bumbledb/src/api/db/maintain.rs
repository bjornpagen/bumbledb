//! Maintenance surface: compaction, size and generation reads.

use std::path::Path;

use super::Db;
use crate::error::{Error, Result};
use crate::storage::GenerationId;
use crate::storage::store::{CloseReport, MapPolicy, Store, UnindexedRows};
use crate::work::{ExecutionPolicy, WorkContext};

impl<S> Db<S> {
    /// Compact into a fresh store at `dest` (which must not exist) under an
    /// explicit operation allowance: one coherent source snapshot supplies
    /// rows, host records, attachment and generation together (ENG-003 by
    /// construction), and the destination adopts them in one durable
    /// transaction — a crash leaves `dest` absent, empty-staged, or complete.
    /// # Errors
    /// `DestinationExists`, storage failure, or stopped work.
    pub fn compact(&self, dest: &Path, work: WorkContext) -> Result<()> {
        let snapshot = self.store.snapshot(&work).map_err(Error::from_store)?;
        let policy = MapPolicy::default();
        let (target, fresh) =
            Store::create(dest, self.schema.as_ref(), policy).map_err(Error::from_store)?;
        target
            .adopt_snapshot(&snapshot, fresh, &UnindexedRows, &work)
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
    /// Storage failure or stopped work.
    pub fn disk_size(&self, work: WorkContext) -> Result<u64> {
        let report = self.store.map_report(&work).map_err(Error::from_store)?;
        Ok(report.populated_file_bytes)
    }

    /// The committed generation.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn generation(&self, work: WorkContext) -> Result<GenerationId> {
        self.store
            .committed_generation(&work)
            .map_err(Error::from_store)
    }

    /// Bounded close: stop admitting transactions and report.
    /// `Incomplete` keeps Closing state — live snapshots stay valid.
    #[must_use = "an incomplete close reports the live readers to release"]
    pub fn close(&self) -> CloseReport {
        let work = crate::api::db::start_operation(ExecutionPolicy {
            input_bytes: 1 << 16,
            working_bytes: 1 << 16,
            scratch_bytes: 1 << 16,
            result_bytes: 1 << 16,
            rows: 1 << 16,
            work_units: 1 << 16,
            timeout: std::time::Duration::from_secs(60),
        });
        match work {
            Ok(work) => self.store.close(&work),
            Err(_) => CloseReport::Incomplete {
                live_transactions: 0,
                oldest_age: None,
            },
        }
    }
}
