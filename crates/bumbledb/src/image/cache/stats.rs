use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(super) struct CacheCounters {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub builds: AtomicU64,
    pub evicted: AtomicU64,
}

/// One reading of the cache counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub builds: u64,
    pub evicted: u64,
}

impl CacheCounters {
    pub(super) fn read(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            builds: self.builds.load(Ordering::Relaxed),
            evicted: self.evicted.load(Ordering::Relaxed),
        }
    }
}
