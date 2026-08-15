//! Construction of an empty [`ImageCache`], shaped by its schema.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::schema::{RelationBody, Schema};
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

use super::{CacheInner, ImageCache, stats};

impl ImageCache {
    /// An empty cache for one schema: the generation map starts bare, and
    /// closed relations get a `OnceLock` slot keyed by [`RelationId`].
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        let closed = schema
            .relations()
            .iter()
            .enumerate()
            .filter_map(|(idx, relation)| match relation.body() {
                RelationBody::Closed { .. } => Some((
                    RelationId(u32::try_from(idx).expect("relation count fits u32")),
                    OnceLock::new(),
                )),
                RelationBody::Ordinary { .. } => None,
            })
            .collect();
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                newest: GenerationId::initial(),
            }),
            closed,
            counters: stats::CacheCounters::new(),
        }
    }
}
