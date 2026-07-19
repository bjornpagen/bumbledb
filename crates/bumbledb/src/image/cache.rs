//! The environment image cache (docs/architecture/50-storage.md) — the mechanism whose absence was
//! v5's quietest failure (post-mortem §26).
//!
//! Keyed by `(relation, generation)` where generation is the reader's
//! *snapshot-sourced* storage tx id — never an in-process counter
//! (`docs/architecture/50-storage.md`'s race-closing rule). At each
//! state-changing commit the writer [`ImageCache::advance`]s the cache:
//! entries of relations the commit **deleted from** drop (their ordinals
//! shifted — evict-and-rebuild, exactly as before); delete-free
//! relations' images are retained as **append bases** — the next reader
//! at the new generation copies columns and decodes only the tail
//! ([`crate::image::append`]; row-id high-water monotonicity is the
//! prefix property), or carries the same `Arc` forward when the relation
//! is untouched. Readers pinned at older generations keep their `Arc`s
//! alive until their transactions end. There is no memory-pressure
//! eviction, ever — the scale axiom.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::image::RelationImage;
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

mod advance;
/// Test-gated today: production commits go through [`ImageCache::advance`];
/// the retain-newest form survives as the tests' one-call commit
/// simulation and the measurement wave's lineage-disabled A/B twin (the
/// gate lifts when the bench knob lands).
#[cfg(test)]
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

/// One cached image plus the append boundary it was built against: the
/// relation's row-id high-water, read in the image's own build
/// transaction — snapshot-consistent by construction. Every row in the
/// image has id strictly below it; every row a later commit adds has id
/// at or above it, so a tail scan from here decodes exactly the rows the
/// image is missing ([`crate::image::append`]).
struct Cached {
    image: Arc<RelationImage>,
    row_id_next: u64,
}

struct CacheInner {
    /// **The lineage law:** an entry at generation `g < newest` exists
    /// only if every state-changing commit in `(g, newest]` was
    /// delete-free for its relation — maintained unconditionally by
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
    /// map stays O(relations) and the scale axiom's
    /// no-memory-pressure-eviction stance is unstrained.
    map: HashMap<(RelationId, GenerationId), Cached>,
    /// The newest generation the cache has been advanced to. A reader
    /// below this builds query-locally without inserting (accepted — the
    /// cost lands on the stale pinned reader alone and poisons nothing
    /// shared). The old parenthetical here — "writes are bursty and
    /// rare" — is RETRACTED: it was a workload assumption, never a
    /// measurement, and steady-write hosts are real; they are served by
    /// the copy-on-append path, not by an assumption about write
    /// frequency.
    newest: GenerationId,
}

/// The cross-transaction image cache, shared by reader threads. The mutex
/// covers map operations only — never a build — and every critical
/// section is panic-free (map probes, Arc clones, generation compares),
/// so the `expect("cache mutex")` unwraps can never observe poison from
/// this module's own code. Keep it that way: builds, decodes, and
/// anything else that can panic stay outside the lock.
pub struct ImageCache {
    inner: Mutex<CacheInner>,
    /// Relation slot → closed-relation slot (`None` = ordinary),
    /// fixed at construction from the schema — the index into `closed`.
    closed_slots: Box<[Option<u32>]>,
    /// Synthesized closed-relation images, indexed by closed slot —
    /// keyed OUTSIDE the generation map (`docs/architecture/50-storage.md`
    /// § virtual relations): a closed relation's storage is the theory
    /// and its "generation" is the fingerprint, so each slot builds on
    /// first touch and is **never evicted, never rebuilt** —
    /// [`ImageCache::advance`] (and its lineage-disabled twin
    /// [`ImageCache::evict_older_than`]) skips it by construction,
    /// because it is not in the generation-keyed map at all.
    closed: Box<[OnceLock<Arc<RelationImage>>]>,
    #[cfg(feature = "trace")]
    counters: stats::CacheCounters,
}

impl ImageCache {
    /// The synthesized-image slot of `rel`: `None` = ordinary (a foreign
    /// id also answers `None` — the ordinary path types that error).
    fn closed_slot(&self, rel: RelationId) -> Option<&OnceLock<Arc<RelationImage>>> {
        let slot = (*self
            .closed_slots
            .get(usize::try_from(rel.0).expect("64-bit usize"))?)?;
        Some(&self.closed[usize::try_from(slot).expect("64-bit usize")])
    }
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
