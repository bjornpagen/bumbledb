//! Test-only key-set observability over the cache map.
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

use super::ImageCache;

impl ImageCache {
    pub(super) fn keys(&self) -> Vec<(RelationId, GenerationId)> {
        let mut keys = Vec::new();
        for (rel, cache) in self.ordinary_slots() {
            let inner = cache.lock();
            keys.extend(inner.map.keys().map(|&generation| (rel, generation)));
        }
        keys.sort_unstable();
        keys
    }
}
