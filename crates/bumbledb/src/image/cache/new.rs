//! Construction of an empty [`ImageCache`], shaped by its schema.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::schema::Schema;
use crate::storage::env::GenerationId;
use bumbledb_theory::schema::RelationId;

use super::{CacheInner, ImageCache, stats};

impl ImageCache {
    /// An empty cache for one schema: the generation map starts bare, and
    /// the `closed` slot array is sized here — one [`OnceLock`] per
    /// closed relation, in declaration order (the closed slot).
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        let closed_ids: Box<[RelationId]> = schema
            .relations()
            .iter()
            .enumerate()
            .filter(|(_, relation)| relation.body().closed_rows().is_some())
            .map(|(idx, _)| RelationId(u32::try_from(idx).expect("relation count fits u32")))
            .collect();
        let closed = (0..closed_ids.len()).map(|_| OnceLock::new()).collect();
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                newest: GenerationId::initial(),
            }),
            closed_ids,
            closed,
            counters: stats::CacheCounters::new(),
        }
    }
}
