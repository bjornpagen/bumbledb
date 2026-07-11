//! The non-building cache probe (docs/architecture/40-execution.md:
//! prepare-time statistics peek).

use std::sync::Arc;

use crate::error::Result;
use crate::image::RelationImage;
use crate::schema::RelationId;
use crate::storage::env::ReadTxn;

use super::ImageCache;

impl ImageCache {
    /// The resident image for `(rel, current generation)` — **never
    /// builds** (docs/architecture/40-execution.md: prepare-time statistics peek; a cold
    /// cache falls back to schema-derived bounds and floors).
    ///
    /// # Errors
    ///
    /// `Lmdb` from the generation read.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex (a prior panic while holding it).
    pub fn peek(&self, txn: &ReadTxn<'_>, rel: RelationId) -> Result<Option<Arc<RelationImage>>> {
        // A closed relation's slot, once synthesized, is resident forever
        // — same never-builds contract, and no generation to read.
        if let Some(slot) = self.closed_slot(rel) {
            return Ok(slot.get().map(Arc::clone));
        }
        let generation = txn.generation()?;
        let inner = self.inner.lock().expect("cache mutex");
        Ok(inner.map.get(&(rel, generation)).map(Arc::clone))
    }
}
