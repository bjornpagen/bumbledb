//! Image text ownership under explicit collection.
use std::sync::Arc;

use super::R;
use crate::image::cache::ImageCache;
use crate::image::testsupport::TestSource;
use crate::image::{RelationImage, TransientImage};
use crate::ir::Value;
use crate::schema::{Schema, ValidateDescriptor};
use crate::work::GenerationHandle;
use bumbledb_theory::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Text".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "text".into(),
                    value_type: ValueType::String,
                },
            ],
            extension: None,
        }],
        statements: vec![],
    }
    .validate()
    .unwrap()
}

fn image(schema: &Schema, cache: &ImageCache, names: &[&str]) -> Arc<RelationImage> {
    let rows = names
        .iter()
        .enumerate()
        .map(|(id, text)| vec![Value::U64(id as u64), Value::String((*text).into())])
        .collect();
    TestSource::new(schema, &[(R, rows)]).image(cache, R)
}

fn collect(generation: &GenerationHandle) {
    generation.lock_resolver().reclaim_unowned();
}

#[test]
fn images_pin_distinct_text_not_every_row_or_the_other_images_history() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let names: Vec<_> = (0..8192)
        .map(|i| if i % 2 == 0 { "alpha" } else { "beta" })
        .collect();
    let stable = image(&schema, &cache, &names);
    let changing = image(&schema, &cache, &["obsolete"]);
    let generation = cache.acquire();
    assert_eq!(stable.texts.0.len(), 2, "one owner per distinct text");
    assert_eq!(changing.texts.0.len(), 1);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 3);
    drop(changing);
    collect(&generation);
    assert_eq!(generation.resolver().lookup("obsolete"), None);
    assert_eq!(generation.lock_resolver().len(), 2);
    assert_eq!(stable.row_count(), 8192);
    let reader = Arc::clone(&stable);
    drop(stable);
    collect(&generation);
    assert!(generation.resolver().lookup("alpha").is_some());
    assert!(generation.resolver().lookup("beta").is_some());
    drop(reader);
    collect(&generation);
    assert_eq!(
        generation.lock_resolver().len(),
        0,
        "the cache's namespace handle cannot keep dead image text alive"
    );
}

#[test]
fn derived_rows_own_their_text_after_the_source_and_old_readers_drop() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let source = image(&schema, &cache, &["alpha", "beta"]);
    let generation = cache.acquire();
    let mut derived = TransientImage::default();
    let rows = [[source.column_words(1)[0]], [source.column_words(1)[1]]];
    let old_reader = derived.refill(
        &[ValueType::String],
        2,
        &generation,
        rows.iter().map(<[_; 1]>::as_slice),
    );
    drop(source);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 2);
    let next_source = image(&schema, &cache, &["next"]);
    let new_rows = [[next_source.column_words(1)[0]]];
    let new_reader = derived.refill(
        &[ValueType::String],
        1,
        &generation,
        new_rows.iter().map(<[_; 1]>::as_slice),
    );
    assert!(!Arc::ptr_eq(&old_reader, &new_reader));
    drop(next_source);
    collect(&generation);
    assert_eq!(
        generation.lock_resolver().len(),
        3,
        "both generations of image are live"
    );
    assert_eq!(
        generation
            .resolver()
            .with_text(old_reader.column_words(0)[0], str::to_owned),
        Some("alpha".into())
    );
    drop(old_reader);
    collect(&generation);
    assert_eq!(generation.resolver().lookup("alpha"), None);
    assert_eq!(generation.resolver().lookup("beta"), None);
    assert_eq!(
        generation
            .resolver()
            .with_text(new_reader.column_words(0)[0], str::to_owned),
        Some("next".into())
    );
    drop(derived);
    collect(&generation);
    assert!(generation.resolver().lookup("next").is_some());
    drop(new_reader);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 0);
}

#[test]
fn unique_large_to_small_refill_releases_obsolete_text_and_reuses_warm_storage() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let names: Vec<_> = (0..1024).map(|i| format!("entry-{i}")).collect();
    let borrowed: Vec<_> = names.iter().map(String::as_str).collect();
    let source = image(&schema, &cache, &borrowed);
    let generation = cache.acquire();
    let rows: Vec<_> = source.column_words(1).iter().map(|&word| [word]).collect();
    let mut derived = TransientImage::default();
    let published = derived.refill(
        &[ValueType::String],
        rows.len(),
        &generation,
        rows.iter().map(<[_; 1]>::as_slice),
    );
    let address = Arc::as_ptr(&published);
    drop(source);
    drop(published);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 1024);
    let small = derived.refill(
        &[ValueType::String],
        1,
        &generation,
        rows[..1].iter().map(<[_; 1]>::as_slice),
    );
    assert_eq!(
        Arc::as_ptr(&small),
        address,
        "uniquely owned slabs are reused"
    );
    assert_eq!(small.texts.0.len(), 1);
    collect(&generation);
    assert_eq!(
        generation.lock_resolver().len(),
        1,
        "obsolete payload does not follow slab high-water"
    );
    drop(small);
    let before = crate::alloc_counter::snapshot().window;
    for _ in 0..32 {
        let small = derived.refill(
            &[ValueType::String],
            1,
            &generation,
            rows[..1].iter().map(<[_; 1]>::as_slice),
        );
        assert_eq!(Arc::as_ptr(&small), address);
    }
    let after = crate::alloc_counter::snapshot().window;
    #[cfg(feature = "alloc-counter")]
    assert_eq!(
        after.allocs - before.allocs,
        0,
        "warm text ownership reuses its storage"
    );
    eprintln!(
        "derived warm text refill: allocations={}, bytes={}",
        after.allocs - before.allocs,
        after.alloc_bytes - before.alloc_bytes
    );
}

#[test]
fn refill_cannot_confuse_equal_token_bits_from_different_namespaces() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let first = image(&schema, &cache, &["before"]);
    let old_generation = cache.acquire();
    let mut derived = TransientImage::default();
    let old_rows = [[first.column_words(1)[0]]];
    drop(derived.refill(
        &[ValueType::String],
        1,
        &old_generation,
        old_rows.iter().map(<[_; 1]>::as_slice),
    ));
    cache.clear();
    let second = image(&schema, &cache, &["after"]);
    let new_generation = cache.acquire();
    let new_rows = [[second.column_words(1)[0]]];
    assert_eq!(
        old_rows, new_rows,
        "fixture exercises token-bit reuse across namespaces"
    );
    let published = derived.refill(
        &[ValueType::String],
        1,
        &new_generation,
        new_rows.iter().map(<[_; 1]>::as_slice),
    );
    drop(first);
    drop(second);
    collect(&old_generation);
    collect(&new_generation);
    assert_eq!(old_generation.lock_resolver().len(), 0);
    assert_eq!(
        new_generation
            .resolver()
            .with_text(published.column_words(0)[0], str::to_owned),
        Some("after".into())
    );
}

#[test]
fn failed_refill_drops_partial_text_owners_and_remains_reusable() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let source = image(&schema, &cache, &["before", "after"]);
    let generation = cache.acquire();
    let words = source.column_words(1).to_vec();
    let mut derived = TransientImage::default();
    drop(derived.refill(
        &[ValueType::String],
        1,
        &generation,
        std::iter::once(&words[..1]),
    ));
    let stopped = crate::api::prepared::source::unbounded_work();
    stopped.cancel();
    let result = derived.refill_drained(None, &[ValueType::String], 2, &generation, |_, write| {
        write(&words[1..]);
        stopped
            .checkpoint()
            .map_err(crate::api::prepared::source::work_error)
    });
    assert!(result.is_err());
    if let TransientImage::Occupied { image, .. } = &derived {
        assert_eq!(image.row_count(), 0);
        assert!(image.texts.0.is_empty());
    } else {
        panic!("failed drain retains reusable slabs");
    }
    let retry = derived.refill(
        &[ValueType::String],
        1,
        &generation,
        std::iter::once(&words[..1]),
    );
    assert_eq!(retry.row_count(), 1);
    drop(source);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 1);
}

#[test]
fn unknown_resident_text_cannot_publish_a_derived_image() {
    let generation = crate::image::build::test_generation();
    let mut derived = TransientImage::default();
    let result = derived.refill_drained(None, &[ValueType::String], 1, &generation, |_, write| {
        write(&[42]);
        Ok(())
    });
    assert!(matches!(result, Err(crate::Error::Corruption(_))));
    let TransientImage::Occupied { image, .. } = derived else {
        panic!("allocated reusable image");
    };
    assert_eq!(image.row_count(), 0);
    assert!(image.texts.0.is_empty());
}

#[test]
fn concurrent_collection_preserves_images_and_atomically_acquired_text_owners() {
    let schema = schema();
    let cache = ImageCache::new(&schema);
    let stable = image(&schema, &cache, &["stable"]);
    let generation = cache.acquire();
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let generation = &generation;
            scope.spawn(move || {
                let work = crate::api::prepared::source::unbounded_work();
                for turn in 0..128 {
                    let text = format!("worker-{worker}-{turn}");
                    let (token, owner) = {
                        let mut resolver = generation.lock_resolver();
                        let token = resolver.intern(&text, &work).unwrap();
                        (token, resolver.owned_text(token).unwrap())
                    };
                    collect(generation);
                    assert_eq!(
                        generation.resolver().with_text(token, str::to_owned),
                        Some(text)
                    );
                    assert!(generation.resolver().lookup("stable").is_some());
                    drop(owner);
                }
            });
        }
    });
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 1);
    assert_eq!(
        generation
            .resolver()
            .with_text(stable.column_words(1)[0], str::to_owned),
        Some("stable".into())
    );
    drop(stable);
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 0);
}

#[test]
fn decoded_rows_own_text_until_replacement_and_discard_failed_partial_decodes() {
    let schema = schema();
    let generation = crate::image::test_generation();
    let work = crate::api::prepared::source::unbounded_work();
    let interner = crate::image::intern::InternerHandle::new(&generation, &work);
    let fields = schema.relation(R).fields();
    let mut row = crate::image::canon::RowWords::new(&[ValueType::U64, ValueType::String]);
    let encode = |text: &str| {
        crate::canonical::CanonicalRow::encode(
            fields,
            &[Value::U64(1), Value::String(text.into())],
            &work,
        )
        .unwrap()
    };
    let first = encode("first");
    crate::api::prepared::decode_row(&mut row, fields, first.as_bytes(), &interner, &work, true)
        .unwrap();
    collect(&generation);
    let token = row.span_words(bumbledb_theory::schema::FieldId(1))[0];
    assert_eq!(
        generation.resolver().with_text(token, str::to_owned),
        Some("first".into())
    );
    let second = encode("second");
    crate::api::prepared::decode_row(&mut row, fields, second.as_bytes(), &interner, &work, true)
        .unwrap();
    collect(&generation);
    assert_eq!(generation.resolver().lookup("first"), None);
    assert!(generation.resolver().lookup("second").is_some());
    let mut malformed = encode("partial").as_bytes().to_vec();
    malformed.push(0);
    assert!(
        crate::api::prepared::decode_row(&mut row, fields, &malformed, &interner, &work, true)
            .is_err()
    );
    collect(&generation);
    assert_eq!(
        generation.lock_resolver().len(),
        0,
        "malformed rows retain no successful prefix"
    );
    crate::api::prepared::decode_row(&mut row, fields, second.as_bytes(), &interner, &work, true)
        .unwrap();
    row.release_memory();
    collect(&generation);
    assert_eq!(generation.lock_resolver().len(), 0);
}
