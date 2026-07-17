use super::*;
use crate::encoding::{ValueRef, encode_fact};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::{Environment, GenerationId};
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "x".into(),
                value_type: ValueType::U64,
                generation: Generation::Fresh,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

const fn gid(word: u64) -> GenerationId {
    GenerationId::from_storage(word)
}

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
    let cache = ImageCache::new(&schema);

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
    let cache = ImageCache::new(&schema);

    let old_txn = env.read_txn().expect("txn");
    let old_image = cache.get_or_build(&old_txn, &schema, R).expect("build");
    assert_eq!(old_image.row_count(), 1);
    assert_eq!(cache.keys(), vec![(R, gid(1))]);

    // A state-changing commit, then the writer evicts.
    insert_one(&env, &schema, 2);
    cache.evict_older_than(gid(2));
    assert_eq!(cache.keys(), vec![]);

    // A new reader builds and caches the new generation.
    let new_txn = env.read_txn().expect("txn");
    let new_image = cache.get_or_build(&new_txn, &schema, R).expect("build");
    assert_eq!(new_image.row_count(), 2);
    assert_eq!(cache.keys(), vec![(R, gid(2))]);
    assert!(!Arc::ptr_eq(&old_image, &new_image));

    // The pinned old reader still reads its old image (its Arc lives on
    // past eviction), and its snapshot still answers at generation 1.
    assert_eq!(old_image.row_count(), 1);
    assert_eq!(old_txn.generation().expect("generation").value(), 1);
}

#[test]
fn old_generation_miss_builds_without_populating_the_map() {
    let dir = TempDir::new("cache-old-miss");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_one(&env, &schema, 1);
    let cache = ImageCache::new(&schema);

    // Pin a reader at generation 1, then advance the world.
    let old_txn = env.read_txn().expect("txn");
    insert_one(&env, &schema, 2);
    cache.evict_older_than(gid(2));

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
    let cache = ImageCache::new(&schema);

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
    assert_eq!(cache.keys(), vec![(R, gid(1))]);
}

#[test]
fn a_no_op_commit_does_not_invalidate_the_cache() {
    let dir = TempDir::new("cache-noop");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_one(&env, &schema, 1);
    let cache = ImageCache::new(&schema);

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

/// R(x u64 fresh) plus the closed Currency { `minor_units` } = { Usd(2),
/// Eur(0) }: the ordinary relation drives generations, the closed one
/// lives outside them.
fn closed_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![FieldDescriptor {
                    name: "x".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                }],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    bumbledb_theory::schema::Row {
                        handle: "Usd".into(),
                        values: Box::new([crate::ir::Value::U64(2)]),
                    },
                    bumbledb_theory::schema::Row {
                        handle: "Eur".into(),
                        values: Box::new([crate::ir::Value::U64(0)]),
                    },
                ])),
                name: "Currency".into(),
                fields: vec![FieldDescriptor {
                    name: "minor_units".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                }],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const CURRENCY: RelationId = RelationId(1);

/// The closed image is synthesized once into its `OnceLock` slot — every
/// reader shares one Arc, the generation map never sees it, and a
/// state-changing commit plus eviction leaves it untouched (never
/// evicted, never rebuilt).
#[test]
fn closed_images_synthesize_once_and_survive_eviction() {
    let dir = TempDir::new("cache-closed");
    let schema = closed_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);

    let txn = env.read_txn().expect("txn");
    let first = cache.get_or_build(&txn, &schema, CURRENCY).expect("build");
    let second = cache.get_or_build(&txn, &schema, CURRENCY).expect("build");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.row_count(), 2);
    assert_eq!(cache.keys(), vec![], "never in the generation map");
    drop(txn);

    // A state-changing commit + eviction: the slot is untouched by
    // construction — it is not in the generation-keyed map at all.
    assert!(insert_one(&env, &schema, 1));
    cache.evict_older_than(gid(u64::MAX));
    let txn = env.read_txn().expect("txn");
    let third = cache.get_or_build(&txn, &schema, CURRENCY).expect("build");
    assert!(Arc::ptr_eq(&first, &third), "warm across every generation");

    // `peek` sees the resident slot without a build — same Arc.
    let peeked = cache
        .peek(&txn, CURRENCY)
        .expect("peek")
        .expect("resident forever");
    assert!(Arc::ptr_eq(&first, &peeked));
}

#[cfg(feature = "trace")]
#[test]
fn counters_track_hit_miss_build_evict_exactly() {
    let dir = TempDir::new("cache-stats");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    assert!(insert_one(&env, &schema, 1));
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let base = cache.stats();
    cache.get_or_build(&txn, &schema, R).expect("build"); // miss + build
    cache.get_or_build(&txn, &schema, R).expect("hit"); // hit
    let after = cache.stats();
    assert_eq!(after.misses - base.misses, 1);
    assert_eq!(after.builds - base.builds, 1);
    assert_eq!(after.hits - base.hits, 1);

    let (images, bytes) = cache.resident();
    assert_eq!(images, 1);
    assert!(bytes > 0);

    cache.evict_older_than(gid(u64::MAX));
    let evicted = cache.stats();
    assert_eq!(evicted.evicted - after.evicted, 1);
    assert_eq!(cache.resident(), (0, 0));
}
