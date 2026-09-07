//! Allocation-refusal discriminators for fallible COLT force/growth.
//! Exercises the production force, chunk and resize admission boundaries.

use super::*;
use crate::work::{ExecutionPolicy, Resource, WorkError};

fn working(bytes: u64) -> crate::work::WorkContext {
    ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes: bytes,
        scratch_bytes: u64::MAX,
        result_bytes: u64::MAX,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: std::time::Duration::from_secs(3600),
    }
    .start()
    .expect("policy")
}

fn is_working_exhaustion(error: &WorkError) -> bool {
    matches!(
        error,
        WorkError::Exhausted {
            resource: Resource::WorkingBytes,
            ..
        }
    )
}

fn join_colt(view: &std::sync::Arc<crate::image::RelationImage>) -> Colt {
    Colt::new(all(view), &[], vec![vec![0], vec![1]])
}

fn iter_once(colt: &mut Colt, cursor: Cursor) -> Result<(usize, BatchToken), WorkError> {
    let mut keys = vec![0u64; 8];
    let keys_only = colt.iter_keys_batch(cursor, 0, BatchToken::default(), &mut keys, 8);
    let mut children = vec![Cursor::Row(0); 8];
    let with_children = colt.iter_batch(
        cursor,
        0,
        BatchToken::default(),
        &mut keys,
        &mut children,
        8,
    );
    assert_eq!(
        keys_only, with_children,
        "child output never changes admission"
    );
    with_children
}

/// First map admission refuses before `maps[0]` can be indexed.
#[test]
fn first_map_refusal_is_typed_and_does_not_index() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..32).map(|i| (i, i)).collect();
    let view = view_of(&schema, &rows);
    let work = working(0);
    let mut colt = join_colt(&view);
    colt.bind(Some(&work));
    let root = Colt::root();
    let error = colt
        .force_root()
        .expect_err("zero working bytes refuse the first map");
    assert!(
        is_working_exhaustion(&error),
        "typed WorkingBytes refusal licenses the bounded fallback, got {error:?}"
    );
    assert!(
        colt.forced_capacity(root).is_none(),
        "the node stays unforced; no sentinel map index"
    );
    assert_eq!(colt.watermark(), 1, "failed force rolls back pool lengths");
    let probe = colt.get_prehashed(root, 0, &[0], hash_key(&[0]));
    assert!(
        probe.is_err(),
        "a miss is not fabricated after map refusal, got {probe:?}"
    );
    let drained = iter_once(&mut colt, root);
    assert!(
        drained.is_err(),
        "iteration must not succeed as empty output after map refusal, got {drained:?}"
    );
}

/// First duplicate-key chunk refuses before `chunks[0]` can be indexed.
#[test]
fn first_duplicate_chunk_refusal_is_typed() {
    let schema = schema();
    let roomy = working(u64::MAX);
    let unique = view_of(&schema, &[(1, 10)]);
    let mut probe = join_colt(&unique);
    probe.bind(Some(&roomy));
    probe.force_root().expect("measure a map-only force");
    let map_cost = probe.charged_bytes();
    assert!(map_cost > 0, "the map-only force must charge capacity");

    let tight = working(map_cost.saturating_add(32));
    let duplicates = view_of(&schema, &[(1, 10), (1, 20)]);
    let mut colt = join_colt(&duplicates);
    colt.bind(Some(&tight));
    let error = colt
        .force_root()
        .expect_err("map-only budget cannot pay the first duplicate-key chunk");
    assert!(
        is_working_exhaustion(&error),
        "typed WorkingBytes refusal licenses the bounded fallback, got {error:?}"
    );
    assert!(
        colt.forced_capacity(Colt::root()).is_none(),
        "chunk refusal rolls back; no half-forced map"
    );
    assert_eq!(colt.chunks.len(), 0, "no sentinel chunk index");
    let drained = iter_once(&mut colt, Colt::root());
    assert!(
        drained.is_err(),
        "iteration must not succeed as empty output after chunk refusal, got {drained:?}"
    );
}

/// A later `grow_map` resize refuses before ingest continues into a full table.
#[test]
fn later_resize_refusal_is_typed_and_does_not_hang() {
    let schema = schema();
    let roomy = working(u64::MAX);
    let compact: Vec<(u64, u64)> = (0..25).map(|i| (i, i)).collect();
    let compact_view = view_of(&schema, &compact);
    let mut probe = join_colt(&compact_view);
    probe.bind(Some(&roomy));
    probe.force_root().expect("measure an ungrowable force");
    let compact_cost = probe.charged_bytes();

    let tight = working(compact_cost + 64);
    let growing: Vec<(u64, u64)> = (0..30).map(|i| (i, i)).collect();
    let growing_view = view_of(&schema, &growing);
    let mut colt = join_colt(&growing_view);
    colt.bind(Some(&tight));
    let error = colt
        .force_root()
        .expect_err("a later resize must refuse instead of probing a full table");
    assert!(
        is_working_exhaustion(&error),
        "typed WorkingBytes refusal licenses the bounded fallback, got {error:?}"
    );
    assert!(
        colt.forced_capacity(Colt::root()).is_none(),
        "resize refusal rolls back; probe_walk never sees a saturated map"
    );
    let drained = iter_once(&mut colt, Colt::root());
    assert!(
        drained.is_err(),
        "iteration must not succeed as empty output after resize refusal, got {drained:?}"
    );
}

/// Same-shaped re-execution reuses retained capacity and does not re-charge.
#[test]
fn repeated_same_shape_executions_plateau_capacity_and_charges() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..64).map(|i| (i % 8, i)).collect();
    let image = view_of(&schema, &rows);
    let work = working(u64::MAX);
    let mut colt = join_colt(&image);
    colt.bind(Some(&work));
    colt.force_root().expect("first force");
    let retained = colt.retained_bytes();
    let charged = colt.charged_bytes();
    assert!(retained > 0 && charged > 0);
    for round in 0..8 {
        let old = colt.reset(all(&image));
        drop(old);
        colt.bind(Some(&work));
        colt.force_root().expect("repeat force");
        assert_eq!(
            colt.retained_bytes(),
            retained,
            "retained capacity plateaued at round {round}"
        );
        assert_eq!(
            colt.charged_bytes(),
            charged,
            "retained charges plateaued at round {round}"
        );
    }
}

#[test]
fn forced_probes_preserve_pools_and_work_until_reset_requires_construction() {
    let schema = schema();
    let rows: Vec<_> = (0..16).map(|i| (i, i)).collect();
    let image = view_of(&schema, &rows);
    let work = working(u64::MAX);
    let mut colt = join_colt(&image);
    colt.bind(Some(&work));
    colt.force_root().unwrap();
    let root = Colt::root();
    let hit = colt.get_prehashed(root, 0, &[7], hash_key(&[7])).unwrap();
    assert!(hit.is_some());
    let lengths = |colt: &Colt| {
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
    };
    let before = lengths(&colt);
    let retained = colt.retained_bytes();
    let charged = colt.charged_bytes();
    let units = work.used(Resource::WorkUnits);
    let bytes = work.used(Resource::WorkingBytes);
    for _ in 0..32 {
        assert_eq!(
            colt.get_prehashed(root, 0, &[7], hash_key(&[7])).unwrap(),
            hit
        );
        assert_eq!(
            colt.get_prehashed(root, 0, &[99], hash_key(&[99])).unwrap(),
            None
        );
    }
    assert_eq!(lengths(&colt), before);
    assert_eq!(colt.retained_bytes(), retained);
    assert_eq!(colt.charged_bytes(), charged);
    assert_eq!(work.used(Resource::WorkingBytes), bytes);
    assert_eq!(work.used(Resource::WorkUnits), units);

    // A different same-shaped view invalidates the forced map. Retained
    // allocation cannot bypass construction, its work poll, or rollback.
    let replacement: Vec<_> = (100..116).map(|i| (i, i)).collect();
    let replacement = view_of(&schema, &replacement);
    drop(colt.reset(all(&replacement)));
    let stopped = working(u64::MAX);
    stopped.cancel();
    colt.bind(Some(&stopped));
    assert_eq!(colt.force_root(), Err(WorkError::Cancelled));
    assert!(colt.forced_capacity(root).is_none());
    let resumed = working(u64::MAX);
    colt.bind(Some(&resumed));
    colt.force_root().unwrap();
    assert!(resumed.used(Resource::WorkUnits) > 0);
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
    assert_eq!(colt.charged_bytes(), charged);
}

/// Rebinding a fresh ledger clears a cancelled prior context.
#[test]
fn bind_resets_cancelled_work_without_poisoning_the_next_execution() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..16).map(|i| (i, i)).collect();
    let view = view_of(&schema, &rows);
    let cancelled = working(u64::MAX);
    cancelled.cancel();
    let mut colt = join_colt(&view);
    colt.bind(Some(&cancelled));
    assert_eq!(
        colt.force_root(),
        Err(WorkError::Cancelled),
        "the cancelled ledger refuses this operation"
    );
    assert!(
        colt.forced_capacity(Colt::root()).is_none(),
        "cancellation rolls back; the next bind starts clean"
    );

    let fresh = working(u64::MAX);
    colt.bind(Some(&fresh));
    colt.force_root()
        .expect("a new bind must not inherit the cancelled ledger");
    assert!(
        colt.forced_capacity(Colt::root()).is_some(),
        "the rebound execution forced a real map"
    );
    let drained = iter_once(&mut colt, Colt::root()).expect("rebound iteration");
    assert!(
        drained.0 > 0,
        "rebound force produced keys, not empty output"
    );
}

#[test]
fn pool_growth_is_geometric_and_failed_growth_has_no_phantom_charge() {
    let work = working(u64::MAX);
    let mut pool = Vec::<u32>::new();
    let mut charges = Vec::new();
    let mut allocations = 0;
    for value in 0..4096 {
        let capacity = pool.capacity();
        super::super::reserve_pool(pool.len() + 1, &mut pool, Some(&work), &mut charges).unwrap();
        allocations += usize::from(capacity != pool.capacity());
        pool.push(value);
    }
    assert!(
        allocations <= 10,
        "one allocation per doubling, not per distinct key"
    );
    let retained = pool.capacity() * std::mem::size_of::<u32>()
        + charges.capacity() * std::mem::size_of::<crate::work::ByteReservation>();
    assert_eq!(work.used(Resource::WorkingBytes), retained as u64);

    let tight = working(0);
    let capacity = pool.capacity();
    let count = charges.len();
    assert!(
        super::super::reserve_pool(capacity + 1, &mut pool, Some(&tight), &mut charges).is_err()
    );
    assert_eq!(pool.capacity(), capacity);
    assert_eq!(charges.len(), count);
    assert_eq!(tight.used(Resource::WorkingBytes), 0);
    assert_eq!(pool, (0..4096).collect::<Vec<_>>());
    drop(pool);
    drop(charges);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}
