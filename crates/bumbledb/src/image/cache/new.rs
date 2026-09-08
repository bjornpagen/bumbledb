//! Construction of an empty database-owned image cache.
use super::{ImageCache, RelationSlot};
use crate::schema::Schema;
use crate::work::cache::GenerationProtocol;

impl ImageCache {
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_store(relation.body()))
                .collect(),
            protocol: GenerationProtocol::new(),
        }
    }
}
