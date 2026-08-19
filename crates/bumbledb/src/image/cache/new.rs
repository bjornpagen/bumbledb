//! Construction of an empty [`ImageCache`], shaped by its schema.

use crate::schema::Schema;

use super::{ImageCache, RelationSlot, stats};

impl ImageCache {
    /// An empty cache for one schema: one [`RelationSlot`] per relation,
    /// parsed from the schema body once. Closed relations get a
    /// generation-free `OnceLock`; ordinary relations get a
    /// [`super::GenerationCache`].
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_store(relation.body()))
                .collect(),
            counters: stats::CacheCounters::new(),
        }
    }
}
