//! Cache observability counters. Under the `trace` feature these are
//! per-op atomics (a cost the default build must not carry — the
//! feature ruling, docs/architecture/60-validation.md); off, the
//! counters type is a ZST and every method an inline empty body, so
//! instrumented call sites are written once, `#[cfg]`-free — the
//! obs.rs law, applied to the cache. Reader: the benchmark report.

#[cfg(feature = "trace")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "trace")]
#[derive(Debug, Default)]
pub(super) struct CacheCounters {
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    /// Copy-on-append extensions of a surviving base — the miss arm that
    /// copied columns and decoded only the tail.
    appends: AtomicU64,
    /// Carry-forwards — the zero-copy miss arm that re-keyed an untouched
    /// relation's `Arc` at the reader's generation.
    carries: AtomicU64,
    evicted: AtomicU64,
}

#[cfg(feature = "trace")]
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

    pub(super) fn append(&self) {
        self.appends.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn carry(&self) {
        self.carries.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn evicted(&self, entries: u64) {
        self.evicted.fetch_add(entries, Ordering::Relaxed);
    }

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

/// One reading of the cache counters. A miss resolves through exactly one
/// of `builds` / `appends` / `carries` — the delete-fallback pin's
/// instrument: a delta that deleted from a relation forces its next read
/// through `builds`; a delete-free one lands in `appends` or `carries`.
#[cfg(feature = "trace")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub builds: u64,
    pub appends: u64,
    pub carries: u64,
    pub evicted: u64,
}

// ---------------------------------------------------------------------
// Feature off: identical signatures, empty bodies, ZST — call sites
// never write #[cfg] (the obs.rs law).
// ---------------------------------------------------------------------

/// The cache counters (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[derive(Debug)]
pub(super) struct CacheCounters;

#[cfg(not(feature = "trace"))]
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
    pub(super) fn append(&self) {}

    #[inline]
    pub(super) fn carry(&self) {}

    #[inline]
    pub(super) fn evicted(&self, _: u64) {}
}
