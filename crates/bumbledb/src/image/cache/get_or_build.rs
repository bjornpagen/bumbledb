//! The read/build path: return the epoch's image, building outside the
//! slot lock from one canonical-row scan. Heap epochs never enter the
//! generation map — a heap instance has no durable identity to key by, so
//! its images rebuild per execution (correctness over reuse).
//! Admission is charged onto the shared image before slab growth.
//! Cache refusal is [`ResidentAdmit::BeyondMemory`], not a swallowed
//! allocation Error — L05 execute/spill must open scratch text from it.
use std::sync::{Arc, OnceLock};

use crate::api::prepared::source::QuerySource;
use crate::error::Result;
use crate::image::ViewEpoch;
use crate::image::{RelationImage, ResidentAdmit, build_from_source, synthesize_closed};
use crate::schema::Schema;
use crate::storage::store::RelationVersion;
use crate::work::GenerationHandle;
use bumbledb_theory::schema::RelationId;

use super::{Cached, ImageCache, RelationSlot, VersionCache};

impl ImageCache {
    pub(crate) fn get_or_build_at(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
        epoch: ViewEpoch,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        let generation = self.acquire();
        self.get_or_build_with(source, schema, rel, epoch, &generation)
    }

    /// Build or hit using a caller-held generation. Source review must
    /// find this handle on every retained token consumer.
    pub(crate) fn get_or_build_with(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
        epoch: ViewEpoch,
        generation: &GenerationHandle,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        match (self.slot(rel), epoch) {
            (RelationSlot::Closed(slot), ViewEpoch::Closed) => {
                self.get_or_synthesize(schema, rel, slot, generation)
            }
            (RelationSlot::Ordinary(cache), ViewEpoch::Store(version)) => {
                self.get_or_build_ordinary(source, schema, rel, cache, version, generation)
            }
            (RelationSlot::Ordinary(_), ViewEpoch::Heap(_)) => {
                self.counters.miss();
                self.build_full(source, schema, rel, generation)
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
        cache: &VersionCache,
        version: RelationVersion,
        generation: &GenerationHandle,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        {
            let inner = cache.lock();
            if let Some(cached) = inner.map.get(&version) {
                self.counters.hit();
                crate::obs::event(
                    crate::obs::names::CACHE_HIT,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                return Ok(ResidentAdmit::Ready(Arc::clone(&cached.image)));
            }
        }
        self.counters.miss();

        let image = match self.build_full(source, schema, rel, generation)? {
            ResidentAdmit::Ready(image) => image,
            ResidentAdmit::BeyondMemory(exhausted) => {
                return Ok(ResidentAdmit::BeyondMemory(exhausted));
            }
        };

        let mut inner = cache.lock();
        if version < inner.newest {
            crate::obs::event(
                crate::obs::names::CACHE_QUERY_LOCAL,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            return Ok(ResidentAdmit::Ready(image));
        }
        inner.newest = version;
        match inner.map.entry(version) {
            std::collections::hash_map::Entry::Occupied(winner) => {
                crate::obs::event(
                    crate::obs::names::CACHE_ADOPT,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                Ok(ResidentAdmit::Ready(Arc::clone(&winner.get().image)))
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Cached {
                    image: Arc::clone(&image),
                });
                inner.map.retain(|&v, _| v >= version);
                Ok(ResidentAdmit::Ready(image))
            }
        }
    }

    fn build_full(
        &self,
        source: &QuerySource<'_>,
        schema: &Schema,
        rel: RelationId,
        generation: &GenerationHandle,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        let mut span = crate::obs::span_args(
            crate::obs::names::IMAGE_BUILD,
            crate::obs::TraceArgs::Count(u64::from(rel.0)),
        );
        self.counters.build();
        let admitted = build_from_source(source, schema, generation, rel)?;
        if let ResidentAdmit::Ready(image) = &admitted {
            span.set_pair(u64::from(rel.0), image.byte_size() as u64);
        }
        Ok(admitted)
    }

    fn get_or_synthesize(
        &self,
        schema: &Schema,
        rel: RelationId,
        slot: &OnceLock<Arc<RelationImage>>,
        generation: &GenerationHandle,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        if let Some(image) = slot.get() {
            self.counters.hit();
            crate::obs::event(
                crate::obs::names::CACHE_HIT,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            return Ok(ResidentAdmit::Ready(Arc::clone(image)));
        }
        self.counters.miss();
        if let Some(image) = slot.get() {
            return Ok(ResidentAdmit::Ready(Arc::clone(image)));
        }
        let mut span = crate::obs::span_args(
            crate::obs::names::IMAGE_BUILD,
            crate::obs::TraceArgs::Count(u64::from(rel.0)),
        );
        self.counters.build();
        let built = match synthesize_closed(rel, schema.relation(rel), generation.clone())? {
            ResidentAdmit::Ready(built) => built,
            ResidentAdmit::BeyondMemory(exhausted) => {
                return Ok(ResidentAdmit::BeyondMemory(exhausted));
            }
        };
        span.set_pair(u64::from(rel.0), built.byte_size() as u64);
        match slot.set(Arc::clone(&built)) {
            Ok(()) => Ok(ResidentAdmit::Ready(built)),
            Err(_) => Ok(ResidentAdmit::Ready(Arc::clone(
                slot.get().expect("closed image lost the race to a winner"),
            ))),
        }
    }
}
