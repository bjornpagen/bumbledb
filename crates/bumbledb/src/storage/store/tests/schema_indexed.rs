//! F3 finding B regressions: the schema-derived determinant index is real.
//!
//! The store compiles every sealed key statement at open and maintains its
//! determinant entries inside the same transaction as each row mutation —
//! with the production call shape (`UnindexedRows`: no auxiliary entries).
//! These tests pin: symmetric maintenance across insert/replace/delete,
//! bucket-shaped (never relation-shaped) competitor enumeration for
//! judgment (E-ADMIT acceleration, structural work counts), pointwise
//! scalar-prefix bucketing (chapter 10), long text determinants outside
//! LMDB keys (chapter 10 §3), exact confirmation under forced collisions
//! (Q-COLLISION/HASH-02), and snapshot adoption rebuilding the index.

use super::*;
use crate::schema::judge::JudgeBudget;
use crate::schema::{FieldId, StatementDescriptor};
use crate::storage::store::det_index::determinant_bytes;
use crate::storage::store::fingerprint::FP_LEN;
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};
use crate::storage::store::keys::{MEMBERSHIP_KEY_LEN, TAG_DETERMINANT};
use crate::work::Resource;

const USER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const USER_EMAIL_KEY: StatementId = StatementId(1);
const BOOKING_KEY: StatementId = StatementId(2);

/// `User { id: u64, email: str, name: str }` with id and email keys, plus
/// `Booking { room: u64, span: interval u64 }` with the pointwise key
/// `Booking(room, span) -> Booking`.
fn keyed_schema() -> Schema {
    use bumbledb_theory::schema::IntervalElement;
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
                    FieldDescriptor {
                        name: "name".into(),
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
        ],
    }
    .validate()
    .expect("keyed schema validates")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::String(email.into()),
        Value::String("someone".into()),
    ]
}

fn booking(room: u64, start: u64, end: u64) -> Vec<Value> {
    vec![
        Value::U64(room),
        Value::IntervalU64(crate::Interval::new(start, end).expect("nonempty span")),
    ]
}

fn keyed_changes(
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

/// The production commit shape: `UnindexedRows` plus the schema judge.
fn judged_commit(store: &Store, schema: &Schema, changes: &ChangeSet) -> StoreCommit {
    let judge = SchemaJudge {
        schema,
        budget: JudgeBudget::default(),
    };
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare_incremental(
            crate::schema::judge::LawfulParent::established(),
            changes,
            &UnindexedRows,
            &judge,
        )
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared
            .seal(NO_HOST)
            .expect("seal")
            .commit()
            .expect("commit"),
        Prepared::Rejected(violations) => panic!("unexpected rejection: {violations:?}"),
    }
}

/// Count committed determinant entries (and pin every data key far below
/// the LMDB key bound) through one coherent snapshot.
fn determinant_entry_count(store: &Store) -> u64 {
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let mut count = 0u64;
    snapshot
        .entry_census(&context, &mut |is_meta, tag, key_len, _| {
            if !is_meta {
                assert!(
                    key_len <= crate::schema::LMDB_KEY_LIMIT,
                    "no data key may exceed the LMDB key bound (saw {key_len})"
                );
                if tag == TAG_DETERMINANT {
                    count += 1;
                }
            }
            Ok(())
        })
        .expect("census");
    count
}

/// Committed-state bucket lookup through the snapshot: projected bytes by
/// the one convention, then the bucket ids.
fn committed_bucket(
    store: &Store,
    schema: &Schema,
    statement: StatementId,
    determinant: &[Value],
) -> Vec<super::super::format::RowId> {
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let key = snapshot
        .determinants()
        .projection_of(statement)
        .expect("a sealed key statement compiles");
    let _ = schema;
    let projected =
        determinant_bytes(key, determinant, &context).expect("projected bytes");
    snapshot
        .determinant_candidates(key.id, &projected, &context)
        .expect("bucket enumeration")
}

#[test]
fn row_mutations_maintain_schema_determinant_entries_symmetrically() {
    let (_dir, path) = store_dir("schema-indexed-maintenance");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;

    // Two users (two keys each) and one booking (one key): 5 entries.
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[
                (USER, user(1, "a@example")),
                (USER, user(2, "b@example")),
                (BOOKING, booking(9, 0, 10)),
            ],
            &[],
        ),
    );
    assert_eq!(determinant_entry_count(&store), 5);

    // Replacement (one command, remove + add of a new email): the old
    // email's bucket empties, the new email's bucket fills — atomically
    // with the row.
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[(USER, user(1, "c@example"))],
            &[(USER, user(1, "a@example"))],
        ),
    );
    assert_eq!(determinant_entry_count(&store), 5);
    assert_eq!(
        committed_bucket(
            &store,
            &schema,
            USER_EMAIL_KEY,
            &[Value::String("a@example".into())]
        )
        .len(),
        0,
        "the replaced determinant's bucket is empty"
    );
    assert_eq!(
        committed_bucket(
            &store,
            &schema,
            USER_EMAIL_KEY,
            &[Value::String("c@example".into())]
        )
        .len(),
        1,
        "the replacement's bucket holds exactly the new row"
    );

    // Deletes remove entries symmetrically; an emptied store has none.
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[],
            &[
                (USER, user(1, "c@example")),
                (USER, user(2, "b@example")),
                (BOOKING, booking(9, 0, 10)),
            ],
        ),
    );
    assert_eq!(determinant_entry_count(&store), 0);
}

/// Capturing judge: enumerates the proposed final state's competitors for
/// one email through the index and records the structural work cost.
struct CaptureCompetitors {
    email: &'static str,
    seen: std::cell::RefCell<Vec<Box<[Value]>>>,
    enumeration_work: std::cell::Cell<u64>,
}

impl CandidateJudge for CaptureCompetitors {
    type Rejection = std::convert::Infallible;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let before = work.used(Resource::WorkUnits);
        let mut seen = Vec::new();
        candidate
            .visit_determinant_competitors(
                USER_EMAIL_KEY,
                &[Value::String(self.email.into())],
                work,
                &mut |_, values| {
                    seen.push(values.to_vec().into_boxed_slice());
                    Ok(true)
                },
            )?
            .expect("the email key is a sealed statement of this schema");
        self.enumeration_work
            .set(work.used(Resource::WorkUnits) - before);
        *self.seen.borrow_mut() = seen;
        Ok(Judgment::Admitted)
    }
}

#[test]
fn judgment_enumeration_sees_all_competitors_without_scanning_the_relation() {
    let (_dir, path) = store_dir("schema-indexed-competitors");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;

    // Seed a wide relation: 512 committed users with distinct emails.
    let committed_users: Vec<(RelationId, Vec<Value>)> = (0..512u64)
        .map(|n| (USER, user(n, &format!("user{n}@example"))))
        .collect();
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &committed_users, &[]),
    );

    // Propose one MORE row competing with a committed email. The judge
    // must see both the committed row and the proposed row for that
    // determinant — committed index + delta, the proposed final state.
    let capture = CaptureCompetitors {
        email: "user77@example",
        seen: std::cell::RefCell::new(Vec::new()),
        enumeration_work: std::cell::Cell::new(0),
    };
    let changes = keyed_changes(&schema, &[(USER, user(100_077, "user77@example"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &capture)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared.abort(),
        Prepared::Rejected(never) => match never {},
    }

    let seen = capture.seen.borrow();
    assert_eq!(seen.len(), 2, "both competing proposals are visible");
    let mut ids: Vec<u64> = seen
        .iter()
        .map(|row| match row[0] {
            Value::U64(id) => id,
            ref other => panic!("user rows lead with a u64 id, saw {other:?}"),
        })
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![77, 100_077]);

    // STRUCTURAL: enumeration cost is bucket-shaped, not relation-shaped.
    // 512 committed rows; the bucket walk + confirmation of 2 candidates
    // must stay far below one work unit per relation row.
    let cost = capture.enumeration_work.get();
    assert!(
        cost < 128,
        "competitor enumeration must not scan the relation (512 rows): \
         consumed {cost} work units"
    );
}

#[test]
fn pointwise_keys_bucket_by_their_scalar_prefix() {
    let (_dir, path) = store_dir("schema-indexed-pointwise");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[
                (BOOKING, booking(1, 0, 10)),
                (BOOKING, booking(1, 10, 20)),
                (BOOKING, booking(2, 0, 10)),
            ],
            &[],
        ),
    );
    // The determinant group is the scalar prefix: both room-1 bookings
    // cohabit one bucket (their disjoint spans are the judged tail), room
    // 2 stands alone.
    assert_eq!(
        committed_bucket(&store, &schema, BOOKING_KEY, &[Value::U64(1)]).len(),
        2
    );
    assert_eq!(
        committed_bucket(&store, &schema, BOOKING_KEY, &[Value::U64(2)]).len(),
        1
    );
    assert_eq!(
        committed_bucket(&store, &schema, BOOKING_KEY, &[Value::U64(3)]).len(),
        0
    );
}

#[test]
fn long_text_determinants_stay_out_of_lmdb_keys_and_still_resolve() {
    let (_dir, path) = store_dir("schema-indexed-long-text");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;
    // Far past the 511-byte LMDB key bound.
    let long_a = "a".repeat(4096) + "@example";
    let long_b = "b".repeat(4096) + "@example";
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[(USER, user(1, &long_a)), (USER, user(2, &long_b))],
            &[],
        ),
    );
    // The census assertion inside pins every data key ≤ 29 bytes.
    assert_eq!(determinant_entry_count(&store), 4);
    let bucket = committed_bucket(
        &store,
        &schema,
        USER_EMAIL_KEY,
        &[Value::String(long_a.as_str().into())],
    );
    assert_eq!(bucket.len(), 1, "the long determinant resolves to its row");
    assert!(
        committed_bucket(
            &store,
            &schema,
            USER_EMAIL_KEY,
            &[Value::String((long_a + "x").as_str().into())],
        )
        .is_empty(),
        "a near-miss long determinant misses"
    );
}

#[test]
fn forced_collisions_widen_buckets_but_never_answers() {
    let (_dir, path) = store_dir("schema-indexed-collision");
    let schema = keyed_schema();
    let store =
        Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0xAB; FP_LEN])
            .expect("forced-collision store");
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[
                (USER, user(1, "a@example")),
                (USER, user(2, "b@example")),
                (USER, user(3, "c@example")),
            ],
            &[],
        ),
    );
    // Every determinant shares the one forced bucket…
    assert_eq!(
        committed_bucket(
            &store,
            &schema,
            USER_EMAIL_KEY,
            &[Value::String("b@example".into())]
        )
        .len(),
        3,
        "the forced bucket holds every user's email entry"
    );
    // …and exact confirmation still isolates the one true competitor.
    let capture = CaptureCompetitors {
        email: "b@example",
        seen: std::cell::RefCell::new(Vec::new()),
        enumeration_work: std::cell::Cell::new(0),
    };
    let changes = keyed_changes(&schema, &[(USER, user(9, "unrelated@example"))], &[]);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(&changes, &UnindexedRows, &capture)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared.abort(),
        Prepared::Rejected(never) => match never {},
    }
    let seen = capture.seen.borrow();
    assert_eq!(
        seen.len(),
        1,
        "collision cohabitants are excluded by exact decoded equality"
    );
    assert_eq!(seen[0][0], Value::U64(2));
}

#[test]
fn adopt_snapshot_rebuilds_the_determinant_index() {
    let (_dir, path) = store_dir("schema-indexed-adopt-src");
    let (_dir2, dest_path) = store_dir("schema-indexed-adopt-dest");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default()).expect("create").0;
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[
                (USER, user(1, "a@example")),
                (USER, user(2, "b@example")),
                (BOOKING, booking(4, 2, 6)),
            ],
            &[],
        ),
    );
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let (dest, fresh) = Store::create(&dest_path, &schema, MapPolicy::default()).expect("dest");
    dest.adopt_snapshot(&snapshot, fresh, &UnindexedRows, &context)
        .expect("adopt");
    assert_eq!(determinant_entry_count(&dest), 5);
    assert_eq!(
        committed_bucket(
            &dest,
            &schema,
            USER_EMAIL_KEY,
            &[Value::String("b@example".into())]
        )
        .len(),
        1,
        "the adopted store answers keyed lookups through its own index"
    );
}

/// CORE-002: the bucket visitor confirms rows one at a time without a
/// pre-collected id vector.
#[test]
fn visit_determinant_bucket_streams_candidates() {
    use crate::storage::store::rows;
    let dir = TempDir::new("bounded-visitor");
    let schema = keyed_schema();
    let store = Store::create(&dir.path().join("store"), &schema, MapPolicy::default())
        .expect("create")
        .0;
    let work = work();
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[
                (USER, user(1, "alpha@example.com")),
                (USER, user(2, "beta@example.com")),
            ],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work).expect("snapshot");
    let inner = snapshot.store_inner();
    let txn = snapshot.read_txn();
    let key = snapshot
        .determinants()
        .projection_of(USER_EMAIL_KEY)
        .expect("email key");
    let projected = determinant_bytes(key, &[Value::String("alpha@example.com".into())], &work)
        .expect("project");
    let routing = rows::routing_for_projected(inner, key.id, &projected).expect("route");
    let mut count = 0u32;
    rows::visit_determinant_bucket(inner, txn, key.id, &routing, &work, &mut |_id| {
        count += 1;
        Ok(true)
    })
    .expect("visit");
    assert_eq!(count, 1, "exactly the matching email row in the bucket");
}

/// L05 seam: committed visits are named by `ProjectionId`.
/// Verification NotRun.
#[test]
fn owned_snapshot_visit_projection_takes_projection_id() {
    let dir = TempDir::new("visit-projection");
    let schema = keyed_schema();
    let store = Store::create(&dir.path().join("store"), &schema, MapPolicy::default())
        .expect("create")
        .0;
    let work = work();
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[(USER, user(1, "alpha@example.com"))], &[]),
    );
    let snapshot = store.snapshot(&work).expect("snapshot");
    let key = snapshot
        .determinants()
        .projection_of(USER_EMAIL_KEY)
        .expect("email key");
    let projected = determinant_bytes(key, &[Value::String("alpha@example.com".into())], &work)
        .expect("project");
    let mut seen = 0u32;
    snapshot
        .visit_projection(key.id, &projected, &work, &mut |_id, bytes| {
            seen += 1;
            assert!(!bytes.is_empty());
            Ok(true)
        })
        .expect("visit");
    assert_eq!(seen, 1);
}
