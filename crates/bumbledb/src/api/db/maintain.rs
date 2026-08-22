use std::path::Path;

use super::Db;
use crate::error::Result;
use crate::storage::env::GenerationId;

impl<S> Db<S> {
    #[cfg(feature = "trace")]
    #[must_use]
    pub fn cache_stats(&self) -> crate::image::cache::stats::CacheStats {
        self.cache.stats()
    }

    #[cfg(feature = "trace")]
    #[must_use]
    pub fn cache_resident(&self) -> (u64, u64) {
        self.cache.resident()
    }

    /// that must not exist): one [`crate::storage::env::PublishStep`]
    /// # Errors
    pub fn compact(&self, dest: &Path) -> Result<()> {
        let catalog = crate::storage::env::PublishCatalog::store(&self.env, self.schema.as_ref())?;
        drop(crate::storage::env::Environment::publish(dest, &catalog)?);
        crate::obs::event(
            crate::obs::names::COMPACT_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(())
    }

    /// # Errors
    pub fn disk_size(&self) -> Result<u64> {
        self.env.disk_size()
    }

    /// # Errors
    pub fn generation(&self) -> Result<GenerationId> {
        self.env.read_txn()?.generation()
    }
}
