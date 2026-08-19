use std::path::Path;

use super::Db;
use crate::error::Result;
use crate::storage::env::GenerationId;

impl<S> Db<S> {
    /// The image cache's counters (feature `trace`; reader: the
    /// benchmark report).
    #[cfg(feature = "trace")]
    #[must_use]
    pub fn cache_stats(&self) -> crate::image::cache::stats::CacheStats {
        self.cache.stats()
    }

    /// Resident cached images and their total slab bytes (feature
    /// `trace`).
    #[cfg(feature = "trace")]
    #[must_use]
    pub fn cache_resident(&self) -> (u64, u64) {
        self.cache.resident()
    }

    /// Publishes a compacted copy of the store to `dest` (a directory
    /// that must not exist): one [`crate::storage::env::PublishStep`]
    /// fold, live `_data` and `_dict` bytes, fresh `_meta` with the
    /// source kind and generation. The source stays open and untouched.
    /// The copy is a first-class store: open it, read it, write to it.
    ///
    /// Durability, exactly: on return the copied `data.mdb` is fsynced,
    /// then `dest` itself (the file's directory entry), then `dest`'s
    /// parent directory (`dest`'s own entry) — the whole dirent chain a
    /// power loss would have to survive for the copy to still exist.
    /// Directories *above* the immediate parent are not fsynced, so a
    /// `dest` whose parent had to be created by this call is only
    /// power-loss-durable if the caller syncs those ancestors itself.
    ///
    /// # Errors
    ///
    /// `DestinationExists` when `dest` exists (never clobbers);
    /// `PublishedButUnsynced` when the copy is complete but a durability
    /// sync of the destination failed; `Io` when `dest` cannot be
    /// created; `Lmdb` from the copy itself.
    pub fn compact(&self, dest: &Path) -> Result<()> {
        let catalog = crate::storage::env::PublishCatalog::store(&self.env, self.schema.as_ref())?;
        drop(crate::storage::env::Environment::publish(
            dest,
            self.env.kind(),
            &catalog,
        )?);
        crate::obs::event(
            crate::obs::names::COMPACT_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(())
    }

    /// The database file's real on-disk size in bytes (a store-level
    /// observability number for the benchmark report).
    ///
    /// # Errors
    ///
    /// `Io` via heed on a failed stat.
    pub fn disk_size(&self) -> Result<u64> {
        self.env.disk_size()
    }

    /// The current committed generation (storage tx id), read through a
    /// fresh snapshot.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot open; `Corruption` on a malformed tx id.
    pub fn generation(&self) -> Result<GenerationId> {
        self.env.read_txn()?.generation()
    }
}
