//! The environment image cache (docs/architecture/40-storage.md) — the mechanism whose absence was
//! v5's quietest failure (post-mortem §26).
//!
//! Keyed by `(relation, generation)` where generation is the reader's
//! *snapshot-sourced* storage tx id — never an in-process counter
//! (`docs/architecture/40-storage.md`'s race-closing rule). Retain-newest
//! eviction runs at each state-changing commit; readers pinned at older
//! generations keep their `Arc`s alive until their transactions end. There
//! is no memory-pressure eviction, ever — the scale axiom.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::image::{build, RelationImage};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;

struct CacheInner {
    map: HashMap<(RelationId, u64), Arc<RelationImage>>,
    /// The newest generation the cache has been evicted to. A reader below
    /// this builds query-locally without inserting (accepted — writes are
    /// bursty and rare).
    newest: u64,
}

/// The cross-transaction image cache, shared by reader threads. The mutex
/// covers map operations only — never a build.
pub struct ImageCache {
    inner: Mutex<CacheInner>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                newest: 0,
            }),
        }
    }

    /// Returns the image of `rel` at the reader's generation, building it
    /// outside the lock on a miss. Two same-generation readers racing to
    /// build may both build; insert-if-absent means the loser adopts the
    /// winner's `Arc` and drops its own (accepted waste, no latch).
    ///
    /// # Errors
    ///
    /// Build errors (`Lmdb`, `Corruption`) propagate.
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
        let generation = txn.generation()?;
        let key = (rel, generation);
        let newest = {
            let inner = self.inner.lock().expect("cache mutex");
            if let Some(image) = inner.map.get(&key) {
                return Ok(Arc::clone(image));
            }
            inner.newest
        };

        // Build outside the lock.
        let image = build(txn, schema, rel)?;

        // An old-generation reader keeps its image query-local: inserting it
        // would poison the map for nobody (its generation is already evicted).
        if generation < newest {
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
            std::collections::hash_map::Entry::Occupied(winner) => Ok(Arc::clone(winner.get())),
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Arc::clone(&image));
                Ok(image)
            }
        }
    }

    /// Retains only entries at or above `generation`; called by the write
    /// path after each state-changing commit (the 60-api doc wires `CommitReport`
    /// here). The map drop only releases the map's reference — pinned
    /// readers keep their images alive.
    ///
    /// # Panics
    ///
    /// Only on a poisoned cache mutex.
    pub fn evict_older_than(&self, generation: u64) {
        let mut inner = self.inner.lock().expect("cache mutex");
        inner.map.retain(|(_, gen), _| *gen >= generation);
        inner.newest = inner.newest.max(generation);
    }

    /// The set of `(relation, generation)` keys currently cached
    /// (test-only observability).
    #[cfg(test)]
    fn keys(&self) -> Vec<(RelationId, u64)> {
        let inner = self.inner.lock().expect("cache mutex");
        let mut keys: Vec<_> = inner.map.keys().copied().collect();
        keys.sort_unstable();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![FieldDescriptor {
                    name: "x".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
                }],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);

    fn fact(schema: &Schema, x: u64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(&[ValueRef::U64(x)], schema.relation(R).layout(), &mut b);
        b
    }

    fn insert_one(env: &Environment, schema: &Schema, x: u64) -> bool {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        delta.insert(&view, R, &fact(schema, x)).expect("insert");
        drop(view);
        commit(delta, env).expect("commit").changed
    }

    #[test]
    fn sequential_readers_share_one_image_instance() {
        let dir = TempDir::new("cache-shared");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_one(&env, &schema, 1);
        let cache = ImageCache::new();

        let txn1 = env.read_txn().expect("txn");
        let first = cache.get_or_build(&txn1, &schema, R).expect("build");
        drop(txn1);
        let txn2 = env.read_txn().expect("txn");
        let second = cache.get_or_build(&txn2, &schema, R).expect("build");
        // The v5 regression detector: no intervening write, identical Arc.
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn eviction_after_commit_leaves_only_the_new_generation() {
        let dir = TempDir::new("cache-evict");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_one(&env, &schema, 1);
        let cache = ImageCache::new();

        let old_txn = env.read_txn().expect("txn");
        let old_image = cache.get_or_build(&old_txn, &schema, R).expect("build");
        assert_eq!(old_image.row_count(), 1);
        assert_eq!(cache.keys(), vec![(R, 1)]);

        // A state-changing commit, then the writer evicts.
        insert_one(&env, &schema, 2);
        cache.evict_older_than(2);
        assert_eq!(cache.keys(), vec![]);

        // A new reader builds and caches the new generation.
        let new_txn = env.read_txn().expect("txn");
        let new_image = cache.get_or_build(&new_txn, &schema, R).expect("build");
        assert_eq!(new_image.row_count(), 2);
        assert_eq!(cache.keys(), vec![(R, 2)]);
        assert!(!Arc::ptr_eq(&old_image, &new_image));

        // The pinned old reader still reads its old image (its Arc lives on
        // past eviction), and its snapshot still answers at generation 1.
        assert_eq!(old_image.row_count(), 1);
        assert_eq!(old_txn.generation().expect("generation"), 1);
    }

    #[test]
    fn old_generation_miss_builds_without_populating_the_map() {
        let dir = TempDir::new("cache-old-miss");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_one(&env, &schema, 1);
        let cache = ImageCache::new();

        // Pin a reader at generation 1, then advance the world.
        let old_txn = env.read_txn().expect("txn");
        insert_one(&env, &schema, 2);
        cache.evict_older_than(2);

        // The old reader misses and builds query-locally: correct data for
        // its snapshot, and the map stays empty.
        let image = cache.get_or_build(&old_txn, &schema, R).expect("build");
        assert_eq!(image.row_count(), 1);
        assert_eq!(cache.keys(), vec![]);
    }

    #[test]
    fn concurrent_same_generation_builders_converge_on_one_arc() {
        let dir = TempDir::new("cache-race");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_one(&env, &schema, 1);
        let cache = ImageCache::new();

        let images = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    scope.spawn(|| {
                        let txn = env.read_txn().expect("txn");
                        cache.get_or_build(&txn, &schema, R).expect("build")
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("thread"))
                .collect::<Vec<_>>()
        });
        // Both may have built, but insert-if-absent hands every caller a
        // clone of one shared instance... unless the loser had already
        // returned before the winner inserted — impossible: adoption happens
        // under the same lock as insertion.
        assert!(Arc::ptr_eq(&images[0], &images[1]));
        assert_eq!(cache.keys(), vec![(R, 1)]);
    }

    #[test]
    fn a_no_op_commit_does_not_invalidate_the_cache() {
        let dir = TempDir::new("cache-noop");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_one(&env, &schema, 1);
        let cache = ImageCache::new();

        let txn = env.read_txn().expect("txn");
        let before = cache.get_or_build(&txn, &schema, R).expect("build");
        drop(txn);

        // Re-inserting an existing fact: changed == false, no eviction runs
        // (the 60-api doc only wires eviction for changed commits), tx id unmoved.
        assert!(!insert_one(&env, &schema, 1));

        let txn = env.read_txn().expect("txn");
        let after = cache.get_or_build(&txn, &schema, R).expect("build");
        assert!(Arc::ptr_eq(&before, &after), "the cache stayed warm");
    }
}
