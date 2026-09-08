//! The version-keyed image cache: hits at the same relation change version,
//! one rebuild per newer version, query-local images for old pinned
//! versions, per-execution rebuilds for heap ticks, owner-scoped closed
//! synthesis, and explicit cache clearing. (The end-to-end per-relation
//! invalidation contract over a real store lives in
//! `image/tests/relation_reuse.rs`.)
use std::sync::Arc;

use crate::image::ViewEpoch;
use crate::image::testsupport::TestSource;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::store::RelationVersion;
use bumbledb_theory::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, Row, SchemaDescriptor, ValueType,
};

use super::ImageCache;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                    },
                ],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Open".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Frozen".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Status".into(),
                fields: vec![],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);
const STATUS: RelationId = RelationId(1);

fn fixture() -> TestSource {
    let rows: Vec<Vec<Value>> = (0..8u64)
        .map(|i| vec![Value::U64(i), Value::String(format!("row-{i}").into())])
        .collect();
    TestSource::new(&schema(), &[(R, rows)])
}

fn generation(value: u64) -> ViewEpoch {
    ViewEpoch::Store(RelationVersion::from_storage(value))
}

#[test]
fn same_generation_reads_hit_the_memo() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let first = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(3))
        .expect("build");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(3))
        .expect("hit");
    assert!(
        Arc::ptr_eq(&first, &second),
        "the same generation returns the same image"
    );
    assert!(
        cache.peek_at(R, generation(3), &cache.acquire()).is_some(),
        "the memo is peekable without building"
    );
}

#[test]
fn a_newer_generation_rebuilds_and_retires_the_old_entry() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(3))
        .expect("build old");
    let new = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(4))
        .expect("build new");
    assert!(!Arc::ptr_eq(&old, &new), "a newer generation rebuilds");
    assert!(
        cache.peek_at(R, generation(3), &cache.acquire()).is_none(),
        "the old generation's entry retired when the newer one landed"
    );
    assert!(cache.peek_at(R, generation(4), &cache.acquire()).is_some());
    // The pinned reader's Arc keeps the old image alive query-local.
    assert_eq!(old.row_count(), new.row_count());
}

#[test]
fn an_old_pinned_generation_builds_query_local_after_a_newer_landed() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let _new = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(9))
        .expect("build new");
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(2))
        .expect("query-local build");
    assert_eq!(old.row_count(), 8, "the old snapshot still gets its image");
    assert!(
        cache.peek_at(R, generation(2), &cache.acquire()).is_none(),
        "old generations never displace the newest memo"
    );
    assert!(
        cache.peek_at(R, generation(9), &cache.acquire()).is_some(),
        "the newest memo survives the pinned reader"
    );
}

#[test]
fn heap_ticks_never_memoize() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let epoch = source.relation_epoch(R).expect("heap epoch");
    assert!(matches!(epoch, ViewEpoch::Heap(_)), "heap fixture");
    let first = cache
        .get_or_build_at(&source, fixture.schema(), R, epoch)
        .expect("build");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), R, epoch)
        .expect("rebuild");
    assert!(
        !Arc::ptr_eq(&first, &second),
        "a heap execution rebuilds every time — no durable identity to key by"
    );
    assert!(
        cache.peek_at(R, epoch, &cache.acquire()).is_none(),
        "nothing was cached"
    );
}

#[test]
fn closed_relations_synthesize_once_per_owner_and_trim_detaches_them() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let first = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("synthesize");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("hit");
    assert!(Arc::ptr_eq(&first, &second), "closed images build once");
    assert_eq!(first.row_count(), 2);

    cache.clear();
    let after = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("still resident");
    assert!(
        !Arc::ptr_eq(&first, &after),
        "closed images rebuild with the new resolver owner"
    );
    assert!(!first.generation().ptr_eq(after.generation()));
    assert_eq!(
        first.row_count(),
        after.row_count(),
        "old pinned image remains valid"
    );
}

#[test]
fn late_old_owner_builds_never_repopulate_or_displace_current_images() {
    let fixture = fixture();
    let source = fixture.source();
    for (relation, epoch) in [(R, generation(1)), (STATUS, ViewEpoch::Closed)] {
        let cache = ImageCache::new(fixture.schema());
        let old_owner = cache.acquire();
        cache.clear();
        let current_owner = cache.acquire();
        let late = cache
            .get_or_build_with(&source, fixture.schema(), relation, epoch, &old_owner)
            .unwrap();
        assert!(late.generation().ptr_eq(&old_owner));
        assert!(
            cache.peek_at(relation, epoch, &old_owner).is_none(),
            "retired owner cannot repopulate an empty slot"
        );
        let current = cache
            .get_or_build_with(&source, fixture.schema(), relation, epoch, &current_owner)
            .unwrap();
        let late_again = cache
            .get_or_build_with(&source, fixture.schema(), relation, epoch, &old_owner)
            .unwrap();
        assert!(late_again.generation().ptr_eq(&old_owner));
        assert!(!Arc::ptr_eq(&late_again, &current));
        assert!(
            cache.peek_at(relation, epoch, &old_owner).is_none(),
            "peek never lends a foreign resolver's tokens"
        );
        assert!(
            Arc::ptr_eq(
                &current,
                &cache.peek_at(relation, epoch, &current_owner).unwrap()
            ),
            "late retired builder cannot displace current owner's cache entry"
        );
        let weak_old = old_owner.downgrade();
        drop(late);
        drop(late_again);
        drop(old_owner);
        assert!(
            weak_old.upgrade().is_none(),
            "no stale slot permanently pins the retired resolver"
        );
    }
}

#[test]
fn closed_images_release_the_retired_owner_when_the_last_reader_drops() {
    // Closed relations cannot contain text. They nevertheless share the
    // operation's generation owner and must not keep a retired resolver
    // alive forever through an unevictable cache entry.
    let fixture = fixture();
    let source = fixture.source();
    let cache = ImageCache::new(fixture.schema());
    let old = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .unwrap();
    let old_owner = old.generation().downgrade();
    let pinned = Arc::clone(&old);
    cache.clear();
    let current_owner = cache.acquire();
    let current = cache
        .get_or_build_with(
            &source,
            fixture.schema(),
            STATUS,
            ViewEpoch::Closed,
            &current_owner,
        )
        .unwrap();
    assert!(current.generation().ptr_eq(&current_owner));
    assert!(!current_owner.ptr_eq(old.generation()));
    assert_eq!(old.column_words(0), &[0, 1]);
    assert_eq!(old.column_words(0), current.column_words(0));
    #[cfg(feature = "alloc-counter")]
    let before = crate::alloc_counter::snapshot().window;
    drop(old);
    assert!(
        old_owner.upgrade().is_some(),
        "the pinned reader keeps its owner"
    );
    #[cfg(feature = "alloc-counter")]
    assert_eq!(crate::alloc_counter::snapshot().window, before);
    drop(pinned);
    assert!(
        old_owner.upgrade().is_none(),
        "trim detached the old closed cache entry"
    );
    #[cfg(feature = "alloc-counter")]
    assert!(crate::alloc_counter::snapshot().window.dealloc_bytes > before.dealloc_bytes);
    assert!(Arc::ptr_eq(
        &current,
        &cache
            .peek_at(STATUS, ViewEpoch::Closed, &current_owner)
            .unwrap()
    ));
}

#[test]
fn clear_detaches_cache_entries_without_invalidating_a_held_image() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .unwrap();
    let weak_image = Arc::downgrade(&image);
    let weak_generation = image.generation().downgrade();
    cache.clear();
    assert!(cache.peek_at(R, generation(1), &cache.acquire()).is_none());
    assert_eq!(cache.image_count(), 0);
    assert!(weak_image.upgrade().is_some());
    assert!(
        image
            .generation()
            .resolver()
            .with_text(image.column_words(1)[0], |text| text == "row-0")
            .unwrap_or(false),
        "old tokens still resolve after rotation"
    );
    #[cfg(feature = "alloc-counter")]
    let before = crate::alloc_counter::snapshot().window;
    drop(image);
    #[cfg(feature = "alloc-counter")]
    assert!(
        crate::alloc_counter::snapshot().window.dealloc_bytes > before.dealloc_bytes,
        "the last image releases real storage"
    );
    assert!(weak_image.upgrade().is_none());
    assert!(weak_generation.upgrade().is_none());
}

#[test]
fn rotation_does_not_alias_old_and_new_tokens() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build");
    let token_before = old.column_words(1)[0];
    let old_generation = old.generation().clone();
    assert_eq!(cache.cache_generation().as_u64(), 0);
    cache.clear();
    assert_eq!(cache.cache_generation().as_u64(), 1);
    let rebuilt = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("rebuild");
    let token_after = rebuilt.column_words(1)[0];
    assert!(
        !old_generation.ptr_eq(rebuilt.generation()),
        "rotation installs a distinct resolver"
    );
    assert!(
        old_generation.tokens_equal(token_before, rebuilt.generation(), token_after),
        "same canonical text remaps exactly across generations"
    );
}

#[test]
fn acquire_is_the_production_pin_and_idle_memos_are_weak() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build");
    let handle = cache.acquire();
    assert!(handle.ptr_eq(image.generation()));
    let weak = cache.weak_current();
    assert!(weak.upgrade().is_some());
    cache.clear();
    assert!(
        weak.upgrade().is_some(),
        "a retained image keeps the old generation alive"
    );
    assert_ne!(
        cache.acquire().identity(),
        handle.identity(),
        "trim rotates current; idle memos must upgrade or rebuild"
    );
}

/// D02: two retained text-bearing images; trim A; ingest different texts;
/// B still resolves the pinned generation. Concurrent trim/admit must not
/// alias tokens; live image owners retain their text.
#[test]
fn d02_shared_meanings_survive_trim_and_do_not_alias() {
    let first = fixture();
    let cache = ImageCache::new(first.schema());
    let source_a = first.source();
    let image_a = cache
        .get_or_build_at(&source_a, first.schema(), R, generation(1))
        .expect("A");
    let rows_b: Vec<Vec<Value>> = (0..8u64)
        .map(|i| vec![Value::U64(i), Value::String(format!("other-{i}").into())])
        .collect();
    let second = TestSource::new(first.schema(), &[(R, rows_b)]);
    let source_b = second.source();
    let image_b = cache
        .get_or_build_at(&source_b, first.schema(), R, generation(1))
        .expect("B hits A's memo");
    assert!(
        Arc::ptr_eq(&image_a, &image_b),
        "same version shares one image"
    );

    let pinned = image_a.generation().clone();
    let token = image_a.column_words(1)[0];
    cache.clear();
    let rebuilt = cache
        .get_or_build_at(&source_b, first.schema(), R, generation(2))
        .expect("new generation after ingest");
    assert!(
        pinned
            .resolver()
            .with_text(token, |text| text == "row-0")
            .unwrap_or(false),
        "pinned B still sees the old snapshot texts"
    );
    assert!(
        rebuilt
            .generation()
            .resolver()
            .with_text(rebuilt.column_words(1)[0], |text| text == "other-0")
            .unwrap_or(false),
        "new admission interns the ingested texts"
    );
    assert!(
        !pinned.ptr_eq(rebuilt.generation()),
        "rotation does not alias old/new resolvers"
    );
}

#[test]
fn d02_concurrent_trim_and_admit_keep_pinned_meanings() {
    let fixture = fixture();
    let cache = std::sync::Arc::new(ImageCache::new(fixture.schema()));
    let source = fixture.source();
    let pinned = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("pin");
    let handle = pinned.generation().clone();
    let token = pinned.column_words(1)[0];

    std::thread::scope(|scope| {
        let cache_trim = std::sync::Arc::clone(&cache);
        scope.spawn(move || {
            for _ in 0..8 {
                cache_trim.clear();
            }
        });
        let cache_build = std::sync::Arc::clone(&cache);
        scope.spawn(move || {
            let source = fixture.source();
            for version in 2..6u64 {
                let _ =
                    cache_build.get_or_build_at(&source, fixture.schema(), R, generation(version));
            }
        });
    });

    assert!(
        handle
            .resolver()
            .with_text(token, |text| text == "row-0")
            .unwrap_or(false),
        "concurrent trim/admit cannot rewrite a pinned resolver"
    );
}

#[test]
fn ordinary_and_closed_images_release_independently_of_cache_membership() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .unwrap();
    let closed = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .unwrap();
    let weak_image = Arc::downgrade(&image);
    let weak_closed = Arc::downgrade(&closed);
    let weak_generation = image.generation().downgrade();
    let clone = Arc::clone(&image);
    cache.clear();
    assert_eq!(cache.image_count(), 0);
    assert!(
        image
            .generation()
            .resolver()
            .with_text(image.column_words(1)[0], |text| text == "row-0")
            .unwrap_or(false)
    );
    drop(clone);
    assert!(weak_image.upgrade().is_some());
    drop(image);
    assert!(weak_image.upgrade().is_none());
    assert!(weak_closed.upgrade().is_some());
    assert!(
        weak_generation.upgrade().is_some(),
        "closed reader still owns this generation"
    );
    assert_eq!(closed.column_words(0), &[0, 1]);
    drop(closed);
    assert!(weak_closed.upgrade().is_none());
    assert!(weak_generation.upgrade().is_none());
}

fn numeric_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Nums".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid numeric fixture")
}

#[test]
fn numeric_images_survive_clear_and_release_after_the_last_reader() {
    let schema = numeric_schema();
    let rows: Vec<_> = (0..4u64).map(|i| vec![Value::U64(i)]).collect();
    let fixture = TestSource::new(&schema, &[(RelationId(0), rows)]);
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), RelationId(0), generation(1))
        .unwrap();
    let weak_image = Arc::downgrade(&image);
    let weak_generation = image.generation().downgrade();
    cache.clear();
    assert_eq!(cache.image_count(), 0);
    assert_eq!(image.row_count(), 4);
    assert_eq!(image.column_words(0), &[0, 1, 2, 3]);
    drop(image);
    assert!(weak_image.upgrade().is_none());
    assert!(weak_generation.upgrade().is_none());
}
