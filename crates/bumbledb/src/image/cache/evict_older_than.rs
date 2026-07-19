//! Retain-newest eviction — [`ImageCache::advance`] with every relation
//! treated as dirty: every below-`generation` entry dropped. The write
//! path's hook is `advance` (docs/architecture/50-storage.md § the image
//! cache); this survives `cfg(test)`-only as the tests' one-call commit
//! simulation (the `lineage-off` A/B knob that once also reached it died
//! with its banked number — the manifest's ruling-4 gravestone).

use super::ImageCache;
use crate::storage::env::GenerationId;

impl ImageCache {
    /// Retains only entries at or above `generation` —
    /// [`ImageCache::advance`] with every relation dirty (no entry
    /// survives as an append base; the next reader of anything rebuilds
    /// from scratch). The map drop only releases the map's reference —
    /// pinned readers keep their images alive. Synthesized closed-relation
    /// images are untouched by construction: they live in the `closed`
    /// slot array, never in this generation-keyed map.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex.
    pub fn evict_older_than(&self, generation: GenerationId) {
        let mut inner = self.inner.lock().expect("cache mutex");
        let before = inner.map.len();
        inner
            .map
            .retain(|(_, entry_gen), _| *entry_gen >= generation);
        self.counters.evicted((before - inner.map.len()) as u64);
        inner.newest = inner.newest.max(generation);
    }
}
