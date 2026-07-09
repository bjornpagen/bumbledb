//! The environment image cache (docs/architecture/50-storage.md) — the mechanism whose absence was
//! v5's quietest failure (post-mortem §26).
//!
//! Keyed by `(relation, generation)` where generation is the reader's
//! *snapshot-sourced* storage tx id — never an in-process counter
//! (`docs/architecture/50-storage.md`'s race-closing rule). Retain-newest
//! eviction runs at each state-changing commit; readers pinned at older
//! generations keep their `Arc`s alive until their transactions end. There
//! is no memory-pressure eviction, ever — the scale axiom.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::image::RelationImage;
use crate::schema::RelationId;

mod evict_older_than;
mod get_or_build;
mod new;
mod peek;

#[cfg(feature = "trace")]
mod resident;
/// Cache observability (feature `trace` only — per-op atomics are a cost
/// the default build must not carry). Reader: the benchmark report.
#[cfg(feature = "trace")]
pub mod stats;

#[cfg(test)]
mod keys;
#[cfg(test)]
mod tests;

struct CacheInner {
    map: HashMap<(RelationId, u64), Arc<RelationImage>>,
    /// The newest generation the cache has been evicted to. A reader below
    /// this builds query-locally without inserting (accepted — writes are
    /// bursty and rare).
    newest: u64,
}

/// The cross-transaction image cache, shared by reader threads. The mutex
/// covers map operations only — never a build — and every critical
/// section is panic-free (map probes, Arc clones, generation compares),
/// so the `expect("cache mutex")` unwraps can never observe poison from
/// this module's own code. Keep it that way: builds, decodes, and
/// anything else that can panic stay outside the lock.
pub struct ImageCache {
    inner: Mutex<CacheInner>,
    #[cfg(feature = "trace")]
    counters: stats::CacheCounters,
}

#[cfg(feature = "trace")]
impl ImageCache {
    /// The cache counters (feature `trace`): hits, misses, builds, and
    /// evicted entries since construction.
    #[must_use]
    pub fn stats(&self) -> stats::CacheStats {
        self.counters.read()
    }
}
