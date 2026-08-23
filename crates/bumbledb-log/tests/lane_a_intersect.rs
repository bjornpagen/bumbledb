//! The loser algebra's pairwise decision, cell by cell: subsumption,
//! strict key disjointness (commute cells included), and the W-class
//! interval test with evaporation widening at its exact boundary. The
//! winner's footprint is never an input — `intersect` recomputes it
//! from the winner's ops, so an understating section cannot steer the
//! decision.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::BTreeMap;

use bumbledb::schema::{RelationId, StatementId};
use bumbledb::{Interval, Value};
use bumbledb_log::codec::{Op, OpKind};
use bumbledb_log::footprint::{CapacityKey, Vocabulary, capacity_profiles, footprint};
use bumbledb_log::intersect::{BaseMeasure, ConflictCause, LoserDecision, intersect};

fn vocabulary() -> Vocabulary {
    Vocabulary::new(&support::schema("booking")).expect("fixture vocabulary")
}

fn booking(slot: u64, customer: u64, room: u64, qty: u64) -> Box<[Value]> {
    Box::from([
        Value::U64(slot),
        Value::U64(customer),
        Value::U64(room),
        Value::U64(qty),
        Value::IntervalU64(Interval::new(0, 5).expect("interval")),
    ])
}

fn insert(relation: u32, rows: Vec<Box<[Value]>>) -> Op {
    Op {
        kind: OpKind::Insert,
        relation: RelationId(relation),
        rows,
    }
}

fn delete(relation: u32, rows: Vec<Box<[Value]>>) -> Op {
    Op {
        kind: OpKind::Delete,
        relation: RelationId(relation),
        rows,
    }
}

fn decide(
    vocab: &Vocabulary,
    loser_ops: &[Op],
    winner_ops: &[Op],
    base: &BTreeMap<CapacityKey, BaseMeasure>,
) -> LoserDecision {
    let loser_footprint = footprint(vocab, loser_ops).expect("loser footprint");
    intersect(vocab, &loser_footprint, loser_ops, winner_ops, base).expect("intersect")
}

fn room_base(
    vocab: &Vocabulary,
    ops: &[Op],
    measure: u64,
    ceiling: Option<u64>,
) -> BTreeMap<CapacityKey, BaseMeasure> {
    capacity_profiles(vocab, ops)
        .expect("profiles")
        .into_keys()
        .map(|key| {
            (
                key,
                BaseMeasure {
                    measure,
                    floor: 0,
                    ceiling,
                },
            )
        })
        .collect()
}

#[test]
fn the_booking_race_conflicts_on_the_shared_determinant() {
    let vocab = vocabulary();
    let loser = [insert(1, vec![booking(5, 1, 4, 1)])];
    let winner = [insert(1, vec![booking(5, 2, 6, 1)])];
    let base = room_base(&vocab, &loser, 0, Some(10));
    let decision = decide(&vocab, &loser, &winner, &base);
    assert!(matches!(
        decision,
        LoserDecision::Conflict(ConflictCause::Key {
            statement: StatementId(0),
            ..
        })
    ));
}

#[test]
fn a_shared_commute_cell_containment_key_still_re_judges() {
    let vocab = vocabulary();
    // Different slots and rooms, one customer group: need x need is a
    // commute cell, and strict disjointness re-judges it anyway.
    let loser = [insert(1, vec![booking(5, 1, 4, 1)])];
    let winner = [insert(1, vec![booking(6, 1, 8, 1)])];
    let base = room_base(&vocab, &loser, 0, Some(10));
    let decision = decide(&vocab, &loser, &winner, &base);
    assert!(matches!(
        decision,
        LoserDecision::Conflict(ConflictCause::Containment {
            statement: StatementId(1),
            ..
        })
    ));
}

#[test]
fn fully_disjoint_batches_republish() {
    let vocab = vocabulary();
    let loser = [insert(1, vec![booking(5, 1, 4, 1)])];
    let winner = [insert(1, vec![booking(6, 2, 8, 1)])];
    let decision = decide(&vocab, &loser, &winner, &BTreeMap::new());
    assert_eq!(decision, LoserDecision::Disjoint);
}

#[test]
fn identical_and_superset_winners_subsume() {
    let vocab = vocabulary();
    let loser = [insert(1, vec![booking(5, 1, 4, 1)])];
    let winner_same = [insert(1, vec![booking(5, 1, 4, 1)])];
    let decision = decide(&vocab, &loser, &winner_same, &BTreeMap::new());
    assert_eq!(decision, LoserDecision::Subsumed);

    let winner_more = [insert(1, vec![booking(5, 1, 4, 1), booking(6, 2, 8, 1)])];
    let decision = decide(&vocab, &loser, &winner_more, &BTreeMap::new());
    assert_eq!(decision, LoserDecision::Subsumed);
}

#[test]
fn a_shared_fact_with_the_opposite_mode_conflicts() {
    let vocab = vocabulary();
    let loser = [insert(1, vec![booking(5, 1, 4, 1)])];
    let winner = [delete(1, vec![booking(5, 1, 4, 1)])];
    let decision = decide(&vocab, &loser, &winner, &BTreeMap::new());
    assert!(matches!(
        decision,
        LoserDecision::Conflict(ConflictCause::Fact { .. })
    ));
}

#[test]
fn the_interval_test_passes_at_slack_and_fails_past_it() {
    let vocab = vocabulary();
    // One room, two child inserts of weight 3: worst-case sum 6.
    let loser = [insert(1, vec![booking(1, 1, 4, 3)])];
    let winner = [insert(1, vec![booking(2, 2, 4, 3)])];

    // slack+ = 10 - 4 = 6: exactly at the bound, commute.
    let base = room_base(&vocab, &loser, 4, Some(10));
    assert_eq!(
        decide(&vocab, &loser, &winner, &base),
        LoserDecision::Disjoint
    );

    // slack+ = 10 - 5 = 5: one past the bound, conflict.
    let base = room_base(&vocab, &loser, 5, Some(10));
    assert!(matches!(
        decide(&vocab, &loser, &winner, &base),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: StatementId(2),
            ..
        })
    ));

    // Unbounded ceiling: only the floor constrains, and it holds.
    let base = room_base(&vocab, &loser, 0, None);
    assert_eq!(
        decide(&vocab, &loser, &winner, &base),
        LoserDecision::Disjoint
    );
}

#[test]
fn evaporation_widens_a_delete_below_its_published_delta() {
    let vocab = vocabulary();
    // Loser deletes weight 5 (interval [-5, 0]); winner inserts weight
    // 3 (interval [0, 3]). A point test on the published deltas would
    // read the joint minimum as -2 and pass at measure 4; the widened
    // minimum is -5 and must refuse there.
    let loser = [delete(1, vec![booking(1, 1, 4, 5)])];
    let winner = [insert(1, vec![booking(2, 2, 4, 3)])];

    let base = room_base(&vocab, &loser, 5, Some(100));
    assert_eq!(
        decide(&vocab, &loser, &winner, &base),
        LoserDecision::Disjoint
    );

    let base = room_base(&vocab, &loser, 4, Some(100));
    assert!(matches!(
        decide(&vocab, &loser, &winner, &base),
        LoserDecision::Conflict(ConflictCause::CapacityInterval {
            statement: StatementId(2),
            ..
        })
    ));
}

#[test]
fn parent_removal_races_any_live_child_interval() {
    let vocab = vocabulary();
    let child = [insert(1, vec![booking(1, 1, 4, 1)])];
    let parent_remove = [delete(
        2,
        vec![Box::from([Value::U64(4), Value::U64(9)]) as Box<[Value]>],
    )];
    let base = room_base(&vocab, &child, 0, Some(10));

    for (loser, winner) in [
        (&child[..], &parent_remove[..]),
        (&parent_remove[..], &child[..]),
    ] {
        assert!(matches!(
            decide(&vocab, loser, winner, &base),
            LoserDecision::Conflict(ConflictCause::CapacityParent {
                statement: StatementId(2),
                ..
            })
        ));
    }
}

#[test]
fn parent_addition_commutes_with_child_adds_only() {
    let vocab = vocabulary();
    let child_add = [insert(1, vec![booking(1, 1, 4, 2)])];
    let child_remove = [delete(1, vec![booking(1, 1, 4, 2)])];
    let parent_add = [insert(
        2,
        vec![Box::from([Value::U64(4), Value::U64(9)]) as Box<[Value]>],
    )];
    let base = room_base(&vocab, &child_add, 0, Some(10));

    assert_eq!(
        decide(&vocab, &child_add, &parent_add, &base),
        LoserDecision::Disjoint
    );
    assert!(matches!(
        decide(&vocab, &child_remove, &parent_add, &base),
        LoserDecision::Conflict(ConflictCause::CapacityParent { .. })
    ));
}

#[test]
fn a_missing_base_measure_is_a_conservative_conflict() {
    let vocab = vocabulary();
    let loser = [insert(1, vec![booking(1, 1, 4, 1)])];
    let winner = [insert(1, vec![booking(2, 2, 4, 1)])];
    assert!(matches!(
        decide(&vocab, &loser, &winner, &BTreeMap::new()),
        LoserDecision::Conflict(ConflictCause::CapacityMeasureMissing {
            statement: StatementId(2),
            ..
        })
    ));
}
