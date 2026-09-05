//! F3 G-C regressions: the incremental production judgment (chapter 10 §4).
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
use crate::work::Resource;
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
        let fields = self.schema.relation(relation).fields();
        for entry in self.candidate.rows(relation)? {
            let (_, bytes) = entry?;
            let decoded = crate::canonical::decode(fields, bytes, self.work)?;
            if !visit(decoded.values())? {
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
            (mine, complete) => panic!(
                "verdicts diverged: production {mine:?} vs reference {complete:?}"
            ),
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
            prepared.seal(NO_HOST).expect("seal").commit().expect("commit");
            true
        }
        Prepared::Rejected(violations) => {
            assert!(!violations.is_empty(), "a rejection names its statements");
            false
        }
    }
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
fn random_mutation(
    rng: &mut XorShift,
    mirror: &Mirror,
) -> (Vec<(RelationId, Vec<Value>)>, Vec<(RelationId, Vec<Value>)>) {
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
                    let id = match row[0] {
                        Value::U64(id) => id,
                        _ => unreachable!(),
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
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;
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
fn a_multi_statement_rejection_is_equal_both_ways_with_all_families() {
    let (_dir, path) = store_dir("incremental-multi");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;
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
        Prepared::Rejected(violations) => {
            let statements: Vec<StatementId> =
                violations.iter().map(|violation| violation.statement).collect();
            assert_eq!(
                statements,
                vec![StatementId(1), StatementId(2), StatementId(4), StatementId(5)],
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
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;

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
                prepared.seal(NO_HOST).expect("seal").commit().expect("commit");
            }
            Prepared::Rejected(never) => match never {},
        }
    }

    // A benign mutation touching none of the standing violations: the
    // production (incremental) judge ADMITS — it may miss what the delta
    // does not touch — while the complete reference on the SAME candidate
    // rejects. This divergence is the premise, asserted, not hidden.
    let benign = build_changes(&schema, &[(USER, user(3, "fresh@example"))], &[]);
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
            let statements: Vec<StatementId> =
                violations.iter().map(|violation| violation.statement).collect();
            assert_eq!(statements, vec![USER_EMAIL_KEY, BOOKING_ROOM_EXISTS]);
            Ok(Judgment::Admitted)
        }
    }
    {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        match owner
            .prepare(&benign, &UnindexedRows, &PremiseWitness { schema: &schema })
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => prepared.abort(),
            Prepared::Rejected(never) => match never {},
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

/// A judge wrapper measuring the production judgment's own work-unit cost.
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
        let before = work.used(Resource::WorkUnits);
        let judged = SchemaJudge::new(self.schema).judge_incremental(
            LawfulParent::established(),
            candidate,
            work,
        )?;
        self.cost.set(work.used(Resource::WorkUnits) - before);
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
            prepared.seal(NO_HOST).expect("seal").commit().expect("commit");
        }
        Prepared::Rejected(never) => match never {},
    }
}

fn measured_one_row_judgment(store: &Store, schema: &Schema, id: u64) -> u64 {
    let judge = MeasuredJudge {
        schema,
        cost: std::cell::Cell::new(0),
    };
    let changes = build_changes(schema, &[(USER, user(id, &format!("solo{id}@example")))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &judge)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared.abort(),
        Prepared::Rejected(violations) => panic!("unexpected rejection: {violations:?}"),
    }
    judge.cost.get()
}

/// STRUCTURAL work regression (never timing): the production judgment of a
/// one-row mutation costs work proportional to the delta's determinant
/// groups, not to the relation — flat under an 8× relation growth and far
/// below one work unit per relation row, with the untouched containment/
/// capacity relations never entering the judgment at all.
#[test]
fn incremental_judgment_work_is_delta_shaped_not_relation_shaped() {
    let (_dir, path) = store_dir("incremental-workcount");
    let schema = delta_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;

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

    assert!(
        small < 256,
        "one-row judgment against 256 rows must be delta-shaped: {small} work units"
    );
    assert!(
        large < 256,
        "one-row judgment against 2048 rows (+192 bookings/rooms) must stay \
         delta-shaped: {large} work units"
    );
    assert!(
        large <= small + 32,
        "judgment work must not grow with the relation: {small} -> {large}"
    );
}
