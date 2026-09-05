//! Construction of an empty [`ImageCache`], shaped by its schema.
use crate::schema::Schema;
use crate::work::{CacheLedger, CachePolicy};
use crate::work::cache::GenerationProtocol;

use super::{ImageCache, RelationSlot, stats};

impl ImageCache {
    /// One database-owned cache: shared across prepared programs on the
    /// same tenant. `cache` carries the cross-operation retention budget.
    #[must_use]
    pub fn with_cache(schema: &Schema, cache: CacheLedger) -> Self {
        Self {
            slots: schema
                .relations()
                .iter()
                .map(|relation| RelationSlot::for_store(relation.body()))
                .collect(),
            counters: stats::CacheCounters::new(),
            protocol: GenerationProtocol::new(cache.clone()),
            cache,
        }
    }

    /// Test and transitional constructor: unbounded cache retention.
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        Self::with_cache(schema, CacheLedger::unbounded())
    }

    /// Host-default retention for one database open.
    #[must_use]
    pub fn with_policy(schema: &Schema, policy: CachePolicy) -> Self {
        Self::with_cache(schema, CacheLedger::new(policy))
    }
}
