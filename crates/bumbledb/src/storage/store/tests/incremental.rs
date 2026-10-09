//! Incremental judgment against complete judgment on the real candidate:
//! verdicts, violation sets and canonical evidence bytes are equal over
//! randomized mutations of every statement family, also under forced
//! fingerprint collisions. The lawful-parent premise is pinned honestly:
//! an unlawful parent hides from incremental judgment, and the sweeper's
//! complete judgment convicts it.

use super::*;
use crate::schema::judge::{JudgedViolation, Judgment};
use crate::schema::{FieldId, Side, StatementDescriptor, StatementKind, Weight};
use crate::storage::store::fingerprint::FP_LEN;
use crate::storage::store::verify::{self, VerifyFinding};
use bumbledb_theory::schema::{Bound, StatementId};

const USER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const ROOM: RelationId = RelationId(2);
const USER_EMAIL_KEY: StatementId = StatementId(1);
const BOOKING_ROOM_EXISTS: StatementId = StatementId(4);

/// `User(id)`, `User(email)`, pointwise `Booking(room, span)`, `Room(id)`,
/// `Booking(room) ⊆ Room(id)`, `Booking(room) <= {0..2} Room(id)` — every
/// judged family, on the physical store.
fn delta_schema() -> Schema {
    use bumbledb_theory::schema::IntervalElement;
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::from([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "User".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "email".into(),
                        value_type: ValueType::String,
                    },
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Booking".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "room".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "span".into(),
                        value_type: ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    },
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Room".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                }],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::from([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: ROOM,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(BOOKING, &[0]),
                target: side(ROOM, &[0]),
            },
            StatementDescriptor::Capacity {
                target: side(ROOM, &[0]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(2)),
                source: side(BOOKING, &[0]),
            },
        ],
    }
    .validate()
    .expect("delta schema validates")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

fn booking(room: u64, start: u64, end: u64) -> Vec<Value> {
    vec![
        Value::U64(room),
        Value::IntervalU64(crate::Interval::new(start, end).expect("nonempty span")),
    ]
}

fn room(id: u64) -> Vec<Value> {
    vec![Value::U64(id)]
}

fn evidence_bytes(schema: &Schema, judged: &[JudgedViolation]) -> Vec<u8> {
    crate::schema::evidence::encode_judged(schema, judged, 1 << 20, &work())
        .expect("evidence encodes")
}

fn build_changes(
    schema: &Schema,
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
) -> ChangeSet {
    change_set(schema, adds, removes)
}

/// Judge one delta both ways, require equal verdicts and evidence bytes,
/// and commit it when admitted. Returns the incremental verdict.
fn compare(store: &Store, schema: &Schema, changes: &ChangeSet) -> Judgment {
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let (incremental, complete) = owner.judge_both(schema, changes).expect("judge both");
    match (&incremental, &complete) {
        (Judgment::Admitted, Judgment::Admitted) => {}
        (Judgment::Rejected(mine), Judgment::Rejected(reference)) => {
            assert_eq!(mine, reference, "violation sets must be equal");
            assert_eq!(
                evidence_bytes(schema, mine),
                evidence_bytes(schema, reference),
                "canonical evidence bytes must be equal"
            );
        }
        (mine, reference) => {
            panic!("verdicts diverged: incremental {mine:?} vs complete {reference:?}")
        }
    }
    drop(owner);
    if incremental == Judgment::Admitted {
        judged_commit(store, schema, changes).expect("the admitted delta commits");
    }
    incremental
}

fn compare_and_commit(
    store: &Store,
    schema: &Schema,
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
) -> bool {
    compare(store, schema, &build_changes(schema, adds, removes)) == Judgment::Admitted
}

fn rejected(judgment: Judgment) -> Box<[JudgedViolation]> {
    match judgment {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("expected a rejection"),
    }
}

fn forced(path: &std::path::Path, schema: &Schema, fp: [u8; FP_LEN]) -> Store {
    Store::create_forced_fingerprint(path, schema, fp).expect("forced-collision store")
}

/// Exercise physical buckets through the existing complete/incremental
/// differential judge, including byte-for-byte canonical rejection evidence.
#[test]
fn interval_prefix_order_coverage_collisions_and_cleanup() {
    use crate::schema::{FixedIntervalElement, IntervalElement};
    for target_type in [
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        ValueType::Interval {
            element: IntervalElement::I64,
        },
        ValueType::Interval {
            element: IntervalElement::F64,
        },
        ValueType::FixedInterval {
            element: FixedIntervalElement::U64,
            width: 10,
        },
        ValueType::FixedInterval {
            element: FixedIntervalElement::I64,
            width: 10,
        },
    ] {
        let (schema, span) = interval_coverage_fixture(target_type);
        let (_dir, path) = store_dir("interval-prefix-differential");
        let store = forced(&path, &schema, [0xA5; FP_LEN]);
        let target_row = |group: &str, start, end, id| {
            (
                RelationId(0),
                vec![
                    Value::String(group.into()),
                    span(start, end),
                    Value::U64(id),
                ],
            )
        };
        let source_row = |group: &str, start, end| {
            (
                RelationId(1),
                vec![Value::String(group.into()), span(start, end)],
            )
        };
        let commit = |adds: &[_], removes: &[_]| compare_and_commit(&store, &schema, adds, removes);
        let late = target_row("a", 10, 20, 1);
        let early = target_row("a", 0, 10, 2);
        let foreign = target_row("b", 20, 30, 3);
        // Neither the physical primary home nor row ordinal has endpoint order.
        assert!(commit(std::slice::from_ref(&late), &[]));
        assert!(commit(&[early.clone(), foreign.clone()], &[]));
        let request = source_row("a", 0, 20);
        assert!(commit(std::slice::from_ref(&request), &[]));
        // A colliding foreign group cannot supply a missing span. Overlap
        // keeps byte-identical rejection evidence despite changed scan order.
        assert!(!commit(&[source_row("a", 20, 30)], &[]));
        assert!(!commit(&[target_row("a", 5, 15, 4)], &[]));
        assert!(!commit(&[], std::slice::from_ref(&early)));
        // Remove the dependency with its interval: no stale index may supply it.
        assert!(commit(&[], &[request.clone(), early.clone()]));
        assert!(!commit(std::slice::from_ref(&request), &[]));
        assert!(commit(std::slice::from_ref(&early), &[]));
        assert!(commit(std::slice::from_ref(&request), &[]));
        {
            let snapshot = store.snapshot(&work()).expect("snapshot");
            assert_eq!(
                store
                    .inner
                    .dets
                    .len(snapshot.read_txn())
                    .expect("det count"),
                4,
                "three target references and one source reference"
            );
        }
        assert!(commit(&[], &[late, early, foreign, request]));
        let context = work();
        let snapshot = store.snapshot(&context).expect("empty snapshot");
        assert!(
            store
                .inner
                .rows
                .is_empty(snapshot.read_txn())
                .expect("rows")
        );
        assert!(
            store
                .inner
                .dets
                .is_empty(snapshot.read_txn())
                .expect("dets")
        );
    }
}

fn interval_coverage_fixture(target_type: ValueType) -> (Schema, impl Fn(u32, u32) -> Value) {
    use crate::schema::IntervalElement;
    use crate::schema::tests::{containment, fd, field, side};
    let element = target_type.interval_element().expect("interval domain");
    let span = move |start: u32, end: u32| match element {
        IntervalElement::U64 => {
            Value::IntervalU64(crate::Interval::new(u64::from(start), u64::from(end)).unwrap())
        }
        IntervalElement::I64 => {
            Value::IntervalI64(crate::Interval::new(i64::from(start), i64::from(end)).unwrap())
        }
        IntervalElement::F64 => Value::IntervalF64(
            crate::Interval::new(
                crate::F64::from(f64::from(start)),
                crate::F64::from(f64::from(end)),
            )
            .unwrap(),
        ),
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Coverage".into(),
                fields: vec![
                    field("group", ValueType::String),
                    field("span", target_type),
                    field("id", ValueType::U64),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Request".into(),
                fields: vec![
                    field("group", ValueType::String),
                    field("span", ValueType::Interval { element }),
                ],
                extension: None,
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(2)]),
            fd(RelationId(0), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("pointwise coverage schema");
    (schema, span)
}

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// A mirror of the committed state, used to sample deletes/replaces of
/// rows that really exist.
#[derive(Default)]
struct Mirror {
    rows: Vec<(RelationId, Vec<Value>)>,
}

impl Mirror {
    fn apply(&mut self, adds: &[(RelationId, Vec<Value>)], removes: &[(RelationId, Vec<Value>)]) {
        self.rows.retain(|row| !removes.contains(row));
        for add in adds {
            if !self.rows.contains(add) {
                self.rows.push(add.clone());
            }
        }
    }

    fn sample(&self, rng: &mut XorShift, relation: RelationId) -> Option<Vec<Value>> {
        let of: Vec<&Vec<Value>> = self
            .rows
            .iter()
            .filter(|(id, _)| *id == relation)
            .map(|(_, values)| values)
            .collect();
        if of.is_empty() {
            None
        } else {
            let at = usize::try_from(rng.below(u64::try_from(of.len()).expect("small fixture")))
                .expect("bounded index");
            Some(of[at].clone())
        }
    }
}

/// One randomized mutation: a handful of adds/removes across all three
/// relations, biased toward key/containment/capacity collisions.
type Rows = Vec<(RelationId, Vec<Value>)>;

fn random_mutation(rng: &mut XorShift, mirror: &Mirror) -> (Rows, Rows) {
    let mut adds = Vec::new();
    let mut removes = Vec::new();
    let moves = 1 + rng.below(3);
    for _ in 0..moves {
        match rng.below(8) {
            0 => adds.push((USER, user(rng.below(24), &format!("mail{}", rng.below(10))))),
            1 => {
                if let Some(row) = mirror.sample(rng, USER) {
                    removes.push((USER, row));
                }
            }
            2 => {
                // Replace: same id, new email — remove + add in one command.
                if let Some(row) = mirror.sample(rng, USER) {
                    let Value::U64(id) = row[0] else {
                        unreachable!()
                    };
                    removes.push((USER, row));
                    adds.push((USER, user(id, &format!("mail{}", rng.below(10)))));
                }
            }
            3 => adds.push((ROOM, room(rng.below(6)))),
            4 => {
                if let Some(row) = mirror.sample(rng, ROOM) {
                    removes.push((ROOM, row));
                }
            }
            5 | 6 => {
                let start = rng.below(40);
                adds.push((
                    BOOKING,
                    booking(rng.below(8), start, start + 1 + rng.below(5)),
                ));
            }
            _ => {
                if let Some(row) = mirror.sample(rng, BOOKING) {
                    removes.push((BOOKING, row));
                }
            }
        }
    }
    if adds.is_empty() && removes.is_empty() {
        adds.push((ROOM, room(rng.below(6))));
    }
    (adds, removes)
}

fn run_differential(store: &Store, schema: &Schema, seed: u64, iterations: u32) {
    let mut rng = XorShift(seed);
    let mut mirror = Mirror::default();
    let mut admitted = 0u32;
    let mut rejected = 0u32;
    for _ in 0..iterations {
        let (adds, removes) = random_mutation(&mut rng, &mirror);
        if compare_and_commit(store, schema, &adds, &removes) {
            mirror.apply(&adds, &removes);
            admitted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(admitted > 0, "the differential must exercise admissions");
    assert!(rejected > 0, "the differential must exercise rejections");
}

#[test]
fn incremental_judgment_matches_complete_judgment_on_randomized_mutations() {
    let (_dir, path) = store_dir("incremental-differential");
    let schema = delta_schema();
    let store = create_with(&path, &schema);
    run_differential(&store, &schema, 0x00C0_FFEE_D00D_F00D, 90);
}

#[test]
fn incremental_judgment_matches_complete_judgment_under_forced_collisions() {
    let (_dir, path) = store_dir("incremental-collision");
    let schema = delta_schema();
    let store = forced(&path, &schema, [0x5A; FP_LEN]);
    run_differential(&store, &schema, 0x1BAD_B002_CAFE_BABE, 40);
}

#[test]
fn capacity_measure_follows_target_row_order_not_delta_group_order() {
    for forced_collision in [false, true] {
        let (_dir, path) = store_dir("capacity-measure-order");
        let schema = delta_schema();
        let store = if forced_collision {
            forced(&path, &schema, [0x5A; FP_LEN])
        } else {
            create_with(&path, &schema)
        };
        for group in [1, 0] {
            assert!(compare_and_commit(
                &store,
                &schema,
                &[(ROOM, room(group)), (BOOKING, booking(group, 0, 1))],
                &[],
            ));
        }
        let changes = build_changes(
            &schema,
            &[
                (BOOKING, booking(0, 1, 2)),
                (BOOKING, booking(0, 2, 3)),
                (BOOKING, booking(1, 1, 2)),
                (BOOKING, booking(1, 2, 3)),
                (BOOKING, booking(1, 3, 4)),
            ],
            &[],
        );
        let violations = rejected(compare(&store, &schema, &changes));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, StatementKind::Capacity);
        assert_eq!(violations[0].measure, Some(3));
        let snapshot = store.snapshot(&work()).expect("snapshot");
        assert_eq!(snapshot.row_count(BOOKING).expect("count"), 2);
    }
}

#[test]
fn a_multi_statement_rejection_is_equal_both_ways_with_all_families() {
    let (_dir, path) = store_dir("incremental-multi");
    let schema = delta_schema();
    let store = create_with(&path, &schema);
    assert!(compare_and_commit(
        &store,
        &schema,
        &[
            (USER, user(1, "a@example")),
            (ROOM, room(1)),
            (BOOKING, booking(1, 0, 5)),
            (BOOKING, booking(1, 5, 8)),
        ],
        &[],
    ));
    let changes = build_changes(
        &schema,
        &[
            (USER, user(2, "a@example")),
            (BOOKING, booking(1, 4, 6)),
            (BOOKING, booking(99, 0, 1)),
        ],
        &[],
    );
    let violations = rejected(compare(&store, &schema, &changes));
    let statements: Vec<StatementId> = violations
        .iter()
        .map(|violation| violation.statement)
        .collect();
    assert_eq!(
        statements,
        [
            StatementId(1),
            StatementId(2),
            StatementId(4),
            StatementId(5)
        ]
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == StatementKind::Capacity
                && violation.measure == Some(3))
    );
}

#[test]
fn an_unlawful_parent_hides_from_incremental_judgment_and_the_sweeper_convicts() {
    let (_dir, path) = store_dir("incremental-unlawful");
    let schema = delta_schema();
    let store = create_with(&path, &schema);
    commit_changes(
        &store,
        &build_changes(
            &schema,
            &[
                (USER, user(1, "dup@example")),
                (USER, user(2, "dup@example")),
                (BOOKING, booking(99, 0, 1)),
                (ROOM, room(1)),
            ],
            &[],
        ),
    );
    let benign = build_changes(&schema, &[(USER, user(3, "fresh@example"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let (incremental, complete) = owner.judge_both(&schema, &benign).expect("judge both");
    assert_eq!(
        incremental,
        Judgment::Admitted,
        "incremental judgment misses untouched standing violations"
    );
    let Judgment::Rejected(complete) = complete else {
        panic!("complete judgment convicts the unlawful parent");
    };
    let convicted: Vec<StatementId> = complete
        .iter()
        .map(|violation| violation.statement)
        .collect();
    assert_eq!(convicted, [USER_EMAIL_KEY, BOOKING_ROOM_EXISTS]);
    drop(owner);
    let snapshot = store.snapshot(&context).expect("snapshot");
    let findings = verify::sweep(&snapshot, &schema, &context).expect("sweep");
    let swept: Vec<StatementId> = findings
        .iter()
        .filter_map(|finding| match finding {
            VerifyFinding::Judgment(violation) => Some(violation.statement),
            VerifyFinding::Corruption(_) => None,
        })
        .collect();
    assert_eq!(swept, [USER_EMAIL_KEY, BOOKING_ROOM_EXISTS]);
}

fn measured_one_row_candidate(store: &Store, schema: &Schema, id: u64) -> u64 {
    let changes = build_changes(
        schema,
        &[(USER, user(id, &format!("solo{id}@example")))],
        &[],
    );
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    let before = crate::alloc_counter::count();
    let candidate = owner.prepare_judged(schema, &changes).expect("prepare");
    let cost = crate::alloc_counter::count() - before;
    let super::super::candidate::Candidate::Admitted(prepared) = candidate else {
        panic!("a fresh user admits");
    };
    prepared.abort();
    cost
}

/// Allocation requests of a one-row candidate stay flat as the relation
/// grows: incremental judgment never builds relation-sized state.
#[test]
fn one_row_candidates_allocate_independently_of_relation_size() {
    let (_dir, path) = store_dir("incremental-workcount");
    let schema = delta_schema();
    let store = create_with(&path, &schema);
    let occupancy: Vec<(RelationId, Vec<Value>)> = (0..64u64)
        .flat_map(|n| {
            vec![
                (ROOM, room(n)),
                (BOOKING, booking(n, 0, 4)),
                (BOOKING, booking(n, 4, 8)),
            ]
        })
        .collect();
    assert!(compare_and_commit(&store, &schema, &occupancy, &[]));
    let seed = |from: u64, to: u64| {
        let rows: Vec<_> = (from..to)
            .map(|n| (USER, user(n, &format!("user{n}@example"))))
            .collect();
        commit_changes(&store, &build_changes(&schema, &rows, &[]));
    };
    seed(0, 256);
    let small = measured_one_row_candidate(&store, &schema, 1_000_001);
    seed(256, 2048);
    let large = measured_one_row_candidate(&store, &schema, 1_000_002);
    assert!(
        large <= small + 32,
        "candidate allocations must not grow with the relation: {small} -> {large}"
    );
}

#[test]
fn home_preservation_does_not_hide_alternate_or_interval_keys() {
    let schema = delta_schema();
    let (_dir, path) = store_dir("home-preservation-other-laws");
    let store = forced(&path, &schema, [0; FP_LEN]);
    assert!(compare_and_commit(
        &store,
        &schema,
        &[(USER, user(1, "same"))],
        &[]
    ));
    assert!(!compare_and_commit(
        &store,
        &schema,
        &[(USER, user(2, "same"))],
        &[]
    ));
    assert_eq!(store.snapshot(&work()).unwrap().row_count(USER).unwrap(), 1);
    assert!(compare_and_commit(
        &store,
        &schema,
        &[(ROOM, room(7)), (BOOKING, booking(7, 0, 10))],
        &[]
    ));
    assert!(!compare_and_commit(
        &store,
        &schema,
        &[(BOOKING, booking(7, 5, 15))],
        &[]
    ));
}
