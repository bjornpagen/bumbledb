//! Construction of an empty [`ImageCache`], shaped by its schema.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::schema::Schema;
use crate::storage::env::GenerationId;

use super::{CacheInner, ImageCache, stats};

impl ImageCache {
    /// An empty cache for one schema: the generation map starts bare, and
    /// the `closed` slot array is sized here — one [`OnceLock`] per
    /// closed relation, in declaration order (the closed slot).
    #[must_use]
    pub fn new(schema: &Schema) -> Self {
        let mut count = 0u32;
        let closed_slots: Box<[Option<u32>]> = schema
            .relations()
            .iter()
            .map(|relation| {
                relation.is_closed().then(|| {
                    let slot = count;
                    count += 1;
                    slot
                })
            })
            .collect();
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                newest: GenerationId::initial(),
            }),
            closed_slots,
            closed: (0..count).map(|_| OnceLock::new()).collect(),
            counters: stats::CacheCounters::new(),
        }
    }
}
