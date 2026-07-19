//! The lineage-aware commit hook, run by the write path after each
//! state-changing commit (docs/architecture/50-storage.md § the image
//! cache): dirty relations evict, delete-free relations retain their
//! images as append bases.

use super::ImageCache;
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

impl ImageCache {
    /// Advances the cache to `generation` — the one commit → cache wiring
    /// point's hook ([`crate::api`]'s `write_witnessed`). `dirty` is the
    /// commit's delete classification, per relation (the delta's
    /// net-disposition arithmetic, so a cancelled delete-then-reinsert
    /// pair does not dirty its relation), **sorted ascending** — the
    /// deduplicated ordered pass over the delta's `(relation, hash)`-keyed
    /// map guarantees it.
    ///
    /// Entries of dirty relations below `generation` drop — a delete
    /// shifted their ordinals; the next reader rebuilds from scratch,
    /// exactly as every commit used to force. Every other entry is
    /// retained as an **append base**: the next reader at the new
    /// generation extends it ([`crate::image::append`]) or carries it
    /// forward untouched, per [`ImageCache::get_or_build`]'s arms. The
    /// map drop only releases the map's reference — pinned readers keep
    /// their images alive. Synthesized closed-relation images live in the
    /// `closed` slot array, never in this generation-keyed map, and are
    /// untouched by construction.
    ///
    /// This maintains the lineage law (`CacheInner::map`): a surviving
    /// below-newest entry has seen only delete-free commits since its
    /// generation.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex.
    pub fn advance(&self, generation: GenerationId, dirty: &[RelationId]) {
        debug_assert!(dirty.is_sorted(), "the delta's ordered pass sorts dirty");
        let keep = |key: &(RelationId, GenerationId)| {
            let (rel, entry_gen) = *key;
            entry_gen >= generation || dirty.binary_search(&rel).is_err()
        };
        let mut inner = self.inner.lock().expect("cache mutex");
        #[cfg(feature = "trace")]
        {
            let before = inner.map.len();
            inner.map.retain(|key, _| keep(key));
            let evicted = before - inner.map.len();
            self.counters
                .evicted
                .fetch_add(evicted as u64, std::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(not(feature = "trace"))]
        inner.map.retain(|key, _| keep(key));
        inner.newest = inner.newest.max(generation);
    }
}
