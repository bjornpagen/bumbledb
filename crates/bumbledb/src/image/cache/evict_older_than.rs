//! Retain-newest eviction, run by the write path after each state-changing
//! commit (docs/architecture/50-storage.md).

use super::ImageCache;

impl ImageCache {
    /// Retains only entries at or above `generation`; called by the write
    /// path after each state-changing commit (the 60-api doc wires `CommitReport`
    /// here). The map drop only releases the map's reference — pinned
    /// readers keep their images alive. Synthesized closed-relation
    /// images are untouched by construction: they live in the `closed`
    /// slot array, never in this generation-keyed map.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex.
    pub fn evict_older_than(&self, generation: u64) {
        let mut inner = self.inner.lock().expect("cache mutex");
        #[cfg(feature = "trace")]
        {
            let before = inner.map.len();
            inner.map.retain(|(_, gen), _| *gen >= generation);
            let evicted = before - inner.map.len();
            self.counters
                .evicted
                .fetch_add(evicted as u64, std::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(not(feature = "trace"))]
        inner.map.retain(|(_, gen), _| *gen >= generation);
        inner.newest = inner.newest.max(generation);
    }
}
