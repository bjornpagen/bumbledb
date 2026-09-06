//! The read/build path: return the epoch and resolver owner's image, building outside the
//! slot lock from one canonical-row scan. Heap epochs never enter the
//! generation map — a heap instance has no durable identity to key by, so
//! its images rebuild per execution (correctness over reuse).
//! Admission is charged onto the shared image before slab growth.
//! Cache refusal is [`ResidentAdmit::BeyondMemory`], not a swallowed
//! allocation Error — L05 execute/spill must open scratch text from it.
use std::sync::{Arc, Mutex};

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
    #[cfg(test)]
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
                build_from_source(source, schema, generation, rel)
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
            if let Some(cached) = inner.map.get(&version)
                && cached.image.generation().ptr_eq(generation)
            {
                return Ok(ResidentAdmit::Ready(Arc::clone(&cached.image)));
            }
        }
        let image = match build_from_source(source, schema, generation, rel)? {
            ResidentAdmit::Ready(image) => image,
            ResidentAdmit::BeyondMemory(exhausted) => {
                return Ok(ResidentAdmit::BeyondMemory(exhausted));
            }
        };

        let mut inner = cache.lock();
        if version < inner.newest || !generation.ptr_eq(&self.acquire()) {
            return Ok(ResidentAdmit::Ready(image));
        }
        inner.newest = version;
        match inner.map.entry(version) {
            std::collections::hash_map::Entry::Occupied(mut winner) => {
                if winner.get().image.generation().ptr_eq(generation) {
                    Ok(ResidentAdmit::Ready(Arc::clone(&winner.get().image)))
                } else {
                    winner.insert(Cached {
                        image: Arc::clone(&image),
                    });
                    Ok(ResidentAdmit::Ready(image))
                }
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

    fn get_or_synthesize(
        &self,
        schema: &Schema,
        rel: RelationId,
        slot: &Mutex<Option<Arc<RelationImage>>>,
        generation: &GenerationHandle,
    ) -> Result<ResidentAdmit<Arc<RelationImage>>> {
        if let Some(image) = slot.lock().expect("closed cache mutex").as_ref()
            && image.generation().ptr_eq(generation)
        {
            return Ok(ResidentAdmit::Ready(Arc::clone(image)));
        }
        let built = match synthesize_closed(rel, schema.relation(rel), generation.clone())? {
            ResidentAdmit::Ready(built) => built,
            ResidentAdmit::BeyondMemory(exhausted) => {
                return Ok(ResidentAdmit::BeyondMemory(exhausted));
            }
        };
        let mut slot = slot.lock().expect("closed cache mutex");
        if let Some(winner) = slot.as_ref()
            && winner.generation().ptr_eq(generation)
        {
            return Ok(ResidentAdmit::Ready(Arc::clone(winner)));
        }
        if generation.ptr_eq(&self.acquire()) {
            *slot = Some(Arc::clone(&built));
        }
        Ok(ResidentAdmit::Ready(built))
    }
}
