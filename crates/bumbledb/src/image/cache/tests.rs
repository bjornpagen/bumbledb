//! The version-keyed image cache: hits at the same relation change version,
//! one rebuild per newer version, query-local images for old pinned
//! versions, per-execution rebuilds for heap ticks, once-only closed
//! synthesis, and the memory-pressure trim. (The end-to-end per-relation
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
        .expect("build")
        .expect_ready("resident");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(3))
        .expect("hit")
        .expect_ready("resident");
    assert!(
        Arc::ptr_eq(&first, &second),
        "the same generation returns the same image"
    );
    assert!(
        cache.peek_at(R, generation(3)).is_some(),
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
        .expect("build old")
        .expect_ready("resident");
    let new = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(4))
        .expect("build new")
        .expect_ready("resident");
    assert!(!Arc::ptr_eq(&old, &new), "a newer generation rebuilds");
    assert!(
        cache.peek_at(R, generation(3)).is_none(),
        "the old generation's entry retired when the newer one landed"
    );
    assert!(cache.peek_at(R, generation(4)).is_some());
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
        .expect("build new")
        .expect_ready("resident");
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(2))
        .expect("query-local build")
        .expect_ready("resident");
    assert_eq!(old.row_count(), 8, "the old snapshot still gets its image");
    assert!(
        cache.peek_at(R, generation(2)).is_none(),
        "old generations never displace the newest memo"
    );
    assert!(
        cache.peek_at(R, generation(9)).is_some(),
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
        .expect("build")
        .expect_ready("resident");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), R, epoch)
        .expect("rebuild")
        .expect_ready("resident");
    assert!(
        !Arc::ptr_eq(&first, &second),
        "a heap execution rebuilds every time — no durable identity to key by"
    );
    assert!(cache.peek_at(R, epoch).is_none(), "nothing was cached");
}

#[test]
fn closed_relations_synthesize_once_and_never_trim() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let first = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("synthesize")
        .expect_ready("resident");
    let second = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("hit")
        .expect_ready("resident");
    assert!(Arc::ptr_eq(&first, &second), "closed images build once");
    assert_eq!(first.row_count(), 2);

    cache.trim();
    let after = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("still resident")
        .expect_ready("resident");
    assert!(
        Arc::ptr_eq(&first, &after),
        "trim never evicts a closed image"
    );
}

#[test]
fn trim_detaches_map_entries_without_refunding_a_held_image() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build")
        .expect_ready("resident");
    let charged = cache.cache_ledger().used();
    assert!(charged > 0, "admission reserved the slab");

    cache.trim();
    assert!(
        cache.peek_at(R, generation(1)).is_none(),
        "trim evicts generation-keyed map entries"
    );
    assert_eq!(
        cache.cache_ledger().used(),
        charged,
        "D29: dropping cache membership does not refund a retained image"
    );
    assert!(
        image
            .generation()
            .resolver()
            .with_text(image.column_words(1)[0], |text| text == "row-0")
            .unwrap_or(false),
        "old tokens still resolve after rotation"
    );

    drop(image);
    assert!(
        cache.cache_ledger().used() < charged,
        "the last strong owner refunds the slab"
    );
}

#[test]
fn rotation_does_not_alias_old_and_new_tokens() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build")
        .expect_ready("resident");
    let token_before = old.column_words(1)[0];
    let old_generation = old.generation().clone();
    assert_eq!(cache.cache_generation().as_u64(), 0);
    cache.trim();
    assert_eq!(cache.cache_generation().as_u64(), 1);
    let rebuilt = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("rebuild")
        .expect_ready("resident");
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
        .expect("build")
        .expect_ready("resident");
    let handle = cache.acquire();
    assert!(handle.ptr_eq(image.generation()));
    let weak = cache.weak_current();
    assert!(weak.upgrade().is_some());
    cache.trim();
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

#[test]
fn resident_images_charge_the_shared_cache_ledger() {
    use crate::work::CachePolicy;
    let fixture = fixture();
    let policy = CachePolicy {
        cache_bytes: 1 << 20,
    };
    let cache = ImageCache::with_policy(fixture.schema(), policy);
    let source = fixture.source();
    assert_eq!(cache.cache_ledger().used(), 0);
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build")
        .expect_ready("resident");
    assert!(
        cache.cache_ledger().used() >= image.charged_bytes().unwrap_or(0),
        "admitted images reserve retained bytes against the shared ledger"
    );
    assert!(
        image.charged_bytes().is_some(),
        "charge lives inside the shared image, not the map entry"
    );
}

/// D01: cache admission refuses before slab allocate when the allowance
/// cannot cover the estimated image.
#[test]
fn d01_zero_cache_refuses_before_image_allocate() {
    use crate::work::CachePolicy;
    let fixture = fixture();
    let cache = ImageCache::with_policy(
        fixture.schema(),
        CachePolicy { cache_bytes: 0 },
    );
    let source = fixture.source();
    let crate::image::ResidentAdmit::BeyondMemory(exhausted) = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("typed refusal, not Allocation")
    else {
        panic!("zero cache refuses before growth");
    };
    let cap = crate::exec::scratch::ScratchCapability::start(
        crate::api::prepared::source::UNBOUNDED_POLICY,
        crate::exec::scratch::capability::ScratchPolicy::unbounded(),
    )
    .expect("scratch");
    let _store = exhausted.open_nonresident(&cap);
    assert_eq!(cache.cache_ledger().used(), 0);
}

/// D02: two retained text-bearing images; trim A; ingest different texts;
/// B still resolves the pinned generation. Concurrent trim/admit must not
/// alias tokens. Numeric images and text beyond RAM stay on the same ledger.
#[test]
fn d02_shared_meanings_survive_trim_and_do_not_alias() {
    use crate::work::CachePolicy;
    let first = fixture();
    let cache = ImageCache::with_policy(
        first.schema(),
        CachePolicy {
            cache_bytes: 1 << 20,
        },
    );
    let source_a = first.source();
    let image_a = cache
        .get_or_build_at(&source_a, first.schema(), R, generation(1))
        .expect("A")
        .expect_ready("resident");
    let rows_b: Vec<Vec<Value>> = (0..8u64)
        .map(|i| vec![Value::U64(i), Value::String(format!("other-{i}").into())])
        .collect();
    let second = TestSource::new(first.schema(), &[(R, rows_b)]);
    let source_b = second.source();
    let image_b = cache
        .get_or_build_at(&source_b, first.schema(), R, generation(1))
        .expect("B hits A's memo")
        .expect_ready("resident");
    assert!(
        Arc::ptr_eq(&image_a, &image_b),
        "same version shares one charged image"
    );

    let pinned = image_a.generation().clone();
    let token = image_a.column_words(1)[0];
    cache.trim();
    let rebuilt = cache
        .get_or_build_at(&source_b, first.schema(), R, generation(2))
        .expect("new generation after ingest")
        .expect_ready("resident");
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
        .expect("pin")
        .expect_ready("resident");
    let handle = pinned.generation().clone();
    let token = pinned.column_words(1)[0];

    std::thread::scope(|scope| {
        let cache_trim = std::sync::Arc::clone(&cache);
        scope.spawn(move || {
            for _ in 0..8 {
                cache_trim.trim();
            }
        });
        let cache_build = std::sync::Arc::clone(&cache);
        scope.spawn(move || {
            let source = fixture.source();
            for version in 2..6u64 {
                let _ = cache_build.get_or_build_at(
                    &source,
                    fixture.schema(),
                    R,
                    generation(version),
                );
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

/// D29: retain an image across eviction; bytes stay charged until the
/// final strong owner releases. Closed and numeric images use the same
/// ledger. Legal old pins can refuse new resident admission.
#[test]
fn d29_retained_owner_keeps_charge_and_old_text() {
    use crate::work::CachePolicy;
    let fixture = fixture();
    let cache = ImageCache::with_policy(
        fixture.schema(),
        CachePolicy {
            cache_bytes: 1 << 20,
        },
    );
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build")
        .expect_ready("resident");
    let closed = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("closed")
        .expect_ready("resident");
    assert!(
        closed.charged_bytes().is_some(),
        "closed images are charged"
    );
    let used = cache.cache_ledger().used();
    let clone = Arc::clone(&image);
    cache.trim();
    assert_eq!(cache.cache_ledger().used(), used);
    assert!(
        image
            .generation()
            .resolver()
            .with_text(image.column_words(1)[0], |text| text == "row-0")
            .unwrap_or(false)
    );
    drop(clone);
    assert_eq!(cache.cache_ledger().used(), used);
    drop(image);
    assert!(
        cache.cache_ledger().used() < used,
        "refund happens at the last strong owner, not at trim"
    );
}

#[test]
fn d29_pinned_old_generation_can_refuse_new_resident_admission() {
    use crate::work::CachePolicy;
    let fixture = fixture();
    let cache = ImageCache::with_policy(
        fixture.schema(),
        CachePolicy { cache_bytes: 8 },
    );
    let source = fixture.source();
    let crate::image::ResidentAdmit::BeyondMemory(exhausted) = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("typed refusal, not Allocation")
    else {
        panic!("a cache smaller than one image refuses resident admission");
    };
    let cap = crate::exec::scratch::ScratchCapability::start(
        crate::api::prepared::source::UNBOUNDED_POLICY,
        crate::exec::scratch::capability::ScratchPolicy::unbounded(),
    )
    .expect("scratch");
    let _store = exhausted.open_nonresident(&cap);
}

#[test]
fn d02_nonresident_resolution_is_the_real_fallback() {
    use crate::api::prepared::source::UNBOUNDED_POLICY;
    use crate::exec::scratch::capability::ScratchPolicy;
    use crate::image::{ResidentAdmit, SourceImages};
    use crate::work::CachePolicy;
    let fixture = fixture();
    let tiny = ImageCache::with_policy(
        fixture.schema(),
        CachePolicy { cache_bytes: 8 },
    );
    let source = fixture.source();
    let images = SourceImages::bind(&source, &tiny);
    let ResidentAdmit::BeyondMemory(exhausted) = images
        .intern_or_spill("row-0")
        .expect("unbounded work")
    else {
        panic!("tiny cache intern_or_spill must spill");
    };
    match images
        .image(fixture.schema(), R)
        .expect("image seam")
    {
        ResidentAdmit::BeyondMemory(_) => {}
        ResidentAdmit::Ready(_) => panic!("tiny cache image() must spill"),
    }
    let cap = crate::exec::scratch::ScratchCapability::start(
        UNBOUNDED_POLICY,
        ScratchPolicy::unbounded(),
    )
    .expect("scratch");
    let mut store = exhausted.open_nonresident(&cap);
    let token = store
        .intern("row-0", cap.work())
        .expect("nonresident intern");

    let fat = ImageCache::new(fixture.schema());
    let fat_source = fixture.source();
    let fat_images = SourceImages::bind(&fat_source, &fat);
    let resident_tok = fat_images
        .intern_or_spill("row-0")
        .expect("fat intern")
        .expect_ready("unbounded cache intern");
    assert!(crate::image::is_scratch_token(token));
    assert!(crate::image::is_resident_token(resident_tok));
    assert_ne!(token, resident_tok, "intern and scratch ids cannot alias");
    assert!(
        fat_images
            .text_eq(Some(&store))
            .tokens_equal(token, resident_tok)
            .expect("equal"),
        "TextEq unifies intern and scratch without raw word =="
    );
    assert!(
        fat_images
            .generation()
            .tokens_equal(resident_tok, fat_images.generation(), resident_tok),
        "same-generation compare is token identity"
    );
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
fn d02_numeric_images_are_charged_and_survive_trim() {
    let schema = numeric_schema();
    let rows: Vec<Vec<Value>> = (0..4u64).map(|i| vec![Value::U64(i)]).collect();
    let fixture = TestSource::new(&schema, &[(RelationId(0), rows)]);
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), RelationId(0), generation(1))
        .expect("numeric")
        .expect_ready("resident");
    assert!(image.charged_bytes().is_some());
    let used = cache.cache_ledger().used();
    cache.trim();
    assert_eq!(cache.cache_ledger().used(), used);
    assert_eq!(image.row_count(), 4);
}
