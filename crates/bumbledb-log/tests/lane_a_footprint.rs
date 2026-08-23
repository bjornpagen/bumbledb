//! Footprint-law pins beyond the corpus: net disposition, selection
//! filtering, the three weight forms, delta overflow as a typed
//! refusal, closed emptiness, and the empty-determinant single group.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb::schema::{RelationId, StatementId};
use bumbledb::{Interval, Value};
use bumbledb_log::codec::{Op, OpKind};
use bumbledb_log::footprint::{
    CapacityKey, CapacityMode, Entry, FootprintError, Vocabulary, capacity_profiles, footprint,
};

fn vocabulary(schema: &str) -> Vocabulary {
    Vocabulary::new(&support::schema(schema)).expect("fixture vocabulary")
}

fn booking_row(slot: u64, customer: u64, room: u64, qty: u64, span: (u64, u64)) -> Box<[Value]> {
    Box::from([
        Value::U64(slot),
        Value::U64(customer),
        Value::U64(room),
        Value::U64(qty),
        Value::IntervalU64(Interval::new(span.0, span.1).expect("interval")),
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

fn facts(entries: &[Entry]) -> Vec<(OpKind, [u8; 32])> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Fact { fid, mode } => Some((*mode, *fid)),
            _ => None,
        })
        .collect()
}

fn child_deltas(entries: &[Entry]) -> Vec<(u16, i64)> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Capacity {
                statement,
                mode: CapacityMode::ChildDelta(delta),
                ..
            } => Some((statement.0, *delta)),
            _ => None,
        })
        .collect()
}

#[test]
fn net_disposition_last_op_wins() {
    let vocab = vocabulary("booking");
    let row = booking_row(1, 1, 4, 5, (0, 5));

    // Insert then delete of one row: a single net delete survives.
    let entries = footprint(
        &vocab,
        &[insert(1, vec![row.clone()]), delete(1, vec![row.clone()])],
    )
    .expect("footprint");
    let f = facts(&entries);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].0, OpKind::Delete);
    assert_eq!(child_deltas(&entries), vec![(2, -5)]);

    // Delete then insert: a single net insert.
    let entries = footprint(
        &vocab,
        &[delete(1, vec![row.clone()]), insert(1, vec![row.clone()])],
    )
    .expect("footprint");
    let f = facts(&entries);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].0, OpKind::Insert);
    assert_eq!(child_deltas(&entries), vec![(2, 5)]);

    // A duplicated insert nets to one entry with one weight.
    let entries = footprint(
        &vocab,
        &[insert(1, vec![row.clone()]), insert(1, vec![row])],
    )
    .expect("footprint");
    assert_eq!(facts(&entries).len(), 1);
    assert_eq!(child_deltas(&entries), vec![(2, 5)]);
}

#[test]
fn deltas_merge_per_parent_key_and_zero_survives() {
    let vocab = vocabulary("booking");
    // Two children of one room: +5 and -3 merge to one +2 entry.
    let entries = footprint(
        &vocab,
        &[
            insert(1, vec![booking_row(1, 1, 4, 5, (0, 5))]),
            delete(1, vec![booking_row(2, 1, 4, 3, (0, 5))]),
        ],
    )
    .expect("footprint");
    assert_eq!(child_deltas(&entries), vec![(2, 2)]);

    // Balanced spend across distinct rows: the zero-delta entry stays
    // on the wire (the reservation-spend shape).
    let entries = footprint(
        &vocab,
        &[
            delete(1, vec![booking_row(1, 1, 4, 5, (0, 5))]),
            insert(1, vec![booking_row(2, 1, 4, 5, (0, 5))]),
        ],
    )
    .expect("footprint");
    assert_eq!(child_deltas(&entries), vec![(2, 0)]);
}

#[test]
fn selection_gates_participation_and_duration_weighs() {
    let vocab = vocabulary("booking");
    // qty = 2 satisfies the duration capacity's selection: both
    // capacities emit; the duration weight is the interval measure.
    let entries = footprint(
        &vocab,
        &[insert(1, vec![booking_row(1, 1, 4, 2, (10, 25))])],
    )
    .expect("footprint");
    let mut deltas = child_deltas(&entries);
    deltas.sort_unstable();
    assert_eq!(deltas, vec![(2, 2), (3, 15)]);

    // qty = 1 fails the selection: only the field-weight capacity.
    let entries = footprint(
        &vocab,
        &[insert(1, vec![booking_row(1, 1, 4, 1, (10, 25))])],
    )
    .expect("footprint");
    assert_eq!(child_deltas(&entries), vec![(2, 1)]);
}

#[test]
fn unit_weight_counts_rows_at_one_global_group() {
    let vocab = vocabulary("multi");
    let entries = footprint(
        &vocab,
        &[insert(
            6,
            vec![
                Box::from([Value::U64(1)]) as Box<[Value]>,
                Box::from([Value::U64(2)]),
            ],
        )],
    )
    .expect("footprint");
    // Two distinct rows, one empty determinant: one merged entry of 2.
    assert_eq!(child_deltas(&entries), vec![(6, 2)]);
}

#[test]
fn delta_overflow_is_a_typed_refusal() {
    let vocab = vocabulary("booking");
    let error = footprint(
        &vocab,
        &[insert(1, vec![booking_row(1, 1, 4, u64::MAX, (0, 5))])],
    )
    .expect_err("overflowing delta refuses");
    assert!(matches!(
        error,
        FootprintError::DeltaOverflow {
            statement: StatementId(2),
            ..
        }
    ));
}

#[test]
fn closed_relations_and_closed_targets_emit_nothing() {
    let vocab = vocabulary("multi");
    // B is the source of one ordinary-target and one closed-target
    // containment: only the ordinary one emits a need.
    let entries =
        footprint(&vocab, &[insert(1, vec![Box::from([Value::U64(1)])])]).expect("footprint");
    let containments: Vec<u16> = entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Containment { statement, .. } => Some(statement.0),
            _ => None,
        })
        .collect();
    assert_eq!(containments, vec![2]);

    // Ops on the closed relation itself are refused, typed.
    let error = footprint(
        &vocab,
        &[insert(4, vec![Box::from([Value::String("o".into())])])],
    )
    .expect_err("closed relation refuses");
    assert!(matches!(error, FootprintError::ClosedRelation { .. }));
}

#[test]
fn empty_determinant_names_one_group() {
    let vocab = vocabulary("multi");
    let a = footprint(&vocab, &[insert(6, vec![Box::from([Value::U64(1)])])]).expect("footprint");
    let b = footprint(&vocab, &[insert(6, vec![Box::from([Value::U64(2)])])]).expect("footprint");
    let key_of = |entries: &[Entry]| {
        entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Capacity { key, .. } => Some(*key),
                _ => None,
            })
            .expect("capacity entry")
    };
    assert_eq!(key_of(&a), key_of(&b));
}

#[test]
fn profiles_agree_with_published_deltas_and_carry_widening() {
    let vocab = vocabulary("booking");
    let ops = [
        delete(1, vec![booking_row(1, 1, 4, 5, (0, 5))]),
        insert(1, vec![booking_row(2, 1, 4, 3, (0, 5))]),
        insert(
            2,
            vec![Box::from([Value::U64(4), Value::U64(9)]) as Box<[Value]>],
        ),
    ];
    let entries = footprint(&vocab, &ops).expect("footprint");
    let profiles = capacity_profiles(&vocab, &ops).expect("profiles");

    for entry in &entries {
        if let Entry::Capacity {
            statement,
            key,
            mode: CapacityMode::ChildDelta(delta),
        } = entry
        {
            let profile = profiles
                .get(&CapacityKey {
                    statement: *statement,
                    key: *key,
                })
                .expect("profile per published delta");
            assert_eq!(profile.delta, i128::from(*delta));
        }
    }

    let room_profile = profiles
        .iter()
        .find(|(key, profile)| key.statement == StatementId(2) && profile.parent_add)
        .map(|(_, profile)| *profile)
        .expect("room group profile");
    // Net delta -2, insert widens down by 3, delete widens up by 5.
    assert_eq!(room_profile.delta, -2);
    assert_eq!(room_profile.min(), -5);
    assert_eq!(room_profile.max(), 3);
    assert!(room_profile.parent_add);
    assert!(!room_profile.parent_remove);
}

#[test]
fn arity_and_value_shape_refuse_with_context() {
    let vocab = vocabulary("booking");
    let error = footprint(&vocab, &[insert(1, vec![Box::from([Value::U64(1)])])])
        .expect_err("short row refuses");
    assert!(matches!(
        error,
        FootprintError::Arity {
            op: 0,
            relation: RelationId(1),
            row: 0
        }
    ));

    let error = footprint(
        &vocab,
        &[insert(
            1,
            vec![Box::from([
                Value::U64(1),
                Value::U64(1),
                Value::U64(4),
                Value::Bool(true),
                Value::IntervalU64(Interval::new(0, 5).expect("interval")),
            ])],
        )],
    )
    .expect_err("mistyped field refuses");
    assert!(matches!(error, FootprintError::Value { field: 3, .. }));
}
