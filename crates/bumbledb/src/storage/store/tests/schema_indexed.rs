//! F3 finding B regressions: the schema-derived determinant index is real.
//!
//! The store compiles every sealed key statement at open and maintains its
//! determinant entries inside the same transaction as each row mutation —
//! with the production call shape (`UnindexedRows`: no auxiliary entries).
//! These tests pin: symmetric maintenance across insert/replace/delete,
//! bucket-shaped (never relation-shaped) competitor enumeration for
//! judgment (E-ADMIT acceleration, structural work counts), pointwise
//! scalar-prefix bucketing, long text determinants outside
//! LMDB keys, exact confirmation under forced collisions
//! (Q-COLLISION/HASH-02), and snapshot adoption rebuilding the index.

use super::*;
use crate::schema::judge::JudgeBudget;
use crate::schema::{FieldId, StatementDescriptor};
use crate::storage::store::det_index::determinant_bytes;
use crate::storage::store::error::StoreCorruption;
use crate::storage::store::fingerprint::FP_LEN;
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};
use crate::storage::store::keys::TAG_DETERMINANT;
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
) -> Vec<super::super::RowLocator> {
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let key = snapshot
        .determinants()
        .projection_of(statement)
        .expect("a sealed key statement compiles");
    let _ = schema;
    let projected = determinant_bytes(key, determinant, &context).expect("projected bytes");
    snapshot
        .determinant_candidates(key.id, &projected, &context)
        .expect("bucket enumeration")
}

fn assert_user_placement(
    store: &Store,
    schema: &Schema,
    id: u64,
    email: &str,
) -> super::super::RowLocator {
    let context = work();
    let snapshot = store.snapshot(&context).expect("snapshot");
    let primary = snapshot
        .determinants()
        .membership_projection(USER)
        .expect("scalar home");
    let secondary = snapshot
        .determinants()
        .projection_of(USER_EMAIL_KEY)
        .expect("email key");
    let primary_bucket = snapshot
        .determinant_candidates(primary.id, &id.to_be_bytes(), &context)
        .expect("primary rows");
    assert_eq!(primary_bucket.len(), 1);
    let locator = primary_bucket[0];
    assert_eq!(locator.home(), id.to_be_bytes());
    let canonical = crate::canonical::CanonicalRow::encode(
        schema.relation(USER).fields(),
        &user(id, email),
        &context,
    )
    .expect("canonical user");
    assert_eq!(
        snapshot.fetch(USER, locator).unwrap(),
        Some(canonical.as_bytes())
    );
    assert!(snapshot.contains(USER, &canonical, &context).unwrap());
    let inner = snapshot.store_inner();
    let primary_key = inner.keys.row_key(USER, locator).expect("primary key");
    assert_eq!(primary_key.len(), snapshot.physical_key_widths().row + 8);
    assert_eq!(
        inner.data.get(snapshot.read_txn(), &primary_key).unwrap(),
        Some(canonical.as_bytes())
    );
    let absent_index = inner.keys.determinant_bucket(primary.id, &[]).unwrap();
    assert_eq!(
        inner
            .data
            .prefix_iter(snapshot.read_txn(), &absent_index)
            .unwrap()
            .count(),
        0,
        "home index is the primary rows, not a duplicate DET namespace"
    );
    let projected = determinant_bytes(secondary, &[Value::String(email.into())], &context).unwrap();
    assert_eq!(
        snapshot
            .determinant_candidates(secondary.id, &projected, &context)
            .unwrap(),
        primary_bucket
    );
    let routing =
        super::super::rows::routing_for_projected(inner, secondary.id, &projected).unwrap();
    let secondary_key = inner
        .keys
        .determinant_key(secondary.id, &routing, locator.id)
        .unwrap();
    assert_eq!(
        inner.data.get(snapshot.read_txn(), &secondary_key).unwrap(),
        Some(locator.home()),
        "secondary stores only the scalar home; ordinal already lives in its key"
    );
    locator
}

#[test]
fn scalar_home_and_secondary_locator_change_atomically_across_replacement_and_reinsertion() {
    let (_dir, path) = store_dir("scalar-home-move");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .unwrap()
        .0;
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[(USER, user(9, "same@example"))], &[]),
    );
    let old = assert_user_placement(&store, &schema, 9, "same@example");
    let pinned = store.snapshot(&work()).unwrap();
    assert_eq!(
        store.inner.data.len(pinned.read_txn()).unwrap(),
        2,
        "one primary row and one secondary, no directory or membership"
    );

    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[(USER, user(1, "same@example"))],
            &[(USER, user(9, "same@example"))],
        ),
    );
    let moved = assert_user_placement(&store, &schema, 1, "same@example");
    assert!(moved.id > old.id);
    let fresh = store.snapshot(&work()).unwrap();
    assert!(fresh.fetch(USER, old).unwrap().is_none());
    assert!(pinned.fetch(USER, old).unwrap().is_some());
    assert!(pinned.fetch(USER, moved).unwrap().is_none());
    assert!(committed_bucket(&store, &schema, StatementId(0), &[Value::U64(9)]).is_empty());
    drop((fresh, pinned));

    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[], &[(USER, user(1, "same@example"))]),
    );
    assert!(
        store
            .inner
            .data
            .is_empty(store.snapshot(&work()).unwrap().read_txn())
            .unwrap()
    );
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[(USER, user(1, "same@example"))], &[]),
    );
    let reinserted = assert_user_placement(&store, &schema, 1, "same@example");
    assert!(reinserted.id > moved.id);
    assert_eq!(reinserted.home(), moved.home());
    assert!(
        store
            .snapshot(&work())
            .unwrap()
            .fetch(USER, moved)
            .unwrap()
            .is_none()
    );
}

#[test]
fn nonleading_uuid_fixed_bytes_and_reordered_composite_keys_are_primary_homes() {
    let cases = [
        (
            "uuid",
            vec![ValueType::Uuid],
            vec![Value::Uuid(crate::Uuid::from_bytes([0xa5; 16]))],
            vec![FieldId(1)],
            vec![0xa5; 16],
        ),
        (
            "fixed",
            vec![ValueType::FixedBytes { len: 8 }],
            vec![Value::FixedBytes(Box::new([0x5a; 8]))],
            vec![FieldId(1)],
            vec![0x5a; 8],
        ),
        (
            "composite",
            vec![ValueType::U64, ValueType::U64],
            vec![Value::U64(7), Value::U64(9)],
            vec![FieldId(2), FieldId(1)],
            [9u64.to_be_bytes(), 7u64.to_be_bytes()].concat(),
        ),
    ];
    for (label, types, scalars, selected, expected) in cases {
        let (_dir, path) = store_dir(label);
        let mut fields = vec![FieldDescriptor {
            name: "payload".into(),
            value_type: ValueType::String,
        }];
        fields.extend(
            types
                .into_iter()
                .enumerate()
                .map(|(index, value_type)| FieldDescriptor {
                    name: format!("k{index}").into(),
                    value_type,
                }),
        );
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Record".into(),
                fields,
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: USER,
                projection: selected.into_boxed_slice(),
            }],
        }
        .validate()
        .unwrap();
        let store = Store::create(&path, &schema, MapPolicy::default())
            .unwrap()
            .0;
        let mut values = vec![Value::String("payload".into())];
        values.extend(scalars);
        judged_commit(
            &store,
            &schema,
            &keyed_changes(&schema, &[(USER, values.clone())], &[]),
        );
        assert_single_primary_row(&store, &schema, &mut values, &expected);
    }
}

fn assert_single_primary_row(
    store: &Store,
    schema: &Schema,
    values: &mut [Value],
    expected: &[u8],
) {
    let context = work();
    let snapshot = store.snapshot(&context).unwrap();
    let (locator, canonical) = snapshot.rows(USER).unwrap().next().unwrap().unwrap();
    assert_eq!(locator.home(), expected);
    assert_eq!(snapshot.fetch(USER, locator).unwrap(), Some(canonical));
    assert!(snapshot.contains(USER, canonical, &context).unwrap());
    let projection = snapshot.determinants().membership_projection(USER).unwrap();
    assert_eq!(
        snapshot
            .determinant_candidates(projection.id, expected, &context)
            .unwrap(),
        [locator]
    );
    assert_eq!(
        store.inner.data.len(snapshot.read_txn()).unwrap(),
        1,
        "one physical primary entry, no home DET/directory/membership"
    );
    let (physical_key, physical_value) = store
        .inner
        .data
        .iter(snapshot.read_txn())
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(
        physical_key.len(),
        snapshot.physical_key_widths().row + expected.len()
    );
    assert_eq!(
        store.inner.keys.decode_row(physical_key).unwrap(),
        (USER, locator)
    );
    assert_eq!(physical_value, canonical);
    values[0] = Value::String("different body, same key".into());
    let near_miss =
        crate::canonical::CanonicalRow::encode(schema.relation(USER).fields(), values, &context)
            .unwrap();
    assert!(
        !snapshot.contains(USER, &near_miss, &context).unwrap(),
        "home equality never substitutes for full row equality"
    );
}

#[test]
fn row_mutations_maintain_schema_determinant_entries_symmetrically() {
    let (_dir, path) = store_dir("schema-indexed-maintenance");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;

    // Users live directly under their id key, retaining only the secondary
    // email entry. The pointwise booking key remains secondary: 3 entries.
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
    assert_eq!(determinant_entry_count(&store), 3);

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
    assert_eq!(determinant_entry_count(&store), 3);
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
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;

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
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
    judged_commit(
        &store,
        &schema,
        &keyed_changes(
            &schema,
            &[(BOOKING, booking(1, 10, 20)), (BOOKING, booking(2, 0, 10))],
            &[],
        ),
    );
    // Separate commits prevent canonical mutation sorting from making
    // row ordinals accidentally agree with interval endpoint order.
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[(BOOKING, booking(1, 0, 10))], &[]),
    );
    {
        let context = work();
        let snapshot = store.snapshot(&context).expect("snapshot");
        let expected = snapshot.physical_key_widths().determinant_overhead + 8;
        let mut entries = 0;
        snapshot
            .entry_census(&context, &mut |is_meta, tag, key_len, value_len| {
                if !is_meta && tag == TAG_DETERMINANT {
                    // FAIL FIRST: the old physical interval tail adds 16.
                    assert_eq!(key_len, expected, "only scalar route and row ordinal");
                    assert_eq!(value_len, 0, "pointwise-only relation has no home");
                    entries += 1;
                }
                Ok(())
            })
            .expect("physical census");
        assert_eq!(entries, 3);
    }
    let room_one = committed_bucket(&store, &schema, BOOKING_KEY, &[Value::U64(1)]);
    assert_eq!(room_one.len(), 2);
    assert!(
        room_one[0].id.0 < room_one[1].id.0,
        "ordinal, not endpoint order"
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
    judged_commit(
        &store,
        &schema,
        &keyed_changes(&schema, &[], &[(BOOKING, booking(1, 10, 20))]),
    );
    assert_eq!(
        committed_bucket(&store, &schema, BOOKING_KEY, &[Value::U64(1)]),
        vec![room_one[1]],
        "deleting one interval preserves the other physical locator"
    );
    assert_eq!(determinant_entry_count(&store), 2);
    assert_bucket_rejects_extra_route_byte(&store, BOOKING_KEY, &1u64.to_be_bytes(), room_one[1]);
}

fn assert_bucket_rejects_extra_route_byte(
    store: &Store,
    statement: StatementId,
    routing: &[u8],
    locator: super::super::RowLocator,
) {
    let context = work();
    let projection = store.inner.det.projection_of(statement).unwrap().id;
    let mut key = store
        .inner
        .keys
        .determinant_key(projection, routing, locator.id)
        .unwrap()
        .to_vec();
    // Still within the global 16-byte route bound, but not this schema's
    // route width. It must not manufacture a second visit of the same row.
    key.insert(key.len() - 8, 0);
    let mut txn = store.gated_write_txn(&context).unwrap();
    store
        .inner
        .data
        .put(&mut txn.txn, &key, locator.home())
        .unwrap();
    txn.commit().unwrap();
    let snapshot = store.snapshot(&context).unwrap();
    assert!(matches!(
        snapshot.determinant_candidates(projection, routing, &context),
        Err(StoreError::Corruption(StoreCorruption::MalformedKey(_)))
    ));
}

#[test]
fn long_text_determinants_stay_out_of_lmdb_keys_and_still_resolve() {
    let (_dir, path) = store_dir("schema-indexed-long-text");
    let schema = keyed_schema();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
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
    // Only the secondary text key has a determinant entry; the primary
    // id is embedded in each row key, never duplicated in this namespace.
    assert_eq!(determinant_entry_count(&store), 2);
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
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
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
    assert_eq!(determinant_entry_count(&dest), 3);
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
    rows::visit_determinant_bucket(inner, txn, key, &routing, &work, &mut |_locator, _bytes| {
        count += 1;
        Ok(true)
    })
    .expect("visit");
    assert_eq!(count, 1, "exactly the matching email row in the bucket");
}

/// L05 seam: committed visits are named by `ProjectionId`.
/// Verification `NotRun`.
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
    for (statement, value) in [
        (StatementId(0), Value::U64(1)),
        (USER_EMAIL_KEY, Value::String("alpha@example.com".into())),
    ] {
        let key = snapshot.determinants().projection_of(statement).unwrap();
        let projection = snapshot
            .projection(key.id)
            .expect("sealed snapshot projection");
        assert!(std::ptr::eq(key, projection.compiled()));
        let projected = determinant_bytes(key, &[value], &work).expect("project");
        let mut public_row = None;
        snapshot
            .visit_projection(key.id, &projected, &work, &mut |id, bytes| {
                assert!(public_row.replace((id, bytes)).is_none());
                Ok(true)
            })
            .expect("public visit");
        let before = work.used(Resource::WorkUnits);
        let mut resolved_row = None;
        projection
            .visit(&projected, &work, &mut |id, bytes| {
                assert!(resolved_row.replace((id, bytes)).is_none());
                Ok(true)
            })
            .expect("resolved visit");
        assert_eq!(
            work.used(Resource::WorkUnits) - before,
            1,
            "one existing storage step per hit"
        );
        let before_probe = work.used(Resource::WorkUnits);
        let mut probed_row = None;
        projection
            .probe(&projected, &work, &mut |id, bytes| {
                assert!(probed_row.replace((id, bytes)).is_none());
                Ok(false)
            })
            .expect("first-match probe");
        assert_eq!(work.used(Resource::WorkUnits) - before_probe, 1);
        assert_eq!(probed_row, public_row);
        let (public_id, public_bytes) = public_row.expect("public hit");
        assert!(std::ptr::eq(public_bytes, probed_row.unwrap().1));
        let (resolved_id, resolved_bytes) = resolved_row.expect("resolved hit");
        assert_eq!(public_id, resolved_id);
        assert!(
            std::ptr::eq(public_bytes, resolved_bytes),
            "both visitors borrow the same pinned body after cursor drop"
        );
        let stopped = super::work();
        stopped.cancel();
        assert_eq!(
            projection.visit(&projected, &stopped, &mut |_, _| panic!(
                "cancelled before visitor"
            )),
            Err(StoreError::Work(crate::work::WorkError::Cancelled))
        );
        assert_eq!(
            projection.visit(&projected, &work, &mut |_, _| Err(StoreError::Allocation)),
            Err(StoreError::Allocation)
        );
        assert_eq!(
            projection.probe(&projected, &stopped, &mut |_, _| panic!("cancelled probe")),
            Err(StoreError::Work(crate::work::WorkError::Cancelled))
        );
        assert_eq!(
            projection.probe(&projected, &work, &mut |_, _| Err(StoreError::Allocation)),
            Err(StoreError::Allocation)
        );
    }
}

#[test]
fn primary_probe_continues_unready_conflicts_and_preserves_work() {
    use crate::storage::store::staging::UnreadyStore;
    let (_dir, path) = store_dir("primary-probe-conflicts");
    let schema = keyed_schema();
    let context = work();
    let staged = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &context).unwrap();
    let changes = keyed_changes(
        &schema,
        &[
            (USER, user(1, "a@example")),
            (USER, user(1, "b@example")),
            (USER, user(1, "c@example")),
            (USER, user(3, "other@example")),
        ],
        &[],
    );
    staged
        .populate(&context, |stage, work| stage.apply(&changes, work))
        .unwrap();
    staged
        .inspect(&context, |stage, work| {
            let snapshot = stage.snapshot();
            let key = snapshot
                .determinants()
                .projection_of(StatementId(0))
                .unwrap();
            let projection = snapshot.projection(key.id).unwrap();
            let route = 1u64.to_be_bytes();
            let mut ordinary = Vec::new();
            snapshot.visit_projection(key.id, &route, work, &mut |id, bytes| {
                ordinary.push((id, bytes));
                Ok(true)
            })?;
            assert_eq!(ordinary.len(), 3, "staging retains every conflict");
            for limit in [1usize, 2, 3, 4] {
                let before = work.used(Resource::WorkUnits);
                let mut probed = Vec::new();
                projection.probe(&route, work, &mut |id, bytes| {
                    probed.push((id, bytes));
                    Ok(probed.len() < limit)
                })?;
                assert_eq!(probed, ordinary[..limit.min(3)]);
                assert_eq!(work.used(Resource::WorkUnits) - before, probed.len() as u64);
                for ((_, left), (_, right)) in probed.iter().zip(&ordinary) {
                    assert!(
                        std::ptr::eq(*left, *right),
                        "borrow survives cursor destruction"
                    );
                }
            }
            // The first miss stops at another home; the second reaches tree end.
            for missing in [2u64, 4] {
                let route = missing.to_be_bytes();
                let before = work.used(Resource::WorkUnits);
                projection.probe(&route, work, &mut |_, _| panic!("missing home"))?;
                assert_eq!(work.used(Resource::WorkUnits), before);
                let cancelled = super::work();
                cancelled.cancel();
                assert_eq!(
                    projection.probe(&route, &cancelled, &mut |_, _| panic!("cancelled miss")),
                    Err(StoreError::Work(crate::work::WorkError::Cancelled))
                );
            }
            // Both a further candidate and the terminal seek after a singleton
            // must observe cancellation requested by a continuing callback.
            for home in [1u64, 3] {
                let cancelled = super::work();
                let mut visits = 0;
                assert_eq!(
                    projection.probe(&home.to_be_bytes(), &cancelled, &mut |_, _| {
                        visits += 1;
                        cancelled.cancel();
                        Ok(true)
                    }),
                    Err(StoreError::Work(crate::work::WorkError::Cancelled))
                );
                assert_eq!(
                    visits, 1,
                    "continuation observes cancellation after its seek"
                );
            }
            Ok(())
        })
        .unwrap();
}

#[test]
fn primary_probe_validates_first_and_continuation_keys_without_eager_work() {
    for malformed_first in [true, false] {
        let (_dir, path) = store_dir("primary-probe-malformed");
        let schema = keyed_schema();
        let store = Store::create(&path, &schema, MapPolicy::default())
            .unwrap()
            .0;
        judged_commit(
            &store,
            &schema,
            &keyed_changes(&schema, &[(USER, user(1, "a@example"))], &[]),
        );
        let route = 1u64.to_be_bytes();
        let context = work();
        let locator = committed_bucket(&store, &schema, StatementId(0), &[Value::U64(1)])[0];
        let malformed = if malformed_first {
            let mut key = store
                .inner
                .keys
                .row_bucket(USER, &route)
                .unwrap()
                .as_slice()
                .to_vec();
            key.extend_from_slice(&[0; 7]);
            key
        } else {
            let mut key = store
                .inner
                .keys
                .row_key(USER, locator)
                .unwrap()
                .as_slice()
                .to_vec();
            key.push(0);
            key
        };
        let mut txn = store.gated_write_txn(&context).unwrap();
        store.inner.data.put(&mut txn.txn, &malformed, &[]).unwrap();
        txn.commit().unwrap();
        let snapshot = store.snapshot(&context).unwrap();
        let key = snapshot
            .determinants()
            .projection_of(StatementId(0))
            .unwrap();
        let projection = snapshot.projection(key.id).unwrap();
        let mut visits = 0;
        let result = projection.probe(&route, &context, &mut |_, _| {
            visits += 1;
            Ok(false)
        });
        if malformed_first {
            assert!(matches!(result, Err(StoreError::Corruption(_))));
            assert_eq!(visits, 0);
        } else {
            result.unwrap();
            assert_eq!(visits, 1, "stop must not seek the malformed successor");
        }
        visits = 0;
        assert!(matches!(
            projection.probe(&route, &context, &mut |_, _| {
                visits += 1;
                Ok(true)
            }),
            Err(StoreError::Corruption(_))
        ));
        assert_eq!(visits, usize::from(!malformed_first));
    }
}
