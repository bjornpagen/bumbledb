//! Construction of an empty database-owned image cache.
use super::{ImageCache, RelationSlot};
use crate::schema::Schema;
use crate::work::cache::GenerationProtocol;

impl ImageCache {
    #[must_use]
    pub(crate) fn new(schema: &Schema) -> Self {
        Self::with_byte_cap(schema, super::DEFAULT_IMAGE_CACHE_BYTES)
    }

    /// An empty cache that keeps at most `cap` bytes of ordinary image slabs.
    #[must_use]
    pub(crate) fn with_byte_cap(schema: &Schema, cap: usize) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_store(relation.body()))
                .collect(),
            protocol: GenerationProtocol::new(),
            budget: super::Budget {
                cap,
                cached: 0.into(),
                clock: 0.into(),
            },
        }
    }
}
