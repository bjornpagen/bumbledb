//! Construction of an empty [`ImageCache`], shaped by its schema.
use crate::schema::Schema;

use super::{ImageCache, RelationSlot, stats};

impl ImageCache {
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_store(relation.body()))
                .collect(),
            interner: std::sync::Mutex::new(crate::image::intern::TextInterner::default()),
            counters: stats::CacheCounters::new(),
        }
    }
}
