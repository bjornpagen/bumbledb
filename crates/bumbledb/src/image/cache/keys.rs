//! Test-only key-set observability over the cache map.

use crate::schema::RelationId;
use crate::storage::env::GenerationId;

use super::ImageCache;

impl ImageCache {
    /// The set of `(relation, generation)` keys currently cached
    /// (test-only observability).
    pub(super) fn keys(&self) -> Vec<(RelationId, GenerationId)> {
        let inner = self.inner.lock().expect("cache mutex");
        let mut keys: Vec<_> = inner.map.keys().copied().collect();
        keys.sort_unstable();
        keys
    }
}
