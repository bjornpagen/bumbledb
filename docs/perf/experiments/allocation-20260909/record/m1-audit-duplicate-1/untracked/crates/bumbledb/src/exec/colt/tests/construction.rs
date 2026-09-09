use super::*;
use std::collections::BTreeMap;

fn fixture(width: usize, distinct: usize, grouped: bool) -> Arc<crate::image::RelationImage> {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: (0..=width)
                .map(|column| FieldDescriptor {
                    name: format!("c{column}").into(),
                    value_type: ValueType::U64,
                })
                .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .unwrap();
    // The row id sorts first in the canonical image, making the two key
    // schedules genuinely different after source sorting and deduplication.
    let facts = (0..50u64)
        .map(|row| {
            let row_index = usize::try_from(row).unwrap();
            let key = if grouped {
                row_index * distinct / 50
            } else {
                row_index % distinct
            };
            std::iter::once(Value::U64(row))
                .chain((0..width).map(|column| {
                    Value::U64((key as u64).rotate_left(u32::try_from(column * 9).unwrap()))
                }))
                .collect()
        })
        .collect();
    TestSource::new(&schema, &[(R, facts)])
        .image_with_cache(R)
        .1
}

fn levels(width: usize) -> Vec<Vec<usize>> {
    vec![(1..=width).collect(), vec![0]]
}

fn check_rows(colt: &mut Colt, image: &crate::image::RelationImage, width: usize) {
    let mut expected = BTreeMap::<Vec<u64>, Vec<u64>>::new();
    for position in 0..image.row_count() {
        let key = (1..=width)
            .map(|column| image.column_words(column)[position])
            .collect();
        expected
            .entry(key)
            .or_default()
            .push(image.column_words(0)[position]);
    }
    let before = drain(colt, Colt::root(), 0);
    let mut actual = BTreeMap::new();
    for (key, child) in &before {
        assert_eq!(colt.get(Colt::root(), 0, key), Some(*child));
        // Later child construction must leave the older published root valid.
        colt.ensure_forced(*child, 1).unwrap();
        let mut rows: Vec<_> = drain(colt, *child, 1)
            .into_iter()
            .map(|(key, _)| key[0])
            .collect();
        rows.sort_unstable();
        assert!(actual.insert(key.clone(), rows).is_none());
    }
    assert_eq!(actual, expected);
    assert_eq!(drain(colt, Colt::root(), 0), before);
}

#[test]
fn duplicates_do_not_grow_maps_but_new_keys_do() {
    for width in [0, 1, 2, 3, 4, 5, 8] {
        for distinct in [25, 26] {
            for grouped in [false, true] {
                let image = fixture(width, distinct, grouped);
                assert_eq!(image.row_count(), 50);
                let mut colt = Colt::new(all(&image), &[], levels(width));
                for generation in 0..2 {
                    drop(colt.reset(all(&image)));
                    colt.force_root().unwrap();
                    let root = colt.maps[0];
                    let expected_keys = if width == 0 { 1 } else { distinct };
                    let expected_buckets = if expected_keys <= 25 { 8 } else { 16 };
                    assert_eq!(root.len as usize, expected_keys);
                    assert_eq!(
                        root.nbuckets, expected_buckets,
                        "width={width} distinct={distinct} grouped={grouped} generation={generation}"
                    );
                    if expected_buckets == 8 {
                        assert_eq!(
                            (root.ctrl_start, root.bucket_start, root.dense_start),
                            (0, 0, 0)
                        );
                        assert_eq!(colt.ctrl.len(), 64);
                        assert_eq!(colt.buckets.len(), 64 * (width + 1));
                        assert_eq!(colt.dense.len(), expected_keys);
                    }
                    check_rows(&mut colt, &image, width);
                    let mut sibling = Colt::new(all(&image), &[], levels(width));
                    drop(sibling.clone_bound_from(&colt, Vec::new()).unwrap());
                    assert_eq!(sibling.ctrl, colt.ctrl);
                    assert_eq!(sibling.buckets, colt.buckets);
                    assert_eq!(sibling.dense, colt.dense);
                    check_rows(&mut sibling, &image, width);
                }
            }
        }
    }
}

#[test]
fn duplicate_at_the_growth_boundary_propagates_child_allocation_refusal() {
    use crate::work::{WorkContext, WorkError};
    let image = fixture(1, 25, false);
    let mut colt = Colt::new(all(&image), &[], levels(1));
    colt.first_chunk_cap = 2;
    colt.force_root().unwrap();
    let mut map = colt.maps[0];
    let layout = (
        map.nbuckets,
        map.ctrl_start,
        map.bucket_start,
        map.dense_start,
    );
    let before = (colt.ctrl.clone(), colt.buckets.clone(), colt.dense.clone());
    let child = colt.get(Colt::root(), 0, &[0]).unwrap();
    let chunk_lengths = (
        colt.nodes.len(),
        colt.chunks.len(),
        colt.chunk_positions.len(),
    );
    let cancelled = WorkContext::new();
    cancelled.cancel();
    colt.bind(Some(&cancelled));
    assert_eq!(
        colt.ingest_one(&mut map, &[0], hash_key(&[0]), 0),
        Err(WorkError::Cancelled)
    );
    assert_eq!(
        (
            map.nbuckets,
            map.ctrl_start,
            map.bucket_start,
            map.dense_start
        ),
        layout
    );
    assert_eq!(
        (&colt.ctrl, &colt.buckets, &colt.dense),
        (&before.0, &before.1, &before.2)
    );
    assert_eq!(
        (
            colt.nodes.len(),
            colt.chunks.len(),
            colt.chunk_positions.len()
        ),
        chunk_lengths
    );
    assert_eq!(colt.key_count(child), KeyCount::Estimate(2));
    colt.bind(Some(&WorkContext::new()));
    colt.ingest_one(&mut map, &[0], hash_key(&[0]), 0).unwrap();
    assert_eq!(
        (
            map.nbuckets,
            map.ctrl_start,
            map.bucket_start,
            map.dense_start
        ),
        layout
    );
    assert_eq!(map.len, 25);
    assert_eq!(colt.key_count(child), KeyCount::Estimate(3));
}

#[cfg(feature = "alloc-counter")]
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/m1-audit.rs"]
mod audit;
