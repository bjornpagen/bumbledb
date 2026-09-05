//! Cache observability counters. Under the `trace` feature (and in test
//! builds, where they are the deterministic regression hook for the
//! per-relation invalidation contract) these are per-op atomics; off, the
//! counters type is a ZST and every method an inline empty body, so
//! instrumented call sites are written once, `#[cfg]`-free — the
//! obs.rs law, applied to the cache. Reader: the benchmark report.
#[cfg(any(test, feature = "trace"))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(any(test, feature = "trace"))]
#[derive(Debug, Default)]
pub(super) struct CacheCounters {
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    evicted: AtomicU64,
}

#[cfg(any(test, feature = "trace"))]
impl CacheCounters {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn build(&self) {
        self.builds.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn evicted(&self, entries: u64) {
        self.evicted.fetch_add(entries, Ordering::Relaxed);
    }

    pub(super) fn read(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            builds: self.builds.load(Ordering::Relaxed),
            evicted: self.evicted.load(Ordering::Relaxed),
        }
    }
}

/// One reading of the cache counters. A miss resolves through exactly one
/// build; entries are keyed per (relation, relation change version), so an
/// unrelated write produces hits, not builds.
#[cfg(any(test, feature = "trace"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub builds: u64,
    pub evicted: u64,
}

#[cfg(not(any(test, feature = "trace")))]
#[derive(Debug)]
pub(super) struct CacheCounters;

#[cfg(not(any(test, feature = "trace")))]
#[expect(
    clippy::unused_self,
    reason = "signature twins of the trace-mode counters (the obs.rs law)"
)]
impl CacheCounters {
    pub(super) fn new() -> Self {
        Self
    }

    #[inline]
    pub(super) fn hit(&self) {}

    #[inline]
    pub(super) fn miss(&self) {}

    #[inline]
    pub(super) fn build(&self) {}

    #[inline]
    pub(super) fn evicted(&self, _: u64) {}
}
