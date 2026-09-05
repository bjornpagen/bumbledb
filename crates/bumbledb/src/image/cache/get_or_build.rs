//! The read/build path: return the epoch's image, building outside the
//! slot lock from one canonical-row scan. Heap epochs never enter the
//! generation map — a heap instance has no durable identity to key by, so
//! its images rebuild per execution (correctness over reuse).
use std::sync::{Arc, OnceLock};

use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::image::ViewEpoch;
use crate::image::{RelationImage, build_from_source, synthesize_closed};
use crate::schema::Schema;
use crate::storage::GenerationId;
use bumbledb_theory::schema::RelationId;

use super::{Cached, GenerationCache, ImageCache, RelationSlot};

impl ImageCache {
    pub(crate) fn get_or_build_at(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
        epoch: ViewEpoch,
    ) -> Result<Arc<RelationImage>> {
        match (self.slot(rel), epoch) {
            (RelationSlot::Closed(slot), ViewEpoch::Closed) => {
                Ok(self.get_or_synthesize(schema, rel, slot))
            }
            (RelationSlot::Ordinary(cache), ViewEpoch::Store(generation)) => {
                self.get_or_build_ordinary(source, schema, rel, cache, generation)
            }
            (RelationSlot::Ordinary(_), ViewEpoch::Heap(_)) => {
                // Per-execution rebuild: no memo can exist for a heap tick.
                self.counters.miss();
                self.build_full(source, schema, rel)
            }
            (RelationSlot::Closed(_), _) => {
                unreachable!("Closed slot carries no generation")
            }
            (RelationSlot::Ordinary(_), ViewEpoch::Closed) => {
                unreachable!("store generation on a closed image is unrepresentable")
            }
        }
    }

    fn get_or_build_ordinary(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
        cache: &GenerationCache,
        generation: GenerationId,
    ) -> Result<Arc<RelationImage>> {
        {
            let inner = cache.lock();
            if let Some(cached) = inner.map.get(&generation) {
                self.counters.hit();
                crate::obs::event(
                    crate::obs::names::CACHE_HIT,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                return Ok(Arc::clone(&cached.image));
            }
        }
        self.counters.miss();

        let image = self.build_full(source, schema, rel)?;

        let mut inner = cache.lock();
        if generation < inner.newest {
            // A newer execution already advanced this slot: the old
            // snapshot keeps its image query-local, charged to its owner.
            crate::obs::event(
                crate::obs::names::CACHE_QUERY_LOCAL,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            return Ok(image);
        }
        inner.newest = generation;
        match inner.map.entry(generation) {
            std::collections::hash_map::Entry::Occupied(winner) => {
                crate::obs::event(
                    crate::obs::names::CACHE_ADOPT,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                Ok(Arc::clone(&winner.get().image))
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Cached {
                    image: Arc::clone(&image),
                });
                // Old generations retire when a newer one lands; a pinned
                // reader's Arc keeps its image alive query-local.
                inner.map.retain(|&g, _| g >= generation);
                Ok(image)
            }
        }
    }

    /// The from-scratch arm: one full canonical scan and decode.
    fn build_full(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Arc<RelationImage>> {
        let mut span = crate::obs::span_args(
            crate::obs::names::IMAGE_BUILD,
            crate::obs::TraceArgs::Count(u64::from(rel.0)),
        );
        self.counters.build();
        let image = build_from_source(source, schema, &self.interner, rel)?;
        span.set_pair(u64::from(rel.0), image.byte_size() as u64);
        Ok(image)
    }

    fn get_or_synthesize(
        &self,
        schema: &Schema,
        rel: RelationId,
        slot: &OnceLock<Arc<RelationImage>>,
    ) -> Arc<RelationImage> {
        if let Some(image) = slot.get() {
            self.counters.hit();
            crate::obs::event(
                crate::obs::names::CACHE_HIT,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            return Arc::clone(image);
        }
        self.counters.miss();
        let image = slot.get_or_init(|| {
            let mut span = crate::obs::span_args(
                crate::obs::names::IMAGE_BUILD,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            self.counters.build();
            let image = synthesize_closed(rel, schema.relation(rel));
            span.set_pair(u64::from(rel.0), image.byte_size() as u64);
            image
        });
        Arc::clone(image)
    }
}
