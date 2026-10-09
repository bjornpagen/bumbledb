//! Maintenance: compaction, file size, generation and close.

use std::path::Path;

use super::Db;
use crate::error::{Error, Result};
use crate::storage::GenerationId;
use crate::storage::store::CloseReport;
use crate::storage::store::store_env::DATA_FILE;
use crate::work::WorkContext;

impl<S> Db<S> {
    /// Write a compacted image of the committed state into `dest`, a new
    /// directory: `dest/data.mdb` is the image, and `dest` opens as a
    /// database with the same identity, head and host records.
    /// # Errors
    /// `DestinationExists`, storage failure, or cancellation.
    pub fn compact(&self, dest: &Path, work: WorkContext) -> Result<()> {
        if dest.exists() {
            return Err(Error::DestinationExists {
                path: dest.to_path_buf(),
            });
        }
        std::fs::create_dir_all(dest)?;
        self.store
            .write_image(&dest.join(DATA_FILE), &work)
            .map_err(Error::from_store)?;
        std::fs::File::open(dest)?.sync_all()?;
        Ok(())
    }

    /// Populated file bytes of the store: not the virtual map, not resident
    /// memory.
    /// # Errors
    /// I/O failure reading the file metadata.
    pub fn disk_size(&self) -> Result<u64> {
        self.store.file_bytes().map_err(Error::from_store)
    }

    /// The committed generation.
    /// # Errors
    /// Storage failure or cancellation.
    pub fn generation(&self, work: WorkContext) -> Result<GenerationId> {
        self.store
            .committed_generation(&work)
            .map_err(Error::from_store)
    }

    /// Bounded close: stop admitting transactions and report.
    /// `Incomplete` keeps the store closing; live snapshots stay valid.
    #[must_use = "an incomplete close reports the live readers to release"]
    pub fn close(&self, work: &WorkContext) -> CloseReport {
        self.store.close(work)
    }
}
