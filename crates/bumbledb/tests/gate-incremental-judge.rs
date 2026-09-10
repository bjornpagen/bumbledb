//! Gate: incremental production judgment (chapter 10 §4, F3 G-C).
//!
//! For a mutation whose relations carry determinant indexes, the production
//! incremental entry judges only the statements and determinant groups the
//! delta can affect — sound under the LAWFUL-PARENT PREMISE (the committed
//! parent satisfies every statement; production maintains it inductively,
//! and the offline sweeper re-runs the COMPLETE reference judgment so a
//! parent made unlawful outside the admission path stays detectable). The
//! bounded reference judge (`judge_final_state`) remains the oracle.
//!
//! This gate pins complete judgment on the store candidate protocol and
//! incremental locality on the admitted Db integration path:
//! - differential equivalence: randomized small theories/mutations judged
//!   BOTH ways — verdicts, complete violation sets, and canonical evidence
//!   bytes equal, across adds, deletes, replaces and multi-statement
//!   rejections (forced-collision variant under `collision-probe`);
//! - allocation regression: a one-row mutation's allocation requests do not
//!   scale with the parent relation. Run with `alloc-counter` under nextest;
//!   these counts are not physical-read counters. Core no-scan doubles
//!   independently exercise the indexed access contract.
//!
//! Gate families: E-ADMIT (incremental half), G15/PERF-001 (structural
//! judge work), Q-COLLISION (exact verdicts under forced collisions).

use bumbledb::integration::Preparation;
use bumbledb::schema::judge::{
    CandidateFacts, JudgeBudget, JudgedViolation, Judgment as SchemaJudgment, judge_final_state,
};
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::store::{
    CandidateJudge, CandidateState, Judgment, MapPolicy, Prepared, SchemaJudge, Store, StoreResult,
    UnindexedRows,
};
use bumbledb::work::WorkContext;
use bumbledb::{ChangeSet, Interval, Value};

mod common;

const USER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const ROOM: RelationId = RelationId(2);

fn work() -> WorkContext {
    WorkContext::new()
}

/// `User(id)`, `User(email)`, pointwise `Booking(room, span)`, `Room(id)`,
/// `Booking(room) ⊆ Room(id)`, `Booking(room) <= {0..2} Room(id)`.
fn theory_descriptor() -> SchemaDescriptor {
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::from([]),
    };
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "User".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("email", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64)],
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
}

fn theory() -> Schema {
    theory_descriptor()
        .validate()
        .expect("gate theory validates")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

fn booking(room: u64, start: u64, end: u64) -> Vec<Value> {
    vec![
        Value::U64(room),
        Value::IntervalU64(Interval::new(start, end).expect("nonempty span")),
    ]
}

fn room(id: u64) -> Vec<Value> {
    vec![Value::U64(id)]
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

/// The complete reference judgment's view of the SAME candidate: streamed
/// full relations, decoded — the oracle's shape (and the sweeper's).
struct ReferenceFacts<'v, 'a, 'store> {
    candidate: &'v CandidateState<'a, 'store>,
    schema: &'v Schema,
    work: &'v WorkContext,
}

impl CandidateFacts for ReferenceFacts<'_, '_, '_> {
    type Error = bumbledb::store::StoreError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        self.visit_ranked_rows(relation, &mut |_rank, values| visit(values))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: bumbledb::schema::judge::RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        let fields = self.schema.relation(relation).fields();
        for entry in self.candidate.rows(relation)? {
            let (row_id, bytes) = entry?;
            let decoded = bumbledb::canonical::decode(fields, bytes, self.work)?;
            if !visit(row_id.id.0, decoded.values())? {
                break;
            }
        }
        Ok(())
    }
}

fn evidence_bytes(schema: &Schema, judged: &[JudgedViolation]) -> Vec<u8> {
    bumbledb::schema::evidence::encode_judged(schema, judged, 1 << 20, &work())
        .expect("evidence encodes")
}

/// The differential judge: production complete judgment and the
/// complete reference over one candidate; verdicts, complete violation
/// sets and canonical evidence bytes must be equal.
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
        let production = SchemaJudge::new(self.schema).judge(candidate, work)?;
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
        .expect("the reference judgment completes");
        match (&production, &reference) {
            (Judgment::Admitted, SchemaJudgment::Admitted) => {}
            (Judgment::Rejected(mine), SchemaJudgment::Rejected(complete)) => {
                assert_eq!(
                    mine.as_ref(),
                    complete.as_ref(),
                    "production and reference violation sets must be equal"
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
                .seal(bumbledb::store::HostChanges {
                    records: &[],
                    attachment: bumbledb::store::AttachmentChange::Keep,
                })
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

/// Random adds/removes/replaces biased toward key/containment/capacity
/// collisions — small theories, small mutations.
type Rows = Vec<(RelationId, Vec<Value>)>;

fn random_mutation(rng: &mut XorShift, mirror: &Mirror) -> (Rows, Rows) {
    let mut adds = Vec::new();
    let mut removes = Vec::new();
    for _ in 0..=rng.below(3) {
        match rng.below(8) {
            0 => adds.push((USER, user(rng.below(24), &format!("mail{}", rng.below(10))))),
            1 => {
                if let Some(row) = mirror.sample(rng, USER) {
                    removes.push((USER, row));
                }
            }
            2 => {
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
    let (mut admitted, mut rejected) = (0u32, 0u32);
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
fn complete_production_and_independent_reference_agree_on_randomized_mutations() {
    let dir = common::TempDir::new("gate-inc-judge-differential");
    let schema = theory();
    std::fs::create_dir_all(dir.path()).expect("parent dir");
    let store = Store::create(&dir.path().join("store"), &schema, MapPolicy::default())
        .expect("create")
        .0;
    run_differential(&store, &schema, 0xD1FF_E4E7_71A1_0001_u64, 70);
}

/// Under total forced fingerprint collisions every determinant bucket
/// holds every cohabitant; exact decoded confirmation must keep the
/// incremental verdicts byte-equal with the oracle. The forcing
/// constructor exists only under the `collision-probe` feature; run with
/// `--features collision-probe`.
#[cfg(feature = "collision-probe")]
#[test]
fn complete_production_and_independent_reference_agree_under_forced_collisions() {
    use bumbledb::store::FP_LEN;
    let dir = common::TempDir::new("gate-inc-judge-collision");
    let schema = theory();
    std::fs::create_dir_all(dir.path()).expect("parent dir");
    let store = Store::create_forced_fingerprint(
        &dir.path().join("store"),
        &schema,
        MapPolicy::default(),
        [0xEE; FP_LEN],
    )
    .expect("forced-collision store");
    run_differential(&store, &schema, 0xC011_1DED_0000_0002_u64, 40);
}

fn commit_admitted(db: &bumbledb::Db<SchemaDescriptor>, adds: &[(RelationId, Vec<Value>)]) {
    let changes = build_changes(db.schema(), adds, &[]);
    assert!(matches!(
        db.apply(&changes, bumbledb::ApplyExpected::Any, &work())
            .expect("population"),
        bumbledb::ApplyOutcome::Accepted { .. } | bumbledb::ApplyOutcome::NoChange { .. }
    ));
}

fn measured_one_row_judgment(db: &bumbledb::Db<SchemaDescriptor>, id: u64) -> u64 {
    let changes = build_changes(
        db.schema(),
        &[(USER, user(id, &format!("solo{id}@example")))],
        &[],
    );
    let context = work();
    let mut session = db.integration_writer(&context).expect("writer");
    let before = bumbledb::alloc_counter::count();
    let prepared = session.prepare(&changes).expect("prepare");
    assert!(matches!(prepared, Preparation::Accepted(_)));
    bumbledb::alloc_counter::count() - before
}

/// Allocation requests for one-row admission stay flat as the relation grows
/// 8×. This observes allocation behavior, not visited pages or elapsed time.
#[test]
fn one_row_judge_allocations_are_flat_across_relation_growth() {
    let dir = common::TempDir::new("gate-inc-judge-work");
    let db = bumbledb::Db::create(dir.path(), theory_descriptor(), work())
        .expect("create")
        .unwrap();

    // Bookings/rooms a streamed containment/capacity judgment would have
    // to walk; the user mutation never touches them.
    let occupancy: Vec<(RelationId, Vec<Value>)> = (0..64u64)
        .flat_map(|n| {
            vec![
                (ROOM, room(n)),
                (BOOKING, booking(n, 0, 4)),
                (BOOKING, booking(n, 4, 8)),
            ]
        })
        .collect();
    commit_admitted(&db, &occupancy);

    let users: Vec<(RelationId, Vec<Value>)> = (0..512u64)
        .map(|n| (USER, user(n, &format!("user{n}@example"))))
        .collect();
    commit_admitted(&db, &users);
    let small = measured_one_row_judgment(&db, 2_000_001);

    let more: Vec<(RelationId, Vec<Value>)> = (512..4096u64)
        .map(|n| (USER, user(n, &format!("user{n}@example"))))
        .collect();
    commit_admitted(&db, &more);
    let large = measured_one_row_judgment(&db, 2_000_002);

    assert!(
        small < 256,
        "one-row judgment against 512 rows must be delta-shaped: {small} allocation requests"
    );
    assert!(
        large < 256,
        "one-row judgment against 4096 rows (+192 bookings/rooms) must stay \
         delta-shaped: {large} allocation requests"
    );
    assert!(
        large <= small + 32,
        "judgment work must not grow with the relation: {small} -> {large}"
    );
}
