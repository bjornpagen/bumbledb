//! Fallible growth, cooperative cancellation, and retained pool reuse.
use super::*;
use crate::work::{WorkContext, WorkError};

fn join_colt(view: &Arc<crate::image::RelationImage>) -> Colt {
    Colt::new(all(view), &[], vec![vec![0], vec![1]])
}

fn lengths(colt: &Colt) -> [usize; 7] {
    let mark = colt.pool_mark();
    [
        mark.nodes,
        mark.chunks,
        mark.chunk_positions,
        mark.maps,
        mark.ctrl,
        mark.buckets,
        mark.dense,
    ]
}

fn iter_once(colt: &mut Colt, cursor: Cursor) -> Result<(usize, BatchToken), WorkError> {
    let probe = colt.get_prehashed(cursor, 0, &[0], hash_key(&[0]));
    assert_eq!(
        colt.prepare_probe(cursor, 0)
            .map(|probe| probe.contains_prehashed_width::<1>(&[0], hash_key(&[0]))),
        probe.map(|child| child.is_some()),
        "presence propagates construction failure"
    );
    let mut keys = [0; 8];
    let keys_only = colt.iter_keys_batch(cursor, 0, BatchToken::default(), &mut keys, 8);
    let with_children = colt.iter_batch(
        cursor,
        0,
        BatchToken::default(),
        &mut keys,
        &mut [Cursor::Row(0); 8],
        8,
    );
    assert_eq!(
        keys_only, with_children,
        "child output preserves the failure"
    );
    with_children
}

#[test]
fn cancelled_first_map_does_not_publish_or_fabricate_empty_output() {
    let schema = schema();
    let rows: Vec<_> = (0..32).map(|i| (i, i)).collect();
    let view = view_of(&schema, &rows);
    let work = WorkContext::new();
    work.cancel();
    let mut colt = join_colt(&view);
    colt.bind(Some(&work));
    let before = lengths(&colt);
    assert_eq!(colt.force_root(), Err(WorkError::Cancelled));
    assert!(colt.forced_capacity(Colt::root()).is_none());
    assert_eq!(lengths(&colt), before);
    assert_eq!(
        iter_once(&mut colt, Colt::root()),
        Err(WorkError::Cancelled)
    );
    assert_eq!(lengths(&colt), before);

    colt.bind(Some(&WorkContext::new()));
    colt.force_root().unwrap();
    assert_eq!(drain(&mut colt, Colt::root(), 0).len(), rows.len());
}

#[test]
fn cancelled_duplicate_chunk_preserves_the_original_singleton() {
    let schema = schema();
    let view = view_of(&schema, &[(1, 10), (2, 20)]);
    let mut colt = join_colt(&view);
    colt.force_root().unwrap();
    let map = colt.maps[0];
    let (found, index) = colt.probe_hashed(&map, &[1], hash_key(&[1]));
    assert!(found);
    let child_at = map.child_at(index);
    let singleton = colt.buckets[child_at];
    let before = lengths(&colt);
    let stopped = WorkContext::new();
    stopped.cancel();
    colt.bind(Some(&stopped));
    assert_eq!(colt.append_child(child_at, 1), Err(WorkError::Cancelled));
    assert_eq!(lengths(&colt), before);
    assert_eq!(colt.buckets[child_at], singleton);
    assert_eq!(colt.chunks.len(), 0, "no fabricated chunk index");

    colt.bind(Some(&WorkContext::new()));
    colt.append_child(child_at, 1).unwrap();
    let child = colt.get(Colt::root(), 0, &[1]).unwrap();
    let values = drain(&mut colt, child, 1);
    assert_eq!(
        values.iter().map(|(key, _)| key[0]).collect::<Vec<_>>(),
        vec![10, 20]
    );
}

#[test]
fn cancelled_child_construction_preserves_the_readable_root_and_can_retry() {
    let schema = schema();
    let rows: Vec<_> = (0..513).map(|i| (0, i)).collect();
    let view = view_of(&schema, &rows);
    let mut colt = join_colt(&view);
    colt.force_root().unwrap();
    let map = colt.maps[0];
    let layout = (
        map.nbuckets,
        map.ctrl_start,
        map.bucket_start,
        map.dense_start,
    );
    let before = lengths(&colt);
    let stopped = WorkContext::new();
    stopped.cancel();
    let child = colt.get(Colt::root(), 0, &[0]).unwrap();
    colt.bind(Some(&stopped));
    assert_eq!(colt.ensure_forced(child, 1), Err(WorkError::Cancelled));
    assert_eq!(
        (
            colt.maps[0].nbuckets,
            colt.maps[0].ctrl_start,
            colt.maps[0].bucket_start,
            colt.maps[0].dense_start
        ),
        layout
    );
    assert_eq!(lengths(&colt), before);
    assert_eq!(colt.get(Colt::root(), 0, &[0]), Some(child));

    colt.bind(Some(&WorkContext::new()));
    colt.ensure_forced(child, 1).unwrap();
    assert!(colt.maps[1].nbuckets > super::super::force::force_nbuckets(513));
    assert_eq!(
        drain(&mut colt, child, 1)
            .iter()
            .map(|(key, _)| key[0])
            .collect::<Vec<_>>(),
        (0..513).collect::<Vec<_>>()
    );
}

#[test]
fn repeated_same_shape_executions_reuse_retained_pools_without_allocating() {
    let schema = schema();
    let rows: Vec<_> = (0..64).map(|i| (i % 8, i)).collect();
    let image = view_of(&schema, &rows);
    let work = WorkContext::new();
    let mut colt = join_colt(&image);
    colt.bind(Some(&work));
    colt.force_root().unwrap();
    let retained = colt.retained_bytes();
    assert!(retained > 0);
    #[cfg(feature = "alloc-counter")]
    let before = crate::alloc_counter::snapshot().window;
    for round in 0..32 {
        drop(colt.reset(all(&image)));
        colt.bind(Some(&work));
        colt.force_root().unwrap();
        assert_eq!(colt.retained_bytes(), retained, "capacity at round {round}");
    }
    #[cfg(feature = "alloc-counter")]
    assert_eq!(
        crate::alloc_counter::snapshot().window,
        before,
        "no warm pool allocations or frees"
    );
}

#[test]
fn forced_probes_preserve_pools_until_reset_requires_construction() {
    let schema = schema();
    let rows: Vec<_> = (0..16).map(|i| (i, i)).collect();
    let image = view_of(&schema, &rows);
    let work = WorkContext::new();
    let mut colt = join_colt(&image);
    colt.bind(Some(&work));
    colt.force_root().unwrap();
    let root = Colt::root();
    let hit = colt.get_prehashed(root, 0, &[7], hash_key(&[7])).unwrap();
    assert!(hit.is_some());
    let before = lengths(&colt);
    let retained = colt.retained_bytes();
    #[cfg(feature = "alloc-counter")]
    let allocation = crate::alloc_counter::snapshot().window;
    {
        let probe = colt.prepare_probe(root, 0).unwrap();
        let hashes = [hash_key(&[7]), hash_key(&[99])];
        for _ in 0..32 {
            probe.prefetch_batch(&hashes, true);
            probe.prefetch_batch(&hashes, false);
            assert_eq!(probe.get_prehashed_width::<1>(&[7], hashes[0]), hit);
            assert_eq!(probe.get_prehashed_width::<1>(&[99], hashes[1]), None);
            assert!(probe.contains_prehashed_width::<1>(&[7], hashes[0]));
            assert!(!probe.contains_prehashed_width::<1>(&[99], hashes[1]));
        }
    }
    #[cfg(feature = "alloc-counter")]
    assert_eq!(crate::alloc_counter::snapshot().window, allocation);
    assert_eq!(lengths(&colt), before);
    assert_eq!(colt.retained_bytes(), retained);

    // Retained allocation does not bypass construction's cancellation poll.
    let replacement: Vec<_> = (100..116).map(|i| (i, i)).collect();
    let replacement = view_of(&schema, &replacement);
    drop(colt.reset(all(&replacement)));
    let stopped = WorkContext::new();
    stopped.cancel();
    colt.bind(Some(&stopped));
    assert_eq!(
        colt.prepare_probe(root, 0).map(|_| ()),
        Err(WorkError::Cancelled)
    );
    assert!(colt.forced_capacity(root).is_none());
    colt.bind(Some(&WorkContext::new()));
    colt.force_root().unwrap();
    assert_eq!(
        colt.get_prehashed(root, 0, &[7], hash_key(&[7])).unwrap(),
        None
    );
    assert!(
        colt.get_prehashed(root, 0, &[107], hash_key(&[107]))
            .unwrap()
            .is_some()
    );
    assert_eq!(lengths(&colt), before);
    assert_eq!(colt.retained_bytes(), retained);
}

#[test]
fn pool_growth_is_geometric_and_failed_growth_preserves_contents() {
    let work = WorkContext::new();
    let mut pool = Vec::<u32>::new();
    let mut growths = 0;
    for value in 0..4096 {
        let capacity = pool.capacity();
        reserve_pool(pool.len() + 1, &mut pool, Some(&work)).unwrap();
        growths += usize::from(capacity != pool.capacity());
        pool.push(value);
    }
    assert!(growths <= 10, "doubling, not one allocation per key");
    let capacity = pool.capacity();
    let pointer = pool.as_ptr();
    // Vec<u32> cannot represent this layout: exercise reserve failure
    // deterministically without requesting real machine-sized memory.
    assert_eq!(
        reserve_pool(usize::MAX, &mut pool, Some(&work)),
        Err(WorkError::Allocation)
    );
    work.cancel();
    assert_eq!(
        reserve_pool(capacity + 1, &mut pool, Some(&work)),
        Err(WorkError::Cancelled)
    );
    assert_eq!(pool.capacity(), capacity);
    assert_eq!(pool.as_ptr(), pointer);
    assert!(pool.iter().copied().eq(0..4096));
}
