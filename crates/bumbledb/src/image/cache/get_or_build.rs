//! The read/build path: return the reader's image, building outside the
//! lock on a miss (docs/architecture/50-storage.md) — from scratch when
//! no append base survives, by column copy plus tail decode when one
//! does, and at zero copy when the relation was untouched.

use std::sync::Arc;

use crate::error::{CorruptionError, Error, Result};
use crate::image::{RelationImage, append, build, synthesize_closed};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::read;
use bumbledb_theory::schema::RelationId;

use super::{Cached, ImageCache};

/// The relation's surviving append base, cloned out of the map under the
/// probe lock: the immutable image and the append boundary
/// ([`Cached::row_id_next`]). No key rides along — the insert path
/// sweeps EVERY older entry of the relation, not one remembered key.
struct Base {
    image: Arc<RelationImage>,
    row_id_next: u64,
}

impl ImageCache {
    /// Returns the image of `rel` at the reader's generation, building it
    /// outside the lock on a miss. Two same-generation readers racing to
    /// build may both build; insert-if-absent means the loser adopts the
    /// winner's `Arc` and drops its own (accepted waste, no latch).
    ///
    /// A newest-generation miss consults the relation's **append base**
    /// first — the below-newest entry the last commits retained because
    /// they were delete-free for this relation (the lineage law,
    /// `CacheInner::map`). Three arms, decided by the snapshot's row
    /// count against the base's:
    /// - equal ⇒ **carry-forward**: the same immutable `Arc`, re-keyed at
    ///   the reader's generation (delete-free lineage + equal counts ⇒
    ///   zero new rows ⇒ identical content);
    /// - greater ⇒ **append**: a fresh frame, per-column prefix copy,
    ///   tail decode of only the new rows ([`crate::image::append`]);
    /// - less ⇒ typed `Corruption` (`RowCountMismatch`) — under the
    ///   lineage law only storage corruption shrinks a delete-free
    ///   relation's count; hard error, never a silent rebuild.
    ///
    /// EVERY insert — successor or full build — sweeps the relation's
    /// entries below its own generation in the same critical section
    /// (the lineage law's corollary, `CacheInner::map`: no entry
    /// outlives the next insert above it, so a full build whose
    /// snapshot raced ahead of the commit epilogue's `advance`
    /// supersedes the base it never probed instead of stranding it
    /// forever). No base — or a reader below `newest` — takes the full
    /// build exactly as before (below-newest readers stay query-local
    /// and never insert, though they may now hit a retained base at
    /// exactly their generation).
    ///
    /// A **closed** relation branches before the generation map is ever
    /// touched: its image is synthesized from the sealed extension — the
    /// theory is the storage, so there is no generation to key on, no
    /// LMDB read, no eviction. First touch builds into the relation's
    /// `OnceLock` slot; every later reader clones the same `Arc` forever.
    ///
    /// # Errors
    ///
    /// Build/append errors (`Lmdb`, `Corruption`) propagate; synthesis is
    /// pure and cannot fail.
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
        let (newest, base) = {
            let inner = self.inner.lock().expect("cache mutex");
            if let Some(cached) = inner.map.get(&key) {
                self.counters.hit();
                crate::obs::event(
                    crate::obs::names::CACHE_HIT,
                    crate::obs::Category::Cache,
                    u64::from(rel.0),
                    0,
                );
                return Ok(Arc::clone(&cached.image));
            }
            // The append-base probe (newest readers only — a stale reader
            // stays query-local and never inserts, so it never appends).
            // A linear map walk, but the map is O(relations) by the
            // lineage law's corollary, and this is still a panic-free
            // map operation under the lock.
            let base = (generation == inner.newest)
                .then(|| {
                    inner
                        .map
                        .iter()
                        .find(|((r, g), _)| *r == rel && *g < generation)
                        .map(|(_, cached)| Base {
                            image: Arc::clone(&cached.image),
                            row_id_next: cached.row_id_next,
                        })
                })
                .flatten();
            (inner.newest, base)
        };
        self.counters.miss();

        // Build, append, or carry outside the lock.
        let image = match base {
            Some(base) => self.extend(txn, schema, rel, &base)?,
            None => self.build_full(txn, schema, rel)?,
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

        // The append boundary the NEXT reader would extend from, read in
        // this same snapshot — one counter get, paid only on the insert
        // path. Under the one id allocator (R16) the counter is the
        // relation's: a fresh-keyed relation's boundary is its `Q` next
        // value (every committed row id sits strictly below it; a later
        // commit landing UNDER it is the non-tail insert `advance`
        // evicts on), a fresh-less relation's the `S` high-water.
        let row_id_next = match schema.relation(rel).fresh_row_field() {
            Some(field) => crate::storage::delta::read_fresh_next(txn, rel, field)?,
            None => read::row_id_high_water(txn, rel)?,
        };

        let mut inner = self.inner.lock().expect("cache mutex");
        // Re-check under the insert lock: a commit may have advanced past
        // this generation between the first lock and here — inserting
        // against the stale `newest` would undo the advance one entry at
        // a time and leak the image until the next state-changing commit.
        // The base entry stays untouched on this path: if the commit was
        // delete-free for `rel` it is still the lineage-lawful base.
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
                // The winner already replaced the base in its own
                // critical section — nothing to remove.
                Ok(Arc::clone(&winner.get().image))
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Cached {
                    image: Arc::clone(&image),
                    row_id_next,
                });
                // The insert supersedes EVERY older entry of its relation
                // — the same critical section, so no entry outlives the
                // next insert above it (the lineage law's corollary).
                // Removing only the probed base would strand one entry
                // per commit-epilogue race won (a full build whose
                // snapshot ran ahead of `newest` probes no base),
                // monotone forever on a never-deleted relation; sweeping
                // is always sound — map entries are pure caches, and
                // pinned readers keep their `Arc`s.
                inner.map.retain(|&(r, g), _| r != rel || g >= generation);
                Ok(image)
            }
        }
    }

    /// The from-scratch arm: one full LMDB scan and decode, exactly the
    /// pre-lineage miss path.
    fn build_full(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
    ) -> Result<Arc<RelationImage>> {
        let mut span = crate::obs::span_args(
            crate::obs::names::IMAGE_BUILD,
            crate::obs::Category::Image,
            u64::from(rel.0),
            0,
        );
        self.counters.build();
        let image = build(txn, schema, rel)?;
        span.set_args(u64::from(rel.0), image.byte_size() as u64);
        Ok(image)
    }

    /// The lineage arms over a surviving base: carry-forward, append, or
    /// typed corruption, decided by this snapshot's row count. Returns
    /// the image; the insert path's per-relation sweep retires the base.
    fn extend(
        &self,
        txn: &ReadTxn<'_>,
        schema: &Schema,
        rel: RelationId,
        base: &Base,
    ) -> Result<Arc<RelationImage>> {
        let claimed = read::row_count(txn, rel)?;
        let base_rows = base.image.row_count() as u64;
        let image = match claimed.cmp(&base_rows) {
            // Only corruption shrinks a delete-free relation's count —
            // hard error, never a skip (`append` types the same arm for
            // a count that shrank between the probe and its own read;
            // one snapshot, so the two reads agree by construction).
            std::cmp::Ordering::Less => {
                return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                    relation: rel,
                    stored: claimed,
                }));
            }
            // Zero new rows and images are immutable: the same `Arc`,
            // re-keyed at the reader's generation.
            std::cmp::Ordering::Equal => {
                self.counters.carry();
                crate::obs::event(
                    crate::obs::names::CACHE_CARRY,
                    crate::obs::Category::Cache,
                    u64::from(rel.0),
                    0,
                );
                Arc::clone(&base.image)
            }
            // New rows: fresh frame, per-column prefix copy, tail decode
            // from the base's append boundary.
            std::cmp::Ordering::Greater => {
                let mut span = crate::obs::span_args(
                    crate::obs::names::IMAGE_APPEND,
                    crate::obs::Category::Image,
                    u64::from(rel.0),
                    0,
                );
                self.counters.append();
                let image = append(txn, schema, rel, &base.image, base.row_id_next)?;
                span.set_args(u64::from(rel.0), image.byte_size() as u64);
                image
            }
        };
        Ok(image)
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
            self.counters.hit();
            crate::obs::event(
                crate::obs::names::CACHE_HIT,
                crate::obs::Category::Cache,
                u64::from(rel.0),
                0,
            );
            return Arc::clone(image);
        }
        self.counters.miss();
        let image = slot.get_or_init(|| {
            let mut span = crate::obs::span_args(
                crate::obs::names::IMAGE_BUILD,
                crate::obs::Category::Image,
                u64::from(rel.0),
                0,
            );
            self.counters.build();
            let image = synthesize_closed(rel, schema.relation(rel));
            span.set_args(u64::from(rel.0), image.byte_size() as u64);
            image
        });
        Arc::clone(image)
    }
}
