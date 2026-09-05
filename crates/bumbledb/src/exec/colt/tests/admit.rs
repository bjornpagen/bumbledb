//! Allocation-refusal discriminators for fallible COLT force/growth.
//! Verification: NotRun. Each gate consumes the production force, chunk,
//! and resize boundaries — not a `type_name` / `size_of` / fn-ref claim.

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
    let mut children = vec![Cursor::Row(0); 8];
    colt.iter_batch(cursor, 0, BatchToken::default(), &mut keys, &mut children, 8)
}

/// First map admission refuses before `maps[0]` can be indexed.
/// Verification: NotRun.
#[test]
fn first_map_refusal_is_typed_and_does_not_index() {
    let schema = schema();
    let rows: Vec<(u64, u64)> = (0..32).map(|i| (i, i)).collect();
    let view = view_of(&schema, &rows);
    let work = working(0);
    let mut colt = join_colt(&view);
    colt.bind(Some(&work));
    let root = Colt::root();
    let error = colt.force_root().expect_err("zero working bytes refuse the first map");
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
/// Verification: NotRun.
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

/// A later grow_map resize refuses before ingest continues into a full table.
/// Verification: NotRun.
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
/// Verification: NotRun.
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

/// Rebinding a fresh ledger clears a cancelled prior context.
/// Verification: NotRun.
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
    assert!(drained.0 > 0, "rebound force produced keys, not empty output");
}
