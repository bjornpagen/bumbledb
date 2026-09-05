//! The generation-keyed image cache: hits at the same generation, one
//! rebuild per newer generation, query-local images for old pinned
//! generations, per-execution rebuilds for heap ticks, once-only closed
//! synthesis, and the memory-pressure trim.
use std::sync::Arc;

use crate::image::ViewEpoch;
use crate::image::testsupport::TestSource;
use crate::ir::Value;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::GenerationId;
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
    ViewEpoch::Store(GenerationId::from_storage(value))
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
        .expect("build old");
    let new = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(4))
        .expect("build new");
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
        .expect("build new");
    let old = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(2))
        .expect("query-local build");
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
    let epoch = source.epoch();
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
    assert!(cache.peek_at(R, epoch).is_none(), "nothing was cached");
}

#[test]
fn closed_relations_synthesize_once_and_never_trim() {
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

    cache.trim();
    let after = cache
        .get_or_build_at(&source, fixture.schema(), STATUS, ViewEpoch::Closed)
        .expect("still resident");
    assert!(
        Arc::ptr_eq(&first, &after),
        "trim never evicts a closed image"
    );
}

#[test]
fn trim_drops_generation_entries_and_shrinks_retained_bytes() {
    let fixture = fixture();
    let cache = ImageCache::new(fixture.schema());
    let source = fixture.source();
    let image = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("build");
    let resident = cache.retained_bytes();
    assert!(
        resident >= image.byte_size(),
        "retained bytes cover the resident image"
    );

    cache.trim();
    assert!(
        cache.peek_at(R, generation(1)).is_none(),
        "trim evicts generation-keyed images"
    );
    let trimmed = cache.retained_bytes();
    assert!(
        trimmed < resident,
        "retained bytes shrink by the evicted slabs ({trimmed} < {resident})"
    );

    // The interner survives the trim: token stability is the invariant.
    let rebuilt = cache
        .get_or_build_at(&source, fixture.schema(), R, generation(1))
        .expect("rebuild after trim");
    assert_eq!(rebuilt.row_count(), 8);
    let name_column = usize::from(
        rebuilt
            .span(bumbledb_theory::schema::FieldId(1))
            .first_column,
    );
    let mut before: Vec<u64> = image.column_words(name_column).to_vec();
    let mut after: Vec<u64> = rebuilt.column_words(name_column).to_vec();
    before.sort_unstable();
    after.sort_unstable();
    assert_eq!(
        before, after,
        "the same texts intern to the same tokens across the trim"
    );
}
