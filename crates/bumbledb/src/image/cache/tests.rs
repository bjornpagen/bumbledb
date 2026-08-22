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
    commit(delta, env)
        .expect("commit")
        .expect("admitted")
        .changed()
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

    assert!(Arc::ptr_eq(&first, &second));
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

    // returned before the winner inserted — impossible: adoption happens

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

    assert!(!insert_one(&env, &schema, 1));

    let txn = env.read_txn().expect("txn");
    let after = cache.get_or_build(&txn, &schema, R).expect("build");
    assert!(Arc::ptr_eq(&before, &after), "the cache stayed warm");
}

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

    assert!(insert_one(&env, &schema, 1));
    cache.advance(gid(u64::MAX), &[R], &[]);
    let txn = env.read_txn().expect("txn");
    let third = cache.get_or_build(&txn, &schema, CURRENCY).expect("build");
    assert!(Arc::ptr_eq(&first, &third), "warm across every generation");

    let peeked = cache
        .peek(&txn, &schema, CURRENCY)
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
    cache.get_or_build(&txn, &schema, R).expect("build"); 
    cache.get_or_build(&txn, &schema, R).expect("hit"); 
    let after = cache.stats();
    assert_eq!(after.misses - base.misses, 1);
    assert_eq!(after.builds - base.builds, 1);
    assert_eq!(after.hits - base.hits, 1);

    let (images, bytes) = cache.resident();
    assert_eq!(images, 1);
    assert!(bytes > 0);

    cache.advance(gid(u64::MAX), &[R], &[]);
    let evicted = cache.stats();
    assert_eq!(evicted.evicted - after.evicted, 1);
    assert_eq!(cache.resident(), (0, 0));
}

const A: RelationId = RelationId(0);
const B: RelationId = RelationId(1);

fn two_relation_schema() -> Schema {
    let rel = |name: &str| RelationDescriptor {
        extension: None,
        name: name.into(),
        fields: vec![FieldDescriptor {
            name: "x".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        }],
    };
    SchemaDescriptor {
        relations: vec![rel("A"), rel("B")],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn rel_fact(schema: &Schema, rel: RelationId, x: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(&[ValueRef::U64(x)], schema.relation(rel).layout(), &mut b);
    b
}

fn commit_and_advance(
    env: &Environment,
    cache: &ImageCache,
    delta: WriteDelta<'_>,
) -> GenerationId {
    let dirty = delta.dirty_relations();
    let floors = delta.inserted_floors();
    let report = commit(delta, env).expect("commit").expect("admitted");
    assert!(report.changed(), "the fixture commits are state-changing");
    cache.advance(report.generation(), &dirty, &floors);
    report.generation()
}

fn assert_images_identical(a: &RelationImage, b: &RelationImage, columns: usize) {
    assert_eq!(a.row_count(), b.row_count());
    for column in 0..columns {
        assert_eq!(a.column(column), b.column(column), "column {column}");
        assert_eq!(
            a.distinct_count(column),
            b.distinct_count(column),
            "distinct {column}"
        );
    }
}

#[test]
fn a_below_boundary_insert_evicts_the_append_base() {
    let dir = TempDir::new("cache-non-tail-evict");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);

    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &fact(&schema, 5)).expect("insert");
        drop(view);
        commit_and_advance_one(&env, &cache, delta);
    }
    {
        let txn = env.read_txn().expect("txn");
        cache.get_or_build(&txn, &schema, R).expect("base build");
    }
    assert_eq!(cache.keys(), vec![(R, gid(1))]);

    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &fact(&schema, 2)).expect("insert");
        drop(view);
        commit_and_advance_one(&env, &cache, delta);
    }
    assert_eq!(cache.keys(), vec![], "the non-tail insert evicted the base");
    {
        let txn = env.read_txn().expect("txn");
        let image = cache.get_or_build(&txn, &schema, R).expect("full build");
        assert_eq!(image.row_count(), 2);
        assert_eq!(
            image.column(0),
            crate::image::ColumnView::Words(&[2, 5]),
            "scan order is fresh order"
        );
    }
    assert_eq!(cache.keys(), vec![(R, gid(2))]);

    // boundary (Q stayed 6 after the below-boundary insert): the base

    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &fact(&schema, 7)).expect("insert");
        drop(view);
        commit_and_advance_one(&env, &cache, delta);
    }
    assert_eq!(
        cache.keys(),
        vec![(R, gid(2))],
        "the tail insert retained the base"
    );
    let txn = env.read_txn().expect("txn");
    let image = cache.get_or_build(&txn, &schema, R).expect("append");
    assert_eq!(image.row_count(), 3);
    assert_eq!(image.column(0), crate::image::ColumnView::Words(&[2, 5, 7]));
}

fn commit_and_advance_one(env: &Environment, cache: &ImageCache, delta: WriteDelta<'_>) {
    let dirty = delta.dirty_relations();
    let floors = delta.inserted_floors();
    let report = commit(delta, env).expect("commit").expect("admitted");
    assert!(report.changed());
    cache.advance(report.generation(), &dirty, &floors);
}

#[test]
fn advance_drops_dirty_relations_and_retains_the_rest() {
    let dir = TempDir::new("cache-advance");
    let schema = two_relation_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .insert(&view, A, &rel_fact(&schema, A, 1))
            .expect("insert");
        delta
            .insert(&view, B, &rel_fact(&schema, B, 1))
            .expect("insert");
        drop(view);
        commit_and_advance(&env, &cache, delta);
    }
    let txn = env.read_txn().expect("txn");
    cache.get_or_build(&txn, &schema, A).expect("build A");
    cache.get_or_build(&txn, &schema, B).expect("build B");
    drop(txn);
    assert_eq!(cache.keys(), vec![(A, gid(1)), (B, gid(1))]);

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, A, &rel_fact(&schema, A, 1))
        .expect("delete");
    delta
        .insert(&view, B, &rel_fact(&schema, B, 2))
        .expect("insert");
    drop(view);
    commit_and_advance(&env, &cache, delta);
    assert_eq!(cache.keys(), vec![(B, gid(1))]);
}

#[test]
fn an_untouched_relation_carries_the_same_arc_forward() {
    let dir = TempDir::new("cache-carry");
    let schema = two_relation_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .insert(&view, A, &rel_fact(&schema, A, 1))
            .expect("insert");
        delta
            .insert(&view, B, &rel_fact(&schema, B, 1))
            .expect("insert");
        drop(view);
        commit_and_advance(&env, &cache, delta);
    }
    let txn = env.read_txn().expect("txn");
    let b_before = cache.get_or_build(&txn, &schema, B).expect("build B");
    drop(txn);

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, A, &rel_fact(&schema, A, 2))
        .expect("insert");
    drop(view);
    commit_and_advance(&env, &cache, delta);

    let txn = env.read_txn().expect("txn");
    let b_after = cache.get_or_build(&txn, &schema, B).expect("carry B");
    assert!(
        Arc::ptr_eq(&b_before, &b_after),
        "identical content, identical Arc — the carry-forward is zero-copy"
    );
    assert_eq!(
        cache.keys(),
        vec![(B, gid(2))],
        "B re-keyed at the new generation, its base key removed (A was never read)"
    );
}

#[test]
fn chained_insert_only_commits_append_once_and_match_a_full_rebuild() {
    let dir = TempDir::new("cache-append-chain");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    insert_one(&env, &schema, 1);
    {

        let txn = env.read_txn().expect("txn");
        let base = cache.get_or_build(&txn, &schema, R).expect("base build");
        assert_eq!(base.row_count(), 1);
    }

    for x in [2, 3, 4] {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, R, &fact(&schema, x)).expect("insert");
        drop(view);
        commit_and_advance(&env, &cache, delta);
    }
    assert_eq!(
        cache.keys(),
        vec![(R, gid(1))],
        "the base outlives the chain"
    );

    let txn = env.read_txn().expect("txn");
    let appended = cache.get_or_build(&txn, &schema, R).expect("append");
    assert_eq!(
        appended.row_count(),
        4,
        "one append covered all three commits"
    );
    let rebuilt = crate::image::build(&txn.catalog(), &schema, R).expect("from-scratch rebuild");
    assert_images_identical(&appended, &rebuilt, 1);
    assert_eq!(
        cache.keys(),
        vec![(R, gid(4))],
        "the successor replaced its base"
    );

    let again = cache.get_or_build(&txn, &schema, R).expect("hit");
    assert!(Arc::ptr_eq(&appended, &again));
}

#[test]
fn an_epilogue_racing_full_build_supersedes_the_surviving_base() {
    let dir = TempDir::new("cache-race-sweep");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);

    insert_one(&env, &schema, 1);
    {
        let txn = env.read_txn().expect("txn");
        cache.get_or_build(&txn, &schema, R).expect("base build");
    }
    cache.advance(gid(1), &[], &[]);
    assert_eq!(cache.keys(), vec![(R, gid(1))]);

    // opens BEFORE the epilogue's `advance`, so `newest` is still 1 and

    insert_one(&env, &schema, 2);
    let racing = env.read_txn().expect("txn");
    let image = cache.get_or_build(&racing, &schema, R).expect("full build");
    assert_eq!(image.row_count(), 2);
    assert_eq!(
        cache.keys(),
        vec![(R, gid(2))],
        "the racing insert sweeps the stranded base instead of leaking it"
    );

    cache.advance(gid(2), &[], &[]);
    assert_eq!(cache.keys(), vec![(R, gid(2))]);
}

#[test]
fn a_count_below_the_base_is_typed_corruption_never_a_skip() {
    let dir = TempDir::new("cache-append-shrink");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    insert_one(&env, &schema, 1);
    insert_one(&env, &schema, 2);
    {
        let txn = env.read_txn().expect("txn");
        let base = cache.get_or_build(&txn, &schema, R).expect("base build");
        assert_eq!(base.row_count(), 2);
    }

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, R, &fact(&schema, 1)).expect("delete");
    drop(view);
    let report = commit(delta, &env).expect("commit").expect("admitted");
    assert!(report.changed());
    cache.advance(report.generation(), &[], &[]);

    let txn = env.read_txn().expect("txn");
    let err = cache
        .get_or_build(&txn, &schema, R)
        .expect_err("shrunk count");
    assert!(
        matches!(
            err,
            crate::error::Error::Corruption(crate::error::CorruptionError::RowCountMismatch {
                relation: R,
                stored: 1
            })
        ),
        "{err:?}"
    );
}

#[cfg(feature = "trace")]
#[test]
fn counters_pin_the_per_relation_arm_selection() {
    let dir = TempDir::new("cache-arm-pin");
    let schema = two_relation_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let cache = ImageCache::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .insert(&view, A, &rel_fact(&schema, A, 1))
            .expect("insert");
        delta
            .insert(&view, B, &rel_fact(&schema, B, 1))
            .expect("insert");
        drop(view);
        commit_and_advance(&env, &cache, delta);
    }
    let txn = env.read_txn().expect("txn");
    cache.get_or_build(&txn, &schema, A).expect("build A");
    cache.get_or_build(&txn, &schema, B).expect("build B");
    drop(txn);
    let seeded = cache.stats();
    assert_eq!((seeded.builds, seeded.appends, seeded.carries), (2, 0, 0));

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .delete(&view, A, &rel_fact(&schema, A, 1))
        .expect("delete");
    delta
        .insert(&view, B, &rel_fact(&schema, B, 2))
        .expect("insert");
    drop(view);
    commit_and_advance(&env, &cache, delta);

    let txn = env.read_txn().expect("txn");
    cache.get_or_build(&txn, &schema, A).expect("rebuild A");
    let after_a = cache.stats();
    assert_eq!(
        (after_a.builds, after_a.appends, after_a.carries),
        (3, 0, 0),
        "the deleted-from relation rebuilds from scratch"
    );
    cache.get_or_build(&txn, &schema, B).expect("append B");
    let after_b = cache.stats();
    assert_eq!(
        (after_b.builds, after_b.appends, after_b.carries),
        (3, 1, 0),
        "the delete-free relation appends"
    );
    drop(txn);

    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta
        .insert(&view, A, &rel_fact(&schema, A, 9))
        .expect("insert");
    drop(view);
    commit_and_advance(&env, &cache, delta);
    let txn = env.read_txn().expect("txn");
    cache.get_or_build(&txn, &schema, A).expect("append A");
    cache.get_or_build(&txn, &schema, B).expect("carry B");
    let end = cache.stats();
    assert_eq!(
        (end.builds, end.appends, end.carries),
        (3, 2, 1),
        "insert-only: the touched relation appends, the untouched one carries"
    );
}
