//! state-changing commit: dirty relations evict, delete-free relations retain their
//! images as append bases.
//! The lineage-aware commit hook, run by the write path after each
use super::{Cached, ImageCache};
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

impl ImageCache {
    /// Entries of dirty relations below `generation` drop — a delete
    /// map drop only releases the map's reference — pinned readers keep
    /// # Panics
    pub fn advance(
        &self,
        generation: GenerationId,
        dirty: &[RelationId],
        floors: &[(RelationId, u64)],
    ) {
        debug_assert!(dirty.is_sorted(), "the delta's ordered pass sorts dirty");
        debug_assert!(
            floors.is_sorted_by_key(|&(rel, _)| rel),
            "the delta's ordered pass sorts floors"
        );
        let keep = |rel: RelationId, entry_gen: GenerationId, cached: &Cached| {
            entry_gen >= generation
                || (dirty.binary_search(&rel).is_err()
                    && floors
                        .binary_search_by_key(&rel, |&(r, _)| r)
                        .map_or(true, |idx| floors[idx].1 >= cached.row_id_next))
        };
        for (rel, cache) in self.ordinary_slots() {
            let mut inner = cache.lock();
            let before = inner.map.len();
            inner
                .map
                .retain(|&entry_gen, cached| keep(rel, entry_gen, cached));
            self.counters.evicted((before - inner.map.len()) as u64);
            inner.newest = inner.newest.max(generation);
        }
    }
}
