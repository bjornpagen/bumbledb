//! F3 G-C regressions: the incremental production judgment.
//!
//! `SchemaJudge` now judges a delta-carrying candidate through
//! `judge_final_state_delta_local`: statements the delta cannot affect are
//! skipped and key statements are judged from the delta-touched determinant
//! groups (`CandidateState::visit_determinant_competitors`) instead of streaming
//! whole relations. These tests pin, on the REAL store candidate path:
//!
//! - differential equivalence with the complete reference judgment —
//!   verdicts, complete violation sets, and canonical evidence bytes equal
//!   over randomized theories/mutations (adds, deletes, replaces,
//!   multi-statement rejections), also under forced total fingerprint
//!   collisions;
//! - the lawful-parent premise pinned honestly: a parent seeded UNLAWFULLY
//!   through the test judge can hide from the incremental path, the
//!   complete reference convicts it, and the store sweeper — which always
//!   re-runs the COMPLETE judgment — reports it;
//! - structural work counts: judging a small mutation costs work
//!   proportional to the delta's groups, not to the relation (flat across
//!   an 8× relation growth, under a flat ceiling).

use super::*;
use crate::schema::judge::{
    CandidateFacts, JudgeBudget, JudgedViolation, Judgment as SchemaJudgment, LawfulParent,
    judge_final_state,
};
use crate::schema::{FieldId, Side, StatementDescriptor, StatementKind, Weight};
use crate::storage::store::fingerprint::FP_LEN;
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};
use crate::storage::store::verify::{self, VerifyFinding};
use bumbledb_theory::schema::Bound;

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

/// The complete reference judgment's view of the SAME candidate: streamed
/// full relations, decoded — exactly the sweeper's judgment shape.
struct ReferenceFacts<'v, 'a, 'store> {
    candidate: &'v CandidateState<'a, 'store>,
    schema: &'v Schema,
    work: &'v WorkContext,
}

impl CandidateFacts for ReferenceFacts<'_, '_, '_> {
    type Error = StoreError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<(), StoreError> {
        self.visit_ranked_rows(relation, &mut |_rank, values| visit(values))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: crate::schema::judge::RankedRowVisitor<'_, StoreError>,
    ) -> Result<(), StoreError> {
        let fields = self.schema.relation(relation).fields();
        for entry in self.candidate.rows(relation)? {
            let (row_id, bytes) = entry?;
            let decoded = crate::canonical::decode(fields, bytes, self.work)?;
            if !visit(row_id.id.0, decoded.values())? {
                break;
            }
        }
        Ok(())
    }
}

fn evidence_bytes(schema: &Schema, judged: &[JudgedViolation]) -> Vec<u8> {
    crate::schema::evidence::encode_judged(schema, judged, 1 << 20, &work())
        .expect("evidence encodes")
}

/// The differential judge: runs the PRODUCTION `SchemaJudge` (incremental
/// for delta-carrying candidates) and the complete reference judgment over
/// one candidate state, requires verdicts, complete violation sets, and
/// canonical evidence bytes equal, then returns the production outcome.
struct CompareJudge<'s> {
    schema: &'s Schema,
}

impl CandidateJudge for CompareJudge<'_> {
    type Rejection = Box<[JudgedViolation]>;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let production = SchemaJudge::new(self.schema).judge_incremental(
            LawfulParent::established(),
            candidate,
            work,
        )?;
        let reference_view = ReferenceFacts {
            candidate,
            schema: self.schema,
            work,
        };
        let reference =
            judge_final_state(self.schema, &reference_view, work, JudgeBudget::default())
                .expect("the reference judgment completes");
        match (&production, &reference) {
            (Judgment::Admitted, SchemaJudgment::Admitted) => {}
            (Judgment::Rejected(mine), SchemaJudgment::Rejected(complete)) => {
                assert_eq!(
                    mine.as_ref(),
                    complete.as_ref(),
                    "incremental and complete violation sets must be equal"
                );
                assert_eq!(
                    evidence_bytes(self.schema, mine),
                    evidence_bytes(self.schema, complete),
                    "canonical evidence bytes must be byte-equal"
                );
            }
            (mine, complete) => {
                panic!("verdicts diverged: production {mine:?} vs reference {complete:?}")
            }
        }
        Ok(production)
    }
}

fn build_changes(
    schema: &Schema,
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
) -> ChangeSet {
    let mut builder = ChangeSet::builder(schema, work());
    for (relation, values) in removes {
        builder.delete(*relation, values).expect("stage delete");
    }
    for (relation, values) in adds {
        builder.insert(*relation, values).expect("stage insert");
    }
    builder.finish().expect("sealed change set")
}

/// Prepare one delta under the differential judge; commit on admission.
/// Returns whether the mutation was admitted.
fn compare_and_commit(
    store: &Store,
    schema: &Schema,
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
) -> bool {
    let changes = build_changes(schema, adds, removes);
    let judge = CompareJudge { schema };
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &judge)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => {
            prepared
                .seal(NO_HOST)
                .expect("seal")
                .commit()
                .expect("commit");
            true
        }
        Prepared::Rejected {
            rejection: violations,
            ..
        } => {
            assert!(!violations.is_empty(), "a rejection names its statements");
            false
        }
    }
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
        let store =
            Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0xA5; FP_LEN])
                .expect("colliding scalar groups");
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
            let context = work();
            let snapshot = store.snapshot(&context).expect("snapshot");
            let mut entries = 0;
            snapshot
                .entry_census(&context, &mut |is_meta, tag, key_len, _| {
                    if !is_meta && tag == super::super::keys::TAG_DETERMINANT {
                        assert_eq!(
                            key_len,
                            snapshot.physical_key_widths().determinant_overhead + FP_LEN,
                            "ordinary, float, and fixed intervals add no physical tail"
                        );
                        entries += 1;
                    }
                    Ok(())
                })
                .expect("physical census");
            assert_eq!(
                entries, 4,
                "three target references and one source reference"
            );
        }
        assert!(commit(&[], &[late, early, foreign, request]));
        let context = work();
        let snapshot = store.snapshot(&context).expect("empty snapshot");
        assert!(
            store
                .inner
                .data
                .is_empty(snapshot.read_txn())
                .expect("all rows and indexes removed")
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
fn incremental_judge_matches_the_complete_judge_on_randomized_mutations() {
    let (_dir, path) = store_dir("incremental-differential");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
    run_differential(&store, &schema, 0x00C0_FFEE_D00D_F00D, 90);
}

#[test]
fn incremental_judge_matches_the_complete_judge_under_forced_collisions() {
    let (_dir, path) = store_dir("incremental-collision");
    let schema = delta_schema();
    // Every fingerprint collides: every bucket holds every cohabitant, and
    // exact decoded confirmation alone separates groups. Verdicts must
    // still be byte-equal with the complete judge.
    let store =
        Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0x5A; FP_LEN])
            .expect("forced-collision store");
    run_differential(&store, &schema, 0x1BAD_B002_CAFE_BABE, 40);
}

#[test]
fn capacity_measure_follows_target_row_order_not_delta_group_order() {
    for forced_collision in [false, true] {
        let (_dir, path) = store_dir("capacity-measure-order");
        let schema = delta_schema();
        let store = if forced_collision {
            Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0x5A; FP_LEN])
                .expect("forced-collision store")
        } else {
            Store::create(&path, &schema, MapPolicy::default())
                .expect("create")
                .0
        };
        // Separate lawful commits make the target row order room1, room0,
        // independent of the canonical order within each change set.
        for group in [1, 0] {
            assert!(compare_and_commit(
                &store,
                &schema,
                &[(ROOM, room(group)), (BOOKING, booking(group, 0, 1))],
                &[],
            ));
        }
        // Canonical delta order visits room0 then room1. Their totals are
        // three and four; the reference target order must witness three.
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
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        match owner
            .prepare(&changes, &UnindexedRows, &CompareJudge { schema: &schema })
            .expect("prepare")
        {
            Prepared::Admitted(_) => panic!("both room capacities are exceeded"),
            Prepared::Rejected {
                rejection: violations,
                ..
            } => {
                assert_eq!(violations.len(), 1);
                assert_eq!(violations[0].kind, StatementKind::Capacity);
                assert_eq!(violations[0].measure, Some(3));
            }
        }
        drop(owner);
        let snapshot = store.snapshot(&context).expect("snapshot");
        assert_eq!(snapshot.row_count(BOOKING).expect("count"), 2);
    }
}

#[test]
fn a_multi_statement_rejection_is_equal_both_ways_with_all_families() {
    let (_dir, path) = store_dir("incremental-multi");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
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
    // One delta violating the email key, the pointwise booking key, the
    // room containment, and the room capacity at once; CompareJudge pins
    // both paths equal, and the rejection names every statement.
    let changes = build_changes(
        &schema,
        &[
            (USER, user(2, "a@example")),
            (BOOKING, booking(1, 4, 6)),
            (BOOKING, booking(99, 0, 1)),
        ],
        &[],
    );
    let judge = CompareJudge { schema: &schema };
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &judge)
        .expect("prepare")
    {
        Prepared::Admitted(_) => panic!("this delta violates four statements"),
        Prepared::Rejected {
            rejection: violations,
            ..
        } => {
            let statements: Vec<StatementId> = violations
                .iter()
                .map(|violation| violation.statement)
                .collect();
            assert_eq!(
                statements,
                vec![
                    StatementId(1),
                    StatementId(2),
                    StatementId(4),
                    StatementId(5)
                ],
                "every violated family is named in canonical order"
            );
            assert!(
                violations
                    .iter()
                    .any(|violation| violation.kind == StatementKind::Capacity
                        && violation.measure == Some(3)),
                "the capacity violation witnesses its exact widened measure"
            );
        }
    }
}

struct PremiseWitness<'s> {
    schema: &'s Schema,
}
impl CandidateJudge for PremiseWitness<'_> {
    type Rejection = std::convert::Infallible;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let production = SchemaJudge::new(self.schema).judge_incremental(
            LawfulParent::established(),
            candidate,
            work,
        )?;
        assert!(
            matches!(production, Judgment::Admitted),
            "the incremental judge misses untouched standing violations"
        );
        let reference = judge_final_state(
            self.schema,
            &ReferenceFacts {
                candidate,
                schema: self.schema,
                work,
            },
            work,
            JudgeBudget::default(),
        )
        .expect("reference completes");
        let SchemaJudgment::Rejected(violations) = reference else {
            panic!("the complete judge must convict the unlawful parent");
        };
        let statements: Vec<StatementId> = violations
            .iter()
            .map(|violation| violation.statement)
            .collect();
        assert_eq!(statements, vec![USER_EMAIL_KEY, BOOKING_ROOM_EXISTS]);
        Ok(Judgment::Admitted)
    }
}

/// The lawful-parent premise, pinned honestly on the physical store: a
/// parent seeded UNLAWFULLY through a permissive test judge (a state the
/// production admission path cannot produce) can hide from the incremental
/// production judgment; the complete reference convicts it on the same
/// candidate; and the store sweeper — which always re-runs the COMPLETE
/// judgment — reports it offline.
#[test]
fn an_unlawful_parent_hides_from_the_incremental_judge_and_the_sweeper_convicts() {
    let (_dir, path) = store_dir("incremental-unlawful");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;

    // Seed the unlawful parent: duplicate emails and an orphan booking,
    // committed past judgment through the permissive test judge.
    let seeded = build_changes(
        &schema,
        &[
            (USER, user(1, "dup@example")),
            (USER, user(2, "dup@example")),
            (BOOKING, booking(99, 0, 1)),
            (ROOM, room(1)),
        ],
        &[],
    );
    {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        match owner
            .prepare(&seeded, &UnindexedRows, &AdmitAll)
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => {
                prepared
                    .seal(NO_HOST)
                    .expect("seal")
                    .commit()
                    .expect("commit");
            }
            Prepared::Rejected {
                rejection: never, ..
            } => match never {},
        }
    }

    // A benign mutation touching none of the standing violations: the
    // production (incremental) judge ADMITS — it may miss what the delta
    // does not touch — while the complete reference on the SAME candidate
    // rejects. This divergence is the premise, asserted, not hidden.
    let benign = build_changes(&schema, &[(USER, user(3, "fresh@example"))], &[]);
    {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        match owner
            .prepare(&benign, &UnindexedRows, &PremiseWitness { schema: &schema })
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => prepared.abort(),
            Prepared::Rejected {
                rejection: never, ..
            } => match never {},
        }
    }

    // The sweeper re-runs the COMPLETE judgment over the committed state:
    // the unlawful parent is detectable there, always.
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let findings = verify::sweep(&snapshot, &schema, &context).expect("sweep completes");
    let convicted: Vec<StatementId> = findings
        .iter()
        .filter_map(|finding| match finding {
            VerifyFinding::Judgment(violation) => Some(violation.statement),
            VerifyFinding::Corruption(_) => None,
        })
        .collect();
    assert_eq!(
        convicted,
        vec![USER_EMAIL_KEY, BOOKING_ROOM_EXISTS],
        "the sweeper's complete re-judgment convicts the unlawful parent"
    );
}

/// A judge wrapper measuring actual allocation requests inside judgment.
struct MeasuredJudge<'s> {
    schema: &'s Schema,
    cost: std::cell::Cell<u64>,
}

impl CandidateJudge for MeasuredJudge<'_> {
    type Rejection = Box<[JudgedViolation]>;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let before = crate::alloc_counter::count();
        let judged = SchemaJudge::new(self.schema).judge_incremental(
            LawfulParent::established(),
            candidate,
            work,
        )?;
        self.cost.set(crate::alloc_counter::count() - before);
        Ok(judged)
    }
}

fn seed_users(store: &Store, schema: &Schema, from: u64, to: u64) {
    let rows: Vec<(RelationId, Vec<Value>)> = (from..to)
        .map(|n| (USER, user(n, &format!("user{n}@example"))))
        .collect();
    let changes = build_changes(schema, &rows, &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &AdmitAll)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => {
            prepared
                .seal(NO_HOST)
                .expect("seal")
                .commit()
                .expect("commit");
        }
        Prepared::Rejected {
            rejection: never, ..
        } => match never {},
    }
}

fn measured_one_row_judgment(store: &Store, schema: &Schema, id: u64) -> u64 {
    let judge = MeasuredJudge {
        schema,
        cost: std::cell::Cell::new(0),
    };
    let changes = build_changes(
        schema,
        &[(USER, user(id, &format!("solo{id}@example")))],
        &[],
    );
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &judge)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared.abort(),
        Prepared::Rejected {
            rejection: violations,
            ..
        } => panic!("unexpected rejection: {violations:?}"),
    }
    judge.cost.get()
}

/// Allocation regression, not timing or a count of physical storage reads.
/// One-row judgment must not create relation-sized temporary representations.
/// The judge's no-scan doubles independently enforce indexed traversal.
#[test]
fn incremental_judgment_allocations_are_delta_shaped_not_relation_shaped() {
    let (_dir, path) = store_dir("incremental-workcount");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;

    // A large booking/room population that a streamed containment/capacity
    // judgment would have to walk — the user mutation must never touch it.
    // (Rooms hold at most 2 bookings, so occupancy spreads over many rooms.)
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

    seed_users(&store, &schema, 0, 256);
    let small = measured_one_row_judgment(&store, &schema, 1_000_001);

    seed_users(&store, &schema, 256, 2048);
    let large = measured_one_row_judgment(&store, &schema, 1_000_002);

    #[cfg(feature = "alloc-counter")]
    {
        assert!(
            small > 0 && large > 0,
            "the allocator counter must observe real work"
        );
        assert!(
            small < 256,
            "one-row judgment against 256 rows must be delta-shaped: {small} allocation requests"
        );
        assert!(
            large < 256,
            "one-row judgment against 2048 rows (+192 bookings/rooms) must stay \
         delta-shaped: {large} allocation requests"
        );
        assert!(
            large <= small + 32,
            "judgment allocations must not grow with the relation: {small} -> {large}"
        );
    }
    let _ = (small, large);
}

#[test]
fn selected_home_preservation_does_not_hide_alternate_or_interval_keys() {
    let schema = delta_schema();
    let (_dir, path) = store_dir("home-preservation-other-laws");
    let store = Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0; FP_LEN])
        .unwrap();
    assert!(compare_and_commit(
        &store,
        &schema,
        &[(USER, user(1, "same"))],
        &[]
    ));
    // Different exact homes, but the alternate text key still conflicts.
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
