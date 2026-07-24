//! The lineage-aware commit hook, run by the write path after each
//! state-changing commit (docs/architecture/50-storage.md § the image
//! cache): dirty relations evict, delete-free relations retain their
//! images as append bases.

use super::{Cached, ImageCache};
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

impl ImageCache {
    /// Advances the cache to `generation` — the one commit → cache wiring
    /// point's hook ([`crate::api`]'s `write_witnessed`). `dirty` is the
    /// commit's delete classification, per relation (the delta's
    /// net-disposition arithmetic, so a cancelled delete-then-reinsert
    /// pair does not dirty its relation), **sorted ascending** — the
    /// deduplicated ordered pass over the delta's `(relation, hash)`-keyed
    /// map guarantees it. `floors` is the commit's smallest inserted row
    /// id per fresh-keyed relation (`WriteDelta::inserted_floors`), same
    /// order guarantee.
    ///
    /// Entries of dirty relations below `generation` drop — a delete
    /// shifted their ordinals — and so do entries whose relation this
    /// commit inserted into BELOW the entry's append boundary: under the
    /// one id allocator (R16) an explicit fresh re-supply can land an `F`
    /// key under a retained base, and a tail decode would silently miss
    /// it, so the non-tail insert evicts exactly as a delete does (the
    /// prefix property is enforced here, never assumed from counter
    /// shape). The next reader rebuilds from scratch. Every other entry
    /// is retained as an **append base**: the next reader at the new
    /// generation extends it ([`crate::image::append`]) or carries it
    /// forward untouched, per [`ImageCache::get_or_build`]'s arms. The
    /// map drop only releases the map's reference — pinned readers keep
    /// their images alive. Synthesized closed-relation images live in the
    /// `closed` slot array, never in this generation-keyed map, and are
    /// untouched by construction.
    ///
    /// This maintains the lineage law (`CacheInner::map`): a surviving
    /// below-newest entry has seen only delete-free, tail-only commits
    /// since its generation.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex.
    pub fn advance(&self, generation: GenerationId, dirty: &[RelationId], floors: &[(RelationId, u64)]) {
        debug_assert!(dirty.is_sorted(), "the delta's ordered pass sorts dirty");
        debug_assert!(
            floors.is_sorted_by_key(|&(rel, _)| rel),
            "the delta's ordered pass sorts floors"
        );
        let keep = |key: &(RelationId, GenerationId), cached: &Cached| {
            let (rel, entry_gen) = *key;
            entry_gen >= generation
                || (dirty.binary_search(&rel).is_err()
                    && floors
                        .binary_search_by_key(&rel, |&(r, _)| r)
                        .map_or(true, |idx| floors[idx].1 >= cached.row_id_next))
        };
        let mut inner = self.inner.lock().expect("cache mutex");
        let before = inner.map.len();
        inner.map.retain(|key, cached| keep(key, cached));
        self.counters.evicted((before - inner.map.len()) as u64);
        inner.newest = inner.newest.max(generation);
    }
}
