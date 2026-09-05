//! The prepared query's image cache: one [`RelationSlot`] per relation,
//! indexed by [`RelationId`], plus the one execution-scoped text interner
//! every image build, param bind, literal latch and answer resolution
//! shares (token equality is text equality — `image/intern.rs`).
//!
//! The schema-body partition is parsed once at construction: a closed
//! relation is a generation-free [`OnceLock`], an ordinary store relation
//! is a [`GenerationCache`]. A store generation on a closed image is
//! unrepresentable — the closed arm has no generation field. Heap-instance
//! executions never enter the generation map (their `ViewEpoch::Heap`
//! ticks are per-execution; images rebuild each run because a heap
//! instance carries no durable identity to key a memo by).
//!
//! Invalidation is generation-keyed: building at a newer generation
//! retains only entries at or above it, so a write's next read pays one
//! full rebuild per touched relation and old pinned snapshots keep their
//! old images (charged to their owners). The old write-path `advance`
//! lineage hook (append bases, dirty-relation eviction) is deleted with
//! the transitional storage; per-relation change tracking for
//! untouched-relation reuse across generations is a recorded C04 seam
//! request to P02R (see `implementation/packets/P03.md`).
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::image::RelationImage;
use crate::image::intern::TextInterner;
use crate::schema::RelationBody;
use crate::storage::GenerationId;
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

struct Cached {
    image: Arc<RelationImage>,
}

pub(crate) struct GenerationCache {
    inner: Mutex<GenerationInner>,
}

struct GenerationInner {
    map: HashMap<GenerationId, Cached>,

    newest: GenerationId,
}

pub(crate) enum RelationSlot {
    Closed(OnceLock<Arc<RelationImage>>),
    Ordinary(GenerationCache),
}

impl RelationSlot {
    pub(crate) fn for_store(body: &RelationBody) -> Self {
        match body {
            RelationBody::Closed { .. } => Self::Closed(OnceLock::new()),
            RelationBody::Ordinary => Self::Ordinary(GenerationCache::new()),
        }
    }
}

impl GenerationCache {
    fn new() -> Self {
        Self {
            inner: Mutex::new(GenerationInner {
                map: HashMap::new(),
                newest: GenerationId::initial(),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, GenerationInner> {
        self.inner.lock().expect("cache mutex")
    }
}

/// The prepared query's relation-image cache plus the shared text
/// interner. One [`RelationSlot`] per schema relation. The mutex on each
/// slot's critical section is panic-free (map probes, Arc clones,
/// generation compares), so the `expect("cache mutex")` unwraps can
/// never observe poison from this module's own code. Keep it that way:
/// builds, decodes, and anything else that can panic stay outside the
/// lock. Closed slots are [`OnceLock`]s — first touch builds, never
/// evicted, never rebuilt.
pub struct ImageCache {
    slots: Box<[RelationSlot]>,
    interner: Mutex<TextInterner>,
    counters: stats::CacheCounters,
}

impl ImageCache {
    pub(crate) fn slot(&self, relation: RelationId) -> &RelationSlot {
        &self.slots[relation.0 as usize]
    }

    /// The one text→token map this cache's images and binds share.
    pub(crate) fn interner(&self) -> &Mutex<TextInterner> {
        &self.interner
    }

    /// Retained cache bytes: every resident image slab plus the interner's
    /// text (a host budgeting figure, not an allocator measurement).
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        let images: usize = self
            .slots
            .iter()
            .map(|slot| match slot {
                RelationSlot::Closed(slot) => slot.get().map_or(0, |image| image.byte_size()),
                RelationSlot::Ordinary(cache) => cache
                    .lock()
                    .map
                    .values()
                    .map(|cached| cached.image.byte_size())
                    .sum(),
            })
            .sum();
        images
            + self
                .interner
                .lock()
                .expect("interner mutex")
                .retained_bytes()
    }

    /// Drop every generation-keyed image (memory-pressure trim). Closed
    /// images and the interner stay: token stability is the cache's
    /// invariant, and the trim unit for text is dropping the whole cache
    /// (re-prepare). The next execution rebuilds what it touches.
    pub fn trim(&self) {
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

#[cfg(feature = "trace")]
impl ImageCache {
    #[must_use]
    #[expect(
        dead_code,
        reason = "trace-mode counter read side; the recorded reader is the \
                  benchmark report (P14 `--features obs`)"
    )]
    pub fn stats(&self) -> stats::CacheStats {
        self.counters.read()
    }
}
