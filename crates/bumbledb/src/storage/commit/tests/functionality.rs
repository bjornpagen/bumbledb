//! The pointwise-key matrix (PRD 07 criteria): per-cell tests of the
//! ordered-neighbor probe, each in one delta and across deltas, plus the
//! ray (`end == MAX` = `[s, ∞)`) and delete-then-reinsert cases.
//!
//! The incumbent everywhere is `Booking(room 1, [10, 20), tag 0)`; each
//! cell inserts one contender and asserts the judgment.

use super::*;

use crate::error::{Error, Violation};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// Applies both facts as inserts in one delta against an empty base.
fn in_delta(name: &str, a: &[u8], b: &[u8]) -> crate::error::Result<()> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, BOOKING, a).expect("record insert");
    delta.insert(&view, BOOKING, b).expect("record insert");
    drop(view);
    let result = commit(delta, &env).map(|_| ());
    if result.is_err() {
        // An aborted commit leaves the base state untouched.
        assert!(committed_data(&env).is_empty());
    }
    result
}

/// Commits `first`, then inserts `second` in a fresh delta.
fn cross_delta(name: &str, first: &[u8], second: &[u8]) -> crate::error::Result<()> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, BOOKING, first).expect("record insert");
        drop(view);
        commit(delta, &env).expect("base commit");
    }
    let before = committed_data(&env);
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.insert(&view, BOOKING, second).expect("record insert");
    drop(view);
    let result = commit(delta, &env).map(|_| ());
    if result.is_err() {
        assert_eq!(committed_data(&env), before);
    }
    result
}

/// In-delta violation: application order follows fact-hash order, so
/// either fact may be the incumbent — assert the pair, not the roles.
fn assert_in_delta_violation(result: crate::error::Result<()>, a: &[u8], b: &[u8]) {
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Functionality {
            statement,
            fact,
            incumbent,
        },
    ] = violations.as_slice()
    else {
        panic!("expected one key citation, got {violations:?}");
    };
    assert_eq!(*statement, BOOKING_KEY);
    let incumbent = incumbent
        .as_deref()
        .expect("pointwise arm names both facts");
    assert!(
        (**fact == *a && incumbent == b) || (**fact == *b && incumbent == a),
        "violation names {fact:?} against {incumbent:?}"
    );
}

/// Cross-delta violation: the committed fact is the incumbent, the new
/// fact the offender — the roles are deterministic.
fn assert_cross_delta_violation(result: crate::error::Result<()>, first: &[u8], second: &[u8]) {
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Functionality {
            statement,
            fact,
            incumbent,
        },
    ] = violations.as_slice()
    else {
        panic!("expected one key citation, got {violations:?}");
    };
    assert_eq!(*statement, BOOKING_KEY);
    assert_eq!(**fact, *second);
    assert_eq!(incumbent.as_deref(), Some(first));
}

// ---------- the matrix: violating cells ----------

#[test]
fn overlap_left_in_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 5, 15, 1);
    assert_in_delta_violation(in_delta("fd-overlap-left-in", &a, &b), &a, &b);
}

#[test]
fn overlap_left_cross_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 5, 15, 1);
    assert_cross_delta_violation(cross_delta("fd-overlap-left-cross", &a, &b), &a, &b);
}

#[test]
fn overlap_right_in_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 15, 25, 1);
    assert_in_delta_violation(in_delta("fd-overlap-right-in", &a, &b), &a, &b);
}

#[test]
fn overlap_right_cross_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 15, 25, 1);
    assert_cross_delta_violation(cross_delta("fd-overlap-right-cross", &a, &b), &a, &b);
}

#[test]
fn containment_in_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 12, 18, 1);
    assert_in_delta_violation(in_delta("fd-containment-in", &a, &b), &a, &b);
}

#[test]
fn containment_cross_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 12, 18, 1);
    assert_cross_delta_violation(cross_delta("fd-containment-cross", &a, &b), &a, &b);
}

#[test]
fn exact_duplicate_interval_in_delta_aborts() {
    // Distinct facts (the tag differs) sharing one exact determinant: caught
    // by the put-conflict, not the neighbor probe.
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 10, 20, 1);
    assert_in_delta_violation(in_delta("fd-exact-dup-in", &a, &b), &a, &b);
}

#[test]
fn exact_duplicate_interval_cross_delta_aborts() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 10, 20, 1);
    assert_cross_delta_violation(cross_delta("fd-exact-dup-cross", &a, &b), &a, &b);
}

// ---------- the matrix: passing cells ----------

#[test]
fn adjacent_left_in_delta_passes() {
    // `pe == s`: half-open adjacency shares no point.
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 5, 10, 1);
    in_delta("fd-adjacent-left-in", &a, &b).expect("adjacency is legal");
}

#[test]
fn adjacent_left_cross_delta_passes() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 5, 10, 1);
    cross_delta("fd-adjacent-left-cross", &a, &b).expect("adjacency is legal");
}

#[test]
fn adjacent_right_in_delta_passes() {
    // `ns == e`: the successor may start exactly where the insert ends.
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 20, 25, 1);
    in_delta("fd-adjacent-right-in", &a, &b).expect("adjacency is legal");
}

#[test]
fn adjacent_right_cross_delta_passes() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 20, 25, 1);
    cross_delta("fd-adjacent-right-cross", &a, &b).expect("adjacency is legal");
}

#[test]
fn disjoint_in_delta_passes() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 30, 40, 1);
    in_delta("fd-disjoint-in", &a, &b).expect("disjoint intervals coexist");
}

#[test]
fn disjoint_cross_delta_passes() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 1, 30, 40, 1);
    cross_delta("fd-disjoint-cross", &a, &b).expect("disjoint intervals coexist");
}

#[test]
fn same_interval_different_prefix_in_delta_passes() {
    // The scalar prefix is the group: another room, same interval.
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 2, 10, 20, 1);
    in_delta("fd-other-prefix-in", &a, &b).expect("groups are independent");
}

#[test]
fn same_interval_different_prefix_cross_delta_passes() {
    let schema = schema();
    let a = booking_fact(&schema, 1, 10, 20, 0);
    let b = booking_fact(&schema, 2, 10, 20, 1);
    cross_delta("fd-other-prefix-cross", &a, &b).expect("groups are independent");
}

// ---------- final-state judgment ----------

#[test]
fn delete_then_reinsert_overlapping_in_one_delta_passes() {
    // Judged against the final state: the delete frees the window, so
    // the overlapping replacement lands — deletes apply before inserts.
    let dir = TempDir::new("fd-delete-reinsert");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = booking_fact(&schema, 1, 10, 20, 0);
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, BOOKING, &old).expect("record insert");
        drop(view);
        commit(delta, &env).expect("base commit");
    }
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    delta.delete(&view, BOOKING, &old).expect("record delete");
    delta
        .insert(&view, BOOKING, &booking_fact(&schema, 1, 15, 25, 1))
        .expect("record insert");
    drop(view);
    commit(delta, &env).expect("the freed window admits the replacement");
}

// ---------- rays (`end == MAX` denotes `[s, ∞)`; no special code) ----------

#[test]
fn two_rays_in_one_group_abort() {
    // `[5, ∞)` and `[9, ∞)`: two rays share every point past the later
    // start, so a pointwise key can never hold both — the ordinary strict
    // comparisons judge the overlap, since ∞ is just the largest end.
    let schema = schema();
    let a = booking_fact(&schema, 1, 5, u64::MAX, 0);
    let b = booking_fact(&schema, 1, 9, u64::MAX, 1);
    assert_cross_delta_violation(cross_delta("fd-ray-overlap", &a, &b), &a, &b);
}

#[test]
fn bounded_interval_adjacent_to_ray_passes() {
    // `[5, 9)` then `[9, ∞)`: adjacency at the ray's start.
    let schema = schema();
    let a = booking_fact(&schema, 1, 5, 9, 0);
    let b = booking_fact(&schema, 1, 9, u64::MAX, 1);
    cross_delta("fd-ray-adjacent", &a, &b).expect("adjacency at the ray's start is legal");
}
