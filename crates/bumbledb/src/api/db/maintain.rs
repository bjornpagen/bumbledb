use std::path::Path;

use super::Db;
use crate::error::Result;

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

    /// Writes a compacted copy of the store to `dest` (a directory that
    /// must not exist): live pages only, freelist dropped, sequential
    /// layout (docs/architecture/50-storage.md). The source stays open and untouched —
    /// compaction is a copy, never in-place, so the source remains the
    /// fallback until the caller swaps directories. The copy is a
    /// first-class store: open it, read it, write to it.
    ///
    /// # Errors
    ///
    /// `Io` when `dest` exists or cannot be created (never clobbers);
    /// `Lmdb` from the copy itself.
    pub fn compact(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            return Err(crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("compact refuses to clobber {}", dest.display()),
            )));
        }
        std::fs::create_dir_all(dest).map_err(crate::error::Error::Io)?;
        let data = dest.join("data.mdb");
        let mut file = std::fs::File::create(&data).map_err(crate::error::Error::Io)?;
        self.env.copy_compacted(&mut file)?;
        // Durable before the caller swaps directories: the file, then
        // its directory entry.
        file.sync_all().map_err(crate::error::Error::Io)?;
        std::fs::File::open(dest)
            .and_then(|dir| dir.sync_all())
            .map_err(crate::error::Error::Io)?;
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
    pub fn generation(&self) -> Result<u64> {
        self.env.read_txn()?.generation()
    }
}
