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

    /// Writes a compacted copy of the store to `dest` (a directory that
    /// must not exist): live pages only, freelist dropped, sequential
    /// layout (docs/architecture/50-storage.md). The source stays open and untouched —
    /// compaction is a copy, never in-place, so the source remains the
    /// fallback until the caller swaps directories. The copy is a
    /// first-class store: open it, read it, write to it.
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
    /// `Io` when `dest` exists or cannot be created (never clobbers), or
    /// when any sync of the durability chain fails; `Lmdb` from the copy
    /// itself.
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
        // Durable before the caller swaps directories: the file, its
        // dirent in `dest`, then `dest`'s own dirent in the parent —
        // without the parent sync, power loss could keep a durable file
        // inside a directory entry that was never made durable. The
        // chain sync is shared with `Environment::create`'s birth
        // (finding 022 — one mechanism, two sites).
        file.sync_all().map_err(crate::error::Error::Io)?;
        crate::storage::env::sync_dirent_chain(dest).map_err(crate::error::Error::Io)?;
        crate::obs::event(
            crate::obs::names::COMPACT_DURABLE,
            crate::obs::Category::Storage,
            2,
            0,
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
