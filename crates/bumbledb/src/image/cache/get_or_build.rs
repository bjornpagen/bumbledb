//! The read/build path: return the reader's image, building outside the
//! lock on a miss (docs/architecture/50-storage.md).

use std::sync::Arc;

use crate::error::Result;
use crate::image::{build, synthesize_closed, RelationImage};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;

use super::ImageCache;

impl ImageCache {
    /// Returns the image of `rel` at the reader's generation, building it
    /// outside the lock on a miss. Two same-generation readers racing to
    /// build may both build; insert-if-absent means the loser adopts the
    /// winner's `Arc` and drops its own (accepted waste, no latch).
    ///
    /// A **closed** relation branches before the generation map is ever
    /// touched: its image is synthesized from the sealed extension — the
    /// theory is the storage, so there is no generation to key on, no
    /// LMDB read, no eviction. First touch builds into the relation's
    /// `OnceLock` slot; every later reader clones the same `Arc` forever.
    ///
    /// # Errors
    ///
    /// Build errors (`Lmdb`, `Corruption`) propagate; synthesis is pure
    /// and cannot fail.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex (a prior panic while holding it).
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Arc<RelationImage>> {
        if self.closed_slot(rel).is_some() {
            return Ok(self.get_or_synthesize(schema, rel));
        }
        let generation = txn.generation()?;
        let key = (rel, generation);
        let newest = {
            let inner = self.inner.lock().expect("cache mutex");
            if let Some(image) = inner.map.get(&key) {
                #[cfg(feature = "trace")]
                self.counters
                    .hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                crate::obs::event(
                    crate::obs::names::CACHE_HIT,
                    crate::obs::Category::Cache,
                    u64::from(rel.0),
                    0,
                );
                return Ok(Arc::clone(image));
            }
            inner.newest
        };
        #[cfg(feature = "trace")]
        self.counters
            .misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Build outside the lock.
        let image = {
            let mut span = crate::obs::span_args(
                crate::obs::names::IMAGE_BUILD,
                crate::obs::Category::Image,
                u64::from(rel.0),
                0,
            );
            #[cfg(feature = "trace")]
            self.counters
                .builds
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let image = build(txn, schema, rel)?;
            span.set_args(u64::from(rel.0), image.byte_size() as u64);
            image
        };

        // An old-generation reader keeps its image query-local: inserting it
        // would poison the map for nobody (its generation is already evicted).
        if generation < newest {
            crate::obs::event(
                crate::obs::names::CACHE_QUERY_LOCAL,
                crate::obs::Category::Cache,
                u64::from(rel.0),
                0,
            );
            return Ok(image);
        }

        let mut inner = self.inner.lock().expect("cache mutex");
        // Re-check under the insert lock: a commit may have evicted this
        // generation between the first lock and here — inserting against
        // the stale `newest` would undo the eviction one entry at a time
        // and leak the image until the next state-changing commit.
        if generation < inner.newest {
            return Ok(image);
        }
        match inner.map.entry(key) {
            std::collections::hash_map::Entry::Occupied(winner) => {
                crate::obs::event(
                    crate::obs::names::CACHE_ADOPT,
                    crate::obs::Category::Cache,
                    u64::from(rel.0),
                    0,
                );
                Ok(Arc::clone(winner.get()))
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Arc::clone(&image));
                Ok(image)
            }
        }
    }

    /// The virtual branch: the synthesized image of a closed relation,
    /// built into its `OnceLock` slot on first touch. Losers of an init
    /// race block on the winner's synthesis (`OnceLock::get_or_init`) and
    /// adopt its Arc — exactly one build per slot per process, ever.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `rel` is not closed
    /// (the caller probed `closed_slot` first).
    fn get_or_synthesize(&self, schema: &Schema, rel: RelationId) -> Arc<RelationImage> {
        let slot = self.closed_slot(rel).expect("caller probed closed_slot");
        if let Some(image) = slot.get() {
            #[cfg(feature = "trace")]
            self.counters
                .hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            crate::obs::event(
                crate::obs::names::CACHE_HIT,
                crate::obs::Category::Cache,
                u64::from(rel.0),
                0,
            );
            return Arc::clone(image);
        }
        #[cfg(feature = "trace")]
        self.counters
            .misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let image = slot.get_or_init(|| {
            let mut span = crate::obs::span_args(
                crate::obs::names::IMAGE_BUILD,
                crate::obs::Category::Image,
                u64::from(rel.0),
                0,
            );
            #[cfg(feature = "trace")]
            self.counters
                .builds
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let image = synthesize_closed(rel, schema.relation(rel));
            span.set_args(u64::from(rel.0), image.byte_size() as u64);
            image
        });
        Arc::clone(image)
    }
}
