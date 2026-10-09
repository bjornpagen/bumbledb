//! The database-owned relation-image cache: one [`RelationSlot`] per
//! relation and one synchronized [`GenerationProtocol`]. Map entries are
//! eviction references to [`RelationImage`] owners; images own their slabs
//! and share a resolver generation.
//!
//! Prepared query state (selection, trie, COLT pools) stays separate; only
//! immutable relation images and text tokens are shared here. An execution
//! that interprets tokens holds a [`GenerationHandle`]. Explicit clearing
//! detaches map membership and rotates the current generation; live owners
//! keep their exact old meanings.
//!
//! Reuse requires both the requested resolver owner and (relation, relation
//! change version): a write to relation A never invalidates relation B's
//! image. Cached ordinary slabs stay under a byte cap, evicting the least
//! recently used image first; an image larger than the cap is never cached.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::image::RelationImage;
#[cfg(test)]
use crate::image::epoch::CacheGeneration;
use crate::schema::RelationBody;
use crate::storage::store::RelationVersion;
#[cfg(test)]
use crate::work::cache::WeakGenerationHandle;
use crate::work::cache::{GenerationHandle, GenerationProtocol};
use bumbledb_theory::schema::RelationId;

mod get_or_build;
mod new;
mod peek;

#[cfg(test)]
mod tests;

/// The default cap on cached image slab bytes: the working set of a small
/// (512 MB) device, next to LMDB's page cache.
pub(crate) const DEFAULT_IMAGE_CACHE_BYTES: usize = 128 << 20;

/// Cache membership shares the image's existing owner.
struct Cached {
    image: Arc<RelationImage>,
    bytes: usize,
    /// The budget clock at the last hit or insert.
    used: u64,
}

/// Cached slab bytes against their cap, and the clock that orders eviction.
struct Budget {
    cap: usize,
    cached: AtomicUsize,
    clock: AtomicU64,
}

impl Budget {
    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }

    fn admit(&self, bytes: usize) {
        self.cached.fetch_add(bytes, Ordering::Relaxed);
    }

    fn release(&self, bytes: usize) {
        self.cached.fetch_sub(bytes, Ordering::Relaxed);
    }

    fn over(&self) -> bool {
        self.cached.load(Ordering::Relaxed) > self.cap
    }
}

pub(crate) struct VersionCache {
    inner: Mutex<VersionInner>,
}

struct VersionInner {
    map: HashMap<RelationVersion, Cached>,

    newest: RelationVersion,
}

pub(crate) enum RelationSlot {
    Closed(Mutex<Option<Arc<RelationImage>>>),
    Ordinary(VersionCache),
}

impl RelationSlot {
    pub(crate) fn for_store(body: &RelationBody) -> Self {
        match body {
            RelationBody::Closed { .. } => Self::Closed(Mutex::new(None)),
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

/// The database-owned relation-image cache plus generation-owned
/// text resolution. One instance per database; prepared programs hold
/// `Arc<ImageCache>` handles to the same owner.
pub struct ImageCache {
    slots: Box<[RelationSlot]>,
    protocol: GenerationProtocol,
    budget: Budget,
}

impl ImageCache {
    pub(crate) fn slot(&self, relation: RelationId) -> &RelationSlot {
        &self.slots[relation.0 as usize]
    }

    /// Direct test evidence of cache membership, without event recording.
    #[cfg(test)]
    pub(crate) fn image_count(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| match slot {
                RelationSlot::Ordinary(cache) => cache.lock().map.len(),
                RelationSlot::Closed(slot) => {
                    usize::from(slot.lock().expect("cache mutex").is_some())
                }
            })
            .sum()
    }

    /// Acquire the current generation. Every token-bearing consumer holds
    /// this handle (directly or through its image) for the execution.
    #[must_use]
    pub fn acquire(&self) -> GenerationHandle {
        self.protocol.acquire()
    }

    /// Weak/versioned current generation for idle prepared memo caches.
    #[must_use]
    #[cfg(test)]
    pub fn weak_current(&self) -> WeakGenerationHandle {
        self.protocol.acquire().downgrade()
    }

    /// The current whole-cache generation identity.
    #[must_use]
    #[cfg(test)]
    pub fn cache_generation(&self) -> CacheGeneration {
        self.protocol.identity()
    }

    /// Rotate the current generation and detach all cached image entries.
    /// Live image / handle owners keep their resolver and slabs.
    /// The cache's previous current handle is dropped here so idle
    /// generations are not preserved forever.
    pub fn clear(&self) {
        // Rotate first: a builder holding an old resolver cannot publish
        // after detachment. Publication checks the current owner while
        // holding its slot lock, which detachment must then acquire.
        let _ = self.protocol.rotate();
        self.detach_map_entries();
    }

    fn detach_map_entries(&self) {
        for slot in &self.slots {
            match slot {
                RelationSlot::Ordinary(cache) => {
                    for (_, cached) in cache.lock().map.drain() {
                        self.budget.release(cached.bytes);
                    }
                }
                RelationSlot::Closed(slot) => {
                    *slot.lock().expect("closed cache mutex") = None;
                }
            }
        }
    }

    /// Cached ordinary slab bytes.
    #[cfg(test)]
    pub(crate) fn cached_bytes(&self) -> usize {
        self.budget.cached.load(Ordering::Relaxed)
    }

    /// Evicts least recently used images until the cached bytes fit the cap.
    /// Locks one slot at a time, so it runs after an insert releases its own.
    fn evict_to_cap(&self) {
        while self.budget.over() {
            let oldest = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(index, slot)| match slot {
                    RelationSlot::Ordinary(cache) => cache
                        .lock()
                        .map
                        .iter()
                        .map(|(&version, cached)| (cached.used, index, version))
                        .min(),
                    RelationSlot::Closed(_) => None,
                })
                .min();
            let Some((used, index, version)) = oldest else {
                return;
            };
            let RelationSlot::Ordinary(cache) = &self.slots[index] else {
                unreachable!("only ordinary slots hold budgeted images");
            };
            let mut inner = cache.lock();
            if inner
                .map
                .get(&version)
                .is_some_and(|cached| cached.used == used)
                && let Some(cached) = inner.map.remove(&version)
            {
                self.budget.release(cached.bytes);
            }
        }
    }
}
