//! v5's quietest failure (post-mortem §26).
//! One [`RelationSlot`] per relation, indexed by [`RelationId`]. The
//! schema-body partition is parsed once at construction: a closed
//! relation is a generation-free [`OnceLock`], an ordinary store
//! relation is a [`GenerationCache`]. A store generation on a closed
//! image is unrepresentable — the closed arm has no generation field.
//! arm, R16) — drop (their ordinals shifted or their prefix broke —

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::image::RelationImage;
use crate::schema::RelationBody;
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

mod advance;
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
mod keys;
#[cfg(test)]
mod tests;

struct Cached {
    image: Arc<RelationImage>,
    row_id_next: u64,
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
    Frozen(OnceLock<Arc<RelationImage>>),
    Ordinary(GenerationCache),
}

impl RelationSlot {
    pub(crate) fn for_store(body: &RelationBody) -> Self {
        match body {
            RelationBody::Closed { .. } => Self::Closed(OnceLock::new()),
            RelationBody::Ordinary { .. } => Self::Ordinary(GenerationCache::new()),
        }
    }

    pub(crate) fn for_frozen(body: &RelationBody) -> Self {
        match body {
            RelationBody::Closed { .. } => Self::Closed(OnceLock::new()),
            RelationBody::Ordinary { .. } => Self::Frozen(OnceLock::new()),
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

/// The cross-transaction image cache, shared by reader threads. One
/// [`RelationSlot`] per schema relation. The mutex on each
/// every critical section is panic-free (map probes, Arc clones,
/// generation compares), so the `expect("cache mutex")` unwraps can
/// never observe poison from this module's own code. Keep it that way:
/// builds, decodes, and anything else that can panic stay outside the
/// lock. Closed (and, on a heap source, frozen) slots are
/// [`OnceLock`]s — first touch builds, never evicted, never rebuilt —
pub struct ImageCache {
    slots: Box<[RelationSlot]>,
    counters: stats::CacheCounters,
}

impl ImageCache {
    pub(crate) fn slot(&self, relation: RelationId) -> &RelationSlot {
        &self.slots[relation.0 as usize]
    }

    fn ordinary_slots(&self) -> impl Iterator<Item = (RelationId, &GenerationCache)> {
        self.slots.iter().enumerate().filter_map(|(idx, slot)| {
            let RelationSlot::Ordinary(cache) = slot else {
                return None;
            };
            Some((
                RelationId(u32::try_from(idx).expect("relation count fits u32")),
                cache,
            ))
        })
    }
}

#[cfg(feature = "trace")]
impl ImageCache {

    #[must_use]
    pub fn stats(&self) -> stats::CacheStats {
        self.counters.read()
    }
}
