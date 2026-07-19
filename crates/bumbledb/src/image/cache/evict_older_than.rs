//! Retain-newest eviction — [`ImageCache::advance`] with lineage
//! disabled: every relation treated as dirty, every below-`generation`
//! entry dropped. The write path's hook is `advance`
//! (docs/architecture/50-storage.md § the image cache); this survives as
//! the lineage-disabled twin — the tests' one-call commit simulation and
//! the measurement wave's A/B knob (the `StridePadder::with_tolerance`
//! falsifier precedent: both behaviors lay out in one process).

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
        #[cfg(feature = "trace")]
        {
            let before = inner.map.len();
            inner
                .map
                .retain(|(_, entry_gen), _| *entry_gen >= generation);
            let evicted = before - inner.map.len();
            self.counters
                .evicted
                .fetch_add(evicted as u64, std::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(not(feature = "trace"))]
        inner
            .map
            .retain(|(_, entry_gen), _| *entry_gen >= generation);
        inner.newest = inner.newest.max(generation);
    }
}
