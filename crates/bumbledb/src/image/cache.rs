//! The environment image cache (docs/architecture/50-storage.md) — the mechanism whose absence was
//! v5's quietest failure (post-mortem §26).
//!
//! One [`RelationSlot`] per relation, indexed by [`RelationId`]. The
//! schema-body partition is parsed once at construction: a closed
//! relation is a generation-free [`OnceLock`], an ordinary store
//! relation is a [`GenerationCache`]. A store generation on a closed
//! image is unrepresentable — the closed arm has no generation field.
//! Keyed by generation where generation is the reader's *snapshot-sourced*
//! storage tx id — never an in-process counter
//! (`docs/architecture/50-storage.md`'s race-closing rule). At each
//! state-changing commit the writer [`ImageCache::advance`]s the cache:
//! entries of relations the commit **deleted from** — or **inserted into
//! below the retained base's boundary** (the one id allocator's non-tail
//! arm, R16) — drop (their ordinals shifted or their prefix broke —
//! evict-and-rebuild); every other image is retained as an **append
//! base** — the next reader at the new generation copies columns and
//! decodes only the tail ([`crate::image::append`]; tail-only insertion
//! is the prefix property, enforced by that eviction), or carries the
//! same `Arc` forward when the relation is untouched. Readers pinned at older generations keep their `Arc`s
//! alive until their transactions end. There is no memory-pressure
//! eviction, ever — the scale axiom.

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
/// default build must not carry), a ZST twin with inline empty bodies
/// off — call sites are written once, `#[cfg]`-free (the obs.rs law).
/// Reader: the benchmark report.
pub mod stats;

#[cfg(test)]
mod keys;
#[cfg(test)]
mod tests;

/// One cached image plus the append boundary it was built against: the
/// relation's next row id — the `Q` next value on a fresh-keyed
/// relation, the `S` high-water otherwise (the one id allocator, R16) —
/// read in the image's own build transaction, snapshot-consistent by
/// construction. Every row in the image has id strictly below it; a
/// later commit landing UNDER it (explicit fresh re-supply) evicts this
/// entry in `advance`, so a surviving entry's tail scan from here
/// decodes exactly the rows the image is missing
/// ([`crate::image::append`]).
struct Cached {
    image: Arc<RelationImage>,
    row_id_next: u64,
}

/// Per-relation generation-keyed images. Lives only on
/// [`RelationSlot::Ordinary`]; a closed or frozen slot cannot hold one.
pub(crate) struct GenerationCache {
    /// **The lineage law:** an entry at generation `g < newest` exists
    /// only if every state-changing commit in `(g, newest]` was
    /// delete-free for this relation — maintained unconditionally by
    /// [`ImageCache::advance`] (a commit drops the entries of relations
    /// it deleted from, at every generation below the new one, and
    /// retains the rest as append bases). **Corollary, unconditional:**
    /// every insert in [`ImageCache::get_or_build`] — append, carry, or
    /// full build — sweeps the relation's entries below its own
    /// generation in the same critical section, so no entry can outlive
    /// the next insert above it: quiescent flow keeps exactly one entry
    /// per relation, and a reader racing the commit epilogue (its
    /// snapshot ahead of `newest`) supersedes the base it never probed
    /// instead of stranding it — the pre-sweep design leaked one whole
    /// image per race won, forever, on a never-deleted relation. Surplus
    /// is transient and bounded by concurrently racing readers (a reader
    /// still at the pre-race `newest` can re-add one entry below the
    /// racer's until the next insert sweeps both), never monotone: the
    /// map stays O(1) per relation and the scale axiom's
    /// no-memory-pressure-eviction stance is unstrained.
    inner: Mutex<GenerationInner>,
}

struct GenerationInner {
    map: HashMap<GenerationId, Cached>,
    /// The newest generation this slot has been advanced to. A reader
    /// below this builds query-locally without inserting (accepted — the
    /// cost lands on the stale pinned reader alone and poisons nothing
    /// shared). The old parenthetical here — "writes are bursty and
    /// rare" — is RETRACTED: it was a workload assumption, never a
    /// measurement, and steady-write hosts are real; they are served by
    /// the copy-on-append path, not by an assumption about write
    /// frequency.
    newest: GenerationId,
}

/// One relation's image slot. Arms mirror [`crate::image::ViewEpoch`]:
/// closed theory, frozen heap, or store generation cache. The closed
/// arm carries no generation — a store generation on a closed image is
/// unrepresentable. A frozen source never constructs
/// [`RelationSlot::Ordinary`]; a store [`ImageCache`] never constructs
/// [`RelationSlot::Frozen`].
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
/// [`GenerationCache`] covers map operations only — never a build — and
/// every critical section is panic-free (map probes, Arc clones,
/// generation compares), so the `expect("cache mutex")` unwraps can
/// never observe poison from this module's own code. Keep it that way:
/// builds, decodes, and anything else that can panic stay outside the
/// lock. Closed (and, on a heap source, frozen) slots are
/// [`OnceLock`]s — first touch builds, never evicted, never rebuilt —
/// and [`ImageCache::advance`] skips them by matching
/// [`RelationSlot::Ordinary`] only.
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
    /// The cache counters (feature `trace`): hits, misses, builds,
    /// appends, carries, and evicted entries since construction.
    #[must_use]
    pub fn stats(&self) -> stats::CacheStats {
        self.counters.read()
    }
}
