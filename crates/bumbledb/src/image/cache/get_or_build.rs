//! The read/build path: return the reader's image, building outside the
//! no append base survives, by column copy plus tail decode when one
//! does, and at zero copy when the relation was untouched.

use std::sync::{Arc, OnceLock};

use crate::error::{CorruptionError, Error, Result};
use crate::image::ViewEpoch;
#[cfg(test)]
use crate::image::bind::{ImageBind, LmdbSource};
use crate::image::{RelationImage, append, build, synthesize_closed};
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;
use crate::storage::env::GenerationId;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

use super::{Cached, GenerationCache, ImageCache, RelationSlot};

struct Base {
    image: Arc<RelationImage>,
    row_id_next: u64,
}

impl ImageCache {
    /// LMDB read, no eviction. First touch builds into the relation's

    /// # Errors

    /// # Panics

    #[cfg(test)]
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Arc<RelationImage>> {
        LmdbSource::bind(txn, self).image(schema, rel)
    }

    pub(crate) fn get_or_build_at(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
        epoch: ViewEpoch,
    ) -> Result<Arc<RelationImage>> {
        match (self.slot(rel), epoch) {
            (RelationSlot::Closed(slot), ViewEpoch::Closed) => {
                Ok(self.get_or_synthesize(schema, rel, slot))
            }
            (RelationSlot::Ordinary(cache), ViewEpoch::Store(generation)) => {
                self.get_or_build_ordinary(txn, schema, rel, cache, generation)
            }
            (RelationSlot::Closed(_), _) => {
                unreachable!("Closed slot carries no generation")
            }
            (RelationSlot::Frozen(_), _) => {
                unreachable!("store ImageCache never constructs Frozen slots")
            }
            (RelationSlot::Ordinary(_), _) => {
                unreachable!("store generation on a closed image is unrepresentable")
            }
        }
    }

    fn get_or_build_ordinary(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
        cache: &GenerationCache,
        generation: GenerationId,
    ) -> Result<Arc<RelationImage>> {
        let (newest, base) = {
            let inner = cache.lock();
            if let Some(cached) = inner.map.get(&generation) {
                self.counters.hit();
                crate::obs::event(
                    crate::obs::names::CACHE_HIT,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                return Ok(Arc::clone(&cached.image));
            }

            let base = (generation == inner.newest)
                .then(|| {
                    inner
                        .map
                        .iter()
                        .find(|&(&g, _)| g < generation)
                        .map(|(_, cached)| Base {
                            image: Arc::clone(&cached.image),
                            row_id_next: cached.row_id_next,
                        })
                })
                .flatten();
            (inner.newest, base)
        };
        self.counters.miss();

        let image = match base {
            Some(base) => self.extend(txn, schema, rel, &base)?,
            None => self.build_full(txn, schema, rel)?,
        };

        if generation < newest {
            crate::obs::event(
                crate::obs::names::CACHE_QUERY_LOCAL,
                crate::obs::TraceArgs::Count(u64::from(rel.0)),
            );
            return Ok(image);
        }

        let row_id_next = match schema.fresh_mint_field(rel) {
            Some(field) => crate::storage::delta::read_fresh_next(txn, rel, field)?,
            None => txn.catalog().row_id_high_water(rel)?,
        };

        let mut inner = cache.lock();

        if generation < inner.newest {
            return Ok(image);
        }
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
                    row_id_next,
                });

                inner.map.retain(|&g, _| g >= generation);
                Ok(image)
            }
        }
    }

    /// The from-scratch arm: one full LMDB scan and decode, exactly the

    fn build_full(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Arc<RelationImage>> {
        let mut span = crate::obs::span_args(
            crate::obs::names::IMAGE_BUILD,
            crate::obs::TraceArgs::Count(u64::from(rel.0)),
        );
        self.counters.build();
        let image = build(&txn.catalog(), schema, rel)?;
        span.set_pair(u64::from(rel.0), image.byte_size() as u64);
        Ok(image)
    }

    fn extend(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
        base: &Base,
    ) -> Result<Arc<RelationImage>> {
        let catalog = txn.catalog();
        let claimed = catalog.row_count(rel)?;
        let base_rows = base.image.row_count() as u64;
        let image = match claimed.cmp(&base_rows) {
            std::cmp::Ordering::Less => {
                return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                    relation: rel,
                    stored: claimed,
                }));
            }

            std::cmp::Ordering::Equal => {
                self.counters.carry();
                crate::obs::event(
                    crate::obs::names::CACHE_CARRY,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                Arc::clone(&base.image)
            }

            std::cmp::Ordering::Greater => {
                let mut span = crate::obs::span_args(
                    crate::obs::names::IMAGE_APPEND,
                    crate::obs::TraceArgs::Count(u64::from(rel.0)),
                );
                self.counters.append();
                let image = append(&catalog, schema, rel, &base.image, base.row_id_next)?;
                span.set_pair(u64::from(rel.0), image.byte_size() as u64);
                image
            }
        };
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
