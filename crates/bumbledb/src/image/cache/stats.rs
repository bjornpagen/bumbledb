use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(super) struct CacheCounters {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub builds: AtomicU64,
    /// Copy-on-append extensions of a surviving base — the miss arm that
    /// copied columns and decoded only the tail.
    pub appends: AtomicU64,
    /// Carry-forwards — the zero-copy miss arm that re-keyed an untouched
    /// relation's `Arc` at the reader's generation.
    pub carries: AtomicU64,
    pub evicted: AtomicU64,
}

/// One reading of the cache counters. A miss resolves through exactly one
/// of `builds` / `appends` / `carries` — the delete-fallback pin's
/// instrument: a delta that deleted from a relation forces its next read
/// through `builds`; a delete-free one lands in `appends` or `carries`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub builds: u64,
    pub appends: u64,
    pub carries: u64,
    pub evicted: u64,
}

impl CacheCounters {
    pub(super) fn read(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            builds: self.builds.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            carries: self.carries.load(Ordering::Relaxed),
            evicted: self.evicted.load(Ordering::Relaxed),
        }
    }
}
