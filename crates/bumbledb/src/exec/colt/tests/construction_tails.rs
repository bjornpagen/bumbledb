use super::*;
use crate::image::TransientImage;

fn image(width: usize, count: usize) -> Arc<crate::image::RelationImage> {
    let rows: Vec<Vec<u64>> = (0..u64::try_from(count).unwrap())
        .map(|id| [id, id % 2].into_iter().chain(key(width, id / 4)).collect())
        .collect();
    TransientImage::default().refill(
        &vec![ValueType::U64; width + 2],
        rows.len(),
        &crate::image::test_generation(),
        rows.iter().map(Vec::as_slice),
    )
}

fn key(width: usize, value: u64) -> Vec<u64> {
    (0..width)
        .map(|column| {
            value
                .wrapping_mul(31)
                .rotate_left(u32::try_from(column * 9).unwrap())
        })
        .collect()
}

fn live_lengths(colt: &Colt) -> [usize; 3] {
    colt.maps.iter().fold([0; 3], |mut total, map| {
        total[0] += map.nbuckets * 8;
        total[1] += map.nbuckets * map.stride();
        total[2] += map.len as usize;
        total
    })
}

#[test]
fn successful_construction_retains_only_live_table_lengths() {
    let image = image(1, 513);
    let mut colt = Colt::new(all(&image), &[], vec![vec![0]]);
    colt.force_root().unwrap();
    assert!(colt.maps[0].nbuckets > super::super::force::force_nbuckets(513));
    assert_eq!(
        [colt.ctrl.len(), colt.buckets.len(), colt.dense.len()],
        live_lengths(&colt),
        "completed construction leaves no retired table contents"
    );
}

fn force_preserving_prefix(colt: &mut Colt, cursor: Cursor, level: usize) {
    let ctrl = colt.ctrl.clone();
    let buckets = colt.buckets.clone();
    let dense = colt.dense.clone();
    let layouts: Vec<_> = colt
        .maps
        .iter()
        .map(|m| {
            (
                m.arity,
                m.nbuckets,
                m.len,
                m.ctrl_start,
                m.bucket_start,
                m.dense_start,
            )
        })
        .collect();
    colt.ensure_forced(cursor, level).unwrap();
    assert_eq!(&colt.ctrl[..ctrl.len()], ctrl);
    assert_eq!(&colt.buckets[..buckets.len()], buckets);
    assert_eq!(&colt.dense[..dense.len()], dense);
    for (map, layout) in colt.maps.iter().zip(layouts) {
        assert_eq!(
            (
                map.arity,
                map.nbuckets,
                map.len,
                map.ctrl_start,
                map.bucket_start,
                map.dense_start
            ),
            layout
        );
    }
}

fn assert_contents(colt: &mut Colt, width: usize, count: u64) {
    let root = drain(colt, Colt::root(), 0);
    assert_eq!(root.iter().map(|(k, _)| k[0]).collect::<Vec<_>>(), [0, 1]);
    for (group_key, group) in root {
        force_preserving_prefix(colt, group, 1);
        let rows = drain(colt, group, 1);
        let mut expected: Vec<(Vec<u64>, Vec<u64>)> = Vec::new();
        for id in (0..count).filter(|id| id % 2 == group_key[0]) {
            let key = key(width, id / 4);
            if let Some((last, ids)) = expected.last_mut()
                && *last == key
            {
                ids.push(id);
            } else {
                expected.push((key, vec![id]));
            }
        }
        assert_eq!(rows.len(), expected.len());
        for ((key_words, child), (expected_key, expected_ids)) in rows.into_iter().zip(expected) {
            assert_eq!(key_words, expected_key);
            assert_eq!(colt.get(group, 1, &key_words), Some(child));
            if expected_ids.len() > 2 {
                force_preserving_prefix(colt, child, 2);
            } else {
                colt.ensure_forced(child, 2).unwrap();
            }
            let actual: Vec<_> = drain(colt, child, 2)
                .into_iter()
                .map(|(k, _)| k[0])
                .collect();
            assert_eq!(actual, expected_ids);
        }
    }
}

#[test]
fn construction_preserves_older_maps_dense_order_children_clones_and_resets() {
    for width in [0, 1, 2, 3, 4, 5, 8] {
        let image = image(width, 513);
        let levels = vec![vec![1], (2..width + 2).collect(), vec![0]];
        let mut colt = Colt::new(all(&image), &[], levels.clone());
        for _ in 0..2 {
            colt.force_root().unwrap();
            assert_contents(&mut colt, width, 513);
            let mut cloned = Colt::new(all(&image), &[], levels.clone());
            drop(cloned.clone_bound_from(&colt, Vec::new()).unwrap());
            assert_eq!(colt.ctrl, cloned.ctrl);
            assert_eq!(colt.buckets, cloned.buckets);
            assert_eq!(colt.dense, cloned.dense);
            assert_contents(&mut cloned, width, 513);
            drop(colt.reset(all(&image)));
        }
    }
}
