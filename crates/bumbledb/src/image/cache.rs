//! The database-owned relation-image cache: one [`RelationSlot`] per
//! relation and one synchronized [`GenerationProtocol`]. Map entries are
//! eviction references to [`RelationImage`] owners; slab charge and the
//! resolver live inside the shared allocation / generation handle.
//!
//! Prepared query state (selection, trie, COLT pools) stays separate; only
//! immutable relation images and text tokens are shared here. An execution
//! that interprets tokens holds a [`GenerationHandle`]. Pressure detaches
//! map membership and rotates the current generation; live owners keep
//! exact old meanings and their charges.
//!
//! Invalidation is keyed by (relation, relation change version): the store
//! advances a relation's version exactly when a committed transaction
//! changed that relation's rows, so a write to relation A never invalidates
//! relation B's image, and a host-record/attachment-only generation bump
//! invalidates nothing (PERF-001 / APP-MUTATE — audit-core #1).
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::image::RelationImage;
use crate::image::epoch::{CacheGeneration, TextGeneration};
use crate::schema::RelationBody;
use crate::storage::store::RelationVersion;
use crate::work::CacheLedger;
use crate::work::cache::{GenerationHandle, GenerationProtocol, WeakGenerationHandle};
use bumbledb_theory::schema::RelationId;

mod get_or_build;
mod new;
mod peek;

#[cfg(feature = "trace")]
mod resident;
/// Cache observability: real per-op atomics under `trace` (a cost the
/// off — call sites are written once, `#[cfg]`-free (the obs.rs law).
/// Reader: the benchmark report.
/// default build must not carry), a ZST twin with inline empty bodies
pub mod stats;

#[cfg(test)]
mod tests;

/// Eviction reference only. Charge lives on [`RelationImage`].
struct Cached {
    image: Arc<RelationImage>,
}

pub(crate) struct VersionCache {
    inner: Mutex<VersionInner>,
}

struct VersionInner {
    map: HashMap<RelationVersion, Cached>,

    newest: RelationVersion,
}

pub(crate) enum RelationSlot {
    Closed(OnceLock<Arc<RelationImage>>),
    Ordinary(VersionCache),
}

impl RelationSlot {
    pub(crate) fn for_store(body: &RelationBody) -> Self {
        match body {
            RelationBody::Closed { .. } => Self::Closed(OnceLock::new()),
            RelationBody::Ordinary => Self::Ordinary(VersionCache::new()),
        }
    }
}

impl VersionCache {
    fn new() -> Self {
        Self {
            inner: Mutex::new(VersionInner {
                map: HashMap::new(),
                newest: RelationVersion::initial(),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VersionInner> {
        self.inner.lock().expect("cache mutex")
    }
}

/// The database-owned bounded relation-image cache plus generation-owned
/// text resolution. One instance per database; prepared programs hold
/// `Arc<ImageCache>` handles to the same owner.
pub struct ImageCache {
    slots: Box<[RelationSlot]>,
    counters: stats::CacheCounters,
    cache: CacheLedger,
    protocol: GenerationProtocol,
}

impl ImageCache {
    pub(crate) fn slot(&self, relation: RelationId) -> &RelationSlot {
        &self.slots[relation.0 as usize]
    }

    /// The shared retained-cache ledger every image and text token charges.
    pub(crate) fn cache_ledger(&self) -> &CacheLedger {
        &self.cache
    }

    /// Acquire the current generation. Every token-bearing consumer holds
    /// this handle (directly or through its image) for the execution.
    #[must_use]
    pub fn acquire(&self) -> GenerationHandle {
        self.protocol.acquire()
    }

    /// Weak/versioned current generation for idle prepared memo caches.
    #[must_use]
    pub fn weak_current(&self) -> WeakGenerationHandle {
        self.protocol.acquire().downgrade()
    }

    /// The current whole-cache generation identity.
    #[must_use]
    pub fn cache_generation(&self) -> CacheGeneration {
        self.protocol.identity()
    }

    #[must_use]
    pub fn text_generation(&self) -> TextGeneration {
        TextGeneration::of(self.cache_generation())
    }

    /// Retained cache bytes: the ledger, not map membership. Images held
    /// only by executions still count until their last strong owner drops.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        usize::try_from(self.cache.used()).unwrap_or(usize::MAX)
    }

    /// Detach version-keyed map entries and rotate the current generation.
    /// Live image / handle owners keep their resolver and slab charge.
    /// The cache's previous current handle is dropped here so idle
    /// generations are not preserved forever.
    pub fn trim(&self) {
        self.detach_map_entries();
        let _ = self.protocol.rotate(&self.cache);
    }

    fn detach_map_entries(&self) {
        for slot in &self.slots {
            if let RelationSlot::Ordinary(cache) = slot {
                let mut inner = cache.lock();
                let evicted = inner.map.len();
                inner.map.clear();
                self.counters.evicted(evicted as u64);
            }
        }
    }
}

#[cfg(any(test, feature = "trace"))]
impl ImageCache {
    /// The rebuild/hit counters: the deterministic regression hook for the
    /// per-relation invalidation contract (test builds), and the trace-mode
    /// read side whose recorded reader is the benchmark report (P14
    /// `--features obs`).
    #[must_use]
    #[cfg_attr(
        all(feature = "trace", not(test)),
        expect(
            dead_code,
            reason = "trace-mode counter read side; the recorded reader is \
                      the benchmark report (P14 `--features obs`)"
        )
    )]
    pub fn stats(&self) -> stats::CacheStats {
        self.counters.read()
    }
}
